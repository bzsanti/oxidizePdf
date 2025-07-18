//! Example demonstrating PDF error recovery capabilities
//!
//! This example shows how to handle corrupted or malformed PDF files.

use oxidize_pdf::{
    detect_corruption, validate_pdf, Document, Font, Page, PdfRecovery, RecoveryOptions,
};
use std::fs::{self, File};
use std::io::Write;
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create output directory
    fs::create_dir_all("output/recovery")?;

    println!("=== PDF Error Recovery Example ===\n");

    // First, create some corrupted PDFs for testing
    create_test_files()?;

    // Example 1: Detect corruption
    example_detect_corruption()?;

    // Example 2: Basic recovery
    example_basic_recovery()?;

    // Example 3: Partial recovery
    example_partial_recovery()?;

    // Example 4: Validation
    example_validation()?;

    // Example 5: Aggressive recovery
    example_aggressive_recovery()?;

    println!("\n✓ All recovery examples completed!");

    Ok(())
}

/// Create test files with various types of corruption
fn create_test_files() -> Result<(), Box<dyn std::error::Error>> {
    println!("Creating test files with various corruption types...\n");

    // 1. Missing header
    let mut file = File::create("output/recovery/no_header.pdf")?;
    writeln!(file, "1 0 obj")?;
    writeln!(file, "<< /Type /Catalog >>")?;
    writeln!(file, "endobj")?;
    writeln!(file, "%%EOF")?;

    // 2. Truncated file
    let mut file = File::create("output/recovery/truncated.pdf")?;
    writeln!(file, "%PDF-1.7")?;
    writeln!(file, "1 0 obj")?;
    write!(file, "<< /Type /Catalog")?; // Incomplete

    // 3. Missing EOF
    let mut file = File::create("output/recovery/no_eof.pdf")?;
    writeln!(file, "%PDF-1.7")?;
    writeln!(file, "1 0 obj")?;
    writeln!(file, "<< /Type /Catalog >>")?;
    writeln!(file, "endobj")?;
    // Missing %%EOF

    // 4. Corrupted but recoverable
    let mut file = File::create("output/recovery/recoverable.pdf")?;
    writeln!(file, "%PDF-1.7")?;
    writeln!(file, "GARBAGE DATA HERE")?;
    writeln!(file, "3 0 obj")?;
    writeln!(file, "<< /Type /Page /MediaBox [0 0 612 792] >>")?;
    writeln!(file, "endobj")?;
    writeln!(file, "5 0 obj")?;
    writeln!(file, "<< /Length 44 >>")?;
    writeln!(file, "stream")?;
    writeln!(file, "BT /F1 12 Tf 100 700 Td (Recovered text) Tj ET")?;
    writeln!(file, "endstream")?;
    writeln!(file, "endobj")?;
    writeln!(file, "MORE GARBAGE")?;
    writeln!(file, "%%EOF")?;

    // 5. Valid PDF for comparison
    let mut doc = Document::new();
    doc.set_title("Valid Test PDF");
    let mut page = Page::a4();
    page.text()
        .set_font(Font::Helvetica, 14.0)
        .at(50.0, 750.0)
        .write("This is a valid PDF for comparison")?;
    doc.add_page(page);
    doc.save("output/recovery/valid.pdf")?;

    Ok(())
}

/// Example 1: Detect corruption in PDF files
fn example_detect_corruption() -> Result<(), Box<dyn std::error::Error>> {
    println!("Example 1: Corruption Detection");
    println!("-------------------------------");

    let test_files = vec![
        ("valid.pdf", "Valid PDF"),
        ("no_header.pdf", "Missing header"),
        ("truncated.pdf", "Truncated file"),
        ("no_eof.pdf", "Missing EOF"),
        ("recoverable.pdf", "Corrupted but recoverable"),
    ];

    for (filename, description) in test_files {
        let path = format!("output/recovery/{}", filename);

        print!("Analyzing {} ({})... ", filename, description);

        match detect_corruption(&path) {
            Ok(report) => {
                if report.severity == 0 {
                    println!("✓ No corruption detected");
                } else {
                    println!("✗ Corruption detected!");
                    println!("  - Type: {:?}", report.corruption_type);
                    println!("  - Severity: {}/10", report.severity);
                    println!("  - Errors: {}", report.errors.len());
                    if !report.recoverable_sections.is_empty() {
                        println!(
                            "  - Recoverable sections: {}",
                            report.recoverable_sections.len()
                        );
                    }
                }
            }
            Err(e) => println!("✗ Error analyzing file: {}", e),
        }
    }

    println!();
    Ok(())
}

/// Example 2: Basic recovery
fn example_basic_recovery() -> Result<(), Box<dyn std::error::Error>> {
    println!("Example 2: Basic Recovery");
    println!("-------------------------");

    let options = RecoveryOptions::default()
        .with_partial_content(true)
        .with_max_errors(50);

    let mut recovery = PdfRecovery::new(options);

    // Try to recover the file with missing EOF
    println!("Attempting to recover 'no_eof.pdf'...");

    match recovery.recover_document("output/recovery/no_eof.pdf") {
        Ok(mut doc) => {
            println!("✓ Recovery successful!");
            println!("  - Pages recovered: {}", doc.page_count());

            // Save recovered document
            doc.save("output/recovery/no_eof_recovered.pdf")?;
            println!("  - Saved as: no_eof_recovered.pdf");
        }
        Err(e) => {
            println!("✗ Full recovery failed: {}", e);
            println!("  Warnings: {:?}", recovery.warnings());
        }
    }

    println!();
    Ok(())
}

