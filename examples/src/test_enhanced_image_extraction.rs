use oxidize_pdf::operations::extract_images::{
    extract_images_from_pdf, ExtractImagesOptions, ImagePreprocessingOptions,
};
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔍 Testing Enhanced Image Extraction for Scanned PDFs");

    // Configure output directory
    let output_dir = PathBuf::from("examples/results");
    std::fs::create_dir_all(&output_dir)?;

    // Configure preprocessing options for scanned documents
    let preprocessing = ImagePreprocessingOptions {
        auto_correct_rotation: true,
        enhance_contrast: true,
        denoise: true,
        upscale_small_images: true,
        upscale_threshold: 500, // Upscale images smaller than 500px
        upscale_factor: 3,      // 3x upscaling for better OCR
        force_grayscale: true,  // Convert to grayscale for better text OCR
    };

    let options = ExtractImagesOptions {
        output_dir: output_dir.clone(),
        name_pattern: "enhanced_{page}_{index}.{format}".to_string(),
        extract_inline: true,
        min_size: Some(50), // Extract even small images
        create_dir: true,
        preprocessing,
    };

    // Test with the FIS2 PDF file (only first file for testing)
    let test_files = ["/Users/santifdezmunoz/Downloads/ocr/FIS2 160930 O&M Agreement ESS.pdf"];

    for pdf_file in &test_files {
        let pdf_path = PathBuf::from(pdf_file);

        if !pdf_path.exists() {
            println!("⚠️  File not found: {pdf_file}");
            continue;
        }

        println!("\n📄 Processing: {pdf_file}");

        match extract_images_from_pdf(&pdf_path, options.clone()) {
            Ok(extracted_images) => {
                println!(
                    "✅ Successfully extracted {} images",
                    extracted_images.len()
                );

                for image in &extracted_images {
                    println!(
                        "  📸 Page {}, Image {}: {}x{} -> {}",
                        image.page_number + 1,
                        image.image_index + 1,
                        image.width,
                        image.height,
                        image.file_path.display()
                    );
                }

                // Print some information about the PDF pages
                println!("  📑 Let's check if there are inline images or other image content...");
            }
            Err(e) => {
                println!("❌ Failed to extract images: {e}");
            }
        }
    }

    println!("\n🎯 Enhanced image extraction test completed!");
    println!("Check the 'examples/results' directory for extracted images");

    Ok(())
}

// Removed OCR function temporarily to focus on image extraction
