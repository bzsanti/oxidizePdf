//! High-level PDF Reader API
//!
//! Provides a simple interface for reading PDF files

use super::encryption_handler::EncryptionHandler;
use super::header::PdfHeader;
use super::indexed_object_stream::IndexedObjectStream;
use super::objects::{PdfArray, PdfDictionary, PdfObject, PdfString};
use super::stack_safe::StackSafeContext;
use super::trailer::PdfTrailer;
use super::xref::{
    find_byte_pattern, read_object_window, read_window_at, scan_page_object_refs, XRefTable,
};
use super::{ParseError, ParseResult};
use crate::objects::ObjectId;
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, Read, Seek, SeekFrom};
use std::path::Path;

/// Resolve a dictionary value that is expected to be an array into an owned
/// [`PdfArray`], transparently following a single level of indirection.
///
/// ISO 32000-1 §7.3.10 permits any object — including `/Kids` — to be written
/// as an indirect reference (`N G R`) instead of inline; iText 5.5.9 emits page
/// trees this way. Returns `None` if the value is absent, is neither an array
/// nor a reference to one, or cannot be resolved. Only one level is followed,
/// which covers every real-world page tree observed (a reference-to-a-reference
/// `/Kids` chain is not produced by any known writer).
pub(crate) fn resolve_to_array<R: Read + Seek>(
    reader: &mut PdfReader<R>,
    value: Option<&PdfObject>,
) -> Option<PdfArray> {
    match value {
        Some(PdfObject::Reference(num, gen)) => {
            reader.get_object(*num, *gen).ok()?.as_array().cloned()
        }
        other => other.and_then(|o| o.as_array()).cloned(),
    }
}

/// Bounded window for manual dictionary extraction (Issue #339). Object headers
/// are located by the chunked scanner and only this many bytes are read at the
/// object offset, instead of buffering the whole file. Large enough for any
/// realistic catalog / pages dictionary (incl. multi-thousand-entry `/Kids`).
const MANUAL_DICT_WINDOW: usize = 256 * 1024;

/// High-level PDF reader
pub struct PdfReader<R: Read + Seek> {
    reader: BufReader<R>,
    header: PdfHeader,
    xref: XRefTable,
    trailer: PdfTrailer,
    /// Cache of loaded objects
    object_cache: HashMap<(u32, u16), PdfObject>,
    /// Cache of object streams
    object_stream_cache: HashMap<u32, IndexedObjectStream>,
    /// Page tree navigator
    page_tree: Option<super::page_tree::PageTree>,
    /// Stack-safe parsing context
    parse_context: StackSafeContext,
    /// Parsing options
    options: super::ParseOptions,
    /// Encryption handler (if PDF is encrypted)
    encryption_handler: Option<EncryptionHandler>,
    /// Track objects currently being reconstructed (circular reference detection)
    objects_being_reconstructed: std::sync::Mutex<std::collections::HashSet<u32>>,
    /// Maximum reconstruction depth (prevents pathological cases)
    max_reconstruction_depth: u32,
    /// Verification must not synthesize missing or malformed objects.
    allow_object_reconstruction: bool,
}

impl<R: Read + Seek> PdfReader<R> {
    /// Get parsing options
    pub fn options(&self) -> &super::ParseOptions {
        &self.options
    }

    /// Disable object repair for callers assessing structure rather than recovering it.
    pub(crate) fn disable_object_reconstruction(&mut self) {
        self.allow_object_reconstruction = false;
    }

    /// Check if the PDF is encrypted
    pub fn is_encrypted(&self) -> bool {
        self.encryption_handler.is_some()
    }

    /// Access the parsed document trailer.
    ///
    /// Exposes the already-parsed [`PdfTrailer`], which carries the base
    /// `startxref` offset (`xref_offset`), and the `/Root`, `/Info`, `/ID`
    /// and `/Size` entries. Required to build a conformant ISO 32000-1
    /// §7.5.6 incremental update (the appended trailer must chain its
    /// `/Prev` to this offset and reuse the base `/Root` and `/ID`).
    pub fn trailer(&self) -> &PdfTrailer {
        &self.trailer
    }

    /// Replace only the I/O buffer, preserving logical position and parser state.
    pub(crate) fn with_buffer_capacity(mut self, capacity: usize) -> ParseResult<Self> {
        let position = self.reader.stream_position()?;
        self.reader.seek(SeekFrom::Start(position))?;
        self.reader = BufReader::with_capacity(capacity.max(1), self.reader.into_inner());
        Ok(self)
    }

    /// Return the latest in-use indirect-object references known to the xref.
    pub(crate) fn object_references(&self) -> Vec<(u32, u16)> {
        self.xref.in_use_references()
    }

    /// Return the storage offset of the latest definition of an object.
    pub(crate) fn object_storage_offset(&self, object_number: u32) -> Option<u64> {
        self.xref.object_storage_offset(object_number)
    }

    /// Whether the current definition resides in an object stream.
    pub(crate) fn object_is_compressed(&self, object_number: u32) -> bool {
        self.xref.is_compressed(object_number)
    }

    /// Return physical xref revisions from the base revision to the latest.
    pub(crate) fn xref_revisions(&self) -> Vec<super::xref::XRefRevision> {
        self.xref.revisions_oldest_first()
    }

    /// Check if the PDF is unlocked (can read encrypted content)
    pub fn is_unlocked(&self) -> bool {
        match &self.encryption_handler {
            Some(handler) => handler.is_unlocked(),
            None => true, // Unencrypted PDFs are always "unlocked"
        }
    }

    /// Get mutable access to encryption handler
    pub fn encryption_handler_mut(&mut self) -> Option<&mut EncryptionHandler> {
        self.encryption_handler.as_mut()
    }

    /// Get access to encryption handler
    pub fn encryption_handler(&self) -> Option<&EncryptionHandler> {
        self.encryption_handler.as_ref()
    }

    /// Try to unlock PDF with password
    pub fn unlock_with_password(&mut self, password: &str) -> ParseResult<bool> {
        match &mut self.encryption_handler {
            Some(handler) => {
                // A password that does not match returns Ok(false); an Err means
                // the encryption dictionary itself could not be processed — a
                // truncated /U, an unsupported revision. Both were collapsed into
                // "wrong password", which is how the real cause of issue #459
                // stayed invisible: the reporter was told to find a password for
                // a document whose empty password was already correct.
                //
                // The owner password still gets its turn before any error is
                // raised: a document whose /O is unusable can open on its /U.
                let user = handler.unlock_with_user_password(password);
                if matches!(user, Ok(true)) {
                    return Ok(true);
                }
                let owner = handler.unlock_with_owner_password(password);
                if matches!(owner, Ok(true)) {
                    return Ok(true);
                }
                match (user, owner) {
                    (Err(e), _) | (Ok(_), Err(e)) => Err(e),
                    (Ok(_), Ok(_)) => Ok(false),
                }
            }
            None => Ok(true), // Not encrypted
        }
    }

    /// Try to unlock with empty password
    pub fn try_empty_password(&mut self) -> ParseResult<bool> {
        match &mut self.encryption_handler {
            Some(handler) => Ok(handler.try_empty_password().unwrap_or(false)),
            None => Ok(true), // Not encrypted
        }
    }

    /// Unlock encrypted PDF with password
    ///
    /// Attempts to unlock the PDF using the provided password (tries both user
    /// and owner passwords). If the PDF is not encrypted, this method returns
    /// `Ok(())` immediately.
    ///
    /// # Arguments
    ///
    /// * `password` - User or owner password for the PDF
    ///
    /// # Errors
    ///
    /// Returns `ParseError::WrongPassword` if the password is incorrect.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use oxidize_pdf::parser::PdfReader;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut reader = PdfReader::open("encrypted.pdf")?;
    ///
    /// if reader.is_encrypted() {
    ///     reader.unlock("password")?;
    /// }
    ///
    /// let catalog = reader.catalog()?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn unlock(&mut self, password: &str) -> ParseResult<()> {
        // If not encrypted, nothing to do
        if !self.is_encrypted() {
            return Ok(());
        }

        // Early return if already unlocked (idempotent)
        if self.is_unlocked() {
            return Ok(());
        }

        // Try to unlock with password (tries user and owner)
        let success = self.unlock_with_password(password)?;

        if success {
            Ok(())
        } else {
            Err(ParseError::WrongPassword)
        }
    }

    /// Check if PDF is locked and return error if so
    fn ensure_unlocked(&self) -> ParseResult<()> {
        if self.is_encrypted() && !self.is_unlocked() {
            return Err(ParseError::PdfLocked);
        }
        Ok(())
    }

    /// Decrypt an object if encryption is active
    ///
    /// This method recursively decrypts strings and streams within the object.
    /// Objects that don't contain encrypted data (numbers, names, booleans, null,
    /// references) are returned unchanged.
    fn decrypt_object_if_needed(
        &self,
        obj: PdfObject,
        obj_num: u32,
        gen_num: u16,
    ) -> ParseResult<PdfObject> {
        // Only decrypt if encryption is active and unlocked
        let handler = match &self.encryption_handler {
            Some(h) if h.is_unlocked() => h,
            _ => return Ok(obj), // Not encrypted or not unlocked
        };

        let obj_id = ObjectId::new(obj_num, gen_num);

        match obj {
            PdfObject::String(ref s) => {
                // Decrypt string
                let decrypted_bytes = handler.decrypt_string(s.as_bytes(), &obj_id)?;
                Ok(PdfObject::String(PdfString::new(decrypted_bytes)))
            }
            PdfObject::Stream(ref stream) => {
                // Check if stream should be decrypted (Identity filter means no decryption)
                let should_decrypt = stream
                    .dict
                    .get("StmF")
                    .and_then(|o| o.as_name())
                    .map(|n| n.0.as_str() != "Identity")
                    .unwrap_or(true); // Default: decrypt if no /StmF

                if should_decrypt {
                    let decrypted_data = handler.decrypt_stream(&stream.data, &obj_id)?;

                    // Create new stream with decrypted data
                    let mut new_stream = stream.clone();
                    new_stream.data = decrypted_data;
                    Ok(PdfObject::Stream(new_stream))
                } else {
                    Ok(obj) // Don't decrypt /Identity streams
                }
            }
            PdfObject::Dictionary(ref dict) => {
                // Recursively decrypt dictionary values
                let mut new_dict = PdfDictionary::new();
                for (key, value) in dict.0.iter() {
                    let decrypted_value =
                        self.decrypt_object_if_needed(value.clone(), obj_num, gen_num)?;
                    new_dict.insert(key.0.clone(), decrypted_value);
                }
                Ok(PdfObject::Dictionary(new_dict))
            }
            PdfObject::Array(ref arr) => {
                // Recursively decrypt array elements
                let mut new_arr = Vec::new();
                for elem in arr.0.iter() {
                    let decrypted_elem =
                        self.decrypt_object_if_needed(elem.clone(), obj_num, gen_num)?;
                    new_arr.push(decrypted_elem);
                }
                Ok(PdfObject::Array(PdfArray(new_arr)))
            }
            // Other types (Integer, Real, Boolean, Name, Null, Reference) don't get encrypted
            _ => Ok(obj),
        }
    }
}

impl PdfReader<File> {
    /// Open a PDF file from a path
    pub fn open<P: AsRef<Path>>(path: P) -> ParseResult<Self> {
        #[cfg(feature = "verbose-debug")]
        {
            use std::io::Write;
            if let Ok(mut f) = std::fs::File::create("/tmp/pdf_open_debug.log") {
                writeln!(f, "Opening file: {:?}", path.as_ref()).ok();
            }
        }
        let file = File::open(path)?;
        // Use lenient options by default for maximum compatibility
        let options = super::ParseOptions::lenient();
        Self::new_with_options(file, options)
    }

    /// Open a PDF file from a path with strict parsing
    pub fn open_strict<P: AsRef<Path>>(path: P) -> ParseResult<Self> {
        let file = File::open(path)?;
        let options = super::ParseOptions::strict();
        Self::new_with_options(file, options)
    }

    /// Open a PDF file from a path with custom parsing options
    pub fn open_with_options<P: AsRef<Path>>(
        path: P,
        options: super::ParseOptions,
    ) -> ParseResult<Self> {
        let file = File::open(path)?;
        Self::new_with_options(file, options)
    }

    /// Open a PDF file as a PdfDocument
    pub fn open_document<P: AsRef<Path>>(
        path: P,
    ) -> ParseResult<super::document::PdfDocument<File>> {
        let reader = Self::open(path)?;
        Ok(reader.into_document())
    }
}

impl<R: Read + Seek> PdfReader<R> {
    /// Create a new PDF reader from a reader
    ///
    /// Uses default parsing options with `lenient_streams` enabled for
    /// compatibility with real-world PDFs that use indirect references for
    /// stream lengths. Use `new_with_options` with `ParseOptions::strict()`
    /// if you need fully strict validation.
    pub fn new(reader: R) -> ParseResult<Self> {
        // Enable lenient_streams by default to handle indirect Length references
        // This is consistent with PdfReader::open() behavior
        let mut options = super::ParseOptions::default();
        options.lenient_streams = true;
        Self::new_with_options(reader, options)
    }

