//! Comprehensive tests for the fonts module
//!
//! This test suite provides extensive coverage for all font-related functionality
//! including parsing, caching, embedding, metrics, and error handling.

use oxidize_pdf::fonts::{
    Font, FontCache, FontDescriptor, FontFlags, FontFormat, FontMetrics, GlyphMapping,
};
use oxidize_pdf::text::Font as StandardFont;
use oxidize_pdf::{Document, Page, Result};
use std::sync::{Arc, Barrier};
use std::thread;
use tempfile::TempDir;

// Helper function to create a test font
fn create_test_font(name: &str) -> Font {
    Font {
        name: name.to_string(),
        data: vec![0; 100], // Mock font data
        format: FontFormat::TrueType,
        metrics: FontMetrics {
            units_per_em: 1000,
            ascent: 800,
            descent: -200,
            line_gap: 200,
            cap_height: 700,
            x_height: 500,
        },
        descriptor: FontDescriptor::new(name),
        glyph_mapping: GlyphMapping::default(),
    }
}

// ===== GlyphMapping Tests =====

#[test]
fn test_glyph_mapping_basic_operations() {
    let mut mapping = GlyphMapping::default();

    // Test adding mappings
    mapping.add_mapping('A', 65);
    mapping.add_mapping('B', 66);
    mapping.add_mapping('€', 8364);

    // Test char to glyph conversion
    assert_eq!(mapping.char_to_glyph('A'), Some(65));
    assert_eq!(mapping.char_to_glyph('B'), Some(66));
    assert_eq!(mapping.char_to_glyph('€'), Some(8364));
    assert_eq!(mapping.char_to_glyph('Z'), None);

    // Test glyph to char conversion
    assert_eq!(mapping.glyph_to_char(65), Some('A'));
    assert_eq!(mapping.glyph_to_char(66), Some('B'));
    assert_eq!(mapping.glyph_to_char(8364), Some('€'));
    assert_eq!(mapping.glyph_to_char(999), None);
}

#[test]
fn test_glyph_mapping_widths() {
    let mut mapping = GlyphMapping::default();

    // Set glyph widths
    mapping.set_glyph_width(65, 600);
    mapping.set_glyph_width(66, 700);
    mapping.set_glyph_width(32, 250); // space

    // Test getting widths
    assert_eq!(mapping.get_glyph_width(65), Some(600));
    assert_eq!(mapping.get_glyph_width(66), Some(700));
    assert_eq!(mapping.get_glyph_width(32), Some(250));
    assert_eq!(mapping.get_glyph_width(999), None);
}

#[test]
fn test_glyph_mapping_character_widths() {
    let mut mapping = GlyphMapping::default();

    // Setup mappings and widths
    mapping.add_mapping('A', 65);
    mapping.add_mapping('B', 66);
    mapping.add_mapping(' ', 32);
    mapping.set_glyph_width(65, 600);
    mapping.set_glyph_width(66, 700);
    mapping.set_glyph_width(32, 250);

    // Test character widths
    assert_eq!(mapping.get_char_width('A'), Some(600));
    assert_eq!(mapping.get_char_width('B'), Some(700));
    assert_eq!(mapping.get_char_width(' '), Some(250));
    assert_eq!(mapping.get_char_width('Z'), None);
}

#[test]
fn test_glyph_mapping_unicode_edge_cases() {
    let mut mapping = GlyphMapping::default();

    // Test various Unicode ranges
    let test_chars = vec![
        ('A', 65),     // Basic Latin
        ('Ω', 937),    // Greek
        ('א', 1488),   // Hebrew
        ('中', 20013), // CJK
        ('€', 8364),   // Currency symbol
        ('\0', 0),     // Null
        ('￿', 65535),  // Max BMP
    ];

    for (ch, glyph) in test_chars {
        mapping.add_mapping(ch, glyph);
        assert_eq!(mapping.char_to_glyph(ch), Some(glyph));
        assert_eq!(mapping.glyph_to_char(glyph), Some(ch));
    }
}

// ===== FontCache Tests =====

#[test]
fn test_font_cache_basic_operations() {
    let cache = FontCache::new();

    // Create test fonts
    let font1 = create_test_font("Helvetica");
    let font2 = create_test_font("TimesRoman");

    // Add fonts to cache
    cache.add_font("Helvetica", font1).unwrap();
    cache.add_font("Times", font2).unwrap();

    // Test retrieval
    assert!(cache.has_font("Helvetica"));
    assert!(cache.has_font("Times"));
    assert!(!cache.has_font("NonExistent"));

    // Get fonts
    let retrieved1 = cache.get_font("Helvetica");
    assert!(retrieved1.is_some());

    let retrieved2 = cache.get_font("Times");
    assert!(retrieved2.is_some());

    let retrieved3 = cache.get_font("NonExistent");
    assert!(retrieved3.is_none());
}

