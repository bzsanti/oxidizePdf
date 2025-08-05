//! Lists demonstration
//!
//! This example showcases the list features including:
//! - Multiple numbering styles (decimal, alphabetic, roman)
//! - Custom bullet styles
//! - Nested lists
//! - Text wrapping for long items
//! - Custom formatting and colors
//! - International numbering styles

use oxidize_pdf::{
    BulletStyle, Color, Document, Font, ListElement, ListOptions, ListStyle, ListType, OrderedList,
    OrderedListStyle, Page, PageLists, Result, UnorderedList,
};

fn main() -> Result<()> {
    let mut doc = Document::new();
    doc.set_title("Lists Demo");
    doc.set_author("oxidize-pdf");

    // Page 1: Basic list styles
    create_basic_lists_page(&mut doc)?;

    // Page 2: Advanced numbering styles
    create_numbering_styles_page(&mut doc)?;

    // Page 3: Nested lists
    create_nested_lists_page(&mut doc)?;

    // Page 4: Formatted lists with wrapping
    create_formatted_lists_page(&mut doc)?;

    // Page 5: Document examples
    create_document_examples_page(&mut doc)?;

    doc.save("lists_demo.pdf")?;
    println!("✅ Created lists_demo.pdf");

    Ok(())
}

fn create_basic_lists_page(doc: &mut Document) -> Result<()> {
    let mut page = Page::a4();

    // Title
    page.text()
        .set_font(Font::HelveticaBold, 20.0)
        .at(50.0, 750.0)
        .write("Basic List Styles")?;

    // Ordered list with decimal numbers
    page.text()
        .set_font(Font::HelveticaBold, 14.0)
        .at(50.0, 700.0)
        .write("Ordered List (Decimal):")?;

    page.add_quick_ordered_list(
        vec![
            "First item".to_string(),
            "Second item".to_string(),
            "Third item".to_string(),
        ],
        70.0,
        670.0,
        OrderedListStyle::Decimal,
    )?;

    // Unordered list with bullets
    page.text()
        .set_font(Font::HelveticaBold, 14.0)
        .at(50.0, 580.0)
        .write("Unordered List (Bullets):")?;

    page.add_quick_unordered_list(
        vec![
            "Apples".to_string(),
            "Bananas".to_string(),
            "Cherries".to_string(),
        ],
        70.0,
        550.0,
        BulletStyle::Disc,
    )?;

    // Ordered list with upper alphabetic
    page.text()
        .set_font(Font::HelveticaBold, 14.0)
        .at(300.0, 700.0)
        .write("Ordered List (Letters):")?;

    page.add_quick_ordered_list(
        vec!["Alpha".to_string(), "Beta".to_string(), "Gamma".to_string()],
        320.0,
        670.0,
        OrderedListStyle::UpperAlpha,
    )?;

    // Unordered list with different bullets
    page.text()
        .set_font(Font::HelveticaBold, 14.0)
        .at(300.0, 580.0)
        .write("Different Bullet Styles:")?;

    let mut y = 550.0;
    for (bullet, label) in [
        (BulletStyle::Circle, "Circle bullet"),
        (BulletStyle::Square, "Square bullet"),
        (BulletStyle::Dash, "Dash bullet"),
        (BulletStyle::Custom('→'), "Arrow bullet"),
        (BulletStyle::Custom('★'), "Star bullet"),
    ] {
        page.add_quick_unordered_list(vec![label.to_string()], 320.0, y, bullet)?;
        y -= 30.0;
    }

    // Roman numerals
    page.text()
        .set_font(Font::HelveticaBold, 14.0)
        .at(50.0, 350.0)
        .write("Roman Numerals:")?;

    let mut roman_list = OrderedList::new(OrderedListStyle::UpperRoman);
    roman_list
        .add_item("Introduction".to_string())
        .add_item("Main Content".to_string())
        .add_item("Conclusion".to_string())
        .add_item("References".to_string())
        .add_item("Appendix".to_string());

    page.add_ordered_list(&roman_list, 70.0, 320.0)?;

    // Custom styled list
    page.text()
        .set_font(Font::HelveticaBold, 14.0)
        .at(50.0, 180.0)
        .write("Professional Style List:")?;

    let professional_style = ListStyle::professional(ListType::Ordered(OrderedListStyle::Decimal));
    page.add_styled_ordered_list(
        vec![
            "Executive Summary".to_string(),
            "Market Analysis".to_string(),
            "Financial Projections".to_string(),
            "Risk Assessment".to_string(),
        ],
        70.0,
        150.0,
        professional_style,
    )?;

    doc.add_page(page);
    Ok(())
}

