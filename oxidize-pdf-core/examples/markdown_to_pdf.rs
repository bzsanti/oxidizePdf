//! Simple Markdown to PDF converter with Unicode support

use oxidize_pdf::{Color, CustomFont, Document, FontManager, Page, Result};
use std::sync::Arc;

fn main() -> Result<()> {
    // Read the markdown file
    let markdown_path = "/Users/santifdezmunoz/Downloads/pyme_ia_readiness_checklist.md";
    let markdown_content =
        std::fs::read_to_string(markdown_path).expect("Failed to read markdown file");

    println!("Converting Markdown to PDF with Unicode support...");

    // Create document
    let mut document = Document::new();
    document.set_title("PYME IA Readiness Checklist");
    document.set_author("Oxidize PDF");

    // Load Unicode-capable font
    let mut font_manager = FontManager::new();

    // Try to load Arial Unicode font
    let font_path = "/System/Library/Fonts/Supplemental/Arial Unicode.ttf";
    if !std::path::Path::new(font_path).exists() {
        eprintln!("Arial Unicode font not found at: {}", font_path);
        return Err(oxidize_pdf::PdfError::InvalidStructure(
            "Required font not available".to_string(),
        ));
    }

    let font = CustomFont::load_truetype_font(font_path)?;
    let font_name = font_manager.register_font(font)?;
    println!("Font registered as: {}", font_name);

    // Create pages from markdown content
    let mut page = Page::a4();
    {
        let graphics = page.graphics();
        graphics.set_font_manager(Arc::new(font_manager.clone()));
        graphics.set_custom_font(&font_name, 11.0);
        graphics.set_fill_color(Color::black());
    }

    // Simple markdown parsing and rendering
    let lines = markdown_content.lines();
    let mut y_position = 750.0;

    for line in lines {
        if y_position < 50.0 {
            // Add current page and create new one
            document.add_page(page);
            page = Page::a4();
            let graphics = page.graphics();
            graphics.set_font_manager(Arc::new(font_manager.clone()));
            graphics.set_custom_font(&font_name, 11.0);
            graphics.set_fill_color(Color::black());
            y_position = 750.0;
        }

        let graphics = page.graphics();

        // Simple markdown handling
        if line.starts_with("# ") {
            // H1 heading
            graphics.set_custom_font(&font_name, 20.0);
            graphics.set_fill_color(Color::black());
            graphics.draw_text_hex(&line[2..], 50.0, y_position)?;
            y_position -= 25.0;
        } else if line.starts_with("## ") {
            // H2 heading
            graphics.set_custom_font(&font_name, 16.0);
            graphics.set_fill_color(Color::rgb(0.2, 0.2, 0.2));
            graphics.draw_text_hex(&line[3..], 50.0, y_position)?;
            y_position -= 20.0;
        } else if line.starts_with("### ") {
            // H3 heading
            graphics.set_custom_font(&font_name, 14.0);
            graphics.set_fill_color(Color::rgb(0.3, 0.3, 0.3));
            graphics.draw_text_hex(&line[4..], 50.0, y_position)?;
            y_position -= 18.0;
        } else if line.starts_with("- [ ] ") {
            // Unchecked checkbox
            graphics.set_custom_font(&font_name, 11.0);
            graphics.set_fill_color(Color::black());
            graphics.draw_text_hex("☐", 50.0, y_position)?;
            graphics.draw_text_hex(&line[6..], 70.0, y_position)?;
            y_position -= 15.0;
        } else if line.starts_with("- [x] ") || line.starts_with("- [X] ") {
            // Checked checkbox
            graphics.set_custom_font(&font_name, 11.0);
            graphics.set_fill_color(Color::black());
            graphics.draw_text_hex("☑", 50.0, y_position)?;
            graphics.draw_text_hex(&line[6..], 70.0, y_position)?;
            y_position -= 15.0;
        } else if line.starts_with("- ") {
            // Bullet point
            graphics.set_custom_font(&font_name, 11.0);
            graphics.set_fill_color(Color::black());
            graphics.draw_text_hex("•", 50.0, y_position)?;
            graphics.draw_text_hex(&line[2..], 70.0, y_position)?;
            y_position -= 15.0;
        } else if line.starts_with("**") && line.ends_with("**") && line.len() > 4 {
            // Bold text
            graphics.set_custom_font(&font_name, 11.0);
            graphics.set_fill_color(Color::black());
            let text = &line[2..line.len() - 2];
            graphics.draw_text_hex(text, 50.0, y_position)?;
            y_position -= 15.0;
        } else if !line.is_empty() {
            // Regular paragraph
            graphics.set_custom_font(&font_name, 11.0);
            graphics.set_fill_color(Color::black());

            // Simple word wrapping
            let max_width = 500.0;
            let char_width = 6.0; // Approximate
            let max_chars = (max_width / char_width) as usize;

            if line.len() > max_chars {
                let mut remaining = line;
                while !remaining.is_empty() {
                    // Find last whitespace within max_chars
                    let chunk_end = remaining
                        .char_indices()
                        .take_while(|(i, _)| *i < max_chars)
                        .filter(|(_, c)| c.is_whitespace())
                        .last()
                        .map(|(i, _)| i)
                        .unwrap_or_else(|| remaining.len().min(max_chars));

                    let chunk = &remaining[..chunk_end];
                    graphics.draw_text_hex(chunk, 50.0, y_position)?;
                    y_position -= 15.0;

                    remaining = remaining[chunk_end..].trim_start();

                    // TODO: Fix page break handling
                    // if y_position < 50.0 {
                    //     break; // Exit loop to handle page change outside
                    // }
                }
            } else {
                graphics.draw_text_hex(line, 50.0, y_position)?;
                y_position -= 15.0;
            }
        } else {
            // Empty line
            y_position -= 10.0;
        }
    }

    // Add the last page
    document.add_page(page);

    // Save the PDF
    let output_path = "pyme_ia_checklist.pdf";
    document.save(output_path)?;

    println!("PDF saved to: {}", output_path);
    println!("Please open the PDF to verify the conversion.");

    Ok(())
}