    /// Create a new PDF reader with custom parsing options
    pub fn new_with_options(reader: R, options: super::ParseOptions) -> ParseResult<Self> {
        let mut buf_reader = BufReader::new(reader);

        // Check if file is empty
        let start_pos = buf_reader.stream_position()?;
        buf_reader.seek(SeekFrom::End(0))?;
        let file_size = buf_reader.stream_position()?;
        buf_reader.seek(SeekFrom::Start(start_pos))?;

        if file_size == 0 {
            return Err(ParseError::EmptyFile);
        }

        // Parse header
        let header = PdfHeader::parse(&mut buf_reader)?;
        #[cfg(feature = "verbose-debug")]
        tracing::debug!("Header parsed: version {}", header.version);

        // Parse xref table
        let xref = XRefTable::parse_with_options(&mut buf_reader, &options)?;
        #[cfg(feature = "verbose-debug")]
        tracing::debug!("XRef table parsed with {} entries", xref.len());

        // Get trailer
        let trailer_dict = xref.trailer().ok_or(ParseError::InvalidTrailer)?.clone();

        let xref_offset = xref.xref_offset();
        let trailer = PdfTrailer::from_dict(trailer_dict, xref_offset)?;

        // Validate trailer
        trailer.validate()?;

        // Check for encryption
        let encryption_handler = if EncryptionHandler::detect_encryption(trailer.dict()) {
            if let Ok(Some((encrypt_obj_num, encrypt_gen_num))) = trailer.encrypt() {
                // We need to temporarily create the reader to load the encryption dictionary
                let mut temp_reader = Self {
                    reader: buf_reader,
                    header: header.clone(),
                    xref: xref.clone(),
                    trailer: trailer.clone(),
                    object_cache: HashMap::new(),
                    object_stream_cache: HashMap::new(),
                    page_tree: None,
                    parse_context: StackSafeContext::new(),
                    options: options.clone(),
                    encryption_handler: None,
                    objects_being_reconstructed: std::sync::Mutex::new(
                        std::collections::HashSet::new(),
                    ),
                    max_reconstruction_depth: 100,
                    allow_object_reconstruction: true,
                };

                // Load encryption dictionary
                let encrypt_obj = temp_reader.get_object(encrypt_obj_num, encrypt_gen_num)?;
                if let Some(encrypt_dict) = encrypt_obj.as_dict() {
                    // Get file ID from trailer
                    let file_id = trailer.id().and_then(|id_obj| {
                        if let PdfObject::Array(ref id_array) = id_obj {
                            if let Some(PdfObject::String(ref id_bytes)) = id_array.get(0) {
                                Some(id_bytes.as_bytes().to_vec())
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    });

                    match EncryptionHandler::new(encrypt_dict, file_id) {
                        Ok(mut handler) => {
                            // Auto-unlock with empty password (common for permission-restricted PDFs)
                            let _ = handler.try_empty_password();
                            // Move the reader back out
                            buf_reader = temp_reader.reader;
                            Some(handler)
                        }
                        Err(_) => {
                            // Move reader back and continue without encryption
                            let _ = temp_reader.reader;
                            return Err(ParseError::EncryptionNotSupported);
                        }
                    }
                } else {
                    let _ = temp_reader.reader;
                    return Err(ParseError::EncryptionNotSupported);
                }
            } else {
                return Err(ParseError::EncryptionNotSupported);
            }
        } else {
            None
        };

        Ok(Self {
            reader: buf_reader,
            header,
            xref,
            trailer,
            object_cache: HashMap::new(),
            object_stream_cache: HashMap::new(),
            page_tree: None,
            parse_context: StackSafeContext::new(),
            options,
            encryption_handler,
            objects_being_reconstructed: std::sync::Mutex::new(std::collections::HashSet::new()),
            max_reconstruction_depth: 100,
            allow_object_reconstruction: true,
        })
    }

    /// Get the PDF version
    pub fn version(&self) -> &super::header::PdfVersion {
        &self.header.version
    }

    /// Get the document catalog
    pub fn catalog(&mut self) -> ParseResult<&PdfDictionary> {
        // Try to get root from trailer
        let (obj_num, gen_num) = match self.trailer.root() {
            Ok(root) => {
                // FIX for Issue #83: Validate that Root actually points to a Catalog
                // In signed PDFs, Root might point to /Type/Sig instead of /Type/Catalog
                if let Ok(obj) = self.get_object(root.0, root.1) {
                    if let Some(dict) = obj.as_dict() {
                        // Check if it's really a catalog
                        if let Some(type_obj) = dict.get("Type") {
                            if let Some(type_name) = type_obj.as_name() {
                                if type_name.0 != "Catalog" {
                                    tracing::warn!("Trailer /Root points to /Type/{} (not Catalog), scanning for real catalog", type_name.0);
                                    // Root points to wrong object type, scan for real catalog
                                    if let Ok(catalog_ref) = self.find_catalog_object() {
                                        catalog_ref
                                    } else {
                                        root // Fallback to original if scan fails
                                    }
                                } else {
                                    root // It's a valid catalog
                                }
                            } else {
                                root // No type field, assume it's catalog
                            }
                        } else {
                            root // No Type key, assume it's catalog
                        }
                    } else {
                        root // Not a dict, will fail later but keep trying
                    }
                } else {
                    root // Can't get object, will fail later
                }
            }
            Err(_) => {
                // If Root is missing, try fallback methods
                #[cfg(debug_assertions)]
                tracing::warn!("Trailer missing Root entry, attempting recovery");

                // First try the fallback method
                if let Some(root) = self.trailer.find_root_fallback() {
                    root
                } else {
                    // Last resort: scan for Catalog object
                    if let Ok(catalog_ref) = self.find_catalog_object() {
                        catalog_ref
                    } else {
                        return Err(ParseError::MissingKey("Root".to_string()));
                    }
                }
            }
        };

        // Check if we need to attempt reconstruction by examining the object type first
        let needs_reconstruction = {
            match self.get_object(obj_num, gen_num) {
                Ok(catalog) => {
                    // Check if it's already a valid dictionary
                    if catalog.as_dict().is_some() {
                        // It's a valid dictionary, no reconstruction needed
                        false
                    } else {
                        // Not a dictionary, needs reconstruction
                        true
                    }
                }
                Err(_) => {
                    // Failed to get object, needs reconstruction
                    true
                }
            }
        };

        if !needs_reconstruction {
            // Object is valid, get it again to return the reference
            let catalog = self.get_object(obj_num, gen_num)?;
            return catalog.as_dict().ok_or_else(|| ParseError::SyntaxError {
                position: 0,
                message: format!("Catalog object {} {} is not a dictionary", obj_num, gen_num),
            });
        }

        // Return error if all reconstruction attempts failed
        Err(ParseError::SyntaxError {
            position: 0,
            message: format!(
                "Catalog object {} could not be parsed or reconstructed as a dictionary",
                obj_num
            ),
        })
    }

    /// Get the document info dictionary
    pub fn info(&mut self) -> ParseResult<Option<&PdfDictionary>> {
        match self.trailer.info() {
            Some((obj_num, gen_num)) => {
                let info = self.get_object(obj_num, gen_num)?;
                Ok(info.as_dict())
            }
            None => Ok(None),
        }
    }

    /// Get an object by reference with circular reference protection
    pub fn get_object(&mut self, obj_num: u32, gen_num: u16) -> ParseResult<&PdfObject> {
        // Check if PDF is locked (encrypted but not unlocked)
        self.ensure_unlocked()?;

        let key = (obj_num, gen_num);

        // Fast path: check cache first
        if self.object_cache.contains_key(&key) {
            return Ok(&self.object_cache[&key]);
        }

        // PROTECTION 1: Check for circular reference
        {
            let being_loaded =
                self.objects_being_reconstructed
                    .lock()
                    .map_err(|_| ParseError::SyntaxError {
                        position: 0,
                        message: "Mutex poisoned during circular reference check".to_string(),
                    })?;
            if being_loaded.contains(&obj_num) {
                drop(being_loaded);
                if self.options.collect_warnings {}
                self.object_cache.insert(key, PdfObject::Null);
                return Ok(&self.object_cache[&key]);
            }
        }

        // PROTECTION 2: Check depth limit
        {
            let being_loaded =
                self.objects_being_reconstructed
                    .lock()
                    .map_err(|_| ParseError::SyntaxError {
                        position: 0,
                        message: "Mutex poisoned during depth limit check".to_string(),
                    })?;
            let depth = being_loaded.len() as u32;
            if depth >= self.max_reconstruction_depth {
                drop(being_loaded);
                if self.options.collect_warnings {}
                return Err(ParseError::SyntaxError {
                    position: 0,
                    message: format!(
                        "Maximum object loading depth ({}) exceeded",
                        self.max_reconstruction_depth
                    ),
                });
            }
        }

        // Mark object as being loaded
        self.objects_being_reconstructed
            .lock()
            .map_err(|_| ParseError::SyntaxError {
                position: 0,
                message: "Mutex poisoned while marking object as being loaded".to_string(),
            })?
            .insert(obj_num);

        // Load object - if successful, it will be in cache
        match self.load_object_from_disk(obj_num, gen_num) {
            Ok(_) => {
                // Object successfully loaded, now unmark and return from cache
                self.objects_being_reconstructed
                    .lock()
                    .map_err(|_| ParseError::SyntaxError {
                        position: 0,
                        message: "Mutex poisoned while unmarking object after successful load"
                            .to_string(),
                    })?
                    .remove(&obj_num);
                // Object must be in cache now
                Ok(&self.object_cache[&key])
            }
            Err(e) => {
                // Loading failed, unmark and propagate error
                // Note: If mutex is poisoned here, we prioritize the original error
                if let Ok(mut guard) = self.objects_being_reconstructed.lock() {
                    guard.remove(&obj_num);
                }
                Err(e)
            }
        }
    }

    /// Internal method to load an object from disk without stack management
    fn load_object_from_disk(&mut self, obj_num: u32, gen_num: u16) -> ParseResult<&PdfObject> {
        let key = (obj_num, gen_num);

        // Check cache first
        if self.object_cache.contains_key(&key) {
            return Ok(&self.object_cache[&key]);
        }

        // Check if this is a compressed object
        if let Some(ext_entry) = self.xref.get_extended_entry(obj_num) {
            if let Some((stream_obj_num, index_in_stream)) = ext_entry.compressed_info {
                // This is a compressed object - need to extract from object stream
                return self.get_compressed_object(
                    obj_num,
                    gen_num,
                    stream_obj_num,
                    index_in_stream,
                );
            }
        } else {
        }

        // Get xref entry and extract needed values
        let (current_offset, _generation) = {
            let entry = self.xref.get_entry(obj_num);

            match entry {
                Some(entry) => {
                    if !entry.in_use {
                        // Free object
                        self.object_cache.insert(key, PdfObject::Null);
                        return Ok(&self.object_cache[&key]);
                    }

                    if entry.generation != gen_num {
                        if self.options.lenient_syntax {
                            // In lenient mode, warn but use the available generation
                            if self.options.collect_warnings {
                                tracing::warn!("Object {} generation mismatch - expected {}, found {}, using available",
                                    obj_num, gen_num, entry.generation);
                            }
                        } else {
                            return Err(ParseError::InvalidReference(obj_num, gen_num));
                        }
                    }

                    (entry.offset, entry.generation)
                }
                None => {
                    if self.is_reconstructible_object(obj_num) {
                        match self.attempt_manual_object_reconstruction(obj_num, gen_num, 0) {
                            Ok(_) => return Ok(&self.object_cache[&key]),
                            // No matching object exists in the bytes. Retain the
                            // lenient absent-reference policy, but never hide a
                            // parse failure for an object that actually exists.
                            Err(ParseError::InvalidReference(_, _)) => {}
                            Err(error) => return Err(error),
                        }
                    }
                    if self.options.lenient_syntax {
                        if self.options.collect_warnings {
                            tracing::warn!(
                                "Object {} {} R not found, returning null object",
                                obj_num,
                                gen_num
                            );
                        }
                        self.object_cache.insert(key, PdfObject::Null);
                        return Ok(&self.object_cache[&key]);
                    }
                    return Err(ParseError::InvalidReference(obj_num, gen_num));
                }
            }
        };

        if current_offset == 0 {
            return Err(ParseError::SyntaxError {
                position: 0,
                message: format!("in-use reference {obj_num} {gen_num} R has zero storage offset; object is absent"),
            });
        }

        // Try normal parsing first - only use manual reconstruction as fallback

        // Seek to the (potentially corrected) object position
        self.reader.seek(std::io::SeekFrom::Start(current_offset))?;

        // Parse object header (obj_num gen_num obj) - but skip if we already positioned after it
        let mut lexer =
            super::lexer::Lexer::new_with_options(&mut self.reader, self.options.clone());

        // Parse object header normally for all objects
        {
            // Read object number with recovery
            let token = lexer.next_token()?;
            let read_obj_num = match token {
                super::lexer::Token::Integer(n) => n as u32,
                _ => {
                    // Try fallback recovery (simplified implementation)
                    if self.options.lenient_syntax {
                        // For now, use the expected object number and issue warning
                        if self.options.collect_warnings {
                            tracing::debug!(
                                "Warning: Using expected object number {obj_num} instead of parsed token: {:?}",
                                token
                            );
                        }
                        obj_num
                    } else {
                        return Err(ParseError::SyntaxError {
                            position: current_offset as usize,
                            message: "Expected object number".to_string(),
                        });
                    }
                }
            };

            if read_obj_num != obj_num && !self.options.lenient_syntax {
                return Err(ParseError::SyntaxError {
                    position: current_offset as usize,
                    message: format!(
                        "Object number mismatch: expected {obj_num}, found {read_obj_num}"
                    ),
                });
            }

            // Read generation number with recovery
            let token = lexer.next_token()?;
            let _read_gen_num = match token {
                super::lexer::Token::Integer(n) => n as u16,
                _ => {
                    // Try fallback recovery
                    if self.options.lenient_syntax {
                        if self.options.collect_warnings {
                            tracing::warn!(
                                "Using generation 0 instead of parsed token for object {obj_num}"
                            );
                        }
                        0
                    } else {
                        return Err(ParseError::SyntaxError {
                            position: current_offset as usize,
                            message: "Expected generation number".to_string(),
                        });
                    }
                }
            };

            // Read 'obj' keyword
            let token = lexer.next_token()?;
            match token {
                super::lexer::Token::Obj => {}
                _ => {
                    if self.options.lenient_syntax {
                        // In lenient mode, warn but continue
                        if self.options.collect_warnings {
                            tracing::warn!("Expected 'obj' keyword for object {obj_num} {gen_num}, continuing anyway");
                        }
                    } else {
                        return Err(ParseError::SyntaxError {
                            position: current_offset as usize,
                            message: "Expected 'obj' keyword".to_string(),
                        });
                    }
                }
            }
        }

        // Check recursion depth and parse object
        self.parse_context.enter()?;

        let obj = match PdfObject::parse_with_options(&mut lexer, &self.options) {
            Ok(obj) => {
                self.parse_context.exit();
                // Debug: Print what object we actually parsed
                obj
            }
            Err(e) => {
                self.parse_context.exit();

                // Attempt manual reconstruction as fallback for known problematic objects
                if self.is_reconstructible_object(obj_num)
                    && self.can_attempt_manual_reconstruction(&e)
                {
                    match self.attempt_manual_object_reconstruction(
                        obj_num,
                        gen_num,
                        current_offset,
                    ) {
                        Ok(reconstructed_obj) => {
                            return Ok(reconstructed_obj);
                        }
                        Err(_reconstruction_error) => {}
                    }
                }

                return Err(e);
            }
        };

        // Read 'endobj' keyword
        let token = lexer.next_token()?;
        match token {
            super::lexer::Token::EndObj => {}
            _ => {
                if self.options.lenient_syntax {
                    // In lenient mode, warn but continue
                    if self.options.collect_warnings {
                        tracing::warn!("Expected 'endobj' keyword after object {obj_num} {gen_num}, continuing anyway");
                    }
                } else {
                    return Err(ParseError::SyntaxError {
                        position: current_offset as usize,
                        message: "Expected 'endobj' keyword".to_string(),
                    });
                }
            }
        };

        // Decrypt if encryption is active
        let decrypted_obj = self.decrypt_object_if_needed(obj, obj_num, gen_num)?;

        // Cache the decrypted object
        self.object_cache.insert(key, decrypted_obj);

        Ok(&self.object_cache[&key])
    }

    /// Resolve a reference to get the actual object
    pub fn resolve<'a>(&'a mut self, obj: &'a PdfObject) -> ParseResult<&'a PdfObject> {
        match obj {
            PdfObject::Reference(obj_num, gen_num) => self.get_object(*obj_num, *gen_num),
            _ => Ok(obj),
        }
    }

    /// Resolve a stream length reference to get the actual length value
    /// This is a specialized method for handling indirect references in stream Length fields
    pub fn resolve_stream_length(&mut self, obj: &PdfObject) -> ParseResult<Option<usize>> {
        match obj {
            PdfObject::Integer(len) => {
                if *len >= 0 {
                    Ok(Some(*len as usize))
                } else {
                    // Negative lengths are invalid, treat as missing
                    Ok(None)
                }
            }
            PdfObject::Reference(obj_num, gen_num) => {
                let resolved = self.get_object(*obj_num, *gen_num)?;
                match resolved {
                    PdfObject::Integer(len) => {
                        if *len >= 0 {
                            Ok(Some(*len as usize))
                        } else {
                            Ok(None)
                        }
                    }
                    _ => {
                        // Reference doesn't point to a valid integer
                        Ok(None)
                    }
                }
            }
            _ => {
                // Not a valid length type
                Ok(None)
            }
        }
    }

    /// Get a compressed object from an object stream
    fn get_compressed_object(
        &mut self,
        obj_num: u32,
        gen_num: u16,
        stream_obj_num: u32,
        _index_in_stream: u32,
    ) -> ParseResult<&PdfObject> {
        let key = (obj_num, gen_num);

        self.ensure_object_stream(stream_obj_num)?;
        let obj = self.object_stream_cache[&stream_obj_num].object(obj_num, &self.options)?;

        // The containing ObjStm was decrypted by ensure_object_stream. Its
        // embedded objects are plaintext and must not be decrypted again.
        self.object_cache.insert(key, obj);
        Ok(&self.object_cache[&key])
    }

    /// Retain decoded bytes, never all parsed objects. Bound aggregate retained
    /// bytes to 128 MiB, except that one larger stream can be held alone. The
    /// existing decoder limits still govern a single stream's allocation.
    fn ensure_object_stream(&mut self, stream_obj_num: u32) -> ParseResult<()> {
        const CACHE_BYTES: usize = 128 * 1024 * 1024;
        if self.object_stream_cache.contains_key(&stream_obj_num) {
            return Ok(());
        }
        let options = self.options.clone();
        let stream = self
            .get_object(stream_obj_num, 0)?
            .as_stream()
            .ok_or_else(|| ParseError::SyntaxError {
                position: 0,
                message: format!("Object {stream_obj_num} is not a stream"),
            })?;
        let indexed = IndexedObjectStream::parse(stream, &options)?;
        let retained: usize = self
            .object_stream_cache
            .values()
            .map(IndexedObjectStream::decoded_len)
            .sum();
        if retained.saturating_add(indexed.decoded_len()) > CACHE_BYTES {
            self.object_stream_cache.clear();
        }
        self.object_stream_cache.insert(stream_obj_num, indexed);
        Ok(())
    }

    /// Structure-owner arrays are temporary positional metadata. Do not retain
    /// their dense representation in the reader or document resource caches.
    pub(crate) fn sparse_array(
        &mut self,
        obj_num: u32,
        gen_num: u16,
    ) -> ParseResult<HashMap<usize, PdfObject>> {
        self.ensure_unlocked()?;
        let compressed = self
            .xref
            .get_extended_entry(obj_num)
            .and_then(|entry| entry.compressed_info);
        if let Some((stream, _)) = compressed {
            self.ensure_object_stream(stream)?;
            // As with get_compressed_object, the containing stream has already
            // been decrypted; sparse values retain that plaintext representation.
            return self.object_stream_cache[&stream].sparse_array(obj_num, &self.options);
        }
        // Ordinary objects already use the reader's validated offset, generation
        // and encryption handling. Take the temporary array out of its cache.
        self.get_object(obj_num, gen_num)?;
        match self.object_cache.remove(&(obj_num, gen_num)) {
            Some(PdfObject::Array(array)) => Ok(array
                .0
                .into_iter()
                .enumerate()
                .filter(|(_, value)| !matches!(value, PdfObject::Null))
                .collect()),
            _ => Err(ParseError::SyntaxError {
                position: 0,
                message: "Structure owners must be an array".to_string(),
            }),
        }
    }

    /// Get the page tree root
    pub fn pages(&mut self) -> ParseResult<&PdfDictionary> {
        // Get the pages reference from catalog first
        let (pages_obj_num, pages_gen_num) = {
            let catalog = self.catalog()?;

            // First try to get Pages reference
            if let Some(pages_ref) = catalog.get("Pages") {
                match pages_ref {
                    PdfObject::Reference(obj_num, gen_num) => (*obj_num, *gen_num),
                    _ => {
                        return Err(ParseError::SyntaxError {
                            position: 0,
                            message: "Pages must be a reference".to_string(),
                        })
                    }
                }
            } else {
                if !self.options.lenient_syntax || !self.allow_object_reconstruction {
                    return Err(ParseError::MissingKey("Pages".into()));
                }
                // If Pages is missing, try to find page objects by scanning
                if self.options.collect_warnings {
                    tracing::warn!(
                        "Catalog missing Pages entry, attempting recovery from page objects"
                    );
                }

                // Look for objects that have Type = Page
                if let Ok(page_refs) = self.find_page_objects() {
                    if !page_refs.is_empty() {
                        // Create a synthetic Pages dictionary
                        return self.create_synthetic_pages_dict(&page_refs);
                    }
                }

                // If Pages is missing and we have lenient parsing, try to find it
                if self.options.lenient_syntax {
                    if self.options.collect_warnings {
                        tracing::warn!("Missing Pages in catalog, searching for page tree");
                    }
                    // Search for a Pages object in the document
                    let mut found_pages = None;
                    for i in 1..self.xref.len() as u32 {
                        if let Ok(obj) = self.get_object(i, 0) {
                            if let Some(dict) = obj.as_dict() {
                                if let Some(obj_type) = dict.get("Type").and_then(|t| t.as_name()) {
                                    if obj_type.0 == "Pages" {
                                        found_pages = Some((i, 0));
                                        break;
                                    }
                                }
                            }
                        }
                    }
                    if let Some((obj_num, gen_num)) = found_pages {
                        (obj_num, gen_num)
                    } else {
                        return Err(ParseError::MissingKey("Pages".to_string()));
                    }
                } else {
                    return Err(ParseError::MissingKey("Pages".to_string()));
                }
            }
        };

        // Now we can get the pages object without holding a reference to catalog
        // First, check if we need double indirection by peeking at the object
        let needs_double_resolve = {
            let pages_obj = self.get_object(pages_obj_num, pages_gen_num)?;
            pages_obj.as_reference()
        };

        // If it's a reference, resolve the double indirection
        let (final_obj_num, final_gen_num) =
            if let Some((ref_obj_num, ref_gen_num)) = needs_double_resolve {
                (ref_obj_num, ref_gen_num)
            } else {
                (pages_obj_num, pages_gen_num)
            };

        // Preserve the complete Pages reference through validation and recovery.
        let actual_pages_ref = {
            // Check if the referenced object is valid (in a scope to drop borrows)
            let is_valid_dict = {
                let pages_obj = self.get_object(final_obj_num, final_gen_num)?;
                pages_obj.as_dict().is_some()
            };

            if is_valid_dict {
                // The referenced object is valid
                (final_obj_num, final_gen_num)
            } else {
                // If Pages reference resolves to Null or non-dictionary, try to find Pages manually (corrupted PDF)
                #[cfg(debug_assertions)]
                tracing::warn!("Pages reference invalid, searching for valid Pages object");

                if self.options.lenient_syntax {
                    // Search for a valid Pages object number
                    let xref_len = self.xref.len() as u32;
                    let mut found_pages_num = None;

                    for i in 1..xref_len {
                        // Check in a scope to drop the borrow
                        let is_pages = {
                            if let Ok(obj) = self.get_object(i, 0) {
                                if let Some(dict) = obj.as_dict() {
                                    if let Some(obj_type) =
                                        dict.get("Type").and_then(|t| t.as_name())
                                    {
                                        obj_type.0 == "Pages"
                                    } else {
                                        false
                                    }
                                } else {
                                    false
                                }
                            } else {
                                false
                            }
                        };

                        if is_pages {
                            found_pages_num = Some(i);
                            break;
                        }
                    }

                    if let Some(obj_num) = found_pages_num {
                        #[cfg(debug_assertions)]
                        tracing::debug!("Found valid Pages object at {} 0 R", obj_num);
                        (obj_num, 0)
                    } else {
                        // No valid Pages found
                        return Err(ParseError::SyntaxError {
                            position: 0,
                            message: "Pages is not a dictionary and no valid Pages object found"
                                .to_string(),
                        });
                    }
                } else {
                    // Lenient mode disabled, can't search
                    return Err(ParseError::SyntaxError {
                        position: 0,
                        message: "Pages is not a dictionary".to_string(),
                    });
                }
            }
        };

        // Now get the final Pages object (all validation/search done above)
        let pages_obj = self.get_object(actual_pages_ref.0, actual_pages_ref.1)?;
        pages_obj.as_dict().ok_or_else(|| ParseError::SyntaxError {
            position: 0,
            message: "Pages object is not a dictionary".to_string(),
        })
    }

    /// Get the number of pages
    pub fn page_count(&mut self) -> ParseResult<u32> {
        /// Maximum page count accepted from the /Count entry.
        /// PDFs claiming more pages than this are likely malformed or malicious.
        const MAX_PAGE_COUNT: u32 = 100_000;

        // Try standard method first
        match self.pages() {
            Ok(pages) => {
                // Read /Count and /Kids up front. Each may be inline or an
                // indirect reference (ISO 32000-1 §7.3.10); extract Copy values
                // now so the `pages` borrow ends before we resolve references.
                let count_inline = pages.get("Count").and_then(|o| o.as_integer());
                let count_ref = pages.get("Count").and_then(|o| o.as_reference());
                let kids_len_inline = pages
                    .get("Kids")
                    .and_then(|o| o.as_array())
                    .map(|a| a.0.len());
                let kids_ref = pages.get("Kids").and_then(|o| o.as_reference());

                // Resolve /Count to an integer, whether inline or indirect.
                let count = count_inline.or_else(|| {
                    count_ref
                        .and_then(|(n, g)| self.get_object(n, g).ok().and_then(|o| o.as_integer()))
                });
                if let Some(count) = count {
                    let count = count as u32;
                    if count <= MAX_PAGE_COUNT {
                        return Ok(count);
                    }
                    tracing::warn!(
                        "PDF /Count {} exceeds limit {}, falling back to Kids array length",
                        count,
                        MAX_PAGE_COUNT
                    );
                    // Fall through to Kids counting
                }

                // If Count is missing, invalid, or exceeds limit, count the
                // Kids array — inline or resolved from an indirect reference.
                if let Some(len) = kids_len_inline {
                    return Ok(len as u32);
                }
                if let Some((n, g)) = kids_ref {
                    if let Some(len) = self
                        .get_object(n, g)
                        .ok()
                        .and_then(|o| o.as_array())
                        .map(|a| a.0.len())
                    {
                        return Ok(len as u32);
                    }
                }

                Ok(0)
            }
            Err(error) => {
                if !self.options.lenient_syntax || !self.allow_object_reconstruction {
                    return Err(error);
                }
                // If standard method fails, try fallback extraction
                tracing::debug!("Standard page extraction failed, trying direct extraction");
                self.page_count_fallback()
            }
        }
    }

    /// Fallback method to extract page count directly from content for corrupted PDFs
    fn page_count_fallback(&mut self) -> ParseResult<u32> {
        // Fallback: count individual page objects
        if let Some(count) = self.count_page_objects_directly() {
            if self.options.collect_warnings {
                tracing::warn!(count, "Recovered page count by scanning page objects");
            }
            return Ok(count);
        }

        Ok(0)
    }

    #[allow(dead_code)]
    fn find_object_pattern(&mut self, obj_num: u32, gen_num: u16) -> Option<u64> {
        let pattern = format!("{} {} obj", obj_num, gen_num);

        // Save current position
        let original_pos = self.reader.stream_position().unwrap_or(0);

        // Search from the beginning of the file
        if self.reader.seek(SeekFrom::Start(0)).is_err() {
            return None;
        }

        // Read the entire file in chunks to search for the pattern
        let mut buffer = vec![0u8; 8192];
        let mut file_content = Vec::new();

        loop {
            match self.reader.read(&mut buffer) {
                Ok(0) => break, // EOF
                Ok(bytes_read) => {
                    file_content.extend_from_slice(&buffer[..bytes_read]);
                }
                Err(_) => return None,
            }
        }

        // Convert to string and search
        let content = String::from_utf8_lossy(&file_content);
        if let Some(pattern_pos) = content.find(&pattern) {
            // Now search for the << after the pattern
            let after_pattern = pattern_pos + pattern.len();
            let search_area = &content[after_pattern..];

            if let Some(dict_start_offset) = search_area.find("<<") {
                let dict_start_pos = after_pattern + dict_start_offset;

                // Restore original position
                self.reader.seek(SeekFrom::Start(original_pos)).ok();
                return Some(dict_start_pos as u64);
            } else {
            }
        }

        // Restore original position
        self.reader.seek(SeekFrom::Start(original_pos)).ok();
        None
    }

    /// Determine if we should attempt manual reconstruction for this error
    fn can_attempt_manual_reconstruction(&self, error: &ParseError) -> bool {
        match error {
            // These are the types of errors that might be fixable with manual reconstruction
            ParseError::SyntaxError { .. } => true,
            ParseError::UnexpectedToken { .. } => true,
            // Don't attempt reconstruction for other error types
            _ => false,
        }
    }

    /// Recovery is a lenient policy, independent of the object's identity.
    fn is_reconstructible_object(&self, _obj_num: u32) -> bool {
        self.allow_object_reconstruction
            && (self.options.lenient_syntax || self.options.lenient_streams)
    }

    /// The outer get_object owns recursion tracking. Recovery only reads a bounded
    /// window and never substitutes guessed objects or modifies the xref table.
    fn attempt_manual_object_reconstruction(
        &mut self,
        obj_num: u32,
        gen_num: u16,
        _current_offset: u64,
    ) -> ParseResult<&PdfObject> {
        let object = self.extract_object_at_generation(obj_num, gen_num)?;
        let object = self.decrypt_object_if_needed(object, obj_num, gen_num)?;
        if self.options.collect_warnings {
            tracing::warn!(obj_num, gen_num, "Recovered object from scanned PDF bytes");
        }
        self.object_cache.insert((obj_num, gen_num), object);
        Ok(&self.object_cache[&(obj_num, gen_num)])
    }

    #[cfg(test)]
    fn extract_object_or_stream_manually(&mut self, obj_num: u32) -> ParseResult<PdfObject> {
        self.extract_object_at_generation(obj_num, 0)
    }

    fn extract_object_at_generation(
        &mut self,
        obj_num: u32,
        gen_num: u16,
    ) -> ParseResult<PdfObject> {
        use super::lexer::{Lexer, Token};
        let original_pos = self.reader.stream_position()?;
        let result = (|| {
            let (offset, window) =
                read_object_window(&mut self.reader, obj_num, MANUAL_DICT_WINDOW)?
                    .ok_or(ParseError::InvalidReference(obj_num, gen_num))?;
            let options = self.options.clone();
            let mut lexer = Lexer::new_with_options(std::io::Cursor::new(&window), options.clone());
            if lexer.next_token()? != Token::Integer(i64::from(obj_num))
                || lexer.next_token()? != Token::Integer(i64::from(gen_num))
            {
                return Err(ParseError::InvalidReference(obj_num, gen_num));
            }
            lexer.expect_keyword("obj")?;
            if lexer.next_token()? != Token::DictStart {
                return Err(ParseError::SyntaxError {
                    position: offset as usize,
                    message: "Recovery requires an object dictionary".into(),
                });
            }
            // Use PDF tokens, not bracket/string searches: literal strings and
            // nested dictionaries must not truncate or move the object boundary.
            let dict = PdfObject::parse_dictionary_inner_with_options(&mut lexer, &options)?;
            let mut next = lexer.next_token()?;
            while matches!(next, Token::Comment(_)) {
                next = lexer.next_token()?;
            }
            match next {
                Token::Stream if self.options.lenient_streams => {
                    lexer.read_newline()?;
                    self.reconstruct_stream_object_bounded(dict, offset + lexer.position() as u64)
                }
                Token::EndObj if self.options.lenient_syntax => Ok(PdfObject::Dictionary(dict)),
                token => Err(ParseError::UnexpectedToken {
                    expected: "stream or endobj".into(),
                    found: format!("{token:?}"),
                }),
            }
        })();
        self.reader.seek(SeekFrom::Start(original_pos))?;
        result
    }

    /// Reconstruct a stream object using bounded reads (Issue #339).
    ///
    /// The dictionary was already parsed from a bounded window; the stream body is
    /// read directly at its absolute file offset, bounded by `/Length` (or, if
    /// `/Length` is indirect, by resolving that length object; or, if absent, by a
    /// bounded scan to `endstream`). `data_abs` follows the parsed stream newline.
    fn reconstruct_stream_object_bounded(
        &mut self,
        dictionary: PdfDictionary,
        data_abs: u64,
    ) -> ParseResult<PdfObject> {
        use crate::parser::objects::{PdfName, PdfStream};
        // Only dictionaries successfully parsed from actual bytes reach here.
        let length = match dictionary.get("Length") {
            Some(PdfObject::Integer(n)) => usize::try_from(*n).ok(),
            Some(PdfObject::Reference(n, g)) => self.resolve_length_object(*n, *g)?,
            _ => None,
        };
        let mut dict = dictionary.0;

        // Read the stream body, bounded by its real size rather than the whole file.
        let data = match length {
            Some(len) => {
                // Validate against storage before allocating from an untrusted /Length.
                let file_end = self.reader.seek(SeekFrom::End(0))?;
                if len as u64 > file_end.saturating_sub(data_abs) {
                    return Err(ParseError::SyntaxError {
                        position: data_abs as usize,
                        message: "Recovered stream length exceeds available file bytes".into(),
                    });
                }
                let tail = read_window_at(&mut self.reader, data_abs + len as u64, 256)?;
                let mut end = super::lexer::Lexer::new(std::io::Cursor::new(tail));
                if end.next_token()? != super::lexer::Token::EndStream {
                    return Err(ParseError::SyntaxError {
                        position: data_abs as usize,
                        message: "Recovered stream length does not end at endstream".into(),
                    });
                }
                // Exact length preserves binary payloads containing endstream.
                let body = read_window_at(&mut self.reader, data_abs, len)?;
                dict.insert(
                    PdfName("Length".to_string()),
                    PdfObject::Integer(len as i64),
                );
                body
            }
            None => {
                // Unknown length: scan forward in bounded windows to `endstream`,
                // retaining only the stream bytes (O(stream size), not O(file)).
                let body = self.read_stream_until_endstream(data_abs)?;
                // Pin /Length to the bytes actually read. The generic dict parse may
                // have carried an unresolvable indirect `/Length N G R`; replacing it
                // keeps the dictionary consistent with the body and avoids a dangling
                // reference downstream.
                dict.insert(
                    PdfName("Length".to_string()),
                    PdfObject::Integer(body.len() as i64),
                );
                body
            }
        };

        Ok(PdfObject::Stream(PdfStream {
            dict: PdfDictionary(dict),
            data,
        }))
    }

    /// Resolve an indirect `/Length` object value via a small bounded window. The
    /// length object is a bare integer (`N G obj <int> endobj`), so a tiny read at
    /// its offset suffices — no recursion through `get_object`, avoiding the false
    /// circular dependency this manual path exists to break.
    fn resolve_length_object(
        &mut self,
        len_obj: u32,
        generation: u16,
    ) -> ParseResult<Option<usize>> {
        let Some((_, window)) = read_object_window(&mut self.reader, len_obj, 4096)? else {
            return Ok(None);
        };
        use super::lexer::{Lexer, Token};
        let mut lexer = Lexer::new(std::io::Cursor::new(window));
        if lexer.next_token()? != Token::Integer(i64::from(len_obj))
            || lexer.next_token()? != Token::Integer(i64::from(generation))
        {
            return Ok(None);
        }
        lexer.expect_keyword("obj")?;
        if let Token::Integer(n) = lexer.next_token()? {
            if lexer.next_token()? == Token::EndObj {
                return Ok(usize::try_from(n).ok());
            }
        }
        Ok(None)
    }

    /// Read a stream body of unknown length by scanning forward in bounded windows
    /// from `data_abs` until `endstream`, accumulating only the stream bytes. Peak
    /// memory is O(window + stream size), never O(file size).
    fn read_stream_until_endstream(&mut self, data_abs: u64) -> ParseResult<Vec<u8>> {
        const WIN: usize = 64 * 1024;
        const MAX_STREAM: usize = 64 * 1024 * 1024; // runaway guard

        let mut acc: Vec<u8> = Vec::new();
        let mut offset = data_abs;
        let mut searched = 0usize;
        loop {
            let chunk = read_window_at(&mut self.reader, offset, WIN)?;
            if chunk.is_empty() {
                return Err(ParseError::SyntaxError {
                    position: data_abs as usize,
                    message: "Recovered stream has no endstream marker".into(),
                });
            }
            acc.extend_from_slice(&chunk);

            // Search the freshly added region with an 8-byte overlap so an
            // "endstream" straddling a window boundary is not missed.
            let from = searched.saturating_sub(b"endstream".len() - 1);
            if let Some(rel) = find_byte_pattern(&acc[from..], b"endstream") {
                acc.truncate(from + rel);
                break;
            }
            searched = acc.len();
            offset += chunk.len() as u64;

            if chunk.len() < WIN {
                return Err(ParseError::SyntaxError {
                    position: data_abs as usize,
                    message: "Recovered stream has no endstream marker".into(),
                });
            }
            if acc.len() > MAX_STREAM {
                return Err(ParseError::SyntaxError {
                    position: data_abs as usize,
                    message: "Recovered stream exceeds 64 MiB without endstream".into(),
                });
            }
        }

        // Trim a single trailing EOL before "endstream" (not part of the data).
        if acc.last() == Some(&b'\n') {
            acc.pop();
            if acc.last() == Some(&b'\r') {
                acc.pop();
            }
        } else if acc.last() == Some(&b'\r') {
            acc.pop();
        }
        Ok(acc)
    }

    #[allow(dead_code)]
    fn extract_catalog_directly(
        &mut self,
        obj_num: u32,
        gen_num: u16,
    ) -> ParseResult<&PdfDictionary> {
        // Find the catalog object in the XRef table
        if let Some(entry) = self.xref.get_entry(obj_num) {
            // Seek to the object's position
            if self.reader.seek(SeekFrom::Start(entry.offset)).is_err() {
                return Err(ParseError::SyntaxError {
                    position: 0,
                    message: "Failed to seek to catalog object".to_string(),
                });
            }

            // Read content around the object
            let mut buffer = vec![0u8; 2048];
            if let Ok(bytes_read) = self.reader.read(&mut buffer) {
                let content = String::from_utf8_lossy(&buffer[..bytes_read]);
                tracing::debug!("Raw catalog content:\n{}", content);

                // Look for the dictionary pattern << ... >>
                if let Some(dict_start) = content.find("<<") {
                    if let Some(dict_end) = content[dict_start..].find(">>") {
                        let dict_content = &content[dict_start..dict_start + dict_end + 2];
                        tracing::debug!("Found dictionary content: {}", dict_content);

                        // Try to parse this directly as a dictionary
                        if let Ok(dict) = self.parse_dictionary_from_string(dict_content) {
                            // Cache the parsed dictionary
                            let key = (obj_num, gen_num);
                            self.object_cache.insert(key, PdfObject::Dictionary(dict));

                            // Return reference to cached object
                            if let Some(PdfObject::Dictionary(ref dict)) =
                                self.object_cache.get(&key)
                            {
                                return Ok(dict);
                            }
                        }
                    }
                }
            }
        }

        Err(ParseError::SyntaxError {
            position: 0,
            message: "Failed to extract catalog directly".to_string(),
        })
    }

    #[allow(dead_code)]
    fn parse_dictionary_from_string(&self, dict_str: &str) -> ParseResult<PdfDictionary> {
        use crate::parser::lexer::{Lexer, Token};

        // Create a lexer from the dictionary string
        let mut cursor = std::io::Cursor::new(dict_str.as_bytes());
        let mut lexer = Lexer::new_with_options(&mut cursor, self.options.clone());

        // Parse the dictionary
        match lexer.next_token()? {
            Token::DictStart => {
                let mut dict = std::collections::HashMap::new();

                loop {
                    let token = lexer.next_token()?;
                    match token {
                        Token::DictEnd => break,
                        Token::Name(key) => {
                            // Parse the value
                            let value = PdfObject::parse_with_options(&mut lexer, &self.options)?;
                            dict.insert(crate::parser::objects::PdfName(key), value);
                        }
                        _ => {
                            return Err(ParseError::SyntaxError {
                                position: 0,
                                message: "Invalid dictionary format".to_string(),
                            });
                        }
                    }
                }

                Ok(PdfDictionary(dict))
            }
            _ => Err(ParseError::SyntaxError {
                position: 0,
                message: "Expected dictionary start".to_string(),
            }),
        }
    }

    /// Count page objects directly by scanning for "/Type /Page"
    fn count_page_objects_directly(&mut self) -> Option<u32> {
        let mut page_count = 0;

        // Iterate through all objects and count those with Type = Page
        for obj_num in 1..self.xref.len() as u32 {
            if let Ok(obj) = self.get_object(obj_num, 0) {
                if let Some(dict) = obj.as_dict() {
                    if let Some(obj_type) = dict.get("Type").and_then(|t| t.as_name()) {
                        if obj_type.0 == "Page" {
                            page_count += 1;
                        }
                    }
                }
            }
        }

        if page_count > 0 {
            Some(page_count)
        } else {
            None
        }
    }

    /// Effective PDF version: the greater of the header and catalog /Version.
    ///
    /// Catalog versions can raise, but cannot lower, the header version.
    /// An invalid catalog version name is ignored. Resolving the catalog can
    /// fail, so unlike [`Self::version`] this method returns a result.
    pub fn effective_version(&mut self) -> ParseResult<super::header::PdfVersion> {
        let mut version = self.version().clone();
        let mut value = self.catalog()?.get("Version").cloned();
        let mut visited = Vec::new();
        while let Some(PdfObject::Reference(id, generation)) = value {
            if visited.contains(&(id, generation)) {
                return Err(ParseError::CircularReference);
            }
            if visited.len() == 64 {
                return Err(ParseError::SyntaxError {
                    position: 0,
                    message: "catalog Version reference chain exceeds 64 objects".into(),
                });
            }
            visited.push((id, generation));
            value = Some(self.get_object(id, generation)?.clone());
        }
        if let Some(name) = value.as_ref().and_then(|v| v.as_name()) {
            if let [major @ b'0'..=b'9', b'.', minor @ b'0'..=b'9'] = name.0.as_bytes() {
                let (major, minor) = (major - b'0', minor - b'0');
                if (major, minor) > (version.major, version.minor) {
                    version = super::header::PdfVersion::new(major, minor);
                }
            }
        }
        Ok(version)
    }

    /// Get metadata from the document
    pub fn metadata(&mut self) -> ParseResult<DocumentMetadata> {
        let mut metadata = DocumentMetadata::default();
        let version = self.effective_version()?;

        if let Some(info_dict) = self.info()? {
            if let Some(title) = info_dict.get("Title").and_then(|o| o.as_string()) {
                metadata.title = Some(title.to_text_with_version(&version));
            }
            if let Some(author) = info_dict.get("Author").and_then(|o| o.as_string()) {
                metadata.author = Some(author.to_text_with_version(&version));
            }
            if let Some(subject) = info_dict.get("Subject").and_then(|o| o.as_string()) {
                metadata.subject = Some(subject.to_text_with_version(&version));
            }
            if let Some(keywords) = info_dict.get("Keywords").and_then(|o| o.as_string()) {
                metadata.keywords = Some(keywords.to_text_with_version(&version));
            }
            if let Some(creator) = info_dict.get("Creator").and_then(|o| o.as_string()) {
                metadata.creator = Some(creator.to_text_with_version(&version));
            }
            if let Some(producer) = info_dict.get("Producer").and_then(|o| o.as_string()) {
                metadata.producer = Some(producer.to_text_with_version(&version));
            }
        }

        metadata.version = self.version().to_string();
        metadata.page_count = self.page_count().ok();

        Ok(metadata)
    }

    /// Initialize the page tree navigator if not already done
    fn ensure_page_tree(&mut self) -> ParseResult<()> {
        if self.page_tree.is_none() {
            let page_count = self.page_count()?;
            self.page_tree = Some(super::page_tree::PageTree::new(page_count));
        }
        Ok(())
    }

    /// Get a specific page by index (0-based)
    ///
    /// Note: This method is currently not implemented due to borrow checker constraints.
    /// The page_tree needs mutable access to both itself and the reader, which requires
    /// a redesign of the architecture. Use PdfDocument instead for page access.
    pub fn get_page(&mut self, _index: u32) -> ParseResult<&super::page_tree::ParsedPage> {
        self.ensure_page_tree()?;

        // The page_tree needs mutable access to both itself and the reader
        // This requires a redesign of the architecture to avoid the borrow checker issue
        // For now, users should convert to PdfDocument using into_document() for page access
        Err(ParseError::SyntaxError {
            position: 0,
            message: "get_page not implemented due to borrow checker constraints. Use PdfDocument instead.".to_string(),
        })
    }

    /// Get all pages
    pub fn get_all_pages(&mut self) -> ParseResult<Vec<super::page_tree::ParsedPage>> {
        let page_count = self.page_count()?;
        let mut pages = Vec::with_capacity(page_count as usize);

        for i in 0..page_count {
            let page = self.get_page(i)?.clone();
            pages.push(page);
        }

        Ok(pages)
    }

    /// Convert this reader into a PdfDocument for easier page access
    pub fn into_document(self) -> super::document::PdfDocument<R> {
        super::document::PdfDocument::new(self)
    }

    /// Clear the parse context (useful to avoid false circular references)
    pub fn clear_parse_context(&mut self) {
        self.parse_context = StackSafeContext::new();
    }

    /// Get a mutable reference to the parse context
    pub fn parse_context_mut(&mut self) -> &mut StackSafeContext {
        &mut self.parse_context
    }

    /// Find all page objects by scanning the entire PDF in bounded chunks.
    ///
    /// Issue #339: replaces a whole-file `read_to_end` with a chunked scan that
    /// probes each object header with a small bounded window, keeping peak memory
    /// O(chunk) regardless of file size. A scan error degrades to an empty list,
    /// matching the previous behavior.
    fn find_page_objects(&mut self) -> ParseResult<Vec<(u32, u16)>> {
        let original_pos = self.reader.stream_position().unwrap_or(0);
        let result = scan_page_object_refs(&mut self.reader);
        self.reader.seek(SeekFrom::Start(original_pos)).ok();
        Ok(result.unwrap_or_default())
    }

    /// Find catalog object by scanning
    fn find_catalog_object(&mut self) -> ParseResult<(u32, u16)> {
        // FIX for Issue #83: Scan for actual catalog object, not just assume object 1
        // In signed PDFs, object 1 is often /Type/Sig (signature), not the catalog

        // Get all object numbers from xref
        let obj_numbers: Vec<u32> = self.xref.entries().keys().copied().collect();

        // Scan objects looking for /Type/Catalog
        for obj_num in obj_numbers {
            // Try to get object (generation 0 is most common)
            if let Ok(obj) = self.get_object(obj_num, 0) {
                if let Some(dict) = obj.as_dict() {
                    // Check if it's a catalog
                    if let Some(type_obj) = dict.get("Type") {
                        if let Some(type_name) = type_obj.as_name() {
                            if type_name.0 == "Catalog" {
                                return Ok((obj_num, 0));
                            }
                            // Skip known non-catalog types
                            if type_name.0 == "Sig"
                                || type_name.0 == "Pages"
                                || type_name.0 == "Page"
                            {
                                continue;
                            }
                        }
                    }
                }
            }
        }

        // Fallback: try common object numbers if scan failed
        for obj_num in [1, 2, 3, 4, 5] {
            if let Ok(obj) = self.get_object(obj_num, 0) {
                if let Some(dict) = obj.as_dict() {
                    // Check if it has catalog-like properties (Pages key)
                    if dict.contains_key("Pages") {
                        return Ok((obj_num, 0));
                    }
                }
            }
        }

        Err(ParseError::MissingKey(
            "Could not find Catalog object".to_string(),
        ))
    }

    /// Create a synthetic Pages dictionary when the catalog is missing one
    fn create_synthetic_pages_dict(
        &mut self,
        page_refs: &[(u32, u16)],
    ) -> ParseResult<&PdfDictionary> {
        use super::objects::{PdfArray, PdfName};

        // Validate and repair page objects first
        let mut valid_page_refs = Vec::new();
        for (obj_num, gen_num) in page_refs {
            if let Ok(page_obj) = self.get_object(*obj_num, *gen_num) {
                if let Some(page_dict) = page_obj.as_dict() {
                    // Ensure this is actually a page object
                    if let Some(obj_type) = page_dict.get("Type").and_then(|t| t.as_name()) {
                        if obj_type.0 == "Page" {
                            valid_page_refs.push((*obj_num, *gen_num));
                            continue;
                        }
                    }

                    // If no Type but has page-like properties, treat as page
                    if page_dict.contains_key("MediaBox") || page_dict.contains_key("Contents") {
                        valid_page_refs.push((*obj_num, *gen_num));
                    }
                }
            }
        }

        if valid_page_refs.is_empty() {
            return Err(ParseError::SyntaxError {
                position: 0,
                message: "No valid page objects found for synthetic Pages tree".to_string(),
            });
        }

        // Create hierarchical tree for many pages (more than 10)
        if valid_page_refs.len() > 10 {
            return self.create_hierarchical_pages_tree(&valid_page_refs);
        }

        // Create simple flat tree for few pages
        let mut kids = PdfArray::new();
        for (obj_num, gen_num) in &valid_page_refs {
            kids.push(PdfObject::Reference(*obj_num, *gen_num));
        }

        // Create synthetic Pages dictionary
        let mut pages_dict = PdfDictionary::new();
        pages_dict.insert(
            "Type".to_string(),
            PdfObject::Name(PdfName("Pages".to_string())),
        );
        pages_dict.insert("Kids".to_string(), PdfObject::Array(kids));
        pages_dict.insert(
            "Count".to_string(),
            PdfObject::Integer(valid_page_refs.len() as i64),
        );

        // Find a common MediaBox from the pages
        let mut media_box = None;
        for (obj_num, gen_num) in valid_page_refs.iter().take(3) {
            if let Ok(page_obj) = self.get_object(*obj_num, *gen_num) {
                if let Some(page_dict) = page_obj.as_dict() {
                    if let Some(mb) = page_dict.get("MediaBox") {
                        media_box = Some(mb.clone());
                    }
                }
            }
        }

        // Use default Letter size if no MediaBox found
        if let Some(mb) = media_box {
            pages_dict.insert("MediaBox".to_string(), mb);
        } else {
            let mut mb_array = PdfArray::new();
            mb_array.push(PdfObject::Integer(0));
            mb_array.push(PdfObject::Integer(0));
            mb_array.push(PdfObject::Integer(612));
            mb_array.push(PdfObject::Integer(792));
            pages_dict.insert("MediaBox".to_string(), PdfObject::Array(mb_array));
        }

        // Store in cache with a synthetic object number
        let synthetic_key = (u32::MAX - 1, 0);
        self.object_cache
            .insert(synthetic_key, PdfObject::Dictionary(pages_dict));

        // Return reference to cached dictionary
        if let PdfObject::Dictionary(dict) = &self.object_cache[&synthetic_key] {
            Ok(dict)
        } else {
            unreachable!("Just inserted dictionary")
        }
    }

    /// Create a hierarchical Pages tree for documents with many pages
    fn create_hierarchical_pages_tree(
        &mut self,
        page_refs: &[(u32, u16)],
    ) -> ParseResult<&PdfDictionary> {
        use super::objects::{PdfArray, PdfName};

        const PAGES_PER_NODE: usize = 10; // Max pages per intermediate node

        // Split pages into groups
        let chunks: Vec<&[(u32, u16)]> = page_refs.chunks(PAGES_PER_NODE).collect();
        let mut intermediate_nodes = Vec::new();

        // Create intermediate Pages nodes for each chunk
        for (chunk_idx, chunk) in chunks.iter().enumerate() {
            let mut kids = PdfArray::new();
            for (obj_num, gen_num) in chunk.iter() {
                kids.push(PdfObject::Reference(*obj_num, *gen_num));
            }

            let mut intermediate_dict = PdfDictionary::new();
            intermediate_dict.insert(
                "Type".to_string(),
                PdfObject::Name(PdfName("Pages".to_string())),
            );
            intermediate_dict.insert("Kids".to_string(), PdfObject::Array(kids));
            intermediate_dict.insert("Count".to_string(), PdfObject::Integer(chunk.len() as i64));

            // Store intermediate node with synthetic object number
            let intermediate_key = (u32::MAX - 2 - chunk_idx as u32, 0);
            self.object_cache
                .insert(intermediate_key, PdfObject::Dictionary(intermediate_dict));

            intermediate_nodes.push(intermediate_key);
        }

        // Create root Pages node that references intermediate nodes
        let mut root_kids = PdfArray::new();
        for (obj_num, gen_num) in &intermediate_nodes {
            root_kids.push(PdfObject::Reference(*obj_num, *gen_num));
        }

        let mut root_pages_dict = PdfDictionary::new();
        root_pages_dict.insert(
            "Type".to_string(),
            PdfObject::Name(PdfName("Pages".to_string())),
        );
        root_pages_dict.insert("Kids".to_string(), PdfObject::Array(root_kids));
        root_pages_dict.insert(
            "Count".to_string(),
            PdfObject::Integer(page_refs.len() as i64),
        );

        // Add MediaBox if available
        if let Some((obj_num, gen_num)) = page_refs.first() {
            if let Ok(page_obj) = self.get_object(*obj_num, *gen_num) {
                if let Some(page_dict) = page_obj.as_dict() {
                    if let Some(mb) = page_dict.get("MediaBox") {
                        root_pages_dict.insert("MediaBox".to_string(), mb.clone());
                    }
                }
            }
        }

        // Store root Pages dictionary
        let root_key = (u32::MAX - 1, 0);
        self.object_cache
            .insert(root_key, PdfObject::Dictionary(root_pages_dict));

        // Return reference to cached dictionary
        if let PdfObject::Dictionary(dict) = &self.object_cache[&root_key] {
            Ok(dict)
        } else {
            unreachable!("Just inserted dictionary")
        }
    }

    // =========================================================================
    // Digital Signatures API
    // =========================================================================

    /// Detect all signature fields in the PDF
    ///
    /// Returns a list of signature fields found in the document's AcroForm.
    /// This method only detects signatures; use `verify_signatures()` for
    /// complete validation.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use oxidize_pdf::parser::PdfReader;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut reader = PdfReader::open("signed.pdf")?;
    /// let signatures = reader.signatures()?;
    ///
    /// println!("Found {} signature(s)", signatures.len());
    /// for sig in &signatures {
    ///     println!("  Filter: {}", sig.filter);
    ///     if sig.is_pades() {
    ///         println!("  Type: PAdES");
    ///     }
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn signatures(&mut self) -> ParseResult<Vec<crate::signatures::SignatureField>> {
        crate::signatures::detect_signature_fields(self).map_err(|e| ParseError::SyntaxError {
            position: 0,
            message: format!("Failed to detect signatures: {}", e),
        })
    }

    /// Verify all signatures in the PDF using Mozilla's CA bundle
    ///
    /// This is a convenience method that uses the default trust store
    /// (Mozilla CA bundle). For custom trust stores, use
    /// `verify_signatures_with_trust_store()`.
    ///
    /// # Returns
    ///
    /// A vector of `FullSignatureValidationResult` for each signature found.
    /// Each result includes:
    /// - Hash verification status
    /// - Cryptographic signature verification status
    /// - Certificate validation status
    /// - Detection of modifications after signing
    ///
    /// # Example
    ///
    /// ```no_run
    /// use oxidize_pdf::parser::PdfReader;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut reader = PdfReader::open("signed.pdf")?;
    /// let results = reader.verify_signatures()?;
    ///
    /// for result in &results {
    ///     if result.is_valid() {
    ///         println!("Valid signature from: {}", result.signer_name());
    ///     } else {
    ///         println!("Invalid: {:?}", result.validation_errors());
    ///     }
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn verify_signatures(
        &mut self,
    ) -> ParseResult<Vec<crate::signatures::FullSignatureValidationResult>> {
        self.verify_signatures_with_trust_store(crate::signatures::TrustStore::default())
    }

    /// Verify all signatures in the PDF with a custom trust store
    ///
    /// Use this method when you need to validate certificates against a
    /// custom CA bundle instead of the Mozilla CA bundle.
    ///
    /// # Arguments
    ///
    /// * `trust_store` - The trust store containing root certificates
    ///
    /// # Example
    ///
    /// ```no_run
    /// use oxidize_pdf::parser::PdfReader;
    /// use oxidize_pdf::signatures::TrustStore;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut reader = PdfReader::open("signed.pdf")?;
    ///
    /// // Use empty trust store (no trusted CAs)
    /// let trust_store = TrustStore::empty();
    /// let results = reader.verify_signatures_with_trust_store(trust_store)?;
    ///
    /// for result in &results {
    ///     if !result.is_valid() {
    ///         // Expected: certificates won't be trusted
    ///         println!("Not trusted: {}", result.signer_name());
    ///     }
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn verify_signatures_with_trust_store(
        &mut self,
        trust_store: crate::signatures::TrustStore,
    ) -> ParseResult<Vec<crate::signatures::FullSignatureValidationResult>> {
        use crate::signatures::{
            has_incremental_update, parse_pkcs7_signature_detailed, verify_signature_detailed,
            FullSignatureValidationResult,
        };

        // First, read the entire PDF bytes (needed for hash computation)
        let original_pos = self.reader.stream_position().unwrap_or(0);
        self.reader.seek(SeekFrom::Start(0))?;

        let mut pdf_bytes = Vec::new();
        self.reader.read_to_end(&mut pdf_bytes)?;

        // Restore original position
        self.reader.seek(SeekFrom::Start(original_pos)).ok();

        // Detect all signature fields
        let signature_fields = self.signatures()?;

        let mut results = Vec::new();

        for field in signature_fields {
            let mut result = FullSignatureValidationResult {
                field: field.clone(),
                signer_name: None,
                signing_time: None,
                hash_valid: false,
                signature_valid: false,
                certificate_result: None,
                has_modifications_after_signing: false,
                errors: Vec::new(),
                warnings: Vec::new(),
            };

            // Check for incremental updates
            result.has_modifications_after_signing =
                has_incremental_update(&pdf_bytes, &field.byte_range);

            // Parse the PKCS#7/CMS signature
            let parsed_sig = match parse_pkcs7_signature_detailed(&field.contents) {
                Ok(sig) => sig,
                Err(e) => {
                    result
                        .errors
                        .push(format!("Failed to parse signature: {}", e));
                    results.push(result);
                    continue;
                }
            };

            // Extract signer name and signing time
            result.signing_time = parsed_sig.signing_time.clone();
            result.signer_name = parsed_sig.signer_common_name().ok();

            // Verify the cryptographic signature
            match verify_signature_detailed(&pdf_bytes, &parsed_sig, &field.byte_range) {
                Ok(verification) => {
                    result.hash_valid = verification.hash_valid;
                    result.signature_valid = verification.signature_valid;
                    if let Some(details) = verification.details {
                        result.warnings.push(details);
                    }
                }
                Err(e) => {
                    result
                        .errors
                        .push(format!("Signature verification failed: {}", e));
                }
            }

            // Validate the certificate
            match crate::signatures::validate_certificate_chain(
                &parsed_sig.signer_certificate_der,
                &parsed_sig.certificates_der,
                &trust_store,
                None,
            ) {
                Ok(cert_result) => {
                    result.certificate_result =
                        Some(cert_result.into_certificate_result_fail_closed());
                }
                Err(e) => {
                    result
                        .errors
                        .push(format!("Certificate validation failed: {}", e));
                }
            }

            results.push(result);
        }

        Ok(results)
    }
}

/// Document metadata
#[derive(Debug, Default, Clone)]
pub struct DocumentMetadata {
    pub title: Option<String>,
    pub author: Option<String>,
    pub subject: Option<String>,
    pub keywords: Option<String>,
    pub creator: Option<String>,
    pub producer: Option<String>,
    pub creation_date: Option<String>,
    pub modification_date: Option<String>,
    pub version: String,
    pub page_count: Option<u32>,
}

pub struct EOLIter<'s> {
    remainder: &'s str,
}
impl<'s> Iterator for EOLIter<'s> {
    type Item = &'s str;

