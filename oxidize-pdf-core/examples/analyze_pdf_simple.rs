//! Simple PDF Analysis Tool
//!
//! This tool performs a basic analysis of PDFs to identify rendering issues.

use oxidize_pdf::parser::PdfReader;
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::Write;
use std::path::PathBuf;
use std::time::Instant;

#[derive(Debug)]
struct Report {
    total: usize,
    success: usize,
    errors: HashMap<String, usize>,
    error_files: Vec<(String, String)>,
}

fn main() {
    println!("Simple PDF Analysis Tool");
    println!("========================\n");

    let fixtures_path =
        PathBuf::from("/Users/santifdezmunoz/Documents/repos/BelowZero/oxidize-pdf/tests/fixtures");

    let mut pdf_files = Vec::new();
    for entry in fs::read_dir(&fixtures_path).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) == Some("pdf") {
            pdf_files.push(path);
        }
    }
    pdf_files.sort();

    println!("Found {} PDF files\n", pdf_files.len());

    let mut report = Report {
        total: pdf_files.len(),
        success: 0,
        errors: HashMap::new(),
        error_files: Vec::new(),
    };

    let start = Instant::now();

    for (idx, path) in pdf_files.iter().enumerate() {
        if idx % 100 == 0 {
            println!("Progress: {}/{}", idx, pdf_files.len());
        }

        let filename = path.file_name().unwrap().to_string_lossy().to_string();

        // Simple test: can we open and get basic info?
        match PdfReader::open(&path) {
            Ok(mut reader) => {
                // Try to get page count
                match reader.page_count() {
                    Ok(_) => {
                        report.success += 1;
                    }
                    Err(e) => {
                        let error_type = format!("PageCount: {}", categorize_error(&e.to_string()));
                        *report.errors.entry(error_type.clone()).or_insert(0) += 1;
                        report.error_files.push((filename, error_type));
                    }
                }
            }
            Err(e) => {
                let error_type = categorize_error(&e.to_string());
                *report.errors.entry(error_type.clone()).or_insert(0) += 1;
                report.error_files.push((filename, error_type));
            }
        }
    }

    let duration = start.elapsed();

    // Print results
    println!("\n=== ANALYSIS COMPLETE ===");
    println!("Total time: {:.2}s", duration.as_secs_f64());
    println!("\nResults:");
    println!("  Total PDFs: {}", report.total);
    println!(
        "  Successful: {} ({:.1}%)",
        report.success,
        (report.success as f64 / report.total as f64) * 100.0
    );
    println!(
        "  Failed: {} ({:.1}%)",
        report.total - report.success,
        ((report.total - report.success) as f64 / report.total as f64) * 100.0
    );

    println!("\nError Categories:");
    let mut errors: Vec<_> = report.errors.iter().collect();
    errors.sort_by(|a, b| b.1.cmp(a.1));

    for (error, count) in errors.iter().take(10) {
        println!("  {}: {} PDFs", error, count);
    }

    // Save detailed report
    let mut file = File::create("simple_pdf_analysis.txt").unwrap();
    writeln!(file, "Simple PDF Analysis Report").unwrap();
    writeln!(file, "=========================\n").unwrap();
    writeln!(file, "Total PDFs: {}", report.total).unwrap();
    writeln!(
        file,
        "Successful: {} ({:.1}%)",
        report.success,
        (report.success as f64 / report.total as f64) * 100.0
    )
    .unwrap();
    writeln!(
        file,
        "Failed: {} ({:.1}%)\n",
        report.total - report.success,
        ((report.total - report.success) as f64 / report.total as f64) * 100.0
    )
    .unwrap();

    writeln!(file, "Error Categories:").unwrap();
    for (error, count) in errors.iter() {
        writeln!(file, "  {}: {} PDFs", error, count).unwrap();
    }

    writeln!(file, "\nFirst 100 Failed PDFs:").unwrap();
    for (filename, error) in report.error_files.iter().take(100) {
        writeln!(file, "  {}: {}", filename, error).unwrap();
    }

    println!("\nReport saved to: simple_pdf_analysis.txt");
}

fn categorize_error(error: &str) -> String {
    if error.contains("Invalid header") || error.contains("PDF header") {
        "InvalidHeader".to_string()
    } else if error.contains("xref") || error.contains("cross-reference") {
        "XrefError".to_string()
    } else if error.contains("trailer") {
        "TrailerError".to_string()
    } else if error.contains("stream") {
        "StreamError".to_string()
    } else if error.contains("Unsupported") {
        "UnsupportedFeature".to_string()
    } else if error.contains("EOF") || error.contains("Unexpected end") {
        "UnexpectedEOF".to_string()
    } else if error.contains("stack overflow") {
        "StackOverflow".to_string()
    } else if error.contains("Invalid reference") {
        "InvalidReference".to_string()
    } else if error.contains("Missing") {
        "MissingKey".to_string()
    } else {
        "Other".to_string()
    }
}
