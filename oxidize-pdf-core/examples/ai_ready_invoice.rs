//! Example of creating an AI-Ready PDF with semantic entity marking
//!
//! This example demonstrates how to create a PDF with regions marked
//! for AI/ML processing using the Community Edition semantic features.
//!
//! To run: cargo run --example ai_ready_invoice --features semantic

use oxidize_pdf::{Color, Document, Font, Page};

#[cfg(feature = "semantic")]
use oxidize_pdf::semantic::{EntityType, SemanticMarker};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(not(feature = "semantic"))]
    {
        println!("This example requires the 'semantic' feature.");
        println!("Run with: cargo run --example ai_ready_invoice --features semantic");
        return Ok(());
    }

    #[cfg(feature = "semantic")]
    {
        // Create document and page
        let mut doc = Document::new();
        let mut page = Page::a4();

        // Add visual content first
        create_invoice_content(&mut page)?;

        // Now mark semantic regions for AI processing
        mark_semantic_regions(&mut page);

        // Add page to document
        doc.add_page(page);

        // Set document metadata
        doc.set_title("AI-Ready Invoice");
        doc.set_author("oxidizePdf Pro");

        // Save the PDF
        doc.save("ai_ready_invoice.pdf")?;

        println!("✅ Created AI-Ready PDF: ai_ready_invoice.pdf");
    }

    Ok(())
}

fn create_invoice_content(page: &mut Page) -> Result<(), Box<dyn std::error::Error>> {
    // Header
    page.text()
        .set_font(Font::HelveticaBold, 28.0)
        .at(50.0, 750.0)
        .write("INVOICE")?;

    // Invoice details
    page.text()
        .set_font(Font::Helvetica, 12.0)
        .at(400.0, 750.0)
        .write("Invoice #: INV-2024-001")?
        .at(400.0, 730.0)
        .write("Date: 2024-01-15")?
        .at(400.0, 710.0)
        .write("Due Date: 2024-02-14")?;

    // Company info
    page.text()
        .set_font(Font::HelveticaBold, 14.0)
        .at(50.0, 680.0)
        .write("Acme Corporation")?
        .set_font(Font::Helvetica, 12.0)
        .at(50.0, 660.0)
        .write("123 Business St")?
        .at(50.0, 645.0)
        .write("City, ST 12345")?;

    // Customer info
    page.text()
        .set_font(Font::HelveticaBold, 12.0)
        .at(50.0, 600.0)
        .write("Bill To:")?
        .set_font(Font::Helvetica, 12.0)
        .at(50.0, 580.0)
        .write("John Doe")?
        .at(50.0, 565.0)
        .write("456 Customer Ave")?
        .at(50.0, 550.0)
        .write("Town, ST 67890")?;

    // Invoice table
    page.graphics()
        .set_stroke_color(Color::black())
        .set_line_width(1.0)
        .rect(50.0, 450.0, 500.0, 80.0)
        .stroke();

    // Table headers
    page.text()
        .set_font(Font::HelveticaBold, 12.0)
        .at(60.0, 510.0)
        .write("Description")?
        .at(250.0, 510.0)
        .write("Quantity")?
        .at(350.0, 510.0)
        .write("Price")?
        .at(450.0, 510.0)
        .write("Total")?;

    // Table content
    page.text()
        .set_font(Font::Helvetica, 11.0)
        .at(60.0, 480.0)
        .write("Professional Services")?
        .at(270.0, 480.0)
        .write("10")?
        .at(360.0, 480.0)
        .write("$100.00")?
        .at(450.0, 480.0)
        .write("$1,000.00")?;

    // Total
    page.text()
        .set_font(Font::HelveticaBold, 14.0)
        .at(400.0, 400.0)
        .write("Total: $1,000.00")?;

    Ok(())
}

#[cfg(feature = "semantic")]
fn mark_semantic_regions(page: &mut Page) {
    let mut marker = SemanticMarker::new(page);

    // Mark invoice header
    marker
        .mark(EntityType::Heading, (50.0, 740.0, 200.0, 40.0))
        .with_metadata("type", "invoice_header")
        .with_confidence(0.99)
        .build();

    // Mark invoice number
    marker
        .mark_text((400.0, 740.0, 150.0, 20.0))
        .with_metadata("field", "invoice_number")
        .with_metadata("value", "INV-2024-001")
        .with_schema("https://schema.org/Invoice")
        .with_confidence(0.98)
        .build();

    // Mark invoice date
    marker
        .mark_text((400.0, 720.0, 150.0, 20.0))
        .with_metadata("field", "invoice_date")
        .with_metadata("value", "2024-01-15")
        .with_metadata("format", "YYYY-MM-DD")
        .with_confidence(0.97)
        .build();

    // Mark company info
    marker
        .mark_text((50.0, 640.0, 200.0, 60.0))
        .with_metadata("type", "company_info")
        .with_metadata("company_name", "Acme Corporation")
        .with_schema("https://schema.org/Organization")
        .with_confidence(0.95)
        .build();

    // Mark customer info
    marker
        .mark_text((50.0, 540.0, 200.0, 80.0))
        .with_metadata("type", "customer_info")
        .with_metadata("customer_name", "John Doe")
        .with_schema("https://schema.org/Person")
        .with_confidence(0.94)
        .build();

    // Mark invoice table
    marker
        .mark_table((50.0, 450.0, 500.0, 80.0))
        .with_metadata("type", "line_items")
        .with_metadata("rows", "1")
        .with_metadata("columns", "4")
        .with_confidence(0.96)
        .build();

    // Mark total amount
    marker
        .mark_text((400.0, 390.0, 150.0, 25.0))
        .with_metadata("field", "total_amount")
        .with_metadata("value", "1000.00")
        .with_metadata("currency", "USD")
        .with_schema("https://schema.org/MonetaryAmount")
        .with_confidence(0.99)
        .build();
}

#[cfg(not(feature = "semantic"))]
fn mark_semantic_regions(_page: &mut Page) {
    // No-op when semantic feature is not enabled
}
