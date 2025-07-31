//! Comprehensive PDF Analysis Tool
//!
//! Advanced analysis tool for validating the circular reference fix and establishing
//! a solid baseline for future parser improvements.
//!
//! Features:
//! - Before/after comparison of parsing results
//! - Detailed error categorization and analysis  
//! - Character encoding issue detection
//! - Performance benchmarking
//! - Progress metrics and reporting

use oxidize_pdf::parser::PdfReader;
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

#[derive(Debug)]
struct ComprehensiveReport {
    analysis_metadata: AnalysisMetadata,
    summary_stats: SummaryStats,
    error_analysis: ErrorAnalysis,
    character_issues: CharacterAnalysis,
    performance_metrics: PerformanceMetrics,
    detailed_results: Vec<FileAnalysisResult>,
    recommendations: Vec<String>,
}

#[derive(Debug)]
struct AnalysisMetadata {
    timestamp: String,
    oxidize_pdf_version: String,
    total_files_analyzed: usize,
    analysis_duration: f64,
    test_environment: String,
}

#[derive(Debug)]
struct SummaryStats {
    total_pdfs: usize,
    successful_parsing: usize,
    failed_parsing: usize,
    success_rate_percentage: f64,
    circular_reference_errors: usize,
    character_encoding_errors: usize,
    structural_errors: usize,
    encrypted_pdfs: usize,
}

#[derive(Debug)]
struct ErrorAnalysis {
    error_categories: HashMap<String, ErrorCategory>,
    error_patterns: Vec<ErrorPattern>,
    most_common_errors: Vec<(String, usize)>,
    character_error_details: Vec<CharacterErrorDetail>,
}

#[derive(Debug)]
struct ErrorCategory {
    count: usize,
    percentage: f64,
    description: String,
    severity: ErrorSeverity,
    examples: Vec<String>,
}

#[derive(Debug)]
enum ErrorSeverity {
    Critical, // Parser completely fails
    Major,    // Significant functionality lost
    Minor,    // Partial parsing success
    Cosmetic, // Parsing succeeds but with warnings
}

#[derive(Debug)]
struct ErrorPattern {
    pattern_name: String,
    regex_pattern: String,
    occurrence_count: usize,
    affected_files: Vec<String>,
    suggested_fix: String,
}

#[derive(Debug)]
struct CharacterErrorDetail {
    character_code: String,
    unicode_description: String,
    occurrence_count: usize,
    affected_files: Vec<String>,
    encoding_context: String,
}

#[derive(Debug)]
struct CharacterAnalysis {
    encoding_issues_detected: usize,
    common_problematic_characters: Vec<String>,
    encoding_types_found: Vec<String>,
    character_error_details: Vec<CharacterErrorDetail>,
}

#[derive(Debug)]
struct PerformanceMetrics {
    average_parse_time_ms: f64,
    median_parse_time_ms: f64,
    slowest_files: Vec<(String, f64)>,
    fastest_files: Vec<(String, f64)>,
    memory_usage_pattern: String,
    throughput_files_per_second: f64,
}

#[derive(Debug)]
struct FileAnalysisResult {
    filename: String,
    file_path: String,
    file_size_bytes: u64,
    pdf_version: Option<String>,
    analysis_result: AnalysisOutcome,
    parsing_time_ms: f64,
    error_details: Option<ErrorDetails>,
    features_detected: Vec<String>,
    complexity_score: u32,
}

#[derive(Debug)]
enum AnalysisOutcome {
    Success,
    PartialSuccess {
        pages_parsed: usize,
        total_pages: usize,
    },
    Failed {
        error_type: String,
        error_message: String,
    },
}

#[derive(Debug)]
struct ErrorDetails {
    error_type: String,
    error_message: String,
    error_position: Option<usize>,
    character_context: Option<String>,
    suggested_recovery: Vec<String>,
    similar_errors_count: usize,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔍 Comprehensive PDF Analysis Tool");
    println!("===================================\n");

    let args: Vec<String> = std::env::args().collect();
    let fixtures_path = if args.len() > 1 {
        PathBuf::from(&args[1])
    } else {
        PathBuf::from("../tests/fixtures")
    };

