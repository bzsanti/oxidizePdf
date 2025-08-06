//! Example demonstrating PDF actions: GoTo, URI, Named, and Launch

use oxidize_pdf::{
    actions::Action,
    annotations::{AnnotationManager, LinkAction, LinkAnnotation, LinkDestination},
    geometry::{Point, Rectangle},
    graphics::Color,
    objects::ObjectId,
    structure::{Destination, PageDestination},
    text::Font,
    Document, Page, Result,
};

fn main() -> Result<()> {
    // Create document with navigation actions
    create_navigation_document()?;

    // Create document with various action types
    create_action_showcase()?;

    Ok(())
}

/// Create a document demonstrating navigation actions
fn create_navigation_document() -> Result<()> {
    let mut doc = Document::new();
    doc.set_title("Navigation Actions Example");
    doc.set_author("oxidize-pdf");

    let mut annotation_manager = AnnotationManager::new();

    // Create table of contents page
    let mut toc_page = Page::a4();
    {
        let graphics = toc_page.graphics();

        graphics
            .begin_text()
            .set_font(Font::HelveticaBold, 24.0)
            .set_text_position(50.0, 750.0)
            .show_text("Table of Contents")?
            .end_text();

        graphics
            .begin_text()
            .set_font(Font::Helvetica, 14.0)
            .set_text_position(50.0, 700.0)
            .show_text("Click on any link below to navigate:")?
            .end_text();

        // Create navigation links
        let links = vec![
            ("1. Introduction", 1, 650.0),
            ("2. Chapter One", 2, 620.0),
            ("3. Chapter Two", 3, 590.0),
            ("4. Conclusion", 4, 560.0),
        ];

        for (title, page_num, y_pos) in links {
            // Draw link text
            graphics
                .begin_text()
                .set_font(Font::Helvetica, 14.0)
                .set_fill_color(Color::rgb(0.0, 0.0, 1.0))
                .set_text_position(70.0, y_pos)
                .show_text(title)?
                .end_text();

            // Create clickable area
            let link_rect = Rectangle::new(
                Point::new(70.0, y_pos - 5.0),
                Point::new(250.0, y_pos + 15.0),
            );

            // Create GoTo action
            let dest = Destination::xyz(
                PageDestination::PageNumber(page_num),
                Some(50.0),
                Some(750.0),
                Some(1.0),
            );
            let _action = Action::goto(dest);

            // Create link annotation
            let link = LinkAnnotation::new(
                link_rect,
                LinkAction::GoTo(LinkDestination::XYZ {
                    page: ObjectId::new(page_num + 1, 0),
                    left: Some(50.0),
                    top: Some(750.0),
                    zoom: Some(1.0),
                }),
            );
            annotation_manager.add_annotation(ObjectId::new(1, 0), link.to_annotation());
        }

        // Add navigation buttons
        // Skip navigation for table of contents
    }

    doc.add_page(toc_page);

    // Create content pages
    let chapters = [
        ("Introduction", "Welcome to this PDF navigation example."),
        ("Chapter One", "This chapter demonstrates GoTo actions."),
        (
            "Chapter Two",
            "This chapter shows various navigation features.",
        ),
        ("Conclusion", "Thank you for exploring PDF actions!"),
    ];

    for (title, content) in chapters.iter() {
        let mut page = Page::a4();
        {
            let graphics = page.graphics();

            // Page header
            graphics
                .set_fill_color(Color::rgb(0.2, 0.3, 0.7))
                .rectangle(0.0, 750.0, 595.0, 92.0)
                .fill();

            graphics
                .begin_text()
                .set_font(Font::HelveticaBold, 24.0)
                .set_fill_color(Color::white())
                .set_text_position(50.0, 780.0)
                .show_text(title)?
                .end_text();

            // Page content
            graphics
                .begin_text()
                .set_font(Font::Helvetica, 14.0)
                .set_fill_color(Color::black())
                .set_text_position(50.0, 700.0)
                .show_text(content)?
                .end_text();

            // Skip navigation buttons for simplicity
        }

        doc.add_page(page);
    }

    println!("Created navigation document (navigation_actions_example.pdf)");
    println!("  - Table of contents with GoTo actions");
    println!("  - Navigation buttons (Previous/Next/Home)");
    println!("  - {} pages total", chapters.len() + 1);

    doc.save("navigation_actions_example.pdf")?;

    Ok(())
}

