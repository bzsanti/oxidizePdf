//! ISO Compliance Matrix Processing
//!
//! This module loads and processes the ISO compliance matrix from TOML format,
//! providing access to requirements, verification levels, and compliance tracking.
//!
//! DUAL FILE SYSTEM:
//! - ISO_COMPLIANCE_MATRIX.toml: Immutable definitions (NEVER modify)
//! - ISO_VERIFICATION_STATUS.toml: Mutable verification state (ONLY this changes)

#![allow(deprecated)]

use crate::error::{PdfError, Result};
use crate::verification::{IsoRequirement, VerificationLevel};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;

/// Complete ISO compliance matrix loaded from TOML (IMMUTABLE)
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct IsoMatrix {
    pub metadata: MatrixMetadata,
    #[serde(flatten)]
    pub sections: HashMap<String, IsoSection>,
    pub overall_summary: OverallSummary,
    pub validation_tools: ValidationTools,
}

/// Verification status loaded from separate TOML file (MUTABLE)
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct VerificationStatus {
    pub metadata: StatusMetadata,
    pub status: HashMap<String, RequirementStatus>,
    pub statistics: StatusStatistics,
}

/// Combined system that reads both files
#[derive(Debug, Clone)]
pub struct ComplianceSystem {
    pub matrix: IsoMatrix,          // Immutable definitions
    pub status: VerificationStatus, // Current state
}

/// Metadata for verification status file
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct StatusMetadata {
    pub last_updated: String,
    pub matrix_version: String,
    pub total_requirements: u32,
    pub note: String,
    pub warning: String,
}

/// Status of individual requirement
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RequirementStatus {
    pub level: u8,
    pub implementation: String,
    pub test_file: String,
    pub verified: bool,
    pub last_checked: String,
    pub notes: String,
}

