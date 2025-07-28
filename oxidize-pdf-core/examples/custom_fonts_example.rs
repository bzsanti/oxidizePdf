//! Example demonstrating Type 1 and TrueType font support

use oxidize_pdf::{
    text::{
        CustomFont, EncodingDifference, ExtendedFont, ExtendedFontManager, Font, FontDescriptor,
        FontEncoding, FontFlags, FontMetrics,
    },
    Document, Page, Result,
};

fn main() -> Result<()> {
    // Create a new document
    let mut doc = Document::new();

    // Create a font manager
    let mut font_manager = ExtendedFontManager::new();

    // Create a new page
    let mut page = Page::a4();
    let graphics = page.graphics();

    // Example 1: Using standard Type 1 fonts
    {
        graphics
            .begin_text()
            .set_font(Font::Helvetica, 16.0)
            .set_text_position(50.0, 750.0)
            .show_text("Standard Type 1 Fonts (Base 14)")?
            .end_text();

        let fonts = vec![
            (Font::TimesRoman, "Times Roman: The quick brown fox"),
            (Font::TimesBold, "Times Bold: jumps over the lazy dog"),
            (Font::TimesItalic, "Times Italic: 0123456789"),
            (Font::Helvetica, "Helvetica: ABCDEFGHIJKLMNOPQRSTUVWXYZ"),
            (
                Font::HelveticaBold,
                "Helvetica Bold: abcdefghijklmnopqrstuvwxyz",
            ),
            (Font::Courier, "Courier: Fixed width font"),
            (Font::Symbol, "Symbol: αβγδεζηθικλμνξοπρστυφχψω"),
        ];

        let mut y = 700.0;
        for (font, text) in fonts {
            graphics
                .begin_text()
                .set_font(font, 12.0)
                .set_text_position(70.0, y)
                .show_text(text)?
                .end_text();
            y -= 25.0;
        }
    }

    // Example 2: Creating a custom Type 1 font
    {
        graphics
            .begin_text()
            .set_font(Font::HelveticaBold, 16.0)
            .set_text_position(50.0, 450.0)
            .show_text("Custom Type 1 Font")?
            .end_text();

        // Create custom Type 1 font with specific metrics
        let flags = FontFlags {
            serif: true,
            non_symbolic: true,
            ..Default::default()
        };

        let descriptor = FontDescriptor::new(
            "CustomSerif".to_string(),
            flags,
            [-200.0, -250.0, 1100.0, 950.0],
            -12.0,  // Italic angle
            750.0,  // Ascent
            -250.0, // Descent
            700.0,  // Cap height
            90.0,   // Stem width
        );

        // Custom character widths (simplified)
        let mut widths = Vec::new();
        for i in 32..=126 {
            // Variable widths based on character
            let width = match i {
                32 => 250.0,       // Space
                73 | 105 => 333.0, // I, i (narrow)
                77 | 87 => 944.0,  // M, W (wide)
                _ => 556.0,        // Default
            };
            widths.push(width);
        }

        let metrics = FontMetrics::new(32, 126, widths, 250.0);

        let custom_font = CustomFont::new_type1(
            "CustomSerif".to_string(),
            FontEncoding::WinAnsiEncoding,
            descriptor,
            metrics,
        );

        let font_name = font_manager.register_custom_font(custom_font)?;

        graphics
            .begin_text()
            .set_font(Font::Helvetica, 12.0) // Would use custom font with full integration
            .set_text_position(70.0, 400.0)
            .show_text("Custom Type 1 font registered as: ")?
            .show_text(&font_name)?
            .end_text();
    }

    // Example 3: Custom encoding with Type 1 font
    {
        graphics
            .begin_text()
            .set_font(Font::HelveticaBold, 16.0)
            .set_text_position(50.0, 350.0)
            .show_text("Custom Encoding")?
            .end_text();

        // Create font with custom encoding differences
        let encoding_differences = vec![
            EncodingDifference {
                code: 128,
                names: vec!["Euro".to_string()],
            },
            EncodingDifference {
                code: 160,
                names: vec![
                    "space".to_string(),
                    "exclamdown".to_string(),
                    "cent".to_string(),
                ],
            },
        ];

        let custom_encoding = FontEncoding::Custom(encoding_differences);

        let descriptor = FontDescriptor::new(
            "CustomEncoded".to_string(),
            FontFlags {
                non_symbolic: true,
                ..Default::default()
            },
            [-166.0, -225.0, 1000.0, 931.0],
            0.0,
            718.0,
            -207.0,
            718.0,
            88.0,
        );

        let metrics = FontMetrics::new(32, 255, vec![556.0; 224], 278.0);

        let encoded_font = CustomFont::new_type1(
            "Helvetica-Custom".to_string(),
            custom_encoding,
            descriptor,
            metrics,
        );

        let font_name = font_manager.register_custom_font(encoded_font)?;

        graphics
            .begin_text()
            .set_font(Font::Helvetica, 12.0)
            .set_text_position(70.0, 300.0)
            .show_text("Font with custom encoding registered as: ")?
            .show_text(&font_name)?
            .end_text();
    }

    // Example 4: TrueType font simulation
    {
        graphics
            .begin_text()
            .set_font(Font::HelveticaBold, 16.0)
            .set_text_position(50.0, 250.0)
            .show_text("TrueType Font Support")?
            .end_text();

        // Create TrueType font
        let flags = FontFlags {
            non_symbolic: true,
            ..Default::default()
        };

        let mut descriptor = FontDescriptor::new(
            "ArialRegular".to_string(),
            flags,
            [-665.0, -325.0, 2000.0, 1006.0],
            0.0,
            905.0,
            -212.0,
            728.0,
            88.0,
        );

        // TrueType specific metrics
        descriptor.x_height = Some(519.0);
        descriptor.avg_width = Some(441.0);
        descriptor.max_width = Some(2000.0);
        descriptor.font_family = Some("Arial".to_string());

        // Simplified glyph widths
        let widths = vec![750.0; 224]; // All same width for simplicity

        let metrics = FontMetrics::new(32, 255, widths, 750.0);

        let truetype_font = CustomFont::new_truetype(
            "ArialRegular".to_string(),
            FontEncoding::WinAnsiEncoding,
            descriptor,
            metrics,
        );

        let font_name = font_manager.register_custom_font(truetype_font)?;

        graphics
            .begin_text()
            .set_font(Font::Helvetica, 12.0)
            .set_text_position(70.0, 200.0)
            .show_text("TrueType font registered as: ")?
            .show_text(&font_name)?
            .end_text();
    }

    // Example 5: Font metrics demonstration
    {
        graphics
            .begin_text()
            .set_font(Font::HelveticaBold, 16.0)
            .set_text_position(50.0, 150.0)
            .show_text("Font Metrics and Measurement")?
            .end_text();

        // Demonstrate text measurement with extended fonts
        let helvetica = ExtendedFont::from_standard(Font::Helvetica);
        let text = "Hello, World!";
        let font_size = 12.0;
        let width = helvetica.measure_text(text, font_size);

        graphics
            .begin_text()
            .set_font(Font::Helvetica, 10.0)
            .set_text_position(70.0, 100.0)
            .show_text(&format!(
                "Text '{}' in Helvetica 12pt = {:.2} points wide",
                text, width
            ))?
            .end_text();

        // Show font information
        let info = format!(
            "Registered {} custom fonts",
            font_manager.fonts().len() - 14
        ); // Minus standard fonts
        graphics
            .begin_text()
            .set_font(Font::Helvetica, 10.0)
            .set_text_position(70.0, 80.0)
            .show_text(&info)?
            .end_text();
    }

    // Add page to document
    doc.add_page(page);

    // Save the document
    doc.save("custom_fonts_example.pdf")?;
    println!("Created custom_fonts_example.pdf demonstrating Type 1 and TrueType font support");

    Ok(())
}