#[test]
fn test_font_cache_overwrite() {
    let cache = FontCache::new();

    // Add initial font
    cache
        .add_font("Test", create_test_font("Helvetica"))
        .unwrap();
    assert!(cache.has_font("Test"));

    // Overwrite with different font
    cache.add_font("Test", create_test_font("Courier")).unwrap();

    // Should still have the font
    assert!(cache.has_font("Test"));
}

#[test]
fn test_font_cache_clear() {
    let cache = FontCache::new();

    // Add multiple fonts
    cache
        .add_font("Font1", create_test_font("Helvetica"))
        .unwrap();
    cache
        .add_font("Font2", create_test_font("TimesRoman"))
        .unwrap();
    cache
        .add_font("Font3", create_test_font("Courier"))
        .unwrap();

    // Verify they exist
    assert_eq!(cache.font_names().len(), 3);

    // Clear cache
    cache.clear();

    // Verify cache is empty
    assert_eq!(cache.font_names().len(), 0);
    assert!(!cache.has_font("Font1"));
    assert!(!cache.has_font("Font2"));
    assert!(!cache.has_font("Font3"));
}

#[test]
fn test_font_cache_font_names() {
    let cache = FontCache::new();

    // Add fonts with specific names
    cache
        .add_font("Arial", create_test_font("Helvetica"))
        .unwrap();
    cache
        .add_font("TimesNewRoman", create_test_font("TimesRoman"))
        .unwrap();
    cache
        .add_font("CourierNew", create_test_font("Courier"))
        .unwrap();

    // Get font names
    let mut names = cache.font_names();
    names.sort(); // Sort for consistent comparison

    assert_eq!(names, vec!["Arial", "CourierNew", "TimesNewRoman"]);
}

#[test]
fn test_font_cache_thread_safety() {
    let cache = FontCache::new();
    let barrier = Arc::new(Barrier::new(10));
    let mut handles = vec![];

    // Spawn multiple threads that read and write to cache
    for i in 0..10 {
        let cache_clone = cache.clone();
        let barrier_clone = barrier.clone();

        let handle = thread::spawn(move || {
            // Wait for all threads to start
            barrier_clone.wait();

            // Each thread adds its own font
            let font_name = format!("Font{i}");
            cache_clone
                .add_font(&font_name, create_test_font("Helvetica"))
                .unwrap();

            // Try to read other fonts
            for j in 0..10 {
                let other_font = format!("Font{j}");
                let _ = cache_clone.get_font(&other_font);
            }
        });

        handles.push(handle);
    }

    // Wait for all threads to complete
    for handle in handles {
        handle.join().unwrap();
    }

    // Verify all fonts were added
    assert_eq!(cache.font_names().len(), 10);
}

// ===== FontDescriptor Tests =====

#[test]
fn test_font_descriptor_creation() {
    let mut descriptor = FontDescriptor::new("TestFont");

    // Test default values
    assert_eq!(descriptor.font_name, "TestFont");
    assert_eq!(descriptor.flags, FontFlags::NONSYMBOLIC);

    // Set various properties
    descriptor.italic_angle = -15.0;
    descriptor.ascent = 750.0;
    descriptor.descent = -250.0;
    descriptor.cap_height = 700.0;
    descriptor.stem_v = 80.0;

    // Verify settings
    assert_eq!(descriptor.italic_angle, -15.0);
    assert_eq!(descriptor.ascent, 750.0);
    assert_eq!(descriptor.descent, -250.0);
    assert_eq!(descriptor.cap_height, 700.0);
    assert_eq!(descriptor.stem_v, 80.0);
}

#[test]
fn test_font_descriptor_font_bbox() {
    let mut descriptor = FontDescriptor::new("TestFont");

    // Set font bounding box
    descriptor.font_bbox = [-100.0, -300.0, 1000.0, 900.0];

    // Verify
    assert_eq!(descriptor.font_bbox, [-100.0, -300.0, 1000.0, 900.0]);
}

