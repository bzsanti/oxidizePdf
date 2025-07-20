//! Example demonstrating streaming support for incremental PDF processing
//!
//! This example shows how to process large PDFs incrementally without
//! loading the entire document into memory.

use oxidize_pdf::streaming::{
    process_in_chunks, stream_text, ChunkOptions, ParseEvent, StreamingDocument, StreamingOptions,
    TextStreamOptions, TextStreamer,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Streaming Support Example\n");

    // Example 1: Streaming Document Processing
    println!("1. Streaming Document Processing");
    println!("-------------------------------");
    demo_streaming_document()?;

    // Example 2: Incremental Parsing
    println!("\n2. Incremental Parsing");
    println!("---------------------");
    demo_incremental_parsing()?;

    // Example 3: Chunk Processing
    println!("\n3. Chunk-based Processing");
    println!("------------------------");
    demo_chunk_processing()?;

    // Example 4: Text Streaming
    println!("\n4. Text Streaming");
    println!("----------------");
    demo_text_streaming()?;

    Ok(())
}

fn demo_streaming_document() -> Result<(), Box<dyn std::error::Error>> {
    // Configure streaming options
    let options = StreamingOptions::default()
        .with_buffer_size(512 * 1024) // 512KB buffer
        .with_page_cache_size(3) // Cache only 3 pages
        .with_memory_limit(50 * 1024 * 1024); // 50MB limit

    println!("Streaming options:");
    println!("  - Buffer size: 512KB");
    println!("  - Page cache: 3 pages");
    println!("  - Memory limit: 50MB");

    // In a real scenario, open an actual PDF file
    // let file = File::open("large_document.pdf")?;
    // let mut doc = StreamingDocument::new(file, options)?;

    // For demo, use mock data
    let data = std::io::Cursor::new(b"%PDF-1.7\n");
    let mut doc = StreamingDocument::new(data, options)?;

    // Process pages incrementally
    let mut page_count = 0;
    while let Some(page) = doc.next_page()? {
        println!("\nProcessing page {}", page.number() + 1);
        println!("  Size: {}x{} points", page.width(), page.height());

        // Extract text from page
        let text = page.extract_text_streaming()?;
        println!(
            "  Text preview: {}",
            text.chars().take(50).collect::<String>()
        );

        page_count += 1;

        // Check memory usage
        if page_count % 10 == 0 {
            println!("  Memory usage: {} bytes", doc.memory_usage());
        }
    }

    println!("\nTotal pages processed: {}", page_count);
    println!("[Demo mode - actual PDF processing would show real content]");

    Ok(())
}

fn demo_incremental_parsing() -> Result<(), Box<dyn std::error::Error>> {
    use oxidize_pdf::streaming::process_incrementally;

    println!("Parsing PDF incrementally...");

    // Simulate PDF content
    let pdf_content = b"%PDF-1.7\n\
        1 0 obj\n\
        << /Type /Catalog /Pages 2 0 R >>\n\
        endobj\n\
        2 0 obj\n\
        << /Type /Pages /Kids [] /Count 0 >>\n\
        endobj\n\
        xref\n\
        0 3\n\
        0000000000 65535 f\n\
        0000000010 00000 n\n\
        0000000053 00000 n\n\
        trailer\n\
        << /Size 3 /Root 1 0 R >>\n\
        startxref\n\
        116\n\
        %%EOF";

    let cursor = std::io::Cursor::new(pdf_content);

    // Process incrementally
    process_incrementally(cursor, |event| {
        match event {
            ParseEvent::Header { version } => {
                println!("  Found PDF header: version {}", version);
            }
            ParseEvent::ObjectStart { id, generation } => {
                println!("  Object start: {} {} obj", id, generation);
            }
            ParseEvent::ObjectEnd { id, .. } => {
                println!("  Object end: {} endobj", id);
            }
            ParseEvent::XRef { .. } => {
                println!("  Found cross-reference table");
            }
            ParseEvent::Trailer { .. } => {
                println!("  Found trailer dictionary");
            }
            ParseEvent::EndOfFile => {
                println!("  Reached end of file");
            }
            _ => {}
        }
        Ok(())
    })?;

    Ok(())
}

