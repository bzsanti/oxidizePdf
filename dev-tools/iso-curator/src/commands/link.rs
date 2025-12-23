//! Link command - links scan results to curated matrix requirements
//!
//! This command updates the curated ISO compliance matrix with implementation
//! references discovered by the scan command.

use anyhow::Result;
use colored::Colorize;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;

/// Scan match from JSON export
#[derive(Debug, Clone, Deserialize)]
struct ScanMatch {
    file_path: String,
    line_number: usize,
    iso_section: String,
    context: String,
    confidence: f64,
}

/// Scan results from JSON export
#[derive(Debug, Deserialize)]
struct ScanResults {
    summary: ScanSummary,
    matches: Vec<ScanMatch>,
}

#[derive(Debug, Deserialize)]
struct ScanSummary {
    files_scanned: usize,
    files_with_matches: usize,
    total_matches: usize,
}

/// Curated requirement structure (matching TOML format)
#[derive(Debug, Clone, Serialize, Deserialize)]
struct CuratedRequirement {
    id: String,
    name: String,
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
    test_refs: Vec<String>,
    #[serde(default)]
    verification_level: u8,
    #[serde(default)]
    consolidates: Vec<String>,
}

/// Feature area section in curated matrix
#[derive(Debug, Clone, Serialize, Deserialize)]
struct FeatureAreaSection {
    requirements: Vec<CuratedRequirement>,
}

/// Curated matrix metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
struct CuratedMetadata {
    version: String,
    curation_date: String,
    original_count: u32,
    curated_count: u32,
    reduction_ratio: f64,
}

