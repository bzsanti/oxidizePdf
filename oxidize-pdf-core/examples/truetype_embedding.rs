//! Example of embedding TrueType fonts in PDF documents
//!
//! This example demonstrates how to:
//! - Load a TrueType font from file
//! - Create a PDF with embedded TrueType font
//! - Use font subsetting to reduce file size

use oxidize_pdf::text::fonts::truetype::TrueTypeFont;
use oxidize_pdf::text::{CustomFont, FontManager};
use oxidize_pdf::{Document, Page, Result};
use std::path::Path;

fn main() -> Result<()> {
    // Create a new document
    let mut doc = Document::new();
    doc.set_title("TrueType Font Embedding Example");
    doc.set_author("oxidize-pdf");

    // Create a font manager
    let mut font_manager = FontManager::new();

    // Example: Load a TrueType font (you would provide a real font file)
    let font_path = "path/to/your/font.ttf";

    if Path::new(font_path).exists() {
        // Load TrueType font
        match CustomFont::load_truetype_font(font_path) {
            Ok(mut custom_font) => {
                println!("Loaded font: {}", custom_font.name);

                // Create a page
                let mut page = Page::a4();

                // Register the font
                let font_id = font_manager.register_font(custom_font.clone())?;

                // Use the font to write text
                let text = "Hello, TrueType Fonts! ABCDEFGHIJKLMNOPQRSTUVWXYZ 0123456789";

                // Mark characters as used for subsetting
                custom_font.mark_characters_used(text);

                // Add text to page
                page.text()
                    .set_font(oxidize_pdf::text::Font::Helvetica, 24.0)
                    .at(50.0, 750.0)
                    .write("TrueType Font Embedding Example")?;

                page.text()
                    .set_font(oxidize_pdf::text::Font::Helvetica, 12.0)
                    .at(50.0, 700.0)
                    .write(&format!("Using font: {}", custom_font.name))?;

                // TODO: Once custom font support is fully integrated:
                // page.text()
                //     .set_custom_font(&font_id, 16.0)
                //     .at(50.0, 650.0)
                //     .write(text)?;

                doc.add_page(page);

                // Get subset font data
                if let Some(subset_data) = custom_font.get_subset_font_data()? {
                    println!(
                        "Original font size: {} bytes",
                        custom_font.font_data.as_ref().map(|d| d.len()).unwrap_or(0)
                    );
                    println!("Subset font size: {} bytes", subset_data.len());
                    println!("Used glyphs: {}", custom_font.used_glyphs.len());
                }
            }
            Err(e) => {
                eprintln!("Failed to load font: {}", e);
            }
        }
    } else {
        println!("Font file not found. Please provide a valid TrueType font file.");

        // Demo with built-in fonts
        let mut page = Page::a4();

        page.text()
            .set_font(oxidize_pdf::text::Font::Helvetica, 24.0)
            .at(50.0, 750.0)
            .write("TrueType Font Support")?;

        page.text()
            .set_font(oxidize_pdf::text::Font::Helvetica, 12.0)
            .at(50.0, 700.0)
            .write("oxidize-pdf now supports TrueType font embedding and subsetting!")?;

        page.text().at(50.0, 650.0).write("Features:")?;

        let features = vec![
            "• Parse TrueType/OpenType fonts",
            "• Extract font metrics and character mappings",
            "• Create font subsets with only used glyphs",
            "• Embed fonts in PDF documents",
            "• Support for various cmap formats (0, 4, 6, 12)",
        ];

        let mut y = 620.0;
        for feature in features {
            page.text().at(70.0, y).write(feature)?;
            y -= 20.0;
        }

        doc.add_page(page);
    }

    // Save the document
    doc.save("truetype_embedding_example.pdf")?;
    println!("Created: truetype_embedding_example.pdf");

    Ok(())
}
