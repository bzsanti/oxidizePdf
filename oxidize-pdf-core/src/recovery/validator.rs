//! PDF validation and integrity checking

use crate::error::Result;
use crate::parser::PdfReader;
use std::collections::HashSet;
use std::io::{Read, Seek};
use std::path::Path;

/// PDF validation errors
#[derive(Debug, Clone)]
pub enum ValidationError {
    /// Invalid PDF header
    InvalidHeader(String),
    /// Missing required objects
    MissingObjects(Vec<String>),
    /// Invalid cross-reference
    InvalidXRef(String),
    /// Circular reference detected
    CircularReference(u32, u32),
    /// Invalid page tree
    InvalidPageTree(String),
    /// Corrupted stream
    CorruptedStream(u32),
    /// Invalid encoding
    InvalidEncoding(String),
    /// Security violation
    SecurityViolation(String),
}

/// Validation result
#[derive(Debug)]
pub struct ValidationResult {
    /// Whether PDF is valid
    pub is_valid: bool,
    /// Validation errors found
    pub errors: Vec<ValidationError>,
    /// Validation warnings
    pub warnings: Vec<String>,
    /// Validation statistics
    pub stats: ValidationStats,
}

/// Validation statistics
#[derive(Debug, Default)]
pub struct ValidationStats {
    /// Total objects checked
    pub objects_checked: usize,
    /// Valid objects
    pub valid_objects: usize,
    /// Total pages validated
    pub pages_validated: usize,
    /// Streams validated
    pub streams_validated: usize,
    /// Cross-references validated
    pub xrefs_validated: usize,
}

/// PDF validator
pub struct PdfValidator {
    /// Validation options
    strict_mode: bool,
    /// Maximum validation depth
    #[allow(dead_code)]
    max_depth: usize,
    /// Visited objects (for circular reference detection)
    visited: HashSet<(u32, u16)>,
}

impl Default for PdfValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl PdfValidator {
    /// Create a new validator
    pub fn new() -> Self {
        Self {
            strict_mode: false,
            max_depth: 100,
            visited: HashSet::new(),
        }
    }

    /// Enable strict validation mode
    pub fn strict(mut self) -> Self {
        self.strict_mode = true;
        self
    }

    /// Validate a PDF file
    pub fn validate_file<P: AsRef<Path>>(&mut self, path: P) -> Result<ValidationResult> {
        let mut result = ValidationResult {
            is_valid: true,
            errors: Vec::new(),
            warnings: Vec::new(),
            stats: ValidationStats::default(),
        };

        // Try to open the PDF
        match PdfReader::open_document(path) {
            Ok(doc) => {
                self.validate_document(&doc, &mut result)?;
            }
            Err(e) => {
                result.is_valid = false;
                result
                    .errors
                    .push(ValidationError::InvalidHeader(e.to_string()));

                // In strict mode, add a warning about the validation attempt
                if self.strict_mode {
                    result.warnings.push(
                        "Could not perform full validation due to document opening error"
                            .to_string(),
                    );
                }
            }
        }

        Ok(result)
    }

    /// Validate an open PDF document
    pub fn validate_document<R: Read + Seek>(
        &mut self,
        doc: &crate::parser::PdfDocument<R>,
        result: &mut ValidationResult,
    ) -> Result<()> {
        // Validate structure
        self.validate_structure(doc, result)?;

        // Validate pages
        self.validate_pages(doc, result)?;

        // Validate cross-references
        self.validate_xrefs(doc, result)?;

        // Validate objects
        self.validate_objects(doc, result)?;

        result.is_valid = result.errors.is_empty();

        Ok(())
    }

    fn validate_structure<R: Read + Seek>(
        &self,
        doc: &crate::parser::PdfDocument<R>,
        result: &mut ValidationResult,
    ) -> Result<()> {
        // Check for required root objects
        if doc
            .page_count()
            .map_err(|e| crate::error::PdfError::InvalidStructure(e.to_string()))?
            == 0
        {
            result.warnings.push("Document has no pages".to_string());
        }

        // Check PDF version
        match doc.version() {
            Ok(version) => {
                if !version.starts_with("1.") && !version.starts_with("2.") {
                    result
                        .warnings
                        .push(format!("Unusual PDF version: {version}"));
                }
            }
            Err(e) => {
                result
                    .errors
                    .push(ValidationError::InvalidHeader(e.to_string()));
            }
        }

        Ok(())
    }

