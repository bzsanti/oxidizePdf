//! PDF/A Validation Engine
//!
//! This module provides the core validation logic for PDF/A compliance.
//! It checks PDF documents against the requirements of PDF/A standards.

use super::error::{PdfAError, ValidationError};
use super::types::{PdfALevel, ValidationResult, ValidationWarning};
use super::xmp::XmpMetadata;
use crate::parser::PdfReader;
use std::io::{Read, Seek};

/// Extracted catalog data for validation
struct CatalogData {
    metadata_ref: Option<(u32, u16)>,
    names_ref: Option<(u32, u16)>,
    names_inline: Option<crate::parser::objects::PdfDictionary>,
    open_action_ref: Option<(u32, u16)>,
    open_action_inline: Option<crate::parser::objects::PdfDictionary>,
    aa_ref: Option<(u32, u16)>,
    aa_inline: Option<crate::parser::objects::PdfDictionary>,
}

/// PDF/A Validator
///
/// Validates PDF documents against PDF/A standards (ISO 19005).
///
/// # Example
///
/// ```rust,ignore
/// use oxidize_pdf::parser::PdfReader;
/// use oxidize_pdf::pdfa::{PdfAValidator, PdfALevel};
///
/// let mut reader = PdfReader::open("document.pdf")?;
/// let validator = PdfAValidator::new(PdfALevel::A1b);
/// let result = validator.validate(&mut reader)?;
///
/// if result.is_valid() {
///     println!("Document is PDF/A-1b compliant!");
/// } else {
///     for error in result.errors() {
///         println!("Violation: {}", error);
///     }
/// }
/// ```
#[derive(Debug, Clone)]
pub struct PdfAValidator {
    /// Target PDF/A level for validation
    level: PdfALevel,
    /// Whether to collect all errors or stop at first
    collect_all_errors: bool,
}

impl PdfAValidator {
    /// Create a new validator for the specified PDF/A level
    pub fn new(level: PdfALevel) -> Self {
        Self {
            level,
            collect_all_errors: true,
        }
    }

    /// Set whether to collect all errors or stop at first error
    pub fn collect_all_errors(mut self, collect: bool) -> Self {
        self.collect_all_errors = collect;
        self
    }

    /// Get the target PDF/A level
    pub fn level(&self) -> PdfALevel {
        self.level
    }

    /// Validate a PDF document against the configured PDF/A level
    ///
    /// Returns a `ValidationResult` containing all errors and warnings found.
    pub fn validate<R: Read + Seek>(
        &self,
        reader: &mut PdfReader<R>,
    ) -> Result<ValidationResult, PdfAError> {
        let mut errors = Vec::new();
        let mut warnings = Vec::new();

        // Check encryption (forbidden in all PDF/A levels)
        self.check_encryption(reader, &mut errors);

        // Check PDF version compatibility
        self.check_pdf_version(reader, &mut errors)?;

        // Extract catalog data we need for validation before doing further operations
        let catalog_data = self.extract_catalog_data(reader)?;

        // Check XMP metadata (required in all PDF/A levels)
        self.check_metadata_from_data(reader, &catalog_data, &mut errors, &mut warnings)?;

        // Check for JavaScript (forbidden in all PDF/A levels)
        self.check_javascript_from_data(reader, &catalog_data, &mut errors)?;

        // Check external references (forbidden in all PDF/A levels)
        self.check_external_references_from_data(reader, &catalog_data, &mut errors)?;

        // Check transparency (forbidden in PDF/A-1, limited in PDF/A-2+)
        // Note: Full implementation would traverse page tree
        if !self.level.allows_transparency() {
            // Placeholder for transparency check
        }

        // Check compression (LZW forbidden in PDF/A-1)
        // Note: Full implementation would scan all streams
        if !self.level.allows_lzw() {
            // Placeholder for LZW check
        }

        Ok(ValidationResult::with_errors_and_warnings(
            self.level, errors, warnings,
        ))
    }

    /// Check if PDF is encrypted (encryption is forbidden in PDF/A)
    fn check_encryption<R: Read + Seek>(
        &self,
        reader: &PdfReader<R>,
        errors: &mut Vec<ValidationError>,
    ) {
        if reader.is_encrypted() {
            errors.push(ValidationError::EncryptionForbidden);
        }
    }

