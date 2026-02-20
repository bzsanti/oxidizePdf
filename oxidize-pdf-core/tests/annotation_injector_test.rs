//! Integration tests for annotation injection functionality

use std::io::Cursor;

use oxidize_pdf::operations::{
    AnnotationColor, AnnotationIcon, AnnotationInjector, AnnotationInjectorError, AnnotationRect,
    HighlightAnnotationSpec, PdfEditor, StampAnnotationSpec, StampName, TextAnnotationSpec,
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

// TI6.1 - Add text note to single page PDF
#[test]
fn test_add_text_note_single_page() {
    let pdf_bytes = create_test_pdf(1);
    let mut editor = PdfEditor::from_bytes(pdf_bytes).unwrap();

    let spec = TextAnnotationSpec::new(100.0, 700.0, "Review this section")
        .with_icon(AnnotationIcon::Comment)
        .with_color(AnnotationColor::yellow())
        .with_open(true);

    let result = AnnotationInjector::add_text_note(&mut editor, 0, spec);
    assert!(result.is_ok());
    assert_eq!(editor.pending_annotation_count(), 1);
}

// TI6.2 - Add highlight annotation to PDF
#[test]
fn test_add_highlight_annotation() {
    let pdf_bytes = create_test_pdf(2);
    let mut editor = PdfEditor::from_bytes(pdf_bytes).unwrap();

    let rect = AnnotationRect::new(50.0, 700.0, 200.0, 20.0).unwrap();
    let spec = HighlightAnnotationSpec::from_rect(&rect).with_color(AnnotationColor::green());

    let result = AnnotationInjector::add_highlight(&mut editor, 0, spec);
    assert!(result.is_ok());
    assert_eq!(editor.pending_annotation_count(), 1);
}

// TI6.3 - Add URL link annotation
#[test]
fn test_add_url_link() {
    let pdf_bytes = create_test_pdf(1);
    let mut editor = PdfEditor::from_bytes(pdf_bytes).unwrap();

    let rect = AnnotationRect::new(100.0, 500.0, 150.0, 20.0).unwrap();

    let result =
        AnnotationInjector::add_link_url(&mut editor, 0, rect, "https://www.rust-lang.org");
    assert!(result.is_ok());
    assert_eq!(editor.pending_annotation_count(), 1);
}

// TI6.4 - Add internal page link
#[test]
fn test_add_internal_page_link() {
    let pdf_bytes = create_test_pdf(5);
    let mut editor = PdfEditor::from_bytes(pdf_bytes).unwrap();

    let rect = AnnotationRect::new(100.0, 750.0, 100.0, 20.0).unwrap();

    // Link from page 0 to page 4
    let result = AnnotationInjector::add_link_page(&mut editor, 0, rect, 4);
    assert!(result.is_ok());
    assert_eq!(editor.pending_annotation_count(), 1);
}

// TI6.5 - Add stamp annotation
#[test]
fn test_add_stamp_annotation() {
    let pdf_bytes = create_test_pdf(1);
    let mut editor = PdfEditor::from_bytes(pdf_bytes).unwrap();

    let rect = AnnotationRect::new(200.0, 600.0, 150.0, 50.0).unwrap();
    let spec = StampAnnotationSpec::new(rect, StampName::Approved)
        .with_contents("Approved by review team");

    let result = AnnotationInjector::add_stamp(&mut editor, 0, spec);
    assert!(result.is_ok());
    assert_eq!(editor.pending_annotation_count(), 1);
}

// TI6.6 - Error on out of bounds page index
#[test]
fn test_error_page_out_of_bounds() {
    let pdf_bytes = create_test_pdf(2);
    let mut editor = PdfEditor::from_bytes(pdf_bytes).unwrap();

    let spec = TextAnnotationSpec::new(100.0, 700.0, "Test");
    let result = AnnotationInjector::add_text_note(&mut editor, 10, spec);

    assert!(result.is_err());
    match result.unwrap_err() {
        AnnotationInjectorError::PageIndexOutOfBounds { index, page_count } => {
            assert_eq!(index, 10);
            assert_eq!(page_count, 2);
        }
        e => panic!("Expected PageIndexOutOfBounds, got {:?}", e),
    }
}

// TI6.7 - Error on invalid link target page
#[test]
fn test_error_invalid_link_target() {
    let pdf_bytes = create_test_pdf(3);
    let mut editor = PdfEditor::from_bytes(pdf_bytes).unwrap();

    let rect = AnnotationRect::new(100.0, 500.0, 100.0, 20.0).unwrap();

    // Try to link to page 10 which doesn't exist
    let result = AnnotationInjector::add_link_page(&mut editor, 0, rect, 10);

    assert!(result.is_err());
    match result.unwrap_err() {
        AnnotationInjectorError::InvalidLinkTarget { target, page_count } => {
            assert_eq!(target, 10);
            assert_eq!(page_count, 3);
        }
        e => panic!("Expected InvalidLinkTarget, got {:?}", e),
    }
}

// TI6.8 - Multiple annotations on same page
#[test]
fn test_multiple_annotations_same_page() {
    let pdf_bytes = create_test_pdf(1);
    let mut editor = PdfEditor::from_bytes(pdf_bytes).unwrap();

    // Add text note
    let text_spec = TextAnnotationSpec::new(100.0, 700.0, "Note 1");
    AnnotationInjector::add_text_note(&mut editor, 0, text_spec).unwrap();

    // Add another text note
    let text_spec2 = TextAnnotationSpec::new(200.0, 700.0, "Note 2");
    AnnotationInjector::add_text_note(&mut editor, 0, text_spec2).unwrap();

    // Add highlight
    let rect = AnnotationRect::new(50.0, 600.0, 200.0, 20.0).unwrap();
    let highlight_spec = HighlightAnnotationSpec::from_rect(&rect);
    AnnotationInjector::add_highlight(&mut editor, 0, highlight_spec).unwrap();

    // Add stamp
    let stamp_rect = AnnotationRect::new(300.0, 500.0, 100.0, 40.0).unwrap();
    let stamp_spec = StampAnnotationSpec::new(stamp_rect, StampName::Draft);
    AnnotationInjector::add_stamp(&mut editor, 0, stamp_spec).unwrap();

    assert_eq!(editor.pending_annotation_count(), 4);
}

// TI6.9 - Annotations on different pages
#[test]
fn test_annotations_different_pages() {
    let pdf_bytes = create_test_pdf(3);
    let mut editor = PdfEditor::from_bytes(pdf_bytes).unwrap();

    // Add annotation to page 0
    let spec0 = TextAnnotationSpec::new(100.0, 700.0, "Page 0 note");
    AnnotationInjector::add_text_note(&mut editor, 0, spec0).unwrap();

    // Add annotation to page 1
    let spec1 = TextAnnotationSpec::new(100.0, 700.0, "Page 1 note");
    AnnotationInjector::add_text_note(&mut editor, 1, spec1).unwrap();

    // Add annotation to page 2
    let spec2 = TextAnnotationSpec::new(100.0, 700.0, "Page 2 note");
    AnnotationInjector::add_text_note(&mut editor, 2, spec2).unwrap();

    assert_eq!(editor.pending_annotation_count(), 3);
}

// TI6.10 - PDF is parseable after adding annotations and saving
#[test]
fn test_annotated_pdf_parseable() {
    let pdf_bytes = create_test_pdf(2);
    let mut editor = PdfEditor::from_bytes(pdf_bytes).unwrap();

    // Add various annotations
    let text_spec = TextAnnotationSpec::new(100.0, 700.0, "Test note");
    AnnotationInjector::add_text_note(&mut editor, 0, text_spec).unwrap();

    let rect = AnnotationRect::new(50.0, 600.0, 200.0, 20.0).unwrap();
    AnnotationInjector::add_link_url(&mut editor, 1, rect, "https://example.com").unwrap();

    // Save and verify parseable
    let output_bytes = editor.save_to_bytes().unwrap();

    // Verify PDF header
    assert!(output_bytes.starts_with(b"%PDF-"));

    // Re-parse should succeed
    let editor2 = PdfEditor::from_bytes(output_bytes);
    assert!(editor2.is_ok());
    assert_eq!(editor2.unwrap().page_count(), 2);
}

// TI6.11 - Pending annotation count helper
#[test]
fn test_pending_annotation_count() {
    let pdf_bytes = create_test_pdf(1);
    let mut editor = PdfEditor::from_bytes(pdf_bytes).unwrap();

    assert_eq!(AnnotationInjector::pending_count(&editor), 0);

    let spec = TextAnnotationSpec::new(100.0, 700.0, "Test");
    AnnotationInjector::add_text_note(&mut editor, 0, spec).unwrap();

    assert_eq!(AnnotationInjector::pending_count(&editor), 1);
}

// TI6.12 - All stamp types can be added
#[test]
fn test_all_stamp_types() {
    let pdf_bytes = create_test_pdf(1);
    let mut editor = PdfEditor::from_bytes(pdf_bytes).unwrap();

    let stamps = [
        StampName::Approved,
        StampName::Draft,
        StampName::Confidential,
        StampName::Final,
        StampName::Expired,
        StampName::NotApproved,
        StampName::AsIs,
        StampName::Sold,
        StampName::Departmental,
        StampName::ForComment,
        StampName::TopSecret,
        StampName::NotForPublicRelease,
        StampName::ForPublicRelease,
        StampName::Experimental,
    ];

    for (i, stamp_name) in stamps.iter().enumerate() {
        let y = 800.0 - (i as f64 * 55.0);
        let rect = AnnotationRect::new(100.0, y, 100.0, 40.0).unwrap();
        let spec = StampAnnotationSpec::new(rect, *stamp_name);
        AnnotationInjector::add_stamp(&mut editor, 0, spec).unwrap();
    }

    assert_eq!(editor.pending_annotation_count(), stamps.len());
}

// TI6.13 - Highlight with quad points
#[test]
fn test_highlight_with_quad_points() {
    let pdf_bytes = create_test_pdf(1);
    let mut editor = PdfEditor::from_bytes(pdf_bytes).unwrap();

    // 8 points for one quad (4 corners x 2 coordinates)
    let quad_points = vec![
        50.0, 720.0, // top-left
        250.0, 720.0, // top-right
        50.0, 700.0, // bottom-left
        250.0, 700.0, // bottom-right
    ];

    let spec = HighlightAnnotationSpec::new(quad_points)
        .unwrap()
        .with_color(AnnotationColor::blue());

    let result = AnnotationInjector::add_highlight(&mut editor, 0, spec);
    assert!(result.is_ok());
}

// TI6.14 - All text annotation icons
#[test]
fn test_all_text_annotation_icons() {
    let pdf_bytes = create_test_pdf(1);
    let mut editor = PdfEditor::from_bytes(pdf_bytes).unwrap();

    let icons = [
        AnnotationIcon::Note,
        AnnotationIcon::Comment,
        AnnotationIcon::Key,
        AnnotationIcon::Help,
        AnnotationIcon::NewParagraph,
        AnnotationIcon::Paragraph,
        AnnotationIcon::Insert,
    ];

    for (i, icon) in icons.iter().enumerate() {
        let y = 800.0 - (i as f64 * 30.0);
        let spec = TextAnnotationSpec::new(100.0, y, format!("Icon: {:?}", icon)).with_icon(*icon);
        AnnotationInjector::add_text_note(&mut editor, 0, spec).unwrap();
    }

    assert_eq!(editor.pending_annotation_count(), icons.len());
}