/// Overall statistics from status file
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct StatusStatistics {
    pub level_0_count: u32,
    pub level_1_count: u32,
    pub level_2_count: u32,
    pub level_3_count: u32,
    pub level_4_count: u32,
    pub average_level: f64,
    pub compliance_percentage: f64,
    pub last_calculated: String,
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
    pub requirement_type: String, // mandatory/optional/recommended
    pub page: u32,
    pub original_text: String,
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

/// Parse using Serde with simplified structure that handles TOML correctly
fn parse_compliance_matrix_with_serde(toml_content: &str) -> Result<IsoMatrix> {
    // Use direct TOML deserialization instead of complex custom deserializer
    toml::from_str::<IsoMatrix>(toml_content)
        .map_err(|e| PdfError::ParseError(format!("Failed to parse matrix TOML: {}", e)))
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

/// Load verification status from TOML file
pub fn load_verification_status(path: &str) -> Result<VerificationStatus> {
    let toml_content = fs::read_to_string(path).map_err(PdfError::Io)?;
    toml::from_str(&toml_content)
        .map_err(|e| PdfError::ParseError(format!("Failed to parse verification status: {}", e)))
}

/// Load default verification status from project root
pub fn load_default_verification_status() -> Result<VerificationStatus> {
    let potential_paths = [
        "../ISO_VERIFICATION_STATUS.toml",
        "../../ISO_VERIFICATION_STATUS.toml",
        "ISO_VERIFICATION_STATUS.toml",
        "./ISO_VERIFICATION_STATUS.toml",
    ];

    for path in &potential_paths {
        if std::path::Path::new(path).exists() {
            return load_verification_status(path);
        }
    }

    Err(PdfError::Io(std::io::Error::new(
        std::io::ErrorKind::NotFound,
        "ISO_VERIFICATION_STATUS.toml not found in any expected location",
    )))
}

/// Load complete compliance system (matrix + status)
pub fn load_compliance_system() -> Result<ComplianceSystem> {
    let matrix = load_default_matrix()?;
    let status = load_default_verification_status()?;

    Ok(ComplianceSystem { matrix, status })
}

impl ComplianceSystem {
    /// Get all requirements with complete info (definition + status)
    pub fn get_all_requirements(&self) -> Vec<IsoRequirement> {
        let mut requirements = Vec::new();

        for section in self.matrix.sections.values() {
            for req_data in &section.requirements {
                let status = self.status.status.get(&req_data.id);

                let level = status
                    .map(|s| {
                        VerificationLevel::from_u8(s.level)
                            .unwrap_or(VerificationLevel::NotImplemented)
                    })
                    .unwrap_or(VerificationLevel::NotImplemented);

                requirements.push(IsoRequirement {
                    id: req_data.id.clone(),
                    name: req_data.name.clone(),
                    description: req_data.description.clone(),
                    iso_reference: req_data.iso_reference.clone(),
                    implementation: status.and_then(|s| {
                        if s.implementation.is_empty() {
                            None
                        } else {
                            Some(s.implementation.clone())
                        }
                    }),
                    test_file: status.and_then(|s| {
                        if s.test_file.is_empty() {
                            None
                        } else {
                            Some(s.test_file.clone())
                        }
                    }),
                    level,
                    verified: status.map(|s| s.verified).unwrap_or(false),
                    notes: status
                        .map(|s| s.notes.clone())
                        .unwrap_or(req_data.requirement_type.clone()),
                });
            }
        }

        requirements
    }

    /// Get requirement info combining matrix definition + current status
    pub fn get_requirement_info(&self, id: &str) -> Option<RequirementInfo> {
        // Find definition in matrix
        for section in self.matrix.sections.values() {
            for req_data in &section.requirements {
                if req_data.id == id {
                    // Find status
                    let status = self.status.status.get(id);

                    return Some(RequirementInfo {
                        id: req_data.id.clone(),
                        name: req_data.name.clone(),
                        description: req_data.description.clone(),
                        iso_reference: req_data.iso_reference.clone(),
                        requirement_type: req_data.requirement_type.clone(),
                        page: req_data.page,
                        level: status.map(|s| s.level).unwrap_or(0),
                        implementation: status
                            .map(|s| s.implementation.clone())
                            .unwrap_or_default(),
                        test_file: status.map(|s| s.test_file.clone()).unwrap_or_default(),
                        verified: status.map(|s| s.verified).unwrap_or(false),
                        last_checked: status
                            .map(|s| s.last_checked.clone())
                            .unwrap_or("never".to_string()),
                        notes: status.map(|s| s.notes.clone()).unwrap_or_default(),
                    });
                }
            }
        }
        None
    }

    /// Get requirements for a specific section with status
    pub fn get_section_requirements(&self, section_id: &str) -> Option<Vec<IsoRequirement>> {
        if let Some(section) = self.matrix.sections.get(section_id) {
            let mut requirements = Vec::new();

            for req_data in &section.requirements {
                let status = self.status.status.get(&req_data.id);

                let level = status
                    .map(|s| {
                        VerificationLevel::from_u8(s.level)
                            .unwrap_or(VerificationLevel::NotImplemented)
                    })
                    .unwrap_or(VerificationLevel::NotImplemented);

                requirements.push(IsoRequirement {
                    id: req_data.id.clone(),
                    name: req_data.name.clone(),
                    description: req_data.description.clone(),
                    iso_reference: req_data.iso_reference.clone(),
                    implementation: status.and_then(|s| {
                        if s.implementation.is_empty() {
                            None
                        } else {
                            Some(s.implementation.clone())
                        }
                    }),
                    test_file: status.and_then(|s| {
                        if s.test_file.is_empty() {
                            None
                        } else {
                            Some(s.test_file.clone())
                        }
                    }),
                    level,
                    verified: status.map(|s| s.verified).unwrap_or(false),
                    notes: status
                        .map(|s| s.notes.clone())
                        .unwrap_or(req_data.requirement_type.clone()),
                });
            }

            Some(requirements)
        } else {
            None
        }
    }

    /// Calculate compliance statistics from current status
    pub fn calculate_compliance_stats(&self) -> ComplianceStats {
        ComplianceStats {
            total_requirements: self.status.statistics.level_0_count
                + self.status.statistics.level_1_count
                + self.status.statistics.level_2_count
                + self.status.statistics.level_3_count
                + self.status.statistics.level_4_count,
            implemented_requirements: self.status.statistics.level_1_count
                + self.status.statistics.level_2_count
                + self.status.statistics.level_3_count
                + self.status.statistics.level_4_count,
            average_compliance_percentage: self.status.statistics.compliance_percentage,
            level_0_count: self.status.statistics.level_0_count,
            level_1_count: self.status.statistics.level_1_count,
            level_2_count: self.status.statistics.level_2_count,
            level_3_count: self.status.statistics.level_3_count,
            level_4_count: self.status.statistics.level_4_count,
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

    /// Update verification status for a requirement
    pub fn update_requirement_status(
        &mut self,
        id: &str,
        new_status: RequirementStatus,
    ) -> Result<()> {
        self.status.status.insert(id.to_string(), new_status);

        // Update metadata
        self.status.metadata.last_updated = chrono::Utc::now().to_rfc3339();

        // Recalculate statistics
        self.recalculate_statistics();

        Ok(())
    }

    /// Save status file (matrix is never saved - it's immutable)
    pub fn save_status(&self, path: &str) -> Result<()> {
        let toml_content = toml::to_string_pretty(&self.status)
            .map_err(|e| PdfError::ParseError(format!("Failed to serialize status: {}", e)))?;

        fs::write(path, toml_content).map_err(PdfError::Io)?;
        Ok(())
    }

    /// Recalculate statistics based on current status
    fn recalculate_statistics(&mut self) {
        let mut level_counts = [0u32; 5];
        let mut total_level = 0u32;

        for status in self.status.status.values() {
            if status.level <= 4 {
                level_counts[status.level as usize] += 1;
                total_level += status.level as u32;
            }
        }

        let total_requirements = self.status.status.len() as u32;
        let average_level = if total_requirements > 0 {
            total_level as f64 / total_requirements as f64
        } else {
            0.0
        };

        self.status.statistics = StatusStatistics {
            level_0_count: level_counts[0],
            level_1_count: level_counts[1],
            level_2_count: level_counts[2],
            level_3_count: level_counts[3],
            level_4_count: level_counts[4],
            average_level,
            compliance_percentage: (average_level / 4.0) * 100.0,
            last_calculated: chrono::Utc::now().to_rfc3339(),
        };
    }
}

/// Combined requirement information (definition + status)
#[derive(Debug, Clone)]
pub struct RequirementInfo {
    pub id: String,
    pub name: String,
    pub description: String,
    pub iso_reference: String,
    pub requirement_type: String,
    pub page: u32,
    pub level: u8,
    pub implementation: String,
    pub test_file: String,
    pub verified: bool,
    pub last_checked: String,
    pub notes: String,
}

impl IsoMatrix {
    /// Get all requirement definitions (without verification status)
    /// Note: This only returns definitions. Use ComplianceSystem::get_all_requirements() for full info.
    pub fn get_all_requirement_definitions(&self) -> Vec<&IsoRequirementData> {
        let mut requirements = Vec::new();

        for section in self.sections.values() {
            for req_data in &section.requirements {
                requirements.push(req_data);
            }
        }

        requirements
    }

    /// DEPRECATED: Use ComplianceSystem::get_all_requirements() instead
    #[deprecated(
        note = "Use ComplianceSystem::get_all_requirements() for complete requirement info"
    )]
    pub fn get_all_requirements(&self) -> Vec<IsoRequirement> {
        let mut requirements = Vec::new();

        for section in self.sections.values() {
            for req_data in &section.requirements {
                // Return basic requirement with no verification status
                requirements.push(IsoRequirement {
                    id: req_data.id.clone(),
                    name: req_data.name.clone(),
                    description: req_data.description.clone(),
                    iso_reference: req_data.iso_reference.clone(),
                    implementation: None, // No status info in matrix
                    test_file: None,      // No status info in matrix
                    level: VerificationLevel::NotImplemented, // Default level
                    verified: false,      // Default verification
                    notes: req_data.requirement_type.clone(), // Use type as notes
                });
            }
        }

        requirements
    }

    /// DEPRECATED: Use ComplianceSystem::get_section_requirements() instead
    #[deprecated(
        note = "Use ComplianceSystem::get_section_requirements() for complete requirement info"
    )]
    pub fn get_section_requirements(&self, section_id: &str) -> Option<Vec<IsoRequirement>> {
        if let Some(section) = self.sections.get(section_id) {
            let mut requirements = Vec::new();

            for req_data in &section.requirements {
                // Return basic requirement with no verification status
                requirements.push(IsoRequirement {
                    id: req_data.id.clone(),
                    name: req_data.name.clone(),
                    description: req_data.description.clone(),
                    iso_reference: req_data.iso_reference.clone(),
                    implementation: None, // No status info in matrix
                    test_file: None,      // No status info in matrix
                    level: VerificationLevel::NotImplemented, // Default level
                    verified: false,      // Default verification
                    notes: req_data.requirement_type.clone(), // Use type as notes
                });
            }

            Some(requirements)
        } else {
            None
        }
    }

    /// DEPRECATED: Use ComplianceSystem::get_requirement_info() instead
    #[deprecated(
        note = "Use ComplianceSystem::get_requirement_info() for complete requirement info"
    )]
    pub fn get_requirement(&self, requirement_id: &str) -> Option<IsoRequirement> {
        for section in self.sections.values() {
            for req_data in &section.requirements {
                if req_data.id == requirement_id {
                    return Some(IsoRequirement {
                        id: req_data.id.clone(),
                        name: req_data.name.clone(),
                        description: req_data.description.clone(),
                        iso_reference: req_data.iso_reference.clone(),
                        implementation: None, // No status info in matrix
                        test_file: None,      // No status info in matrix
                        level: VerificationLevel::NotImplemented, // Default level
                        verified: false,      // Default verification
                        notes: req_data.requirement_type.clone(), // Use type as notes
                    });
                }
            }
        }
        None
    }

    /// DEPRECATED: Use ComplianceSystem::calculate_compliance_stats() instead
    #[deprecated(note = "Use ComplianceSystem::calculate_compliance_stats() for real statistics")]
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

    /// DEPRECATED: Use ComplianceSystem methods instead
    #[deprecated(note = "Use ComplianceSystem::get_unimplemented_requirements() for real status")]
    pub fn get_unimplemented_requirements(&self) -> Vec<IsoRequirement> {
        self.get_all_requirements()
            .into_iter()
            .filter(|req| req.level == VerificationLevel::NotImplemented)
            .collect()
    }

    /// DEPRECATED: Use ComplianceSystem methods instead
    #[deprecated(
        note = "Use ComplianceSystem::get_partially_implemented_requirements() for real status"
    )]
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

    /// DEPRECATED: Use ComplianceSystem methods instead  
    #[deprecated(note = "Use ComplianceSystem::get_compliant_requirements() for real status")]
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
requirement_type = "mandatory"
page = 42
original_text = "Document catalog must have /Type /Catalog entry"