    /// Check PDF version compatibility with the target PDF/A level
    fn check_pdf_version<R: Read + Seek>(
        &self,
        reader: &PdfReader<R>,
        errors: &mut Vec<ValidationError>,
    ) -> Result<(), PdfAError> {
        let version = reader.version();
        let version_str = version.to_string();

        let required = self.level.required_pdf_version();

        // Parse versions for comparison
        let actual_parts: Vec<u8> = version_str
            .split('.')
            .filter_map(|s| s.parse().ok())
            .collect();

        // Get major and minor versions
        let (actual_major, actual_minor) = (
            actual_parts.first().copied().unwrap_or(1),
            actual_parts.get(1).copied().unwrap_or(0),
        );

        // For PDF/A-1, PDF version must be exactly 1.4
        // For PDF/A-2 and PDF/A-3, PDF version must be 1.7 or compatible
        let is_compatible = match self.level.part() {
            1 => actual_major == 1 && actual_minor == 4,
            2 | 3 => actual_major == 1 && actual_minor >= 4 && actual_minor <= 7,
            _ => false,
        };

        if !is_compatible {
            errors.push(ValidationError::IncompatiblePdfVersion {
                actual: version_str,
                required: required.to_string(),
            });
        }

        Ok(())
    }

    /// Extract data from catalog that we need for validation
    fn extract_catalog_data<R: Read + Seek>(
        &self,
        reader: &mut PdfReader<R>,
    ) -> Result<CatalogData, PdfAError> {
        let catalog = reader
            .catalog()
            .map_err(|e| PdfAError::ParseError(e.to_string()))?;

        let metadata_ref = catalog
            .get("Metadata")
            .and_then(|obj| obj.as_reference())
            .map(|(n, g)| (n, g));

        let names_ref = catalog
            .get("Names")
            .and_then(|obj| obj.as_reference())
            .map(|(n, g)| (n, g));

        let names_inline = catalog.get("Names").and_then(|obj| obj.as_dict()).cloned();

        let open_action_ref = catalog
            .get("OpenAction")
            .and_then(|obj| obj.as_reference())
            .map(|(n, g)| (n, g));

        let open_action_inline = catalog
            .get("OpenAction")
            .and_then(|obj| obj.as_dict())
            .cloned();

        let aa_ref = catalog
            .get("AA")
            .and_then(|obj| obj.as_reference())
            .map(|(n, g)| (n, g));

        let aa_inline = catalog.get("AA").and_then(|obj| obj.as_dict()).cloned();

        Ok(CatalogData {
            metadata_ref,
            names_ref,
            names_inline,
            open_action_ref,
            open_action_inline,
            aa_ref,
            aa_inline,
        })
    }

    /// Check for XMP metadata using extracted catalog data
    fn check_metadata_from_data<R: Read + Seek>(
        &self,
        reader: &mut PdfReader<R>,
        catalog_data: &CatalogData,
        errors: &mut Vec<ValidationError>,
        _warnings: &mut Vec<ValidationWarning>,
    ) -> Result<(), PdfAError> {
        // Check for Metadata stream
        let metadata_ref = match catalog_data.metadata_ref {
            Some(r) => r,
            None => {
                errors.push(ValidationError::XmpMetadataMissing);
                return Ok(());
            }
        };

        // Resolve the reference
        let obj = reader
            .get_object(metadata_ref.0, metadata_ref.1)
            .map_err(|e| PdfAError::ParseError(e.to_string()))?;

        // Check if it's a stream
        let stream = match obj.as_stream() {
            Some(s) => s,
            None => {
                errors.push(ValidationError::XmpMetadataMissing);
                return Ok(());
            }
        };

        // Parse the XMP metadata
        let xmp_data = String::from_utf8_lossy(&stream.data);
        let xmp = match XmpMetadata::parse(&xmp_data) {
            Ok(x) => x,
            Err(_) => {
                errors.push(ValidationError::XmpMetadataMissing);
                return Ok(());
            }
        };

        // Check for PDF/A identifier
        match &xmp.pdfa_id {
            None => {
                errors.push(ValidationError::XmpMissingPdfAIdentifier);
            }
            Some(pdfa_id) => {
                // Validate the PDF/A identifier matches our target level
                let expected_part = self.level.part();
                let expected_conformance = self.level.conformance();

                if pdfa_id.part != expected_part {
                    errors.push(ValidationError::XmpInvalidPdfAIdentifier {
                        details: format!(
                            "Part mismatch: expected {}, found {}",
                            expected_part, pdfa_id.part
                        ),
                    });
                } else if pdfa_id.conformance != expected_conformance {
                    errors.push(ValidationError::XmpInvalidPdfAIdentifier {
                        details: format!(
                            "Conformance mismatch: expected {:?}, found {:?}",
                            expected_conformance, pdfa_id.conformance
                        ),
                    });
                }
            }
        }

        Ok(())
    }

