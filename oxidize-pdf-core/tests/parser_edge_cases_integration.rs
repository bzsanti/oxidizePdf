//! Parser Edge Cases Integration Tests
//!
//! Comprehensive tests for parser robustness with corrupted, malformed, and edge case PDF files.
//! These tests ensure the parser handles problematic inputs gracefully without panicking.
//!
//! Test categories:
//! - Corrupted PDF structures (headers, xref, objects)
//! - Memory exhaustion scenarios
//! - Encoding edge cases and malformed data
//! - Circular references and infinite loops
//! - Boundary conditions and limits
//! - Recovery from parsing errors

use oxidize_pdf::document::Document;
use oxidize_pdf::parser::{ParseOptions, PdfReader};
use oxidize_pdf::Page;
use std::fs;
use std::io::Cursor;
use tempfile::TempDir;

/// Test parsing completely empty file
#[test]
fn test_empty_file_handling() {
    let empty_data = Vec::new();
    let cursor = Cursor::new(empty_data);

    let result = PdfReader::new(cursor);
    assert!(result.is_err());

    if let Err(error) = result {
        println!("Empty file error: {error}");
        // Should get a specific error, not panic
        assert!(error.to_string().contains("empty") || error.to_string().contains("Invalid"));
    }
}

/// Test parsing file with only whitespace
#[test]
fn test_whitespace_only_file() {
    let whitespace_data = b"   \t\r\n   \t\r\n   ".to_vec();
    let cursor = Cursor::new(whitespace_data);

    let result = PdfReader::new(cursor);
    assert!(result.is_err());

    if let Err(error) = result {
        println!("Whitespace-only file error: {error}");
        assert!(!error.to_string().is_empty());
    }
}

/// Test parsing file with invalid PDF header
#[test]
fn test_invalid_pdf_header() {
    let invalid_headers = vec![
        b"PD-1.4\n".to_vec(),      // Missing F
        b"%PDF\n".to_vec(),        // Missing version
        b"%PDF-99.99\n".to_vec(),  // Invalid version
        b"PDF-1.4\n".to_vec(),     // Missing %
        b"%pdf-1.4\n".to_vec(),    // Lowercase
        b"%PDF-1.4".to_vec(),      // Missing newline
        b"%%PDF-1.4\n".to_vec(),   // Double %
        b"%PDF-1.4\r".to_vec(),    // Only CR
        b"%PDF-1.4\r\n%".to_vec(), // Truncated comment
    ];

    for (i, invalid_header) in invalid_headers.iter().enumerate() {
        println!(
            "Testing invalid header {}: {:?}",
            i,
            String::from_utf8_lossy(invalid_header)
        );

        let cursor = Cursor::new(invalid_header.clone());
        let result = PdfReader::new(cursor);

        assert!(result.is_err(), "Invalid header {i} should fail parsing");

        if let Err(error) = result {
            println!("  Error: {error}");
            assert!(
                error.to_string().contains("header")
                    || error.to_string().contains("Invalid")
                    || error.to_string().contains("PDF"),
                "Error should mention header or PDF format issue"
            );
        }
    }
}

/// Test parsing file with truncated content after valid header
#[test]
fn test_truncated_after_header() {
    let truncated_files = vec![
        b"%PDF-1.4\n%".to_vec(),                      // Truncated comment
        b"%PDF-1.4\n%%EOF".to_vec(),                  // Missing xref
        b"%PDF-1.4\n1 0 obj\n".to_vec(),              // Truncated object
        b"%PDF-1.4\n1 0 obj\n<<".to_vec(),            // Truncated dictionary
        b"%PDF-1.4\n1 0 obj\n<</Type".to_vec(),       // Truncated name
        b"%PDF-1.4\n1 0 obj\n<</Type /Page".to_vec(), // Truncated dictionary
    ];

    for (i, truncated_data) in truncated_files.iter().enumerate() {
        println!(
            "Testing truncated file {}: {:?}",
            i,
            String::from_utf8_lossy(truncated_data)
        );

        let cursor = Cursor::new(truncated_data.clone());
        let result = PdfReader::new(cursor);

        // Should handle gracefully, not panic
        match result {
            Ok(_) => println!("  Unexpectedly succeeded"),
            Err(error) => {
                println!("  Error (expected): {error}");
                assert!(!error.to_string().is_empty());
            }
        }
    }
}

