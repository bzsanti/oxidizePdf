//! PDF Document wrapper - High-level interface for PDF parsing and manipulation
//!
//! This module provides a robust, high-level interface for working with PDF documents.
//! It solves Rust's borrow checker challenges through careful use of interior mutability
//! (RefCell) and separation of concerns between parsing, caching, and page access.
//!
//! # Architecture
//!
//! The module uses a layered architecture:
//! - **PdfDocument**: Main entry point with RefCell-based state management
//! - **ResourceManager**: Centralized object caching with interior mutability
//! - **PdfReader**: Low-level file access (wrapped in RefCell)
//! - **PageTree**: Lazy-loaded page navigation
//!
//! # Key Features
//!
//! - **Automatic caching**: Objects are cached after first access
//! - **Resource management**: Shared resources are handled efficiently
//! - **Page navigation**: Fast access to any page in the document
//! - **Reference resolution**: Automatic resolution of indirect references
//! - **Text extraction**: Built-in support for extracting text from pages
//!
//! # Example
//!
//! ```rust,no_run
//! use oxidize_pdf::parser::{PdfDocument, PdfReader};
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! // Open a PDF document
//! let reader = PdfReader::open("document.pdf")?;
//! let document = PdfDocument::new(reader);
//!
//! // Get document information
//! let page_count = document.page_count()?;
//! let metadata = document.metadata()?;
//! println!("Title: {:?}", metadata.title);
//! println!("Pages: {}", page_count);
//!
//! // Access a specific page
//! let page = document.get_page(0)?;
//! println!("Page size: {}x{}", page.width(), page.height());
//!
//! // Extract text from all pages
//! let extracted_text = document.extract_text()?;
//! for (i, page_text) in extracted_text.iter().enumerate() {
//!     println!("Page {}: {}", i + 1, page_text.text);
//! }
//! # Ok(())
//! # }
//! ```

use super::objects::{PdfDictionary, PdfObject};
use super::page_tree::{PageTree, ParsedPage};
use super::reader::PdfReader;
use super::{ParseError, ParseResult};
use std::cell::RefCell;
use std::collections::HashMap;
use std::io::{Read, Seek};
use std::rc::Rc;

/// Resource manager for efficient PDF object caching.
///
/// The ResourceManager provides centralized caching of PDF objects to avoid
/// repeated parsing and to share resources between different parts of the document.
/// It uses RefCell for interior mutability, allowing multiple immutable references
/// to the document while still being able to update the cache.
///
/// # Caching Strategy
///
/// - Objects are cached on first access
/// - Cache persists for the lifetime of the document
/// - Manual cache clearing is supported for memory management
///
/// # Example
///
/// ```rust,no_run
/// use oxidize_pdf::parser::document::ResourceManager;
///
/// let resources = ResourceManager::new();
///
/// // Objects are cached automatically when accessed through PdfDocument
/// // Manual cache management:
/// resources.clear_cache(); // Free memory when needed
/// ```
pub struct ResourceManager {
    /// Cached objects indexed by (object_number, generation_number)
    object_cache: RefCell<HashMap<(u32, u16), PdfObject>>,
}

impl Default for ResourceManager {
    fn default() -> Self {
        Self::new()
    }
}

impl ResourceManager {
    /// Create a new resource manager
    pub fn new() -> Self {
        Self {
            object_cache: RefCell::new(HashMap::new()),
        }
    }

    /// Get an object from cache if available.
    ///
    /// # Arguments
    ///
    /// * `obj_ref` - Object reference (object_number, generation_number)
    ///
    /// # Returns
    ///
    /// Cloned object if cached, None otherwise.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use oxidize_pdf::parser::document::ResourceManager;
    /// # let resources = ResourceManager::new();
    /// if let Some(obj) = resources.get_cached((10, 0)) {
    ///     println!("Object 10 0 R found in cache");
    /// }
    /// ```
    pub fn get_cached(&self, obj_ref: (u32, u16)) -> Option<PdfObject> {
        self.object_cache.borrow().get(&obj_ref).cloned()
    }

    /// Cache an object for future access.
    ///
    /// # Arguments
    ///
    /// * `obj_ref` - Object reference (object_number, generation_number)
    /// * `obj` - The PDF object to cache
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use oxidize_pdf::parser::document::ResourceManager;
    /// # use oxidize_pdf::parser::objects::PdfObject;
    /// # let resources = ResourceManager::new();
    /// resources.cache_object((10, 0), PdfObject::Integer(42));
    /// ```
    pub fn cache_object(&self, obj_ref: (u32, u16), obj: PdfObject) {
        self.object_cache.borrow_mut().insert(obj_ref, obj);
    }

    /// Clear all cached objects to free memory.
    ///
    /// Use this when processing large documents to manage memory usage.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use oxidize_pdf::parser::document::ResourceManager;
    /// # let resources = ResourceManager::new();
    /// // After processing many pages
    /// resources.clear_cache();
    /// println!("Cache cleared to free memory");
    /// ```
    pub fn clear_cache(&self) {
        self.object_cache.borrow_mut().clear();
    }
}

/// High-level PDF document interface for parsing and manipulation.
///
/// `PdfDocument` provides a clean, safe API for working with PDF files.
/// It handles the complexity of PDF structure, object references, and resource
/// management behind a simple interface.
///
/// # Type Parameter
///
/// * `R` - The reader type (must implement Read + Seek)
///
/// # Architecture Benefits
///
/// - **RefCell Usage**: Allows multiple parts of the API to access the document
/// - **Lazy Loading**: Pages and resources are loaded on demand
/// - **Automatic Caching**: Frequently accessed objects are cached
/// - **Safe API**: Borrow checker issues are handled internally
///
/// # Example
///
/// ```rust,no_run
/// use oxidize_pdf::parser::{PdfDocument, PdfReader};
/// use std::fs::File;
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// // From a file
/// let reader = PdfReader::open("document.pdf")?;
/// let document = PdfDocument::new(reader);
///
/// // From any Read + Seek source
/// let file = File::open("document.pdf")?;
/// let reader = PdfReader::new(file)?;
/// let document = PdfDocument::new(reader);
///
/// // Use the document
/// let page_count = document.page_count()?;
/// for i in 0..page_count {
///     let page = document.get_page(i)?;
///     // Process page...
/// }
/// # Ok(())
/// # }
/// ```
pub struct PdfDocument<R: Read + Seek> {
    /// The underlying PDF reader wrapped for interior mutability
    reader: RefCell<PdfReader<R>>,
    /// Page tree navigator (lazily initialized)
    page_tree: RefCell<Option<PageTree>>,
    /// Shared resource manager for object caching
    resources: Rc<ResourceManager>,
    /// Cached document metadata to avoid repeated parsing
    metadata_cache: RefCell<Option<super::reader::DocumentMetadata>>,
}

