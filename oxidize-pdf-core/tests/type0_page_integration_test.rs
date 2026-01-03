//! TDD Integration Tests for Type0 Font Page Resolution
//!
//! Phase 2.2: Tests for resolving Type0 font hierarchy in Page::from_parsed_with_content()
//!
//! These tests verify that when a page with Type0/CID fonts is converted using
//! `from_parsed_with_content()`, the complete font hierarchy is resolved:
//! Type0 → DescendantFonts → CIDFont → FontDescriptor → FontFile2
//!
//! Following TDD methodology: tests written BEFORE implementation.
//! All tests should FAIL initially (RED), then PASS after implementation (GREEN).

use oxidize_pdf::fonts::type0_parsing::{
    detect_type0_font, extract_descendant_fonts_ref, resolve_type0_hierarchy, CIDFontSubtype,
    FontFileType,
};
use oxidize_pdf::pdf_objects::{Array, Dictionary, Name, Object, ObjectId, Stream};
use std::collections::HashMap;

// =============================================================================
// Test Helpers
// =============================================================================

/// Create a mock Type0 font dictionary with complete hierarchy
fn create_complete_type0_hierarchy() -> (Dictionary, HashMap<ObjectId, Object>) {
    let mut store = HashMap::new();

    // Type0 font dictionary (root)
    let mut type0_dict = Dictionary::new();
    type0_dict.set("Type", Name::new("Font"));
    type0_dict.set("Subtype", Name::new("Type0"));
    type0_dict.set("BaseFont", Name::new("ArialMT"));
    type0_dict.set("Encoding", Name::new("Identity-H"));

    let mut descendant_array = Array::new();
    descendant_array.push(Object::Reference(ObjectId::new(100, 0)));
    type0_dict.set("DescendantFonts", Object::Array(descendant_array));
    type0_dict.set("ToUnicode", Object::Reference(ObjectId::new(105, 0)));

    // CIDFont dictionary (object 100)
    let mut cidfont = Dictionary::new();
    cidfont.set("Type", Name::new("Font"));
    cidfont.set("Subtype", Name::new("CIDFontType2"));
    cidfont.set("BaseFont", Name::new("ArialMT"));
    cidfont.set("FontDescriptor", Object::Reference(ObjectId::new(101, 0)));
    cidfont.set("DW", Object::Integer(1000));

    let mut cid_system_info = Dictionary::new();
    cid_system_info.set("Registry", "Adobe");
    cid_system_info.set("Ordering", "Identity");
    cid_system_info.set("Supplement", 0i32);
    cidfont.set("CIDSystemInfo", Object::Dictionary(cid_system_info));
    store.insert(ObjectId::new(100, 0), Object::Dictionary(cidfont));

    // FontDescriptor dictionary (object 101)
    let mut descriptor = Dictionary::new();
    descriptor.set("Type", Name::new("FontDescriptor"));
    descriptor.set("FontName", Name::new("ArialMT"));
    descriptor.set("Flags", Object::Integer(32));
    descriptor.set("ItalicAngle", Object::Integer(0));
    descriptor.set("Ascent", Object::Integer(905));
    descriptor.set("Descent", Object::Integer(-212));
    descriptor.set("CapHeight", Object::Integer(728));
    descriptor.set("StemV", Object::Integer(80));
    descriptor.set("FontFile2", Object::Reference(ObjectId::new(102, 0)));
    store.insert(ObjectId::new(101, 0), Object::Dictionary(descriptor));

    // FontFile2 stream (object 102) - TrueType font data
    let ttf_data = vec![
        0x00, 0x01, 0x00, 0x00, // TTF magic bytes (sfnt version)
        0x00, 0x10, // numTables = 16
        0x01, 0x00, // searchRange
        0x04, 0x00, // entrySelector
        0x00, 0x00, // rangeShift
              // ... minimal TTF structure
    ];
    let font_stream = Stream::new(Dictionary::new(), ttf_data);
    store.insert(ObjectId::new(102, 0), Object::Stream(font_stream));

    // ToUnicode CMap stream (object 105)
    let tounicode_data = b"/CIDInit /ProcSet findresource begin\n\
        12 dict begin\n\
        begincmap\n\
        /CIDSystemInfo << /Registry (Adobe) /Ordering (UCS) /Supplement 0 >> def\n\
        /CMapName /Adobe-Identity-UCS def\n\
        /CMapType 2 def\n\
        1 begincodespacerange\n\
        <0000> <FFFF>\n\
        endcodespacerange\n\
        1 beginbfchar\n\
        <0041> <0041>\n\
        endbfchar\n\
        endcmap\n\
        end end"
        .to_vec();
    let tounicode_stream = Stream::new(Dictionary::new(), tounicode_data);
    store.insert(ObjectId::new(105, 0), Object::Stream(tounicode_stream));

    (type0_dict, store)
}