/// Test parsing with malformed xref table
#[test]
fn test_malformed_xref_table() {
    let malformed_xrefs = vec![
        // Missing xref keyword
        create_pdf_with_xref(""),
        // Invalid xref format
        create_pdf_with_xref("xref\ninvalid format\n"),
        // Negative numbers
        create_pdf_with_xref("xref\n-1 1\n0000000000 65535 f \n"),
        // Out of range generation numbers
        create_pdf_with_xref("xref\n0 1\n0000000000 99999 f \n"),
        // Missing entries
        create_pdf_with_xref("xref\n0 2\n0000000000 65535 f \n"),
        // Invalid offset format
        create_pdf_with_xref("xref\n0 1\ninvalid_offset 00000 f \n"),
        // Mixed invalid entries
        create_pdf_with_xref(
            "xref\n0 3\n0000000000 65535 f \n0000000015 00000 n \ninvalid_entry\n",
        ),
    ];

    for (i, pdf_data) in malformed_xrefs.iter().enumerate() {
        println!("Testing malformed xref {i}");

        let cursor = Cursor::new(pdf_data.clone());
        let result = PdfReader::new(cursor);

        // Should handle gracefully
        match result {
            Ok(_) => println!("  Unexpectedly succeeded"),
            Err(error) => {
                println!("  Error (expected): {error}");
                assert!(
                    error.to_string().contains("xref")
                        || error.to_string().contains("Invalid")
                        || error.to_string().contains("Parse")
                        || error.to_string().contains("Syntax")
                        || error.to_string().contains("keyword"),
                    "Error should mention xref or parsing issue"
                );
            }
        }
    }
}

/// Test parsing with circular object references
#[test]
fn test_circular_references() {
    let circular_pdf = create_pdf_with_circular_refs();
    let cursor = Cursor::new(circular_pdf);

    let result = PdfReader::new(cursor);

    match result {
        Ok(mut reader) => {
            println!("Reader created, attempting to parse document...");

            // Try to access objects that might cause infinite loops
            // Use a timeout to prevent tests from hanging
            let start = std::time::Instant::now();
            let timeout = std::time::Duration::from_secs(5);

            let mut object_accessed = false;
            while start.elapsed() < timeout {
                // Try to read some objects
                match reader.get_object(1, 0) {
                    Ok(_) => {
                        object_accessed = true;
                        break;
                    }
                    Err(_) => {
                        // Try next object
                        if reader.get_object(2, 0).is_ok() {
                            object_accessed = true;
                            break;
                        }
                        break; // Give up after trying a couple objects
                    }
                }
            }

            if start.elapsed() >= timeout {
                panic!("Timeout accessing objects - possible infinite loop");
            }

            println!("  Circular reference handled safely (accessed object: {object_accessed})");
        }
        Err(error) => {
            println!("  Parse error (acceptable): {error}");
        }
    }
}

/// Test with extremely large objects that could cause memory issues
#[test]
fn test_memory_exhaustion_protection() {
    // Create PDF with extremely large objects
    let large_object_pdfs = vec![
        create_pdf_with_large_string(1_000_000), // 1MB string
        create_pdf_with_large_array(100_000),    // 100K array elements
        create_pdf_with_large_stream(5_000_000), // 5MB stream
    ];

    for (i, pdf_data) in large_object_pdfs.iter().enumerate() {
        println!("Testing memory exhaustion protection {i}");

        let cursor = Cursor::new(pdf_data.clone());
        let result = PdfReader::new(cursor);

        match result {
            Ok(_) => println!("  Large object handled"),
            Err(error) => {
                println!("  Error (may be expected): {error}");
                // Should get a proper error, not run out of memory
                assert!(
                    error.to_string().contains("too large")
                        || error.to_string().contains("memory")
                        || error.to_string().contains("limit")
                        || error.to_string().contains("Invalid")
                        || error.to_string().contains("Parse")
                        || error.to_string().contains("Syntax")
                        || error.to_string().contains("xref")
                        || error.to_string().contains("keyword")
                );
            }
        }
    }
}

