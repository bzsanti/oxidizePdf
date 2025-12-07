use oxidize_pdf::{Color, Document, Font, Page, Result};

fn main() -> Result<()> {
    println!("🔍 Font Subsetting & Glyph Rendering Test");
    println!("==========================================");

    let mut doc = Document::new();
    doc.set_title("Font Subsetting Test");

    let mut page = Page::a4();

    // Y position tracker
    let mut y_pos = 780.0;
    let line_height = 35.0;

    // Title
    page.text()
        .set_font(Font::HelveticaBold, 16.0)
        .at(50.0, y_pos)
        .write("Font Subsetting & Glyph Rendering Test")?;
    y_pos -= line_height * 1.5;

    // Test 1: Basic ASCII characters
    page.text()
        .set_font(Font::Helvetica, 12.0)
        .at(50.0, y_pos)
        .write("1. Basic ASCII (subsetting should include only used chars):")?;
    y_pos -= 30.0;

    let ascii_test = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";

    page.text()
        .set_font(Font::custom("Arial".to_string()), 11.0)
        .at(70.0, y_pos)
        .write(ascii_test)?;
    y_pos -= 25.0;

    page.text()
        .set_font(Font::Helvetica, 9.0)
        .at(70.0, y_pos)
        .write("(62 unique glyphs - font will be subset)")?;
    y_pos -= 35.0;

    // Test 2: Special characters and punctuation
    page.text()
        .set_font(Font::Helvetica, 12.0)
        .at(50.0, y_pos)
        .write("2. Special Characters & Punctuation:")?;
    y_pos -= 30.0;

    let special_test = "!@#$%^&*()_+-=[]{}|;':\",./<>?`~";

    page.text()
        .set_font(Font::Helvetica, 12.0)
        .at(70.0, y_pos)
        .write(special_test)?;
    y_pos -= 35.0;

    // Test 3: Extended Latin characters
    page.text()
        .set_font(Font::Helvetica, 12.0)
        .at(50.0, y_pos)
        .write("3. Extended Latin (Latin-1 Supplement):")?;
    y_pos -= 30.0;

    // Using subset of Latin-1 that's likely supported
    let latin_test = "àáâãäåæçèéêëìíîïñòóôõöøùúûüýÿ";

    page.text()
        .set_font(Font::Helvetica, 12.0)
        .at(70.0, y_pos)
        .write(latin_test)?;
    y_pos -= 35.0;

    // Test 4: Currency and common symbols
    page.text()
        .set_font(Font::Helvetica, 12.0)
        .at(50.0, y_pos)
        .write("4. Common Symbols:")?;
    y_pos -= 30.0;

    // Common symbols that should be in standard fonts
    let symbol_test = "© ® ™ € £ ¥ ¢ § ¶ • ° ± × ÷";

    page.text()
        .set_font(Font::Helvetica, 12.0)
        .at(70.0, y_pos)
        .write(symbol_test)?;
    y_pos -= 35.0;

    // Test 5: Minimal character set for maximum subsetting
    page.text()
        .set_font(Font::Helvetica, 12.0)
        .at(50.0, y_pos)
        .write("5. Subsetting Efficiency (minimal character set):")?;
    y_pos -= 30.0;

    let minimal_test = "AAAAA BBBBB CCCCC DDDDD EEEEE";

    page.text()
        .set_font(Font::custom("Arial".to_string()), 12.0)
        .at(70.0, y_pos)
        .write(minimal_test)?;
    y_pos -= 25.0;

    page.text()
        .set_font(Font::Helvetica, 9.0)
        .at(70.0, y_pos)
        .write("(Only 6 glyphs: A,B,C,D,E,space - maximum subsetting)")?;
    y_pos -= 35.0;

    // Test 6: Font metrics preservation
    page.text()
        .set_font(Font::Helvetica, 12.0)
        .at(50.0, y_pos)
        .write("6. Font Metrics Test (baseline alignment):")?;
    y_pos -= 30.0;

    // Draw baseline
    let gc = page.graphics();
    gc.set_stroke_color(Color::rgb(0.8, 0.8, 0.8));
    gc.move_to(70.0, y_pos);
    gc.line_to(500.0, y_pos);
    gc.stroke();

    let align_test = "Baseline Test gjpqy";

    page.text()
        .set_font(Font::Helvetica, 14.0)
        .at(70.0, y_pos)
        .write(align_test)?;

    page.text()
        .set_font(Font::TimesRoman, 14.0)
        .at(250.0, y_pos)
        .write(align_test)?;

    page.text()
        .set_font(Font::Courier, 14.0)
        .at(400.0, y_pos)
        .write(align_test)?;

    y_pos -= 50.0;

    // Test 7: Character width preservation
    page.text()
        .set_font(Font::Helvetica, 12.0)
        .at(50.0, y_pos)
        .write("7. Character Width Test:")?;
    y_pos -= 30.0;

    // Test with monospace vs proportional
    let width_test = "iiiiii WWWWWW 111111 MMMMMM";

    page.text()
        .set_font(Font::Courier, 12.0)
        .at(70.0, y_pos)
        .write(&format!("Courier: {}", width_test))?;
    y_pos -= 20.0;

    page.text()
        .set_font(Font::Helvetica, 12.0)
        .at(70.0, y_pos)
        .write(&format!("Helvetica: {}", width_test))?;
    y_pos -= 40.0;

    // Summary
    page.text()
        .set_font(Font::HelveticaBold, 12.0)
        .at(50.0, y_pos)
        .write("Expected Results:")?;
    y_pos -= 25.0;

    let summary = [
        "- All characters render without missing glyphs",
        "- Fonts are subset to include only used characters",
        "- Baseline alignment is preserved across fonts",
        "- Character widths are correct (no overlap)",
        "- PDF file size is optimized through subsetting",
    ];

    for line in summary {
        page.text()
            .set_font(Font::Helvetica, 10.0)
            .at(70.0, y_pos)
            .write(line)?;
        y_pos -= 18.0;
    }

    // Add page to document
    doc.add_page(page);

    // Save PDF
    let output_path = "examples/results/font_subsetting_test.pdf";
    doc.save(output_path)?;

    println!("✅ Font subsetting test PDF created: {}", output_path);
    println!("\n📊 Subsetting Statistics:");
    println!("   - Tests include 62+ unique ASCII characters");
    println!("   - Special characters and Latin-1 supplement");
    println!("   - Minimal subset test with only 6 glyphs");
    println!("\n🔍 Verify in PDF:");
    println!("   1. All glyphs render correctly");
    println!("   2. Check PDF file size (optimized through subsetting)");
    println!("   3. Text is searchable and selectable");
    println!("   4. Font metrics preserved (baseline, widths)");

    Ok(())
}