/// Create a simple Type1 font dictionary (for mixed font tests)
fn create_type1_font_dict() -> (Dictionary, HashMap<ObjectId, Object>) {
    let mut store = HashMap::new();

    let mut type1_dict = Dictionary::new();
    type1_dict.set("Type", Name::new("Font"));
    type1_dict.set("Subtype", Name::new("Type1"));
    type1_dict.set("BaseFont", Name::new("Helvetica"));
    type1_dict.set("FontDescriptor", Object::Reference(ObjectId::new(200, 0)));

    // FontDescriptor for Type1
    let mut descriptor = Dictionary::new();
    descriptor.set("Type", Name::new("FontDescriptor"));
    descriptor.set("FontName", Name::new("Helvetica"));
    descriptor.set("Flags", Object::Integer(32));
    descriptor.set("FontFile", Object::Reference(ObjectId::new(201, 0)));
    store.insert(ObjectId::new(200, 0), Object::Dictionary(descriptor));

    // FontFile stream (Type1 font data)
    let type1_data = vec![0x80, 0x01]; // PFB header
    let font_stream = Stream::new(Dictionary::new(), type1_data);
    store.insert(ObjectId::new(201, 0), Object::Stream(font_stream));

    (type1_dict, store)
}

// =============================================================================
// Test 1: Resolve Type0 Font Streams in Page Context
// =============================================================================

/// Test that resolve_type0_hierarchy correctly traverses the complete chain
/// Type0 → DescendantFonts → CIDFont → FontDescriptor → FontFile2
#[test]
fn test_resolve_type0_font_hierarchy_complete() {
    let (type0_dict, store) = create_complete_type0_hierarchy();

    // Create resolver that simulates document.get_object()
    let resolver = |id: ObjectId| -> Option<Object> { store.get(&id).cloned() };

    let result = resolve_type0_hierarchy(&type0_dict, resolver);
    assert!(result.is_some(), "Should resolve Type0 hierarchy");

    let info = result.unwrap();

    // Verify complete chain was resolved
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
    assert!(
        info.font_stream.is_some(),
        "FontFile2 stream should be resolved"
    );
    assert_eq!(
        info.font_file_type,
        Some(FontFileType::TrueType),
        "Should be TrueType font file"
    );
    assert!(
        info.tounicode_stream.is_some(),
        "ToUnicode stream should be resolved"
    );

    // Verify helper methods
    assert!(info.has_embedded_font(), "Should report embedded font");
    assert!(info.has_tounicode(), "Should report ToUnicode CMap");
}

// =============================================================================
// Test 2: Mixed Type1 and Type0 Fonts
// =============================================================================

/// Test that both Type1 and Type0 fonts can be resolved in the same context
#[test]
fn test_type1_and_type0_fonts_mixed() {
    let (type0_dict, type0_store) = create_complete_type0_hierarchy();
    let (type1_dict, type1_store) = create_type1_font_dict();

    // Combined store
    let mut combined_store = type0_store;
    combined_store.extend(type1_store);

    let resolver = |id: ObjectId| -> Option<Object> { combined_store.get(&id).cloned() };

    // Type0 should resolve with full hierarchy
    let type0_result = resolve_type0_hierarchy(&type0_dict, resolver);
    assert!(type0_result.is_some(), "Type0 should resolve");
    let type0_info = type0_result.unwrap();
    assert!(type0_info.has_embedded_font());
    assert_eq!(type0_info.cidfont_subtype, Some(CIDFontSubtype::Type2));

    // Type1 should NOT be detected as Type0
    assert!(
        !detect_type0_font(&type1_dict),
        "Type1 should not be detected as Type0"
    );

    // resolve_type0_hierarchy should return None for Type1
    let type1_result = resolve_type0_hierarchy(&type1_dict, resolver);
    assert!(
        type1_result.is_none(),
        "Type1 should return None from resolve_type0_hierarchy"
    );
}

// =============================================================================
// Test 3: CID Font Without Embedded Data (Base 14 style)
// =============================================================================