/// Test with malformed dictionary structures
#[test]
fn test_malformed_dictionaries() {
    let malformed_dicts = vec![
        // Unclosed dictionaries
        create_pdf_with_dict("<<"),
        create_pdf_with_dict("<</Type /Page"),
        create_pdf_with_dict("<</Type /Page /Parent"),
        // Invalid key-value pairs
        create_pdf_with_dict("<<//Invalid>>"),
        create_pdf_with_dict("<</Key>>"),
        create_pdf_with_dict("<<Key /Value>>"), // Missing /
        // Nested malformed dictionaries
        create_pdf_with_dict("<</Outer <</Inner>>"),
        create_pdf_with_dict("<</A <</B <</C>>>>"),
        // Mixed with invalid objects
        create_pdf_with_dict("<</Type /Page /Invalid [unclosed array>>"),
    ];

    for (i, pdf_data) in malformed_dicts.iter().enumerate() {
        println!("Testing malformed dictionary {i}");

        let cursor = Cursor::new(pdf_data.clone());
        let result = PdfReader::new(cursor);

        match result {
            Ok(mut reader) => {
                println!("  Reader created, trying to access objects...");
                match reader.get_object(1, 0) {
                    Ok(_) => println!("    Object access succeeded"),
                    Err(error) => println!("    Object access error: {error}"),
                }
            }
            Err(error) => {
                println!("  Parse error: {error}");
                assert!(!error.to_string().is_empty());
            }
        }
    }
}

/// Test with malformed array structures
#[test]
fn test_malformed_arrays() {
    let malformed_arrays = vec![
        // Unclosed arrays
        create_pdf_with_array("["),
        create_pdf_with_array("[1 2 3"),
        create_pdf_with_array("[/Type /Page"),
        // Invalid elements
        create_pdf_with_array("[invalid_element]"),
        create_pdf_with_array("[1 2 /]"), // Invalid name
        create_pdf_with_array("[<incomplete_hex]"),
        // Nested malformed arrays
        create_pdf_with_array("[[[]"),
        create_pdf_with_array("[1 [2 [3]"),
        // Mixed with other malformed objects
        create_pdf_with_array("[<</Unclosed dict >]"),
    ];

    for (i, pdf_data) in malformed_arrays.iter().enumerate() {
        println!("Testing malformed array {i}");

        let cursor = Cursor::new(pdf_data.clone());
        let result = PdfReader::new(cursor);

        match result {
            Ok(mut reader) => {
                println!("  Reader created, trying to access objects...");
                match reader.get_object(1, 0) {
                    Ok(_) => println!("    Object access succeeded"),
                    Err(error) => println!("    Object access error: {error}"),
                }
            }
            Err(error) => {
                println!("  Parse error: {error}");
                assert!(
                    error.to_string().contains("array")
                        || error.to_string().contains("Invalid")
                        || error.to_string().contains("Parse")
                        || error.to_string().contains("Syntax")
                        || error.to_string().contains("keyword")
                );
            }
        }
    }
}

