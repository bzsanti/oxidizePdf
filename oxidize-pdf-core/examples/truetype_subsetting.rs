//! Example demonstrating TrueType font subsetting concepts
//!
//! This example shows:
//! - What font subsetting is and why it matters
//! - How glyphs are mapped to characters
//! - Size reduction benefits of subsetting
//! - Educational visualization of subsetting process
//!
//! Note: This is an educational example showing font subsetting concepts.
//! The actual subsetting is handled internally by oxidize-pdf when embedding fonts.

use oxidize_pdf::error::Result;
use oxidize_pdf::{Color, Document, Font, Page};
use std::fs;

fn main() -> Result<()> {
    println!("Creating TrueType Font Subsetting educational example...");

    // Create a new document
    let mut doc = Document::new();
    doc.set_title("TrueType Font Subsetting Example");
    doc.set_author("Oxidize PDF");

    // Create demonstration pages
    create_subsetting_intro_page(&mut doc)?;
    create_size_comparison_page(&mut doc)?;
    create_glyph_mapping_page(&mut doc)?;

    // Save the document
    let output = "examples/results/truetype_subsetting.pdf";
    fs::create_dir_all("examples/results")?;
    doc.save(output)?;

    println!("Created {}", output);

    // Print educational summary
    print_subsetting_summary();

    Ok(())
}

/// Create a page explaining font subsetting
fn create_subsetting_intro_page(doc: &mut Document) -> Result<()> {
    let mut page = Page::a4();

    // Title
    page.text()
        .set_font(Font::HelveticaBold, 24.0)
        .at(50.0, 780.0)
        .write("TrueType Font Subsetting")?;

    page.text()
        .set_font(Font::Helvetica, 12.0)
        .at(50.0, 755.0)
        .write("Reducing PDF file size by including only used glyphs")?;

    // What is font subsetting section
    let mut y = 710.0;
    page.text()
        .set_font(Font::HelveticaBold, 14.0)
        .at(50.0, y)
        .write("What is Font Subsetting?")?;

    y -= 25.0;
    let intro_text = [
        "Font subsetting is the process of creating a smaller version of a font",
        "that contains only the glyphs (characters) actually used in a document.",
        "",
        "A typical TrueType font contains thousands of glyphs:",
        "  - Arial: ~3,300 glyphs",
        "  - Times New Roman: ~3,400 glyphs",
        "  - Full Unicode fonts: ~65,000+ glyphs",
        "",
        "Most documents use only 50-200 unique characters!",
    ];

    page.text().set_font(Font::Helvetica, 11.0);
    for line in &intro_text {
        page.text().at(60.0, y).write(line)?;
        y -= 15.0;
    }

    // Benefits section
    y -= 20.0;
    page.text()
        .set_font(Font::HelveticaBold, 14.0)
        .at(50.0, y)
        .write("Benefits of Font Subsetting")?;

    y -= 25.0;

    // Draw benefit boxes
    let benefits = [
        ("File Size", "50-95% smaller fonts"),
        ("Speed", "Faster download/render"),
        ("Bandwidth", "Lower network usage"),
        ("Quality", "Full font fidelity"),
    ];

    for (i, (title, desc)) in benefits.iter().enumerate() {
        let x = 50.0 + (i as f64 * 130.0);

        // Box background
        page.graphics()
            .set_fill_color(Color::rgb(0.9, 0.95, 1.0))
            .set_stroke_color(Color::rgb(0.3, 0.5, 0.8))
            .set_line_width(1.0)
            .rect(x, y - 45.0, 120.0, 50.0)
            .fill_stroke();

        // Title
        page.text()
            .set_font(Font::HelveticaBold, 10.0)
            .at(x + 10.0, y - 10.0)
            .write(title)?;

        // Description
        page.text()
            .set_font(Font::Helvetica, 9.0)
            .at(x + 10.0, y - 30.0)
            .write(desc)?;
    }

    // Example section
    y -= 100.0;
    page.text()
        .set_font(Font::HelveticaBold, 14.0)
        .at(50.0, y)
        .write("Example: \"Hello World!\"")?;

    y -= 25.0;
    page.text()
        .set_font(Font::Helvetica, 11.0)
        .at(60.0, y)
        .write("This simple phrase uses only 10 unique characters:")?;

    y -= 20.0;

    // Show unique characters
    let unique_chars = get_unique_characters("Hello World!");
    page.text()
        .set_font(Font::Courier, 14.0)
        .at(80.0, y)
        .write(&format!("{:?}", unique_chars))?;

    y -= 30.0;
    page.text()
        .set_font(Font::Helvetica, 11.0)
        .at(60.0, y)
        .write("Full Arial font: 3,300 glyphs (756 KB)")?;

    y -= 15.0;
    page.text()
        .at(60.0, y)
        .write("Subset for \"Hello World!\": 10 glyphs (~8 KB)")?;

    y -= 15.0;
    page.text()
        .set_font(Font::HelveticaBold, 11.0)
        .at(60.0, y)
        .write("Size reduction: 98.9%")?;

    // Visual bar comparison
    y -= 40.0;
    page.text()
        .set_font(Font::Helvetica, 10.0)
        .at(60.0, y)
        .write("Full font:")?;

    page.graphics()
        .set_fill_color(Color::rgb(0.8, 0.2, 0.2))
        .rect(140.0, y - 5.0, 350.0, 15.0)
        .fill();

    y -= 25.0;
    page.text().at(60.0, y).write("Subset:")?;

    page.graphics()
        .set_fill_color(Color::rgb(0.2, 0.7, 0.3))
        .rect(140.0, y - 5.0, 4.0, 15.0)
        .fill();

    doc.add_page(page);
    Ok(())
}

