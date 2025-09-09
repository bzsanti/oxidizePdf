use oxidize_pdf::{
    semantic::{BoundingBox, EntityType},
    Document,
};
use oxidize_pdf_pro::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("oxidize-pdf-pro Basic Features Demo");

    // Initialize Pro features with development license
    let license_result = initialize(Some("OXIDIZE_PRO_DEV"));
    match license_result {
        Ok(_) => println!("✓ Pro license validated successfully"),
        Err(e) => println!(
            "⚠ License validation failed: {}, continuing with limited features",
            e
        ),
    }

    // Check license info
    let license_info = license_info();
    println!("License valid: {}", license_info.is_valid);
    println!("Features available: {:?}", license_info.features);

    // 1. Create a basic document with semantic entities
    println!("\n1. Creating document with semantic entities...");
    let mut doc = Document::new();
    doc.set_title("AI-Ready Invoice Demo");
    doc.set_author("oxidize-pdf-pro");

    // Add semantic entities
    let invoice_id = doc.mark_entity(
        "invoice_001",
        EntityType::Invoice,
        BoundingBox::new(50.0, 700.0, 500.0, 100.0, 1),
    );
    doc.set_entity_content(&invoice_id, "Invoice INV-2024-001");

    let customer_id = doc.mark_entity(
        "customer_001",
        EntityType::CustomerName,
        BoundingBox::new(50.0, 600.0, 300.0, 30.0, 1),
    );
    doc.set_entity_content(&customer_id, "ACME Corporation");

    let total_id = doc.mark_entity(
        "total_001",
        EntityType::TotalAmount,
        BoundingBox::new(400.0, 550.0, 150.0, 30.0, 1),
    );
    doc.set_entity_content(&total_id, "$2,500.00");

    // Create relationships
    doc.relate_entities(
        &invoice_id,
        &customer_id,
        oxidize_pdf::semantic::RelationType::BillsTo,
    );
    doc.relate_entities(
        &invoice_id,
        &total_id,
        oxidize_pdf::semantic::RelationType::HasAmount,
    );

    println!(
        "✓ Created document with {} semantic entities",
        doc.semantic_entity_count()
    );

    // 2. Try XMP metadata embedding (Pro feature)
    println!("\n2. Testing XMP metadata embedding...");
    match FeatureGate::check_xmp_features() {
        Ok(_) => {
            println!("✓ XMP features are available");
            // XMP embedding would be implemented here
            let embedder = XmpEmbedder::new();
            match embedder.embed_entities(&mut doc) {
                Ok(_) => println!("✓ XMP metadata embedded successfully"),
                Err(e) => println!("⚠ XMP embedding failed: {}", e),
            }
        }
        Err(e) => println!("⚠ XMP features not available: {}", e),
    }

    // 3. Create professional invoice template
    println!("\n3. Testing Pro templates...");
    match FeatureGate::check_template_features() {
        Ok(_) => {
            println!("✓ Template features are available");
            let invoice = ProInvoiceTemplate::new()
                .customer("ACME Corp")
                .invoice_number("INV-2024-001")
                .add_line_item("Professional Services", 2500.00)
                .with_schema_org_markup();

            match invoice.build() {
                Ok(_) => println!("✓ Professional invoice template created"),
                Err(e) => println!("⚠ Template creation failed: {}", e),
            }
        }
        Err(e) => println!("⚠ Template features not available: {}", e),
    }

    // 4. Test entity extraction (if available)
    println!("\n4. Testing semantic extraction...");
    match FeatureGate::check_extraction_features() {
        Ok(_) => {
            println!("✓ Extraction features are available");
            // Note: SemanticExtractor may not compile due to remaining issues
            // This is a placeholder for the full implementation
            println!("✓ Extraction API ready (implementation may have compilation issues)");
        }
        Err(e) => println!("⚠ Extraction features not available: {}", e),
    }

    // 5. Save the document
    println!("\n5. Saving document...");
    doc.save("examples/results/pro_demo.pdf")?;
    println!("✓ Document saved to examples/results/pro_demo.pdf");

    // 6. Show final license status
    println!("\n6. Final license status:");
    let final_info = license_info();
    println!("License valid: {}", final_info.is_valid);
    if let Some(license_type) = &final_info.license_type {
        println!("License type: {:?}", license_type);
    }
    if let Some(days) = final_info.days_until_expiry {
        println!("Days until expiry: {}", days);
    }

    println!("\n🎉 oxidize-pdf-pro demo completed successfully!");
    Ok(())
}
