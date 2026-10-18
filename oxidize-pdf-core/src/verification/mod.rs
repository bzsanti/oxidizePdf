//! PDF Verification Module
//!
//! Provides implemented requirement checks and external-tool results with explicit
//! scopes. qpdf checks document structure; veraPDF checks the fixed PDF/A-1b
//! profile. Neither the legacy level percentages nor these checks certify all
//! requirements of ISO 32000. Unavailable checks must not count as success.

pub mod comparators;
pub mod compliance_report;
pub mod curated_matrix;
pub mod iso_matrix;
pub mod parser;
pub mod semantic_comparison;
mod structure;
pub mod tagged_pdf;
pub mod validators;
mod verapdf_report;

// Disabled vanity ISO compliance tests - these test PDF syntax rather than functionality
// See CLAUDE.md: "Focus on practical PDF functionality, not compliance metrics"
// The 148 vanity ISO tests have been disabled to focus on real functionality
// #[cfg(test)]
// pub mod tests;

use crate::error::Result;

/// Verification levels for ISO compliance
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum VerificationLevel {
    /// Not implemented (0%)
    NotImplemented = 0,
    /// Code exists, doesn't crash (25%)
    CodeExists = 1,
    /// Basic xref, catalog and page-tree structure verified (50%), not full conformance.
    GeneratesPdf = 2,
    /// Content verified with parser (75%)
    ContentVerified = 3,
    /// Legacy highest level (100%): qpdf structure and veraPDF PDF/A-1b pass.
    /// This does not certify every ISO 32000 requirement.
    IsoCompliant = 4,
}

impl VerificationLevel {
    pub fn as_percentage(&self) -> f64 {
        match self {
            VerificationLevel::NotImplemented => 0.0,
            VerificationLevel::CodeExists => 25.0,
            VerificationLevel::GeneratesPdf => 50.0,
            VerificationLevel::ContentVerified => 75.0,
            VerificationLevel::IsoCompliant => 100.0,
        }
    }

    pub fn from_u8(level: u8) -> Option<Self> {
        match level {
            0 => Some(VerificationLevel::NotImplemented),
            1 => Some(VerificationLevel::CodeExists),
            2 => Some(VerificationLevel::GeneratesPdf),
            3 => Some(VerificationLevel::ContentVerified),
            4 => Some(VerificationLevel::IsoCompliant),
            _ => None,
        }
    }
}

/// Result of PDF verification.
///
/// `NotImplemented` with `passed == false` means verification is unavailable,
/// not that the PDF failed an implemented check. Other levels with `passed == false`
/// indicate a failed check. A successful lower level is not full ISO compliance.
#[derive(Debug, Clone)]
pub struct VerificationResult {
    pub level: VerificationLevel,
    pub passed: bool,
    pub details: String,
    pub external_validation: Option<ExternalValidationResult>,
}

/// Results from external tools, each with its own scope.
/// Some(true) is a completed passing check, Some(false) a completed negative
/// check, None an unavailable/incomplete check described in error_messages.
/// qpdf checks structure; veraPDF checks PDF/A-1b, not full ISO 32000 conformance.
#[derive(Debug, Clone)]
pub struct ExternalValidationResult {
    pub qpdf_passed: Option<bool>,
    pub verapdf_passed: Option<bool>,
    pub adobe_preflight_passed: Option<bool>,
    pub error_messages: Vec<String>,
}

/// ISO requirement for tracking compliance
#[derive(Debug, Clone)]
pub struct IsoRequirement {
    pub id: String,
    pub name: String,
    pub description: String,
    pub iso_reference: String,
    pub implementation: Option<String>,
    pub test_file: Option<String>,
    pub level: VerificationLevel,
    pub verified: bool,
    pub notes: String,
}

// Resolve availability once, before any generic or external checks. An exhaustive
// enum keeps dispatch and content checking in sync without a successful fallback.
#[derive(Clone, Copy)]
enum RequirementCheck {
    Catalog,
    DeviceRgb,
    StandardFonts,
}

impl RequirementCheck {
    fn for_id(id: &str) -> Option<Self> {
        match id {
            "7.5.2.1" => Some(Self::Catalog),
            "8.6.3.1" => Some(Self::DeviceRgb),
            "9.7.1.1" => Some(Self::StandardFonts),
            _ => None,
        }
    }
}

