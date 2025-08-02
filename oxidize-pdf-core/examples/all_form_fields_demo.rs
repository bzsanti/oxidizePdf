//! Comprehensive demo of all form field types
//!
//! This example demonstrates all available form field types in oxidize-pdf

use oxidize_pdf::geometry::{Point, Rectangle};
use oxidize_pdf::page_forms::PageForms;
use oxidize_pdf::{Document, Font, Page, Result};

fn main() -> Result<()> {
    println!("📋 Creating comprehensive form fields demo...");

    let mut document = Document::new();
    document.set_title("All Form Fields Demo");
    document.set_subject("Demonstrates all form field types");

    let mut page = Page::a4();

    // Header
    page.text()
        .set_font(Font::HelveticaBold, 20.0)
        .at(50.0, 770.0)
        .write("All Form Fields Demo")?
        .set_font(Font::Helvetica, 10.0)
        .at(50.0, 750.0)
        .write("This PDF demonstrates all available form field types in oxidize-pdf")?;

    let mut y = 700.0;

    // 1. Text Fields
    page.text()
        .set_font(Font::HelveticaBold, 14.0)
        .at(50.0, y)
        .write("1. Text Fields")?;
    y -= 25.0;

    page.text()
        .set_font(Font::Helvetica, 10.0)
        .at(50.0, y)
        .write("Name:")?;

    page.add_text_field(
        "name",
        Rectangle::new(Point::new(120.0, y - 5.0), Point::new(300.0, y + 15.0)),
        Some("Enter your name"),
    )?;
    y -= 25.0;

    page.text().at(50.0, y).write("Email:")?;

    page.add_text_field(
        "email",
        Rectangle::new(Point::new(120.0, y - 5.0), Point::new(300.0, y + 15.0)),
        None,
    )?;
    y -= 40.0;

    // 2. Checkboxes
    page.text()
        .set_font(Font::HelveticaBold, 14.0)
        .at(50.0, y)
        .write("2. Checkboxes")?;
    y -= 25.0;

    page.add_checkbox(
        "newsletter",
        Rectangle::new(Point::new(50.0, y - 5.0), Point::new(65.0, y + 10.0)),
        false,
    )?;
    page.text()
        .set_font(Font::Helvetica, 10.0)
        .at(70.0, y)
        .write("Subscribe to newsletter")?;
    y -= 20.0;

    page.add_checkbox(
        "terms",
        Rectangle::new(Point::new(50.0, y - 5.0), Point::new(65.0, y + 10.0)),
        false,
    )?;
    page.text()
        .at(70.0, y)
        .write("I agree to terms and conditions")?;
    y -= 40.0;

    // 3. Radio Buttons
    page.text()
        .set_font(Font::HelveticaBold, 14.0)
        .at(50.0, y)
        .write("3. Radio Buttons - Preferred Contact")?;
    y -= 25.0;

    // Note: Radio buttons with same name form a group
    page.add_radio_button(
        "contact",
        Rectangle::new(Point::new(50.0, y - 5.0), Point::new(65.0, y + 10.0)),
        "email",
        true, // Default selected
    )?;
    page.text()
        .set_font(Font::Helvetica, 10.0)
        .at(70.0, y)
        .write("Email")?;

    page.add_radio_button(
        "contact",
        Rectangle::new(Point::new(150.0, y - 5.0), Point::new(165.0, y + 10.0)),
        "phone",
        false,
    )?;
    page.text().at(170.0, y).write("Phone")?;

    page.add_radio_button(
        "contact",
        Rectangle::new(Point::new(250.0, y - 5.0), Point::new(265.0, y + 10.0)),
        "mail",
        false,
    )?;
    page.text().at(270.0, y).write("Mail")?;
    y -= 40.0;

    // 4. Dropdown (ComboBox)
    page.text()
        .set_font(Font::HelveticaBold, 14.0)
        .at(50.0, y)
        .write("4. Dropdown Menu (ComboBox)")?;
    y -= 25.0;

    page.text()
        .set_font(Font::Helvetica, 10.0)
        .at(50.0, y)
        .write("Country:")?;

    let countries = vec![
        ("us", "United States"),
        ("uk", "United Kingdom"),
        ("ca", "Canada"),
        ("au", "Australia"),
        ("de", "Germany"),
        ("fr", "France"),
        ("es", "Spain"),
        ("it", "Italy"),
        ("jp", "Japan"),
        ("br", "Brazil"),
    ];

    page.add_combo_box(
        "country",
        Rectangle::new(Point::new(120.0, y - 5.0), Point::new(300.0, y + 15.0)),
        countries,
        Some("us"),
    )?;
    y -= 40.0;

    // 5. List Box
    page.text()
        .set_font(Font::HelveticaBold, 14.0)
        .at(50.0, y)
        .write("5. List Box - Select Interests")?;
    y -= 25.0;

    let interests = vec![
        ("tech", "Technology"),
        ("sports", "Sports"),
        ("music", "Music"),
        ("art", "Art"),
        ("travel", "Travel"),
        ("cooking", "Cooking"),
        ("reading", "Reading"),
        ("gaming", "Gaming"),
    ];

    page.add_list_box(
        "interests",
        Rectangle::new(Point::new(50.0, y - 80.0), Point::new(200.0, y)),
        interests,
        vec![0, 2], // Pre-select Technology and Music
        true,       // Allow multi-select
    )?;

    page.text()
        .set_font(Font::Helvetica, 9.0)
        .at(210.0, y - 10.0)
        .write("(Hold Ctrl/Cmd to select multiple)")?;
    y -= 100.0;

    // 6. Push Buttons
    page.text()
        .set_font(Font::HelveticaBold, 14.0)
        .at(50.0, y)
        .write("6. Push Buttons")?;
    y -= 30.0;

    page.add_push_button(
        "submit",
        Rectangle::new(Point::new(50.0, y - 5.0), Point::new(130.0, y + 20.0)),
        "Submit",
    )?;

    page.add_push_button(
        "reset",
        Rectangle::new(Point::new(140.0, y - 5.0), Point::new(220.0, y + 20.0)),
        "Reset",
    )?;

    page.add_push_button(
        "print",
        Rectangle::new(Point::new(230.0, y - 5.0), Point::new(310.0, y + 20.0)),
        "Print",
    )?;
    y -= 50.0;

    // Instructions
    page.text()
        .set_font(Font::HelveticaBold, 12.0)
        .at(50.0, y)
        .write("Instructions:")?
        .set_font(Font::Helvetica, 9.0)
        .at(50.0, y - 15.0)
        .write("• Text fields: Click to enter text")?
        .at(50.0, y - 27.0)
        .write("• Checkboxes: Click to check/uncheck")?
        .at(50.0, y - 39.0)
        .write("• Radio buttons: Click to select one option")?
        .at(50.0, y - 51.0)
        .write("• Dropdown: Click to select from list")?
        .at(50.0, y - 63.0)
        .write("• List box: Click to select one or more items")?
        .at(50.0, y - 75.0)
        .write("• Push buttons: Click to perform actions (requires JavaScript)")?;

    // Enable forms and add page
    document.enable_forms();
    document.add_page(page);

    // Save
    document.save("all_form_fields_demo.pdf")?;

    println!("\n✅ Created all_form_fields_demo.pdf");
    println!("\n📋 Form fields included:");
    println!("  ✓ Text fields (name, email)");
    println!("  ✓ Checkboxes (newsletter, terms)");
    println!("  ✓ Radio buttons (contact preference)");
    println!("  ✓ Dropdown/ComboBox (country)");
    println!("  ✓ List box with multi-select (interests)");
    println!("  ✓ Push buttons (submit, reset, print)");
    println!("\n🔍 Test in different PDF readers to verify compatibility");

    Ok(())
}
