//! TDD Tests for Type0 (Composite) Font Parsing
//!
//! These tests verify detection and resolution of Type0 font structures
//! per ISO 32000-1 Section 9.7.
//!
//! Type0 hierarchy:
//! Type0 → DescendantFonts → CIDFont → FontDescriptor → FontFile2 → Stream
//!
//! Phase 1-4: Detection and hierarchy resolution tests
//! Phase 5: Integration tests for resolve_type0_hierarchy

use std::collections::HashMap;

use oxidize_pdf::pdf_objects::{Array, Dictionary, Name, Object, ObjectId, Stream};

// Import all type0_parsing functions
use oxidize_pdf::fonts::type0_parsing::{
    detect_cidfont_subtype, detect_type0_font, extract_descendant_fonts_ref, extract_tounicode_ref,
    resolve_type0_hierarchy, CIDFontSubtype, FontFileType,
};

/// Helper to create a mock Type0 font dictionary
fn create_type0_font_dict() -> Dictionary {
    let mut dict = Dictionary::new();
    dict.set("Type", Name::new("Font"));
    dict.set("Subtype", Name::new("Type0"));
    dict.set("BaseFont", Name::new("Arial-Bold"));
    dict.set("Encoding", Name::new("Identity-H"));

    // DescendantFonts is an array with a single reference to a CIDFont
    let mut descendant_array = Array::new();
    descendant_array.push(Object::Reference(ObjectId::new(15, 0)));
    dict.set("DescendantFonts", Object::Array(descendant_array));

    // ToUnicode CMap reference
    dict.set("ToUnicode", Object::Reference(ObjectId::new(20, 0)));

    dict
}

/// Helper to create a mock CIDFont dictionary (CIDFontType2 - TrueType outlines)
fn create_cidfont_type2_dict() -> Dictionary {
    let mut dict = Dictionary::new();
    dict.set("Type", Name::new("Font"));
    dict.set("Subtype", Name::new("CIDFontType2"));
    dict.set("BaseFont", Name::new("Arial-Bold"));

    // CIDSystemInfo (required)
    let mut cid_system_info = Dictionary::new();
    cid_system_info.set("Registry", "Adobe");
    cid_system_info.set("Ordering", "Identity");
    cid_system_info.set("Supplement", 0i32);
    dict.set("CIDSystemInfo", Object::Dictionary(cid_system_info));

    // FontDescriptor reference
    dict.set("FontDescriptor", Object::Reference(ObjectId::new(16, 0)));

    // W (widths) array reference
    dict.set("W", Object::Reference(ObjectId::new(17, 0)));

    dict
}

/// Helper to create a mock CIDFont dictionary (CIDFontType0 - CFF outlines)
fn create_cidfont_type0_dict() -> Dictionary {
    let mut dict = Dictionary::new();
    dict.set("Type", Name::new("Font"));
    dict.set("Subtype", Name::new("CIDFontType0"));
    dict.set("BaseFont", Name::new("KozMinPro-Regular"));

    // CIDSystemInfo (required)
    let mut cid_system_info = Dictionary::new();
    cid_system_info.set("Registry", "Adobe");
    cid_system_info.set("Ordering", "Japan1");
    cid_system_info.set("Supplement", 6i32);
    dict.set("CIDSystemInfo", Object::Dictionary(cid_system_info));

    // FontDescriptor reference
    dict.set("FontDescriptor", Object::Reference(ObjectId::new(25, 0)));

    dict
}

/// Helper to create a mock Type1 font dictionary (NOT Type0)
fn create_type1_font_dict() -> Dictionary {
    let mut dict = Dictionary::new();
    dict.set("Type", Name::new("Font"));
    dict.set("Subtype", Name::new("Type1"));
    dict.set("BaseFont", Name::new("Helvetica"));
    dict
}

// =============================================================================
// Test 1: Detect Type0 font from dictionary
// =============================================================================