[[section_7_5.requirements]]
id = "7.5.2.2"
name = "Catalog Version Entry"
description = "Optional /Version entry in catalog"
iso_reference = "7.5.2, Table 3.25"
requirement_type = "optional"
page = 42
original_text = "Optional /Version entry in document catalog"

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
        assert_eq!(section.requirements[0].requirement_type, "mandatory");
    }

    #[test]
    fn test_get_all_requirements() {
        let toml_content = create_test_matrix_toml();
        let matrix: IsoMatrix = toml::from_str(&toml_content).unwrap();

        let requirements = matrix.get_all_requirements();
        assert_eq!(requirements.len(), 2);

        let req1 = &requirements[0];
        assert_eq!(req1.id, "7.5.2.1");
        assert_eq!(req1.level, VerificationLevel::NotImplemented); // Default level in matrix-only
        assert!(!req1.verified); // Default verification in matrix-only

        let req2 = &requirements[1];
        assert_eq!(req2.id, "7.5.2.2");
        assert_eq!(req2.level, VerificationLevel::NotImplemented); // Default level in matrix-only
        assert!(!req2.verified); // Default verification in matrix-only
    }

    #[test]
    fn test_get_requirement_by_id() {
        let toml_content = create_test_matrix_toml();
        let matrix: IsoMatrix = toml::from_str(&toml_content).unwrap();

        let req = matrix.get_requirement("7.5.2.1").unwrap();
        assert_eq!(req.name, "Catalog Type Entry");
        assert_eq!(req.level, VerificationLevel::NotImplemented); // Default level in matrix-only

        assert!(matrix.get_requirement("nonexistent").is_none());
    }

    #[test]
    fn test_calculate_compliance_stats() {
        let toml_content = create_test_matrix_toml();
        let matrix: IsoMatrix = toml::from_str(&toml_content).unwrap();

        let stats = matrix.calculate_compliance_stats();
        assert_eq!(stats.total_requirements, 2);
        assert_eq!(stats.implemented_requirements, 0); // None implemented in matrix-only
        assert_eq!(stats.level_0_count, 2); // Both at level 0 in matrix-only
        assert_eq!(stats.level_1_count, 0);
        assert_eq!(stats.level_2_count, 0);
        assert_eq!(stats.level_3_count, 0);
        assert_eq!(stats.level_4_count, 0);
        assert_eq!(stats.average_compliance_percentage, 0.0); // All at level 0
    }

    #[test]
    fn test_get_unimplemented_requirements() {
        let toml_content = create_test_matrix_toml();
        let matrix: IsoMatrix = toml::from_str(&toml_content).unwrap();

        let unimplemented = matrix.get_unimplemented_requirements();
        assert_eq!(unimplemented.len(), 2); // Both at level 0 in matrix-only
                                            // Both requirements are at NotImplemented level by default
        let ids: Vec<String> = unimplemented.iter().map(|r| r.id.clone()).collect();
        assert!(ids.contains(&"7.5.2.1".to_string()));
        assert!(ids.contains(&"7.5.2.2".to_string()));
    }
}
