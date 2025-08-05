//! Example demonstrating PDF annotations support
//!
//! This example shows how to add various types of annotations to a PDF page.

use oxidize_pdf::{
    annotations::{
        AnnotationBorderStyle as BorderStyle, AnnotationType, FreeTextAnnotation, Icon, LinkAction,
        LinkAnnotation, LinkDestination, MarkupAnnotation, MarkupType, QuadPoints,
        SquareAnnotation, TextAnnotation,
    },
    geometry::{Point, Rectangle},
    graphics::Color,
    objects::ObjectReference,
    Document, Font, Page, Result,
};

fn main() -> Result<()> {
    // Create a new document
    let mut doc = Document::new();
    doc.set_title("Annotations Example");
    doc.set_author("oxidize-pdf");

    // Create a page
    let mut page = Page::a4();

    // Add some content to annotate
    page.text()
        .set_font(Font::Helvetica, 24.0)
        .at(50.0, 750.0)
        .write("PDF Annotations Example")?;

    page.text()
        .set_font(Font::Helvetica, 12.0)
        .at(50.0, 700.0)
        .write("This document demonstrates various annotation types:")?;

    // 1. Text Annotation (Sticky Note)
    page.text()
        .at(50.0, 650.0)
        .write("1. Text annotation (hover over the icon) →")?;

    let text_annot = TextAnnotation::new(Point::new(350.0, 640.0))
        .with_contents("This is a text annotation, also known as a sticky note.")
        .with_icon(Icon::Note)
        .to_annotation();
    page.add_annotation(text_annot);

    // 2. Link Annotation
    page.text()
        .at(50.0, 600.0)
        .write("2. Click here to go to page 2")?;

    // Note: In a real app, you'd get the page reference from doc.add_page()
    // For now, we'll use a placeholder
    let page2_ref = ObjectReference::new(2, 0);
    let link_annot = LinkAnnotation::new(
        Rectangle::new(Point::new(50.0, 590.0), Point::new(200.0, 610.0)),
        LinkAction::GoTo(LinkDestination::Fit { page: page2_ref }),
    )
    .to_annotation();
    page.add_annotation(link_annot);

    // 3. URL Link
    page.text()
        .at(50.0, 550.0)
        .write("3. Visit oxidize-pdf on GitHub")?;

    let url_link = LinkAnnotation::new(
        Rectangle::new(Point::new(50.0, 540.0), Point::new(220.0, 560.0)),
        LinkAction::URI {
            uri: "https://github.com/oxidize-pdf/oxidize-pdf".to_string(),
        },
    )
    .to_annotation();
    page.add_annotation(url_link);

    // 4. Highlight Annotation
    page.text()
        .at(50.0, 500.0)
        .write("4. This text is highlighted with a yellow marker")?;

    let highlight_annot = MarkupAnnotation::new(
        MarkupType::Highlight,
        Rectangle::new(Point::new(50.0, 490.0), Point::new(300.0, 510.0)),
        QuadPoints::from_rect(&Rectangle::new(
            Point::new(90.0, 490.0),
            Point::new(300.0, 510.0),
        )),
    )
    .to_annotation();
    page.add_annotation(highlight_annot);

    // 5. Underline Annotation
    page.text()
        .at(50.0, 450.0)
        .write("5. This text has an underline annotation")?;

    let underline_annot = MarkupAnnotation::new(
        MarkupType::Underline,
        Rectangle::new(Point::new(50.0, 440.0), Point::new(250.0, 460.0)),
        QuadPoints::from_rect(&Rectangle::new(
            Point::new(90.0, 440.0),
            Point::new(250.0, 460.0),
        )),
    )
    .to_annotation()
    .with_color(Color::rgb(1.0, 0.0, 0.0)); // Red underline
    page.add_annotation(underline_annot);

    // 6. Free Text Annotation
    let free_text = FreeTextAnnotation::new(
        Rectangle::new(Point::new(350.0, 400.0), Point::new(550.0, 500.0)),
        "This is a free text annotation.\nIt can contain multiple lines\nand appears directly on the page.",
    ).to_annotation()
        .with_color(Color::rgb(0.0, 0.0, 1.0)); // Blue text
    page.add_annotation(free_text);

    // 7. Square Annotation
    let square = SquareAnnotation::new(Rectangle::new(
        Point::new(50.0, 300.0),
        Point::new(150.0, 350.0),
    ))
    .to_annotation()
    .with_contents("This is a square annotation")
    .with_color(Color::rgb(0.0, 0.5, 0.0)) // Green
    .with_border(BorderStyle::default());
    page.add_annotation(square);

    // 8. Circle Annotation
    let mut circle_ann = SquareAnnotation::new(Rectangle::new(
        Point::new(200.0, 300.0),
        Point::new(300.0, 350.0),
    ))
    .to_annotation();
    circle_ann.annotation_type = AnnotationType::Circle;
    circle_ann.contents = Some("This is a circle annotation".to_string());
    circle_ann.color = Some(Color::rgb(0.5, 0.0, 0.5)); // Purple
    circle_ann.border = Some(BorderStyle::default());
    page.add_annotation(circle_ann);

    // Add the page to the document
    doc.add_page(page);

    // Create a second page (for the link destination)
    let mut page2 = Page::a4();
    page2
        .text()
        .set_font(Font::Helvetica, 24.0)
        .at(50.0, 750.0)
        .write("Page 2")?;

    page2
        .text()
        .set_font(Font::Helvetica, 12.0)
        .at(50.0, 700.0)
        .write("You arrived here by clicking the link on page 1!")?;

    doc.add_page(page2);

    // Save the document
    doc.save("annotations_example.pdf")?;
    println!("Created annotations_example.pdf");

    // Print summary
    println!("\nAnnotations created:");
    println!("- Text annotation (sticky note)");
    println!("- Internal link to page 2");
    println!("- External URL link");
    println!("- Highlight annotation");
    println!("- Underline annotation");
    println!("- Free text annotation");
    println!("- Square annotation");
    println!("- Circle annotation");

    Ok(())
}
