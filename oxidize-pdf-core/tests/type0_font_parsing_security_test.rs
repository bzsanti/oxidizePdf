//! Security Tests for Type0 Font Parsing
//!
//! These tests verify protection against:
//! - Circular references (can cause infinite loops/stack overflow)
//! - Oversized font streams (zip bomb / memory exhaustion attacks)
//!
//! Following TDD methodology: tests written BEFORE implementation.
//! All tests in Phase 2-3 should FAIL initially, then PASS after Phase 4.

use std::collections::HashMap;

use oxidize_pdf::fonts::type0_parsing::{resolve_type0_hierarchy, MAX_FONT_STREAM_SIZE};
use oxidize_pdf::pdf_objects::{Array, Dictionary, Name, Object, ObjectId, Stream};

// =============================================================================
// Helper Functions
// =============================================================================

/// Create a basic Type0 font dictionary pointing to CIDFont at object 15
fn create_type0_font_dict() -> Dictionary {
    let mut dict = Dictionary::new();
    dict.set("Type", Name::new("Font"));
    dict.set("Subtype", Name::new("Type0"));
    dict.set("BaseFont", Name::new("TestFont"));
    dict.set("Encoding", Name::new("Identity-H"));

    let mut descendant_array = Array::new();
    descendant_array.push(Object::Reference(ObjectId::new(15, 0)));
    dict.set("DescendantFonts", Object::Array(descendant_array));

    dict
}

/// Create a complete valid object store for testing
fn create_complete_object_store() -> HashMap<ObjectId, Object> {
    let mut store = HashMap::new();

    // CIDFont dictionary (object 15)
    let mut cidfont = Dictionary::new();
    cidfont.set("Type", Name::new("Font"));
    cidfont.set("Subtype", Name::new("CIDFontType2"));
    cidfont.set("BaseFont", Name::new("TestFont"));
    cidfont.set("FontDescriptor", Object::Reference(ObjectId::new(16, 0)));
    store.insert(ObjectId::new(15, 0), Object::Dictionary(cidfont));

    // FontDescriptor dictionary (object 16)
    let mut descriptor = Dictionary::new();
    descriptor.set("Type", Name::new("FontDescriptor"));
    descriptor.set("FontName", Name::new("TestFont"));
    descriptor.set("FontFile2", Object::Reference(ObjectId::new(17, 0)));
    store.insert(ObjectId::new(16, 0), Object::Dictionary(descriptor));

    // FontFile2 stream (object 17) - small valid TTF-like data
    let font_stream = Stream::new(
        Dictionary::new(),
        vec![0x00, 0x01, 0x00, 0x00], // TTF magic bytes
    );
    store.insert(ObjectId::new(17, 0), Object::Stream(font_stream));

    store
}

// =============================================================================
// Phase 2: Circular Reference Tests (RED - should fail initially)
// =============================================================================

/// Test: CIDFont references back to Type0 (circular reference)
/// Expected: Should detect loop and return partial info, not hang
#[test]
fn test_circular_reference_cidfont_to_type0() {
    let type0_dict = create_type0_font_dict();
    let mut store = HashMap::new();

    // CIDFont (15) → FontDescriptor points back to CIDFont itself (circular!)
    let mut cidfont = Dictionary::new();
    cidfont.set("Type", Name::new("Font"));
    cidfont.set("Subtype", Name::new("CIDFontType2"));
    cidfont.set("FontDescriptor", Object::Reference(ObjectId::new(15, 0))); // Self-reference!
    store.insert(ObjectId::new(15, 0), Object::Dictionary(cidfont));

    let resolver = |id: ObjectId| -> Option<Object> { store.get(&id).cloned() };

    let result = resolve_type0_hierarchy(&type0_dict, resolver);

    // Should return Some with partial info, not loop forever
    assert!(
        result.is_some(),
        "Should return partial info, not loop forever"
    );

    let info = result.unwrap();
    // Should detect circular ref and stop at FontDescriptor
    assert!(
        info.font_descriptor.is_none(),
        "Should detect self-reference and not resolve FontDescriptor"
    );
}

/// Test: FontDescriptor references back to CIDFont (chain circular reference)
/// Expected: Should detect loop and return partial info
#[test]
fn test_circular_reference_descriptor_to_cidfont() {
    let type0_dict = create_type0_font_dict();
    let mut store = HashMap::new();

    // CIDFont (15) → FontDescriptor (16)
    let mut cidfont = Dictionary::new();
    cidfont.set("Subtype", Name::new("CIDFontType2"));
    cidfont.set("FontDescriptor", Object::Reference(ObjectId::new(16, 0)));
    store.insert(ObjectId::new(15, 0), Object::Dictionary(cidfont));

    // FontDescriptor (16) → FontFile points back to CIDFont (15) - circular!
    let mut descriptor = Dictionary::new();
    descriptor.set("Type", Name::new("FontDescriptor"));
    descriptor.set("FontFile2", Object::Reference(ObjectId::new(15, 0))); // Points back!
    store.insert(ObjectId::new(16, 0), Object::Dictionary(descriptor));

    let resolver = |id: ObjectId| -> Option<Object> { store.get(&id).cloned() };

    let result = resolve_type0_hierarchy(&type0_dict, resolver);
    assert!(result.is_some());

    let info = result.unwrap();
    // FontDescriptor should be resolved, but FontFile should detect circular ref
    assert!(
        info.font_descriptor.is_some(),
        "FontDescriptor should be resolved"
    );
    assert!(
        info.font_stream.is_none(),
        "Should detect circular ref at FontFile level"
    );
}

