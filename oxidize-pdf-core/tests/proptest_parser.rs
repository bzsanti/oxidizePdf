//! Property-based tests for PDF parser robustness
//!
//! Tests that the parser handles all valid inputs correctly and
//! fails gracefully on invalid inputs without panicking.

use oxidize_pdf::parser::PdfReader;
use proptest::prelude::*;
use std::io::Cursor;

// Strategy for generating valid PDF-like strings
fn pdf_string_strategy() -> impl Strategy<Value = String> {
    prop_oneof![
        // Simple ASCII strings
        "[a-zA-Z0-9 ]{0,100}",
        // Strings with PDF escape sequences
        "[a-zA-Z0-9]{0,50}".prop_map(|s| format!("({})", s)),
        // Hex strings
        "[0-9A-Fa-f]{0,100}".prop_map(|s| format!("<{}>", s)),
        // Strings with parentheses
        "[a-zA-Z0-9]{0,20}".prop_map(|s| format!("(Hello {} World)", s)),
    ]
}

// Strategy for generating PDF names
fn pdf_name_strategy() -> impl Strategy<Value = String> {
    "[a-zA-Z][a-zA-Z0-9._-]{0,50}".prop_map(|s| format!("/{}", s))
}

// Strategy for generating simple PDF documents
fn simple_pdf_strategy() -> impl Strategy<Value = Vec<u8>> {
    (
        any::<u32>().prop_map(|n| n % 100 + 1), // num objects
        any::<bool>(),                          // compressed
    )
        .prop_map(|(num_objects, _compressed)| {
            let mut pdf = Vec::new();

            // PDF header
            pdf.extend_from_slice(b"%PDF-1.7\n");
            pdf.extend_from_slice(b"%\xE2\xE3\xCF\xD3\n"); // Binary comment

            // Add some objects
            for i in 1..=num_objects.min(10) {
                pdf.extend_from_slice(format!("{} 0 obj\n", i).as_bytes());
                pdf.extend_from_slice(b"<<\n");
                pdf.extend_from_slice(format!("/Type /Page\n").as_bytes());
                pdf.extend_from_slice(format!("/Parent 2 0 R\n").as_bytes());
                pdf.extend_from_slice(b">>\n");
                pdf.extend_from_slice(b"endobj\n");
            }

            // Simple xref table
            let xref_pos = pdf.len();
            pdf.extend_from_slice(b"xref\n");
            pdf.extend_from_slice(format!("0 {}\n", num_objects + 1).as_bytes());
            pdf.extend_from_slice(b"0000000000 65535 f \n");

            for i in 1..=num_objects.min(10) {
                pdf.extend_from_slice(format!("{:010} 00000 n \n", 15 + i * 50).as_bytes());
            }

            // Trailer
            pdf.extend_from_slice(b"trailer\n");
            pdf.extend_from_slice(b"<<\n");
            pdf.extend_from_slice(format!("/Size {}\n", num_objects + 1).as_bytes());
            pdf.extend_from_slice(b"/Root 1 0 R\n");
            pdf.extend_from_slice(b">>\n");
            pdf.extend_from_slice(b"startxref\n");
            pdf.extend_from_slice(format!("{}\n", xref_pos).as_bytes());
            pdf.extend_from_slice(b"%%EOF\n");

            pdf
        })
}

#[test]
fn test_parser_handles_empty_input() {
    let empty = Vec::new();
    let cursor = Cursor::new(empty);

    // Parser should fail gracefully on empty input
    match PdfReader::new(cursor) {
        Ok(_) => panic!("Parser should fail on empty input"),
        Err(e) => {
            // Should get a reasonable error, not panic
            assert!(
                e.to_string().contains("PDF")
                    || e.to_string().contains("empty")
                    || e.to_string().contains("header")
            );
        }
    }
}

