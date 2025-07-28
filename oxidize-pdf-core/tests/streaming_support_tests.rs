//! Integration tests for streaming support features

use oxidize_pdf::streaming::PageStreamer;
use oxidize_pdf::{
    process_in_chunks, stream_text, ChunkOptions, ChunkProcessor, ChunkType, IncrementalParser,
    ParseEvent, StreamOptions, StreamingDocument, TextStreamOptions, TextStreamer,
};
use std::io::Cursor;

#[test]
fn test_streaming_document_basic() {
    let data = b"%PDF-1.7\n";
    let cursor = Cursor::new(data);
    let options = StreamOptions::default();

    let doc = StreamingDocument::new(cursor, options);
    assert!(doc.is_ok());
}

#[test]
fn test_streaming_document_pages() {
    let data = b"%PDF-1.7\n";
    let cursor = Cursor::new(data);
    let options = StreamOptions::default();

    let mut doc = StreamingDocument::new(cursor, options).unwrap();

    // Process up to 5 pages
    let mut page_count = 0;
    for _ in 0..5 {
        match doc.next_page() {
            Ok(Some(page)) => {
                assert_eq!(page.number(), page_count);
                page_count += 1;
            }
            Ok(None) => break,
            Err(_) => panic!("Error processing page"),
        }
    }

    assert!(page_count > 0);
}

#[test]
fn test_streaming_options_profiles() {
    // Test minimal memory profile
    let minimal = oxidize_pdf::streaming::StreamingOptions::minimal_memory();
    assert_eq!(minimal.buffer_size, 64 * 1024);
    assert_eq!(minimal.page_cache_size, 1);
    assert_eq!(minimal.memory_limit, 10 * 1024 * 1024);

    // Test fast processing profile
    let fast = oxidize_pdf::streaming::StreamingOptions::fast_processing();
    assert_eq!(fast.buffer_size, 1024 * 1024);
    assert_eq!(fast.page_cache_size, 10);
    assert_eq!(fast.memory_limit, 500 * 1024 * 1024);
}

#[test]
fn test_incremental_parser() {
    let mut parser = IncrementalParser::new();

    // Feed PDF header
    parser.feed(b"%PDF-1.7\n").unwrap();
    let events = parser.take_events();

    assert_eq!(events.len(), 1);
    match &events[0] {
        ParseEvent::Header { version } => assert_eq!(version, "1.7"),
        _ => panic!("Expected Header event"),
    }

    // Feed object
    parser
        .feed(b"1 0 obj\n<< /Type /Catalog >>\nendobj\n")
        .unwrap();
    let events = parser.take_events();

    assert!(events.len() >= 2);
    assert!(matches!(events[0], ParseEvent::ObjectStart { .. }));
}

#[test]
fn test_chunk_processor() {
    let options = ChunkOptions {
        max_chunk_size: 50,
        ..Default::default()
    };

    let mut processor = ChunkProcessor::new(options);

    // Process text content
    let content = b"BT /F1 12 Tf 100 700 Td (This is a long text that should be chunked) Tj ET";
    let chunks = processor.process_content(content).unwrap();

    assert!(chunks.len() > 1); // Should be split into multiple chunks
    assert!(chunks.iter().all(|c| c.size <= 50));
    assert!(chunks[0].chunk_type == ChunkType::Text);
}

#[test]
fn test_text_streamer() {
    let options = TextStreamOptions::default();
    let mut streamer = TextStreamer::new(options);

    // Process text content
    let content = b"BT /F1 14 Tf 100 700 Td (Hello) Tj ET \
                    BT /F1 12 Tf 100 650 Td (World) Tj ET";

    let chunks = streamer.process_chunk(content).unwrap();

    assert_eq!(chunks.len(), 2);
    assert_eq!(chunks[0].text, "Hello");
    assert_eq!(chunks[0].font_size, 14.0);
    assert_eq!(chunks[1].text, "World");
    assert_eq!(chunks[1].font_size, 12.0);
}

#[test]
fn test_text_extraction_sorted() {
    let options = TextStreamOptions {
        sort_by_position: true,
        ..Default::default()
    };

    let mut streamer = TextStreamer::new(options);

    // Add text in reverse order (bottom to top)
    streamer
        .process_chunk(b"BT /F1 12 Tf 100 100 Td (Bottom) Tj ET")
        .unwrap();
    streamer
        .process_chunk(b"BT /F1 12 Tf 100 400 Td (Middle) Tj ET")
        .unwrap();
    streamer
        .process_chunk(b"BT /F1 12 Tf 100 700 Td (Top) Tj ET")
        .unwrap();

    let text = streamer.extract_text();
    assert_eq!(text, "Top Middle Bottom");
}

