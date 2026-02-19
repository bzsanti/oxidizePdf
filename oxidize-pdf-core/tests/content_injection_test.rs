//! Integration tests for Content Injection
//!
//! These tests verify that text, images, and graphics can be injected
//! onto existing PDF pages and that the output is valid.

use oxidize_pdf::operations::{
    CircleInjectionSpec, ContentInjector, ImageFormat, LineInjectionSpec, PdfEditor,
    RectInjectionSpec, TextInjectionSpec,
};
use oxidize_pdf::text::Font;
use oxidize_pdf::{Document, Page};
use std::io::Cursor;

/// Helper function to create a minimal valid PDF for testing
fn create_test_pdf(page_count: usize) -> Vec<u8> {
    let mut doc = Document::new();
    for _ in 0..page_count {
        // Create A4 page (595 x 842 points)
        let page = Page::new(595.0, 842.0);
        doc.add_page(page);
    }

    let config = oxidize_pdf::writer::WriterConfig::default();
    let mut output = Vec::new();
    {
        let cursor = Cursor::new(&mut output);
        let mut writer = oxidize_pdf::writer::PdfWriter::with_config(cursor, config);
        writer
            .write_document(&mut doc)
            .expect("Failed to create test PDF");
    }
    output
}

// TI2.1 - Agregar texto a PDF existente
#[test]
fn test_injection_text_to_existing_pdf() {
    let pdf_bytes = create_test_pdf(1);
    let mut editor = PdfEditor::from_bytes(pdf_bytes).unwrap();

    let spec = TextInjectionSpec::new(100.0, 700.0, "Hello World");
    let result = ContentInjector::add_text(&mut editor, 0, spec);
    assert!(result.is_ok(), "Should add text without error");

    // Verify we can save the PDF
    let output = editor.save_to_bytes().unwrap();
    assert!(output.starts_with(b"%PDF-"), "Output should be valid PDF");
}

// TI2.2 - ContentInjector rechaza pagina fuera de rango
#[test]
fn test_injection_page_out_of_bounds() {
    let pdf_bytes = create_test_pdf(2);
    let mut editor = PdfEditor::from_bytes(pdf_bytes).unwrap();

    let spec = TextInjectionSpec::new(100.0, 100.0, "Test");
    let result = ContentInjector::add_text(&mut editor, 5, spec);

    assert!(result.is_err(), "Should error on out of bounds page");
}

// TI2.3 - Agregar multiple contenido a la misma pagina
#[test]
fn test_injection_multiple_items_same_page() {
    let pdf_bytes = create_test_pdf(1);
    let mut editor = PdfEditor::from_bytes(pdf_bytes).unwrap();

    // Add multiple text items
    for i in 0..3 {
        let y = 700.0 - (i as f64 * 50.0);
        let spec = TextInjectionSpec::new(100.0, y, format!("Line {}", i + 1));
        ContentInjector::add_text(&mut editor, 0, spec).unwrap();
    }

    // Verify PDF is still valid
    let output = editor.save_to_bytes().unwrap();
    let editor2 = PdfEditor::from_bytes(output).unwrap();
    assert_eq!(editor2.page_count(), 1);
}

// TI2.4 - Inyectar en todas las paginas de un PDF multipagina
#[test]
fn test_injection_all_pages() {
    let pdf_bytes = create_test_pdf(3);
    let mut editor = PdfEditor::from_bytes(pdf_bytes).unwrap();

    for i in 0..3 {
        let spec = TextInjectionSpec::new(100.0, 700.0, format!("Page {} header", i + 1));
        ContentInjector::add_text(&mut editor, i, spec).unwrap();
    }

    // Verify roundtrip
    let output = editor.save_to_bytes().unwrap();
    let editor2 = PdfEditor::from_bytes(output).unwrap();
    assert_eq!(editor2.page_count(), 3);
}

// TI2.5 - El PDF resultante es parseable (roundtrip)
#[test]
fn test_injection_output_parseable() {
    let pdf_bytes = create_test_pdf(2);
    let mut editor = PdfEditor::from_bytes(pdf_bytes).unwrap();

    // Add various content
    ContentInjector::add_text(
        &mut editor,
        0,
        TextInjectionSpec::new(50.0, 800.0, "Header"),
    )
    .unwrap();

    ContentInjector::add_line(
        &mut editor,
        0,
        LineInjectionSpec::new(50.0, 750.0, 545.0, 750.0),
    )
    .unwrap();

    ContentInjector::add_rect(
        &mut editor,
        1,
        RectInjectionSpec::new(100.0, 500.0, 200.0, 100.0),
    )
    .unwrap();

    // First roundtrip
    let output1 = editor.save_to_bytes().unwrap();
    let mut editor2 = PdfEditor::from_bytes(output1).unwrap();

    // Second roundtrip
    let output2 = editor2.save_to_bytes().unwrap();
    let editor3 = PdfEditor::from_bytes(output2).unwrap();

    assert_eq!(editor3.page_count(), 2, "Page count should be preserved");
}

