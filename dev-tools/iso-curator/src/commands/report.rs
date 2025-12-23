//! Report command - generates compliance reports from the curated matrix
//!
//! This command analyzes the curated matrix and generates detailed
//! compliance reports with statistics by feature area, priority, and type.

use anyhow::Result;
use colored::Colorize;
use serde::Deserialize;
use std::collections::HashMap;
use std::fs;

/// Curated requirement from TOML
#[derive(Debug, Deserialize)]
struct CuratedRequirement {
    id: String,
    name: String,
    #[allow(dead_code)]
    description: String,
    iso_section: String,
    requirement_type: String,
    priority: String,
    feature_area: String,
    #[serde(default)]
    implemented: bool,
    #[serde(default)]
    implementation_refs: Vec<String>,
    #[serde(default)]
    verification_level: u8,
}

/// Feature area section
#[derive(Debug, Deserialize)]
struct FeatureArea {
    requirements: Vec<CuratedRequirement>,
}

/// Curated matrix metadata
#[derive(Debug, Deserialize)]
struct CuratedMetadata {
    version: String,
    curation_date: String,
    original_count: u32,
    curated_count: u32,
    reduction_ratio: f64,
}

/// Statistics for a category
#[derive(Debug, Default)]
struct CategoryStats {
    total: usize,
    implemented: usize,
    with_refs: usize,
    high_verification: usize,
}

impl CategoryStats {
    fn percentage(&self) -> f64 {
        if self.total > 0 {
            (self.implemented as f64 / self.total as f64) * 100.0
        } else {
            0.0
        }
    }
}

