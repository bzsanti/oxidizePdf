//! Example demonstrating XRef streams (PDF 1.5+)
//!
//! This example shows how to create PDFs with XRef streams instead of
//! traditional XRef tables, which provides better compression and supports
//! object streams.

use oxidize_pdf::writer::WriterConfig;
use oxidize_pdf::{Color, Document, Font, Page, Result};
use std::fs;

fn main() -> Result<()> {
    // Create a simple document
    let mut doc = Document::new();
    doc.set_title("XRef Stream Example");
    doc.set_author("oxidize-pdf");
    doc.set_subject("Demonstrating XRef streams");

    // Add multiple pages to create more objects
    for i in 1..=5 {
        let mut page = Page::a4();

        // Add a title
        page.text()
            .set_font(Font::HelveticaBold, 24.0)
            .at(50.0, 750.0)
            .write(&format!("Page {i}"))?;

        // Add some content
        page.text()
            .set_font(Font::Helvetica, 12.0)
            .at(50.0, 700.0)
            .write("This PDF uses XRef streams instead of traditional XRef tables.")?;

        page.text()
            .at(50.0, 680.0)
            .write("XRef streams were introduced in PDF 1.5 and provide:")?;

        page.text()
            .at(70.0, 660.0)
            .write("• Better compression of cross-reference data")?;

        page.text()
            .at(70.0, 640.0)
            .write("• Support for compressed object streams")?;

        page.text()
            .at(70.0, 620.0)
            .write("• More efficient storage for large PDFs")?;

        // Add some graphics
        page.graphics()
            .set_fill_color(Color::rgb(0.9, 0.9, 0.9))
            .rect(50.0, 550.0, 495.0, 50.0)
            .fill();

        page.graphics()
            .set_stroke_color(Color::rgb(0.0, 0.0, 0.0))
            .set_line_width(2.0)
            .rect(50.0, 550.0, 495.0, 50.0)
            .stroke();

        page.text()
            .set_font(Font::HelveticaBold, 14.0)
            .at(297.5 - 50.0, 570.0) // Center text
            .write(&format!("Object Count Example {i}"))?;

        doc.add_page(page);
    }

    // Save with traditional XRef table (PDF 1.7)
    doc.save("output/xref_table_example.pdf")?;
    println!("Created PDF with traditional XRef table: output/xref_table_example.pdf");

    // Save with XRef streams (PDF 1.5)
    let config = WriterConfig {
        use_xref_streams: true,
        pdf_version: "1.5".to_string(),
        compress_streams: true,
    };
    doc.save_with_config("output/xref_stream_example.pdf", config)?;
    println!("Created PDF with XRef streams: output/xref_stream_example.pdf");

    // Compare file sizes
    let table_size = fs::metadata("output/xref_table_example.pdf")?.len();
    let stream_size = fs::metadata("output/xref_stream_example.pdf")?.len();

    println!("\nFile size comparison:");
    println!("  Traditional XRef table: {table_size} bytes");
    println!("  XRef stream:           {stream_size} bytes");

    let savings = if stream_size < table_size {
        let saved = table_size - stream_size;
        let percent = (saved as f64 / table_size as f64) * 100.0;
        format!("{saved} bytes ({percent:.1}% smaller)")
    } else {
        "No savings (document too small)".to_string()
    };

    println!("  Savings:               {savings}");

    Ok(())
}
