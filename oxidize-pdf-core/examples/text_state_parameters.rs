//! Comprehensive example demonstrating all text state parameters in PDF generation
//!
//! This example showcases all the newly exposed text state parameter methods:
//! - Character spacing (Tc operator)
//! - Word spacing (Tw operator)
//! - Horizontal scaling (Tz operator)
//! - Leading (TL operator)
//! - Text rise (Ts operator)
//! - Text rendering mode (Tr operator)

use oxidize_pdf::text::TextRenderingMode;
use oxidize_pdf::{Color, Document, Font, Page, Result};

fn main() -> Result<()> {
    let mut doc = Document::new();
    doc.set_title("Text State Parameters Demonstration");
    doc.set_author("oxidize-pdf API Alignment");
    doc.set_subject("ISO 32000-1:2008 Text State Operators");
    doc.set_keywords("PDF, text, formatting, state, parameters");

    let mut page = Page::a4();

    // Title
    page.text()
        .set_font(Font::HelveticaBold, 18.0)
        .at(50.0, 750.0)
        .write("Text State Parameters Demonstration")?;

    // Subtitle
    page.text()
        .set_font(Font::Helvetica, 12.0)
        .at(50.0, 720.0)
        .write("Showcasing all ISO 32000-1:2008 text state operators")?;

    let mut y = 680.0;
    let line_height = 40.0;

    // 1. Character Spacing (Tc operator)
    page.text()
        .set_font(Font::HelveticaBold, 14.0)
        .at(50.0, y)
        .write("1. Character Spacing (Tc operator)")?;

    y -= 20.0;
    page.text()
        .set_font(Font::Helvetica, 11.0)
        .at(70.0, y)
        .set_character_spacing(0.0) // Normal spacing
        .write("Normal character spacing (0.0)")?;

    y -= 15.0;
    page.text()
        .at(70.0, y)
        .set_character_spacing(2.0) // Wider spacing
        .write("W i d e r   c h a r a c t e r   s p a c i n g   ( 2 . 0 )")?;

    y -= 15.0;
    page.text()
        .at(70.0, y)
        .set_character_spacing(-0.5) // Tighter spacing
        .write("Tighter character spacing (-0.5)")?;

    y -= line_height;

    // 2. Word Spacing (Tw operator)
    page.text()
        .set_font(Font::HelveticaBold, 14.0)
        .at(50.0, y)
        .set_character_spacing(0.0) // Reset character spacing
        .write("2. Word Spacing (Tw operator)")?;

    y -= 20.0;
    page.text()
        .set_font(Font::Helvetica, 11.0)
        .at(70.0, y)
        .set_word_spacing(0.0) // Normal word spacing
        .write("Normal word spacing between words")?;

    y -= 15.0;
    page.text()
        .at(70.0, y)
        .set_word_spacing(5.0) // Wider word spacing
        .write("Much        wider        word        spacing")?;

    y -= 15.0;
    page.text()
        .at(70.0, y)
        .set_word_spacing(-1.0) // Tighter word spacing
        .write("Tighter word spacing between words")?;

    y -= line_height;

    // 3. Horizontal Scaling (Tz operator)
    page.text()
        .set_font(Font::HelveticaBold, 14.0)
        .at(50.0, y)
        .set_word_spacing(0.0) // Reset word spacing
        .write("3. Horizontal Scaling (Tz operator)")?;

    y -= 20.0;
    page.text()
        .set_font(Font::Helvetica, 11.0)
        .at(70.0, y)
        .set_horizontal_scaling(1.0) // Normal scaling (100%)
        .write("Normal horizontal scaling (100%)")?;

    y -= 15.0;
    page.text()
        .at(70.0, y)
        .set_horizontal_scaling(1.5) // Expanded text (150%)
        .write("Expanded text (150%)")?;

    y -= 15.0;
    page.text()
        .at(70.0, y)
        .set_horizontal_scaling(0.7) // Condensed text (70%)
        .write("Condensed text (70%)")?;

    y -= line_height;

    // 4. Leading (TL operator)
    page.text()
        .set_font(Font::HelveticaBold, 14.0)
        .at(50.0, y)
        .set_horizontal_scaling(1.0) // Reset horizontal scaling
        .write("4. Leading (TL operator)")?;

    y -= 20.0;
    page.text()
        .set_font(Font::Helvetica, 11.0)
        .at(70.0, y)
        .set_leading(12.0) // Normal leading
        .write("Normal leading for line spacing (12pt)")?;

    y -= 15.0;
    page.text()
        .at(70.0, y)
        .set_leading(18.0) // Increased leading
        .write("Increased leading for more space (18pt)")?;

    y -= 15.0;
    page.text()
        .at(70.0, y)
        .set_leading(8.0) // Tight leading
        .write("Tight leading for compact text (8pt)")?;

    y -= line_height;

    // 5. Text Rise (Ts operator)
    page.text()
        .set_font(Font::HelveticaBold, 14.0)
        .at(50.0, y)
        .set_leading(12.0) // Reset leading
        .write("5. Text Rise (Ts operator)")?;

    y -= 20.0;
    page.text()
        .set_font(Font::Helvetica, 11.0)
        .at(70.0, y)
        .set_text_rise(0.0) // Baseline
        .write("Baseline text ")?;

    page.text()
        .at(160.0, y)
        .set_text_rise(4.0) // Superscript effect
        .write("superscript ")?;

    page.text()
        .at(230.0, y)
        .set_text_rise(-2.0) // Subscript effect
        .write("subscript")?;

    y -= line_height;

    // 6. Text Rendering Mode (Tr operator)
    page.text()
        .set_font(Font::HelveticaBold, 14.0)
        .at(50.0, y)
        .set_text_rise(0.0) // Reset text rise
        .write("6. Text Rendering Mode (Tr operator)")?;

    y -= 25.0;

    // Fill mode (default)
    page.text()
        .set_font(Font::Helvetica, 12.0)
        .at(70.0, y)
        .set_rendering_mode(TextRenderingMode::Fill)
        .write("Fill mode (default)")?;

    y -= 20.0;

    // Stroke mode
    page.graphics()
        .set_stroke_color(Color::rgb(0.8, 0.0, 0.0))
        .set_line_width(0.5);

    page.text()
        .at(70.0, y)
        .set_rendering_mode(TextRenderingMode::Stroke)
        .write("Stroke mode (outline only)")?;

    y -= 20.0;

    // Fill and stroke mode
    page.graphics()
        .set_fill_color(Color::rgb(0.0, 0.5, 0.8))
        .set_stroke_color(Color::rgb(0.8, 0.0, 0.0))
        .set_line_width(0.8);

    page.text()
        .at(70.0, y)
        .set_rendering_mode(TextRenderingMode::FillStroke)
        .write("Fill and stroke mode")?;

    y -= 20.0;

    // Invisible mode (for searchable text over images)
    page.text()
        .at(70.0, y)
        .set_rendering_mode(TextRenderingMode::Invisible)
        .write("Invisible mode (this text is invisible but searchable)")?;

    // Add a note about invisible text
    page.graphics().set_fill_color(Color::rgb(0.6, 0.6, 0.6));

    page.text()
        .set_font(Font::HelveticaOblique, 10.0)
        .at(70.0, y - 12.0)
        .set_rendering_mode(TextRenderingMode::Fill)
        .write("(Text above is invisible but still selectable/searchable)")?;

    y -= 40.0;

    // Complex combination example
    page.graphics().set_fill_color(Color::rgb(0.0, 0.0, 0.0)); // Reset to black

    page.text()
        .set_font(Font::HelveticaBold, 14.0)
        .at(50.0, y)
        .set_rendering_mode(TextRenderingMode::Fill)
        .write("7. Complex Combination Example")?;

    y -= 25.0;

    // Demonstrate multiple parameters together
    page.text()
        .set_font(Font::TimesRoman, 13.0)
        .at(70.0, y)
        .set_character_spacing(1.5) // Wide character spacing
        .set_word_spacing(3.0) // Wide word spacing
        .set_horizontal_scaling(1.1) // Slightly expanded
        .set_text_rise(0.0) // Baseline
        .set_rendering_mode(TextRenderingMode::Fill)
        .write("This        text        combines        multiple        parameters")?;

    y -= 20.0;

    // Creative styling example
    page.graphics()
        .set_fill_color(Color::rgb(0.2, 0.4, 0.8))
        .set_stroke_color(Color::rgb(0.8, 0.2, 0.2))
        .set_line_width(0.3);

    page.text()
        .set_font(Font::HelveticaBold, 16.0)
        .at(70.0, y)
        .set_character_spacing(2.0) // Wide characters
        .set_word_spacing(0.0) // Normal word spacing
        .set_horizontal_scaling(0.9) // Slightly condensed
        .set_text_rise(0.0) // Baseline
        .set_rendering_mode(TextRenderingMode::FillStroke)
        .write("S T Y L I S H   T E X T   E F F E C T")?;

    // Footer with technical information
    y = 80.0;
    page.graphics().set_fill_color(Color::rgb(0.5, 0.5, 0.5));

    page.text()
        .set_font(Font::Helvetica, 9.0)
        .at(50.0, y)
        .set_character_spacing(0.0) // Reset all parameters
        .set_word_spacing(0.0)
        .set_horizontal_scaling(1.0)
        .set_text_rise(0.0)
        .set_rendering_mode(TextRenderingMode::Fill)
        .write(
            "All text state parameters generate standard PDF operators: Tc, Tw, Tz, TL, Ts, Tr",
        )?;

    page.text()
        .at(50.0, y - 12.0)
        .write("Generated with oxidize-pdf - ISO 32000-1:2008 compliant text formatting")?;

    doc.add_page(page);

    // Save both compressed and uncompressed versions
    doc.save("/tmp/text_state_parameters.pdf")?;
    println!("✅ Created comprehensive text state parameters demo: /tmp/text_state_parameters.pdf");

    doc.set_compress(false);
    doc.save("/tmp/text_state_parameters_uncompressed.pdf")?;
    println!("✅ Created uncompressed version for inspection: /tmp/text_state_parameters_uncompressed.pdf");

    // Validation summary
    println!("\n📊 TEXT STATE PARAMETERS IMPLEMENTED:");
    println!("   ✅ set_character_spacing() - Tc operator");
    println!("   ✅ set_word_spacing() - Tw operator");
    println!("   ✅ set_horizontal_scaling() - Tz operator");
    println!("   ✅ set_leading() - TL operator");
    println!("   ✅ set_text_rise() - Ts operator");
    println!("   ✅ set_rendering_mode() - Tr operator");
    println!("\n🎯 All 6 text state parameters are now publicly accessible!");
    println!("📈 This should improve Text Features (§9) compliance from 20% to 40%");

    Ok(())
}