/// Test that we can detect a Type0 font by checking `/Subtype /Type0`
#[test]
fn test_detect_type0_font_from_dict() {
    let type0_dict = create_type0_font_dict();
    let type1_dict = create_type1_font_dict();

    // Should return true for Type0 font
    assert!(
        detect_type0_font(&type0_dict),
        "Should detect Type0 font from /Subtype /Type0"
    );

    // Should return false for Type1 font
    assert!(
        !detect_type0_font(&type1_dict),
        "Should NOT detect Type1 font as Type0"
    );

    // Should return false for empty dictionary
    let empty_dict = Dictionary::new();
    assert!(
        !detect_type0_font(&empty_dict),
        "Should NOT detect empty dict as Type0"
    );
}

// =============================================================================
// Test 2: Extract DescendantFonts reference
// =============================================================================

/// Test extraction of `/DescendantFonts` array reference
#[test]
fn test_extract_descendant_fonts_reference() {
    let type0_dict = create_type0_font_dict();

    // Should extract the reference to the CIDFont
    let result = extract_descendant_fonts_ref(&type0_dict);
    assert!(result.is_some(), "Should extract DescendantFonts reference");

    let refs = result.unwrap();
    assert_eq!(refs.len(), 1, "Should have exactly one descendant font");
    assert_eq!(
        refs[0],
        ObjectId::new(15, 0),
        "Should extract correct object reference"
    );

    // Type1 font should return None (no DescendantFonts)
    let type1_dict = create_type1_font_dict();
    assert!(
        extract_descendant_fonts_ref(&type1_dict).is_none(),
        "Type1 font should have no DescendantFonts"
    );
}

// =============================================================================
// Test 3: Detect CIDFont subtype (CIDFontType0 vs CIDFontType2)
// =============================================================================

/// Test detection of CIDFont subtype (CFF vs TrueType outlines)
#[test]
fn test_detect_cidfont_subtype() {
    let cidfont_type0 = create_cidfont_type0_dict();
    let cidfont_type2 = create_cidfont_type2_dict();

    // CIDFontType0 = CFF outlines (PostScript-based)
    assert_eq!(
        detect_cidfont_subtype(&cidfont_type0),
        Some(CIDFontSubtype::Type0),
        "Should detect CIDFontType0 (CFF outlines)"
    );

    // CIDFontType2 = TrueType outlines
    assert_eq!(
        detect_cidfont_subtype(&cidfont_type2),
        Some(CIDFontSubtype::Type2),
        "Should detect CIDFontType2 (TrueType outlines)"
    );

    // Type1 font should return None (not a CIDFont)
    let type1_dict = create_type1_font_dict();
    assert_eq!(
        detect_cidfont_subtype(&type1_dict),
        None,
        "Type1 font should not be detected as CIDFont"
    );
}

// =============================================================================
// Test 4: Extract ToUnicode CMap reference
// =============================================================================

/// Test extraction of `/ToUnicode` CMap reference
#[test]
fn test_extract_tounicode_cmap_reference() {
    let type0_dict = create_type0_font_dict();

    // Should extract ToUnicode reference
    let result = extract_tounicode_ref(&type0_dict);
    assert!(result.is_some(), "Should extract ToUnicode reference");
    assert_eq!(
        result.unwrap(),
        ObjectId::new(20, 0),
        "Should extract correct ToUnicode object reference"
    );

    // Type1 font without ToUnicode should return None
    let type1_dict = create_type1_font_dict();
    assert!(
        extract_tounicode_ref(&type1_dict).is_none(),
        "Type1 font without ToUnicode should return None"
    );

    // Empty dict should return None
    let empty_dict = Dictionary::new();
    assert!(
        extract_tounicode_ref(&empty_dict).is_none(),
        "Empty dict should return None for ToUnicode"
    );
}

// =============================================================================
// Phase 5: Integration Tests for resolve_type0_hierarchy
// =============================================================================

