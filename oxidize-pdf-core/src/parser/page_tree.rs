//! PDF Page Tree Parser
//!
//! This module handles navigation and extraction of pages from the PDF page tree structure.
//! The page tree is a hierarchical structure that organizes pages in a PDF document,
//! allowing for efficient access and inheritance of properties from parent nodes.
//!
//! # Overview
//!
//! The PDF page tree consists of:
//! - **Page Tree Nodes**: Internal nodes that can contain other nodes or pages
//! - **Page Objects**: Leaf nodes representing individual pages
//! - **Inherited Properties**: Resources, MediaBox, CropBox, and Rotate can be inherited from parent nodes
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
//! // Get a specific page
//! let page = document.get_page(0)?;
//!
//! // Access page properties
//! println!("Page size: {}x{} points", page.width(), page.height());
//! println!("Rotation: {}°", page.rotation);
//!
//! // Get page resources
//! if let Some(resources) = page.get_resources() {
//!     println!("Page has resources");
//! }
//! # Ok(())
//! # }
//! ```

use super::document::PdfDocument;
use super::objects::{PdfDictionary, PdfObject, PdfStream};
use super::reader::PdfReader;
use super::{ParseError, ParseResult};
use std::collections::HashMap;
use std::io::{Read, Seek};

/// Represents a single page in the PDF with all its properties and resources.
///
/// A `ParsedPage` contains all the information needed to render or analyze a PDF page,
/// including its dimensions, content streams, resources, and inherited properties from
/// parent page tree nodes.
///
/// # Fields
///
/// * `obj_ref` - Object reference (object number, generation number) pointing to this page in the PDF
/// * `dict` - Complete page dictionary containing all page-specific entries
/// * `inherited_resources` - Resources inherited from parent page tree nodes
/// * `media_box` - Page dimensions in PDF units [llx, lly, urx, ury]
/// * `crop_box` - Optional visible area of the page
/// * `rotation` - Page rotation in degrees (0, 90, 180, or 270)
///
/// # Example
///
/// ```rust,no_run
/// use oxidize_pdf::parser::{PdfDocument, PdfReader};
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let reader = PdfReader::open("document.pdf")?;
/// let document = PdfDocument::new(reader);
/// let page = document.get_page(0)?;
///
/// // Access page properties
/// let (obj_num, gen_num) = page.obj_ref;
/// println!("Page object: {} {} R", obj_num, gen_num);
///
/// // Get page dimensions
/// let [llx, lly, urx, ury] = page.media_box;
/// println!("MediaBox: ({}, {}) to ({}, {})", llx, lly, urx, ury);
///
/// // Check for content
/// if let Some(contents) = page.dict.get("Contents") {
///     println!("Page has content streams");
/// }
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone)]
pub struct ParsedPage {
    /// Object reference to this page in the form (object_number, generation_number).
    /// This uniquely identifies the page object in the PDF file.
    pub obj_ref: (u32, u16),

    /// Page dictionary containing all page-specific entries like Contents, Resources, etc.
    /// This is the raw PDF dictionary for the page object.
    pub dict: PdfDictionary,

    /// Resources inherited from parent page tree nodes.
    /// These are automatically merged during page tree traversal.
    pub inherited_resources: Option<PdfDictionary>,

    /// MediaBox defining the page dimensions in PDF units (typically points).
    /// Format: [lower_left_x, lower_left_y, upper_right_x, upper_right_y]
    pub media_box: [f64; 4],

    /// CropBox defining the visible area of the page.
    /// If None, the entire MediaBox is visible.
    pub crop_box: Option<[f64; 4]>,

    /// Page rotation in degrees. Valid values are 0, 90, 180, or 270.
    /// The rotation is applied clockwise.
    pub rotation: i32,
}

/// Page tree navigator
pub struct PageTree {
    /// Total number of pages
    page_count: u32,
    /// Cached pages by index
    pages: HashMap<u32, ParsedPage>,
    /// Root pages dictionary (for navigation)
    #[allow(dead_code)]
    pages_dict: Option<PdfDictionary>,
}

impl PageTree {
    /// Create a new page tree navigator
    pub fn new(page_count: u32) -> Self {
        Self {
            page_count,
            pages: HashMap::new(),
            pages_dict: None,
        }
    }

