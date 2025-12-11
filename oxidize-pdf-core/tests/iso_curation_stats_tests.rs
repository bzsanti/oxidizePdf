//! ISO Curation Statistics Tests - Phase 1.4 (TDD Red Phase)
//!
//! These tests define the TARGET METRICS for the curated matrix.
//! All tests should FAIL initially (because the curated matrix doesn't exist).
//!
//! Target metrics:
//! - 200-500 curated requirements (vs 7,775 original)
//! - All requirements have priority (P0-P3)
//! - All requirements have feature area
//! - P0 requirements: ~10-20% of total
//! - >70% of requirements mapped to code
//! - Reduction ratio: >90%

use std::path::Path;

/// Structures for loading the curated matrix (to be implemented in Phase 4)
mod curated_matrix {
    use serde::Deserialize;

    #[derive(Debug, Clone, Deserialize)]
    pub struct CuratedMatrix {
        pub metadata: CuratedMetadata,
        pub requirements: Vec<CuratedRequirement>,
    }

    #[derive(Debug, Clone, Deserialize)]
    pub struct CuratedMetadata {
        pub version: String,
        pub curation_date: String,
        pub original_count: usize,
        pub curated_count: usize,
        pub reduction_ratio: f64,
    }

    #[derive(Debug, Clone, Deserialize)]
    pub struct CuratedRequirement {
        pub id: String,
        pub name: String,
        pub description: String,
        pub iso_section: String,
        pub requirement_type: String,
        pub priority: String,
        pub feature_area: String,
        pub implemented: bool,
        #[serde(default)]
        pub implementation_refs: Vec<String>,
        #[serde(default)]
        pub test_refs: Vec<String>,
        pub verification_level: u8,
        #[serde(default)]
        pub consolidates: Vec<String>,
    }

    /// Attempts to load the curated matrix from disk
    pub fn load_curated_matrix() -> Result<CuratedMatrix, String> {
        let path = "../../ISO_COMPLIANCE_MATRIX_CURATED.toml";

        if !Path::new(path).exists() {
            return Err(format!(
                "Curated matrix not found at '{}'. Run Phase 3 (manual curation) first.",
                path
            ));
        }

        // STUB: Always fails (RED phase)
        // Will be implemented in Phase 4
        Err("Not implemented - Phase 4".to_string())
    }

    /// Loads the original matrix to compare sizes
    pub fn get_original_count() -> usize {
        7775 // Known count from ISO_COMPLIANCE_MATRIX.toml
    }

    use std::path::Path;
}

// =============================================================================
// TEST GROUP 1: Matrix Size and Reduction
// =============================================================================

#[test]
fn test_curated_matrix_exists() {
    let path = Path::new("../../ISO_COMPLIANCE_MATRIX_CURATED.toml");

    assert!(
        path.exists(),
        "Curated matrix must exist at ISO_COMPLIANCE_MATRIX_CURATED.toml"
    );
}

#[test]
fn test_curated_matrix_loads_successfully() {
    let result = curated_matrix::load_curated_matrix();

    assert!(
        result.is_ok(),
        "Curated matrix should load without errors: {:?}",
        result.err()
    );
}

#[test]
fn test_curated_matrix_size_within_target_range() {
    let matrix = curated_matrix::load_curated_matrix().expect("Failed to load curated matrix");

    let count = matrix.requirements.len();

    assert!(
        count >= 200,
        "Curated matrix should have at least 200 requirements (got {})",
        count
    );
    assert!(
        count <= 500,
        "Curated matrix should have at most 500 requirements (got {})",
        count
    );
}

#[test]
fn test_reduction_ratio_exceeds_90_percent() {
    let matrix = curated_matrix::load_curated_matrix().expect("Failed to load curated matrix");

    let original_count = curated_matrix::get_original_count();
    let curated_count = matrix.requirements.len();
    let reduction_ratio = 1.0 - (curated_count as f64 / original_count as f64);

    assert!(
        reduction_ratio >= 0.90,
        "Reduction ratio should be >= 90% (got {:.1}%: {} -> {})",
        reduction_ratio * 100.0,
        original_count,
        curated_count
    );
}

// =============================================================================
// TEST GROUP 2: Required Fields
// =============================================================================

#[test]
fn test_all_requirements_have_priority() {
    let matrix = curated_matrix::load_curated_matrix().expect("Failed to load curated matrix");

    let valid_priorities = ["P0", "P1", "P2", "P3"];

    for req in &matrix.requirements {
        assert!(
            valid_priorities.contains(&req.priority.as_str()),
            "Requirement '{}' has invalid priority '{}' (must be P0-P3)",
            req.id,
            req.priority
        );
    }
}

