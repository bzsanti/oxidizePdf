//! Character Encoding Analysis Tool
//!
//! Advanced analysis tool specifically designed to analyze character encoding issues
//! in PDF files. This tool helps identify patterns in encoding problems and provides
//! detailed recommendations for improvement.

use oxidize_pdf::parser::encoding::{
    CharacterDecoder, EncodingOptions, EncodingType, EnhancedDecoder,
};
use oxidize_pdf::parser::{ParseOptions, PdfReader};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;

/// Character encoding analysis report
#[derive(Debug, Clone)]
struct CharacterAnalysisReport {
    analysis_metadata: AnalysisMetadata,
    encoding_statistics: EncodingStatistics,
    character_issues: Vec<CharacterIssueDetail>,
    encoding_patterns: Vec<EncodingPattern>,
    file_analysis: Vec<FileEncodingAnalysis>,
    recommendations: Vec<String>,
}

#[derive(Debug, Clone)]
struct AnalysisMetadata {
    timestamp: String,
    total_files: usize,
    analysis_duration: f64,
    decoder_version: String,
}

#[derive(Debug, Clone)]
struct EncodingStatistics {
    total_encoding_errors: usize,
    utf8_issues: usize,
    latin1_issues: usize,
    windows1252_issues: usize,
    macroman_issues: usize,
    mixed_encoding_files: usize,
    successful_recoveries: usize,
    failed_recoveries: usize,
}

#[derive(Debug, Clone)]
struct CharacterIssueDetail {
    character_byte: u8,
    unicode_char: Option<char>,
    occurrence_count: usize,
    detected_encodings: Vec<EncodingType>,
    problem_files: Vec<String>,
    context_samples: Vec<String>,
}

#[derive(Debug, Clone)]
struct EncodingPattern {
    pattern_name: String,
    byte_sequence: Vec<u8>,
    likely_encodings: Vec<EncodingType>,
    occurrence_count: usize,
    resolution_success_rate: f64,
    affected_files: Vec<String>,
}

#[derive(Debug, Clone)]
struct FileEncodingAnalysis {
    filename: String,
    file_size: u64,
    detected_encodings: Vec<EncodingType>,
    encoding_issues_count: usize,
    most_problematic_bytes: Vec<u8>,
    resolution_outcome: ResolutionOutcome,
    confidence_score: f64,
}

#[derive(Debug, Clone)]
enum ResolutionOutcome {
    FullyResolved,
    PartiallyResolved { success_rate: f64 },
    Failed { reason: String },
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔍 Character Encoding Analysis Tool");
    println!("===================================\n");

    let args: Vec<String> = std::env::args().collect();
    let fixtures_path = if args.len() > 1 {
        PathBuf::from(&args[1])
    } else {
        PathBuf::from("../tests/fixtures")
    };

    if !fixtures_path.exists() {
        println!("⚠️  Fixtures directory not found: {:?}", fixtures_path);
        return Ok(());
    }

    let start_time = Instant::now();

    // Initialize analysis report
    let mut report = CharacterAnalysisReport {
        analysis_metadata: AnalysisMetadata {
            timestamp: chrono::Utc::now().to_rfc3339(),
            total_files: 0,
            analysis_duration: 0.0,
            decoder_version: "1.0.0".to_string(),
        },
        encoding_statistics: EncodingStatistics {
            total_encoding_errors: 0,
            utf8_issues: 0,
            latin1_issues: 0,
            windows1252_issues: 0,
            macroman_issues: 0,
            mixed_encoding_files: 0,
            successful_recoveries: 0,
            failed_recoveries: 0,
        },
        character_issues: Vec::new(),
        encoding_patterns: Vec::new(),
        file_analysis: Vec::new(),
        recommendations: Vec::new(),
    };

    // Find PDF files with character encoding issues
    let pdf_files = find_pdf_files(&fixtures_path)?;
    report.analysis_metadata.total_files = pdf_files.len();

    println!(
        "📁 Found {} PDF files to analyze for character encoding issues",
        pdf_files.len()
    );

