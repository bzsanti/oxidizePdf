use oxidize_pdf::layout::{DocumentBuilder, RichText, TextSpan};
use oxidize_pdf::writer::WriterConfig;
use oxidize_pdf::{Color, Font};

#[test]
fn test_text_span_measures_width() {
    let span = TextSpan::new("Hello", Font::Helvetica, 12.0, Color::black());
    let expected = oxidize_pdf::measure_text("Hello", &Font::Helvetica, 12.0);
    assert!(
        (span.measure_width() - expected).abs() < 0.001,
        "span width {:.3} should match measure_text {:.3}",
        span.measure_width(),
        expected
    );
}

#[test]
fn test_rich_text_total_width_is_sum() {
    let rich = RichText::new(vec![
        TextSpan::new("Hello ", Font::Helvetica, 12.0, Color::black()),
        TextSpan::new("World", Font::HelveticaBold, 12.0, Color::black()),
    ]);

    let expected = oxidize_pdf::measure_text("Hello ", &Font::Helvetica, 12.0)
        + oxidize_pdf::measure_text("World", &Font::HelveticaBold, 12.0);
    assert!(
        (rich.total_width() - expected).abs() < 0.001,
        "total width {:.3} should be sum of spans {:.3}",
        rich.total_width(),
        expected
    );
}

#[test]
fn test_rich_text_renders_mixed_fonts() {
    let rich = RichText::new(vec![
        TextSpan::new("Total: ", Font::HelveticaBold, 14.0, Color::black()),
        TextSpan::new("$1,234.56", Font::Helvetica, 14.0, Color::gray(0.3)),
    ]);

    let mut doc = DocumentBuilder::a4().add_rich_text(rich).build().unwrap();

    let config = WriterConfig {
        compress_streams: false,
        ..WriterConfig::default()
    };
    let bytes = doc.to_bytes_with_config(config).unwrap();
    let content = String::from_utf8_lossy(&bytes);

    assert!(
        content.contains("Total: "),
        "bold label must appear in stream"
    );
    assert!(content.contains("$1,234.56"), "value must appear in stream");
    // Both fonts must be referenced
    assert!(
        content.contains("/Helvetica-Bold 14.00 Tf"),
        "bold font must be set in stream"
    );
    assert!(
        content.contains("/Helvetica 14.00 Tf"),
        "regular font must be set in stream"
    );
}

#[test]
fn test_rich_text_in_document_builder() {
    let rich = RichText::new(vec![
        TextSpan::new(
            "RICH_MARKER_",
            Font::HelveticaBold,
            16.0,
            Color::rgb(1.0, 0.0, 0.0),
        ),
        TextSpan::new("FOUND", Font::Helvetica, 12.0, Color::black()),
    ]);

    let mut doc = DocumentBuilder::a4()
        .add_text("Before rich text", Font::Helvetica, 12.0)
        .add_spacer(10.0)
        .add_rich_text(rich)
        .add_spacer(10.0)
        .add_text("After rich text", Font::Helvetica, 12.0)
        .build()
        .unwrap();

    let config = WriterConfig {
        compress_streams: false,
        ..WriterConfig::default()
    };
    let bytes = doc.to_bytes_with_config(config).unwrap();
    let content = String::from_utf8_lossy(&bytes);

    assert!(bytes.starts_with(b"%PDF"));
    assert!(content.contains("RICH_MARKER_"), "rich text label present");
    assert!(content.contains("FOUND"), "rich text value present");
    assert!(
        content.contains("Before rich text"),
        "text before rich text present"
    );
    assert!(
        content.contains("After rich text"),
        "text after rich text present"
    );
}
