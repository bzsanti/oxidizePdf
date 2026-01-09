//! Type0 (Composite) Font Parsing Utilities
//!
//! This module provides detection and extraction utilities for Type0 fonts
//! per ISO 32000-1 Section 9.7 (CIDFonts and CMaps).
//!
//! ## Type0 Font Hierarchy
//!
//! ```text
//! Type0 Font Dict
//!   ├── /Subtype /Type0
//!   ├── /DescendantFonts [ ref_to_CIDFont ]
//!   └── /ToUnicode (optional) ref_to_CMap
//!
//! CIDFont Dict
//!   ├── /Subtype /CIDFontType0 or /CIDFontType2
//!   ├── /CIDSystemInfo { Registry, Ordering, Supplement }
//!   ├── /FontDescriptor ref_to_descriptor
//!   └── /W (widths) or /DW (default width)
//!
//! FontDescriptor Dict
//!   └── /FontFile2 (TrueType) or /FontFile3 (CFF)
//! ```
//!
//! ## Usage
//!
//! ```rust,ignore
//! use oxidize_pdf::fonts::type0_parsing::{detect_type0_font, extract_descendant_fonts_ref};
//! use oxidize_pdf::pdf_objects::Dictionary;
//!
//! let font_dict: Dictionary = /* from PDF */;
//!
//! if detect_type0_font(&font_dict) {
//!     if let Some(refs) = extract_descendant_fonts_ref(&font_dict) {
//!         // refs contains ObjectIds to CIDFont dictionaries
//!     }
//! }
//! ```

use crate::pdf_objects::{Dictionary, Object, ObjectId, Stream};

// =============================================================================
// Security Constants
// =============================================================================

/// Maximum font stream size (10MB) to prevent zip bombs and memory exhaustion.
/// Real-world embedded fonts rarely exceed 5MB.
pub const MAX_FONT_STREAM_SIZE: usize = 10 * 1024 * 1024;

// =============================================================================
// Phase 1: Detection Types and Functions
// =============================================================================

/// CIDFont subtype indicating outline format
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CIDFontSubtype {
    /// CIDFontType0 - CFF (Compact Font Format) outlines
    /// Used for PostScript-based CID fonts
    Type0,
    /// CIDFontType2 - TrueType outlines
    /// Used for TrueType-based CID fonts (most common)
    Type2,
}

/// Detect if a font dictionary is a Type0 (composite) font
///
/// A Type0 font has `/Subtype /Type0` in its dictionary.
///
/// # Arguments
/// * `dict` - The font dictionary to check
///
/// # Returns
/// `true` if the dictionary represents a Type0 font, `false` otherwise
///
/// # Example
/// ```rust,ignore
/// use oxidize_pdf::fonts::type0_parsing::detect_type0_font;
///
/// if detect_type0_font(&font_dict) {
///     println!("This is a Type0 composite font");
/// }
/// ```
pub fn detect_type0_font(dict: &Dictionary) -> bool {
    // Check for /Subtype /Type0
    if let Some(Object::Name(subtype)) = dict.get("Subtype") {
        return subtype.as_str() == "Type0";
    }
    false
}

/// Extract the DescendantFonts references from a Type0 font dictionary
///
/// The `/DescendantFonts` entry is an array containing a single reference
/// to a CIDFont dictionary (per ISO 32000-1, Type0 fonts have exactly one descendant).
///
/// # Arguments
/// * `dict` - The Type0 font dictionary
///
/// # Returns
/// `Some(Vec<ObjectId>)` containing the descendant font references,
/// or `None` if the entry doesn't exist or is invalid.
///
/// # Note
/// While the spec requires exactly one descendant, this function returns
/// a Vec to handle malformed PDFs that might have multiple.
pub fn extract_descendant_fonts_ref(dict: &Dictionary) -> Option<Vec<ObjectId>> {
    let descendants = dict.get("DescendantFonts")?;

    match descendants {
        Object::Array(arr) => {
            let refs: Vec<ObjectId> = arr
                .iter()
                .filter_map(|obj| {
                    if let Object::Reference(id) = obj {
                        Some(*id)
                    } else {
                        None
                    }
                })
                .collect();

            if refs.is_empty() {
                None
            } else {
                Some(refs)
            }
        }
        // Some PDFs might inline the dictionary instead of using a reference
        Object::Reference(id) => Some(vec![*id]),
        _ => None,
    }
}

/// Detect the CIDFont subtype from a CIDFont dictionary
///
/// CIDFonts have either `/Subtype /CIDFontType0` (CFF outlines)
/// or `/Subtype /CIDFontType2` (TrueType outlines).
///
/// # Arguments
/// * `dict` - The CIDFont dictionary
///
/// # Returns
/// `Some(CIDFontSubtype)` if the dictionary is a valid CIDFont,
/// `None` if it's not a CIDFont or has an unknown subtype.
pub fn detect_cidfont_subtype(dict: &Dictionary) -> Option<CIDFontSubtype> {
    if let Some(Object::Name(subtype)) = dict.get("Subtype") {
        match subtype.as_str() {
            "CIDFontType0" => Some(CIDFontSubtype::Type0),
            "CIDFontType2" => Some(CIDFontSubtype::Type2),
            _ => None,
        }
    } else {
        None
    }
}

