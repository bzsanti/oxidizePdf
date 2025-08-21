//! PDF Comparators for Verification
//!
//! This module provides functions to compare generated PDFs with reference PDFs
//! to verify structural and content equivalence for ISO compliance testing.

use crate::error::Result;
use crate::verification::parser::{parse_pdf, ParsedPdf};
use std::collections::HashMap;

/// Difference between two PDFs
#[derive(Debug, Clone)]
pub struct PdfDifference {
    pub location: String,
    pub expected: String,
    pub actual: String,
    pub severity: DifferenceSeverity,
}

#[derive(Debug, Clone, PartialEq)]
pub enum DifferenceSeverity {
    /// Critical differences that break ISO compliance
    Critical,
    /// Important differences that may affect functionality
    Important,
    /// Minor differences that don't affect compliance
    Minor,
    /// Cosmetic differences (timestamps, IDs, etc.)
    Cosmetic,
}

/// Result of PDF comparison
#[derive(Debug, Clone)]
pub struct ComparisonResult {
    pub structurally_equivalent: bool,
    pub content_equivalent: bool,
    pub differences: Vec<PdfDifference>,
    pub similarity_score: f64, // 0.0 to 1.0
}

/// Compare two PDFs for structural equivalence
pub fn compare_pdfs(generated: &[u8], reference: &[u8]) -> Result<ComparisonResult> {
    let parsed_generated = parse_pdf(generated)?;
    let parsed_reference = parse_pdf(reference)?;

    let differences = find_differences(&parsed_generated, &parsed_reference);
    let similarity_score = calculate_similarity_score(&differences);

    let structurally_equivalent = differences.iter().all(|diff| {
        diff.severity == DifferenceSeverity::Cosmetic || diff.severity == DifferenceSeverity::Minor
    });

    let content_equivalent = differences
        .iter()
        .all(|diff| diff.severity == DifferenceSeverity::Cosmetic);

    Ok(ComparisonResult {
        structurally_equivalent,
        content_equivalent,
        differences,
        similarity_score,
    })
}

/// Find differences between two parsed PDFs
fn find_differences(generated: &ParsedPdf, reference: &ParsedPdf) -> Vec<PdfDifference> {
    let mut differences = Vec::new();

    // Compare versions (minor difference unless major version change)
    if generated.version != reference.version {
        let severity = if generated.version.chars().next() != reference.version.chars().next() {
            DifferenceSeverity::Important
        } else {
            DifferenceSeverity::Minor
        };

        differences.push(PdfDifference {
            location: "PDF Version".to_string(),
            expected: reference.version.clone(),
            actual: generated.version.clone(),
            severity,
        });
    }

    // Compare catalogs
    differences.extend(compare_catalogs(&generated.catalog, &reference.catalog));

    // Compare page trees
    differences.extend(compare_page_trees(
        &generated.page_tree,
        &reference.page_tree,
    ));

    // Compare fonts
    differences.extend(compare_fonts(&generated.fonts, &reference.fonts));

    // Compare color spaces
    differences.extend(compare_color_spaces(generated, reference));

    // Compare graphics states
    differences.extend(compare_graphics_states(
        &generated.graphics_states,
        &reference.graphics_states,
    ));

    // Compare text objects
    differences.extend(compare_text_objects(
        &generated.text_objects,
        &reference.text_objects,
    ));

    // Compare annotations
    differences.extend(compare_annotations(
        &generated.annotations,
        &reference.annotations,
    ));

    // Compare cross-reference validity
    if generated.xref_valid != reference.xref_valid {
        differences.push(PdfDifference {
            location: "Cross-reference table".to_string(),
            expected: reference.xref_valid.to_string(),
            actual: generated.xref_valid.to_string(),
            severity: DifferenceSeverity::Critical,
        });
    }

    differences
}

/// Compare document catalogs
fn compare_catalogs(
    generated: &Option<HashMap<String, String>>,
    reference: &Option<HashMap<String, String>>,
) -> Vec<PdfDifference> {
    let mut differences = Vec::new();

    match (generated, reference) {
        (Some(gen_catalog), Some(ref_catalog)) => {
            // Check required entries
            for key in ["Type", "Pages"] {
                match (gen_catalog.get(key), ref_catalog.get(key)) {
                    (Some(gen_val), Some(ref_val)) => {
                        if gen_val != ref_val {
                            differences.push(PdfDifference {
                                location: format!("Catalog/{}", key),
                                expected: ref_val.clone(),
                                actual: gen_val.clone(),
                                severity: DifferenceSeverity::Critical,
                            });
                        }
                    }
                    (None, Some(ref_val)) => {
                        differences.push(PdfDifference {
                            location: format!("Catalog/{}", key),
                            expected: ref_val.clone(),
                            actual: "missing".to_string(),
                            severity: DifferenceSeverity::Critical,
                        });
                    }
                    (Some(gen_val), None) => {
                        differences.push(PdfDifference {
                            location: format!("Catalog/{}", key),
                            expected: "missing".to_string(),
                            actual: gen_val.clone(),
                            severity: DifferenceSeverity::Minor,
                        });
                    }
                    (None, None) => {} // Both missing - check if required
                }
            }
        }
        (None, Some(_)) => {
            differences.push(PdfDifference {
                location: "Document Catalog".to_string(),
                expected: "present".to_string(),
                actual: "missing".to_string(),
                severity: DifferenceSeverity::Critical,
            });
        }
        (Some(_), None) => {
            differences.push(PdfDifference {
                location: "Document Catalog".to_string(),
                expected: "missing".to_string(),
                actual: "present".to_string(),
                severity: DifferenceSeverity::Minor,
            });
        }
        (None, None) => {
            differences.push(PdfDifference {
                location: "Document Catalog".to_string(),
                expected: "present".to_string(),
                actual: "missing".to_string(),
                severity: DifferenceSeverity::Critical,
            });
        }
    }

    differences
}

