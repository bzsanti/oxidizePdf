//! Integration tests for PDF overlay/watermark operations

use oxidize_pdf::operations::{overlay_pdf, OverlayOptions, OverlayPosition, PageRange};
use oxidize_pdf::{Document, Font, Page};

/// Helper: creates a test PDF with the given number of pages and text
fn create_test_pdf(path: &std::path::Path, num_pages: usize, label: &str) {
    let mut doc = Document::new();
    for i in 0..num_pages {
        let mut page = Page::a4();
        page.text()
            .set_font(Font::Helvetica, 24.0)
            .at(100.0, 700.0)
            .write(&format!("{} - Page {}", label, i + 1))
            .unwrap();
        doc.add_page(page);
    }
    doc.save(path).unwrap();
}

/// Helper: counts pages in a PDF file
fn count_pages(path: &std::path::Path) -> usize {
    use oxidize_pdf::parser::{PdfDocument, PdfReader};
    let reader = PdfReader::open(path).unwrap();
    let doc = PdfDocument::new(reader);
    doc.page_count().unwrap() as usize
}

#[test]
fn test_overlay_pdf_creates_output_file() {
    let dir = tempfile::tempdir().unwrap();
    let base = dir.path().join("base.pdf");
    let overlay = dir.path().join("overlay.pdf");
    let output = dir.path().join("output.pdf");

    create_test_pdf(&base, 2, "Base");
    create_test_pdf(&overlay, 1, "Overlay");

    let result = overlay_pdf(&base, &overlay, &output, OverlayOptions::default());
    assert!(result.is_ok(), "overlay_pdf failed: {:?}", result.err());
    assert!(output.exists(), "Output file was not created");
}

#[test]
fn test_overlay_pdf_preserves_page_count() {
    let dir = tempfile::tempdir().unwrap();
    let base = dir.path().join("base.pdf");
    let overlay = dir.path().join("overlay.pdf");
    let output = dir.path().join("output.pdf");

    create_test_pdf(&base, 3, "Base");
    create_test_pdf(&overlay, 1, "Overlay");

    overlay_pdf(&base, &overlay, &output, OverlayOptions::default()).unwrap();

    assert_eq!(count_pages(&output), 3);
}

#[test]
fn test_overlay_with_opacity() {
    let dir = tempfile::tempdir().unwrap();
    let base = dir.path().join("base.pdf");
    let overlay = dir.path().join("overlay.pdf");
    let output = dir.path().join("output.pdf");

    create_test_pdf(&base, 1, "Base");
    create_test_pdf(&overlay, 1, "Overlay");

    let opts = OverlayOptions {
        opacity: 0.5,
        ..Default::default()
    };
    let result = overlay_pdf(&base, &overlay, &output, opts);
    assert!(
        result.is_ok(),
        "overlay with opacity failed: {:?}",
        result.err()
    );
    assert!(output.exists());
}

#[test]
fn test_overlay_with_position_bottom_right() {
    let dir = tempfile::tempdir().unwrap();
    let base = dir.path().join("base.pdf");
    let overlay = dir.path().join("overlay.pdf");
    let output = dir.path().join("output.pdf");

    create_test_pdf(&base, 1, "Base");
    create_test_pdf(&overlay, 1, "Overlay");

    let opts = OverlayOptions {
        position: OverlayPosition::BottomRight,
        ..Default::default()
    };
    let result = overlay_pdf(&base, &overlay, &output, opts);
    assert!(
        result.is_ok(),
        "overlay with position failed: {:?}",
        result.err()
    );
}

#[test]
fn test_overlay_with_scale() {
    let dir = tempfile::tempdir().unwrap();
    let base = dir.path().join("base.pdf");
    let overlay = dir.path().join("overlay.pdf");
    let output = dir.path().join("output.pdf");

    create_test_pdf(&base, 1, "Base");
    create_test_pdf(&overlay, 1, "Overlay");

    let opts = OverlayOptions {
        scale: 0.5,
        ..Default::default()
    };
    let result = overlay_pdf(&base, &overlay, &output, opts);
    assert!(
        result.is_ok(),
        "overlay with scale failed: {:?}",
        result.err()
    );
}

#[test]
fn test_overlay_repeat_single_page_on_multipage() {
    let dir = tempfile::tempdir().unwrap();
    let base = dir.path().join("base.pdf");
    let overlay = dir.path().join("overlay.pdf");
    let output = dir.path().join("output.pdf");

    create_test_pdf(&base, 3, "Base");
    create_test_pdf(&overlay, 1, "Overlay");

    let opts = OverlayOptions {
        repeat: true,
        ..Default::default()
    };
    overlay_pdf(&base, &overlay, &output, opts).unwrap();
    assert_eq!(count_pages(&output), 3);
}

#[test]
fn test_overlay_specific_pages_only() {
    let dir = tempfile::tempdir().unwrap();
    let base = dir.path().join("base.pdf");
    let overlay = dir.path().join("overlay.pdf");
    let output = dir.path().join("output.pdf");

    create_test_pdf(&base, 4, "Base");
    create_test_pdf(&overlay, 1, "Overlay");

    let opts = OverlayOptions {
        pages: PageRange::List(vec![0, 2]),
        repeat: true,
        ..Default::default()
    };
    let result = overlay_pdf(&base, &overlay, &output, opts);
    assert!(
        result.is_ok(),
        "overlay specific pages failed: {:?}",
        result.err()
    );
    assert_eq!(count_pages(&output), 4); // All pages preserved, overlay on 0 and 2 only
}