    /// Check for JavaScript using extracted catalog data
    fn check_javascript_from_data<R: Read + Seek>(
        &self,
        reader: &mut PdfReader<R>,
        catalog_data: &CatalogData,
        errors: &mut Vec<ValidationError>,
    ) -> Result<(), PdfAError> {
        // Check Names dictionary for JavaScript
        if let Some((obj_num, gen_num)) = catalog_data.names_ref {
            let names_obj = reader
                .get_object(obj_num, gen_num)
                .map_err(|e| PdfAError::ParseError(e.to_string()))?;

            if let Some(names_dict) = names_obj.as_dict() {
                if names_dict.get("JavaScript").is_some() {
                    errors.push(ValidationError::JavaScriptForbidden {
                        location: "Names/JavaScript".to_string(),
                    });
                }
            }
        } else if let Some(ref names_dict) = catalog_data.names_inline {
            if names_dict.get("JavaScript").is_some() {
                errors.push(ValidationError::JavaScriptForbidden {
                    location: "Names/JavaScript".to_string(),
                });
            }
        }

        // Check OpenAction for JavaScript
        if let Some((obj_num, gen_num)) = catalog_data.open_action_ref {
            let action_obj = reader
                .get_object(obj_num, gen_num)
                .map_err(|e| PdfAError::ParseError(e.to_string()))?;

            if let Some(action_dict) = action_obj.as_dict() {
                if self.is_javascript_action(action_dict) {
                    errors.push(ValidationError::JavaScriptForbidden {
                        location: "OpenAction".to_string(),
                    });
                }
            }
        } else if let Some(ref action_dict) = catalog_data.open_action_inline {
            if self.is_javascript_action(action_dict) {
                errors.push(ValidationError::JavaScriptForbidden {
                    location: "OpenAction".to_string(),
                });
            }
        }

        // Check AA (Additional Actions) dictionary
        if let Some((obj_num, gen_num)) = catalog_data.aa_ref {
            let aa_obj = reader
                .get_object(obj_num, gen_num)
                .map_err(|e| PdfAError::ParseError(e.to_string()))?;

            if let Some(aa_dict) = aa_obj.as_dict().cloned() {
                if self.check_aa_dict_for_javascript(reader, &aa_dict)? {
                    errors.push(ValidationError::JavaScriptForbidden {
                        location: "Catalog/AA".to_string(),
                    });
                }
            }
        } else if let Some(ref aa_dict) = catalog_data.aa_inline {
            if self.check_aa_dict_for_javascript(reader, aa_dict)? {
                errors.push(ValidationError::JavaScriptForbidden {
                    location: "Catalog/AA".to_string(),
                });
            }
        }

        Ok(())
    }

    /// Check if a dictionary is a JavaScript action
    fn is_javascript_action(&self, dict: &crate::parser::objects::PdfDictionary) -> bool {
        if let Some(action_type) = dict.get("S") {
            if let Some(name) = action_type.as_name() {
                return name.0 == "JavaScript";
            }
        }
        false
    }

    /// Check AA dictionary for JavaScript actions
    fn check_aa_dict_for_javascript<R: Read + Seek>(
        &self,
        reader: &mut PdfReader<R>,
        aa_dict: &crate::parser::objects::PdfDictionary,
    ) -> Result<bool, PdfAError> {
        // Check each action in the AA dictionary
        for (_key, value) in aa_dict.0.iter() {
            let action_dict = if let Some((obj_num, gen_num)) = value.as_reference() {
                let obj = reader
                    .get_object(obj_num, gen_num)
                    .map_err(|e| PdfAError::ParseError(e.to_string()))?;
                obj.as_dict().cloned()
            } else {
                value.as_dict().cloned()
            };

            if let Some(dict) = action_dict {
                if self.is_javascript_action(&dict) {
                    return Ok(true);
                }
            }
        }

        Ok(false)
    }

    /// Check external references using extracted catalog data
    fn check_external_references_from_data<R: Read + Seek>(
        &self,
        reader: &mut PdfReader<R>,
        catalog_data: &CatalogData,
        errors: &mut Vec<ValidationError>,
    ) -> Result<(), PdfAError> {
        // Check OpenAction for remote GoTo
        if let Some((obj_num, gen_num)) = catalog_data.open_action_ref {
            let action_obj = reader
                .get_object(obj_num, gen_num)
                .map_err(|e| PdfAError::ParseError(e.to_string()))?;

            if let Some(action_dict) = action_obj.as_dict() {
                self.check_for_external_action(action_dict, errors);
            }
        } else if let Some(ref action_dict) = catalog_data.open_action_inline {
            self.check_for_external_action(action_dict, errors);
        }

        Ok(())
    }

