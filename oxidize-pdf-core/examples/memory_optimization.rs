//! Example demonstrating memory optimization features
//!
//! This example shows how to use lazy loading and streaming
//! to work with large PDF files efficiently.

use oxidize_pdf::memory::{MemoryOptions, StreamingOptions};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Memory Optimization Example\n");

    // Example 1: Lazy Loading
    println!("1. Lazy Loading Demo");
    println!("-------------------");
    demo_lazy_loading()?;

    // Example 2: Streaming Processing
    println!("\n2. Streaming Processing Demo");
    println!("---------------------------");
    demo_streaming()?;

    // Example 3: Memory Options
    println!("\n3. Memory Options Demo");
    println!("---------------------");
    demo_memory_options()?;

    Ok(())
}

fn demo_lazy_loading() -> Result<(), Box<dyn std::error::Error>> {
    // Configure memory options for large files
    let _options = MemoryOptions::large_file()
        .with_cache_size(100) // Cache up to 100 objects
        .with_lazy_loading(true);

    println!("Memory options configured:");
    println!("  - Lazy loading: enabled");
    println!("  - Cache size: 100 objects");
    println!("  - Memory mapping: enabled");

    // In a real scenario, you would open an actual PDF file
    // For demo purposes, we'll show the API usage

    // let document = LazyDocument::open("large_document.pdf", options)?;
    // println!("\nDocument opened with {} pages", document.page_count());

    // Pages are loaded only when accessed
    // let page = document.get_page(0)?;
    // println!("First page loaded: {}x{} points", page.width(), page.height());

    // Memory stats
    // let stats = document.memory_stats();
    // println!("\nMemory statistics:");
    // println!("  - Cache hits: {}", stats.cache_hits);
    // println!("  - Cache misses: {}", stats.cache_misses);
    // println!("  - Cached objects: {}", stats.cached_objects);

    println!("\n[Demo mode - actual file processing would happen here]");

    Ok(())
}

fn demo_streaming() -> Result<(), Box<dyn std::error::Error>> {
    // Configure streaming options
    let _options = StreamingOptions {
        buffer_size: 128 * 1024,          // 128KB buffer
        max_stream_size: 5 * 1024 * 1024, // 5MB max stream
        skip_images: true,                // Skip image processing
        skip_fonts: false,
    };

    println!("Streaming options configured:");
    println!("  - Buffer size: 128KB");
    println!("  - Max stream size: 5MB");
    println!("  - Skip images: yes");
    println!("  - Skip fonts: no");

    // In a real scenario, you would process an actual PDF
    // let reader = std::fs::File::open("document.pdf")?;
    // let mut processor = StreamProcessor::new(reader, options);

    // Process pages incrementally
    // processor.process_pages(|page_num, page_data| {
    //     println!("Processing page {}", page_num + 1);
    //     println!("  Size: {}x{}", page_data.width, page_data.height);
    //     if let Some(text) = &page_data.text {
    //         println!("  Text preview: {}",
    //             text.chars().take(50).collect::<String>());
    //     }
    //     Ok(ProcessingAction::Continue)
    // })?;

    // Extract text to a file without loading entire PDF
    // let mut output = std::fs::File::create("extracted_text.txt")?;
    // processor.extract_text_streaming(&mut output)?;

    println!("\n[Demo mode - streaming would process pages incrementally]");

    Ok(())
}

fn demo_memory_options() -> Result<(), Box<dyn std::error::Error>> {
    println!("Different memory optimization strategies:\n");

    // Small file optimization
    let small_opts = MemoryOptions::small_file();
    println!("Small file optimization:");
    println!("  - Lazy loading: {}", small_opts.lazy_loading);
    println!("  - Memory mapping: {}", small_opts.memory_mapping);
    println!("  - Cache size: {}", small_opts.cache_size);

    // Large file optimization
    let large_opts = MemoryOptions::large_file();
    println!("\nLarge file optimization:");
    println!("  - Lazy loading: {}", large_opts.lazy_loading);
    println!("  - Memory mapping: {}", large_opts.memory_mapping);
    println!("  - Cache size: {}", large_opts.cache_size);
    println!("  - Buffer size: {}KB", large_opts.buffer_size / 1024);

    // Custom optimization
    let custom_opts = MemoryOptions::default()
        .with_lazy_loading(true)
        .with_memory_mapping(false)
        .with_cache_size(500)
        .with_streaming(true);

    println!("\nCustom optimization:");
    println!("  - Lazy loading: {}", custom_opts.lazy_loading);
    println!("  - Memory mapping: {}", custom_opts.memory_mapping);
    println!("  - Cache size: {}", custom_opts.cache_size);
    println!("  - Streaming: {}", custom_opts.streaming);

    println!(
        "\nMemory mapping threshold: {}MB",
        custom_opts.mmap_threshold / (1024 * 1024)
    );

    Ok(())
}

// Example of custom streaming callback
#[allow(dead_code)]
fn process_pdf_with_callback() -> Result<(), Box<dyn std::error::Error>> {
    // This would be used with a real PDF file
    // let reader = std::fs::File::open("document.pdf")?;
    // let mut processor = StreamProcessor::new(reader, StreamingOptions::default());

    // processor.process_with(|event| {
    //     match event {
    //         ProcessingEvent::Start => {
    //             println!("Starting PDF processing...");
    //         }
    //         ProcessingEvent::Header { version } => {
    //             println!("PDF version: {}", version);
    //         }
    //         ProcessingEvent::Page(page_data) => {
    //             println!("Found page {}: {}x{} points",
    //                 page_data.number + 1,
    //                 page_data.width,
    //                 page_data.height
    //             );
    //         }
    //         ProcessingEvent::End => {
    //             println!("Processing complete!");
    //         }
    //         _ => {}
    //     }
    //     Ok(ProcessingAction::Continue)
    // })?;

    Ok(())
}