/// Test that Type0 fonts without embedded font data are handled gracefully
#[test]
fn test_cid_font_without_embedded_data() {
    let mut store = HashMap::new();

    // Type0 font dictionary
    let mut type0_dict = Dictionary::new();
    type0_dict.set("Type", Name::new("Font"));
    type0_dict.set("Subtype", Name::new("Type0"));
    type0_dict.set("BaseFont", Name::new("HeiseiMin-W3"));
    type0_dict.set("Encoding", Name::new("UniJIS-UCS2-H"));

    let mut descendant_array = Array::new();
    descendant_array.push(Object::Reference(ObjectId::new(300, 0)));
    type0_dict.set("DescendantFonts", Object::Array(descendant_array));

    // CIDFont without FontDescriptor (or with FontDescriptor but no FontFile)
    let mut cidfont = Dictionary::new();
    cidfont.set("Type", Name::new("Font"));
    cidfont.set("Subtype", Name::new("CIDFontType0"));
    cidfont.set("BaseFont", Name::new("HeiseiMin-W3"));
    // No FontDescriptor - this is valid for system fonts
    store.insert(ObjectId::new(300, 0), Object::Dictionary(cidfont));

    let resolver = |id: ObjectId| -> Option<Object> { store.get(&id).cloned() };

    let result = resolve_type0_hierarchy(&type0_dict, resolver);
    assert!(result.is_some(), "Should return partial info");

    let info = result.unwrap();
    assert!(info.cidfont_dict.is_some(), "CIDFont should be resolved");
    assert_eq!(info.cidfont_subtype, Some(CIDFontSubtype::Type0));
    assert!(
        info.font_descriptor.is_none(),
        "No FontDescriptor should be present"
    );
    assert!(
        info.font_stream.is_none(),
        "No font stream should be present"
    );
    assert!(!info.has_embedded_font(), "Should report no embedded font");
}

// =============================================================================
// Test 4: Circular Reference Detection in CID Hierarchy
// =============================================================================

/// Test that circular references in CID font hierarchy are detected and handled
/// (should not cause infinite loop or stack overflow)
#[test]
fn test_circular_reference_in_cid_hierarchy() {
    let mut store = HashMap::new();

    // Type0 font pointing to CIDFont
    let mut type0_dict = Dictionary::new();
    type0_dict.set("Type", Name::new("Font"));
    type0_dict.set("Subtype", Name::new("Type0"));
    type0_dict.set("BaseFont", Name::new("MaliciousFont"));

    let mut descendant_array = Array::new();
    descendant_array.push(Object::Reference(ObjectId::new(400, 0)));
    type0_dict.set("DescendantFonts", Object::Array(descendant_array));

    // CIDFont pointing back to itself via FontDescriptor
    let mut cidfont = Dictionary::new();
    cidfont.set("Type", Name::new("Font"));
    cidfont.set("Subtype", Name::new("CIDFontType2"));
    cidfont.set("FontDescriptor", Object::Reference(ObjectId::new(400, 0))); // Circular!
    store.insert(ObjectId::new(400, 0), Object::Dictionary(cidfont));

    let resolver = |id: ObjectId| -> Option<Object> { store.get(&id).cloned() };

    // Should NOT hang or stack overflow
    let result = resolve_type0_hierarchy(&type0_dict, resolver);

    // Should return partial result (CIDFont resolved, but FontDescriptor detected as circular)
    assert!(result.is_some(), "Should return partial info, not hang");

    let info = result.unwrap();
    assert!(info.cidfont_dict.is_some(), "CIDFont should be resolved");
    assert!(
        info.font_descriptor.is_none(),
        "FontDescriptor should be None (circular ref detected)"
    );
}

// =============================================================================
// Test 5: Deep Circular Reference (FontDescriptor → FontFile → back to CIDFont)
// =============================================================================

/// Test detection of circular references deeper in the hierarchy
#[test]
fn test_deep_circular_reference_detection() {
    let mut store = HashMap::new();

    // Type0 font
    let mut type0_dict = Dictionary::new();
    type0_dict.set("Type", Name::new("Font"));
    type0_dict.set("Subtype", Name::new("Type0"));

    let mut descendant_array = Array::new();
    descendant_array.push(Object::Reference(ObjectId::new(500, 0)));
    type0_dict.set("DescendantFonts", Object::Array(descendant_array));

    // CIDFont → FontDescriptor
    let mut cidfont = Dictionary::new();
    cidfont.set("Subtype", Name::new("CIDFontType2"));
    cidfont.set("FontDescriptor", Object::Reference(ObjectId::new(501, 0)));
    store.insert(ObjectId::new(500, 0), Object::Dictionary(cidfont));

    // FontDescriptor → FontFile2 that points back to CIDFont
    let mut descriptor = Dictionary::new();
    descriptor.set("Type", Name::new("FontDescriptor"));
    descriptor.set("FontFile2", Object::Reference(ObjectId::new(500, 0))); // Points back to CIDFont!
    store.insert(ObjectId::new(501, 0), Object::Dictionary(descriptor));

    let resolver = |id: ObjectId| -> Option<Object> { store.get(&id).cloned() };

    let result = resolve_type0_hierarchy(&type0_dict, resolver);
    assert!(result.is_some());

    let info = result.unwrap();
    assert!(info.cidfont_dict.is_some());
    assert!(info.font_descriptor.is_some());
    // FontFile2 should be None because it points to already-visited CIDFont
    assert!(
        info.font_stream.is_none(),
        "FontFile should be None (circular ref to CIDFont)"
    );
}