    if pdf_files.is_empty() {
        println!("❌ No PDF files found. Please check the fixtures path.");
        return Ok(());
    }

    // Character issue tracking
    let mut character_frequency: HashMap<u8, CharacterIssueDetail> = HashMap::new();
    let mut encoding_pattern_tracker: HashMap<Vec<u8>, EncodingPattern> = HashMap::new();

    // Analyze each PDF for character encoding issues
    for (idx, path) in pdf_files.iter().enumerate() {
        if idx % 20 == 0 {
            println!(
                "📊 Progress: {}/{} ({:.1}%)",
                idx,
                pdf_files.len(),
                (idx as f64 / pdf_files.len() as f64) * 100.0
            );
        }

        let file_analysis = analyze_pdf_encoding(
            path,
            &mut character_frequency,
            &mut encoding_pattern_tracker,
        );

        // Update statistics
        match &file_analysis.resolution_outcome {
            ResolutionOutcome::FullyResolved => {
                report.encoding_statistics.successful_recoveries += 1
            }
            ResolutionOutcome::PartiallyResolved { .. } => {
                report.encoding_statistics.successful_recoveries += 1
            }
            ResolutionOutcome::Failed { .. } => report.encoding_statistics.failed_recoveries += 1,
        }

        report.encoding_statistics.total_encoding_errors += file_analysis.encoding_issues_count;
        report.file_analysis.push(file_analysis);
    }

    // Process character frequency data
    report.character_issues = character_frequency.into_values().collect();
    report
        .character_issues
        .sort_by(|a, b| b.occurrence_count.cmp(&a.occurrence_count));

    // Process encoding patterns
    report.encoding_patterns = encoding_pattern_tracker.into_values().collect();
    report
        .encoding_patterns
        .sort_by(|a, b| b.occurrence_count.cmp(&a.occurrence_count));

    // Calculate final statistics
    finalize_statistics(&mut report);

    // Generate comprehensive recommendations
    report.recommendations = generate_encoding_recommendations(&report);

    // Record analysis duration
    report.analysis_metadata.analysis_duration = start_time.elapsed().as_secs_f64();

    // Generate reports
    generate_json_report(&report)?;
    generate_markdown_report(&report)?;
    generate_character_map_report(&report)?;

    println!("\n✅ Character encoding analysis complete!");
    println!(
        "📊 Total encoding errors found: {}",
        report.encoding_statistics.total_encoding_errors
    );
    println!(
        "✅ Successful recoveries: {}",
        report.encoding_statistics.successful_recoveries
    );
    println!(
        "❌ Failed recoveries: {}",
        report.encoding_statistics.failed_recoveries
    );
    println!("📄 Reports generated:");
    println!("   - character_encoding_analysis.json");
    println!("   - character_encoding_report.md");
    println!("   - character_map_report.md");

    Ok(())
}

fn find_pdf_files(fixtures_path: &Path) -> Result<Vec<PathBuf>, Box<dyn std::error::Error>> {
    let mut pdf_files = Vec::new();

    if fixtures_path.exists() && fixtures_path.is_dir() {
        for entry in fs::read_dir(fixtures_path)? {
            let entry = entry?;
            let path = entry.path();
            if let Some(extension) = path.extension() {
                if extension.to_str() == Some("pdf") {
                    pdf_files.push(path);
                }
            }
        }
    }

    pdf_files.sort();
    Ok(pdf_files)
}

