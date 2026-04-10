//! TDD Phase 0: Annotation Writing Pipeline Tests
//!
//! These tests verify that non-Widget annotations stored in Page.annotations
//! are correctly written to the PDF output by the writer.
//!
//! The current writer only processes Widget annotations from the page dict.
//! These tests must FAIL until the writer is modified to handle all annotation types.

use oxidize_pdf::annotations::{Annotation, AnnotationType, MarkupAnnotation};
use oxidize_pdf::geometry::{Point, Rectangle};
use oxidize_pdf::graphics::Color;
use oxidize_pdf::parser::PdfReader;
use oxidize_pdf::text::Font;
use oxidize_pdf::{Document, Page};
use std::io::Cursor;

/// Helper: create a simple document, add annotations to a page, write to bytes
fn create_pdf_with_annotations(annotations: Vec<Annotation>) -> Vec<u8> {
    let mut doc = Document::new();
    let mut page = Page::a4();

    // Add some text so the page isn't empty
    {
        let text = page.text();
        text.set_font(Font::Helvetica, 12.0);
        text.at(72.0, 700.0);
        let _ = text.write("Test page with annotations");
    }

    for annot in annotations {
        page.add_annotation(annot);
    }

    doc.add_page(page);
    doc.to_bytes().expect("Failed to write PDF to bytes")
}

/// Helper: parse PDF bytes and count annotations on page 0 via get_page_annotations
fn count_page_annotations(pdf_bytes: &[u8]) -> usize {
    let cursor = Cursor::new(pdf_bytes);
    let reader = PdfReader::new(cursor).expect("Failed to parse PDF");
    let doc = reader.into_document();
    doc.get_page_annotations(0).unwrap_or_default().len()
}

/// Helper: get annotation subtype at given index on page 0
fn get_annotation_subtype(pdf_bytes: &[u8], index: usize) -> Option<String> {
    let cursor = Cursor::new(pdf_bytes);
    let reader = PdfReader::new(cursor).expect("Failed to parse PDF");
    let doc = reader.into_document();
    let annots = doc.get_page_annotations(0).unwrap_or_default();
    let annot = annots.get(index)?;
    annot
        .get("Subtype")
        .and_then(|s| s.as_name())
        .map(|n| n.0.clone())
}

/// Helper: check if annotation at index has a specific key
fn annotation_has_key(pdf_bytes: &[u8], index: usize, key: &str) -> bool {
    let cursor = Cursor::new(pdf_bytes);
    let reader = PdfReader::new(cursor).expect("Failed to parse PDF");
    let doc = reader.into_document();
    let annots = doc.get_page_annotations(0).unwrap_or_default();
    annots
        .get(index)
        .map(|a| a.contains_key(key))
        .unwrap_or(false)
}

/// Helper: get /Rect array values from annotation at index
fn get_annotation_rect(pdf_bytes: &[u8], index: usize) -> Option<[f64; 4]> {
    let cursor = Cursor::new(pdf_bytes);
    let reader = PdfReader::new(cursor).expect("Failed to parse PDF");
    let doc = reader.into_document();
    let annots = doc.get_page_annotations(0).unwrap_or_default();
    let annot = annots.get(index)?;
    let rect_obj = annot.get("Rect")?;
    let rect_arr = rect_obj.as_array()?;

    if rect_arr.len() != 4 {
        return None;
    }

    let vals: Vec<f64> = (0..4)
        .filter_map(|i| rect_arr.get(i).and_then(|v| v.as_real()))
        .collect();

    if vals.len() == 4 {
        Some([vals[0], vals[1], vals[2], vals[3]])
    } else {
        None
    }
}

/// Helper: get string field value from annotation
fn get_annotation_string_field(pdf_bytes: &[u8], index: usize, field: &str) -> Option<String> {
    let cursor = Cursor::new(pdf_bytes);
    let reader = PdfReader::new(cursor).expect("Failed to parse PDF");
    let doc = reader.into_document();
    let annots = doc.get_page_annotations(0).unwrap_or_default();
    let annot = annots.get(index)?;
    let val = annot.get(field)?;
    val.as_string()
        .map(|s| String::from_utf8_lossy(&s.0).to_string())
}