fn create_numbering_styles_page(doc: &mut Document) -> Result<()> {
    let mut page = Page::a4();

    // Title
    page.text()
        .set_font(Font::HelveticaBold, 20.0)
        .at(50.0, 750.0)
        .write("Advanced Numbering Styles")?;

    // Decimal with leading zeros
    page.text()
        .set_font(Font::HelveticaBold, 12.0)
        .at(50.0, 700.0)
        .write("Leading Zeros:")?;

    let mut leading_zero_list = OrderedList::new(OrderedListStyle::DecimalLeadingZero);
    leading_zero_list
        .add_item("First item".to_string())
        .add_item("Second item".to_string())
        .add_item("Third item".to_string());
    page.add_ordered_list(&leading_zero_list, 70.0, 670.0)?;

    // Greek letters
    page.text()
        .set_font(Font::HelveticaBold, 12.0)
        .at(300.0, 700.0)
        .write("Greek Letters:")?;

    let mut greek_list = OrderedList::new(OrderedListStyle::GreekLower);
    greek_list
        .add_item("Alpha".to_string())
        .add_item("Beta".to_string())
        .add_item("Gamma".to_string())
        .add_item("Delta".to_string())
        .add_item("Epsilon".to_string());
    page.add_ordered_list(&greek_list, 320.0, 670.0)?;

    // Hebrew letters
    page.text()
        .set_font(Font::HelveticaBold, 12.0)
        .at(50.0, 550.0)
        .write("Hebrew Letters:")?;

    let mut hebrew_list = OrderedList::new(OrderedListStyle::Hebrew);
    hebrew_list
        .add_item("Aleph".to_string())
        .add_item("Bet".to_string())
        .add_item("Gimel".to_string())
        .add_item("Dalet".to_string());
    page.add_ordered_list(&hebrew_list, 70.0, 520.0)?;

    // Japanese Hiragana
    page.text()
        .set_font(Font::HelveticaBold, 12.0)
        .at(300.0, 550.0)
        .write("Japanese Hiragana:")?;

    let mut hiragana_list = OrderedList::new(OrderedListStyle::Hiragana);
    hiragana_list
        .add_item("First".to_string())
        .add_item("Second".to_string())
        .add_item("Third".to_string())
        .add_item("Fourth".to_string());
    page.add_ordered_list(&hiragana_list, 320.0, 520.0)?;

    // Japanese Katakana
    page.text()
        .set_font(Font::HelveticaBold, 12.0)
        .at(50.0, 400.0)
        .write("Japanese Katakana:")?;

    let mut katakana_list = OrderedList::new(OrderedListStyle::Katakana);
    katakana_list
        .add_item("First".to_string())
        .add_item("Second".to_string())
        .add_item("Third".to_string())
        .add_item("Fourth".to_string());
    page.add_ordered_list(&katakana_list, 70.0, 370.0)?;

    // Chinese numbers
    page.text()
        .set_font(Font::HelveticaBold, 12.0)
        .at(300.0, 400.0)
        .write("Chinese Numbers:")?;

    let mut chinese_list = OrderedList::new(OrderedListStyle::ChineseSimplified);
    chinese_list
        .add_item("First".to_string())
        .add_item("Second".to_string())
        .add_item("Third".to_string())
        .add_item("Fourth".to_string())
        .add_item("Fifth".to_string());
    page.add_ordered_list(&chinese_list, 320.0, 370.0)?;

    // Custom prefix and suffix
    page.text()
        .set_font(Font::HelveticaBold, 12.0)
        .at(50.0, 250.0)
        .write("Custom Prefix/Suffix:")?;

    let mut custom_list = OrderedList::new(OrderedListStyle::Decimal);
    let mut options = ListOptions::default();
    options.marker_prefix = "Chapter ".to_string();
    options.marker_suffix = ":".to_string();
    options.marker_font = Font::HelveticaBold;
    options.marker_color = Some(Color::rgb(0.2, 0.4, 0.7));
    custom_list.set_options(options);

    custom_list
        .add_item("Introduction".to_string())
        .add_item("Background".to_string())
        .add_item("Methodology".to_string());
    page.add_ordered_list(&custom_list, 70.0, 220.0)?;

    doc.add_page(page);
    Ok(())
}

