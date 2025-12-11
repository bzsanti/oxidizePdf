//! Curated ISO Compliance Matrix Processing
//!
//! This module loads the curated ISO compliance matrix (ISO_COMPLIANCE_MATRIX_CURATED.toml)
//! which contains 310 verified requirements instead of 7,775 text fragments.
//!
//! Key differences from original matrix:
//! - Only contains valid, verifiable requirements
//! - Priority-based classification (P0/P1/P2/P3)
//! - Feature area grouping (parser, writer, graphics, fonts, etc.)
//! - Requirement type classification (mandatory/optional/recommendation)

use crate::error::{PdfError, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// Curated ISO compliance matrix loaded from TOML
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CuratedIsoMatrix {
    /// Metadata about the curation process
    pub metadata: CuratedMetadata,
    /// Requirements grouped by feature area
    #[serde(flatten)]
    pub areas: HashMap<String, FeatureArea>,
}

/// Metadata about the curated matrix
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CuratedMetadata {
    /// Version of the curated matrix
    pub version: String,
    /// Date of curation
    pub curation_date: String,
    /// Original fragment count (7775)
    pub original_count: u32,
    /// Curated requirement count
    pub curated_count: u32,
    /// Reduction ratio (0.96 = 96% reduction)
    pub reduction_ratio: f64,
}

/// Feature area containing requirements
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct FeatureArea {
    /// Requirements in this feature area
    pub requirements: Vec<CuratedRequirement>,
}

/// A curated ISO requirement
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CuratedRequirement {
    /// Unique requirement ID
    pub id: String,
    /// Human-readable name
    pub name: String,
    /// Full requirement description
    pub description: String,
    /// ISO 32000-1:2008 section reference
    pub iso_section: String,
    /// Type: mandatory, optional, recommendation
    pub requirement_type: String,
    /// Priority: P0 (critical), P1 (high), P2 (medium), P3 (low)
    pub priority: String,
    /// Feature area: parser, writer, graphics, fonts, text, content, encryption, etc.
    pub feature_area: String,
    /// Whether this requirement is implemented
    #[serde(default)]
    pub implemented: bool,
    /// References to implementation code
    #[serde(default)]
    pub implementation_refs: Vec<String>,
    /// References to test files
    #[serde(default)]
    pub test_refs: Vec<String>,
    /// Verification level (0-4)
    #[serde(default)]
    pub verification_level: u8,
    /// IDs of original fragments that were consolidated
    #[serde(default)]
    pub consolidates: Vec<String>,
}

/// Priority level for requirements
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Priority {
    /// P0: Critical - Must be implemented for basic PDF validity
    Critical = 0,
    /// P1: High - Important for common use cases
    High = 1,
    /// P2: Medium - Standard features
    Medium = 2,
    /// P3: Low - Advanced/optional features
    Low = 3,
}

impl Priority {
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "P0" => Some(Priority::Critical),
            "P1" => Some(Priority::High),
            "P2" => Some(Priority::Medium),
            "P3" => Some(Priority::Low),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Priority::Critical => "P0",
            Priority::High => "P1",
            Priority::Medium => "P2",
            Priority::Low => "P3",
        }
    }
}

impl CuratedIsoMatrix {
    /// Load curated matrix from TOML file
    pub fn load(path: &Path) -> Result<Self> {
        let content = fs::read_to_string(path).map_err(PdfError::Io)?;
        toml::from_str(&content)
            .map_err(|e| PdfError::ParseError(format!("Failed to parse curated matrix: {}", e)))
    }

