//! AI-Ready Invoice Example
//!
//! Demonstrates creating a PDF invoice with semantic entity marking for AI/ML processing.
//! The generated PDF contains both visual content and machine-readable metadata that can be
//! extracted and processed by automated systems.

use oxidize_pdf::semantic::{BoundingBox, EntityType, RelationType};
use oxidize_pdf::{Color, Document, Font, Page, Result};

fn main() -> Result<()> {
    println!("🤖 Creating AI-Ready Invoice PDF...\n");

    // Create document
    let mut doc = Document::new();
    doc.set_title("AI-Ready Invoice Example");
    doc.set_author("oxidize-pdf");

    // Create page
    let mut page = Page::a4();
    let width = page.width();

    // Header section
    page.text()
        .set_font(Font::HelveticaBold, 24.0)
        .at(50.0, 750.0)
        .write("INVOICE")?;

    // Mark invoice region
    let invoice_id = doc.mark_entity(
        "invoice_main".to_string(),
        EntityType::Invoice,
        BoundingBox::new(50.0, 50.0, (width - 100.0) as f32, 750.0, 1),
    );
    doc.set_entity_content(&invoice_id, "Invoice Document");
    doc.add_entity_metadata(&invoice_id, "currency", "USD");
    doc.add_entity_metadata(&invoice_id, "status", "unpaid");
    doc.set_entity_confidence(&invoice_id, 1.0);

    // Invoice number
    page.text()
        .set_font(Font::Helvetica, 12.0)
        .at(50.0, 720.0)
        .write("Invoice #: INV-2024-001")?;

    let inv_num_id = doc.mark_entity(
        "invoice_number".to_string(),
        EntityType::InvoiceNumber,
        BoundingBox::new(50.0, 720.0, 200.0, 15.0, 1),
    );
    doc.set_entity_content(&inv_num_id, "INV-2024-001");
    doc.add_entity_metadata(&inv_num_id, "value", "INV-2024-001");
    doc.set_entity_confidence(&inv_num_id, 1.0);
    doc.relate_entities(&inv_num_id, &invoice_id, RelationType::IsPartOf);

    // Date
    page.text()
        .set_font(Font::Helvetica, 12.0)
        .at(50.0, 700.0)
        .write("Date: 2024-10-05")?;

    let date_id = doc.mark_entity(
        "invoice_date".to_string(),
        EntityType::Date,
        BoundingBox::new(50.0, 700.0, 150.0, 15.0, 1),
    );
    doc.set_entity_content(&date_id, "2024-10-05");
    doc.add_entity_metadata(&date_id, "value", "2024-10-05");
    doc.add_entity_metadata(&date_id, "format", "ISO8601");
    doc.set_entity_confidence(&date_id, 1.0);
    doc.relate_entities(&date_id, &invoice_id, RelationType::IsPartOf);

    // Due date
    page.text()
        .set_font(Font::Helvetica, 12.0)
        .at(50.0, 680.0)
        .write("Due Date: 2024-11-05")?;

    let due_date_id = doc.mark_entity(
        "due_date".to_string(),
        EntityType::DueDate,
        BoundingBox::new(50.0, 680.0, 150.0, 15.0, 1),
    );
    doc.set_entity_content(&due_date_id, "2024-11-05");
    doc.add_entity_metadata(&due_date_id, "value", "2024-11-05");
    doc.add_entity_metadata(&due_date_id, "daysFromNow", "30");
    doc.set_entity_confidence(&due_date_id, 1.0);
    doc.relate_entities(&due_date_id, &invoice_id, RelationType::IsPartOf);

    // Customer section
    page.text()
        .set_font(Font::HelveticaBold, 14.0)
        .at(50.0, 640.0)
        .write("Bill To:")?;

    page.text()
        .set_font(Font::Helvetica, 12.0)
        .at(50.0, 620.0)
        .write("Acme Corporation")?;

    let customer_id = doc.mark_entity(
        "customer".to_string(),
        EntityType::CustomerName,
        BoundingBox::new(50.0, 620.0, 200.0, 15.0, 1),
    );
    doc.set_entity_content(&customer_id, "Acme Corporation");
    doc.add_entity_metadata(&customer_id, "name", "Acme Corporation");
    doc.add_entity_metadata(&customer_id, "type", "Organization");
    doc.set_entity_confidence(&customer_id, 0.99);
    doc.relate_entities(&customer_id, &invoice_id, RelationType::IsPartOf);

    page.text()
        .set_font(Font::Helvetica, 11.0)
        .at(50.0, 605.0)
        .write("123 Business St")?;

    page.text()
        .set_font(Font::Helvetica, 11.0)
        .at(50.0, 590.0)
        .write("New York, NY 10001")?;

    let address_id = doc.mark_entity(
        "customer_address".to_string(),
        EntityType::Address,
        BoundingBox::new(50.0, 590.0, 200.0, 30.0, 1),
    );
    doc.set_entity_content(&address_id, "123 Business St, New York, NY 10001");
    doc.add_entity_metadata(&address_id, "street", "123 Business St");
    doc.add_entity_metadata(&address_id, "city", "New York");
    doc.add_entity_metadata(&address_id, "state", "NY");
    doc.add_entity_metadata(&address_id, "postalCode", "10001");
    doc.set_entity_confidence(&address_id, 0.98);
    doc.relate_entities(&address_id, &customer_id, RelationType::IsPartOf);

    // Line items header
    let y_start = 540.0;
    page.graphics()
        .set_stroke_color(Color::gray(0.3))
        .set_line_width(0.5)
        .move_to(50.0, y_start)
        .line_to(width - 50.0, y_start)
        .stroke();

    page.text()
        .set_font(Font::HelveticaBold, 11.0)
        .at(50.0, y_start - 20.0)
        .write("Description")?;

    page.text()
        .set_font(Font::HelveticaBold, 11.0)
        .at(300.0, y_start - 20.0)
        .write("Quantity")?;

    page.text()
        .set_font(Font::HelveticaBold, 11.0)
        .at(400.0, y_start - 20.0)
        .write("Unit Price")?;

    page.text()
        .set_font(Font::HelveticaBold, 11.0)
        .at(500.0, y_start - 20.0)
        .write("Amount")?;

    // Line item 1
    let mut y = y_start - 45.0;
    page.text()
        .set_font(Font::Helvetica, 10.0)
        .at(50.0, y)
        .write("Professional Services - Web Development")?;

    page.text()
        .set_font(Font::Helvetica, 10.0)
        .at(310.0, y)
        .write("40")?;

    page.text()
        .set_font(Font::Helvetica, 10.0)
        .at(410.0, y)
        .write("$150.00")?;

    page.text()
        .set_font(Font::Helvetica, 10.0)
        .at(500.0, y)
        .write("$6,000.00")?;

    let line_item1_id = doc.mark_entity(
        "line_item_1".to_string(),
        EntityType::LineItem,
        BoundingBox::new(50.0, y as f32, (width - 100.0) as f32, 15.0, 1),
    );
    doc.set_entity_content(&line_item1_id, "Professional Services - Web Development");
    doc.add_entity_metadata(
        &line_item1_id,
        "description",
        "Professional Services - Web Development",
    );
    doc.add_entity_metadata(&line_item1_id, "quantity", "40");
    doc.add_entity_metadata(&line_item1_id, "unitPrice", "150.00");
    doc.add_entity_metadata(&line_item1_id, "amount", "6000.00");
    doc.set_entity_confidence(&line_item1_id, 0.95);
    doc.relate_entities(&line_item1_id, &invoice_id, RelationType::IsPartOf);

    // Line item 2
    y -= 25.0;
    page.text()
        .set_font(Font::Helvetica, 10.0)
        .at(50.0, y)
        .write("Cloud Hosting - Monthly")?;

    page.text()
        .set_font(Font::Helvetica, 10.0)
        .at(310.0, y)
        .write("1")?;

    page.text()
        .set_font(Font::Helvetica, 10.0)
        .at(410.0, y)
        .write("$299.00")?;

    page.text()
        .set_font(Font::Helvetica, 10.0)
        .at(510.0, y)
        .write("$299.00")?;

    let line_item2_id = doc.mark_entity(
        "line_item_2".to_string(),
        EntityType::LineItem,
        BoundingBox::new(50.0, y as f32, (width - 100.0) as f32, 15.0, 1),
    );
    doc.set_entity_content(&line_item2_id, "Cloud Hosting - Monthly");
    doc.add_entity_metadata(&line_item2_id, "description", "Cloud Hosting - Monthly");
    doc.add_entity_metadata(&line_item2_id, "quantity", "1");
    doc.add_entity_metadata(&line_item2_id, "unitPrice", "299.00");
    doc.add_entity_metadata(&line_item2_id, "amount", "299.00");
    doc.set_entity_confidence(&line_item2_id, 0.95);
    doc.relate_entities(&line_item2_id, &invoice_id, RelationType::IsPartOf);

    // Totals section
    y -= 50.0;
    page.graphics()
        .set_stroke_color(Color::gray(0.3))
        .set_line_width(0.5)
        .move_to(350.0, y)
        .line_to(width - 50.0, y)
        .stroke();

    y -= 25.0;
    page.text()
        .set_font(Font::Helvetica, 11.0)
        .at(400.0, y)
        .write("Subtotal:")?;

    page.text()
        .set_font(Font::Helvetica, 11.0)
        .at(500.0, y)
        .write("$6,299.00")?;

    // Tax
    y -= 20.0;
    page.text()
        .set_font(Font::Helvetica, 11.0)
        .at(400.0, y)
        .write("Tax (8.5%):")?;

    page.text()
        .set_font(Font::Helvetica, 11.0)
        .at(510.0, y)
        .write("$535.42")?;

    let tax_id = doc.mark_entity(
        "tax_amount".to_string(),
        EntityType::TaxAmount,
        BoundingBox::new(400.0, y as f32, 150.0, 15.0, 1),
    );
    doc.set_entity_content(&tax_id, "$535.42");
    doc.add_entity_metadata(&tax_id, "value", "535.42");
    doc.add_entity_metadata(&tax_id, "rate", "8.5");
    doc.add_entity_metadata(&tax_id, "type", "percentage");
    doc.set_entity_confidence(&tax_id, 1.0);
    doc.relate_entities(&tax_id, &invoice_id, RelationType::IsPartOf);

    // Total
    y -= 25.0;
    page.graphics()
        .set_stroke_color(Color::gray(0.3))
        .set_line_width(1.0)
        .move_to(350.0, y + 5.0)
        .line_to(width - 50.0, y + 5.0)
        .stroke();

    y -= 20.0;
    page.text()
        .set_font(Font::HelveticaBold, 14.0)
        .at(400.0, y)
        .write("TOTAL:")?;

    page.text()
        .set_font(Font::HelveticaBold, 14.0)
        .at(495.0, y)
        .write("$6,834.42")?;

    let total_id = doc.mark_entity(
        "total_amount".to_string(),
        EntityType::TotalAmount,
        BoundingBox::new(400.0, y as f32, 150.0, 18.0, 1),
    );
    doc.set_entity_content(&total_id, "$6,834.42");
    doc.add_entity_metadata(&total_id, "value", "6834.42");
    doc.add_entity_metadata(&total_id, "currency", "USD");
    doc.add_entity_metadata(&total_id, "formatted", "$6,834.42");
    doc.set_entity_confidence(&total_id, 1.0);
    doc.relate_entities(&total_id, &invoice_id, RelationType::IsPartOf);

    // Payment instructions
    y -= 60.0;
    page.text()
        .set_font(Font::HelveticaBold, 11.0)
        .at(50.0, y)
        .write("Payment Instructions:")?;

    y -= 20.0;
    page.text()
        .set_font(Font::Helvetica, 10.0)
        .at(50.0, y)
        .write("Please make payment via wire transfer to:")?;

    y -= 15.0;
    page.text()
        .set_font(Font::Helvetica, 9.0)
        .at(50.0, y)
        .write("Bank: Example Bank | Account: 1234567890 | Routing: 987654321")?;

    // Footer
    page.text()
        .set_font(Font::Helvetica, 8.0)
        .at(50.0, 50.0)
        .write("Thank you for your business!")?;

    doc.add_page(page);

    // Save PDF
    let pdf_path = "examples/results/ai_ready_invoice.pdf";
    doc.save(pdf_path)?;
    println!("✅ PDF saved to: {}", pdf_path);

    // Export semantic entities as JSON-LD
    #[cfg(feature = "semantic")]
    {
        let json_ld = doc.export_semantic_entities_json_ld()?;
        let json_path = "examples/results/ai_ready_invoice_entities.jsonld";
        std::fs::write(json_path, &json_ld)?;
        println!("✅ JSON-LD entities exported to: {}", json_path);
        println!("\n📄 JSON-LD Preview (first 1000 chars):");
        println!("{}", &json_ld[..json_ld.len().min(1000)]);
        if json_ld.len() > 1000 {
            println!("... (truncated)");
        }
    }

    #[cfg(not(feature = "semantic"))]
    {
        println!("\n⚠️  Run with --features semantic to export JSON-LD");
    }

    // Print summary
    println!("\n📊 Entity Summary:");
    println!("   Total entities: {}", doc.semantic_entity_count());
    println!("   Invoice: 1");
    println!("   Line items: 2");
    println!("   Customer info: 2");
    println!("   Financial data: 3 (invoice number, tax, total)");
    println!("   Dates: 2 (invoice date, due date)");

    println!("\n🎯 Use Case:");
    println!("   This invoice can now be processed by:");
    println!("   - Automated invoice extraction systems");
    println!("   - Accounting software with AI integration");
    println!("   - ML models trained on invoice data");
    println!("   - Document understanding pipelines");

    Ok(())
}