/// Test: FontFile stream references back (deep circular reference)
/// Expected: Should detect loop even at deepest level
#[test]
fn test_circular_reference_deep_chain() {
    let type0_dict = create_type0_font_dict();
    let mut store = HashMap::new();

    // Build a valid chain: CIDFont → FontDescriptor → FontFile
    let mut cidfont = Dictionary::new();
    cidfont.set("Subtype", Name::new("CIDFontType2"));
    cidfont.set("FontDescriptor", Object::Reference(ObjectId::new(16, 0)));
    store.insert(ObjectId::new(15, 0), Object::Dictionary(cidfont));

    let mut descriptor = Dictionary::new();
    descriptor.set("FontFile2", Object::Reference(ObjectId::new(17, 0)));
    store.insert(ObjectId::new(16, 0), Object::Dictionary(descriptor));

    // FontFile stream is valid but we also add ToUnicode that points back
    let font_stream = Stream::new(Dictionary::new(), vec![0x00, 0x01, 0x00, 0x00]);
    store.insert(ObjectId::new(17, 0), Object::Stream(font_stream));

    // ToUnicode points back to CIDFont (unusual but tests deep circular detection)
    let mut type0_with_tounicode = type0_dict.clone();
    type0_with_tounicode.set("ToUnicode", Object::Reference(ObjectId::new(15, 0)));

    let resolver = |id: ObjectId| -> Option<Object> { store.get(&id).cloned() };

    let result = resolve_type0_hierarchy(&type0_with_tounicode, resolver);
    assert!(result.is_some());

    let info = result.unwrap();
    // ToUnicode pointing to already-visited CIDFont should be detected
    assert!(
        info.tounicode_stream.is_none(),
        "Should detect circular ref in ToUnicode"
    );
}

// =============================================================================
// Phase 3: Font Stream Size Tests (RED - should fail initially)
// =============================================================================

/// Test: Reject font stream larger than MAX_FONT_STREAM_SIZE
/// Expected: font_stream should be None for oversized streams
#[test]
fn test_font_stream_size_exceeds_limit() {
    let type0_dict = create_type0_font_dict();
    let mut store = create_complete_object_store();

    // Replace font stream with one that exceeds the limit (10MB + 1 byte)
    let huge_data = vec![0x42; MAX_FONT_STREAM_SIZE + 1];
    let huge_stream = Stream::new(Dictionary::new(), huge_data);
    store.insert(ObjectId::new(17, 0), Object::Stream(huge_stream));

    let resolver = |id: ObjectId| -> Option<Object> { store.get(&id).cloned() };

    let result = resolve_type0_hierarchy(&type0_dict, resolver);
    assert!(result.is_some());

    let info = result.unwrap();
    // Should reject the oversized stream
    assert!(
        info.font_stream.is_none(),
        "Should reject stream > MAX_FONT_STREAM_SIZE"
    );
    assert!(
        info.font_file_type.is_none(),
        "font_file_type should also be None when stream rejected"
    );
}

/// Test: Accept font stream exactly at MAX_FONT_STREAM_SIZE
/// Expected: font_stream should be Some (boundary condition - exactly at limit is OK)
#[test]
fn test_font_stream_size_at_limit() {
    let type0_dict = create_type0_font_dict();
    let mut store = create_complete_object_store();

    // Stream exactly at the limit
    let limit_data = vec![0x00; MAX_FONT_STREAM_SIZE];
    let limit_stream = Stream::new(Dictionary::new(), limit_data);
    store.insert(ObjectId::new(17, 0), Object::Stream(limit_stream));

    let resolver = |id: ObjectId| -> Option<Object> { store.get(&id).cloned() };

    let result = resolve_type0_hierarchy(&type0_dict, resolver);
    assert!(result.is_some());

    let info = result.unwrap();
    // Should accept stream at exactly MAX_FONT_STREAM_SIZE
    assert!(
        info.font_stream.is_some(),
        "Should accept stream at MAX_FONT_STREAM_SIZE"
    );
}

/// Test: Accept normal-sized font stream (sanity check)
/// Expected: font_stream should be Some for typical font sizes
#[test]
fn test_font_stream_size_normal() {
    let type0_dict = create_type0_font_dict();
    let mut store = create_complete_object_store();

    // Normal font stream (100KB - typical embedded font)
    let normal_data = vec![0x00; 100 * 1024];
    let normal_stream = Stream::new(Dictionary::new(), normal_data);
    store.insert(ObjectId::new(17, 0), Object::Stream(normal_stream));

    let resolver = |id: ObjectId| -> Option<Object> { store.get(&id).cloned() };

    let result = resolve_type0_hierarchy(&type0_dict, resolver);
    assert!(result.is_some());

    let info = result.unwrap();
    assert!(
        info.font_stream.is_some(),
        "Should accept normal-sized stream"
    );
    assert_eq!(
        info.font_stream.as_ref().unwrap().data.len(),
        100 * 1024,
        "Stream data should be preserved"
    );
}
