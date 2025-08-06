//! Working forms test - using the simplified approach
//!
//! This creates forms using a more direct approach that actually works

use oxidize_pdf::annotations::{Annotation, AnnotationType};
use oxidize_pdf::forms::{create_checkbox_dict, create_text_field_dict};
use oxidize_pdf::geometry::{Point, Rectangle};
use oxidize_pdf::{Document, Font, Page, Result};

fn main() -> Result<()> {
    println!("🎯 Creating working forms test...");

    let mut document = Document::new();
    document.set_title("Working Forms Test");

    let mut page = Page::new(612.0, 792.0);

    // Add title
    page.text()
        .set_font(Font::HelveticaBold, 16.0)
        .at(50.0, 720.0)
        .write("Working Forms Test")?
        .set_font(Font::Helvetica, 12.0)
        .at(50.0, 680.0)
        .write("These forms should actually work!")?;

    // Create text field
    page.text().at(50.0, 620.0).write("Name:")?;

    let text_rect = Rectangle::new(Point::new(150.0, 610.0), Point::new(400.0, 630.0));

    // Create a working text field dictionary
    let text_field_dict = create_text_field_dict("name_field", text_rect, Some("Enter your name"));

    // Add as annotation to page
    let mut text_annot = Annotation::new(AnnotationType::Widget, text_rect);
    for (key, value) in text_field_dict.entries() {
        text_annot.properties.set(key, value.clone());
    }
    page.annotations_mut().push(text_annot);

    // Create checkbox
    page.text().at(50.0, 560.0).write("I agree:")?;

    let check_rect = Rectangle::new(Point::new(150.0, 555.0), Point::new(165.0, 570.0));

    // Create a working checkbox dictionary
    let checkbox_dict = create_checkbox_dict(
        "agree_checkbox",
        check_rect,
        false, // unchecked by default
    );

    // Add as annotation to page
    let mut check_annot = Annotation::new(AnnotationType::Widget, check_rect);
    for (key, value) in checkbox_dict.entries() {
        check_annot.properties.set(key, value.clone());
    }
    page.annotations_mut().push(check_annot);

    // Add instructions
    page.text()
        .set_font(Font::HelveticaBold, 12.0)
        .at(50.0, 500.0)
        .write("Instructions:")?
        .set_font(Font::Helvetica, 10.0)
        .at(50.0, 480.0)
        .write("1. Click on the text field and type your name")?
        .at(50.0, 465.0)
        .write("2. Click on the checkbox to check/uncheck it")?
        .at(50.0, 450.0)
        .write("3. Save the PDF to preserve your input")?;

    // Enable forms in document - this creates the AcroForm
    let _ = document.enable_forms();

    // Add page to document
    document.add_page(page);

    // Save
    document.save("working_forms_test.pdf")?;

    println!("\n✅ Created working_forms_test.pdf");
    println!("\n🔍 This PDF uses:");
    println!("  - Simplified form field creation");
    println!("  - Fields added directly as page annotations");
    println!("  - Combined widget/field objects");
    println!("\n📋 Expected result:");
    println!("  - Forms should be interactive");
    println!("  - Text field should accept input");
    println!("  - Checkbox should be clickable");

    Ok(())
}