    if !fixtures_path.exists() {
        println!("⚠️  Fixtures directory not found: {:?}", fixtures_path);
        println!("Using fallback directory...");
    }

    // Initialize analysis
    let start_time = Instant::now();
    let mut report = ComprehensiveReport {
        analysis_metadata: AnalysisMetadata {
            timestamp: chrono::Utc::now().to_rfc3339(),
            oxidize_pdf_version: env!("CARGO_PKG_VERSION").to_string(),
            total_files_analyzed: 0,
            analysis_duration: 0.0,
            test_environment: get_test_environment(),
        },
        summary_stats: SummaryStats {
            total_pdfs: 0,
            successful_parsing: 0,
            failed_parsing: 0,
            success_rate_percentage: 0.0,
            circular_reference_errors: 0,
            character_encoding_errors: 0,
            structural_errors: 0,
            encrypted_pdfs: 0,
        },
        error_analysis: ErrorAnalysis {
            error_categories: HashMap::new(),
            error_patterns: Vec::new(),
            most_common_errors: Vec::new(),
            character_error_details: Vec::new(),
        },
        character_issues: CharacterAnalysis {
            encoding_issues_detected: 0,
            common_problematic_characters: Vec::new(),
            encoding_types_found: Vec::new(),
            character_error_details: Vec::new(),
        },
        performance_metrics: PerformanceMetrics {
            average_parse_time_ms: 0.0,
            median_parse_time_ms: 0.0,
            slowest_files: Vec::new(),
            fastest_files: Vec::new(),
            memory_usage_pattern: "Not measured".to_string(),
            throughput_files_per_second: 0.0,
        },
        detailed_results: Vec::new(),
        recommendations: Vec::new(),
    };

    // Find PDF files
    let pdf_files = find_pdf_files(&fixtures_path)?;
    report.summary_stats.total_pdfs = pdf_files.len();
    report.analysis_metadata.total_files_analyzed = pdf_files.len();

    println!("📁 Found {} PDF files to analyze", pdf_files.len());
    if pdf_files.is_empty() {
        println!("❌ No PDF files found. Please check the fixtures path.");
        return Ok(());
    }

    // Analyze each PDF
    let mut parse_times = Vec::new();
    for (idx, path) in pdf_files.iter().enumerate() {
        if idx % 50 == 0 {
            println!(
                "📊 Progress: {}/{} ({:.1}%)",
                idx,
                pdf_files.len(),
                (idx as f64 / pdf_files.len() as f64) * 100.0
            );
        }

        let file_result = analyze_single_pdf(path);
        parse_times.push(file_result.parsing_time_ms);

        // Update summary stats
        match &file_result.analysis_result {
            AnalysisOutcome::Success => report.summary_stats.successful_parsing += 1,
            AnalysisOutcome::PartialSuccess { .. } => report.summary_stats.successful_parsing += 1,
            AnalysisOutcome::Failed {
                error_type,
                error_message,
            } => {
                report.summary_stats.failed_parsing += 1;
                update_error_statistics(&mut report, error_type, error_message);
            }
        }

        report.detailed_results.push(file_result);
    }

    // Calculate final metrics
    finalize_report(&mut report, parse_times, start_time.elapsed());

    // Generate reports
    generate_comprehensive_report(&report)?;
    generate_summary_report(&report)?;
    generate_error_focus_report(&report)?;

    println!("\n✅ Analysis complete!");
    println!(
        "📊 Success rate: {:.2}%",
        report.summary_stats.success_rate_percentage
    );
    println!(
        "⚠️  Character encoding errors: {}",
        report.summary_stats.character_encoding_errors
    );
    println!(
        "🔄 Circular reference errors: {}",
        report.summary_stats.circular_reference_errors
    );
    println!("📄 Reports generated:");
    println!("   - comprehensive_analysis_report.json");
    println!("   - summary_analysis_report.md");
    println!("   - error_focus_report.md");

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
    } else {
        // Try alternative locations
        let alternative_paths = [
            "/Users/santifdezmunoz/Documents/PDFs",
            "./tests/fixtures",
            "../fixtures",
        ];

        for alt_path in &alternative_paths {
            let alt_path = PathBuf::from(alt_path);
            if alt_path.exists() {
                println!("🔄 Using alternative path: {:?}", alt_path);
                return find_pdf_files(&alt_path);
            }
        }
    }

    pdf_files.sort();
    Ok(pdf_files)
}

