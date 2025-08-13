//! Example demonstrating ICCBased color profiles and Indexed color spaces
//!
//! This example shows how to use:
//! - ICC color profiles (sRGB, Adobe RGB, CMYK profiles)
//! - Indexed color spaces with limited palettes
//! - Web-safe colors and custom palettes
//! - Color space conversions

use oxidize_pdf::{Document, Page, Result};
use oxidize_pdf::graphics::{
    Color, GraphicsContext, IccProfile, IccProfileManager, IndexedColorSpace,
    IndexedColorManager, StandardIccProfile,
};
use oxidize_pdf::geometry::{Point, Rectangle};
use oxidize_pdf::text::Font;
use std::fs;

fn main() -> Result<()> {
    println!("Creating ICC and Indexed Color Spaces example...");

    // Create a new document
    let mut doc = Document::new();
    doc.set_title("ICC and Indexed Color Spaces Example");
    doc.set_author("Oxidize PDF");

    // Create pages for different examples
    create_icc_profiles_page(&mut doc)?;
    create_indexed_colors_page(&mut doc)?;
    create_palette_comparison_page(&mut doc)?;

    // Save the document
    let output = "examples/results/icc_indexed_colors.pdf";
    fs::create_dir_all("examples/results")?;
    doc.save(output)?;
    
    println!("✓ Created {}", output);
    Ok(())
}

/// Create a page demonstrating ICC color profiles
fn create_icc_profiles_page(doc: &mut Document) -> Result<()> {
    let mut page = Page::new(595.0, 842.0); // A4 size
    let mut graphics = GraphicsContext::new();
    
    // Title
    graphics.set_font(&Font::helvetica_bold(), 16.0);
    graphics.text_at(Point::new(50.0, 780.0), "ICC Color Profiles");
    
    // Create ICC profile manager
    let mut icc_manager = IccProfileManager::new();
    
    // Add standard profiles
    let srgb_name = icc_manager.add_standard_profile(StandardIccProfile::SRgb)?;
    let adobe_rgb_name = icc_manager.add_standard_profile(StandardIccProfile::AdobeRgb)?;
    let cmyk_name = icc_manager.add_standard_profile(StandardIccProfile::CoatedFogra39)?;
    let gray_name = icc_manager.add_standard_profile(StandardIccProfile::GrayGamma22)?;
    
    // Display profile information
    let mut y = 740.0;
    graphics.set_font(&Font::helvetica(), 12.0);
    
    // sRGB Profile
    graphics.text_at(Point::new(50.0, y), "sRGB Profile:");
    if let Some(profile) = icc_manager.get_profile(&srgb_name) {
        draw_color_swatches(&mut graphics, 200.0, y - 10.0, profile.is_rgb())?;
        graphics.set_font(&Font::helvetica(), 10.0);
        graphics.text_at(
            Point::new(50.0, y - 20.0),
            &format!("Components: {}, Size: {} bytes", profile.components, profile.size())
        );
    }
    
    y -= 80.0;
    
    // Adobe RGB Profile
    graphics.set_font(&Font::helvetica(), 12.0);
    graphics.text_at(Point::new(50.0, y), "Adobe RGB Profile:");
    if let Some(profile) = icc_manager.get_profile(&adobe_rgb_name) {
        draw_adobe_rgb_swatches(&mut graphics, 200.0, y - 10.0)?;
        graphics.set_font(&Font::helvetica(), 10.0);
        graphics.text_at(
            Point::new(50.0, y - 20.0),
            &format!("Components: {}, Wider gamut for print", profile.components)
        );
    }
    
    y -= 80.0;
    
    // CMYK Profile
    graphics.set_font(&Font::helvetica(), 12.0);
    graphics.text_at(Point::new(50.0, y), "CMYK Profile (Coated FOGRA39):");
    if let Some(profile) = icc_manager.get_profile(&cmyk_name) {
        draw_cmyk_swatches(&mut graphics, 200.0, y - 10.0)?;
        graphics.set_font(&Font::helvetica(), 10.0);
        graphics.text_at(
            Point::new(50.0, y - 20.0),
            &format!("Components: {}, ISO standard for printing", profile.components)
        );
    }
    
    y -= 80.0;
    
    // Grayscale Profile
    graphics.set_font(&Font::helvetica(), 12.0);
    graphics.text_at(Point::new(50.0, y), "Grayscale Profile:");
    if let Some(profile) = icc_manager.get_profile(&gray_name) {
        draw_grayscale_swatches(&mut graphics, 200.0, y - 10.0)?;
        graphics.set_font(&Font::helvetica(), 10.0);
        graphics.text_at(
            Point::new(50.0, y - 20.0),
            &format!("Components: {}, Gamma 2.2", profile.components)
        );
    }
    
    // Add summary
    graphics.set_font(&Font::helvetica(), 10.0);
    graphics.text_at(
        Point::new(50.0, 100.0),
        &format!("Total ICC profiles loaded: {}", icc_manager.count())
    );
    
    page.set_graphics_context(graphics);
    doc.add_page(page);
    Ok(())
}

