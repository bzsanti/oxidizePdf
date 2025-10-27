use oxidize_pdf::parser::{ParseOptions, PdfReader};
use std::fs::File;
use std::path::Path;

fn test_pdf_with_options(path: &Path, lenient: bool) -> Result<(), Box<dyn std::error::Error>> {
    let file = File::open(path)?;

    let options = ParseOptions {
        lenient_streams: lenient,
        max_recovery_bytes: 2000,
        collect_warnings: false,
    };

    match PdfReader::new_with_options(file, options) {
        Ok(mut reader) => {
            println!("✓ Successfully parsed: {}", path.display());

            // Try to get basic info
            if let Ok(page_count) = reader.page_count() {
                println!("  Pages: {}", page_count);
            }

            if let Ok(metadata) = reader.metadata() {
                if let Some(title) = metadata.title {
                    println!("  Title: {}", title);
                }
            }

            Ok(())
        }
        Err(e) => {
            println!("✗ Failed to parse: {} - {}", path.display(), e);
            Err(Box::new(e))
        }
    }
}

fn main() {
    println!("Testing lenient parsing on PDF files...\n");

    // Test on the PDFs that were failing before
    let test_files = vec![
        "/Users/santifdezmunoz/Documents/repos/BelowZero/oxidize-pdf/tests/fixtures/000031.pdf",
        "/Users/santifdezmunoz/Documents/repos/BelowZero/oxidize-pdf/tests/fixtures/000040.pdf",
        "/Users/santifdezmunoz/Documents/repos/BelowZero/oxidize-pdf/tests/fixtures/000080.pdf",
        // Add more problematic PDFs here
    ];

    for file_path in &test_files {
        let path = Path::new(file_path);
        if !path.exists() {
            println!("File not found: {}", file_path);
            continue;
        }

        println!("\nTesting: {}", path.file_name().unwrap().to_string_lossy());

        // Test strict mode
        println!("Strict mode:");
        let _ = test_pdf_with_options(path, false);

        // Test lenient mode
        println!("Lenient mode:");
        let _ = test_pdf_with_options(path, true);
    }

    println!("\n\nTesting all PDFs in fixtures directory...\n");

    // Now test all PDFs
    let fixtures_dir =
        Path::new("/Users/santifdezmunoz/Documents/repos/BelowZero/oxidize-pdf/tests/fixtures");

    let mut total = 0;
    let mut strict_success = 0;
    let mut lenient_success = 0;
    let mut lenient_only_success = 0;

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
                        lenient_only_success += 1;
                        println!("  ➜ Fixed by lenient mode!");
                    }
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
    println!("PDFs fixed by lenient mode: {}", lenient_only_success);
}
