//! Professional Unicode Demo - Practical symbols for real-world PDFs
//!
//! Demonstrates oxidize-pdf's Unicode capabilities for business documents,
//! technical reports, and international content.

use oxidize_pdf::{Color, Document, Page, Result};

fn main() -> Result<()> {
    println!("═══════════════════════════════════════════════");
    println!("  Oxidize-PDF Professional Unicode Demo");
    println!("═══════════════════════════════════════════════\n");

    // Create document with metadata
    let mut document = Document::new();
    document.set_title("Professional Unicode Capabilities");
    document.set_author("Oxidize-PDF Team");
    document.set_creator("Oxidize-PDF v1.1.7");
    document.set_keywords("Unicode, Professional, Business, Technical");

    // Try Arial Unicode first (has all symbols), fallback to Arial
    let unicode_font_path = "/System/Library/Fonts/Supplemental/Arial Unicode.ttf";
    let arial_font_path = "/System/Library/Fonts/Supplemental/Arial.ttf";

    let font_path = if std::path::Path::new(unicode_font_path).exists() {
        println!("Loading Arial Unicode font for complete symbol support...");
        unicode_font_path
    } else if std::path::Path::new(arial_font_path).exists() {
        println!("Loading Arial font (some symbols may not render)...");
        arial_font_path
    } else {
        eprintln!("No suitable font found");
        return Ok(());
    };

    document.add_font("Arial", font_path)?;

    // Create main page
    let mut page = Page::new(612.0, 792.0);
    let graphics = page.graphics();

    // Title
    graphics.set_custom_font("Arial", 28.0);
    graphics.set_fill_color(Color::rgb(0.0, 0.2, 0.4));
    graphics.draw_text("Oxidize-PDF Professional Unicode", 50.0, 720.0)?;

    graphics.set_custom_font("Arial", 12.0);
    graphics.set_fill_color(Color::rgb(0.3, 0.3, 0.3));
    graphics.draw_text(
        "The Rust PDF library for production applications",
        50.0,
        695.0,
    )?;

    // Reset to black for content
    graphics.set_fill_color(Color::black());

    let mut y = 650.0;

    // Section 1: Business Essentials
    graphics.set_custom_font("Arial", 16.0);
    graphics.set_fill_color(Color::rgb(0.0, 0.3, 0.6));
    graphics.draw_text("► Business Essentials", 50.0, y)?;
    graphics.set_fill_color(Color::black());
    y -= 30.0;

    graphics.set_custom_font("Arial", 12.0);

    // Currencies
    graphics.draw_text("Currencies:", 70.0, y)?;
    y -= 20.0;
    graphics.draw_text("  • USD: $1,234.56  EUR: €987.65  GBP: £456.78", 70.0, y)?;
    y -= 18.0;
    graphics.draw_text("  • JPY: ¥12,345  CNY: ¥876.54  INR: ₹5,432", 70.0, y)?;
    y -= 25.0;

    // Legal symbols
    graphics.draw_text("Legal & Copyright:", 70.0, y)?;
    y -= 20.0;
    graphics.draw_text("  • Copyright © 2024 • Registered ® • Trademark™", 70.0, y)?;
    y -= 18.0;
    graphics.draw_text("  • Section § 3.2 • Paragraph ¶ • Reference †", 70.0, y)?;
    y -= 25.0;

    // Checkmarks and status
    graphics.draw_text("Status Indicators:", 70.0, y)?;
    y -= 20.0;
    graphics.draw_text("  ☐ Pending  ☑ Completed  ☒ Cancelled", 70.0, y)?;
    y -= 18.0;
    graphics.draw_text("  ✓ Approved  ✗ Rejected  ⚠ Warning", 70.0, y)?;
    y -= 35.0;

    // Section 2: Technical Documentation
    graphics.set_custom_font("Arial", 16.0);
    graphics.set_fill_color(Color::rgb(0.0, 0.3, 0.6));
    graphics.draw_text("► Technical Documentation", 50.0, y)?;
    graphics.set_fill_color(Color::black());
    y -= 30.0;

    graphics.set_custom_font("Arial", 12.0);

    // Math and Science
    graphics.draw_text("Mathematics:", 70.0, y)?;
    y -= 20.0;
    graphics.draw_text(
        "  • Basic: 2 + 2 = 4, 10 − 5 = 5, 3 × 4 = 12, 8 ÷ 2 = 4",
        70.0,
        y,
    )?;
    y -= 18.0;
    graphics.draw_text("  • Advanced: x² + y² = r², ∑(i=1→n) = n(n+1)/2", 70.0, y)?;
    y -= 18.0;
    graphics.draw_text("  • Comparisons: a ≤ b, x ≥ y, p ≈ 3.14, A ≠ B", 70.0, y)?;
    y -= 25.0;

    // Arrows and directions
    graphics.draw_text("Navigation & Flow:", 70.0, y)?;
    y -= 20.0;
    graphics.draw_text("  • Directions: ← Left  → Right  ↑ Up  ↓ Down", 70.0, y)?;
    y -= 18.0;
    graphics.draw_text("  • Process: Step 1 → Step 2 → Step 3 → Complete", 70.0, y)?;
    y -= 25.0;

    // Fractions and percentages
    graphics.draw_text("Measurements:", 70.0, y)?;
    y -= 20.0;
    graphics.draw_text("  • Fractions: ½ cup, ¼ teaspoon, ¾ complete", 70.0, y)?;
    y -= 18.0;
    graphics.draw_text(
        "  • Temperature: 20°C = 68°F, Water boils at 100°C",
        70.0,
        y,
    )?;
    y -= 35.0;

    // Section 3: International Support
    graphics.set_custom_font("Arial", 16.0);
    graphics.set_fill_color(Color::rgb(0.0, 0.3, 0.6));
    graphics.draw_text("► International Support", 50.0, y)?;
    graphics.set_fill_color(Color::black());
    y -= 30.0;

    graphics.set_custom_font("Arial", 12.0);

    // European languages
    graphics.draw_text("European Languages:", 70.0, y)?;
    y -= 20.0;
    graphics.draw_text("  • Spanish: ñ, á, é, í, ó, ú, ü, ¿, ¡", 70.0, y)?;
    y -= 18.0;
    graphics.draw_text("  • French: à, è, é, ê, ë, ç, œ, æ", 70.0, y)?;
    y -= 18.0;
    graphics.draw_text("  • German: ä, ö, ü, ß, Ä, Ö, Ü", 70.0, y)?;
    y -= 18.0;
    graphics.draw_text("  • Portuguese: ã, õ, â, ê, ô, ç", 70.0, y)?;
    y -= 35.0;

    // Section 4: Why Choose Oxidize-PDF?
    graphics.set_custom_font("Arial", 16.0);
    graphics.set_fill_color(Color::rgb(0.0, 0.3, 0.6));
    graphics.draw_text("► Why Choose Oxidize-PDF?", 50.0, y)?;
    graphics.set_fill_color(Color::black());
    y -= 30.0;

    graphics.set_custom_font("Arial", 12.0);

    let features = vec![
        "✓ Full Unicode support with automatic encoding detection",
        "✓ Dynamic font embedding with CID/GID mapping",
        "✓ Accurate glyph width calculation for perfect spacing",
        "✓ Support for Type0 fonts with Identity-H encoding",
        "✓ Production-ready with comprehensive test coverage",
        "✓ Pure Rust implementation - no C dependencies",
        "✓ Memory efficient with streaming support",
    ];

    for feature in features {
        graphics.draw_text(feature, 70.0, y)?;
        y -= 20.0;
    }

    // Footer
    y = 60.0;
    graphics.set_custom_font("Arial", 10.0);
    graphics.set_fill_color(Color::rgb(0.5, 0.5, 0.5));
    graphics.draw_text(
        "Generated with Oxidize-PDF • github.com/your-org/oxidize-pdf",
        150.0,
        y,
    )?;

    document.add_page(page);

    // Save
    let output_file = "test-pdfs/unicode_professional_demo.pdf";
    document.save(output_file)?;

    // Summary
    println!("\n✅ Successfully generated: {}", output_file);
    println!("\n📊 Demo Statistics:");
    println!("  • Currency symbols: 6");
    println!("  • Mathematical operators: 15+");
    println!("  • International characters: 40+");
    println!("  • Business symbols: 20+");
    println!("\n🚀 Oxidize-PDF: Ready for production use!");
    println!("═══════════════════════════════════════════════\n");

    Ok(())
}