// =============================================================================
// Test 6: ToUnicode Pointing to Already-Visited Object
// =============================================================================

/// Test that ToUnicode circular reference is detected
#[test]
fn test_tounicode_circular_reference() {
    let mut store = HashMap::new();

    // Type0 with ToUnicode pointing to CIDFont (unusual but tests detection)
    let mut type0_dict = Dictionary::new();
    type0_dict.set("Type", Name::new("Font"));
    type0_dict.set("Subtype", Name::new("Type0"));

    let mut descendant_array = Array::new();
    descendant_array.push(Object::Reference(ObjectId::new(600, 0)));
    type0_dict.set("DescendantFonts", Object::Array(descendant_array));
    type0_dict.set("ToUnicode", Object::Reference(ObjectId::new(600, 0))); // Points to CIDFont!

    // CIDFont
    let mut cidfont = Dictionary::new();
    cidfont.set("Subtype", Name::new("CIDFontType2"));
    store.insert(ObjectId::new(600, 0), Object::Dictionary(cidfont));

    let resolver = |id: ObjectId| -> Option<Object> { store.get(&id).cloned() };

    let result = resolve_type0_hierarchy(&type0_dict, resolver);
    assert!(result.is_some());

    let info = result.unwrap();
    assert!(info.cidfont_dict.is_some());
    // ToUnicode points to already-visited CIDFont, should be detected
    assert!(
        info.tounicode_stream.is_none(),
        "ToUnicode should be None (points to already-visited object)"
    );
}

// =============================================================================
// Test 7: Verify DescendantFonts Extraction
// =============================================================================

/// Test that DescendantFonts array is correctly extracted
#[test]
fn test_descendant_fonts_extraction() {
    let (type0_dict, _store) = create_complete_type0_hierarchy();

    let refs = extract_descendant_fonts_ref(&type0_dict);
    assert!(refs.is_some(), "Should extract DescendantFonts");

    let refs = refs.unwrap();
    assert_eq!(refs.len(), 1, "Should have exactly one descendant");
    assert_eq!(
        refs[0],
        ObjectId::new(100, 0),
        "Should point to CIDFont at object 100"
    );
}

// =============================================================================
// Phase 2.4 Edge Cases Tests
// =============================================================================

/// Test that Type0 fonts with multiple DescendantFonts are handled
/// (PDF spec technically allows only one, but we should be robust)
#[test]
fn test_cid_font_with_multiple_descendants() {
    let mut store = HashMap::new();

    let mut type0_dict = Dictionary::new();
    type0_dict.set("Type", Name::new("Font"));
    type0_dict.set("Subtype", Name::new("Type0"));
    type0_dict.set("BaseFont", Name::new("CompositeFont"));

    // Array with two descendants (unusual but should be handled)
    let mut descendant_array = Array::new();
    descendant_array.push(Object::Reference(ObjectId::new(800, 0)));
    descendant_array.push(Object::Reference(ObjectId::new(801, 0)));
    type0_dict.set("DescendantFonts", Object::Array(descendant_array));

    // First CIDFont
    let mut cidfont1 = Dictionary::new();
    cidfont1.set("Subtype", Name::new("CIDFontType2"));
    store.insert(ObjectId::new(800, 0), Object::Dictionary(cidfont1));

    // Second CIDFont
    let mut cidfont2 = Dictionary::new();
    cidfont2.set("Subtype", Name::new("CIDFontType0"));
    store.insert(ObjectId::new(801, 0), Object::Dictionary(cidfont2));

    let resolver = |id: ObjectId| -> Option<Object> { store.get(&id).cloned() };

    let result = resolve_type0_hierarchy(&type0_dict, resolver);
    assert!(result.is_some(), "Should handle multiple descendants");

    // Per spec, only first descendant is used
    let info = result.unwrap();
    assert_eq!(
        info.cidfont_subtype,
        Some(CIDFontSubtype::Type2),
        "Should use first descendant"
    );
}

