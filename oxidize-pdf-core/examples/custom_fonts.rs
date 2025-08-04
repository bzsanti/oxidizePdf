//! Example demonstrating custom font loading and usage in PDF generation
//!
//! This example shows how to:
//! - Load custom TrueType/OpenType fonts
//! - Use them alongside standard PDF fonts
//! - Measure text with custom fonts
//! - Handle font embedding

use oxidize_pdf::{Document, Page, Font, Color, Result};

fn main() -> Result<()> {
    // Create a new document
    let mut doc = Document::new();
    doc.set_title("Custom Fonts Example");
    doc.set_author("oxidize-pdf");
    
    // For this example, we'll create a dummy font
    // In real usage, you would load an actual TTF/OTF file
    let font_data = create_sample_font_data();
    
    // Add custom fonts to the document
    doc.add_font_from_bytes("MyCustomFont", font_data.clone())?;
    doc.add_font_from_bytes("MyCustomFontBold", font_data)?;
    
    // You can also load from file:
    // doc.add_font("Arial", "/path/to/arial.ttf")?;
    
    // Create a page showcasing different fonts
    let mut page = Page::a4();
    
    // Title with standard font
    page.graphics()
        .set_fill_color(Color::rgb(0.2, 0.2, 0.8));
    
    page.text()
        .set_font(Font::HelveticaBold, 24.0)
        .at(50.0, 750.0)
        .write("Custom Fonts in oxidize-pdf")?;
    
    // Subtitle
    page.graphics()
        .set_fill_color(Color::rgb(0.4, 0.4, 0.4));
    
    page.text()
        .set_font(Font::Helvetica, 12.0)
        .at(50.0, 720.0)
        .write("Mixing standard and custom fonts in your PDFs")?;
    
    // Section 1: Standard fonts
    page.graphics()
        .set_fill_color(Color::BLACK);
    
    page.text()
        .set_font(Font::HelveticaBold, 16.0)
        .at(50.0, 650.0)
        .write("Standard PDF Fonts:")?;
    
    let standard_fonts = [
        ("Helvetica", Font::Helvetica),
        ("Times Roman", Font::TimesRoman),
        ("Courier", Font::Courier),
    ];
    
    let mut y = 620.0;
    for (name, font) in &standard_fonts {
        page.text()
            .set_font(font.clone(), 12.0)
            .at(70.0, y)
            .write(&format!("{}: The quick brown fox jumps over the lazy dog", name))?;
        y -= 25.0;
    }
    
    // Section 2: Custom fonts
    page.text()
        .set_font(Font::HelveticaBold, 16.0)
        .at(50.0, 500.0)
        .write("Custom Loaded Fonts:")?;
    
    // Use custom font
    page.text()
        .set_font(Font::custom("MyCustomFont"), 12.0)
        .at(70.0, 470.0)
        .write("MyCustomFont: This text uses a custom loaded font!")?;
    
    page.text()
        .set_font(Font::custom("MyCustomFontBold"), 14.0)
        .at(70.0, 440.0)
        .write("MyCustomFontBold: Custom fonts can be any size!")?;
    
    // Section 3: Font features demonstration
    page.text()
        .set_font(Font::HelveticaBold, 16.0)
        .at(50.0, 380.0)
        .write("Font Features:")?;
    
    // Character spacing with custom font
    page.text()
        .set_font(Font::custom("MyCustomFont"), 12.0)
        .set_character_spacing(2.0)
        .at(70.0, 350.0)
        .write("Character spacing example")?;
    
    // Word spacing
    page.text()
        .set_font(Font::custom("MyCustomFont"), 12.0)
        .set_word_spacing(10.0)
        .at(70.0, 320.0)
        .write("Word spacing example with custom font")?;
    
    // Text rendering modes
    page.text()
        .set_font(Font::custom("MyCustomFontBold"), 18.0)
        .set_rendering_mode(oxidize_pdf::text::TextRenderingMode::FillStroke)
        .at(70.0, 280.0)
        .write("Outlined Text Effect")?;
    
    // Section 4: Mixed fonts in one line (multiple text objects)
    page.text()
        .set_font(Font::HelveticaBold, 16.0)
        .at(50.0, 220.0)
        .write("Mixed Fonts:")?;
    
    // Combine different fonts
    page.text()
        .set_font(Font::Helvetica, 12.0)
        .at(70.0, 190.0)
        .write("This is ")?;
    
    page.text()
        .set_font(Font::custom("MyCustomFont"), 12.0)
        .at(115.0, 190.0)
        .write("custom font")?;
    
    page.text()
        .set_font(Font::Helvetica, 12.0)
        .at(185.0, 190.0)
        .write(" mixed with ")?;
    
    page.text()
        .set_font(Font::HelveticaBold, 12.0)
        .at(250.0, 190.0)
        .write("standard bold")?;
    
    page.text()
        .set_font(Font::Helvetica, 12.0)
        .at(325.0, 190.0)
        .write(" text.")?;
    
    // Add information about loaded fonts
    page.text()
        .set_font(Font::Helvetica, 10.0)
        .at(50.0, 100.0)
        .write("Loaded custom fonts:")?;
    
    let font_names = doc.custom_font_names();
    let mut y = 80.0;
    for name in font_names {
        page.text()
            .set_font(Font::Helvetica, 10.0)
            .at(70.0, y)
            .write(&format!("• {}", name))?;
        y -= 15.0;
    }
    
    // Add the page to the document
    doc.add_page(page);
    
    // Create a second page with more advanced usage
    let mut page2 = Page::a4();
    
    page2.text()
        .set_font(Font::HelveticaBold, 20.0)
        .at(50.0, 750.0)
        .write("Advanced Custom Font Usage")?;
    
    // Demonstrate text measurement (once implemented)
    page2.text()
        .set_font(Font::Helvetica, 12.0)
        .at(50.0, 700.0)
        .write("Custom fonts support all PDF text features:")?;
    
    // Text rise (superscript/subscript)
    page2.text()
        .set_font(Font::custom("MyCustomFont"), 12.0)
        .at(50.0, 650.0)
        .write("Normal text with ")?;
    
    page2.text()
        .set_font(Font::custom("MyCustomFont"), 8.0)
        .set_text_rise(5.0)
        .at(150.0, 650.0)
        .write("superscript")?;
    
    page2.text()
        .set_font(Font::custom("MyCustomFont"), 12.0)
        .set_text_rise(0.0)
        .at(200.0, 650.0)
        .write(" and ")?;
    
    page2.text()
        .set_font(Font::custom("MyCustomFont"), 8.0)
        .set_text_rise(-3.0)
        .at(230.0, 650.0)
        .write("subscript")?;
    
    // Horizontal scaling
    page2.text()
        .set_font(Font::custom("MyCustomFont"), 14.0)
        .set_horizontal_scaling(150.0)
        .at(50.0, 600.0)
        .write("Stretched text")?;
    
    page2.text()
        .set_font(Font::custom("MyCustomFont"), 14.0)
        .set_horizontal_scaling(50.0)
        .at(50.0, 570.0)
        .write("Compressed text")?;
    
    doc.add_page(page2);
    
    // Save the document
    doc.save("custom_fonts_example.pdf")?;
    println!("PDF saved as 'custom_fonts_example.pdf'");
    
    // Also demonstrate saving to memory
    let pdf_bytes = doc.to_bytes()?;
    println!("PDF size with embedded fonts: {} bytes", pdf_bytes.len());
    
    Ok(())
}