fn create_nested_lists_page(doc: &mut Document) -> Result<()> {
    let mut page = Page::a4();

    // Title
    page.text()
        .set_font(Font::HelveticaBold, 20.0)
        .at(50.0, 750.0)
        .write("Nested Lists")?;

    // Create a complex nested list
    let mut main_list = OrderedList::new(OrderedListStyle::Decimal);

    // First item with sub-list
    let mut sub_list1 = UnorderedList::new(BulletStyle::Circle);
    sub_list1
        .add_item("Sub-item 1.1".to_string())
        .add_item("Sub-item 1.2".to_string())
        .add_item("Sub-item 1.3".to_string());

    main_list.add_item_with_children(
        "First main item".to_string(),
        vec![ListElement::Unordered(sub_list1)],
    );

    // Second item with nested ordered list
    let mut sub_list2 = OrderedList::new(OrderedListStyle::LowerAlpha);

    // Add a deeply nested list
    let mut sub_sub_list = UnorderedList::new(BulletStyle::Dash);
    sub_sub_list
        .add_item("Deep item i".to_string())
        .add_item("Deep item ii".to_string());

    sub_list2
        .add_item("Sub-item 2.a".to_string())
        .add_item_with_children(
            "Sub-item 2.b (with nested items)".to_string(),
            vec![ListElement::Unordered(sub_sub_list)],
        )
        .add_item("Sub-item 2.c".to_string());

    main_list.add_item_with_children(
        "Second main item".to_string(),
        vec![ListElement::Ordered(sub_list2)],
    );

    // Third item without children
    main_list.add_item("Third main item (no children)".to_string());

    // Fourth item with mixed children
    let mut sub_list3_ordered = OrderedList::new(OrderedListStyle::LowerRoman);
    sub_list3_ordered
        .add_item("Roman sub-item i".to_string())
        .add_item("Roman sub-item ii".to_string());

    let mut sub_list3_unordered = UnorderedList::new(BulletStyle::Square);
    sub_list3_unordered
        .add_item("Square bullet item".to_string())
        .add_item("Another square item".to_string());

    main_list.add_item_with_children(
        "Fourth main item (mixed children)".to_string(),
        vec![
            ListElement::Ordered(sub_list3_ordered),
            ListElement::Unordered(sub_list3_unordered),
        ],
    );

    page.add_ordered_list(&main_list, 70.0, 700.0)?;

    // Document structure example
    page.text()
        .set_font(Font::HelveticaBold, 16.0)
        .at(50.0, 350.0)
        .write("Document Structure Example:")?;

    let mut doc_list = OrderedList::new(OrderedListStyle::UpperRoman);
    let mut doc_options = ListOptions::default();
    doc_options.font_size = 12.0;
    doc_options.line_spacing = 1.4;
    doc_list.set_options(doc_options);

    // Create sections
    let mut section1_list = OrderedList::new(OrderedListStyle::Decimal);
    section1_list
        .add_item("Problem Statement".to_string())
        .add_item("Research Questions".to_string())
        .add_item("Objectives".to_string());

    let mut section2_list = OrderedList::new(OrderedListStyle::Decimal);
    let mut section2_subsection = OrderedList::new(OrderedListStyle::LowerAlpha);
    section2_subsection
        .add_item("Quantitative Methods".to_string())
        .add_item("Qualitative Methods".to_string());

    section2_list
        .add_item("Literature Review".to_string())
        .add_item_with_children(
            "Research Design".to_string(),
            vec![ListElement::Ordered(section2_subsection)],
        )
        .add_item("Data Collection".to_string());

    doc_list
        .add_item_with_children(
            "Introduction".to_string(),
            vec![ListElement::Ordered(section1_list)],
        )
        .add_item_with_children(
            "Methodology".to_string(),
            vec![ListElement::Ordered(section2_list)],
        )
        .add_item("Results and Discussion".to_string())
        .add_item("Conclusion".to_string());

    page.add_ordered_list(&doc_list, 70.0, 320.0)?;

    doc.add_page(page);
    Ok(())
}

