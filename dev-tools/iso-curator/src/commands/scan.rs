//! Scan command - scans codebase for ISO compliance implementations
//!
//! This command analyzes source files to detect which ISO requirements
//! have been implemented and updates the curated matrix accordingly.

use anyhow::Result;
use colored::Colorize;
use regex::Regex;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// Implementation detection result
#[derive(Debug, Clone)]
pub struct ImplementationMatch {
    pub file_path: String,
    pub line_number: usize,
    pub iso_section: String,
    pub context: String,
    pub confidence: f64,
}

/// Scan results summary
#[derive(Debug, Default)]
pub struct ScanResults {
    pub total_files_scanned: usize,
    pub files_with_matches: usize,
    pub total_matches: usize,
    pub matches_by_section: HashMap<String, Vec<ImplementationMatch>>,
    pub unique_sections: usize,
}

/// Run the scan command
pub fn run(source_path: &str, output: Option<String>, verbose: bool) -> Result<()> {
    println!("{}", "Scanning codebase for ISO implementations...".cyan());

    let source = Path::new(source_path);
    if !source.exists() {
        anyhow::bail!("Source path does not exist: {}", source_path);
    }

    let mut results = ScanResults::default();

    // Scan all Rust files
    scan_directory(source, &mut results, verbose)?;

    // Print results
    print_results(&results);

    // Export if requested
    if let Some(output_path) = output {
        export_results(&results, &output_path)?;
        println!(
            "\n{} {}",
            "Results saved to:".green(),
            output_path.white()
        );
    }

    Ok(())
}

/// Recursively scan directory for Rust files
fn scan_directory(dir: &Path, results: &mut ScanResults, verbose: bool) -> Result<()> {
    if dir.is_file() {
        if dir.extension().map_or(false, |ext| ext == "rs") {
            scan_file(dir, results, verbose)?;
        }
        return Ok(());
    }

    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        // Skip target directory and hidden directories
        let name = path.file_name().unwrap_or_default().to_string_lossy();
        if name.starts_with('.') || name == "target" {
            continue;
        }

        if path.is_dir() {
            scan_directory(&path, results, verbose)?;
        } else if path.extension().map_or(false, |ext| ext == "rs") {
            scan_file(&path, results, verbose)?;
        }
    }

    Ok(())
}

/// Scan a single Rust file for ISO references
fn scan_file(path: &Path, results: &mut ScanResults, verbose: bool) -> Result<()> {
    let content = fs::read_to_string(path)?;
    results.total_files_scanned += 1;

    let matches = find_iso_references(&content, path);

    if !matches.is_empty() {
        results.files_with_matches += 1;
        results.total_matches += matches.len();

        if verbose {
            println!(
                "  {} - {} matches",
                path.display().to_string().dimmed(),
                matches.len()
            );
        }

        for m in matches {
            let section_key = extract_main_section(&m.iso_section);
            results
                .matches_by_section
                .entry(section_key)
                .or_default()
                .push(m);
        }
    }

    Ok(())
}

