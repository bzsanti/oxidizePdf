//! Tests for parser/objects.rs to improve code coverage
//!
//! Coverage goal: Increase from 49.2% (160/325 lines) to 60%+ (195+ lines)
//!
//! Focus areas:
//! - Error paths in parsing (lines 350, 359, 395-397, 403-404, 413, 415, 417, 428-430)
//! - Type conversion methods (as_*) edge cases
//! - PdfDictionary helper methods
//! - PdfString and PdfName utilities

use oxidize_pdf::parser::lexer::Lexer;
use oxidize_pdf::parser::objects::{PdfArray, PdfDictionary, PdfName, PdfObject, PdfString};
use oxidize_pdf::parser::{ParseError, ParseOptions};
use std::io::Cursor;

// ============================================================================
// PdfObject Type Conversion Tests (Pure Logic)
// ============================================================================

#[test]
fn test_as_bool_with_boolean() {
    let obj = PdfObject::Boolean(true);
    assert_eq!(obj.as_bool(), Some(true));

    let obj = PdfObject::Boolean(false);
    assert_eq!(obj.as_bool(), Some(false));
}

#[test]
fn test_as_bool_with_non_boolean() {
    let obj = PdfObject::Integer(1);
    assert_eq!(obj.as_bool(), None);

    let obj = PdfObject::Null;
    assert_eq!(obj.as_bool(), None);
}

#[test]
fn test_as_integer_with_integer() {
    let obj = PdfObject::Integer(42);
    assert_eq!(obj.as_integer(), Some(42));

    let obj = PdfObject::Integer(-100);
    assert_eq!(obj.as_integer(), Some(-100));
}

#[test]
fn test_as_integer_with_non_integer() {
    let obj = PdfObject::Real(3.14);
    assert_eq!(obj.as_integer(), None);

    let obj = PdfObject::Boolean(true);
    assert_eq!(obj.as_integer(), None);
}

#[test]
fn test_as_real_with_real() {
    let obj = PdfObject::Real(3.14159);
    assert_eq!(obj.as_real(), Some(3.14159));
}

#[test]
fn test_as_real_with_integer_conversion() {
    // as_real should convert integers to floats
    let obj = PdfObject::Integer(42);
    assert_eq!(obj.as_real(), Some(42.0));
}

#[test]
fn test_as_real_with_non_numeric() {
    let obj = PdfObject::Null;
    assert_eq!(obj.as_real(), None);

    let obj = PdfObject::Boolean(false);
    assert_eq!(obj.as_real(), None);
}

#[test]
fn test_as_string_with_string() {
    let string = PdfString::new(b"Hello".to_vec());
    let obj = PdfObject::String(string.clone());

    assert!(obj.as_string().is_some());
    assert_eq!(obj.as_string().unwrap().as_bytes(), b"Hello");
}

#[test]
fn test_as_string_with_non_string() {
    let obj = PdfObject::Integer(42);
    assert_eq!(obj.as_string(), None);
}

#[test]
fn test_as_name_with_name() {
    let name = PdfName::new("Type".to_string());
    let obj = PdfObject::Name(name.clone());

    assert!(obj.as_name().is_some());
    assert_eq!(obj.as_name().unwrap().as_str(), "Type");
}

#[test]
fn test_as_name_with_non_name() {
    let obj = PdfObject::Boolean(true);
    assert_eq!(obj.as_name(), None);
}

#[test]
fn test_as_array_with_array() {
    let array = PdfArray::new();
    let obj = PdfObject::Array(array);

    assert!(obj.as_array().is_some());
}

#[test]
fn test_as_array_with_non_array() {
    let obj = PdfObject::Null;
    assert_eq!(obj.as_array(), None);
}

#[test]
fn test_as_dict_with_dictionary() {
    let dict = PdfDictionary::new();
    let obj = PdfObject::Dictionary(dict);

    assert!(obj.as_dict().is_some());
}