fn create_formatted_lists_page(doc: &mut Document) -> Result<()> {
    let mut page = Page::a4();

    // Title
    page.text()
        .set_font(Font::HelveticaBold, 20.0)
        .at(50.0, 750.0)
        .write("Formatted Lists with Text Wrapping")?;

    // List with long text that wraps
    page.text()
        .set_font(Font::HelveticaBold, 14.0)
        .at(50.0, 700.0)
        .write("Text Wrapping Example:")?;

    let mut wrap_list = OrderedList::new(OrderedListStyle::Decimal);
    let mut wrap_options = ListOptions::default();
    wrap_options.max_width = Some(400.0);
    wrap_options.font_size = 11.0;
    wrap_options.line_spacing = 1.3;
    wrap_options.paragraph_spacing = 5.0;
    wrap_list.set_options(wrap_options);

    wrap_list
        .add_item("This is a very long list item that demonstrates text wrapping functionality. When the text exceeds the maximum width, it automatically wraps to the next line with proper indentation.".to_string())
        .add_item("Short item.".to_string())
        .add_item("Another long item that shows how the wrapping works consistently across different items in the list. The wrapped lines are indented to align with the start of the text.".to_string());

    page.add_ordered_list(&wrap_list, 70.0, 670.0)?;

    // Checklist style with separators
    page.text()
        .set_font(Font::HelveticaBold, 14.0)
        .at(50.0, 480.0)
        .write("Checklist with Separators:")?;

    let checklist_style = ListStyle::checklist();
    page.add_styled_unordered_list(
        vec![
            "Complete project documentation".to_string(),
            "Review code with team".to_string(),
            "Run all unit tests".to_string(),
            "Deploy to staging environment".to_string(),
            "Perform user acceptance testing".to_string(),
        ],
        70.0,
        450.0,
        checklist_style,
    )?;

    // Presentation style list
    page.text()
        .set_font(Font::HelveticaBold, 14.0)
        .at(50.0, 280.0)
        .write("Presentation Style:")?;

    let presentation_style = ListStyle::presentation(ListType::Unordered(BulletStyle::Custom('▸')));
    page.add_styled_unordered_list(
        vec![
            "Key Achievement #1".to_string(),
            "Key Achievement #2".to_string(),
            "Key Achievement #3".to_string(),
        ],
        70.0,
        250.0,
        presentation_style,
    )?;

    // Colored markers
    page.text()
        .set_font(Font::HelveticaBold, 14.0)
        .at(50.0, 150.0)
        .write("Colored Markers:")?;

    let mut colored_list = UnorderedList::new(BulletStyle::Custom('●'));
    let mut colored_options = ListOptions::default();
    colored_options.marker_color = Some(Color::rgb(0.8, 0.2, 0.2));
    colored_options.font_size = 12.0;
    colored_list.set_options(colored_options);

    colored_list
        .add_item("Red bullet point".to_string())
        .add_item("Another red bullet".to_string())
        .add_item("Third red bullet".to_string());

    page.add_unordered_list(&colored_list, 70.0, 120.0)?;

    doc.add_page(page);
    Ok(())
}