/// Run the link command
pub fn run(
    scan_results_path: &str,
    curated_matrix_path: &str,
    output: Option<String>,
    min_confidence: f64,
) -> Result<()> {
    println!("{}", "Linking scan results to curated matrix...".cyan());

    // Load scan results
    let scan_content = fs::read_to_string(scan_results_path)?;
    let scan_results: ScanResults = serde_json::from_str(&scan_content)?;

    println!(
        "  Loaded {} scan matches from {}",
        scan_results.matches.len(),
        scan_results_path
    );

    // Load curated matrix
    let matrix_content = fs::read_to_string(curated_matrix_path)?;
    let mut matrix_value: toml::Value = toml::from_str(&matrix_content)?;

    // Build multiple indexes for matching
    // 1. By main section (7, 8, 9, etc.)
    let mut main_section_matches: HashMap<String, Vec<&ScanMatch>> = HashMap::new();
    // 2. By file path keywords (for feature area matching)
    let mut file_keyword_matches: HashMap<String, Vec<&ScanMatch>> = HashMap::new();

    for m in &scan_results.matches {
        if m.confidence >= min_confidence {
            // Main section index
            let main_section = m.iso_section.split('.').next().unwrap_or("").to_string();
            main_section_matches
                .entry(main_section)
                .or_default()
                .push(m);

            // File keyword index
            let keywords = extract_file_keywords(&m.file_path);
            for keyword in keywords {
                file_keyword_matches.entry(keyword).or_default().push(m);
            }
        }
    }

    println!(
        "  {} main sections, {} file keywords (confidence >= {:.0}%)",
        main_section_matches.len(),
        file_keyword_matches.len(),
        min_confidence * 100.0
    );

    // Track statistics
    let mut linked_count = 0;
    let mut total_refs_added = 0;
    let mut already_implemented = 0;

    // Feature area to file keyword mapping
    let area_keywords: HashMap<&str, Vec<&str>> = [
        ("parser", vec!["parser", "xref", "trailer", "header", "lexer"]),
        ("writer", vec!["writer", "pdf_writer"]),
        ("graphics", vec!["graphics", "color", "pattern", "shading", "transparency", "image"]),
        ("fonts", vec!["font", "cmap", "encoding", "ttf", "truetype"]),
        ("text", vec!["text", "extraction", "layout"]),
        ("content", vec!["content", "stream", "operation"]),
        ("encryption", vec!["encryption", "aes", "rc4", "crypt"]),
        ("doc_metadata", vec!["metadata", "xmp"]),
        ("metadata", vec!["metadata", "xmp"]),
        ("interactive", vec!["annotation", "form", "action", "outline", "destination"]),
        ("advanced", vec!["structure", "tagged", "marked_content"]),
    ]
    .iter()
    .cloned()
    .collect();

    // Process each feature area
    let table = matrix_value
        .as_table_mut()
        .expect("Matrix should be a table");

    for (area_name, area_value) in table.iter_mut() {
        if area_name == "metadata" {
            continue;
        }

        if let Some(area_table) = area_value.as_table_mut() {
            if let Some(requirements) = area_table.get_mut("requirements") {
                if let Some(req_array) = requirements.as_array_mut() {
                    for req_value in req_array.iter_mut() {
                        if let Some(req_table) = req_value.as_table_mut() {
                            // Get requirement info
                            let iso_section = req_table
                                .get("iso_section")
                                .and_then(|v| v.as_str())
                                .unwrap_or("");
                            let feature_area = req_table
                                .get("feature_area")
                                .and_then(|v| v.as_str())
                                .unwrap_or("");

                            // Check if already implemented
                            let was_implemented = req_table
                                .get("implemented")
                                .and_then(|v| v.as_bool())
                                .unwrap_or(false);

                            if was_implemented {
                                already_implemented += 1;
                                continue;
                            }

                            // Find matching implementations using multiple strategies
                            let mut matched_refs: Vec<&ScanMatch> = Vec::new();

                            // Strategy 1: Match by main ISO section number
                            let main_section = iso_section.split('.').next().unwrap_or("");
                            if let Some(matches) = main_section_matches.get(main_section) {
                                // Filter by feature area keywords
                                if let Some(keywords) = area_keywords.get(feature_area) {
                                    for m in matches.iter() {
                                        if keywords.iter().any(|k| m.file_path.contains(k)) {
                                            matched_refs.push(*m);
                                        }
                                    }
                                }
                            }

                            // Strategy 2: Direct feature area keyword match
                            if matched_refs.is_empty() {
                                if let Some(keywords) = area_keywords.get(feature_area) {
                                    for keyword in keywords {
                                        if let Some(matches) = file_keyword_matches.get(*keyword) {
                                            matched_refs.extend(matches.iter().take(3).copied()); // Limit to top 3
                                        }
                                    }
                                }
                            }

                            // Deduplicate and limit
                            matched_refs.sort_by(|a, b| {
                                b.confidence.partial_cmp(&a.confidence).unwrap()
                            });
                            matched_refs.dedup_by(|a, b| {
                                a.file_path == b.file_path && a.line_number == b.line_number
                            });
                            matched_refs.truncate(5); // Max 5 refs per requirement

                            if !matched_refs.is_empty() {
                                // Add implementation references
                                let refs: Vec<String> = matched_refs
                                    .iter()
                                    .map(|m| {
                                        let short_path =
                                            m.file_path.replace("../../oxidize-pdf-core/src/", "");
                                        format!("{}:{}", short_path, m.line_number)
                                    })
                                    .collect();

                                // Update the requirement
                                req_table.insert(
                                    "implemented".to_string(),
                                    toml::Value::Boolean(true),
                                );
                                req_table.insert(
                                    "implementation_refs".to_string(),
                                    toml::Value::Array(
                                        refs.iter()
                                            .map(|s| toml::Value::String(s.clone()))
                                            .collect(),
                                    ),
                                );

                                // Set verification level based on confidence
                                let max_confidence = matched_refs
                                    .iter()
                                    .map(|m| m.confidence)
                                    .fold(0.0f64, |a, b| a.max(b));
                                let verification_level = if max_confidence >= 0.9 {
                                    3
                                } else if max_confidence >= 0.7 {
                                    2
                                } else {
                                    1
                                };
                                req_table.insert(
                                    "verification_level".to_string(),
                                    toml::Value::Integer(verification_level),
                                );

                                linked_count += 1;
                                total_refs_added += refs.len();
                            }
                        }
                    }
                }
            }
        }
    }

    // Print results
    println!("\n{}", "Linking Results".yellow().bold());
    println!("  Already implemented: {}", already_implemented);
    println!(
        "  Newly linked: {} {}",
        linked_count.to_string().green(),
        "requirements"
    );
    println!("  Total refs added: {}", total_refs_added);

    // Calculate new compliance percentage
    let total_requirements = count_total_requirements(&matrix_value);
    let implemented_count = already_implemented + linked_count;
    let compliance_pct = (implemented_count as f64 / total_requirements as f64) * 100.0;

    println!("\n{}", "Compliance Status".yellow().bold());
    println!(
        "  Total requirements: {}",
        total_requirements.to_string().white()
    );
    println!(
        "  Implemented: {} ({:.1}%)",
        implemented_count.to_string().green(),
        compliance_pct
    );
    println!(
        "  Not implemented: {}",
        (total_requirements - implemented_count).to_string().red()
    );

    // Write output
    let output_path = output.unwrap_or_else(|| curated_matrix_path.to_string());
    let output_content = toml::to_string_pretty(&matrix_value)?;
    fs::write(&output_path, output_content)?;

    println!(
        "\n{} {}",
        "Updated matrix saved to:".green(),
        output_path.white()
    );

    Ok(())
}

/// Extract keywords from file path for matching
fn extract_file_keywords(path: &str) -> Vec<String> {
    let mut keywords = Vec::new();

    // Extract filename without extension
    if let Some(filename) = path.split('/').last() {
        let name = filename.trim_end_matches(".rs");
        keywords.push(name.to_lowercase());

        // Split by underscore for compound names
        for part in name.split('_') {
            if part.len() > 2 {
                keywords.push(part.to_lowercase());
            }
        }
    }

    // Extract directory names
    for part in path.split('/') {
        if !part.is_empty() && !part.ends_with(".rs") && part != "src" {
            keywords.push(part.to_lowercase());
        }
    }

    keywords
}

/// Normalize section number for matching
#[allow(dead_code)]
fn normalize_section(section: &str) -> String {
    // Extract main section and first subsection (e.g., "7.5" from "7.5.4")
    let parts: Vec<&str> = section.split('.').collect();
    if parts.len() >= 2 {
        format!("{}.{}", parts[0], parts[1])
    } else {
        section.to_string()
    }
}

/// Count total requirements in matrix
fn count_total_requirements(matrix: &toml::Value) -> usize {
    let mut count = 0;
    if let Some(table) = matrix.as_table() {
        for (key, value) in table {
            if key == "metadata" {
                continue;
            }
            if let Some(area) = value.as_table() {
                if let Some(reqs) = area.get("requirements") {
                    if let Some(arr) = reqs.as_array() {
                        count += arr.len();
                    }
                }
            }
        }
    }
    count
}