/// Test with invalid character encodings
#[test]
fn test_invalid_encodings() {
    let invalid_encodings = vec![
        // Invalid UTF-8 sequences
        create_pdf_with_string(vec![0xFF, 0xFE, 0xFD]),
        create_pdf_with_string(vec![0x80, 0x81, 0x82]),
        create_pdf_with_string(vec![0xC0, 0x80]), // Overlong encoding
        // Invalid hex strings
        create_pdf_with_hex_string("G0"),   // Invalid hex digit
        create_pdf_with_hex_string("ABC"),  // Odd length
        create_pdf_with_hex_string("ZZZZ"), // Invalid hex
        // Control characters
        create_pdf_with_string(vec![0x00, 0x01, 0x02, 0x03]),
        // Mixed invalid sequences
        create_pdf_with_string(vec![0x41, 0xFF, 0x42, 0x80, 0x43]),
    ];

    for (i, pdf_data) in invalid_encodings.iter().enumerate() {
        println!("Testing invalid encoding {i}");

        let cursor = Cursor::new(pdf_data.clone());
        let result = PdfReader::new(cursor);

        match result {
            Ok(mut reader) => {
                println!("  Reader created, trying to access string object...");
                match reader.get_object(1, 0) {
                    Ok(obj) => println!("    Object accessed: {obj:?}"),
                    Err(error) => println!("    Object access error: {error}"),
                }
            }
            Err(error) => {
                println!("  Parse error: {error}");
                assert!(
                    error.to_string().contains("encoding")
                        || error.to_string().contains("string")
                        || error.to_string().contains("Invalid")
                        || error.to_string().contains("Parse")
                        || error.to_string().contains("Syntax")
                        || error.to_string().contains("keyword")
                );
            }
        }
    }
}

/// Test parser limits and boundary conditions
#[test]
fn test_parser_limits() {
    let limit_cases = vec![
        // Maximum nesting depth
        create_deeply_nested_pdf(1000),
        // Very long names
        create_pdf_with_long_name(10000),
        // Many objects
        create_pdf_with_many_objects(1000),
        // Extremely large numbers
        create_pdf_with_extreme_numbers(),
    ];

    for (i, pdf_data) in limit_cases.iter().enumerate() {
        println!("Testing parser limit case {i}");

        let cursor = Cursor::new(pdf_data.clone());
        let start_time = std::time::Instant::now();

        let result = PdfReader::new(cursor);
        let parse_time = start_time.elapsed();

        println!("  Parse time: {parse_time:?}");

        // Should not take too long (prevent infinite loops)
        assert!(
            parse_time < std::time::Duration::from_secs(30),
            "Parsing took too long"
        );

        match result {
            Ok(_) => println!("  Limit case handled successfully"),
            Err(error) => {
                println!("  Parse error: {error}");
                assert!(
                    error.to_string().contains("limit")
                        || error.to_string().contains("too large")
                        || error.to_string().contains("nested")
                        || error.to_string().contains("Invalid")
                        || error.to_string().contains("Parse")
                        || error.to_string().contains("Syntax")
                        || error.to_string().contains("xref")
                        || error.to_string().contains("keyword")
                );
            }
        }
    }
}

/// Test error recovery scenarios
#[test]
fn test_error_recovery() {
    let recovery_cases = vec![
        // Partially valid PDF with some bad objects
        create_pdf_with_mixed_validity(),
        // PDF with recoverable xref issues
        create_pdf_with_recoverable_xref(),
        // PDF with some corrupt streams but valid structure
        create_pdf_with_corrupt_streams(),
    ];

    for (i, pdf_data) in recovery_cases.iter().enumerate() {
        println!("Testing error recovery case {i}");

        let cursor = Cursor::new(pdf_data.clone());

        // Try with lenient parsing options
        let lenient_options = ParseOptions::tolerant();

        let result = PdfReader::new_with_options(cursor, lenient_options);

        match result {
            Ok(mut reader) => {
                println!("  Recovery successful - trying to access objects");
                // Should be able to access at least some objects
                let mut objects_found = 0;
                for obj_num in 1..10 {
                    if reader.get_object(obj_num, 0).is_ok() {
                        objects_found += 1;
                    }
                }
                assert!(
                    objects_found > 0,
                    "Should find at least one object after recovery"
                );
            }
            Err(error) => {
                println!("  Recovery failed: {error}");
                // Even with recovery, some files might be too corrupt
                assert!(!error.to_string().is_empty());
            }
        }
    }
}

