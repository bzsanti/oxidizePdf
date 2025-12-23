//! ISO Curation Statistics Tests - Phase 4 (TDD Green Phase)
//!
//! These tests verify the curated matrix meets target metrics.
//! Uses the CuratedIsoMatrix from verification module.
//!
//! Target metrics:
//! - 200-500 curated requirements (vs 7,775 original)
//! - All requirements have priority (P0-P3)
//! - All requirements have feature area
//! - Reduction ratio: >90%

use oxidize_pdf::verification::curated_matrix::{CuratedIsoMatrix, Priority};
use std::path::Path;

// =============================================================================
// TEST GROUP 1: Matrix Size and Reduction
// =============================================================================

#[test]
fn test_curated_matrix_exists() {
    // Try multiple paths (test runs from different directories)
    let paths = [
        "ISO_COMPLIANCE_MATRIX_CURATED.toml",
        "../ISO_COMPLIANCE_MATRIX_CURATED.toml",
        "../../ISO_COMPLIANCE_MATRIX_CURATED.toml",
    ];

    let exists = paths.iter().any(|p| Path::new(p).exists());

    assert!(
        exists,
        "Curated matrix must exist at ISO_COMPLIANCE_MATRIX_CURATED.toml"
    );
}

#[test]
fn test_curated_matrix_loads_successfully() {
    let result = CuratedIsoMatrix::load_default();

    assert!(
        result.is_ok(),
        "Curated matrix should load without errors: {:?}",
        result.err()
    );
}

#[test]
fn test_curated_matrix_size_within_target_range() {
    let matrix = CuratedIsoMatrix::load_default().expect("Failed to load curated matrix");

    let count = matrix.total_count();

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
    let matrix = CuratedIsoMatrix::load_default().expect("Failed to load curated matrix");

    let original_count = 7775usize; // Known count from ISO_COMPLIANCE_MATRIX.toml
    let curated_count = matrix.total_count();
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
    let matrix = CuratedIsoMatrix::load_default().expect("Failed to load curated matrix");

    let valid_priorities = ["P0", "P1", "P2", "P3"];

    for req in matrix.all_requirements() {
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
    let matrix = CuratedIsoMatrix::load_default().expect("Failed to load curated matrix");

    // Note: both "metadata" and "doc_metadata" are valid
    // (doc_metadata is renamed from metadata in TOML section names)
    let valid_areas = [
        "parser",
        "writer",
        "graphics",
        "fonts",
        "text",
        "content",
        "encryption",
        "metadata",
        "doc_metadata",
        "interactive",
        "advanced",
    ];

    for req in matrix.all_requirements() {
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
    let matrix = CuratedIsoMatrix::load_default().expect("Failed to load curated matrix");

    let valid_types = ["mandatory", "recommendation", "recommended", "optional"];

    for req in matrix.all_requirements() {
        assert!(
            valid_types.contains(&req.requirement_type.as_str()),
            "Requirement '{}' has invalid requirement_type '{}'",
            req.id,
            req.requirement_type
        );
    }
}

#[test]
fn test_all_requirements_have_non_empty_description() {
    let matrix = CuratedIsoMatrix::load_default().expect("Failed to load curated matrix");

    for req in matrix.all_requirements() {
        assert!(
            !req.description.trim().is_empty(),
            "Requirement '{}' has empty description",
            req.id
        );
    }
}

// =============================================================================
// TEST GROUP 3: Priority Distribution
// =============================================================================

#[test]
fn test_priority_distribution_reasonable() {
    let matrix = CuratedIsoMatrix::load_default().expect("Failed to load curated matrix");

    let stats = matrix.calculate_stats();
    let total = stats.total_requirements;

    // P0 should exist (critical requirements)
    let p0_count = stats.priority_count("P0");
    assert!(p0_count > 0, "Should have some P0 (Critical) requirements");

    // P1 should exist (high priority)
    let p1_count = stats.priority_count("P1");
    assert!(p1_count > 0, "Should have some P1 (High) requirements");

    // P2 should be the largest group (medium priority)
    let p2_count = stats.priority_count("P2");
    let p2_percentage = (p2_count as f64 / total as f64) * 100.0;
    assert!(
        p2_percentage >= 50.0,
        "P2 (Medium) should be >= 50% of total (got {:.1}%)",
        p2_percentage
    );
}

// =============================================================================
// TEST GROUP 4: Feature Area Coverage
// =============================================================================

#[test]
fn test_multiple_feature_areas_represented() {
    let matrix = CuratedIsoMatrix::load_default().expect("Failed to load curated matrix");

    let areas = matrix.feature_areas();

    assert!(
        areas.len() >= 3,
        "Should have at least 3 feature areas (got {})",
        areas.len()
    );
}

// =============================================================================
// TEST GROUP 5: API Tests
// =============================================================================

#[test]
fn test_by_priority_api() {
    let matrix = CuratedIsoMatrix::load_default().expect("Failed to load curated matrix");

    let critical = matrix.by_priority(Priority::Critical);
    let high = matrix.by_priority(Priority::High);

    // All critical requirements should have priority P0
    for req in &critical {
        assert_eq!(req.priority, "P0");
    }

    // All high priority should have P1
    for req in &high {
        assert_eq!(req.priority, "P1");
    }
}

#[test]
fn test_mandatory_requirements_api() {
    let matrix = CuratedIsoMatrix::load_default().expect("Failed to load curated matrix");

    let mandatory = matrix.mandatory_requirements();

    // All mandatory requirements should have requirement_type = mandatory
    for req in &mandatory {
        assert_eq!(req.requirement_type, "mandatory");
    }
}

#[test]
fn test_by_section_api() {
    let matrix = CuratedIsoMatrix::load_default().expect("Failed to load curated matrix");

    let section_7 = matrix.by_section("7");
    let section_8 = matrix.by_section("8");

    // Section 7 and 8 should have requirements
    assert!(!section_7.is_empty(), "Section 7 should have requirements");
    assert!(!section_8.is_empty(), "Section 8 should have requirements");
}

// =============================================================================
// TEST GROUP 6: Metadata Validation
// =============================================================================

#[test]
fn test_metadata_contains_curation_info() {
    let matrix = CuratedIsoMatrix::load_default().expect("Failed to load curated matrix");

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
        matrix.metadata.curated_count as usize,
        matrix.total_count(),
        "Metadata curated_count should match actual requirements"
    );
}

#[test]
fn test_reduction_ratio_in_metadata() {
    let matrix = CuratedIsoMatrix::load_default().expect("Failed to load curated matrix");

    assert!(
        matrix.metadata.reduction_ratio > 0.90,
        "Metadata reduction_ratio should be > 90% (got {:.1}%)",
        matrix.metadata.reduction_ratio * 100.0
    );
}