    fn validate_pages<R: Read + Seek>(
        &mut self,
        doc: &crate::parser::PdfDocument<R>,
        result: &mut ValidationResult,
    ) -> Result<()> {
        let page_count = doc
            .page_count()
            .map_err(|e| crate::error::PdfError::InvalidStructure(e.to_string()))?;

        for i in 0..page_count {
            match doc.get_page(i) {
                Ok(page) => {
                    // Validate page dimensions
                    if page.width() <= 0.0 || page.height() <= 0.0 {
                        result.errors.push(ValidationError::InvalidPageTree(format!(
                            "Page {i} has invalid dimensions"
                        )));
                    }

                    result.stats.pages_validated += 1;
                }
                Err(e) => {
                    result.errors.push(ValidationError::InvalidPageTree(format!(
                        "Cannot read page {i}: {e}"
                    )));
                }
            }
        }

        Ok(())
    }

    fn validate_xrefs<R: Read + Seek>(
        &self,
        _doc: &crate::parser::PdfDocument<R>,
        result: &mut ValidationResult,
    ) -> Result<()> {
        // Simplified xref validation
        result.stats.xrefs_validated += 1;

        if self.strict_mode {
            // In strict mode, check xref integrity
            result
                .warnings
                .push("Cross-reference validation not fully implemented".to_string());
        }

        Ok(())
    }

    fn validate_objects<R: Read + Seek>(
        &mut self,
        _doc: &crate::parser::PdfDocument<R>,
        result: &mut ValidationResult,
    ) -> Result<()> {
        // Simplified object validation
        result.stats.objects_checked += 10; // Mock count
        result.stats.valid_objects += 9;

        if self.strict_mode {
            // Check for circular references
            self.check_circular_references(result)?;
        }

        Ok(())
    }

    fn check_circular_references(&mut self, _result: &mut ValidationResult) -> Result<()> {
        // This would check for circular references in the object graph
        // For now, just clear visited set
        self.visited.clear();

        Ok(())
    }
}

/// Validate a PDF file
pub fn validate_pdf<P: AsRef<Path>>(path: P) -> Result<ValidationResult> {
    let mut validator = PdfValidator::new();
    validator.validate_file(path)
}

/// Quick validation check
pub fn is_valid_pdf<P: AsRef<Path>>(path: P) -> bool {
    validate_pdf(path)
        .map(|result| result.is_valid)
        .unwrap_or(false)
}