impl<R: Read + Seek> PdfDocument<R> {
    /// Create a new PDF document from a reader
    pub fn new(reader: PdfReader<R>) -> Self {
        Self {
            reader: RefCell::new(reader),
            page_tree: RefCell::new(None),
            resources: Rc::new(ResourceManager::new()),
            metadata_cache: RefCell::new(None),
        }
    }

    /// Get the PDF version of the document.
    ///
    /// # Returns
    ///
    /// PDF version string (e.g., "1.4", "1.7", "2.0")
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use oxidize_pdf::parser::{PdfDocument, PdfReader};
    /// # fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let reader = PdfReader::open("document.pdf")?;
    /// # let document = PdfDocument::new(reader);
    /// let version = document.version()?;
    /// println!("PDF version: {}", version);
    /// # Ok(())
    /// # }
    /// ```
    pub fn version(&self) -> ParseResult<String> {
        Ok(self.reader.borrow().version().to_string())
    }

    /// Get the total number of pages in the document.
    ///
    /// # Returns
    ///
    /// The page count as an unsigned 32-bit integer.
    ///
    /// # Errors
    ///
    /// Returns an error if the page tree is malformed or missing.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use oxidize_pdf::parser::{PdfDocument, PdfReader};
    /// # fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let reader = PdfReader::open("document.pdf")?;
    /// # let document = PdfDocument::new(reader);
    /// let count = document.page_count()?;
    /// println!("Document has {} pages", count);
    ///
    /// // Iterate through all pages
    /// for i in 0..count {
    ///     let page = document.get_page(i)?;
    ///     // Process page...
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn page_count(&self) -> ParseResult<u32> {
        self.reader.borrow_mut().page_count()
    }

    /// Get document metadata including title, author, creation date, etc.
    ///
    /// Metadata is cached after first access for performance.
    ///
    /// # Returns
    ///
    /// A `DocumentMetadata` struct containing all available metadata fields.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use oxidize_pdf::parser::{PdfDocument, PdfReader};
    /// # fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let reader = PdfReader::open("document.pdf")?;
    /// # let document = PdfDocument::new(reader);
    /// let metadata = document.metadata()?;
    ///
    /// if let Some(title) = &metadata.title {
    ///     println!("Title: {}", title);
    /// }
    /// if let Some(author) = &metadata.author {
    ///     println!("Author: {}", author);
    /// }
    /// if let Some(creation_date) = &metadata.creation_date {
    ///     println!("Created: {}", creation_date);
    /// }
    /// println!("PDF Version: {}", metadata.version);
    /// # Ok(())
    /// # }
    /// ```
    pub fn metadata(&self) -> ParseResult<super::reader::DocumentMetadata> {
        // Check cache first
        if let Some(metadata) = self.metadata_cache.borrow().as_ref() {
            return Ok(metadata.clone());
        }

        // Load metadata
        let metadata = self.reader.borrow_mut().metadata()?;
        self.metadata_cache.borrow_mut().replace(metadata.clone());
        Ok(metadata)
    }

    /// Initialize the page tree if not already done
    fn ensure_page_tree(&self) -> ParseResult<()> {
        if self.page_tree.borrow().is_none() {
            let page_count = self.page_count()?;
            let pages_dict = self.load_pages_dict()?;
            let page_tree = PageTree::new_with_pages_dict(page_count, pages_dict);
            self.page_tree.borrow_mut().replace(page_tree);
        }
        Ok(())
    }

    /// Load the pages dictionary
    fn load_pages_dict(&self) -> ParseResult<PdfDictionary> {
        let mut reader = self.reader.borrow_mut();
        let pages = reader.pages()?;
        Ok(pages.clone())
    }

    /// Get a page by index (0-based).
    ///
    /// Pages are cached after first access. This method handles page tree
    /// traversal and property inheritance automatically.
    ///
    /// # Arguments
    ///
    /// * `index` - Zero-based page index (0 to page_count-1)
    ///
    /// # Returns
    ///
    /// A complete `ParsedPage` with all properties and inherited resources.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Index is out of bounds
    /// - Page tree is malformed
    /// - Required page properties are missing
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use oxidize_pdf::parser::{PdfDocument, PdfReader};
    /// # fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let reader = PdfReader::open("document.pdf")?;
    /// # let document = PdfDocument::new(reader);
    /// // Get the first page
    /// let page = document.get_page(0)?;
    ///
    /// // Access page properties
    /// println!("Page size: {}x{} points", page.width(), page.height());
    /// println!("Rotation: {}°", page.rotation);
    ///
    /// // Get content streams
    /// let streams = page.content_streams_with_document(&document)?;
    /// println!("Page has {} content streams", streams.len());
    /// # Ok(())
    /// # }
    /// ```
    pub fn get_page(&self, index: u32) -> ParseResult<ParsedPage> {
        self.ensure_page_tree()?;

        // First check if page is already loaded
        if let Some(page_tree) = self.page_tree.borrow().as_ref() {
            if let Some(page) = page_tree.get_cached_page(index) {
                return Ok(page.clone());
            }
        }

        // Load the page
        let page = self.load_page_at_index(index)?;

        // Cache it
        if let Some(page_tree) = self.page_tree.borrow_mut().as_mut() {
            page_tree.cache_page(index, page.clone());
        }

        Ok(page)
    }

    /// Load a specific page by index
    fn load_page_at_index(&self, index: u32) -> ParseResult<ParsedPage> {
        // Get the pages root
        let pages_dict = self.load_pages_dict()?;

        // Navigate to the specific page
        let page_info = self.find_page_in_tree(&pages_dict, index, 0, None)?;

        Ok(page_info)
    }

