//! Integration tests for PdfEditor
//!
//! These tests verify the PdfEditor works correctly with real PDF parsing
//! and serialization, testing the full roundtrip flow.

use oxidize_pdf::operations::{ModificationError, PdfEditor, PdfEditorOptions};
use oxidize_pdf::{Document, Page};
use std::io::Cursor;
use tempfile::tempdir;

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

// TI1.1 - Load and save minimal PDF without modifications
#[test]
fn test_editor_load_and_save_minimal_pdf() {
    let pdf_bytes = create_test_pdf(1);
    let original_len = pdf_bytes.len();

    let mut editor = PdfEditor::from_bytes(pdf_bytes).unwrap();
    let output = editor.save_to_bytes().unwrap();

    // Output should be valid PDF
    assert!(output.starts_with(b"%PDF-"));

    // Output should end with %%EOF
    let output_str = String::from_utf8_lossy(&output);
    assert!(
        output_str.contains("%%EOF"),
        "PDF should end with %%EOF marker"
    );

    // Output should be reasonable size (not empty, not unreasonably large)
    assert!(output.len() > 100, "Output should be at least 100 bytes");
    assert!(
        output.len() < original_len * 5,
        "Output should not be unreasonably larger than input"
    );
}

// TI1.2 - Load multi-page PDF and verify page count
#[test]
fn test_editor_multi_page_pdf() {
    let pdf_bytes = create_test_pdf(3);

    let editor = PdfEditor::from_bytes(pdf_bytes).unwrap();
    assert_eq!(editor.page_count(), 3);

    // Verify each page can be accessed
    for i in 0..3 {
        let size = editor.get_page_size(i);
        assert!(size.is_ok(), "Should be able to get size of page {}", i);
    }

    // Page 3 (index 3) should be out of bounds
    assert!(editor.get_page_size(3).is_err());
}

// TI1.3 - Editor handles empty/malformed PDF gracefully
#[test]
fn test_editor_invalid_pdf_no_panic() {
    // Empty bytes should error, not panic
    let result = PdfEditor::from_bytes(vec![]);
    assert!(result.is_err());

    // Random bytes should error, not panic
    let result = PdfEditor::from_bytes(vec![1, 2, 3, 4, 5]);
    assert!(result.is_err());

    // Truncated PDF header should error
    let result = PdfEditor::from_bytes(b"%PDF".to_vec());
    assert!(result.is_err());
}

// TI1.4 - save_to_file creates readable file
#[test]
fn test_editor_save_to_file_readable() {
    let pdf_bytes = create_test_pdf(2);
    let mut editor = PdfEditor::from_bytes(pdf_bytes).unwrap();

    let temp_dir = tempdir().unwrap();
    let output_path = temp_dir.path().join("output.pdf");

    // Save to file
    editor.save(&output_path).unwrap();

    // File should exist and be readable
    assert!(output_path.exists());

    // Should be able to re-open the saved file
    let editor2 = PdfEditor::open(&output_path).unwrap();
    assert_eq!(editor2.page_count(), 2);
}

// TI1.5 - Various editor options work correctly
#[test]
fn test_editor_various_options() {
    let pdf_bytes = create_test_pdf(1);

    // Default options
    let options1 = PdfEditorOptions::default();
    assert!(options1.compress);
    assert!(!options1.incremental);

    // Incremental mode
    let options2 = PdfEditorOptions::default().with_incremental();
    assert!(options2.incremental);

    // No compression
    let options3 = PdfEditorOptions::default().with_compress(false);
    assert!(!options3.compress);

    // Chained options
    let options4 = PdfEditorOptions::default()
        .with_incremental()
        .with_compress(false);
    assert!(options4.incremental);
    assert!(!options4.compress);

    // All options should produce valid editors
    let editor = PdfEditor::from_bytes_with_options(pdf_bytes.clone(), options1).unwrap();
    assert_eq!(editor.page_count(), 1);

    let mut editor = PdfEditor::from_bytes_with_options(pdf_bytes.clone(), options3).unwrap();
    let output = editor.save_to_bytes().unwrap();
    assert!(output.starts_with(b"%PDF-"));
}

// Additional integration tests

#[test]
fn test_editor_preserves_page_dimensions() {
    // Create pages with different sizes
    let mut doc = Document::new();
    doc.add_page(Page::new(612.0, 792.0)); // US Letter
    doc.add_page(Page::new(595.0, 842.0)); // A4
    doc.add_page(Page::new(842.0, 595.0)); // A4 Landscape

    let config = oxidize_pdf::writer::WriterConfig::default();
    let mut output = Vec::new();
    {
        let cursor = Cursor::new(&mut output);
        let mut writer = oxidize_pdf::writer::PdfWriter::with_config(cursor, config);
        writer
            .write_document(&mut doc)
            .expect("Failed to write PDF");
    }

    let editor = PdfEditor::from_bytes(output).unwrap();

    // Verify dimensions are preserved
    let (w1, h1) = editor.get_page_size(0).unwrap();
    assert!((w1 - 612.0).abs() < 1.0);
    assert!((h1 - 792.0).abs() < 1.0);

    let (w2, h2) = editor.get_page_size(1).unwrap();
    assert!((w2 - 595.0).abs() < 1.0);
    assert!((h2 - 842.0).abs() < 1.0);

    let (w3, h3) = editor.get_page_size(2).unwrap();
    assert!((w3 - 842.0).abs() < 1.0);
    assert!((h3 - 595.0).abs() < 1.0);
}

#[test]
fn test_editor_roundtrip_multiple_times() {
    let pdf_bytes = create_test_pdf(2);

    // First roundtrip
    let mut editor1 = PdfEditor::from_bytes(pdf_bytes).unwrap();
    let bytes1 = editor1.save_to_bytes().unwrap();

    // Second roundtrip
    let mut editor2 = PdfEditor::from_bytes(bytes1).unwrap();
    let bytes2 = editor2.save_to_bytes().unwrap();

    // Third roundtrip
    let editor3 = PdfEditor::from_bytes(bytes2).unwrap();

    // All should have same page count
    assert_eq!(editor1.page_count(), 2);
    assert_eq!(editor2.page_count(), 2);
    assert_eq!(editor3.page_count(), 2);
}

#[test]
fn test_editor_file_not_found_error() {
    let result = PdfEditor::open("/nonexistent/path/to/file.pdf");
    assert!(result.is_err());

    match result.unwrap_err() {
        ModificationError::Io(io_err) => {
            assert_eq!(io_err.kind(), std::io::ErrorKind::NotFound);
        }
        e => panic!("Expected Io error, got {:?}", e),
    }
}

#[test]
fn test_editor_large_page_count() {
    // Create PDF with many pages
    let pdf_bytes = create_test_pdf(50);

    let mut editor = PdfEditor::from_bytes(pdf_bytes).unwrap();
    assert_eq!(editor.page_count(), 50);

    // Verify roundtrip with many pages
    let output = editor.save_to_bytes().unwrap();
    let editor2 = PdfEditor::from_bytes(output).unwrap();
    assert_eq!(editor2.page_count(), 50);
}

#[test]
fn test_editor_options_accessor() {
    let options = PdfEditorOptions::default()
        .with_compress(false)
        .with_incremental();

    let pdf_bytes = create_test_pdf(1);
    let editor = PdfEditor::from_bytes_with_options(pdf_bytes, options).unwrap();

    // Verify options are accessible
    let retrieved_options = editor.options();
    assert!(!retrieved_options.compress);
    assert!(retrieved_options.incremental);
}