/// Compare page trees
fn compare_page_trees(
    generated: &Option<crate::verification::parser::PageTree>,
    reference: &Option<crate::verification::parser::PageTree>,
) -> Vec<PdfDifference> {
    let mut differences = Vec::new();

    match (generated, reference) {
        (Some(gen_tree), Some(ref_tree)) => {
            if gen_tree.page_count != ref_tree.page_count {
                differences.push(PdfDifference {
                    location: "Page Tree/Count".to_string(),
                    expected: ref_tree.page_count.to_string(),
                    actual: gen_tree.page_count.to_string(),
                    severity: DifferenceSeverity::Critical,
                });
            }

            if gen_tree.root_type != ref_tree.root_type {
                differences.push(PdfDifference {
                    location: "Page Tree/Type".to_string(),
                    expected: ref_tree.root_type.clone(),
                    actual: gen_tree.root_type.clone(),
                    severity: DifferenceSeverity::Critical,
                });
            }
        }
        (None, Some(_)) => {
            differences.push(PdfDifference {
                location: "Page Tree".to_string(),
                expected: "present".to_string(),
                actual: "missing".to_string(),
                severity: DifferenceSeverity::Critical,
            });
        }
        (Some(_), None) => {
            differences.push(PdfDifference {
                location: "Page Tree".to_string(),
                expected: "missing".to_string(),
                actual: "present".to_string(),
                severity: DifferenceSeverity::Minor,
            });
        }
        (None, None) => {} // Both missing - may be ok for minimal PDFs
    }

    differences
}

/// Compare font lists
fn compare_fonts(generated: &[String], reference: &[String]) -> Vec<PdfDifference> {
    let mut differences = Vec::new();

    // Check for missing fonts
    for ref_font in reference {
        if !generated.contains(ref_font) {
            differences.push(PdfDifference {
                location: format!("Fonts/{}", ref_font),
                expected: "present".to_string(),
                actual: "missing".to_string(),
                severity: DifferenceSeverity::Important,
            });
        }
    }

    // Check for extra fonts (usually not a problem)
    for gen_font in generated {
        if !reference.contains(gen_font) {
            differences.push(PdfDifference {
                location: format!("Fonts/{}", gen_font),
                expected: "missing".to_string(),
                actual: "present".to_string(),
                severity: DifferenceSeverity::Minor,
            });
        }
    }

    differences
}

/// Compare color space usage
fn compare_color_spaces(generated: &ParsedPdf, reference: &ParsedPdf) -> Vec<PdfDifference> {
    let mut differences = Vec::new();

    if generated.uses_device_rgb != reference.uses_device_rgb {
        differences.push(PdfDifference {
            location: "Color Spaces/DeviceRGB".to_string(),
            expected: reference.uses_device_rgb.to_string(),
            actual: generated.uses_device_rgb.to_string(),
            severity: DifferenceSeverity::Important,
        });
    }

    if generated.uses_device_cmyk != reference.uses_device_cmyk {
        differences.push(PdfDifference {
            location: "Color Spaces/DeviceCMYK".to_string(),
            expected: reference.uses_device_cmyk.to_string(),
            actual: generated.uses_device_cmyk.to_string(),
            severity: DifferenceSeverity::Important,
        });
    }

    if generated.uses_device_gray != reference.uses_device_gray {
        differences.push(PdfDifference {
            location: "Color Spaces/DeviceGray".to_string(),
            expected: reference.uses_device_gray.to_string(),
            actual: generated.uses_device_gray.to_string(),
            severity: DifferenceSeverity::Important,
        });
    }

    differences
}

/// Compare graphics states
fn compare_graphics_states(
    generated: &[crate::verification::parser::GraphicsState],
    reference: &[crate::verification::parser::GraphicsState],
) -> Vec<PdfDifference> {
    let mut differences = Vec::new();

    if generated.len() != reference.len() {
        differences.push(PdfDifference {
            location: "Graphics States/Count".to_string(),
            expected: reference.len().to_string(),
            actual: generated.len().to_string(),
            severity: DifferenceSeverity::Important,
        });
    }

    // Compare first few graphics states (detailed comparison would be complex)
    let min_len = generated.len().min(reference.len());
    for i in 0..min_len.min(3) {
        // Only compare first 3 for performance
        let gen_state = &generated[i];
        let ref_state = &reference[i];

        if gen_state.line_width != ref_state.line_width {
            differences.push(PdfDifference {
                location: format!("Graphics State {}/LineWidth", i),
                expected: format!("{:?}", ref_state.line_width),
                actual: format!("{:?}", gen_state.line_width),
                severity: DifferenceSeverity::Minor,
            });
        }
    }

    differences
}

