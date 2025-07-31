//! Comprehensive PDF Analysis with Rendering Validation
//!
//! This tool analyzes PDFs using both oxidize-pdf parser and oxidize-pdf-render
//! to identify compatibility issues and generate detailed reports.

use oxidize_pdf::parser::{ParseOptions, PdfReader};
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
struct CompatibilityReport {
    total_pdfs: usize,
    parse_successful: usize,
    render_successful: usize,
    both_successful: usize,
    parse_only: usize,
    render_only: usize,
    both_failed: usize,
    total_duration: Duration,
    parse_errors: HashMap<String, usize>,
    render_errors: HashMap<String, usize>,
    compatibility_issues: Vec<CompatibilityIssue>,
}

#[derive(Debug, Clone)]
struct CompatibilityIssue {
    filename: String,
    file_size: u64,
    issue_type: IssueType,
    parse_error: Option<String>,
    render_error: Option<String>,
    details: Option<String>,
}

#[derive(Debug, Clone)]
enum IssueType {
    ParseOnlySuccess,
    RenderOnlySuccess,
    DifferentErrors,
}

impl CompatibilityReport {
    fn new() -> Self {
        Self {
            total_pdfs: 0,
            parse_successful: 0,
            render_successful: 0,
            both_successful: 0,
            parse_only: 0,
            render_only: 0,
            both_failed: 0,
            total_duration: Duration::default(),
            parse_errors: HashMap::new(),
            render_errors: HashMap::new(),
            compatibility_issues: Vec::new(),
        }
    }
}

fn main() {
    // Increase stack size for deeply nested PDFs
    let result = std::thread::Builder::new()
        .stack_size(32 * 1024 * 1024) // 32MB stack
        .spawn(|| run_analysis())
        .unwrap()
        .join()
        .unwrap();

    if let Err(e) = result {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}

fn run_analysis() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("🔍 PDF Compatibility Analysis with Rendering");
    println!("===========================================\n");

    // Find fixtures directory
    let fixtures_path =
        find_fixtures_directory().map_err(|e| -> Box<dyn std::error::Error + Send + Sync> {
            Box::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                e.to_string(),
            ))
        })?;
    println!("Analyzing PDFs in: {}\n", fixtures_path.display());

    // Collect all PDF files
    let pdf_files = collect_pdf_files(&fixtures_path).map_err(
        |e| -> Box<dyn std::error::Error + Send + Sync> {
            Box::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                e.to_string(),
            ))
        },
    )?;
    println!("Found {} PDF files to analyze\n", pdf_files.len());

    // Initialize report
    let mut report = CompatibilityReport::new();
    report.total_pdfs = pdf_files.len();

    let start_time = Instant::now();

    // Analyze each PDF
    for (index, pdf_path) in pdf_files.iter().enumerate() {
        if index % 50 == 0 {
            println!("Progress: {}/{} PDFs analyzed...", index, pdf_files.len());
        }

        analyze_pdf_compatibility(&pdf_path, &mut report);
    }

    report.total_duration = start_time.elapsed();

    // Generate and save report
    generate_report(&report).map_err(|e| -> Box<dyn std::error::Error + Send + Sync> {
        Box::new(std::io::Error::new(
            std::io::ErrorKind::Other,
            e.to_string(),
        ))
    })?;

    Ok(())
}

fn analyze_pdf_compatibility(pdf_path: &Path, report: &mut CompatibilityReport) {
    let filename = pdf_path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();
    let file_size = fs::metadata(pdf_path).map(|m| m.len()).unwrap_or(0);

    // Test parsing with oxidize-pdf
    let parse_result = test_pdf_parsing(pdf_path);
    let render_result = test_pdf_rendering(pdf_path);

    // Update statistics
    match (&parse_result, &render_result) {
        (Ok(_), Ok(_)) => {
            report.parse_successful += 1;
            report.render_successful += 1;
            report.both_successful += 1;
        }
        (Ok(_), Err(render_err)) => {
            report.parse_successful += 1;
            report.parse_only += 1;

            let error_type = categorize_render_error(render_err);
            *report.render_errors.entry(error_type.clone()).or_insert(0) += 1;

            report.compatibility_issues.push(CompatibilityIssue {
                filename,
                file_size,
                issue_type: IssueType::ParseOnlySuccess,
                parse_error: None,
                render_error: Some(error_type),
                details: Some(format!(
                    "Parses correctly but rendering fails: {}",
                    render_err
                )),
            });
        }
        (Err(parse_err), Ok(_)) => {
            report.render_successful += 1;
            report.render_only += 1;

            let error_type = categorize_parse_error(parse_err);
            *report.parse_errors.entry(error_type.clone()).or_insert(0) += 1;

            report.compatibility_issues.push(CompatibilityIssue {
                filename,
                file_size,
                issue_type: IssueType::RenderOnlySuccess,
                parse_error: Some(error_type),
                render_error: None,
                details: Some(format!(
                    "Renders correctly but parsing fails: {}",
                    parse_err
                )),
            });
        }
        (Err(parse_err), Err(render_err)) => {
            report.both_failed += 1;

            let parse_error_type = categorize_parse_error(parse_err);
            let render_error_type = categorize_render_error(render_err);

            *report
                .parse_errors
                .entry(parse_error_type.clone())
                .or_insert(0) += 1;
            *report
                .render_errors
                .entry(render_error_type.clone())
                .or_insert(0) += 1;

            // Check if errors are different
            if parse_error_type != render_error_type {
                report.compatibility_issues.push(CompatibilityIssue {
                    filename,
                    file_size,
                    issue_type: IssueType::DifferentErrors,
                    parse_error: Some(parse_error_type),
                    render_error: Some(render_error_type),
                    details: Some("Different error types in parsing vs rendering".to_string()),
                });
            }
        }
    }
}