/// Create a page showing size comparisons
fn create_size_comparison_page(doc: &mut Document) -> Result<()> {
    let mut page = Page::a4();

    // Title
    page.text()
        .set_font(Font::HelveticaBold, 18.0)
        .at(50.0, 780.0)
        .write("Font Size Comparison")?;

    // Create comparison table
    let comparisons = [
        ("Arial", 756_072, 23_456, 3381, 52),
        ("Times New Roman", 934_556, 31_234, 3381, 76),
        ("Courier New", 652_432, 18_765, 2665, 43),
        ("Calibri", 1_234_567, 45_678, 3053, 94),
    ];

    let mut y = 720.0;

    // Table headers
    page.text()
        .set_font(Font::HelveticaBold, 11.0)
        .at(50.0, y)
        .write("Font Name")?;
    page.text().at(180.0, y).write("Original")?;
    page.text().at(270.0, y).write("Subset")?;
    page.text().at(350.0, y).write("Reduction")?;
    page.text().at(430.0, y).write("Glyphs")?;

    // Draw header line
    y -= 5.0;
    page.graphics()
        .set_stroke_color(Color::gray(0.3))
        .set_line_width(0.5)
        .move_to(50.0, y)
        .line_to(520.0, y)
        .stroke();

    y -= 20.0;
    page.text().set_font(Font::Helvetica, 10.0);

    for (name, original, subset, glyphs_orig, glyphs_sub) in &comparisons {
        page.text().at(50.0, y).write(name)?;
        page.text().at(180.0, y).write(&format_bytes(*original))?;
        page.text().at(270.0, y).write(&format_bytes(*subset))?;

        let reduction = calculate_reduction(*original, *subset);
        page.text()
            .at(350.0, y)
            .write(&format!("{:.1}%", reduction))?;
        page.text()
            .at(430.0, y)
            .write(&format!("{}/{}", glyphs_sub, glyphs_orig))?;

        y -= 20.0;
    }

    // Visual bar chart
    y -= 30.0;
    page.text()
        .set_font(Font::HelveticaBold, 14.0)
        .at(50.0, y)
        .write("Visual Size Comparison")?;

    y -= 30.0;
    let max_width = 300.0;
    let max_size = 1_234_567.0;

    for (name, original, subset, _, _) in &comparisons {
        // Font name
        page.text()
            .set_font(Font::Helvetica, 9.0)
            .at(50.0, y + 12.0)
            .write(name)?;

        // Original bar (red)
        let orig_width = (*original as f64 / max_size) * max_width;
        page.graphics()
            .set_fill_color(Color::rgb(0.85, 0.3, 0.3))
            .rect(150.0, y + 10.0, orig_width, 12.0)
            .fill();

        // Subset bar (green)
        let subset_width = (*subset as f64 / max_size) * max_width;
        page.graphics()
            .set_fill_color(Color::rgb(0.3, 0.75, 0.35))
            .rect(150.0, y - 5.0, subset_width, 12.0)
            .fill();

        y -= 40.0;
    }

    // Legend
    y -= 10.0;
    page.graphics()
        .set_fill_color(Color::rgb(0.85, 0.3, 0.3))
        .rect(150.0, y, 20.0, 12.0)
        .fill();
    page.text()
        .set_font(Font::Helvetica, 10.0)
        .at(175.0, y + 2.0)
        .write("Original Font")?;

    page.graphics()
        .set_fill_color(Color::rgb(0.3, 0.75, 0.35))
        .rect(280.0, y, 20.0, 12.0)
        .fill();
    page.text().at(305.0, y + 2.0).write("Subset Font")?;

    // Key insight
    y -= 50.0;
    page.graphics()
        .set_fill_color(Color::rgb(1.0, 0.98, 0.9))
        .set_stroke_color(Color::rgb(0.8, 0.7, 0.3))
        .set_line_width(1.5)
        .rect(50.0, y - 50.0, 500.0, 60.0)
        .fill_stroke();

    page.text()
        .set_font(Font::HelveticaBold, 12.0)
        .at(60.0, y - 10.0)
        .write("Key Insight")?;

    page.text()
        .set_font(Font::Helvetica, 10.0)
        .at(60.0, y - 30.0)
        .write("Average document uses <100 unique characters = 97%+ size reduction!")?;

    doc.add_page(page);
    Ok(())
}