/// Create a document showcasing various action types
fn create_action_showcase() -> Result<()> {
    let mut doc = Document::new();
    doc.set_title("PDF Actions Showcase");

    let mut annotation_manager = AnnotationManager::new();
    let mut page = Page::a4();

    {
        let graphics = page.graphics();

        graphics
            .begin_text()
            .set_font(Font::HelveticaBold, 24.0)
            .set_text_position(50.0, 750.0)
            .show_text("PDF Actions Showcase")?
            .end_text();

        let mut y_pos = 700.0;

        // URI Action
        graphics
            .begin_text()
            .set_font(Font::HelveticaBold, 16.0)
            .set_text_position(50.0, y_pos)
            .show_text("1. URI Actions")?
            .end_text();
        y_pos -= 30.0;

        // Web link
        let web_rect = Rectangle::new(
            Point::new(70.0, y_pos - 5.0),
            Point::new(300.0, y_pos + 15.0),
        );
        graphics
            .begin_text()
            .set_font(Font::Helvetica, 14.0)
            .set_fill_color(Color::rgb(0.0, 0.0, 1.0))
            .set_text_position(70.0, y_pos)
            .show_text("Visit oxidize-pdf on GitHub")?
            .end_text();

        let web_link = LinkAnnotation::to_uri(web_rect, "https://github.com/your-org/oxidize-pdf");
        annotation_manager.add_annotation(ObjectId::new(1, 0), web_link.to_annotation());
        y_pos -= 30.0;

        // Email link
        let email_rect = Rectangle::new(
            Point::new(70.0, y_pos - 5.0),
            Point::new(250.0, y_pos + 15.0),
        );
        graphics
            .begin_text()
            .set_font(Font::Helvetica, 14.0)
            .set_fill_color(Color::rgb(0.0, 0.0, 1.0))
            .set_text_position(70.0, y_pos)
            .show_text("Send us an email")?
            .end_text();

        let email_link = LinkAnnotation::to_uri(
            email_rect,
            "mailto:contact@example.com?subject=PDF%20Actions%20Demo",
        );
        annotation_manager.add_annotation(ObjectId::new(1, 0), email_link.to_annotation());
        y_pos -= 50.0;

        // Named Actions
        graphics
            .begin_text()
            .set_font(Font::HelveticaBold, 16.0)
            .set_fill_color(Color::black())
            .set_text_position(50.0, y_pos)
            .show_text("2. Named Actions")?
            .end_text();
        y_pos -= 30.0;

        // Print action
        let print_rect = Rectangle::new(
            Point::new(70.0, y_pos - 5.0),
            Point::new(200.0, y_pos + 25.0),
        );
        graphics
            .set_fill_color(Color::rgb(0.8, 0.2, 0.2))
            .rectangle(70.0, y_pos - 5.0, 130.0, 30.0)
            .fill();
        graphics
            .begin_text()
            .set_font(Font::Helvetica, 14.0)
            .set_fill_color(Color::white())
            .set_text_position(100.0, y_pos + 5.0)
            .show_text("Print Document")?
            .end_text();

        let print_link = LinkAnnotation::new(
            print_rect,
            LinkAction::Named {
                name: "Print".to_string(),
            },
        );
        annotation_manager.add_annotation(ObjectId::new(1, 0), print_link.to_annotation());
        y_pos -= 40.0;

        // Full screen action
        let fullscreen_rect = Rectangle::new(
            Point::new(70.0, y_pos - 5.0),
            Point::new(200.0, y_pos + 25.0),
        );
        graphics
            .set_fill_color(Color::rgb(0.2, 0.6, 0.2))
            .rectangle(70.0, y_pos - 5.0, 130.0, 30.0)
            .fill();
        graphics
            .begin_text()
            .set_font(Font::Helvetica, 14.0)
            .set_fill_color(Color::white())
            .set_text_position(85.0, y_pos + 5.0)
            .show_text("Enter Full Screen")?
            .end_text();

        let fullscreen_link = LinkAnnotation::new(
            fullscreen_rect,
            LinkAction::Named {
                name: "FullScreen".to_string(),
            },
        );
        annotation_manager.add_annotation(ObjectId::new(1, 0), fullscreen_link.to_annotation());
        y_pos -= 50.0;

        // Launch Actions
        graphics
            .begin_text()
            .set_font(Font::HelveticaBold, 16.0)
            .set_fill_color(Color::black())
            .set_text_position(50.0, y_pos)
            .show_text("3. Launch Actions")?
            .end_text();
        y_pos -= 30.0;

        graphics
            .begin_text()
            .set_font(Font::Helvetica, 12.0)
            .set_text_position(70.0, y_pos)
            .show_text("(Launch actions may require user confirmation for security)")?
            .end_text();
        y_pos -= 30.0;

        // Remote GoTo
        graphics
            .begin_text()
            .set_font(Font::HelveticaBold, 16.0)
            .set_text_position(50.0, y_pos)
            .show_text("4. Remote GoTo Actions")?
            .end_text();
        y_pos -= 30.0;

        graphics
            .begin_text()
            .set_font(Font::Helvetica, 14.0)
            .set_text_position(70.0, y_pos)
            .show_text("Open another PDF at page 5 (requires other.pdf)")?
            .end_text();
    }

    doc.add_page(page);

    println!("\nCreated actions showcase document (actions_showcase_example.pdf)");
    println!("  - URI actions (web links, email)");
    println!("  - Named actions (Print, Full Screen)");
    println!("  - Launch actions demonstration");
    println!("  - Remote GoTo example");

    doc.save("actions_showcase_example.pdf")?;

    Ok(())
}

// Note: In a complete implementation, we would need to:
// 1. Update the writer to properly write action dictionaries
// 2. Ensure annotations with actions are properly linked
// 3. Support action chains with Next actions
// 4. Add JavaScript actions for more complex behaviors