fn analyze_pdf_encoding(
    path: &Path,
    character_frequency: &mut HashMap<u8, CharacterIssueDetail>,
    pattern_tracker: &mut HashMap<Vec<u8>, EncodingPattern>,
) -> FileEncodingAnalysis {
    let filename = path.file_name().unwrap().to_string_lossy().to_string();
    let file_size = fs::metadata(path).map(|m| m.len()).unwrap_or(0);

    let mut analysis = FileEncodingAnalysis {
        filename: filename.clone(),
        file_size,
        detected_encodings: Vec::new(),
        encoding_issues_count: 0,
        most_problematic_bytes: Vec::new(),
        resolution_outcome: ResolutionOutcome::FullyResolved,
        confidence_score: 1.0,
    };

    // Try parsing with STRICT options first to catch real encoding errors
    let strict_options = ParseOptions::strict();
    let lenient_options = ParseOptions::lenient();

    // First try to open the file normally to handle file access
    let file = match std::fs::File::open(path) {
        Ok(file) => file,
        Err(e) => {
            analysis.resolution_outcome = ResolutionOutcome::Failed {
                reason: format!("File access error: {}", e),
            };
            analysis.confidence_score = 0.0;
            return analysis;
        }
    };

    // STRICT PARSING FIRST - to detect real encoding errors
    match PdfReader::new_with_options(file, strict_options) {
        Ok(_reader) => {
            // Successfully opened with strict parsing - file is clean
            analysis.resolution_outcome = ResolutionOutcome::FullyResolved;
            analysis.confidence_score = 1.0;
        }
        Err(strict_error) => {
            let strict_error_msg = strict_error.to_string();

            // Check if this is a character encoding error
            if is_character_encoding_error(&strict_error_msg) {
                analysis.encoding_issues_count += 1;

                // Extract problematic bytes from error message
                if let Some(byte) = extract_problematic_byte(&strict_error_msg) {
                    analysis.most_problematic_bytes.push(byte);

                    // Update character frequency tracking
                    let char_detail =
                        character_frequency
                            .entry(byte)
                            .or_insert_with(|| CharacterIssueDetail {
                                character_byte: byte,
                                unicode_char: None,
                                occurrence_count: 0,
                                detected_encodings: Vec::new(),
                                problem_files: Vec::new(),
                                context_samples: Vec::new(),
                            });

                    char_detail.occurrence_count += 1;
                    char_detail.problem_files.push(filename.clone());
                    char_detail.context_samples.push(strict_error_msg.clone());

                    // Try to decode the problematic byte with different encodings
                    analyze_byte_with_encodings(byte, char_detail);
                }

                // NOW TRY LENIENT PARSING to see if we can recover
                let file2 = match std::fs::File::open(path) {
                    Ok(f) => f,
                    Err(_) => {
                        analysis.resolution_outcome = ResolutionOutcome::Failed {
                            reason: "Cannot reopen file for lenient parsing".to_string(),
                        };
                        return analysis;
                    }
                };

                match PdfReader::new_with_options(file2, lenient_options) {
                    Ok(_lenient_reader) => {
                        // Lenient parsing succeeded - we can recover from this error
                        analysis.resolution_outcome = ResolutionOutcome::PartiallyResolved {
                            success_rate: 0.8, // Character encoding issues have good recovery rate
                        };
                        analysis.confidence_score = 0.8;
                    }
                    Err(_lenient_error) => {
                        // Even lenient parsing failed - this is a serious issue
                        analysis.resolution_outcome = ResolutionOutcome::Failed {
                            reason: format!(
                                "Both strict and lenient parsing failed: {}",
                                strict_error_msg
                            ),
                        };
                        analysis.confidence_score = 0.0;
                    }
                }

                // Attempt enhanced encoding recovery
                if let Ok(recovery_info) = attempt_encoding_recovery(path) {
                    analysis.detected_encodings = recovery_info.successful_encodings;
                    if recovery_info.success_rate > analysis.confidence_score {
                        analysis.confidence_score = recovery_info.success_rate;
                    }
                }
            } else {
                // Not a character encoding error - try lenient to see if it's recoverable
                let file2 = match std::fs::File::open(path) {
                    Ok(f) => f,
                    Err(_) => {
                        analysis.resolution_outcome = ResolutionOutcome::Failed {
                            reason: "Cannot reopen file for lenient parsing".to_string(),
                        };
                        return analysis;
                    }
                };

                match PdfReader::new_with_options(file2, lenient_options) {
                    Ok(_lenient_reader) => {
                        // Other type of error but lenient parsing works
                        analysis.resolution_outcome =
                            ResolutionOutcome::PartiallyResolved { success_rate: 0.7 };
                        analysis.confidence_score = 0.7;
                    }
                    Err(_lenient_error) => {
                        // Neither strict nor lenient work - serious structural issue
                        analysis.resolution_outcome = ResolutionOutcome::Failed {
                            reason: strict_error_msg,
                        };
                        analysis.confidence_score = 0.0;
                    }
                }
            }
        }
    }

    analysis
}