fn create_document_examples_page(doc: &mut Document) -> Result<()> {
    let mut page = Page::a4();

    // Title
    page.text()
        .set_font(Font::HelveticaBold, 20.0)
        .at(50.0, 750.0)
        .write("Real Document Examples")?;

    // Recipe ingredients
    page.text()
        .set_font(Font::HelveticaBold, 16.0)
        .at(50.0, 700.0)
        .write("Recipe: Chocolate Chip Cookies")?;

    page.text()
        .set_font(Font::HelveticaBold, 12.0)
        .at(50.0, 670.0)
        .write("Ingredients:")?;

    let ingredients_style = ListStyle::minimal(ListType::Unordered(BulletStyle::Dash));
    page.add_styled_unordered_list(
        vec![
            "2¼ cups all-purpose flour".to_string(),
            "1 tsp baking soda".to_string(),
            "1 tsp salt".to_string(),
            "1 cup butter, softened".to_string(),
            "¾ cup granulated sugar".to_string(),
            "¾ cup packed brown sugar".to_string(),
            "2 large eggs".to_string(),
            "2 tsp vanilla extract".to_string(),
            "2 cups chocolate chips".to_string(),
        ],
        70.0,
        640.0,
        ingredients_style,
    )?;

    // Instructions
    page.text()
        .set_font(Font::HelveticaBold, 12.0)
        .at(50.0, 430.0)
        .write("Instructions:")?;

    let mut instructions = OrderedList::new(OrderedListStyle::Decimal);
    let mut inst_options = ListOptions::default();
    inst_options.font_size = 10.0;
    inst_options.max_width = Some(500.0);
    inst_options.paragraph_spacing = 3.0;
    instructions.set_options(inst_options);

    instructions
        .add_item("Preheat oven to 375°F (190°C).".to_string())
        .add_item("In a small bowl, mix flour, baking soda, and salt. Set aside.".to_string())
        .add_item("In a large bowl, beat butter and sugars until creamy. Add eggs and vanilla, beating well.".to_string())
        .add_item("Gradually add flour mixture to butter mixture, beating until well blended. Stir in chocolate chips.".to_string())
        .add_item("Drop rounded tablespoons of dough onto ungreased cookie sheets.".to_string())
        .add_item("Bake 9-11 minutes or until golden brown. Cool on cookie sheets for 2 minutes; remove to wire racks.".to_string());

    page.add_ordered_list(&instructions, 70.0, 400.0)?;

    // Legal document outline
    page.text()
        .set_font(Font::HelveticaBold, 16.0)
        .at(50.0, 230.0)
        .write("Contract Outline")?;

    let mut legal_list = OrderedList::new(OrderedListStyle::UpperRoman);
    let mut legal_options = ListOptions::default();
    legal_options.font = Font::TimesRoman;
    legal_options.font_size = 11.0;
    legal_options.marker_prefix = "Article ".to_string();
    legal_options.marker_suffix = ".".to_string();
    legal_options.marker_font = Font::TimesBold;
    legal_list.set_options(legal_options);

    // Add articles with subsections
    let mut subsection1 = OrderedList::new(OrderedListStyle::Decimal);
    subsection1
        .add_item("Scope of Work".to_string())
        .add_item("Deliverables".to_string())
        .add_item("Timeline".to_string());

    let mut subsection2 = OrderedList::new(OrderedListStyle::Decimal);
    subsection2
        .add_item("Payment Schedule".to_string())
        .add_item("Late Payment Penalties".to_string());

    legal_list
        .add_item_with_children(
            "DEFINITIONS AND SCOPE".to_string(),
            vec![ListElement::Ordered(subsection1)],
        )
        .add_item_with_children(
            "PAYMENT TERMS".to_string(),
            vec![ListElement::Ordered(subsection2)],
        )
        .add_item("CONFIDENTIALITY".to_string())
        .add_item("TERMINATION".to_string())
        .add_item("GOVERNING LAW".to_string());

    page.add_ordered_list(&legal_list, 70.0, 200.0)?;

    doc.add_page(page);
    Ok(())
}
