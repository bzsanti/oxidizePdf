//! Tests for ObjectStream parsing functionality

use super::*;
use crate::parser::objects::{PdfDictionary, PdfName, PdfObject, PdfStream};
use crate::parser::ParseOptions;
use std::collections::HashMap;

/// Helper to create a test stream dictionary with required entries
fn create_test_stream_dict(n: i64, first: i64) -> PdfDictionary {
    let mut dict = PdfDictionary::new();
    dict.insert("N".to_string(), PdfObject::Integer(n));
    dict.insert("First".to_string(), PdfObject::Integer(first));
    dict.insert(
        "Type".to_string(),
        PdfObject::Name(PdfName::new("ObjStm".to_string())),
    );
    dict
}

/// Helper to create test stream data with object number/offset pairs and objects
fn _create_test_stream_data(objects: &[(u32, &str)]) -> Vec<u8> {
    let mut data = Vec::new();

    // Write object number/offset pairs
    let mut current_offset = 0u32;
    for (obj_num, _) in objects {
        data.extend_from_slice(format!("{} {} ", obj_num, current_offset).as_bytes());
        current_offset += 10; // Assume each object takes 10 bytes for simplicity
    }

    // Write objects at their offsets
    for (_, obj_data) in objects {
        // Pad to the expected offset
        while data.len() % 10 != 0 {
            data.push(b' ');
        }
        data.extend_from_slice(obj_data.as_bytes());
    }

    data
}

#[test]
fn test_object_stream_parse_valid() {
    let dict = create_test_stream_dict(2, 6); // 2 objects, first at offset 6
    let data = b"1 0 2 4 42 null".to_vec(); // obj1 at 0, obj2 at 4, then "42" and "null"

    let stream = PdfStream { dict, data };
    let options = ParseOptions::default();
    let result = ObjectStream::parse(stream, &options);

    assert!(result.is_ok(), "Should parse valid object stream");
    let obj_stream = result.unwrap();

    assert_eq!(obj_stream.n, 2, "Should have correct number of objects");
    assert_eq!(obj_stream.first, 6, "Should have correct first offset");
}

#[test]
fn test_object_stream_parse_missing_n() {
    let mut dict = PdfDictionary::new();
    dict.insert("First".to_string(), PdfObject::Integer(10));

    let stream = PdfStream { dict, data: vec![] };
    let options = ParseOptions::default();
    let result = ObjectStream::parse(stream, &options);

    assert!(result.is_err(), "Should fail when N is missing");
    if let Err(ParseError::MissingKey(key)) = result {
        assert_eq!(key, "N", "Should report missing N key");
    } else {
        panic!("Expected MissingKey error for N");
    }
}

#[test]
fn test_object_stream_parse_missing_first() {
    let mut dict = PdfDictionary::new();
    dict.insert("N".to_string(), PdfObject::Integer(2));

    let stream = PdfStream { dict, data: vec![] };
    let options = ParseOptions::default();
    let result = ObjectStream::parse(stream, &options);

    assert!(result.is_err(), "Should fail when First is missing");
    if let Err(ParseError::MissingKey(key)) = result {
        assert_eq!(key, "First", "Should report missing First key");
    } else {
        panic!("Expected MissingKey error for First");
    }
}

#[test]
fn test_object_stream_parse_invalid_n_type() {
    let mut dict = PdfDictionary::new();
    dict.insert(
        "N".to_string(),
        PdfObject::Name(PdfName::new("invalid".to_string())),
    );
    dict.insert("First".to_string(), PdfObject::Integer(10));

    let stream = PdfStream { dict, data: vec![] };
    let options = ParseOptions::default();
    let result = ObjectStream::parse(stream, &options);

    assert!(result.is_err(), "Should fail when N is not an integer");
}

#[test]
fn test_object_stream_parse_invalid_first_type() {
    let mut dict = PdfDictionary::new();
    dict.insert("N".to_string(), PdfObject::Integer(2));
    dict.insert(
        "First".to_string(),
        PdfObject::Name(PdfName::new("invalid".to_string())),
    );

    let stream = PdfStream { dict, data: vec![] };
    let options = ParseOptions::default();
    let result = ObjectStream::parse(stream, &options);

    assert!(result.is_err(), "Should fail when First is not an integer");
}

#[test]
fn test_object_stream_get_object() {
    let dict = create_test_stream_dict(1, 4);
    let data = b"5 0 42".to_vec(); // Object 5 at offset 0, contains "42"

    let stream = PdfStream { dict, data };
    let mut obj_stream = ObjectStream {
        stream,
        n: 1,
        first: 4,
        objects: HashMap::new(),
    };

    // Manually insert an object for testing
    obj_stream.objects.insert(5, PdfObject::Integer(42));

    let result = obj_stream.get_object(5);
    assert!(result.is_some(), "Should find existing object");

    let result = obj_stream.get_object(999);
    assert!(
        result.is_none(),
        "Should return None for non-existent object"
    );
}

