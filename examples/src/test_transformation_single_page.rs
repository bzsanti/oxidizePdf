use oxidize_pdf::operations::extract_images::{
    extract_images_from_pages, ExtractImagesOptions, ImagePreprocessingOptions,
};
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔧 Testing Transformation Matrix Application - Single Page");

    let output_dir = PathBuf::from("examples/results");
    std::fs::create_dir_all(&output_dir)?;

    let preprocessing = ImagePreprocessingOptions {
        auto_correct_rotation: true,
        enhance_contrast: true,
        denoise: false,              // Disable to see transformation effect clearly
        upscale_small_images: false, // Disable to see original size
        upscale_threshold: 500,
        upscale_factor: 3,
        force_grayscale: false, // Keep original colors
    };

    let options = ExtractImagesOptions {
        output_dir: output_dir.clone(),
        name_pattern: "transformed_{page}_{index}.{format}".to_string(),
        extract_inline: true,
        min_size: Some(50),
        create_dir: true,
        preprocessing,
    };

    let pdf_path =
        PathBuf::from("/Users/santifdezmunoz/Downloads/ocr/FIS2 160930 O&M Agreement ESS.pdf");

    if !pdf_path.exists() {
        println!("❌ FIS2 PDF not found");
        return Ok(());
    }

    println!("📄 Processing first page of: {}", pdf_path.display());

    // Extract only from page 0 (first page)
    match extract_images_from_pages(&pdf_path, &[0], options) {
        Ok(extracted_images) => {
            println!(
                "✅ Successfully extracted {} images from page 1",
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
        }
        Err(e) => {
            println!("❌ Failed to extract images: {e}");
        }
    }

    println!("\n🎯 Transformation test completed!");
    println!("Check the 'examples/results' directory for transformed images");

    Ok(())
}