/// Test with real-world corrupted PDF samples
#[test]
fn test_real_world_corrupted_samples() {
    // Create some realistic corruption scenarios
    let temp_dir = TempDir::new().unwrap();

    // Create a valid PDF first, then corrupt it
    let mut valid_doc = Document::new();
    valid_doc.set_title("Test Document");

    let page = Page::a4();
    valid_doc.add_page(page);
    // Add some basic content to make it a realistic PDF
    // (We'll skip complex text operations to avoid API complications)

    let valid_path = temp_dir.path().join("valid.pdf");
    valid_doc.save(&valid_path).unwrap();

    // Read the valid PDF and create corrupted versions
    let valid_data = fs::read(&valid_path).unwrap();

    let corrupted_versions = vec![
        corrupt_pdf_header(&valid_data),
        corrupt_pdf_xref(&valid_data),
        corrupt_pdf_objects(&valid_data),
        corrupt_pdf_streams(&valid_data),
        corrupt_pdf_trailer(&valid_data),
        truncate_pdf(&valid_data, 0.5), // Remove half the file
        truncate_pdf(&valid_data, 0.9), // Remove 10%
    ];

    for (i, corrupted_data) in corrupted_versions.iter().enumerate() {
        println!("Testing real-world corruption scenario {i}");

        let cursor = Cursor::new(corrupted_data.clone());
        let result = PdfReader::new(cursor);

        match result {
            Ok(mut reader) => {
                println!("  Corrupted PDF parsed (recovery successful)");
                // Try to access a few objects to verify the reader is functional
                let mut accessible_objects = 0;
                for obj_num in 1..5 {
                    if reader.get_object(obj_num, 0).is_ok() {
                        accessible_objects += 1;
                    }
                }
                println!("  Accessible objects: {accessible_objects}");
            }
            Err(error) => {
                println!("  Corruption detected: {error}");
                assert!(!error.to_string().is_empty());
                // Make sure we get proper error message
                assert!(
                    error.to_string().contains("header")
                        || error.to_string().contains("Invalid")
                        || error.to_string().contains("Parse")
                        || error.to_string().contains("xref")
                        || error.to_string().contains("trailer")
                );
            }
        }
    }
}

// Helper functions to create test PDFs with specific issues

fn create_pdf_with_xref(xref_content: &str) -> Vec<u8> {
    format!(
        "%PDF-1.4\n1 0 obj\n<</Type /Catalog /Pages 2 0 R>>\nendobj\n{xref_content}\ntrailer\n<</Size 1 /Root 1 0 R>>\nstartxref\n100\n%%EOF"
    ).into_bytes()
}

fn create_pdf_with_circular_refs() -> Vec<u8> {
    "%PDF-1.4\n\
        1 0 obj\n<</Type /Catalog /Pages 2 0 R>>\nendobj\n\
        2 0 obj\n<</Type /Pages /Kids [3 0 R] /Count 1 /Parent 4 0 R>>\nendobj\n\
        3 0 obj\n<</Type /Page /Parent 2 0 R /Next 4 0 R>>\nendobj\n\
        4 0 obj\n<</Type /Page /Parent 2 0 R /Next 3 0 R>>\nendobj\n\
        xref\n0 5\n0000000000 65535 f \n0000000015 00000 n \n0000000068 00000 n \n0000000125 00000 n \n0000000180 00000 n \n\
        trailer\n<</Size 5 /Root 1 0 R>>\nstartxref\n230\n%%EOF".to_string().into_bytes()
}

fn create_pdf_with_large_string(size: usize) -> Vec<u8> {
    let large_string = "A".repeat(size);
    format!(
        "%PDF-1.4\n1 0 obj\n({large_string})\nendobj\nxref\n0 2\n0000000000 65535 f \n0000000015 00000 n \ntrailer\n<</Size 2>>\nstartxref\n50\n%%EOF"
    ).into_bytes()
}

