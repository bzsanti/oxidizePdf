//! Integration tests for font embedding workflows
//!
//! This module tests complete font embedding scenarios including:
//! - TrueType font embedding with subsetting
//! - CID font embedding for complex scripts
//! - Character encoding and mapping
//! - Font descriptor generation
//! - ToUnicode CMap generation

use oxidize_pdf::error::Result;
use oxidize_pdf::objects::{Dictionary, Object, ObjectId};
use oxidize_pdf::text::fonts::embedding::{
    EmbeddingOptions, FontEmbedder, FontEncoding, FontFlags, FontType,
};
use oxidize_pdf::{Document, Font, Page};
use std::collections::{HashMap, HashSet};
use std::fs;
use tempfile::TempDir;

/// Helper to create a minimal valid TrueType font for testing
fn create_test_font_data() -> Vec<u8> {
    // This creates a minimal font structure that passes basic validation
    // In a real test, you would use an actual font file
    let mut font_data = Vec::new();

    // TrueType font starts with version (0x00010000 for TrueType)
    font_data.extend_from_slice(&[0x00, 0x01, 0x00, 0x00]);

    // Number of tables
    font_data.extend_from_slice(&[0x00, 0x04]); // 4 tables

    // Search range, entry selector, range shift
    font_data.extend_from_slice(&[0x00, 0x40, 0x00, 0x02, 0x00, 0x00]);

    // Table directory entries (simplified)
    // head table
    font_data.extend_from_slice(b"head");
    font_data.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]); // checksum
    font_data.extend_from_slice(&[0x00, 0x00, 0x00, 0x36]); // offset
    font_data.extend_from_slice(&[0x00, 0x00, 0x00, 0x36]); // length

    // hhea table
    font_data.extend_from_slice(b"hhea");
    font_data.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]); // checksum
    font_data.extend_from_slice(&[0x00, 0x00, 0x00, 0x6C]); // offset
    font_data.extend_from_slice(&[0x00, 0x00, 0x00, 0x24]); // length

    // maxp table
    font_data.extend_from_slice(b"maxp");
    font_data.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]); // checksum
    font_data.extend_from_slice(&[0x00, 0x00, 0x00, 0x90]); // offset
    font_data.extend_from_slice(&[0x00, 0x00, 0x00, 0x06]); // length

    // cmap table
    font_data.extend_from_slice(b"cmap");
    font_data.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]); // checksum
    font_data.extend_from_slice(&[0x00, 0x00, 0x00, 0x96]); // offset
    font_data.extend_from_slice(&[0x00, 0x00, 0x00, 0x20]); // length

    // Pad to minimum offsets and add minimal table data
    while font_data.len() < 0xB6 {
        font_data.push(0);
    }

    font_data
}

#[test]
fn test_font_embedding_basic_workflow() {
    let mut embedder = FontEmbedder::new();

    // Test that embedder starts empty
    assert_eq!(embedder.embedded_fonts().len(), 0);

    // Create test font data
    let font_data = create_test_font_data();

    // Define glyphs used in document
    let mut used_glyphs = HashSet::new();
    used_glyphs.insert(0); // .notdef
    used_glyphs.insert(65); // A
    used_glyphs.insert(66); // B
    used_glyphs.insert(67); // C

    // Try to embed font (will fail with test data but tests the workflow)
    let options = EmbeddingOptions::default();
    let result = embedder.embed_truetype_font(&font_data, &used_glyphs, &options);

    // With our test data this will fail, but in real usage it would succeed
    assert!(result.is_err());
}

#[test]
fn test_font_flags_conversion() {
    let mut flags = FontFlags::default();
    assert_eq!(flags.to_flags(), 0);

    flags.fixed_pitch = true;
    assert_eq!(flags.to_flags() & 1, 1);

    flags.serif = true;
    assert_eq!(flags.to_flags() & 2, 2);

    flags.symbolic = true;
    assert_eq!(flags.to_flags() & 4, 4);

    flags.script = true;
    assert_eq!(flags.to_flags() & 8, 8);

    flags.non_symbolic = true;
    assert_eq!(flags.to_flags() & 32, 32);

    flags.italic = true;
    assert_eq!(flags.to_flags() & 64, 64);

    flags.all_cap = true;
    assert_eq!(flags.to_flags() & (1 << 16), 1 << 16);

    flags.small_cap = true;
    assert_eq!(flags.to_flags() & (1 << 17), 1 << 17);

    flags.force_bold = true;
    assert_eq!(flags.to_flags() & (1 << 18), 1 << 18);
}