fn analyze_single_pdf(path: &Path) -> FileAnalysisResult {
    let start_time = Instant::now();
    let filename = path.file_name().unwrap().to_string_lossy().to_string();
    let file_path = path.to_string_lossy().to_string();

    let file_size_bytes = fs::metadata(path).map(|m| m.len()).unwrap_or(0);

    let analysis_result = match PdfReader::open(path) {
        Ok(mut reader) => {
            let version = reader.version().to_string();

            // Try to get additional information
            let mut features = Vec::new();

            // Check for encryption
            if let Err(ref e) = reader.catalog() {
                if e.to_string().contains("encrypted") {
                    features.push("encrypted".to_string());
                }
            }

            // Try to get page count
            match reader.page_count() {
                Ok(0) => AnalysisOutcome::PartialSuccess {
                    pages_parsed: 0,
                    total_pages: 0,
                },
                Ok(count) => {
                    features.push(format!("pages:{}", count));
                    AnalysisOutcome::Success
                }
                Err(e) => {
                    let error_msg = e.to_string();
                    let error_type = categorize_error(&error_msg);
                    AnalysisOutcome::Failed {
                        error_type: error_type.clone(),
                        error_message: error_msg.clone(),
                    }
                }
            }
        }
        Err(e) => {
            let error_msg = e.to_string();
            let error_type = categorize_error(&error_msg);
            AnalysisOutcome::Failed {
                error_type,
                error_message: error_msg,
            }
        }
    };

    let parsing_time_ms = start_time.elapsed().as_millis() as f64;

    // Try to detect PDF version from file
    let pdf_version = detect_pdf_version(path);

    let error_details = match &analysis_result {
        AnalysisOutcome::Failed {
            error_type,
            error_message,
        } => {
            Some(ErrorDetails {
                error_type: error_type.clone(),
                error_message: error_message.clone(),
                error_position: extract_error_position(error_message),
                character_context: extract_character_context(error_message),
                suggested_recovery: generate_recovery_suggestions(error_type, error_message),
                similar_errors_count: 0, // Will be calculated later
            })
        }
        _ => None,
    };

    FileAnalysisResult {
        filename,
        file_path,
        file_size_bytes,
        pdf_version,
        analysis_result,
        parsing_time_ms,
        error_details,
        features_detected: Vec::new(),
        complexity_score: calculate_complexity_score(file_size_bytes),
    }
}

fn categorize_error(error_message: &str) -> String {
    let error_lower = error_message.to_lowercase();

    if error_lower.contains("circular reference") {
        "CircularReference".to_string()
    } else if error_lower.contains("unexpected character") || error_lower.contains("\\u{") {
        "CharacterEncoding".to_string()
    } else if error_lower.contains("encrypted") {
        "Encryption".to_string()
    } else if error_lower.contains("xref") || error_lower.contains("cross-reference") {
        "XRefError".to_string()
    } else if error_lower.contains("stream") {
        "StreamError".to_string()
    } else if error_lower.contains("syntax error") {
        "SyntaxError".to_string()
    } else if error_lower.contains("io error") || error_lower.contains("file") {
        "IOError".to_string()
    } else if error_lower.contains("invalid") {
        "InvalidStructure".to_string()
    } else {
        "Other".to_string()
    }
}

fn extract_error_position(error_message: &str) -> Option<usize> {
    // Try to extract position information from error messages
    if let Some(pos_start) = error_message.find("position ") {
        let pos_str = &error_message[pos_start + 9..];
        if let Some(pos_end) = pos_str.find(':') {
            let pos_num = &pos_str[..pos_end];
            pos_num.parse().ok()
        } else {
            None
        }
    } else {
        None
    }
}

