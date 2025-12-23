//! Consolidate command - groups and merges related fragments

use anyhow::Result;
use colored::Colorize;
use std::path::Path;

use crate::curation::consolidator::{consolidate_fragments, merge_group};
use crate::matrix::IsoMatrix;
use crate::report::generate_curated_toml;

/// Run the consolidate command
pub fn run(matrix_path: &str, interactive: bool, output: &str, dry_run: bool) -> Result<()> {
    println!("{}", "Loading matrix...".cyan());

    let matrix = IsoMatrix::load(Path::new(matrix_path))?;
    let fragments = matrix.flatten();

    println!(
        "Loaded {} fragments, starting consolidation...",
        fragments.len()
    );

    // Run consolidation
    let groups = consolidate_fragments(&fragments);

    println!(
        "\n{} {} consolidation groups",
        "Found".green(),
        groups.len()
    );

    // Statistics
    let total_consolidated: usize = groups.iter().map(|g| g.fragment_ids.len()).sum();
    let avg_per_group = if groups.is_empty() {
        0.0
    } else {
        total_consolidated as f64 / groups.len() as f64
    };

    println!(
        "  Fragments consolidated: {} (avg {:.1} per group)",
        total_consolidated, avg_per_group
    );

    // Show groups by section
    println!("\n{}", "Groups by Section".yellow().bold());
    let mut section_counts: std::collections::HashMap<String, usize> =
        std::collections::HashMap::new();
    for group in &groups {
        let main_section = group.section.split('.').next().unwrap_or("?");
        *section_counts.entry(main_section.to_string()).or_insert(0) += 1;
    }
    let mut sections: Vec<_> = section_counts.iter().collect();
    sections.sort_by(|a, b| a.0.cmp(b.0));
    for (section, count) in sections {
        println!("  Section {:>2}: {} groups", section, count);
    }

    // Interactive mode
    if interactive {
        println!("\n{}", "Interactive mode not yet implemented".yellow());
        println!("Use --dry-run to preview consolidations");
    }

    // Show preview if dry run
    if dry_run {
        println!("\n{}", "Preview (first 10 groups)".yellow().bold());
        println!("{}", "-".repeat(80));

        for (i, group) in groups.iter().take(10).enumerate() {
            let cohesion_color = if group.cohesion_score > 0.8 {
                format!("{:.2}", group.cohesion_score).green()
            } else if group.cohesion_score > 0.5 {
                format!("{:.2}", group.cohesion_score).yellow()
            } else {
                format!("{:.2}", group.cohesion_score).red()
            };

            println!(
                "\n{}. {} - {} (cohesion: {})",
                i + 1,
                group.section.cyan(),
                group.topic.white(),
                cohesion_color
            );
            println!(
                "   {} fragments, pages {}-{}",
                group.fragment_ids.len(),
                group.page_range.0,
                group.page_range.1
            );
            println!("   IDs: {}", group.fragment_ids.join(", ").dimmed());

            // Show first description preview
            if let Some(desc) = group.descriptions.first() {
                let preview = truncate(desc, 100);
                println!("   Preview: {}", preview.dimmed());
            }
        }

        if groups.len() > 10 {
            println!("\n  ... and {} more groups", groups.len() - 10);
        }
    }

    // Generate output if not dry run
    if !dry_run {
        println!("\n{}", "Generating curated requirements...".cyan());

        let curated: Vec<_> = groups.iter().map(merge_group).collect();

        println!(
            "  Generated {} curated requirements",
            curated.len()
        );

        // Calculate reduction
        let original = fragments.len();
        let reduction = 1.0 - (curated.len() as f64 / original as f64);
        println!(
            "  Reduction: {:.1}% ({} -> {})",
            reduction * 100.0,
            original,
            curated.len()
        );

        // Generate TOML
        let toml_content = generate_curated_toml(&curated);

        // Write output
        std::fs::write(output, &toml_content)?;
        println!(
            "\n{} {}",
            "Curated matrix saved to:".green(),
            output.white()
        );

        // Show distribution summary
        println!("\n{}", "Priority Distribution".yellow().bold());
        let mut by_priority: std::collections::HashMap<String, usize> =
            std::collections::HashMap::new();
        for req in &curated {
            *by_priority.entry(req.priority.clone()).or_insert(0) += 1;
        }
        for priority in &["P0", "P1", "P2", "P3"] {
            let count = by_priority.get(*priority).unwrap_or(&0);
            let pct = (*count as f64 / curated.len() as f64) * 100.0;
            println!("  {}: {} ({:.1}%)", priority, count, pct);
        }
    }

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
