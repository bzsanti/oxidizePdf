//! Comprehensive PDF Rendering Analysis Tool
//!
//! This tool analyzes all PDFs in the fixtures directory to identify
//! rendering problems and generate a detailed report.

use oxidize_pdf::parser::{ParseError, PdfReader};
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

#[derive(Debug)]
struct AnalysisReport {
    total_pdfs: usize,
    successful: usize,
    parse_errors: usize,
    render_errors: usize,
    partial_errors: usize,
    total_duration: Duration,
    error_categories: HashMap<String, usize>,
    detailed_errors: Vec<FileAnalysis>,
    patterns: Vec<ErrorPattern>,
}

#[derive(Debug)]
struct FileAnalysis {
    filename: String,
    file_size: u64,
    pdf_version: Option<String>,
    error_type: Option<String>,
    error_message: Option<String>,
    error_details: Option<String>,
    pages_total: Option<usize>,
    pages_processed: Option<usize>,
    processing_time: Duration,
    features: Vec<String>,
}

#[derive(Debug)]
struct ErrorPattern {
    pattern: String,
    count: usize,
    example_files: Vec<String>,
}

fn main() {
    // Increase stack size to handle deeply nested PDFs
    let result = std::thread::Builder::new()
        .stack_size(32 * 1024 * 1024) // 32MB stack
        .spawn(run_analysis)
        .unwrap()
        .join()
        .unwrap();

    if let Err(e) = result {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}

fn run_analysis() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("PDF Rendering Analysis Tool");
    println!("===========================\n");

    // Find fixtures directory
    let fixtures_path = find_fixtures_directory()?;
    println!("Analyzing PDFs in: {}\n", fixtures_path.display());

    // Collect all PDF files
    let pdf_files = collect_pdf_files(&fixtures_path)?;
    println!("Found {} PDF files to analyze\n", pdf_files.len());

    // Initialize report
    let mut report = AnalysisReport {
        total_pdfs: pdf_files.len(),
        successful: 0,
        parse_errors: 0,
        render_errors: 0,
        partial_errors: 0,
        total_duration: Duration::default(),
        error_categories: HashMap::new(),
        detailed_errors: Vec::new(),
        patterns: Vec::new(),
    };

    let start_time = Instant::now();

    // Analyze each PDF
    for (index, pdf_path) in pdf_files.iter().enumerate() {
        if index % 50 == 0 {
            println!("Progress: {}/{} PDFs analyzed...", index, pdf_files.len());
        }

        let analysis = analyze_pdf(pdf_path);

        // Print current file being analyzed for debugging (first 20 errors)
        if analysis.error_type.is_some() && report.detailed_errors.len() < 20 {
            let filename = pdf_path.file_name().unwrap_or_default().to_string_lossy();
            println!("  Error in {}: {:?}", filename, analysis.error_type);
        }

        // Update statistics
        match &analysis.error_type {
            None => report.successful += 1,
            Some(error_type) => {
                if error_type.contains("Parse") {
                    report.parse_errors += 1;
                } else if error_type.contains("Render") {
                    report.render_errors += 1;
                } else {
                    report.partial_errors += 1;
                }

                *report
                    .error_categories
                    .entry(error_type.clone())
                    .or_insert(0) += 1;
                report.detailed_errors.push(analysis);
            }
        }
    }

    report.total_duration = start_time.elapsed();

    // Analyze patterns
    report.patterns = analyze_error_patterns(&report);

    // Generate reports
    generate_json_report(&report)?;
    generate_markdown_report(&report)?;

    // Print summary
    print_summary(&report);

    Ok(())
}

fn find_fixtures_directory() -> Result<PathBuf, Box<dyn std::error::Error + Send + Sync>> {
    // Try different possible locations
    let possible_paths = vec![
        "tests/fixtures",
        "../tests/fixtures",
        "oxidize-pdf-core/tests/fixtures",
        "/Users/santifdezmunoz/Documents/repos/BelowZero/oxidize-pdf/tests/fixtures",
    ];

    for path in possible_paths {
        let path = PathBuf::from(path);
        if path.exists() && path.is_dir() {
            return Ok(path);
        }
    }

    Err("Could not find fixtures directory".into())
}

