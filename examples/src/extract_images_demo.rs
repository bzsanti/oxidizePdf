//! Example: Extract images from a PDF
//!
//! Demonstrates how to extract all images from a PDF file and save them to disk.

use oxidize_pdf::{extract_images_from_pdf, ExtractImagesOptions};
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Image Extraction Example ===\n");

    // Configure extraction options
    let options = ExtractImagesOptions {
        output_dir: PathBuf::from("examples/results/extracted_images"),
        name_pattern: "page_{page}_image_{index}.{format}".to_string(),
        extract_inline: true,
        min_size: Some(10), // Skip images smaller than 10x10 pixels
        create_dir: true,
        ..Default::default()
    };

    // Extract images from a PDF
    let input_pdf = "tests/fixtures/sample.pdf";

    match extract_images_from_pdf(input_pdf, options) {
        Ok(images) => {
            println!("Successfully extracted {} images:\n", images.len());

            for image in &images {
                println!(
                    "  Page {}, Image {}: {}x{} {} -> {}",
                    image.page_number + 1,
                    image.image_index + 1,
                    image.width,
                    image.height,
                    match image.format {
                        oxidize_pdf::graphics::ImageFormat::Jpeg => "JPEG",
                        oxidize_pdf::graphics::ImageFormat::Png => "PNG",
                        oxidize_pdf::graphics::ImageFormat::Tiff => "TIFF",
                        oxidize_pdf::graphics::ImageFormat::Raw => "RAW",
                    },
                    image.file_path.display()
                );
            }

            println!("\nImages extracted to: examples/results/extracted_images/");
        }
        Err(e) => {
            eprintln!("Error extracting images: {}", e);
            eprintln!("\nNote: This example requires a PDF file at tests/fixtures/sample.pdf");
            eprintln!("You can use any PDF file with images for testing.");
        }
    }

    Ok(())
}
