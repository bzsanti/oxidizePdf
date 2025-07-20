//! Example demonstrating PDF metadata features in oxidize-pdf
//!
//! This example shows how to set various metadata fields including:
//! - Title, Author, Subject, Keywords
//! - Creator and Producer information
//! - Creation and modification dates

use chrono::{Local, TimeZone, Utc};
use oxidize_pdf::{Document, Font, Page};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create a new document
    let mut doc = Document::new();

    // Set comprehensive metadata
    doc.set_title("Metadata Example Document");
    doc.set_author("Jane Doe");
    doc.set_subject("Demonstrating PDF metadata capabilities");
    doc.set_keywords("metadata, example, oxidize-pdf, rust");

    // Set creator and producer information
    doc.set_creator("My Awesome Application v2.0");
    doc.set_producer("oxidize-pdf Custom Build");

    // Set specific creation date (e.g., project start date)
    let creation_date = Utc.with_ymd_and_hms(2024, 1, 15, 10, 30, 0).unwrap();
    doc.set_creation_date(creation_date);

    // You can also use local time
    let local_mod_date = Local.with_ymd_and_hms(2024, 3, 20, 14, 45, 0).unwrap();
    doc.set_modification_date_local(local_mod_date);

    // Create a page with content
    let mut page = Page::a4();

    page.text()
        .set_font(Font::HelveticaBold, 24.0)
        .at(50.0, 750.0)
        .write("Document with Rich Metadata")?
        .set_font(Font::Helvetica, 14.0)
        .at(50.0, 700.0)
        .write("This PDF demonstrates comprehensive metadata support.")?
        .at(50.0, 670.0)
        .write(&format!(
            "Created: {}",
            creation_date.format("%B %d, %Y at %H:%M UTC")
        ))?
        .at(50.0, 640.0)
        .write(&format!(
            "Modified: {}",
            local_mod_date.format("%B %d, %Y at %H:%M %Z")
        ))?;

    // Add metadata information to the page
    page.text()
        .set_font(Font::HelveticaBold, 16.0)
        .at(50.0, 580.0)
        .write("Metadata Information:")?
        .set_font(Font::Helvetica, 12.0)
        .at(70.0, 550.0)
        .write("• Title: Metadata Example Document")?
        .at(70.0, 530.0)
        .write("• Author: Jane Doe")?
        .at(70.0, 510.0)
        .write("• Subject: Demonstrating PDF metadata capabilities")?
        .at(70.0, 490.0)
        .write("• Keywords: metadata, example, oxidize-pdf, rust")?
        .at(70.0, 470.0)
        .write("• Creator: My Awesome Application v2.0")?
        .at(70.0, 450.0)
        .write("• Producer: oxidize-pdf Custom Build")?;

    // Add note about automatic modification date
    page.text()
        .set_font(Font::HelveticaOblique, 10.0)
        .at(50.0, 400.0)
        .write("Note: The modification date is automatically updated when saving the document.")?;

    // Add the page to the document
    doc.add_page(page);

    // Save the document - this will automatically update the modification date
    doc.save("metadata_example.pdf")?;

    println!("Created metadata_example.pdf with comprehensive metadata");
    println!("\nMetadata Summary:");
    println!("  Title: Metadata Example Document");
    println!("  Author: Jane Doe");
    println!("  Subject: Demonstrating PDF metadata capabilities");
    println!("  Keywords: metadata, example, oxidize-pdf, rust");
    println!("  Creator: My Awesome Application v2.0");
    println!("  Producer: oxidize-pdf Custom Build");
    println!(
        "  Creation Date: {}",
        creation_date.format("%Y-%m-%d %H:%M:%S UTC")
    );
    println!("  Modification Date: Automatically set to current time");
    println!("\nYou can verify the metadata using a PDF reader's Document Properties dialog.");

    Ok(())
}
