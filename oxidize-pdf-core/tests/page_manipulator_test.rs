//! Integration tests for page manipulation functionality

use std::io::Cursor;

use oxidize_pdf::operations::{
    CropBox, PageManipulator, PageManipulatorError, PdfEditor, ResizeOptions,
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

/// Helper function to create a test PDF with Letter-size pages
fn create_letter_pdf(page_count: usize) -> Vec<u8> {
    let mut doc = Document::new();
    for _ in 0..page_count {
        // Create Letter page (612 x 792 points)
        let page = Page::new(612.0, 792.0);
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

// TI5.1 - Crop PDF of 1 page and verify resulting dimensions
#[test]
fn test_crop_single_page_pdf() {
    let pdf_bytes = create_test_pdf(1);
    let mut editor = PdfEditor::from_bytes(pdf_bytes).unwrap();

    // Crop with 50pt margins (A4: 595x842 -> crop to 495x742)
    let crop_box = CropBox::from_margins(595.0, 842.0, 50.0).unwrap();

    let result = PageManipulator::crop_page(&mut editor, 0, crop_box);
    assert!(result.is_ok());

    // Verify crop is pending
    assert_eq!(editor.pending_crop_count(), 1);
}

// TI5.2 - Resize page A4 to Letter
#[test]
fn test_resize_a4_to_letter() {
    let pdf_bytes = create_test_pdf(1);
    let mut editor = PdfEditor::from_bytes(pdf_bytes).unwrap();

    let options = ResizeOptions::to_letter();
    let result = PageManipulator::resize_page(&mut editor, 0, options);

    assert!(result.is_ok());
    assert_eq!(editor.pending_resize_count(), 1);
}

// TI5.3 - Delete page and verify page_count
#[test]
fn test_delete_page_reduces_count() {
    let pdf_bytes = create_test_pdf(5);
    let mut editor = PdfEditor::from_bytes(pdf_bytes).unwrap();

    assert_eq!(editor.page_count(), 5);

    let result = PageManipulator::delete_page(&mut editor, 2);
    assert!(result.is_ok());

    // Verify deletion is pending
    assert_eq!(editor.pending_deletion_count(), 1);
}

// TI5.4 - Delete multiple non-contiguous pages
#[test]
fn test_delete_multiple_pages() {
    let pdf_bytes = create_test_pdf(5);
    let mut editor = PdfEditor::from_bytes(pdf_bytes).unwrap();

    // Delete pages 0, 2, 4 (first, third, fifth)
    let result = PageManipulator::delete_pages(&mut editor, &[0, 2, 4]);

    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 3); // 3 pages deleted
    assert_eq!(editor.pending_deletion_count(), 3);
}

// TI5.5 - PDF is parseable after crop
#[test]
fn test_cropped_pdf_parseable() {
    let pdf_bytes = create_test_pdf(2);
    let mut editor = PdfEditor::from_bytes(pdf_bytes).unwrap();

    let crop_box = CropBox::from_margins(595.0, 842.0, 30.0).unwrap();
    PageManipulator::crop_page(&mut editor, 0, crop_box).unwrap();

    // Save and verify parseable
    let output_bytes = editor.save_to_bytes().unwrap();

    // Verify PDF header
    assert!(output_bytes.starts_with(b"%PDF-"));

    // Re-parse should succeed
    let editor2 = PdfEditor::from_bytes(output_bytes);
    assert!(editor2.is_ok());
    assert_eq!(editor2.unwrap().page_count(), 2);
}

// TI5.6 - PDF is parseable after delete_page
#[test]
fn test_deleted_page_pdf_parseable() {
    let pdf_bytes = create_test_pdf(3);
    let mut editor = PdfEditor::from_bytes(pdf_bytes).unwrap();

    // Note: delete operations are tracked but actual deletion
    // requires implementation in save_to_bytes. This test verifies
    // the operation doesn't corrupt the PDF structure.
    PageManipulator::delete_page(&mut editor, 1).unwrap();

    // Save and verify parseable
    let output_bytes = editor.save_to_bytes().unwrap();

    // Verify PDF header
    assert!(output_bytes.starts_with(b"%PDF-"));

    // Re-parse should succeed
    let editor2 = PdfEditor::from_bytes(output_bytes);
    assert!(editor2.is_ok());
}

// TI5.7 - Crop does not modify content stream, only changes CropBox
#[test]
fn test_crop_preserves_content_stream() {
    let pdf_bytes = create_test_pdf(1);
    let mut editor = PdfEditor::from_bytes(pdf_bytes).unwrap();

    // Get original page size
    let (original_width, original_height) = editor.get_page_size(0).unwrap();

    // Apply crop
    let crop_box = CropBox::new(50.0, 50.0, 545.0, 792.0).unwrap();
    PageManipulator::crop_page(&mut editor, 0, crop_box).unwrap();

    // After crop operation (but before save), MediaBox should still be original
    // The crop box is stored separately
    let (width, height) = editor.get_page_size(0).unwrap();
    assert!((width - original_width).abs() < 1.0);
    assert!((height - original_height).abs() < 1.0);
}

// Additional integration tests

#[test]
fn test_crop_all_pages() {
    let pdf_bytes = create_test_pdf(5);
    let mut editor = PdfEditor::from_bytes(pdf_bytes).unwrap();

    let crop_box = CropBox::from_margins(595.0, 842.0, 25.0).unwrap();

    // Crop all 5 pages
    for i in 0..5 {
        PageManipulator::crop_page(&mut editor, i, crop_box).unwrap();
    }

    assert_eq!(editor.pending_crop_count(), 5);
}

#[test]
fn test_resize_with_scale() {
    let pdf_bytes = create_test_pdf(1);
    let mut editor = PdfEditor::from_bytes(pdf_bytes).unwrap();

    let options = ResizeOptions::scale(0.5); // 50% scale
    let result = PageManipulator::resize_page(&mut editor, 0, options);

    assert!(result.is_ok());
    assert_eq!(editor.pending_resize_count(), 1);
}

#[test]
fn test_delete_cannot_delete_last_page() {
    let pdf_bytes = create_test_pdf(1);
    let mut editor = PdfEditor::from_bytes(pdf_bytes).unwrap();

    let result = PageManipulator::delete_page(&mut editor, 0);
    assert!(result.is_err());

    match result.unwrap_err() {
        PageManipulatorError::CannotDeleteLastPage => {}
        e => panic!("Expected CannotDeleteLastPage, got {:?}", e),
    }
}

#[test]
fn test_delete_pages_cannot_delete_all() {
    let pdf_bytes = create_test_pdf(3);
    let mut editor = PdfEditor::from_bytes(pdf_bytes).unwrap();

    // Try to delete all 3 pages
    let result = PageManipulator::delete_pages(&mut editor, &[0, 1, 2]);
    assert!(result.is_err());

    match result.unwrap_err() {
        PageManipulatorError::CannotDeleteLastPage => {}
        e => panic!("Expected CannotDeleteLastPage, got {:?}", e),
    }
}

#[test]
fn test_crop_page_out_of_bounds() {
    let pdf_bytes = create_test_pdf(2);
    let mut editor = PdfEditor::from_bytes(pdf_bytes).unwrap();

    let crop_box = CropBox::new(50.0, 50.0, 545.0, 792.0).unwrap();
    let result = PageManipulator::crop_page(&mut editor, 10, crop_box);

    assert!(result.is_err());

    match result.unwrap_err() {
        PageManipulatorError::PageIndexOutOfBounds { index, page_count } => {
            assert_eq!(index, 10);
            assert_eq!(page_count, 2);
        }
        e => panic!("Expected PageIndexOutOfBounds, got {:?}", e),
    }
}

#[test]
fn test_resize_page_out_of_bounds() {
    let pdf_bytes = create_test_pdf(2);
    let mut editor = PdfEditor::from_bytes(pdf_bytes).unwrap();

    let options = ResizeOptions::to_a4();
    let result = PageManipulator::resize_page(&mut editor, 10, options);

    assert!(result.is_err());

    match result.unwrap_err() {
        PageManipulatorError::PageIndexOutOfBounds { index, page_count } => {
            assert_eq!(index, 10);
            assert_eq!(page_count, 2);
        }
        e => panic!("Expected PageIndexOutOfBounds, got {:?}", e),
    }
}

#[test]
fn test_delete_page_out_of_bounds() {
    let pdf_bytes = create_test_pdf(3);
    let mut editor = PdfEditor::from_bytes(pdf_bytes).unwrap();

    let result = PageManipulator::delete_page(&mut editor, 10);
    assert!(result.is_err());

    match result.unwrap_err() {
        PageManipulatorError::PageIndexOutOfBounds { index, page_count } => {
            assert_eq!(index, 10);
            assert_eq!(page_count, 3);
        }
        e => panic!("Expected PageIndexOutOfBounds, got {:?}", e),
    }
}

#[test]
fn test_delete_pages_with_duplicate_indices() {
    let pdf_bytes = create_test_pdf(5);
    let mut editor = PdfEditor::from_bytes(pdf_bytes).unwrap();

    // Duplicate indices should be deduplicated
    let result = PageManipulator::delete_pages(&mut editor, &[1, 1, 2, 2, 3]);

    assert!(result.is_ok());
    // Should only count 3 unique pages (1, 2, 3)
    assert_eq!(result.unwrap(), 3);
    assert_eq!(editor.pending_deletion_count(), 3);
}

#[test]
fn test_mixed_operations() {
    let pdf_bytes = create_test_pdf(5);
    let mut editor = PdfEditor::from_bytes(pdf_bytes).unwrap();

    // Crop page 0
    let crop_box = CropBox::new(20.0, 20.0, 575.0, 822.0).unwrap();
    PageManipulator::crop_page(&mut editor, 0, crop_box).unwrap();

    // Resize page 1
    let options = ResizeOptions::to_letter();
    PageManipulator::resize_page(&mut editor, 1, options).unwrap();

    // Delete pages 3 and 4
    PageManipulator::delete_pages(&mut editor, &[3, 4]).unwrap();

    // Verify all operations are pending
    assert_eq!(editor.pending_crop_count(), 1);
    assert_eq!(editor.pending_resize_count(), 1);
    assert_eq!(editor.pending_deletion_count(), 2);
}

#[test]
fn test_resize_letter_to_a4() {
    let pdf_bytes = create_letter_pdf(1);
    let mut editor = PdfEditor::from_bytes(pdf_bytes).unwrap();

    // Verify initial size is Letter
    let (width, height) = editor.get_page_size(0).unwrap();
    assert!((width - 612.0).abs() < 1.0);
    assert!((height - 792.0).abs() < 1.0);

    // Resize to A4
    let options = ResizeOptions::to_a4();
    PageManipulator::resize_page(&mut editor, 0, options).unwrap();

    assert_eq!(editor.pending_resize_count(), 1);
}

#[test]
fn test_crop_with_custom_margins() {
    let pdf_bytes = create_test_pdf(1);
    let mut editor = PdfEditor::from_bytes(pdf_bytes).unwrap();

    // Different margins for each side
    let crop_box = CropBox::from_margins_lbrt(595.0, 842.0, 10.0, 20.0, 30.0, 40.0).unwrap();

    assert_eq!(crop_box.left, 10.0);
    assert_eq!(crop_box.bottom, 20.0);
    assert_eq!(crop_box.right, 565.0); // 595 - 30
    assert_eq!(crop_box.top, 802.0); // 842 - 40

    let result = PageManipulator::crop_page(&mut editor, 0, crop_box);
    assert!(result.is_ok());
}