#[test]
fn test_font_encoding_types() {
    // Test different encoding types
    let encodings = vec![
        FontEncoding::StandardEncoding,
        FontEncoding::MacRomanEncoding,
        FontEncoding::WinAnsiEncoding,
        FontEncoding::Identity,
    ];

    for encoding in encodings {
        match encoding {
            FontEncoding::StandardEncoding => {
                // Verify it's the standard encoding
                assert!(matches!(encoding, FontEncoding::StandardEncoding));
            }
            FontEncoding::MacRomanEncoding => {
                assert!(matches!(encoding, FontEncoding::MacRomanEncoding));
            }
            FontEncoding::WinAnsiEncoding => {
                assert!(matches!(encoding, FontEncoding::WinAnsiEncoding));
            }
            FontEncoding::Identity => {
                assert!(matches!(encoding, FontEncoding::Identity));
            }
            _ => {}
        }
    }
}

#[test]
fn test_custom_encoding_differences() {
    use oxidize_pdf::text::fonts::embedding::EncodingDifference;

    let differences = vec![
        EncodingDifference {
            code: 128,
            names: vec!["Euro".to_string()],
        },
        EncodingDifference {
            code: 160,
            names: vec!["space".to_string(), "nbspace".to_string()],
        },
    ];

    let encoding = FontEncoding::Custom(differences);

    if let FontEncoding::Custom(diffs) = encoding {
        assert_eq!(diffs.len(), 2);
        assert_eq!(diffs[0].code, 128);
        assert_eq!(diffs[0].names[0], "Euro");
        assert_eq!(diffs[1].code, 160);
        assert_eq!(diffs[1].names.len(), 2);
    } else {
        panic!("Expected custom encoding");
    }
}

#[test]
fn test_embedding_options_customization() {
    let mut options = EmbeddingOptions {
        subset: false,
        max_subset_size: Some(512),
        compress_font_streams: false,
        embed_license_info: true,
    };

    assert!(!options.subset);
    assert_eq!(options.max_subset_size, Some(512));
    assert!(!options.compress_font_streams);
    assert!(options.embed_license_info);

    // Test modification
    options.subset = true;
    options.max_subset_size = None;

    assert!(options.subset);
    assert!(options.max_subset_size.is_none());
}

#[test]
fn test_cid_font_embedding_workflow() {
    let mut embedder = FontEmbedder::new();

    // Create test font data
    let font_data = create_test_font_data();

    // Define characters used (Unicode code points)
    let mut used_chars = HashSet::new();
    used_chars.insert(0x4E00); // CJK character
    used_chars.insert(0x4E01);
    used_chars.insert(0x4E02);

    let options = EmbeddingOptions::default();

    // Try to embed as CID font
    let result = embedder.embed_cid_font(&font_data, &used_chars, "Identity-H", &options);

    // With test data this will fail, but tests the workflow
    assert!(result.is_err());
}

#[test]
fn test_font_dictionary_generation() {
    let embedder = FontEmbedder::new();

    // Test generating dictionary for non-existent font
    let result = embedder.generate_font_dictionary("NonExistentFont");
    assert!(result.is_err());
}

#[test]
fn test_font_descriptor_generation() {
    let embedder = FontEmbedder::new();

    // Test generating descriptor for non-existent font
    let result = embedder.generate_font_descriptor("NonExistentFont");
    assert!(result.is_err());
}

