//! Simple forms using the new API
//!
//! This demonstrates the simplified forms API

use oxidize_pdf::geometry::{Point, Rectangle};
use oxidize_pdf::page_forms::PageForms;
use oxidize_pdf::{Document, Font, Page, Result};

fn main() -> Result<()> {
    println!("📝 Creating PDF with simple forms API...");

    let mut document = Document::new();
    document.set_title("Simple Forms API Test");

    let mut page = Page::a4();

    // Add title
    page.text()
        .set_font(Font::HelveticaBold, 18.0)
        .at(50.0, 750.0)
        .write("Simple Forms API")?
        .set_font(Font::Helvetica, 12.0)
        .at(50.0, 710.0)
        .write("This demonstrates the simplified forms API")?;

    // Add form fields using the new API
    page.text().at(50.0, 650.0).write("Your Name:")?;

    page.add_text_field(
        "name",
        Rectangle::new(Point::new(150.0, 640.0), Point::new(400.0, 660.0)),
        Some("Enter your name here"),
    )?;

    page.text().at(50.0, 600.0).write("Email:")?;

    page.add_text_field(
        "email",
        Rectangle::new(Point::new(150.0, 590.0), Point::new(400.0, 610.0)),
        None,
    )?;

    page.text().at(50.0, 550.0).write("Newsletter:")?;

    page.add_checkbox(
        "newsletter",
        Rectangle::new(Point::new(150.0, 545.0), Point::new(165.0, 560.0)),
        false,
    )?;

    page.text()
        .at(170.0, 550.0)
        .write("Subscribe to newsletter")?;

    page.text().at(50.0, 500.0).write("Terms:")?;

    page.add_checkbox(
        "terms",
        Rectangle::new(Point::new(150.0, 495.0), Point::new(165.0, 510.0)),
        false,
    )?;

    page.text()
        .at(170.0, 500.0)
        .write("I agree to the terms and conditions")?;

    // Enable forms and add page
    document.enable_forms();
    document.add_page(page);

    // Save
    document.save("simple_forms_api.pdf")?;

    println!("\n✅ Created simple_forms_api.pdf");
    println!("\n🎯 Using the new simplified API:");
    println!("  - page.add_text_field()");
    println!("  - page.add_checkbox()");
    println!("\n📋 Benefits:");
    println!("  - No need to manage widgets separately");
    println!("  - Automatic field dictionary creation");
    println!("  - Forms work out of the box!");

    Ok(())
}
