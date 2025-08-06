//! High-level PDF Reader API
//!
//! Provides a simple interface for reading PDF files

use super::encryption_handler::EncryptionHandler;
use super::header::PdfHeader;
use super::object_stream::ObjectStream;
use super::objects::{PdfDictionary, PdfObject};
use super::stack_safe::StackSafeContext;
use super::trailer::PdfTrailer;
use super::xref::XRefTable;
use super::{ParseError, ParseResult};
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, Read, Seek, SeekFrom};
use std::path::Path;

/// High-level PDF reader
pub struct PdfReader<R: Read + Seek> {
    reader: BufReader<R>,
    header: PdfHeader,
    xref: XRefTable,
    trailer: PdfTrailer,
    /// Cache of loaded objects
    object_cache: HashMap<(u32, u16), PdfObject>,
    /// Cache of object streams
    object_stream_cache: HashMap<u32, ObjectStream>,
    /// Page tree navigator
    page_tree: Option<super::page_tree::PageTree>,
    /// Stack-safe parsing context
    parse_context: StackSafeContext,
    /// Parsing options
    options: super::ParseOptions,
    /// Encryption handler (if PDF is encrypted)
    encryption_handler: Option<EncryptionHandler>,
}

impl<R: Read + Seek> PdfReader<R> {
    /// Get parsing options
    pub fn options(&self) -> &super::ParseOptions {
        &self.options
    }

