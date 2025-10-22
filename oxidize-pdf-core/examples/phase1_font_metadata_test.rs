//! Phase 1: Font Metadata Extraction Test
//!
//! Tests that TextFragment correctly captures font_name, is_bold, and is_italic.
//!
//! Run with: cargo run --example phase1_font_metadata_test

use oxidize_pdf::document::Document;
use oxidize_pdf::parser::{PdfDocument, PdfReader};
use oxidize_pdf::text::extraction::{ExtractionOptions, TextExtractor};
use oxidize_pdf::text::Font;
use oxidize_pdf::writer::{PdfWriter, WriterConfig};
use oxidize_pdf::Page;
use std::fs::File;
use std::io::BufWriter;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔤 Phase 1: Font Metadata Extraction Test\n");

    // Step 1: Create PDF with multiple fonts
    println!("1. Creating PDF with multiple font styles...");
    let mut doc = Document::new();
    let mut page = Page::new(595.0, 842.0); // A4 size

    // Regular text
    page.text()
        .set_font(Font::Helvetica, 14.0)
        .at(50.0, 750.0)
        .write("Regular text (Helvetica)")?;

    // Bold text
    page.text()
        .set_font(Font::HelveticaBold, 14.0)
        .at(50.0, 720.0)
        .write("Bold text (Helvetica-Bold)")?;

    // Italic text
    page.text()
        .set_font(Font::HelveticaOblique, 14.0)
        .at(50.0, 690.0)
        .write("Italic text (Helvetica-Oblique)")?;

    // Bold + Italic
    page.text()
        .set_font(Font::HelveticaBoldOblique, 14.0)
        .at(50.0, 660.0)
        .write("Bold Italic text (Helvetica-BoldOblique)")?;

    // Times family
    page.text()
        .set_font(Font::TimesRoman, 14.0)
        .at(50.0, 620.0)
        .write("Times Regular")?;

    page.text()
        .set_font(Font::TimesBold, 14.0)
        .at(50.0, 590.0)
        .write("Times Bold")?;

    page.text()
        .set_font(Font::TimesItalic, 14.0)
        .at(50.0, 560.0)
        .write("Times Italic")?;

    page.text()
        .set_font(Font::TimesBoldItalic, 14.0)
        .at(50.0, 530.0)
        .write("Times BoldItalic")?;

    doc.add_page(page);

    let pdf_path = "examples/results/phase1_font_metadata_test.pdf";
    std::fs::create_dir_all("examples/results")?;
    let file = File::create(pdf_path)?;
    let mut writer = PdfWriter::with_config(BufWriter::new(file), WriterConfig::default());
    writer.write_document(&mut doc)?;
    println!("   ✓ Created: {}\n", pdf_path);

    // Step 2: Extract text with font metadata
    println!("2. Extracting text with font metadata...");
    let file = File::open(pdf_path)?;
    let reader = PdfReader::new(file)?;
    let pdf_doc = PdfDocument::new(reader);

    let options = ExtractionOptions {
        preserve_layout: true,
        ..Default::default()
    };
    let mut extractor = TextExtractor::with_options(options);
    let extracted = extractor.extract_from_page(&pdf_doc, 0)?;

    println!("   ✓ Extracted {} fragments\n", extracted.fragments.len());

    // Step 3: Verify font metadata
    println!("3. Verifying font metadata:");
    println!("   {:<45} {:>10} {:>10} {:>8}", "Text", "Font", "Bold", "Italic");
    println!("   {}", "-".repeat(78));

    let mut tests_passed = 0;
    let mut tests_failed = 0;

    for fragment in &extracted.fragments {
        let font_name = fragment.font_name.as_deref().unwrap_or("Unknown");
        let text_preview = if fragment.text.len() > 40 {
            format!("{}...", &fragment.text[..37])
        } else {
            fragment.text.clone()
        };

        println!(
            "   {:<45} {:>10} {:>10} {:>8}",
            text_preview,
            font_name,
            if fragment.is_bold { "✓" } else { "-" },
            if fragment.is_italic { "✓" } else { "-" }
        );

        // Verify expectations
        let expected = match fragment.text.as_str() {
            t if t.contains("Regular text") || t.contains("Times Regular") => (false, false),
            t if t.contains("Bold Italic") || t.contains("BoldItalic") => (true, true),
            t if t.contains("Bold") => (true, false),
            t if t.contains("Italic") => (false, true),
            _ => continue,
        };

        if (fragment.is_bold, fragment.is_italic) == expected {
            tests_passed += 1;
        } else {
            tests_failed += 1;
            eprintln!(
                "   ❌ MISMATCH: '{}' expected {:?}, got ({}, {})",
                fragment.text, expected, fragment.is_bold, fragment.is_italic
            );
        }
    }

    println!("\n4. Test Results:");
    println!("   ✓ Passed: {}", tests_passed);
    if tests_failed > 0 {
        println!("   ❌ Failed: {}", tests_failed);
    }
    println!();

    if tests_failed == 0 && tests_passed >= 4 {
        println!("✅ Phase 1 Complete: Font metadata extraction working correctly!");
        Ok(())
    } else {
        eprintln!("❌ Phase 1 Failed: Font metadata extraction has issues");
        std::process::exit(1);
    }
}