/// Extract the ToUnicode CMap reference from a font dictionary
///
/// The `/ToUnicode` entry provides a CMap that maps character codes
/// to Unicode values, essential for text extraction from Type0 fonts.
///
/// # Arguments
/// * `dict` - The font dictionary (Type0 or other)
///
/// # Returns
/// `Some(ObjectId)` if a ToUnicode reference exists,
/// `None` if the entry doesn't exist or is not a reference.
pub fn extract_tounicode_ref(dict: &Dictionary) -> Option<ObjectId> {
    if let Some(Object::Reference(id)) = dict.get("ToUnicode") {
        Some(*id)
    } else {
        None
    }
}

// =============================================================================
// Phase 2: Hierarchy Resolution Types and Functions
// =============================================================================

/// Font file type indicating the stream format
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FontFileType {
    /// FontFile - Type 1 font data (PostScript)
    Type1,
    /// FontFile2 - TrueType font data
    TrueType,
    /// FontFile3 - CFF or OpenType font data
    CFF,
}

/// Information about a resolved Type0 font hierarchy
#[derive(Debug, Clone)]
pub struct Type0FontInfo {
    /// The Type0 font dictionary
    pub type0_dict: Dictionary,
    /// The CIDFont dictionary (from DescendantFonts)
    pub cidfont_dict: Option<Dictionary>,
    /// CIDFont subtype (Type0=CFF, Type2=TrueType)
    pub cidfont_subtype: Option<CIDFontSubtype>,
    /// The FontDescriptor dictionary
    pub font_descriptor: Option<Dictionary>,
    /// The embedded font stream (FontFile, FontFile2, or FontFile3)
    pub font_stream: Option<Stream>,
    /// Type of embedded font file
    pub font_file_type: Option<FontFileType>,
    /// ToUnicode CMap stream (if present)
    pub tounicode_stream: Option<Stream>,
}

impl Type0FontInfo {
    /// Create a new Type0FontInfo with just the Type0 dictionary
    pub fn new(type0_dict: Dictionary) -> Self {
        Self {
            type0_dict,
            cidfont_dict: None,
            cidfont_subtype: None,
            font_descriptor: None,
            font_stream: None,
            font_file_type: None,
            tounicode_stream: None,
        }
    }

    /// Check if the font has embedded data
    pub fn has_embedded_font(&self) -> bool {
        self.font_stream.is_some()
    }

    /// Check if ToUnicode CMap is available
    pub fn has_tounicode(&self) -> bool {
        self.tounicode_stream.is_some()
    }
}

/// Extract the FontDescriptor reference from a font dictionary (CIDFont or simple font)
///
/// # Arguments
/// * `dict` - The font dictionary (CIDFont, Type1, TrueType, etc.)
///
/// # Returns
/// `Some(ObjectId)` if a FontDescriptor reference exists,
/// `None` if the entry doesn't exist or is not a reference.
pub fn extract_font_descriptor_ref(dict: &Dictionary) -> Option<ObjectId> {
    if let Some(Object::Reference(id)) = dict.get("FontDescriptor") {
        Some(*id)
    } else {
        None
    }
}

/// Extract the embedded font file reference from a FontDescriptor dictionary
///
/// Checks for FontFile, FontFile2, and FontFile3 entries (in that order).
///
/// # Arguments
/// * `descriptor` - The FontDescriptor dictionary
///
/// # Returns
/// `Some((ObjectId, FontFileType))` if a font file reference exists,
/// `None` if no font file is embedded.
pub fn extract_font_file_ref(descriptor: &Dictionary) -> Option<(ObjectId, FontFileType)> {
    // Check in order of precedence
    if let Some(Object::Reference(id)) = descriptor.get("FontFile") {
        return Some((*id, FontFileType::Type1));
    }
    if let Some(Object::Reference(id)) = descriptor.get("FontFile2") {
        return Some((*id, FontFileType::TrueType));
    }
    if let Some(Object::Reference(id)) = descriptor.get("FontFile3") {
        return Some((*id, FontFileType::CFF));
    }
    None
}

/// Extract the W (widths) array reference from a CIDFont dictionary
///
/// # Arguments
/// * `cidfont` - The CIDFont dictionary
///
/// # Returns
/// `Some(ObjectId)` if a W array reference exists,
/// `None` if the entry doesn't exist or is not a reference.
pub fn extract_widths_ref(cidfont: &Dictionary) -> Option<ObjectId> {
    if let Some(Object::Reference(id)) = cidfont.get("W") {
        Some(*id)
    } else {
        None
    }
}

/// Extract the default width (DW) from a CIDFont dictionary
///
/// # Arguments
/// * `cidfont` - The CIDFont dictionary
///
/// # Returns
/// The default width value, or 1000 if not specified (per ISO 32000-1)
pub fn extract_default_width(cidfont: &Dictionary) -> i64 {
    if let Some(obj) = cidfont.get("DW") {
        obj.as_integer().unwrap_or(1000)
    } else {
        1000 // Default per ISO 32000-1
    }
}

