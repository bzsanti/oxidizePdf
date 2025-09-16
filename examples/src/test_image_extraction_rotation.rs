use oxidize_pdf::operations::extract_images::{extract_images_from_pdf, ExtractImagesOptions};
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔍 Testing image extraction to identify rotation issue...");

    // Test files path - try both PDFs
    let test_pdfs = vec![
        "/Users/santifdezmunoz/Downloads/ocr/FIS2 160930 O&M Agreement ESS.pdf",
        "/Users/santifdezmunoz/Downloads/ocr/MADRIDEJOS_O&M CONTRACT_2013.pdf",
    ];

    let mut test_pdf = None;
    for pdf_path in test_pdfs {
        if std::path::Path::new(pdf_path).exists() {
            println!("📄 Trying PDF: {}", pdf_path);
            test_pdf = Some(pdf_path);
            break;
        }
    }

    let test_pdf = test_pdf.expect("No test PDF found");

    // Create output directory for extracted images
    let output_dir = PathBuf::from("examples/results");
    if !output_dir.exists() {
        std::fs::create_dir_all(&output_dir)?;
    }

    // Extract images from PDF with detailed debug output
    let extract_options = ExtractImagesOptions {
        output_dir: output_dir.clone(),
        name_pattern: "debug_extracted_{width}x{height}.{format}".to_string(),
        extract_inline: true,
        min_size: Some(100), // Only extract images > 100px
        create_dir: true,
    };

    println!("📄 Extracting images from: {}", test_pdf);

    let extracted_images = extract_images_from_pdf(test_pdf, extract_options)?;

    println!("✅ Extracted {} images:", extracted_images.len());
    for (idx, img) in extracted_images.iter().enumerate() {
        println!(
            "  [{}] Original dimensions: {}x{} -> {}",
            idx + 1,
            img.width,
            img.height,
            img.file_path.display()
        );

        // Check if this might be the rotated image
        if img.width > img.height && (img.width as f32 / img.height as f32) > 1.3 {
            println!("    📊 Landscape image detected (might need rotation check)");
        } else if img.height > img.width && (img.height as f32 / img.width as f32) > 1.3 {
            println!("    📊 Portrait image detected");
        }

        // Only process first few images to test
        if idx >= 2 {
            break;
        }
    }

    // Show file information for debugging
    if let Some(first_image) = extracted_images.first() {
        let image_data = std::fs::read(&first_image.file_path)?;
        println!("\n🔍 First image details:");
        println!("  📁 File size: {} bytes", image_data.len());
        println!(
            "  📐 Reported dimensions: {}x{}",
            first_image.width, first_image.height
        );
        println!("  🎨 Format: {:?}", first_image.format);

        // Try to detect actual image format and dimensions
        if image_data.len() >= 8 {
            if &image_data[0..8] == b"\x89PNG\r\n\x1a\n" {
                println!("  ✅ Confirmed PNG format");
                // For PNG, we could parse header to get actual dimensions
            } else if image_data.len() >= 2 && image_data[0] == 0xFF && image_data[1] == 0xD8 {
                println!("  ✅ Confirmed JPEG format");
            }
        }

        println!("  💡 This image will be used to test OCR rotation correction");
    }

    println!("\n🎯 Next steps:");
    println!("1. Check if the extracted image appears rotated in a viewer");
    println!("2. Implement rotation detection and correction");
    println!("3. Test OCR on corrected images");

    Ok(())
}