/// Create a page showing glyph mapping
fn create_glyph_mapping_page(doc: &mut Document) -> Result<()> {
    let mut page = Page::a4();

    // Title
    page.text()
        .set_font(Font::HelveticaBold, 18.0)
        .at(50.0, 780.0)
        .write("Glyph Mapping Process")?;

    page.text()
        .set_font(Font::Helvetica, 11.0)
        .at(50.0, 755.0)
        .write("How characters become glyphs in the subset font")?;

    // Step-by-step process
    let mut y = 710.0;
    page.text()
        .set_font(Font::HelveticaBold, 14.0)
        .at(50.0, y)
        .write("The Subsetting Process")?;

    y -= 30.0;

    let steps = [
        ("1. Scan Document", "Identify all unique characters used"),
        (
            "2. Map to Unicode",
            "Convert characters to Unicode code points",
        ),
        ("3. Find Glyphs", "Look up glyph IDs in font's cmap table"),
        ("4. Extract Glyphs", "Copy only needed glyphs to new font"),
        ("5. Create Mapping", "Build new character-to-glyph mapping"),
        ("6. Embed Font", "Include subset font in PDF"),
    ];

    for (step, desc) in &steps {
        // Step box
        page.graphics()
            .set_fill_color(Color::rgb(0.2, 0.4, 0.7))
            .rect(60.0, y - 15.0, 130.0, 25.0)
            .fill();

        page.text()
            .set_font(Font::HelveticaBold, 10.0)
            .at(70.0, y - 5.0)
            .write(step)?;

        page.text()
            .set_font(Font::Helvetica, 10.0)
            .at(200.0, y - 5.0)
            .write(desc)?;

        y -= 35.0;
    }

    // Character to glyph mapping example
    y -= 20.0;
    page.text()
        .set_font(Font::HelveticaBold, 14.0)
        .at(50.0, y)
        .write("Example: Character to Glyph Mapping")?;

    y -= 25.0;

    // Headers
    page.text()
        .set_font(Font::HelveticaBold, 10.0)
        .at(60.0, y)
        .write("Char")?;
    page.text().at(120.0, y).write("Unicode")?;
    page.text().at(200.0, y).write("Original GID")?;
    page.text().at(300.0, y).write("Subset GID")?;

    y -= 5.0;
    page.graphics()
        .set_stroke_color(Color::gray(0.5))
        .set_line_width(0.5)
        .move_to(60.0, y)
        .line_to(380.0, y)
        .stroke();

    y -= 20.0;

    let mappings = [
        ('H', 0x0048, 43, 1),
        ('e', 0x0065, 72, 2),
        ('l', 0x006C, 79, 3),
        ('o', 0x006F, 82, 4),
        (' ', 0x0020, 3, 5),
        ('W', 0x0057, 58, 6),
        ('r', 0x0072, 85, 7),
        ('d', 0x0064, 71, 8),
        ('!', 0x0021, 4, 9),
    ];

    page.text().set_font(Font::Courier, 10.0);

    for (ch, unicode, orig_gid, new_gid) in &mappings {
        let char_display = if *ch == ' ' {
            "' '"
        } else {
            &format!("'{}'", ch)
        };
        page.text().at(60.0, y).write(char_display)?;
        page.text()
            .at(120.0, y)
            .write(&format!("U+{:04X}", unicode))?;
        page.text().at(220.0, y).write(&format!("{}", orig_gid))?;
        page.text().at(320.0, y).write(&format!("{}", new_gid))?;

        y -= 16.0;
    }

    // Note at bottom
    y -= 30.0;
    page.text()
        .set_font(Font::Helvetica, 10.0)
        .at(50.0, y)
        .write("Note: In subset fonts, glyph IDs are renumbered sequentially")?;

    y -= 15.0;
    page.text()
        .at(50.0, y)
        .write("starting from 0 (.notdef) to minimize file size.")?;

    // oxidize-pdf note
    y -= 40.0;
    page.graphics()
        .set_fill_color(Color::rgb(0.95, 0.95, 0.95))
        .set_stroke_color(Color::gray(0.6))
        .set_line_width(1.0)
        .rect(50.0, y - 40.0, 500.0, 50.0)
        .fill_stroke();

    page.text()
        .set_font(Font::HelveticaBold, 11.0)
        .at(60.0, y - 10.0)
        .write("oxidize-pdf Font Handling")?;

    page.text()
        .set_font(Font::Helvetica, 10.0)
        .at(60.0, y - 28.0)
        .write("Font subsetting is handled automatically when embedding TrueType fonts.")?;

    doc.add_page(page);
    Ok(())
}