fn is_character_encoding_error(error_msg: &str) -> bool {
    let error_lower = error_msg.to_lowercase();
    error_lower.contains("unexpected character") 
        || error_lower.contains("invalid character")
        || error_lower.contains("encoding")
        || error_lower.contains("\\u{")
        || error_lower.contains("character:")
        || error_lower.contains("utf")
        || error_lower.contains("unicode")
        || error_lower.contains("non-printable")
        || error_lower.contains("control character")
        // Common problematic characters that indicate encoding issues
        || error_msg.contains('\u{0007}') // Bell character
        || error_msg.contains('\u{0080}') // Control character  
        || error_msg.contains('\u{008c}') // Latin-1 supplement
        || error_msg.contains('\u{0081}') // Control character
}

fn extract_problematic_byte(error_msg: &str) -> Option<u8> {
    // Try to extract byte value from error messages like "Unexpected character: \u{8c}"
    if let Some(start) = error_msg.find("\\u{") {
        let hex_part = &error_msg[start + 3..];
        if let Some(end) = hex_part.find('}') {
            let hex_str = &hex_part[..end];
            if let Ok(value) = u8::from_str_radix(hex_str, 16) {
                return Some(value);
            }
        }
    }

    // Try to extract from patterns like "character: X" where X is the actual char
    if let Some(start) = error_msg.find("character: ") {
        let char_part = &error_msg[start + 11..];
        if let Some(ch) = char_part.chars().next() {
            if ch as u32 <= 255 {
                return Some(ch as u8);
            }
        }
    }

    // Check for common problematic characters directly in the message
    for ch in error_msg.chars() {
        let ch_u32 = ch as u32;
        if (ch_u32 >= 0x80 && ch_u32 <= 0x9F) || // Control characters in Latin-1 range
           (ch_u32 == 0x07) || // Bell character
           (ch_u32 >= 0x00 && ch_u32 <= 0x1F && ch_u32 != 0x09 && ch_u32 != 0x0A && ch_u32 != 0x0D)
        {
            // Control chars except tab, LF, CR
            return Some(ch as u8);
        }
    }

    // Try hex patterns like "0x8c" or "8ch"
    if let Some(hex_match) = extract_hex_from_message(error_msg) {
        return Some(hex_match);
    }

    None
}

fn extract_hex_from_message(msg: &str) -> Option<u8> {
    // Look for patterns like "0x8c", "0X8C", or isolated hex digits
    if let Some(start) = msg.find("0x") {
        let hex_part = &msg[start + 2..];
        let hex_str: String = hex_part.chars().take(2).collect();
        if let Ok(value) = u8::from_str_radix(&hex_str, 16) {
            return Some(value);
        }
    }

    if let Some(start) = msg.find("0X") {
        let hex_part = &msg[start + 2..];
        let hex_str: String = hex_part.chars().take(2).collect();
        if let Ok(value) = u8::from_str_radix(&hex_str, 16) {
            return Some(value);
        }
    }

    None
}

fn analyze_byte_with_encodings(byte: u8, char_detail: &mut CharacterIssueDetail) {
    let decoder = EnhancedDecoder::new();
    let encodings_to_try = [
        EncodingType::Latin1,
        EncodingType::Windows1252,
        EncodingType::MacRoman,
        EncodingType::Utf8,
    ];

    for encoding in &encodings_to_try {
        if let Ok(text) = decoder.decode_with_encoding(&[byte], *encoding, true) {
            if let Some(ch) = text.chars().next() {
                if ch != '\u{FFFD}' {
                    // Not a replacement character
                    char_detail.detected_encodings.push(*encoding);
                    if char_detail.unicode_char.is_none() {
                        char_detail.unicode_char = Some(ch);
                    }
                }
            }
        }
    }
}

#[derive(Debug)]
struct RecoveryInfo {
    successful_encodings: Vec<EncodingType>,
    success_rate: f64,
}

