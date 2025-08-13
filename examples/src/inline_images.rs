//! Example demonstrating inline images in PDF content streams
//!
//! Inline images are small images that are embedded directly in the content stream
//! rather than stored as separate XObject resources. They're useful for small icons
//! or patterns according to ISO 32000-1 Section 8.9.7.

use oxidize_pdf::{Document, Page, Result};
use oxidize_pdf::parser::content::{ContentOperation, ContentParser};
use std::collections::HashMap;

fn main() -> Result<()> {
    // Create a new document
    let mut doc = Document::new();
    let mut page = Page::new(612.0, 792.0);
    
    // Get graphics context
    let mut graphics = page.graphics();
    
    // Title
    graphics
        .set_font_size(16.0)
        .move_to(50.0, 750.0)
        .show_text("Inline Images Example")?;
    
    // Explanation text
    graphics
        .set_font_size(10.0)
        .move_to(50.0, 720.0)
        .show_text("Inline images are embedded directly in the content stream")?
        .move_to(50.0, 705.0)
        .show_text("using the BI...ID...EI operators. They're best for small images.")?;
    
    // Example 1: Simple 1-bit checkerboard pattern (8x8 pixels)
    // This creates a small black and white pattern
    graphics
        .move_to(50.0, 650.0)
        .show_text("Example 1: 8x8 Checkerboard Pattern (1-bit)")?;
    
    // Inline image content stream
    // BI = Begin Image
    // /W = Width, /H = Height, /BPC = Bits Per Component, /CS = ColorSpace
    // ID = Image Data follows
    // EI = End Image
    let inline_image_1 = r#"
q
100 100 0 0 50 600 cm
BI
/W 8
/H 8
/BPC 1
/CS /G
ID
ÿÿÿÿÿÿÿÿ
EI
Q
"#;
    
    // Add raw content stream (in real implementation, would use proper API)
    graphics.add_raw_content(inline_image_1)?;
    
    // Example 2: Small grayscale gradient (4x4 pixels, 8-bit)
    graphics
        .move_to(50.0, 550.0)
        .show_text("Example 2: 4x4 Grayscale Gradient (8-bit)")?;
    
    let inline_image_2 = r#"
q
50 50 0 0 200 500 cm
BI
/W 4
/H 4
/BPC 8
/CS /G
ID
 @`€ @`€ @`€ @`€
EI
Q
"#;
    
    graphics.add_raw_content(inline_image_2)?;
    
    // Example 3: Small RGB color patch (2x2 pixels)
    graphics
        .move_to(50.0, 450.0)
        .show_text("Example 3: 2x2 RGB Color Patch")?;
    
    let inline_image_3 = r#"
q
100 100 0 0 350 400 cm
BI
/W 2
/H 2
/BPC 8
/CS /RGB
ID
ÿ  ÿÿ  ÿÿ ÿÿÿ
EI
Q
"#;
    
    graphics.add_raw_content(inline_image_3)?;
    
    // Example 4: Using abbreviated names
    graphics
        .move_to(50.0, 350.0)
        .show_text("Example 4: Using Abbreviated Names")?;
    
    // Demonstrate abbreviated inline image syntax
    // G = DeviceGray, RGB = DeviceRGB, AHx = ASCIIHexDecode
    let inline_image_4 = r#"
q
50 50 0 0 50 300 cm
BI
/W 4 /H 4 /CS /G /BPC 4 /F /AHx
ID
0123456789ABCDEF
0123456789ABCDEF
>
EI
Q
"#;
    
    graphics.add_raw_content(inline_image_4)?;
    
    // Add information about inline images
    graphics
        .set_font_size(10.0)
        .move_to(50.0, 250.0)
        .show_text("Inline Image Properties:")?
        .move_to(70.0, 235.0)
        .show_text("• Best for images < 4KB")?
        .move_to(70.0, 220.0)
        .show_text("• Embedded directly in content stream")?
        .move_to(70.0, 205.0)
        .show_text("• No reuse across pages")?
        .move_to(70.0, 190.0)
        .show_text("• Abbreviated key names allowed")?;
    
    // Comparison with XObject images
    graphics
        .move_to(50.0, 160.0)
        .show_text("XObject Images (Do operator):")?
        .move_to(70.0, 145.0)
        .show_text("• Better for larger images")?
        .move_to(70.0, 130.0)
        .show_text("• Stored in resource dictionary")?
        .move_to(70.0, 115.0)
        .show_text("• Can be reused multiple times")?
        .move_to(70.0, 100.0)
        .show_text("• Support for masks and transparency")?;
    
    // ISO compliance note
    graphics
        .set_font_size(8.0)
        .move_to(50.0, 70.0)
        .show_text("ISO 32000-1 Section 8.9.7 - Inline Images")?;
    
    // Add the page to the document
    doc.add_page(page);
    
    // Save the document
    doc.save("examples/results/inline_images.pdf")?;
    
    println!("✅ Inline images example created: examples/results/inline_images.pdf");
    println!("   - Demonstrates BI/ID/EI operators");
    println!("   - Shows abbreviated parameter names");
    println!("   - Includes different color spaces");
    println!("   - Compares with XObject images");
    
    // Demonstrate parsing inline images
    println!("\n📋 Parsing inline image example:");
    let test_content = b"BI /W 10 /H 10 /CS /RGB /BPC 8 ID \x00\x01\x02 EI";
    match ContentParser::parse(test_content) {
        Ok(ops) => {
            for op in ops {
                if let ContentOperation::InlineImage { params, data } = op {
                    println!("   Found inline image:");
                    println!("   - Width: {:?}", params.get("Width"));
                    println!("   - Height: {:?}", params.get("Height"));
                    println!("   - ColorSpace: {:?}", params.get("ColorSpace"));
                    println!("   - Data size: {} bytes", data.len());
                }
            }
        }
        Err(e) => println!("   Parse error: {}", e),
    }
    
    Ok(())
}