/// Create a page demonstrating indexed color spaces
fn create_indexed_colors_page(doc: &mut Document) -> Result<()> {
    let mut page = Page::new(595.0, 842.0);
    let mut graphics = GraphicsContext::new();
    
    // Title
    graphics.set_font(&Font::helvetica_bold(), 16.0);
    graphics.text_at(Point::new(50.0, 780.0), "Indexed Color Spaces");
    
    let mut y = 740.0;
    
    // Web-safe palette
    graphics.set_font(&Font::helvetica(), 12.0);
    graphics.text_at(Point::new(50.0, y), "Web-Safe Color Palette (216 colors):");
    y -= 20.0;
    
    let web_safe = IndexedColorSpace::web_safe_palette()?;
    draw_indexed_palette(&mut graphics, 50.0, y, &web_safe, 36)?; // Show first 36 colors
    
    graphics.set_font(&Font::helvetica(), 10.0);
    graphics.text_at(
        Point::new(50.0, y - 70.0),
        &format!("Total colors: {}, Max index: {}", web_safe.color_count(), web_safe.max_index())
    );
    
    y -= 120.0;
    
    // Grayscale palette
    graphics.set_font(&Font::helvetica(), 12.0);
    graphics.text_at(Point::new(50.0, y), "16-Level Grayscale Palette:");
    y -= 20.0;
    
    let grayscale = IndexedColorSpace::grayscale_palette(16)?;
    draw_indexed_palette(&mut graphics, 50.0, y, &grayscale, 16)?;
    
    graphics.set_font(&Font::helvetica(), 10.0);
    graphics.text_at(
        Point::new(50.0, y - 40.0),
        &format!("Total grays: {}, Max index: {}", grayscale.color_count(), grayscale.max_index())
    );
    
    y -= 100.0;
    
    // Custom RGB palette
    graphics.set_font(&Font::helvetica(), 12.0);
    graphics.text_at(Point::new(50.0, y), "Custom Limited Palette:");
    y -= 20.0;
    
    let custom_colors = vec![
        Color::rgb(0.8, 0.0, 0.0),  // Dark Red
        Color::rgb(1.0, 0.2, 0.2),  // Light Red
        Color::rgb(0.0, 0.6, 0.0),  // Dark Green
        Color::rgb(0.2, 1.0, 0.2),  // Light Green
        Color::rgb(0.0, 0.0, 0.8),  // Dark Blue
        Color::rgb(0.3, 0.3, 1.0),  // Light Blue
        Color::rgb(1.0, 0.8, 0.0),  // Yellow
        Color::rgb(1.0, 0.5, 0.0),  // Orange
    ];
    
    let custom_palette = IndexedColorSpace::from_palette(&custom_colors)?;
    draw_indexed_palette(&mut graphics, 50.0, y, &custom_palette, 8)?;
    
    graphics.set_font(&Font::helvetica(), 10.0);
    graphics.text_at(
        Point::new(50.0, y - 40.0),
        "Custom palette for specific design needs"
    );
    
    page.set_graphics_context(graphics);
    doc.add_page(page);
    Ok(())
}