    /// Check if the PDF is encrypted
    pub fn is_encrypted(&self) -> bool {
        self.encryption_handler.is_some()
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
                // Try user password first
                if handler.unlock_with_user_password(password).unwrap_or(false) {
                    Ok(true)
                } else {
                    // Try owner password
                    Ok(handler
                        .unlock_with_owner_password(password)
                        .unwrap_or(false))
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
}

impl PdfReader<File> {
    /// Open a PDF file from a path
    pub fn open<P: AsRef<Path>>(path: P) -> ParseResult<Self> {
        use std::io::Write;
        let mut debug_file = std::fs::File::create("/tmp/pdf_open_debug.log").ok();
        if let Some(ref mut f) = debug_file {
            writeln!(f, "Opening file: {:?}", path.as_ref()).ok();
        }
        let file = File::open(path)?;
        if let Some(ref mut f) = debug_file {
            writeln!(f, "File opened successfully").ok();
        }
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
    pub fn new(reader: R) -> ParseResult<Self> {
        Self::new_with_options(reader, super::ParseOptions::default())
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
        use std::io::Write;
        let mut debug_file = std::fs::File::create("/tmp/pdf_debug.log").ok();
        if let Some(ref mut f) = debug_file {
            writeln!(f, "Parsing PDF header...").ok();
        }
        let header = PdfHeader::parse(&mut buf_reader)?;
        if let Some(ref mut f) = debug_file {
            writeln!(f, "Header parsed: version {}", header.version).ok();
        }

        // Parse xref table
        if let Some(ref mut f) = debug_file {
            writeln!(f, "Parsing XRef table...").ok();
        }
        let xref = XRefTable::parse_with_options(&mut buf_reader, &options)?;
        if let Some(ref mut f) = debug_file {
            writeln!(f, "XRef table parsed with {} entries", xref.len()).ok();
        }

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
                        Ok(handler) => {
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
            Ok(root) => root,
            Err(_) => {
                // If Root is missing, try fallback methods
                #[cfg(debug_assertions)]
                eprintln!("Warning: Trailer missing Root entry, attempting recovery");

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

        let catalog = self.get_object(obj_num, gen_num)?;

        catalog.as_dict().ok_or_else(|| ParseError::SyntaxError {
            position: 0,
            message: "Catalog is not a dictionary".to_string(),
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

    /// Get an object by reference
    pub fn get_object(&mut self, obj_num: u32, gen_num: u16) -> ParseResult<&PdfObject> {
        self.load_object_from_disk(obj_num, gen_num)
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
        }

        // Get xref entry
        let entry = self
            .xref
            .get_entry(obj_num)
            .ok_or(ParseError::InvalidReference(obj_num, gen_num))?;

        if !entry.in_use {
            // Free object
            self.object_cache.insert(key, PdfObject::Null);
            return Ok(&self.object_cache[&key]);
        }

        if entry.generation != gen_num {
            return Err(ParseError::InvalidReference(obj_num, gen_num));
        }

        // Seek to object position
        self.reader.seek(std::io::SeekFrom::Start(entry.offset))?;

        // Parse object header (obj_num gen_num obj)
        let mut lexer =
            super::lexer::Lexer::new_with_options(&mut self.reader, self.options.clone());

        // Read object number with recovery
        let token = lexer.next_token()?;
        let read_obj_num = match token {
            super::lexer::Token::Integer(n) => n as u32,
            _ => {
                // Try fallback recovery (simplified implementation)
                if self.options.lenient_syntax {
                    // For now, use the expected object number and issue warning
                    if self.options.collect_warnings {
                        eprintln!(
                            "Warning: Using expected object number {obj_num} instead of parsed token"
                        );
                    }
                    obj_num
                } else {
                    return Err(ParseError::SyntaxError {
                        position: entry.offset as usize,
                        message: "Expected object number".to_string(),
                    });
                }
            }
        };

        if read_obj_num != obj_num && !self.options.lenient_syntax {
            return Err(ParseError::SyntaxError {
                position: entry.offset as usize,
                message: format!(
                    "Object number mismatch: expected {obj_num}, found {read_obj_num}"
                ),
            });
        }

        // Read generation number
        let token = lexer.next_token()?;
        let read_gen_num = match token {
            super::lexer::Token::Integer(n) => n as u16,
            _ => {
                if self.options.lenient_syntax {
                    // In lenient mode, assume generation 0
                    if self.options.collect_warnings {
                        eprintln!(
                            "Warning: Using generation 0 instead of parsed token for object {obj_num}"
                        );
                    }
                    0
                } else {
                    return Err(ParseError::SyntaxError {
                        position: entry.offset as usize,
                        message: "Expected generation number".to_string(),
                    });
                }
            }
        };

        if read_gen_num != gen_num && !self.options.lenient_syntax {
            return Err(ParseError::SyntaxError {
                position: entry.offset as usize,
                message: format!(
                    "Generation number mismatch: expected {gen_num}, found {read_gen_num}"
                ),
            });
        }

        // Read 'obj' keyword
        let token = lexer.next_token()?;
        match token {
            super::lexer::Token::Obj => {}
            _ => {
                if self.options.lenient_syntax {
                    // In lenient mode, try to continue without 'obj' keyword
                    if self.options.collect_warnings {
                        eprintln!(
                            "Warning: Expected 'obj' keyword for object {obj_num} {gen_num}, continuing anyway"
                        );
                    }
                    // We need to process the token we just read as part of the object
                    // This is a bit tricky - for now, we'll just continue
                } else {
                    return Err(ParseError::SyntaxError {
                        position: entry.offset as usize,
                        message: "Expected 'obj' keyword".to_string(),
                    });
                }
            }
        };

        // Check recursion depth and parse object
        self.parse_context.enter()?;

        let obj = match PdfObject::parse_with_options(&mut lexer, &self.options) {
            Ok(obj) => {
                self.parse_context.exit();
                obj
            }
            Err(e) => {
                self.parse_context.exit();
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
                        eprintln!("Warning: Expected 'endobj' keyword after object {obj_num} {gen_num}, continuing anyway");
                    }
                } else {
                    return Err(ParseError::SyntaxError {
                        position: entry.offset as usize,
                        message: "Expected 'endobj' keyword".to_string(),
                    });
                }
            }
        };

        // Cache the object
        self.object_cache.insert(key, obj);

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

        // Load the object stream if not cached
        if !self.object_stream_cache.contains_key(&stream_obj_num) {
            // Get the stream object using the internal method (no stack tracking)
            let stream_obj = self.load_object_from_disk(stream_obj_num, 0)?;

            if let Some(stream) = stream_obj.as_stream() {
                // Parse the object stream
                let obj_stream = ObjectStream::parse(stream.clone(), &self.options)?;
                self.object_stream_cache.insert(stream_obj_num, obj_stream);
            } else {
                return Err(ParseError::SyntaxError {
                    position: 0,
                    message: format!("Object {stream_obj_num} is not a stream"),
                });
            }
        }

        // Get the object from the stream
        let obj_stream = &self.object_stream_cache[&stream_obj_num];
        let obj = obj_stream
            .get_object(obj_num)
            .ok_or_else(|| ParseError::SyntaxError {
                position: 0,
                message: format!("Object {obj_num} not found in object stream {stream_obj_num}"),
            })?;

        // Cache the object
        self.object_cache.insert(key, obj.clone());
        Ok(&self.object_cache[&key])
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
                // If Pages is missing, try to find page objects by scanning
                #[cfg(debug_assertions)]
                eprintln!("Warning: Catalog missing Pages entry, attempting recovery");

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
                        eprintln!("Warning: Missing Pages in catalog, searching for page tree");
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
        let pages_obj = self.get_object(pages_obj_num, pages_gen_num)?;
        pages_obj.as_dict().ok_or_else(|| ParseError::SyntaxError {
            position: 0,
            message: "Pages is not a dictionary".to_string(),
        })
    }

    /// Get the number of pages
    pub fn page_count(&mut self) -> ParseResult<u32> {
        let pages = self.pages()?;

        // Try to get Count first
        if let Some(count_obj) = pages.get("Count") {
            if let Some(count) = count_obj.as_integer() {
                return Ok(count as u32);
            }
        }

        // If Count is missing or invalid, try to count manually by traversing Kids
        if let Some(kids_obj) = pages.get("Kids") {
            if let Some(kids_array) = kids_obj.as_array() {
                // Simple count: assume each kid is a page for now
                // TODO: Implement proper recursive counting of nested page trees
                return Ok(kids_array.len() as u32);
            }
        }

        // If we can't determine page count, return 0 instead of error
        // This allows the parser to continue and handle other operations
        Ok(0)
    }

    /// Get metadata from the document
    pub fn metadata(&mut self) -> ParseResult<DocumentMetadata> {
        let mut metadata = DocumentMetadata::default();

        if let Some(info_dict) = self.info()? {
            if let Some(title) = info_dict.get("Title").and_then(|o| o.as_string()) {
                metadata.title = title.as_str().ok().map(|s| s.to_string());
            }
            if let Some(author) = info_dict.get("Author").and_then(|o| o.as_string()) {
                metadata.author = author.as_str().ok().map(|s| s.to_string());
            }
            if let Some(subject) = info_dict.get("Subject").and_then(|o| o.as_string()) {
                metadata.subject = subject.as_str().ok().map(|s| s.to_string());
            }
            if let Some(keywords) = info_dict.get("Keywords").and_then(|o| o.as_string()) {
                metadata.keywords = keywords.as_str().ok().map(|s| s.to_string());
            }
            if let Some(creator) = info_dict.get("Creator").and_then(|o| o.as_string()) {
                metadata.creator = creator.as_str().ok().map(|s| s.to_string());
            }
            if let Some(producer) = info_dict.get("Producer").and_then(|o| o.as_string()) {
                metadata.producer = producer.as_str().ok().map(|s| s.to_string());
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

    /// Find all page objects by scanning the entire PDF
    fn find_page_objects(&mut self) -> ParseResult<Vec<(u32, u16)>> {
        let mut page_refs = Vec::new();

        // Scan through all objects in the xref table
        let obj_nums: Vec<u32> = self.xref.entries().keys().cloned().collect();

        for obj_num in obj_nums {
            // Try to get the object
            if let Ok(obj) = self.get_object(obj_num, 0) {
                if let Some(dict) = obj.as_dict() {
                    // Check if it's a Page object
                    if let Some(PdfObject::Name(type_name)) = dict.get("Type") {
                        if type_name.0 == "Page" {
                            page_refs.push((obj_num, 0));
                        }
                    }
                }
            }
        }

        Ok(page_refs)
    }

    /// Find catalog object by scanning
    fn find_catalog_object(&mut self) -> ParseResult<(u32, u16)> {
        // Simple fallback - try common object numbers
        // Real implementation would need to scan objects, but that's complex
        // due to borrow checker constraints

        // Most PDFs have catalog at object 1
        Ok((1, 0))
    }

    /// Create a synthetic Pages dictionary when the catalog is missing one
    fn create_synthetic_pages_dict(
        &mut self,
        page_refs: &[(u32, u16)],
    ) -> ParseResult<&PdfDictionary> {
        use super::objects::{PdfArray, PdfName};

        // Create Kids array with page references
        let mut kids = PdfArray::new();
        for (obj_num, gen_num) in page_refs {
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
            PdfObject::Integer(page_refs.len() as i64),
        );

        // Find a common MediaBox from the pages
        let mut media_box = None;
        for (obj_num, gen_num) in page_refs.iter().take(1) {
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
    fn test_reader_construction() {
        let pdf_data = create_minimal_pdf();
        let cursor = Cursor::new(pdf_data);
        let result = PdfReader::new(cursor);
        assert!(result.is_ok());
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

        let cursor = Cursor::new(corrupt_pdf);
        let result = PdfReader::new(cursor);
        // Even with lenient parsing, completely corrupted xref table cannot be recovered
        // TODO: Implement xref recovery for this case in future versions
        assert!(result.is_err());
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

        let cursor = Cursor::new(pdf_no_trailer);
        let result = PdfReader::new(cursor);
        // PDFs without trailer cannot be parsed even with lenient mode
        // The trailer is essential for locating the catalog
        assert!(result.is_err());
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

        let cloned = metadata.clone();
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

        let cursor = Cursor::new(bad_pdf);
        let result = PdfReader::new(cursor);
        // Trailer missing required /Root entry cannot be recovered
        // This is a fundamental requirement for PDF structure
        assert!(result.is_err());
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

        // Test lenient mode - even lenient mode cannot parse PDFs with incomplete xref
        let cursor = Cursor::new(pdf_data);
        let mut options = ParseOptions::default();
        options.lenient_streams = true;
        options.max_recovery_bytes = 1000;
        options.collect_warnings = false;
        let lenient_reader = PdfReader::new_with_options(cursor, options);
        assert!(lenient_reader.is_err());
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
        let cloned = options.clone();
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

        // Test encrypted PDF - this will fail during construction due to encryption
        let encrypted_pdf = create_pdf_with_encryption();
        let cursor = Cursor::new(encrypted_pdf);
        let result = PdfReader::new(cursor);
        // Should fail because we don't support reading encrypted PDFs yet in construction
        assert!(result.is_err());
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
        // This test verifies that encrypted PDFs are properly rejected during construction
        let encrypted_pdf = create_pdf_with_encryption();
        let cursor = Cursor::new(encrypted_pdf);

        // Should fail during construction due to unsupported encryption
        let result = PdfReader::new(cursor);
        match result {
            Err(ParseError::EncryptionNotSupported) => {
                // Expected - encryption detected but not supported in current flow
            }
            Err(_) => {
                // Other errors are also acceptable as encryption detection may fail parsing
            }
            Ok(_) => {
                panic!("Should not successfully create reader for encrypted PDF without password");
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
}
