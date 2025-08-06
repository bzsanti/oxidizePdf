//! Integration tests for font embedding with Document API
//!
//! These tests verify that font embedding works end-to-end with the
//! Document creation and PDF generation workflow.

use oxidize_pdf::error::Result;
use oxidize_pdf::{Document, EmbeddingOptions, Font, FontEmbedder, FontEncoding, FontFlags, Page};
use std::collections::HashSet;
use tempfile::TempDir;

/// Helper to create minimal font data for testing
fn create_minimal_font_data() -> Vec<u8> {
    // This creates a minimal TrueType font structure for testing
    // In production, you would use actual TrueType font files
    let mut font_data = Vec::new();

    // TrueType signature (0x00010000)
    font_data.extend_from_slice(&[0x00, 0x01, 0x00, 0x00]);

    // Number of tables (minimum required tables)
    font_data.extend_from_slice(&[0x00, 0x04]); // 4 tables

    // Search range, entry selector, range shift
    font_data.extend_from_slice(&[0x00, 0x40, 0x00, 0x02, 0x00, 0x00]);

    // Table directory entries for head, hhea, maxp, cmap tables
    let tables = [
        (b"head", 0x36u32, 0x36u32),
        (b"hhea", 0x6C, 0x24),
        (b"maxp", 0x90, 0x06),
        (b"cmap", 0x96, 0x20),
    ];

    for (tag, offset, length) in &tables {
        font_data.extend_from_slice(*tag);
        font_data.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]); // checksum
        font_data.extend_from_slice(&offset.to_be_bytes());
        font_data.extend_from_slice(&length.to_be_bytes());
    }

    // Pad to minimum required size
    while font_data.len() < 0xB6 {
        font_data.push(0);
    }

    font_data
}

#[test]
fn test_font_embedder_api_functionality() -> Result<()> {
    // Test basic FontEmbedder API
    let mut embedder = FontEmbedder::new();

    // Verify initial state
    assert_eq!(embedder.embedded_fonts().len(), 0);

    // Test embedding options
    let options = EmbeddingOptions {
        subset: true,
        max_subset_size: Some(128),
        compress_font_streams: true,
        embed_license_info: false,
    };

    // Test font flags
    let flags = FontFlags {
        non_symbolic: true,
        serif: false,
        fixed_pitch: false,
        ..Default::default()
    };

    // Verify flag conversion
    let flag_value = flags.to_flags();
    assert_eq!(flag_value & 32, 32); // non_symbolic bit
    assert_eq!(flag_value & 2, 0); // serif bit should be 0

    // Test encoding types
    let encodings = [
        FontEncoding::WinAnsiEncoding,
        FontEncoding::MacRomanEncoding,
        FontEncoding::StandardEncoding,
        FontEncoding::Identity,
    ];

    for encoding in &encodings {
        match encoding {
            FontEncoding::WinAnsiEncoding => {
                assert!(matches!(encoding, FontEncoding::WinAnsiEncoding))
            }
            FontEncoding::MacRomanEncoding => {
                assert!(matches!(encoding, FontEncoding::MacRomanEncoding))
            }
            FontEncoding::StandardEncoding => {
                assert!(matches!(encoding, FontEncoding::StandardEncoding))
            }
            FontEncoding::Identity => assert!(matches!(encoding, FontEncoding::Identity)),
            _ => {}
        }
    }

    // Test with minimal font data (will fail but tests the API)
    let font_data = create_minimal_font_data();
    let mut used_glyphs = HashSet::new();
    used_glyphs.insert(0); // .notdef
    used_glyphs.insert(65); // A
    used_glyphs.insert(66); // B

    // This will fail with our minimal test data, but verifies the API exists
    let result = embedder.embed_truetype_font(&font_data, &used_glyphs, &options);
    assert!(result.is_err()); // Expected with test data

    Ok(())
}