#[test]
fn test_overlay_empty_base_returns_error() {
    let dir = tempfile::tempdir().unwrap();
    let base = dir.path().join("base.pdf");
    let overlay = dir.path().join("overlay.pdf");
    let output = dir.path().join("output.pdf");

    // Create an empty PDF (no pages)
    let mut doc = Document::new();
    doc.save(&base).unwrap();

    create_test_pdf(&overlay, 1, "Overlay");

    let result = overlay_pdf(&base, &overlay, &output, OverlayOptions::default());
    assert!(result.is_err());
}

#[test]
fn test_overlay_invalid_opacity_clamped() {
    let dir = tempfile::tempdir().unwrap();
    let base = dir.path().join("base.pdf");
    let overlay = dir.path().join("overlay.pdf");
    let output = dir.path().join("output.pdf");

    create_test_pdf(&base, 1, "Base");
    create_test_pdf(&overlay, 1, "Overlay");

    let opts = OverlayOptions {
        opacity: 2.5,
        ..Default::default()
    };
    // Should NOT error — opacity is clamped to 1.0
    let result = overlay_pdf(&base, &overlay, &output, opts);
    assert!(
        result.is_ok(),
        "opacity clamping failed: {:?}",
        result.err()
    );
}

#[test]
fn test_overlay_zero_scale_returns_error() {
    let dir = tempfile::tempdir().unwrap();
    let base = dir.path().join("base.pdf");
    let overlay = dir.path().join("overlay.pdf");
    let output = dir.path().join("output.pdf");

    create_test_pdf(&base, 1, "Base");
    create_test_pdf(&overlay, 1, "Overlay");

    let opts = OverlayOptions {
        scale: 0.0,
        ..Default::default()
    };
    let result = overlay_pdf(&base, &overlay, &output, opts);
    assert!(result.is_err());
}

#[test]
fn test_overlay_page_range_out_of_bounds() {
    let dir = tempfile::tempdir().unwrap();
    let base = dir.path().join("base.pdf");
    let overlay = dir.path().join("overlay.pdf");
    let output = dir.path().join("output.pdf");

    create_test_pdf(&base, 2, "Base");
    create_test_pdf(&overlay, 1, "Overlay");

    let opts = OverlayOptions {
        pages: PageRange::List(vec![0, 5]),
        ..Default::default()
    };
    let result = overlay_pdf(&base, &overlay, &output, opts);
    assert!(result.is_err());
}

#[test]
fn test_overlay_with_custom_position() {
    let dir = tempfile::tempdir().unwrap();
    let base = dir.path().join("base.pdf");
    let overlay = dir.path().join("overlay.pdf");
    let output = dir.path().join("output.pdf");

    create_test_pdf(&base, 1, "Base");
    create_test_pdf(&overlay, 1, "Overlay");

    let opts = OverlayOptions {
        position: OverlayPosition::Custom(100.0, 200.0),
        ..Default::default()
    };
    let result = overlay_pdf(&base, &overlay, &output, opts);
    assert!(result.is_ok(), "custom position failed: {:?}", result.err());
}

#[test]
fn test_overlay_multipage_overlay_on_multipage_base() {
    let dir = tempfile::tempdir().unwrap();
    let base = dir.path().join("base.pdf");
    let overlay = dir.path().join("overlay.pdf");
    let output = dir.path().join("output.pdf");

    create_test_pdf(&base, 3, "Base");
    create_test_pdf(&overlay, 3, "Overlay");

    let result = overlay_pdf(&base, &overlay, &output, OverlayOptions::default());
    assert!(
        result.is_ok(),
        "multi-page overlay failed: {:?}",
        result.err()
    );
    assert_eq!(count_pages(&output), 3);
}

#[test]
fn test_overlay_all_positions() {
    let dir = tempfile::tempdir().unwrap();
    let base = dir.path().join("base.pdf");
    let overlay = dir.path().join("overlay.pdf");

    create_test_pdf(&base, 1, "Base");
    create_test_pdf(&overlay, 1, "Overlay");

    let positions = vec![
        OverlayPosition::Center,
        OverlayPosition::TopLeft,
        OverlayPosition::TopRight,
        OverlayPosition::BottomLeft,
        OverlayPosition::BottomRight,
        OverlayPosition::Custom(50.0, 50.0),
    ];

    for (i, pos) in positions.into_iter().enumerate() {
        let output = dir.path().join(format!("output_{i}.pdf"));
        let opts = OverlayOptions {
            position: pos,
            ..Default::default()
        };
        let result = overlay_pdf(&base, &overlay, &output, opts);
        assert!(result.is_ok(), "Position {i} failed: {:?}", result.err());
    }
}

#[test]
fn test_overlay_combined_options() {
    let dir = tempfile::tempdir().unwrap();
    let base = dir.path().join("base.pdf");
    let overlay = dir.path().join("overlay.pdf");
    let output = dir.path().join("output.pdf");

    create_test_pdf(&base, 3, "Base");
    create_test_pdf(&overlay, 1, "Watermark");

    let opts = OverlayOptions {
        pages: PageRange::All,
        position: OverlayPosition::Center,
        opacity: 0.3,
        scale: 0.8,
        repeat: true,
    };
    let result = overlay_pdf(&base, &overlay, &output, opts);
    assert!(
        result.is_ok(),
        "combined options failed: {:?}",
        result.err()
    );
    assert_eq!(count_pages(&output), 3);
}
