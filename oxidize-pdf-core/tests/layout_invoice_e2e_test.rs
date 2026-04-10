mod common;

use oxidize_pdf::layout::{DocumentBuilder, PageConfig, RichText, TextSpan};
use oxidize_pdf::text::Table;
use oxidize_pdf::writer::WriterConfig;
use oxidize_pdf::{Color, Font};

#[test]
fn test_invoice_e2e_multipage_with_all_features() {
    // Header with rich text
    let header = RichText::new(vec![
        TextSpan::new("FACTURA ", Font::HelveticaBold, 20.0, Color::black()),
        TextSpan::new(
            "#2026-001",
            Font::Helvetica,
            20.0,
            Color::rgb(0.3, 0.3, 0.3),
        ),
    ]);

    // Table with 50 rows to force pagination
    let mut table = Table::new(vec![60.0, 250.0, 80.0, 80.0]);
    for i in 0..50 {
        table
            .add_row(vec![
                format!("{}", i + 1),
                format!("Servicio de consultoria #{}", i + 1),
                "$150.00".into(),
                format!("${:.2}", (i + 1) as f64 * 150.0),
            ])
            .unwrap();
    }

    // Total with rich text
    let total_line = RichText::new(vec![
        TextSpan::new("TOTAL: ", Font::HelveticaBold, 14.0, Color::black()),
        TextSpan::new("$7,650.00", Font::Helvetica, 14.0, Color::black()),
    ]);

    let config = PageConfig::a4_with_margins(50.0, 50.0, 50.0, 50.0);
    let mut doc = DocumentBuilder::new(config)
        .add_rich_text(header)
        .add_spacer(20.0)
        .add_text("Cliente: ACME Corp", Font::Helvetica, 12.0)
        .add_text("Fecha: 2026-04-09", Font::Helvetica, 12.0)
        .add_spacer(15.0)
        .add_table(table)
        .add_spacer(10.0)
        .add_rich_text(total_line)
        .build()
        .unwrap();

    let config = WriterConfig {
        compress_streams: false,
        ..WriterConfig::default()
    };
    let bytes = doc.to_bytes_with_config(config).unwrap();

    assert!(bytes.starts_with(b"%PDF"), "must be valid PDF");

    let content = String::from_utf8_lossy(&bytes);

    // Content verification
    assert!(content.contains("FACTURA"), "header rich text present");
    assert!(content.contains("#2026-001"), "invoice number present");
    assert!(content.contains("ACME Corp"), "client name present");
    assert!(content.contains("2026-04-09"), "date present");
    assert!(content.contains("TOTAL"), "total label present");
    assert!(content.contains("$7,650.00"), "total amount present");

    // Multi-page verification
    let pages = common::count_pages(&bytes);
    assert!(
        pages >= 2,
        "50-row invoice must span >= 2 pages, got {}",
        pages
    );
}
