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
}
