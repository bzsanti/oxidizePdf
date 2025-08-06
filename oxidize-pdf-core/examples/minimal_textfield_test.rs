//! Minimal text field test - simplest possible form field
//!
//! This creates the absolute minimum PDF with a single text field
//! to debug form functionality.

use oxidize_pdf::forms::{BorderStyle, TextField, Widget, WidgetAppearance};
use oxidize_pdf::geometry::{Point, Rectangle};
use oxidize_pdf::graphics::Color;
use oxidize_pdf::{Document, Font, Page, Result};

fn main() -> Result<()> {
    println!("📝 Creating minimal text field test...");

    // Create document
    let mut document = Document::new();
    document.set_title("Minimal TextField Test");

    // Create page
    let mut page = Page::new(612.0, 792.0); // Letter size

    // Add some text
    page.text()
        .set_font(Font::Helvetica, 14.0)
        .at(50.0, 700.0)
        .write("Minimal Text Field Test")?
        .at(50.0, 650.0)
        .write("Enter text:")?;

    // Create widget - simple rectangle
    let widget = Widget::new(Rectangle::new(
        Point::new(150.0, 640.0),
        Point::new(400.0, 660.0),
    ))
    .with_appearance(WidgetAppearance {
        border_color: Some(Color::black()),
        background_color: Some(Color::white()),
        border_width: 1.0,
        border_style: BorderStyle::Solid,
    });

    // IMPORTANT: Add widget to page BEFORE adding page to document
    println!("  Adding widget to page...");
    page.add_form_widget(widget.clone());

    // Add page to document
    println!("  Adding page to document...");
    document.add_page(page);

    // Enable forms
    println!("  Enabling forms...");
    let form_manager = document.enable_forms();

    // Create text field
    println!("  Creating text field...");
    let text_field = TextField::new("test_field").with_default_value("Type here");

    // Add field to form manager
    println!("  Adding field to form manager...");
    let field_ref = form_manager.add_text_field(text_field, widget, None)?;
    println!("  Field reference: {field_ref:?}");

    let field_count = form_manager.field_count();
    let has_acro_form = document.acro_form().is_some();

    // Save
    println!("  Saving PDF...");
    document.save("minimal_textfield_test.pdf")?;

    println!("\n✅ Created minimal_textfield_test.pdf");
    println!("\n🔍 Debug info:");
    println!("  - Document has AcroForm: {has_acro_form}");
    println!("  - Form manager field count: {field_count}");

    println!("\n📋 Expected behavior:");
    println!("  - Should see a text field with black border");
    println!("  - Should be able to click and type in it");
    println!("  - Default text should be 'Type here'");

    Ok(())
}
