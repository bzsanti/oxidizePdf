//! Integration tests for watermark functionality

use std::io::Cursor;

use oxidize_pdf::operations::{
    PdfEditor, WatermarkLayer, WatermarkPageRange, WatermarkPosition, WatermarkSpec, Watermarker,
};
use oxidize_pdf::writer::{PdfWriter, WriterConfig};
use oxidize_pdf::{Document, Page};

/// Helper function to create a minimal valid PDF for testing
fn create_test_pdf(page_count: usize) -> Vec<u8> {
    let mut doc = Document::new();
    for _ in 0..page_count {
        // Create A4 page (595 x 842 points)
        let page = Page::new(595.0, 842.0);
        doc.add_page(page);
    }

    let config = WriterConfig::default();
    let mut output = Vec::new();
    {
        let cursor = Cursor::new(&mut output);
        let mut writer = PdfWriter::with_config(cursor, config);
        writer
            .write_document(&mut doc)
            .expect("Failed to create test PDF");
    }
    output
}

// TI3.1 - Agregar watermark de texto a PDF de 1 pagina
#[test]
fn test_watermark_text_single_page() {
    let pdf_bytes = create_test_pdf(1);
    let mut editor = PdfEditor::from_bytes(pdf_bytes).unwrap();

    let watermark = WatermarkSpec::text("CONFIDENTIAL")
        .with_opacity(0.3)
        .with_position(WatermarkPosition::Center);

    let result = Watermarker::apply(&mut editor, watermark, WatermarkPageRange::All);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 1); // 1 page watermarked
}

// TI3.2 - Agregar watermark a PDF de 5 paginas (todas)
#[test]
fn test_watermark_all_pages_5_page_pdf() {
    let pdf_bytes = create_test_pdf(5);
    let mut editor = PdfEditor::from_bytes(pdf_bytes).unwrap();

    let watermark = WatermarkSpec::text("DRAFT").with_opacity(0.25);

    let result = Watermarker::apply_to_all(&mut editor, watermark);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 5); // All 5 pages watermarked
}

// TI3.3 - Watermark diagonal (texto rotado 45 grados)
#[test]
fn test_watermark_diagonal_text() {
    let pdf_bytes = create_test_pdf(1);
    let mut editor = PdfEditor::from_bytes(pdf_bytes).unwrap();

    let watermark = WatermarkSpec::text("SAMPLE")
        .with_rotation(45.0)
        .with_opacity(0.4);

    let result = Watermarker::apply(&mut editor, watermark, WatermarkPageRange::All);
    assert!(result.is_ok());
}

// TI3.4 - Watermark en foreground vs background produce diferente configuracion
#[test]
fn test_watermark_foreground_vs_background() {
    // Test foreground watermark
    let pdf_bytes = create_test_pdf(1);
    let mut editor_fg = PdfEditor::from_bytes(pdf_bytes.clone()).unwrap();

    let watermark_fg = WatermarkSpec::text("FOREGROUND").with_layer(WatermarkLayer::Foreground);

    let result_fg = Watermarker::apply(&mut editor_fg, watermark_fg, WatermarkPageRange::All);
    assert!(result_fg.is_ok());

    // Test background watermark
    let mut editor_bg = PdfEditor::from_bytes(pdf_bytes).unwrap();

    let watermark_bg = WatermarkSpec::text("BACKGROUND").with_layer(WatermarkLayer::Background);

    let result_bg = Watermarker::apply(&mut editor_bg, watermark_bg, WatermarkPageRange::All);
    assert!(result_bg.is_ok());
}

// TI3.5 - Watermark de imagen (PNG bytes) en pagina
#[test]
fn test_watermark_image_stamp() {
    let pdf_bytes = create_test_pdf(1);
    let mut editor = PdfEditor::from_bytes(pdf_bytes).unwrap();

    // Create a minimal valid PNG (1x1 red pixel)
    let png_data = vec![
        0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, // PNG signature
        0x00, 0x00, 0x00, 0x0D, // IHDR length
        0x49, 0x48, 0x44, 0x52, // IHDR type
        0x00, 0x00, 0x00, 0x01, // width: 1
        0x00, 0x00, 0x00, 0x01, // height: 1
        0x08, 0x02, // bit depth: 8, color type: RGB
        0x00, 0x00, 0x00, // compression, filter, interlace
        0x90, 0x77, 0x53, 0xDE, // CRC
    ];

    let watermark = WatermarkSpec::image(png_data, 100.0, 50.0)
        .with_opacity(0.5)
        .with_position(WatermarkPosition::BottomRight);

    let result = Watermarker::apply(&mut editor, watermark, WatermarkPageRange::All);
    assert!(result.is_ok());
}

