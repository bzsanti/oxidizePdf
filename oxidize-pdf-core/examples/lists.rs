//! Example demonstrating list rendering in PDF documents

use oxidize_pdf::{
    graphics::Color,
    text::{BulletStyle, ListElement, ListOptions, OrderedList, OrderedListStyle, UnorderedList},
    Document, Font, Page, Result,
};

fn main() -> Result<()> {
    // Create a new document
    let mut doc = Document::new();

    // Create a new page
    let mut page = Page::a4();

    // Example 1: Simple ordered list with decimal numbers
    {
        let mut list = OrderedList::new(OrderedListStyle::Decimal);
        list.set_position(50.0, 750.0);

        list.add_item("First item in the list".to_string())
            .add_item("Second item in the list".to_string())
            .add_item("Third item in the list".to_string())
            .add_item("Fourth item in the list".to_string());

        // Render the list
        page.graphics().render_list(&ListElement::Ordered(list))?;
    }

    // Example 2: Unordered list with different bullet styles
    {
        let mut list = UnorderedList::new(BulletStyle::Disc);
        list.set_position(50.0, 650.0);

        // Configure options
        let options = ListOptions {
            font: Font::Helvetica,
            font_size: 12.0,
            text_color: Color::rgb(0.2, 0.2, 0.8),
            ..Default::default()
        };
        list.set_options(options);

        list.add_item("Features of oxidize-pdf".to_string())
            .add_item("Native Rust implementation".to_string())
            .add_item("Zero external dependencies".to_string())
            .add_item("High performance".to_string());

        // Render the list
        page.graphics().render_list(&ListElement::Unordered(list))?;
    }

    // Example 3: Nested lists
    {
        let mut parent_list = OrderedList::new(OrderedListStyle::UpperAlpha);
        parent_list.set_position(50.0, 500.0);

        // Create nested unordered list
        let mut nested_unordered = UnorderedList::new(BulletStyle::Dash);
        nested_unordered
            .add_item("Subitem 1.1".to_string())
            .add_item("Subitem 1.2".to_string())
            .add_item("Subitem 1.3".to_string());

        // Create nested ordered list
        let mut nested_ordered = OrderedList::new(OrderedListStyle::LowerRoman);
        nested_ordered
            .add_item("Subitem 2.1".to_string())
            .add_item("Subitem 2.2".to_string());

        // Add items with children
        parent_list.add_item_with_children(
            "First main topic".to_string(),
            vec![ListElement::Unordered(nested_unordered)],
        );
        parent_list.add_item_with_children(
            "Second main topic".to_string(),
            vec![ListElement::Ordered(nested_ordered)],
        );
        parent_list.add_item("Third main topic (no subitems)".to_string());

        // Render the list
        page.graphics()
            .render_list(&ListElement::Ordered(parent_list))?;
    }

    // Example 4: Roman numerals list
    {
        let mut list = OrderedList::new(OrderedListStyle::UpperRoman);
        list.set_position(50.0, 300.0).set_start_number(1);

        let options = ListOptions {
            font: Font::TimesRoman,
            font_size: 14.0,
            line_spacing: 1.5,
            ..Default::default()
        };
        list.set_options(options);

        list.add_item("Chapter One: Introduction".to_string())
            .add_item("Chapter Two: Getting Started".to_string())
            .add_item("Chapter Three: Advanced Topics".to_string())
            .add_item("Chapter Four: Conclusion".to_string());

        // Render the list
        page.graphics().render_list(&ListElement::Ordered(list))?;
    }

    // Example 5: Custom bullet style
    {
        let mut list = UnorderedList::new(BulletStyle::Custom('★'));
        list.set_position(50.0, 150.0);

        let options = ListOptions {
            font: Font::Helvetica,
            font_size: 11.0,
            text_color: Color::rgb(0.8, 0.2, 0.2),
            ..Default::default()
        };
        list.set_options(options);

        list.add_item("Premium feature 1".to_string())
            .add_item("Premium feature 2".to_string())
            .add_item("Premium feature 3".to_string());

        // Render the list
        page.graphics().render_list(&ListElement::Unordered(list))?;
    }

    // Add page to document
    doc.add_page(page);

    // Save the document
    doc.save("lists_example.pdf")?;
    println!("Created lists_example.pdf with various list examples");

    Ok(())
}
