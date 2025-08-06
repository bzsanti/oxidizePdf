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
fn test_custom_font_loading_from_bytes() -> Result<()> {
    let mut doc = Document::new();

    // Create dummy font data for testing
    // In real use, this would be actual TTF/OTF data
    let font_data = create_minimal_ttf_data();

    doc.add_font_from_bytes("TestFont", font_data)?;
    assert!(doc.has_custom_font("TestFont"));

    let font_names = doc.custom_font_names();
    assert!(font_names.contains(&"TestFont".to_string()));

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
fn test_custom_font_with_text() -> Result<()> {
    let mut doc = Document::new();

    // Add dummy font
    let font_data = create_minimal_ttf_data();
    doc.add_font_from_bytes("TestFont", font_data)?;

    // Create page with custom font
    let mut page = Page::a4();

    // Mix standard and custom fonts
    page.text()
        .set_font(Font::Helvetica, 12.0)
        .at(50.0, 750.0)
        .write("Standard Font: Helvetica")?;

    page.text()
        .set_font(Font::custom("TestFont"), 14.0)
        .at(50.0, 700.0)
        .write("Custom Font: TestFont")?;

    doc.add_page(page);

    // Save to memory
    let pdf_bytes = doc.to_bytes()?;
    assert!(!pdf_bytes.is_empty());

    Ok(())
}

#[test]
fn test_font_cache_operations() -> Result<()> {
    let mut doc = Document::new();

    // Test empty cache
    assert_eq!(doc.custom_font_names().len(), 0);
    assert!(!doc.has_custom_font("NonExistent"));

    // Add fonts
    doc.add_font_from_bytes("Font1", create_minimal_ttf_data())?;
    doc.add_font_from_bytes("Font2", create_minimal_ttf_data())?;

    // Test cache operations
    assert!(doc.has_custom_font("Font1"));
    assert!(doc.has_custom_font("Font2"));
    assert!(!doc.has_custom_font("Font3"));

    let names = doc.custom_font_names();
    assert_eq!(names.len(), 2);
    assert!(names.contains(&"Font1".to_string()));
    assert!(names.contains(&"Font2".to_string()));

    Ok(())
}

/// Create minimal valid TTF data for testing
/// This creates a basic TTF header with required tables
fn create_minimal_ttf_data() -> Vec<u8> {
    let mut data = Vec::new();

    // TTF header
    data.extend_from_slice(&[0x00, 0x01, 0x00, 0x00]); // version
    data.extend_from_slice(&[0x00, 0x05]); // numTables (5)
    data.extend_from_slice(&[0x00, 0x80]); // searchRange
    data.extend_from_slice(&[0x00, 0x02]); // entrySelector
    data.extend_from_slice(&[0x00, 0x30]); // rangeShift

    // Table directory entries (16 bytes each)
    // We need at least: head, hhea, name, cmap, hmtx

    // head table
    data.extend_from_slice(b"head");
    data.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]); // checksum
    data.extend_from_slice(&[0x00, 0x00, 0x00, 0x80]); // offset
    data.extend_from_slice(&[0x00, 0x00, 0x00, 0x36]); // length (54 bytes)

    // hhea table
    data.extend_from_slice(b"hhea");
    data.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]); // checksum
    data.extend_from_slice(&[0x00, 0x00, 0x00, 0xB6]); // offset
    data.extend_from_slice(&[0x00, 0x00, 0x00, 0x24]); // length (36 bytes)

    // name table
    data.extend_from_slice(b"name");
    data.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]); // checksum
    data.extend_from_slice(&[0x00, 0x00, 0x00, 0xDA]); // offset
    data.extend_from_slice(&[0x00, 0x00, 0x00, 0x06]); // length

    // cmap table
    data.extend_from_slice(b"cmap");
    data.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]); // checksum
    data.extend_from_slice(&[0x00, 0x00, 0x00, 0xE0]); // offset
    data.extend_from_slice(&[0x00, 0x00, 0x00, 0x04]); // length

    // hmtx table
    data.extend_from_slice(b"hmtx");
    data.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]); // checksum
    data.extend_from_slice(&[0x00, 0x00, 0x00, 0xE4]); // offset
    data.extend_from_slice(&[0x00, 0x00, 0x00, 0x04]); // length

    // Add minimal table data
    while data.len() < 0x80 {
        data.push(0);
    }

    // head table data (54 bytes)
    data.extend_from_slice(&[0x00, 0x01, 0x00, 0x00]); // version
    data.extend_from_slice(&[0x00, 0x01, 0x00, 0x00]); // fontRevision
    data.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]); // checkSumAdjustment
    data.extend_from_slice(&[0x5F, 0x0F, 0x3C, 0xF5]); // magicNumber
    data.extend_from_slice(&[0x00, 0x00]); // flags
    data.extend_from_slice(&[0x03, 0xE8]); // unitsPerEm (1000)
    data.extend_from_slice(&[0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]); // created
    data.extend_from_slice(&[0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]); // modified
    data.extend_from_slice(&[0x00, 0x00]); // xMin
    data.extend_from_slice(&[0xFF, 0x00]); // yMin
    data.extend_from_slice(&[0x04, 0x00]); // xMax
    data.extend_from_slice(&[0x04, 0x00]); // yMax
    data.extend_from_slice(&[0x00, 0x00]); // macStyle
    data.extend_from_slice(&[0x00, 0x08]); // lowestRecPPEM
    data.extend_from_slice(&[0x00, 0x02]); // fontDirectionHint

    // hhea table data (36 bytes)
    while data.len() < 0xB6 {
        data.push(0);
    }
    data.extend_from_slice(&[0x00, 0x01, 0x00, 0x00]); // version
    data.extend_from_slice(&[0x03, 0x20]); // ascent (800)
    data.extend_from_slice(&[0xFF, 0x38]); // descent (-200)
    data.extend_from_slice(&[0x00, 0xC8]); // lineGap (200)
    data.extend_from_slice(&[0x04, 0x00]); // advanceWidthMax
    data.extend_from_slice(&[0x00, 0x00]); // minLeftSideBearing
    data.extend_from_slice(&[0x00, 0x00]); // minRightSideBearing
    data.extend_from_slice(&[0x04, 0x00]); // xMaxExtent
    data.extend_from_slice(&[0x00, 0x01]); // caretSlopeRise
    data.extend_from_slice(&[0x00, 0x00]); // caretSlopeRun
    data.extend_from_slice(&[0x00, 0x00]); // caretOffset
    data.extend_from_slice(&[0x00, 0x00]); // reserved
    data.extend_from_slice(&[0x00, 0x00]); // reserved
    data.extend_from_slice(&[0x00, 0x00]); // reserved
    data.extend_from_slice(&[0x00, 0x00]); // reserved
    data.extend_from_slice(&[0x00, 0x00]); // metricDataFormat
    data.extend_from_slice(&[0x00, 0x01]); // numOfLongHorMetrics

    // Minimal name table
    while data.len() < 0xDA {
        data.push(0);
    }
    data.extend_from_slice(&[0x00, 0x00, 0x00, 0x00, 0x00, 0x00]);

    // Minimal cmap table
    while data.len() < 0xE0 {
        data.push(0);
    }
    data.extend_from_slice(&[0x00, 0x00, 0x00, 0x01]);

    // Minimal hmtx table
    while data.len() < 0xE4 {
        data.push(0);
    }
    data.extend_from_slice(&[0x02, 0x58, 0x00, 0x00]); // width 600, lsb 0

    // Ensure minimum size
    while data.len() < 1000 {
        data.push(0);
    }

    data
}
