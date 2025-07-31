use oxidize_pdf::parser::{ParseOptions, PdfReader};
use std::fs::File;
use std::path::Path;

fn test_pdf_with_options(path: &Path, lenient: bool) -> Result<(), Box<dyn std::error::Error>> {
    let file = File::open(path)?;

    let options = if lenient {
        ParseOptions::lenient()
    } else {
        ParseOptions::strict()
    };

    match PdfReader::new_with_options(file, options) {
        Ok(mut reader) => {
            // Try to get basic info
            let page_count = reader.page_count().unwrap_or(0);
            let version = reader.version();

            println!("  ✓ Success - Pages: {}, Version: {}", page_count, version);
            Ok(())
        }
        Err(e) => {
            println!("  ✗ Failed: {}", e);
            Err(Box::new(e))
        }
    }
}

fn main() {
    println!("Testing lenient parsing on PDF files...\n");

    // Test specific problematic PDFs first
    let fixtures_dir =
        Path::new("/Users/santifdezmunoz/Documents/repos/BelowZero/oxidize-pdf/tests/fixtures");

    println!("=== Testing Individual PDFs ===\n");

    // Try to find PDFs that might have stream length issues
    let test_patterns = vec!["000031.pdf", "000040.pdf", "000080.pdf"];

    for pattern in &test_patterns {
        let path = fixtures_dir.join(pattern);
        if path.exists() {
            println!("{}", pattern);
            print!("  Strict:  ");
            let strict_ok = test_pdf_with_options(&path, false).is_ok();
            print!("  Lenient: ");
            let lenient_ok = test_pdf_with_options(&path, true).is_ok();

            if !strict_ok && lenient_ok {
                println!("  🎉 FIXED by lenient mode!");
            }
            println!();
        }
    }

    println!("\n=== Testing All PDFs ===\n");

    let mut total = 0;
    let mut strict_success = 0;
    let mut lenient_success = 0;
    let mut lenient_fixed = Vec::new();
    let mut still_failing = Vec::new();

    if let Ok(entries) = std::fs::read_dir(fixtures_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("pdf") {
                total += 1;

                // Test strict mode
                let strict_ok = test_pdf_with_options(&path, false).is_ok();
                if strict_ok {
                    strict_success += 1;
                }

                // Test lenient mode
                let lenient_ok = test_pdf_with_options(&path, true).is_ok();
                if lenient_ok {
                    lenient_success += 1;
                    if !strict_ok {
                        lenient_fixed.push(path.file_name().unwrap().to_string_lossy().to_string());
                    }
                } else if !strict_ok {
                    still_failing.push(path.file_name().unwrap().to_string_lossy().to_string());
                }
            }
        }
    }

    println!("\n=== Summary ===");
    println!("Total PDFs tested: {}", total);
    println!(
        "Strict mode success: {} ({:.1}%)",
        strict_success,
        (strict_success as f64 / total as f64) * 100.0
    );
    println!(
        "Lenient mode success: {} ({:.1}%)",
        lenient_success,
        (lenient_success as f64 / total as f64) * 100.0
    );
    println!("PDFs fixed by lenient mode: {}", lenient_fixed.len());

    if !lenient_fixed.is_empty() {
        println!("\nPDFs fixed by lenient mode:");
        for (i, pdf) in lenient_fixed.iter().enumerate() {
            println!("  {}. {}", i + 1, pdf);
            if i >= 9 {
                println!("  ... and {} more", lenient_fixed.len() - 10);
                break;
            }
        }
    }

    if !still_failing.is_empty() {
        println!(
            "\nPDFs still failing in lenient mode: {} files",
            still_failing.len()
        );
        for (i, pdf) in still_failing.iter().enumerate() {
            if i < 5 {
                println!("  - {}", pdf);
            }
        }
        if still_failing.len() > 5 {
            println!("  ... and {} more", still_failing.len() - 5);
        }
    }
}