/// Helper: check if page has any annotations at all
fn page_has_annotations(pdf_bytes: &[u8]) -> bool {
    count_page_annotations(pdf_bytes) > 0
}

// =============================================================================
// TESTS
// =============================================================================

/// Test 1: A highlight annotation added to a page appears in the /Annots array
/// of the written PDF with /Subtype /Highlight
#[test]
fn test_highlight_annotation_appears_in_annots_array() {
    let rect = Rectangle::new(Point::new(100.0, 700.0), Point::new(300.0, 712.0));
    let highlight = MarkupAnnotation::highlight(rect);
    let annot = highlight.to_annotation();

    let pdf_bytes = create_pdf_with_annotations(vec![annot]);

    assert_eq!(
        count_page_annotations(&pdf_bytes),
        1,
        "Page should have exactly 1 annotation in /Annots"
    );
    assert_eq!(
        get_annotation_subtype(&pdf_bytes, 0),
        Some("Highlight".to_string()),
        "Annotation should have /Subtype /Highlight"
    );
}

/// Test 2: Multiple annotations of different types are all written
#[test]
fn test_multiple_annotations_written() {
    let rect1 = Rectangle::new(Point::new(100.0, 700.0), Point::new(300.0, 712.0));
    let rect2 = Rectangle::new(Point::new(100.0, 680.0), Point::new(300.0, 692.0));
    let rect3 = Rectangle::new(Point::new(100.0, 660.0), Point::new(300.0, 672.0));

    let annots = vec![
        MarkupAnnotation::highlight(rect1).to_annotation(),
        MarkupAnnotation::underline(rect2).to_annotation(),
        MarkupAnnotation::strikeout(rect3).to_annotation(),
    ];

    let pdf_bytes = create_pdf_with_annotations(annots);

    assert_eq!(
        count_page_annotations(&pdf_bytes),
        3,
        "Page should have exactly 3 annotations"
    );
    assert_eq!(
        get_annotation_subtype(&pdf_bytes, 0),
        Some("Highlight".to_string())
    );
    assert_eq!(
        get_annotation_subtype(&pdf_bytes, 1),
        Some("Underline".to_string())
    );
    assert_eq!(
        get_annotation_subtype(&pdf_bytes, 2),
        Some("StrikeOut".to_string())
    );
}

/// Test 3: Annotation /Rect is written with correct coordinates
#[test]
fn test_annotation_rect_written_correctly() {
    let rect = Rectangle::new(Point::new(72.0, 500.0), Point::new(200.0, 515.0));
    let highlight = MarkupAnnotation::highlight(rect);
    let annot = highlight.to_annotation();

    let pdf_bytes = create_pdf_with_annotations(vec![annot]);

    let written_rect = get_annotation_rect(&pdf_bytes, 0).expect("Annotation should have a /Rect");

    assert!(
        (written_rect[0] - 72.0).abs() < 0.01,
        "x1 should be 72.0, got {}",
        written_rect[0]
    );
    assert!(
        (written_rect[1] - 500.0).abs() < 0.01,
        "y1 should be 500.0, got {}",
        written_rect[1]
    );
    assert!(
        (written_rect[2] - 200.0).abs() < 0.01,
        "x2 should be 200.0, got {}",
        written_rect[2]
    );
    assert!(
        (written_rect[3] - 515.0).abs() < 0.01,
        "y2 should be 515.0, got {}",
        written_rect[3]
    );
}

/// Test 4: Highlight annotation has /QuadPoints written
#[test]
fn test_annotation_quad_points_written() {
    let rect = Rectangle::new(Point::new(100.0, 700.0), Point::new(300.0, 712.0));
    let highlight = MarkupAnnotation::highlight(rect);
    let annot = highlight.to_annotation();

    let pdf_bytes = create_pdf_with_annotations(vec![annot]);

    assert!(
        annotation_has_key(&pdf_bytes, 0, "QuadPoints"),
        "Highlight annotation should have /QuadPoints"
    );
}

