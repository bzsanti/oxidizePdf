//! ISO Compliance Matrix Processing
//!
//! This module loads and processes the ISO compliance matrix from TOML format,
//! providing access to requirements, verification levels, and compliance tracking.

use crate::error::{PdfError, Result};
use crate::verification::{IsoRequirement, VerificationLevel};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;

/// Complete ISO compliance matrix loaded from TOML
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct IsoMatrix {
    pub metadata: MatrixMetadata,
    pub sections: HashMap<String, IsoSection>,
    pub overall_summary: OverallSummary,
    pub validation_tools: ValidationTools,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MatrixMetadata {
    pub version: String,
    pub total_features: u32,
    pub specification: String,
    pub methodology: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct IsoSection {
    pub name: String,
    pub iso_section: String,
    pub total_requirements: u32,
    pub summary: SectionSummary,
    pub requirements: Vec<IsoRequirementData>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SectionSummary {
    pub implemented: u32,
    pub average_level: f64,
    pub compliance_percentage: f64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct IsoRequirementData {
    pub id: String,
    pub name: String,
    pub description: String,
    pub iso_reference: String,
    pub implementation: String,
    pub test_file: String,
    pub level: u8,
    pub verified: bool,
    pub external_validation: Option<String>,
    pub notes: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct OverallSummary {
    pub total_sections: u32,
    pub total_requirements: u32,
    pub total_implemented: u32,
    pub average_level: f64,
    pub real_compliance_percentage: f64,
    pub level_0_count: u32,
    pub level_1_count: u32,
    pub level_2_count: u32,
    pub level_3_count: u32,
    pub level_4_count: u32,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ValidationTools {
    pub external_validators: Vec<String>,
    pub internal_parser: bool,
    pub reference_pdfs: bool,
    pub automated_testing: bool,
}

/// Load ISO compliance matrix from TOML file
pub fn load_matrix(path: &str) -> Result<IsoMatrix> {
    let toml_content = fs::read_to_string(path).map_err(PdfError::Io)?;

    // Try direct Serde deserialization first, with custom structure for TOML arrays
    parse_compliance_matrix_with_serde(&toml_content)
}

/// Parse using Serde with custom structure that handles TOML array of tables
fn parse_compliance_matrix_with_serde(toml_content: &str) -> Result<IsoMatrix> {
    use serde::de::{Deserializer, MapAccess, Visitor};
    use std::fmt;

    // Custom deserializer that handles sections with their nested requirements
    #[derive(Deserialize)]
    struct TomlMatrix {
        metadata: MatrixMetadata,
        overall_summary: OverallSummary,
        validation_tools: ValidationTools,
        #[serde(flatten, deserialize_with = "deserialize_sections")]
        sections: HashMap<String, IsoSection>,
    }

    fn deserialize_sections<'de, D>(
        deserializer: D,
    ) -> std::result::Result<HashMap<String, IsoSection>, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct SectionsVisitor;

        impl<'de> Visitor<'de> for SectionsVisitor {
            type Value = HashMap<String, IsoSection>;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a map of sections")
            }

            fn visit_map<A>(self, mut map: A) -> std::result::Result<Self::Value, A::Error>
            where
                A: MapAccess<'de>,
            {
                let mut sections = HashMap::new();
                let mut requirements_map: HashMap<String, Vec<IsoRequirementData>> = HashMap::new();

                // First pass: collect all data
                while let Some(key) = map.next_key::<String>()? {
                    if key.starts_with("section_") {
                        if key.contains(".requirements") {
                            // This is a requirements array
                            let section_name = key.replace(".requirements", "");
                            let reqs: Vec<IsoRequirementData> = map.next_value()?;
                            requirements_map.insert(section_name, reqs);
                        } else if key.starts_with("section_") && !key.contains(".") {
                            // This is a section definition
                            #[derive(Deserialize)]
                            struct SectionBase {
                                name: String,
                                iso_section: String,
                                total_requirements: u32,
                                summary: SectionSummary,
                            }

                            let section_base: SectionBase = map.next_value()?;
                            let requirements =
                                requirements_map.get(&key).cloned().unwrap_or_default();

                            let section = IsoSection {
                                name: section_base.name,
                                iso_section: section_base.iso_section,
                                total_requirements: section_base.total_requirements,
                                summary: section_base.summary,
                                requirements,
                            };

                            sections.insert(key, section);
                        }
                    } else {
                        // Skip non-section keys
                        let _: toml::Value = map.next_value()?;
                    }
                }

                Ok(sections)
            }
        }

        deserializer.deserialize_map(SectionsVisitor)
    }

    let matrix: TomlMatrix = toml::from_str(toml_content)
        .map_err(|e| PdfError::ParseError(format!("Failed to parse TOML matrix: {}", e)))?;

    Ok(IsoMatrix {
        metadata: matrix.metadata,
        sections: matrix.sections,
        overall_summary: matrix.overall_summary,
        validation_tools: matrix.validation_tools,
    })
}



/// Load default matrix from project root
pub fn load_default_matrix() -> Result<IsoMatrix> {
    // Try multiple potential locations for the matrix file
    let potential_paths = [
        "../ISO_COMPLIANCE_MATRIX.toml",
        "../../ISO_COMPLIANCE_MATRIX.toml",
        "ISO_COMPLIANCE_MATRIX.toml",
        "./ISO_COMPLIANCE_MATRIX.toml",
    ];

    for path in &potential_paths {
        if std::path::Path::new(path).exists() {
            return load_matrix(path);
        }
    }

    Err(PdfError::Io(std::io::Error::new(
        std::io::ErrorKind::NotFound,
        "ISO_COMPLIANCE_MATRIX.toml not found in any expected location",
    )))
}

impl IsoMatrix {
    /// Get all requirements across all sections
    pub fn get_all_requirements(&self) -> Vec<IsoRequirement> {
        let mut requirements = Vec::new();

        for section in self.sections.values() {
            for req_data in &section.requirements {
                let level = VerificationLevel::from_u8(req_data.level)
                    .unwrap_or(VerificationLevel::NotImplemented);

                requirements.push(IsoRequirement {
                    id: req_data.id.clone(),
                    name: req_data.name.clone(),
                    description: req_data.description.clone(),
                    iso_reference: req_data.iso_reference.clone(),
                    implementation: if req_data.implementation == "None" {
                        None
                    } else {
                        Some(req_data.implementation.clone())
                    },
                    test_file: if req_data.test_file == "None" {
                        None
                    } else {
                        Some(req_data.test_file.clone())
                    },
                    level,
                    verified: req_data.verified,
                    notes: req_data.notes.clone(),
                });
            }
        }

        requirements
    }

    /// Get requirements for a specific section
    pub fn get_section_requirements(&self, section_id: &str) -> Option<Vec<IsoRequirement>> {
        if let Some(section) = self.sections.get(section_id) {
            let mut requirements = Vec::new();

            for req_data in &section.requirements {
                let level = VerificationLevel::from_u8(req_data.level)
                    .unwrap_or(VerificationLevel::NotImplemented);

                requirements.push(IsoRequirement {
                    id: req_data.id.clone(),
                    name: req_data.name.clone(),
                    description: req_data.description.clone(),
                    iso_reference: req_data.iso_reference.clone(),
                    implementation: if req_data.implementation == "None" {
                        None
                    } else {
                        Some(req_data.implementation.clone())
                    },
                    test_file: if req_data.test_file == "None" {
                        None
                    } else {
                        Some(req_data.test_file.clone())
                    },
                    level,
                    verified: req_data.verified,
                    notes: req_data.notes.clone(),
                });
            }

            Some(requirements)
        } else {
            None
        }
    }

    /// Get requirement by ID
    pub fn get_requirement(&self, requirement_id: &str) -> Option<IsoRequirement> {
        for section in self.sections.values() {
            for req_data in &section.requirements {
                if req_data.id == requirement_id {
                    let level = VerificationLevel::from_u8(req_data.level)
                        .unwrap_or(VerificationLevel::NotImplemented);

                    return Some(IsoRequirement {
                        id: req_data.id.clone(),
                        name: req_data.name.clone(),
                        description: req_data.description.clone(),
                        iso_reference: req_data.iso_reference.clone(),
                        implementation: if req_data.implementation == "None" {
                            None
                        } else {
                            Some(req_data.implementation.clone())
                        },
                        test_file: if req_data.test_file == "None" {
                            None
                        } else {
                            Some(req_data.test_file.clone())
                        },
                        level,
                        verified: req_data.verified,
                        notes: req_data.notes.clone(),
                    });
                }
            }
        }
        None
    }

    /// Calculate compliance statistics
    pub fn calculate_compliance_stats(&self) -> ComplianceStats {
        let all_requirements = self.get_all_requirements();
        let total_count = all_requirements.len();

        let mut level_counts = [0u32; 5]; // 0-4
        let mut total_percentage = 0.0;
        let mut implemented_count = 0;

        for req in &all_requirements {
            let level_index = req.level as usize;
            level_counts[level_index] += 1;

            let percentage = req.level.as_percentage();
            total_percentage += percentage;

            if req.level as u8 > 0 {
                implemented_count += 1;
            }
        }

        let average_percentage = if total_count > 0 {
            total_percentage / total_count as f64
        } else {
            0.0
        };

        ComplianceStats {
            total_requirements: total_count as u32,
            implemented_requirements: implemented_count,
            average_compliance_percentage: average_percentage,
            level_0_count: level_counts[0],
            level_1_count: level_counts[1],
            level_2_count: level_counts[2],
            level_3_count: level_counts[3],
            level_4_count: level_counts[4],
        }
    }

    /// Get requirements that need implementation (level 0)
    pub fn get_unimplemented_requirements(&self) -> Vec<IsoRequirement> {
        self.get_all_requirements()
            .into_iter()
            .filter(|req| req.level == VerificationLevel::NotImplemented)
            .collect()
    }

    /// Get requirements that need better verification (level 1-2)
    pub fn get_partially_implemented_requirements(&self) -> Vec<IsoRequirement> {
        self.get_all_requirements()
            .into_iter()
            .filter(|req| {
                matches!(
                    req.level,
                    VerificationLevel::CodeExists | VerificationLevel::GeneratesPdf
                )
            })
            .collect()
    }

    /// Get fully compliant requirements (level 4)
    pub fn get_compliant_requirements(&self) -> Vec<IsoRequirement> {
        self.get_all_requirements()
            .into_iter()
            .filter(|req| req.level == VerificationLevel::IsoCompliant)
            .collect()
    }
}

/// Compliance statistics calculated from matrix
#[derive(Debug, Clone)]
pub struct ComplianceStats {
    pub total_requirements: u32,
    pub implemented_requirements: u32,
    pub average_compliance_percentage: f64,
    pub level_0_count: u32,
    pub level_1_count: u32,
    pub level_2_count: u32,
    pub level_3_count: u32,
    pub level_4_count: u32,
}

impl ComplianceStats {
    /// Get compliance percentage as a string for display
    pub fn compliance_percentage_display(&self) -> String {
        format!("{:.1}%", self.average_compliance_percentage)
    }

    /// Get implementation percentage (non-zero levels)
    pub fn implementation_percentage(&self) -> f64 {
        if self.total_requirements > 0 {
            (self.implemented_requirements as f64 / self.total_requirements as f64) * 100.0
        } else {
            0.0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::NamedTempFile;

    use std::io::Write;

    fn create_test_matrix_toml() -> String {
        r#"
[metadata]
version = "2025-08-21"
total_features = 3
specification = "ISO 32000-1:2008"
methodology = "docs/ISO_TESTING_METHODOLOGY.md"

[section_7_5]
name = "Document Structure"
iso_section = "7.5"
total_requirements = 2

[section_7_5.summary]
implemented = 1
average_level = 2.0
compliance_percentage = 50.0

[[section_7_5.requirements]]
id = "7.5.2.1"
name = "Catalog Type Entry"
description = "Document catalog must have /Type /Catalog"
iso_reference = "7.5.2, Table 3.25"
implementation = "src/document.rs:156-160"
test_file = "tests/iso_verification/section_7/test_catalog.rs"
level = 3
verified = true
notes = "Implemented and verified"

[[section_7_5.requirements]]
id = "7.5.2.2"
name = "Catalog Version Entry"
description = "Optional /Version entry in catalog"
iso_reference = "7.5.2, Table 3.25"
implementation = "None"
test_file = "None"
level = 0
verified = false
notes = "Not implemented"

[overall_summary]
total_sections = 1
total_requirements = 2
total_implemented = 1
average_level = 1.5
real_compliance_percentage = 37.5
level_0_count = 1
level_1_count = 0
level_2_count = 0
level_3_count = 1
level_4_count = 0

[validation_tools]
external_validators = ["qpdf"]
internal_parser = true
reference_pdfs = false
automated_testing = false
"#
        .to_string()
    }

    #[test]
    fn test_load_matrix() {
        let toml_content = create_test_matrix_toml();
        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(toml_content.as_bytes()).unwrap();

        let matrix = load_matrix(temp_file.path().to_str().unwrap()).unwrap();

        assert_eq!(matrix.metadata.total_features, 3);
        assert_eq!(matrix.sections.len(), 1);
        assert!(matrix.sections.contains_key("section_7_5"));

        let section = &matrix.sections["section_7_5"];
        assert_eq!(section.requirements.len(), 2);
        assert_eq!(section.requirements[0].id, "7.5.2.1");
        assert_eq!(section.requirements[0].level, 3);
    }

    #[test]
    fn test_get_all_requirements() {
        let toml_content = create_test_matrix_toml();
        let matrix: IsoMatrix = toml::from_str(&toml_content).unwrap();

        let requirements = matrix.get_all_requirements();
        assert_eq!(requirements.len(), 2);

        let req1 = &requirements[0];
        assert_eq!(req1.id, "7.5.2.1");
        assert_eq!(req1.level, VerificationLevel::ContentVerified);
        assert!(req1.verified);

        let req2 = &requirements[1];
        assert_eq!(req2.id, "7.5.2.2");
        assert_eq!(req2.level, VerificationLevel::NotImplemented);
        assert!(!req2.verified);
    }

    #[test]
    fn test_get_requirement_by_id() {
        let toml_content = create_test_matrix_toml();
        let matrix: IsoMatrix = toml::from_str(&toml_content).unwrap();

        let req = matrix.get_requirement("7.5.2.1").unwrap();
        assert_eq!(req.name, "Catalog Type Entry");
        assert_eq!(req.level, VerificationLevel::ContentVerified);

        assert!(matrix.get_requirement("nonexistent").is_none());
    }

    #[test]
    fn test_calculate_compliance_stats() {
        let toml_content = create_test_matrix_toml();
        let matrix: IsoMatrix = toml::from_str(&toml_content).unwrap();

        let stats = matrix.calculate_compliance_stats();
        assert_eq!(stats.total_requirements, 2);
        assert_eq!(stats.implemented_requirements, 1);
        assert_eq!(stats.level_0_count, 1);
        assert_eq!(stats.level_3_count, 1);
        assert_eq!(stats.average_compliance_percentage, 37.5); // (0 + 75) / 2
    }

    #[test]
    fn test_get_unimplemented_requirements() {
        let toml_content = create_test_matrix_toml();
        let matrix: IsoMatrix = toml::from_str(&toml_content).unwrap();

        let unimplemented = matrix.get_unimplemented_requirements();
        assert_eq!(unimplemented.len(), 1);
        assert_eq!(unimplemented[0].id, "7.5.2.2");
    }
}