/// Create a page comparing different color palettes
fn create_palette_comparison_page(doc: &mut Document) -> Result<()> {
    let mut page = Page::new(595.0, 842.0);
    let mut graphics = GraphicsContext::new();
    
    // Title
    graphics.set_font(&Font::helvetica_bold(), 16.0);
    graphics.text_at(Point::new(50.0, 780.0), "Color Palette Comparison");
    
    // Create manager for indexed colors
    let mut indexed_manager = IndexedColorManager::new();
    
    // Add standard palettes
    let web_name = indexed_manager.create_web_safe()?;
    let gray64_name = indexed_manager.create_grayscale(64)?;
    let gray256_name = indexed_manager.create_grayscale(256)?;
    
    let mut y = 720.0;
    
    // Demonstrate color quantization
    graphics.set_font(&Font::helvetica(), 12.0);
    graphics.text_at(Point::new(50.0, y), "Color Quantization Example:");
    y -= 30.0;
    
    // Original colors
    graphics.text_at(Point::new(50.0, y), "Original colors:");
    let test_colors = vec![
        Color::rgb(0.75, 0.25, 0.5),
        Color::rgb(0.33, 0.66, 0.99),
        Color::rgb(0.9, 0.8, 0.1),
        Color::rgb(0.1, 0.9, 0.5),
    ];
    
    for (i, color) in test_colors.iter().enumerate() {
        let x = 200.0 + (i as f64 * 60.0);
        graphics.set_fill_color(*color);
        graphics.fill_rect(Rectangle::from_position_and_size(x, y - 20.0, 50.0, 20.0));
    }
    
    y -= 50.0;
    
    // Quantized to web-safe
    graphics.set_fill_color(Color::black());
    graphics.text_at(Point::new(50.0, y), "Quantized to web-safe:");
    
    if let Some(web_space) = indexed_manager.get_space(&web_name) {
        for (i, color) in test_colors.iter().enumerate() {
            let index = web_space.find_closest_index(color);
            if let Some(quantized) = web_space.get_color(index) {
                let x = 200.0 + (i as f64 * 60.0);
                graphics.set_fill_color(quantized);
                graphics.fill_rect(Rectangle::from_position_and_size(x, y - 20.0, 50.0, 20.0));
            }
        }
    }
    
    y -= 60.0;
    
    // Statistics
    graphics.set_fill_color(Color::black());
    graphics.set_font(&Font::helvetica_bold(), 12.0);
    graphics.text_at(Point::new(50.0, y), "Palette Statistics:");
    
    graphics.set_font(&Font::helvetica(), 10.0);
    y -= 20.0;
    
    for (name, label) in &[
        (web_name.clone(), "Web-Safe Palette"),
        (gray64_name, "64-Level Grayscale"),
        (gray256_name, "256-Level Grayscale"),
    ] {
        if let Some(space) = indexed_manager.get_space(name) {
            graphics.text_at(
                Point::new(70.0, y),
                &format!("{}: {} colors", label, space.color_count())
            );
            y -= 15.0;
        }
    }
    
    // Benefits of indexed colors
    y -= 30.0;
    graphics.set_font(&Font::helvetica_bold(), 12.0);
    graphics.text_at(Point::new(50.0, y), "Benefits of Indexed Color Spaces:");
    
    graphics.set_font(&Font::helvetica(), 10.0);
    let benefits = vec![
        "• Reduced file size for images with limited colors",
        "• Efficient storage of logos and graphics",
        "• Consistent color reproduction",
        "• Fast color lookup operations",
        "• Ideal for GIF-like images in PDFs",
    ];
    
    y -= 20.0;
    for benefit in benefits {
        graphics.text_at(Point::new(70.0, y), benefit);
        y -= 15.0;
    }
    
    page.set_graphics_context(graphics);
    doc.add_page(page);
    Ok(())
}

