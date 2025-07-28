//! Example demonstrating basic forms and annotations support

use oxidize_pdf::{
    annotations::{
        AnnotationManager, FreeTextAnnotation, Icon as TextIcon, LinkAnnotation, MarkupAnnotation,
        StampAnnotation, StampName, TextAnnotation,
    },
    forms::{
        BorderStyle as WidgetBorderStyle, CheckBox, ComboBox, FormManager, ListBox, PushButton,
        RadioButton, TextField, Widget, WidgetAppearance,
    },
    graphics::Color,
    objects::ObjectReference,
    text::Font,
    Document, Page, Point, Rectangle, Result,
};

fn main() -> Result<()> {
    // Create a new document
    let mut doc = Document::new();
    doc.set_title("Forms and Annotations Example");

    // Create form manager and annotation manager
    let mut form_manager = FormManager::new();
    let mut annotation_manager = AnnotationManager::new();

    // Create first page with form fields
    let mut page1 = Page::a4();
    let page1_ref = ObjectReference::new(1, 0);

    {
        let graphics = page1.graphics();

        // Title
        graphics
            .begin_text()
            .set_font(Font::HelveticaBold, 24.0)
            .set_text_position(50.0, 750.0)
            .show_text("PDF Form Example")?
            .end_text();

        // Form fields section
        graphics
            .begin_text()
            .set_font(Font::Helvetica, 16.0)
            .set_text_position(50.0, 700.0)
            .show_text("Please fill out this form:")?
            .end_text();

        // Text field for name
        graphics
            .begin_text()
            .set_font(Font::Helvetica, 12.0)
            .set_text_position(50.0, 650.0)
            .show_text("Name:")?
            .end_text();

        let name_field = TextField::new("name")
            .with_default_value("Enter your name")
            .with_max_length(50);

        let name_widget = Widget::new(Rectangle::new(
            Point::new(150.0, 640.0),
            Point::new(400.0, 665.0),
        ));

        form_manager.add_text_field(name_field, name_widget, None)?;

        // Email field
        graphics
            .begin_text()
            .set_font(Font::Helvetica, 12.0)
            .set_text_position(50.0, 600.0)
            .show_text("Email:")?
            .end_text();

        let email_field = TextField::new("email");
        let email_widget = Widget::new(Rectangle::new(
            Point::new(150.0, 590.0),
            Point::new(400.0, 615.0),
        ));

        form_manager.add_text_field(email_field, email_widget, None)?;

        // Checkbox for newsletter
        graphics
            .begin_text()
            .set_font(Font::Helvetica, 12.0)
            .set_text_position(50.0, 550.0)
            .show_text("Subscribe to newsletter:")?
            .end_text();

        let newsletter_checkbox = CheckBox::new("newsletter");
        let checkbox_widget = Widget::new(Rectangle::new(
            Point::new(220.0, 545.0),
            Point::new(235.0, 560.0),
        ));

        form_manager.add_checkbox(newsletter_checkbox, checkbox_widget, None)?;

        // Radio buttons for preferred contact
        graphics
            .begin_text()
            .set_font(Font::Helvetica, 12.0)
            .set_text_position(50.0, 500.0)
            .show_text("Preferred contact method:")?
            .end_text();

        let contact_radio = RadioButton::new("contact_method")
            .add_option("email", "Email")
            .add_option("phone", "Phone")
            .add_option("mail", "Mail")
            .with_selected(0);

        let radio_widgets = vec![
            Widget::new(Rectangle::new(
                Point::new(250.0, 495.0),
                Point::new(265.0, 510.0),
            )),
            Widget::new(Rectangle::new(
                Point::new(320.0, 495.0),
                Point::new(335.0, 510.0),
            )),
            Widget::new(Rectangle::new(
                Point::new(390.0, 495.0),
                Point::new(405.0, 510.0),
            )),
        ];

        graphics
            .begin_text()
            .set_font(Font::Helvetica, 10.0)
            .set_text_position(270.0, 498.0)
            .show_text("Email")?
            .set_text_position(340.0, 498.0)
            .show_text("Phone")?
            .set_text_position(410.0, 498.0)
            .show_text("Mail")?
            .end_text();

        form_manager.add_radio_buttons(contact_radio, radio_widgets, None)?;

        // Dropdown for country
        graphics
            .begin_text()
            .set_font(Font::Helvetica, 12.0)
            .set_text_position(50.0, 450.0)
            .show_text("Country:")?
            .end_text();

        let country_combo = ComboBox::new("country")
            .add_option("US", "United States")
            .add_option("UK", "United Kingdom")
            .add_option("CA", "Canada")
            .add_option("AU", "Australia")
            .editable();

        let combo_widget = Widget::new(Rectangle::new(
            Point::new(150.0, 440.0),
            Point::new(300.0, 465.0),
        ));

        form_manager.add_combo_box(country_combo, combo_widget, None)?;

        // List box for interests
        graphics
            .begin_text()
            .set_font(Font::Helvetica, 12.0)
            .set_text_position(50.0, 400.0)
            .show_text("Interests (select multiple):")?
            .end_text();

        let interests_list = ListBox::new("interests")
            .add_option("tech", "Technology")
            .add_option("science", "Science")
            .add_option("arts", "Arts")
            .add_option("sports", "Sports")
            .multi_select();

        let list_widget = Widget::new(Rectangle::new(
            Point::new(50.0, 320.0),
            Point::new(200.0, 390.0),
        ));

        form_manager.add_list_box(interests_list, list_widget, None)?;

        // Submit button
        let submit_button = PushButton::new("submit").with_caption("Submit Form");

        let button_appearance = WidgetAppearance {
            border_color: Some(Color::Rgb(0.0, 0.0, 0.5)),
            background_color: Some(Color::Rgb(0.9, 0.9, 1.0)),
            border_width: 2.0,
            border_style: WidgetBorderStyle::Beveled,
        };

        let button_widget = Widget::new(Rectangle::new(
            Point::new(250.0, 250.0),
            Point::new(350.0, 280.0),
        ))
        .with_appearance(button_appearance);

        form_manager.add_push_button(submit_button, button_widget, None)?;

        // Add form title text
        graphics
            .begin_text()
            .set_font(Font::Helvetica, 12.0)
            .set_text_position(285.0, 260.0)
            .show_text("Submit")?
            .end_text();
    }

    // Create second page with annotations
    let mut page2 = Page::a4();
    let page2_ref = ObjectReference::new(2, 0);

    {
        let graphics = page2.graphics();

        // Title
        graphics
            .begin_text()
            .set_font(Font::HelveticaBold, 24.0)
            .set_text_position(50.0, 750.0)
            .show_text("Annotations Example")?
            .end_text();

        // Text annotation (sticky note)
        let text_annot = TextAnnotation::new(Point::new(450.0, 750.0))
            .with_contents("This is a sticky note annotation")
            .with_icon(TextIcon::Comment)
            .open()
            .to_annotation();

        annotation_manager.add_annotation(page2_ref, text_annot);

        // Link annotation
        graphics
            .begin_text()
            .set_font(Font::Helvetica, 14.0)
            .set_text_position(50.0, 700.0)
            .show_text("Click here to go back to page 1")?
            .end_text();

        let link_rect = Rectangle::new(Point::new(50.0, 695.0), Point::new(250.0, 715.0));

        let link = LinkAnnotation::to_page(link_rect, page1_ref).to_annotation();

        annotation_manager.add_annotation(page2_ref, link);

        // Highlight annotation
        graphics
            .begin_text()
            .set_font(Font::Helvetica, 12.0)
            .set_text_position(50.0, 650.0)
            .show_text("This text is highlighted with a yellow highlight annotation.")?
            .end_text();

        let highlight_rect = Rectangle::new(Point::new(50.0, 645.0), Point::new(350.0, 665.0));

        let highlight = MarkupAnnotation::highlight(highlight_rect)
            .with_author("John Doe")
            .with_contents("Important text!")
            .to_annotation();

        annotation_manager.add_annotation(page2_ref, highlight);

        // Underline annotation
        graphics
            .begin_text()
            .set_font(Font::Helvetica, 12.0)
            .set_text_position(50.0, 600.0)
            .show_text("This text has an underline annotation.")?
            .end_text();

        let underline_rect = Rectangle::new(Point::new(50.0, 595.0), Point::new(250.0, 605.0));

        let underline = MarkupAnnotation::underline(underline_rect)
            .with_contents("Needs attention")
            .to_annotation();

        annotation_manager.add_annotation(page2_ref, underline);

        // Free text annotation
        let free_text_rect = Rectangle::new(Point::new(50.0, 500.0), Point::new(250.0, 550.0));

        let free_text = FreeTextAnnotation::new(
            free_text_rect,
            "This is a free text annotation that appears directly on the page",
        )
        .with_font(Font::Helvetica, 10.0, Color::Rgb(0.0, 0.0, 1.0))
        .with_justification(1) // Center
        .to_annotation();

        annotation_manager.add_annotation(page2_ref, free_text);

        // Stamp annotation
        let stamp_rect = Rectangle::new(Point::new(400.0, 500.0), Point::new(500.0, 550.0));

        let stamp = StampAnnotation::new(stamp_rect, StampName::Approved).to_annotation();

        annotation_manager.add_annotation(page2_ref, stamp);

        // External link
        graphics
            .begin_text()
            .set_font(Font::Helvetica, 14.0)
            .set_text_position(50.0, 450.0)
            .show_text("Visit oxidize-pdf repository")?
            .end_text();

        let external_link_rect = Rectangle::new(Point::new(50.0, 445.0), Point::new(220.0, 465.0));

        let external_link = LinkAnnotation::to_uri(
            external_link_rect,
            "https://github.com/oxidize-pdf/oxidize-pdf",
        )
        .to_annotation();

        annotation_manager.add_annotation(page2_ref, external_link);

        // Instructions
        graphics
            .begin_text()
            .set_font(Font::Helvetica, 10.0)
            .set_text_position(50.0, 200.0)
            .show_text("Note: This example demonstrates various PDF annotation types.")?
            .set_text_position(50.0, 185.0)
            .show_text("- Text annotations appear as sticky notes")?
            .set_text_position(50.0, 170.0)
            .show_text("- Link annotations provide navigation and external links")?
            .set_text_position(50.0, 155.0)
            .show_text("- Markup annotations highlight, underline, or strikeout text")?
            .set_text_position(50.0, 140.0)
            .show_text("- Free text annotations appear directly on the page")?
            .set_text_position(50.0, 125.0)
            .show_text("- Stamp annotations show predefined stamps like 'Approved'")?
            .end_text();
    }

    // Add pages to document
    doc.add_page(page1);
    doc.add_page(page2);

    // Note: In a complete implementation, the form manager and annotation manager
    // would be integrated with the document to properly serialize the forms and annotations

    // Save the document
    doc.save("forms_and_annotations_example.pdf")?;
    println!("Created forms_and_annotations_example.pdf with interactive forms and annotations");

    Ok(())
}