fn attempt_encoding_recovery(path: &Path) -> Result<RecoveryInfo, Box<dyn std::error::Error>> {
    let decoder = EnhancedDecoder::new();
    let encodings_to_try = [
        EncodingType::Windows1252,
        EncodingType::Latin1,
        EncodingType::MacRoman,
        EncodingType::Utf8,
    ];

    let mut successful_encodings = Vec::new();
    let mut total_attempts = 0;
    let mut successful_attempts = 0;

    // Read first few KB of the file to test encoding
    if let Ok(content) = fs::read(path) {
        let sample = if content.len() > 4096 {
            &content[..4096]
        } else {
            &content
        };

        for encoding in &encodings_to_try {
            total_attempts += 1;
            let options = EncodingOptions {
                lenient_mode: true,
                preferred_encoding: Some(*encoding),
                max_replacements: 100,
                log_issues: false,
            };

            if let Ok(result) = decoder.decode(sample, &options) {
                let replacement_ratio = result.replacement_count as f64 / sample.len() as f64;
                if replacement_ratio < 0.1 {
                    // Less than 10% replacement characters
                    successful_encodings.push(*encoding);
                    successful_attempts += 1;
                }
            }
        }
    }

    let success_rate = if total_attempts > 0 {
        successful_attempts as f64 / total_attempts as f64
    } else {
        0.0
    };

    Ok(RecoveryInfo {
        successful_encodings,
        success_rate,
    })
}

fn finalize_statistics(report: &mut CharacterAnalysisReport) {
    // Count encoding-specific issues
    for char_issue in &report.character_issues {
        for encoding in &char_issue.detected_encodings {
            match encoding {
                EncodingType::Utf8 => report.encoding_statistics.utf8_issues += 1,
                EncodingType::Latin1 => report.encoding_statistics.latin1_issues += 1,
                EncodingType::Windows1252 => report.encoding_statistics.windows1252_issues += 1,
                EncodingType::MacRoman => report.encoding_statistics.macroman_issues += 1,
                EncodingType::Mixed => report.encoding_statistics.mixed_encoding_files += 1,
                _ => {}
            }
        }
    }
}

fn generate_encoding_recommendations(report: &CharacterAnalysisReport) -> Vec<String> {
    let mut recommendations = Vec::new();

    // Character encoding specific recommendations
    if report.encoding_statistics.windows1252_issues > 0 {
        recommendations.push("Implement robust Windows-1252 character mapping for smart quotes and special characters".to_string());
    }

    if report.encoding_statistics.latin1_issues > 0 {
        recommendations.push(
            "Add fallback Latin-1 encoding detection for European character sets".to_string(),
        );
    }

    if report.encoding_statistics.mixed_encoding_files > 0 {
        recommendations.push(
            "Implement multi-encoding detection for files with mixed character sets".to_string(),
        );
    }

    // Recovery success rate recommendations
    let recovery_rate = if report.encoding_statistics.total_encoding_errors > 0 {
        report.encoding_statistics.successful_recoveries as f64
            / report.encoding_statistics.total_encoding_errors as f64
    } else {
        1.0
    };

    if recovery_rate < 0.8 {
        recommendations.push(
            "Improve encoding recovery algorithms - current success rate is below 80%".to_string(),
        );
    }

    // Top character issues recommendations
    for (i, char_issue) in report.character_issues.iter().take(5).enumerate() {
        if char_issue.occurrence_count > 10 {
            recommendations.push(format!(
                "Priority {}: Address byte 0x{:02X} ({}) found in {} files",
                i + 1,
                char_issue.character_byte,
                char_issue.unicode_char.unwrap_or('?'),
                char_issue.problem_files.len()
            ));
        }
    }

    if recommendations.is_empty() {
        recommendations.push("Character encoding handling appears to be working well".to_string());
    }

    recommendations
}

