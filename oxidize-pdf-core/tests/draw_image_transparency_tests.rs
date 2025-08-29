//! Tests for GraphicsContext::draw_image_with_transparency

use oxidize_pdf::error::Result;
use oxidize_pdf::graphics::{Color, GraphicsContext, Image, MaskType};
use oxidize_pdf::{Document, Page};

#[test]
fn test_draw_image_basic() -> Result<()> {
    let mut gc = GraphicsContext::new();

    // Test basic image drawing
    gc.draw_image("Image1", 100.0, 200.0, 150.0, 100.0);

    let ops = gc.get_operations();
    assert!(ops.contains("q\n"));
    assert!(ops.contains("150.00 0 0 100.00 100.00 200.00 cm"));
    assert!(ops.contains("/Image1 Do"));
    assert!(ops.contains("Q\n"));

    Ok(())
}

#[test]
fn test_draw_image_with_transparency() -> Result<()> {
    let mut gc = GraphicsContext::new();

    // Test image drawing with transparency mask
    gc.draw_image_with_transparency("Image1", 100.0, 200.0, 150.0, 100.0, Some("Mask1"));

    let ops = gc.get_operations();

    // Should save state
    assert!(ops.contains("q\n"));

    // Should apply soft mask via ExtGState
    assert!(ops.contains("/GS"));
    assert!(ops.contains(" gs"));

    // Should draw the image
    assert!(ops.contains("/Image1 Do"));

    // Should restore state
    assert!(ops.contains("Q\n"));

    Ok(())
}

#[test]
fn test_draw_image_without_mask() -> Result<()> {
    let mut gc = GraphicsContext::new();

    // Test image drawing without transparency mask
    gc.draw_image_with_transparency("Image2", 50.0, 100.0, 200.0, 150.0, None);

    let ops = gc.get_operations();

    // Should save state
    assert!(ops.contains("q\n"));

    // Should NOT apply soft mask since mask_name is None
    assert!(!ops.contains("/SMask"));

    // Should draw the image
    assert!(ops.contains("/Image2 Do"));

    // Should restore state
    assert!(ops.contains("Q\n"));

    Ok(())
}

#[test]
fn test_multiple_images_with_transparency() -> Result<()> {
    let mut gc = GraphicsContext::new();

    // Draw multiple images with different transparency settings
    gc.draw_image_with_transparency("Image1", 10.0, 10.0, 100.0, 100.0, Some("Mask1"));
    gc.draw_image_with_transparency("Image2", 120.0, 10.0, 100.0, 100.0, None);
    gc.draw_image_with_transparency("Image3", 230.0, 10.0, 100.0, 100.0, Some("Mask3"));

    let ops = gc.get_operations();

    // Should have three image drawing operations
    assert_eq!(ops.matches("/Image1 Do").count(), 1);
    assert_eq!(ops.matches("/Image2 Do").count(), 1);
    assert_eq!(ops.matches("/Image3 Do").count(), 1);

    // Should have appropriate number of save/restore pairs
    assert_eq!(ops.matches("q\n").count(), 3);
    assert_eq!(ops.matches("Q\n").count(), 3);

    Ok(())
}

#[test]
fn test_draw_image_with_other_operations() -> Result<()> {
    let mut gc = GraphicsContext::new();

    // Mix image drawing with other graphics operations
    gc.set_fill_color(Color::rgb(1.0, 0.0, 0.0))
        .rectangle(0.0, 0.0, 100.0, 100.0)
        .fill();

    gc.draw_image_with_transparency("Logo", 10.0, 10.0, 80.0, 80.0, Some("LogoMask"));

    gc.set_stroke_color(Color::rgb(0.0, 0.0, 1.0))
        .set_line_width(2.0)
        .rectangle(5.0, 5.0, 90.0, 90.0)
        .stroke();

    let ops = gc.get_operations();

    // Should have all operations in correct order
    assert!(ops.contains("1.000 0.000 0.000 rg")); // Red fill color
    assert!(ops.contains("0.00 0.00 100.00 100.00 re")); // Rectangle
    assert!(ops.contains("f\n")); // Fill
    assert!(ops.contains("/Logo Do")); // Image
    assert!(ops.contains("0.000 0.000 1.000 RG")); // Blue stroke color
    assert!(ops.contains("2.00 w")); // Line width
    assert!(ops.contains("5.00 5.00 90.00 90.00 re")); // Rectangle
    assert!(ops.contains("S\n")); // Stroke

    Ok(())
}

