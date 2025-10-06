//! Corrupted PDF Recovery Example
//!
//! Demonstrates oxidize-pdf's robust error recovery capabilities for handling
//! corrupted or malformed PDF files.
//!
//! # Features Demonstrated
//!
//! - Automatic XRef recovery for broken cross-reference tables
//! - Lenient parsing mode for malformed structures
//! - Partial content extraction from damaged files
//! - Recovery statistics and reporting
//! - Multiple recovery strategies
//!
//! # Use Cases
//!
//! - Processing PDFs from unreliable sources
//! - Recovering data from damaged archives
//! - Building resilient document processing pipelines
//! - Handling legacy or poorly-generated PDFs
//!
//! # Run Example
//!
//! ```bash
//! cargo run --example recovery_corrupted_pdf
//! ```

use oxidize_pdf::recovery::{quick_recover, PdfRecovery, RecoveryOptions};
use oxidize_pdf::{Document, Page, PdfReader};
use std::fs;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== oxidize-pdf: Corrupted PDF Recovery Example ===\n");

    // Create examples/results directory
    fs::create_dir_all("examples/results")?;

    // Example 1: Quick recovery with defaults
    println!("📋 Example 1: Quick Recovery");
    println!("─────────────────────────────");
    demonstrate_quick_recovery()?;

    // Example 2: Custom recovery options
    println!("\n📋 Example 2: Custom Recovery Options");
    println!("─────────────────────────────────────");
    demonstrate_custom_recovery()?;

    // Example 3: Partial content extraction
    println!("\n📋 Example 3: Partial Content Extraction");
    println!("───────────────────────────────────────");
    demonstrate_partial_extraction()?;

    // Example 4: Recovery statistics
    println!("\n📋 Example 4: Recovery Statistics");
    println!("────────────────────────────────");
    demonstrate_recovery_stats()?;

    println!("\n✅ All examples completed successfully!");
    println!("📁 Output files: examples/results/");

    Ok(())
}

/// Example 1: Quick recovery with default settings
fn demonstrate_quick_recovery() -> Result<(), Box<dyn std::error::Error>> {
    println!("Using quick_recover() for simple cases...\n");

    // Create a slightly corrupted PDF for testing
    let test_pdf_path = "examples/results/test_corrupted.pdf";
    create_test_pdf_with_issues(test_pdf_path)?;

    // Try quick recovery
    match quick_recover(test_pdf_path) {
        Ok(mut doc) => {
            println!("✅ Document recovered successfully!");
            println!("   Pages recovered: {}", doc.page_count());

            // Save recovered document
            let output_path = "examples/results/recovered_quick.pdf";
            doc.save(output_path)?;
            println!("   Saved to: {}", output_path);
        }
        Err(e) => {
            println!("⚠️  Quick recovery failed: {}", e);
            println!("   Try custom recovery options for better results");
        }
    }

    Ok(())
}

/// Example 2: Custom recovery with aggressive mode
fn demonstrate_custom_recovery() -> Result<(), Box<dyn std::error::Error>> {
    println!("Using custom RecoveryOptions for maximum resilience...\n");

    // Configure aggressive recovery
    let options = RecoveryOptions::default()
        .with_aggressive_recovery(true)
        .with_partial_content(true)
        .with_max_errors(200)
        .with_memory_limit(100 * 1024 * 1024); // 100MB

    let mut recovery = PdfRecovery::new(options);

    let test_pdf_path = "examples/results/test_corrupted.pdf";

    match recovery.recover_document(test_pdf_path) {
        Ok(mut doc) => {
            println!("✅ Document recovered with custom options!");
            println!("   Pages: {}", doc.page_count());

            // Show recovery warnings
            let warnings = recovery.warnings();
            if !warnings.is_empty() {
                println!("\n   Recovery warnings:");
                for warning in warnings.iter().take(5) {
                    println!("   - {}", warning);
                }
                if warnings.len() > 5 {
                    println!("   ... and {} more warnings", warnings.len() - 5);
                }
            }

            let output_path = "examples/results/recovered_custom.pdf";
            doc.save(output_path)?;
            println!("\n   Saved to: {}", output_path);
        }
        Err(e) => {
            println!("❌ Recovery failed: {}", e);
        }
    }

    Ok(())
}

