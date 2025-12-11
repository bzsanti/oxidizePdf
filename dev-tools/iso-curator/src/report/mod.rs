//! Report generation module
//!
//! Generates reports in various formats:
//! - Terminal (colored output)
//! - JSON (for processing)
//! - TOML (curated matrix)

use crate::curation::consolidator::CuratedRequirement;
use crate::curation::ValidationResult;
use crate::matrix::FlatRequirement;
use colored::Colorize;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Analysis report for a matrix
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisReport {
    pub total_fragments: usize,
    pub valid_count: usize,
    pub invalid_count: usize,
    pub valid_percentage: f64,
    pub by_reason: HashMap<String, usize>,
    pub by_type: HashMap<String, usize>,
    pub by_section: HashMap<String, SectionStats>,
}

/// Statistics for a section
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SectionStats {
    pub total: usize,
    pub valid: usize,
    pub invalid: usize,
}

/// Validation result for a single fragment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FragmentValidation {
    pub id: String,
    pub section: String,
    pub is_valid: bool,
    pub reason: String,
    pub confidence: f64,
    pub description_preview: String,
}

impl AnalysisReport {
    /// Create a new empty report
    pub fn new() -> Self {
        Self {
            total_fragments: 0,
            valid_count: 0,
            invalid_count: 0,
            valid_percentage: 0.0,
            by_reason: HashMap::new(),
            by_type: HashMap::new(),
            by_section: HashMap::new(),
        }
    }

    /// Add a validation result to the report
    pub fn add_result(&mut self, fragment: &FlatRequirement, result: &ValidationResult) {
        self.total_fragments += 1;

        if result.is_valid {
            self.valid_count += 1;
        } else {
            self.invalid_count += 1;
            *self.by_reason.entry(result.reason.clone()).or_insert(0) += 1;
        }

        // Track by type
        *self.by_type.entry(fragment.requirement_type.clone()).or_insert(0) += 1;

        // Track by section
        let section = extract_main_section(&fragment.iso_section);
        let stats = self.by_section.entry(section).or_insert(SectionStats {
            total: 0,
            valid: 0,
            invalid: 0,
        });
        stats.total += 1;
        if result.is_valid {
            stats.valid += 1;
        } else {
            stats.invalid += 1;
        }
    }

    /// Finalize calculations
    pub fn finalize(&mut self) {
        if self.total_fragments > 0 {
            self.valid_percentage = (self.valid_count as f64 / self.total_fragments as f64) * 100.0;
        }
    }

    /// Print report to terminal with colors
    pub fn print_terminal(&self) {
        println!("\n{}", "═".repeat(60).cyan());
        println!("{}", " ISO Matrix Analysis Report ".cyan().bold());
        println!("{}", "═".repeat(60).cyan());

        println!("\n{}", "Summary".yellow().bold());
        println!("  Total fragments: {}", self.total_fragments.to_string().white());
        println!(
            "  Valid:   {} ({:.1}%)",
            self.valid_count.to_string().green(),
            self.valid_percentage
        );
        println!(
            "  Invalid: {} ({:.1}%)",
            self.invalid_count.to_string().red(),
            100.0 - self.valid_percentage
        );

        println!("\n{}", "Invalid by Reason".yellow().bold());
        let mut reasons: Vec<_> = self.by_reason.iter().collect();
        reasons.sort_by(|a, b| b.1.cmp(a.1));
        for (reason, count) in reasons.iter().take(10) {
            println!("  {:>5} - {}", count.to_string().red(), reason);
        }

        println!("\n{}", "By Section".yellow().bold());
        let mut sections: Vec<_> = self.by_section.iter().collect();
        sections.sort_by(|a, b| a.0.cmp(b.0));
        for (section, stats) in sections {
            let pct = if stats.total > 0 {
                (stats.valid as f64 / stats.total as f64) * 100.0
            } else {
                0.0
            };
            let pct_color = if pct > 50.0 {
                format!("{:.1}%", pct).green()
            } else if pct > 20.0 {
                format!("{:.1}%", pct).yellow()
            } else {
                format!("{:.1}%", pct).red()
            };
            println!(
                "  Section {:>2}: {:>4} total, {:>4} valid ({})",
                section, stats.total, stats.valid, pct_color
            );
        }

        println!("\n{}", "═".repeat(60).cyan());
    }
}

/// Extract main section number (e.g., "7" from "7.3.5")
fn extract_main_section(iso_section: &str) -> String {
    iso_section
        .split('.')
        .next()
        .unwrap_or("unknown")
        .to_string()
}