// TI3.6 - Output PDF parseable despues de watermark
#[test]
fn test_watermark_output_parseable() {
    let pdf_bytes = create_test_pdf(2);
    let mut editor = PdfEditor::from_bytes(pdf_bytes).unwrap();

    let watermark = WatermarkSpec::text("TEST")
        .with_opacity(0.3)
        .with_position(WatermarkPosition::Center);

    Watermarker::apply(&mut editor, watermark, WatermarkPageRange::All).unwrap();

    // Save and re-parse
    let output_bytes = editor.save_to_bytes().unwrap();

    // Verify the output is a valid PDF
    assert!(output_bytes.starts_with(b"%PDF-"));

    // Re-parse should succeed
    let editor2 = PdfEditor::from_bytes(output_bytes);
    assert!(editor2.is_ok());
    assert_eq!(editor2.unwrap().page_count(), 2);
}

// TI3.7 - Watermark en rango de paginas (1-3 de PDF de 5)
#[test]
fn test_watermark_page_range_partial() {
    let pdf_bytes = create_test_pdf(5);
    let mut editor = PdfEditor::from_bytes(pdf_bytes).unwrap();

    let watermark = WatermarkSpec::text("PARTIAL");

    // Apply to pages 1-3 (0-indexed: 1, 2, 3)
    let page_range = WatermarkPageRange::Range { start: 1, end: 3 };
    let result = Watermarker::apply(&mut editor, watermark, page_range);

    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 3); // Pages 1, 2, 3 = 3 pages
}

// Additional integration tests

#[test]
fn test_watermark_validates_opacity() {
    let pdf_bytes = create_test_pdf(1);
    let mut editor = PdfEditor::from_bytes(pdf_bytes).unwrap();

    // Invalid opacity should fail
    let watermark = WatermarkSpec::text("INVALID").with_opacity(1.5);

    let result = Watermarker::apply(&mut editor, watermark, WatermarkPageRange::All);
    assert!(result.is_err());
}

#[test]
fn test_watermark_empty_text_fails() {
    let pdf_bytes = create_test_pdf(1);
    let mut editor = PdfEditor::from_bytes(pdf_bytes).unwrap();

    // Empty text should fail
    let watermark = WatermarkSpec::text("");

    let result = Watermarker::apply(&mut editor, watermark, WatermarkPageRange::All);
    assert!(result.is_err());
}

#[test]
fn test_watermark_specific_pages() {
    let pdf_bytes = create_test_pdf(5);
    let mut editor = PdfEditor::from_bytes(pdf_bytes).unwrap();

    let watermark = WatermarkSpec::text("SPECIFIC");

    // Apply to specific pages: 0, 2, 4 (first, third, fifth)
    let page_range = WatermarkPageRange::Pages(vec![0, 2, 4]);
    let result = Watermarker::apply(&mut editor, watermark, page_range);

    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 3); // 3 specific pages
}

#[test]
fn test_watermark_single_page() {
    let pdf_bytes = create_test_pdf(5);
    let mut editor = PdfEditor::from_bytes(pdf_bytes).unwrap();

    let watermark = WatermarkSpec::text("SINGLE");

    // Apply to single page: page 2 (0-indexed)
    let page_range = WatermarkPageRange::Single(2);
    let result = Watermarker::apply(&mut editor, watermark, page_range);

    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 1); // 1 page
}

#[test]
fn test_watermark_out_of_bounds_page() {
    let pdf_bytes = create_test_pdf(3);
    let mut editor = PdfEditor::from_bytes(pdf_bytes).unwrap();

    let watermark = WatermarkSpec::text("OUT OF BOUNDS");

    // Single page out of bounds - should return 0 pages (filtered out)
    let page_range = WatermarkPageRange::Single(10);
    let result = Watermarker::apply(&mut editor, watermark, page_range);

    // With current implementation, out of bounds pages are filtered
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 0);
}

#[test]
fn test_multiple_watermarks() {
    let pdf_bytes = create_test_pdf(2);
    let mut editor = PdfEditor::from_bytes(pdf_bytes).unwrap();

    // Apply first watermark
    let watermark1 = WatermarkSpec::text("CONFIDENTIAL")
        .with_opacity(0.2)
        .with_position(WatermarkPosition::TopLeft);

    Watermarker::apply(&mut editor, watermark1, WatermarkPageRange::All).unwrap();

    // Apply second watermark
    let watermark2 = WatermarkSpec::text("DRAFT")
        .with_opacity(0.3)
        .with_position(WatermarkPosition::BottomRight);

    let result = Watermarker::apply(&mut editor, watermark2, WatermarkPageRange::All);
    assert!(result.is_ok());
}

#[test]
fn test_watermark_with_all_positions() {
    let pdf_bytes = create_test_pdf(1);

    let positions = vec![
        WatermarkPosition::Center,
        WatermarkPosition::TopLeft,
        WatermarkPosition::TopRight,
        WatermarkPosition::BottomLeft,
        WatermarkPosition::BottomRight,
        WatermarkPosition::Custom(100.0, 200.0),
    ];

    for position in positions {
        let mut editor = PdfEditor::from_bytes(pdf_bytes.clone()).unwrap();
        let watermark = WatermarkSpec::text("TEST").with_position(position);

        let result = Watermarker::apply(&mut editor, watermark, WatermarkPageRange::All);
        assert!(result.is_ok(), "Failed for position {:?}", position);
    }
}
