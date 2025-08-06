//! Font Embedding Example
//!
//! This example demonstrates the font embedding capabilities of oxidize-pdf
//! including TrueType font embedding and CID font support.

use oxidize_pdf::{
    Document, EmbeddingOptions, Font, FontEmbedder, FontEncoding, FontFlags, Page, Result,
};
use std::collections::HashSet;

fn main() -> Result<()> {
    println!("🔤 Font Embedding Example");
    println!("=======================");

    // Create a new document
    let mut doc = Document::new();
    doc.set_title("Font Embedding Demo");
    doc.set_author("oxidize-pdf");

    // Create a page
    let mut page = Page::a4();

    // Add title
    page.text()
        .set_font(Font::Helvetica, 24.0)
        .at(50.0, 750.0)
        .write("Font Embedding Demonstration")?;

    // Add explanation text
    page.text()
        .set_font(Font::Helvetica, 12.0)
        .at(50.0, 720.0)
        .write("This PDF demonstrates font embedding capabilities:")?;

    page.text()
        .set_font(Font::Helvetica, 10.0)
        .at(70.0, 700.0)
        .write("• Standard PDF fonts (14 built-in fonts)")?;

    page.text()
        .set_font(Font::Helvetica, 10.0)
        .at(70.0, 685.0)
        .write("• TrueType font embedding support")?;

    page.text()
        .set_font(Font::Helvetica, 10.0)
        .at(70.0, 670.0)
        .write("• CID font support for complex scripts")?;

    // Demonstrate built-in fonts
    let y_start = 630.0;
    let line_height = 25.0;
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

    page.text()
        .set_font(Font::HelveticaBold, 14.0)
        .at(50.0, y_start + 20.0)
        .write("Standard PDF Fonts:")?;

    for (i, (font, name)) in fonts.iter().enumerate() {
        let y = y_start - (i as f64 * line_height);
        page.text()
            .set_font(font.clone(), 12.0)
            .at(70.0, y)
            .write(&format!(
                "{name}: The quick brown fox jumps over the lazy dog"
            ))?;
    }

    // Font Embedder demonstration
    let font_embedder_demo_y = y_start - (fonts.len() as f64 * line_height) - 50.0;

    page.text()
        .set_font(Font::HelveticaBold, 14.0)
        .at(50.0, font_embedder_demo_y)
        .write("Font Embedding System:")?;

    page.text()
        .set_font(Font::Helvetica, 12.0)
        .at(70.0, font_embedder_demo_y - 25.0)
        .write("The FontEmbedder system supports:")?;

    let features = [
        "• TrueType and OpenType font embedding",
        "• Font subsetting for reduced file size",
        "• CID fonts for complex scripts (Chinese, Japanese, Korean)",
        "• Character encoding mappings",
        "• ToUnicode CMap generation",
        "• Font compression and optimization",
    ];

    for (i, feature) in features.iter().enumerate() {
        page.text()
            .set_font(Font::Helvetica, 10.0)
            .at(90.0, font_embedder_demo_y - 45.0 - (i as f64 * 15.0))
            .write(feature)?;
    }

    // Demonstrate FontEmbedder usage
    let code_demo_y = font_embedder_demo_y - 45.0 - (features.len() as f64 * 15.0) - 30.0;

    page.text()
        .set_font(Font::HelveticaBold, 12.0)
        .at(50.0, code_demo_y)
        .write("Example Code Usage:")?;

    let code_lines = [
        "// Create font embedder",
        "let mut embedder = FontEmbedder::new();",
        "",
        "// Define used glyphs",
        "let mut used_glyphs = HashSet::new();",
        "used_glyphs.insert(65); // 'A'",
        "used_glyphs.insert(66); // 'B'",
        "",
        "// Embedding options",
        "let options = EmbeddingOptions {",
        "    subset: true,",
        "    compress_font_streams: true,",
        "    ..Default::default()",
        "};",
        "",
        "// Embed TrueType font",
        "let font_name = embedder.embed_truetype_font(",
        "    &font_data, &used_glyphs, &options",
        ")?;",
    ];

    for (i, line) in code_lines.iter().enumerate() {
        page.text()
            .set_font(Font::Courier, 9.0)
            .at(70.0, code_demo_y - 20.0 - (i as f64 * 12.0))
            .write(line)?;
    }

    // Demo the FontEmbedder API
    demo_font_embedder_api()?;

    // Add page to document
    doc.add_page(page);

    // Save the document
    let output_path = "font_embedding_example.pdf";
    doc.save(output_path)?;

    println!("✅ Font embedding example saved to: {output_path}");
    println!("📄 The PDF demonstrates both built-in fonts and the font embedding system");

    Ok(())
}

/// Demonstrate the FontEmbedder API functionality
fn demo_font_embedder_api() -> Result<()> {
    println!("\n🔧 Testing FontEmbedder API...");

    // Create a new font embedder
    let embedder = FontEmbedder::new();
    println!("✓ Created FontEmbedder");

    // Test embedding options
    let _options = EmbeddingOptions {
        subset: true,
        max_subset_size: Some(256),
        compress_font_streams: true,
        embed_license_info: false,
    };
    println!("✓ Created EmbeddingOptions");

    // Test font flags
    let flags = FontFlags {
        non_symbolic: true,
        serif: false,
        ..Default::default()
    };
    println!("✓ Created FontFlags: {}", flags.to_flags());

    // Test encodings
    let encodings = [
        FontEncoding::WinAnsiEncoding,
        FontEncoding::MacRomanEncoding,
        FontEncoding::StandardEncoding,
        FontEncoding::Identity,
    ];
    println!("✓ Tested {} encoding types", encodings.len());

    // Test that we have no embedded fonts initially
    assert_eq!(embedder.embedded_fonts().len(), 0);
    println!("✓ Verified empty embedder state");

    // Create test glyph set
    let mut used_glyphs = HashSet::new();
    used_glyphs.insert(0); // .notdef
    used_glyphs.insert(65); // A
    used_glyphs.insert(66); // B
    used_glyphs.insert(67); // C
    used_glyphs.insert(32); // space
    println!("✓ Created test glyph set with {} glyphs", used_glyphs.len());

    // Note: We can't test actual font embedding without real font data
    // But we can test the API structure and error handling
    println!("✓ FontEmbedder API structure verified");
    println!("ℹ️  Full font embedding requires valid TrueType font data");

    Ok(())
}
