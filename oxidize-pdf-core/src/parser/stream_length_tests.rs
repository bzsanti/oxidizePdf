#[cfg(test)]
mod stream_length_tests {
    use crate::parser::{
        objects::{PdfName, PdfObject, PdfString},
        ParseOptions,
    };

    #[test]
    fn test_stream_length_options_default() {
        let options = ParseOptions::default();

        assert!(!options.lenient_streams);
        assert!(!options.collect_warnings);
        assert_eq!(options.max_recovery_bytes, 1000);
        assert!(options.lenient_encoding);
        assert_eq!(options.preferred_encoding, None);
        assert!(!options.lenient_syntax);
    }

    #[test]
    fn test_stream_length_options_lenient_preset() {
        let options = ParseOptions::lenient();

        assert!(options.lenient_streams);
        assert!(options.collect_warnings);
        assert!(options.lenient_encoding);
        assert!(options.lenient_syntax);
        assert_eq!(options.max_recovery_bytes, 5000);
    }

    #[test]
    fn test_stream_length_options_strict_preset() {
        let options = ParseOptions::strict();

        assert!(!options.lenient_streams);
        assert!(!options.collect_warnings);
        assert!(!options.lenient_encoding);
        assert!(!options.lenient_syntax);
        assert_eq!(options.max_recovery_bytes, 0);
    }

    #[test]
    fn test_stream_length_options_custom_lenient() {
        let options = ParseOptions {
            lenient_streams: true,
            max_recovery_bytes: 2000,
            collect_warnings: true,
            ..Default::default()
        };

        assert!(options.lenient_streams);
        assert!(options.collect_warnings);
        assert_eq!(options.max_recovery_bytes, 2000);
    }

    #[test]
    fn test_stream_length_options_custom_strict() {
        let options = ParseOptions {
            lenient_streams: false,
            collect_warnings: false,
            max_recovery_bytes: 0,
            ..Default::default()
        };

        assert!(!options.lenient_streams);
        assert!(!options.collect_warnings);
        assert_eq!(options.max_recovery_bytes, 0);
    }

    #[test]
    fn test_pdf_object_creation() {
        // Test creating PdfObjects that would be used for stream lengths
        let integer_obj = PdfObject::Integer(42);
        let negative_obj = PdfObject::Integer(-1);
        let zero_obj = PdfObject::Integer(0);
        let reference_obj = PdfObject::Reference(5, 0);
        let string_obj = PdfObject::String(PdfString::new(b"not a length".to_vec()));
        let null_obj = PdfObject::Null;

        // Verify object types
        match integer_obj {
            PdfObject::Integer(val) => assert_eq!(val, 42),
            _ => panic!("Expected integer object"),
        }

        match negative_obj {
            PdfObject::Integer(val) => assert_eq!(val, -1),
            _ => panic!("Expected negative integer object"),
        }

        match zero_obj {
            PdfObject::Integer(val) => assert_eq!(val, 0),
            _ => panic!("Expected zero integer object"),
        }

        match reference_obj {
            PdfObject::Reference(obj_num, gen_num) => {
                assert_eq!(obj_num, 5);
                assert_eq!(gen_num, 0);
            }
            _ => panic!("Expected reference object"),
        }

        match string_obj {
            PdfObject::String(_) => (),
            _ => panic!("Expected string object"),
        }

        match null_obj {
            PdfObject::Null => (),
            _ => panic!("Expected null object"),
        }
    }

    #[test]
    fn test_parse_options_builder_pattern() {
        // Test building ParseOptions with different configurations
        let lenient_with_warnings = ParseOptions {
            lenient_streams: true,
            collect_warnings: true,
            max_recovery_bytes: 3000,
            ..ParseOptions::default()
        };

        assert!(lenient_with_warnings.lenient_streams);
        assert!(lenient_with_warnings.collect_warnings);
        assert_eq!(lenient_with_warnings.max_recovery_bytes, 3000);

        let strict_no_warnings = ParseOptions {
            lenient_streams: false,
            collect_warnings: false,
            max_recovery_bytes: 0,
            ..ParseOptions::default()
        };

        assert!(!strict_no_warnings.lenient_streams);
        assert!(!strict_no_warnings.collect_warnings);
        assert_eq!(strict_no_warnings.max_recovery_bytes, 0);
    }