/// Helper function to draw RGB color swatches
fn draw_color_swatches(graphics: &mut GraphicsContext, x: f64, y: f64, _is_rgb: bool) -> Result<()> {
    let colors = vec![
        Color::red(),
        Color::green(),
        Color::blue(),
        Color::yellow(),
        Color::cyan(),
        Color::magenta(),
    ];
    
    for (i, color) in colors.iter().enumerate() {
        graphics.set_fill_color(*color);
        graphics.fill_rect(Rectangle::from_position_and_size(
            x + (i as f64 * 35.0),
            y,
            30.0,
            30.0,
        ));
    }
    
    Ok(())
}

/// Helper function to draw Adobe RGB swatches (simulated wider gamut)
fn draw_adobe_rgb_swatches(graphics: &mut GraphicsContext, x: f64, y: f64) -> Result<()> {
    // Simulating wider gamut with more saturated colors
    let colors = vec![
        Color::rgb(1.0, 0.0, 0.0),   // Pure red
        Color::rgb(0.0, 1.0, 0.0),   // Pure green
        Color::rgb(0.0, 0.0, 1.0),   // Pure blue
        Color::rgb(1.0, 0.5, 0.0),   // Orange
        Color::rgb(0.5, 0.0, 1.0),   // Purple
        Color::rgb(0.0, 1.0, 0.5),   // Teal
    ];
    
    for (i, color) in colors.iter().enumerate() {
        graphics.set_fill_color(*color);
        graphics.fill_rect(Rectangle::from_position_and_size(
            x + (i as f64 * 35.0),
            y,
            30.0,
            30.0,
        ));
    }
    
    Ok(())
}

/// Helper function to draw CMYK swatches
fn draw_cmyk_swatches(graphics: &mut GraphicsContext, x: f64, y: f64) -> Result<()> {
    let colors = vec![
        Color::cmyk(1.0, 0.0, 0.0, 0.0),  // Cyan
        Color::cmyk(0.0, 1.0, 0.0, 0.0),  // Magenta
        Color::cmyk(0.0, 0.0, 1.0, 0.0),  // Yellow
        Color::cmyk(0.0, 0.0, 0.0, 1.0),  // Black
        Color::cmyk(1.0, 0.0, 1.0, 0.0),  // Green (C+Y)
        Color::cmyk(0.0, 1.0, 1.0, 0.0),  // Red (M+Y)
    ];
    
    for (i, color) in colors.iter().enumerate() {
        graphics.set_fill_color(*color);
        graphics.fill_rect(Rectangle::from_position_and_size(
            x + (i as f64 * 35.0),
            y,
            30.0,
            30.0,
        ));
    }
    
    Ok(())
}

/// Helper function to draw grayscale swatches
fn draw_grayscale_swatches(graphics: &mut GraphicsContext, x: f64, y: f64) -> Result<()> {
    for i in 0..8 {
        let gray = i as f64 / 7.0;
        graphics.set_fill_color(Color::gray(gray));
        graphics.fill_rect(Rectangle::from_position_and_size(
            x + (i as f64 * 35.0),
            y,
            30.0,
            30.0,
        ));
    }
    
    Ok(())
}

/// Helper function to draw indexed color palette
fn draw_indexed_palette(
    graphics: &mut GraphicsContext,
    x: f64,
    y: f64,
    palette: &IndexedColorSpace,
    max_colors: usize,
) -> Result<()> {
    let colors_to_show = max_colors.min(palette.color_count());
    let cols = 12;
    let size = 15.0;
    
    for i in 0..colors_to_show {
        let index = i as u8;
        if let Some(color) = palette.get_color(index) {
            let col = i % cols;
            let row = i / cols;
            
            graphics.set_fill_color(color);
            graphics.fill_rect(Rectangle::from_position_and_size(
                x + (col as f64 * (size + 2.0)),
                y - (row as f64 * (size + 2.0)),
                size,
                size,
            ));
        }
    }
    
    Ok(())
}