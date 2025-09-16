//! Test OCR functionality with specific date extraction
//! Tests extraction of the text "30 September 2016" from images

#[cfg(feature = "ocr-tesseract")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use std::path::Path;
    use std::io::Cursor;
    use oxidize_pdf::text::{RustyTesseractProvider, OcrOptions, OcrProvider};
    use oxidize_pdf::graphics::ImageFormat;

    println!("🔍 OCR DATE EXTRACTION TEST");
    println!("===========================");
    println!("Target text: '30 September 2016'");

    // Create OCR provider optimized for contract documents
    let ocr_provider = match RustyTesseractProvider::for_contracts() {
        Ok(provider) => {
            println!("✅ Tesseract OCR Provider ready (contracts optimized)");
            provider
        }
        Err(e) => {
            println!("❌ Cannot initialize Tesseract: {}", e);
            println!("   Make sure tesseract is installed: brew install tesseract");
            return Ok(());
        }
    };

    // Create a simple test image with the target text
    // For now we'll create a synthetic test, but in production you'd use real scanned images
    let test_image_data = create_test_image_with_date()?;

    println!("\n🖼️  Processing test image...");

    // Setup OCR options
    let ocr_options = OcrOptions {
        min_confidence: 0.3,
        preserve_layout: true,
        language: "eng".to_string(),
        ..Default::default()
    };

    // Process the image
    match ocr_provider.process_image(&test_image_data, &ocr_options) {
        Ok(result) => {
            println!("✅ OCR PROCESSING SUCCESSFUL!");
            println!("   📝 Text length: {} characters", result.text.len());
            println!("   📈 Confidence: {:.1}%", result.confidence * 100.0);
            println!("   ⏱️  Processing time: {} ms", result.processing_time_ms);
            println!("   🔧 Engine: {}", result.engine_name);

            println!("\n📖 EXTRACTED TEXT:");
            println!("==================");
            println!("\"{}\"", result.text);

            // Check if our target date is found
            if result.text.contains("30 September 2016") {
                println!("\n🎉 SUCCESS! Target date '30 September 2016' found!");
            } else if result.text.contains("30") && result.text.contains("September") && result.text.contains("2016") {
                println!("\n⚠️  PARTIAL MATCH: Found date components separately");
                println!("    Looking for variations...");

                // Check common OCR misreadings
                let variations = vec![
                    "30 September 2016",
                    "3O September 2016", // O instead of 0
                    "30 September 201 6", // space in year
                    "30 September 20l6", // l instead of 1
                    "30 Sept 2016",
                    "30/09/2016",
                    "2016-09-30"
                ];

                for variant in &variations {
                    if result.text.contains(variant) {
                        println!("    ✅ Found variant: '{}'", variant);
                    }
                }
            } else {
                println!("\n❌ TARGET DATE NOT FOUND");
                println!("    Searching for date-related patterns...");

                // Look for any dates
                use regex::Regex;
                let date_patterns = vec![
                    r"\d{1,2}\s+\w+\s+\d{4}",  // "30 September 2016"
                    r"\d{1,2}/\d{1,2}/\d{4}",  // "30/09/2016"
                    r"\d{4}-\d{2}-\d{2}",      // "2016-09-30"
                    r"\w+\s+\d{1,2},?\s+\d{4}", // "September 30, 2016"
                ];

                for pattern in &date_patterns {
                    if let Ok(regex) = Regex::new(pattern) {
                        for captures in regex.find_iter(&result.text) {
                            println!("    📅 Found date pattern: '{}'", captures.as_str());
                        }
                    }
                }
            }

            // Show fragments if available
            if !result.fragments.is_empty() {
                println!("\n🔤 TEXT FRAGMENTS ({})", result.fragments.len());
                for (i, fragment) in result.fragments.iter().enumerate() {
                    if i < 5 { // Show first 5 fragments
                        println!("  {}: \"{}\" (conf: {:.1}%)",
                            i + 1,
                            fragment.text.trim(),
                            fragment.confidence * 100.0
                        );
                    }
                }
            }
        }
        Err(e) => {
            println!("❌ OCR PROCESSING FAILED: {}", e);
        }
    }

    println!("\n🏁 Date extraction test completed!");
    Ok(())
}

#[cfg(feature = "ocr-tesseract")]
fn create_test_image_with_date() -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    // For this test, we'll create a simple white image with black text
    // In a real scenario, you'd load an actual scanned document

    println!("📝 Creating synthetic test image with text '30 September 2016'...");

    // Create a simple 400x200 PNG with white background and black text
    // This is a minimal implementation - in production you'd use an image library

    // For now, let's create a minimal test case that would work with real images
    // We'll return empty data and suggest using real images

    println!("⚠️  Using synthetic test - for real testing, provide actual scanned images");
    println!("   Suggested: Place test images in /Users/santifdezmunoz/Downloads/ocr/");

    // Return minimal PNG data (this won't actually contain readable text)
    let png_data = create_minimal_png();
    Ok(png_data)
}

#[cfg(feature = "ocr-tesseract")]
fn create_minimal_png() -> Vec<u8> {
    // Minimal PNG file structure (8x8 transparent PNG)
    vec![
        0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, // PNG signature
        0x00, 0x00, 0x00, 0x0D, // IHDR length
        0x49, 0x48, 0x44, 0x52, // IHDR
        0x00, 0x00, 0x00, 0x08, // Width: 8
        0x00, 0x00, 0x00, 0x08, // Height: 8
        0x08, 0x02, 0x00, 0x00, 0x00, // Bit depth, Color type, Compression, Filter, Interlace
        0x4B, 0x6D, 0x29, 0xDC, // CRC
        0x00, 0x00, 0x00, 0x09, // IDAT length
        0x49, 0x44, 0x41, 0x54, // IDAT
        0x08, 0x1D, 0x01, 0x02, 0x00, 0xFD, 0xFF, 0x00, 0x00, // Compressed data
        0x00, 0x00, 0x00, 0x00, // IEND length
        0x49, 0x45, 0x4E, 0x44, // IEND
        0xAE, 0x42, 0x60, 0x82  // CRC
    ]
}

#[cfg(not(feature = "ocr-tesseract"))]
fn main() {
    println!("❌ OCR feature not enabled");
    println!("💡 Enable with: cargo run --example test_ocr_date_extraction --features ocr-tesseract");
}