/// Get unique characters from text
fn get_unique_characters(text: &str) -> Vec<char> {
    let mut chars: Vec<char> = text.chars().collect();
    chars.sort_unstable();
    chars.dedup();
    chars
}

/// Format bytes to human-readable string
fn format_bytes(bytes: usize) -> String {
    if bytes < 1024 {
        format!("{} B", bytes)
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{:.2} MB", bytes as f64 / (1024.0 * 1024.0))
    }
}

/// Calculate percentage reduction
fn calculate_reduction(original: usize, subset: usize) -> f64 {
    if original == 0 {
        0.0
    } else {
        100.0 - (subset as f64 / original as f64 * 100.0)
    }
}

/// Print educational summary to console
fn print_subsetting_summary() {
    println!("\n=== Font Subsetting Benefits ===");
    println!("1. Dramatically reduces PDF file size (50-98%)");
    println!("2. Faster PDF download and rendering");
    println!("3. Lower bandwidth usage");
    println!("4. Maintains full font quality");
    println!("5. Supports all Unicode characters used");
    println!("\noxidize-pdf handles font subsetting automatically!");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unique_characters() {
        let chars = get_unique_characters("Hello");
        assert_eq!(chars.len(), 4); // H, e, l, o
    }

    #[test]
    fn test_format_bytes() {
        assert_eq!(format_bytes(500), "500 B");
        assert_eq!(format_bytes(2048), "2.0 KB");
        assert_eq!(format_bytes(1_048_576), "1.00 MB");
    }

    #[test]
    fn test_reduction() {
        let reduction = calculate_reduction(1000, 100);
        assert!((reduction - 90.0).abs() < 0.1);
    }
}