    fn next(&mut self) -> Option<Self::Item> {
        if self.remainder.is_empty() {
            return None;
        }

        if let Some((i, sep)) = ["\r\n", "\n", "\r"]
            .iter()
            .filter_map(|&sep| self.remainder.find(sep).map(|i| (i, sep)))
            .min_by_key(|(i, _)| *i)
        {
            let (line, rest) = self.remainder.split_at(i);
            self.remainder = &rest[sep.len()..];
            Some(line)
        } else {
            let line = self.remainder;
            self.remainder = "";
            Some(line)
        }
    }
}
pub trait PDFLines: AsRef<str> {
    fn pdf_lines(&self) -> EOLIter<'_> {
        EOLIter {
            remainder: self.as_ref(),
        }
    }
}
impl PDFLines for &str {}
impl<'a> PDFLines for std::borrow::Cow<'a, str> {}
impl PDFLines for String {}

#[cfg(test)]
mod tests {

    use super::*;
    use crate::parser::objects::{PdfName, PdfString};
    use crate::parser::test_helpers::*;
    use crate::parser::ParseOptions;
    use std::io::Cursor;

    #[test]
    fn encrypted_compressed_sparse_arrays_preserve_strings_and_positions() {
        for variant in [
            "plain",
            "rc4",
            "aes128",
            "aes256r5",
            "aes256r6",
            "aes256r6_uncompressed",
        ] {
            let bytes = std::fs::read(format!(
                "{}/tests/fixtures/issue_832_{variant}.pdf",
                env!("CARGO_MANIFEST_DIR")
            ))
            .unwrap();
            let mut reader = PdfReader::new(Cursor::new(bytes)).unwrap();
            assert!(reader.unlock_with_password("").unwrap());
            let (id, generation) = reader
                .catalog()
                .unwrap()
                .get("Sparse")
                .unwrap()
                .as_reference()
                .unwrap();
            for _ in 0..2 {
                let entries = reader.sparse_array(id, generation).unwrap();
                assert_eq!(entries.len(), 4, "{variant}");
                assert!(!entries.contains_key(&0));
                assert!(!entries.contains_key(&2));
                assert_eq!(
                    entries[&1].as_string().unwrap().as_bytes(),
                    b"Identity",
                    "{variant}"
                );
                assert_eq!(
                    entries[&3]
                        .as_dict()
                        .unwrap()
                        .get("Label")
                        .unwrap()
                        .as_string()
                        .unwrap()
                        .as_bytes(),
                    b"Adobe",
                    "{variant}"
                );
                assert_eq!(
                    entries[&4].as_string().unwrap().as_bytes(),
                    b"",
                    "{variant}"
                );
                assert_eq!(
                    entries[&5].as_string().unwrap().as_bytes(),
                    b"0123456789abcdef0123456789abcdef",
                    "{variant}"
                );
            }
        }
    }

