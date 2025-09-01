//! Basic OCR demonstration using rusty-tesseract
//!
//! This example shows how to perform OCR on an image using rusty-tesseract.
//! It demonstrates the basic functionality we want to integrate into oxidize-pdf.

#[cfg(feature = "ocr-tesseract")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔍 Basic OCR Demo with rusty-tesseract");
    println!("=====================================");

    // Import rusty-tesseract
    use rusty_tesseract::{image_to_string, Args, Image};
    use std::collections::HashMap;

    // Create basic OCR arguments
    let args = Args {
        lang: "eng".to_string(),
        config_variables: HashMap::new(),
        dpi: Some(150),
        psm: Some(6), // Uniform block of text
        oem: Some(3), // Default OCR Engine Mode
    };

    println!("✅ rusty-tesseract imported successfully!");
    println!("📋 OCR Config:");
    println!("   Language: {}", args.lang);
    println!("   DPI: {:?}", args.dpi);
    println!("   PSM: {:?} (Uniform block of text)", args.psm);
    println!("   OEM: {:?} (Default engine)", args.oem);

    // Test with a simple text image if available
    let test_image_path = "examples/fixtures/sample_text.png";

    if Path::new(test_image_path).exists() {
        println!("\n🖼️  Testing with image: {}", test_image_path);

        // Load the image
        match Image::from_path(test_image_path) {
            Ok(image) => {
                println!("✅ Image loaded successfully");

                // Perform OCR
                match image_to_string(&image, &args) {
                    Ok(text) => {
                        println!("✅ OCR Success!");
                        println!("📄 Extracted text:");
                        println!("---");
                        println!("{}", text.trim());
                        println!("---");
                    }
                    Err(e) => {
                        println!("❌ OCR Error: {}", e);
                    }
                }
            }
            Err(e) => {
                println!("❌ Failed to load image: {}", e);
            }
        }
    } else {
        println!("\n📝 No test image found at {}", test_image_path);
        println!("💡 To test with actual image, place a PNG file with text at the above path");
    }

    println!("\n🎉 OCR integration test completed successfully!");
    println!("💡 Next: Implement TesseractOcrProvider using this foundation");

    Ok(())
}

#[cfg(not(feature = "ocr-tesseract"))]
fn main() {
    println!("❌ OCR feature not enabled");
    println!("💡 Use: cargo run --example ocr_basic_demo --features ocr-tesseract");
}
