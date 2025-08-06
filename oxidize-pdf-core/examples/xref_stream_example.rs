//! Example of XRef stream support for PDF 1.5+
//!
//! This example demonstrates:
//! - Creating PDFs with cross-reference streams
//! - Reading PDFs with compressed objects
//! - Hybrid reference files

use oxidize_pdf::parser::xref_stream::{XRefEntry, XRefStreamBuilder};
use oxidize_pdf::{Document, Page, Result};

fn main() -> Result<()> {
    println!("XRef Stream Example - PDF 1.5+ Support");
    println!("=====================================\n");

    // Example 1: Create a PDF with xref stream
    create_pdf_with_xref_stream()?;

    // Example 2: Demonstrate xref stream structure
    demonstrate_xref_stream_structure();

    // Example 3: Show benefits of xref streams
    show_xref_stream_benefits();

    Ok(())
}

/// Create a PDF that uses xref streams
fn create_pdf_with_xref_stream() -> Result<()> {
    let mut doc = Document::new();
    doc.set_title("XRef Stream Example");
    // PDF 1.5 introduced xref streams

    // Create a page with content
    let mut page = Page::a4();

    page.text()
        .set_font(oxidize_pdf::text::Font::Helvetica, 16.0)
        .at(50.0, 750.0)
        .write("PDF 1.5+ XRef Stream Features")?;

    page.text()
        .set_font(oxidize_pdf::text::Font::Helvetica, 12.0)
        .at(50.0, 700.0)
        .write("This PDF demonstrates cross-reference streams:")?;

    let features = vec![
        "• More compact than traditional xref tables",
        "• Support for compressed objects",
        "• Better error recovery",
        "• Incremental update efficiency",
        "• Stream-based structure",
    ];

    let mut y = 670.0;
    for feature in features {
        page.text().at(70.0, y).write(feature)?;
        y -= 20.0;
    }

    // Add information about object compression
    page.text()
        .set_font(oxidize_pdf::text::Font::Helvetica, 14.0)
        .at(50.0, 550.0)
        .write("Object Stream Compression:")?;

    page.text()
        .set_font(oxidize_pdf::text::Font::Helvetica, 12.0)
        .at(50.0, 520.0)
        .write("PDF 1.5+ can compress multiple objects into object streams,")?;

    page.text()
        .at(50.0, 500.0)
        .write("significantly reducing file size for documents with many objects.")?;

    doc.add_page(page);

    // Save with xref stream (when supported)
    doc.save("xref_stream_example.pdf")?;
    println!("Created: xref_stream_example.pdf");

    Ok(())
}

/// Demonstrate the structure of xref streams
fn demonstrate_xref_stream_structure() {
    println!("\nXRef Stream Structure:");
    println!("=====================");

    // Create a sample xref stream
    let mut builder = XRefStreamBuilder::new();

    // Add various types of entries

    // Entry 0: Always free with generation 65535
    builder.add_entry(
        0,
        XRefEntry::Free {
            next_free_object: 0,
            generation: 65535,
        },
    );

    // Entry 1: Regular in-use object
    builder.add_entry(
        1,
        XRefEntry::InUse {
            offset: 15,
            generation: 0,
        },
    );

    // Entry 2: Another in-use object
    builder.add_entry(
        2,
        XRefEntry::InUse {
            offset: 1234,
            generation: 0,
        },
    );

    // Entry 3: Compressed object in stream 5
    builder.add_entry(
        3,
        XRefEntry::Compressed {
            stream_object_number: 5,
            index_within_stream: 0,
        },
    );

    // Entry 4: Another compressed object
    builder.add_entry(
        4,
        XRefEntry::Compressed {
            stream_object_number: 5,
            index_within_stream: 1,
        },
    );

    // Entry 5: The object stream itself
    builder.add_entry(
        5,
        XRefEntry::InUse {
            offset: 2000,
            generation: 0,
        },
    );

    println!("Sample XRef Stream Entries:");
    println!("  Object 0: Free (next=0, gen=65535)");
    println!("  Object 1: In use at offset 15");
    println!("  Object 2: In use at offset 1234");
    println!("  Object 3: Compressed in stream 5, index 0");
    println!("  Object 4: Compressed in stream 5, index 1");
    println!("  Object 5: In use at offset 2000 (object stream)");

    // Build the stream
    match builder.build() {
        Ok((dict, data)) => {
            println!("\nXRef Stream Dictionary:");
            if let Some(w_array) = dict.get("W") {
                println!("  /W {w_array:?} (field widths)");
            }
            if let Some(size) = dict.get("Size") {
                println!("  /Size {size:?} (number of entries)");
            }
            if let Some(filter) = dict.get("Filter") {
                println!("  /Filter {filter:?} (compression)");
            }
            println!("  Stream data size: {} bytes (compressed)", data.len());
        }
        Err(e) => {
            println!("Error building xref stream: {e}");
        }
    }
}

/// Show the benefits of xref streams
fn show_xref_stream_benefits() {
    println!("\n\nBenefits of XRef Streams:");
    println!("========================");

    println!("\n1. Size Comparison:");
    println!("   Traditional xref table entry: ~20 bytes");
    println!("   XRef stream entry: 3-6 bytes (compressed)");
    println!("   Savings: ~70% for large documents");

    println!("\n2. Object Compression Support:");
    println!("   - Multiple objects in single stream");
    println!("   - Shared compression dictionary");
    println!("   - Ideal for forms, annotations, metadata");

    println!("\n3. Better Structure:");
    println!("   - Binary format (no parsing ambiguity)");
    println!("   - Checksummed streams");
    println!("   - Self-contained with trailer info");

    println!("\n4. Incremental Updates:");
    println!("   - Efficient append-only updates");
    println!("   - Previous xref chain preserved");
    println!("   - Hybrid mode for compatibility");

    // Example of space savings calculation
    let num_objects = 1000;
    let traditional_size = num_objects * 20; // bytes
    let xref_stream_size = num_objects * 4; // average compressed
    let savings = ((traditional_size - xref_stream_size) as f64 / traditional_size as f64) * 100.0;

    println!("\n5. Example with {num_objects} objects:");
    println!("   Traditional: {traditional_size} bytes");
    println!("   XRef stream: {xref_stream_size} bytes");
    println!("   Space saved: {savings:.1}%");
}

/// Show hybrid reference file structure
#[allow(dead_code)]
fn demonstrate_hybrid_reference() {
    println!("\n\nHybrid Reference Files:");
    println!("======================");

    println!("\nA hybrid file contains both:");
    println!("1. Traditional xref table (for PDF <1.5 readers)");
    println!("2. XRef stream with /XRefStm entry");

    println!("\nStructure:");
    println!("  xref");
    println!("  0 6");
    println!("  0000000000 65535 f");
    println!("  0000000015 00000 n");
    println!("  ...");
    println!("  trailer");
    println!("  << /Size 6");
    println!("     /XRefStm 2000  % Points to xref stream");
    println!("     /Root 1 0 R");
    println!("  >>");

    println!("\nBenefits:");
    println!("- Backward compatibility");
    println!("- Modern features available");
    println!("- Graceful degradation");
}