#[test]
fn test_process_in_chunks_callback() {
    let data = b"BT /F1 12 Tf (Text content) Tj ET";
    let cursor = Cursor::new(data);

    let options = ChunkOptions {
        buffer_size: 10, // Small buffer to force multiple reads
        ..Default::default()
    };

    let mut chunk_count = 0;
    process_in_chunks(cursor, options, |chunk| {
        chunk_count += 1;
        assert!(!chunk.data.is_empty());
        Ok(())
    })
    .unwrap();

    assert!(chunk_count > 0);
}

#[test]
fn test_stream_text_callback() {
    let stream1 = b"BT /F1 12 Tf 100 700 Td (First) Tj ET".to_vec();
    let stream2 = b"BT /F1 14 Tf 100 650 Td (Second) Tj ET".to_vec();
    let streams = vec![stream1, stream2];

    let mut collected_text = Vec::new();
    stream_text(streams, |chunk| {
        collected_text.push(chunk.text);
        Ok(())
    })
    .unwrap();

    assert_eq!(collected_text.len(), 2);
    assert_eq!(collected_text[0], "First");
    assert_eq!(collected_text[1], "Second");
}

#[test]
fn test_memory_management() {
    let data = b"%PDF-1.7\n";
    let cursor = Cursor::new(data);

    let options = StreamOptions::default().with_memory_limit(1024); // Very low limit

    let mut doc = StreamingDocument::new(cursor, options).unwrap();

    // Process multiple pages
    for _ in 0..10 {
        let _ = doc.next_page();
    }

    // Memory usage should be controlled
    assert!(doc.memory_usage() <= 2048); // Allow some overhead
}

#[test]
fn test_page_cache_eviction() {
    let data = b"%PDF-1.7\n";
    let cursor = Cursor::new(data);

    let options = StreamOptions::default().with_page_cache_size(2); // Only cache 2 pages

    let mut doc = StreamingDocument::new(cursor, options).unwrap();

    // Process 5 pages
    let mut pages = Vec::new();
    for _ in 0..5 {
        if let Some(page) = doc.next_page().unwrap() {
            pages.push(page.number());
        }
    }

    // Clear cache and verify it works
    doc.clear_cache();
    assert_eq!(doc.memory_usage(), 0);
}

#[test]
fn test_chunk_type_filtering() {
    let options = ChunkOptions {
        chunk_types: vec![ChunkType::Image], // Only process images
        ..Default::default()
    };

    let mut processor = ChunkProcessor::new(options);

    // Text content should be filtered out
    let text_content = b"BT /F1 12 Tf (Text) Tj ET";
    let text_chunks = processor.process_content(text_content).unwrap();
    assert_eq!(text_chunks.len(), 0);

    // Image content should be processed
    let image_content = b"\xFF\xD8\xFF\xE0"; // JPEG header
    let image_chunks = processor.process_content(image_content).unwrap();
    assert!(!image_chunks.is_empty());
    assert_eq!(image_chunks[0].chunk_type, ChunkType::Image);
}

#[test]
fn test_streaming_page_methods() {
    let data = b"%PDF-1.7\n";
    let cursor = Cursor::new(data);
    let mut streamer = PageStreamer::new(cursor);

    // Get first page
    let page = streamer.next().unwrap().unwrap();

    assert_eq!(page.number(), 0);
    assert_eq!(page.width(), 595.0);
    assert_eq!(page.height(), 842.0);

    let media_box = page.media_box();
    assert_eq!(media_box, [0.0, 0.0, 595.0, 842.0]);

    let text = page.extract_text_streaming().unwrap();
    assert!(text.contains("page 1")); // 0-indexed, so page 0 is "page 1"
}

#[test]
fn test_text_chunk_with_font_filter() {
    let options = TextStreamOptions {
        min_font_size: 12.0,
        ..Default::default()
    };

    let mut streamer = TextStreamer::new(options);

    // Small font - should be filtered
    let small = b"BT /F1 8 Tf 100 700 Td (Small) Tj ET";
    let small_chunks = streamer.process_chunk(small).unwrap();
    assert_eq!(small_chunks.len(), 0);

    // Large font - should pass
    let large = b"BT /F1 16 Tf 100 650 Td (Large) Tj ET";
    let large_chunks = streamer.process_chunk(large).unwrap();
    assert_eq!(large_chunks.len(), 1);
    assert_eq!(large_chunks[0].text, "Large");
}