/// Create minimal sample font data for demonstration
/// In real usage, you would load actual font files
fn create_sample_font_data() -> Vec<u8> {
    // This creates a minimal valid TTF structure
    // Real fonts would have actual glyph data, metrics, etc.
    let mut data = Vec::new();
    
    // TTF header
    data.extend_from_slice(&[0x00, 0x01, 0x00, 0x00]); // version
    data.extend_from_slice(&[0x00, 0x05]); // numTables
    data.extend_from_slice(&[0x00, 0x80]); // searchRange
    data.extend_from_slice(&[0x00, 0x02]); // entrySelector
    data.extend_from_slice(&[0x00, 0x30]); // rangeShift
    
    // Basic table directory
    // head table
    data.extend_from_slice(b"head");
    data.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]); // checksum
    data.extend_from_slice(&[0x00, 0x00, 0x00, 0x80]); // offset
    data.extend_from_slice(&[0x00, 0x00, 0x00, 0x36]); // length
    
    // hhea table
    data.extend_from_slice(b"hhea");
    data.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]); // checksum
    data.extend_from_slice(&[0x00, 0x00, 0x00, 0xB6]); // offset
    data.extend_from_slice(&[0x00, 0x00, 0x00, 0x24]); // length
    
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
    
    // Pad to offset
    while data.len() < 0x80 {
        data.push(0);
    }
    
    // Minimal head table
    data.extend_from_slice(&[0x00, 0x01, 0x00, 0x00]); // version
    data.extend_from_slice(&[0x00, 0x01, 0x00, 0x00]); // fontRevision
    data.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]); // checkSumAdjustment
    data.extend_from_slice(&[0x5F, 0x0F, 0x3C, 0xF5]); // magicNumber
    data.extend_from_slice(&[0x00, 0x00]); // flags
    data.extend_from_slice(&[0x03, 0xE8]); // unitsPerEm (1000)
    
    // Add remaining minimal data
    while data.len() < 1000 {
        data.push(0);
    }
    
    data
}