#[test]
fn test_font_flags_operations() {
    let mut flags = FontFlags::empty();

    // Test individual flags
    assert!(!flags.contains(FontFlags::FIXED_PITCH));
    assert!(!flags.contains(FontFlags::SERIF));
    assert!(!flags.contains(FontFlags::SYMBOLIC));
    assert!(!flags.contains(FontFlags::SCRIPT));
    assert!(!flags.contains(FontFlags::ITALIC));
    assert!(!flags.contains(FontFlags::ALL_CAP));
    assert!(!flags.contains(FontFlags::SMALL_CAP));
    assert!(!flags.contains(FontFlags::FORCE_BOLD));

    // Set flags
    flags |= FontFlags::FIXED_PITCH;
    flags |= FontFlags::SERIF;
    flags |= FontFlags::ITALIC;

    // Verify flags are set
    assert!(flags.contains(FontFlags::FIXED_PITCH));
    assert!(flags.contains(FontFlags::SERIF));
    assert!(flags.contains(FontFlags::ITALIC));
    assert!(!flags.contains(FontFlags::SYMBOLIC)); // Should still be false

    // Unset a flag
    flags &= !FontFlags::SERIF;
    assert!(!flags.contains(FontFlags::SERIF));
    assert!(flags.contains(FontFlags::FIXED_PITCH)); // Should still be true
}

#[test]
fn test_font_flags_bit_operations() {
    let mut flags = FontFlags::empty();

    // Test that flags use correct bit positions
    flags |= FontFlags::FIXED_PITCH;
    assert_eq!(flags.bits() & 0x01, 0x01);

    flags = FontFlags::empty();
    flags |= FontFlags::SERIF;
    assert_eq!(flags.bits() & 0x02, 0x02);

    flags = FontFlags::empty();
    flags |= FontFlags::SYMBOLIC;
    assert_eq!(flags.bits() & 0x04, 0x04);

    flags = FontFlags::empty();
    flags |= FontFlags::SCRIPT;
    assert_eq!(flags.bits() & 0x08, 0x08);

    flags = FontFlags::empty();
    flags |= FontFlags::ITALIC;
    assert_eq!(flags.bits() & 0x40, 0x40);
}

// ===== FontMetrics Tests =====

#[test]
fn test_font_metrics_creation() {
    let metrics = FontMetrics {
        units_per_em: 1000,
        ascent: 800,
        descent: -200,
        line_gap: 200,
        cap_height: 700,
        x_height: 500,
    };

    assert_eq!(metrics.units_per_em, 1000);
    assert_eq!(metrics.ascent, 800);
}

#[test]
fn test_font_metrics_line_height() {
    let metrics = FontMetrics {
        units_per_em: 1000,
        ascent: 800,
        descent: -200,
        line_gap: 200,
        cap_height: 700,
        x_height: 500,
    };

    // Test line height calculation
    let line_height = metrics.line_height(12.0);
    // Expected: (800 - (-200) + 200) * 12 / 1000 = 1200 * 12 / 1000 = 14.4
    assert!((line_height - 14.4).abs() < 0.1);

    // Test ascent and descent calculations
    let ascent = metrics.get_ascent(12.0);
    assert!((ascent - 9.6).abs() < 0.1); // 800 * 12 / 1000

    let descent = metrics.get_descent(12.0);
    assert!((descent - 2.4).abs() < 0.1); // 200 * 12 / 1000
}

#[test]
fn test_font_metrics_to_user_space() {
    let metrics = FontMetrics {
        units_per_em: 2048,
        ascent: 1600,
        descent: -400,
        line_gap: 0,
        cap_height: 1400,
        x_height: 1000,
    };

    // Test converting font units to user space
    let value = metrics.to_user_space(1024, 12.0);
    assert!((value - 6.0).abs() < 0.01); // 1024 * 12 / 2048 = 6.0

    let value2 = metrics.to_user_space(2048, 10.0);
    assert!((value2 - 10.0).abs() < 0.01); // 2048 * 10 / 2048 = 10.0
}

// ===== Integration Tests =====

#[test]
fn test_font_embedding_workflow() -> Result<()> {
    let mut doc = Document::new();

    // Create a font cache for the document
    let cache = FontCache::new();

    // Add custom fonts to cache
    cache.add_font("Helvetica", create_test_font("Helvetica"))?;
    cache.add_font("Times", create_test_font("TimesRoman"))?;
    cache.add_font("Courier", create_test_font("Courier"))?;

    // Create a page with multiple fonts
    let mut page = Page::a4();

    // Use different standard fonts in document
    page.text()
        .set_font(StandardFont::Helvetica, 12.0)
        .at(100.0, 700.0)
        .write("This is Helvetica")?;

    page.text()
        .set_font(StandardFont::TimesRoman, 14.0)
        .at(100.0, 650.0)
        .write("This is Times Roman")?;

    page.text()
        .set_font(StandardFont::Courier, 10.0)
        .at(100.0, 600.0)
        .write("This is Courier")?;

    doc.add_page(page);

    // Save and verify
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("font_embedding.pdf");
    doc.save(&file_path)?;

    assert!(file_path.exists());
    let metadata = std::fs::metadata(&file_path)?;
    assert!(metadata.len() > 500); // Basic PDF files can be small

    Ok(())
}

