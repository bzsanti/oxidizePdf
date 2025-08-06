//! Example demonstrating PDF forms support
//!
//! This example shows how to add various types of form fields to a PDF page using the PageForms trait.

use oxidize_pdf::{
    geometry::{Point, Rectangle},
    page_forms::PageForms,
    Document, Font, Page, Result,
};

fn main() -> Result<()> {
    // Create a new document
    let mut doc = Document::new();
    doc.set_title("Forms Example");
    doc.set_author("oxidize-pdf");

    // Add a page
    let mut page = Page::a4();
    // Using Font::Helvetica directly to avoid move issues

    // Set up page dimensions
    page.text()
        .set_font(Font::Helvetica, 14.0)
        .at(50.0, 750.0)
        .write("PDF Forms Example")?;

    // Add a text field
    page.text()
        .set_font(Font::Helvetica, 12.0)
        .at(50.0, 700.0)
        .write("Name:")?;

    page.add_text_field(
        "name",
        Rectangle::new(Point::new(100.0, 695.0), Point::new(300.0, 715.0)),
        Some("Enter your name"),
    )?;

    // Add a checkbox
    page.text()
        .set_font(Font::Helvetica, 12.0)
        .at(50.0, 650.0)
        .write("Subscribe to newsletter:")?;

    page.add_checkbox(
        "subscribe",
        Rectangle::new(Point::new(200.0, 645.0), Point::new(215.0, 660.0)),
        true,
    )?;

    // Add radio buttons
    page.text()
        .set_font(Font::Helvetica, 12.0)
        .at(50.0, 600.0)
        .write("Contact method:")?;

    page.add_radio_button(
        "contact",
        Rectangle::new(Point::new(50.0, 570.0), Point::new(65.0, 585.0)),
        "email",
        true,
    )?;

    page.text()
        .set_font(Font::Helvetica, 12.0)
        .at(70.0, 575.0)
        .write("Email")?;

    page.add_radio_button(
        "contact",
        Rectangle::new(Point::new(130.0, 570.0), Point::new(145.0, 585.0)),
        "phone",
        false,
    )?;

    page.text()
        .set_font(Font::Helvetica, 12.0)
        .at(150.0, 575.0)
        .write("Phone")?;

    // Add a dropdown/combo box
    page.text()
        .set_font(Font::Helvetica, 12.0)
        .at(50.0, 520.0)
        .write("Country:")?;

    page.add_combo_box(
        "country",
        Rectangle::new(Point::new(100.0, 515.0), Point::new(200.0, 535.0)),
        vec![
            ("US", "United States"),
            ("CA", "Canada"),
            ("UK", "United Kingdom"),
        ],
        Some("US"),
    )?;

    // Add a listbox
    page.text()
        .set_font(Font::Helvetica, 12.0)
        .at(50.0, 470.0)
        .write("Interests:")?;

    page.add_list_box(
        "interests",
        Rectangle::new(Point::new(100.0, 420.0), Point::new(200.0, 480.0)),
        vec![
            ("tech", "Technology"),
            ("sports", "Sports"),
            ("music", "Music"),
        ],
        vec![0, 2],
        true,
    )?;

    // Add a push button
    page.add_push_button(
        "submit",
        Rectangle::new(Point::new(50.0, 350.0), Point::new(150.0, 380.0)),
        "Submit Form",
    )?;

    // Add the page to the document
    doc.add_page(page);

    // Save the document
    doc.save("forms_example.pdf")?;
    println!("Created forms_example.pdf with interactive form fields");

    Ok(())
}