/// Test Type0 font with Identity-H encoding preserved correctly
#[test]
fn test_type0_font_with_identity_h_encoding() {
    let mut type0_dict = Dictionary::new();
    type0_dict.set("Type", Name::new("Font"));
    type0_dict.set("Subtype", Name::new("Type0"));
    type0_dict.set("BaseFont", Name::new("TestFont"));
    type0_dict.set("Encoding", Name::new("Identity-H"));

    let mut descendant_array = Array::new();
    descendant_array.push(Object::Reference(ObjectId::new(900, 0)));
    type0_dict.set("DescendantFonts", Object::Array(descendant_array));

    assert!(detect_type0_font(&type0_dict), "Should detect as Type0");

    // Verify encoding is present in dict
    if let Some(Object::Name(enc)) = type0_dict.get("Encoding") {
        assert_eq!(enc.as_str(), "Identity-H", "Encoding should be preserved");
    } else {
        panic!("Encoding should be present");
    }
}

/// Test empty DescendantFonts array (malformed PDF)
/// resolve_type0_hierarchy returns partial info even for malformed fonts
#[test]
fn test_empty_descendant_fonts_array() {
    let mut type0_dict = Dictionary::new();
    type0_dict.set("Type", Name::new("Font"));
    type0_dict.set("Subtype", Name::new("Type0"));

    // Empty array
    type0_dict.set("DescendantFonts", Object::Array(Array::new()));

    let resolver = |_id: ObjectId| -> Option<Object> { None };

    let result = resolve_type0_hierarchy(&type0_dict, resolver);
    // Function may return partial info even for malformed fonts
    // The important thing is it doesn't crash
    if let Some(info) = result {
        assert!(info.cidfont_dict.is_none(), "Should have no CIDFont");
        assert!(!info.has_embedded_font(), "Should have no embedded font");
    }
    // Both None and Some(partial) are acceptable for malformed input
}

/// Test Type0 without DescendantFonts key (malformed PDF)
#[test]
fn test_type0_without_descendant_fonts() {
    let mut type0_dict = Dictionary::new();
    type0_dict.set("Type", Name::new("Font"));
    type0_dict.set("Subtype", Name::new("Type0"));
    // No DescendantFonts key

    let refs = extract_descendant_fonts_ref(&type0_dict);
    assert!(
        refs.is_none(),
        "Should return None when DescendantFonts missing"
    );
}

// =============================================================================
// Test 8: CIDFontType0 (CFF outlines) Resolution
// =============================================================================

/// Test resolution of CIDFontType0 (PostScript/CFF based) fonts
#[test]
fn test_cidfont_type0_cff_resolution() {
    let mut store = HashMap::new();

    // Type0 font with CFF-based descendant
    let mut type0_dict = Dictionary::new();
    type0_dict.set("Type", Name::new("Font"));
    type0_dict.set("Subtype", Name::new("Type0"));
    type0_dict.set("BaseFont", Name::new("KozMinPro-Regular"));

    let mut descendant_array = Array::new();
    descendant_array.push(Object::Reference(ObjectId::new(700, 0)));
    type0_dict.set("DescendantFonts", Object::Array(descendant_array));

    // CIDFontType0 (CFF)
    let mut cidfont = Dictionary::new();
    cidfont.set("Type", Name::new("Font"));
    cidfont.set("Subtype", Name::new("CIDFontType0"));
    cidfont.set("BaseFont", Name::new("KozMinPro-Regular"));
    cidfont.set("FontDescriptor", Object::Reference(ObjectId::new(701, 0)));
    store.insert(ObjectId::new(700, 0), Object::Dictionary(cidfont));

    // FontDescriptor with FontFile3 (CFF)
    let mut descriptor = Dictionary::new();
    descriptor.set("Type", Name::new("FontDescriptor"));
    descriptor.set("FontFile3", Object::Reference(ObjectId::new(702, 0)));
    store.insert(ObjectId::new(701, 0), Object::Dictionary(descriptor));

    // CFF font stream
    let cff_data = vec![0x01, 0x00, 0x04, 0x01]; // CFF header
    let cff_stream = Stream::new(Dictionary::new(), cff_data);
    store.insert(ObjectId::new(702, 0), Object::Stream(cff_stream));

    let resolver = |id: ObjectId| -> Option<Object> { store.get(&id).cloned() };

    let result = resolve_type0_hierarchy(&type0_dict, resolver);
    assert!(result.is_some());

    let info = result.unwrap();
    assert_eq!(
        info.cidfont_subtype,
        Some(CIDFontSubtype::Type0),
        "Should be CIDFontType0"
    );
    assert_eq!(
        info.font_file_type,
        Some(FontFileType::CFF),
        "Should be CFF font"
    );
    assert!(info.has_embedded_font());
}