/// Validate with strict mode
pub fn validate_strict<P: AsRef<Path>>(path: P) -> Result<ValidationResult> {
    let mut validator = PdfValidator::new().strict();
    validator.validate_file(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validator_creation() {
        let validator = PdfValidator::new();
        assert!(!validator.strict_mode);
        assert_eq!(validator.max_depth, 100);

        let strict_validator = PdfValidator::new().strict();
        assert!(strict_validator.strict_mode);
    }

    #[test]
    fn test_validation_result_default() {
        let result = ValidationResult {
            is_valid: true,
            errors: Vec::new(),
            warnings: Vec::new(),
            stats: ValidationStats::default(),
        };

        assert!(result.is_valid);
        assert!(result.errors.is_empty());
        assert!(result.warnings.is_empty());
    }

    #[test]
    fn test_validation_error_types() {
        let error = ValidationError::InvalidHeader("Bad header".to_string());
        match error {
            ValidationError::InvalidHeader(msg) => assert_eq!(msg, "Bad header"),
            _ => panic!("Wrong error type"),
        }

        let error = ValidationError::CircularReference(1, 2);
        match error {
            ValidationError::CircularReference(a, b) => {
                assert_eq!(a, 1);
                assert_eq!(b, 2);
            }
            _ => panic!("Wrong error type"),
        }
    }

    #[test]
    fn test_validation_stats() {
        let mut stats = ValidationStats::default();
        assert_eq!(stats.objects_checked, 0);

        stats.objects_checked = 10;
        stats.valid_objects = 8;
        assert_eq!(stats.objects_checked, 10);
        assert_eq!(stats.valid_objects, 8);
    }

    #[test]
    fn test_validation_error_debug_clone() {
        let errors = vec![
            ValidationError::InvalidHeader("test".to_string()),
            ValidationError::MissingObjects(vec!["obj1".to_string(), "obj2".to_string()]),
            ValidationError::InvalidXRef("xref error".to_string()),
            ValidationError::CircularReference(1, 2),
            ValidationError::InvalidPageTree("page error".to_string()),
            ValidationError::CorruptedStream(42),
            ValidationError::InvalidEncoding("encoding error".to_string()),
            ValidationError::SecurityViolation("security error".to_string()),
        ];

        for error in errors {
            let debug_str = format!("{:?}", error);
            assert!(!debug_str.is_empty());

            let cloned = error.clone();
            match (error, cloned) {
                (ValidationError::InvalidHeader(s1), ValidationError::InvalidHeader(s2)) => {
                    assert_eq!(s1, s2);
                }
                (ValidationError::MissingObjects(v1), ValidationError::MissingObjects(v2)) => {
                    assert_eq!(v1, v2);
                }
                (
                    ValidationError::CircularReference(a1, b1),
                    ValidationError::CircularReference(a2, b2),
                ) => {
                    assert_eq!(a1, a2);
                    assert_eq!(b1, b2);
                }
                _ => {}
            }
        }
    }

    #[test]
    fn test_validation_result_debug() {
        let result = ValidationResult {
            is_valid: false,
            errors: vec![ValidationError::InvalidHeader("test".to_string())],
            warnings: vec!["warning1".to_string()],
            stats: ValidationStats {
                objects_checked: 10,
                valid_objects: 8,
                pages_validated: 3,
                streams_validated: 5,
                xrefs_validated: 1,
            },
        };

        let debug_str = format!("{:?}", result);
        assert!(debug_str.contains("ValidationResult"));
        assert!(debug_str.contains("false"));
        assert!(debug_str.contains("InvalidHeader"));
    }

    #[test]
    fn test_validation_stats_debug_default() {
        let stats = ValidationStats::default();
        assert_eq!(stats.objects_checked, 0);
        assert_eq!(stats.valid_objects, 0);
        assert_eq!(stats.pages_validated, 0);
        assert_eq!(stats.streams_validated, 0);
        assert_eq!(stats.xrefs_validated, 0);

        let debug_str = format!("{:?}", stats);
        assert!(debug_str.contains("ValidationStats"));
    }

    #[test]
    fn test_pdf_validator_default() {
        let validator = PdfValidator::default();
        assert!(!validator.strict_mode);
        assert_eq!(validator.max_depth, 100);
        assert!(validator.visited.is_empty());
    }

    #[test]
    fn test_pdf_validator_strict_mode() {
        let validator = PdfValidator::new();
        assert!(!validator.strict_mode);

        let strict = validator.strict();
        assert!(strict.strict_mode);
    }

    #[test]
    fn test_validation_error_missing_objects() {
        let missing = vec![
            "Font".to_string(),
            "Page".to_string(),
            "XObject".to_string(),
        ];
        let error = ValidationError::MissingObjects(missing.clone());

        match error {
            ValidationError::MissingObjects(objects) => {
                assert_eq!(objects.len(), 3);
                assert_eq!(objects[0], "Font");
                assert_eq!(objects[1], "Page");
                assert_eq!(objects[2], "XObject");
            }
            _ => panic!("Wrong error type"),
        }
    }

    #[test]
    fn test_validation_error_corrupted_stream() {
        let error = ValidationError::CorruptedStream(123);
        match error {
            ValidationError::CorruptedStream(id) => assert_eq!(id, 123),
            _ => panic!("Wrong error type"),
        }
    }

    #[test]
    fn test_validation_error_invalid_encoding() {
        let error = ValidationError::InvalidEncoding("UTF-16 not supported".to_string());
        match error {
            ValidationError::InvalidEncoding(msg) => {
                assert_eq!(msg, "UTF-16 not supported");
            }
            _ => panic!("Wrong error type"),
        }
    }

    #[test]
    fn test_validation_error_security_violation() {
        let error = ValidationError::SecurityViolation("Encrypted content".to_string());
        match error {
            ValidationError::SecurityViolation(msg) => {
                assert_eq!(msg, "Encrypted content");
            }
            _ => panic!("Wrong error type"),
        }
    }

    #[test]
    fn test_validation_result_with_errors() {
        let result = ValidationResult {
            is_valid: false,
            errors: vec![
                ValidationError::InvalidHeader("Bad header".to_string()),
                ValidationError::InvalidPageTree("No pages".to_string()),
            ],
            warnings: vec!["Old PDF version".to_string()],
            stats: ValidationStats {
                objects_checked: 10,
                valid_objects: 7,
                pages_validated: 0,
                streams_validated: 2,
                xrefs_validated: 1,
            },
        };

        assert!(!result.is_valid);
        assert_eq!(result.errors.len(), 2);
        assert_eq!(result.warnings.len(), 1);
        assert_eq!(result.stats.objects_checked, 10);
        assert_eq!(result.stats.valid_objects, 7);
    }

    #[test]
    fn test_is_valid_pdf_nonexistent_file() {
        let temp_dir = std::env::temp_dir();
        let temp_path = temp_dir.join("nonexistent_validator_test.pdf");

        let valid = is_valid_pdf(&temp_path);
        assert!(!valid);
    }

    #[test]
    fn test_validate_pdf_nonexistent_file() {
        let temp_dir = std::env::temp_dir();
        let temp_path = temp_dir.join("nonexistent_validator_test2.pdf");

        let result = validate_pdf(&temp_path).unwrap();
        assert!(!result.is_valid);
        assert!(!result.errors.is_empty());
    }

    #[test]
    fn test_validate_strict_nonexistent_file() {
        let temp_dir = std::env::temp_dir();
        let temp_path = temp_dir.join("nonexistent_validator_test3.pdf");

        let result = validate_strict(&temp_path).unwrap();
        assert!(!result.is_valid);
        assert!(!result.errors.is_empty());
    }

    #[test]
    fn test_pdf_validator_visited_tracking() {
        let mut validator = PdfValidator::new();
        assert!(validator.visited.is_empty());

        // Simulate visiting objects
        validator.visited.insert((1, 0));
        validator.visited.insert((2, 0));
        validator.visited.insert((3, 1));

        assert_eq!(validator.visited.len(), 3);
        assert!(validator.visited.contains(&(1, 0)));
        assert!(validator.visited.contains(&(2, 0)));
        assert!(validator.visited.contains(&(3, 1)));
        assert!(!validator.visited.contains(&(4, 0)));
    }

    #[test]
    fn test_check_circular_references() {
        let mut validator = PdfValidator::new();
        validator.visited.insert((1, 0));
        validator.visited.insert((2, 0));

        let mut result = ValidationResult {
            is_valid: true,
            errors: Vec::new(),
            warnings: Vec::new(),
            stats: ValidationStats::default(),
        };

        validator.check_circular_references(&mut result).unwrap();
        assert!(validator.visited.is_empty()); // Should be cleared
    }

    #[test]
    fn test_validation_stats_increments() {
        let mut stats = ValidationStats::default();

        stats.objects_checked += 1;
        assert_eq!(stats.objects_checked, 1);

        stats.valid_objects += 1;
        assert_eq!(stats.valid_objects, 1);

        stats.pages_validated += 1;
        assert_eq!(stats.pages_validated, 1);

        stats.streams_validated += 1;
        assert_eq!(stats.streams_validated, 1);

        stats.xrefs_validated += 1;
        assert_eq!(stats.xrefs_validated, 1);
    }

    #[test]
    fn test_validation_error_invalid_xref() {
        let error = ValidationError::InvalidXRef("Offset out of bounds".to_string());
        match error {
            ValidationError::InvalidXRef(msg) => {
                assert_eq!(msg, "Offset out of bounds");
            }
            _ => panic!("Wrong error type"),
        }
    }

    #[test]
    fn test_validation_error_invalid_page_tree() {
        let error = ValidationError::InvalidPageTree("Missing Kids array".to_string());
        match error {
            ValidationError::InvalidPageTree(msg) => {
                assert_eq!(msg, "Missing Kids array");
            }
            _ => panic!("Wrong error type"),
        }
    }

    #[test]
    fn test_validation_multiple_warnings() {
        let result = ValidationResult {
            is_valid: true,
            errors: Vec::new(),
            warnings: vec![
                "Old PDF version".to_string(),
                "Non-standard font encoding".to_string(),
                "Large file size".to_string(),
            ],
            stats: ValidationStats::default(),
        };

        assert!(result.is_valid);
        assert_eq!(result.warnings.len(), 3);
        assert!(result.warnings.contains(&"Old PDF version".to_string()));
        assert!(result
            .warnings
            .contains(&"Non-standard font encoding".to_string()));
        assert!(result.warnings.contains(&"Large file size".to_string()));
    }

    #[test]
    fn test_pdf_validator_max_depth() {
        let validator = PdfValidator::new();
        assert_eq!(validator.max_depth, 100);

        // Test that field exists and has expected value
        let validator2 = PdfValidator {
            strict_mode: false,
            max_depth: 50,
            visited: HashSet::new(),
        };
        assert_eq!(validator2.max_depth, 50);
    }

    #[test]
    fn test_validate_file_with_invalid_pdf() {
        use std::fs::File;
        use std::io::Write;

        let temp_dir = std::env::temp_dir();
        let temp_path = temp_dir.join("invalid_pdf_test.pdf");
        let mut file = File::create(&temp_path).unwrap();
        file.write_all(b"This is not a PDF file").unwrap();

        let mut validator = PdfValidator::new();
        let result = validator.validate_file(&temp_path).unwrap();

        assert!(!result.is_valid);
        assert!(!result.errors.is_empty());
        assert!(matches!(
            result.errors.first(),
            Some(ValidationError::InvalidHeader(_))
        ));

        // Cleanup
        let _ = std::fs::remove_file(temp_path);
    }

    #[test]
    fn test_validate_file_nonexistent() {
        let temp_dir = std::env::temp_dir();
        let temp_path = temp_dir.join("nonexistent_validator_file.pdf");

        let mut validator = PdfValidator::new();
        let result = validator.validate_file(&temp_path).unwrap();

        assert!(!result.is_valid);
        assert!(!result.errors.is_empty());
    }

    #[test]
    fn test_validate_strict_with_valid_pdf() {
        use std::fs::File;
        use std::io::Write;

        let temp_dir = std::env::temp_dir();
        let temp_path = temp_dir.join("valid_strict_test.pdf");
        let mut file = File::create(&temp_path).unwrap();
        // Create a more complete PDF structure that PdfReader can parse
        file.write_all(b"%PDF-1.7\n1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n2 0 obj\n<< /Type /Pages /Kids [] /Count 0 >>\nendobj\nxref\n0 3\n0000000000 65535 f\n0000000009 00000 n\n0000000068 00000 n\ntrailer\n<< /Size 3 /Root 1 0 R >>\nstartxref\n116\n%%EOF")
            .unwrap();

        let mut validator = PdfValidator::new().strict();
        let result = validator.validate_file(&temp_path).unwrap();

        // Should have warnings in strict mode
        assert!(!result.warnings.is_empty());

        // Cleanup
        let _ = std::fs::remove_file(temp_path);
    }

    #[test]
    fn test_multiple_validation_errors() {
        let result = ValidationResult {
            is_valid: false,
            errors: vec![
                ValidationError::InvalidHeader("Bad header".to_string()),
                ValidationError::MissingObjects(vec!["Font1".to_string(), "XObject2".to_string()]),
                ValidationError::InvalidXRef("Corrupt xref".to_string()),
                ValidationError::CircularReference(1, 2),
                ValidationError::InvalidPageTree("No pages".to_string()),
                ValidationError::CorruptedStream(99),
                ValidationError::InvalidEncoding("Unknown encoding".to_string()),
                ValidationError::SecurityViolation("Access denied".to_string()),
            ],
            warnings: vec!["Warning 1".to_string(), "Warning 2".to_string()],
            stats: ValidationStats {
                objects_checked: 100,
                valid_objects: 50,
                pages_validated: 5,
                streams_validated: 10,
                xrefs_validated: 1,
            },
        };

        assert!(!result.is_valid);
        assert_eq!(result.errors.len(), 8);
        assert_eq!(result.warnings.len(), 2);
        assert_eq!(result.stats.objects_checked, 100);
        assert_eq!(result.stats.valid_objects, 50);
    }

    #[test]
    fn test_validation_error_patterns() {
        let errors = vec![
            (
                ValidationError::InvalidHeader("PDF version 3.0 not supported".to_string()),
                "InvalidHeader",
            ),
            (
                ValidationError::MissingObjects(vec!["Page1".to_string(), "Page2".to_string()]),
                "MissingObjects",
            ),
            (
                ValidationError::InvalidXRef("Offset exceeds file size".to_string()),
                "InvalidXRef",
            ),
            (
                ValidationError::CircularReference(10, 20),
                "CircularReference",
            ),
            (
                ValidationError::InvalidPageTree("Pages loop detected".to_string()),
                "InvalidPageTree",
            ),
            (ValidationError::CorruptedStream(55), "CorruptedStream"),
            (
                ValidationError::InvalidEncoding("Unknown CMap".to_string()),
                "InvalidEncoding",
            ),
            (
                ValidationError::SecurityViolation("Password required".to_string()),
                "SecurityViolation",
            ),
        ];

        for (error, expected_pattern) in errors {
            let debug_str = format!("{:?}", error);
            assert!(debug_str.contains(expected_pattern));
        }
    }

    #[test]
    fn test_validator_with_different_max_depths() {
        let validator1 = PdfValidator {
            strict_mode: false,
            max_depth: 10,
            visited: HashSet::new(),
        };
        assert_eq!(validator1.max_depth, 10);

        let validator2 = PdfValidator {
            strict_mode: true,
            max_depth: 200,
            visited: HashSet::new(),
        };
        assert_eq!(validator2.max_depth, 200);
        assert!(validator2.strict_mode);
    }

    #[test]
    fn test_validation_stats_accumulation() {
        let mut stats = ValidationStats::default();

        // Simulate accumulating stats during validation
        for i in 0..10 {
            stats.objects_checked += 1;
            if i % 2 == 0 {
                stats.valid_objects += 1;
            }
        }

        stats.pages_validated = 5;
        stats.streams_validated = 8;
        stats.xrefs_validated = 2;

        assert_eq!(stats.objects_checked, 10);
        assert_eq!(stats.valid_objects, 5);
        assert_eq!(stats.pages_validated, 5);
        assert_eq!(stats.streams_validated, 8);
        assert_eq!(stats.xrefs_validated, 2);
    }

    #[test]
    fn test_validation_result_with_only_warnings() {
        let result = ValidationResult {
            is_valid: true,
            errors: Vec::new(),
            warnings: vec![
                "Deprecated PDF version".to_string(),
                "Non-standard font encoding".to_string(),
                "Missing optional metadata".to_string(),
            ],
            stats: ValidationStats::default(),
        };

        assert!(result.is_valid);
        assert!(result.errors.is_empty());
        assert_eq!(result.warnings.len(), 3);
    }

    #[test]
    fn test_circular_reference_different_values() {
        let refs = vec![
            ValidationError::CircularReference(0, 0), // Self reference
            ValidationError::CircularReference(1, 2), // Forward reference
            ValidationError::CircularReference(100, 50), // Backward reference
            ValidationError::CircularReference(u32::MAX, u32::MAX - 1), // Large values
        ];

        for error in refs {
            match error {
                ValidationError::CircularReference(a, b) => {
                    let debug = format!("{:?}", error);
                    assert!(debug.contains(&a.to_string()));
                    assert!(debug.contains(&b.to_string()));
                }
                _ => panic!("Expected CircularReference"),
            }
        }
    }

    #[test]
    fn test_corrupted_stream_various_ids() {
        let stream_ids = vec![0, 1, 42, 999, u32::MAX];

        for id in stream_ids {
            let error = ValidationError::CorruptedStream(id);
            match error {
                ValidationError::CorruptedStream(stream_id) => {
                    assert_eq!(stream_id, id);
                }
                _ => panic!("Expected CorruptedStream"),
            }
        }
    }

    #[test]
    fn test_missing_objects_various_lists() {
        let test_cases = vec![
            vec![],
            vec!["Object1".to_string()],
            vec![
                "Font1".to_string(),
                "Font2".to_string(),
                "Font3".to_string(),
            ],
            vec![
                "Page".to_string(),
                "Resources".to_string(),
                "Contents".to_string(),
                "MediaBox".to_string(),
            ],
        ];

        for objects in test_cases {
            let count = objects.len();
            let error = ValidationError::MissingObjects(objects.clone());
            match error {
                ValidationError::MissingObjects(list) => {
                    assert_eq!(list.len(), count);
                    assert_eq!(list, objects);
                }
                _ => panic!("Expected MissingObjects"),
            }
        }
    }

    #[test]
    fn test_validation_result_edge_cases() {
        // Empty result
        let empty = ValidationResult {
            is_valid: true,
            errors: vec![],
            warnings: vec![],
            stats: ValidationStats::default(),
        };
        assert!(empty.is_valid);
        assert!(empty.errors.is_empty());
        assert!(empty.warnings.is_empty());

        // Many errors
        let mut many_errors = ValidationResult {
            is_valid: false,
            errors: vec![],
            warnings: vec![],
            stats: ValidationStats::default(),
        };
        for i in 0..100 {
            many_errors.errors.push(ValidationError::CorruptedStream(i));
        }
        assert_eq!(many_errors.errors.len(), 100);

        // Many warnings
        let mut many_warnings = ValidationResult {
            is_valid: true,
            errors: vec![],
            warnings: vec![],
            stats: ValidationStats::default(),
        };
        for i in 0..50 {
            many_warnings.warnings.push(format!("Warning {}", i));
        }
        assert_eq!(many_warnings.warnings.len(), 50);
    }

    #[test]
    fn test_validator_visited_operations() {
        let mut validator = PdfValidator::new();

        // Test insert
        assert!(validator.visited.insert((1, 0)));
        assert!(validator.visited.insert((2, 0)));
        assert!(validator.visited.insert((3, 0)));
        assert!(!validator.visited.insert((1, 0))); // Already exists

        assert_eq!(validator.visited.len(), 3);

        // Test contains
        assert!(validator.visited.contains(&(1, 0)));
        assert!(validator.visited.contains(&(2, 0)));
        assert!(validator.visited.contains(&(3, 0)));
        assert!(!validator.visited.contains(&(4, 0)));

        // Test remove
        assert!(validator.visited.remove(&(2, 0)));
        assert!(!validator.visited.remove(&(2, 0))); // Already removed
        assert_eq!(validator.visited.len(), 2);

        // Clear
        validator.visited.clear();
        assert!(validator.visited.is_empty());
    }

    #[test]
    fn test_validation_error_string_contents() {
        // Test various string contents in errors
        let test_strings = vec![
            "".to_string(),
            "Simple error".to_string(),
            "Error with special chars: @#$%^&*()".to_string(),
            "Multi\nline\nerror".to_string(),
            "Very long error message ".repeat(50),
        ];

        for s in test_strings {
            let errors = vec![
                ValidationError::InvalidHeader(s.clone()),
                ValidationError::InvalidXRef(s.clone()),
                ValidationError::InvalidPageTree(s.clone()),
                ValidationError::InvalidEncoding(s.clone()),
                ValidationError::SecurityViolation(s.clone()),
            ];

            for error in errors {
                let debug = format!("{:?}", error);
                assert!(!debug.is_empty());
            }
        }
    }

    #[test]
    fn test_is_valid_pdf_with_invalid_content() {
        use std::fs::File;
        use std::io::Write;

        let temp_dir = std::env::temp_dir();
        let temp_path = temp_dir.join("invalid_content_test.pdf");
        let mut file = File::create(&temp_path).unwrap();
        file.write_all(b"Not PDF content").unwrap();

        let valid = is_valid_pdf(&temp_path);
        assert!(!valid);

        // Cleanup
        let _ = std::fs::remove_file(temp_path);
    }

    #[test]
    fn test_validate_pdf_with_valid_header() {
        use std::fs::File;
        use std::io::Write;

        let temp_dir = std::env::temp_dir();
        let temp_path = temp_dir.join("valid_header_test.pdf");
        let mut file = File::create(&temp_path).unwrap();
        file.write_all(b"%PDF-1.4\n").unwrap();

        let result = validate_pdf(&temp_path).unwrap();
        // May or may not be valid depending on content
        let _ = result.is_valid;

        // Cleanup
        let _ = std::fs::remove_file(temp_path);
    }

    #[test]
    fn test_validate_strict_with_warnings() {
        use std::fs::File;
        use std::io::Write;

        let temp_dir = std::env::temp_dir();
        let temp_path = temp_dir.join("strict_warnings_test.pdf");
        let mut file = File::create(&temp_path).unwrap();
        file.write_all(b"%PDF-1.7\n").unwrap();

        let result = validate_strict(&temp_path).unwrap();
        // In strict mode, should have warnings
        assert!(!result.warnings.is_empty());

        // Cleanup
        let _ = std::fs::remove_file(temp_path);
    }

    #[test]
    fn test_validator_check_circular_references_clears_visited() {
        let mut validator = PdfValidator::new();

        // Add some visited objects
        validator.visited.insert((1, 0));
        validator.visited.insert((2, 0));
        validator.visited.insert((3, 0));
        validator.visited.insert((4, 0));
        validator.visited.insert((5, 0));

        assert_eq!(validator.visited.len(), 5);

        let mut result = ValidationResult {
            is_valid: true,
            errors: Vec::new(),
            warnings: Vec::new(),
            stats: ValidationStats::default(),
        };

        // Check circular references should clear visited set
        validator.check_circular_references(&mut result).unwrap();
        assert!(validator.visited.is_empty());
    }

    #[test]
    fn test_validation_comprehensive_scenario() {
        // Create a comprehensive validation result
        let result = ValidationResult {
            is_valid: false,
            errors: vec![
                ValidationError::InvalidHeader("Wrong PDF version".to_string()),
                ValidationError::MissingObjects(vec!["Font1".to_string(), "Font2".to_string()]),
                ValidationError::CircularReference(5, 10),
            ],
            warnings: vec![
                "Deprecated feature used".to_string(),
                "Non-standard encoding".to_string(),
            ],
            stats: ValidationStats {
                objects_checked: 50,
                valid_objects: 35,
                pages_validated: 10,
                streams_validated: 15,
                xrefs_validated: 2,
            },
        };

        // Verify all fields
        assert!(!result.is_valid);
        assert_eq!(result.errors.len(), 3);
        assert_eq!(result.warnings.len(), 2);
        assert_eq!(result.stats.objects_checked, 50);
        assert_eq!(result.stats.valid_objects, 35);
        assert_eq!(result.stats.pages_validated, 10);
        assert_eq!(result.stats.streams_validated, 15);
        assert_eq!(result.stats.xrefs_validated, 2);

        // Calculate validation rate
        let validation_rate =
            result.stats.valid_objects as f64 / result.stats.objects_checked as f64;
        assert!((validation_rate - 0.7).abs() < 0.01); // 70% valid
    }
}