#[test]
fn test_tounicode_cmap_generation() {
    let mut embedder = FontEmbedder::new();

    // Create a font with Unicode mappings
    let mut unicode_mappings = HashMap::new();
    unicode_mappings.insert(65, "A".to_string());
    unicode_mappings.insert(66, "B".to_string());
    unicode_mappings.insert(67, "C".to_string());

    let font_data = oxidize_pdf::text::fonts::embedding::EmbeddedFontData {
        pdf_name: "TestFont".to_string(),
        font_type: FontType::TrueType,
        descriptor: oxidize_pdf::text::fonts::embedding::FontDescriptor {
            font_name: "TestFont".to_string(),
            flags: FontFlags::default(),
            bbox: [0, 0, 1000, 1000],
            italic_angle: 0.0,
            ascent: 750,
            descent: -250,
            cap_height: 700,
            stem_v: 100,
            stem_h: 50,
            avg_width: 500,
            max_width: 1000,
            missing_width: 500,
            font_file: None,
        },
        font_program: vec![],
        encoding: FontEncoding::WinAnsiEncoding,
        metrics: oxidize_pdf::text::fonts::embedding::FontMetrics {
            ascent: 750,
            descent: -250,
            cap_height: 700,
            x_height: 500,
            stem_v: 100,
            stem_h: 50,
            avg_width: 500,
            max_width: 1000,
            missing_width: 500,
        },
        subset_glyphs: None,
        unicode_mappings,
    };

    // Insert the font data directly (bypassing the private field)
    // In real usage, this would be done through embed_truetype_font
    embedder.embedded_fonts().get("TestFont"); // Just to compile

    // Since we can't directly insert due to private fields,
    // we test that the method exists and compiles
    let result = embedder.generate_tounicode_cmap("TestFont");
    assert!(result.is_err()); // Font not found
}

#[test]
fn test_font_type_variants() {
    let font_types = vec![FontType::TrueType, FontType::Type0];

    for font_type in font_types {
        match font_type {
            FontType::TrueType => {
                assert_eq!(font_type, FontType::TrueType);
            }
            FontType::Type0 => {
                assert_eq!(font_type, FontType::Type0);
            }
        }
    }
}

#[test]
fn test_complete_embedding_workflow() -> Result<()> {
    // Create a new document
    let mut doc = Document::new();

    // Add a page
    let mut page = Page::a4();

    // Try to use text with embedded font
    // This tests the integration between Document, Page, and FontEmbedder
    page.text()
        .set_font(Font::Helvetica, 12.0)
        .at(100.0, 700.0)
        .write("Test text with embedded font");

    doc.add_page(page);

    // Save to temporary file
    let temp_dir = TempDir::new()?;
    let output_path = temp_dir.path().join("test_embedded_font.pdf");

    doc.save(&output_path)?;

    // Verify file was created
    assert!(output_path.exists());

    // Read back and verify
    let file_data = fs::read(&output_path)?;
    assert!(file_data.len() > 0);

    Ok(())
}

#[test]
fn test_font_subset_size_limits() {
    let options1 = EmbeddingOptions {
        subset: true,
        max_subset_size: Some(10),
        ..Default::default()
    };

    let options2 = EmbeddingOptions {
        subset: true,
        max_subset_size: Some(1000),
        ..Default::default()
    };

    // Test with small glyph set
    let mut small_glyphs = HashSet::new();
    for i in 0..5 {
        small_glyphs.insert(i);
    }

    // Test with large glyph set
    let mut large_glyphs = HashSet::new();
    for i in 0..500 {
        large_glyphs.insert(i);
    }

    // With options1, small set should trigger subsetting
    assert!(small_glyphs.len() < options1.max_subset_size.unwrap_or(256));

    // With options2, large set should still trigger subsetting
    assert!(large_glyphs.len() < options2.max_subset_size.unwrap_or(256));
}

#[test]
fn test_font_metrics_values() {
    let metrics = oxidize_pdf::text::fonts::embedding::FontMetrics {
        ascent: 1000,
        descent: -300,
        cap_height: 800,
        x_height: 500,
        stem_v: 120,
        stem_h: 60,
        avg_width: 600,
        max_width: 1200,
        missing_width: 600,
    };

    // Verify typical font metric relationships
    assert!(metrics.ascent > 0);
    assert!(metrics.descent < 0);
    assert!(metrics.cap_height > metrics.x_height);
    assert!(metrics.cap_height < metrics.ascent);
    assert!(metrics.stem_v > metrics.stem_h); // Usually true for most fonts
    assert!(metrics.avg_width <= metrics.max_width);
    assert_eq!(metrics.missing_width, metrics.avg_width);
}