#[test]
fn test_as_dict_with_non_dict() {
    let obj = PdfObject::Integer(1);
    assert_eq!(obj.as_dict(), None);
}

#[test]
fn test_as_reference_with_reference() {
    let obj = PdfObject::Reference(10, 0);
    assert_eq!(obj.as_reference(), Some((10, 0)));

    let obj = PdfObject::Reference(42, 5);
    assert_eq!(obj.as_reference(), Some((42, 5)));
}

#[test]
fn test_as_reference_with_non_reference() {
    let obj = PdfObject::Null;
    assert_eq!(obj.as_reference(), None);
}

// ============================================================================
// PdfString Tests
// ============================================================================

#[test]
fn test_pdfstring_as_str_valid_utf8() {
    let string = PdfString::new(b"Hello World".to_vec());
    assert!(string.as_str().is_ok());
    assert_eq!(string.as_str().unwrap(), "Hello World");
}

#[test]
fn test_pdfstring_as_str_invalid_utf8() {
    let string = PdfString::new(vec![0xFF, 0xFE, 0xFD]);
    assert!(string.as_str().is_err());
}

#[test]
fn test_pdfstring_as_bytes() {
    let data = vec![0x48, 0x65, 0x6C, 0x6C, 0x6F]; // "Hello"
    let string = PdfString::new(data.clone());
    assert_eq!(string.as_bytes(), data.as_slice());
}

#[test]
fn test_pdfstring_equality() {
    let s1 = PdfString::new(b"test".to_vec());
    let s2 = PdfString::new(b"test".to_vec());
    let s3 = PdfString::new(b"other".to_vec());

    assert_eq!(s1, s2);
    assert_ne!(s1, s3);
}

// ============================================================================
// PdfName Tests
// ============================================================================

#[test]
fn test_pdfname_as_str() {
    let name = PdfName::new("Type".to_string());
    assert_eq!(name.as_str(), "Type");
}

#[test]
fn test_pdfname_equality() {
    let n1 = PdfName::new("Pages".to_string());
    let n2 = PdfName::new("Pages".to_string());
    let n3 = PdfName::new("Page".to_string());

    assert_eq!(n1, n2);
    assert_ne!(n1, n3);
}

// ============================================================================
// PdfArray Tests
// ============================================================================

#[test]
fn test_pdfarray_len_and_is_empty() {
    let mut array = PdfArray::new();
    assert_eq!(array.len(), 0);
    assert!(array.is_empty());

    array.push(PdfObject::Integer(1));
    assert_eq!(array.len(), 1);
    assert!(!array.is_empty());

    array.push(PdfObject::Integer(2));
    assert_eq!(array.len(), 2);
}

#[test]
fn test_pdfarray_push_and_get() {
    let mut array = PdfArray::new();
    array.push(PdfObject::Integer(42));
    array.push(PdfObject::Boolean(true));

    assert_eq!(array.get(0), Some(&PdfObject::Integer(42)));
    assert_eq!(array.get(1), Some(&PdfObject::Boolean(true)));
    assert_eq!(array.get(2), None);
}

#[test]
fn test_pdfarray_multiple_elements() {
    let mut array = PdfArray::new();
    array.push(PdfObject::Integer(1));
    array.push(PdfObject::Integer(2));
    array.push(PdfObject::Integer(3));

    assert_eq!(array.len(), 3);
    assert_eq!(array.get(0).unwrap().as_integer(), Some(1));
    assert_eq!(array.get(1).unwrap().as_integer(), Some(2));
    assert_eq!(array.get(2).unwrap().as_integer(), Some(3));
}

// ============================================================================
// PdfDictionary Tests
// ============================================================================

#[test]
fn test_pdfdictionary_insert_and_get() {
    let mut dict = PdfDictionary::new();
    dict.insert(
        "Type".to_string(),
        PdfObject::Name(PdfName::new("Page".to_string())),
    );

    assert!(dict.get("Type").is_some());
    assert_eq!(
        dict.get("Type").unwrap().as_name().unwrap().as_str(),
        "Page"
    );
}

