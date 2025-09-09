//! OCR Selective Regions Demo
//!
//! This example demonstrates how to use OCR with selective region processing.
//! It shows how to process only specific parts of a document instead of the entire image.

fn main() -> Result<(), Box<dyn std::error::Error>> {
    use oxidize_pdf::text::{MockOcrProvider, OcrOptions, OcrProvider, OcrRegion};

    println!("🔍 OCR Selective Regions Demo");
    println!("============================");

    // Initialize Mock OCR provider (works without external dependencies)
    let provider = MockOcrProvider::new();

    // Create OCR options
    let options = OcrOptions {
        language: "eng".to_string(),
        min_confidence: 0.7,
        preserve_layout: true,
        regions: None, // Will be set per region
        ..Default::default()
    };

    // Define regions of interest (these would typically come from document analysis)
    let regions = vec![
        OcrRegion::with_label(50, 50, 200, 100, "header"),
        OcrRegion::with_label(50, 200, 400, 150, "main_content"),
        OcrRegion::with_label(50, 400, 150, 80, "footer"),
        OcrRegion::with_label(300, 400, 200, 120, "sidebar"),
    ];

    println!("\n📋 Processing {} regions:", regions.len());
    for (i, region) in regions.iter().enumerate() {
        println!(
            "  {}. {}: {}x{} at ({}, {})",
            i + 1,
            region.label.as_ref().unwrap_or(&"unlabeled".to_string()),
            region.width,
            region.height,
            region.x,
            region.y
        );
    }

    // Create mock image data for each region (in practice, these would be actual cropped images)
    let mock_images = create_mock_region_images(&regions);

    // Create image-region pairs
    let image_region_pairs: Vec<(&[u8], &OcrRegion)> = mock_images
        .iter()
        .zip(regions.iter())
        .map(|(img, region)| (img.as_slice(), region))
        .collect();

    // Process all regions at once
    println!("\n⚡ Processing all regions...");
    let start_time = std::time::Instant::now();

    let results = provider.process_image_regions(&image_region_pairs, &options)?;

    let total_time = start_time.elapsed();
    println!(
        "✅ Processed {} regions in {:.2}ms",
        results.len(),
        total_time.as_millis()
    );

    // Display results for each region
    println!("\n📊 OCR Results by Region:");
    println!("{}", "=".repeat(50));

    for (i, result) in results.iter().enumerate() {
        let region = &regions[i];
        let label = region.label.as_deref().unwrap_or("unlabeled");

        println!("\n📍 Region {}: {} ", i + 1, label);
        println!("   Confidence: {:.1}%", result.confidence * 100.0);
        println!("   Text: \"{}\"", result.text.trim());
        println!("   Fragments: {}", result.fragments.len());

        // Show fragment details
        for (j, fragment) in result.fragments.iter().enumerate() {
            println!(
                "     {}. \"{}\" at ({:.0}, {:.0}) - {:.1}%",
                j + 1,
                fragment.text.trim(),
                fragment.x,
                fragment.y,
                fragment.confidence * 100.0
            );
        }

        if let Some(processed_region) = &result.processed_region {
            println!(
                "   Processed region: {}x{} at ({}, {})",
                processed_region.width,
                processed_region.height,
                processed_region.x,
                processed_region.y
            );
        }
    }

    // Demonstrate region filtering
    demonstrate_region_filtering(&results, &regions)?;

    // Performance analysis
    analyze_performance(&results, total_time)?;

    Ok(())
}

fn create_mock_region_images(regions: &[oxidize_pdf::text::OcrRegion]) -> Vec<Vec<u8>> {
    // Create different mock JPEG data for different region types
    regions
        .iter()
        .map(|region| {
            let label = region.label.as_deref().unwrap_or("default");

            match label {
                "header" => create_mock_jpeg("INVOICE #2024-001"),
                "main_content" => {
                    create_mock_jpeg("Customer: John Doe\nAmount: $1,234.56\nDate: 2024-03-15")
                }
                "footer" => create_mock_jpeg("Page 1 of 1"),
                "sidebar" => create_mock_jpeg("Tax ID: 12-3456789\nTotal Due: $1,234.56"),
                _ => create_mock_jpeg("Sample text content"),
            }
        })
        .collect()
}

fn create_mock_jpeg(text_hint: &str) -> Vec<u8> {
    // Create a minimal JPEG header that will be processed by the mock OCR
    // In practice, this would be actual image data
    let mut jpeg_data = vec![
        0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10, 0x4A, 0x46, 0x49, 0x46, 0x00, 0x01, 0x01, 0x01, 0x00,
        0x48, 0x00, 0x48, 0x00, 0x00,
    ];

    // Add some variation based on the text hint to make results different
    let variation = text_hint.len() % 256;
    jpeg_data.push(variation as u8);

    // JPEG end marker
    jpeg_data.extend_from_slice(&[0xFF, 0xD9]);

    jpeg_data
}

fn demonstrate_region_filtering(
    results: &[oxidize_pdf::text::OcrProcessingResult],
    regions: &[oxidize_pdf::text::OcrRegion],
) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n🔍 Region Filtering Examples:");
    println!("{}", "-".repeat(40));

    // Filter by region type
    let header_results: Vec<_> = results
        .iter()
        .zip(regions.iter())
        .filter(|(_, region)| {
            region
                .label
                .as_ref()
                .map_or(false, |label| label == "header")
        })
        .collect();

    println!("Header regions: {}", header_results.len());
    for (result, _region) in header_results {
        println!(
            "  - \"{}\": {:.1}% confidence",
            result.text.trim(),
            result.confidence * 100.0
        );
    }

    // Filter by confidence threshold
    let high_confidence_results: Vec<_> = results
        .iter()
        .filter(|result| result.confidence > 0.8)
        .collect();

    println!(
        "High confidence regions (>80%): {}",
        high_confidence_results.len()
    );
    for result in high_confidence_results {
        let region_label = result
            .processed_region
            .as_ref()
            .and_then(|r| r.label.as_ref())
            .map_or("unlabeled", |v| v.as_str());
        println!(
            "  - {}: \"{}\": {:.1}%",
            region_label,
            result.text.trim(),
            result.confidence * 100.0
        );
    }

    Ok(())
}

fn analyze_performance(
    results: &[oxidize_pdf::text::OcrProcessingResult],
    total_time: std::time::Duration,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n⚡ Performance Analysis:");
    println!("{}", "-".repeat(40));

    let total_fragments: usize = results.iter().map(|r| r.fragments.len()).sum();
    let avg_confidence: f64 =
        results.iter().map(|r| r.confidence).sum::<f64>() / results.len() as f64;
    let total_processing_time: u64 = results.iter().map(|r| r.processing_time_ms).sum();

    println!("Total regions processed: {}", results.len());
    println!("Total text fragments: {}", total_fragments);
    println!("Average confidence: {:.1}%", avg_confidence * 100.0);
    println!("Total processing time: {}ms", total_processing_time);
    println!("Wall clock time: {}ms", total_time.as_millis());
    println!(
        "Average time per region: {:.1}ms",
        total_processing_time as f64 / results.len() as f64
    );

    // Calculate throughput
    if total_time.as_millis() > 0 {
        let regions_per_second = (results.len() as f64 * 1000.0) / total_time.as_millis() as f64;
        println!("Throughput: {:.1} regions/second", regions_per_second);
    }

    Ok(())
}