fn create_pdf_with_large_array(elements: usize) -> Vec<u8> {
    let array_content = (0..elements)
        .map(|i| i.to_string())
        .collect::<Vec<_>>()
        .join(" ");
    format!(
        "%PDF-1.4\n1 0 obj\n[{array_content}]\nendobj\nxref\n0 2\n0000000000 65535 f \n0000000015 00000 n \ntrailer\n<</Size 2>>\nstartxref\n50\n%%EOF"
    ).into_bytes()
}

fn create_pdf_with_large_stream(size: usize) -> Vec<u8> {
    let stream_data = "x".repeat(size);
    format!(
        "%PDF-1.4\n1 0 obj\n<</Length {size}>>\nstream\n{stream_data}\nendstream\nendobj\nxref\n0 2\n0000000000 65535 f \n0000000015 00000 n \ntrailer\n<</Size 2>>\nstartxref\n100\n%%EOF"
    ).into_bytes()
}

fn create_pdf_with_dict(dict_content: &str) -> Vec<u8> {
    format!(
        "%PDF-1.4\n1 0 obj\n{dict_content}\nendobj\nxref\n0 2\n0000000000 65535 f \n0000000015 00000 n \ntrailer\n<</Size 2>>\nstartxref\n50\n%%EOF"
    ).into_bytes()
}

fn create_pdf_with_array(array_content: &str) -> Vec<u8> {
    format!(
        "%PDF-1.4\n1 0 obj\n{array_content}\nendobj\nxref\n0 2\n0000000000 65535 f \n0000000015 00000 n \ntrailer\n<</Size 2>>\nstartxref\n50\n%%EOF"
    ).into_bytes()
}

fn create_pdf_with_string(bytes: Vec<u8>) -> Vec<u8> {
    let mut result = b"%PDF-1.4\n1 0 obj\n(".to_vec();
    result.extend_from_slice(&bytes);
    result.extend_from_slice(b")\nendobj\nxref\n0 2\n0000000000 65535 f \n0000000015 00000 n \ntrailer\n<</Size 2>>\nstartxref\n50\n%%EOF");
    result
}

fn create_pdf_with_hex_string(hex_content: &str) -> Vec<u8> {
    format!(
        "%PDF-1.4\n1 0 obj\n<{hex_content}>\nendobj\nxref\n0 2\n0000000000 65535 f \n0000000015 00000 n \ntrailer\n<</Size 2>>\nstartxref\n50\n%%EOF"
    ).into_bytes()
}

fn create_deeply_nested_pdf(depth: usize) -> Vec<u8> {
    let mut nested_dict = String::new();
    for _ in 0..depth {
        nested_dict.push_str("<<");
    }
    nested_dict.push_str("/Type /Test");
    for _ in 0..depth {
        nested_dict.push_str(">>");
    }

    create_pdf_with_dict(&nested_dict)
}

fn create_pdf_with_long_name(length: usize) -> Vec<u8> {
    let long_name = format!("/{}", "A".repeat(length));
    create_pdf_with_dict(&format!("<</LongName {long_name}>>"))
}

fn create_pdf_with_many_objects(count: usize) -> Vec<u8> {
    let mut pdf = String::from("%PDF-1.4\n");

    for i in 1..=count {
        pdf.push_str(&format!("{i} 0 obj\n<</Type /Test /Index {i}>>\nendobj\n"));
    }

    pdf.push_str("xref\n");
    pdf.push_str(&format!("0 {}\n", count + 1));
    pdf.push_str("0000000000 65535 f \n");

    let mut offset = 15; // Start after header
    for i in 1..=count {
        pdf.push_str(&format!("{offset:010} 00000 n \n"));
        offset += format!("{i} 0 obj\n<</Type /Test /Index {i}>>\nendobj\n").len();
    }

    pdf.push_str(&format!("trailer\n<</Size {}>>\nstartxref\n", count + 1));
    pdf.push_str(&format!("{offset}\n%%EOF"));

    pdf.into_bytes()
}

