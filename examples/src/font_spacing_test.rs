use oxidize_pdf::{Color, Document, Font, Page, Result};

fn main() -> Result<()> {
    println!("🔍 Font Spacing Verification Test");
    println!("==================================");

    let mut doc = Document::new();
    doc.set_title("Font Spacing Test");

    let mut page = Page::a4();

    // Test text for spacing
    let test_text = "Hello World! Testing spacing 123.";
    let spacing_test = "i i i i i | | | | | W W W W W";
    let kerning_test = "AVATAR Wave VAV ToTo";

    // Y position tracker
    let mut y_pos = 750.0;
    let line_height = 40.0;

    // Use graphics context for colored titles
    let gc = page.graphics();
    gc.set_fill_color(Color::rgb(0.0, 0.0, 0.0));

    // Title
    page.text()
        .set_font(Font::Helvetica, 16.0)
        .at(50.0, y_pos)
        .write("Font Spacing Test - All Font Types")?;
    y_pos -= line_height * 1.5;

    // Test 1: Standard fonts (no subsetting)
    page.text()
        .set_font(Font::Helvetica, 12.0)
        .at(50.0, y_pos)
        .write("1. Standard Fonts (Built-in):")?;
    y_pos -= line_height;

    page.text()
        .set_font(Font::Helvetica, 14.0)
        .at(70.0, y_pos)
        .write(&format!("Helvetica: {}", test_text))?;
    y_pos -= 25.0;

    page.text()
        .set_font(Font::TimesRoman, 14.0)
        .at(70.0, y_pos)
        .write(&format!("Times: {}", test_text))?;
    y_pos -= 25.0;

    page.text()
        .set_font(Font::Courier, 14.0)
        .at(70.0, y_pos)
        .write(&format!("Courier: {}", test_text))?;
    y_pos -= line_height;

    // Test 2: Custom fonts
    page.text()
        .set_font(Font::Helvetica, 12.0)
        .at(50.0, y_pos)
        .write("2. Custom TrueType Fonts:")?;
    y_pos -= line_height;

    // Using custom font names (actual loading would require font file support)
    let custom_fonts = ["Arial", "Avenir", "Georgia"];

    for font_name in &custom_fonts {
        let custom_font = Font::custom(font_name.to_string());
        page.text()
            .set_font(custom_font, 14.0)
            .at(70.0, y_pos)
            .write(&format!("{}: {}", font_name, test_text))?;
        y_pos -= 25.0;
    }

    y_pos -= 15.0;

    // Test 3: Spacing patterns
    page.text()
        .set_font(Font::Helvetica, 12.0)
        .at(50.0, y_pos)
        .write("3. Spacing Pattern Tests:")?;
    y_pos -= line_height;

    page.text()
        .set_font(Font::Helvetica, 14.0)
        .at(70.0, y_pos)
        .write(&format!("Narrow/Wide: {}", spacing_test))?;
    y_pos -= 25.0;

    page.text()
        .set_font(Font::custom("Arial".to_string()), 14.0)
        .at(70.0, y_pos)
        .write(&format!("Arial: {}", spacing_test))?;
    y_pos -= 35.0;

    // Test 4: Kerning test
    page.text()
        .set_font(Font::Helvetica, 12.0)
        .at(50.0, y_pos)
        .write("4. Kerning Tests:")?;
    y_pos -= line_height;

    page.text()
        .set_font(Font::Helvetica, 14.0)
        .at(70.0, y_pos)
        .write(&format!("Helvetica: {}", kerning_test))?;
    y_pos -= 25.0;

    page.text()
        .set_font(Font::custom("Arial".to_string()), 14.0)
        .at(70.0, y_pos)
        .write(&format!("Arial: {}", kerning_test))?;
    y_pos -= 35.0;

    // Test 5: Grid alignment test
    page.text()
        .set_font(Font::Helvetica, 12.0)
        .at(50.0, y_pos)
        .write("5. Grid Alignment (should be evenly spaced):")?;
    y_pos -= line_height;

    // Draw grid markers
    let gc = page.graphics();
    gc.set_stroke_color(Color::rgb(0.8, 0.8, 0.8));
    for i in 0..10 {
        let x = 70.0 + (i as f64 * 40.0);
        gc.move_to(x, y_pos + 10.0);
        gc.line_to(x, y_pos - 10.0);
        gc.stroke();
    }

    let grid_text = "A B C D E F G H I";
    page.text()
        .set_font(Font::Courier, 14.0)
        .at(70.0, y_pos)
        .write(grid_text)?;
    y_pos -= 25.0;

    page.text()
        .set_font(Font::custom("Arial".to_string()), 14.0)
        .at(70.0, y_pos - 5.0)
        .write(grid_text)?;

    // Add page to document
    doc.add_page(page);

    // Save PDF
    let output_path = "examples/results/font_spacing_test.pdf";
    doc.save(output_path)?;

    println!("✅ Font spacing test PDF created: {}", output_path);
    println!("\n📋 Tests included:");
    println!("   1. Standard PDF fonts (Helvetica, Times, Courier)");
    println!("   2. Custom fonts demonstration");
    println!("   3. Narrow/wide character spacing patterns");
    println!("   4. Kerning pair tests (AV, WA, To, etc.)");
    println!("   5. Grid alignment verification");
    println!("\n🔍 Check the PDF for:");
    println!("   - Consistent spacing between characters");
    println!("   - No overlapping text");
    println!("   - Proper kerning pairs");
    println!("   - Grid alignment accuracy");

    Ok(())
}