/// Find ISO references in file content
fn find_iso_references(content: &str, path: &Path) -> Vec<ImplementationMatch> {
    let mut matches = Vec::new();
    let path_str = path.display().to_string();

    // Pre-compiled patterns to detect ISO references (simpler patterns for performance)
    let section_pattern = Regex::new(r"(?i)section\s+(\d+(?:\.\d+)*)").unwrap();
    let iso_pattern = Regex::new(r"(?i)ISO\s*32000.*?(\d+\.\d+)").unwrap();
    let implements_pattern = Regex::new(r"(?i)implements?\s+.*?(\d+\.\d+)").unwrap();

    for (line_num, line) in content.lines().enumerate() {
        // Check for section references
        for cap in section_pattern.captures_iter(line) {
            if let Some(section) = cap.get(1) {
                let section_str = section.as_str().to_string();
                if is_valid_iso_section(&section_str) {
                    matches.push(ImplementationMatch {
                        file_path: path_str.clone(),
                        line_number: line_num + 1,
                        iso_section: section_str,
                        context: truncate_context(line, 100),
                        confidence: 0.85,
                    });
                }
            }
        }

        // Check for ISO references
        for cap in iso_pattern.captures_iter(line) {
            if let Some(section) = cap.get(1) {
                let section_str = section.as_str().to_string();
                if is_valid_iso_section(&section_str) {
                    matches.push(ImplementationMatch {
                        file_path: path_str.clone(),
                        line_number: line_num + 1,
                        iso_section: section_str,
                        context: truncate_context(line, 100),
                        confidence: 0.9,
                    });
                }
            }
        }

        // Check for implements references
        for cap in implements_pattern.captures_iter(line) {
            if let Some(section) = cap.get(1) {
                let section_str = section.as_str().to_string();
                if is_valid_iso_section(&section_str) {
                    matches.push(ImplementationMatch {
                        file_path: path_str.clone(),
                        line_number: line_num + 1,
                        iso_section: section_str,
                        context: truncate_context(line, 100),
                        confidence: 0.95,
                    });
                }
            }
        }
    }

    // Also detect implicit implementations based on module/struct names
    let implicit_matches = detect_implicit_implementations(content, &path_str);
    matches.extend(implicit_matches);

    matches
}

/// Detect implicit ISO implementations based on naming conventions
fn detect_implicit_implementations(content: &str, path: &str) -> Vec<ImplementationMatch> {
    let mut matches = Vec::new();

    // Map of keywords to ISO sections
    let keyword_sections = [
        // Section 7 - Syntax
        ("xref", "7.5.4"),
        ("trailer", "7.5.5"),
        ("cross_reference", "7.5.4"),
        ("object_stream", "7.5.7"),
        ("linearized", "7.5.8"),
        ("header", "7.5.2"),
        ("catalog", "7.7.2"),
        // Section 8 - Graphics
        ("graphics_state", "8.4"),
        ("color_space", "8.6"),
        ("pattern", "8.7"),
        ("shading", "8.7.4"),
        ("transparency", "8.5"),
        ("soft_mask", "8.5.4"),
        ("form_xobject", "8.10"),
        // Section 9 - Text
        ("font", "9.5"),
        ("cmap", "9.10"),
        ("encoding", "9.6"),
        ("text_state", "9.3"),
        // Section 10 - Rendering
        ("rendering", "10"),
        // Section 11 - Interactive
        ("annotation", "12.5"),
        ("form_field", "12.7"),
        ("acroform", "12.7"),
        // Section 12 - Document Structure
        ("outline", "12.3.3"),
        ("name_tree", "7.9.6"),
        ("page_label", "12.4.2"),
    ];

    for (line_num, line) in content.lines().enumerate() {
        // Check struct/impl/mod definitions
        if line.contains("struct ") || line.contains("impl ") || line.contains("mod ") {
            let line_lower = line.to_lowercase();
            for (keyword, section) in &keyword_sections {
                if line_lower.contains(keyword) {
                    matches.push(ImplementationMatch {
                        file_path: path.to_string(),
                        line_number: line_num + 1,
                        iso_section: section.to_string(),
                        context: truncate_context(line, 100),
                        confidence: 0.6, // Lower confidence for implicit matches
                    });
                    break; // Only one match per line
                }
            }
        }
    }

    matches
}

/// Check if section number is valid for ISO 32000-1
fn is_valid_iso_section(section: &str) -> bool {
    // ISO 32000-1 has sections 7-14
    if let Some(main) = section.split('.').next() {
        if let Ok(num) = main.parse::<u32>() {
            return (7..=14).contains(&num);
        }
    }
    false
}

/// Extract main section number
fn extract_main_section(section: &str) -> String {
    section
        .split('.')
        .next()
        .unwrap_or("unknown")
        .to_string()
}