/// Generate TOML for curated matrix
pub fn generate_curated_toml(requirements: &[CuratedRequirement]) -> String {
    let mut output = String::new();

    // Metadata
    output.push_str("[metadata]\n");
    output.push_str(&format!("version = \"1.0.0\"\n"));
    output.push_str(&format!(
        "curation_date = \"{}\"\n",
        chrono_lite_date()
    ));
    output.push_str(&format!("original_count = 7775\n"));
    output.push_str(&format!("curated_count = {}\n", requirements.len()));
    let reduction = 1.0 - (requirements.len() as f64 / 7775.0);
    output.push_str(&format!("reduction_ratio = {:.4}\n", reduction));
    output.push_str("\n");

    // Group by feature area (prefix to avoid collision with [metadata] section)
    let mut by_area: HashMap<String, Vec<&CuratedRequirement>> = HashMap::new();
    for req in requirements {
        // Rename "metadata" area to "doc_metadata" to avoid TOML collision
        let area = if req.feature_area == "metadata" {
            "doc_metadata".to_string()
        } else {
            req.feature_area.clone()
        };
        by_area.entry(area).or_default().push(req);
    }

    for (area, reqs) in by_area {
        output.push_str(&format!("[{}]\n", area));

        for req in reqs {
            output.push_str(&format!("[[{}.requirements]]\n", area));
            output.push_str(&format!("id = \"{}\"\n", req.id));
            output.push_str(&format!("name = \"{}\"\n", escape_toml(&req.name)));
            output.push_str(&format!(
                "description = \"\"\"\n{}\n\"\"\"\n",
                escape_toml(&req.description)
            ));
            output.push_str(&format!("iso_section = \"{}\"\n", req.iso_section));
            output.push_str(&format!("requirement_type = \"{}\"\n", req.requirement_type));
            output.push_str(&format!("priority = \"{}\"\n", req.priority));
            output.push_str(&format!("feature_area = \"{}\"\n", req.feature_area));
            output.push_str("implemented = false\n");
            output.push_str("implementation_refs = []\n");
            output.push_str("test_refs = []\n");
            output.push_str("verification_level = 0\n");
            if !req.consolidates.is_empty() {
                output.push_str(&format!(
                    "consolidates = [{}]\n",
                    req.consolidates
                        .iter()
                        .map(|s| format!("\"{}\"", s))
                        .collect::<Vec<_>>()
                        .join(", ")
                ));
            }
            output.push_str("\n");
        }
    }

    output
}

/// Escape special characters for TOML
fn escape_toml(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', " ")
        .replace('\r', "")
}

/// Simple date string (avoiding full chrono dependency)
fn chrono_lite_date() -> String {
    // For now, use a placeholder - in real use, would use chrono
    "2025-12-11".to_string()
}

/// Statistics summary for classification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassificationReport {
    pub total: usize,
    pub by_type: HashMap<String, usize>,
    pub by_priority: HashMap<String, usize>,
    pub by_area: HashMap<String, usize>,
}

impl ClassificationReport {
    pub fn new() -> Self {
        Self {
            total: 0,
            by_type: HashMap::new(),
            by_priority: HashMap::new(),
            by_area: HashMap::new(),
        }
    }

    pub fn print_terminal(&self) {
        println!("\n{}", "═".repeat(60).cyan());
        println!("{}", " Classification Report ".cyan().bold());
        println!("{}", "═".repeat(60).cyan());

        println!("\n{}", "By Type".yellow().bold());
        for (typ, count) in &self.by_type {
            let pct = (*count as f64 / self.total as f64) * 100.0;
            println!("  {:>12}: {:>4} ({:.1}%)", typ, count, pct);
        }

        println!("\n{}", "By Priority".yellow().bold());
        for priority in &["P0", "P1", "P2", "P3"] {
            let count = self.by_priority.get(*priority).unwrap_or(&0);
            let pct = (*count as f64 / self.total as f64) * 100.0;
            let colored = match *priority {
                "P0" => format!("{:>4}", count).red(),
                "P1" => format!("{:>4}", count).yellow(),
                "P2" => format!("{:>4}", count).white(),
                _ => format!("{:>4}", count).cyan(),
            };
            println!("  {}: {} ({:.1}%)", priority, colored, pct);
        }

        println!("\n{}", "By Feature Area".yellow().bold());
        let mut areas: Vec<_> = self.by_area.iter().collect();
        areas.sort_by(|a, b| b.1.cmp(a.1));
        for (area, count) in areas {
            let pct = (*count as f64 / self.total as f64) * 100.0;
            println!("  {:>12}: {:>4} ({:.1}%)", area, count, pct);
        }

        println!("\n{}", "═".repeat(60).cyan());
    }
}

impl Default for AnalysisReport {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for ClassificationReport {
    fn default() -> Self {
        Self::new()
    }
}