// =============================================================================
// Phase 3: Full Hierarchy Resolution
// =============================================================================

/// Resolve the complete Type0 font hierarchy using a resolver function
///
/// This function walks the entire Type0 font hierarchy:
/// `Type0 → DescendantFonts → CIDFont → FontDescriptor → FontFile`
///
/// It also resolves the optional ToUnicode CMap stream.
///
/// # Arguments
/// * `type0_dict` - The Type0 font dictionary
/// * `resolver` - A function that resolves ObjectId references to Objects
///
/// # Returns
/// `Some(Type0FontInfo)` with all resolved components, or `None` if not a Type0 font.
///
/// # Security
///
/// This function implements two security protections:
///
/// 1. **Circular reference detection**: Uses a HashSet to track visited ObjectIds.
///    If a circular reference is detected, the function returns partial info and
///    logs a warning via `tracing::warn`. This prevents infinite loops and stack
///    overflow from malicious PDFs.
///
/// 2. **Font stream size validation**: Rejects font streams larger than
///    [`MAX_FONT_STREAM_SIZE`] (10MB) to prevent zip bomb and memory exhaustion
///    attacks. Oversized streams are logged and `font_stream` is set to `None`.
///
/// # Example
/// ```rust,ignore
/// use oxidize_pdf::fonts::type0_parsing::resolve_type0_hierarchy;
///
/// let info = resolve_type0_hierarchy(&font_dict, |id| document.get_object(id));
/// if let Some(info) = info {
///     if info.has_embedded_font() {
///         // Process embedded font data
///     }
/// }
/// ```
pub fn resolve_type0_hierarchy<F>(type0_dict: &Dictionary, resolver: F) -> Option<Type0FontInfo>
where
    F: Fn(ObjectId) -> Option<Object>,
{
    use std::collections::HashSet;

    // Must be a Type0 font
    if !detect_type0_font(type0_dict) {
        return None;
    }

    let mut info = Type0FontInfo::new(type0_dict.clone());
    let mut visited = HashSet::new(); // Track visited ObjectIds for circular ref detection

    // Step 1: Resolve DescendantFonts → CIDFont
    if let Some(refs) = extract_descendant_fonts_ref(type0_dict) {
        // Type0 fonts have exactly one descendant (per ISO 32000-1)
        if let Some(cidfont_ref) = refs.first() {
            // Check for circular reference
            if visited.contains(cidfont_ref) {
                tracing::warn!("Circular reference detected at CIDFont {:?}", cidfont_ref);
                return Some(info);
            }
            visited.insert(*cidfont_ref);

            if let Some(Object::Dictionary(cidfont)) = resolver(*cidfont_ref) {
                // Detect CIDFont subtype
                info.cidfont_subtype = detect_cidfont_subtype(&cidfont);

                // Step 2: Resolve CIDFont → FontDescriptor
                if let Some(desc_ref) = extract_font_descriptor_ref(&cidfont) {
                    // Check for circular reference
                    if visited.contains(&desc_ref) {
                        tracing::warn!(
                            "Circular reference detected at FontDescriptor {:?}",
                            desc_ref
                        );
                        info.cidfont_dict = Some(cidfont);
                        return Some(info);
                    }
                    visited.insert(desc_ref);

                    if let Some(Object::Dictionary(descriptor)) = resolver(desc_ref) {
                        // Step 3: Resolve FontDescriptor → FontFile stream
                        if let Some((file_ref, file_type)) = extract_font_file_ref(&descriptor) {
                            // Check for circular reference
                            if visited.contains(&file_ref) {
                                tracing::warn!(
                                    "Circular reference detected at FontFile {:?}",
                                    file_ref
                                );
                                info.font_descriptor = Some(descriptor);
                                info.cidfont_dict = Some(cidfont);
                                return Some(info);
                            }
                            visited.insert(file_ref);

                            if let Some(Object::Stream(stream)) = resolver(file_ref) {
                                // Security: Validate stream size to prevent zip bombs
                                if stream.data.len() > MAX_FONT_STREAM_SIZE {
                                    tracing::warn!(
                                        "Font stream size {} exceeds limit {} for {:?}",
                                        stream.data.len(),
                                        MAX_FONT_STREAM_SIZE,
                                        file_ref
                                    );
                                    // Don't set font_stream/font_file_type for oversized streams
                                } else {
                                    info.font_stream = Some(stream);
                                    info.font_file_type = Some(file_type);
                                }
                            }
                        }

                        info.font_descriptor = Some(descriptor);
                    }
                }

                info.cidfont_dict = Some(cidfont);
            }
        }
    }

    // Step 4: Resolve ToUnicode CMap stream (optional but important for text extraction)
    if let Some(tounicode_ref) = extract_tounicode_ref(type0_dict) {
        // Check for circular reference (ToUnicode pointing to already-visited object)
        if visited.contains(&tounicode_ref) {
            tracing::warn!(
                "Circular reference detected at ToUnicode {:?}",
                tounicode_ref
            );
            return Some(info);
        }

        if let Some(Object::Stream(stream)) = resolver(tounicode_ref) {
            info.tounicode_stream = Some(stream);
        }
    }

    Some(info)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pdf_objects::{Array, Name};

    #[test]
    fn test_detect_type0_font() {
        let mut type0_dict = Dictionary::new();
        type0_dict.set("Subtype", Name::new("Type0"));

        assert!(detect_type0_font(&type0_dict));

        let mut type1_dict = Dictionary::new();
        type1_dict.set("Subtype", Name::new("Type1"));

        assert!(!detect_type0_font(&type1_dict));

        let empty_dict = Dictionary::new();
        assert!(!detect_type0_font(&empty_dict));
    }

    #[test]
    fn test_extract_descendant_fonts_ref() {
        let mut dict = Dictionary::new();
        let mut arr = Array::new();
        arr.push(Object::Reference(ObjectId::new(10, 0)));
        dict.set("DescendantFonts", Object::Array(arr));

        let result = extract_descendant_fonts_ref(&dict);
        assert!(result.is_some());
        let refs = result.unwrap();
        assert_eq!(refs.len(), 1);
        assert_eq!(refs[0], ObjectId::new(10, 0));

        // Empty array should return None
        let mut empty_arr_dict = Dictionary::new();
        empty_arr_dict.set("DescendantFonts", Object::Array(Array::new()));
        assert!(extract_descendant_fonts_ref(&empty_arr_dict).is_none());

        // No DescendantFonts should return None
        let no_descendants = Dictionary::new();
        assert!(extract_descendant_fonts_ref(&no_descendants).is_none());
    }

    #[test]
    fn test_detect_cidfont_subtype() {
        let mut type0_dict = Dictionary::new();
        type0_dict.set("Subtype", Name::new("CIDFontType0"));
        assert_eq!(
            detect_cidfont_subtype(&type0_dict),
            Some(CIDFontSubtype::Type0)
        );

        let mut type2_dict = Dictionary::new();
        type2_dict.set("Subtype", Name::new("CIDFontType2"));
        assert_eq!(
            detect_cidfont_subtype(&type2_dict),
            Some(CIDFontSubtype::Type2)
        );

        let mut truetype_dict = Dictionary::new();
        truetype_dict.set("Subtype", Name::new("TrueType"));
        assert_eq!(detect_cidfont_subtype(&truetype_dict), None);

        let empty_dict = Dictionary::new();
        assert_eq!(detect_cidfont_subtype(&empty_dict), None);
    }

    #[test]
    fn test_extract_tounicode_ref() {
        let mut dict = Dictionary::new();
        dict.set("ToUnicode", Object::Reference(ObjectId::new(20, 0)));

        assert_eq!(extract_tounicode_ref(&dict), Some(ObjectId::new(20, 0)));

        // No ToUnicode
        let empty_dict = Dictionary::new();
        assert!(extract_tounicode_ref(&empty_dict).is_none());

        // Wrong type (Name instead of Reference)
        let mut wrong_type = Dictionary::new();
        wrong_type.set("ToUnicode", Name::new("Identity-H"));
        assert!(extract_tounicode_ref(&wrong_type).is_none());
    }

    #[test]
    fn test_descendant_fonts_with_direct_reference() {
        // Some PDFs might have a direct reference instead of array
        let mut dict = Dictionary::new();
        dict.set("DescendantFonts", Object::Reference(ObjectId::new(15, 0)));

        let result = extract_descendant_fonts_ref(&dict);
        assert!(result.is_some());
        let refs = result.unwrap();
        assert_eq!(refs.len(), 1);
        assert_eq!(refs[0], ObjectId::new(15, 0));
    }

    // =========================================================================
    // Phase 2 Tests: Hierarchy Resolution
    // =========================================================================

    #[test]
    fn test_type0_font_info_new() {
        let mut dict = Dictionary::new();
        dict.set("Subtype", Name::new("Type0"));

        let info = Type0FontInfo::new(dict);

        assert!(info.cidfont_dict.is_none());
        assert!(info.cidfont_subtype.is_none());
        assert!(info.font_descriptor.is_none());
        assert!(info.font_stream.is_none());
        assert!(info.font_file_type.is_none());
        assert!(info.tounicode_stream.is_none());
        assert!(!info.has_embedded_font());
        assert!(!info.has_tounicode());
    }

    #[test]
    fn test_extract_font_descriptor_ref() {
        let mut dict = Dictionary::new();
        dict.set("FontDescriptor", Object::Reference(ObjectId::new(25, 0)));

        assert_eq!(
            extract_font_descriptor_ref(&dict),
            Some(ObjectId::new(25, 0))
        );

        // No FontDescriptor
        let empty_dict = Dictionary::new();
        assert!(extract_font_descriptor_ref(&empty_dict).is_none());

        // Wrong type
        let mut wrong_type = Dictionary::new();
        wrong_type.set("FontDescriptor", Name::new("SomeFont"));
        assert!(extract_font_descriptor_ref(&wrong_type).is_none());
    }

    #[test]
    fn test_extract_font_file_ref_truetype() {
        let mut descriptor = Dictionary::new();
        descriptor.set("FontFile2", Object::Reference(ObjectId::new(30, 0)));

        let result = extract_font_file_ref(&descriptor);
        assert!(result.is_some());
        let (id, file_type) = result.unwrap();
        assert_eq!(id, ObjectId::new(30, 0));
        assert_eq!(file_type, FontFileType::TrueType);
    }

    #[test]
    fn test_extract_font_file_ref_cff() {
        let mut descriptor = Dictionary::new();
        descriptor.set("FontFile3", Object::Reference(ObjectId::new(35, 0)));

        let result = extract_font_file_ref(&descriptor);
        assert!(result.is_some());
        let (id, file_type) = result.unwrap();
        assert_eq!(id, ObjectId::new(35, 0));
        assert_eq!(file_type, FontFileType::CFF);
    }

    #[test]
    fn test_extract_font_file_ref_type1() {
        let mut descriptor = Dictionary::new();
        descriptor.set("FontFile", Object::Reference(ObjectId::new(40, 0)));

        let result = extract_font_file_ref(&descriptor);
        assert!(result.is_some());
        let (id, file_type) = result.unwrap();
        assert_eq!(id, ObjectId::new(40, 0));
        assert_eq!(file_type, FontFileType::Type1);
    }

    #[test]
    fn test_extract_font_file_ref_precedence() {
        // If multiple FontFile entries exist, FontFile takes precedence
        let mut descriptor = Dictionary::new();
        descriptor.set("FontFile", Object::Reference(ObjectId::new(1, 0)));
        descriptor.set("FontFile2", Object::Reference(ObjectId::new(2, 0)));
        descriptor.set("FontFile3", Object::Reference(ObjectId::new(3, 0)));

        let result = extract_font_file_ref(&descriptor);
        assert!(result.is_some());
        let (id, file_type) = result.unwrap();
        assert_eq!(id, ObjectId::new(1, 0));
        assert_eq!(file_type, FontFileType::Type1);
    }

    #[test]
    fn test_extract_font_file_ref_none() {
        let empty_descriptor = Dictionary::new();
        assert!(extract_font_file_ref(&empty_descriptor).is_none());
    }

    #[test]
    fn test_extract_widths_ref() {
        let mut cidfont = Dictionary::new();
        cidfont.set("W", Object::Reference(ObjectId::new(50, 0)));

        assert_eq!(extract_widths_ref(&cidfont), Some(ObjectId::new(50, 0)));

        // No W entry
        let empty_dict = Dictionary::new();
        assert!(extract_widths_ref(&empty_dict).is_none());

        // W is inline array (not reference)
        let mut inline_w = Dictionary::new();
        inline_w.set("W", Object::Array(Array::new()));
        assert!(extract_widths_ref(&inline_w).is_none());
    }

    #[test]
    fn test_extract_default_width() {
        let mut cidfont = Dictionary::new();
        cidfont.set("DW", Object::Integer(500));

        assert_eq!(extract_default_width(&cidfont), 500);

        // No DW entry - should return 1000 (ISO default)
        let empty_dict = Dictionary::new();
        assert_eq!(extract_default_width(&empty_dict), 1000);

        // Wrong type (should use default)
        let mut wrong_type = Dictionary::new();
        wrong_type.set("DW", Name::new("invalid"));
        assert_eq!(extract_default_width(&wrong_type), 1000);
    }

    #[test]
    fn test_font_file_type_equality() {
        assert_eq!(FontFileType::Type1, FontFileType::Type1);
        assert_eq!(FontFileType::TrueType, FontFileType::TrueType);
        assert_eq!(FontFileType::CFF, FontFileType::CFF);
        assert_ne!(FontFileType::Type1, FontFileType::TrueType);
        assert_ne!(FontFileType::TrueType, FontFileType::CFF);
    }

    // =========================================================================
    // Phase 3 Tests: Full Hierarchy Resolution
    // =========================================================================

    use std::collections::HashMap;

    /// Helper to create a mock object store for testing hierarchy resolution
    fn create_mock_object_store() -> HashMap<ObjectId, Object> {
        let mut store = HashMap::new();

        // CIDFont dictionary (object 15)
        let mut cidfont = Dictionary::new();
        cidfont.set("Type", Name::new("Font"));
        cidfont.set("Subtype", Name::new("CIDFontType2"));
        cidfont.set("BaseFont", Name::new("Arial-Bold"));
        cidfont.set("FontDescriptor", Object::Reference(ObjectId::new(16, 0)));
        cidfont.set("DW", Object::Integer(1000));
        store.insert(ObjectId::new(15, 0), Object::Dictionary(cidfont));

        // FontDescriptor dictionary (object 16)
        let mut descriptor = Dictionary::new();
        descriptor.set("Type", Name::new("FontDescriptor"));
        descriptor.set("FontName", Name::new("Arial-Bold"));
        descriptor.set("FontFile2", Object::Reference(ObjectId::new(17, 0)));
        store.insert(ObjectId::new(16, 0), Object::Dictionary(descriptor));

        // FontFile2 stream (object 17)
        let font_stream = Stream::new(
            Dictionary::new(),
            vec![0x00, 0x01, 0x00, 0x00], // TTF magic bytes
        );
        store.insert(ObjectId::new(17, 0), Object::Stream(font_stream));

        // ToUnicode CMap stream (object 20)
        let tounicode_stream = Stream::new(
            Dictionary::new(),
            b"/CIDInit /ProcSet findresource begin".to_vec(),
        );
        store.insert(ObjectId::new(20, 0), Object::Stream(tounicode_stream));

        store
    }

    /// Helper to create a Type0 font dictionary for testing
    fn create_test_type0_dict() -> Dictionary {
        let mut dict = Dictionary::new();
        dict.set("Type", Name::new("Font"));
        dict.set("Subtype", Name::new("Type0"));
        dict.set("BaseFont", Name::new("Arial-Bold"));
        dict.set("Encoding", Name::new("Identity-H"));

        let mut descendant_array = Array::new();
        descendant_array.push(Object::Reference(ObjectId::new(15, 0)));
        dict.set("DescendantFonts", Object::Array(descendant_array));

        dict.set("ToUnicode", Object::Reference(ObjectId::new(20, 0)));

        dict
    }

    #[test]
    fn test_resolve_type0_hierarchy_complete() {
        use super::resolve_type0_hierarchy;

        let type0_dict = create_test_type0_dict();
        let store = create_mock_object_store();

        // Create resolver closure
        let resolver = |id: ObjectId| -> Option<Object> { store.get(&id).cloned() };

        let result = resolve_type0_hierarchy(&type0_dict, resolver);
        assert!(result.is_some(), "Should resolve complete hierarchy");

        let info = result.unwrap();

        // Verify CIDFont was resolved
        assert!(
            info.cidfont_dict.is_some(),
            "Should have CIDFont dictionary"
        );
        assert_eq!(
            info.cidfont_subtype,
            Some(CIDFontSubtype::Type2),
            "Should detect CIDFontType2"
        );

        // Verify FontDescriptor was resolved
        assert!(info.font_descriptor.is_some(), "Should have FontDescriptor");

        // Verify font stream was resolved
        assert!(info.font_stream.is_some(), "Should have font stream");
        assert_eq!(
            info.font_file_type,
            Some(FontFileType::TrueType),
            "Should be TrueType"
        );

        // Verify ToUnicode was resolved
        assert!(
            info.tounicode_stream.is_some(),
            "Should have ToUnicode stream"
        );
    }

    #[test]
    fn test_resolve_type0_hierarchy_missing_cidfont() {
        use super::resolve_type0_hierarchy;

        let type0_dict = create_test_type0_dict();
        let store: HashMap<ObjectId, Object> = HashMap::new(); // Empty store

        let resolver = |id: ObjectId| -> Option<Object> { store.get(&id).cloned() };

        let result = resolve_type0_hierarchy(&type0_dict, resolver);
        assert!(
            result.is_some(),
            "Should return partial info even with missing refs"
        );

        let info = result.unwrap();
        assert!(info.cidfont_dict.is_none(), "CIDFont should be None");
        assert!(
            info.font_descriptor.is_none(),
            "FontDescriptor should be None"
        );
        assert!(info.font_stream.is_none(), "Font stream should be None");
    }

    #[test]
    fn test_resolve_type0_hierarchy_partial_chain() {
        use super::resolve_type0_hierarchy;

        let type0_dict = create_test_type0_dict();
        let mut store = HashMap::new();

        // Only add CIDFont, not FontDescriptor
        let mut cidfont = Dictionary::new();
        cidfont.set("Subtype", Name::new("CIDFontType2"));
        cidfont.set("FontDescriptor", Object::Reference(ObjectId::new(16, 0)));
        store.insert(ObjectId::new(15, 0), Object::Dictionary(cidfont));

        let resolver = |id: ObjectId| -> Option<Object> { store.get(&id).cloned() };

        let result = resolve_type0_hierarchy(&type0_dict, resolver);
        assert!(result.is_some());

        let info = result.unwrap();
        assert!(info.cidfont_dict.is_some(), "CIDFont should be resolved");
        assert_eq!(info.cidfont_subtype, Some(CIDFontSubtype::Type2));
        assert!(
            info.font_descriptor.is_none(),
            "FontDescriptor should be None"
        );
        assert!(info.font_stream.is_none(), "Font stream should be None");
    }

    #[test]
    fn test_resolve_type0_hierarchy_cff_font() {
        use super::resolve_type0_hierarchy;

        let type0_dict = create_test_type0_dict();
        let mut store = HashMap::new();

        // CIDFont with CFF outlines
        let mut cidfont = Dictionary::new();
        cidfont.set("Subtype", Name::new("CIDFontType0"));
        cidfont.set("FontDescriptor", Object::Reference(ObjectId::new(16, 0)));
        store.insert(ObjectId::new(15, 0), Object::Dictionary(cidfont));

        // FontDescriptor with FontFile3 (CFF)
        let mut descriptor = Dictionary::new();
        descriptor.set("FontFile3", Object::Reference(ObjectId::new(17, 0)));
        store.insert(ObjectId::new(16, 0), Object::Dictionary(descriptor));

        // CFF font stream
        let cff_stream = Stream::new(Dictionary::new(), vec![0x01, 0x00, 0x04, 0x00]);
        store.insert(ObjectId::new(17, 0), Object::Stream(cff_stream));

        let resolver = |id: ObjectId| -> Option<Object> { store.get(&id).cloned() };

        let result = resolve_type0_hierarchy(&type0_dict, resolver);
        assert!(result.is_some());

        let info = result.unwrap();
        assert_eq!(info.cidfont_subtype, Some(CIDFontSubtype::Type0));
        assert_eq!(info.font_file_type, Some(FontFileType::CFF));
        assert!(info.font_stream.is_some());
    }

    #[test]
    fn test_resolve_type0_hierarchy_not_type0_font() {
        use super::resolve_type0_hierarchy;

        // Type1 font (not Type0)
        let mut type1_dict = Dictionary::new();
        type1_dict.set("Subtype", Name::new("Type1"));
        type1_dict.set("BaseFont", Name::new("Helvetica"));

        let store = create_mock_object_store();
        let resolver = |id: ObjectId| -> Option<Object> { store.get(&id).cloned() };

        let result = resolve_type0_hierarchy(&type1_dict, resolver);
        assert!(result.is_none(), "Should return None for non-Type0 font");
    }

    // =========================================================================
    // Phase 6 Tests: Edge Cases
    // =========================================================================

    #[test]
    fn test_multiple_descendant_fonts() {
        // Malformed PDF with multiple descendants (ISO 32000-1 says only one)
        let mut dict = Dictionary::new();
        dict.set("Subtype", Name::new("Type0"));

        let mut arr = Array::new();
        arr.push(Object::Reference(ObjectId::new(10, 0)));
        arr.push(Object::Reference(ObjectId::new(11, 0)));
        arr.push(Object::Reference(ObjectId::new(12, 0)));
        dict.set("DescendantFonts", Object::Array(arr));

        // Should extract all references (graceful handling of malformed PDFs)
        let result = extract_descendant_fonts_ref(&dict);
        assert!(result.is_some());
        let refs = result.unwrap();
        assert_eq!(refs.len(), 3, "Should extract all descendant refs");
    }

    #[test]
    fn test_descendant_fonts_with_inline_dict() {
        // Some malformed PDFs might have inline dicts instead of references
        let mut dict = Dictionary::new();
        dict.set("Subtype", Name::new("Type0"));

        let mut arr = Array::new();
        // Inline dictionary instead of reference
        let mut inline_cidfont = Dictionary::new();
        inline_cidfont.set("Subtype", Name::new("CIDFontType2"));
        arr.push(Object::Dictionary(inline_cidfont));
        dict.set("DescendantFonts", Object::Array(arr));

        // Should return None because we only extract references
        let result = extract_descendant_fonts_ref(&dict);
        assert!(
            result.is_none(),
            "Inline dicts should not be extracted as refs"
        );
    }

    #[test]
    fn test_wrong_object_type_in_subtype() {
        let mut dict = Dictionary::new();
        // Subtype is Integer instead of Name
        dict.set("Subtype", Object::Integer(0));

        assert!(!detect_type0_font(&dict));
        assert!(detect_cidfont_subtype(&dict).is_none());
    }

    #[test]
    fn test_descendant_fonts_wrong_type() {
        let mut dict = Dictionary::new();
        dict.set("Subtype", Name::new("Type0"));
        // DescendantFonts is Integer instead of Array or Reference
        dict.set("DescendantFonts", Object::Integer(15));

        assert!(extract_descendant_fonts_ref(&dict).is_none());
    }

    #[test]
    fn test_resolver_returns_wrong_object_type() {
        use super::resolve_type0_hierarchy;

        let type0_dict = create_test_type0_dict();
        let mut store = HashMap::new();

        // CIDFont reference points to Integer instead of Dictionary
        store.insert(ObjectId::new(15, 0), Object::Integer(42));

        let resolver = |id: ObjectId| -> Option<Object> { store.get(&id).cloned() };

        let result = resolve_type0_hierarchy(&type0_dict, resolver);
        assert!(result.is_some()); // Should return partial info

        let info = result.unwrap();
        assert!(
            info.cidfont_dict.is_none(),
            "CIDFont should be None when resolved to wrong type"
        );
    }

    #[test]
    fn test_font_descriptor_returns_wrong_type() {
        use super::resolve_type0_hierarchy;

        let type0_dict = create_test_type0_dict();
        let mut store = HashMap::new();

        // Valid CIDFont
        let mut cidfont = Dictionary::new();
        cidfont.set("Subtype", Name::new("CIDFontType2"));
        cidfont.set("FontDescriptor", Object::Reference(ObjectId::new(16, 0)));
        store.insert(ObjectId::new(15, 0), Object::Dictionary(cidfont));

        // FontDescriptor reference points to Stream instead of Dictionary
        let stream = Stream::new(Dictionary::new(), vec![0x00]);
        store.insert(ObjectId::new(16, 0), Object::Stream(stream));

        let resolver = |id: ObjectId| -> Option<Object> { store.get(&id).cloned() };

        let result = resolve_type0_hierarchy(&type0_dict, resolver);
        assert!(result.is_some());

        let info = result.unwrap();
        assert!(info.cidfont_dict.is_some(), "CIDFont should be resolved");
        assert!(
            info.font_descriptor.is_none(),
            "FontDescriptor should be None when wrong type"
        );
    }

    #[test]
    fn test_font_file_returns_wrong_type() {
        use super::resolve_type0_hierarchy;

        let type0_dict = create_test_type0_dict();
        let mut store = HashMap::new();

        // Valid CIDFont
        let mut cidfont = Dictionary::new();
        cidfont.set("Subtype", Name::new("CIDFontType2"));
        cidfont.set("FontDescriptor", Object::Reference(ObjectId::new(16, 0)));
        store.insert(ObjectId::new(15, 0), Object::Dictionary(cidfont));

        // Valid FontDescriptor
        let mut descriptor = Dictionary::new();
        descriptor.set("FontFile2", Object::Reference(ObjectId::new(17, 0)));
        store.insert(ObjectId::new(16, 0), Object::Dictionary(descriptor));

        // FontFile2 reference points to Dictionary instead of Stream
        let wrong_dict = Dictionary::new();
        store.insert(ObjectId::new(17, 0), Object::Dictionary(wrong_dict));

        let resolver = |id: ObjectId| -> Option<Object> { store.get(&id).cloned() };

        let result = resolve_type0_hierarchy(&type0_dict, resolver);
        assert!(result.is_some());

        let info = result.unwrap();
        assert!(info.font_descriptor.is_some());
        assert!(
            info.font_stream.is_none(),
            "Font stream should be None when wrong type"
        );
        assert!(info.font_file_type.is_none());
    }

    #[test]
    fn test_tounicode_returns_wrong_type() {
        use super::resolve_type0_hierarchy;

        let type0_dict = create_test_type0_dict();
        let mut store = HashMap::new();

        // ToUnicode reference points to Dictionary instead of Stream
        let wrong_dict = Dictionary::new();
        store.insert(ObjectId::new(20, 0), Object::Dictionary(wrong_dict));

        let resolver = |id: ObjectId| -> Option<Object> { store.get(&id).cloned() };

        let result = resolve_type0_hierarchy(&type0_dict, resolver);
        assert!(result.is_some());

        let info = result.unwrap();
        assert!(
            info.tounicode_stream.is_none(),
            "ToUnicode should be None when wrong type"
        );
    }

    #[test]
    fn test_cidfont_subtype_clone_copy() {
        let subtype = CIDFontSubtype::Type2;
        let cloned = subtype;
        assert_eq!(subtype, cloned);

        let subtype2 = CIDFontSubtype::Type0;
        assert_ne!(subtype, subtype2);
    }

    #[test]
    fn test_font_file_type_clone_copy() {
        let file_type = FontFileType::TrueType;
        let cloned = file_type;
        assert_eq!(file_type, cloned);
    }

    #[test]
    fn test_type0_font_info_clone() {
        let mut dict = Dictionary::new();
        dict.set("Subtype", Name::new("Type0"));

        let info = Type0FontInfo::new(dict);
        let cloned = info.clone();

        assert!(!cloned.has_embedded_font());
        assert!(!cloned.has_tounicode());
    }

    #[test]
    fn test_empty_descendant_array_edge_case() {
        let mut dict = Dictionary::new();
        dict.set("Subtype", Name::new("Type0"));
        dict.set("DescendantFonts", Object::Array(Array::new()));

        assert!(extract_descendant_fonts_ref(&dict).is_none());
    }

    #[test]
    fn test_mixed_array_with_refs_and_other_types() {
        let mut dict = Dictionary::new();
        dict.set("Subtype", Name::new("Type0"));

        let mut arr = Array::new();
        arr.push(Object::Reference(ObjectId::new(10, 0)));
        arr.push(Object::Integer(42)); // Not a reference
        arr.push(Object::Reference(ObjectId::new(11, 0)));
        arr.push(Object::Name(Name::new("SomeName"))); // Not a reference
        dict.set("DescendantFonts", Object::Array(arr));

        // Should only extract the references
        let result = extract_descendant_fonts_ref(&dict);
        assert!(result.is_some());
        let refs = result.unwrap();
        assert_eq!(refs.len(), 2, "Should only extract valid references");
        assert_eq!(refs[0], ObjectId::new(10, 0));
        assert_eq!(refs[1], ObjectId::new(11, 0));
    }
}