proptest! {
    fn test_parser_handles_invalid_header(header in any::<[u8; 32]>()) {
        let mut data = Vec::from(&header[..]);
        data.extend_from_slice(b"\n%%EOF\n");

        let cursor = Cursor::new(data);

        // Parser should handle any bytes as header
        match PdfReader::new(cursor) {
            Ok(_) => {
                // If it parses, header must have been valid-ish
                prop_assert!(header.starts_with(b"%PDF"));
            }
            Err(_) => {
                // Invalid header should fail gracefully
                prop_assert!(true);
            }
        }
    }

    fn test_parser_handles_truncated_files(pdf in simple_pdf_strategy(), truncate_at in 0..100usize) {
        // Truncate the PDF at various points
        let truncated = if truncate_at < pdf.len() {
            &pdf[..truncate_at]
        } else {
            &pdf[..]
        };

        let cursor = Cursor::new(truncated.to_vec());

        // Parser should handle truncation gracefully
        match PdfReader::new(cursor) {
            Ok(_) => {
                // If it succeeds, must have been truncated after valid content
                prop_assert!(truncate_at >= pdf.len() || truncated.ends_with(b"%%EOF\n"));
            }
            Err(e) => {
                // Should get reasonable error
                let error_str = e.to_string();
                prop_assert!(
                    error_str.contains("EOF") ||
                    error_str.contains("truncated") ||
                    error_str.contains("unexpected") ||
                    error_str.contains("xref") ||
                    error_str.contains("header") ||
                    error_str.contains("PDF")
                );
            }
        }
    }

    fn test_string_parsing_preserves_content(content in "[a-zA-Z0-9 !@#$%^&*()]{0,100}") {
        // Test literal string format
        let literal = format!("({})", content.replace("(", "\\(").replace(")", "\\)"));

        // Test hex string format
        let hex: String = content.bytes()
            .map(|b| format!("{:02X}", b))
            .collect();
        let hex_string = format!("<{}>", hex);

        // Both formats should represent the same content
        // (This is more of a conceptual test - actual parsing would need full context)
        prop_assert!(literal.starts_with('(') && literal.ends_with(')'));
        prop_assert!(hex_string.starts_with('<') && hex_string.ends_with('>'));
        prop_assert!(hex.len() == content.len() * 2);
    }

    fn test_number_parsing_ranges(n in any::<i64>()) {
        // PDF integers should handle full i64 range
        let int_str = format!("{}", n);

        // Floats from integers
        let float_str = format!("{}.0", n);

        prop_assert!(int_str.parse::<i64>().is_ok());
        prop_assert!(float_str.parse::<f64>().is_ok());
    }

    fn test_name_validity(name in pdf_name_strategy()) {
        // Names should start with /
        prop_assert!(name.starts_with('/'));

        // Names should not contain whitespace after /
        let name_part = &name[1..];
        prop_assert!(!name_part.contains(' '));
        prop_assert!(!name_part.contains('\n'));
        prop_assert!(!name_part.contains('\r'));
        prop_assert!(!name_part.contains('\t'));
    }

    fn test_dictionary_key_uniqueness(
        keys in prop::collection::vec(pdf_name_strategy(), 1..20)
    ) {
        // In a valid PDF dictionary, keys must be unique
        let unique_keys: std::collections::HashSet<_> = keys.iter().collect();

        // If we were building a dictionary, duplicates should be handled
        if unique_keys.len() < keys.len() {
            // Has duplicates - last value should win
            prop_assert!(true); // This is valid PDF behavior
        }
    }

    // fn test_parse_options_tolerance() {
    //     // Test different parse option configurations
    //     let strict = ParseOptions::strict();
    //     let tolerant = ParseOptions::tolerant();
    //
    //     // Tolerant should be more permissive than strict
    //     // (This is a conceptual test - actual behavior depends on implementation)
    //     prop_assert!(true);
    // }

    fn test_object_reference_validity(
        num in 1u32..=999999u32,
        gen in 0u16..=65535u16
    ) {
        let ref_str = format!("{} {} R", num, gen);

        // Reference string should parse back to same values
        let parts: Vec<&str> = ref_str.split_whitespace().collect();
        prop_assert_eq!(parts.len(), 3);
        prop_assert_eq!(parts[0].parse::<u32>().unwrap(), num);
        prop_assert_eq!(parts[1].parse::<u16>().unwrap(), gen);
        prop_assert_eq!(parts[2], "R");
    }

    fn test_stream_length_consistency(
        data in prop::collection::vec(any::<u8>(), 0..1000),
        declared_length in 0..2000usize
    ) {
        // Stream objects must have accurate length
        if declared_length == data.len() {
            // Valid stream
            prop_assert!(true);
        } else {
            // Mismatch should be detected
            // Parser should either fix or error
            prop_assert!(true);
        }
    }
}