/// Run the report command
pub fn run(curated_path: &str, output: Option<String>, detailed: bool) -> Result<()> {
    println!("{}", "ISO 32000-1:2008 Compliance Report".cyan().bold());
    println!("{}", "═".repeat(60).cyan());

    // Load curated matrix
    let content = fs::read_to_string(curated_path)?;
    let matrix: toml::Value = toml::from_str(&content)?;

    // Extract metadata
    let metadata = matrix
        .get("metadata")
        .and_then(|m| toml::from_str::<CuratedMetadata>(&toml::to_string(m).unwrap_or_default()).ok());

    if let Some(meta) = &metadata {
        println!("\n{}", "Matrix Information".yellow().bold());
        println!("  Version: {}", meta.version);
        println!("  Curation date: {}", meta.curation_date);
        println!("  Original fragments: {}", meta.original_count);
        println!("  Curated requirements: {}", meta.curated_count);
        println!(
            "  Reduction: {:.1}%",
            meta.reduction_ratio * 100.0
        );
    }

    // Collect all requirements
    let mut all_requirements: Vec<CuratedRequirement> = Vec::new();
    let table = matrix.as_table().expect("Matrix should be a table");

    for (area_name, area_value) in table {
        if area_name == "metadata" {
            continue;
        }
        if let Ok(area) = toml::from_str::<FeatureArea>(&toml::to_string(area_value).unwrap_or_default()) {
            all_requirements.extend(area.requirements);
        }
    }

    // Calculate overall statistics
    let total = all_requirements.len();
    let implemented = all_requirements.iter().filter(|r| r.implemented).count();
    let with_refs = all_requirements
        .iter()
        .filter(|r| !r.implementation_refs.is_empty())
        .count();
    let high_verification = all_requirements
        .iter()
        .filter(|r| r.verification_level >= 3)
        .count();

    println!("\n{}", "Overall Compliance".yellow().bold());
    println!("  Total requirements: {}", total);
    println!(
        "  Implemented: {} ({:.1}%)",
        implemented.to_string().green(),
        (implemented as f64 / total as f64) * 100.0
    );
    println!(
        "  With code references: {} ({:.1}%)",
        with_refs.to_string().green(),
        (with_refs as f64 / total as f64) * 100.0
    );
    println!(
        "  High verification (level 3+): {} ({:.1}%)",
        high_verification.to_string().green(),
        (high_verification as f64 / total as f64) * 100.0
    );

    // Stats by priority
    let mut by_priority: HashMap<String, CategoryStats> = HashMap::new();
    for req in &all_requirements {
        let stats = by_priority.entry(req.priority.clone()).or_default();
        stats.total += 1;
        if req.implemented {
            stats.implemented += 1;
        }
        if !req.implementation_refs.is_empty() {
            stats.with_refs += 1;
        }
        if req.verification_level >= 3 {
            stats.high_verification += 1;
        }
    }

    println!("\n{}", "Compliance by Priority".yellow().bold());
    for priority in &["P0", "P1", "P2", "P3"] {
        if let Some(stats) = by_priority.get(*priority) {
            let label = match *priority {
                "P0" => "P0 (Critical)".red(),
                "P1" => "P1 (High)".yellow(),
                "P2" => "P2 (Medium)".white(),
                "P3" => "P3 (Low)".dimmed(),
                _ => priority.to_string().white(),
            };
            println!(
                "  {}: {}/{} ({:.1}%)",
                label,
                stats.implemented,
                stats.total,
                stats.percentage()
            );
        }
    }

    // Stats by feature area
    let mut by_area: HashMap<String, CategoryStats> = HashMap::new();
    for req in &all_requirements {
        let stats = by_area.entry(req.feature_area.clone()).or_default();
        stats.total += 1;
        if req.implemented {
            stats.implemented += 1;
        }
        if !req.implementation_refs.is_empty() {
            stats.with_refs += 1;
        }
        if req.verification_level >= 3 {
            stats.high_verification += 1;
        }
    }

    println!("\n{}", "Compliance by Feature Area".yellow().bold());
    let mut areas: Vec<_> = by_area.iter().collect();
    areas.sort_by(|a, b| b.1.total.cmp(&a.1.total));

    for (area, stats) in &areas {
        let pct = stats.percentage();
        let pct_color = if pct >= 90.0 {
            format!("{:.1}%", pct).green()
        } else if pct >= 70.0 {
            format!("{:.1}%", pct).yellow()
        } else {
            format!("{:.1}%", pct).red()
        };
        println!(
            "  {:15} {:>3}/{:>3} ({})",
            area, stats.implemented, stats.total, pct_color
        );
    }

    // Stats by requirement type
    let mut by_type: HashMap<String, CategoryStats> = HashMap::new();
    for req in &all_requirements {
        let stats = by_type.entry(req.requirement_type.clone()).or_default();
        stats.total += 1;
        if req.implemented {
            stats.implemented += 1;
        }
    }

    println!("\n{}", "Compliance by Requirement Type".yellow().bold());
    for req_type in &["mandatory", "optional", "recommendation", "recommended"] {
        if let Some(stats) = by_type.get(*req_type) {
            let label = match *req_type {
                "mandatory" => "Mandatory".red(),
                "optional" => "Optional".white(),
                "recommendation" | "recommended" => "Recommended".yellow(),
                _ => req_type.to_string().white(),
            };
            println!(
                "  {}: {}/{} ({:.1}%)",
                label,
                stats.implemented,
                stats.total,
                stats.percentage()
            );
        }
    }

    // Detailed output if requested
    if detailed {
        println!("\n{}", "Unimplemented Requirements".yellow().bold());
        let unimplemented: Vec<_> = all_requirements
            .iter()
            .filter(|r| !r.implemented)
            .collect();

        if unimplemented.is_empty() {
            println!("  {} All requirements are implemented!", "✓".green());
        } else {
            for req in unimplemented.iter().take(20) {
                println!(
                    "  {} {} - {} ({})",
                    "✗".red(),
                    req.id,
                    req.name,
                    req.priority
                );
            }
            if unimplemented.len() > 20 {
                println!("  ... and {} more", unimplemented.len() - 20);
            }
        }

        println!("\n{}", "Low Verification Level (<3)".yellow().bold());
        let low_verification: Vec<_> = all_requirements
            .iter()
            .filter(|r| r.verification_level < 3 && r.implemented)
            .collect();

        if low_verification.is_empty() {
            println!("  {} All implemented requirements have high verification!", "✓".green());
        } else {
            for req in low_verification.iter().take(10) {
                println!(
                    "  {} {} - {} (level {})",
                    "!".yellow(),
                    req.id,
                    req.name,
                    req.verification_level
                );
            }
            if low_verification.len() > 10 {
                println!("  ... and {} more", low_verification.len() - 10);
            }
        }
    }

    // Export to file if requested
    if let Some(output_path) = output {
        let report = generate_json_report(&all_requirements, metadata.as_ref())?;
        fs::write(&output_path, report)?;
        println!(
            "\n{} {}",
            "Report saved to:".green(),
            output_path.white()
        );
    }

    println!("\n{}", "═".repeat(60).cyan());

    Ok(())
}