    /// Create a new page tree navigator with pages dictionary
    pub fn new_with_pages_dict(page_count: u32, pages_dict: PdfDictionary) -> Self {
        Self {
            page_count,
            pages: HashMap::new(),
            pages_dict: Some(pages_dict),
        }
    }

    /// Get a cached page by index (0-based)
    pub fn get_cached_page(&self, index: u32) -> Option<&ParsedPage> {
        self.pages.get(&index)
    }

    /// Cache a page
    pub fn cache_page(&mut self, index: u32, page: ParsedPage) {
        self.pages.insert(index, page);
    }

    /// Get the total page count
    pub fn page_count(&self) -> u32 {
        self.page_count
    }

    /// Load a specific page by traversing the page tree
    #[allow(dead_code)]
    fn load_page_at_index<R: Read + Seek>(
        &self,
        reader: &mut PdfReader<R>,
        node: &PdfDictionary,
        target_index: u32,
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

                // Inheritable attributes: Resources, MediaBox, CropBox, Rotate
                if let Some(resources) = node.get("Resources") {
                    if !merged_inherited.contains_key("Resources") {
                        merged_inherited.insert("Resources".to_string(), resources.clone());
                    }
                }
                if let Some(media_box) = node.get("MediaBox") {
                    if !merged_inherited.contains_key("MediaBox") {
                        merged_inherited.insert("MediaBox".to_string(), media_box.clone());
                    }
                }
                if let Some(crop_box) = node.get("CropBox") {
                    if !merged_inherited.contains_key("CropBox") {
                        merged_inherited.insert("CropBox".to_string(), crop_box.clone());
                    }
                }
                if let Some(rotate) = node.get("Rotate") {
                    if !merged_inherited.contains_key("Rotate") {
                        merged_inherited.insert("Rotate".to_string(), rotate.clone());
                    }
                }

                // Find which kid contains our target page
                let mut current_index = 0;
                for kid_ref in &kids.0 {
                    let kid_ref =
                        kid_ref
                            .as_reference()
                            .ok_or_else(|| ParseError::SyntaxError {
                                position: 0,
                                message: "Kids array must contain references".to_string(),
                            })?;

                    // Get the kid object info first
                    let (_kid_type, count, is_target) = {
                        let kid_obj = reader.get_object(kid_ref.0, kid_ref.1)?;
                        let kid_dict =
                            kid_obj.as_dict().ok_or_else(|| ParseError::SyntaxError {
                                position: 0,
                                message: "Page tree node must be a dictionary".to_string(),
                            })?;

                        let kid_type = kid_dict
                            .get_type()
                            .ok_or_else(|| ParseError::MissingKey("Type".to_string()))?;

                        let count = if kid_type == "Pages" {
                            // This is another page tree node
                            kid_dict
                                .get("Count")
                                .and_then(|obj| obj.as_integer())
                                .ok_or_else(|| ParseError::MissingKey("Count".to_string()))?
                                as u32
                        } else {
                            // This is a page
                            1
                        };

                        let is_target = target_index < current_index + count;
                        (kid_type.to_string(), count, is_target)
                    };

                    if is_target {
                        // Found the right subtree/page
                        // Get the kid dict again
                        let kid_obj = reader.get_object(kid_ref.0, kid_ref.1)?;
                        let kid_dict = kid_obj.as_dict().unwrap();

                        // TODO: Fix borrow checker issue with recursive calls
                        // For now, return a placeholder
                        return Ok(ParsedPage {
                            obj_ref: kid_ref,
                            dict: kid_dict.clone(),
                            inherited_resources: Some(merged_inherited.clone()),
                            media_box: [0.0, 0.0, 612.0, 792.0],
                            crop_box: None,
                            rotation: 0,
                        });
                    }

                    current_index += count;
                }

                Err(ParseError::SyntaxError {
                    position: 0,
                    message: "Page not found in tree".to_string(),
                })
            }
            "Page" => {
                // This is a page object
                if target_index != 0 {
                    return Err(ParseError::SyntaxError {
                        position: 0,
                        message: "Page index mismatch".to_string(),
                    });
                }

                // Get page reference (we need to track back from the current node)
                // For now, use a placeholder
                let obj_ref = (0, 0); // TODO: Get actual reference

                // Extract page attributes
                let media_box = Self::get_rectangle(node, inherited, "MediaBox")?
                    .ok_or_else(|| ParseError::MissingKey("MediaBox".to_string()))?;

                let crop_box = Self::get_rectangle(node, inherited, "CropBox")?;

                let rotation = Self::get_integer(node, inherited, "Rotate")?.unwrap_or(0) as i32;

                // Get resources
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
                    dict: node.clone(),
                    inherited_resources,
                    media_box,
                    crop_box,
                    rotation,
                })
            }
            _ => Err(ParseError::SyntaxError {
                position: 0,
                message: format!("Invalid page tree node type: {node_type}"),
            }),
        }
    }

    /// Get a rectangle value, checking both node and inherited dictionaries
    #[allow(dead_code)]
    fn get_rectangle(
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

    /// Get an integer value, checking both node and inherited dictionaries
    #[allow(dead_code)]
    fn get_integer(
        node: &PdfDictionary,
        inherited: Option<&PdfDictionary>,
        key: &str,
    ) -> ParseResult<Option<i64>> {
        let value = node.get(key).or_else(|| inherited.and_then(|i| i.get(key)));

        Ok(value.and_then(|obj| obj.as_integer()))
    }
}