#[test]
fn test_pdfdictionary_get_type() {
    let mut dict = PdfDictionary::new();
    dict.insert(
        "Type".to_string(),
        PdfObject::Name(PdfName::new("Catalog".to_string())),
    );

    assert_eq!(dict.get_type(), Some("Catalog"));
}

#[test]
fn test_pdfdictionary_get_type_missing() {
    let dict = PdfDictionary::new();
    assert_eq!(dict.get_type(), None);
}

#[test]
fn test_pdfdictionary_contains_key() {
    let mut dict = PdfDictionary::new();
    dict.insert("Pages".to_string(), PdfObject::Integer(1));

    assert!(dict.contains_key("Pages"));
    assert!(!dict.contains_key("Type"));
}

#[test]
fn test_pdfdictionary_multiple_keys() {
    let mut dict = PdfDictionary::new();
    dict.insert(
        "Type".to_string(),
        PdfObject::Name(PdfName::new("Page".to_string())),
    );
    dict.insert("Count".to_string(), PdfObject::Integer(5));

    assert!(dict.contains_key("Type"));
    assert!(dict.contains_key("Count"));
    assert!(!dict.contains_key("Missing"));

    assert_eq!(
        dict.get("Type").unwrap().as_name().unwrap().as_str(),
        "Page"
    );
    assert_eq!(dict.get("Count").unwrap().as_integer(), Some(5));
}

// ============================================================================
// Parsing Error Path Tests
// ============================================================================

#[test]
fn test_parse_with_eof() {
    // Empty input should return EOF error
    let data = b"";
    let cursor = Cursor::new(data);
    let mut lexer = Lexer::new(cursor);

    let result = PdfObject::parse(&mut lexer);
    assert!(result.is_err());
}

#[test]
fn test_parse_with_options() {
    // Test parsing with custom options
    let data = b"true";
    let cursor = Cursor::new(data);
    let mut lexer = Lexer::new(cursor);

    let options = ParseOptions::lenient();
    let result = PdfObject::parse_with_options(&mut lexer, &options);

    assert!(result.is_ok());
    assert_eq!(result.unwrap(), PdfObject::Boolean(true));
}

#[test]
fn test_parse_negative_integer() {
    let data = b"-42";
    let cursor = Cursor::new(data);
    let mut lexer = Lexer::new(cursor);

    let result = PdfObject::parse(&mut lexer);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().as_integer(), Some(-42));
}

#[test]
fn test_parse_large_integer() {
    let data = b"99999999";
    let cursor = Cursor::new(data);
    let mut lexer = Lexer::new(cursor);

    let result = PdfObject::parse(&mut lexer);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().as_integer(), Some(99999999));
}

#[test]
fn test_parse_reference() {
    let data = b"10 0 R";
    let cursor = Cursor::new(data);
    let mut lexer = Lexer::new(cursor);

    let result = PdfObject::parse(&mut lexer);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().as_reference(), Some((10, 0)));
}

#[test]
fn test_parse_array() {
    let data = b"[1 2 3]";
    let cursor = Cursor::new(data);
    let mut lexer = Lexer::new(cursor);

    let result = PdfObject::parse(&mut lexer);
    assert!(result.is_ok());

    if let Some(array) = result.unwrap().as_array() {
        assert_eq!(array.len(), 3);
        assert_eq!(array.get(0).unwrap().as_integer(), Some(1));
        assert_eq!(array.get(1).unwrap().as_integer(), Some(2));
        assert_eq!(array.get(2).unwrap().as_integer(), Some(3));
    } else {
        panic!("Expected array");
    }
}

#[test]
fn test_parse_empty_array() {
    let data = b"[]";
    let cursor = Cursor::new(data);
    let mut lexer = Lexer::new(cursor);

    let result = PdfObject::parse(&mut lexer);
    assert!(result.is_ok());

    if let Some(array) = result.unwrap().as_array() {
        assert_eq!(array.len(), 0);
        assert!(array.is_empty());
    } else {
        panic!("Expected array");
    }
}