    /// Load default curated matrix from project root
    pub fn load_default() -> Result<Self> {
        let potential_paths = [
            "../ISO_COMPLIANCE_MATRIX_CURATED.toml",
            "../../ISO_COMPLIANCE_MATRIX_CURATED.toml",
            "ISO_COMPLIANCE_MATRIX_CURATED.toml",
            "./ISO_COMPLIANCE_MATRIX_CURATED.toml",
        ];

        for path in &potential_paths {
            let path = Path::new(path);
            if path.exists() {
                return Self::load(path);
            }
        }

        Err(PdfError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "ISO_COMPLIANCE_MATRIX_CURATED.toml not found in any expected location",
        )))
    }

    /// Get all requirements as a flat list
    pub fn all_requirements(&self) -> Vec<&CuratedRequirement> {
        self.areas
            .values()
            .flat_map(|area| &area.requirements)
            .collect()
    }

    /// Get total requirement count
    pub fn total_count(&self) -> usize {
        self.areas
            .values()
            .map(|area| area.requirements.len())
            .sum()
    }

    /// Get requirements by priority
    pub fn by_priority(&self, priority: Priority) -> Vec<&CuratedRequirement> {
        let priority_str = priority.as_str();
        self.all_requirements()
            .into_iter()
            .filter(|req| req.priority == priority_str)
            .collect()
    }

    /// Get critical requirements (P0) - must be implemented for basic PDF validity
    pub fn critical_requirements(&self) -> Vec<&CuratedRequirement> {
        self.by_priority(Priority::Critical)
    }

    /// Get high priority requirements (P1) - important for common use cases
    pub fn high_priority_requirements(&self) -> Vec<&CuratedRequirement> {
        self.by_priority(Priority::High)
    }

    /// Get requirements by feature area
    pub fn by_feature_area(&self, area: &str) -> Vec<&CuratedRequirement> {
        // Handle the doc_metadata -> metadata mapping
        let area_key = if area == "metadata" {
            "doc_metadata"
        } else {
            area
        };

        self.areas
            .get(area_key)
            .map(|a| a.requirements.iter().collect())
            .unwrap_or_default()
    }

    /// Get requirements by type (mandatory/optional/recommendation)
    pub fn by_type(&self, req_type: &str) -> Vec<&CuratedRequirement> {
        self.all_requirements()
            .into_iter()
            .filter(|req| req.requirement_type == req_type)
            .collect()
    }

    /// Get mandatory requirements
    pub fn mandatory_requirements(&self) -> Vec<&CuratedRequirement> {
        self.by_type("mandatory")
    }

    /// Get unimplemented requirements
    pub fn unimplemented(&self) -> Vec<&CuratedRequirement> {
        self.all_requirements()
            .into_iter()
            .filter(|req| !req.implemented)
            .collect()
    }

    /// Get requirements by ISO section prefix (e.g., "7" for Section 7)
    pub fn by_section(&self, section_prefix: &str) -> Vec<&CuratedRequirement> {
        self.all_requirements()
            .into_iter()
            .filter(|req| req.iso_section.starts_with(section_prefix))
            .collect()
    }

    /// Get requirement by ID
    pub fn get(&self, id: &str) -> Option<&CuratedRequirement> {
        self.all_requirements().into_iter().find(|req| req.id == id)
    }

    /// Calculate compliance statistics
    pub fn calculate_stats(&self) -> CuratedComplianceStats {
        let all = self.all_requirements();
        let total = all.len();
        let implemented = all.iter().filter(|r| r.implemented).count();

        let mut by_priority = HashMap::new();
        let mut by_area = HashMap::new();
        let mut by_type = HashMap::new();

        for req in &all {
            *by_priority.entry(req.priority.clone()).or_insert(0) += 1;
            *by_area.entry(req.feature_area.clone()).or_insert(0) += 1;
            *by_type.entry(req.requirement_type.clone()).or_insert(0) += 1;
        }

        CuratedComplianceStats {
            total_requirements: total,
            implemented_count: implemented,
            implementation_percentage: if total > 0 {
                (implemented as f64 / total as f64) * 100.0
            } else {
                0.0
            },
            by_priority,
            by_feature_area: by_area,
            by_type,
        }
    }

    /// Get all feature area names
    pub fn feature_areas(&self) -> Vec<&str> {
        self.areas.keys().map(|s| s.as_str()).collect()
    }
}

/// Statistics for curated matrix compliance
#[derive(Debug, Clone)]
pub struct CuratedComplianceStats {
    pub total_requirements: usize,
    pub implemented_count: usize,
    pub implementation_percentage: f64,
    pub by_priority: HashMap<String, usize>,
    pub by_feature_area: HashMap<String, usize>,
    pub by_type: HashMap<String, usize>,
}

impl CuratedComplianceStats {
    /// Get count for a specific priority
    pub fn priority_count(&self, priority: &str) -> usize {
        *self.by_priority.get(priority).unwrap_or(&0)
    }

    /// Get critical (P0) requirement count
    pub fn critical_count(&self) -> usize {
        self.priority_count("P0")
    }