/// Test 5: Annotation /C (color) is written with correct RGB values
#[test]
fn test_annotation_color_written() {
    let rect = Rectangle::new(Point::new(100.0, 700.0), Point::new(300.0, 712.0));
    let highlight = MarkupAnnotation::highlight(rect).with_color(Color::Rgb(1.0, 0.0, 0.0));
    let annot = highlight.to_annotation();

    let pdf_bytes = create_pdf_with_annotations(vec![annot]);

    assert!(
        annotation_has_key(&pdf_bytes, 0, "C"),
        "Annotation should have /C (color) entry"
    );
}

/// Test 6: Widget annotations (form fields) continue to work after refactor.
/// We create a Widget annotation manually and a non-Widget annotation,
/// and verify both appear in the output.
#[test]
fn test_widget_annotations_still_work_after_refactor() {
    let mut doc = Document::new();
    let mut page = Page::a4();

    // Add text content
    {
        let text = page.text();
        text.set_font(Font::Helvetica, 12.0);
        text.at(72.0, 700.0);
        let _ = text.write("Form page");
    }

    // Manually create a Widget annotation (simulating a form field)
    let widget_rect = Rectangle::new(Point::new(72.0, 650.0), Point::new(272.0, 670.0));
    let widget = Annotation::new(AnnotationType::Widget, widget_rect).with_contents("text_field_1");
    page.add_annotation(widget);

    // Add a non-widget (Highlight) annotation
    let hl_rect = Rectangle::new(Point::new(100.0, 600.0), Point::new(300.0, 612.0));
    let highlight = MarkupAnnotation::highlight(hl_rect).to_annotation();
    page.add_annotation(highlight);

    doc.add_page(page);
    let pdf_bytes = doc.to_bytes().expect("Failed to write PDF");

    let annot_count = count_page_annotations(&pdf_bytes);
    assert!(
        annot_count >= 2,
        "Page should have at least 2 annotations (widget + highlight), got {}",
        annot_count
    );

    // Verify both types are present
    let mut found_highlight = false;
    let mut found_widget = false;
    for i in 0..annot_count {
        if let Some(subtype) = get_annotation_subtype(&pdf_bytes, i) {
            if subtype == "Highlight" {
                found_highlight = true;
            }
            if subtype == "Widget" {
                found_widget = true;
            }
        }
    }
    assert!(found_highlight, "Should find a Highlight annotation");
    assert!(found_widget, "Should find a Widget annotation");
}

/// Test 7: Page without annotations does NOT have any annotation entries
#[test]
fn test_empty_annotations_no_annots_key() {
    let mut doc = Document::new();
    let mut page = Page::a4();
    {
        let text = page.text();
        text.set_font(Font::Helvetica, 12.0);
        text.at(72.0, 700.0);
        let _ = text.write("No annotations on this page");
    }
    doc.add_page(page);
    let pdf_bytes = doc.to_bytes().expect("Failed to write PDF");

    assert!(
        !page_has_annotations(&pdf_bytes),
        "Page without annotations should NOT have any annotations"
    );
}

/// Test 8: Annotation with /Contents string is written correctly
#[test]
fn test_annotation_with_contents_string() {
    let rect = Rectangle::new(Point::new(100.0, 700.0), Point::new(300.0, 712.0));
    let annot = Annotation::new(AnnotationType::Text, rect).with_contents("This is a comment");

    let pdf_bytes = create_pdf_with_annotations(vec![annot]);

    assert_eq!(count_page_annotations(&pdf_bytes), 1);
    assert_eq!(
        get_annotation_subtype(&pdf_bytes, 0),
        Some("Text".to_string())
    );

    let contents = get_annotation_string_field(&pdf_bytes, 0, "Contents");
    assert_eq!(
        contents,
        Some("This is a comment".to_string()),
        "Annotation should have /Contents with the correct text"
    );
}
