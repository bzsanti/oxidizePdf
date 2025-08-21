//! Integration tests for font handling

use oxidize_pdf::fonts::{Font, FontCache, FontDescriptor, FontFormat, FontLoader, FontMetrics};
use oxidize_pdf::{Document, Page};

#[test]
fn test_font_loading_and_embedding() {
    // Create test font data (minimal TTF header)
    let mut ttf_data = vec![0x00, 0x01, 0x00, 0x00]; // TTF magic
    ttf_data.extend_from_slice(&[0x00; 100]); // Padding

    // Load font
    let font_data = FontLoader::load_from_bytes(ttf_data.clone()).unwrap();
    assert_eq!(font_data.format, FontFormat::TrueType);

    // Validate font
    assert!(FontLoader::validate(&font_data).is_ok());
}

#[test]
fn test_font_cache_operations() {
    let cache = FontCache::new();

    // Create test fonts
    let font1 = Font {
        name: "TestFont1".to_string(),
        data: vec![0; 100],
        format: FontFormat::TrueType,
        metrics: FontMetrics::default(),
        descriptor: FontDescriptor::new("TestFont1"),
        glyph_mapping: Default::default(),
    };

    let font2 = Font {
        name: "TestFont2".to_string(),
        data: vec![0; 200],
        format: FontFormat::OpenType,
        metrics: FontMetrics::default(),
        descriptor: FontDescriptor::new("TestFont2"),
        glyph_mapping: Default::default(),
    };

    // Add fonts to cache
    cache.add_font("Font1", font1).unwrap();
    cache.add_font("Font2", font2).unwrap();

    // Verify cache operations
    assert_eq!(cache.len(), 2);
    assert!(cache.has_font("Font1"));
    assert!(cache.has_font("Font2"));
    assert!(!cache.has_font("Font3"));

    // Retrieve font
    let retrieved = cache.get_font("Font1").unwrap();
    assert_eq!(retrieved.name, "TestFont1");

    // Clear cache
    cache.clear();
    assert_eq!(cache.len(), 0);
    assert!(cache.is_empty());
}

#[test]
fn test_font_metrics_calculations() {
    let metrics = FontMetrics {
        units_per_em: 1000,
        ascent: 800,
        descent: -200,
        line_gap: 200,
        cap_height: 700,
        x_height: 500,
    };

    let font_size = 12.0;

    // Test metric conversions - use approximate comparisons for floats
    assert!((metrics.get_ascent(font_size) - 9.6).abs() < 0.01);
    assert!((metrics.get_descent(font_size) - 2.4).abs() < 0.01);
    assert!((metrics.line_height(font_size) - 14.4).abs() < 0.01);

    // Test text measurement
    let glyph_mapping = Default::default();
    let measurement = metrics.measure_text("Test", font_size, &glyph_mapping);

    assert!((measurement.height - 14.4).abs() < 0.01);
    assert!((measurement.ascent - 9.6).abs() < 0.01);
    assert!((measurement.descent - 2.4).abs() < 0.01);
}

#[test]
fn test_pdf_with_embedded_font() {
    let mut doc = Document::new();

    // Create and add a page
    let mut page = Page::new(595.0, 842.0);
    page.text()
        .set_font(oxidize_pdf::text::Font::Helvetica, 12.0)
        .at(100.0, 700.0)
        .write("Hello, World!")
        .ok();
    doc.add_page(page);

    // Save to bytes
    let pdf_bytes = doc.to_bytes().unwrap();

    // Verify PDF contains font references
    let pdf_str = String::from_utf8_lossy(&pdf_bytes);
    assert!(pdf_str.contains("/Font"));
    assert!(pdf_str.contains("/Type"));
}

#[test]
fn test_font_descriptor_creation() {
    use oxidize_pdf::fonts::FontFlags;

    let descriptor = FontDescriptor::new("TestFont");

    // Test basic properties
    assert_eq!(descriptor.font_name, "TestFont");
    assert_eq!(descriptor.flags, FontFlags::NONSYMBOLIC); // Default flags

    // Test bounding box
    assert_eq!(descriptor.font_bbox, [0.0, 0.0, 1000.0, 1000.0]);

    // Test other metrics
    assert_eq!(descriptor.ascent, 800.0);
    assert_eq!(descriptor.descent, -200.0);
    assert_eq!(descriptor.cap_height, 700.0);
    assert_eq!(descriptor.stem_v, 80.0);
}

#[test]
fn test_glyph_mapping() {
    use oxidize_pdf::fonts::GlyphMapping;

    let mut mapping = GlyphMapping::default();

    // Add character mappings
    mapping.add_mapping('A', 65);
    mapping.add_mapping('B', 66);
    mapping.add_mapping('€', 8364);

    // Set glyph widths
    mapping.set_glyph_width(65, 600);
    mapping.set_glyph_width(66, 650);
    mapping.set_glyph_width(8364, 700);

    // Test mappings
    assert_eq!(mapping.char_to_glyph('A'), Some(65));
    assert_eq!(mapping.char_to_glyph('B'), Some(66));
    assert_eq!(mapping.char_to_glyph('€'), Some(8364));
    assert_eq!(mapping.char_to_glyph('Z'), None);

    // Test widths
    assert_eq!(mapping.get_char_width('A'), Some(600));
    assert_eq!(mapping.get_char_width('B'), Some(650));
    assert_eq!(mapping.get_char_width('€'), Some(700));
    assert_eq!(mapping.get_char_width('Z'), None);
}

#[test]
fn test_font_format_detection() {
    // Test various font format detections
    let ttf_magic = vec![0x00, 0x01, 0x00, 0x00];
    assert_eq!(
        FontFormat::detect(&ttf_magic).unwrap(),
        FontFormat::TrueType
    );

    let otf_magic = vec![0x4F, 0x54, 0x54, 0x4F];
    assert_eq!(
        FontFormat::detect(&otf_magic).unwrap(),
        FontFormat::OpenType
    );

    let ttf_true = vec![0x74, 0x72, 0x75, 0x65];
    assert_eq!(FontFormat::detect(&ttf_true).unwrap(), FontFormat::TrueType);

    let invalid = vec![0xFF, 0xFF, 0xFF, 0xFF];
    assert!(FontFormat::detect(&invalid).is_err());

    let too_small = vec![0x00, 0x01];
    assert!(FontFormat::detect(&too_small).is_err());
}
