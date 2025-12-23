//! Analyze command - validates requirements in the matrix

use anyhow::Result;
use colored::Colorize;
use std::path::Path;

use crate::curation::is_valid_requirement;
use crate::matrix::IsoMatrix;
use crate::report::{AnalysisReport, FragmentValidation};

/// Run the analyze command
pub fn run(
    matrix_path: &str,
    detailed: bool,
    output: Option<String>,
    filter: Option<String>,
) -> Result<()> {
    println!("{}", "Loading matrix...".cyan());

    let matrix = IsoMatrix::load(Path::new(matrix_path))?;
    let fragments = matrix.flatten();

    println!(
        "Loaded {} fragments from {} sections",
        fragments.len(),
        matrix.sections.len()
    );

    let mut report = AnalysisReport::new();
    let mut validations: Vec<FragmentValidation> = Vec::new();

    // Filter fragments if pattern provided
    let filtered: Vec<_> = if let Some(ref pattern) = filter {
        fragments
            .iter()
            .filter(|f| {
                f.description.contains(pattern)
                    || f.id.contains(pattern)
                    || f.iso_section.contains(pattern)
            })
            .collect()
    } else {
        fragments.iter().collect()
    };

    println!("{}", "Analyzing fragments...".cyan());

    for fragment in &filtered {
        let result = is_valid_requirement(&fragment.description);
        report.add_result(fragment, &result);

        if detailed || !result.is_valid {
            validations.push(FragmentValidation {
                id: fragment.id.clone(),
                section: fragment.iso_section.clone(),
                is_valid: result.is_valid,
                reason: result.reason.clone(),
                confidence: result.confidence,
                description_preview: truncate(&fragment.description, 80),
            });
        }
    }

    report.finalize();

    // Print to terminal
    report.print_terminal();

    // Show detailed results if requested
    if detailed {
        println!("\n{}", "Detailed Results".yellow().bold());
        println!("{}", "-".repeat(80));

        for v in &validations {
            let status = if v.is_valid {
                "VALID".green()
            } else {
                "INVALID".red()
            };
            println!(
                "[{}] {} ({})",
                status,
                v.id.white(),
                v.section.cyan()
            );
            println!("  Reason: {}", v.reason);
            println!("  Preview: {}", v.description_preview.dimmed());
            println!();
        }
    }

    // Export to JSON if requested
    if let Some(output_path) = output {
        let json = serde_json::to_string_pretty(&report)?;
        std::fs::write(&output_path, json)?;
        println!(
            "\n{} {}",
            "Report saved to:".green(),
            output_path.white()
        );
    }

    // Summary
    println!(
        "\n{}: {}/{} fragments are valid requirements ({:.1}% can be discarded)",
        "Result".green().bold(),
        report.valid_count,
        report.total_fragments,
        100.0 - report.valid_percentage
    );

    Ok(())
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