fn generate_json_report(
    report: &CharacterAnalysisReport,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut json = String::new();
    json.push_str("{\n");
    json.push_str(&format!(
        "  \"timestamp\": \"{}\",\n",
        report.analysis_metadata.timestamp
    ));
    json.push_str(&format!(
        "  \"total_files\": {},\n",
        report.analysis_metadata.total_files
    ));
    json.push_str(&format!(
        "  \"analysis_duration\": {:.2},\n",
        report.analysis_metadata.analysis_duration
    ));

    json.push_str("  \"encoding_statistics\": {\n");
    json.push_str(&format!(
        "    \"total_encoding_errors\": {},\n",
        report.encoding_statistics.total_encoding_errors
    ));
    json.push_str(&format!(
        "    \"successful_recoveries\": {},\n",
        report.encoding_statistics.successful_recoveries
    ));
    json.push_str(&format!(
        "    \"failed_recoveries\": {},\n",
        report.encoding_statistics.failed_recoveries
    ));
    json.push_str(&format!(
        "    \"utf8_issues\": {},\n",
        report.encoding_statistics.utf8_issues
    ));
    json.push_str(&format!(
        "    \"windows1252_issues\": {},\n",
        report.encoding_statistics.windows1252_issues
    ));
    json.push_str(&format!(
        "    \"latin1_issues\": {}\n",
        report.encoding_statistics.latin1_issues
    ));
    json.push_str("  },\n");

    json.push_str(&format!(
        "  \"character_issues_count\": {},\n",
        report.character_issues.len()
    ));
    json.push_str(&format!(
        "  \"encoding_patterns_count\": {}\n",
        report.encoding_patterns.len()
    ));
    json.push_str("}\n");

    fs::write("character_encoding_analysis.json", json)?;
    Ok(())
}

fn generate_markdown_report(
    report: &CharacterAnalysisReport,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut output = String::new();

    output.push_str("# Character Encoding Analysis Report\n\n");
    output.push_str(&format!(
        "**Generated:** {}\n",
        report.analysis_metadata.timestamp
    ));
    output.push_str(&format!(
        "**Files Analyzed:** {}\n",
        report.analysis_metadata.total_files
    ));
    output.push_str(&format!(
        "**Analysis Duration:** {:.2}s\n\n",
        report.analysis_metadata.analysis_duration
    ));

    // Statistics Section
    output.push_str("## 📊 Encoding Statistics\n\n");
    output.push_str(&format!(
        "- **Total Encoding Errors:** {}\n",
        report.encoding_statistics.total_encoding_errors
    ));
    output.push_str(&format!(
        "- **Successful Recoveries:** {} ({:.1}%)\n",
        report.encoding_statistics.successful_recoveries,
        if report.encoding_statistics.total_encoding_errors > 0 {
            (report.encoding_statistics.successful_recoveries as f64
                / report.encoding_statistics.total_encoding_errors as f64)
                * 100.0
        } else {
            100.0
        }
    ));
    output.push_str(&format!(
        "- **Failed Recoveries:** {}\n",
        report.encoding_statistics.failed_recoveries
    ));
    output.push_str(&format!(
        "- **Windows-1252 Issues:** {}\n",
        report.encoding_statistics.windows1252_issues
    ));
    output.push_str(&format!(
        "- **Latin-1 Issues:** {}\n",
        report.encoding_statistics.latin1_issues
    ));
    output.push_str(&format!(
        "- **UTF-8 Issues:** {}\n\n",
        report.encoding_statistics.utf8_issues
    ));

    // Top Character Issues
    output.push_str("## 🔤 Most Problematic Characters\n\n");
    output.push_str("| Byte | Unicode Char | Count | Detected Encodings | Files Affected |\n");
    output.push_str("|------|--------------|-------|-------------------|----------------|\n");

    for char_issue in report.character_issues.iter().take(10) {
        let unicode_display = char_issue
            .unicode_char
            .map(|c| format!("{} ({})", c, c as u32))
            .unwrap_or_else(|| "Unknown".to_string());

        let encodings = char_issue
            .detected_encodings
            .iter()
            .map(|e| e.name())
            .collect::<Vec<_>>()
            .join(", ");

        output.push_str(&format!(
            "| 0x{:02X} | {} | {} | {} | {} |\n",
            char_issue.character_byte,
            unicode_display,
            char_issue.occurrence_count,
            if encodings.is_empty() {
                "None".to_string()
            } else {
                encodings
            },
            char_issue.problem_files.len()
        ));
    }

    output.push_str("\n");

    // Recommendations
    output.push_str("## 💡 Recommendations\n\n");
    for (i, rec) in report.recommendations.iter().enumerate() {
        output.push_str(&format!("{}. {}\n", i + 1, rec));
    }

    // Failed Files Section
    output.push_str("\n## ❌ Files with Encoding Issues\n\n");
    let failed_files: Vec<&FileEncodingAnalysis> = report
        .file_analysis
        .iter()
        .filter(|f| matches!(f.resolution_outcome, ResolutionOutcome::Failed { .. }))
        .collect();

    if failed_files.is_empty() {
        output.push_str("No files with unresolvable encoding issues found! ✅\n");
    } else {
        for file in failed_files.iter().take(20) {
            output.push_str(&format!("### {}\n", file.filename));
            output.push_str(&format!("- **Size:** {} bytes\n", file.file_size));
            output.push_str(&format!(
                "- **Encoding Issues:** {}\n",
                file.encoding_issues_count
            ));
            output.push_str(&format!(
                "- **Confidence:** {:.1}%\n",
                file.confidence_score * 100.0
            ));
            if let ResolutionOutcome::Failed { reason } = &file.resolution_outcome {
                output.push_str(&format!("- **Failure Reason:** {}\n", reason));
            }
            output.push_str("\n");
        }
    }

    fs::write("character_encoding_report.md", output)?;
    Ok(())
}