/// Truncate context line
fn truncate_context(s: &str, max_len: usize) -> String {
    let s = s.trim();
    if s.chars().count() <= max_len {
        s.to_string()
    } else {
        let truncated: String = s.chars().take(max_len - 3).collect();
        format!("{}...", truncated)
    }
}

/// Print scan results to terminal
fn print_results(results: &ScanResults) {
    println!("\n{}", "═".repeat(60).cyan());
    println!("{}", " Implementation Scan Results ".cyan().bold());
    println!("{}", "═".repeat(60).cyan());

    println!("\n{}", "Summary".yellow().bold());
    println!("  Files scanned: {}", results.total_files_scanned);
    println!("  Files with matches: {}", results.files_with_matches);
    println!("  Total matches: {}", results.total_matches);

    // Count unique sections
    let unique_sections: std::collections::HashSet<_> = results
        .matches_by_section
        .values()
        .flat_map(|matches| matches.iter().map(|m| &m.iso_section))
        .collect();

    println!("  Unique ISO sections: {}", unique_sections.len());

    println!("\n{}", "Matches by Main Section".yellow().bold());
    let mut sections: Vec<_> = results.matches_by_section.iter().collect();
    sections.sort_by(|a, b| a.0.cmp(b.0));

    for (section, matches) in &sections {
        // Count high-confidence matches
        let high_conf = matches.iter().filter(|m| m.confidence >= 0.8).count();
        let total = matches.len();

        let confidence_color = if high_conf > total / 2 {
            format!("{}/{}", high_conf, total).green()
        } else {
            format!("{}/{}", high_conf, total).yellow()
        };

        println!(
            "  Section {:>2}: {:>4} matches ({} high confidence)",
            section, total, confidence_color
        );
    }

    // Top files with most matches
    println!("\n{}", "Top Files".yellow().bold());
    let mut file_counts: HashMap<&str, usize> = HashMap::new();
    for matches in results.matches_by_section.values() {
        for m in matches {
            *file_counts.entry(&m.file_path).or_insert(0) += 1;
        }
    }

    let mut files: Vec<_> = file_counts.iter().collect();
    files.sort_by(|a, b| b.1.cmp(a.1));

    for (file, count) in files.iter().take(10) {
        // Shorten path for display
        let short_path = file
            .replace("oxidize-pdf-core/src/", "")
            .replace("oxidize-pdf-core/", "");
        println!("  {:>4} - {}", count, short_path.dimmed());
    }

    println!("\n{}", "═".repeat(60).cyan());
}

/// Export results to JSON
fn export_results(results: &ScanResults, path: &str) -> Result<()> {
    #[derive(serde::Serialize)]
    struct ExportMatch {
        file_path: String,
        line_number: usize,
        iso_section: String,
        context: String,
        confidence: f64,
    }

    #[derive(serde::Serialize)]
    struct ExportResults {
        summary: ExportSummary,
        matches: Vec<ExportMatch>,
    }

    #[derive(serde::Serialize)]
    struct ExportSummary {
        files_scanned: usize,
        files_with_matches: usize,
        total_matches: usize,
    }

    let mut all_matches: Vec<ExportMatch> = Vec::new();
    for matches in results.matches_by_section.values() {
        for m in matches {
            all_matches.push(ExportMatch {
                file_path: m.file_path.clone(),
                line_number: m.line_number,
                iso_section: m.iso_section.clone(),
                context: m.context.clone(),
                confidence: m.confidence,
            });
        }
    }

    // Sort by section then file
    all_matches.sort_by(|a, b| {
        a.iso_section
            .cmp(&b.iso_section)
            .then(a.file_path.cmp(&b.file_path))
    });

    let export = ExportResults {
        summary: ExportSummary {
            files_scanned: results.total_files_scanned,
            files_with_matches: results.files_with_matches,
            total_matches: results.total_matches,
        },
        matches: all_matches,
    };

    let json = serde_json::to_string_pretty(&export)?;
    fs::write(path, json)?;

    Ok(())
}