fn create_pdf_with_extreme_numbers() -> Vec<u8> {
    create_pdf_with_dict(&format!(
        "<</VeryLarge {} /VerySmall {} /Negative {} /Zero 0>>",
        i64::MAX,
        f64::MIN_POSITIVE,
        i64::MIN
    ))
}

fn create_pdf_with_mixed_validity() -> Vec<u8> {
    "%PDF-1.4\n\
        1 0 obj\n<</Type /Catalog /Pages 2 0 R>>\nendobj\n\
        2 0 obj\n<</Type /Pages>>\nendobj\n\
        3 0 obj\n<<INVALID OBJECT\nendobj\n\
        4 0 obj\n<</Type /Font /Name /Arial>>\nendobj\n\
        xref\n0 5\n0000000000 65535 f \n0000000015 00000 n \n0000000068 00000 n \n0000000100 00000 n \n0000000130 00000 n \n\
        trailer\n<</Size 5 /Root 1 0 R>>\nstartxref\n180\n%%EOF".to_string().into_bytes()
}

fn create_pdf_with_recoverable_xref() -> Vec<u8> {
    "%PDF-1.4\n\
        1 0 obj\n<</Type /Catalog>>\nendobj\n\
        xref\n0 2\n0000000000 65535 f \n0000000015 00000 n \n\
        trailer\n<</Size 2 /Root 1 0 R>>\nstartxref\n50\n%%EOF"
        .to_string()
        .into_bytes()
}

fn create_pdf_with_corrupt_streams() -> Vec<u8> {
    "%PDF-1.4\n\
        1 0 obj\n<</Type /Catalog>>\nendobj\n\
        2 0 obj\n<</Length 100>>\nstream\nCORRUPT_STREAM_DATA_HERE\nendstream\nendobj\n\
        xref\n0 3\n0000000000 65535 f \n0000000015 00000 n \n0000000050 00000 n \n\
        trailer\n<</Size 3 /Root 1 0 R>>\nstartxref\n120\n%%EOF"
        .to_string()
        .into_bytes()
}

// Functions to corrupt existing valid PDFs
fn corrupt_pdf_header(data: &[u8]) -> Vec<u8> {
    let mut corrupted = data.to_vec();
    if corrupted.len() > 5 {
        corrupted[1] = b'X'; // Change %PDF to %XDF
    }
    corrupted
}

fn corrupt_pdf_xref(data: &[u8]) -> Vec<u8> {
    let mut corrupted = data.to_vec();
    if let Some(xref_pos) = find_in_bytes(&corrupted, b"xref") {
        if xref_pos + 10 < corrupted.len() {
            corrupted[xref_pos + 5] = b'X'; // Corrupt xref
        }
    }
    corrupted
}

fn corrupt_pdf_objects(data: &[u8]) -> Vec<u8> {
    let mut corrupted = data.to_vec();
    if let Some(obj_pos) = find_in_bytes(&corrupted, b"obj") {
        if obj_pos > 0 {
            corrupted[obj_pos - 1] = b'X'; // Corrupt object reference
        }
    }
    corrupted
}

fn corrupt_pdf_streams(data: &[u8]) -> Vec<u8> {
    let mut corrupted = data.to_vec();
    if let Some(stream_pos) = find_in_bytes(&corrupted, b"stream") {
        if stream_pos + 10 < corrupted.len() {
            corrupted[stream_pos + 8] = 0xFF; // Insert invalid byte in stream
        }
    }
    corrupted
}

fn corrupt_pdf_trailer(data: &[u8]) -> Vec<u8> {
    let mut corrupted = data.to_vec();
    if let Some(trailer_pos) = find_in_bytes(&corrupted, b"trailer") {
        if trailer_pos + 10 < corrupted.len() {
            corrupted[trailer_pos + 7] = b'X'; // Corrupt trailer
        }
    }
    corrupted
}

fn truncate_pdf(data: &[u8], factor: f64) -> Vec<u8> {
    let new_size = (data.len() as f64 * factor) as usize;
    data[..new_size.min(data.len())].to_vec()
}

fn find_in_bytes(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}