    /// Get mandatory requirement count
    pub fn mandatory_count(&self) -> usize {
        *self.by_type.get("mandatory").unwrap_or(&0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_toml() -> String {
        r#"
[metadata]
version = "1.0.0"
curation_date = "2025-12-11"
original_count = 7775
curated_count = 3
reduction_ratio = 0.9996

[parser]
[[parser.requirements]]
id = "7.5.2-catalog"
name = "Document Catalog"
description = "The root of a document's object hierarchy is the catalog dictionary."
iso_section = "7.5.2"
requirement_type = "mandatory"
priority = "P0"
feature_area = "parser"
implemented = true
implementation_refs = ["src/parser/document.rs"]
test_refs = ["tests/parser_tests.rs"]
verification_level = 3
consolidates = ["7.5.2.1", "7.5.2.2"]

[writer]
[[writer.requirements]]
id = "7.3.2-boolean"
name = "Boolean Objects"
description = "Boolean objects represent logical true and false values."
iso_section = "7.3.2"
requirement_type = "mandatory"
priority = "P1"
feature_area = "writer"
implemented = false
implementation_refs = []
test_refs = []
verification_level = 0
consolidates = []

[[writer.requirements]]
id = "7.3.3-integers"
name = "Integer Objects"
description = "PDF supports integer objects."
iso_section = "7.3.3"
requirement_type = "optional"
priority = "P2"
feature_area = "writer"
implemented = false
implementation_refs = []
test_refs = []
verification_level = 0
consolidates = []
"#
        .to_string()
    }

    #[test]
    fn test_parse_curated_matrix() {
        let toml = create_test_toml();
        let matrix: CuratedIsoMatrix = toml::from_str(&toml).unwrap();

        assert_eq!(matrix.metadata.version, "1.0.0");
        assert_eq!(matrix.metadata.curated_count, 3);
        assert_eq!(matrix.total_count(), 3);
    }

    #[test]
    fn test_all_requirements() {
        let toml = create_test_toml();
        let matrix: CuratedIsoMatrix = toml::from_str(&toml).unwrap();

        let all = matrix.all_requirements();
        assert_eq!(all.len(), 3);

        let ids: Vec<&str> = all.iter().map(|r| r.id.as_str()).collect();
        assert!(ids.contains(&"7.5.2-catalog"));
        assert!(ids.contains(&"7.3.2-boolean"));
        assert!(ids.contains(&"7.3.3-integers"));
    }

    #[test]
    fn test_by_priority() {
        let toml = create_test_toml();
        let matrix: CuratedIsoMatrix = toml::from_str(&toml).unwrap();

        let p0 = matrix.by_priority(Priority::Critical);
        assert_eq!(p0.len(), 1);
        assert_eq!(p0[0].id, "7.5.2-catalog");

        let p1 = matrix.by_priority(Priority::High);
        assert_eq!(p1.len(), 1);
        assert_eq!(p1[0].id, "7.3.2-boolean");

        let p2 = matrix.by_priority(Priority::Medium);
        assert_eq!(p2.len(), 1);
    }

    #[test]
    fn test_by_feature_area() {
        let toml = create_test_toml();
        let matrix: CuratedIsoMatrix = toml::from_str(&toml).unwrap();

        let parser_reqs = matrix.by_feature_area("parser");
        assert_eq!(parser_reqs.len(), 1);

        let writer_reqs = matrix.by_feature_area("writer");
        assert_eq!(writer_reqs.len(), 2);
    }

    #[test]
    fn test_mandatory_requirements() {
        let toml = create_test_toml();
        let matrix: CuratedIsoMatrix = toml::from_str(&toml).unwrap();

        let mandatory = matrix.mandatory_requirements();
        assert_eq!(mandatory.len(), 2);
    }

    #[test]
    fn test_unimplemented() {
        let toml = create_test_toml();
        let matrix: CuratedIsoMatrix = toml::from_str(&toml).unwrap();

        let unimplemented = matrix.unimplemented();
        assert_eq!(unimplemented.len(), 2);
    }

    #[test]
    fn test_get_by_id() {
        let toml = create_test_toml();
        let matrix: CuratedIsoMatrix = toml::from_str(&toml).unwrap();

        let req = matrix.get("7.5.2-catalog").unwrap();
        assert_eq!(req.name, "Document Catalog");
        assert!(req.implemented);

        assert!(matrix.get("nonexistent").is_none());
    }

    #[test]
    fn test_calculate_stats() {
        let toml = create_test_toml();
        let matrix: CuratedIsoMatrix = toml::from_str(&toml).unwrap();

        let stats = matrix.calculate_stats();
        assert_eq!(stats.total_requirements, 3);
        assert_eq!(stats.implemented_count, 1);
        assert!((stats.implementation_percentage - 33.33).abs() < 1.0);
        assert_eq!(stats.critical_count(), 1);
        assert_eq!(stats.mandatory_count(), 2);
    }

    #[test]
    fn test_priority_from_str() {
        assert_eq!(Priority::from_str("P0"), Some(Priority::Critical));
        assert_eq!(Priority::from_str("P1"), Some(Priority::High));
        assert_eq!(Priority::from_str("P2"), Some(Priority::Medium));
        assert_eq!(Priority::from_str("P3"), Some(Priority::Low));
        assert_eq!(Priority::from_str("invalid"), None);
    }

    #[test]
    fn test_by_section() {
        let toml = create_test_toml();
        let matrix: CuratedIsoMatrix = toml::from_str(&toml).unwrap();

        let section_7 = matrix.by_section("7");
        assert_eq!(section_7.len(), 3);

        let section_7_5 = matrix.by_section("7.5");
        assert_eq!(section_7_5.len(), 1);
        assert_eq!(section_7_5[0].id, "7.5.2-catalog");
    }
}