/// Example 3: Partial recovery
fn example_partial_recovery() -> Result<(), Box<dyn std::error::Error>> {
    println!("Example 3: Partial Recovery");
    println!("---------------------------");

    let options = RecoveryOptions::default().with_aggressive_recovery(true);

    let mut recovery = PdfRecovery::new(options);

    println!("Attempting partial recovery of 'recoverable.pdf'...");

    match recovery.recover_partial("output/recovery/recoverable.pdf") {
        Ok(partial) => {
            println!("✓ Partial recovery completed:");
            println!("  - Total objects found: {}", partial.total_objects);
            println!("  - Objects recovered: {}", partial.recovered_objects);
            println!("  - Pages recovered: {}", partial.recovered_pages.len());

            if let Some(metadata) = &partial.metadata {
                println!("  - Metadata entries: {}", metadata.len());
            }

            // Display recovered content
            for page in &partial.recovered_pages {
                println!(
                    "  - Page {}: {} content",
                    page.page_number,
                    if page.has_text { "has" } else { "no" }
                );
            }

            // Create a new document from recovered pages
            if !partial.recovered_pages.is_empty() {
                let mut doc = Document::new();
                for recovered_page in &partial.recovered_pages {
                    let mut page = Page::a4();
                    page.text()
                        .set_font(Font::Helvetica, 12.0)
                        .at(50.0, 750.0)
                        .write(&format!("Recovered Page {}", recovered_page.page_number))?;
                    page.text().at(50.0, 700.0).write(&recovered_page.content)?;
                    doc.add_page(page);
                }
                doc.save("output/recovery/partial_recovery.pdf")?;
                println!("  - Saved recovered content as: partial_recovery.pdf");
            }
        }
        Err(e) => {
            println!("✗ Partial recovery failed: {}", e);
        }
    }

    println!();
    Ok(())
}

/// Example 4: PDF validation
fn example_validation() -> Result<(), Box<dyn std::error::Error>> {
    println!("Example 4: PDF Validation");
    println!("-------------------------");

    let files = vec!["valid.pdf", "no_header.pdf", "no_eof_recovered.pdf"];

    for filename in files {
        let path = format!("output/recovery/{}", filename);

        if !Path::new(&path).exists() {
            continue;
        }

        print!("Validating {}... ", filename);

        match validate_pdf(&path) {
            Ok(result) => {
                if result.is_valid {
                    println!("✓ Valid");
                } else {
                    println!("✗ Invalid");
                    println!("  - Errors: {}", result.errors.len());
                    for error in &result.errors {
                        println!("    • {:?}", error);
                    }
                }

                if !result.warnings.is_empty() {
                    println!("  - Warnings: {}", result.warnings.len());
                    for warning in &result.warnings {
                        println!("    • {}", warning);
                    }
                }

                println!(
                    "  - Stats: {} objects checked, {} pages validated",
                    result.stats.objects_checked, result.stats.pages_validated
                );
            }
            Err(e) => println!("✗ Validation error: {}", e),
        }
    }

    println!();
    Ok(())
}

/// Example 5: Aggressive recovery with multiple strategies
fn example_aggressive_recovery() -> Result<(), Box<dyn std::error::Error>> {
    println!("Example 5: Aggressive Recovery");
    println!("------------------------------");

    // Create a heavily corrupted file
    let mut file = File::create("output/recovery/heavily_corrupted.pdf")?;
    writeln!(file, "CORRUPTED HEADER")?;
    writeln!(file, "Random garbage data here")?;
    writeln!(file, "5 0 obj")?;
    writeln!(file, "<< /Type /Page /MediaBox [0 0 612 792] >>")?;
    writeln!(file, "endobj")?;
    writeln!(file, "More corruption")?;
    writeln!(file, "7 0 obj")?;
    writeln!(file, "<< /Length 50 >> stream")?;
    writeln!(file, "BT /F1 14 Tf 100 600 Td (Some text content) Tj ET")?;
    writeln!(file, "endstream")?;
    writeln!(file, "endobj")?;
    writeln!(file, "Final corruption")?;
    drop(file);

    println!("Created heavily corrupted test file");

    let options = RecoveryOptions::default()
        .with_aggressive_recovery(true)
        .with_partial_content(true)
        .with_max_errors(200);

    let mut recovery = PdfRecovery::new(options);

    println!("Attempting aggressive recovery...");

    match recovery.recover_document("output/recovery/heavily_corrupted.pdf") {
        Ok(mut doc) => {
            println!("✓ Aggressive recovery succeeded!");
            println!("  - Pages: {}", doc.page_count());
            doc.save("output/recovery/aggressive_recovered.pdf")?;
            println!("  - Saved as: aggressive_recovered.pdf");
        }
        Err(_) => {
            println!("✗ Full recovery failed, trying partial recovery...");

            if let Ok(partial) = recovery.recover_partial("output/recovery/heavily_corrupted.pdf") {
                println!("✓ Partial recovery succeeded:");
                println!(
                    "  - Objects recovered: {}/{}",
                    partial.recovered_objects, partial.total_objects
                );
                println!("  - Pages found: {}", partial.recovered_pages.len());
            }
        }
    }

    println!("\nRecovery warnings:");
    for warning in recovery.warnings() {
        println!("  • {}", warning);
    }

    println!();
    Ok(())
}