#[test]
fn test_all_requirements_have_feature_area() {
    let matrix = curated_matrix::load_curated_matrix().expect("Failed to load curated matrix");

    let valid_areas = [
        "parser",
        "writer",
        "graphics",
        "fonts",
        "text",
        "content",
        "encryption",
        "metadata",
        "interactive",
        "advanced",
    ];

    for req in &matrix.requirements {
        assert!(
            valid_areas.contains(&req.feature_area.as_str()),
            "Requirement '{}' has invalid feature_area '{}' (must be one of: {:?})",
            req.id,
            req.feature_area,
            valid_areas
        );
    }
}

#[test]
fn test_all_requirements_have_requirement_type() {
    let matrix = curated_matrix::load_curated_matrix().expect("Failed to load curated matrix");

    let valid_types = ["mandatory", "recommended", "optional"];

    for req in &matrix.requirements {
        assert!(
            valid_types.contains(&req.requirement_type.as_str()),
            "Requirement '{}' has invalid requirement_type '{}'",
            req.id,
            req.requirement_type
        );
    }
}

#[test]
fn test_all_requirements_have_semantic_id() {
    let matrix = curated_matrix::load_curated_matrix().expect("Failed to load curated matrix");

    for req in &matrix.requirements {
        // Semantic ID should be like "7.3.5-stream-length" not just "7.110"
        assert!(
            req.id.contains('-'),
            "Requirement '{}' should have semantic ID format (e.g., '7.3.5-stream-length')",
            req.id
        );

        // Should start with section number
        assert!(
            req.id.chars().next().map_or(false, |c| c.is_ascii_digit()),
            "Requirement ID '{}' should start with section number",
            req.id
        );
    }
}

#[test]
fn test_all_requirements_have_non_empty_description() {
    let matrix = curated_matrix::load_curated_matrix().expect("Failed to load curated matrix");

    for req in &matrix.requirements {
        assert!(
            !req.description.trim().is_empty(),
            "Requirement '{}' has empty description",
            req.id
        );

        assert!(
            req.description.len() >= 50,
            "Requirement '{}' description too short ({} chars, min 50)",
            req.id,
            req.description.len()
        );
    }
}

// =============================================================================
// TEST GROUP 3: Priority Distribution
// =============================================================================

#[test]
fn test_p0_requirements_between_10_and_20_percent() {
    let matrix = curated_matrix::load_curated_matrix().expect("Failed to load curated matrix");

    let total = matrix.requirements.len();
    let p0_count = matrix
        .requirements
        .iter()
        .filter(|r| r.priority == "P0")
        .count();

    let p0_percentage = (p0_count as f64 / total as f64) * 100.0;

    assert!(
        p0_percentage >= 10.0,
        "P0 (Critical) should be >= 10% of total (got {:.1}%: {}/{})",
        p0_percentage,
        p0_count,
        total
    );
    assert!(
        p0_percentage <= 20.0,
        "P0 (Critical) should be <= 20% of total (got {:.1}%: {}/{})",
        p0_percentage,
        p0_count,
        total
    );
}

#[test]
fn test_p1_requirements_between_25_and_40_percent() {
    let matrix = curated_matrix::load_curated_matrix().expect("Failed to load curated matrix");

    let total = matrix.requirements.len();
    let p1_count = matrix
        .requirements
        .iter()
        .filter(|r| r.priority == "P1")
        .count();

    let p1_percentage = (p1_count as f64 / total as f64) * 100.0;

    assert!(
        p1_percentage >= 25.0 && p1_percentage <= 40.0,
        "P1 (High) should be 25-40% of total (got {:.1}%: {}/{})",
        p1_percentage,
        p1_count,
        total
    );
}

#[test]
fn test_p3_requirements_under_15_percent() {
    let matrix = curated_matrix::load_curated_matrix().expect("Failed to load curated matrix");

    let total = matrix.requirements.len();
    let p3_count = matrix
        .requirements
        .iter()
        .filter(|r| r.priority == "P3")
        .count();

    let p3_percentage = (p3_count as f64 / total as f64) * 100.0;

    assert!(
        p3_percentage <= 15.0,
        "P3 (Low) should be <= 15% of total (got {:.1}%: {}/{})",
        p3_percentage,
        p3_count,
        total
    );
}

// =============================================================================
// TEST GROUP 4: Feature Area Coverage
// =============================================================================