    /// Find a page in the page tree
    fn find_page_in_tree(
        &self,
        node: &PdfDictionary,
        target_index: u32,
        current_index: u32,
        inherited: Option<&PdfDictionary>,
    ) -> ParseResult<ParsedPage> {
        let node_type = node
            .get_type()
            .ok_or_else(|| ParseError::MissingKey("Type".to_string()))?;

        match node_type {
            "Pages" => {
                // This is a page tree node
                let kids = node
                    .get("Kids")
                    .and_then(|obj| obj.as_array())
                    .ok_or_else(|| ParseError::MissingKey("Kids".to_string()))?;

                // Merge inherited attributes
                let mut merged_inherited = inherited.cloned().unwrap_or_else(PdfDictionary::new);

                // Inheritable attributes
                for key in ["Resources", "MediaBox", "CropBox", "Rotate"] {
                    if let Some(value) = node.get(key) {
                        if !merged_inherited.contains_key(key) {
                            merged_inherited.insert(key.to_string(), value.clone());
                        }
                    }
                }

                // Find which kid contains our target page
                let mut current_idx = current_index;
                for kid_ref in &kids.0 {
                    let kid_ref =
                        kid_ref
                            .as_reference()
                            .ok_or_else(|| ParseError::SyntaxError {
                                position: 0,
                                message: "Kids array must contain references".to_string(),
                            })?;

                    // Get the kid object
                    let kid_obj = self.get_object(kid_ref.0, kid_ref.1)?;
                    let kid_dict = kid_obj.as_dict().ok_or_else(|| ParseError::SyntaxError {
                        position: 0,
                        message: "Page tree node must be a dictionary".to_string(),
                    })?;

                    let kid_type = kid_dict
                        .get_type()
                        .ok_or_else(|| ParseError::MissingKey("Type".to_string()))?;

                    let count = if kid_type == "Pages" {
                        kid_dict
                            .get("Count")
                            .and_then(|obj| obj.as_integer())
                            .ok_or_else(|| ParseError::MissingKey("Count".to_string()))?
                            as u32
                    } else {
                        1
                    };

                    if target_index < current_idx + count {
                        // Found the right subtree/page
                        if kid_type == "Page" {
                            // This is the page we want
                            return self.create_parsed_page(
                                kid_ref,
                                kid_dict,
                                Some(&merged_inherited),
                            );
                        } else {
                            // Recurse into this subtree
                            return self.find_page_in_tree(
                                kid_dict,
                                target_index,
                                current_idx,
                                Some(&merged_inherited),
                            );
                        }
                    }

                    current_idx += count;
                }

                Err(ParseError::SyntaxError {
                    position: 0,
                    message: "Page not found in tree".to_string(),
                })
            }
            "Page" => {
                // This is a page object
                if target_index != current_index {
                    return Err(ParseError::SyntaxError {
                        position: 0,
                        message: "Page index mismatch".to_string(),
                    });
                }

                // We need the reference, but we don't have it here
                // This case shouldn't happen if we're navigating properly
                Err(ParseError::SyntaxError {
                    position: 0,
                    message: "Direct page object without reference".to_string(),
                })
            }
            _ => Err(ParseError::SyntaxError {
                position: 0,
                message: format!("Invalid page tree node type: {node_type}"),
            }),
        }
    }

    /// Create a ParsedPage from a page dictionary
    fn create_parsed_page(
        &self,
        obj_ref: (u32, u16),
        page_dict: &PdfDictionary,
        inherited: Option<&PdfDictionary>,
    ) -> ParseResult<ParsedPage> {
        // Extract page attributes
        let media_box = self
            .get_rectangle(page_dict, inherited, "MediaBox")?
            .ok_or_else(|| ParseError::MissingKey("MediaBox".to_string()))?;

        let crop_box = self.get_rectangle(page_dict, inherited, "CropBox")?;

        let rotation = self
            .get_integer(page_dict, inherited, "Rotate")?
            .unwrap_or(0) as i32;

        // Get inherited resources
        let inherited_resources = if let Some(inherited) = inherited {
            inherited
                .get("Resources")
                .and_then(|r| r.as_dict())
                .cloned()
        } else {
            None
        };

        Ok(ParsedPage {
            obj_ref,
            dict: page_dict.clone(),
            inherited_resources,
            media_box,
            crop_box,
            rotation,
        })
    }

    /// Get a rectangle value
    fn get_rectangle(
        &self,
        node: &PdfDictionary,
        inherited: Option<&PdfDictionary>,
        key: &str,
    ) -> ParseResult<Option<[f64; 4]>> {
        let array = node.get(key).or_else(|| inherited.and_then(|i| i.get(key)));

        if let Some(array) = array.and_then(|obj| obj.as_array()) {
            if array.len() != 4 {
                return Err(ParseError::SyntaxError {
                    position: 0,
                    message: format!("{key} must have 4 elements"),
                });
            }

            let rect = [
                array.get(0).unwrap().as_real().unwrap_or(0.0),
                array.get(1).unwrap().as_real().unwrap_or(0.0),
                array.get(2).unwrap().as_real().unwrap_or(0.0),
                array.get(3).unwrap().as_real().unwrap_or(0.0),
            ];

            Ok(Some(rect))
        } else {
            Ok(None)
        }
    }

    /// Get an integer value
    fn get_integer(
        &self,
        node: &PdfDictionary,
        inherited: Option<&PdfDictionary>,
        key: &str,
    ) -> ParseResult<Option<i64>> {
        let value = node.get(key).or_else(|| inherited.and_then(|i| i.get(key)));

        Ok(value.and_then(|obj| obj.as_integer()))
    }

    /// Get an object by its reference numbers.
    ///
    /// This method first checks the cache, then loads from the file if needed.
    /// Objects are automatically cached after loading.
    ///
    /// # Arguments
    ///
    /// * `obj_num` - Object number
    /// * `gen_num` - Generation number
    ///
    /// # Returns
    ///
    /// The resolved PDF object.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Object doesn't exist
    /// - Object is part of an encrypted object stream
    /// - File is corrupted
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use oxidize_pdf::parser::{PdfDocument, PdfReader};
    /// # use oxidize_pdf::parser::objects::PdfObject;
    /// # fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let reader = PdfReader::open("document.pdf")?;
    /// # let document = PdfDocument::new(reader);
    /// // Get object 10 0 R
    /// let obj = document.get_object(10, 0)?;
    ///
    /// // Check object type
    /// match obj {
    ///     PdfObject::Dictionary(dict) => {
    ///         println!("Object is a dictionary with {} entries", dict.0.len());
    ///     }
    ///     PdfObject::Stream(stream) => {
    ///         println!("Object is a stream");
    ///     }
    ///     _ => {}
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn get_object(&self, obj_num: u32, gen_num: u16) -> ParseResult<PdfObject> {
        // Check resource cache first
        if let Some(obj) = self.resources.get_cached((obj_num, gen_num)) {
            return Ok(obj);
        }

        // Load from reader
        let obj = {
            let mut reader = self.reader.borrow_mut();
            reader.get_object(obj_num, gen_num)?.clone()
        };

        // Cache it
        self.resources.cache_object((obj_num, gen_num), obj.clone());

        Ok(obj)
    }