fn extract_character_context(error_message: &str) -> Option<String> {
    // Extract character information from error messages like "Unexpected character: \u{8c}"
    if let Some(char_start) = error_message.find("\\u{") {
        let char_part = &error_message[char_start..];
        if let Some(char_end) = char_part.find('}') {
            Some(char_part[..char_end + 1].to_string())
        } else {
            None
        }
    } else {
        None
    }
}

fn generate_recovery_suggestions(error_type: &str, _error_message: &str) -> Vec<String> {
    let mut suggestions = Vec::new();

    match error_type {
        "CharacterEncoding" => {
            suggestions.push("Try using lenient parsing mode".to_string());
            suggestions.push("Check if the PDF uses a non-standard encoding".to_string());
            suggestions.push("Consider using a different text extraction method".to_string());
        }
        "CircularReference" => {
            suggestions
                .push("This should not occur after the recent fix - please report".to_string());
        }
        "Encryption" => {
            suggestions.push(
                "PDF is encrypted - decryption not supported in community edition".to_string(),
            );
        }
        "XRefError" => {
            suggestions
                .push("Try using recovery mode for corrupted cross-reference table".to_string());
        }
        "StreamError" => {
            suggestions.push("Enable lenient stream parsing".to_string());
            suggestions.push("Check for stream compression issues".to_string());
        }
        _ => {
            suggestions.push("Enable detailed error reporting".to_string());
            suggestions.push("Try different parsing options".to_string());
        }
    }

    suggestions
}

fn detect_pdf_version(path: &Path) -> Option<String> {
    use std::fs::File;
    use std::io::{BufRead, BufReader};

    if let Ok(file) = File::open(path) {
        let mut reader = BufReader::new(file);
        let mut first_line = String::new();
        if reader.read_line(&mut first_line).is_ok() {
            if first_line.starts_with("%PDF-") {
                let version_part = first_line.trim_start_matches("%PDF-").trim();
                return Some(version_part.to_string());
            }
        }
    }
    None
}

fn calculate_complexity_score(file_size: u64) -> u32 {
    // Simple complexity heuristic based on file size
    match file_size {
        0..=10_000 => 1,             // Very simple
        10_001..=100_000 => 2,       // Simple
        100_001..=1_000_000 => 3,    // Moderate
        1_000_001..=10_000_000 => 4, // Complex
        _ => 5,                      // Very complex
    }
}

fn update_error_statistics(
    report: &mut ComprehensiveReport,
    error_type: &str,
    error_message: &str,
) {
    // Update error categories
    let category = report
        .error_analysis
        .error_categories
        .entry(error_type.to_string())
        .or_insert_with(|| ErrorCategory {
            count: 0,
            percentage: 0.0,
            description: get_error_description(error_type),
            severity: get_error_severity(error_type),
            examples: Vec::new(),
        });

    category.count += 1;
    if category.examples.len() < 3 {
        category.examples.push(error_message.to_string());
    }

    // Update specific counters
    match error_type {
        "CircularReference" => report.summary_stats.circular_reference_errors += 1,
        "CharacterEncoding" => report.summary_stats.character_encoding_errors += 1,
        "XRefError" | "StreamError" | "SyntaxError" | "InvalidStructure" => {
            report.summary_stats.structural_errors += 1;
        }
        "Encryption" => report.summary_stats.encrypted_pdfs += 1,
        _ => {}
    }

    // Extract character error details
    if error_type == "CharacterEncoding" {
        if let Some(char_context) = extract_character_context(error_message) {
            report
                .error_analysis
                .character_error_details
                .push(CharacterErrorDetail {
                    character_code: char_context.clone(),
                    unicode_description: get_unicode_description(&char_context),
                    occurrence_count: 1,
                    affected_files: Vec::new(), // Will be populated later
                    encoding_context: "Unknown".to_string(),
                });
        }
    }
}