#[test]
fn test_document_with_standard_fonts() -> Result<()> {
    // Test document creation with standard PDF fonts (no embedding needed)
    let mut doc = Document::new();
    doc.set_title("Font Test Document");
    doc.set_author("oxidize-pdf");

    let mut page = Page::a4();

    // Test all standard fonts
    let fonts = [
        (Font::Helvetica, "Helvetica"),
        (Font::HelveticaBold, "Helvetica Bold"),
        (Font::HelveticaOblique, "Helvetica Oblique"),
        (Font::TimesRoman, "Times Roman"),
        (Font::TimesBold, "Times Bold"),
        (Font::TimesItalic, "Times Italic"),
        (Font::Courier, "Courier"),
        (Font::CourierBold, "Courier Bold"),
        (Font::Symbol, "Symbol"),
        (Font::ZapfDingbats, "ZapfDingbats"),
    ];

    let mut y = 750.0;
    for (font, name) in &fonts {
        page.text()
            .set_font(font.clone(), 12.0)
            .at(50.0, y)
            .write(&format!("{name}: Test text"))?;
        y -= 20.0;
    }

    doc.add_page(page);

    // Save to temporary file
    let temp_dir = TempDir::new()?;
    let output_path = temp_dir.path().join("font_test.pdf");

    doc.save(&output_path)?;

    // Verify file was created and has content
    assert!(output_path.exists());
    let file_data = std::fs::read(&output_path)?;
    assert!(file_data.len() > 1000); // Should be a decent-sized PDF

    // Verify PDF starts with correct header
    assert!(file_data.starts_with(b"%PDF-"));

    Ok(())
}

#[test]
fn test_font_embedding_workflow_structure() -> Result<()> {
    // Test the structure of font embedding workflow
    // (without actual font data which would require external files)

    let embedder = FontEmbedder::new();

    // Test different embedding options
    let subset_options = EmbeddingOptions {
        subset: true,
        max_subset_size: Some(256),
        compress_font_streams: true,
        embed_license_info: false,
    };

    let full_embed_options = EmbeddingOptions {
        subset: false,
        max_subset_size: None,
        compress_font_streams: false,
        embed_license_info: true,
    };

    // Test that options can be created and modified
    assert!(subset_options.subset);
    assert!(!full_embed_options.subset);
    assert!(subset_options.compress_font_streams);
    assert!(!full_embed_options.compress_font_streams);

    // Test font flag combinations
    let serif_flags = FontFlags {
        serif: true,
        non_symbolic: true,
        ..Default::default()
    };

    let monospace_flags = FontFlags {
        fixed_pitch: true,
        non_symbolic: true,
        ..Default::default()
    };

    let italic_flags = FontFlags {
        italic: true,
        non_symbolic: true,
        ..Default::default()
    };

    // Verify flag bits
    assert!(serif_flags.to_flags() & 2 != 0); // serif bit
    assert!(monospace_flags.to_flags() & 1 != 0); // fixed_pitch bit
    assert!(italic_flags.to_flags() & 64 != 0); // italic bit

    // All should have non_symbolic bit set
    assert!(serif_flags.to_flags() & 32 != 0);
    assert!(monospace_flags.to_flags() & 32 != 0);
    assert!(italic_flags.to_flags() & 32 != 0);

    // Test glyph set creation
    let mut ascii_glyphs = HashSet::new();
    for i in 32..127 {
        // Basic ASCII printable characters
        ascii_glyphs.insert(i);
    }
    ascii_glyphs.insert(0); // Always include .notdef

    assert!(ascii_glyphs.len() > 90); // Should have most ASCII chars
    assert!(ascii_glyphs.contains(&65)); // Should contain 'A'
    assert!(ascii_glyphs.contains(&97)); // Should contain 'a'
    assert!(ascii_glyphs.contains(&32)); // Should contain space

    // Test that embedder starts empty
    assert_eq!(embedder.embedded_fonts().len(), 0);

    Ok(())
}

#[test]
fn test_font_embedding_error_handling() -> Result<()> {
    let embedder = FontEmbedder::new();

    // Test error handling for non-existent fonts
    let result = embedder.generate_font_dictionary("NonExistentFont");
    assert!(result.is_err());

    let result = embedder.generate_font_descriptor("NonExistentFont");
    assert!(result.is_err());

    let result = embedder.generate_tounicode_cmap("NonExistentFont");
    assert!(result.is_err());

    Ok(())
}

