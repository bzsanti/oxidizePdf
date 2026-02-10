//! PDF/A Validation Engine
//!
//! This module provides the core validation logic for PDF/A compliance.
//! It checks PDF documents against the requirements of PDF/A standards.

use super::error::{PdfAError, ValidationError};
use super::types::{PdfAConformance, PdfALevel, ValidationResult, ValidationWarning};
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
        if !self.level.allows_transparency() {
            self.check_transparency(reader, &mut errors)?;
        }

        // Check compression (LZW forbidden in PDF/A-1)
        if !self.level.allows_lzw() {
            self.check_lzw_compression(reader, &mut errors)?;
        }

        // Check embedded files
        self.check_embedded_files(reader, &catalog_data, &mut errors)?;

        // Check fonts (must be embedded, Level A requires ToUnicode)
        self.check_fonts(reader, &mut errors)?;

        // Check color spaces and output intent
        self.check_color_spaces(reader, &mut errors)?;

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

    /// Check for transparency usage (forbidden in PDF/A-1)
    fn check_transparency<R: Read + Seek>(
        &self,
        reader: &mut PdfReader<R>,
        errors: &mut Vec<ValidationError>,
    ) -> Result<(), PdfAError> {
        let page_count = reader
            .page_count()
            .map_err(|e| PdfAError::ParseError(e.to_string()))?;

        for page_idx in 0..page_count {
            // Get page dictionary
            let page_dict = self.get_page_dict(reader, page_idx)?;

            // Check Resources for ExtGState with transparency
            if let Some(resources) = self.get_resources_dict(reader, &page_dict)? {
                // Check ExtGState entries
                if let Some(ext_gstate) = resources.get("ExtGState") {
                    self.check_ext_gstate_transparency(reader, ext_gstate, page_idx, errors)?;
                }

                // Check XObject entries for transparency groups
                if let Some(xobjects) = resources.get("XObject") {
                    self.check_xobject_transparency(reader, xobjects, page_idx, errors)?;
                }
            }
        }

        Ok(())
    }

    /// Get a page dictionary by index
    fn get_page_dict<R: Read + Seek>(
        &self,
        reader: &mut PdfReader<R>,
        page_idx: u32,
    ) -> Result<crate::parser::objects::PdfDictionary, PdfAError> {
        // Get pages dict
        let pages_dict = reader
            .pages()
            .map_err(|e| PdfAError::ParseError(e.to_string()))?
            .clone();

        // Get Kids array
        let kids = pages_dict
            .get("Kids")
            .and_then(|k| k.as_array())
            .ok_or_else(|| PdfAError::ParseError("Pages missing Kids array".to_string()))?;

        // Get page reference
        let page_ref = kids
            .0
            .get(page_idx as usize)
            .ok_or_else(|| PdfAError::ParseError(format!("Page {} not found", page_idx)))?;

        // Resolve page reference
        if let Some((obj_num, gen_num)) = page_ref.as_reference() {
            let page_obj = reader
                .get_object(obj_num, gen_num)
                .map_err(|e| PdfAError::ParseError(e.to_string()))?;
            page_obj
                .as_dict()
                .cloned()
                .ok_or_else(|| PdfAError::ParseError("Page is not a dictionary".to_string()))
        } else if let Some(dict) = page_ref.as_dict() {
            Ok(dict.clone())
        } else {
            Err(PdfAError::ParseError("Invalid page reference".to_string()))
        }
    }

    /// Get Resources dictionary from page, resolving if needed
    fn get_resources_dict<R: Read + Seek>(
        &self,
        reader: &mut PdfReader<R>,
        page_dict: &crate::parser::objects::PdfDictionary,
    ) -> Result<Option<crate::parser::objects::PdfDictionary>, PdfAError> {
        let resources_obj = match page_dict.get("Resources") {
            Some(obj) => obj,
            None => return Ok(None),
        };

        if let Some((obj_num, gen_num)) = resources_obj.as_reference() {
            let resolved = reader
                .get_object(obj_num, gen_num)
                .map_err(|e| PdfAError::ParseError(e.to_string()))?;
            Ok(resolved.as_dict().cloned())
        } else {
            Ok(resources_obj.as_dict().cloned())
        }
    }

    /// Check ExtGState dictionary for transparency settings
    fn check_ext_gstate_transparency<R: Read + Seek>(
        &self,
        reader: &mut PdfReader<R>,
        ext_gstate_obj: &crate::parser::objects::PdfObject,
        page_idx: u32,
        errors: &mut Vec<ValidationError>,
    ) -> Result<(), PdfAError> {
        let ext_gstate_dict = if let Some((obj_num, gen_num)) = ext_gstate_obj.as_reference() {
            let obj = reader
                .get_object(obj_num, gen_num)
                .map_err(|e| PdfAError::ParseError(e.to_string()))?;
            obj.as_dict().cloned()
        } else {
            ext_gstate_obj.as_dict().cloned()
        };

        let ext_gstate_dict = match ext_gstate_dict {
            Some(d) => d,
            None => return Ok(()),
        };

        // Check each graphics state entry
        for (gs_name, gs_value) in ext_gstate_dict.0.iter() {
            let gs_dict = if let Some((obj_num, gen_num)) = gs_value.as_reference() {
                let obj = reader
                    .get_object(obj_num, gen_num)
                    .map_err(|e| PdfAError::ParseError(e.to_string()))?;
                obj.as_dict().cloned()
            } else {
                gs_value.as_dict().cloned()
            };

            if let Some(gs_dict) = gs_dict {
                // Check for CA (stroke alpha) != 1.0
                if let Some(ca) = gs_dict.get("CA") {
                    let val = ca.as_real().or_else(|| ca.as_integer().map(|i| i as f64));
                    if let Some(val) = val {
                        if (val - 1.0).abs() > f64::EPSILON {
                            errors.push(ValidationError::TransparencyForbidden {
                                location: format!(
                                    "Page {}, ExtGState/{}/CA",
                                    page_idx + 1,
                                    &gs_name.0
                                ),
                            });
                        }
                    }
                }

                // Check for ca (fill alpha) != 1.0
                if let Some(ca) = gs_dict.get("ca") {
                    let val = ca.as_real().or_else(|| ca.as_integer().map(|i| i as f64));
                    if let Some(val) = val {
                        if (val - 1.0).abs() > f64::EPSILON {
                            errors.push(ValidationError::TransparencyForbidden {
                                location: format!(
                                    "Page {}, ExtGState/{}/ca",
                                    page_idx + 1,
                                    &gs_name.0
                                ),
                            });
                        }
                    }
                }

                // Check for SMask
                if let Some(smask) = gs_dict.get("SMask") {
                    // SMask /None is allowed
                    let is_none = smask.as_name().map(|n| n.0 == "None").unwrap_or(false);
                    if !is_none {
                        errors.push(ValidationError::TransparencyForbidden {
                            location: format!(
                                "Page {}, ExtGState/{}/SMask",
                                page_idx + 1,
                                &gs_name.0
                            ),
                        });
                    }
                }

                // Check for BM (blend mode) != Normal
                if let Some(bm) = gs_dict.get("BM") {
                    if let Some(name) = bm.as_name() {
                        if name.0 != "Normal" && name.0 != "Compatible" {
                            errors.push(ValidationError::TransparencyForbidden {
                                location: format!(
                                    "Page {}, ExtGState/{}/BM={}",
                                    page_idx + 1,
                                    &gs_name.0,
                                    &name.0
                                ),
                            });
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Check XObject dictionary for transparency groups
    fn check_xobject_transparency<R: Read + Seek>(
        &self,
        reader: &mut PdfReader<R>,
        xobject_obj: &crate::parser::objects::PdfObject,
        page_idx: u32,
        errors: &mut Vec<ValidationError>,
    ) -> Result<(), PdfAError> {
        let xobject_dict = if let Some((obj_num, gen_num)) = xobject_obj.as_reference() {
            let obj = reader
                .get_object(obj_num, gen_num)
                .map_err(|e| PdfAError::ParseError(e.to_string()))?;
            obj.as_dict().cloned()
        } else {
            xobject_obj.as_dict().cloned()
        };

        let xobject_dict = match xobject_dict {
            Some(d) => d,
            None => return Ok(()),
        };

        // Check each XObject entry
        for (xo_name, xo_value) in xobject_dict.0.iter() {
            let xo_stream_dict = if let Some((obj_num, gen_num)) = xo_value.as_reference() {
                let obj = reader
                    .get_object(obj_num, gen_num)
                    .map_err(|e| PdfAError::ParseError(e.to_string()))?;
                if let Some(stream) = obj.as_stream() {
                    Some(stream.dict.clone())
                } else {
                    obj.as_dict().cloned()
                }
            } else if let Some(stream) = xo_value.as_stream() {
                Some(stream.dict.clone())
            } else {
                xo_value.as_dict().cloned()
            };

            if let Some(xo_dict) = xo_stream_dict {
                // Check for transparency group (/Group with /S /Transparency)
                if let Some(group) = xo_dict.get("Group") {
                    let group_dict = if let Some((obj_num, gen_num)) = group.as_reference() {
                        let obj = reader
                            .get_object(obj_num, gen_num)
                            .map_err(|e| PdfAError::ParseError(e.to_string()))?;
                        obj.as_dict().cloned()
                    } else {
                        group.as_dict().cloned()
                    };

                    if let Some(group_dict) = group_dict {
                        if let Some(s) = group_dict.get("S") {
                            if let Some(name) = s.as_name() {
                                if name.0 == "Transparency" {
                                    errors.push(ValidationError::TransparencyForbidden {
                                        location: format!(
                                            "Page {}, XObject/{} has transparency group",
                                            page_idx + 1,
                                            &xo_name.0
                                        ),
                                    });
                                }
                            }
                        }
                    }
                }

                // Check for SMask in Image XObjects
                if let Some(subtype) = xo_dict.get("Subtype") {
                    if let Some(name) = subtype.as_name() {
                        if name.0 == "Image" {
                            if xo_dict.get("SMask").is_some() {
                                errors.push(ValidationError::TransparencyForbidden {
                                    location: format!(
                                        "Page {}, Image XObject/{} has SMask",
                                        page_idx + 1,
                                        &xo_name.0
                                    ),
                                });
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Check for LZW compression (forbidden in PDF/A-1)
    ///
    /// Note: This performs a sample check on page resources. A full implementation
    /// would scan all streams in the document.
    fn check_lzw_compression<R: Read + Seek>(
        &self,
        reader: &mut PdfReader<R>,
        errors: &mut Vec<ValidationError>,
    ) -> Result<(), PdfAError> {
        let page_count = reader
            .page_count()
            .map_err(|e| PdfAError::ParseError(e.to_string()))?;

        for page_idx in 0..page_count {
            let page_dict = self.get_page_dict(reader, page_idx)?;

            // Check Resources for XObjects that might use LZW
            if let Some(resources) = self.get_resources_dict(reader, &page_dict)? {
                if let Some(xobjects) = resources.get("XObject") {
                    self.check_xobjects_for_lzw(reader, xobjects, page_idx, errors)?;
                }
            }

            // Check content stream(s)
            if let Some(contents) = page_dict.get("Contents") {
                self.check_contents_for_lzw(reader, contents, page_idx, errors)?;
            }
        }

        Ok(())
    }

    /// Check XObjects for LZW compression
    fn check_xobjects_for_lzw<R: Read + Seek>(
        &self,
        reader: &mut PdfReader<R>,
        xobject_obj: &crate::parser::objects::PdfObject,
        page_idx: u32,
        errors: &mut Vec<ValidationError>,
    ) -> Result<(), PdfAError> {
        let xobject_dict = if let Some((obj_num, gen_num)) = xobject_obj.as_reference() {
            let obj = reader
                .get_object(obj_num, gen_num)
                .map_err(|e| PdfAError::ParseError(e.to_string()))?;
            obj.as_dict().cloned()
        } else {
            xobject_obj.as_dict().cloned()
        };

        let xobject_dict = match xobject_dict {
            Some(d) => d,
            None => return Ok(()),
        };

        for (_xo_name, xo_value) in xobject_dict.0.iter() {
            if let Some((obj_num, gen_num)) = xo_value.as_reference() {
                if let Ok(obj) = reader.get_object(obj_num, gen_num) {
                    if let Some(stream) = obj.as_stream() {
                        self.check_stream_for_lzw(&stream.dict, obj_num, errors);
                    }
                }
            }
        }

        // Suppress unused variable warning
        let _ = page_idx;
        Ok(())
    }

    /// Check content streams for LZW compression
    fn check_contents_for_lzw<R: Read + Seek>(
        &self,
        reader: &mut PdfReader<R>,
        contents: &crate::parser::objects::PdfObject,
        _page_idx: u32,
        errors: &mut Vec<ValidationError>,
    ) -> Result<(), PdfAError> {
        // Contents can be a reference or an array of references
        if let Some((obj_num, gen_num)) = contents.as_reference() {
            if let Ok(obj) = reader.get_object(obj_num, gen_num) {
                if let Some(stream) = obj.as_stream() {
                    self.check_stream_for_lzw(&stream.dict, obj_num, errors);
                }
            }
        } else if let Some(arr) = contents.as_array() {
            for item in &arr.0 {
                if let Some((obj_num, gen_num)) = item.as_reference() {
                    if let Ok(obj) = reader.get_object(obj_num, gen_num) {
                        if let Some(stream) = obj.as_stream() {
                            self.check_stream_for_lzw(&stream.dict, obj_num, errors);
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Check a stream dictionary for LZW filter
    fn check_stream_for_lzw(
        &self,
        dict: &crate::parser::objects::PdfDictionary,
        obj_num: u32,
        errors: &mut Vec<ValidationError>,
    ) {
        if let Some(filter) = dict.get("Filter") {
            // Filter can be a name or array
            if let Some(name) = filter.as_name() {
                if name.0 == "LZWDecode" {
                    errors.push(ValidationError::LzwCompressionForbidden {
                        object_id: format!("{} 0", obj_num),
                    });
                }
            } else if let Some(arr) = filter.as_array() {
                for (idx, f) in arr.0.iter().enumerate() {
                    if let Some(name) = f.as_name() {
                        if name.0 == "LZWDecode" {
                            errors.push(ValidationError::LzwCompressionForbidden {
                                object_id: format!("{} 0 (filter {})", obj_num, idx),
                            });
                        }
                    }
                }
            }
        }
    }

    /// Check for embedded files (forbidden in PDF/A-1 and PDF/A-2)
    fn check_embedded_files<R: Read + Seek>(
        &self,
        reader: &mut PdfReader<R>,
        catalog_data: &CatalogData,
        errors: &mut Vec<ValidationError>,
    ) -> Result<(), PdfAError> {
        if self.level.allows_embedded_files() {
            // PDF/A-3 allows embedded files, but they must have proper metadata
            // Full validation would check AF entries
            return Ok(());
        }

        // Check Names/EmbeddedFiles
        if let Some((obj_num, gen_num)) = catalog_data.names_ref {
            let names_obj = reader
                .get_object(obj_num, gen_num)
                .map_err(|e| PdfAError::ParseError(e.to_string()))?;

            if let Some(names_dict) = names_obj.as_dict() {
                if names_dict.get("EmbeddedFiles").is_some() {
                    errors.push(ValidationError::EmbeddedFileForbidden);
                }
            }
        } else if let Some(ref names_dict) = catalog_data.names_inline {
            if names_dict.get("EmbeddedFiles").is_some() {
                errors.push(ValidationError::EmbeddedFileForbidden);
            }
        }

        Ok(())
    }

    /// Check that all fonts are properly embedded
    fn check_fonts<R: Read + Seek>(
        &self,
        reader: &mut PdfReader<R>,
        errors: &mut Vec<ValidationError>,
    ) -> Result<(), PdfAError> {
        let page_count = reader
            .page_count()
            .map_err(|e| PdfAError::ParseError(e.to_string()))?;
        let requires_tounicode = self.level.conformance() == PdfAConformance::A;

        for page_idx in 0..page_count {
            let page_dict = self.get_page_dict(reader, page_idx)?;

            if let Some(resources) = self.get_resources_dict(reader, &page_dict)? {
                if let Some(fonts_obj) = resources.get("Font") {
                    self.check_font_resources(reader, fonts_obj, requires_tounicode, errors)?;
                }
            }
        }

        Ok(())
    }

    /// Check font resources dictionary
    fn check_font_resources<R: Read + Seek>(
        &self,
        reader: &mut PdfReader<R>,
        fonts_obj: &crate::parser::objects::PdfObject,
        requires_tounicode: bool,
        errors: &mut Vec<ValidationError>,
    ) -> Result<(), PdfAError> {
        let fonts_dict = if let Some((obj_num, gen_num)) = fonts_obj.as_reference() {
            let obj = reader
                .get_object(obj_num, gen_num)
                .map_err(|e| PdfAError::ParseError(e.to_string()))?;
            obj.as_dict().cloned()
        } else {
            fonts_obj.as_dict().cloned()
        };

        let fonts_dict = match fonts_dict {
            Some(d) => d,
            None => return Ok(()),
        };

        // Check each font
        for (font_name, font_ref) in fonts_dict.0.iter() {
            let font_dict = if let Some((obj_num, gen_num)) = font_ref.as_reference() {
                let obj = reader
                    .get_object(obj_num, gen_num)
                    .map_err(|e| PdfAError::ParseError(e.to_string()))?;
                obj.as_dict().cloned()
            } else {
                font_ref.as_dict().cloned()
            };

            if let Some(font_dict) = font_dict {
                self.check_single_font(
                    reader,
                    &font_name.0,
                    &font_dict,
                    requires_tounicode,
                    errors,
                )?;
            }
        }

        Ok(())
    }

    /// Check a single font for PDF/A compliance
    fn check_single_font<R: Read + Seek>(
        &self,
        reader: &mut PdfReader<R>,
        font_name: &str,
        font_dict: &crate::parser::objects::PdfDictionary,
        requires_tounicode: bool,
        errors: &mut Vec<ValidationError>,
    ) -> Result<(), PdfAError> {
        // Get font type
        let font_type = font_dict
            .get("Subtype")
            .and_then(|s| s.as_name())
            .map(|n| n.0.clone())
            .unwrap_or_default();

        // Type3 fonts have different requirements
        if font_type == "Type3" {
            // Type3 fonts are always considered "embedded" as they define glyphs inline
            // But for Level A, they still need character mapping
            if requires_tounicode && font_dict.get("ToUnicode").is_none() {
                errors.push(ValidationError::FontMissingToUnicode {
                    font_name: font_name.to_string(),
                });
            }
            return Ok(());
        }

        // For Type0 (composite) fonts, check the descendant font
        if font_type == "Type0" {
            return self.check_type0_font(reader, font_name, font_dict, requires_tounicode, errors);
        }

        // Check FontDescriptor for embedding (Type1, TrueType, etc.)
        let font_descriptor = self.get_font_descriptor(reader, font_dict)?;

        if let Some(desc) = font_descriptor {
            // Check for font embedding: FontFile, FontFile2, or FontFile3
            let has_fontfile = desc.get("FontFile").is_some()
                || desc.get("FontFile2").is_some()
                || desc.get("FontFile3").is_some();

            if !has_fontfile {
                errors.push(ValidationError::FontNotEmbedded {
                    font_name: font_name.to_string(),
                });
            }
        } else {
            // No FontDescriptor means the font is not embedded
            // Exception: standard 14 fonts technically don't need FontDescriptor
            // but PDF/A still requires them to be embedded
            errors.push(ValidationError::FontNotEmbedded {
                font_name: font_name.to_string(),
            });
        }

        // For Level A conformance, check ToUnicode
        if requires_tounicode && font_dict.get("ToUnicode").is_none() {
            // Check if font has proper encoding that allows Unicode mapping
            let has_encoding = font_dict.get("Encoding").is_some();
            if !has_encoding {
                errors.push(ValidationError::FontMissingToUnicode {
                    font_name: font_name.to_string(),
                });
            }
        }

        Ok(())
    }

    /// Check Type0 (composite) font for embedding
    fn check_type0_font<R: Read + Seek>(
        &self,
        reader: &mut PdfReader<R>,
        font_name: &str,
        font_dict: &crate::parser::objects::PdfDictionary,
        requires_tounicode: bool,
        errors: &mut Vec<ValidationError>,
    ) -> Result<(), PdfAError> {
        // Get DescendantFonts array
        let descendants = match font_dict.get("DescendantFonts") {
            Some(d) => d,
            None => {
                errors.push(ValidationError::FontNotEmbedded {
                    font_name: font_name.to_string(),
                });
                return Ok(());
            }
        };

        let desc_array = if let Some((obj_num, gen_num)) = descendants.as_reference() {
            let obj = reader
                .get_object(obj_num, gen_num)
                .map_err(|e| PdfAError::ParseError(e.to_string()))?;
            obj.as_array().cloned()
        } else {
            descendants.as_array().cloned()
        };

        let desc_array = match desc_array {
            Some(a) => a,
            None => return Ok(()),
        };

        // Check first descendant font (CIDFont)
        if let Some(cid_font_ref) = desc_array.0.first() {
            let cid_font_dict = if let Some((obj_num, gen_num)) = cid_font_ref.as_reference() {
                let obj = reader
                    .get_object(obj_num, gen_num)
                    .map_err(|e| PdfAError::ParseError(e.to_string()))?;
                obj.as_dict().cloned()
            } else {
                cid_font_ref.as_dict().cloned()
            };

            if let Some(cid_dict) = cid_font_dict {
                // Check CIDFont's FontDescriptor
                let font_descriptor = self.get_font_descriptor(reader, &cid_dict)?;

                if let Some(desc) = font_descriptor {
                    let has_fontfile = desc.get("FontFile").is_some()
                        || desc.get("FontFile2").is_some()
                        || desc.get("FontFile3").is_some();

                    if !has_fontfile {
                        errors.push(ValidationError::FontNotEmbedded {
                            font_name: font_name.to_string(),
                        });
                    }
                } else {
                    errors.push(ValidationError::FontNotEmbedded {
                        font_name: font_name.to_string(),
                    });
                }
            }
        }

        // For Level A, check ToUnicode or CMap
        if requires_tounicode && font_dict.get("ToUnicode").is_none() {
            // Type0 fonts might use Identity-H/V encoding which is acceptable
            // if combined with proper CIDToGIDMap
            let encoding = font_dict.get("Encoding").and_then(|e| e.as_name());
            let is_identity = encoding
                .map(|n| n.0 == "Identity-H" || n.0 == "Identity-V")
                .unwrap_or(false);

            if !is_identity {
                errors.push(ValidationError::FontMissingToUnicode {
                    font_name: font_name.to_string(),
                });
            }
        }

        Ok(())
    }

    /// Get FontDescriptor from a font dictionary
    fn get_font_descriptor<R: Read + Seek>(
        &self,
        reader: &mut PdfReader<R>,
        font_dict: &crate::parser::objects::PdfDictionary,
    ) -> Result<Option<crate::parser::objects::PdfDictionary>, PdfAError> {
        let desc_ref = match font_dict.get("FontDescriptor") {
            Some(d) => d,
            None => return Ok(None),
        };

        if let Some((obj_num, gen_num)) = desc_ref.as_reference() {
            let obj = reader
                .get_object(obj_num, gen_num)
                .map_err(|e| PdfAError::ParseError(e.to_string()))?;
            Ok(obj.as_dict().cloned())
        } else {
            Ok(desc_ref.as_dict().cloned())
        }
    }

    /// Check color spaces for PDF/A compliance
    ///
    /// PDF/A requires device-independent color spaces or a properly defined
    /// OutputIntent. Device-dependent color spaces (DeviceRGB, DeviceCMYK,
    /// DeviceGray) are only allowed if an OutputIntent is present.
    fn check_color_spaces<R: Read + Seek>(
        &self,
        reader: &mut PdfReader<R>,
        errors: &mut Vec<ValidationError>,
    ) -> Result<(), PdfAError> {
        // First, check if there's an OutputIntent in the catalog
        let has_output_intent = self.has_output_intent(reader)?;

        let page_count = reader
            .page_count()
            .map_err(|e| PdfAError::ParseError(e.to_string()))?;

        for page_idx in 0..page_count {
            let page_dict = self.get_page_dict(reader, page_idx)?;

            if let Some(resources) = self.get_resources_dict(reader, &page_dict)? {
                // Check ColorSpace dictionary
                if let Some(cs_obj) = resources.get("ColorSpace") {
                    self.check_colorspace_dict(
                        reader,
                        cs_obj,
                        page_idx,
                        has_output_intent,
                        errors,
                    )?;
                }

                // Check XObjects for uncalibrated color spaces in images
                if let Some(xobjects) = resources.get("XObject") {
                    self.check_xobject_colorspaces(
                        reader,
                        xobjects,
                        page_idx,
                        has_output_intent,
                        errors,
                    )?;
                }
            }
        }

        Ok(())
    }

    /// Check if the document has a valid OutputIntent
    fn has_output_intent<R: Read + Seek>(
        &self,
        reader: &mut PdfReader<R>,
    ) -> Result<bool, PdfAError> {
        let catalog = reader
            .catalog()
            .map_err(|e| PdfAError::ParseError(e.to_string()))?;

        if let Some(output_intents) = catalog.get("OutputIntents") {
            let arr = if let Some((obj_num, gen_num)) = output_intents.as_reference() {
                let obj = reader
                    .get_object(obj_num, gen_num)
                    .map_err(|e| PdfAError::ParseError(e.to_string()))?;
                obj.as_array().cloned()
            } else {
                output_intents.as_array().cloned()
            };

            if let Some(arr) = arr {
                // Check if any OutputIntent has the required PDF/A subtype
                for item in &arr.0 {
                    let intent_dict = if let Some((obj_num, gen_num)) = item.as_reference() {
                        let obj = reader
                            .get_object(obj_num, gen_num)
                            .map_err(|e| PdfAError::ParseError(e.to_string()))?;
                        obj.as_dict().cloned()
                    } else {
                        item.as_dict().cloned()
                    };

                    if let Some(dict) = intent_dict {
                        // Check for GTS_PDFA1 or similar subtype
                        if let Some(subtype) = dict.get("S") {
                            if let Some(name) = subtype.as_name() {
                                if name.0.contains("PDFA") || name.0.contains("PDF/A") {
                                    return Ok(true);
                                }
                            }
                        }
                        // Also check for DestOutputProfile
                        if dict.get("DestOutputProfile").is_some() {
                            return Ok(true);
                        }
                    }
                }
            }
        }

        Ok(false)
    }

    /// Check ColorSpace dictionary entries
    fn check_colorspace_dict<R: Read + Seek>(
        &self,
        reader: &mut PdfReader<R>,
        cs_obj: &crate::parser::objects::PdfObject,
        page_idx: u32,
        has_output_intent: bool,
        errors: &mut Vec<ValidationError>,
    ) -> Result<(), PdfAError> {
        let cs_dict = if let Some((obj_num, gen_num)) = cs_obj.as_reference() {
            let obj = reader
                .get_object(obj_num, gen_num)
                .map_err(|e| PdfAError::ParseError(e.to_string()))?;
            obj.as_dict().cloned()
        } else {
            cs_obj.as_dict().cloned()
        };

        let cs_dict = match cs_dict {
            Some(d) => d,
            None => return Ok(()),
        };

        for (cs_name, cs_value) in cs_dict.0.iter() {
            self.validate_colorspace(
                reader,
                &cs_name.0,
                cs_value,
                page_idx,
                has_output_intent,
                errors,
            )?;
        }

        Ok(())
    }

    /// Validate a single color space entry
    fn validate_colorspace<R: Read + Seek>(
        &self,
        reader: &mut PdfReader<R>,
        cs_name: &str,
        cs_value: &crate::parser::objects::PdfObject,
        page_idx: u32,
        has_output_intent: bool,
        errors: &mut Vec<ValidationError>,
    ) -> Result<(), PdfAError> {
        // Color space can be a name or an array
        let cs_type = if let Some(name) = cs_value.as_name() {
            name.0.clone()
        } else if let Some(arr) = cs_value.as_array() {
            // First element is the color space name
            arr.0
                .first()
                .and_then(|o| o.as_name())
                .map(|n| n.0.clone())
                .unwrap_or_default()
        } else if let Some((obj_num, gen_num)) = cs_value.as_reference() {
            // Resolve reference
            let obj = reader
                .get_object(obj_num, gen_num)
                .map_err(|e| PdfAError::ParseError(e.to_string()))?;
            if let Some(name) = obj.as_name() {
                name.0.clone()
            } else if let Some(arr) = obj.as_array() {
                arr.0
                    .first()
                    .and_then(|o| o.as_name())
                    .map(|n| n.0.clone())
                    .unwrap_or_default()
            } else {
                return Ok(());
            }
        } else {
            return Ok(());
        };

        // Check if it's a device-dependent color space
        if self.is_device_dependent_colorspace(&cs_type) && !has_output_intent {
            errors.push(ValidationError::InvalidColorSpace {
                color_space: cs_type,
                location: format!("Page {}, ColorSpace/{}", page_idx + 1, cs_name),
            });
        }

        Ok(())
    }

    /// Check XObjects for device-dependent color spaces
    fn check_xobject_colorspaces<R: Read + Seek>(
        &self,
        reader: &mut PdfReader<R>,
        xobjects_obj: &crate::parser::objects::PdfObject,
        page_idx: u32,
        has_output_intent: bool,
        errors: &mut Vec<ValidationError>,
    ) -> Result<(), PdfAError> {
        let xobjects_dict = if let Some((obj_num, gen_num)) = xobjects_obj.as_reference() {
            let obj = reader
                .get_object(obj_num, gen_num)
                .map_err(|e| PdfAError::ParseError(e.to_string()))?;
            obj.as_dict().cloned()
        } else {
            xobjects_obj.as_dict().cloned()
        };

        let xobjects_dict = match xobjects_dict {
            Some(d) => d,
            None => return Ok(()),
        };

        for (xo_name, xo_ref) in xobjects_dict.0.iter() {
            let xo_dict = if let Some((obj_num, gen_num)) = xo_ref.as_reference() {
                let obj = reader
                    .get_object(obj_num, gen_num)
                    .map_err(|e| PdfAError::ParseError(e.to_string()))?;
                if let Some(stream) = obj.as_stream() {
                    Some(stream.dict.clone())
                } else {
                    obj.as_dict().cloned()
                }
            } else if let Some(stream) = xo_ref.as_stream() {
                Some(stream.dict.clone())
            } else {
                xo_ref.as_dict().cloned()
            };

            if let Some(dict) = xo_dict {
                // Check if it's an Image XObject
                let is_image = dict
                    .get("Subtype")
                    .and_then(|s| s.as_name())
                    .map(|n| n.0 == "Image")
                    .unwrap_or(false);

                if is_image {
                    // Check ColorSpace of the image
                    if let Some(cs) = dict.get("ColorSpace") {
                        let cs_type = if let Some(name) = cs.as_name() {
                            name.0.clone()
                        } else if let Some(arr) = cs.as_array() {
                            arr.0
                                .first()
                                .and_then(|o| o.as_name())
                                .map(|n| n.0.clone())
                                .unwrap_or_default()
                        } else {
                            String::new()
                        };

                        if self.is_device_dependent_colorspace(&cs_type) && !has_output_intent {
                            errors.push(ValidationError::InvalidColorSpace {
                                color_space: cs_type,
                                location: format!("Page {}, XObject/{}", page_idx + 1, &xo_name.0),
                            });
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Check if a color space is device-dependent
    fn is_device_dependent_colorspace(&self, cs_type: &str) -> bool {
        matches!(cs_type, "DeviceRGB" | "DeviceCMYK" | "DeviceGray")
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

    #[test]
    fn test_check_stream_for_lzw_single_filter() {
        use crate::parser::objects::{PdfDictionary, PdfName, PdfObject};

        let validator = PdfAValidator::new(PdfALevel::A1b);
        let mut dict = PdfDictionary::new();
        dict.insert(
            "Filter".to_string(),
            PdfObject::Name(PdfName("LZWDecode".to_string())),
        );

        let mut errors = Vec::new();
        validator.check_stream_for_lzw(&dict, 10, &mut errors);

        assert_eq!(errors.len(), 1);
        assert!(matches!(
            &errors[0],
            ValidationError::LzwCompressionForbidden { object_id } if object_id == "10 0"
        ));
    }

    #[test]
    fn test_check_stream_for_lzw_array_filter() {
        use crate::parser::objects::{PdfArray, PdfDictionary, PdfName, PdfObject};

        let validator = PdfAValidator::new(PdfALevel::A1b);
        let mut dict = PdfDictionary::new();
        let filters = PdfArray(vec![
            PdfObject::Name(PdfName("FlateDecode".to_string())),
            PdfObject::Name(PdfName("LZWDecode".to_string())),
        ]);
        dict.insert("Filter".to_string(), PdfObject::Array(filters));

        let mut errors = Vec::new();
        validator.check_stream_for_lzw(&dict, 20, &mut errors);

        assert_eq!(errors.len(), 1);
        assert!(matches!(
            &errors[0],
            ValidationError::LzwCompressionForbidden { object_id } if object_id.contains("20 0")
        ));
    }

    #[test]
    fn test_check_stream_for_lzw_no_lzw() {
        use crate::parser::objects::{PdfDictionary, PdfName, PdfObject};

        let validator = PdfAValidator::new(PdfALevel::A1b);
        let mut dict = PdfDictionary::new();
        dict.insert(
            "Filter".to_string(),
            PdfObject::Name(PdfName("FlateDecode".to_string())),
        );

        let mut errors = Vec::new();
        validator.check_stream_for_lzw(&dict, 30, &mut errors);

        assert_eq!(errors.len(), 0);
    }

    #[test]
    fn test_check_stream_for_lzw_no_filter() {
        use crate::parser::objects::PdfDictionary;

        let validator = PdfAValidator::new(PdfALevel::A1b);
        let dict = PdfDictionary::new();

        let mut errors = Vec::new();
        validator.check_stream_for_lzw(&dict, 40, &mut errors);

        assert_eq!(errors.len(), 0);
    }

    #[test]
    fn test_pdfa_level_allows_lzw() {
        // PDF/A-1 does not allow LZW
        assert!(!PdfALevel::A1b.allows_lzw());
        assert!(!PdfALevel::A1a.allows_lzw());

        // PDF/A-2 and PDF/A-3 allow LZW
        assert!(PdfALevel::A2b.allows_lzw());
        assert!(PdfALevel::A3b.allows_lzw());
    }

    #[test]
    fn test_pdfa_level_allows_embedded_files() {
        // PDF/A-1 and PDF/A-2 do not allow embedded files
        assert!(!PdfALevel::A1b.allows_embedded_files());
        assert!(!PdfALevel::A2b.allows_embedded_files());

        // PDF/A-3 allows embedded files
        assert!(PdfALevel::A3b.allows_embedded_files());
    }

    #[test]
    fn test_is_device_dependent_colorspace() {
        let validator = PdfAValidator::new(PdfALevel::A1b);

        // Device-dependent color spaces
        assert!(validator.is_device_dependent_colorspace("DeviceRGB"));
        assert!(validator.is_device_dependent_colorspace("DeviceCMYK"));
        assert!(validator.is_device_dependent_colorspace("DeviceGray"));

        // Device-independent color spaces
        assert!(!validator.is_device_dependent_colorspace("CalRGB"));
        assert!(!validator.is_device_dependent_colorspace("CalGray"));
        assert!(!validator.is_device_dependent_colorspace("Lab"));
        assert!(!validator.is_device_dependent_colorspace("ICCBased"));
        assert!(!validator.is_device_dependent_colorspace("Indexed"));
        assert!(!validator.is_device_dependent_colorspace("Pattern"));
    }

    #[test]
    fn test_pdfa_conformance_level_a() {
        // Level A requires ToUnicode for accessible conformance
        assert_eq!(PdfALevel::A1a.conformance(), PdfAConformance::A);
        assert_eq!(PdfALevel::A2a.conformance(), PdfAConformance::A);
        assert_eq!(PdfALevel::A3a.conformance(), PdfAConformance::A);

        // Level B is basic conformance
        assert_eq!(PdfALevel::A1b.conformance(), PdfAConformance::B);
        assert_eq!(PdfALevel::A2b.conformance(), PdfAConformance::B);
        assert_eq!(PdfALevel::A3b.conformance(), PdfAConformance::B);

        // Level U is Unicode conformance
        assert_eq!(PdfALevel::A2u.conformance(), PdfAConformance::U);
        assert_eq!(PdfALevel::A3u.conformance(), PdfAConformance::U);
    }
}