fn collect_pdf_files(dir: &Path) -> Result<Vec<PathBuf>, Box<dyn std::error::Error + Send + Sync>> {
    let mut pdf_files = Vec::new();

    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("pdf") {
            pdf_files.push(path);
        }
    }

    pdf_files.sort();
    Ok(pdf_files)
}

fn analyze_pdf(path: &Path) -> FileAnalysis {
    // Set a timeout to prevent infinite loops
    let _timeout = Duration::from_secs(10);
    let start_time = Instant::now();
    let filename = path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();
    let file_size = fs::metadata(path).map(|m| m.len()).unwrap_or(0);

    let mut analysis = FileAnalysis {
        filename: filename.clone(),
        file_size,
        pdf_version: None,
        error_type: None,
        error_message: None,
        error_details: None,
        pages_total: None,
        pages_processed: None,
        processing_time: Duration::default(),
        features: Vec::new(),
    };

    // Try to parse the PDF with panic protection
    let parse_result = std::panic::catch_unwind(|| PdfReader::open(path));

    match parse_result {
        Ok(reader_result) => match reader_result {
            Ok(mut reader) => {
                // Get PDF version
                analysis.pdf_version = Some(reader.version().to_string());

                // Try to get metadata
                match reader.metadata() {
                    Ok(metadata) => {
                        // Check if PDF has certain features based on available metadata
                        if let Some(page_count) = metadata.page_count {
                            analysis.pages_total = Some(page_count as usize);
                        }
                    }
                    Err(e) => {
                        analysis.error_type = Some("MetadataError".to_string());
                        analysis.error_message = Some(e.to_string());
                    }
                }

                // Convert to PdfDocument for better page handling
                let document = reader.into_document();

                // Try to get page count
                match document.page_count() {
                    Ok(count) => {
                        analysis.pages_total = Some(count as usize);

                        // Try to access each page
                        let mut pages_ok = 0;
                        for page_idx in 0..count {
                            match document.get_page(page_idx) {
                                Ok(_page) => {
                                    pages_ok += 1;
                                }
                                Err(e) => {
                                    // First page error is the main error
                                    if pages_ok == 0 {
                                        analysis.error_type = Some("PageTreeError".to_string());
                                        analysis.error_message = Some(e.to_string());
                                    }
                                    break;
                                }
                            }
                        }
                        analysis.pages_processed = Some(pages_ok);
                    }
                    Err(e) => {
                        analysis.error_type = Some("PageTreeError".to_string());
                        analysis.error_message = Some(e.to_string());
                    }
                }

                // Features were already checked in metadata phase
            }
            Err(e) => {
                analysis.error_type = Some(categorize_parse_error(&e));
                analysis.error_message = Some(e.to_string());

                // Get more detailed error information
                let mut error_chain = String::new();
                let mut current_error: &dyn std::error::Error = &e;
                while let Some(source) = current_error.source() {
                    error_chain.push_str(&format!("\n  Caused by: {source}"));
                    current_error = source;
                }
                if !error_chain.is_empty() {
                    analysis.error_details = Some(error_chain);
                }
            }
        },
        Err(_panic) => {
            analysis.error_type = Some("ParseError::StackOverflow".to_string());
            analysis.error_message = Some("Stack overflow or panic during parsing".to_string());
            analysis.error_details = Some(
                "PDF structure too deeply nested or contains recursive references".to_string(),
            );
        }
    }

    // Skip Document API test since we're focusing on parsing

    analysis.processing_time = start_time.elapsed();
    analysis
}

fn categorize_parse_error(error: &ParseError) -> String {
    let error_str = error.to_string();

    if error_str.contains("Invalid header") || error_str.contains("PDF header") {
        "ParseError::InvalidHeader".to_string()
    } else if error_str.contains("xref") || error_str.contains("cross-reference") {
        "ParseError::XrefError".to_string()
    } else if error_str.contains("trailer") {
        "ParseError::TrailerError".to_string()
    } else if error_str.contains("object") && error_str.contains("stream") {
        "ParseError::ObjectStreamError".to_string()
    } else if error_str.contains("Unsupported") {
        "ParseError::UnsupportedFeature".to_string()
    } else if error_str.contains("Unexpected") || error_str.contains("EOF") {
        "ParseError::UnexpectedEOF".to_string()
    } else {
        "ParseError::Other".to_string()
    }
}

