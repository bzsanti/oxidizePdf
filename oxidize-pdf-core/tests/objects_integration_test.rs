//! Integration tests for PDF objects module
//! Tests the interaction between different object types and their serialization

use oxidize_pdf::parser::objects::{
    PdfArray, PdfDictionary, PdfName, PdfObject, PdfStream, PdfString,
};
use std::collections::HashMap;

#[test]
fn test_object_creation_and_conversion() {
    // Test all object types
    let null = PdfObject::Null;
    let boolean = PdfObject::Boolean(true);
    let integer = PdfObject::Integer(42);
    let real = PdfObject::Real(3.14159);
    let string = PdfObject::String(PdfString::new(b"Hello PDF".to_vec()));
    let name = PdfObject::Name(PdfName::new("Type".to_string()));

    // Verify type checking
    assert!(null.is_null());
    assert!(boolean.as_bool().is_some());
    assert_eq!(integer.as_integer(), Some(42));
    assert!(real.as_real().is_some());
    assert!(string.as_string().is_some());
    assert!(name.as_name().is_some());
}

#[test]
fn test_array_operations() {
    let mut array = PdfArray(Vec::new());

    // Add various object types
    array.0.push(PdfObject::Integer(1));
    array.0.push(PdfObject::Real(2.5));
    array
        .0
        .push(PdfObject::String(PdfString::new(b"text".to_vec())));
    array.0.push(PdfObject::Boolean(true));
    array.0.push(PdfObject::Null);

    assert_eq!(array.0.len(), 5);

    // Access elements
    assert_eq!(array.0[0].as_integer(), Some(1));
    assert!(array.0[4].is_null());

    // Create nested array
    let mut nested = PdfArray(Vec::new());
    nested.0.push(PdfObject::Integer(10));
    nested.0.push(PdfObject::Integer(20));
    array.0.push(PdfObject::Array(nested));

    assert_eq!(array.0.len(), 6);
}

#[test]
fn test_dictionary_operations() {
    let mut dict = PdfDictionary::new();

    // Insert various types
    dict.insert(
        "Type".to_string(),
        PdfObject::Name(PdfName::new("Page".to_string())),
    );
    dict.insert("Count".to_string(), PdfObject::Integer(10));
    dict.insert(
        "Title".to_string(),
        PdfObject::String(PdfString::new(b"Test".to_vec())),
    );

    // Access entries
    assert_eq!(dict.0.len(), 3);
    assert!(dict.0.contains_key(&PdfName::new("Type".to_string())));

    // Nested dictionary
    let mut nested = PdfDictionary::new();
    nested.insert("Nested".to_string(), PdfObject::Boolean(true));
    dict.insert("SubDict".to_string(), PdfObject::Dictionary(nested));

    assert_eq!(dict.0.len(), 4);
}

#[test]
fn test_stream_object() {
    let mut dict = PdfDictionary::new();
    dict.insert("Length".to_string(), PdfObject::Integer(100));
    dict.insert(
        "Filter".to_string(),
        PdfObject::Name(PdfName::new("FlateDecode".to_string())),
    );

    let data = b"This is stream data that would normally be compressed".to_vec();
    let stream = PdfStream {
        dict: dict.clone(),
        data: data.clone(),
    };

    assert_eq!(stream.data, data);
    assert!(stream
        .dict
        .0
        .contains_key(&PdfName::new("Length".to_string())));
}

#[test]
fn test_object_references() {
    // Test indirect object references
    let ref1 = PdfObject::Reference(10, 0);
    let ref2 = PdfObject::Reference(20, 1);

    assert!(ref1.as_reference().is_some());
    assert_eq!(ref1.as_reference(), Some((10, 0)));
    assert_eq!(ref2.as_reference(), Some((20, 1)));

    // References in arrays
    let mut array = PdfArray(Vec::new());
    array.0.push(ref1);
    array.0.push(ref2);
    array.0.push(PdfObject::Integer(5));

    assert_eq!(array.0.len(), 3);
}

#[test]
fn test_complex_nested_structures() {
    // Create a complex page-like structure
    let mut page = PdfDictionary::new();
    page.insert(
        "Type".to_string(),
        PdfObject::Name(PdfName::new("Page".to_string())),
    );
    page.insert("Parent".to_string(), PdfObject::Reference(1, 0));

    // MediaBox array
    let mut media_box = PdfArray(Vec::new());
    media_box.0.push(PdfObject::Integer(0));
    media_box.0.push(PdfObject::Integer(0));
    media_box.0.push(PdfObject::Real(612.0));
    media_box.0.push(PdfObject::Real(792.0));
    page.insert("MediaBox".to_string(), PdfObject::Array(media_box));

    // Resources dictionary
    let mut resources = PdfDictionary::new();

    // Font subdictionary
    let mut fonts = PdfDictionary::new();
    let mut f1 = PdfDictionary::new();
    f1.insert(
        "Type".to_string(),
        PdfObject::Name(PdfName::new("Font".to_string())),
    );
    f1.insert(
        "Subtype".to_string(),
        PdfObject::Name(PdfName::new("Type1".to_string())),
    );
    f1.insert(
        "BaseFont".to_string(),
        PdfObject::Name(PdfName::new("Helvetica".to_string())),
    );
    fonts.insert("F1".to_string(), PdfObject::Dictionary(f1));

    resources.insert("Font".to_string(), PdfObject::Dictionary(fonts));
    page.insert("Resources".to_string(), PdfObject::Dictionary(resources));

    // Contents reference
    page.insert("Contents".to_string(), PdfObject::Reference(4, 0));

    // Verify structure
    let page_map = page.0;
    assert!(page_map.contains_key(&PdfName::new("Type".to_string())));
    assert!(page_map.contains_key(&PdfName::new("MediaBox".to_string())));
    assert!(page_map.contains_key(&PdfName::new("Resources".to_string())));
}