impl ParsedPage {
    /// Get the effective page width accounting for rotation.
    ///
    /// The width is calculated from the MediaBox and adjusted based on the page rotation.
    /// For 90° or 270° rotations, the width and height are swapped.
    ///
    /// # Returns
    ///
    /// The page width in PDF units (typically points, where 1 point = 1/72 inch)
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use oxidize_pdf::parser::{PdfDocument, PdfReader};
    /// # fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let reader = PdfReader::open("document.pdf")?;
    /// # let document = PdfDocument::new(reader);
    /// let page = document.get_page(0)?;
    /// let width_pts = page.width();
    /// let width_inches = width_pts / 72.0;
    /// let width_mm = width_pts * 25.4 / 72.0;
    /// println!("Page width: {} points ({:.2} inches, {:.2} mm)", width_pts, width_inches, width_mm);
    /// # Ok(())
    /// # }
    /// ```
    pub fn width(&self) -> f64 {
        match self.rotation {
            90 | 270 => self.media_box[3] - self.media_box[1],
            _ => self.media_box[2] - self.media_box[0],
        }
    }

    /// Get the effective page height accounting for rotation.
    ///
    /// The height is calculated from the MediaBox and adjusted based on the page rotation.
    /// For 90° or 270° rotations, the width and height are swapped.
    ///
    /// # Returns
    ///
    /// The page height in PDF units (typically points, where 1 point = 1/72 inch)
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use oxidize_pdf::parser::{PdfDocument, PdfReader};
    /// # fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let reader = PdfReader::open("document.pdf")?;
    /// # let document = PdfDocument::new(reader);
    /// let page = document.get_page(0)?;
    /// println!("Page dimensions: {}x{} points", page.width(), page.height());
    /// if page.rotation != 0 {
    ///     println!("Page is rotated {} degrees", page.rotation);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn height(&self) -> f64 {
        match self.rotation {
            90 | 270 => self.media_box[2] - self.media_box[0],
            _ => self.media_box[3] - self.media_box[1],
        }
    }

    /// Get the content streams for this page using a PdfReader.
    ///
    /// Content streams contain the actual drawing instructions (operators) that render
    /// text, graphics, and images on the page. A page may have multiple content streams
    /// which are concatenated during rendering.
    ///
    /// # Arguments
    ///
    /// * `reader` - Mutable reference to the PDF reader
    ///
    /// # Returns
    ///
    /// A vector of decompressed content stream data. Each vector contains the raw bytes
    /// of a content stream ready for parsing.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The Contents entry is malformed
    /// - Stream decompression fails
    /// - Referenced objects cannot be resolved
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use oxidize_pdf::parser::{PdfReader, ParsedPage};
    /// # fn example(page: &ParsedPage, reader: &mut PdfReader<std::fs::File>) -> Result<(), Box<dyn std::error::Error>> {
    /// let streams = page.content_streams(reader)?;
    /// for (i, stream) in streams.iter().enumerate() {
    ///     println!("Content stream {}: {} bytes", i, stream.len());
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn content_streams<R: Read + Seek>(
        &self,
        reader: &mut PdfReader<R>,
    ) -> ParseResult<Vec<Vec<u8>>> {
        let mut streams = Vec::new();

        if let Some(contents) = self.dict.get("Contents") {
            // First resolve contents to check its type
            let contents_type = match contents {
                PdfObject::Reference(obj_num, gen_num) => {
                    let resolved = reader.get_object(*obj_num, *gen_num)?;
                    match resolved {
                        PdfObject::Stream(_) => "stream",
                        PdfObject::Array(_) => "array",
                        _ => "other",
                    }
                }
                PdfObject::Stream(_) => "stream",
                PdfObject::Array(_) => "array",
                _ => "other",
            };

            match contents_type {
                "stream" => {
                    let resolved = reader.resolve(contents)?;
                    if let PdfObject::Stream(stream) = resolved {
                        streams.push(stream.decode()?);
                    }
                }
                "array" => {
                    // Get array references first
                    let refs: Vec<(u32, u16)> = {
                        let resolved = reader.resolve(contents)?;
                        if let PdfObject::Array(array) = resolved {
                            array
                                .0
                                .iter()
                                .filter_map(|obj| {
                                    if let PdfObject::Reference(num, gen) = obj {
                                        Some((*num, *gen))
                                    } else {
                                        None
                                    }
                                })
                                .collect()
                        } else {
                            Vec::new()
                        }
                    };

                    // Now resolve each reference
                    for (obj_num, gen_num) in refs {
                        let obj = reader.get_object(obj_num, gen_num)?;
                        if let PdfObject::Stream(stream) = obj {
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

    /// Get content streams using PdfDocument (recommended method).
    ///
    /// This is the preferred method for accessing content streams as it uses the
    /// document's caching and resource management capabilities.
    ///
    /// # Arguments
    ///
    /// * `document` - Reference to the PDF document
    ///
    /// # Returns
    ///
    /// A vector of decompressed content stream data ready for parsing with `ContentParser`.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use oxidize_pdf::parser::{PdfDocument, PdfReader};
    /// # use oxidize_pdf::parser::content::ContentParser;
    /// # fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let reader = PdfReader::open("document.pdf")?;
    /// let document = PdfDocument::new(reader);
    /// let page = document.get_page(0)?;
    ///
    /// // Get content streams
    /// let streams = page.content_streams_with_document(&document)?;
    ///
    /// // Parse each stream
    /// for stream_data in streams {
    ///     let operations = ContentParser::parse_content(&stream_data)?;
    ///     println!("Stream has {} operations", operations.len());
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn content_streams_with_document<R: Read + Seek>(
        &self,
        document: &PdfDocument<R>,
    ) -> ParseResult<Vec<Vec<u8>>> {
        document.get_page_content_streams(self)
    }

    /// Get the effective resources for this page (including inherited).
    ///
    /// Resources include fonts, images (XObjects), color spaces, patterns, and other
    /// assets needed to render the page. This method returns page-specific resources
    /// if present, otherwise falls back to inherited resources from parent nodes.
    ///
    /// # Returns
    ///
    /// The Resources dictionary if available, or None if the page has no resources.
    ///
    /// # Resource Categories
    ///
    /// The Resources dictionary may contain:
    /// - `Font` - Font definitions used by text operators
    /// - `XObject` - External objects (images, form XObjects)
    /// - `ColorSpace` - Color space definitions
    /// - `Pattern` - Pattern definitions for fills
    /// - `Shading` - Shading dictionaries
    /// - `ExtGState` - Graphics state parameter dictionaries
    /// - `Properties` - Property list dictionaries
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use oxidize_pdf::parser::{PdfDocument, PdfReader};
    /// # fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let reader = PdfReader::open("document.pdf")?;
    /// # let document = PdfDocument::new(reader);
    /// # let page = document.get_page(0)?;
    /// if let Some(resources) = page.get_resources() {
    ///     // Check for fonts
    ///     if let Some(fonts) = resources.get("Font").and_then(|f| f.as_dict()) {
    ///         println!("Page uses {} fonts", fonts.0.len());
    ///     }
    ///     
    ///     // Check for images
    ///     if let Some(xobjects) = resources.get("XObject").and_then(|x| x.as_dict()) {
    ///         println!("Page has {} XObjects", xobjects.0.len());
    ///     }
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn get_resources(&self) -> Option<&PdfDictionary> {
        self.dict
            .get("Resources")
            .and_then(|r| r.as_dict())
            .or(self.inherited_resources.as_ref())
    }

    /// Clone this page with all inherited resources merged into the page dictionary.
    ///
    /// This is useful when extracting a page for separate processing or when you need
    /// a self-contained page object with all resources explicitly included.
    ///
    /// # Returns
    ///
    /// A cloned page with inherited resources merged into the Resources entry
    /// of the page dictionary.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use oxidize_pdf::parser::{PdfDocument, PdfReader};
    /// # fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let reader = PdfReader::open("document.pdf")?;
    /// # let document = PdfDocument::new(reader);
    /// # let page = document.get_page(0)?;
    /// // Get a self-contained page with all resources
    /// let standalone_page = page.clone_with_resources();
    ///
    /// // The cloned page now has all resources in its dictionary
    /// assert!(standalone_page.dict.contains_key("Resources"));
    /// # Ok(())
    /// # }
    /// ```
    pub fn clone_with_resources(&self) -> Self {
        let mut cloned = self.clone();

        // Merge inherited resources into the page dictionary if needed
        if let Some(inherited) = &self.inherited_resources {
            if !cloned.dict.contains_key("Resources") {
                cloned.dict.insert(
                    "Resources".to_string(),
                    PdfObject::Dictionary(inherited.clone()),
                );
            }
        }

        cloned
    }

    /// Get all objects referenced by this page (for extraction or analysis).
    ///
    /// This method recursively collects all objects referenced by the page, including:
    /// - Content streams
    /// - Resources (fonts, images, etc.)
    /// - Nested objects within resources
    ///
    /// This is useful for extracting a complete page with all its dependencies or
    /// for analyzing the object graph of a page.
    ///
    /// # Arguments
    ///
    /// * `reader` - Mutable reference to the PDF reader
    ///
    /// # Returns
    ///
    /// A HashMap mapping object references (obj_num, gen_num) to their resolved objects.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use oxidize_pdf::parser::{PdfReader, ParsedPage};
    /// # fn example(page: &ParsedPage, reader: &mut PdfReader<std::fs::File>) -> Result<(), Box<dyn std::error::Error>> {
    /// let referenced_objects = page.get_referenced_objects(reader)?;
    ///
    /// println!("Page references {} objects", referenced_objects.len());
    /// for ((obj_num, gen_num), obj) in &referenced_objects {
    ///     println!("  {} {} R: {:?}", obj_num, gen_num, obj);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn get_referenced_objects<R: Read + Seek>(
        &self,
        reader: &mut PdfReader<R>,
    ) -> ParseResult<HashMap<(u32, u16), PdfObject>> {
        let mut objects = HashMap::new();
        let mut to_process = Vec::new();

        // Start with Contents
        if let Some(contents) = self.dict.get("Contents") {
            Self::collect_references(contents, &mut to_process);
        }

        // Add Resources
        if let Some(resources) = self.get_resources() {
            for value in resources.0.values() {
                Self::collect_references(value, &mut to_process);
            }
        }

        // Process all references
        while let Some((obj_num, gen_num)) = to_process.pop() {
            if let std::collections::hash_map::Entry::Vacant(e) = objects.entry((obj_num, gen_num))
            {
                let obj = reader.get_object(obj_num, gen_num)?;

                // Collect nested references
                Self::collect_references_from_object(obj, &mut to_process);

                e.insert(obj.clone());
            }
        }

        Ok(objects)
    }

    /// Collect object references from a PDF object
    fn collect_references(obj: &PdfObject, refs: &mut Vec<(u32, u16)>) {
        match obj {
            PdfObject::Reference(obj_num, gen_num) => {
                refs.push((*obj_num, *gen_num));
            }
            PdfObject::Array(array) => {
                for item in &array.0 {
                    Self::collect_references(item, refs);
                }
            }
            PdfObject::Dictionary(dict) => {
                for value in dict.0.values() {
                    Self::collect_references(value, refs);
                }
            }
            _ => {}
        }
    }

    /// Collect references from an object (after resolution)
    fn collect_references_from_object(obj: &PdfObject, refs: &mut Vec<(u32, u16)>) {
        match obj {
            PdfObject::Array(array) => {
                for item in &array.0 {
                    Self::collect_references(item, refs);
                }
            }
            PdfObject::Dictionary(dict) | PdfObject::Stream(PdfStream { dict, .. }) => {
                for value in dict.0.values() {
                    Self::collect_references(value, refs);
                }
            }
            _ => {}
        }
    }
}

#[cfg(test)]
#[path = "page_tree_tests.rs"]
mod page_tree_tests;