fn generate_character_map_report(
    report: &CharacterAnalysisReport,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut output = String::new();

    output.push_str("# Character Mapping Reference\n\n");
    output.push_str(
        "This reference shows how problematic bytes map to different character encodings.\n\n",
    );

    output.push_str("## 🗺️ Character Mapping Table\n\n");
    output.push_str("| Byte | Hex | Latin-1 | Windows-1252 | MacRoman | UTF-8 |\n");
    output.push_str("|------|-----|---------|---------------|----------|-------|\n");

    let decoder = EnhancedDecoder::new();

    // Focus on the most problematic characters
    for char_issue in report.character_issues.iter().take(20) {
        let byte = char_issue.character_byte;
        output.push_str(&format!("| {} | 0x{:02X} ", byte, byte));

        // Test each encoding
        let encodings = [
            EncodingType::Latin1,
            EncodingType::Windows1252,
            EncodingType::MacRoman,
            EncodingType::Utf8,
        ];

        for encoding in &encodings {
            let result = decoder
                .decode_with_encoding(&[byte], *encoding, true)
                .unwrap_or_else(|_| "❌".to_string());

            let display = if result.contains('\u{FFFD}') {
                "�".to_string()
            } else if result.is_empty() {
                "∅".to_string()
            } else {
                result.chars().next().unwrap_or('?').to_string()
            };

            output.push_str(&format!("| {} ", display));
        }

        output.push_str("|\n");
    }

    output.push_str("\n## 📋 Legend\n\n");
    output.push_str("- ❌ = Encoding not supported\n");
    output.push_str("- � = Replacement character (invalid)\n");
    output.push_str("- ∅ = Empty result\n");
    output.push_str("- Character shown = Successfully decoded\n");

    fs::write("character_map_report.md", output)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_problematic_byte() {
        assert_eq!(
            extract_problematic_byte("Unexpected character: \\u{8c}"),
            Some(0x8c)
        );
        assert_eq!(
            extract_problematic_byte("Unexpected character: \\u{80}"),
            Some(0x80)
        );
        assert_eq!(extract_problematic_byte("No character here"), None);
    }

    #[test]
    fn test_is_character_encoding_error() {
        assert!(is_character_encoding_error("Unexpected character: \\u{8c}"));
        assert!(is_character_encoding_error("Invalid character in stream"));
        assert!(is_character_encoding_error("Encoding error occurred"));
        assert!(!is_character_encoding_error("File not found"));
    }
}
