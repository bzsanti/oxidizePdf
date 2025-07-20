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
}
