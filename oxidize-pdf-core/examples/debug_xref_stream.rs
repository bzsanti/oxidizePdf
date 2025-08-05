//! Debug XRef stream generation

use oxidize_pdf::writer::WriterConfig;
use oxidize_pdf::{Document, Font, Page, Result};
use std::fs;

fn main() -> Result<()> {
    // Create a minimal document
    let mut doc = Document::new();
    doc.set_title("XRef Debug");

    let mut page = Page::a4();
    page.text()
        .set_font(Font::Helvetica, 12.0)
        .at(100.0, 700.0)
        .write("Test")?;

    doc.add_page(page);

    // Write to buffer with XRef streams
    let mut buffer = Vec::new();
    {
        let config = WriterConfig {
            use_xref_streams: true,
            pdf_version: "1.5".to_string(),
            compress_streams: true,
        };
        let mut writer = oxidize_pdf::writer::PdfWriter::with_config(&mut buffer, config);
        writer.write_document(&mut doc)?;
    }

    // Save to file for inspection
    fs::write("output/debug_xref_stream.pdf", &buffer)?;

    // Print some debug info
    let content = String::from_utf8_lossy(&buffer);

    // Find the xref stream position
    if let Some(startxref_pos) = content.rfind("\nstartxref\n") {
        let after_startxref = &content[startxref_pos + 11..];
        if let Some(eof_pos) = after_startxref.find("\n%%EOF") {
            let xref_offset_str = &after_startxref[..eof_pos];
            if let Ok(xref_offset) = xref_offset_str.trim().parse::<usize>() {
                println!("XRef stream offset: {}", xref_offset);

                // Show the xref stream object
                if xref_offset < buffer.len() {
                    // Show context around xref position
                    let start = xref_offset.saturating_sub(50);
                    let end = (xref_offset + 200).min(buffer.len());
                    println!("\nContext around XRef position {}:", xref_offset);
                    println!("---");
                    println!("{}", String::from_utf8_lossy(&buffer[start..end]));
                    println!("---");

                    let xref_content = &content[xref_offset..];
                    if let Some(endobj_pos) = xref_content.find("\nendobj\n") {
                        let xref_obj = &xref_content[..endobj_pos + 8];
                        println!("\nXRef stream object:");
                        println!("{}", xref_obj);

                        // Show hex dump of stream data
                        if let Some(stream_start) = xref_obj.find("\nstream\n") {
                            if let Some(endstream_pos) = xref_obj.find("\nendstream") {
                                let stream_start_abs = xref_offset + stream_start + 8;
                                let endstream_abs = xref_offset + endstream_pos;

                                println!(
                                    "\nStream positions: start={}, end={}",
                                    stream_start_abs, endstream_abs
                                );

                                // Get the actual stream data
                                let stream_bytes = &buffer[stream_start_abs..endstream_abs];
                                println!("Stream data ({} bytes):", stream_bytes.len());

                                // Show hex dump
                                for (i, chunk) in stream_bytes.chunks(16).enumerate().take(4) {
                                    print!("{:04x}: ", i * 16);
                                    for byte in chunk {
                                        print!("{:02x} ", byte);
                                    }
                                    println!();
                                }
                                if stream_bytes.len() > 64 {
                                    println!("... ({} more bytes)", stream_bytes.len() - 64);
                                }

                                // Try to decompress using flate2 directly
                                println!("\nAttempting to decompress...");
                                use flate2::read::ZlibDecoder;
                                use std::io::Read;

                                let mut decoder = ZlibDecoder::new(&stream_bytes[..]);
                                let mut decompressed = Vec::new();
                                match decoder.read_to_end(&mut decompressed) {
                                    Ok(_) => {
                                        println!(
                                            "Decompression successful! {} bytes -> {} bytes",
                                            stream_bytes.len(),
                                            decompressed.len()
                                        );

                                        // Show decompressed data
                                        println!("\nDecompressed data:");
                                        for (i, chunk) in
                                            decompressed.chunks(16).enumerate().take(4)
                                        {
                                            print!("{:04x}: ", i * 16);
                                            for byte in chunk {
                                                print!("{:02x} ", byte);
                                            }
                                            println!();
                                        }
                                    }
                                    Err(e) => {
                                        println!("Decompression failed: {}", e);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(())
}
