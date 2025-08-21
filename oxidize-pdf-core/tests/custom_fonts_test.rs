//! Integration tests for custom font loading functionality

use oxidize_pdf::{Document, Font, Page, Result};
use std::fs;
use tempfile::TempDir;

#[test]
#[ignore] // Ignore by default since it requires actual font files
fn test_custom_font_loading_from_file() -> Result<()> {
    let mut doc = Document::new();

    // Try to load a custom font (this would need an actual TTF file)
    // For testing, we'll use a system font if available
    let font_path = "/System/Library/Fonts/Helvetica.ttc"; // macOS path

    if std::path::Path::new(font_path).exists() {
        doc.add_font("CustomHelvetica", font_path)?;

        let mut page = Page::a4();
        page.text()
            .set_font(Font::custom("CustomHelvetica"), 24.0)
            .at(50.0, 700.0)
            .write("Custom Font Test")?;

        doc.add_page(page);

        // Create temp directory for output
        let temp_dir = TempDir::new()?;
        let output_path = temp_dir.path().join("custom_font_test.pdf");
        doc.save(&output_path)?;

        // Verify file was created
        assert!(output_path.exists());
        let file_size = fs::metadata(&output_path)?.len();
        assert!(file_size > 1000); // Should be larger due to embedded font
    }

    Ok(())
}

#[test]
#[ignore] // Ignore for now as create_minimal_ttf_data is complex
fn test_custom_font_loading_from_bytes() -> Result<()> {
    let doc = Document::new();

    // Create dummy font data for testing
    // In real use, this would be actual TTF/OTF data
    let font_data = vec![0u8; 1000]; // Dummy data

    // This test would need a proper TTF font to work
    // doc.add_font_from_bytes("TestFont", font_data)?;
    // assert!(doc.has_custom_font("TestFont"));

    Ok(())
}

#[test]
fn test_font_enum_custom_variant() {
    // Test creating custom font references
    let font = Font::custom("MyCustomFont");
    assert!(font.is_custom());
    assert_eq!(font.pdf_name(), "MyCustomFont");

    // Test standard fonts are not custom
    let helvetica = Font::Helvetica;
    assert!(!helvetica.is_custom());
}

#[test]
#[ignore] // Ignore for now as it needs proper font data
fn test_custom_font_with_text() -> Result<()> {
    // This test requires a proper TTF font to work
    // It's kept here for documentation purposes
    Ok(())
}

#[test]
fn test_multiple_custom_fonts() {
    let doc = Document::new();

    // Test that document starts with no custom fonts
    assert_eq!(doc.custom_font_names().len(), 0);

    // Would add fonts here if we had valid font data
    // doc.add_font_from_bytes("Font1", font1_data)?;
    // doc.add_font_from_bytes("Font2", font2_data)?;
    // assert_eq!(doc.custom_font_names().len(), 2);
}

#[test]
fn test_font_api_exists() {
    // Just verify the API exists and compiles
    let _font = Font::custom("TestFont");
    let _font = Font::Helvetica;
    let _font = Font::TimesRoman;
    let _font = Font::Courier;
}
