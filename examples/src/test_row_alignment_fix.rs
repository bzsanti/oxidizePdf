use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔧 Testing Row Alignment Fix for Extracted Images");

    // Find the problematic image
    let image_path = PathBuf::from("fis2_extracted_1169x1653.jpg");

    if !image_path.exists() {
        println!("⚠️  Image not found at current location, checking git status files...");

        // Try to find it in current directory
        let candidates = [
            "fis2_extracted_1169x1653.jpg",
            "./fis2_extracted_1169x1653.jpg",
            "../fis2_extracted_1169x1653.jpg",
            "examples/results/debug_extracted_1169x1653.jpg",
        ];

        let mut found_path = None;
        for candidate in &candidates {
            let path = PathBuf::from(candidate);
            if path.exists() {
                found_path = Some(path);
                break;
            }
        }

        match found_path {
            Some(path) => {
                println!("✅ Found image at: {}", path.display());
                test_row_realignment(&path)?;
            }
            None => {
                println!("❌ Could not find the problematic image file.");
                println!("Please ensure one of these files exists:");
                for candidate in &candidates {
                    println!("  - {candidate}");
                }
                return Ok(());
            }
        }
    } else {
        test_row_realignment(&image_path)?;
    }

    Ok(())
}

fn test_row_realignment(image_path: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    println!("🔍 Analyzing image: {}", image_path.display());

    // Read the raw image data first
    let raw_data = std::fs::read(image_path)?;
    println!("📊 Raw file size: {} bytes", raw_data.len());

    // If this is a JPEG, try to decode it and get raw pixel data
    #[cfg(feature = "external-images")]
    {
        use image::GenericImageView;

        let img = image::load_from_memory(&raw_data)?;
        let (width, height) = img.dimensions();

        println!("📐 Image dimensions: {}x{}", width, height);
        println!("🎨 Image format: {:?}", img.color());

        // Convert to grayscale for analysis
        let gray_img = img.to_luma8();
        let pixel_data = gray_img.as_raw();

        println!("📝 Pixel data length: {} bytes", pixel_data.len());

        // Try to detect if there's row misalignment by analyzing patterns
        detect_row_misalignment(pixel_data, width, height)?;

        // Apply our row stride correction
        let corrected_img = fix_row_alignment(pixel_data, width, height)?;

        // Save the corrected image
        let output_path = PathBuf::from("examples/results").join("corrected_alignment.png");
        std::fs::create_dir_all(output_path.parent().unwrap())?;

        corrected_img.save(&output_path)?;
        println!("💾 Saved corrected image to: {}", output_path.display());
    }

    #[cfg(not(feature = "external-images"))]
    {
        println!("⚠️  External-images feature not enabled. Run with --features external-images");
    }

    Ok(())
}

#[cfg(feature = "external-images")]
fn detect_row_misalignment(
    pixel_data: &[u8],
    width: u32,
    height: u32,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("🔍 Analyzing for row misalignment patterns...");

    let bytes_per_row = width as usize;

    // Sample a few rows to check for misalignment patterns
    let sample_rows = [height / 4, height / 2, 3 * height / 4];

    for &row_idx in &sample_rows {
        if row_idx >= height {
            continue;
        }

        let row_start = row_idx as usize * bytes_per_row;
        let row_end = row_start + bytes_per_row;

        if row_end <= pixel_data.len() {
            let row_data = &pixel_data[row_start..row_end];

            // Count transitions from white to black and black to white
            let mut transitions = 0;
            for i in 1..row_data.len() {
                if (row_data[i] > 128) != (row_data[i - 1] > 128) {
                    transitions += 1;
                }
            }

            println!("  Row {}: {} transitions", row_idx, transitions);

            // Show a sample of the row data
            let sample_len = (width / 10).min(20) as usize;
            let sample_data: Vec<String> = row_data[..sample_len]
                .iter()
                .map(|&b| {
                    if b > 128 {
                        "⬜".to_string()
                    } else {
                        "⬛".to_string()
                    }
                })
                .collect();

            println!("  Sample: {}", sample_data.join(""));
        }
    }

    Ok(())
}

#[cfg(feature = "external-images")]
fn fix_row_alignment(
    pixel_data: &[u8],
    width: u32,
    height: u32,
) -> Result<image::ImageBuffer<image::Luma<u8>, Vec<u8>>, Box<dyn std::error::Error>> {
    use image::{ImageBuffer, Luma};

    println!("🔧 Attempting to fix row alignment...");

    let mut corrected_data = Vec::new();
    let bytes_per_row = width as usize;

    // Try different row stride corrections
    let possible_strides = [
        bytes_per_row,            // Original
        (bytes_per_row + 1) & !1, // 2-byte aligned
        (bytes_per_row + 3) & !3, // 4-byte aligned
        (bytes_per_row + 7) & !7, // 8-byte aligned
        bytes_per_row + 1,        // +1 padding
        bytes_per_row + 2,        // +2 padding
        bytes_per_row + 4,        // +4 padding
    ];

    // Try each stride and see which one gives the most reasonable result
    for (i, &stride) in possible_strides.iter().enumerate() {
        println!("  Trying stride {}: {} bytes per row", i, stride);

        let expected_total = stride * height as usize;
        if expected_total > pixel_data.len() {
            println!(
                "    ❌ Stride too large (need {} bytes, have {})",
                expected_total,
                pixel_data.len()
            );
            continue;
        }

        // Extract using this stride
        let mut test_data = Vec::new();
        for row in 0..height {
            let row_start = row as usize * stride;
            let row_end = row_start + bytes_per_row;

            if row_end <= pixel_data.len() {
                test_data.extend_from_slice(&pixel_data[row_start..row_end]);
            } else {
                // Fill with white if we run out of data
                test_data.resize(test_data.len() + bytes_per_row, 255);
            }
        }

        // Save this attempt for comparison
        if test_data.len() == (width * height) as usize {
            let test_img = ImageBuffer::<Luma<u8>, Vec<u8>>::from_raw(width, height, test_data);

            if let Some(img) = test_img {
                let output_path =
                    PathBuf::from("examples/results").join(format!("alignment_test_{}.png", i));
                std::fs::create_dir_all(output_path.parent().unwrap())?;
                img.save(&output_path)?;
                println!("    💾 Saved test to: {}", output_path.display());

                // Use the first working one as our corrected data
                if corrected_data.is_empty() {
                    corrected_data = img.into_raw();
                }
            }
        }
    }

    // Create the corrected image
    if corrected_data.is_empty() {
        // Fallback to original data
        corrected_data = pixel_data.to_vec();
    }

    let corrected_img = ImageBuffer::<Luma<u8>, Vec<u8>>::from_raw(width, height, corrected_data)
        .ok_or("Failed to create corrected image")?;

    Ok(corrected_img)
}