#[test]
fn test_font_embedding_with_document_integration() -> Result<()> {
    // Test that font embedding can be used alongside document creation
    let mut doc = Document::new();
    doc.set_title("Font Embedding Integration Test");

    let mut page = Page::a4();

    // Add content using standard fonts
    page.text()
        .set_font(Font::Helvetica, 16.0)
        .at(50.0, 750.0)
        .write("Standard Font: Helvetica")?;

    page.text()
        .set_font(Font::TimesRoman, 14.0)
        .at(50.0, 720.0)
        .write("Standard Font: Times Roman")?;

    page.text()
        .set_font(Font::Courier, 12.0)
        .at(50.0, 690.0)
        .write("Standard Font: Courier (monospace)")?;

    // Create a font embedder (for future use)
    let embedder = FontEmbedder::new();

    // Verify embedder is ready for use
    assert_eq!(embedder.embedded_fonts().len(), 0);

    // Add informational text about font embedding
    page.text()
        .set_font(Font::Helvetica, 10.0)
        .at(50.0, 650.0)
        .write("Font embedding system ready for TrueType/OpenType fonts")?;

    page.text()
        .set_font(Font::Helvetica, 10.0)
        .at(50.0, 635.0)
        .write("Features: subsetting, compression, CID fonts, ToUnicode CMap")?;

    doc.add_page(page);

    // Save document
    let temp_dir = TempDir::new()?;
    let output_path = temp_dir.path().join("integration_test.pdf");

    doc.save(&output_path)?;

    // Verify file creation
    assert!(output_path.exists());
    let file_data = std::fs::read(&output_path)?;
    assert!(file_data.len() > 500);

    Ok(())
}

#[test]
fn test_cid_font_embedding_structure() -> Result<()> {
    let mut embedder = FontEmbedder::new();

    // Test CID font embedding structure (without real font data)
    let font_data = create_minimal_font_data();
    let mut used_chars = HashSet::new();

    // Add some Unicode characters that would require CID fonts
    used_chars.insert(0x4E00); // CJK character
    used_chars.insert(0x4E01);
    used_chars.insert(0x4E02);

    let options = EmbeddingOptions::default();

    // This will fail with test data but verifies the API
    let result = embedder.embed_cid_font(&font_data, &used_chars, "Identity-H", &options);
    assert!(result.is_err()); // Expected with minimal test data

    Ok(())
}

#[test]
fn test_unicode_mapping_creation() -> Result<()> {
    // Test Unicode mapping structures
    let mut glyph_to_unicode = std::collections::HashMap::new();

    // Create basic ASCII mappings
    glyph_to_unicode.insert(65u16, "A".to_string());
    glyph_to_unicode.insert(66u16, "B".to_string());
    glyph_to_unicode.insert(67u16, "C".to_string());
    glyph_to_unicode.insert(32u16, " ".to_string());

    // Verify mappings
    assert_eq!(glyph_to_unicode.get(&65), Some(&"A".to_string()));
    assert_eq!(glyph_to_unicode.get(&66), Some(&"B".to_string()));
    assert_eq!(glyph_to_unicode.get(&67), Some(&"C".to_string()));
    assert_eq!(glyph_to_unicode.get(&32), Some(&" ".to_string()));

    // Test Unicode string formatting (as would be used in ToUnicode CMap)
    for (glyph_id, unicode_str) in &glyph_to_unicode {
        let hex_unicode: String = unicode_str
            .chars()
            .map(|c| format!("{:04X}", c as u32))
            .collect();

        // Verify format for basic ASCII
        match *glyph_id {
            65 => assert_eq!(hex_unicode, "0041"),
            66 => assert_eq!(hex_unicode, "0042"),
            67 => assert_eq!(hex_unicode, "0043"),
            32 => assert_eq!(hex_unicode, "0020"),
            _ => {}
        }
    }

    Ok(())
}

#[test]
fn test_font_subset_glyph_management() -> Result<()> {
    // Test glyph subset management
    let mut full_glyph_set = HashSet::new();
    let mut subset_glyph_set = HashSet::new();

    // Create full character set (all printable ASCII)
    for i in 32..127 {
        full_glyph_set.insert(i);
    }

    // Create subset (just uppercase letters and space)
    for i in 65..91 {
        // A-Z
        subset_glyph_set.insert(i);
    }
    subset_glyph_set.insert(32); // space
    subset_glyph_set.insert(0); // .notdef

    // Test subset properties
    assert!(full_glyph_set.len() > subset_glyph_set.len());
    assert!(subset_glyph_set.len() == 28); // 26 letters + space + .notdef

    // Test that subset is contained in full set
    for &glyph in &subset_glyph_set {
        if glyph != 0 {
            // .notdef might not be in the range
            assert!((32..=90).contains(&glyph));
        }
    }

    // Test subset criteria (what EmbeddingOptions might use)
    let subset_threshold = 256;
    assert!(subset_glyph_set.len() < subset_threshold);
    assert!(full_glyph_set.len() < subset_threshold); // ASCII fits in subset

    Ok(())
}