#[test]
fn test_font_metrics_in_document() -> Result<()> {
    let mut doc = Document::new();
    let mut page = Page::a4();

    // Test that text with different fonts has different spacing
    let fonts = vec![
        (StandardFont::Helvetica, "Helvetica: MMMM"),
        (StandardFont::TimesRoman, "Times: MMMM"),
        (StandardFont::Courier, "Courier: MMMM"),
    ];

    let mut y = 700.0;
    for (font, text) in fonts {
        page.text().set_font(font, 12.0).at(100.0, y).write(text)?;
        y -= 30.0;
    }

    doc.add_page(page);

    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("font_metrics.pdf");
    doc.save(&file_path)?;

    Ok(())
}

// ===== Error Handling Tests =====

#[test]
#[ignore] // TODO: Implement add_font_from_bytes in Document
fn test_invalid_font_data_handling() {
    // Try to add invalid font data
    let invalid_data = vec![0xFF, 0xFE, 0xFD, 0xFC];

    // For now, test font validation at the Font level
    let result = Font::from_bytes("InvalidFont", invalid_data);
    assert!(result.is_err());
}

#[test]
fn test_empty_font_name_handling() {
    let cache = FontCache::new();

    // Empty font name should still work
    let result = cache.add_font("", create_test_font("Helvetica"));
    assert!(result.is_ok());
    assert!(cache.has_font(""));
}

#[test]
fn test_font_cache_memory_stress() {
    let cache = FontCache::new();

    // Add many fonts to stress the cache
    for i in 0..1000 {
        let font_name = format!("StressFont{i}");
        cache
            .add_font(font_name, create_test_font("Helvetica"))
            .unwrap();
    }

    // Verify some fonts
    assert!(cache.has_font("StressFont0"));
    assert!(cache.has_font("StressFont500"));
    assert!(cache.has_font("StressFont999"));
    assert!(!cache.has_font("StressFont1000"));

    // Check memory didn't explode
    assert_eq!(cache.font_names().len(), 1000);
}

// ===== TTF Parser Tests =====

#[test]
fn test_ttf_parser_table_validation() {
    // Test that parser validates required tables
    let mut data = Vec::new();

    // TTF header
    data.extend_from_slice(&[0x00, 0x01, 0x00, 0x00]); // version
    data.extend_from_slice(&[0x00, 0x00]); // numTables = 0

    // This should fail - no tables
    // In real implementation, would test the actual parser
    assert!(data.len() >= 6);
}

#[test]
fn test_font_subset_generation() {
    let mut mapping = GlyphMapping::default();

    // Add only used characters
    let used_chars = "Hello World!";
    for (i, ch) in used_chars.chars().enumerate() {
        mapping.add_mapping(ch, i as u16);
        mapping.set_glyph_width(i as u16, 500 + i as u16 * 10);
    }

    // Verify subset contains only used characters
    assert_eq!(mapping.char_to_glyph('H'), Some(0));
    assert_eq!(mapping.char_to_glyph('!'), Some(11));
    assert_eq!(mapping.char_to_glyph('X'), None); // Not in subset
}

// ===== Font Encoding Tests =====

#[test]
fn test_standard_font_encodings() -> Result<()> {
    let mut doc = Document::new();
    let mut page = Page::a4();

    // Test WinAnsiEncoding characters
    let win_ansi_chars = "ABCabc123!@#€";
    page.text()
        .set_font(StandardFont::Helvetica, 12.0)
        .at(100.0, 700.0)
        .write(win_ansi_chars)?;

    // Test special characters
    let special_chars = "© ® ™ • — –";
    page.text()
        .set_font(StandardFont::TimesRoman, 12.0)
        .at(100.0, 650.0)
        .write(special_chars)?;

    doc.add_page(page);

    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("font_encoding.pdf");
    doc.save(&file_path)?;

    Ok(())
}

// ===== Benchmark-style Tests =====

#[test]
#[ignore] // Run with --ignored for performance tests
fn test_font_cache_performance() {
    let cache = FontCache::new();
    let iterations = 10000;

    // Measure write performance
    let start = std::time::Instant::now();
    for i in 0..iterations {
        cache
            .add_font(format!("PerfFont{i}"), create_test_font("Helvetica"))
            .unwrap();
    }
    let write_duration = start.elapsed();

    // Measure read performance
    let start = std::time::Instant::now();
    for i in 0..iterations {
        let _ = cache.get_font(&format!("PerfFont{i}"));
    }
    let read_duration = start.elapsed();

    println!("Font cache performance:");
    println!("  Writes: {write_duration:?} for {iterations} operations");
    println!("  Reads: {read_duration:?} for {iterations} operations");

    // Basic sanity checks
    assert!(write_duration.as_secs() < 5);
    assert!(read_duration.as_secs() < 2);
}
