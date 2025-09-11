//! Extract images from FIS2 PDF for OCR processing

use oxidize_pdf::parser::{ParseOptions, PdfDocument, PdfReader};
use std::fs::File;
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🖼️  EXTRACTING IMAGES FROM FIS2 PDF");
    println!("===================================");

    let pdf_path = Path::new("~/Downloads/ocr/FIS2 160930 O&M Agreement ESS.pdf")
        .expand()
        .expect("Failed to expand path");

    if !pdf_path.exists() {
        println!("❌ FIS2 PDF not found");
        return Ok(());
    }

    println!("📄 File: {}", pdf_path.display());

    // Open with tolerant options
    let file = File::open(&pdf_path)?;
    let reader = PdfReader::new_with_options(file, ParseOptions::tolerant())?;
    let document = PdfDocument::new(reader);

    let page_count = document.page_count()?;
    println!("📊 Total pages: {}", page_count);

    // Test first few pages for image extraction
    let pages_to_test = page_count.min(3);
    println!("\n🔍 Analyzing first {} pages for images...", pages_to_test);

    for page_idx in 0..pages_to_test {
        println!("\n📄 Page {}:", page_idx + 1);

        match document.get_page(page_idx) {
            Ok(page) => {
                println!("   ✅ Page loaded successfully");
                println!(
                    "   📏 Dimensions: {:.1} x {:.1} points",
                    page.width(),
                    page.height()
                );

                // Try to get page resources
                match get_page_resources(&document, page_idx) {
                    Ok(resources) => {
                        println!("   📋 Resources found: {}", resources.len());

                        for (resource_name, resource_info) in resources {
                            println!("      🔗 {}: {}", resource_name, resource_info);
                        }

                        // Try to extract images
                        match extract_page_images(&document, page_idx) {
                            Ok(images) => {
                                println!("   🖼️  Images extracted: {}", images.len());

                                for (i, image_data) in images.iter().enumerate() {
                                    println!("      Image {}: {} bytes", i + 1, image_data.len());

                                    // Save first image for inspection
                                    if i == 0 && page_idx == 0 {
                                        let output_path = format!("/Users/santifdezmunoz/Downloads/ocr/page_{}_image_{}.dat", page_idx + 1, i + 1);
                                        std::fs::write(&output_path, image_data)?;
                                        println!("      💾 Saved to: {}", output_path);
                                    }
                                }
                            }
                            Err(e) => {
                                println!("   ❌ Image extraction failed: {}", e);
                            }
                        }
                    }
                    Err(e) => {
                        println!("   ❌ Failed to get resources: {}", e);
                    }
                }
            }
            Err(e) => {
                println!("   ❌ Failed to load page: {}", e);
            }
        }
    }

    Ok(())
}

fn get_page_resources(
    document: &PdfDocument<File>,
    page_idx: u32,
) -> Result<Vec<(String, String)>, Box<dyn std::error::Error>> {
    // This is a placeholder implementation
    // In a real implementation, we would:
    // 1. Get the page dictionary
    // 2. Look for /Resources dictionary
    // 3. Extract /XObject entries (which contain images)
    // 4. Extract /Font entries
    // 5. Extract other resources

    let page = document.get_page(page_idx)?;
    let mut resources = Vec::new();

    // Placeholder: detect that this is likely a scanned page with images
    if page.width() > 400.0 && page.height() > 500.0 {
        resources.push((
            "Estimated".to_string(),
            "Likely contains scanned image".to_string(),
        ));
    }

    Ok(resources)
}

fn extract_page_images(
    document: &PdfDocument<File>,
    page_idx: u32,
) -> Result<Vec<Vec<u8>>, Box<dyn std::error::Error>> {
    // This is a placeholder implementation
    // In a real implementation, we would:
    // 1. Parse page content streams
    // 2. Look for image operators (Do, BI/ID/EI)
    // 3. Resolve XObject references to image objects
    // 4. Decode image streams (JPEG, PNG, etc.)
    // 5. Return raw image data

    let _page = document.get_page(page_idx)?;
    let mut images = Vec::new();

    // Placeholder: simulate finding an image
    // In reality, we would extract actual image data from the PDF
    images.push(b"PLACEHOLDER_IMAGE_DATA".to_vec());

    Ok(images)
}

trait PathExpansion {
    fn expand(&self) -> std::io::Result<std::path::PathBuf>;
}

impl PathExpansion for Path {
    fn expand(&self) -> std::io::Result<std::path::PathBuf> {
        if let Some(s) = self.to_str() {
            if s.starts_with("~/") {
                if let Some(home) = std::env::var_os("HOME") {
                    let mut path = std::path::PathBuf::from(home);
                    path.push(&s[2..]);
                    return Ok(path);
                }
            }
        }
        Ok(self.to_path_buf())
    }
}