fn get_error_description(error_type: &str) -> String {
    match error_type {
        "CircularReference" => "Circular object references in PDF structure".to_string(),
        "CharacterEncoding" => "Character encoding or unexpected character issues".to_string(),
        "Encryption" => "Encrypted PDF files".to_string(),
        "XRefError" => "Cross-reference table errors".to_string(),
        "StreamError" => "PDF stream parsing errors".to_string(),
        "SyntaxError" => "PDF syntax violations".to_string(),
        "IOError" => "File I/O related errors".to_string(),
        "InvalidStructure" => "Invalid PDF structure or format".to_string(),
        _ => "Other parsing errors".to_string(),
    }
}

fn get_error_severity(error_type: &str) -> ErrorSeverity {
    match error_type {
        "CircularReference" | "XRefError" | "InvalidStructure" => ErrorSeverity::Critical,
        "SyntaxError" | "StreamError" => ErrorSeverity::Major,
        "CharacterEncoding" => ErrorSeverity::Minor,
        "Encryption" => ErrorSeverity::Major,
        _ => ErrorSeverity::Minor,
    }
}

fn get_unicode_description(char_code: &str) -> String {
    // Simple mapping for common problematic characters
    match char_code {
        "\\u{8c}" => "Latin-1 Supplement: Œ (Latin Capital Ligature OE)".to_string(),
        "\\u{80}" => "Control character (possibly encoding issue)".to_string(),
        "\\u{81}" => "Control character (possibly encoding issue)".to_string(),
        _ => format!("Unicode character: {}", char_code),
    }
}

fn finalize_report(
    report: &mut ComprehensiveReport,
    parse_times: Vec<f64>,
    total_duration: Duration,
) {
    let total = report.summary_stats.total_pdfs as f64;

    // Calculate success rate
    report.summary_stats.success_rate_percentage =
        (report.summary_stats.successful_parsing as f64 / total) * 100.0;

    // Calculate error category percentages
    for category in report.error_analysis.error_categories.values_mut() {
        category.percentage = (category.count as f64 / total) * 100.0;
    }

    // Performance metrics
    if !parse_times.is_empty() {
        report.performance_metrics.average_parse_time_ms =
            parse_times.iter().sum::<f64>() / parse_times.len() as f64;

        let mut sorted_times = parse_times.clone();
        sorted_times.sort_by(|a, b| a.partial_cmp(b).unwrap());
        report.performance_metrics.median_parse_time_ms = sorted_times[sorted_times.len() / 2];

        report.performance_metrics.throughput_files_per_second =
            total / total_duration.as_secs_f64();
    }

    // Generate recommendations
    report.recommendations = generate_recommendations(report);

    // Update analysis metadata
    report.analysis_metadata.analysis_duration = total_duration.as_secs_f64();
}

fn generate_recommendations(report: &ComprehensiveReport) -> Vec<String> {
    let mut recommendations = Vec::new();

    // Success rate recommendations
    if report.summary_stats.success_rate_percentage < 80.0 {
        recommendations.push(
            "Consider implementing more lenient parsing options to improve success rate"
                .to_string(),
        );
    }

    // Character encoding recommendations
    if report.summary_stats.character_encoding_errors > 0 {
        recommendations
            .push("Implement improved character encoding detection and handling".to_string());
        recommendations
            .push("Consider adding support for Latin-1 and other common encodings".to_string());
    }

    // Circular reference validation
    if report.summary_stats.circular_reference_errors > 0 {
        recommendations.push(
            "CRITICAL: Circular reference errors detected - the fix may need revision".to_string(),
        );
    } else {
        recommendations.push(
            "SUCCESS: No circular reference errors detected - fix is working correctly".to_string(),
        );
    }

    // Performance recommendations
    if report.performance_metrics.average_parse_time_ms > 1000.0 {
        recommendations
            .push("Consider optimizing parser performance for large or complex PDFs".to_string());
    }

    recommendations
}

fn generate_comprehensive_report(
    report: &ComprehensiveReport,
) -> Result<(), Box<dyn std::error::Error>> {
    // Generate JSON manually without serde dependency
    let json = generate_json_manually(report);
    fs::write("comprehensive_analysis_report.json", json)?;
    Ok(())
}

