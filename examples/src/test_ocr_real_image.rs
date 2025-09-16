//! Test OCR functionality with real image containing "30 September 2016"

#[cfg(feature = "ocr-tesseract")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use std::path::Path;
    use std::fs;
    use oxidize_pdf::text::{RustyTesseractProvider, OcrOptions, OcrProvider};

    println!("🔍 OCR REAL IMAGE TEST");
    println!("======================");
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

    // Test with the created image
    let test_image_path = "/Users/santifdezmunoz/Downloads/ocr/test_date_image.png";

    if !Path::new(test_image_path).exists() {
        println!("❌ Test image not found: {}", test_image_path);
        println!("   Run the Python script first to create the test image");
        return Ok(());
    }

    println!("\n🖼️  Loading test image: {}", test_image_path);

    // Read the image data
    let image_data = fs::read(test_image_path)?;
    println!("   📊 Image size: {} bytes", image_data.len());

    // Setup OCR options optimized for clean text
    let ocr_options = OcrOptions {
        min_confidence: 0.3,
        preserve_layout: true,
        language: "eng".to_string(),
        ..Default::default()
    };

    println!("\n🔤 Running OCR processing...");

    // Process the image
    match ocr_provider.process_image(&image_data, &ocr_options) {
        Ok(result) => {
            println!("✅ OCR PROCESSING SUCCESSFUL!");
            println!("   📝 Text length: {} characters", result.text.len());
            println!("   📈 Confidence: {:.1}%", result.confidence * 100.0);
            println!("   ⏱️  Processing time: {} ms", result.processing_time_ms);
            println!("   🔧 Engine: {}", result.engine_name);
            println!("   🌐 Language: {}", result.language);

            println!("\n📖 EXTRACTED TEXT:");
            println!("==================");
            println!("\"{}\"", result.text);

            // Check if our target date is found
            let target_date = "30 September 2016";
            if result.text.contains(target_date) {
                println!("\n🎉 SUCCESS! Target date '{}' found!", target_date);

                // Find the position and context
                if let Some(pos) = result.text.find(target_date) {
                    let start = pos.saturating_sub(20);
                    let end = (pos + target_date.len() + 20).min(result.text.len());
                    let context = &result.text[start..end];
                    println!("   📍 Context: \"...{}...\"", context);
                }

                return Ok(());
            }

            // Check for variations and common OCR errors
            println!("\n🔍 TARGET DATE NOT FOUND - Checking variations...");

            let variations = vec![
                "30 September 2016",
                "3O September 2016", // O instead of 0
                "30 September 201 6", // space in year
                "30 September 20l6", // l instead of 1
                "30 September 2Ol6", // O instead of 0 in year
                "30 Sept 2016",
                "30-September-2016",
                "30/09/2016",
                "2016-09-30",
                "September 30, 2016",
                "Sep 30 2016"
            ];

            let mut found_variation = false;
            for variant in &variations {
                if result.text.contains(variant) {
                    println!("   ✅ Found variant: '{}'", variant);
                    found_variation = true;
                }
            }

            if !found_variation {
                // Check for individual components
                println!("\n🔍 Checking for date components:");
                let components = ["30", "September", "Sept", "2016", "09"];
                for component in &components {
                    if result.text.contains(component) {
                        println!("   ✓ Found: '{}'", component);
                    } else {
                        println!("   ✗ Missing: '{}'", component);
                    }
                }

                // Look for any dates using regex
                use regex::Regex;
                println!("\n🔍 Looking for any date patterns:");
                let date_patterns = vec![
                    (r"\d{1,2}\s+\w+\s+\d{4}", "DD Month YYYY"),
                    (r"\d{1,2}/\d{1,2}/\d{4}", "DD/MM/YYYY"),
                    (r"\d{4}-\d{2}-\d{2}", "YYYY-MM-DD"),
                    (r"\w+\s+\d{1,2},?\s+\d{4}", "Month DD, YYYY"),
                    (r"\d{1,2}-\d{1,2}-\d{4}", "DD-MM-YYYY"),
                ];

                for (pattern, description) in &date_patterns {
                    if let Ok(regex) = Regex::new(pattern) {
                        for capture in regex.find_iter(&result.text) {
                            println!("   📅 Found {} pattern: '{}'", description, capture.as_str());
                        }
                    }
                }
            }

            // Show word-level confidence if available
            if !result.fragments.is_empty() {
                println!("\n🔤 TEXT FRAGMENTS ({}):", result.fragments.len());
                for (i, fragment) in result.fragments.iter().enumerate() {
                    if i < 10 { // Show first 10 fragments
                        println!("  {}: \"{}\" (conf: {:.1}%)",
                            i + 1,
                            fragment.text.trim().chars().take(50).collect::<String>(),
                            fragment.confidence * 100.0
                        );
                    }
                }
                if result.fragments.len() > 10 {
                    println!("  ... and {} more fragments", result.fragments.len() - 10);
                }
            }

        }
        Err(e) => {
            println!("❌ OCR PROCESSING FAILED: {}", e);

            // Provide troubleshooting suggestions
            println!("\n🛠️  TROUBLESHOOTING:");
            println!("   1. Verify tesseract is installed: which tesseract");
            println!("   2. Check tesseract version: tesseract --version");
            println!("   3. Test tesseract directly: tesseract {} output.txt", test_image_path);
            println!("   4. Check available languages: tesseract --list-langs");
        }
    }

    println!("\n🏁 OCR test completed!");
    Ok(())
}

#[cfg(not(feature = "ocr-tesseract"))]
fn main() {
    println!("❌ OCR feature not enabled");
    println!("💡 Enable with: cargo run --example test_ocr_real_image --features ocr-tesseract");
}