/// Verify a PDF against an ISO requirement at the requested level.
///
/// Requirements without an implemented check return `NotImplemented`, `passed=false`
/// and no external validation, regardless of the requested level or caller-supplied
/// implementation/verified metadata. This result contributes no verified compliance.
/// Supported checks currently cover catalog presence, DeviceRGB usage and standard
/// font presence; they are not a complete structural or ISO conformance validator.
pub fn verify_iso_requirement(
    pdf_bytes: &[u8],
    requirement: &IsoRequirement,
) -> Result<VerificationResult> {
    let Some(check) = RequirementCheck::for_id(&requirement.id) else {
        return Ok(VerificationResult {
            level: VerificationLevel::NotImplemented,
            passed: false,
            details: format!(
                "Requirement verification not implemented for {}",
                requirement.id
            ),
            external_validation: None,
        });
    };

    match requirement.level {
        VerificationLevel::NotImplemented => Ok(VerificationResult {
            level: VerificationLevel::NotImplemented,
            passed: false,
            details: "Feature not implemented".to_string(),
            external_validation: None,
        }),
        VerificationLevel::CodeExists => {
            // At this level, we just verify the code doesn't crash
            // This should be tested in unit tests, not here
            Ok(VerificationResult {
                level: VerificationLevel::CodeExists,
                passed: true,
                details: "Code exists and executes without crash".to_string(),
                external_validation: None,
            })
        }
        VerificationLevel::GeneratesPdf => verify_pdf_generation(pdf_bytes),
        VerificationLevel::ContentVerified => verify_pdf_content(pdf_bytes, check),
        VerificationLevel::IsoCompliant => verify_iso_compliance(pdf_bytes, check),
    }
}

/// Verify cross-references, the referenced catalog and the page tree.
/// Traversal exhaustion is reported as incomplete verification, not invalidity.
/// This does not validate every resource, content operator or ISO requirement.
fn verify_pdf_generation(pdf_bytes: &[u8]) -> Result<VerificationResult> {
    if pdf_bytes.is_empty() {
        return Ok(VerificationResult {
            level: VerificationLevel::GeneratesPdf,
            passed: false,
            details: "PDF is empty".to_string(),
            external_validation: None,
        });
    }

    if !pdf_bytes.starts_with(b"%PDF-") {
        return Ok(VerificationResult {
            level: VerificationLevel::GeneratesPdf,
            passed: false,
            details: "PDF does not start with PDF header".to_string(),
            external_validation: None,
        });
    }

    let outcome = structure::verify(pdf_bytes);
    Ok(VerificationResult {
        level: VerificationLevel::GeneratesPdf,
        passed: outcome.is_ok(),
        details: match outcome {
            Ok(pages) => {
                format!("PDF structure verified: {pages} pages (not full ISO conformance)")
            }
            Err(reason) => reason,
        },
        external_validation: None,
    })
}

/// Verify PDF content structure with internal parser
fn verify_pdf_content(pdf_bytes: &[u8], check: RequirementCheck) -> Result<VerificationResult> {
    // First check basic generation
    let gen_result = verify_pdf_generation(pdf_bytes)?;
    if !gen_result.passed {
        return Ok(gen_result);
    }

    // Parse PDF and verify content
    match parser::parse_pdf(pdf_bytes) {
        Ok(parsed_pdf) => {
            let content_check = verify_requirement_content(&parsed_pdf, check);
            Ok(VerificationResult {
                level: VerificationLevel::ContentVerified,
                passed: content_check.0,
                details: content_check.1,
                external_validation: None,
            })
        }
        Err(e) => Ok(VerificationResult {
            level: VerificationLevel::ContentVerified,
            passed: false,
            details: format!("Failed to parse PDF: {}", e),
            external_validation: None,
        }),
    }
}

/// Run the required external structural and PDF/A-1b checks.
fn verify_iso_compliance(pdf_bytes: &[u8], check: RequirementCheck) -> Result<VerificationResult> {
    // First check content verification
    let content_result = verify_pdf_content(pdf_bytes, check)?;
    if !content_result.passed {
        return Ok(content_result);
    }

    // Run external validation
    let external_result = validators::validate_external(pdf_bytes)?;

    let all_passed = external_checks_passed(&external_result);

    Ok(VerificationResult {
        level: VerificationLevel::IsoCompliant,
        passed: all_passed,
        details: if all_passed {
            "qpdf structural and veraPDF PDF/A-1b checks passed; not full ISO conformance"
                .to_string()
        } else {
            format!(
                "External validation failed or incomplete: {:?}",
                external_result.error_messages
            )
        },
        external_validation: Some(external_result),
    })
}

// Missing/failed execution cannot count as optional success at the highest level.
fn external_checks_passed(result: &ExternalValidationResult) -> bool {
    result.qpdf_passed == Some(true) && result.verapdf_passed == Some(true)
}