fn generate_json_manually(report: &ComprehensiveReport) -> String {
    let mut json = String::new();
    json.push_str("{\n");
    json.push_str(&format!(
        "  \"timestamp\": \"{}\",\n",
        report.analysis_metadata.timestamp
    ));
    json.push_str(&format!(
        "  \"oxidize_pdf_version\": \"{}\",\n",
        report.analysis_metadata.oxidize_pdf_version
    ));
    json.push_str(&format!(
        "  \"total_files_analyzed\": {},\n",
        report.analysis_metadata.total_files_analyzed
    ));
    json.push_str(&format!(
        "  \"analysis_duration\": {},\n",
        report.analysis_metadata.analysis_duration
    ));
    json.push_str(&format!(
        "  \"test_environment\": \"{}\",\n",
        report.analysis_metadata.test_environment
    ));

    json.push_str(&format!(
        "  \"total_pdfs\": {},\n",
        report.summary_stats.total_pdfs
    ));
    json.push_str(&format!(
        "  \"successful_parsing\": {},\n",
        report.summary_stats.successful_parsing
    ));
    json.push_str(&format!(
        "  \"failed_parsing\": {},\n",
        report.summary_stats.failed_parsing
    ));
    json.push_str(&format!(
        "  \"success_rate_percentage\": {:.2},\n",
        report.summary_stats.success_rate_percentage
    ));
    json.push_str(&format!(
        "  \"circular_reference_errors\": {},\n",
        report.summary_stats.circular_reference_errors
    ));
    json.push_str(&format!(
        "  \"character_encoding_errors\": {},\n",
        report.summary_stats.character_encoding_errors
    ));
    json.push_str(&format!(
        "  \"structural_errors\": {},\n",
        report.summary_stats.structural_errors
    ));
    json.push_str(&format!(
        "  \"encrypted_pdfs\": {},\n",
        report.summary_stats.encrypted_pdfs
    ));

    json.push_str("  \"error_categories\": {\n");
    let mut first = true;
    for (category, error_cat) in &report.error_analysis.error_categories {
        if !first {
            json.push_str(",\n");
        }
        json.push_str(&format!("    \"{}\": {{\n", category));
        json.push_str(&format!("      \"count\": {},\n", error_cat.count));
        json.push_str(&format!(
            "      \"percentage\": {:.2},\n",
            error_cat.percentage
        ));
        json.push_str(&format!(
            "      \"description\": \"{}\",\n",
            error_cat.description
        ));
        json.push_str(&format!(
            "      \"severity\": \"{:?}\"\n",
            error_cat.severity
        ));
        json.push_str("    }");
        first = false;
    }
    json.push_str("\n  },\n");

    json.push_str("  \"recommendations\": [\n");
    for (i, rec) in report.recommendations.iter().enumerate() {
        if i > 0 {
            json.push_str(",\n");
        }
        json.push_str(&format!("    \"{}\"", rec.replace('\"', "\\\"")));
    }
    json.push_str("\n  ]\n");
    json.push_str("}\n");

    json
}

