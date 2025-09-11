use oxidize_pdf::*;

fn main() -> Result<(), PdfError> {
    println!("🔧 Debug: Testing text rendering directly...");
    
    let mut document = Document::new();
    let mut page = Page::new(595.0, 842.0); // A4 size
    
    // Test 1: Simple text at known position
    println!("Test 1: Rendering simple text...");
    page.text()
        .set_font(Font::Helvetica, 24.0)
        .set_fill_color(Color::black())
        .at(50.0, 800.0) // Near top of page
        .write("TEST: This is simple black text")?;
    
    // Test 2: Different colors
    println!("Test 2: Testing different colors...");
    page.text()
        .set_font(Font::HelveticaBold, 18.0)
        .set_fill_color(Color::red())
        .at(50.0, 750.0)
        .write("TEST: This is red bold text")?;
    
    // Test 3: Test light blue background with dark text (like our theme)
    println!("Test 3: Testing background with text...");
    let graphics = page.graphics();
    
    // Draw background rectangle
    graphics.set_fill_color(Color::hex("#f0f4f8")) // Our theme surface color
            .rect(50.0, 650.0, 300.0, 80.0)
            .fill();
    
    // Draw text on top
    page.text()
        .set_font(Font::Helvetica, 16.0)
        .set_fill_color(Color::hex("#1a202c")) // Dark text
        .at(60.0, 710.0)
        .write("Revenue: $2,547,820")?;
        
    page.text()
        .set_font(Font::Helvetica, 14.0)
        .set_fill_color(Color::hex("#4a5568")) // Secondary text
        .at(60.0, 690.0)
        .write("Total Revenue")?;
    
    // Test 4: Try with exact KPI card background color and positions
    println!("Test 4: Testing KPI card simulation...");
    
    // Card background
    graphics.set_fill_color(Color::hex("#f0f4f8"))
            .rect(50.0, 500.0, 200.0, 120.0)
            .fill();
    
    // Card border
    graphics.set_stroke_color(Color::hex("#e2e8f0"))
            .set_line_width(1.0)
            .rect(50.0, 500.0, 200.0, 120.0)
            .stroke();
    
    // Title (top of card)
    page.text()
        .set_font(Font::Helvetica, 12.0)
        .set_fill_color(Color::hex("#4a5568"))
        .at(58.0, 606.0) // Near top
        .write("Total Revenue")?;
    
    // Value (middle)
    page.text()
        .set_font(Font::HelveticaBold, 18.0)
        .set_fill_color(Color::hex("#1a202c"))
        .at(58.0, 580.0)
        .write("$2,547,820")?;
    
    // Trend (below value)
    page.text()
        .set_font(Font::Helvetica, 11.0)
        .set_fill_color(Color::hex("#38a169")) // Green for positive trend
        .at(58.0, 560.0)
        .write("↗ +12.3% vs Q3 2024")?;
    
    // Add coordinate system reference
    println!("Test 5: Adding coordinate reference...");
    page.text()
        .set_font(Font::Helvetica, 8.0)
        .set_fill_color(Color::blue())
        .at(50.0, 50.0) // Bottom left reference
        .write("PDF Coords: (0,0) is bottom-left, Y increases upward")?;
        
    page.text()
        .set_font(Font::Helvetica, 8.0)
        .set_fill_color(Color::blue())
        .at(50.0, 830.0) // Top reference
        .write("This is near top of page (Y=830)")?;
    
    document.add_page(page);
    
    let output_path = "examples/results/debug_text_rendering.pdf";
    document.save(output_path)?;
    
    println!("✅ Debug PDF saved to: {}", output_path);
    println!("📋 Manual verification needed:");
    println!("  1. Check that all text is visible");
    println!("  2. Verify colors are correct");
    println!("  3. Confirm background/text z-order is correct");
    println!("  4. Check coordinate system understanding");
    
    Ok(())
}