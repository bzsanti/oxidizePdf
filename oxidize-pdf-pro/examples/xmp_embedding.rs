use oxidize_pdf::Document;
use oxidize_pdf_pro::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("XMP Embedding Example");

    // Initialize Pro features
    let license_result = initialize(Some("OXIDIZE_PRO_DEV"));
    match license_result {
        Ok(_) => println!("✓ Pro license validated successfully"),
        Err(e) => println!(
            "⚠ License validation failed: {}, continuing with limited features",
            e
        ),
    }

    // Create a document with semantic entities
    let mut doc = Document::new();
    doc.set_title("XMP Embedding Demo");
    doc.set_author("oxidize-pdf-pro");

    // Add some semantic entities
    let invoice_id = doc.mark_entity(
        "invoice_001",
        oxidize_pdf::semantic::EntityType::Invoice,
        oxidize_pdf::semantic::BoundingBox::new(50.0, 700.0, 500.0, 100.0, 1),
    );
    doc.set_entity_content(&invoice_id, "Invoice INV-2024-001");

    let customer_id = doc.mark_entity(
        "customer_001",
        oxidize_pdf::semantic::EntityType::CustomerName,
        oxidize_pdf::semantic::BoundingBox::new(50.0, 600.0, 300.0, 30.0, 1),
    );
    doc.set_entity_content(&customer_id, "ACME Corporation");

    // Try XMP embedding
    println!("\nTesting XMP metadata embedding...");
    match FeatureGate::check_xmp_features() {
        Ok(_) => {
            println!("✓ XMP features are available");
            let embedder = XmpEmbedder::new();
            match embedder.embed_entities(&mut doc) {
                Ok(_) => println!("✓ XMP metadata embedded successfully"),
                Err(e) => println!("⚠ XMP embedding failed: {}", e),
            }
        }
        Err(e) => println!("⚠ XMP features not available: {}", e),
    }

    // Save the document
    std::fs::create_dir_all("examples/results")?;
    doc.save("examples/results/xmp_demo.pdf")?;
    println!("✓ Document saved to examples/results/xmp_demo.pdf");

    Ok(())
}