#[test]
fn test_string_types() {
    // Literal string
    let literal = PdfString::new(b"Hello (World)".to_vec());
    assert_eq!(literal.as_bytes(), b"Hello (World)");

    // String with escape sequences
    let escaped = PdfString::new(b"Line 1\\nLine 2\\t\\(escaped\\)".to_vec());
    assert!(escaped.as_bytes().len() > 0);

    // Binary string
    let binary = PdfString::new(vec![0xFF, 0x00, 0xAB, 0xCD]);
    assert_eq!(binary.as_bytes().len(), 4);

    // Unicode string (UTF-16BE with BOM)
    let unicode_marker = vec![0xFE, 0xFF]; // UTF-16BE BOM
    let mut unicode = unicode_marker.clone();
    unicode.extend_from_slice(&[0x00, 0x48, 0x00, 0x69]); // "Hi"
    let unicode_str = PdfString::new(unicode);
    assert!(unicode_str.as_bytes().starts_with(&[0xFE, 0xFF]));
}

#[test]
fn test_name_objects() {
    // Simple names
    let name1 = PdfName::new("Type".to_string());
    let name2 = PdfName::new("Page".to_string());

    assert_eq!(name1.as_str(), "Type");
    assert_eq!(name2.as_str(), "Page");

    // Names with special characters
    let special = PdfName::new("Name#20with#20spaces".to_string());
    assert!(special.as_str().contains("#20"));

    // Names as dictionary keys
    let mut dict = PdfDictionary::new();
    dict.insert(name1.as_str().to_string(), PdfObject::Name(name2));
    assert_eq!(dict.0.len(), 1);
}

#[test]
fn test_object_equality() {
    // Test equality for different object types
    let int1 = PdfObject::Integer(42);
    let int2 = PdfObject::Integer(42);
    let int3 = PdfObject::Integer(43);

    assert_eq!(int1, int2);
    assert_ne!(int1, int3);

    // Array equality
    let mut arr1 = PdfArray(Vec::new());
    arr1.0.push(PdfObject::Integer(1));
    arr1.0.push(PdfObject::Integer(2));

    let mut arr2 = PdfArray(Vec::new());
    arr2.0.push(PdfObject::Integer(1));
    arr2.0.push(PdfObject::Integer(2));

    assert_eq!(PdfObject::Array(arr1), PdfObject::Array(arr2));

    // Dictionary equality
    let mut dict1 = PdfDictionary::new();
    dict1.insert("Key".to_string(), PdfObject::Integer(10));

    let mut dict2 = PdfDictionary::new();
    dict2.insert("Key".to_string(), PdfObject::Integer(10));

    assert_eq!(PdfObject::Dictionary(dict1), PdfObject::Dictionary(dict2));
}

#[test]
fn test_object_type_conversions() {
    let boolean = PdfObject::Boolean(true);
    let integer = PdfObject::Integer(42);
    let real = PdfObject::Real(3.14);
    let string = PdfObject::String(PdfString::new(b"text".to_vec()));

    // Test as_* methods
    assert_eq!(boolean.as_bool(), Some(true));
    assert_eq!(boolean.as_integer(), None);

    assert_eq!(integer.as_integer(), Some(42));
    assert_eq!(integer.as_bool(), None);

    assert!(real.as_real().is_some());
    assert_eq!(real.as_string(), None);

    assert!(string.as_string().is_some());
    assert_eq!(string.as_real(), None);

    // Test type checking
    assert!(boolean.as_bool().is_some());
    assert!(boolean.as_integer().is_none());

    assert!(integer.as_integer().is_some());
    // Note: as_real() converts integers to f64
    assert_eq!(integer.as_real(), Some(42.0));

    assert!(real.as_real().is_some());
    assert!(real.as_string().is_none());
}

#[test]
fn test_large_dictionary() {
    let mut dict = PdfDictionary::new();

    // Add many entries
    for i in 0..100 {
        let key = format!("Key{}", i);
        let value = if i % 2 == 0 {
            PdfObject::Integer(i as i64)
        } else {
            PdfObject::String(PdfString::new(format!("Value{}", i).into_bytes()))
        };
        dict.insert(key, value);
    }

    assert_eq!(dict.0.len(), 100);

    // Verify specific entries
    assert!(dict.0.contains_key(&PdfName::new("Key0".to_string())));
    assert!(dict.0.contains_key(&PdfName::new("Key99".to_string())));
}

#[test]
fn test_deeply_nested_structures() {
    // Create deeply nested structure
    let mut root = PdfDictionary::new();

    for level in 0..5 {
        let mut dict = PdfDictionary::new();
        dict.insert("Level".to_string(), PdfObject::Integer(level));

        let mut array = PdfArray(Vec::new());
        for i in 0..3 {
            array.0.push(PdfObject::Integer(i));
        }
        dict.insert("Data".to_string(), PdfObject::Array(array));

        if level > 0 {
            // Reference to previous level
            dict.insert(
                "Parent".to_string(),
                PdfObject::Reference(level as u32 - 1, 0),
            );
        }

        root.insert(format!("Level{}", level), PdfObject::Dictionary(dict));
    }

    assert_eq!(root.0.len(), 5);
}