#[test]
fn test_object_stream_objects() {
    let dict = create_test_stream_dict(2, 6);
    let data = Vec::new();

    let stream = PdfStream { dict, data };
    let mut obj_stream = ObjectStream {
        stream,
        n: 2,
        first: 6,
        objects: HashMap::new(),
    };

    // Add some test objects
    obj_stream.objects.insert(1, PdfObject::Integer(100));
    obj_stream.objects.insert(2, PdfObject::Integer(200));

    let objects = obj_stream.objects();
    assert_eq!(objects.len(), 2, "Should return all objects");
    assert!(objects.contains_key(&1), "Should contain object 1");
    assert!(objects.contains_key(&2), "Should contain object 2");
}

#[test]
fn test_xref_entry_type_free() {
    let entry = XRefEntryType::Free {
        next_free_obj: 42,
        generation: 1,
    };

    let simple = entry.to_simple_entry();
    assert_eq!(simple.offset, 0, "Free entry should have offset 0");
    assert_eq!(simple.generation, 1, "Should preserve generation");
    assert!(!simple.in_use, "Free entry should not be in use");
}

#[test]
fn test_xref_entry_type_in_use() {
    let entry = XRefEntryType::InUse {
        offset: 1024,
        generation: 2,
    };

    let simple = entry.to_simple_entry();
    assert_eq!(simple.offset, 1024, "Should preserve offset");
    assert_eq!(simple.generation, 2, "Should preserve generation");
    assert!(simple.in_use, "In-use entry should be marked as in use");
}

#[test]
fn test_xref_entry_type_compressed() {
    let entry = XRefEntryType::Compressed {
        stream_obj_num: 10,
        index_in_stream: 5,
    };

    let simple = entry.to_simple_entry();
    assert_eq!(simple.offset, 0, "Compressed entry should have offset 0");
    assert_eq!(
        simple.generation, 0,
        "Compressed entry should have generation 0"
    );
    assert!(simple.in_use, "Compressed entry should be marked as in use");
}

#[test]
fn test_xref_entry_type_equality() {
    let free1 = XRefEntryType::Free {
        next_free_obj: 42,
        generation: 1,
    };
    let free2 = XRefEntryType::Free {
        next_free_obj: 42,
        generation: 1,
    };
    let free3 = XRefEntryType::Free {
        next_free_obj: 43,
        generation: 1,
    };

    assert_eq!(free1, free2, "Identical free entries should be equal");
    assert_ne!(free1, free3, "Different free entries should not be equal");

    let in_use = XRefEntryType::InUse {
        offset: 1024,
        generation: 0,
    };
    assert_ne!(free1, in_use, "Free and in-use entries should not be equal");
}

#[test]
fn test_xref_entry_type_clone() {
    let original = XRefEntryType::Compressed {
        stream_obj_num: 15,
        index_in_stream: 3,
    };

    let cloned = original;
    assert_eq!(original, cloned, "Cloned entry should equal original");
}

#[test]
fn test_xref_entry_type_copy() {
    let original = XRefEntryType::InUse {
        offset: 2048,
        generation: 5,
    };

    let copied = original; // Copy trait
    assert_eq!(original, copied, "Copied entry should equal original");
}

#[test]
fn test_object_stream_debug() {
    let dict = create_test_stream_dict(1, 10);
    let data = Vec::new();
    let stream = PdfStream { dict, data };

    let obj_stream = ObjectStream {
        stream,
        n: 1,
        first: 10,
        objects: HashMap::new(),
    };

    let debug_str = format!("{:?}", obj_stream);
    assert!(
        debug_str.contains("ObjectStream"),
        "Debug output should contain struct name"
    );
    assert!(
        debug_str.contains("n: 1"),
        "Debug output should show n value"
    );
    assert!(
        debug_str.contains("first: 10"),
        "Debug output should show first value"
    );
}

#[test]
fn test_xref_entry_type_debug() {
    let entries = [
        XRefEntryType::Free {
            next_free_obj: 42,
            generation: 1,
        },
        XRefEntryType::InUse {
            offset: 1024,
            generation: 2,
        },
        XRefEntryType::Compressed {
            stream_obj_num: 10,
            index_in_stream: 5,
        },
    ];

    for entry in &entries {
        let debug_str = format!("{:?}", entry);
        assert!(!debug_str.is_empty(), "Debug output should not be empty");
        // Each variant should show its specific field names
        match entry {
            XRefEntryType::Free { .. } => assert!(debug_str.contains("Free")),
            XRefEntryType::InUse { .. } => assert!(debug_str.contains("InUse")),
            XRefEntryType::Compressed { .. } => assert!(debug_str.contains("Compressed")),
        }
    }
}
