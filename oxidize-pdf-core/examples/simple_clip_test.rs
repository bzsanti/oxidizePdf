//! Simple test to verify clipping operators are generated in PDF content

use oxidize_pdf::{Color, Document, Page, Result};

fn main() -> Result<()> {
    let mut doc = Document::new();
    doc.set_title("Simple Clip Test");
    doc.set_compress(false); // Disable compression so we can read the content easily

    let mut page = Page::a4();

    // Test basic clipping
    page.graphics()
        .rect(50.0, 600.0, 100.0, 50.0)
        .clip()
        .set_fill_color(Color::red())
        .rect(0.0, 550.0, 200.0, 150.0)
        .fill();

    doc.add_page(page);

    // Generate uncompressed PDF so we can inspect the content streams
    let pdf_bytes = doc.to_bytes()?;
    let pdf_content = String::from_utf8_lossy(&pdf_bytes);

    // Debug: Print relevant parts of the PDF
    println!("=== PDF Content Analysis ===");

    // Look for content streams
    let mut in_stream = false;
    let mut stream_content = String::new();

    for line in pdf_content.lines() {
        if line == "stream" {
            in_stream = true;
            continue;
        }
        if line == "endstream" {
            if !stream_content.trim().is_empty() {
                println!("Content stream found:");
                println!("{}", stream_content);
                println!("---");

                // Check for clipping operator
                if stream_content.contains("W\n") || stream_content.contains("W ") {
                    println!("✅ SUCCESS: Found 'W' clipping operator in content stream");
                } else {
                    println!("❌ No 'W' clipping operator found");
                }
            }
            in_stream = false;
            stream_content.clear();
            continue;
        }
        if in_stream {
            stream_content.push_str(line);
            stream_content.push('\n');
        }
    }

    // Also check if the operations string directly contains the operators
    println!("\n=== Direct Graphics Operations Test ===");
    let mut test_page = Page::a4();
    let ops_before = test_page.graphics().operations().to_string();

    test_page.graphics().rect(10.0, 10.0, 50.0, 50.0).clip();

    let ops_after = test_page.graphics().operations().to_string();

    println!("Operations before: '{}'", ops_before);
    println!("Operations after: '{}'", ops_after);

    if ops_after.contains("W\n") {
        println!("✅ SUCCESS: 'W' operator found in graphics operations");
    } else {
        println!("❌ 'W' operator not found in graphics operations");
    }

    Ok(())
}
