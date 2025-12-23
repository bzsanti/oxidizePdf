//! Classify command - classifies requirements by type and priority

use anyhow::Result;
use colored::Colorize;
use std::path::Path;

use crate::curation::{assign_priority, classify_type, detect_feature_area, is_valid_requirement};
use crate::matrix::IsoMatrix;
use crate::report::ClassificationReport;

/// Run the classify command
pub fn run(matrix_path: &str, section: Option<String>, output: Option<String>) -> Result<()> {
    println!("{}", "Loading matrix...".cyan());

    let matrix = IsoMatrix::load(Path::new(matrix_path))?;
    let fragments = matrix.flatten();

    // Filter by section if provided
    let filtered: Vec<_> = if let Some(ref sec) = section {
        fragments
            .iter()
            .filter(|f| f.iso_section.starts_with(sec))
            .collect()
    } else {
        fragments.iter().collect()
    };

    println!(
        "Classifying {} fragments{}",
        filtered.len(),
        section
            .as_ref()
            .map(|s| format!(" (section {})", s))
            .unwrap_or_default()
    );

    let mut report = ClassificationReport::new();
    let mut classifications: Vec<FragmentClassification> = Vec::new();

    for fragment in &filtered {
        // Only classify valid requirements
        let validation = is_valid_requirement(&fragment.description);
        if !validation.is_valid {
            continue;
        }

        let req_type = classify_type(&fragment.description);
        let priority = assign_priority(&fragment.description);
        let area = detect_feature_area(&fragment.description);

        report.total += 1;
        *report.by_type.entry(req_type.to_string()).or_insert(0) += 1;
        *report.by_priority.entry(priority.to_string()).or_insert(0) += 1;
        *report.by_area.entry(area.to_string()).or_insert(0) += 1;

        classifications.push(FragmentClassification {
            id: fragment.id.clone(),
            section: fragment.iso_section.clone(),
            requirement_type: req_type.to_string(),
            priority: priority.to_string(),
            feature_area: area.to_string(),
            description_preview: truncate(&fragment.description, 60),
        });
    }

    // Print report
    report.print_terminal();

    // Show some examples by priority
    println!("\n{}", "Examples by Priority".yellow().bold());

    for priority in &["P0", "P1", "P2", "P3"] {
        let examples: Vec<_> = classifications
            .iter()
            .filter(|c| c.priority == *priority)
            .take(3)
            .collect();

        if !examples.is_empty() {
            let color = match *priority {
                "P0" => "Critical".red(),
                "P1" => "High".yellow(),
                "P2" => "Medium".white(),
                _ => "Low".cyan(),
            };
            println!("\n  {} ({}):", priority, color);
            for ex in examples {
                println!("    - [{}] {}", ex.id.dimmed(), ex.description_preview);
            }
        }
    }

    // Export if requested
    if let Some(output_path) = output {
        let export = ClassificationExport {
            report: report.clone(),
            classifications,
        };
        let json = serde_json::to_string_pretty(&export)?;
        std::fs::write(&output_path, json)?;
        println!(
            "\n{} {}",
            "Classification saved to:".green(),
            output_path.white()
        );
    }

    Ok(())
}

#[derive(Debug, Clone, serde::Serialize)]
struct FragmentClassification {
    id: String,
    section: String,
    requirement_type: String,
    priority: String,
    feature_area: String,
    description_preview: String,
}

#[derive(Debug, Clone, serde::Serialize)]
struct ClassificationExport {
    report: ClassificationReport,
    classifications: Vec<FragmentClassification>,
}

/// Truncate string to max length with ellipsis (UTF-8 safe)
fn truncate(s: &str, max_len: usize) -> String {
    let s = s.replace('\n', " ").replace('\r', "");
    if s.chars().count() <= max_len {
        s
    } else {
        let truncated: String = s.chars().take(max_len - 3).collect();
        format!("{}...", truncated)
    }
}