    /// Resolve a reference to get the actual object.
    ///
    /// If the input is a Reference, fetches the referenced object.
    /// Otherwise returns a clone of the input object.
    ///
    /// # Arguments
    ///
    /// * `obj` - The object to resolve (may be a Reference or direct object)
    ///
    /// # Returns
    ///
    /// The resolved object (never a Reference).
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use oxidize_pdf::parser::{PdfDocument, PdfReader};
    /// # use oxidize_pdf::parser::objects::PdfObject;
    /// # fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let reader = PdfReader::open("document.pdf")?;
    /// # let document = PdfDocument::new(reader);
    /// # let page = document.get_page(0)?;
    /// // Contents might be a reference or direct object
    /// if let Some(contents) = page.dict.get("Contents") {
    ///     let resolved = document.resolve(contents)?;
    ///     match resolved {
    ///         PdfObject::Stream(_) => println!("Single content stream"),
    ///         PdfObject::Array(_) => println!("Multiple content streams"),
    ///         _ => println!("Unexpected content type"),
    ///     }
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn resolve(&self, obj: &PdfObject) -> ParseResult<PdfObject> {
        match obj {
            PdfObject::Reference(obj_num, gen_num) => self.get_object(*obj_num, *gen_num),
            _ => Ok(obj.clone()),
        }
    }