    #[test]
    fn test_reader_construction() {
        let pdf_data = create_minimal_pdf();
        let cursor = Cursor::new(pdf_data);
        let result = PdfReader::new(cursor);
        assert!(result.is_ok());
    }

    // --- Total string-slicing helpers used by the manual recovery path ---
    //
    // The recovery path builds these from `String::from_utf8_lossy` over
    // arbitrary bytes, so every input below is reachable from a malformed file.
    // The contract under test is totality: a wrong answer is acceptable on
    // garbage, a panic is not.

    /// Reader instrumented with two shared counters so a test can reset them after
    /// `PdfReader::new` and observe only the reads done by the method under test:
    /// - `total_read`: total bytes consumed — distinguishes a bounded locate that
    ///   stops at a front object from an unbounded `read_to_end` of the whole file.
    /// - `max_read`: largest single read request — distinguishes a scan that only
    ///   ever asks for one chunk from a `read_to_end` that grows one huge buffer.
    struct CountingReader<R> {
        inner: R,
        total_read: std::sync::Arc<std::sync::atomic::AtomicUsize>,
        max_read: std::sync::Arc<std::sync::atomic::AtomicUsize>,
    }
    impl<R: Read> Read for CountingReader<R> {
        fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
            self.max_read
                .fetch_max(buf.len(), std::sync::atomic::Ordering::SeqCst);
            let n = self.inner.read(buf)?;
            self.total_read
                .fetch_add(n, std::sync::atomic::Ordering::SeqCst);
            Ok(n)
        }
    }
    impl<R: Seek> Seek for CountingReader<R> {
        fn seek(&mut self, p: SeekFrom) -> std::io::Result<u64> {
            self.inner.seek(p)
        }
    }

    #[test]
    fn test_extract_stream_manually_bounded_honors_length() {
        use crate::parser::objects::PdfObject;
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::Arc;

        // Issue #339: extract_object_or_stream_manually must locate the object
        // and read its stream body bounded by /Length, never buffering the whole
        // file. A large XMP-sized stream (> the 256 KiB dict window) followed by a
        // big unrelated filler object makes the bound observable and proves the
        // body is not truncated at any fixed window.
        const L: usize = 300 * 1024; // 307200, exceeds MANUAL_DICT_WINDOW
        const FILLER: usize = 4 * 1024 * 1024; // dwarfs the target object

        // Deterministic +1 byte ramp: never produces the "endstream" byte run
        // (those bytes are not an arithmetic +1 sequence), so the marker search
        // is unambiguous and full-content equality is meaningful.
        let payload: Vec<u8> = (0..L).map(|i| (i % 251) as u8).collect();
        let filler: Vec<u8> = (0..FILLER).map(|i| ((i + 7) % 251) as u8).collect();

        let header = b"%PDF-1.4\n";
        let obj1 = b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n";
        let obj2 = b"2 0 obj\n<< /Type /Pages /Kids [] /Count 0 >>\nendobj\n";

        let mut data = Vec::new();
        data.extend_from_slice(header);
        let obj1_start = data.len();
        data.extend_from_slice(obj1);
        let obj2_start = data.len();
        data.extend_from_slice(obj2);

        // Object 4: the target stream with a DIRECT /Length. Not listed in xref,
        // so it is only reachable via the manual scan fallback.
        data.extend_from_slice(
            b"4 0 obj\n<< /Type /Metadata /Subtype /XML /Length 307200 >>\nstream\n",
        );
        data.extend_from_slice(&payload);
        data.extend_from_slice(b"\nendstream\nendobj\n");

        // Object 5: large unrelated filler so file size >> target object size.
        data.extend_from_slice(b"5 0 obj\n<< /Length 4194304 >>\nstream\n");
        data.extend_from_slice(&filler);
        data.extend_from_slice(b"\nendstream\nendobj\n");

        // xref lists only 0/1/2; startxref sits at the very end so new() parses it.
        let xref_start = data.len();
        let xref = format!(
            "xref\n0 3\n0000000000 65535 f \n{obj1_start:010} 00000 n \n{obj2_start:010} 00000 n \ntrailer\n<< /Size 3 /Root 1 0 R >>\nstartxref\n{xref_start}\n%%EOF"
        );
        data.extend_from_slice(xref.as_bytes());
        let file_len = data.len();

        let counter = Arc::new(AtomicUsize::new(0));
        let reader = CountingReader {
            inner: Cursor::new(data),
            total_read: counter.clone(),
            max_read: Arc::new(AtomicUsize::new(0)),
        };
        let mut pdf = PdfReader::new_with_options(reader, ParseOptions::tolerant())
            .expect("minimal PDF must parse");

        // Observe only the reads performed by the method under test.
        counter.store(0, Ordering::SeqCst);
        let obj = pdf
            .extract_object_or_stream_manually(4)
            .expect("stream object 4 must be extracted");

        // Content intact: full stream body, not truncated at any fixed window.
        let stream = match &obj {
            PdfObject::Stream(s) => s,
            other => panic!("expected a stream, got {other:?}"),
        };
        assert_eq!(
            stream.data.len(),
            L,
            "stream body must equal /Length (no truncation)"
        );
        assert_eq!(stream.data, payload, "stream body content must be intact");

        // Bounded: the target object sits near the front, so a bounded locate that
        // stops at object 4 reads far less than the whole file (which holds the
        // 4 MiB filler). An unbounded read_to_end would consume the entire file.
        let total_read = counter.load(Ordering::SeqCst);
        assert!(
            total_read <= L + 2 * MANUAL_DICT_WINDOW,
            "manual stream extraction read {total_read} bytes total (file={file_len}); not bounded by object size"
        );
    }

    /// Build a minimal PDF whose object 4 is a stream reachable only via the manual
    /// scan fallback (absent from xref), with the given dictionary body and payload.
    /// Returns the full file bytes. Used by the #351 reconstruction-dict tests.
    #[cfg(test)]
    fn build_manual_stream_pdf(obj4_dict_body: &str, payload: &[u8]) -> Vec<u8> {
        let header = b"%PDF-1.4\n";
        let obj1 = b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n";
        let obj2 = b"2 0 obj\n<< /Type /Pages /Kids [] /Count 0 >>\nendobj\n";

        let mut data = Vec::new();
        data.extend_from_slice(header);
        let obj1_start = data.len();
        data.extend_from_slice(obj1);
        let obj2_start = data.len();
        data.extend_from_slice(obj2);

        // Object 4: stream reachable only via the manual scan (not listed in xref).
        data.extend_from_slice(format!("4 0 obj\n<<{obj4_dict_body}>>\nstream\n").as_bytes());
        data.extend_from_slice(payload);
        data.extend_from_slice(b"\nendstream\nendobj\n");

        // xref lists only 0/1/2; startxref at the end so new() parses the file.
        let xref_start = data.len();
        let xref = format!(
            "xref\n0 3\n0000000000 65535 f \n{obj1_start:010} 00000 n \n{obj2_start:010} 00000 n \ntrailer\n<< /Size 3 /Root 1 0 R >>\nstartxref\n{xref_start}\n%%EOF"
        );
        data.extend_from_slice(xref.as_bytes());
        data
    }

    #[test]
    fn test_issue720_recovery_uses_bytes_and_reports_repair() {
        use std::sync::{Arc, Mutex};
        #[derive(Clone)]
        struct Capture(Arc<Mutex<Vec<u8>>>);
        impl std::io::Write for Capture {
            fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
                self.0.lock().unwrap().extend_from_slice(bytes);
                Ok(bytes.len())
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }
        for id in [4, 113, 200] {
            let bytes =
                build_manual_stream_pdf(" /Length 3 /Parent 2 0 R /Vendor (keep >> text) ", b"abc");
            let bytes = String::from_utf8(bytes)
                .unwrap()
                .replace("4 0 obj", &format!("{id} 0 obj"));
            let mut reader = PdfReader::new_with_options(
                Cursor::new(bytes.into_bytes()),
                ParseOptions::lenient(),
            )
            .unwrap();
            // Force the missing-xref boundary even if constructor scanning repaired it.
            reader.xref = XRefTable::new();
            reader.object_cache.clear();
            let captured = Capture(Arc::new(Mutex::new(Vec::new())));
            let output = captured.clone();
            let subscriber = tracing_subscriber::fmt()
                .without_time()
                .with_ansi(false)
                .with_max_level(tracing::Level::WARN)
                .with_writer(move || output.clone())
                .finish();
            let _guard = tracing::subscriber::set_default(subscriber);
            let stream = reader.get_object(id, 0).unwrap().as_stream().unwrap();
            assert_eq!(stream.data, b"abc");
            assert_eq!(
                stream.dict.get("Parent").unwrap().as_reference(),
                Some((2, 0))
            );
            assert_eq!(
                stream
                    .dict
                    .get("Vendor")
                    .unwrap()
                    .as_string()
                    .unwrap()
                    .as_bytes(),
                b"keep >> text"
            );
            assert!(String::from_utf8(captured.0.lock().unwrap().clone())
                .unwrap()
                .contains("Recovered object from scanned PDF bytes"));
            assert!(
                reader.xref.get_entry(id).is_none(),
                "recovery must not fabricate xref storage"
            );
        }
    }

    #[test]
    fn test_issue720_stream_length_cannot_allocate_beyond_file() {
        let data = build_manual_stream_pdf(" /Length 9223372036854775807 ", b"abc");
        let mut reader =
            PdfReader::new_with_options(Cursor::new(data), ParseOptions::lenient()).unwrap();
        let error = reader.extract_object_or_stream_manually(4).unwrap_err();
        assert!(matches!(error, ParseError::SyntaxError { .. }));
    }

    #[test]
    fn test_issue720_stream_recovery_rejects_stale_length_within_file() {
        let data = build_manual_stream_pdf(" /Length 5 ", b"abc");
        let mut reader =
            PdfReader::new_with_options(Cursor::new(data), ParseOptions::lenient()).unwrap();
        assert!(reader.extract_object_or_stream_manually(4).is_err());
    }

    #[test]
    fn test_issue720_stream_and_dictionary_recovery_have_independent_permissions() {
        for (syntax, streams) in [(false, false), (false, true), (true, false), (true, true)] {
            let data = build_manual_stream_pdf(" /Length 3 ", b"abc");
            let mut options = ParseOptions::default();
            options.lenient_syntax = syntax;
            options.lenient_streams = streams;
            let mut reader = PdfReader::new_with_options(Cursor::new(data), options).unwrap();
            // Both objects have real bytes; force recovery rather than normal loading.
            reader.xref = XRefTable::new();
            reader.object_cache.clear();
            let dictionary = reader.get_object(2, 0);
            assert_eq!(
                dictionary.is_ok(),
                syntax,
                "dictionary permission: {syntax}/{streams}"
            );
            if let Ok(object) = dictionary {
                assert_eq!(
                    object.as_dict().unwrap().get("Count").unwrap().as_integer(),
                    Some(0)
                );
            }
            let stream = reader.get_object(4, 0);
            assert_eq!(
                stream.is_ok(),
                streams,
                "stream permission: {syntax}/{streams}"
            );
            if let Ok(object) = stream {
                assert_eq!(object.as_stream().unwrap().data, b"abc");
            }
        }
    }

    #[test]
    fn test_issue720_disabled_reconstruction_preserves_verification_boundary() {
        let data = build_manual_stream_pdf(" /Length 3 ", b"abc");
        let mut reader =
            PdfReader::new_with_options(Cursor::new(data), ParseOptions::lenient()).unwrap();
        reader.xref = XRefTable::new();
        reader.object_cache.clear();
        reader.disable_object_reconstruction();
        assert!(reader.get_object(4, 0).unwrap().is_null());
    }

    #[test]
    fn test_reconstruct_preserves_non_flate_filter() {
        use crate::parser::objects::PdfObject;

        // Issue #351: the manual stream reconstruction fallback must preserve the
        // stream's /Filter generically, not only /FlateDecode. A DCTDecode stream
        // reached via the manual scan previously lost its /Filter (and /Subtype),
        // causing downstream to treat compressed bytes as raw — a silent wrong
        // result. Body content is irrelevant here; the dictionary is the contract.
        let payload = b"\xff\xd8\xff\xe0not-a-real-jpeg\xff\xd9";
        let len = payload.len();
        let dict_body =
            format!(" /Type /XObject /Subtype /Image /Filter /DCTDecode /Length {len} ");
        let data = build_manual_stream_pdf(&dict_body, payload);

        let mut pdf = PdfReader::new_with_options(Cursor::new(data), ParseOptions::tolerant())
            .expect("minimal PDF must parse");
        let obj = pdf
            .extract_object_or_stream_manually(4)
            .expect("stream object 4 must be extracted");
        let stream = match &obj {
            PdfObject::Stream(s) => s,
            other => panic!("expected a stream, got {other:?}"),
        };

        assert_eq!(
            stream
                .dict
                .get("Filter")
                .and_then(|o| o.as_name())
                .map(|n| n.0.as_str()),
            Some("DCTDecode"),
            "non-Flate /Filter must be preserved in reconstructed stream dict"
        );
        assert_eq!(
            stream
                .dict
                .get("Subtype")
                .and_then(|o| o.as_name())
                .map(|n| n.0.as_str()),
            Some("Image"),
            "/Subtype must be preserved, not dropped"
        );
        assert_eq!(stream.data, payload, "stream body must be intact");
    }

    #[test]
    fn test_reconstruct_preserves_filter_array_and_decodeparms() {
        use crate::parser::objects::PdfObject;

        // Issue #351: filter arrays and /DecodeParms must survive the manual
        // reconstruction, and the parser must tolerate the no-space `/Filter[`
        // spacing variant. A LZWDecode→FlateDecode chain with predictor parms is
        // a realistic case the old hardcoded `/Filter /FlateDecode` match dropped.
        let payload = b"compressed-bytes-placeholder";
        let len = payload.len();
        let dict_body = format!(
            " /Filter[/LZWDecode /FlateDecode] /DecodeParms[null<< /Predictor 12 /Columns 4 >>] /Length {len} "
        );
        let data = build_manual_stream_pdf(&dict_body, payload);

        let mut pdf = PdfReader::new_with_options(Cursor::new(data), ParseOptions::tolerant())
            .expect("minimal PDF must parse");
        let obj = pdf
            .extract_object_or_stream_manually(4)
            .expect("stream object 4 must be extracted");
        let stream = match &obj {
            PdfObject::Stream(s) => s,
            other => panic!("expected a stream, got {other:?}"),
        };

        let filter = stream
            .dict
            .get("Filter")
            .and_then(|o| o.as_array())
            .expect("/Filter array must be preserved");
        let names: Vec<&str> = filter
            .0
            .iter()
            .filter_map(|o| o.as_name())
            .map(|n| n.0.as_str())
            .collect();
        assert_eq!(
            names,
            vec!["LZWDecode", "FlateDecode"],
            "filter array entries must be preserved in order"
        );
        assert!(
            stream.dict.get("DecodeParms").is_some(),
            "/DecodeParms must be preserved alongside the filter chain"
        );
        assert_eq!(stream.data, payload, "stream body must be intact");
    }

    #[test]
    fn test_reconstruct_malformed_dict_rejects_instead_of_discarding_keys() {
        // #720: an incomplete value provides no evidence for an empty dictionary.
        let data =
            build_manual_stream_pdf(" /Length 10 /Filter /DCTDecode /BadKey ", b"0123456789");
        let mut pdf =
            PdfReader::new_with_options(Cursor::new(data), ParseOptions::tolerant()).unwrap();
        assert!(pdf.extract_object_or_stream_manually(4).is_err());
        pdf.xref = XRefTable::new();
        pdf.object_cache.clear();
        assert!(
            pdf.get_object(4, 0).is_err(),
            "existing malformed bytes must not become null"
        );
    }

    #[test]
    fn test_find_page_objects_bounded() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::Arc;

        // Issue #339: find_page_objects scans the whole file to enumerate every
        // page object, so it must do so in bounded chunks — never holding the
        // entire file in memory. A multi-MiB filler makes an unbounded read_to_end
        // (one growing buffer) observable: the largest single read approaches the
        // file size, whereas the chunked scan never asks for more than one chunk.
        const FILLER: usize = 2 * 1024 * 1024;
        let filler: Vec<u8> = (0..FILLER).map(|i| (i % 251) as u8).collect();

        let header = b"%PDF-1.4\n";
        let obj1 = b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n";
        let obj2 = b"2 0 obj\n<< /Type /Pages /Kids [4 0 R 5 0 R 6 0 R] /Count 3 >>\nendobj\n";

        let mut data = Vec::new();
        data.extend_from_slice(header);
        let obj1_start = data.len();
        data.extend_from_slice(obj1);
        let obj2_start = data.len();
        data.extend_from_slice(obj2);

        // Three page objects, not listed in xref, so only the scan finds them.
        for n in [4u32, 5, 6] {
            data.extend_from_slice(
                format!(
                    "{n} 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] >>\nendobj\n"
                )
                .as_bytes(),
            );
        }

        // Large unrelated filler stream object so the file dwarfs the chunk size.
        data.extend_from_slice(b"7 0 obj\n<< /Length 2097152 >>\nstream\n");
        data.extend_from_slice(&filler);
        data.extend_from_slice(b"\nendstream\nendobj\n");

        let xref_start = data.len();
        let xref = format!(
            "xref\n0 3\n0000000000 65535 f \n{obj1_start:010} 00000 n \n{obj2_start:010} 00000 n \ntrailer\n<< /Size 3 /Root 1 0 R >>\nstartxref\n{xref_start}\n%%EOF"
        );
        data.extend_from_slice(xref.as_bytes());
        let file_len = data.len();

        let max_read = Arc::new(AtomicUsize::new(0));
        let reader = CountingReader {
            inner: Cursor::new(data),
            total_read: Arc::new(AtomicUsize::new(0)),
            max_read: max_read.clone(),
        };
        let mut pdf = PdfReader::new_with_options(reader, ParseOptions::tolerant())
            .expect("minimal PDF must parse");

        // Observe only the reads performed by the method under test.
        max_read.store(0, Ordering::SeqCst);
        let pages = pdf.find_page_objects().expect("page scan must succeed");

        // All three page objects discovered (and the /Pages node excluded).
        assert_eq!(
            pages,
            vec![(4, 0), (5, 0), (6, 0)],
            "must find exactly the three /Type /Page objects"
        );

        // Bounded: the scan never requested more than one chunk in a single read.
        let peak = max_read.load(Ordering::SeqCst);
        assert!(
            peak <= 128 * 1024,
            "page scan requested {peak} bytes in one read (file={file_len}); not bounded"
        );
    }

    #[test]
    fn test_signature_verification_reads_full_file_intentionally() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::Arc;

        // Issue #339 boundary: unlike the manual-extraction fallbacks, signature
        // verification MUST read the entire file — the signature digest is computed
        // over the full /ByteRange (ISO 32000-1 §12.8.3.3). This pins that intent so
        // the deliberate read_to_end is not mistakenly "optimized" into a bounded read.
        let data = create_minimal_pdf();
        let file_len = data.len();

        let total = Arc::new(AtomicUsize::new(0));
        let reader = CountingReader {
            inner: Cursor::new(data),
            total_read: total.clone(),
            max_read: Arc::new(AtomicUsize::new(0)),
        };
        let mut pdf = PdfReader::new_with_options(reader, ParseOptions::tolerant())
            .expect("minimal PDF must parse");

        total.store(0, Ordering::SeqCst);
        // No signatures present: the function still reads the whole file up front.
        let _ = pdf.verify_signatures();
        let read = total.load(Ordering::SeqCst);
        assert!(
            read >= file_len,
            "signature verification must read the whole file (read {read}, file {file_len})"
        );
    }

    #[test]
    fn test_reader_version() {
        let pdf_data = create_minimal_pdf();
        let cursor = Cursor::new(pdf_data);
        let reader = PdfReader::new(cursor).unwrap();
        assert_eq!(reader.version().major, 1);
        assert_eq!(reader.version().minor, 4);
    }

    #[test]
    fn test_reader_different_versions() {
        let versions = vec![
            "1.0", "1.1", "1.2", "1.3", "1.4", "1.5", "1.6", "1.7", "2.0",
        ];

        for version in versions {
            let pdf_data = create_pdf_with_version(version);
            let cursor = Cursor::new(pdf_data);
            let reader = PdfReader::new(cursor).unwrap();

            let parts: Vec<&str> = version.split('.').collect();
            assert_eq!(reader.version().major, parts[0].parse::<u8>().unwrap());
            assert_eq!(reader.version().minor, parts[1].parse::<u8>().unwrap());
        }
    }

    #[test]
    fn test_reader_catalog() {
        let pdf_data = create_minimal_pdf();
        let cursor = Cursor::new(pdf_data);
        let mut reader = PdfReader::new(cursor).unwrap();

        let catalog = reader.catalog();
        assert!(catalog.is_ok());

        let catalog_dict = catalog.unwrap();
        assert_eq!(
            catalog_dict.get("Type"),
            Some(&PdfObject::Name(PdfName("Catalog".to_string())))
        );
    }

    #[test]
    fn test_reader_info_none() {
        let pdf_data = create_minimal_pdf();
        let cursor = Cursor::new(pdf_data);
        let mut reader = PdfReader::new(cursor).unwrap();

        let info = reader.info().unwrap();
        assert!(info.is_none());
    }

    #[test]
    fn test_reader_info_present() {
        let pdf_data = create_pdf_with_info();
        let cursor = Cursor::new(pdf_data);
        let mut reader = PdfReader::new(cursor).unwrap();

        let info = reader.info().unwrap();
        assert!(info.is_some());

        let info_dict = info.unwrap();
        assert_eq!(
            info_dict.get("Title"),
            Some(&PdfObject::String(PdfString(
                "Test PDF".to_string().into_bytes()
            )))
        );
        assert_eq!(
            info_dict.get("Author"),
            Some(&PdfObject::String(PdfString(
                "Test Author".to_string().into_bytes()
            )))
        );
    }

    #[test]
    fn test_reader_get_object() {
        let pdf_data = create_minimal_pdf();
        let cursor = Cursor::new(pdf_data);
        let mut reader = PdfReader::new(cursor).unwrap();

        // Get catalog object (1 0 obj)
        let obj = reader.get_object(1, 0);
        assert!(obj.is_ok());

        let catalog = obj.unwrap();
        assert!(catalog.as_dict().is_some());
    }

    #[test]
    fn test_reader_get_invalid_object() {
        let pdf_data = create_minimal_pdf();
        let cursor = Cursor::new(pdf_data);
        let mut reader = PdfReader::new(cursor).unwrap();

        // Try to get non-existent object
        let obj = reader.get_object(999, 0);
        assert!(obj.is_err());
    }

    #[test]
    fn test_reader_get_free_object() {
        let pdf_data = create_minimal_pdf();
        let cursor = Cursor::new(pdf_data);
        let mut reader = PdfReader::new(cursor).unwrap();

        // Object 0 is always free (f flag in xref)
        let obj = reader.get_object(0, 65535);
        assert!(obj.is_ok());
        assert_eq!(obj.unwrap(), &PdfObject::Null);
    }

    #[test]
    fn test_reader_resolve_reference() {
        let pdf_data = create_minimal_pdf();
        let cursor = Cursor::new(pdf_data);
        let mut reader = PdfReader::new(cursor).unwrap();

        // Create a reference to catalog
        let ref_obj = PdfObject::Reference(1, 0);
        let resolved = reader.resolve(&ref_obj);

        assert!(resolved.is_ok());
        assert!(resolved.unwrap().as_dict().is_some());
    }

    #[test]
    fn test_reader_resolve_non_reference() {
        let pdf_data = create_minimal_pdf();
        let cursor = Cursor::new(pdf_data);
        let mut reader = PdfReader::new(cursor).unwrap();

        // Resolve a non-reference object
        let int_obj = PdfObject::Integer(42);
        let resolved = reader.resolve(&int_obj).unwrap();

        assert_eq!(resolved, &PdfObject::Integer(42));
    }

    #[test]
    fn test_reader_cache_behavior() {
        let pdf_data = create_minimal_pdf();
        let cursor = Cursor::new(pdf_data);
        let mut reader = PdfReader::new(cursor).unwrap();

        // Get object first time
        let obj1 = reader.get_object(1, 0).unwrap();
        assert!(obj1.as_dict().is_some());

        // Get same object again - should use cache
        let obj2 = reader.get_object(1, 0).unwrap();
        assert!(obj2.as_dict().is_some());
    }

    #[test]
    fn test_reader_wrong_generation() {
        let pdf_data = create_minimal_pdf();
        let cursor = Cursor::new(pdf_data);
        let mut reader = PdfReader::new(cursor).unwrap();

        // Try to get object with wrong generation number
        let obj = reader.get_object(1, 99);
        assert!(obj.is_err());
    }

    #[test]
    fn test_reader_invalid_pdf() {
        let invalid_data = b"This is not a PDF file";
        let cursor = Cursor::new(invalid_data.to_vec());
        let result = PdfReader::new(cursor);

        assert!(result.is_err());
    }

    #[test]
    fn test_reader_corrupt_xref() {
        let corrupt_pdf = b"%PDF-1.4
1 0 obj
<< /Type /Catalog >>
endobj
xref
corrupted xref table
trailer
<< /Size 2 /Root 1 0 R >>
startxref
24
%%EOF"
            .to_vec();

        // Issue #374: a corrupt xref table is reconstructed by scanning object
        // headers (default options allow recovery). The scan finds object 1 and
        // resolves the catalog.
        let cursor = Cursor::new(corrupt_pdf.clone());
        let mut reader = PdfReader::new(cursor).expect("corrupt xref is reconstructed by scan");
        let catalog = reader
            .catalog()
            .expect("catalog resolves after reconstruction");
        assert_eq!(
            catalog.get("Type"),
            Some(&PdfObject::Name(PdfName("Catalog".to_string()))),
        );

        // strict() disables recovery: the corrupt xref must still fail loudly.
        let cursor = Cursor::new(corrupt_pdf);
        assert!(PdfReader::new_with_options(cursor, ParseOptions::strict()).is_err());
    }

    #[test]
    fn test_reader_missing_trailer() {
        let pdf_no_trailer = b"%PDF-1.4
1 0 obj
<< /Type /Catalog >>
endobj
xref
0 2
0000000000 65535 f 
0000000009 00000 n 
startxref
24
%%EOF"
            .to_vec();

        // Issue #374: without a trailer, recovery locates the catalog by
        // scanning objects for /Type /Catalog and synthesizes the trailer's
        // /Root, so default parsing succeeds.
        let cursor = Cursor::new(pdf_no_trailer.clone());
        let mut reader = PdfReader::new(cursor).expect("missing trailer recovered by object scan");
        let catalog = reader
            .catalog()
            .expect("catalog resolved from scanned /Root");
        assert_eq!(
            catalog.get("Type"),
            Some(&PdfObject::Name(PdfName("Catalog".to_string()))),
        );

        // strict() disables recovery: a missing trailer must still fail.
        let cursor = Cursor::new(pdf_no_trailer);
        assert!(PdfReader::new_with_options(cursor, ParseOptions::strict()).is_err());
    }

    #[test]
    fn test_reader_empty_pdf() {
        let cursor = Cursor::new(Vec::new());
        let result = PdfReader::new(cursor);
        assert!(result.is_err());
    }

    #[test]
    fn test_reader_page_count() {
        let pdf_data = create_minimal_pdf();
        let cursor = Cursor::new(pdf_data);
        let mut reader = PdfReader::new(cursor).unwrap();

        let count = reader.page_count();
        assert!(count.is_ok());
        assert_eq!(count.unwrap(), 0); // Minimal PDF has no pages
    }

    #[test]
    fn test_reader_into_document() {
        let pdf_data = create_minimal_pdf();
        let cursor = Cursor::new(pdf_data);
        let reader = PdfReader::new(cursor).unwrap();

        let document = reader.into_document();
        // Document should be valid
        let page_count = document.page_count();
        assert!(page_count.is_ok());
    }

    #[test]
    fn test_reader_pages_dict() {
        let pdf_data = create_minimal_pdf();
        let cursor = Cursor::new(pdf_data);
        let mut reader = PdfReader::new(cursor).unwrap();

        let pages = reader.pages();
        assert!(pages.is_ok());
        let pages_dict = pages.unwrap();
        assert_eq!(
            pages_dict.get("Type"),
            Some(&PdfObject::Name(PdfName("Pages".to_string())))
        );
    }

    #[test]
    fn test_reader_pdf_with_binary_data() {
        let pdf_data = create_pdf_with_binary_marker();

        let cursor = Cursor::new(pdf_data);
        let result = PdfReader::new(cursor);
        assert!(result.is_ok());
    }

    #[test]
    fn test_reader_metadata() {
        let pdf_data = create_pdf_with_info();
        let cursor = Cursor::new(pdf_data);
        let mut reader = PdfReader::new(cursor).unwrap();

        let metadata = reader.metadata().unwrap();
        assert_eq!(metadata.title, Some("Test PDF".to_string()));
        assert_eq!(metadata.author, Some("Test Author".to_string()));
        assert_eq!(metadata.subject, Some("Testing".to_string()));
        assert_eq!(metadata.version, "1.4".to_string());
    }

    #[test]
    fn test_reader_metadata_empty() {
        let pdf_data = create_minimal_pdf();
        let cursor = Cursor::new(pdf_data);
        let mut reader = PdfReader::new(cursor).unwrap();

        let metadata = reader.metadata().unwrap();
        assert!(metadata.title.is_none());
        assert!(metadata.author.is_none());
        assert_eq!(metadata.version, "1.4".to_string());
        assert_eq!(metadata.page_count, Some(0));
    }

    #[test]
    fn test_reader_object_number_mismatch() {
        // This test validates that the reader properly handles
        // object number mismatches. We'll create a valid PDF
        // and then try to access an object with wrong generation number
        let pdf_data = create_minimal_pdf();
        let cursor = Cursor::new(pdf_data);
        let mut reader = PdfReader::new(cursor).unwrap();

        // Object 1 exists with generation 0
        // Try to get it with wrong generation number
        let result = reader.get_object(1, 99);
        assert!(result.is_err());

        // Also test with a non-existent object number
        let result2 = reader.get_object(999, 0);
        assert!(result2.is_err());
    }

    #[test]
    fn test_document_metadata_struct() {
        let metadata = DocumentMetadata {
            title: Some("Title".to_string()),
            author: Some("Author".to_string()),
            subject: Some("Subject".to_string()),
            keywords: Some("Keywords".to_string()),
            creator: Some("Creator".to_string()),
            producer: Some("Producer".to_string()),
            creation_date: Some("D:20240101".to_string()),
            modification_date: Some("D:20240102".to_string()),
            version: "1.5".to_string(),
            page_count: Some(10),
        };

        assert_eq!(metadata.title, Some("Title".to_string()));
        assert_eq!(metadata.page_count, Some(10));
    }

    #[test]
    fn test_document_metadata_default() {
        let metadata = DocumentMetadata::default();
        assert!(metadata.title.is_none());
        assert!(metadata.author.is_none());
        assert!(metadata.subject.is_none());
        assert!(metadata.keywords.is_none());
        assert!(metadata.creator.is_none());
        assert!(metadata.producer.is_none());
        assert!(metadata.creation_date.is_none());
        assert!(metadata.modification_date.is_none());
        assert_eq!(metadata.version, "".to_string());
        assert!(metadata.page_count.is_none());
    }

    #[test]
    fn test_document_metadata_clone() {
        let metadata = DocumentMetadata {
            title: Some("Test".to_string()),
            version: "1.4".to_string(),
            ..Default::default()
        };

        let cloned = metadata;
        assert_eq!(cloned.title, Some("Test".to_string()));
        assert_eq!(cloned.version, "1.4".to_string());
    }

    #[test]
    fn test_reader_trailer_validation_error() {
        // PDF with invalid trailer (missing required keys)
        let bad_pdf = b"%PDF-1.4
1 0 obj
<< /Type /Catalog >>
endobj
xref
0 2
0000000000 65535 f 
0000000009 00000 n 
trailer
<< /Size 2 >>
startxref
46
%%EOF"
            .to_vec();

        // Issue #374: a trailer without /Root triggers recovery, which locates
        // the catalog by content scan and synthesizes /Root, so default parsing
        // succeeds and resolves the catalog.
        let cursor = Cursor::new(bad_pdf.clone());
        let mut reader = PdfReader::new(cursor).expect("missing /Root recovered by object scan");
        let catalog = reader
            .catalog()
            .expect("catalog resolved from scanned /Root");
        assert_eq!(
            catalog.get("Type"),
            Some(&PdfObject::Name(PdfName("Catalog".to_string()))),
        );

        // strict() disables recovery: a trailer without /Root must still fail.
        let cursor = Cursor::new(bad_pdf);
        assert!(PdfReader::new_with_options(cursor, ParseOptions::strict()).is_err());
    }

    #[test]
    fn test_reader_with_options() {
        let pdf_data = create_minimal_pdf();
        let cursor = Cursor::new(pdf_data);
        let mut options = ParseOptions::default();
        options.lenient_streams = true;
        options.max_recovery_bytes = 2000;
        options.collect_warnings = true;

        let reader = PdfReader::new_with_options(cursor, options);
        assert!(reader.is_ok());
    }

    #[test]
    fn test_lenient_stream_parsing() {
        // Create a PDF with incorrect stream length
        let pdf_data = b"%PDF-1.4
1 0 obj
<< /Type /Catalog /Pages 2 0 R >>
endobj
2 0 obj
<< /Type /Pages /Kids [3 0 R] /Count 1 >>
endobj
3 0 obj
<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 4 0 R >>
endobj
4 0 obj
<< /Length 10 >>
stream
This is a longer stream than 10 bytes
endstream
endobj
xref
0 5
0000000000 65535 f 
0000000009 00000 n 
0000000058 00000 n 
0000000116 00000 n 
0000000219 00000 n 
trailer
<< /Size 5 /Root 1 0 R >>
startxref
299
%%EOF"
            .to_vec();

        // Test strict mode - using strict options since new() is now lenient
        let cursor = Cursor::new(pdf_data.clone());
        let strict_options = ParseOptions::strict();
        let strict_reader = PdfReader::new_with_options(cursor, strict_options);
        // The PDF is malformed (incomplete xref), so even basic parsing fails
        assert!(strict_reader.is_err());

        // Issue #374: default options allow recovery, so a PDF with a mismatched
        // xref (caused here by the incorrect stream /Length) is reconstructed by
        // scanning objects, and the catalog resolves.
        let cursor = Cursor::new(pdf_data);
        let mut options = ParseOptions::default();
        options.lenient_streams = true;
        options.max_recovery_bytes = 1000;
        options.collect_warnings = false;
        let mut reader =
            PdfReader::new_with_options(cursor, options).expect("mismatched xref reconstructed");
        let catalog = reader
            .catalog()
            .expect("catalog resolves after reconstruction");
        assert_eq!(
            catalog.get("Type"),
            Some(&PdfObject::Name(PdfName("Catalog".to_string()))),
        );
    }

    #[test]
    fn test_parse_options_default() {
        let options = ParseOptions::default();
        assert!(!options.lenient_streams);
        assert_eq!(options.max_recovery_bytes, 1000);
        assert!(!options.collect_warnings);
    }

    #[test]
    fn test_parse_options_clone() {
        let mut options = ParseOptions::default();
        options.lenient_streams = true;
        options.max_recovery_bytes = 2000;
        options.collect_warnings = true;
        let cloned = options;
        assert!(cloned.lenient_streams);
        assert_eq!(cloned.max_recovery_bytes, 2000);
        assert!(cloned.collect_warnings);
    }

    // ===== ENCRYPTION INTEGRATION TESTS =====

    #[allow(dead_code)]
    fn create_encrypted_pdf_dict() -> PdfDictionary {
        let mut dict = PdfDictionary::new();
        dict.insert(
            "Filter".to_string(),
            PdfObject::Name(PdfName("Standard".to_string())),
        );
        dict.insert("V".to_string(), PdfObject::Integer(1));
        dict.insert("R".to_string(), PdfObject::Integer(2));
        dict.insert("O".to_string(), PdfObject::String(PdfString(vec![0u8; 32])));
        dict.insert("U".to_string(), PdfObject::String(PdfString(vec![0u8; 32])));
        dict.insert("P".to_string(), PdfObject::Integer(-4));
        dict
    }

    fn create_pdf_with_encryption() -> Vec<u8> {
        // Create a minimal PDF with encryption dictionary
        b"%PDF-1.4
1 0 obj
<< /Type /Catalog /Pages 2 0 R >>
endobj
2 0 obj
<< /Type /Pages /Kids [3 0 R] /Count 1 >>
endobj
3 0 obj
<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] >>
endobj
4 0 obj
<< /Filter /Standard /V 1 /R 2 /O (32 bytes of owner password hash data) /U (32 bytes of user password hash data) /P -4 >>
endobj
xref
0 5
0000000000 65535 f 
0000000009 00000 n 
0000000058 00000 n 
0000000116 00000 n 
0000000201 00000 n 
trailer
<< /Size 5 /Root 1 0 R /Encrypt 4 0 R /ID [(file id)] >>
startxref
295
%%EOF"
            .to_vec()
    }

    #[test]
    fn test_reader_encryption_detection() {
        // Test unencrypted PDF
        let unencrypted_pdf = create_minimal_pdf();
        let cursor = Cursor::new(unencrypted_pdf);
        let reader = PdfReader::new(cursor).unwrap();
        assert!(!reader.is_encrypted());
        assert!(reader.is_unlocked()); // Unencrypted PDFs are always "unlocked"

        // Test encrypted PDF. Its xref offsets don't match, so it goes through
        // reconstruction (Issue #374). The recovery must preserve /Encrypt so
        // the document opens as ENCRYPTED and LOCKED (fail-safe), never silently
        // as plaintext. It is not unlockable with the empty password here
        // because /O and /U are placeholders.
        let encrypted_pdf = create_pdf_with_encryption();
        let cursor = Cursor::new(encrypted_pdf);
        let reader = PdfReader::new(cursor).expect("encrypted PDF opens as locked, not rejected");
        assert!(
            reader.is_encrypted(),
            "must report encrypted, not plaintext"
        );
        assert!(
            !reader.is_unlocked(),
            "must stay locked without a valid password"
        );
    }

    #[test]
    fn test_reader_encryption_methods_unencrypted() {
        let pdf_data = create_minimal_pdf();
        let cursor = Cursor::new(pdf_data);
        let mut reader = PdfReader::new(cursor).unwrap();

        // For unencrypted PDFs, all encryption methods should work
        assert!(!reader.is_encrypted());
        assert!(reader.is_unlocked());
        assert!(reader.encryption_handler().is_none());
        assert!(reader.encryption_handler_mut().is_none());

        // Password attempts should succeed (no encryption)
        assert!(reader.unlock_with_password("any_password").unwrap());
        assert!(reader.try_empty_password().unwrap());
    }

    #[test]
    fn test_reader_encryption_handler_access() {
        let pdf_data = create_minimal_pdf();
        let cursor = Cursor::new(pdf_data);
        let mut reader = PdfReader::new(cursor).unwrap();

        // Test handler access methods
        assert!(reader.encryption_handler().is_none());
        assert!(reader.encryption_handler_mut().is_none());

        // Verify state consistency
        assert!(!reader.is_encrypted());
        assert!(reader.is_unlocked());
    }

    #[test]
    fn test_reader_multiple_password_attempts() {
        let pdf_data = create_minimal_pdf();
        let cursor = Cursor::new(pdf_data);
        let mut reader = PdfReader::new(cursor).unwrap();

        // Multiple attempts on unencrypted PDF should all succeed
        let passwords = vec!["test1", "test2", "admin", "", "password"];
        for password in passwords {
            assert!(reader.unlock_with_password(password).unwrap());
        }

        // Empty password attempts
        for _ in 0..5 {
            assert!(reader.try_empty_password().unwrap());
        }
    }

    #[test]
    fn test_reader_encryption_state_consistency() {
        let pdf_data = create_minimal_pdf();
        let cursor = Cursor::new(pdf_data);
        let mut reader = PdfReader::new(cursor).unwrap();

        // Verify initial state
        assert!(!reader.is_encrypted());
        assert!(reader.is_unlocked());
        assert!(reader.encryption_handler().is_none());

        // State should remain consistent after password attempts
        let _ = reader.unlock_with_password("test");
        assert!(!reader.is_encrypted());
        assert!(reader.is_unlocked());
        assert!(reader.encryption_handler().is_none());

        let _ = reader.try_empty_password();
        assert!(!reader.is_encrypted());
        assert!(reader.is_unlocked());
        assert!(reader.encryption_handler().is_none());
    }

    #[test]
    fn test_reader_encryption_error_handling() {
        // Fail-safe (Issue #374): an encrypted PDF whose xref must be rebuilt
        // must never be opened as plaintext. Two outcomes are safe: rejected
        // during construction, or opened while still reporting encrypted+locked.
        // Opening it as an unlocked/unencrypted document is a fail-open bug.
        let encrypted_pdf = create_pdf_with_encryption();
        let cursor = Cursor::new(encrypted_pdf);

        match PdfReader::new(cursor) {
            // Rejecting the encrypted document is safe.
            Err(_) => {}
            // Opening it is only safe if encryption is still recognized and the
            // document remains locked (no valid password was supplied).
            Ok(reader) => {
                assert!(
                    reader.is_encrypted() && !reader.is_unlocked(),
                    "recovered encrypted PDF must stay encrypted+locked, not open as plaintext"
                );
            }
        }
    }

    #[test]
    fn test_reader_encryption_with_options() {
        let pdf_data = create_minimal_pdf();
        let cursor = Cursor::new(pdf_data);

        // Test with different parsing options
        let strict_options = ParseOptions::strict();
        let strict_reader = PdfReader::new_with_options(cursor, strict_options).unwrap();
        assert!(!strict_reader.is_encrypted());
        assert!(strict_reader.is_unlocked());

        let pdf_data = create_minimal_pdf();
        let cursor = Cursor::new(pdf_data);
        let lenient_options = ParseOptions::lenient();
        let lenient_reader = PdfReader::new_with_options(cursor, lenient_options).unwrap();
        assert!(!lenient_reader.is_encrypted());
        assert!(lenient_reader.is_unlocked());
    }

    #[test]
    fn test_reader_encryption_integration_edge_cases() {
        let pdf_data = create_minimal_pdf();
        let cursor = Cursor::new(pdf_data);
        let mut reader = PdfReader::new(cursor).unwrap();

        // Test edge cases with empty/special passwords
        assert!(reader.unlock_with_password("").unwrap());
        assert!(reader.unlock_with_password("   ").unwrap()); // Spaces
        assert!(reader
            .unlock_with_password("very_long_password_that_exceeds_normal_length")
            .unwrap());
        assert!(reader.unlock_with_password("unicode_test_ñáéíóú").unwrap());

        // Special characters that might cause issues
        assert!(reader.unlock_with_password("pass@#$%^&*()").unwrap());
        assert!(reader.unlock_with_password("pass\nwith\nnewlines").unwrap());
        assert!(reader.unlock_with_password("pass\twith\ttabs").unwrap());
    }

    mod rigorous {
        use super::*;

        // =============================================================================
        // RIGOROUS TESTS FOR ERROR HANDLING
        // =============================================================================

        #[test]
        fn test_reader_invalid_pdf_header() {
            // Not a PDF at all
            let invalid_data = b"This is not a PDF file";
            let cursor = Cursor::new(invalid_data.to_vec());
            let result = PdfReader::new(cursor);

            assert!(result.is_err(), "Should fail on invalid PDF header");
        }

        #[test]
        fn test_reader_truncated_header() {
            // Truncated PDF header
            let truncated = b"%PDF";
            let cursor = Cursor::new(truncated.to_vec());
            let result = PdfReader::new(cursor);

            assert!(result.is_err(), "Should fail on truncated header");
        }

        #[test]
        fn test_reader_empty_file() {
            let empty = Vec::new();
            let cursor = Cursor::new(empty);
            let result = PdfReader::new(cursor);

            assert!(result.is_err(), "Should fail on empty file");
        }

        #[test]
        fn test_reader_malformed_version() {
            // PDF with invalid version number
            let malformed = b"%PDF-X.Y\n%%\xE2\xE3\xCF\xD3\n";
            let cursor = Cursor::new(malformed.to_vec());
            let result = PdfReader::new(cursor);

            // Should either fail or handle gracefully
            if let Ok(reader) = result {
                // If it parsed, version should have some value
                let _version = reader.version();
            }
        }

        #[test]
        fn test_reader_get_nonexistent_object() {
            let pdf_data = create_minimal_pdf();
            let cursor = Cursor::new(pdf_data);
            let mut reader = PdfReader::new(cursor).unwrap();

            // Try to get object that doesn't exist (999 0 obj)
            let result = reader.get_object(999, 0);

            assert!(result.is_err(), "Should fail when object doesn't exist");
        }

        #[test]
        fn test_reader_get_object_wrong_generation() {
            let pdf_data = create_minimal_pdf();
            let cursor = Cursor::new(pdf_data);
            let mut reader = PdfReader::new(cursor).unwrap();

            // Try to get existing object with wrong generation
            let result = reader.get_object(1, 99);

            // Should either fail or return the object with gen 0
            if let Err(e) = result {
                // Expected - wrong generation
                let _ = e;
            }
        }

        // =============================================================================
        // RIGOROUS TESTS FOR OBJECT RESOLUTION
        // =============================================================================

        #[test]
        fn test_resolve_direct_object() {
            let pdf_data = create_minimal_pdf();
            let cursor = Cursor::new(pdf_data);
            let mut reader = PdfReader::new(cursor).unwrap();

            // Create a direct object (not a reference)
            let direct_obj = PdfObject::Integer(42);

            let resolved = reader.resolve(&direct_obj).unwrap();

            // Should return the same object
            assert_eq!(resolved, &PdfObject::Integer(42));
        }

        #[test]
        fn test_resolve_reference() {
            let pdf_data = create_minimal_pdf();
            let cursor = Cursor::new(pdf_data);
            let mut reader = PdfReader::new(cursor).unwrap();

            // Get Pages reference from catalog (extract values before resolve)
            let pages_ref = {
                let catalog = reader.catalog().unwrap();
                if let Some(PdfObject::Reference(obj_num, gen_num)) = catalog.get("Pages") {
                    PdfObject::Reference(*obj_num, *gen_num)
                } else {
                    panic!("Catalog /Pages must be a Reference");
                }
            };

            // Now resolve it
            let resolved = reader.resolve(&pages_ref).unwrap();

            // Resolved object should be a dictionary with Type = Pages
            if let PdfObject::Dictionary(dict) = resolved {
                assert_eq!(
                    dict.get("Type"),
                    Some(&PdfObject::Name(PdfName("Pages".to_string())))
                );
            } else {
                panic!("Expected dictionary, got: {:?}", resolved);
            }
        }

        // =============================================================================
        // RIGOROUS TESTS FOR ENCRYPTION
        // =============================================================================

        #[test]
        fn test_is_encrypted_on_unencrypted() {
            let pdf_data = create_minimal_pdf();
            let cursor = Cursor::new(pdf_data);
            let reader = PdfReader::new(cursor).unwrap();

            assert!(
                !reader.is_encrypted(),
                "Minimal PDF should not be encrypted"
            );
        }

        #[test]
        fn test_is_unlocked_on_unencrypted() {
            let pdf_data = create_minimal_pdf();
            let cursor = Cursor::new(pdf_data);
            let reader = PdfReader::new(cursor).unwrap();

            // Unencrypted PDFs are always "unlocked"
            assert!(reader.is_unlocked(), "Unencrypted PDF should be unlocked");
        }

        #[test]
        fn test_try_empty_password_on_unencrypted() {
            let pdf_data = create_minimal_pdf();
            let cursor = Cursor::new(pdf_data);
            let mut reader = PdfReader::new(cursor).unwrap();

            // Should succeed (no encryption)
            let result = reader.try_empty_password();
            assert!(result.is_ok());
        }

        // =============================================================================
        // RIGOROUS TESTS FOR PARSE OPTIONS
        // =============================================================================

        #[test]
        fn test_reader_with_strict_options() {
            let pdf_data = create_minimal_pdf();
            let cursor = Cursor::new(pdf_data);

            let options = ParseOptions::strict();
            let result = PdfReader::new_with_options(cursor, options);

            assert!(result.is_ok(), "Minimal PDF should parse in strict mode");
        }

        #[test]
        fn test_reader_with_lenient_options() {
            let pdf_data = create_minimal_pdf();
            let cursor = Cursor::new(pdf_data);

            let options = ParseOptions::lenient();
            let result = PdfReader::new_with_options(cursor, options);

            assert!(result.is_ok(), "Minimal PDF should parse in lenient mode");
        }

        #[test]
        fn test_reader_options_accessible() {
            let pdf_data = create_minimal_pdf();
            let cursor = Cursor::new(pdf_data);

            let options = ParseOptions::lenient();
            let reader = PdfReader::new_with_options(cursor, options.clone()).unwrap();

            // Options should be accessible
            let reader_options = reader.options();
            assert_eq!(reader_options.strict_mode, options.strict_mode);
        }

        // =============================================================================
        // RIGOROUS TESTS FOR CATALOG AND INFO
        // =============================================================================

        #[test]
        fn test_catalog_has_required_fields() {
            let pdf_data = create_minimal_pdf();
            let cursor = Cursor::new(pdf_data);
            let mut reader = PdfReader::new(cursor).unwrap();

            let catalog = reader.catalog().unwrap();

            // Catalog MUST have Type = Catalog
            assert_eq!(
                catalog.get("Type"),
                Some(&PdfObject::Name(PdfName("Catalog".to_string()))),
                "Catalog must have /Type /Catalog"
            );

            // Catalog MUST have Pages
            assert!(
                catalog.contains_key("Pages"),
                "Catalog must have /Pages entry"
            );
        }

        #[test]
        fn test_info_fields_when_present() {
            let pdf_data = create_pdf_with_info();
            let cursor = Cursor::new(pdf_data);
            let mut reader = PdfReader::new(cursor).unwrap();

            let info = reader.info().unwrap();
            assert!(info.is_some(), "PDF should have Info dictionary");

            let info_dict = info.unwrap();

            // Verify specific fields exist
            assert!(info_dict.contains_key("Title"), "Info should have Title");
            assert!(info_dict.contains_key("Author"), "Info should have Author");
        }

        #[test]
        fn test_info_none_when_absent() {
            let pdf_data = create_minimal_pdf();
            let cursor = Cursor::new(pdf_data);
            let mut reader = PdfReader::new(cursor).unwrap();

            let info = reader.info().unwrap();
            assert!(info.is_none(), "Minimal PDF should not have Info");
        }

        // =============================================================================
        // RIGOROUS TESTS FOR VERSION PARSING
        // =============================================================================

        #[test]
        fn test_version_exact_values() {
            let pdf_data = create_pdf_with_version("1.7");
            let cursor = Cursor::new(pdf_data);
            let reader = PdfReader::new(cursor).unwrap();

            let version = reader.version();
            assert_eq!(version.major, 1, "Major version must be exact");
            assert_eq!(version.minor, 7, "Minor version must be exact");
        }

        #[test]
        fn test_version_pdf_20() {
            let pdf_data = create_pdf_with_version("2.0");
            let cursor = Cursor::new(pdf_data);
            let reader = PdfReader::new(cursor).unwrap();

            let version = reader.version();
            assert_eq!(version.major, 2, "PDF 2.0 major version");
            assert_eq!(version.minor, 0, "PDF 2.0 minor version");
        }

        // =============================================================================
        // RIGOROUS TESTS FOR PAGES AND PAGE_COUNT
        // =============================================================================

        #[test]
        fn test_pages_returns_pages_dict() {
            let pdf_data = create_minimal_pdf();
            let cursor = Cursor::new(pdf_data);
            let mut reader = PdfReader::new(cursor).unwrap();

            let pages_dict = reader
                .pages()
                .expect("pages() must return Pages dictionary");

            assert_eq!(
                pages_dict.get("Type"),
                Some(&PdfObject::Name(PdfName("Pages".to_string()))),
                "Pages dict must have /Type /Pages"
            );
        }

        #[test]
        fn test_page_count_minimal_pdf() {
            let pdf_data = create_minimal_pdf();
            let cursor = Cursor::new(pdf_data);
            let mut reader = PdfReader::new(cursor).unwrap();

            let count = reader.page_count().expect("page_count() must succeed");
            assert_eq!(count, 0, "Minimal PDF has 0 pages");
        }

        #[test]
        fn test_page_count_with_info_pdf() {
            let pdf_data = create_pdf_with_info();
            let cursor = Cursor::new(pdf_data);
            let mut reader = PdfReader::new(cursor).unwrap();

            let count = reader.page_count().expect("page_count() must succeed");
            assert_eq!(count, 0, "create_pdf_with_info() has Count 0 in Pages dict");
        }

        // =============================================================================
        // RIGOROUS TESTS FOR METADATA
        // =============================================================================

        #[test]
        fn test_metadata_minimal_pdf() {
            let pdf_data = create_minimal_pdf();
            let cursor = Cursor::new(pdf_data);
            let mut reader = PdfReader::new(cursor).unwrap();

            let meta = reader.metadata().expect("metadata() must succeed");

            // Minimal PDF has no metadata fields
            assert!(meta.title.is_none(), "Minimal PDF has no title");
            assert!(meta.author.is_none(), "Minimal PDF has no author");
        }

        #[test]
        fn test_metadata_with_info() {
            let pdf_data = create_pdf_with_info();
            let cursor = Cursor::new(pdf_data);
            let mut reader = PdfReader::new(cursor).unwrap();

            let meta = reader.metadata().expect("metadata() must succeed");

            assert!(meta.title.is_some(), "PDF with Info has title");
            assert_eq!(meta.title.unwrap(), "Test PDF", "Title must match");
            assert!(meta.author.is_some(), "PDF with Info has author");
            assert_eq!(meta.author.unwrap(), "Test Author", "Author must match");
        }

        // =============================================================================
        // RIGOROUS TESTS FOR RESOLVE_STREAM_LENGTH
        // =============================================================================

        #[test]
        fn test_resolve_stream_length_direct_integer() {
            let pdf_data = create_minimal_pdf();
            let cursor = Cursor::new(pdf_data);
            let mut reader = PdfReader::new(cursor).unwrap();

            // Pass a direct integer (Length value)
            let length_obj = PdfObject::Integer(100);

            let length = reader
                .resolve_stream_length(&length_obj)
                .expect("resolve_stream_length must succeed");
            assert_eq!(length, Some(100), "Direct integer must be resolved");
        }

        #[test]
        fn test_resolve_stream_length_negative_integer() {
            let pdf_data = create_minimal_pdf();
            let cursor = Cursor::new(pdf_data);
            let mut reader = PdfReader::new(cursor).unwrap();

            // Negative length is invalid
            let length_obj = PdfObject::Integer(-10);

            let length = reader
                .resolve_stream_length(&length_obj)
                .expect("resolve_stream_length must succeed");
            assert_eq!(length, None, "Negative integer returns None");
        }

        #[test]
        fn test_resolve_stream_length_non_integer() {
            let pdf_data = create_minimal_pdf();
            let cursor = Cursor::new(pdf_data);
            let mut reader = PdfReader::new(cursor).unwrap();

            // Pass a non-integer object
            let name_obj = PdfObject::Name(PdfName("Test".to_string()));

            let length = reader
                .resolve_stream_length(&name_obj)
                .expect("resolve_stream_length must succeed");
            assert_eq!(length, None, "Non-integer object returns None");
        }

        // =============================================================================
        // RIGOROUS TESTS FOR GET_ALL_PAGES
        // =============================================================================

        #[test]
        fn test_get_all_pages_empty_pdf() {
            let pdf_data = create_minimal_pdf();
            let cursor = Cursor::new(pdf_data);
            let mut reader = PdfReader::new(cursor).unwrap();

            let pages = reader
                .get_all_pages()
                .expect("get_all_pages() must succeed");
            assert_eq!(pages.len(), 0, "Minimal PDF has 0 pages");
        }

        #[test]
        fn test_get_all_pages_with_info() {
            let pdf_data = create_pdf_with_info();
            let cursor = Cursor::new(pdf_data);
            let mut reader = PdfReader::new(cursor).unwrap();

            let pages = reader
                .get_all_pages()
                .expect("get_all_pages() must succeed");
            assert_eq!(
                pages.len(),
                0,
                "create_pdf_with_info() has 0 pages (Count 0)"
            );
        }

        // =============================================================================
        // RIGOROUS TESTS FOR INTO_DOCUMENT
        // =============================================================================

        #[test]
        fn test_into_document_consumes_reader() {
            let pdf_data = create_minimal_pdf();
            let cursor = Cursor::new(pdf_data);
            let reader = PdfReader::new(cursor).unwrap();

            let document = reader.into_document();

            // Verify document has valid version
            let version = document.version().expect("Document must have version");
            assert!(
                version.starts_with("1."),
                "Document must have PDF 1.x version, got: {}",
                version
            );

            // Verify document can access page count
            let page_count = document
                .page_count()
                .expect("Document must allow page_count()");
            assert_eq!(
                page_count, 0,
                "Minimal PDF has 0 pages (Count 0 in test helper)"
            );
        }

        // =============================================================================
        // RIGOROUS TESTS FOR PARSE_CONTEXT
        // =============================================================================

        #[test]
        fn test_clear_parse_context() {
            let pdf_data = create_minimal_pdf();
            let cursor = Cursor::new(pdf_data);
            let mut reader = PdfReader::new(cursor).unwrap();

            // Clear parse context (should not panic)
            reader.clear_parse_context();

            // Verify reader still works after clearing
            let version = reader.version();
            assert_eq!(version.major, 1, "Reader must still work after clear");
        }

        #[test]
        fn test_parse_context_mut_accessible() {
            let pdf_data = create_minimal_pdf();
            let cursor = Cursor::new(pdf_data);
            let mut reader = PdfReader::new(cursor).unwrap();

            let context = reader.parse_context_mut();

            // Verify context has expected structure
            let initial_depth = context.depth;
            assert_eq!(initial_depth, 0, "Parse context must start with depth 0");

            // Verify max_depth is set to reasonable value
            assert!(
                context.max_depth > 0,
                "Parse context must have positive max_depth"
            );
        }

        // =============================================================================
        // RIGOROUS TESTS FOR UTILITY FUNCTIONS
        // =============================================================================

        #[test]
        fn test_find_byte_pattern_basic() {
            let haystack = b"Hello World";
            let needle = b"World";
            let pos = find_byte_pattern(haystack, needle);
            assert_eq!(pos, Some(6), "Must find 'World' at position 6");
        }

        #[test]
        fn test_find_byte_pattern_not_found() {
            let haystack = b"Hello World";
            let needle = b"Rust";
            let pos = find_byte_pattern(haystack, needle);
            assert_eq!(pos, None, "Must return None when not found");
        }

        #[test]
        fn test_find_byte_pattern_at_start() {
            let haystack = b"Hello World";
            let needle = b"Hello";
            let pos = find_byte_pattern(haystack, needle);
            assert_eq!(pos, Some(0), "Must find at position 0");
        }
    }
}