/// Compare text objects
fn compare_text_objects(
    generated: &[crate::verification::parser::TextObject],
    reference: &[crate::verification::parser::TextObject],
) -> Vec<PdfDifference> {
    let mut differences = Vec::new();

    if generated.len() != reference.len() {
        differences.push(PdfDifference {
            location: "Text Objects/Count".to_string(),
            expected: reference.len().to_string(),
            actual: generated.len().to_string(),
            severity: DifferenceSeverity::Important,
        });
    }

    // Compare text content (simplified)
    let min_len = generated.len().min(reference.len());
    for i in 0..min_len {
        let gen_text = &generated[i];
        let ref_text = &reference[i];

        if gen_text.text_content != ref_text.text_content {
            differences.push(PdfDifference {
                location: format!("Text Object {}/Content", i),
                expected: ref_text.text_content.clone(),
                actual: gen_text.text_content.clone(),
                severity: DifferenceSeverity::Important,
            });
        }
    }

    differences
}

/// Compare annotations
fn compare_annotations(
    generated: &[crate::verification::parser::Annotation],
    reference: &[crate::verification::parser::Annotation],
) -> Vec<PdfDifference> {
    let mut differences = Vec::new();

    if generated.len() != reference.len() {
        differences.push(PdfDifference {
            location: "Annotations/Count".to_string(),
            expected: reference.len().to_string(),
            actual: generated.len().to_string(),
            severity: DifferenceSeverity::Important,
        });
    }

    differences
}

/// Calculate similarity score based on differences
fn calculate_similarity_score(differences: &[PdfDifference]) -> f64 {
    if differences.is_empty() {
        return 1.0;
    }

    let mut penalty = 0.0;
    for diff in differences {
        penalty += match diff.severity {
            DifferenceSeverity::Critical => 0.3,
            DifferenceSeverity::Important => 0.1,
            DifferenceSeverity::Minor => 0.05,
            DifferenceSeverity::Cosmetic => 0.01,
        };
    }

    (1.0f64 - penalty).max(0.0)
}

/// Check if two PDFs are structurally equivalent for ISO compliance
pub fn pdfs_structurally_equivalent(generated: &[u8], reference: &[u8]) -> bool {
    match compare_pdfs(generated, reference) {
        Ok(result) => result.structurally_equivalent,
        Err(_) => false,
    }
}

/// Extract structural differences between PDFs
pub fn extract_pdf_differences(generated: &[u8], reference: &[u8]) -> Result<Vec<PdfDifference>> {
    let result = compare_pdfs(generated, reference)?;
    Ok(result.differences)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_pdf(version: &str, catalog_type: &str) -> Vec<u8> {
        format!(
            "%PDF-{}\n1 0 obj\n<<\n/Type /{}\n>>\nendobj\n%%EOF",
            version, catalog_type
        )
        .into_bytes()
    }

    #[test]
    fn test_identical_pdfs() {
        let pdf1 = create_test_pdf("1.4", "Catalog");
        let pdf2 = create_test_pdf("1.4", "Catalog");

        let result = compare_pdfs(&pdf1, &pdf2).unwrap();
        assert!(result.content_equivalent);
        assert_eq!(result.similarity_score, 1.0);
    }

    #[test]
    fn test_version_difference() {
        let pdf1 = create_test_pdf("1.4", "Catalog");
        let pdf2 = create_test_pdf("1.7", "Catalog");

        let result = compare_pdfs(&pdf1, &pdf2).unwrap();
        assert!(!result.content_equivalent);
        assert!(result.similarity_score < 1.0);
        assert!(result
            .differences
            .iter()
            .any(|d| d.location == "PDF Version"));
    }

    #[test]
    fn test_structural_difference() {
        let pdf1 = create_test_pdf("1.4", "Catalog");
        let pdf2 = create_test_pdf("1.4", "Document"); // Wrong type

        let result = compare_pdfs(&pdf1, &pdf2).unwrap();
        assert!(!result.structurally_equivalent);
        assert!(result
            .differences
            .iter()
            .any(|d| d.location.contains("Catalog") && d.severity == DifferenceSeverity::Critical));
    }

    #[test]
    fn test_calculate_similarity_score() {
        let differences = vec![PdfDifference {
            location: "test".to_string(),
            expected: "a".to_string(),
            actual: "b".to_string(),
            severity: DifferenceSeverity::Critical,
        }];

        let score = calculate_similarity_score(&differences);
        assert_eq!(score, 0.7); // 1.0 - 0.3 (critical penalty)
    }
}
