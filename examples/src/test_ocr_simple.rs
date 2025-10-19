//! Simple OCR test to verify functionality
//!
//! This example creates a simple PDF with scanned-like content and tests OCR conversion

use oxidize_pdf::operations::pdf_ocr_converter::{ConversionOptions, PdfOcrConverter};
use oxidize_pdf::text::{OcrOptions, RustyTesseractProvider};
use oxidize_pdf::{Color, Document, Font, Page};
use std::fs;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔍 Testing OCR PDF Conversion");

    // First, let's create a simple PDF with text that simulates a scanned document
    create_test_pdf()?;

    // Test the OCR conversion
    test_ocr_conversion()?;

    println!("✅ OCR test completed successfully!");
    Ok(())
}

fn create_test_pdf() -> Result<(), Box<dyn std::error::Error>> {
    println!("📄 Creating test PDF with text...");

    let mut doc = Document::new();
    let mut page = Page::a4();

    // Add some text that simulates a scanned document
    page.text()
        .set_font(Font::Helvetica, 14.0)
        .at(50.0, 750.0)
        .write("This is a test document for OCR")?;

    page.text()
        .set_font(Font::Helvetica, 12.0)
        .at(50.0, 720.0)
        .write("It contains multiple lines of text")?;

    page.text()
        .set_font(Font::Helvetica, 12.0)
        .at(50.0, 690.0)
        .write("That should be converted to searchable content")?;

    // Add a gray rectangle to simulate image background
    page.graphics()
        .set_fill_color(Color::rgb(0.95, 0.95, 0.95))
        .rect(40.0, 680.0, 500.0, 100.0)
        .fill();

    doc.add_page(page);

    // Save the test PDF
    let pdf_bytes = doc.to_bytes()?;
    fs::write("examples/results/test_input.pdf", pdf_bytes)?;

    println!("✅ Test PDF created: examples/results/test_input.pdf");
    Ok(())
}

fn test_ocr_conversion() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔧 Testing OCR conversion...");

    // Initialize Tesseract OCR provider
    let ocr_provider = RustyTesseractProvider::new();
    println!("✅ Tesseract OCR provider initialized");

    let converter = PdfOcrConverter::new()?;
    let options = ConversionOptions {
        ocr_options: OcrOptions {
            language: "eng".to_string(),
            min_confidence: 0.5, // Lower threshold for testing
            ..Default::default()
        },
        min_confidence: 0.5,
        skip_text_pages: false, // Process all pages for testing
        ..Default::default()
    };

    // Convert the test PDF
    match converter.convert_to_searchable_pdf(
        "examples/results/test_input.pdf",
        "examples/results/test_output_searchable.pdf",
        &ocr_provider,
        &options,
    ) {
        Ok(result) => {
            println!("🎉 OCR conversion successful!");
            println!("   Pages processed: {}", result.pages_processed);
            println!("   Pages with OCR: {}", result.pages_ocr_processed);
            println!(
                "   Processing time: {:.2}s",
                result.processing_time.as_secs_f64()
            );
            println!(
                "   Average confidence: {:.1}%",
                result.average_confidence * 100.0
            );
            println!(
                "   Characters extracted: {}",
                result.total_characters_extracted
            );
            println!("✅ Output saved: examples/results/test_output_searchable.pdf");
        }
        Err(e) => {
            println!("❌ OCR conversion failed: {}", e);
            println!("This might be due to:");
            println!("  - Missing Tesseract installation");
            println!("  - Incompatible image format");
            println!("  - Insufficient text in test PDF");
        }
    }

    Ok(())
}