// TI2.6 - Coordenadas negativas no causan panic
#[test]
fn test_injection_negative_coordinates() {
    let pdf_bytes = create_test_pdf(1);
    let mut editor = PdfEditor::from_bytes(pdf_bytes).unwrap();

    // Negative coordinates are valid in PDF (content off-page)
    let spec = TextInjectionSpec::new(-50.0, -100.0, "Off-page text");
    let result = ContentInjector::add_text(&mut editor, 0, spec);
    assert!(result.is_ok(), "Negative coordinates should be allowed");
}

// TI2.7 - Agregar lineas y rectangulos
#[test]
fn test_injection_graphics() {
    let pdf_bytes = create_test_pdf(1);
    let mut editor = PdfEditor::from_bytes(pdf_bytes).unwrap();

    // Add line
    let line = LineInjectionSpec::new(0.0, 0.0, 595.0, 842.0);
    ContentInjector::add_line(&mut editor, 0, line).unwrap();

    // Add rectangle
    let rect = RectInjectionSpec::new(100.0, 100.0, 395.0, 642.0);
    ContentInjector::add_rect(&mut editor, 0, rect).unwrap();

    // Add circle
    let circle = CircleInjectionSpec::new(297.5, 421.0, 100.0);
    ContentInjector::add_circle(&mut editor, 0, circle).unwrap();

    // Verify PDF is valid
    let output = editor.save_to_bytes().unwrap();
    assert!(output.starts_with(b"%PDF-"));
}

// TI2.8 - ImageFormat detection
#[test]
fn test_image_format_detection() {
    // JPEG header
    let jpeg_data = vec![
        0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10, 0x4A, 0x46, 0x49, 0x46, 0x00, 0x01,
    ];
    assert_eq!(ImageFormat::detect(&jpeg_data), Some(ImageFormat::Jpeg));

    // PNG header
    let png_data = vec![
        0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D,
    ];
    assert_eq!(ImageFormat::detect(&png_data), Some(ImageFormat::Png));

    // Unknown format
    let unknown_data = vec![0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07];
    assert_eq!(ImageFormat::detect(&unknown_data), None);
}

// TI2.9 - Text con diferentes fuentes
#[test]
fn test_injection_text_with_fonts() {
    let pdf_bytes = create_test_pdf(1);
    let mut editor = PdfEditor::from_bytes(pdf_bytes).unwrap();

    // Add text with different fonts
    let fonts = [
        Font::Helvetica,
        Font::TimesRoman,
        Font::Courier,
        Font::HelveticaBold,
    ];

    for (i, font) in fonts.iter().enumerate() {
        let y = 800.0 - (i as f64 * 30.0);
        let spec =
            TextInjectionSpec::new(100.0, y, format!("{:?} font", font)).with_font(font.clone());
        ContentInjector::add_text(&mut editor, 0, spec).unwrap();
    }

    let output = editor.save_to_bytes().unwrap();
    assert!(output.starts_with(b"%PDF-"));
}

// TI2.10 - Batch operations
#[test]
fn test_injection_batch_operations() {
    let pdf_bytes = create_test_pdf(5);
    let mut editor = PdfEditor::from_bytes(pdf_bytes).unwrap();

    // Add content to all pages in batch
    for page_idx in 0..5 {
        // Header text
        ContentInjector::add_text(
            &mut editor,
            page_idx,
            TextInjectionSpec::new(50.0, 800.0, format!("Page {}", page_idx + 1)),
        )
        .unwrap();

        // Separator line
        ContentInjector::add_line(
            &mut editor,
            page_idx,
            LineInjectionSpec::new(50.0, 790.0, 545.0, 790.0),
        )
        .unwrap();

        // Footer text
        ContentInjector::add_text(
            &mut editor,
            page_idx,
            TextInjectionSpec::new(250.0, 30.0, "Footer"),
        )
        .unwrap();
    }

    let output = editor.save_to_bytes().unwrap();
    let editor2 = PdfEditor::from_bytes(output).unwrap();
    assert_eq!(editor2.page_count(), 5);
}