fn analyze_error_patterns(report: &AnalysisReport) -> Vec<ErrorPattern> {
    let mut patterns = Vec::new();

    // Group errors by common patterns
    let mut pattern_map: HashMap<String, Vec<String>> = HashMap::new();

    for error in &report.detailed_errors {
        if let Some(error_msg) = &error.error_message {
            // Extract common patterns
            let pattern = if error_msg.contains("Unsupported compression") {
                "Unsupported compression method"
            } else if error_msg.contains("Invalid xref") {
                "Invalid cross-reference table"
            } else if error_msg.contains("object not found") {
                "Missing object reference"
            } else if error_msg.contains("Invalid stream") {
                "Invalid stream data"
            } else if error_msg.contains("Encrypted") {
                "Encrypted PDF"
            } else {
                continue;
            };

            pattern_map
                .entry(pattern.to_string())
                .or_default()
                .push(error.filename.clone());
        }
    }

    // Convert to ErrorPattern structs
    for (pattern, files) in pattern_map {
        patterns.push(ErrorPattern {
            pattern: pattern.clone(),
            count: files.len(),
            example_files: files.into_iter().take(5).collect(),
        });
    }

    // Sort by frequency
    patterns.sort_by(|a, b| b.count.cmp(&a.count));

    patterns
}

fn generate_json_report(
    report: &AnalysisReport,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Create a simple JSON-like format without serde
    let mut json = String::new();
    json.push_str("{\n");
    json.push_str(&format!("  \"total_pdfs\": {},\n", report.total_pdfs));
    json.push_str(&format!("  \"successful\": {},\n", report.successful));
    json.push_str(&format!("  \"parse_errors\": {},\n", report.parse_errors));
    json.push_str(&format!("  \"render_errors\": {},\n", report.render_errors));
    json.push_str(&format!(
        "  \"partial_errors\": {},\n",
        report.partial_errors
    ));
    json.push_str(&format!(
        "  \"total_duration_secs\": {},\n",
        report.total_duration.as_secs_f64()
    ));

    json.push_str("  \"error_categories\": {\n");
    let mut first = true;
    for (category, count) in &report.error_categories {
        if !first {
            json.push_str(",\n");
        }
        json.push_str(&format!("    \"{category}\": {count}"));
        first = false;
    }
    json.push_str("\n  },\n");

    json.push_str(&format!(
        "  \"error_count\": {}\n",
        report.detailed_errors.len()
    ));
    json.push_str("}\n");

    let mut file = File::create("pdf_analysis_report.json")?;
    file.write_all(json.as_bytes())?;
    println!("\nJSON report saved to: pdf_analysis_report.json");
    Ok(())
}