#[test]
fn test_image_transformation_matrix() -> Result<()> {
    let mut gc = GraphicsContext::new();

    // Test different positions and sizes
    gc.draw_image_with_transparency("Test", 123.45, 678.90, 234.56, 345.67, None);

    let ops = gc.get_operations();

    // Check transformation matrix is correct
    assert!(ops.contains("234.56 0 0 345.67 123.45 678.90 cm"));

    Ok(())
}

#[test]
fn test_draw_image_in_document() -> Result<()> {
    let mut doc = Document::new();
    let mut page = Page::new(612.0, 792.0);

    let gc = page.graphics();

    // Draw background
    gc.set_fill_color(Color::rgb(0.9, 0.9, 0.9))
        .rectangle(0.0, 0.0, 612.0, 792.0)
        .fill();

    // Draw image with transparency
    gc.draw_image_with_transparency("MyImage", 100.0, 400.0, 400.0, 300.0, Some("MyMask"));

    // Draw border
    gc.set_stroke_color(Color::black())
        .set_line_width(3.0)
        .rectangle(100.0, 400.0, 400.0, 300.0)
        .stroke();

    doc.add_page(page);

    // Document should be valid
    assert_eq!(doc.page_count(), 1);

    Ok(())
}

#[test]
fn test_opacity_with_image() -> Result<()> {
    let mut gc = GraphicsContext::new();

    // Set opacity and draw a rectangle (which will apply the ExtGState)
    gc.set_opacity(0.5);
    gc.rectangle(0.0, 0.0, 100.0, 100.0);
    gc.fill(); // This will apply the opacity ExtGState

    // Draw image with transparency
    gc.draw_image_with_transparency("SemiTransparent", 0.0, 0.0, 100.0, 100.0, None);

    // Reset opacity
    gc.set_opacity(1.0);
    gc.draw_image("Opaque", 110.0, 0.0, 100.0, 100.0);

    let ops = gc.get_operations();

    // Should have ExtGState application for opacity (from the fill operation)
    assert!(ops.contains(" gs"));

    Ok(())
}

#[test]
fn test_create_png_with_transparency() -> Result<()> {
    // Create a simple RGBA image
    let rgba_data = vec![
        255, 0, 0, 255, // Red, opaque
        0, 255, 0, 192, // Green, 75% opaque
        0, 0, 255, 128, // Blue, 50% opaque
        255, 255, 0, 64, // Yellow, 25% opaque
    ];

    let image = Image::from_rgba_data(rgba_data, 2, 2)?;

    // Check that transparency is detected
    assert!(image.has_transparency());

    // Check we can create a soft mask
    let mask = image.create_mask(MaskType::Soft, None);
    assert!(mask.is_some());

    Ok(())
}

#[test]
fn test_stencil_mask_creation() -> Result<()> {
    // Create an RGBA image with varying alpha levels
    let rgba_data = vec![
        255, 0, 0, 255, // Red, fully opaque
        0, 255, 0, 192, // Green, 75% opaque
        0, 0, 255, 128, // Blue, 50% opaque
        255, 255, 0, 64, // Yellow, 25% opaque
        128, 128, 128, 32, // Gray, 12.5% opaque
        64, 64, 64, 16, // Dark gray, 6.25% opaque
        32, 32, 32, 8, // Darker gray, 3.125% opaque
        0, 0, 0, 0, // Black, transparent
    ];

    let image = Image::from_rgba_data(rgba_data, 4, 2)?;

    // Create stencil mask with threshold of 128
    let mask = image.create_mask(MaskType::Stencil, Some(128));
    assert!(mask.is_some());

    if let Some(stencil) = mask {
        // Stencil should be 1-bit depth
        assert_eq!(stencil.bits_per_component(), 1);
    }

    Ok(())
}