#[test]
fn test_parse_dictionary() {
    let data = b"<< /Type /Page >>";
    let cursor = Cursor::new(data);
    let mut lexer = Lexer::new(cursor);

    let result = PdfObject::parse(&mut lexer);
    assert!(result.is_ok());

    if let Some(dict) = result.unwrap().as_dict() {
        assert_eq!(dict.get_type(), Some("Page"));
    } else {
        panic!("Expected dictionary");
    }
}

#[test]
fn test_parse_empty_dictionary() {
    let data = b"<< >>";
    let cursor = Cursor::new(data);
    let mut lexer = Lexer::new(cursor);

    let result = PdfObject::parse(&mut lexer);
    assert!(result.is_ok());

    if let Some(dict) = result.unwrap().as_dict() {
        // Empty dictionary should have no keys
        assert_eq!(dict.get_type(), None);
    } else {
        panic!("Expected dictionary");
    }
}

#[test]
fn test_parse_null() {
    let data = b"null";
    let cursor = Cursor::new(data);
    let mut lexer = Lexer::new(cursor);

    let result = PdfObject::parse(&mut lexer);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), PdfObject::Null);
}

#[test]
fn test_parse_real_number() {
    let data = b"3.14159";
    let cursor = Cursor::new(data);
    let mut lexer = Lexer::new(cursor);

    let result = PdfObject::parse(&mut lexer);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().as_real(), Some(3.14159));
}

#[test]
fn test_parse_literal_string() {
    let data = b"(Hello World)";
    let cursor = Cursor::new(data);
    let mut lexer = Lexer::new(cursor);

    let result = PdfObject::parse(&mut lexer);
    assert!(result.is_ok());

    if let Some(string) = result.unwrap().as_string() {
        assert_eq!(string.as_str().unwrap(), "Hello World");
    } else {
        panic!("Expected string");
    }
}

#[test]
fn test_parse_name() {
    let data = b"/Type";
    let cursor = Cursor::new(data);
    let mut lexer = Lexer::new(cursor);

    let result = PdfObject::parse(&mut lexer);
    assert!(result.is_ok());

    if let Some(name) = result.unwrap().as_name() {
        assert_eq!(name.as_str(), "Type");
    } else {
        panic!("Expected name");
    }
}

// ============================================================================
// Edge Cases
// ============================================================================

#[test]
fn test_nested_arrays() {
    let data = b"[[1 2] [3 4]]";
    let cursor = Cursor::new(data);
    let mut lexer = Lexer::new(cursor);

    let result = PdfObject::parse(&mut lexer);
    assert!(result.is_ok());

    if let Some(array) = result.unwrap().as_array() {
        assert_eq!(array.len(), 2);

        // Check first nested array
        if let Some(nested1) = array.get(0).and_then(|obj| obj.as_array()) {
            assert_eq!(nested1.len(), 2);
            assert_eq!(nested1.get(0).unwrap().as_integer(), Some(1));
        } else {
            panic!("Expected nested array");
        }
    } else {
        panic!("Expected array");
    }
}

#[test]
fn test_dictionary_with_array_value() {
    let data = b"<< /MediaBox [0 0 612 792] >>";
    let cursor = Cursor::new(data);
    let mut lexer = Lexer::new(cursor);

    let result = PdfObject::parse(&mut lexer);
    assert!(result.is_ok());

    if let Some(dict) = result.unwrap().as_dict() {
        if let Some(array) = dict.get("MediaBox").and_then(|obj| obj.as_array()) {
            assert_eq!(array.len(), 4);
            assert_eq!(array.get(0).unwrap().as_integer(), Some(0));
            assert_eq!(array.get(3).unwrap().as_integer(), Some(792));
        } else {
            panic!("Expected MediaBox array");
        }
    } else {
        panic!("Expected dictionary");
    }
}
