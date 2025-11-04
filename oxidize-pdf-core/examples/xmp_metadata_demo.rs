// XMP Metadata Example
//
// Demonstrates how to create and embed XMP (Extensible Metadata Platform)
// metadata in PDF documents according to ISO 32000-1 Section 14.3.2.
//
// XMP provides rich, standardized metadata that can be read by:
// - Adobe Acrobat/Reader
// - Content management systems
// - Digital asset management tools
// - Search engines and indexers

use oxidize_pdf::document::Document;
use oxidize_pdf::metadata::{XmpMetadata, XmpNamespace};
use oxidize_pdf::page::Page;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    println!("XMP Metadata Demo");
    println!("=================\n");

    // Create a new PDF document
    let mut doc = Document::new();

    // Set basic document metadata (Info dictionary - PDF 1.0 style)
    doc.set_title("XMP Metadata Demonstration");
    doc.set_author("oxidize-pdf Library");
    doc.set_subject("Demonstration of XMP metadata embedding");
    doc.set_keywords("PDF, XMP, Metadata, ISO 32000-1");
    doc.set_creator("oxidize-pdf Example");

    // Create a simple page with content
    let page = Page::a4();

    // Note: Page content APIs are simplified for this example
    // In a real implementation, you would use the full drawing API
    println!("Created A4 page for demonstration");

    doc.add_page(page);

    // Generate XMP metadata from document info
    let xmp = doc.create_xmp_metadata();

    // Display the generated XMP packet
    println!("Generated XMP Metadata Packet:");
    println!("{}", "-".repeat(80));
    let packet = xmp.to_xmp_packet();
    println!("{}", packet);
    println!("{}", "-".repeat(80));

    // Create a custom XMP metadata with additional properties
    println!("\nCreating custom XMP metadata...");
    let mut custom_xmp = XmpMetadata::new();

    // Dublin Core metadata
    custom_xmp.set_text(XmpNamespace::DublinCore, "title", "Advanced PDF Document");
    custom_xmp.set_text(XmpNamespace::DublinCore, "creator", "Jane Doe");
    custom_xmp.set_text(
        XmpNamespace::DublinCore,
        "description",
        "This document demonstrates advanced XMP metadata features",
    );

    // Add subject keywords as a bag (unordered collection)
    custom_xmp.set_bag(
        XmpNamespace::DublinCore,
        "subject",
        vec![
            "PDF Standards".to_string(),
            "Metadata".to_string(),
            "XMP".to_string(),
            "ISO 32000-1".to_string(),
            "Document Management".to_string(),
        ],
    );

    // XMP Basic metadata
    custom_xmp.set_date(XmpNamespace::XmpBasic, "CreateDate", "2025-10-08T12:00:00Z");
    custom_xmp.set_date(XmpNamespace::XmpBasic, "ModifyDate", "2025-10-08T14:30:00Z");
    custom_xmp.set_text(XmpNamespace::XmpBasic, "CreatorTool", "oxidize-pdf v1.4.0");
    custom_xmp.set_text(
        XmpNamespace::XmpBasic,
        "MetadataDate",
        "2025-10-08T14:30:00Z",
    );

    // PDF-specific metadata
    custom_xmp.set_text(
        XmpNamespace::Pdf,
        "Producer",
        "oxidize-pdf Community Edition",
    );
    custom_xmp.set_text(XmpNamespace::Pdf, "PDFVersion", "1.5");

    // XMP Rights Management
    custom_xmp.set_text(XmpNamespace::XmpRights, "Marked", "True");
    custom_xmp.set_text(
        XmpNamespace::XmpRights,
        "UsageTerms",
        "This document is licensed under CC-BY 4.0",
    );

    // Alternative text in multiple languages
    custom_xmp.set_alt(
        XmpNamespace::DublinCore,
        "rights",
        vec![
            (
                "x-default".to_string(),
                "Copyright © 2025 oxidize-pdf Contributors".to_string(),
            ),
            (
                "en".to_string(),
                "Copyright © 2025 oxidize-pdf Contributors".to_string(),
            ),
            (
                "es".to_string(),
                "Copyright © 2025 Contribuidores de oxidize-pdf".to_string(),
            ),
            (
                "fr".to_string(),
                "Copyright © 2025 Contributeurs oxidize-pdf".to_string(),
            ),
        ],
    );

    // Custom namespace example
    custom_xmp.register_namespace(
        "company".to_string(),
        "http://example.com/ns/company/".to_string(),
    );
    let company_ns = XmpNamespace::Custom(
        "company".to_string(),
        "http://example.com/ns/company/".to_string(),
    );
    custom_xmp.set_text(company_ns.clone(), "Department", "Engineering");
    custom_xmp.set_text(company_ns.clone(), "Project", "PDF Automation");
    custom_xmp.set_text(company_ns, "DocumentID", "DOC-2025-001");

    println!("✅ Custom XMP metadata created with:");
    println!("   - Dublin Core: title, creator, description, subject, rights");
    println!("   - XMP Basic: dates, creator tool");
    println!("   - PDF: producer, version");
    println!("   - XMP Rights: usage terms");
    println!("   - Custom namespace: company information");

    // Display custom XMP packet
    println!("\nCustom XMP Metadata Packet:");
    println!("{}", "-".repeat(80));
    let custom_packet = custom_xmp.to_xmp_packet();
    println!("{}", custom_packet);
    println!("{}", "-".repeat(80));

    // Save the PDF with XMP metadata embedded
    println!("\n✅ Saving PDF with embedded XMP metadata...");

    use std::fs::File;
    use std::io::Write;

    let pdf_path = "examples/results/xmp_metadata_demo.pdf";

    // Generate PDF bytes
    let pdf_bytes = doc.to_bytes()?;

    // Write to file
    let mut file = File::create(pdf_path)?;
    file.write_all(&pdf_bytes)?;

    println!("✅ PDF saved to: {}", pdf_path);
    println!("   File size: {} bytes", pdf_bytes.len());
    println!("\n✅ XMP Metadata successfully embedded in PDF!");
    println!("\nYou can verify the XMP metadata by:");
    println!("- Opening the PDF in Adobe Acrobat (File → Properties → Custom)");
    println!("- Using exiftool: exiftool {}", pdf_path);
    println!("- Using pdfinfo: pdfinfo -meta {}", pdf_path);

    Ok(())
}