/// Helper to create a complete mock PDF object store for testing hierarchy resolution
fn create_complete_object_store() -> HashMap<ObjectId, Object> {
    let mut store = HashMap::new();

    // CIDFont dictionary (object 15) - TrueType based
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
    descriptor.set("Flags", Object::Integer(262148));
    descriptor.set(
        "FontBBox",
        Object::Array({
            let mut arr = Array::new();
            arr.push(Object::Integer(-665));
            arr.push(Object::Integer(-210));
            arr.push(Object::Integer(2000));
            arr.push(Object::Integer(1006));
            arr
        }),
    );
    descriptor.set("ItalicAngle", Object::Integer(0));
    descriptor.set("Ascent", Object::Integer(905));
    descriptor.set("Descent", Object::Integer(-212));
    descriptor.set("CapHeight", Object::Integer(728));
    descriptor.set("StemV", Object::Integer(80));
    descriptor.set("FontFile2", Object::Reference(ObjectId::new(17, 0)));
    store.insert(ObjectId::new(16, 0), Object::Dictionary(descriptor));

    // FontFile2 stream (object 17) - TrueType font data
    let font_stream = Stream::new(
        Dictionary::new(),
        vec![0x00, 0x01, 0x00, 0x00], // TTF magic bytes
    );
    store.insert(ObjectId::new(17, 0), Object::Stream(font_stream));

    // ToUnicode CMap stream (object 20)
    let tounicode_stream = Stream::new(
        Dictionary::new(),
        b"/CIDInit /ProcSet findresource begin\n\
          12 dict begin\n\
          begincmap\n\
          /CIDSystemInfo <<\n\
            /Registry (Adobe)\n\
            /Ordering (UCS)\n\
            /Supplement 0\n\
          >> def\n\
          /CMapName /Adobe-Identity-UCS def\n\
          endcmap\n\
          end end"
            .to_vec(),
    );
    store.insert(ObjectId::new(20, 0), Object::Stream(tounicode_stream));

    store
}

/// Integration test: Resolve complete Type0 font hierarchy
#[test]
fn test_integration_resolve_type0_hierarchy_complete() {
    let type0_dict = create_type0_font_dict();
    let store = create_complete_object_store();

    // Create resolver closure that simulates document.get_object()
    let resolver = |id: ObjectId| -> Option<Object> { store.get(&id).cloned() };

    let result = resolve_type0_hierarchy(&type0_dict, resolver);
    assert!(
        result.is_some(),
        "Should successfully resolve Type0 hierarchy"
    );

    let info = result.unwrap();

    // Verify the entire chain was resolved
    assert!(info.cidfont_dict.is_some(), "CIDFont should be resolved");
    assert_eq!(
        info.cidfont_subtype,
        Some(CIDFontSubtype::Type2),
        "Should be CIDFontType2 (TrueType)"
    );
    assert!(
        info.font_descriptor.is_some(),
        "FontDescriptor should be resolved"
    );
    assert!(info.font_stream.is_some(), "Font stream should be resolved");
    assert_eq!(
        info.font_file_type,
        Some(FontFileType::TrueType),
        "Should be TrueType font"
    );
    assert!(
        info.tounicode_stream.is_some(),
        "ToUnicode stream should be resolved"
    );

    // Verify helper methods
    assert!(info.has_embedded_font(), "Should report embedded font");
    assert!(info.has_tounicode(), "Should report ToUnicode CMap");
}