/// Example 3: Partial content extraction when full recovery fails
fn demonstrate_partial_extraction() -> Result<(), Box<dyn std::error::Error>> {
    println!("Attempting partial content extraction...\n");

    let options = RecoveryOptions::default()
        .with_partial_content(true)
        .with_aggressive_recovery(true);

    let mut recovery = PdfRecovery::new(options);
    let test_pdf_path = "examples/results/test_corrupted.pdf";

    match recovery.recover_partial(test_pdf_path) {
        Ok(partial) => {
            println!("✅ Partial recovery successful!");
            println!("   Total objects found: {}", partial.total_objects);
            println!("   Objects recovered: {}", partial.recovered_objects);
            println!("   Pages recovered: {}", partial.recovered_pages.len());

            if partial.total_objects > 0 {
                let recovery_rate =
                    (partial.recovered_objects as f64 / partial.total_objects as f64) * 100.0;
                println!("   Recovery rate: {:.1}%", recovery_rate);
            }

            // Show recovered page info
            if !partial.recovered_pages.is_empty() {
                println!("\n   Recovered pages:");
                for page in partial.recovered_pages.iter().take(3) {
                    println!(
                        "   - Page {}: {} text, {} images",
                        page.page_number,
                        if page.has_text { "has" } else { "no" },
                        if page.has_images { "has" } else { "no" }
                    );
                }
            }

            // Show metadata
            if let Some(metadata) = &partial.metadata {
                println!("\n   Recovered metadata:");
                for (key, value) in metadata.iter().take(3) {
                    println!("   - {}: {}", key, value);
                }
            }
        }
        Err(e) => {
            println!("❌ Partial recovery failed: {}", e);
        }
    }

    Ok(())
}

/// Example 4: Recovery with detailed statistics
fn demonstrate_recovery_stats() -> Result<(), Box<dyn std::error::Error>> {
    println!("Demonstrating recovery statistics and reporting...\n");

    let test_pdf_path = "examples/results/test_corrupted.pdf";

    // Try standard parsing first to show the difference
    println!("Attempting standard parsing...");
    match PdfReader::open_document(test_pdf_path) {
        Ok(_) => println!("✅ Standard parsing succeeded (no recovery needed)"),
        Err(e) => println!("❌ Standard parsing failed: {}", e),
    }

    println!("\nAttempting recovery parsing...");

    let mut recovery = PdfRecovery::new(RecoveryOptions::default());

    match recovery.recover_document(test_pdf_path) {
        Ok(doc) => {
            println!("✅ Recovery parsing succeeded!");
            println!("\n📊 Recovery Statistics:");
            println!("   ├─ Pages: {}", doc.page_count());
            println!("   ├─ Warnings: {}", recovery.warnings().len());

            let warnings = recovery.warnings();
            if !warnings.is_empty() {
                println!("   └─ Recovery details:");
                for (i, warning) in warnings.iter().enumerate() {
                    let prefix = if i == warnings.len() - 1 {
                        "      └─"
                    } else {
                        "      ├─"
                    };
                    println!("{} {}", prefix, warning);
                }
            }
        }
        Err(e) => {
            println!("❌ Even recovery parsing failed: {}", e);
            println!("   This PDF may be severely damaged");
        }
    }

    Ok(())
}

/// Helper: Create a test PDF with intentional issues for demonstration
fn create_test_pdf_with_issues(path: &str) -> Result<(), Box<dyn std::error::Error>> {
    // Create a simple but valid PDF
    let mut doc = Document::new();

    let mut page1 = Page::new(595.0, 842.0); // A4
    page1
        .text()
        .set_font(oxidize_pdf::text::Font::Helvetica, 14.0)
        .at(50.0, 800.0)
        .write("Corrupted PDF Recovery Test")?;

    page1
        .text()
        .at(50.0, 750.0)
        .write("This PDF demonstrates recovery capabilities.")?;

    doc.add_page(page1);

    let mut page2 = Page::new(595.0, 842.0);
    page2
        .text()
        .set_font(oxidize_pdf::text::Font::Helvetica, 12.0)
        .at(50.0, 800.0)
        .write("Page 2 - Additional content")?;

    doc.add_page(page2);

    doc.save(path)?;

    // Note: In a real scenario, the PDF would be actually corrupted
    // For this example, we use a valid PDF to ensure the example runs
    // oxidize-pdf's recovery features will gracefully handle it

    Ok(())
}