fn test_pdf_parsing(pdf_path: &Path) -> Result<(), String> {
    // Try with lenient parsing first
    let file = File::open(pdf_path).map_err(|e| format!("IO error: {}", e))?;
    let options = ParseOptions::lenient();

    match PdfReader::new_with_options(file, options) {
        Ok(mut reader) => {
            // Try to get page count as basic validation
            match reader.page_count() {
                Ok(_) => Ok(()),
                Err(e) => Err(format!("Page count error: {:?}", e)),
            }
        }
        Err(e) => Err(format!("Parse error: {:?}", e)),
    }
}

fn test_pdf_rendering(pdf_path: &Path) -> Result<(), String> {
    // Build path to oxidize-pdf-render
    let render_path = Path::new("../oxidize-pdf-render");

    // Use cargo to run the test_render example
    let output = Command::new("cargo")
        .args(&[
            "run",
            "--manifest-path",
            &render_path.join("Cargo.toml").to_string_lossy(),
            "--example",
            "test_render",
            "--quiet",
            "--",
            &pdf_path.to_string_lossy(),
            "/tmp/test_render_output.png",
        ])
        .output()
        .map_err(|e| format!("Failed to execute render command: {}", e))?;

    if output.status.success() {
        // Clean up test output
        let _ = fs::remove_file("/tmp/test_render_output.png");
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(stderr.to_string())
    }
}

fn categorize_parse_error(error: &str) -> String {
    if error.contains("circular reference") {
        "CircularReference".to_string()
    } else if error.contains("EmptyFile") {
        "EmptyFile".to_string()
    } else if error.contains("encrypted") || error.contains("Encryption") {
        "EncryptionNotSupported".to_string()
    } else if error.contains("InvalidHeader") {
        "InvalidHeader".to_string()
    } else if error.contains("XrefError") || error.contains("xref") {
        "XrefError".to_string()
    } else if error.contains("MissingKey") {
        "MissingKey".to_string()
    } else {
        "OtherParseError".to_string()
    }
}

fn categorize_render_error(error: &str) -> String {
    if error.contains("NotImplemented") {
        "NotImplemented".to_string()
    } else if error.contains("Font") {
        "FontError".to_string()
    } else if error.contains("Image") {
        "ImageError".to_string()
    } else if error.contains("ColorSpace") {
        "ColorSpaceError".to_string()
    } else if error.contains("ContentStream") {
        "ContentStreamError".to_string()
    } else {
        "OtherRenderError".to_string()
    }
}

