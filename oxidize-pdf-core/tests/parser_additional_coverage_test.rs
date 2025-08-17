//! Additional tests to increase parser module coverage
//! Focuses on real API usage and edge cases

use oxidize_pdf::parser::objects::{
    PdfArray, PdfDictionary, PdfName, PdfObject, PdfStream, PdfString,
};
use std::collections::HashMap;

#[cfg(test)]
mod parser_coverage_tests {
    use super::*;

    #[test]
    fn test_pdf_dictionary_operations() {
        let mut dict = PdfDictionary(HashMap::new());

        // Add various types
        dict.0.insert(
            PdfName::new("Type".to_string()),
            PdfObject::Name(PdfName::new("Catalog".to_string())),
        );
        dict.0
            .insert(PdfName::new("Count".to_string()), PdfObject::Integer(42));
        dict.0
            .insert(PdfName::new("Version".to_string()), PdfObject::Real(1.7));
        dict.0
            .insert(PdfName::new("IsNew".to_string()), PdfObject::Boolean(true));

        assert_eq!(dict.0.len(), 4);
        assert!(dict.0.contains_key(&PdfName::new("Type".to_string())));

        // Test retrieval
        if let Some(PdfObject::Integer(val)) = dict.0.get(&PdfName::new("Count".to_string())) {
            assert_eq!(*val, 42);
        }
    }

    #[test]
    fn test_pdf_array_manipulation() {
        let mut array = PdfArray(vec![]);

        // Add different types
        array.0.push(PdfObject::Integer(1));
        array.0.push(PdfObject::Real(2.5));
        array.0.push(PdfObject::String(PdfString(b"test".to_vec())));
        array.0.push(PdfObject::Null);

        assert_eq!(array.0.len(), 4);

        // Test iteration
        let mut count = 0;
        for obj in &array.0 {
            match obj {
                PdfObject::Integer(_)
                | PdfObject::Real(_)
                | PdfObject::String(_)
                | PdfObject::Null => count += 1,
                _ => {}
            }
        }
        assert_eq!(count, 4);
    }

    #[test]
    fn test_pdf_stream_creation() {
        let mut dict = PdfDictionary(HashMap::new());
        dict.0
            .insert(PdfName::new("Length".to_string()), PdfObject::Integer(11));
        dict.0.insert(
            PdfName::new("Filter".to_string()),
            PdfObject::Name(PdfName::new("FlateDecode".to_string())),
        );

        let stream = PdfStream {
            dict: dict.clone(),
            data: b"Hello World".to_vec(),
        };

        assert_eq!(stream.data.len(), 11);
        assert_eq!(stream.raw_data(), b"Hello World");

        // Verify dictionary
        if let Some(PdfObject::Name(name)) = stream.dict.0.get(&PdfName::new("Filter".to_string()))
        {
            assert_eq!(name.0, "FlateDecode");
        }
    }

    #[test]
    fn test_pdf_name_creation_and_equality() {
        let name1 = PdfName::new("Type".to_string());
        let name2 = PdfName::new("Type".to_string());
        let name3 = PdfName::new("Subtype".to_string());

        // Test equality
        assert_eq!(name1, name2);
        assert_ne!(name1, name3);

        // Test as hash key
        let mut dict = HashMap::new();
        dict.insert(name1.clone(), PdfObject::Integer(1));
        dict.insert(name3.clone(), PdfObject::Integer(2));

        assert_eq!(dict.len(), 2);
        assert!(dict.contains_key(&PdfName::new("Type".to_string())));
    }

    #[test]
    fn test_pdf_string_variations() {
        // Regular string
        let str1 = PdfObject::String(PdfString(b"Hello PDF".to_vec()));

        // String with special chars
        let str2 = PdfObject::String(PdfString(b"Line1\nLine2".to_vec()));

        // Empty string
        let str3 = PdfObject::String(PdfString(vec![]));

        // Binary string
        let str4 = PdfObject::String(PdfString(vec![0xFF, 0xFE, 0x00, 0x01]));

        match str1 {
            PdfObject::String(data) => assert_eq!(data.0, b"Hello PDF"),
            _ => panic!("Wrong type"),
        }

        match str3 {
            PdfObject::String(data) => assert!(data.0.is_empty()),
            _ => panic!("Wrong type"),
        }

        match str4 {
            PdfObject::String(data) => assert_eq!(data.0.len(), 4),
            _ => panic!("Wrong type"),
        }
    }

    #[test]
    fn test_pdf_hex_string() {
        // HexString variant doesn't exist in current API, using String instead
        let hex = PdfObject::String(PdfString(vec![0xDE, 0xAD, 0xBE, 0xEF]));

        match hex {
            PdfObject::String(data) => {
                assert_eq!(data.0.len(), 4);
                assert_eq!(data.0[0], 0xDE);
                assert_eq!(data.0[3], 0xEF);
            }
            _ => panic!("Wrong type"),
        }
    }

    #[test]
    fn test_pdf_reference_object() {
        let reference = PdfObject::Reference(10, 0);

        match reference {
            PdfObject::Reference(obj_num, gen_num) => {
                assert_eq!(obj_num, 10);
                assert_eq!(gen_num, 0);
            }
            _ => panic!("Wrong type"),
        }
    }

