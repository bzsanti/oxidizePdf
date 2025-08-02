//! Simple forms test to verify basic AcroForm integration
//!
//! This example creates a minimal form with just a text field to test
//! that forms are being written to the PDF correctly.

use oxidize_pdf::forms::*;
use oxidize_pdf::geometry::{Point, Rectangle};
use oxidize_pdf::{Document, Font, Page};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Creating simple forms test PDF...");

    let mut document = Document::new();
    document.set_title("Simple Forms Test");
    document.set_author("oxidize-pdf");

    // Create widgets first (before enabling forms to avoid borrow conflicts)
    let text_widget = Widget::new(Rectangle::new(
        Point::new(150.0, 650.0),
        Point::new(400.0, 670.0),
    ));

    let checkbox_widget = Widget::new(Rectangle::new(
        Point::new(150.0, 600.0),
        Point::new(165.0, 615.0),
    ));

    let button_widget = Widget::new(Rectangle::new(
        Point::new(150.0, 550.0),
        Point::new(250.0, 570.0),
    ));

    // Create a page
    let mut page = Page::a4();

    // Add title and instructions
    page.text()
        .set_font(Font::Helvetica, 18.0)
        .at(50.0, 750.0)
        .write("Simple Forms Test")?;

    page.text()
        .set_font(Font::Helvetica, 12.0)
        .at(50.0, 720.0)
        .write("This PDF should contain interactive form fields if AcroForm integration works.")?;

    // Add widgets to page as annotations
    page.add_form_widget(text_widget.clone());
    page.add_form_widget(checkbox_widget.clone());
    page.add_form_widget(button_widget.clone());

    // Now enable forms and add fields
    let field_count = {
        let form_manager = document.enable_forms();
        println!("✓ Enabled FormManager in document");

        // Test 1: Add a simple text field
        let text_field = TextField::new("test_field").with_default_value("Type here...");

        let field_ref = form_manager.add_text_field(text_field, text_widget, None)?;
        println!("✓ Created text field with reference: {:?}", field_ref);

        // Test 2: Add a checkbox
        let checkbox_field = CheckBox::new("test_checkbox");

        let checkbox_ref = form_manager.add_checkbox(checkbox_field, checkbox_widget, None)?;
        println!("✓ Created checkbox with reference: {:?}", checkbox_ref);

        // Test 3: Add a button
        let button_field = PushButton::new("test_button").with_caption("Click Me");

        let button_ref = form_manager.add_push_button(button_field, button_widget, None)?;
        println!("✓ Created button with reference: {:?}", button_ref);

        let count = form_manager.field_count();
        println!("✓ Form manager has {} fields", count);
        count
    };

    // Add labels and diagnostic info
    page.text()
        .set_font(Font::Helvetica, 12.0)
        .at(50.0, 655.0)
        .write("Test Field:")?;

    page.text()
        .set_font(Font::Helvetica, 12.0)
        .at(50.0, 605.0)
        .write("Test Checkbox:")?;

    page.text()
        .set_font(Font::Helvetica, 12.0)
        .at(50.0, 555.0)
        .write("Test Button:")?;

    // Add diagnostic info to the PDF
    page.text()
        .set_font(Font::Helvetica, 10.0)
        .at(50.0, 500.0)
        .write("Integration Status:")?;

    page.text()
        .set_font(Font::Helvetica, 9.0)
        .at(50.0, 485.0)
        .write(&format!(
            "• Document has AcroForm: {}",
            document.acro_form().is_some()
        ))?;

    page.text()
        .set_font(Font::Helvetica, 9.0)
        .at(50.0, 470.0)
        .write(&format!("• Form manager field count: {}", field_count))?;

    page.text()
        .set_font(Font::Helvetica, 9.0)
        .at(50.0, 455.0)
        .write("• Form fields should appear as interactive elements if integration works")?;

    // Add the page to the document
    document.add_page(page);

    // Save the document
    document.save("simple_forms_test.pdf")?;

    println!("✓ Created simple_forms_test.pdf");
    println!("\nTo test:");
    println!("1. Open simple_forms_test.pdf in a PDF viewer");
    println!("2. Look for interactive form fields (text field, checkbox, button)");
    println!("3. Try interacting with them");

    if field_count > 0 {
        println!(
            "\n✓ Form manager successfully created {} fields",
            field_count
        );
    } else {
        println!("\n✗ Form manager has no fields - integration may be incomplete");
    }

    Ok(())
}