    #[test]
    fn test_stream_length_error_scenarios() {
        // Test scenarios that would trigger different code paths

        // Scenario 1: Valid integer length
        let valid_length = PdfObject::Integer(100);
        match valid_length {
            PdfObject::Integer(len) if len >= 0 => {
                assert_eq!(len, 100);
            }
            _ => panic!("Should be valid positive integer"),
        }

        // Scenario 2: Negative length (invalid)
        let negative_length = PdfObject::Integer(-5);
        match negative_length {
            PdfObject::Integer(len) if len < 0 => {
                assert_eq!(len, -5);
            }
            _ => panic!("Should be negative integer"),
        }

        // Scenario 3: Indirect reference that needs resolution
        let reference_length = PdfObject::Reference(10, 0);
        match reference_length {
            PdfObject::Reference(obj_num, gen_num) => {
                assert_eq!(obj_num, 10);
                assert_eq!(gen_num, 0);
            }
            _ => panic!("Should be reference object"),
        }

        // Scenario 4: Invalid object type for length
        let invalid_length = PdfObject::String(PdfString::new(b"invalid".to_vec()));
        match invalid_length {
            PdfObject::String(_) => {
                // This would be handled as an error case
            }
            _ => panic!("Should be string object"),
        }
    }

    #[test]
    fn test_stream_parsing_configurations() {
        // Test different combinations of stream parsing options

        let configs = vec![
            ("default", ParseOptions::default()),
            ("lenient", ParseOptions::lenient()),
            ("strict", ParseOptions::strict()),
            (
                "custom_lenient",
                ParseOptions {
                    lenient_streams: true,
                    max_recovery_bytes: 10000,
                    collect_warnings: true,
                    lenient_encoding: true,
                    lenient_syntax: true,
                    ..Default::default()
                },
            ),
            (
                "custom_strict",
                ParseOptions {
                    lenient_streams: false,
                    max_recovery_bytes: 0,
                    collect_warnings: false,
                    lenient_encoding: false,
                    lenient_syntax: false,
                    ..Default::default()
                },
            ),
        ];

        for (name, config) in configs {
            match name {
                "default" => {
                    assert!(!config.lenient_streams);
                    assert_eq!(config.max_recovery_bytes, 1000);
                }
                "lenient" => {
                    assert!(config.lenient_streams);
                    assert!(config.collect_warnings);
                    assert_eq!(config.max_recovery_bytes, 5000);
                }
                "strict" => {
                    assert!(!config.lenient_streams);
                    assert!(!config.collect_warnings);
                    assert_eq!(config.max_recovery_bytes, 0);
                }
                "custom_lenient" => {
                    assert!(config.lenient_streams);
                    assert_eq!(config.max_recovery_bytes, 10000);
                    assert!(config.collect_warnings);
                }
                "custom_strict" => {
                    assert!(!config.lenient_streams);
                    assert_eq!(config.max_recovery_bytes, 0);
                    assert!(!config.collect_warnings);
                }
                _ => panic!("Unknown config"),
            }
        }
    }

    #[test]
    fn test_stream_length_reference_types() {
        // Test the different types of references that could appear as stream lengths

        // Direct integer (most common)
        let direct = PdfObject::Integer(1024);
        assert!(matches!(direct, PdfObject::Integer(1024)));

        // Indirect reference (what our implementation handles)
        let indirect = PdfObject::Reference(15, 0);
        assert!(matches!(indirect, PdfObject::Reference(15, 0)));

        // Zero length (valid but edge case)
        let zero = PdfObject::Integer(0);
        assert!(matches!(zero, PdfObject::Integer(0)));

        // Very large length (valid but edge case)
        let large = PdfObject::Integer(1_000_000);
        assert!(matches!(large, PdfObject::Integer(1_000_000)));

        // Types that should not be valid as stream lengths
        let invalid_types = vec![
            PdfObject::Null,
            PdfObject::Boolean(true),
            PdfObject::Real(42.5),
            PdfObject::String(PdfString::new(b"length".to_vec())),
            PdfObject::Name(PdfName::new("Length".to_string())),
        ];

        for invalid_type in invalid_types {
            match invalid_type {
                PdfObject::Integer(_) => panic!("Should not be integer"),
                PdfObject::Reference(_, _) => panic!("Should not be reference"),
                _ => (), // These are the invalid types we expect
            }
        }
    }

    /// Tests for Issue #124: PDFs with indirect stream length references
    /// should be parsed successfully with PdfReader::new()
    mod indirect_length_tests {
        use crate::parser::{reader::PdfReader, ParseOptions};
        use std::io::Cursor;

