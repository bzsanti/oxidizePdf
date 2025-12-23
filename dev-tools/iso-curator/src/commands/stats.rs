//! Stats command - shows statistics about the matrix

use anyhow::Result;
use colored::Colorize;
use std::collections::HashMap;
use std::path::Path;

use crate::curation::is_valid_requirement;
use crate::matrix::IsoMatrix;

/// Run the stats command
pub fn run(matrix_path: &str, compare: Option<String>) -> Result<()> {
    println!("{}", "Loading matrix...".cyan());

    let matrix = IsoMatrix::load(Path::new(matrix_path))?;
    let fragments = matrix.flatten();

    println!("\n{}", "═".repeat(60).cyan());
    println!("{}", " ISO Compliance Matrix Statistics ".cyan().bold());
    println!("{}", "═".repeat(60).cyan());

    // Basic counts
    println!("\n{}", "Basic Counts".yellow().bold());
    println!("  Total fragments: {}", fragments.len());
    println!("  Sections: {}", matrix.sections.len());

    // By section
    println!("\n{}", "By ISO Section".yellow().bold());
    let mut by_section: HashMap<String, usize> = HashMap::new();
    for frag in &fragments {
        let main = frag.iso_section.split('.').next().unwrap_or("?").to_string();
        *by_section.entry(main).or_insert(0) += 1;
    }
    let mut sections: Vec<_> = by_section.iter().collect();
    sections.sort_by(|a, b| {
        let a_num: u32 = a.0.parse().unwrap_or(999);
        let b_num: u32 = b.0.parse().unwrap_or(999);
        a_num.cmp(&b_num)
    });
    for (section, count) in sections {
        let bar_len = (*count / 100).min(30);
        let bar = "█".repeat(bar_len);
        println!("  Section {:>2}: {:>4} {}", section, count, bar.green());
    }

    // By requirement type
    println!("\n{}", "By Requirement Type".yellow().bold());
    let mut by_type: HashMap<String, usize> = HashMap::new();
    for frag in &fragments {
        *by_type.entry(frag.requirement_type.clone()).or_insert(0) += 1;
    }
    for (typ, count) in &by_type {
        let pct = (*count as f64 / fragments.len() as f64) * 100.0;
        println!("  {:>12}: {:>4} ({:.1}%)", typ, count, pct);
    }

    // Validation preview
    println!("\n{}", "Validation Preview".yellow().bold());
    let sample_size = 500.min(fragments.len());
    let mut valid_count = 0;
    for frag in fragments.iter().take(sample_size) {
        if is_valid_requirement(&frag.description).is_valid {
            valid_count += 1;
        }
    }
    let valid_pct = (valid_count as f64 / sample_size as f64) * 100.0;
    println!(
        "  Sample of {} fragments: {} valid ({:.1}%)",
        sample_size, valid_count, valid_pct
    );
    println!(
        "  Estimated valid total: ~{}",
        (fragments.len() as f64 * valid_pct / 100.0) as usize
    );
    println!(
        "  Estimated invalid: ~{} ({:.1}% reduction possible)",
        (fragments.len() as f64 * (100.0 - valid_pct) / 100.0) as usize,
        100.0 - valid_pct
    );

    // Compare with curated if available
    if let Some(curated_path) = compare {
        println!("\n{}", "Comparison with Curated Matrix".yellow().bold());

        if Path::new(&curated_path).exists() {
            let curated = IsoMatrix::load(Path::new(&curated_path))?;
            let curated_count = curated.total_count();

            println!("  Original: {}", fragments.len());
            println!("  Curated: {}", curated_count);

            let reduction = 1.0 - (curated_count as f64 / fragments.len() as f64);
            println!("  Reduction: {:.1}%", reduction * 100.0);

            // Check if meets targets
            let meets_size = curated_count >= 200 && curated_count <= 500;
            let meets_reduction = reduction >= 0.9;

            println!("\n  Target Compliance:");
            println!(
                "    Size (200-500): {}",
                if meets_size {
                    "PASS".green()
                } else {
                    "FAIL".red()
                }
            );
            println!(
                "    Reduction (>90%): {}",
                if meets_reduction {
                    "PASS".green()
                } else {
                    "FAIL".red()
                }
            );
        } else {
            println!("  Curated matrix not found at: {}", curated_path);
            println!("  Run 'iso-curator consolidate' first to create it.");
        }
    }

    println!("\n{}", "═".repeat(60).cyan());

    Ok(())
}
