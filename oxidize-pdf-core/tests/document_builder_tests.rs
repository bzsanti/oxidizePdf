mod common;

use oxidize_pdf::layout::DocumentBuilder;
use oxidize_pdf::text::Table;
use oxidize_pdf::writer::WriterConfig;
use oxidize_pdf::Font;

#[test]
fn test_document_builder_produces_valid_pdf() {
    let mut doc = DocumentBuilder::a4()
        .add_text("Invoice #001", Font::HelveticaBold, 18.0)
        .add_spacer(10.0)
        .add_text("Date: 2026-04-09", Font::Helvetica, 12.0)
        .build()
        .unwrap();

    let bytes = doc.to_bytes().unwrap();
    assert!(bytes.starts_with(b"%PDF"));
    assert!(bytes.len() > 200);
    assert_eq!(common::count_pages(&bytes), 1);
}

#[test]
fn test_document_builder_text_appears() {
    let mut doc = DocumentBuilder::a4()
        .add_text("BUILDER_MARKER_XYZ789", Font::Helvetica, 12.0)
        .build()
        .unwrap();

    let config = WriterConfig {
        compress_streams: false,
        ..WriterConfig::default()
    };
    let bytes = doc.to_bytes_with_config(config).unwrap();
    let content = String::from_utf8_lossy(&bytes);
    assert!(
        content.contains("BUILDER_MARKER_XYZ789"),
        "marker text must appear in uncompressed PDF stream"
    );
}

#[test]
fn test_document_builder_multipage_with_table() {
    let mut table = Table::new(vec![150.0, 150.0, 150.0]);
    for i in 0..50 {
        table
            .add_row(vec![
                format!("{}", i + 1),
                format!("Item {}", i + 1),
                format!("${:.2}", (i + 1) as f64 * 9.99),
            ])
            .unwrap();
    }

    let mut doc = DocumentBuilder::a4()
        .add_text("INVOICE TITLE", Font::HelveticaBold, 20.0)
        .add_spacer(20.0)
        .add_table(table)
        .build()
        .unwrap();

    let config = WriterConfig {
        compress_streams: false,
        ..WriterConfig::default()
    };
    let bytes = doc.to_bytes_with_config(config).unwrap();
    let content = String::from_utf8_lossy(&bytes);

    assert!(
        content.contains("INVOICE TITLE"),
        "title must appear in PDF"
    );

    let pages = common::count_pages(&bytes);
    assert!(
        pages >= 2,
        "50-row table should span >= 2 pages, got {}",
        pages
    );
}