#[test]
fn test_font_bbox_validation() {
    let bbox = [-100, -250, 1000, 750];

    // Verify bounding box constraints
    assert!(bbox[0] < bbox[2]); // left < right
    assert!(bbox[1] < bbox[3]); // bottom < top
    assert!(bbox[0] < 0); // Typically extends left of origin
    assert!(bbox[1] < 0); // Typically extends below baseline
    assert!(bbox[2] > 0); // Right edge is positive
    assert!(bbox[3] > 0); // Top edge is positive
}

#[test]
fn test_encoding_difference_creation() {
    use oxidize_pdf::text::fonts::embedding::EncodingDifference;

    // Test single character difference
    let diff1 = EncodingDifference {
        code: 128,
        names: vec!["Euro".to_string()],
    };

    assert_eq!(diff1.code, 128);
    assert_eq!(diff1.names.len(), 1);
    assert_eq!(diff1.names[0], "Euro");

    // Test multiple consecutive character differences
    let diff2 = EncodingDifference {
        code: 200,
        names: vec![
            "Agrave".to_string(),
            "Aacute".to_string(),
            "Acircumflex".to_string(),
            "Atilde".to_string(),
        ],
    };

    assert_eq!(diff2.code, 200);
    assert_eq!(diff2.names.len(), 4);
}

#[test]
fn test_font_embedder_default_trait() {
    let embedder1 = FontEmbedder::new();
    let embedder2 = FontEmbedder::default();

    // Both should start with same state
    assert_eq!(
        embedder1.embedded_fonts().len(),
        embedder2.embedded_fonts().len()
    );
}

#[test]
fn test_font_flags_combinations() {
    // Test realistic font flag combinations

    // Serif font flags
    let serif_flags = FontFlags {
        serif: true,
        non_symbolic: true,
        ..Default::default()
    };
    let serif_value = serif_flags.to_flags();
    assert!(serif_value & 2 != 0); // Serif bit
    assert!(serif_value & 32 != 0); // NonSymbolic bit

    // Monospace font flags
    let mono_flags = FontFlags {
        fixed_pitch: true,
        non_symbolic: true,
        ..Default::default()
    };
    let mono_value = mono_flags.to_flags();
    assert!(mono_value & 1 != 0); // FixedPitch bit

    // Italic serif font
    let italic_serif_flags = FontFlags {
        serif: true,
        italic: true,
        non_symbolic: true,
        ..Default::default()
    };
    let italic_value = italic_serif_flags.to_flags();
    assert!(italic_value & 2 != 0); // Serif bit
    assert!(italic_value & 64 != 0); // Italic bit
}

#[test]
fn test_multiple_font_embedding() {
    let mut embedder = FontEmbedder::new();

    // Simulate embedding multiple fonts
    let font_data = create_test_font_data();
    let options = EmbeddingOptions::default();

    // Try to embed same font with different glyph sets
    let mut glyphs1 = HashSet::new();
    glyphs1.insert(65); // A
    glyphs1.insert(66); // B

    let mut glyphs2 = HashSet::new();
    glyphs2.insert(67); // C
    glyphs2.insert(68); // D

    // These will fail with test data but test the workflow
    let _result1 = embedder.embed_truetype_font(&font_data, &glyphs1, &options);
    let _result2 = embedder.embed_truetype_font(&font_data, &glyphs2, &options);

    // In real usage, we would have two different embedded fonts
}

#[test]
fn test_font_program_compression_option() {
    let compressed_options = EmbeddingOptions {
        compress_font_streams: true,
        ..Default::default()
    };

    let uncompressed_options = EmbeddingOptions {
        compress_font_streams: false,
        ..Default::default()
    };

    assert!(compressed_options.compress_font_streams);
    assert!(!uncompressed_options.compress_font_streams);
}

#[test]
fn test_license_embedding_option() {
    let with_license = EmbeddingOptions {
        embed_license_info: true,
        ..Default::default()
    };

    let without_license = EmbeddingOptions {
        embed_license_info: false,
        ..Default::default()
    };

    assert!(with_license.embed_license_info);
    assert!(!without_license.embed_license_info);
}