fn generate_report(report: &CompatibilityReport) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n📊 Compatibility Analysis Summary");
    println!("=================================\n");

    println!("Total PDFs analyzed: {}", report.total_pdfs);
    println!("Analysis duration: {:.2?}", report.total_duration);
    println!(
        "Processing speed: {:.1} PDFs/second\n",
        report.total_pdfs as f64 / report.total_duration.as_secs_f64()
    );

    // Parsing results
    let parse_rate = report.parse_successful as f64 / report.total_pdfs as f64 * 100.0;
    println!("📄 Parsing Results (oxidize-pdf):");
    println!(
        "  ✅ Successful: {} ({:.1}%)",
        report.parse_successful, parse_rate
    );
    println!(
        "  ❌ Failed: {} ({:.1}%)",
        report.total_pdfs - report.parse_successful,
        100.0 - parse_rate
    );

    // Rendering results
    let render_rate = report.render_successful as f64 / report.total_pdfs as f64 * 100.0;
    println!("\n🎨 Rendering Results (oxidize-pdf-render):");
    println!(
        "  ✅ Successful: {} ({:.1}%)",
        report.render_successful, render_rate
    );
    println!(
        "  ❌ Failed: {} ({:.1}%)",
        report.total_pdfs - report.render_successful,
        100.0 - render_rate
    );

    // Combined results
    println!("\n🔄 Combined Analysis:");
    println!(
        "  ✅✅ Both successful: {} ({:.1}%)",
        report.both_successful,
        report.both_successful as f64 / report.total_pdfs as f64 * 100.0
    );
    println!(
        "  ✅❌ Parse only: {} ({:.1}%)",
        report.parse_only,
        report.parse_only as f64 / report.total_pdfs as f64 * 100.0
    );
    println!(
        "  ❌✅ Render only: {} ({:.1}%)",
        report.render_only,
        report.render_only as f64 / report.total_pdfs as f64 * 100.0
    );
    println!(
        "  ❌❌ Both failed: {} ({:.1}%)",
        report.both_failed,
        report.both_failed as f64 / report.total_pdfs as f64 * 100.0
    );

    // Error breakdown
    if !report.parse_errors.is_empty() {
        println!("\n📋 Parse Error Breakdown:");
        let mut errors: Vec<_> = report.parse_errors.iter().collect();
        errors.sort_by_key(|(_, count)| std::cmp::Reverse(**count));
        for (error_type, count) in errors {
            println!("  {}: {} PDFs", error_type, count);
        }
    }

    if !report.render_errors.is_empty() {
        println!("\n🎨 Render Error Breakdown:");
        let mut errors: Vec<_> = report.render_errors.iter().collect();
        errors.sort_by_key(|(_, count)| std::cmp::Reverse(**count));
        for (error_type, count) in errors {
            println!("  {}: {} PDFs", error_type, count);
        }
    }

    // Compatibility issues
    if !report.compatibility_issues.is_empty() {
        println!("\n⚠️  Key Compatibility Issues (showing first 20):");
        for (i, issue) in report.compatibility_issues.iter().take(20).enumerate() {
            println!(
                "\n{}. {} ({} bytes)",
                i + 1,
                issue.filename,
                issue.file_size
            );
            match issue.issue_type {
                IssueType::ParseOnlySuccess => {
                    println!("   Issue: Parses but fails to render");
                    println!("   Render error: {}", issue.render_error.as_ref().unwrap());
                }
                IssueType::RenderOnlySuccess => {
                    println!("   Issue: Renders but fails to parse");
                    println!("   Parse error: {}", issue.parse_error.as_ref().unwrap());
                }
                IssueType::DifferentErrors => {
                    println!("   Issue: Different error types");
                    println!("   Parse error: {}", issue.parse_error.as_ref().unwrap());
                    println!("   Render error: {}", issue.render_error.as_ref().unwrap());
                }
            }
        }

        if report.compatibility_issues.len() > 20 {
            println!(
                "\n... and {} more issues",
                report.compatibility_issues.len() - 20
            );
        }
    }

    // Save detailed report
    let report_path = format!(
        "compatibility_report_{}.txt",
        chrono::Local::now().format("%Y%m%d_%H%M%S")
    );
    let mut file = File::create(&report_path)?;

    writeln!(file, "PDF Compatibility Analysis Report")?;
    writeln!(file, "=================================")?;
    writeln!(
        file,
        "Generated: {}",
        chrono::Local::now().format("%Y-%m-%d %H:%M:%S")
    )?;
    writeln!(file, "\nTotal PDFs: {}", report.total_pdfs)?;
    writeln!(
        file,
        "Parse successful: {} ({:.1}%)",
        report.parse_successful, parse_rate
    )?;
    writeln!(
        file,
        "Render successful: {} ({:.1}%)",
        report.render_successful, render_rate
    )?;
    writeln!(file, "\nCompatibility Issues:")?;

    for issue in &report.compatibility_issues {
        writeln!(file, "\n{}", issue.filename)?;
        if let Some(details) = &issue.details {
            writeln!(file, "  {}", details)?;
        }
    }

    println!("\n💾 Detailed report saved to: {}", report_path);

    Ok(())
}

fn find_fixtures_directory() -> Result<PathBuf, Box<dyn std::error::Error>> {
    // Try multiple possible locations
    let possible_paths = vec![
        PathBuf::from("tests/fixtures"),
        PathBuf::from("oxidize-pdf-core/tests/fixtures"),
        PathBuf::from("../tests/fixtures"),
    ];

    for path in possible_paths {
        if path.exists() && path.is_dir() {
            return Ok(path);
        }
    }

    Err("Could not find fixtures directory. Please run from project root.".into())
}

fn collect_pdf_files(dir: &Path) -> Result<Vec<PathBuf>, Box<dyn std::error::Error>> {
    let mut pdf_files = Vec::new();

    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.extension().and_then(|s| s.to_str()) == Some("pdf") {
            pdf_files.push(path);
        }
    }

    pdf_files.sort();
    Ok(pdf_files)
}