/// Integration test: Resolve Type0 with missing intermediate objects
#[test]
fn test_integration_resolve_type0_hierarchy_partial() {
    let type0_dict = create_type0_font_dict();

    // Object store with only CIDFont - no FontDescriptor or font stream
    let mut partial_store = HashMap::new();
    let mut cidfont = Dictionary::new();
    cidfont.set("Subtype", Name::new("CIDFontType2"));
    cidfont.set("FontDescriptor", Object::Reference(ObjectId::new(16, 0)));
    partial_store.insert(ObjectId::new(15, 0), Object::Dictionary(cidfont));

    let resolver = |id: ObjectId| -> Option<Object> { partial_store.get(&id).cloned() };

    let result = resolve_type0_hierarchy(&type0_dict, resolver);
    assert!(result.is_some());

    let info = result.unwrap();

    // CIDFont should be resolved, but not FontDescriptor or font stream
    assert!(info.cidfont_dict.is_some(), "CIDFont should be resolved");
    assert_eq!(info.cidfont_subtype, Some(CIDFontSubtype::Type2));
    assert!(
        info.font_descriptor.is_none(),
        "FontDescriptor should NOT be resolved (missing)"
    );
    assert!(
        info.font_stream.is_none(),
        "Font stream should NOT be resolved"
    );
    assert!(!info.has_embedded_font(), "Should NOT have embedded font");
}

/// Integration test: Non-Type0 font returns None
#[test]
fn test_integration_resolve_non_type0_returns_none() {
    let type1_dict = create_type1_font_dict();
    let store = create_complete_object_store();

    let resolver = |id: ObjectId| -> Option<Object> { store.get(&id).cloned() };

    let result = resolve_type0_hierarchy(&type1_dict, resolver);
    assert!(
        result.is_none(),
        "Non-Type0 font should return None from resolve_type0_hierarchy"
    );
}

/// Integration test: CFF-based CIDFont (CIDFontType0)
#[test]
fn test_integration_resolve_type0_with_cff_font() {
    // Create Type0 dict pointing to CFF-based CIDFont
    let mut type0_dict = Dictionary::new();
    type0_dict.set("Type", Name::new("Font"));
    type0_dict.set("Subtype", Name::new("Type0"));
    type0_dict.set("BaseFont", Name::new("KozMinPro-Regular"));
    type0_dict.set("Encoding", Name::new("Identity-H"));

    let mut descendant_array = Array::new();
    descendant_array.push(Object::Reference(ObjectId::new(30, 0)));
    type0_dict.set("DescendantFonts", Object::Array(descendant_array));

    // Create CFF-based object store
    let mut store = HashMap::new();

    // CIDFont with CIDFontType0 (CFF outlines)
    let mut cidfont = Dictionary::new();
    cidfont.set("Type", Name::new("Font"));
    cidfont.set("Subtype", Name::new("CIDFontType0"));
    cidfont.set("BaseFont", Name::new("KozMinPro-Regular"));
    cidfont.set("FontDescriptor", Object::Reference(ObjectId::new(31, 0)));
    store.insert(ObjectId::new(30, 0), Object::Dictionary(cidfont));

    // FontDescriptor with FontFile3 (CFF)
    let mut descriptor = Dictionary::new();
    descriptor.set("Type", Name::new("FontDescriptor"));
    descriptor.set("FontName", Name::new("KozMinPro-Regular"));
    descriptor.set("FontFile3", Object::Reference(ObjectId::new(32, 0)));
    store.insert(ObjectId::new(31, 0), Object::Dictionary(descriptor));

    // CFF font stream
    let cff_stream = Stream::new(
        Dictionary::new(),
        vec![0x01, 0x00, 0x04, 0x00], // CFF header bytes
    );
    store.insert(ObjectId::new(32, 0), Object::Stream(cff_stream));

    let resolver = |id: ObjectId| -> Option<Object> { store.get(&id).cloned() };

    let result = resolve_type0_hierarchy(&type0_dict, resolver);
    assert!(result.is_some());

    let info = result.unwrap();
    assert_eq!(
        info.cidfont_subtype,
        Some(CIDFontSubtype::Type0),
        "Should be CIDFontType0 (CFF)"
    );
    assert_eq!(
        info.font_file_type,
        Some(FontFileType::CFF),
        "Should be CFF font"
    );
    assert!(info.has_embedded_font());
}