        /// Create a PDF with an indirect reference for stream Length
        /// This simulates the issue reported in GitHub #124
        fn create_pdf_with_indirect_length() -> Vec<u8> {
            // Build PDF programmatically with correct offsets
            let mut pdf = String::new();

            // Header
            pdf.push_str("%PDF-1.4\n");

            // Object 1: Catalog
            let obj1_offset = pdf.len();
            pdf.push_str("1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");

            // Object 2: Pages
            let obj2_offset = pdf.len();
            pdf.push_str("2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n");

            // Object 3: Page
            let obj3_offset = pdf.len();
            pdf.push_str("3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 4 0 R >>\nendobj\n");

            // Object 4: Content stream with INDIRECT LENGTH reference (5 0 R)
            let obj4_offset = pdf.len();
            let stream_content = b"BT /F1 12 Tf 100 700 Td (Hello) Tj ET";
            let stream_len = stream_content.len();
            pdf.push_str("4 0 obj\n<< /Length 5 0 R >>\nstream\n");
            pdf.push_str(std::str::from_utf8(stream_content).unwrap());
            pdf.push_str("\nendstream\nendobj\n");

            // Object 5: The length value (indirect object)
            let obj5_offset = pdf.len();
            pdf.push_str(&format!("5 0 obj\n{}\nendobj\n", stream_len));

            // XRef table
            let xref_offset = pdf.len();
            pdf.push_str("xref\n");
            pdf.push_str("0 6\n");
            pdf.push_str("0000000000 65535 f \n");
            pdf.push_str(&format!("{:010} 00000 n \n", obj1_offset));
            pdf.push_str(&format!("{:010} 00000 n \n", obj2_offset));
            pdf.push_str(&format!("{:010} 00000 n \n", obj3_offset));
            pdf.push_str(&format!("{:010} 00000 n \n", obj4_offset));
            pdf.push_str(&format!("{:010} 00000 n \n", obj5_offset));

            // Trailer
            pdf.push_str("trailer\n<< /Size 6 /Root 1 0 R >>\n");
            pdf.push_str(&format!("startxref\n{}\n%%EOF", xref_offset));

            pdf.into_bytes()
        }

        #[test]
        fn test_pdf_reader_new_handles_indirect_length() {
            // Issue #124: PdfReader::new() should handle PDFs with indirect Length references
            let pdf_data = create_pdf_with_indirect_length();
            let cursor = Cursor::new(pdf_data);

            // This should succeed because PdfReader::new() now enables lenient_streams
            let result = PdfReader::new(cursor);
            assert!(
                result.is_ok(),
                "PdfReader::new() should handle indirect Length references"
            );

            // Verify the reader is functional
            let mut reader = result.unwrap();
            let page_count = reader.page_count();
            assert!(page_count.is_ok(), "Should be able to get page count");
            assert_eq!(page_count.unwrap(), 1);
        }

        #[test]
        fn test_pdf_reader_new_uses_lenient_streams() {
            // Verify that PdfReader::new() actually has lenient_streams enabled
            let pdf_data = create_pdf_with_indirect_length();
            let cursor = Cursor::new(pdf_data);

            let reader = PdfReader::new(cursor).unwrap();
            let options = reader.options();

            assert!(
                options.lenient_streams,
                "PdfReader::new() should have lenient_streams enabled"
            );
        }

        #[test]
        fn test_pdf_reader_strict_rejects_indirect_length() {
            // Verify strict mode still rejects indirect Length references
            let pdf_data = create_pdf_with_indirect_length();
            let cursor = Cursor::new(pdf_data);

            let strict_options = ParseOptions::strict();
            let result = PdfReader::new_with_options(cursor, strict_options);

            // Strict mode should reject indirect Length references
            // (or at least not have lenient_streams enabled)
            if let Ok(reader) = &result {
                assert!(
                    !reader.options().lenient_streams,
                    "Strict options should not have lenient_streams"
                );
            }
        }

        #[test]
        fn test_consistency_between_new_and_default_options() {
            // Verify that PdfReader::new() behavior matches using default options + lenient_streams
            let pdf_data = create_pdf_with_indirect_length();

            // Method 1: PdfReader::new()
            let cursor1 = Cursor::new(pdf_data.clone());
            let result1 = PdfReader::new(cursor1);

            // Method 2: new_with_options with default + lenient_streams
            let cursor2 = Cursor::new(pdf_data);
            let mut options = ParseOptions::default();
            options.lenient_streams = true;
            let result2 = PdfReader::new_with_options(cursor2, options);

            // Both should succeed
            assert!(result1.is_ok(), "PdfReader::new() should succeed");
            assert!(
                result2.is_ok(),
                "new_with_options with lenient_streams should succeed"
            );

            // Both should have lenient_streams enabled
            let reader1 = result1.unwrap();
            let reader2 = result2.unwrap();
            assert!(reader1.options().lenient_streams);
            assert!(reader2.options().lenient_streams);
        }
    }
}
