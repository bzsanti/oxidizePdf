//! Matrix parsing module - loads ISO_COMPLIANCE_MATRIX.toml

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// The complete ISO compliance matrix as stored in TOML
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct IsoMatrix {
    #[serde(default)]
    pub metadata: Option<MatrixMetadata>,
    #[serde(flatten)]
    pub sections: HashMap<String, SectionData>,
}

/// Metadata about the matrix (may not exist in original)
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MatrixMetadata {
    pub version: Option<String>,
    pub source: Option<String>,
    pub extraction_date: Option<String>,
}

/// Section data with requirements
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SectionData {
    pub requirements: Vec<Requirement>,
}

/// A single requirement/fragment from the matrix
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Requirement {
    pub id: String,
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub page: Option<u32>,
    #[serde(default)]
    pub iso_section: Option<String>,
    #[serde(default)]
    pub iso_reference: Option<String>,
    #[serde(default)]
    pub requirement_type: Option<String>,
    #[serde(default)]
    pub implemented: Option<bool>,
    #[serde(default)]
    pub implementation_refs: Option<Vec<String>>,
    #[serde(default)]
    pub test_refs: Option<Vec<String>>,
    #[serde(default)]
    pub verification_level: Option<u8>,
}

impl IsoMatrix {
    /// Load matrix from TOML file
    pub fn load(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read matrix file: {}", path.display()))?;

        // The matrix structure is complex - sections are top-level keys
        // Try to parse the specific structure
        Self::parse_toml(&content)
    }

    /// Parse TOML content into matrix structure
    fn parse_toml(content: &str) -> Result<Self> {
        // First, try direct parsing
        if let Ok(matrix) = toml::from_str::<IsoMatrix>(content) {
            return Ok(matrix);
        }

        // The actual matrix format has sections like [section_7_document_structure]
        // Each section has [[section_X.requirements]] arrays
        let value: toml::Value = toml::from_str(content)
            .context("Failed to parse TOML content")?;

        let table = value.as_table()
            .context("Matrix root should be a table")?;

        let mut sections = HashMap::new();

        for (key, value) in table {
            if key == "metadata" {
                continue;
            }

            // Each section should have a requirements array
            if let Some(section_table) = value.as_table() {
                if let Some(reqs_value) = section_table.get("requirements") {
                    if let Some(reqs_array) = reqs_value.as_array() {
                        let requirements: Vec<Requirement> = reqs_array
                            .iter()
                            .filter_map(|v| {
                                toml::Value::try_into::<Requirement>(v.clone()).ok()
                            })
                            .collect();

                        if !requirements.is_empty() {
                            sections.insert(key.clone(), SectionData { requirements });
                        }
                    }
                }
            }
        }

        Ok(IsoMatrix {
            metadata: None,
            sections,
        })
    }

    /// Get all requirements as a flat list
    pub fn all_requirements(&self) -> Vec<&Requirement> {
        self.sections
            .values()
            .flat_map(|s| &s.requirements)
            .collect()
    }

    /// Get total requirement count
    pub fn total_count(&self) -> usize {
        self.sections
            .values()
            .map(|s| s.requirements.len())
            .sum()
    }

    /// Get requirements by section name pattern
    pub fn requirements_by_section(&self, pattern: &str) -> Vec<&Requirement> {
        self.sections
            .iter()
            .filter(|(k, _)| k.contains(pattern))
            .flat_map(|(_, s)| &s.requirements)
            .collect()
    }
}

/// Flattened requirement with section info for processing
#[derive(Debug, Clone)]
pub struct FlatRequirement {
    pub section_key: String,
    pub id: String,
    pub name: String,
    pub description: String,
    pub page: u32,
    pub iso_section: String,
    pub requirement_type: String,
}

impl IsoMatrix {
    /// Get all requirements flattened with section info
    pub fn flatten(&self) -> Vec<FlatRequirement> {
        let mut result = Vec::new();

        for (section_key, section_data) in &self.sections {
            for req in &section_data.requirements {
                // Extract section from iso_section, iso_reference, section_key, or id
                let iso_section = extract_iso_section(
                    req.iso_section.as_deref(),
                    req.iso_reference.as_deref(),
                    section_key,
                    &req.id,
                );

                result.push(FlatRequirement {
                    section_key: section_key.clone(),
                    id: req.id.clone(),
                    name: req.name.clone(),
                    description: req.description.clone(),
                    page: req.page.unwrap_or(0),
                    iso_section,
                    requirement_type: req
                        .requirement_type
                        .clone()
                        .unwrap_or_else(|| "unknown".to_string()),
                });
            }
        }

        result
    }
}

/// Extract ISO section number from available fields
fn extract_iso_section(
    iso_section: Option<&str>,
    iso_reference: Option<&str>,
    section_key: &str,
    id: &str,
) -> String {
    // Try iso_section field first
    if let Some(s) = iso_section {
        if !s.is_empty() {
            return s.to_string();
        }
    }

    // Try to extract from iso_reference (format: "Page 11, Section 7.1")
    if let Some(reference) = iso_reference {
        if let Some(idx) = reference.find("Section ") {
            let section_part = &reference[idx + 8..];
            // Extract section number (e.g., "7.1" from "7.1.2")
            let section: String = section_part
                .chars()
                .take_while(|c| c.is_ascii_digit() || *c == '.')
                .collect();
            if !section.is_empty() {
                return section;
            }
        }
    }

    // Try to extract from section_key (format: "section_7_syntax")
    if section_key.starts_with("section_") {
        let parts: Vec<&str> = section_key.split('_').collect();
        if parts.len() >= 2 {
            if let Ok(num) = parts[1].parse::<u32>() {
                return num.to_string();
            }
        }
    }

    // Try to extract from id (format: "7.1" or "7.123")
    let section: String = id
        .chars()
        .take_while(|c| c.is_ascii_digit() || *c == '.')
        .collect();
    if !section.is_empty() && section.contains('.') {
        // Return first two levels (e.g., "7.1" from "7.1.2")
        let parts: Vec<&str> = section.split('.').collect();
        if parts.len() >= 2 {
            return format!("{}.{}", parts[0], parts[1]);
        }
    }

    "unknown".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_empty_matrix() {
        let content = r#"
[section_test]
requirements = []
"#;
        let matrix = IsoMatrix::parse_toml(content).unwrap();
        assert_eq!(matrix.total_count(), 0);
    }

    #[test]
    fn test_parse_single_requirement() {
        let content = r#"
[section_test]
[[section_test.requirements]]
id = "7.1"
name = "Test Requirement"
description = "This is a test requirement."
"#;
        let matrix = IsoMatrix::parse_toml(content).unwrap();
        assert_eq!(matrix.total_count(), 1);
    }
}