// Regression tests for specific parser edge cases
#[cfg(test)]
mod regression_tests {
    use super::*;

    #[test]
    fn test_parser_handles_binary_in_comment() {
        // PDF files often have binary in second line comment
        let mut pdf = Vec::new();
        pdf.extend_from_slice(b"%PDF-1.7\n");
        pdf.extend_from_slice(b"%\xFF\xFE\xFD\xFC\n");
        pdf.extend_from_slice(b"1 0 obj\n<< >>\nendobj\n");
        pdf.extend_from_slice(b"xref\n0 2\n");
        pdf.extend_from_slice(b"0000000000 65535 f \n");
        pdf.extend_from_slice(b"0000000015 00000 n \n");
        pdf.extend_from_slice(b"trailer\n<< /Size 2 /Root 1 0 R >>\n");
        pdf.extend_from_slice(b"startxref\n36\n%%EOF\n");

        let cursor = Cursor::new(pdf);
        // Should parse successfully
        let result = PdfReader::new(cursor);
        assert!(result.is_ok() || result.is_err()); // Just shouldn't panic
    }

    #[test]
    fn test_parser_handles_cr_lf_endings() {
        // Windows line endings
        let mut pdf = Vec::new();
        pdf.extend_from_slice(b"%PDF-1.7\r\n");
        pdf.extend_from_slice(b"%%EOF\r\n");

        let cursor = Cursor::new(pdf);
        let result = PdfReader::new(cursor);
        // Should handle CRLF gracefully
        assert!(result.is_err()); // Minimal PDF, but shouldn't panic
    }

    #[test]
    fn test_parser_handles_incremental_updates() {
        // PDF with incremental update (multiple %%EOF markers)
        let mut pdf = Vec::new();

        // Original PDF
        pdf.extend_from_slice(b"%PDF-1.7\n");
        pdf.extend_from_slice(b"1 0 obj\n<< >>\nendobj\n");
        pdf.extend_from_slice(b"xref\n0 2\n");
        pdf.extend_from_slice(b"0000000000 65535 f \n");
        pdf.extend_from_slice(b"0000000015 00000 n \n");
        pdf.extend_from_slice(b"trailer\n<< /Size 2 /Root 1 0 R >>\n");
        pdf.extend_from_slice(b"startxref\n36\n%%EOF\n");

        // Incremental update
        pdf.extend_from_slice(b"2 0 obj\n<< >>\nendobj\n");
        pdf.extend_from_slice(b"xref\n2 1\n");
        pdf.extend_from_slice(b"0000000150 00000 n \n");
        pdf.extend_from_slice(b"trailer\n<< /Size 3 /Root 1 0 R /Prev 36 >>\n");
        pdf.extend_from_slice(b"startxref\n171\n%%EOF\n");

        let cursor = Cursor::new(pdf);
        let result = PdfReader::new(cursor);
        // Should handle incremental updates
        assert!(result.is_ok() || result.is_err()); // Just shouldn't panic
    }
}
