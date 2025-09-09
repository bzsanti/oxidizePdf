//! AI-Ready Invoice Demo
//!
//! This example demonstrates how to create a PDF invoice with semantic markup
//! that enables automated processing by AI/ML systems.

use oxidize_pdf::semantic::{BoundingBox, EntityType, RelationType};
use oxidize_pdf::{graphics::Color, text::Font, Document, Page};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 Creating AI-Ready Invoice PDF with semantic markup...");

    let mut doc = Document::new();
    doc.set_title("AI-Ready Invoice Demo");
    doc.set_author("oxidize-pdf AI Engine");
    doc.set_subject("Semantic PDF demonstration with entity markup");

    let mut page = Page::a4();

    // ==================== Visual Content ====================

    // Header
    page.text()
        .set_font(Font::Helvetica, 24.0)
        .set_fill_color(Color::rgb(0.2, 0.2, 0.2))
        .at(50.0, 750.0)
        .write("INVOICE")?;

    // Invoice number
    page.text()
        .set_font(Font::Helvetica, 14.0)
        .at(50.0, 720.0)
        .write("Invoice #: INV-2024-001")?;

    // Date
    page.text()
        .set_font(Font::Helvetica, 12.0)
        .at(50.0, 700.0)
        .write("Date: January 15, 2024")?;

    // Customer info
    page.text()
        .set_font(Font::Helvetica, 14.0)
        .set_fill_color(Color::rgb(0.1, 0.1, 0.1))
        .at(50.0, 660.0)
        .write("Bill To:")?;

    page.text()
        .set_font(Font::Helvetica, 12.0)
        .at(50.0, 640.0)
        .write("ACME Corporation")?;

    page.text()
        .set_font(Font::Helvetica, 10.0)
        .at(50.0, 620.0)
        .write("123 Business Street")?;

    page.text()
        .set_font(Font::Helvetica, 10.0)
        .at(50.0, 605.0)
        .write("New York, NY 10001")?;

    // Line items header
    page.text()
        .set_font(Font::Helvetica, 12.0)
        .at(50.0, 560.0)
        .write("Description")?;

    page.text()
        .set_font(Font::Helvetica, 12.0)
        .at(350.0, 560.0)
        .write("Quantity")?;

    page.text()
        .set_font(Font::Helvetica, 12.0)
        .at(420.0, 560.0)
        .write("Price")?;

    page.text()
        .set_font(Font::Helvetica, 12.0)
        .at(480.0, 560.0)
        .write("Amount")?;

    // Draw header line
    page.graphics()
        .set_line_width(1.0)
        .set_stroke_color(Color::rgb(0.5, 0.5, 0.5))
        .move_to(50.0, 555.0)
        .line_to(530.0, 555.0)
        .stroke();

    // Line item 1
    page.text()
        .set_font(Font::Helvetica, 11.0)
        .at(50.0, 530.0)
        .write("Professional Services - Q1 2024")?;

    page.text()
        .set_font(Font::Helvetica, 11.0)
        .at(370.0, 530.0)
        .write("1")?;

    page.text()
        .set_font(Font::Helvetica, 11.0)
        .at(420.0, 530.0)
        .write("$2,500.00")?;

    page.text()
        .set_font(Font::Helvetica, 11.0)
        .at(480.0, 530.0)
        .write("$2,500.00")?;

    // Line item 2
    page.text()
        .set_font(Font::Helvetica, 11.0)
        .at(50.0, 510.0)
        .write("Software License - Annual")?;

    page.text()
        .set_font(Font::Helvetica, 11.0)
        .at(370.0, 510.0)
        .write("1")?;

    page.text()
        .set_font(Font::Helvetica, 11.0)
        .at(420.0, 510.0)
        .write("$1,200.00")?;

    page.text()
        .set_font(Font::Helvetica, 11.0)
        .at(480.0, 510.0)
        .write("$1,200.00")?;

    // Subtotal
    page.text()
        .set_font(Font::Helvetica, 12.0)
        .at(420.0, 480.0)
        .write("Subtotal:")?;

    page.text()
        .set_font(Font::Helvetica, 12.0)
        .at(480.0, 480.0)
        .write("$3,700.00")?;

    // Tax
    page.text()
        .set_font(Font::Helvetica, 12.0)
        .at(420.0, 460.0)
        .write("Tax (8.25%):")?;

    page.text()
        .set_font(Font::Helvetica, 12.0)
        .at(480.0, 460.0)
        .write("$305.25")?;

    // Total
    page.text()
        .set_font(Font::Helvetica, 14.0)
        .set_fill_color(Color::rgb(0.0, 0.0, 0.0))
        .at(420.0, 430.0)
        .write("TOTAL:")?;

    page.text()
        .set_font(Font::Helvetica, 14.0)
        .set_fill_color(Color::rgb(0.0, 0.0, 0.0))
        .at(480.0, 430.0)
        .write("$4,005.25")?;

    // Due date
    page.text()
        .set_font(Font::Helvetica, 12.0)
        .at(50.0, 380.0)
        .write("Due Date: February 14, 2024")?;

    // Payment terms
    page.text()
        .set_font(Font::Helvetica, 10.0)
        .set_fill_color(Color::rgb(0.3, 0.3, 0.3))
        .at(50.0, 350.0)
        .write("Payment Terms: Net 30 days")?;

    // ==================== Semantic Markup ====================

    println!("📝 Adding semantic entity markup...");

    // Mark the main invoice region
    let invoice_id = doc.mark_entity(
        "main_invoice".to_string(),
        EntityType::Invoice,
        BoundingBox::new(50.0, 350.0, 480.0, 400.0, 1),
    );
    doc.set_entity_content(&invoice_id, "Complete invoice document");
    doc.add_entity_metadata(&invoice_id, "currency", "USD");
    doc.add_entity_metadata(&invoice_id, "invoice_date", "2024-01-15");
    doc.set_entity_confidence(&invoice_id, 1.0);

    // Mark invoice number
    let inv_number_id = doc.mark_entity(
        "invoice_number".to_string(),
        EntityType::InvoiceNumber,
        BoundingBox::new(120.0, 720.0, 150.0, 20.0, 1),
    );
    doc.set_entity_content(&inv_number_id, "INV-2024-001");
    doc.set_entity_confidence(&inv_number_id, 1.0);
    doc.relate_entities(&inv_number_id, &invoice_id, RelationType::IsPartOf);

    // Mark customer name
    let customer_id = doc.mark_entity(
        "customer_name".to_string(),
        EntityType::CustomerName,
        BoundingBox::new(50.0, 640.0, 200.0, 20.0, 1),
    );
    doc.set_entity_content(&customer_id, "ACME Corporation");
    doc.add_entity_metadata(&customer_id, "customer_type", "business");
    doc.set_entity_confidence(&customer_id, 1.0);
    doc.relate_entities(&customer_id, &invoice_id, RelationType::IsPartOf);

    // Mark customer address
    let address_id = doc.mark_entity(
        "customer_address".to_string(),
        EntityType::Address,
        BoundingBox::new(50.0, 605.0, 200.0, 35.0, 1),
    );
    doc.set_entity_content(&address_id, "123 Business Street, New York, NY 10001");
    doc.add_entity_metadata(&address_id, "address_type", "billing");
    doc.set_entity_confidence(&address_id, 0.95);
    doc.relate_entities(&address_id, &customer_id, RelationType::IsPartOf);

    // Mark line items
    let line1_id = doc.mark_entity(
        "line_item_1".to_string(),
        EntityType::LineItem,
        BoundingBox::new(50.0, 530.0, 480.0, 20.0, 1),
    );
    doc.set_entity_content(
        &line1_id,
        "Professional Services - Q1 2024, Qty: 1, $2,500.00",
    );
    doc.add_entity_metadata(&line1_id, "item_code", "PROF-SERV-Q1");
    doc.add_entity_metadata(&line1_id, "quantity", "1");
    doc.add_entity_metadata(&line1_id, "unit_price", "2500.00");
    doc.add_entity_metadata(&line1_id, "line_total", "2500.00");
    doc.set_entity_confidence(&line1_id, 1.0);
    doc.relate_entities(&line1_id, &invoice_id, RelationType::IsPartOf);

    let line2_id = doc.mark_entity(
        "line_item_2".to_string(),
        EntityType::LineItem,
        BoundingBox::new(50.0, 510.0, 480.0, 20.0, 1),
    );
    doc.set_entity_content(&line2_id, "Software License - Annual, Qty: 1, $1,200.00");
    doc.add_entity_metadata(&line2_id, "item_code", "SW-LIC-ANN");
    doc.add_entity_metadata(&line2_id, "quantity", "1");
    doc.add_entity_metadata(&line2_id, "unit_price", "1200.00");
    doc.add_entity_metadata(&line2_id, "line_total", "1200.00");
    doc.set_entity_confidence(&line2_id, 1.0);
    doc.relate_entities(&line2_id, &invoice_id, RelationType::IsPartOf);

    // Mark tax amount
    let tax_id = doc.mark_entity(
        "tax_amount".to_string(),
        EntityType::TaxAmount,
        BoundingBox::new(480.0, 460.0, 80.0, 20.0, 1),
    );
    doc.set_entity_content(&tax_id, "$305.25");
    doc.add_entity_metadata(&tax_id, "tax_rate", "8.25");
    doc.add_entity_metadata(&tax_id, "tax_base", "3700.00");
    doc.add_entity_metadata(&tax_id, "amount", "305.25");
    doc.set_entity_confidence(&tax_id, 1.0);
    doc.relate_entities(&tax_id, &invoice_id, RelationType::IsPartOf);

    // Mark total amount
    let total_id = doc.mark_entity(
        "total_amount".to_string(),
        EntityType::TotalAmount,
        BoundingBox::new(480.0, 430.0, 100.0, 20.0, 1),
    );
    doc.set_entity_content(&total_id, "$4,005.25");
    doc.add_entity_metadata(&total_id, "amount", "4005.25");
    doc.add_entity_metadata(&total_id, "currency", "USD");
    doc.set_entity_confidence(&total_id, 1.0);
    doc.relate_entities(&total_id, &invoice_id, RelationType::IsPartOf);

    // Mark due date
    let due_date_id = doc.mark_entity(
        "due_date".to_string(),
        EntityType::DueDate,
        BoundingBox::new(130.0, 380.0, 150.0, 20.0, 1),
    );
    doc.set_entity_content(&due_date_id, "February 14, 2024");
    doc.add_entity_metadata(&due_date_id, "date_format", "MMMM d, yyyy");
    doc.add_entity_metadata(&due_date_id, "iso_date", "2024-02-14");
    doc.set_entity_confidence(&due_date_id, 1.0);
    doc.relate_entities(&due_date_id, &invoice_id, RelationType::IsPartOf);

    // Add page to document
    doc.add_page(page);

    // ==================== Export Results ====================

    // Save the PDF
    let pdf_path = "examples/results/ai_ready_invoice_demo.pdf";
    doc.save(pdf_path)?;
    println!("✅ AI-Ready Invoice PDF created: {}", pdf_path);

    // Export semantic entities for inspection
    #[cfg(feature = "semantic")]
    {
        let json = doc.export_semantic_entities_json()?;
        let json_path = "examples/results/ai_ready_invoice_entities.json";
        std::fs::write(json_path, json)?;
        println!("📊 Semantic entities exported: {}", json_path);
    }

    // Print statistics
    let entity_count = doc.semantic_entity_count();
    println!(
        "🎯 Marked {} semantic entities for AI processing",
        entity_count
    );

    // Show entity breakdown by type
    let invoice_entities = doc.get_entities_by_type(EntityType::Invoice);
    let line_item_entities = doc.get_entities_by_type(EntityType::LineItem);
    let amount_entities = doc.get_entities_by_type(EntityType::TotalAmount);

    println!("   📄 {} Invoice entities", invoice_entities.len());
    println!("   📋 {} Line item entities", line_item_entities.len());
    println!("   💰 {} Amount entities", amount_entities.len());

    println!("\n🚀 Success! This PDF can now be processed by AI/ML systems with:");
    println!("   ✓ Perfect entity extraction (no OCR needed)");
    println!("   ✓ Structured relationships between entities");
    println!("   ✓ Confidence scoring for each marked region");
    println!("   ✓ Rich metadata for business logic");

    Ok(())
}
