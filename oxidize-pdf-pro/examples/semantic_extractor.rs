use oxidize_pdf::Document;
use oxidize_pdf_pro::license::FeatureGate;
use oxidize_pdf_pro::prelude::*;
use oxidize_pdf_pro::{initialize, Result};

fn main() -> Result<()> {
    println!("Semantic Extraction Example");

    // Initialize Pro features
    let license_result = initialize(Some("OXIDIZE_PRO_DEV"));
    match license_result {
        Ok(_) => println!("✓ Pro license validated successfully"),
        Err(e) => println!(
            "⚠ License validation failed: {}, continuing with limited features",
            e
        ),
    }

    // Create a sample document with text content
    let mut doc = Document::new();
    doc.set_title("Invoice Sample for Extraction");
    doc.set_author("oxidize-pdf-pro");

    // Add some text content that we'll extract from
    // In a real scenario, this would be an existing PDF
    println!("\n1. Creating sample document with extractable content...");

    // Try semantic extraction
    println!("\n2. Testing semantic extraction...");
    match FeatureGate::check_extraction_features() {
        Ok(_) => {
            println!("✓ Extraction features are available");

            // Create an extractor
            let mut extractor = match SemanticExtractor::from_document(&doc) {
                Ok(extractor) => {
                    println!("✓ Extractor created successfully");
                    extractor
                }
                Err(e) => {
                    println!("⚠ Failed to create extractor: {}", e);
                    return Ok(());
                }
            };

            // Add custom patterns
            println!("✓ Adding custom extraction patterns...");
            let _ = extractor.add_custom_pattern(
                oxidize_pdf::semantic::EntityType::InvoiceNumber,
                r"(?i)invoice\s*#?\s*:?\s*([A-Z0-9-]+)",
            );

            // Perform extraction
            match extractor.extract_from_document(&doc) {
                Ok(result) => {
                    println!("✓ Extraction completed successfully");
                    println!("  - Entities found: {}", result.entities.len());
                    println!("  - Overall confidence: {:.2}", result.confidence_score);
                    println!("  - Processing time: {}ms", result.processing_time_ms);

                    // Show some results
                    for (id, entity) in result.entities.iter().take(5) {
                        println!(
                            "  - {} ({:?}): {:?}",
                            id,
                            entity.entity_type,
                            entity.content.as_ref().unwrap_or(&"No content".to_string())
                        );
                    }

                    // Export results
                    std::fs::create_dir_all("examples/results")?;
                    let json_export = result.export_to_json()?;
                    std::fs::write("examples/results/extraction_results.json", json_export)?;
                    println!("✓ Results exported to examples/results/extraction_results.json");
                }
                Err(e) => println!("⚠ Extraction failed: {}", e),
            }
        }
        Err(e) => println!("⚠ Extraction features not available: {}", e),
    }

    println!("\n3. Semantic extraction demo completed!");
    Ok(())
}