    #[test]
    fn test_nested_structures() {
        // Create nested array in dictionary
        let mut inner_array = PdfArray(vec![
            PdfObject::Integer(1),
            PdfObject::Integer(2),
            PdfObject::Integer(3),
        ]);

        let mut dict = PdfDictionary(HashMap::new());
        dict.0.insert(
            PdfName::new("Numbers".to_string()),
            PdfObject::Array(inner_array.clone()),
        );

        // Create nested dictionary
        let mut inner_dict = PdfDictionary(HashMap::new());
        inner_dict
            .0
            .insert(PdfName::new("Nested".to_string()), PdfObject::Boolean(true));
        dict.0.insert(
            PdfName::new("SubDict".to_string()),
            PdfObject::Dictionary(inner_dict),
        );

        // Verify nested structures
        if let Some(PdfObject::Array(arr)) = dict.0.get(&PdfName::new("Numbers".to_string())) {
            assert_eq!(arr.0.len(), 3);
        }

        if let Some(PdfObject::Dictionary(sub)) = dict.0.get(&PdfName::new("SubDict".to_string())) {
            assert!(sub.0.contains_key(&PdfName::new("Nested".to_string())));
        }
    }

    #[test]
    fn test_pdf_null_object() {
        let null = PdfObject::Null;

        match null {
            PdfObject::Null => assert!(true),
            _ => panic!("Wrong type"),
        }

        // Test null in dictionary
        let mut dict = PdfDictionary(HashMap::new());
        dict.0
            .insert(PdfName::new("Empty".to_string()), PdfObject::Null);

        if let Some(PdfObject::Null) = dict.0.get(&PdfName::new("Empty".to_string())) {
            assert!(true);
        } else {
            panic!("Null not stored correctly");
        }
    }

    #[test]
    fn test_large_dictionary() {
        let mut dict = PdfDictionary(HashMap::new());

        // Add many entries
        for i in 0..100 {
            let key = PdfName::new(format!("Key{}", i));
            dict.0.insert(key, PdfObject::Integer(i));
        }

        assert_eq!(dict.0.len(), 100);

        // Verify some entries
        if let Some(PdfObject::Integer(val)) = dict.0.get(&PdfName::new("Key50".to_string())) {
            assert_eq!(*val, 50);
        }
    }

    #[test]
    fn test_empty_collections() {
        // Empty dictionary
        let empty_dict = PdfDictionary(HashMap::new());
        assert!(empty_dict.0.is_empty());

        // Empty array
        let empty_array = PdfArray(vec![]);
        assert!(empty_array.0.is_empty());

        // Empty stream
        let mut dict = PdfDictionary(HashMap::new());
        dict.0
            .insert(PdfName::new("Length".to_string()), PdfObject::Integer(0));
        let empty_stream = PdfStream { dict, data: vec![] };
        assert!(empty_stream.data.is_empty());
    }

    #[test]
    fn test_special_characters_in_names() {
        // PDF names can contain special characters
        let name1 = PdfName::new("Name#20with#20spaces".to_string());
        let name2 = PdfName::new("Name/with/slashes".to_string());
        let name3 = PdfName::new("Name(with)parens".to_string());

        assert_eq!(name1.0, "Name#20with#20spaces");
        assert_eq!(name2.0, "Name/with/slashes");
        assert_eq!(name3.0, "Name(with)parens");
    }

    #[test]
    fn test_mixed_type_array() {
        let array = PdfArray(vec![
            PdfObject::Integer(42),
            PdfObject::Real(3.14),
            PdfObject::Boolean(true),
            PdfObject::String(PdfString(b"mixed".to_vec())),
            PdfObject::Null,
            PdfObject::Name(PdfName::new("TypeName".to_string())),
        ]);

        assert_eq!(array.0.len(), 6);

        // Verify each type
        match &array.0[0] {
            PdfObject::Integer(val) => assert_eq!(*val, 42),
            _ => panic!("Wrong type at index 0"),
        }

        match &array.0[1] {
            PdfObject::Real(val) => assert!((val - 3.14).abs() < 0.001),
            _ => panic!("Wrong type at index 1"),
        }

        match &array.0[2] {
            PdfObject::Boolean(val) => assert!(*val),
            _ => panic!("Wrong type at index 2"),
        }
    }

    #[test]
    fn test_stream_with_multiple_filters() {
        let mut dict = PdfDictionary(HashMap::new());

        // Multiple filters as array
        let filters = PdfArray(vec![
            PdfObject::Name(PdfName::new("ASCIIHexDecode".to_string())),
            PdfObject::Name(PdfName::new("FlateDecode".to_string())),
        ]);

        dict.0.insert(
            PdfName::new("Filter".to_string()),
            PdfObject::Array(filters),
        );
        dict.0
            .insert(PdfName::new("Length".to_string()), PdfObject::Integer(100));

        let stream = PdfStream {
            dict: dict.clone(),
            data: vec![0u8; 100],
        };

        // Verify filters array
        if let Some(PdfObject::Array(filters)) =
            stream.dict.0.get(&PdfName::new("Filter".to_string()))
        {
            assert_eq!(filters.0.len(), 2);
        }
    }

    #[test]
    fn test_dictionary_update_operations() {
        let mut dict = PdfDictionary(HashMap::new());

        // Initial insert
        dict.0
            .insert(PdfName::new("Version".to_string()), PdfObject::Real(1.0));
        assert_eq!(dict.0.len(), 1);

        // Update existing
        dict.0
            .insert(PdfName::new("Version".to_string()), PdfObject::Real(2.0));
        assert_eq!(dict.0.len(), 1); // Still 1 entry

        // Verify updated value
        if let Some(PdfObject::Real(val)) = dict.0.get(&PdfName::new("Version".to_string())) {
            assert_eq!(*val, 2.0);
        }

        // Remove entry
        dict.0.remove(&PdfName::new("Version".to_string()));
        assert!(dict.0.is_empty());
    }
}