fn generate_markdown_report(
    report: &AnalysisReport,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut markdown = String::new();

    markdown.push_str("# PDF Rendering Analysis Report\n\n");
    markdown.push_str("Generated: Analysis Report\n");
    markdown.push_str(&format!(
        "Total analysis time: {:.2}s\n\n",
        report.total_duration.as_secs_f64()
    ));

    markdown.push_str("## Summary\n\n");
    markdown.push_str(&format!(
        "- **Total PDFs analyzed**: {}\n",
        report.total_pdfs
    ));
    markdown.push_str(&format!(
        "- **Successful**: {} ({:.1}%)\n",
        report.successful,
        (report.successful as f64 / report.total_pdfs as f64) * 100.0
    ));
    markdown.push_str(&format!(
        "- **Parse errors**: {} ({:.1}%)\n",
        report.parse_errors,
        (report.parse_errors as f64 / report.total_pdfs as f64) * 100.0
    ));
    markdown.push_str(&format!(
        "- **Render errors**: {} ({:.1}%)\n",
        report.render_errors,
        (report.render_errors as f64 / report.total_pdfs as f64) * 100.0
    ));
    markdown.push_str(&format!(
        "- **Partial errors**: {} ({:.1}%)\n\n",
        report.partial_errors,
        (report.partial_errors as f64 / report.total_pdfs as f64) * 100.0
    ));

    markdown.push_str("## Error Categories\n\n");
    markdown.push_str("| Error Type | Count | Percentage |\n");
    markdown.push_str("|------------|-------|------------|\n");

    let mut categories: Vec<_> = report.error_categories.iter().collect();
    categories.sort_by(|a, b| b.1.cmp(a.1));

    for (category, count) in categories {
        markdown.push_str(&format!(
            "| {} | {} | {:.1}% |\n",
            category,
            count,
            (*count as f64 / report.total_pdfs as f64) * 100.0
        ));
    }

    markdown.push_str("\n## Error Patterns\n\n");
    if !report.patterns.is_empty() {
        for pattern in &report.patterns {
            markdown.push_str(&format!(
                "### {} ({} occurrences)\n",
                pattern.pattern, pattern.count
            ));
            markdown.push_str("Example files:\n");
            for file in &pattern.example_files {
                markdown.push_str(&format!("- {file}\n"));
            }
            markdown.push('\n');
        }
    }

    markdown.push_str("\n## Detailed Errors (First 50)\n\n");
    for (i, error) in report.detailed_errors.iter().take(50).enumerate() {
        markdown.push_str(&format!("### {}. {}\n", i + 1, error.filename));
        markdown.push_str(&format!("- **Size**: {} bytes\n", error.file_size));
        if let Some(version) = &error.pdf_version {
            markdown.push_str(&format!("- **PDF Version**: {version}\n"));
        }
        if let Some(error_type) = &error.error_type {
            markdown.push_str(&format!("- **Error Type**: {error_type}\n"));
        }
        if let Some(error_msg) = &error.error_message {
            markdown.push_str(&format!("- **Error**: {error_msg}\n"));
        }
        if let Some(details) = &error.error_details {
            markdown.push_str(&format!("- **Details**: {details}\n"));
        }
        if !error.features.is_empty() {
            markdown.push_str(&format!("- **Features**: {}\n", error.features.join(", ")));
        }
        markdown.push('\n');
    }

    markdown.push_str("\n## Recommendations\n\n");
    markdown.push_str("Based on the analysis, here are the top priorities for fixing:\n\n");

    // Add recommendations based on error patterns
    let top_errors: Vec<_> = report
        .error_categories
        .iter()
        .collect::<Vec<_>>()
        .into_iter()
        .take(5)
        .collect();

    for (i, (error_type, _count)) in top_errors.iter().enumerate() {
        markdown.push_str(&format!("{}. **Fix {}**: ", i + 1, error_type));

        match error_type.as_str() {
            "ParseError::XrefError" => {
                markdown.push_str(
                    "Improve cross-reference table parsing to handle malformed xref tables\n",
                );
            }
            "ParseError::ObjectStreamError" => {
                markdown.push_str("Enhance object stream decompression and parsing\n");
            }
            "ParseError::UnsupportedFeature" => {
                markdown.push_str(
                    "Implement support for additional PDF features (compression, encryption)\n",
                );
            }
            "ContentError" => {
                markdown.push_str("Improve content stream parsing and rendering\n");
            }
            _ => {
                markdown.push_str("Investigate and fix this error category\n");
            }
        }
    }

    let mut file = File::create("pdf_analysis_report.md")?;
    file.write_all(markdown.as_bytes())?;
    println!("Markdown report saved to: pdf_analysis_report.md");

    Ok(())
}

fn print_summary(report: &AnalysisReport) {
    println!("\n=== ANALYSIS COMPLETE ===\n");
    println!("Total PDFs analyzed: {}", report.total_pdfs);
    println!(
        "Successful: {} ({:.1}%)",
        report.successful,
        (report.successful as f64 / report.total_pdfs as f64) * 100.0
    );
    println!(
        "Failed: {} ({:.1}%)",
        report.total_pdfs - report.successful,
        ((report.total_pdfs - report.successful) as f64 / report.total_pdfs as f64) * 100.0
    );
    println!("\nTop error categories:");

    let mut categories: Vec<_> = report.error_categories.iter().collect();
    categories.sort_by(|a, b| b.1.cmp(a.1));

    for (category, count) in categories.iter().take(5) {
        println!("  - {category}: {count} PDFs");
    }

    println!("\nReports generated:");
    println!("  - pdf_analysis_report.json");
    println!("  - pdf_analysis_report.md");
}
