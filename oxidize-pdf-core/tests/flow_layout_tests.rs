mod common;

use oxidize_pdf::layout::{FlowLayout, PageConfig};
use oxidize_pdf::text::Table;
use oxidize_pdf::writer::WriterConfig;
use oxidize_pdf::{Document, Font};

/// Generate PDF bytes with compression disabled for content inspection.
fn to_uncompressed_bytes(doc: &mut Document) -> Vec<u8> {
    let config = WriterConfig {
        compress_streams: false,
        ..WriterConfig::default()
    };
    doc.to_bytes_with_config(config).unwrap()
}

#[test]
fn test_flow_layout_single_page() {
    let config = PageConfig::a4_with_margins(50.0, 50.0, 50.0, 50.0);
    let mut layout = FlowLayout::new(config);
    layout.add_text("Hello World", Font::Helvetica, 12.0);

    let mut doc = Document::new();
    layout.build_into(&mut doc).unwrap();

    let bytes = doc.to_bytes().unwrap();
    assert!(bytes.starts_with(b"%PDF"));
    assert_eq!(
        common::count_pages(&bytes),
        1,
        "short text should produce exactly 1 page"
    );
}

#[test]
fn test_flow_layout_auto_page_break() {
    // Small page: 200×200 pts, 20pt margins each side
    // Usable height = 160pts; line at 14pt font × 1.2 = 16.8pts per line
    // 20 lines = 336pts > 160pts → must create at least 2 pages
    let config = PageConfig::new(200.0, 200.0, 20.0, 20.0, 20.0, 20.0);
    let mut layout = FlowLayout::new(config);
    for _ in 0..20 {
        layout.add_text("This line takes space on the page.", Font::Helvetica, 14.0);
    }

    let mut doc = Document::new();
    layout.build_into(&mut doc).unwrap();

    let bytes = doc.to_bytes().unwrap();
    let pages = common::count_pages(&bytes);
    assert!(
        pages >= 2,
        "20 lines of 14pt text in a 200×200 page should produce >= 2 pages, got {}",
        pages
    );
}

#[test]
fn test_flow_layout_spacer() {
    let config = PageConfig::a4_with_margins(50.0, 50.0, 50.0, 50.0);
    let mut layout = FlowLayout::new(config);
    layout.add_text("First paragraph", Font::Helvetica, 12.0);
    layout.add_spacer(30.0);
    layout.add_text("Second paragraph", Font::Helvetica, 12.0);

    let mut doc = Document::new();
    layout.build_into(&mut doc).unwrap();

    let bytes = to_uncompressed_bytes(&mut doc);
    assert!(bytes.starts_with(b"%PDF"));
    let content = String::from_utf8_lossy(&bytes);
    assert!(
        content.contains("First paragraph"),
        "first text must appear in PDF stream"
    );
    assert!(
        content.contains("Second paragraph"),
        "second text must appear in PDF stream"
    );
}

#[test]
fn test_flow_layout_table_page_break() {
    // Small page where text fills most of the space, then a table forces a break
    let config = PageConfig::new(300.0, 200.0, 20.0, 20.0, 20.0, 20.0);
    let mut layout = FlowLayout::new(config);

    // Fill with text lines (8 lines × 16.8pts ≈ 134pts of 160 usable)
    for _ in 0..8 {
        layout.add_text("Line of text here.", Font::Helvetica, 12.0);
    }

    // Table of 3 rows — won't fit in remaining ~26pts
    let mut table = Table::new(vec![80.0, 80.0, 80.0]);
    table
        .add_row(vec!["A".to_string(), "B".to_string(), "C".to_string()])
        .unwrap();
    table
        .add_row(vec!["1".to_string(), "2".to_string(), "3".to_string()])
        .unwrap();
    table
        .add_row(vec!["X".to_string(), "Y".to_string(), "Z".to_string()])
        .unwrap();
    layout.add_table(table);

    let mut doc = Document::new();
    layout.build_into(&mut doc).unwrap();

    let bytes = doc.to_bytes().unwrap();
    let pages = common::count_pages(&bytes);
    assert!(
        pages >= 2,
        "table after near-full page should force page break, got {} pages",
        pages
    );
}

#[test]
fn test_flow_layout_text_content_appears() {
    let config = PageConfig::a4_with_margins(50.0, 50.0, 50.0, 50.0);
    let mut layout = FlowLayout::new(config);
    layout.add_text("UNIQUE_MARKER_ABC123", Font::Helvetica, 12.0);

    let mut doc = Document::new();
    layout.build_into(&mut doc).unwrap();

    let bytes = to_uncompressed_bytes(&mut doc);
    let content = String::from_utf8_lossy(&bytes);
    assert!(
        content.contains("UNIQUE_MARKER_ABC123"),
        "marker text must appear in PDF stream"
    );
}