    /// Check if action is an external reference
    fn check_for_external_action(
        &self,
        dict: &crate::parser::objects::PdfDictionary,
        errors: &mut Vec<ValidationError>,
    ) {
        if let Some(action_type) = dict.get("S") {
            if let Some(name) = action_type.as_name() {
                if name.0 == "GoToR" || name.0 == "GoToE" || name.0 == "Launch" {
                    errors.push(ValidationError::ExternalReferenceForbidden {
                        reference_type: name.0.clone(),
                    });
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validator_new() {
        let validator = PdfAValidator::new(PdfALevel::A1b);
        assert_eq!(validator.level(), PdfALevel::A1b);
    }

    #[test]
    fn test_validator_level_a2b() {
        let validator = PdfAValidator::new(PdfALevel::A2b);
        assert_eq!(validator.level(), PdfALevel::A2b);
    }

    #[test]
    fn test_validator_level_a3b() {
        let validator = PdfAValidator::new(PdfALevel::A3b);
        assert_eq!(validator.level(), PdfALevel::A3b);
    }

    #[test]
    fn test_validator_collect_all_errors() {
        let validator = PdfAValidator::new(PdfALevel::A1b).collect_all_errors(false);
        assert!(!validator.collect_all_errors);
    }

    #[test]
    fn test_validator_clone() {
        let validator = PdfAValidator::new(PdfALevel::A2u);
        let cloned = validator.clone();
        assert_eq!(cloned.level(), PdfALevel::A2u);
    }

    #[test]
    fn test_validator_debug() {
        let validator = PdfAValidator::new(PdfALevel::A1a);
        let debug_str = format!("{:?}", validator);
        assert!(debug_str.contains("PdfAValidator"));
        assert!(debug_str.contains("A1a"));
    }

    #[test]
    fn test_is_javascript_action_true() {
        use crate::parser::objects::{PdfDictionary, PdfName, PdfObject};

        let validator = PdfAValidator::new(PdfALevel::A1b);
        let mut dict = PdfDictionary::new();
        dict.insert(
            "S".to_string(),
            PdfObject::Name(PdfName("JavaScript".to_string())),
        );

        assert!(validator.is_javascript_action(&dict));
    }

    #[test]
    fn test_is_javascript_action_false() {
        use crate::parser::objects::{PdfDictionary, PdfName, PdfObject};

        let validator = PdfAValidator::new(PdfALevel::A1b);
        let mut dict = PdfDictionary::new();
        dict.insert(
            "S".to_string(),
            PdfObject::Name(PdfName("GoTo".to_string())),
        );

        assert!(!validator.is_javascript_action(&dict));
    }

    #[test]
    fn test_is_javascript_action_no_s_key() {
        use crate::parser::objects::PdfDictionary;

        let validator = PdfAValidator::new(PdfALevel::A1b);
        let dict = PdfDictionary::new();

        assert!(!validator.is_javascript_action(&dict));
    }

    #[test]
    fn test_check_for_external_action_gotor() {
        use crate::parser::objects::{PdfDictionary, PdfName, PdfObject};

        let validator = PdfAValidator::new(PdfALevel::A1b);
        let mut dict = PdfDictionary::new();
        dict.insert(
            "S".to_string(),
            PdfObject::Name(PdfName("GoToR".to_string())),
        );

        let mut errors = Vec::new();
        validator.check_for_external_action(&dict, &mut errors);

        assert_eq!(errors.len(), 1);
        assert!(matches!(
            errors[0],
            ValidationError::ExternalReferenceForbidden { .. }
        ));
    }

    #[test]
    fn test_check_for_external_action_launch() {
        use crate::parser::objects::{PdfDictionary, PdfName, PdfObject};

        let validator = PdfAValidator::new(PdfALevel::A1b);
        let mut dict = PdfDictionary::new();
        dict.insert(
            "S".to_string(),
            PdfObject::Name(PdfName("Launch".to_string())),
        );

        let mut errors = Vec::new();
        validator.check_for_external_action(&dict, &mut errors);

        assert_eq!(errors.len(), 1);
    }

    #[test]
    fn test_check_for_external_action_goto_internal() {
        use crate::parser::objects::{PdfDictionary, PdfName, PdfObject};

        let validator = PdfAValidator::new(PdfALevel::A1b);
        let mut dict = PdfDictionary::new();
        dict.insert(
            "S".to_string(),
            PdfObject::Name(PdfName("GoTo".to_string())),
        );

        let mut errors = Vec::new();
        validator.check_for_external_action(&dict, &mut errors);

        assert_eq!(errors.len(), 0); // Internal GoTo is allowed
    }
}
