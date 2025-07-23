//! Tests for configurable font encoding support

use oxidize_pdf::text::{Font, FontEncoding};
use oxidize_pdf::{Document, Page};
use tempfile::TempDir;

#[test]
fn test_document_default_font_encoding() {
    let mut doc = Document::new();

    // Initially no default encoding
    assert_eq!(doc.default_font_encoding(), None);

    // Set default encoding
    doc.set_default_font_encoding(Some(FontEncoding::WinAnsiEncoding));
    assert_eq!(
        doc.default_font_encoding(),
        Some(FontEncoding::WinAnsiEncoding)
    );

    // Change to different encoding
    doc.set_default_font_encoding(Some(FontEncoding::MacRomanEncoding));
    assert_eq!(
        doc.default_font_encoding(),
        Some(FontEncoding::MacRomanEncoding)
    );

    // Clear encoding
    doc.set_default_font_encoding(None);
    assert_eq!(doc.default_font_encoding(), None);
}

#[test]
fn test_document_with_encoding_saves_successfully() {
    let temp_dir = TempDir::new().unwrap();
    let output_path = temp_dir.path().join("test_encoding.pdf");

    let mut doc = Document::new();
    doc.set_default_font_encoding(Some(FontEncoding::WinAnsiEncoding));

    let mut page = Page::a4();
    page.text()
        .set_font(Font::Helvetica, 12.0)
        .at(100.0, 700.0)
        .write("Hello with encoding!")
        .unwrap();

    doc.add_page(page);

    // Should save without error
    doc.save(&output_path).unwrap();

    // File should exist and have content
    assert!(output_path.exists());
    let metadata = std::fs::metadata(&output_path).unwrap();
    assert!(metadata.len() > 0);
}

#[test]
fn test_document_without_encoding_saves_successfully() {
    let temp_dir = TempDir::new().unwrap();
    let output_path = temp_dir.path().join("test_no_encoding.pdf");

    let mut doc = Document::new();
    // Don't set any default encoding

    let mut page = Page::a4();
    page.text()
        .set_font(Font::TimesRoman, 14.0)
        .at(100.0, 700.0)
        .write("Hello without encoding!")
        .unwrap();

    doc.add_page(page);

    // Should save without error
    doc.save(&output_path).unwrap();

    // File should exist and have content
    assert!(output_path.exists());
    let metadata = std::fs::metadata(&output_path).unwrap();
    assert!(metadata.len() > 0);
}

#[test]
fn test_all_font_encodings() {
    let mut doc = Document::new();

    // Test all encoding types
    let encodings = [
        FontEncoding::WinAnsiEncoding,
        FontEncoding::MacRomanEncoding,
        FontEncoding::StandardEncoding,
        FontEncoding::MacExpertEncoding,
        FontEncoding::Custom("MyEncoding"),
    ];

    for encoding in &encodings {
        doc.set_default_font_encoding(Some(*encoding));
        assert_eq!(doc.default_font_encoding(), Some(*encoding));
    }
}

#[test]
fn test_font_encoding_pdf_names() {
    assert_eq!(FontEncoding::WinAnsiEncoding.pdf_name(), "WinAnsiEncoding");
    assert_eq!(
        FontEncoding::MacRomanEncoding.pdf_name(),
        "MacRomanEncoding"
    );
    assert_eq!(
        FontEncoding::StandardEncoding.pdf_name(),
        "StandardEncoding"
    );
    assert_eq!(
        FontEncoding::MacExpertEncoding.pdf_name(),
        "MacExpertEncoding"
    );
    assert_eq!(
        FontEncoding::Custom("TestEncoding").pdf_name(),
        "TestEncoding"
    );
}

#[test]
fn test_font_with_recommended_encoding() {
    // Test fonts have recommended encoding
    for font in [Font::Helvetica, Font::TimesRoman, Font::Courier] {
        let font_with_enc = font.with_recommended_encoding();
        assert_eq!(font_with_enc.font, font);
        assert_eq!(font_with_enc.encoding, Some(FontEncoding::WinAnsiEncoding));
    }

    // Symbol fonts should not have recommended encoding
    for font in [Font::Symbol, Font::ZapfDingbats] {
        let font_with_enc = font.with_recommended_encoding();
        assert_eq!(font_with_enc.font, font);
        assert_eq!(font_with_enc.encoding, None);
    }
}

#[test]
fn test_font_with_custom_encoding() {
    let font = Font::Helvetica;
    let custom_encoding = FontEncoding::Custom("MyCustomEncoding");

    let font_with_enc = font.with_encoding(custom_encoding);
    assert_eq!(font_with_enc.font, font);
    assert_eq!(font_with_enc.encoding, Some(custom_encoding));
}

#[test]
fn test_font_without_encoding() {
    let font = Font::TimesRoman;

    let font_with_enc = font.without_encoding();
    assert_eq!(font_with_enc.font, font);
    assert_eq!(font_with_enc.encoding, None);
}

#[test]
fn test_document_with_multiple_encoding_types() {
    let temp_dir = TempDir::new().unwrap();
    let output_path = temp_dir.path().join("test_multiple_encodings.pdf");

    let mut doc = Document::new();
    doc.set_default_font_encoding(Some(FontEncoding::StandardEncoding));

    // Create multiple pages with different fonts
    for (i, font) in [Font::Helvetica, Font::TimesRoman, Font::Courier]
        .iter()
        .enumerate()
    {
        let mut page = Page::a4();
        page.text()
            .set_font(*font, 12.0)
            .at(100.0, 700.0)
            .write(&format!("Page {} with {}", i + 1, font.pdf_name()))
            .unwrap();
        doc.add_page(page);
    }

    // Should save without error
    doc.save(&output_path).unwrap();

    // File should exist and have content
    assert!(output_path.exists());
    let metadata = std::fs::metadata(&output_path).unwrap();
    assert!(metadata.len() > 0);
}
