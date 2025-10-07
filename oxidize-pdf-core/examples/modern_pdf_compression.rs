//! Modern PDF Compression Demo (Features 2.2.1 + 2.2.2)
//!
//! Demonstrates the combined use of:
//! - Object Streams (ISO 32000-1 Section 7.5.7)
//! - Cross-Reference Streams (ISO 32000-1 Section 7.5.8)
//!
//! Expected file size reduction: 11-61% compared to legacy PDF 1.4

use oxidize_pdf::document::Document;
use oxidize_pdf::text::Font;
use oxidize_pdf::writer::{PdfWriter, WriterConfig};
use oxidize_pdf::Page;
use std::fs::{self, File};
use std::io::BufWriter;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Modern PDF Compression Demo ===\n");
    println!("Combining Object Streams + XRef Streams for maximum compression\n");

    // Create output directory
    let output_dir = "examples/results/modern_compression";
    fs::create_dir_all(output_dir)?;

    // Test 1: Legacy PDF 1.4 (no compression)
    println!("1. Legacy PDF 1.4 (baseline)");
    let legacy_path = format!("{}/legacy_1.4.pdf", output_dir);
    let mut doc1 = create_test_document()?;
    let legacy_size = write_pdf(&mut doc1, &legacy_path, WriterConfig::legacy())?;
    println!("   File size: {} bytes\n", legacy_size);

    // Test 2: PDF 1.5 with XRef Streams only
    println!("2. PDF 1.5 with XRef Streams only");
    let xref_only_path = format!("{}/xref_streams_only.pdf", output_dir);
    let xref_only_config = WriterConfig {
        use_xref_streams: true,
        use_object_streams: false,
        pdf_version: "1.5".to_string(),
        compress_streams: true,
    };
    let mut doc2 = create_test_document()?;
    let xref_only_size = write_pdf(&mut doc2, &xref_only_path, xref_only_config)?;
    let xref_reduction = calculate_reduction(legacy_size, xref_only_size);
    println!(
        "   File size: {} bytes ({:.1}% reduction)\n",
        xref_only_size, xref_reduction
    );

    // Test 3: PDF 1.5 Modern (XRef + Object Streams)
    println!("3. PDF 1.5 Modern (XRef + Object Streams) - FULLY INTEGRATED ✓");
    let modern_path = format!("{}/modern_1.5.pdf", output_dir);
    let modern_config = WriterConfig::modern();
    let mut doc3 = create_test_document()?;
    let modern_size = write_pdf(&mut doc3, &modern_path, modern_config)?;
    let modern_reduction = calculate_reduction(legacy_size, modern_size);
    println!(
        "   File size: {} bytes ({:.1}% reduction)",
        modern_size, modern_reduction
    );
    println!("   Note: Object streams automatically compress non-stream objects\n");

    // Summary
    println!("{}", "=".repeat(60));
    println!("SUMMARY");
    println!("{}", "=".repeat(60));
    println!(
        "Legacy PDF 1.4:           {:>10} bytes (baseline)",
        legacy_size
    );
    println!(
        "XRef Streams only:        {:>10} bytes (-{:.1}%)",
        xref_only_size, xref_reduction
    );
    println!(
        "Modern (full):            {:>10} bytes (-{:.1}%) ✓",
        modern_size, modern_reduction
    );
    println!();
    println!("Object Stream Compression:");
    println!("  - Non-stream objects automatically compressed");
    println!("  - XRef stream includes Type 2 entries for compressed objects");
    println!("  - Configurable via WriterConfig::modern()");
    println!();
    println!("Output files:");
    println!("  {}/", output_dir);
    println!();
    println!("Note: Actual reduction depends on document structure.");
    println!("      PDFs with many content streams will see lower reduction.");

    Ok(())
}

fn create_test_document() -> Result<Document, Box<dyn std::error::Error>> {
    let mut doc = Document::new();

    // Create multiple pages with content to have more objects
    for page_num in 0..5 {
        let mut page = Page::new(595.0, 842.0); // A4

        // Title
        page.text()
            .set_font(Font::Helvetica, 16.0)
            .at(50.0, 800.0)
            .write(&format!(
                "Page {} - Modern PDF Compression Test",
                page_num + 1
            ))?;

        // Subtitle
        page.text()
            .set_font(Font::Helvetica, 12.0)
            .at(50.0, 770.0)
            .write("Demonstrating Object Streams + XRef Streams")?;

        // Add paragraphs
        let mut y = 730.0;
        for para in 0..15 {
            page.text()
                .set_font(Font::Helvetica, 10.0)
                .at(50.0, y)
                .write(&format!(
                    "Paragraph {}: Object streams compress multiple objects together, reducing file size significantly.",
                    para + 1
                ))?;
            y -= 15.0;
        }

        // Add some rectangles (creates more graphics objects)
        for i in 0..5 {
            let x = 50.0 + (i as f64 * 100.0);
            page.graphics()
                .set_fill_color(oxidize_pdf::Color::rgb(0.3, 0.3, 0.8))
                .rect(x, 50.0, 80.0, 40.0)
                .fill();
        }

        doc.add_page(page);
    }

    Ok(doc)
}

fn write_pdf(
    doc: &mut Document,
    path: &str,
    config: WriterConfig,
) -> Result<u64, Box<dyn std::error::Error>> {
    let file = File::create(path)?;
    let mut writer = PdfWriter::with_config(BufWriter::new(file), config);
    writer.write_document(doc)?;
    Ok(fs::metadata(path)?.len())
}

fn calculate_reduction(baseline: u64, compressed: u64) -> f64 {
    if baseline == 0 {
        return 0.0;
    }
    ((baseline - compressed) as f64 / baseline as f64) * 100.0
}
