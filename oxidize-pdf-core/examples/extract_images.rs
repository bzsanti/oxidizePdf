//! Example of extracting images from a PDF file

use oxidize_pdf::operations::{extract_images_from_pdf, ExtractImagesOptions};
use std::env;
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Get PDF file path from command line arguments
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <pdf_file>", args[0]);
        std::process::exit(1);
    }

    let pdf_path = &args[1];
    let output_dir = PathBuf::from("extracted_images");

    println!("Extracting images from: {pdf_path}");
    println!("Output directory: {}", output_dir.display());

    // Configure extraction options
    let options = ExtractImagesOptions {
        output_dir,
        name_pattern: "page_{page:03}_image_{index:02}.{format}".to_string(),
        extract_inline: true,
        min_size: Some(10), // Skip very small images
        create_dir: true,
    };

    // Extract images
    match extract_images_from_pdf(pdf_path, options) {
        Ok(images) => {
            println!("Successfully extracted {} images:", images.len());
            for image in images {
                println!(
                    "  - Page {}, Image {}: {} ({}x{} pixels)",
                    image.page_number + 1,
                    image.image_index + 1,
                    image.file_path.display(),
                    image.width,
                    image.height
                );
            }
        }
        Err(e) => {
            eprintln!("Error extracting images: {e}");
            std::process::exit(1);
        }
    }

    Ok(())
}