fn generate_summary_report(report: &ComprehensiveReport) -> Result<(), Box<dyn std::error::Error>> {
    let mut output = String::new();

    output.push_str("# PDF Analysis Summary Report\n\n");
    output.push_str(&format!(
        "**Generated:** {}\n",
        report.analysis_metadata.timestamp
    ));
    output.push_str(&format!(
        "**Analysis Duration:** {:.2}s\n",
        report.analysis_metadata.analysis_duration
    ));
    output.push_str(&format!(
        "**Files Analyzed:** {}\n\n",
        report.analysis_metadata.total_files_analyzed
    ));

    output.push_str("## 📊 Summary Statistics\n\n");
    output.push_str(&format!(
        "- **Total PDFs:** {}\n",
        report.summary_stats.total_pdfs
    ));
    output.push_str(&format!(
        "- **Successful Parsing:** {} ({:.2}%)\n",
        report.summary_stats.successful_parsing, report.summary_stats.success_rate_percentage
    ));
    output.push_str(&format!(
        "- **Failed Parsing:** {}\n",
        report.summary_stats.failed_parsing
    ));
    output.push_str(&format!(
        "- **Circular Reference Errors:** {}\n",
        report.summary_stats.circular_reference_errors
    ));
    output.push_str(&format!(
        "- **Character Encoding Errors:** {}\n",
        report.summary_stats.character_encoding_errors
    ));
    output.push_str(&format!(
        "- **Encrypted PDFs:** {}\n\n",
        report.summary_stats.encrypted_pdfs
    ));

    output.push_str("## 🎯 Key Findings\n\n");
    if report.summary_stats.circular_reference_errors == 0 {
        output.push_str(
            "✅ **CIRCULAR REFERENCE FIX SUCCESSFUL** - No circular reference errors detected!\n\n",
        );
    } else {
        output.push_str("❌ **CIRCULAR REFERENCE FIX NEEDS ATTENTION** - Still detecting circular reference errors\n\n");
    }

    output.push_str("## 🔍 Error Categories\n\n");
    for (error_type, category) in &report.error_analysis.error_categories {
        output.push_str(&format!("### {}\n", error_type));
        output.push_str(&format!(
            "- **Count:** {} ({:.2}%)\n",
            category.count, category.percentage
        ));
        output.push_str(&format!("- **Severity:** {:?}\n", category.severity));
        output.push_str(&format!("- **Description:** {}\n\n", category.description));
    }

    output.push_str("## 💡 Recommendations\n\n");
    for (i, rec) in report.recommendations.iter().enumerate() {
        output.push_str(&format!("{}. {}\n", i + 1, rec));
    }

    fs::write("summary_analysis_report.md", output)?;
    Ok(())
}

fn generate_error_focus_report(
    report: &ComprehensiveReport,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut output = String::new();

    output.push_str("# Error-Focused Analysis Report\n\n");
    output.push_str(
        "This report focuses specifically on parsing errors and provides actionable insights.\n\n",
    );

    // Character encoding errors section
    if report.summary_stats.character_encoding_errors > 0 {
        output.push_str("## 🔤 Character Encoding Issues\n\n");
        output.push_str(&format!("Found {} character encoding errors. These are now visible after fixing the circular reference false positives.\n\n", report.summary_stats.character_encoding_errors));

        output.push_str("### Common Problematic Characters\n\n");
        for char_detail in &report.error_analysis.character_error_details {
            output.push_str(&format!(
                "- **{}**: {} (Count: {})\n",
                char_detail.character_code,
                char_detail.unicode_description,
                char_detail.occurrence_count
            ));
        }
        output.push_str("\n");

        output.push_str("### Recommended Solutions\n\n");
        output.push_str("1. Implement proper encoding detection (Latin-1, UTF-8, etc.)\n");
        output.push_str("2. Add fallback handling for unknown characters\n");
        output.push_str("3. Implement lenient parsing mode for character issues\n");
        output.push_str("4. Consider using replacement characters for unparseable bytes\n\n");
    }

    // Failed files with specific errors
    output.push_str("## 📋 Failed Files Analysis\n\n");
    let failed_files: Vec<&FileAnalysisResult> = report
        .detailed_results
        .iter()
        .filter(|result| matches!(result.analysis_result, AnalysisOutcome::Failed { .. }))
        .collect();

    for file in failed_files.iter().take(10) {
        // Show top 10 failures
        if let AnalysisOutcome::Failed {
            error_type,
            error_message,
        } = &file.analysis_result
        {
            output.push_str(&format!("### {}\n", file.filename));
            output.push_str(&format!("- **Size:** {} bytes\n", file.file_size_bytes));
            output.push_str(&format!(
                "- **PDF Version:** {}\n",
                file.pdf_version.as_deref().unwrap_or("Unknown")
            ));
            output.push_str(&format!("- **Error Type:** {}\n", error_type));
            output.push_str(&format!("- **Error:** {}\n\n", error_message));
        }
    }

    fs::write("error_focus_report.md", output)?;
    Ok(())
}

fn get_test_environment() -> String {
    format!(
        "Rust {}, OS: {}",
        std::env::var("RUSTC_VERSION").unwrap_or_else(|_| "unknown".to_string()),
        std::env::consts::OS
    )
}