#[test]
fn test_core_feature_areas_represented() {
    let matrix = curated_matrix::load_curated_matrix().expect("Failed to load curated matrix");

    let core_areas = ["parser", "writer", "graphics", "fonts", "text"];

    for area in core_areas {
        let count = matrix
            .requirements
            .iter()
            .filter(|r| r.feature_area == area)
            .count();

        assert!(
            count > 0,
            "Core feature area '{}' should have at least one requirement",
            area
        );
    }
}

#[test]
fn test_parser_has_most_requirements() {
    let matrix = curated_matrix::load_curated_matrix().expect("Failed to load curated matrix");

    let parser_count = matrix
        .requirements
        .iter()
        .filter(|r| r.feature_area == "parser")
        .count();

    let total = matrix.requirements.len();
    let parser_percentage = (parser_count as f64 / total as f64) * 100.0;

    assert!(
        parser_percentage >= 20.0,
        "Parser should have >= 20% of requirements (got {:.1}%)",
        parser_percentage
    );
}

// =============================================================================
// TEST GROUP 5: Implementation Mapping
// =============================================================================

#[test]
fn test_majority_requirements_mapped_to_code() {
    let matrix = curated_matrix::load_curated_matrix().expect("Failed to load curated matrix");

    let total = matrix.requirements.len();
    let mapped_count = matrix
        .requirements
        .iter()
        .filter(|r| !r.implementation_refs.is_empty())
        .count();

    let mapped_percentage = (mapped_count as f64 / total as f64) * 100.0;

    assert!(
        mapped_percentage >= 70.0,
        "At least 70% of requirements should be mapped to code (got {:.1}%: {}/{})",
        mapped_percentage,
        mapped_count,
        total
    );
}

#[test]
fn test_p0_requirements_all_implemented() {
    let matrix = curated_matrix::load_curated_matrix().expect("Failed to load curated matrix");

    let p0_requirements: Vec<_> = matrix
        .requirements
        .iter()
        .filter(|r| r.priority == "P0")
        .collect();

    for req in &p0_requirements {
        assert!(
            req.implemented,
            "P0 (Critical) requirement '{}' must be implemented",
            req.id
        );
        assert!(
            !req.implementation_refs.is_empty(),
            "P0 requirement '{}' must have implementation references",
            req.id
        );
    }
}

#[test]
fn test_implemented_requirements_have_verification_level() {
    let matrix = curated_matrix::load_curated_matrix().expect("Failed to load curated matrix");

    for req in &matrix.requirements {
        if req.implemented {
            assert!(
                req.verification_level >= 2,
                "Implemented requirement '{}' should have verification_level >= 2 (got {})",
                req.id,
                req.verification_level
            );
        }
    }
}

// =============================================================================
// TEST GROUP 6: Consolidation Tracking
// =============================================================================

#[test]
fn test_consolidation_tracking() {
    let matrix = curated_matrix::load_curated_matrix().expect("Failed to load curated matrix");

    // Some requirements should have consolidates field (merged from multiple fragments)
    let consolidated_count = matrix
        .requirements
        .iter()
        .filter(|r| r.consolidates.len() > 1)
        .count();

    assert!(
        consolidated_count > 0,
        "Some requirements should be consolidated from multiple fragments"
    );
}

#[test]
fn test_total_original_fragments_tracked() {
    let matrix = curated_matrix::load_curated_matrix().expect("Failed to load curated matrix");

    // Sum of all consolidated fragments should be significant
    let total_fragments: usize = matrix
        .requirements
        .iter()
        .map(|r| {
            if r.consolidates.is_empty() {
                1
            } else {
                r.consolidates.len()
            }
        })
        .sum();

    // Should represent a meaningful portion of original 7775
    assert!(
        total_fragments >= 500,
        "Total tracked fragments should be >= 500 (got {})",
        total_fragments
    );
}

// =============================================================================
// TEST GROUP 7: Metadata Validation
// =============================================================================

#[test]
fn test_metadata_contains_curation_info() {
    let matrix = curated_matrix::load_curated_matrix().expect("Failed to load curated matrix");

    assert!(
        !matrix.metadata.version.is_empty(),
        "Metadata should have version"
    );
    assert!(
        !matrix.metadata.curation_date.is_empty(),
        "Metadata should have curation_date"
    );
    assert_eq!(
        matrix.metadata.original_count, 7775,
        "Metadata should record original count"
    );
    assert_eq!(
        matrix.metadata.curated_count,
        matrix.requirements.len(),
        "Metadata curated_count should match actual requirements"
    );
}

// =============================================================================
// TEST SUMMARY - Expected Results
// =============================================================================
// Total tests: 21
// Expected FAILING: 21 (curated matrix doesn't exist yet)
// After Phase 3+4: 21 PASSING (GREEN phase)
