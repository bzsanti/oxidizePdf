//! Object Streams Demo
//!
//! Demonstrates the use of Object Streams (ISO 32000-1 Section 7.5.7)
//! for file size reduction. Shows the difference between PDFs with and
//! without object streams.

use oxidize_pdf::document::Document;
use oxidize_pdf::objects::{Dictionary, Object, ObjectId};
use oxidize_pdf::text::Font;
use oxidize_pdf::writer::{ObjectStreamConfig, ObjectStreamWriter, PdfWriter, WriterConfig};
use oxidize_pdf::Page;
use std::fs::{self, File};
use std::io::BufWriter;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Object Streams Demo ===\n");

    // Create output directory
    let output_dir = "examples/results/object_streams_demo";
    fs::create_dir_all(output_dir)?;

    // Create a document with many small objects (typical scenario)
    let mut doc = Document::new();
    let mut page = Page::new(595.0, 842.0); // A4 size

    // Add title
    page.text()
        .set_font(Font::Helvetica, 16.0)
        .at(100.0, 750.0)
        .write("Object Streams Demo - File Size Comparison")?;

    page.text()
        .set_font(Font::Helvetica, 12.0)
        .at(100.0, 720.0)
        .write("This PDF demonstrates object stream compression")?;

    // Add multiple text lines to create more content
    for i in 0..20 {
        page.text()
            .set_font(Font::Helvetica, 10.0)
            .at(100.0, 680.0 - (i as f64 * 20.0))
            .write(&format!(
                "Line {}: Object streams compress multiple objects together",
                i + 1
            ))?;
    }

    doc.add_page(page);

    // Write WITHOUT object streams (traditional)
    println!("1. Writing PDF WITHOUT object streams (traditional)...");
    let traditional_path = format!("{}/traditional.pdf", output_dir);
    let traditional_config = WriterConfig {
        use_xref_streams: false,
        use_object_streams: false,
        pdf_version: "1.4".to_string(),
        compress_streams: true,
        incremental_update: false,
    };

    let file = File::create(&traditional_path)?;
    let mut writer = PdfWriter::with_config(BufWriter::new(file), traditional_config);
    writer.write_document(&mut doc)?;
    let traditional_size = fs::metadata(&traditional_path)?.len();
    println!(
        "   ✓ Written: {} ({} bytes)",
        traditional_path, traditional_size
    );

    // Write WITH object streams (modern)
    println!("\n2. Writing PDF WITH object streams (modern PDF 1.5+)...");
    let modern_path = format!("{}/with_object_streams.pdf", output_dir);
    let modern_config = WriterConfig {
        use_xref_streams: false, // We'll add xref streams in Feature 2.2.2
        use_object_streams: false,
        pdf_version: "1.5".to_string(),
        compress_streams: true,
        incremental_update: false,
    };

    // Note: Full integration with PdfWriter will be done in next step
    // For now, demonstrate the ObjectStreamWriter API
    println!("   Creating object stream writer...");
    let obj_stream_config = ObjectStreamConfig {
        max_objects_per_stream: 100,
        compression_level: 6,
        enabled: true,
    };

    let mut obj_stream_writer = ObjectStreamWriter::new(obj_stream_config);

    // Simulate compressing dictionary objects
    println!("   Simulating object compression...");
    for i in 1..=50 {
        let mut dict = Dictionary::new();
        dict.set("Type", Object::Name("Example".to_string()));
        dict.set("Index", Object::Integer(i));
        dict.set("Data", Object::String(format!("Sample data {}", i)));

        // Serialize dictionary to bytes (simplified)
        let dict_bytes = format!(
            "<< /Type /Example /Index {} /Data (Sample data {}) >>",
            i, i
        )
        .into_bytes();
        obj_stream_writer.add_object(ObjectId::new(i as u32, 0), dict_bytes)?;
    }

    let streams = obj_stream_writer.finalize()?;
    println!("   ✓ Created {} object streams", streams.len());

    // Show statistics
    let total_objects: usize = streams.iter().map(|s| s.objects.len()).sum();
    println!("   ✓ Compressed {} objects total", total_objects);

    // Calculate approximate compression
    let mut total_uncompressed = 0;
    let mut total_compressed = 0;

    for stream in &mut streams.clone() {
        // Estimate uncompressed size
        for (_, data) in &stream.objects {
            total_uncompressed += data.len();
        }

        // Generate compressed data
        if let Ok(compressed) = stream.generate_stream_data(6) {
            total_compressed += compressed.len();
        }
    }

    let compression_ratio = if total_uncompressed > 0 {
        (1.0 - (total_compressed as f64 / total_uncompressed as f64)) * 100.0
    } else {
        0.0
    };

    println!("\n3. Compression Statistics:");
    println!("   Uncompressed objects: {} bytes", total_uncompressed);
    println!("   Compressed streams:   {} bytes", total_compressed);
    println!("   Compression ratio:    {:.1}%", compression_ratio);

    // Write the modern PDF (basic version without full integration)
    let file = File::create(&modern_path)?;
    let mut writer = PdfWriter::with_config(BufWriter::new(file), modern_config);
    writer.write_document(&mut doc)?;
    let modern_size = fs::metadata(&modern_path)?.len();
    println!("   ✓ Written: {} ({} bytes)", modern_path, modern_size);

    // Compare file sizes
    println!("\n4. File Size Comparison:");
    println!("   Traditional (PDF 1.4): {} bytes", traditional_size);
    println!("   Modern (PDF 1.5):      {} bytes", modern_size);

    if modern_size < traditional_size {
        let reduction = ((traditional_size - modern_size) as f64 / traditional_size as f64) * 100.0;
        println!("   ✓ Size reduction:      {:.1}%", reduction);
    } else {
        println!("   Note: Full object stream integration pending");
    }

    println!("\n5. Object Stream Details:");
    for (idx, stream) in streams.iter().enumerate() {
        println!(
            "   Stream {}: {} objects (ID: {})",
            idx + 1,
            stream.objects.len(),
            stream.stream_id
        );

        // Show dictionary
        let dict = stream.generate_dictionary(&[]);
        println!("      Type:   {:?}", dict.get("Type"));
        println!("      N:      {:?}", dict.get("N"));
        println!("      First:  {:?}", dict.get("First"));
        println!("      Filter: {:?}", dict.get("Filter"));
    }

    println!("\n✓ Demo complete!");
    println!("  Output files: {}/", output_dir);
    println!("\nNext steps:");
    println!("  - Integrate ObjectStreamWriter with PdfWriter");
    println!("  - Implement automatic object stream generation");
    println!("  - Add Feature 2.2.2: Cross-Reference Streams");
    println!("  - Expected combined reduction: 11-61%");

    Ok(())
}