/// Generate JSON report
fn generate_json_report(
    requirements: &[CuratedRequirement],
    metadata: Option<&CuratedMetadata>,
) -> Result<String> {
    #[derive(serde::Serialize)]
    struct Report {
        generated_at: String,
        matrix_version: String,
        summary: Summary,
        by_priority: HashMap<String, Stats>,
        by_feature_area: HashMap<String, Stats>,
        by_type: HashMap<String, Stats>,
    }

    #[derive(serde::Serialize)]
    struct Summary {
        total_requirements: usize,
        implemented: usize,
        implementation_percentage: f64,
        with_code_refs: usize,
        high_verification: usize,
    }

    #[derive(serde::Serialize, Default)]
    struct Stats {
        total: usize,
        implemented: usize,
        percentage: f64,
    }

    let total = requirements.len();
    let implemented = requirements.iter().filter(|r| r.implemented).count();
    let with_refs = requirements.iter().filter(|r| !r.implementation_refs.is_empty()).count();
    let high_verification = requirements.iter().filter(|r| r.verification_level >= 3).count();

    let mut by_priority: HashMap<String, Stats> = HashMap::new();
    let mut by_area: HashMap<String, Stats> = HashMap::new();
    let mut by_type: HashMap<String, Stats> = HashMap::new();

    for req in requirements {
        // Priority
        let stats = by_priority.entry(req.priority.clone()).or_default();
        stats.total += 1;
        if req.implemented {
            stats.implemented += 1;
        }

        // Feature area
        let stats = by_area.entry(req.feature_area.clone()).or_default();
        stats.total += 1;
        if req.implemented {
            stats.implemented += 1;
        }

        // Type
        let stats = by_type.entry(req.requirement_type.clone()).or_default();
        stats.total += 1;
        if req.implemented {
            stats.implemented += 1;
        }
    }

    // Calculate percentages
    for stats in by_priority.values_mut() {
        stats.percentage = if stats.total > 0 {
            (stats.implemented as f64 / stats.total as f64) * 100.0
        } else {
            0.0
        };
    }
    for stats in by_area.values_mut() {
        stats.percentage = if stats.total > 0 {
            (stats.implemented as f64 / stats.total as f64) * 100.0
        } else {
            0.0
        };
    }
    for stats in by_type.values_mut() {
        stats.percentage = if stats.total > 0 {
            (stats.implemented as f64 / stats.total as f64) * 100.0
        } else {
            0.0
        };
    }

    let report = Report {
        generated_at: chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC").to_string(),
        matrix_version: metadata.map(|m| m.version.clone()).unwrap_or_else(|| "unknown".to_string()),
        summary: Summary {
            total_requirements: total,
            implemented,
            implementation_percentage: if total > 0 {
                (implemented as f64 / total as f64) * 100.0
            } else {
                0.0
            },
            with_code_refs: with_refs,
            high_verification,
        },
        by_priority,
        by_feature_area: by_area,
        by_type,
    };

    Ok(serde_json::to_string_pretty(&report)?)
}