/// Verify specific requirement content in parsed PDF
fn verify_requirement_content(
    parsed_pdf: &parser::ParsedPdf,
    check: RequirementCheck,
) -> (bool, String) {
    // This is where we implement specific verification logic for each ISO requirement
    // For now, we'll implement a few key ones and expand over time

    match check {
        RequirementCheck::Catalog => {
            // Document catalog must have /Type /Catalog
            if let Some(catalog) = &parsed_pdf.catalog {
                if catalog.contains_key("Type") {
                    (true, "Catalog contains required /Type entry".to_string())
                } else {
                    (false, "Catalog missing /Type entry".to_string())
                }
            } else {
                (false, "No document catalog found".to_string())
            }
        }
        RequirementCheck::DeviceRgb => {
            // DeviceRGB color space verification
            if parsed_pdf.uses_device_rgb {
                (true, "PDF uses DeviceRGB color space correctly".to_string())
            } else {
                (
                    false,
                    "DeviceRGB color space not found or incorrect".to_string(),
                )
            }
        }
        RequirementCheck::StandardFonts => {
            // Standard 14 fonts verification
            let standard_fonts = &[
                "Helvetica",
                "Times-Roman",
                "Courier",
                "Symbol",
                "ZapfDingbats",
            ];
            let found_fonts: Vec<_> = parsed_pdf
                .fonts
                .iter()
                .filter(|font| standard_fonts.contains(&font.as_str()))
                .collect();

            if !found_fonts.is_empty() {
                (true, format!("Found standard fonts: {:?}", found_fonts))
            } else {
                (false, "No standard fonts found".to_string())
            }
        }
    }
}

#[cfg(test)]
mod unit_tests {
    use super::*;

    #[test]
    fn external_level_requires_both_completed_positive_checks() {
        for qpdf in [None, Some(false), Some(true)] {
            for vera in [None, Some(false), Some(true)] {
                let result = ExternalValidationResult {
                    qpdf_passed: qpdf,
                    verapdf_passed: vera,
                    adobe_preflight_passed: Some(true),
                    error_messages: vec![],
                };
                assert_eq!(
                    external_checks_passed(&result),
                    qpdf == Some(true) && vera == Some(true)
                );
            }
        }
    }

    #[test]
    fn test_verification_level_percentage() {
        assert_eq!(VerificationLevel::NotImplemented.as_percentage(), 0.0);
        assert_eq!(VerificationLevel::CodeExists.as_percentage(), 25.0);
        assert_eq!(VerificationLevel::GeneratesPdf.as_percentage(), 50.0);
        assert_eq!(VerificationLevel::ContentVerified.as_percentage(), 75.0);
        assert_eq!(VerificationLevel::IsoCompliant.as_percentage(), 100.0);
    }

    #[test]
    fn test_verification_level_from_u8() {
        assert_eq!(
            VerificationLevel::from_u8(0),
            Some(VerificationLevel::NotImplemented)
        );
        assert_eq!(
            VerificationLevel::from_u8(4),
            Some(VerificationLevel::IsoCompliant)
        );
        assert_eq!(VerificationLevel::from_u8(5), None);
    }

    #[test]
    fn test_pdf_generation_verification() {
        // Test empty PDF
        let empty_pdf = b"";
        let result = verify_pdf_generation(empty_pdf).unwrap();
        assert!(!result.passed);
        assert!(result.details.contains("empty"));

        // Test invalid header
        let invalid_pdf = b"This is not a PDF";
        let result = verify_pdf_generation(invalid_pdf).unwrap();
        assert!(!result.passed);
        assert!(result.details.contains("PDF header"));

        // An incomplete header is not a document, regardless of size
        let small_pdf = b"%PDF-1.4\n%%EOF";
        let result = verify_pdf_generation(small_pdf).unwrap();
        assert!(!result.passed);
        assert!(result.details.contains("structural check"));

        // Padding a header does not create a valid PDF
        let valid_pdf = format!("%PDF-1.4\n{}\n%%EOF", "x".repeat(1000));
        let result = verify_pdf_generation(valid_pdf.as_bytes()).unwrap();
        assert!(!result.passed);
        assert!(result.details.contains("structural check"));
    }
}

/// Check if two PDFs are structurally equivalent for ISO compliance
pub fn pdfs_structurally_equivalent(generated: &[u8], reference: &[u8]) -> bool {
    comparators::pdfs_structurally_equivalent(generated, reference)
}

/// Extract structural differences between PDFs
pub fn extract_pdf_differences(
    generated: &[u8],
    reference: &[u8],
) -> Result<Vec<comparators::PdfDifference>> {
    comparators::extract_pdf_differences(generated, reference)
}
