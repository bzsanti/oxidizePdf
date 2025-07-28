//! Example demonstrating multi-column layout in PDF documents

use oxidize_pdf::{
    graphics::Color,
    text::{ColumnContent, ColumnLayout, ColumnOptions, TextAlign, TextFormat},
    Document, Font, Page, Result,
};

fn main() -> Result<()> {
    // Create a new document
    let mut doc = Document::new();

    // Create a new page
    let mut page = Page::a4();

    // Example 1: Two-column newsletter layout
    {
        let mut layout = ColumnLayout::new(2, 500.0, 20.0);

        let options = ColumnOptions {
            font: Font::TimesRoman,
            font_size: 11.0,
            line_height: 1.4,
            text_align: TextAlign::Justified,
            balance_columns: true,
            show_separators: true,
            separator_color: Color::gray(0.5),
            ..Default::default()
        };

        layout.set_options(options);

        let content = ColumnContent::new(
            "Lorem ipsum dolor sit amet, consectetur adipiscing elit. Sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. Ut enim ad minim veniam, quis nostrud exercitation ullamco laboris nisi ut aliquip ex ea commodo consequat. Duis aute irure dolor in reprehenderit in voluptate velit esse cillum dolore eu fugiat nulla pariatur. Excepteur sint occaecat cupidatat non proident, sunt in culpa qui officia deserunt mollit anim id est laborum. Sed ut perspiciatis unde omnis iste natus error sit voluptatem accusantium doloremque laudantium, totam rem aperiam, eaque ipsa quae ab illo inventore veritatis et quasi architecto beatae vitae dicta sunt explicabo. Nemo enim ipsam voluptatem quia voluptas sit aspernatur aut odit aut fugit, sed quia consequuntur magni dolores eos qui ratione voluptatem sequi nesciunt."
        );

        page.graphics()
            .render_column_layout(&layout, &content, 50.0, 750.0, 200.0)?;
    }

    // Example 2: Three-column layout with custom widths
    {
        let mut layout = ColumnLayout::with_custom_widths(vec![150.0, 200.0, 150.0], 15.0);

        let options = ColumnOptions {
            font: Font::Helvetica,
            font_size: 10.0,
            text_color: Color::rgb(0.2, 0.2, 0.6),
            balance_columns: false, // Don't balance for this example
            ..Default::default()
        };

        layout.set_options(options);

        let content = ColumnContent::new(
            "This is a three-column layout with custom column widths. The first and third columns are narrower (150pt each) while the middle column is wider (200pt). This demonstrates how you can create asymmetric column layouts for different design needs. The text flows naturally from one column to the next, creating a professional newsletter or magazine-style layout."
        );

        page.graphics()
            .render_column_layout(&layout, &content, 50.0, 500.0, 150.0)?;
    }

    // Example 3: Single wide column (essentially no columns)
    {
        let mut layout = ColumnLayout::new(1, 450.0, 0.0);

        let options = ColumnOptions {
            font: Font::HelveticaBold,
            font_size: 14.0,
            text_align: TextAlign::Center,
            text_color: Color::rgb(0.8, 0.2, 0.2),
            ..Default::default()
        };

        layout.set_options(options);

        let content = ColumnContent::new(
            "NEWSLETTER HEADER - This demonstrates a single column layout that can be used for headers, titles, or other content that should span the full width of the document."
        );

        page.graphics()
            .render_column_layout(&layout, &content, 75.0, 320.0, 50.0)?;
    }

    // Example 4: Four-column layout for compact information
    {
        let mut layout = ColumnLayout::new(4, 480.0, 10.0);

        let options = ColumnOptions {
            font: Font::Courier,
            font_size: 8.0,
            line_height: 1.3,
            text_align: TextAlign::Left,
            balance_columns: true,
            show_separators: true,
            separator_color: Color::gray(0.3),
            separator_width: 0.5,
            ..Default::default()
        };

        layout.set_options(options);

        let content = ColumnContent::new(
            "This four-column layout uses a smaller font size and tighter spacing to fit more information in a compact format. It's perfect for reference materials, indexes, or detailed technical documentation where space efficiency is important. Each column is automatically balanced to distribute content evenly across all four columns."
        );

        page.graphics()
            .render_column_layout(&layout, &content, 60.0, 240.0, 100.0)?;
    }

    // Example 5: Magazine-style layout with formatting
    {
        let mut layout = ColumnLayout::new(2, 450.0, 25.0);

        let options = ColumnOptions {
            font: Font::TimesRoman,
            font_size: 12.0,
            line_height: 1.5,
            text_align: TextAlign::Left,
            balance_columns: true,
            ..Default::default()
        };

        layout.set_options(options);

        let mut content = ColumnContent::new(
            "THE FUTURE OF PDF PROCESSING: In the rapidly evolving world of document processing, PDF technology continues to play a crucial role. Modern applications require robust, efficient, and secure PDF handling capabilities. This is where oxidize-pdf shines as a pure Rust implementation that provides zero-dependency PDF generation and manipulation. The library's architecture focuses on performance, safety, and compliance with PDF standards, making it an ideal choice for both small applications and enterprise-scale document processing systems."
        );

        // Add some text formatting (basic implementation)
        content.add_format(TextFormat::new(0, 33).bold()); // Title

        page.graphics()
            .render_column_layout(&layout, &content, 75.0, 120.0, 100.0)?;
    }

    // Add page to document
    doc.add_page(page);

    // Save the document
    doc.save("multi_column_example.pdf")?;
    println!("Created multi_column_example.pdf with various column layout examples");

    Ok(())
}