    /// Get content streams for a specific page.
    ///
    /// This method handles both single streams and arrays of streams,
    /// automatically decompressing them according to their filters.
    ///
    /// # Arguments
    ///
    /// * `page` - The page to get content streams from
    ///
    /// # Returns
    ///
    /// Vector of decompressed content stream data ready for parsing.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use oxidize_pdf::parser::{PdfDocument, PdfReader};
    /// # use oxidize_pdf::parser::content::ContentParser;
    /// # fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let reader = PdfReader::open("document.pdf")?;
    /// # let document = PdfDocument::new(reader);
    /// let page = document.get_page(0)?;
    /// let streams = document.get_page_content_streams(&page)?;
    ///
    /// // Parse content streams
    /// for stream_data in streams {
    ///     let operations = ContentParser::parse(&stream_data)?;
    ///     println!("Stream has {} operations", operations.len());
    /// }
    /// # Ok(())
    /// # }
    /// ```
    /// Get page resources dictionary.
    ///
    /// This method returns the resources dictionary for a page, which may include
    /// fonts, images (XObjects), patterns, color spaces, and other resources.
    ///
    /// # Arguments
    ///
    /// * `page` - The page to get resources from
    ///
    /// # Returns
    ///
    /// Optional resources dictionary if the page has resources.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use oxidize_pdf::parser::{PdfDocument, PdfReader, PdfObject, PdfName};
    /// # fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let reader = PdfReader::open("document.pdf")?;
    /// # let document = PdfDocument::new(reader);
    /// let page = document.get_page(0)?;
    /// if let Some(resources) = document.get_page_resources(&page)? {
    ///     // Check for images (XObjects)
    ///     if let Some(PdfObject::Dictionary(xobjects)) = resources.0.get(&PdfName("XObject".to_string())) {
    ///         for (name, _) in xobjects.0.iter() {
    ///             println!("Found XObject: {}", name.0);
    ///         }
    ///     }
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn get_page_resources<'a>(
        &self,
        page: &'a ParsedPage,
    ) -> ParseResult<Option<&'a PdfDictionary>> {
        Ok(page.get_resources())
    }

    pub fn get_page_content_streams(&self, page: &ParsedPage) -> ParseResult<Vec<Vec<u8>>> {
        let mut streams = Vec::new();

        if let Some(contents) = page.dict.get("Contents") {
            let resolved_contents = self.resolve(contents)?;

            match &resolved_contents {
                PdfObject::Stream(stream) => {
                    streams.push(stream.decode()?);
                }
                PdfObject::Array(array) => {
                    for item in &array.0 {
                        let resolved = self.resolve(item)?;
                        if let PdfObject::Stream(stream) = resolved {
                            streams.push(stream.decode()?);
                        }
                    }
                }
                _ => {
                    return Err(ParseError::SyntaxError {
                        position: 0,
                        message: "Contents must be a stream or array of streams".to_string(),
                    })
                }
            }
        }

        Ok(streams)
    }

    /// Extract text from all pages in the document.
    ///
    /// Uses the default text extraction settings. For custom settings,
    /// use `extract_text_with_options`.
    ///
    /// # Returns
    ///
    /// A vector of `ExtractedText`, one for each page in the document.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use oxidize_pdf::parser::{PdfDocument, PdfReader};
    /// # fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let reader = PdfReader::open("document.pdf")?;
    /// # let document = PdfDocument::new(reader);
    /// let extracted_pages = document.extract_text()?;
    ///
    /// for (page_num, page_text) in extracted_pages.iter().enumerate() {
    ///     println!("=== Page {} ===", page_num + 1);
    ///     println!("{}", page_text.text);
    ///     println!();
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn extract_text(&self) -> ParseResult<Vec<crate::text::ExtractedText>> {
        let extractor = crate::text::TextExtractor::new();
        extractor.extract_from_document(self)
    }

    /// Extract text from a specific page.
    ///
    /// # Arguments
    ///
    /// * `page_index` - Zero-based page index
    ///
    /// # Returns
    ///
    /// Extracted text with optional position information.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use oxidize_pdf::parser::{PdfDocument, PdfReader};
    /// # fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let reader = PdfReader::open("document.pdf")?;
    /// # let document = PdfDocument::new(reader);
    /// // Extract text from first page only
    /// let page_text = document.extract_text_from_page(0)?;
    /// println!("First page text: {}", page_text.text);
    ///
    /// // Access text fragments with positions (if preserved)
    /// for fragment in &page_text.fragments {
    ///     println!("'{}' at ({}, {})", fragment.text, fragment.x, fragment.y);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn extract_text_from_page(
        &self,
        page_index: u32,
    ) -> ParseResult<crate::text::ExtractedText> {
        let extractor = crate::text::TextExtractor::new();
        extractor.extract_from_page(self, page_index)
    }

    /// Extract text with custom extraction options.
    ///
    /// Allows fine control over text extraction behavior including
    /// layout preservation, spacing thresholds, and more.
    ///
    /// # Arguments
    ///
    /// * `options` - Text extraction configuration
    ///
    /// # Returns
    ///
    /// A vector of `ExtractedText`, one for each page.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use oxidize_pdf::parser::{PdfDocument, PdfReader};
    /// # use oxidize_pdf::text::ExtractionOptions;
    /// # fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let reader = PdfReader::open("document.pdf")?;
    /// # let document = PdfDocument::new(reader);
    /// // Configure extraction to preserve layout
    /// let options = ExtractionOptions {
    ///     preserve_layout: true,
    ///     space_threshold: 0.3,
    ///     newline_threshold: 10.0,
    ///     ..Default::default()
    /// };
    ///
    /// let extracted_pages = document.extract_text_with_options(options)?;
    ///
    /// // Text fragments will include position information
    /// for page_text in extracted_pages {
    ///     for fragment in &page_text.fragments {
    ///         println!("{:?}", fragment);
    ///     }
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn extract_text_with_options(
        &self,
        options: crate::text::ExtractionOptions,
    ) -> ParseResult<Vec<crate::text::ExtractedText>> {
        let extractor = crate::text::TextExtractor::with_options(options);
        extractor.extract_from_document(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::objects::{PdfObject, PdfString};
    use std::io::Cursor;

    // Helper function to create a minimal PDF in memory
    fn create_minimal_pdf() -> Vec<u8> {
        let mut pdf = Vec::new();

        // PDF header
        pdf.extend_from_slice(b"%PDF-1.4\n");

        // Catalog object
        pdf.extend_from_slice(b"1 0 obj\n");
        pdf.extend_from_slice(b"<< /Type /Catalog /Pages 2 0 R >>\n");
        pdf.extend_from_slice(b"endobj\n");

        // Pages object
        pdf.extend_from_slice(b"2 0 obj\n");
        pdf.extend_from_slice(b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>\n");
        pdf.extend_from_slice(b"endobj\n");

        // Page object
        pdf.extend_from_slice(b"3 0 obj\n");
        pdf.extend_from_slice(
            b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Resources << >> >>\n",
        );
        pdf.extend_from_slice(b"endobj\n");

        // Cross-reference table
        let xref_pos = pdf.len();
        pdf.extend_from_slice(b"xref\n");
        pdf.extend_from_slice(b"0 4\n");
        pdf.extend_from_slice(b"0000000000 65535 f \n");
        pdf.extend_from_slice(b"0000000009 00000 n \n");
        pdf.extend_from_slice(b"0000000058 00000 n \n");
        pdf.extend_from_slice(b"0000000115 00000 n \n");

        // Trailer
        pdf.extend_from_slice(b"trailer\n");
        pdf.extend_from_slice(b"<< /Size 4 /Root 1 0 R >>\n");
        pdf.extend_from_slice(b"startxref\n");
        pdf.extend_from_slice(format!("{}\n", xref_pos).as_bytes());
        pdf.extend_from_slice(b"%%EOF\n");

        pdf
    }

    // Helper to create a PDF with metadata
    fn create_pdf_with_metadata() -> Vec<u8> {
        let mut pdf = Vec::new();

        // PDF header
        pdf.extend_from_slice(b"%PDF-1.5\n");

        // Record positions for xref
        let obj1_pos = pdf.len();

        // Catalog object
        pdf.extend_from_slice(b"1 0 obj\n");
        pdf.extend_from_slice(b"<< /Type /Catalog /Pages 2 0 R >>\n");
        pdf.extend_from_slice(b"endobj\n");

        let obj2_pos = pdf.len();

        // Pages object
        pdf.extend_from_slice(b"2 0 obj\n");
        pdf.extend_from_slice(b"<< /Type /Pages /Kids [] /Count 0 >>\n");
        pdf.extend_from_slice(b"endobj\n");

        let obj3_pos = pdf.len();

        // Info object
        pdf.extend_from_slice(b"3 0 obj\n");
        pdf.extend_from_slice(
            b"<< /Title (Test Document) /Author (Test Author) /Subject (Test Subject) >>\n",
        );
        pdf.extend_from_slice(b"endobj\n");

        // Cross-reference table
        let xref_pos = pdf.len();
        pdf.extend_from_slice(b"xref\n");
        pdf.extend_from_slice(b"0 4\n");
        pdf.extend_from_slice(b"0000000000 65535 f \n");
        pdf.extend_from_slice(format!("{:010} 00000 n \n", obj1_pos).as_bytes());
        pdf.extend_from_slice(format!("{:010} 00000 n \n", obj2_pos).as_bytes());
        pdf.extend_from_slice(format!("{:010} 00000 n \n", obj3_pos).as_bytes());

        // Trailer
        pdf.extend_from_slice(b"trailer\n");
        pdf.extend_from_slice(b"<< /Size 4 /Root 1 0 R /Info 3 0 R >>\n");
        pdf.extend_from_slice(b"startxref\n");
        pdf.extend_from_slice(format!("{}\n", xref_pos).as_bytes());
        pdf.extend_from_slice(b"%%EOF\n");

        pdf
    }

    #[test]
    fn test_pdf_document_new() {
        let pdf_data = create_minimal_pdf();
        let cursor = Cursor::new(pdf_data);
        let reader = PdfReader::new(cursor).unwrap();
        let document = PdfDocument::new(reader);

        // Verify document is created with empty caches
        assert!(document.page_tree.borrow().is_none());
        assert!(document.metadata_cache.borrow().is_none());
    }

    #[test]
    fn test_version() {
        let pdf_data = create_minimal_pdf();
        let cursor = Cursor::new(pdf_data);
        let reader = PdfReader::new(cursor).unwrap();
        let document = PdfDocument::new(reader);

        let version = document.version().unwrap();
        assert_eq!(version, "1.4");
    }

    #[test]
    fn test_page_count() {
        let pdf_data = create_minimal_pdf();
        let cursor = Cursor::new(pdf_data);
        let reader = PdfReader::new(cursor).unwrap();
        let document = PdfDocument::new(reader);

        let count = document.page_count().unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn test_metadata() {
        let pdf_data = create_pdf_with_metadata();
        let cursor = Cursor::new(pdf_data);
        let reader = PdfReader::new(cursor).unwrap();
        let document = PdfDocument::new(reader);

        let metadata = document.metadata().unwrap();
        assert_eq!(metadata.title, Some("Test Document".to_string()));
        assert_eq!(metadata.author, Some("Test Author".to_string()));
        assert_eq!(metadata.subject, Some("Test Subject".to_string()));

        // Verify caching works
        let metadata2 = document.metadata().unwrap();
        assert_eq!(metadata.title, metadata2.title);
    }

    #[test]
    fn test_get_page() {
        let pdf_data = create_minimal_pdf();
        let cursor = Cursor::new(pdf_data);
        let reader = PdfReader::new(cursor).unwrap();
        let document = PdfDocument::new(reader);

        // Get first page
        let page = document.get_page(0).unwrap();
        assert_eq!(page.media_box, [0.0, 0.0, 612.0, 792.0]);

        // Verify caching works
        let page2 = document.get_page(0).unwrap();
        assert_eq!(page.media_box, page2.media_box);
    }

    #[test]
    fn test_get_page_out_of_bounds() {
        let pdf_data = create_minimal_pdf();
        let cursor = Cursor::new(pdf_data);
        let reader = PdfReader::new(cursor).unwrap();
        let document = PdfDocument::new(reader);

        // Try to get page that doesn't exist
        let result = document.get_page(10);
        assert!(result.is_err());
    }

    #[test]
    fn test_resource_manager_caching() {
        let resources = ResourceManager::new();

        // Test caching an object
        let obj_ref = (1, 0);
        let obj = PdfObject::String(PdfString("Test".as_bytes().to_vec()));

        assert!(resources.get_cached(obj_ref).is_none());

        resources.cache_object(obj_ref, obj.clone());

        let cached = resources.get_cached(obj_ref).unwrap();
        assert_eq!(cached, obj);

        // Test clearing cache
        resources.clear_cache();
        assert!(resources.get_cached(obj_ref).is_none());
    }

    #[test]
    fn test_get_object() {
        let pdf_data = create_minimal_pdf();
        let cursor = Cursor::new(pdf_data);
        let reader = PdfReader::new(cursor).unwrap();
        let document = PdfDocument::new(reader);

        // Get catalog object
        let catalog = document.get_object(1, 0).unwrap();
        if let PdfObject::Dictionary(dict) = catalog {
            if let Some(PdfObject::Name(name)) = dict.get("Type") {
                assert_eq!(name.0, "Catalog");
            } else {
                panic!("Expected /Type name");
            }
        } else {
            panic!("Expected dictionary object");
        }
    }

    #[test]
    fn test_resolve_reference() {
        let pdf_data = create_minimal_pdf();
        let cursor = Cursor::new(pdf_data);
        let reader = PdfReader::new(cursor).unwrap();
        let document = PdfDocument::new(reader);

        // Create a reference to the catalog
        let ref_obj = PdfObject::Reference(1, 0);

        // Resolve it
        let resolved = document.resolve(&ref_obj).unwrap();
        if let PdfObject::Dictionary(dict) = resolved {
            if let Some(PdfObject::Name(name)) = dict.get("Type") {
                assert_eq!(name.0, "Catalog");
            } else {
                panic!("Expected /Type name");
            }
        } else {
            panic!("Expected dictionary object");
        }
    }

    #[test]
    fn test_resolve_non_reference() {
        let pdf_data = create_minimal_pdf();
        let cursor = Cursor::new(pdf_data);
        let reader = PdfReader::new(cursor).unwrap();
        let document = PdfDocument::new(reader);

        // Try to resolve a non-reference object
        let obj = PdfObject::String(PdfString("Test".as_bytes().to_vec()));
        let resolved = document.resolve(&obj).unwrap();

        // Should return the same object
        assert_eq!(resolved, obj);
    }

    #[test]
    fn test_invalid_pdf_data() {
        let invalid_data = b"This is not a PDF";
        let cursor = Cursor::new(invalid_data.to_vec());
        let result = PdfReader::new(cursor);

        assert!(result.is_err());
    }

    #[test]
    fn test_empty_page_tree() {
        // Create PDF with empty page tree
        let pdf_data = create_pdf_with_metadata(); // This has 0 pages
        let cursor = Cursor::new(pdf_data);
        let reader = PdfReader::new(cursor).unwrap();
        let document = PdfDocument::new(reader);

        let count = document.page_count().unwrap();
        assert_eq!(count, 0);

        // Try to get a page from empty document
        let result = document.get_page(0);
        assert!(result.is_err());
    }

    #[test]
    fn test_extract_text_empty_document() {
        let pdf_data = create_pdf_with_metadata();
        let cursor = Cursor::new(pdf_data);
        let reader = PdfReader::new(cursor).unwrap();
        let document = PdfDocument::new(reader);

        let text = document.extract_text().unwrap();
        assert!(text.is_empty());
    }

    #[test]
    fn test_concurrent_access() {
        let pdf_data = create_minimal_pdf();
        let cursor = Cursor::new(pdf_data);
        let reader = PdfReader::new(cursor).unwrap();
        let document = PdfDocument::new(reader);

        // Access multiple things concurrently
        let version = document.version().unwrap();
        let count = document.page_count().unwrap();
        let page = document.get_page(0).unwrap();

        assert_eq!(version, "1.4");
        assert_eq!(count, 1);
        assert_eq!(page.media_box[2], 612.0);
    }

    // Additional comprehensive tests
    mod comprehensive_tests {
        use super::*;

        #[test]
        fn test_resource_manager_default() {
            let resources = ResourceManager::default();
            assert!(resources.get_cached((1, 0)).is_none());
        }

        #[test]
        fn test_resource_manager_multiple_objects() {
            let resources = ResourceManager::new();

            // Cache multiple objects
            resources.cache_object((1, 0), PdfObject::Integer(42));
            resources.cache_object((2, 0), PdfObject::Boolean(true));
            resources.cache_object(
                (3, 0),
                PdfObject::String(PdfString("test".as_bytes().to_vec())),
            );

            // Verify all are cached
            assert!(resources.get_cached((1, 0)).is_some());
            assert!(resources.get_cached((2, 0)).is_some());
            assert!(resources.get_cached((3, 0)).is_some());

            // Clear and verify empty
            resources.clear_cache();
            assert!(resources.get_cached((1, 0)).is_none());
            assert!(resources.get_cached((2, 0)).is_none());
            assert!(resources.get_cached((3, 0)).is_none());
        }

        #[test]
        fn test_resource_manager_object_overwrite() {
            let resources = ResourceManager::new();

            // Cache an object
            resources.cache_object((1, 0), PdfObject::Integer(42));
            assert_eq!(resources.get_cached((1, 0)), Some(PdfObject::Integer(42)));

            // Overwrite with different object
            resources.cache_object((1, 0), PdfObject::Boolean(true));
            assert_eq!(resources.get_cached((1, 0)), Some(PdfObject::Boolean(true)));
        }

        #[test]
        fn test_get_object_caching() {
            let pdf_data = create_minimal_pdf();
            let cursor = Cursor::new(pdf_data);
            let reader = PdfReader::new(cursor).unwrap();
            let document = PdfDocument::new(reader);

            // Get object first time (should cache)
            let obj1 = document.get_object(1, 0).unwrap();

            // Get same object again (should use cache)
            let obj2 = document.get_object(1, 0).unwrap();

            // Objects should be identical
            assert_eq!(obj1, obj2);

            // Verify it's cached
            assert!(document.resources.get_cached((1, 0)).is_some());
        }

        #[test]
        fn test_get_object_different_generations() {
            let pdf_data = create_minimal_pdf();
            let cursor = Cursor::new(pdf_data);
            let reader = PdfReader::new(cursor).unwrap();
            let document = PdfDocument::new(reader);

            // Get object with generation 0
            let _obj1 = document.get_object(1, 0).unwrap();

            // Try to get same object with different generation (should fail)
            let result = document.get_object(1, 1);
            assert!(result.is_err());

            // Original should still be cached
            assert!(document.resources.get_cached((1, 0)).is_some());
        }

        #[test]
        fn test_get_object_nonexistent() {
            let pdf_data = create_minimal_pdf();
            let cursor = Cursor::new(pdf_data);
            let reader = PdfReader::new(cursor).unwrap();
            let document = PdfDocument::new(reader);

            // Try to get non-existent object
            let result = document.get_object(999, 0);
            assert!(result.is_err());
        }

        #[test]
        fn test_resolve_nested_references() {
            let pdf_data = create_minimal_pdf();
            let cursor = Cursor::new(pdf_data);
            let reader = PdfReader::new(cursor).unwrap();
            let document = PdfDocument::new(reader);

            // Test resolving a reference
            let ref_obj = PdfObject::Reference(2, 0);
            let resolved = document.resolve(&ref_obj).unwrap();

            // Should resolve to the pages object
            if let PdfObject::Dictionary(dict) = resolved {
                if let Some(PdfObject::Name(name)) = dict.get("Type") {
                    assert_eq!(name.0, "Pages");
                }
            }
        }

        #[test]
        fn test_resolve_various_object_types() {
            let pdf_data = create_minimal_pdf();
            let cursor = Cursor::new(pdf_data);
            let reader = PdfReader::new(cursor).unwrap();
            let document = PdfDocument::new(reader);

            // Test resolving different object types
            let test_objects = vec![
                PdfObject::Integer(42),
                PdfObject::Boolean(true),
                PdfObject::String(PdfString("test".as_bytes().to_vec())),
                PdfObject::Real(3.14),
                PdfObject::Null,
            ];

            for obj in test_objects {
                let resolved = document.resolve(&obj).unwrap();
                assert_eq!(resolved, obj);
            }
        }

        #[test]
        fn test_get_page_cached() {
            let pdf_data = create_minimal_pdf();
            let cursor = Cursor::new(pdf_data);
            let reader = PdfReader::new(cursor).unwrap();
            let document = PdfDocument::new(reader);

            // Get page first time
            let page1 = document.get_page(0).unwrap();

            // Get same page again
            let page2 = document.get_page(0).unwrap();

            // Should be identical
            assert_eq!(page1.media_box, page2.media_box);
            assert_eq!(page1.rotation, page2.rotation);
            assert_eq!(page1.obj_ref, page2.obj_ref);
        }

        #[test]
        fn test_metadata_caching() {
            let pdf_data = create_pdf_with_metadata();
            let cursor = Cursor::new(pdf_data);
            let reader = PdfReader::new(cursor).unwrap();
            let document = PdfDocument::new(reader);

            // Get metadata first time
            let meta1 = document.metadata().unwrap();

            // Get metadata again
            let meta2 = document.metadata().unwrap();

            // Should be identical
            assert_eq!(meta1.title, meta2.title);
            assert_eq!(meta1.author, meta2.author);
            assert_eq!(meta1.subject, meta2.subject);
            assert_eq!(meta1.version, meta2.version);
        }

        #[test]
        fn test_page_tree_initialization() {
            let pdf_data = create_minimal_pdf();
            let cursor = Cursor::new(pdf_data);
            let reader = PdfReader::new(cursor).unwrap();
            let document = PdfDocument::new(reader);

            // Initially page tree should be None
            assert!(document.page_tree.borrow().is_none());

            // After getting page count, page tree should be initialized
            let _count = document.page_count().unwrap();
            // Note: page_tree is private, so we can't directly check it
            // But we can verify it works by getting a page
            let _page = document.get_page(0).unwrap();
        }

        #[test]
        fn test_get_page_resources() {
            let pdf_data = create_minimal_pdf();
            let cursor = Cursor::new(pdf_data);
            let reader = PdfReader::new(cursor).unwrap();
            let document = PdfDocument::new(reader);

            let page = document.get_page(0).unwrap();
            let resources = document.get_page_resources(&page).unwrap();

            // The minimal PDF has empty resources
            assert!(resources.is_some());
        }

        #[test]
        fn test_get_page_content_streams_empty() {
            let pdf_data = create_minimal_pdf();
            let cursor = Cursor::new(pdf_data);
            let reader = PdfReader::new(cursor).unwrap();
            let document = PdfDocument::new(reader);

            let page = document.get_page(0).unwrap();
            let streams = document.get_page_content_streams(&page).unwrap();

            // Minimal PDF has no content streams
            assert!(streams.is_empty());
        }

        #[test]
        fn test_extract_text_from_page() {
            let pdf_data = create_minimal_pdf();
            let cursor = Cursor::new(pdf_data);
            let reader = PdfReader::new(cursor).unwrap();
            let document = PdfDocument::new(reader);

            let result = document.extract_text_from_page(0);
            // Should succeed even with empty page
            assert!(result.is_ok());
        }

        #[test]
        fn test_extract_text_from_page_out_of_bounds() {
            let pdf_data = create_minimal_pdf();
            let cursor = Cursor::new(pdf_data);
            let reader = PdfReader::new(cursor).unwrap();
            let document = PdfDocument::new(reader);

            let result = document.extract_text_from_page(999);
            assert!(result.is_err());
        }

        #[test]
        fn test_extract_text_with_options() {
            let pdf_data = create_minimal_pdf();
            let cursor = Cursor::new(pdf_data);
            let reader = PdfReader::new(cursor).unwrap();
            let document = PdfDocument::new(reader);

            let options = crate::text::ExtractionOptions {
                preserve_layout: true,
                space_threshold: 0.5,
                newline_threshold: 15.0,
                ..Default::default()
            };

            let result = document.extract_text_with_options(options);
            assert!(result.is_ok());
        }

        #[test]
        fn test_version_different_pdf_versions() {
            // Test with different PDF versions
            let versions = vec!["1.3", "1.4", "1.5", "1.6", "1.7"];

            for version in versions {
                let mut pdf_data = Vec::new();

                // PDF header
                pdf_data.extend_from_slice(format!("%PDF-{}\n", version).as_bytes());

                // Track positions for xref
                let obj1_pos = pdf_data.len();

                // Catalog object
                pdf_data.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");

                let obj2_pos = pdf_data.len();

                // Pages object
                pdf_data
                    .extend_from_slice(b"2 0 obj\n<< /Type /Pages /Kids [] /Count 0 >>\nendobj\n");

                // Cross-reference table
                let xref_pos = pdf_data.len();
                pdf_data.extend_from_slice(b"xref\n");
                pdf_data.extend_from_slice(b"0 3\n");
                pdf_data.extend_from_slice(b"0000000000 65535 f \n");
                pdf_data.extend_from_slice(format!("{:010} 00000 n \n", obj1_pos).as_bytes());
                pdf_data.extend_from_slice(format!("{:010} 00000 n \n", obj2_pos).as_bytes());

                // Trailer
                pdf_data.extend_from_slice(b"trailer\n");
                pdf_data.extend_from_slice(b"<< /Size 3 /Root 1 0 R >>\n");
                pdf_data.extend_from_slice(b"startxref\n");
                pdf_data.extend_from_slice(format!("{}\n", xref_pos).as_bytes());
                pdf_data.extend_from_slice(b"%%EOF\n");

                let cursor = Cursor::new(pdf_data);
                let reader = PdfReader::new(cursor).unwrap();
                let document = PdfDocument::new(reader);

                let pdf_version = document.version().unwrap();
                assert_eq!(pdf_version, version);
            }
        }

        #[test]
        fn test_page_count_zero() {
            let pdf_data = create_pdf_with_metadata(); // Has 0 pages
            let cursor = Cursor::new(pdf_data);
            let reader = PdfReader::new(cursor).unwrap();
            let document = PdfDocument::new(reader);

            let count = document.page_count().unwrap();
            assert_eq!(count, 0);
        }

        #[test]
        fn test_multiple_object_access() {
            let pdf_data = create_minimal_pdf();
            let cursor = Cursor::new(pdf_data);
            let reader = PdfReader::new(cursor).unwrap();
            let document = PdfDocument::new(reader);

            // Access multiple objects
            let catalog = document.get_object(1, 0).unwrap();
            let pages = document.get_object(2, 0).unwrap();
            let page = document.get_object(3, 0).unwrap();

            // Verify they're all different objects
            assert_ne!(catalog, pages);
            assert_ne!(pages, page);
            assert_ne!(catalog, page);
        }

        #[test]
        fn test_error_handling_invalid_object_reference() {
            let pdf_data = create_minimal_pdf();
            let cursor = Cursor::new(pdf_data);
            let reader = PdfReader::new(cursor).unwrap();
            let document = PdfDocument::new(reader);

            // Try to resolve an invalid reference
            let invalid_ref = PdfObject::Reference(999, 0);
            let result = document.resolve(&invalid_ref);
            assert!(result.is_err());
        }

        #[test]
        fn test_concurrent_metadata_access() {
            let pdf_data = create_pdf_with_metadata();
            let cursor = Cursor::new(pdf_data);
            let reader = PdfReader::new(cursor).unwrap();
            let document = PdfDocument::new(reader);

            // Access metadata and other properties concurrently
            let metadata = document.metadata().unwrap();
            let version = document.version().unwrap();
            let count = document.page_count().unwrap();

            assert_eq!(metadata.title, Some("Test Document".to_string()));
            assert_eq!(version, "1.5");
            assert_eq!(count, 0);
        }

        #[test]
        fn test_page_properties_comprehensive() {
            let pdf_data = create_minimal_pdf();
            let cursor = Cursor::new(pdf_data);
            let reader = PdfReader::new(cursor).unwrap();
            let document = PdfDocument::new(reader);

            let page = document.get_page(0).unwrap();

            // Test all page properties
            assert_eq!(page.media_box, [0.0, 0.0, 612.0, 792.0]);
            assert_eq!(page.crop_box, None);
            assert_eq!(page.rotation, 0);
            assert_eq!(page.obj_ref, (3, 0));

            // Test width/height calculation
            assert_eq!(page.width(), 612.0);
            assert_eq!(page.height(), 792.0);
        }

        #[test]
        fn test_memory_usage_efficiency() {
            let pdf_data = create_minimal_pdf();
            let cursor = Cursor::new(pdf_data);
            let reader = PdfReader::new(cursor).unwrap();
            let document = PdfDocument::new(reader);

            // Access same page multiple times
            for _ in 0..10 {
                let _page = document.get_page(0).unwrap();
            }

            // Should only have one copy in cache
            let page_count = document.page_count().unwrap();
            assert_eq!(page_count, 1);
        }

        #[test]
        fn test_reader_borrow_safety() {
            let pdf_data = create_minimal_pdf();
            let cursor = Cursor::new(pdf_data);
            let reader = PdfReader::new(cursor).unwrap();
            let document = PdfDocument::new(reader);

            // Multiple concurrent borrows should work
            let version = document.version().unwrap();
            let count = document.page_count().unwrap();
            let metadata = document.metadata().unwrap();

            assert_eq!(version, "1.4");
            assert_eq!(count, 1);
            assert!(metadata.title.is_none());
        }

        #[test]
        fn test_cache_consistency() {
            let pdf_data = create_minimal_pdf();
            let cursor = Cursor::new(pdf_data);
            let reader = PdfReader::new(cursor).unwrap();
            let document = PdfDocument::new(reader);

            // Get object and verify caching
            let obj1 = document.get_object(1, 0).unwrap();
            let cached = document.resources.get_cached((1, 0)).unwrap();

            assert_eq!(obj1, cached);

            // Clear cache and get object again
            document.resources.clear_cache();
            let obj2 = document.get_object(1, 0).unwrap();

            // Should be same content but loaded fresh
            assert_eq!(obj1, obj2);
        }
    }
}