fn demo_chunk_processing() -> Result<(), Box<dyn std::error::Error>> {
    use oxidize_pdf::streaming::ChunkType;

    println!("Processing content in chunks...");

    // Configure chunk options
    let options = ChunkOptions {
        max_chunk_size: 1024, // 1KB chunks
        buffer_size: 4096,    // 4KB buffer
        chunk_types: vec![ChunkType::Text, ChunkType::Graphics],
        ..Default::default()
    };

    // Simulate mixed content
    let content = b"BT /F1 12 Tf 100 700 Td (Hello World) Tj ET \
                   100 200 m 300 400 l S \
                   BT /F2 10 Tf 100 650 Td (More text here) Tj ET";

    let cursor = std::io::Cursor::new(content);

    // Process chunks
    let mut text_chunks = 0;
    let mut graphics_chunks = 0;

    process_in_chunks(cursor, options, |chunk| {
        match chunk.chunk_type {
            ChunkType::Text => {
                text_chunks += 1;
                println!(
                    "  Text chunk: {} bytes at position {}",
                    chunk.size, chunk.position
                );
                if let Some(text) = chunk.as_text() {
                    println!("    Content: {}", text.chars().take(30).collect::<String>());
                }
            }
            ChunkType::Graphics => {
                graphics_chunks += 1;
                println!(
                    "  Graphics chunk: {} bytes at position {}",
                    chunk.size, chunk.position
                );
            }
            _ => {}
        }
        Ok(())
    })?;

    println!("\nChunk summary:");
    println!("  Text chunks: {}", text_chunks);
    println!("  Graphics chunks: {}", graphics_chunks);

    Ok(())
}

fn demo_text_streaming() -> Result<(), Box<dyn std::error::Error>> {
    println!("Streaming text extraction...");

    // Configure text streaming
    let options = TextStreamOptions {
        min_font_size: 8.0, // Skip text smaller than 8pt
        preserve_formatting: true,
        sort_by_position: true,
        ..Default::default()
    };

    let mut streamer = TextStreamer::new(options);

    // Simulate content streams from multiple pages
    let page1 = b"BT /F1 16 Tf 100 700 Td (Chapter 1: Introduction) Tj ET \
                  BT /F1 12 Tf 100 650 Td (This is the first paragraph.) Tj ET";

    let page2 = b"BT /F1 12 Tf 100 700 Td (Continuing from previous page...) Tj ET \
                  BT /F1 10 Tf 100 650 Td (More content here.) Tj ET";

    // Process first page
    println!("\nProcessing page 1:");
    let chunks1 = streamer.process_chunk(page1)?;
    for chunk in &chunks1 {
        println!(
            "  Found text: '{}' at ({}, {})",
            chunk.text, chunk.x, chunk.y
        );
    }

    // Process second page
    println!("\nProcessing page 2:");
    let chunks2 = streamer.process_chunk(page2)?;
    for chunk in &chunks2 {
        println!(
            "  Found text: '{}' at ({}, {})",
            chunk.text, chunk.x, chunk.y
        );
    }

    // Extract all text
    println!("\nExtracted text (sorted by position):");
    let full_text = streamer.extract_text();
    println!("  {}", full_text);

    // Alternative: Stream text with callback
    println!("\nStreaming with callback:");
    let streams = vec![page1.to_vec(), page2.to_vec()];
    stream_text(streams, |chunk| {
        println!("  Received: '{}' (size: {}pt)", chunk.text, chunk.font_size);
        Ok(())
    })?;

    Ok(())
}

// Helper function to display memory usage
#[allow(dead_code)]
fn format_bytes(bytes: usize) -> String {
    if bytes < 1024 {
        format!("{} B", bytes)
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    }
}
