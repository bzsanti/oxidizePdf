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

    let font_manager_arc = Arc::new(font_manager);

    // Process markdown content
    let lines: Vec<String> = markdown_content.lines().map(|s| s.to_string()).collect();
    let mut pages = Vec::new();
    let mut current_page = Page::a4();
    let mut y_position = 750.0;

    // Initialize first page
    {
        let graphics = current_page.graphics();
        graphics.set_font_manager(font_manager_arc.clone());
    }

    for line in lines {
        if y_position < 50.0 {
            // Save current page and create new one
            pages.push(current_page);
            current_page = Page::a4();
            y_position = 750.0;

            let graphics = current_page.graphics();
            graphics.set_font_manager(font_manager_arc.clone());
        }

        // Draw content based on markdown format
        let graphics = current_page.graphics();

        if line.starts_with("# ") {
            // H1 heading
            graphics.set_custom_font(&font_name, 20.0);
            graphics.set_fill_color(Color::black());
            graphics.draw_text(&line[2..], 50.0, y_position)?;
            y_position -= 25.0;
        } else if line.starts_with("## ") {
            // H2 heading
            graphics.set_custom_font(&font_name, 16.0);
            graphics.set_fill_color(Color::rgb(0.2, 0.2, 0.2));
            graphics.draw_text(&line[3..], 50.0, y_position)?;
            y_position -= 20.0;
        } else if line.starts_with("### ") {
            // H3 heading
            graphics.set_custom_font(&font_name, 14.0);
            graphics.set_fill_color(Color::rgb(0.3, 0.3, 0.3));
            graphics.draw_text(&line[4..], 50.0, y_position)?;
            y_position -= 18.0;
        } else if line.starts_with("- [ ] ") {
            // Unchecked checkbox
            graphics.set_custom_font(&font_name, 11.0);
            graphics.set_fill_color(Color::black());
            graphics.draw_text("☐", 50.0, y_position)?;

            // Draw the text after the checkbox with word wrapping
            let text = &line[6..];
            draw_wrapped_text(graphics, text, 70.0, &mut y_position, &font_name)?;
        } else if line.starts_with("- [x] ") || line.starts_with("- [X] ") {
            // Checked checkbox
            graphics.set_custom_font(&font_name, 11.0);
            graphics.set_fill_color(Color::black());
            graphics.draw_text("☑", 50.0, y_position)?;

            // Draw the text after the checkbox with word wrapping
            let text = &line[6..];
            draw_wrapped_text(graphics, text, 70.0, &mut y_position, &font_name)?;
        } else if line.starts_with("- ") {
            // Bullet point
            graphics.set_custom_font(&font_name, 11.0);
            graphics.set_fill_color(Color::black());
            graphics.draw_text("•", 50.0, y_position)?;

            // Draw the text after the bullet with word wrapping
            let text = &line[2..];
            draw_wrapped_text(graphics, text, 70.0, &mut y_position, &font_name)?;
        } else if !line.is_empty() {
            // Regular paragraph
            graphics.set_custom_font(&font_name, 11.0);
            graphics.set_fill_color(Color::black());
            draw_wrapped_text(graphics, &line, 50.0, &mut y_position, &font_name)?;
        } else {
            // Empty line
            y_position -= 10.0;
        }
    }

    // Add the last page if it has content
    pages.push(current_page);

    // Add all pages to document
    for page in pages {
        document.add_page(page);
    }

    // Save the PDF
    let output_path = "pyme_ia_checklist.pdf";
    document.save(output_path)?;

    println!("PDF saved to: {}", output_path);
    println!("Please open the PDF to verify the conversion.");

    Ok(())
}

fn draw_wrapped_text(
    graphics: &mut oxidize_pdf::GraphicsContext,
    text: &str,
    x: f64,
    y: &mut f64,
    font_name: &str,
) -> Result<()> {
    // Simple word wrapping
    let max_width = 500.0;
    let char_width = 6.0; // Approximate
    let max_chars = (max_width / char_width) as usize;

    if text.len() > max_chars {
        // Split into words
        let words: Vec<&str> = text.split_whitespace().collect();
        let mut current_line = String::new();

        for word in words {
            if current_line.len() + word.len() + 1 > max_chars {
                // Draw current line and start new one
                if !current_line.is_empty() {
                    graphics.set_custom_font(font_name, 11.0);
                    graphics.draw_text(&current_line, x, *y)?;
                    *y -= 15.0;
                    current_line.clear();
                }
            }

            if !current_line.is_empty() {
                current_line.push(' ');
            }
            current_line.push_str(word);
        }

        // Draw remaining text
        if !current_line.is_empty() {
            graphics.set_custom_font(font_name, 11.0);
            graphics.draw_text(&current_line, x, *y)?;
            *y -= 15.0;
        }
    } else {
        graphics.set_custom_font(font_name, 11.0);
        graphics.draw_text(text, x, *y)?;
        *y -= 15.0;
    }

    Ok(())
}
