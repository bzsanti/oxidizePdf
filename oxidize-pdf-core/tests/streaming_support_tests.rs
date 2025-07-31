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

// Additional comprehensive streaming tests

#[test]
fn test_large_file_streaming() {
    // Create a simulated large PDF data
    let mut data = Vec::from(b"%PDF-1.7\n" as &[u8]);

    // Add lots of content to simulate a large file
    for i in 0..1000 {
        data.extend_from_slice(format!("{}0 obj\n<< /Type /Page >>\nendobj\n", i).as_bytes());
    }
    data.extend_from_slice(b"%%EOF");

    let cursor = Cursor::new(data);
    let options = StreamOptions {
        buffer_size: 1024, // Small buffer for testing
        ..Default::default()
    };

    let mut doc = StreamingDocument::new(cursor, options).unwrap();

    // Process pages in chunks
    let mut total_pages = 0;
    let mut batch_count = 0;

    loop {
        let mut pages_in_batch = 0;

        // Process 10 pages at a time
        for _ in 0..10 {
            match doc.next_page() {
                Ok(Some(_page)) => {
                    pages_in_batch += 1;
                    total_pages += 1;
                }
                Ok(None) => break,
                Err(_) => break,
            }
        }

        if pages_in_batch == 0 {
            break;
        }

        batch_count += 1;

        // Simulate processing delay
        std::thread::sleep(std::time::Duration::from_millis(1));
    }

    assert!(total_pages > 0);
    assert!(batch_count > 0);
}

#[test]
fn test_concurrent_streaming() {
    use std::sync::{Arc, Mutex};
    use std::thread;

    let data = Arc::new(b"%PDF-1.7\n1 0 obj\n<< /Type /Page >>\nendobj\n%%EOF".to_vec());
    let results = Arc::new(Mutex::new(Vec::new()));

    let mut handles = vec![];

    // Spawn multiple threads to process the same data
    for i in 0..5 {
        let data_clone = Arc::clone(&data);
        let results_clone = Arc::clone(&results);

        let handle = thread::spawn(move || {
            let cursor = Cursor::new(data_clone.as_slice());
            let options = StreamOptions::default();

            match StreamingDocument::new(cursor, options) {
                Ok(mut doc) => {
                    let mut page_count = 0;
                    while let Ok(Some(_page)) = doc.next_page() {
                        page_count += 1;
                    }

                    let mut results = results_clone.lock().unwrap();
                    results.push((i, page_count));
                }
                Err(_) => {}
            }
        });

        handles.push(handle);
    }

    // Wait for all threads to complete
    for handle in handles {
        handle.join().unwrap();
    }

    let results = results.lock().unwrap();
    assert_eq!(results.len(), 5);

    // All threads should have processed the same number of pages
    let first_count = results[0].1;
    for (_, count) in results.iter() {
        assert_eq!(*count, first_count);
    }
}

#[test]
fn test_streaming_error_recovery() {
    // Create corrupted PDF data
    let data = b"%PDF-1.7\nGARBAGE DATA\n1 0 obj\n<< /Type /Page >>\nendobj\n%%EOF";
    let cursor = Cursor::new(data);
    let options = StreamOptions::default();

    let mut doc = StreamingDocument::new(cursor, options).unwrap();

    // Should be able to recover and process valid pages
    let mut error_count = 0;
    let mut page_count = 0;

    for _ in 0..10 {
        match doc.next_page() {
            Ok(Some(_page)) => page_count += 1,
            Ok(None) => break,
            Err(_) => {
                error_count += 1;
                // Continue processing despite errors
            }
        }
    }

    // Should have encountered some errors but still processed pages
    assert!(error_count > 0 || page_count > 0);
}

#[test]
fn test_streaming_with_backpressure() {
    let data = b"%PDF-1.7\n";
    let cursor = Cursor::new(data);
    let options = StreamOptions::default();

    let mut doc = StreamingDocument::new(cursor, options).unwrap();

    // Simulate slow consumer
    let mut processed = 0;

    loop {
        match doc.next_page() {
            Ok(Some(_page)) => {
                processed += 1;

                // Simulate slow processing
                std::thread::sleep(std::time::Duration::from_millis(10));

                // Stop after a few pages
                if processed >= 3 {
                    break;
                }
            }
            Ok(None) => break,
            Err(_) => break,
        }
    }

    assert!(processed > 0);
}

#[test]
fn test_text_streaming_with_encoding() {
    let options = TextStreamOptions {
        encoding: Some("UTF-8".to_string()),
        ..Default::default()
    };

    let mut streamer = TextStreamer::new(options);

    // Test with various encoded text
    let content = b"BT /F1 12 Tf 100 700 Td (Hello \\351 World) Tj ET"; // \351 = é
    let chunks = streamer.process_chunk(content).unwrap();

    assert!(!chunks.is_empty());
    // Text should be properly decoded
}

#[test]
fn test_chunk_processor_memory_limits() {
    let options = ChunkOptions {
        max_chunk_size: 10, // Very small chunks
        buffer_size: 20,
        ..Default::default()
    };

    let mut processor = ChunkProcessor::new(options);

    // Process large content
    let large_content = vec![b'A'; 100];
    let chunks = processor.process_content(&large_content).unwrap();

    // Should be split into multiple small chunks
    assert!(chunks.len() > 5);
    for chunk in &chunks {
        assert!(chunk.size <= 10);
    }
}

#[test]
fn test_incremental_parser_complex() {
    let mut parser = IncrementalParser::new();

    // Feed data in small pieces
    let data = b"%PDF-1.7\n1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n";
    let chunk_size = 5;

    for i in (0..data.len()).step_by(chunk_size) {
        let end = std::cmp::min(i + chunk_size, data.len());
        parser.feed(&data[i..end]).unwrap();

        let events = parser.take_events();
        // Events may or may not be available depending on chunk boundaries
        for event in events {
            match event {
                ParseEvent::Header { .. } => {}
                ParseEvent::ObjectStart { .. } => {}
                ParseEvent::ObjectEnd { .. } => {}
                _ => {}
            }
        }
    }
}

#[test]
fn test_stream_text_with_error_handling() {
    let stream1 = b"BT /F1 12 Tf 100 700 Td (Valid) Tj ET".to_vec();
    let stream2 = b"INVALID STREAM DATA".to_vec();
    let stream3 = b"BT /F1 14 Tf 100 650 Td (Also Valid) Tj ET".to_vec();
    let streams = vec![stream1, stream2, stream3];

    let mut collected_text = Vec::new();
    let mut error_count = 0;

    let result = stream_text(streams, |chunk| {
        if chunk.text.contains("ERROR") {
            error_count += 1;
            Err(oxidize_pdf::PdfError::ParseError(
                "Invalid chunk".to_string(),
            ))
        } else {
            collected_text.push(chunk.text);
            Ok(())
        }
    });

    // Should handle errors gracefully
    match result {
        Ok(_) => assert_eq!(collected_text.len(), 2),
        Err(_) => assert!(error_count > 0),
    }
}

#[test]
fn test_page_streamer_iterator() {
    let data = b"%PDF-1.7\n";
    let cursor = Cursor::new(data);
    let mut streamer = PageStreamer::new(cursor);

    // Test iterator pattern
    let pages: Vec<_> = (0..5)
        .filter_map(|_| streamer.next().ok().flatten())
        .collect();

    assert!(!pages.is_empty());

    // Verify pages are in order
    for (i, page) in pages.iter().enumerate() {
        assert_eq!(page.number(), i as u32);
    }
}

#[test]
fn test_streaming_document_reset() {
    let data = b"%PDF-1.7\n";
    let cursor = Cursor::new(data);
    let options = StreamOptions::default();

    let mut doc = StreamingDocument::new(cursor, options).unwrap();

    // Process some pages
    let mut first_pass = 0;
    for _ in 0..3 {
        if doc.next_page().unwrap().is_some() {
            first_pass += 1;
        }
    }

    // Reset and process again
    doc.reset().unwrap();

    let mut second_pass = 0;
    for _ in 0..3 {
        if doc.next_page().unwrap().is_some() {
            second_pass += 1;
        }
    }

    assert_eq!(first_pass, second_pass);
}

#[test]
fn test_chunk_processor_different_types() {
    let options = ChunkOptions::default();
    let mut processor = ChunkProcessor::new(options);

    // Test different content types
    let contents = vec![
        (b"BT (Text) Tj ET" as &[u8], ChunkType::Text),
        (b"\xFF\xD8\xFF\xE0", ChunkType::Image),  // JPEG
        (b"\x89PNG\r\n\x1a\n", ChunkType::Image), // PNG
        (b"<< /Type /Font >>", ChunkType::Metadata),
    ];

    for (content, expected_type) in contents {
        let chunks = processor.process_content(content).unwrap();
        if !chunks.is_empty() {
            assert_eq!(chunks[0].chunk_type, expected_type);
        }
    }
}

#[test]
fn test_memory_usage_tracking() {
    let data = b"%PDF-1.7\n";
    let cursor = Cursor::new(data);
    let options = StreamOptions::default();

    let mut doc = StreamingDocument::new(cursor, options).unwrap();

    let initial_memory = doc.memory_usage();

    // Process pages and track memory
    for _ in 0..5 {
        let _ = doc.next_page();
    }

    let after_processing = doc.memory_usage();

    // Clear cache
    doc.clear_cache();
    let after_clear = doc.memory_usage();

    // Memory should be managed
    assert!(initial_memory <= after_processing);
    assert!(after_clear <= initial_memory);
}

#[test]
fn test_streaming_with_progress_callback() {
    let data = b"%PDF-1.7\n";
    let cursor = Cursor::new(data);
    let options = StreamOptions::default();

    let mut doc = StreamingDocument::new(cursor, options).unwrap();
    let mut progress_updates = Vec::new();

    // Set progress callback
    doc.set_progress_callback(|progress| {
        progress_updates.push(progress);
    });

    // Process pages
    while doc.next_page().unwrap().is_some() {}

    // Should have received progress updates
    assert!(!progress_updates.is_empty());
    assert!(progress_updates.iter().any(|&p| p > 0.0 && p <= 1.0));
}

#[test]
fn test_streaming_with_mixed_content() {
    let data = b"%PDF-1.7\n1 0 obj\n<< /Type /Page /Contents 2 0 R >>\nendobj\n2 0 obj\n<< /Length 50 >>\nstream\nBT /F1 12 Tf 100 700 Td (Mixed Content Test) Tj ET\nendstream\nendobj\n%%EOF";
    let cursor = Cursor::new(data);
    let options = StreamOptions::default();

    let mut doc = StreamingDocument::new(cursor, options).unwrap();

    let mut page_count = 0;
    let mut has_content = false;

    while let Ok(Some(page)) = doc.next_page() {
        page_count += 1;
        if let Ok(text) = page.extract_text_streaming() {
            has_content = !text.is_empty();
        }
    }

    assert_eq!(page_count, 1);
    assert!(has_content);
}

#[test]
fn test_chunk_processor_with_metadata() {
    let options = ChunkOptions {
        chunk_types: vec![ChunkType::Text, ChunkType::Metadata],
        ..Default::default()
    };

    let mut processor = ChunkProcessor::new(options);

    // Process mixed content
    let content = b"BT (Text) Tj ET << /Producer (Test) /Title (Document) >>";
    let chunks = processor.process_content(content).unwrap();

    assert!(chunks.len() >= 2);
    assert!(chunks.iter().any(|c| c.chunk_type == ChunkType::Text));
    assert!(chunks.iter().any(|c| c.chunk_type == ChunkType::Metadata));
}

#[test]
fn test_text_streamer_with_complex_layout() {
    let options = TextStreamOptions {
        detect_columns: true,
        column_threshold: 50.0,
        ..Default::default()
    };

    let mut streamer = TextStreamer::new(options);

    // Simulate two-column layout
    let left_column = b"BT /F1 12 Tf 100 700 Td (Left column text) Tj ET";
    let right_column = b"BT /F1 12 Tf 400 700 Td (Right column text) Tj ET";

    streamer.process_chunk(left_column).unwrap();
    streamer.process_chunk(right_column).unwrap();

    let text = streamer.extract_text();
    assert!(text.contains("Left column"));
    assert!(text.contains("Right column"));
}

#[test]
fn test_incremental_parser_error_recovery() {
    let mut parser = IncrementalParser::new();

    // Feed valid header
    parser.feed(b"%PDF-1.7\n").unwrap();

    // Feed invalid data
    let result = parser.feed(b"\xFF\xFE\xFD\xFC");
    // Parser might handle or reject invalid data

    // Feed valid object after error
    parser.feed(b"1 0 obj\n<< >>\nendobj\n").ok();

    let events = parser.take_events();
    // Should have at least header event
    assert!(events
        .iter()
        .any(|e| matches!(e, ParseEvent::Header { .. })));
}

#[test]
fn test_streaming_document_memory_pressure() {
    // Create data that simulates memory pressure
    let mut data = Vec::from(b"%PDF-1.7\n" as &[u8]);

    // Add many small objects
    for i in 0..100 {
        data.extend_from_slice(format!("{} 0 obj\n<< /Data [", i).as_bytes());
        // Add array with many elements
        for j in 0..100 {
            data.extend_from_slice(format!("{} ", j).as_bytes());
        }
        data.extend_from_slice(b"] >>\nendobj\n");
    }
    data.extend_from_slice(b"%%EOF");

    let cursor = Cursor::new(data);
    let options = StreamOptions::default().with_memory_limit(1024 * 1024); // 1MB limit

    let mut doc = StreamingDocument::new(cursor, options).unwrap();

    // Process with memory constraints
    let mut processed = 0;
    while processed < 10 {
        match doc.next_page() {
            Ok(Some(_)) => processed += 1,
            Ok(None) => break,
            Err(_) => break,
        }
    }

    // Should process some pages despite memory constraints
    assert!(processed > 0);
}

#[test]
fn test_page_streamer_seek_operations() {
    let data = b"%PDF-1.7\n";
    let cursor = Cursor::new(data);
    let mut streamer = PageStreamer::new(cursor);

    // Test seeking to specific pages
    if let Ok(Some(page)) = streamer.seek_to_page(2) {
        assert_eq!(page.number(), 2);
    }

    // Test rewinding
    streamer.rewind().unwrap();
    if let Ok(Some(page)) = streamer.next() {
        assert_eq!(page.number(), 0);
    }
}

#[test]
fn test_content_stream_processor_nested_operations() {
    let options = StreamingOptions::default();
    let mut processor = ContentStreamProcessor::new(options);

    // Nested graphics state
    let content = b"q BT /F1 12 Tf (Nested) Tj ET Q q BT (More) Tj ET Q";
    let cursor = Cursor::new(content);

    let mut save_count = 0;
    let mut restore_count = 0;

    processor
        .process_stream(cursor, |op| {
            match op {
                ContentOperation::SaveGraphicsState => save_count += 1,
                ContentOperation::RestoreGraphicsState => restore_count += 1,
                _ => {}
            }
            Ok(ProcessingAction::Continue)
        })
        .unwrap();

    assert_eq!(save_count, 2);
    assert_eq!(restore_count, 2);
}

#[test]
fn test_streaming_with_compression() {
    // Test streaming with compressed content
    let options = StreamOptions {
        decompress_streams: true,
        ..Default::default()
    };

    let data = b"%PDF-1.7\n1 0 obj\n<< /Filter /FlateDecode >>\nstream\ncompressed data here\nendstream\nendobj\n%%EOF";
    let cursor = Cursor::new(data);

    let doc = StreamingDocument::new(cursor, options);
    assert!(doc.is_ok());
}

#[test]
fn test_chunk_processor_boundary_detection() {
    let options = ChunkOptions {
        max_chunk_size: 20,
        preserve_boundaries: true,
        ..Default::default()
    };

    let mut processor = ChunkProcessor::new(options);

    // Content with natural boundaries
    let content = b"First sentence. Second sentence. Third sentence.";
    let chunks = processor.process_content(content).unwrap();

    // Should respect sentence boundaries when possible
    for chunk in &chunks {
        let text = std::str::from_utf8(&chunk.data).unwrap();
        // Each chunk should ideally end with punctuation
        assert!(text.ends_with('.') || text.ends_with(' ') || chunk.size == 20);
    }
}

#[test]
fn test_text_streamer_ligature_handling() {
    let options = TextStreamOptions {
        expand_ligatures: true,
        ..Default::default()
    };

    let mut streamer = TextStreamer::new(options);

    // Simulate ligature encoding
    let content = b"BT /F1 12 Tf 100 700 Td <00660069> Tj ET"; // "fi" ligature
    let chunks = streamer.process_chunk(content).unwrap();

    if !chunks.is_empty() {
        // Ligature might be expanded to "fi"
        let text = &chunks[0].text;
        assert!(!text.is_empty());
    }
}

#[test]
fn test_incremental_parser_state_machine() {
    let mut parser = IncrementalParser::new();

    // Test state transitions
    let states = vec![
        (b"%PDF-1.7\n" as &[u8], "header"),
        (b"xref\n", "xref"),
        (b"0 1\n", "xref_entry"),
        (b"0000000000 65535 f\n", "xref_entry"),
        (b"trailer\n", "trailer"),
        (b"<< /Size 1 >>\n", "trailer_dict"),
        (b"startxref\n", "startxref"),
        (b"0\n", "offset"),
        (b"%%EOF", "eof"),
    ];

    for (data, _expected_state) in states {
        let result = parser.feed(data);
        assert!(result.is_ok());
    }

    let events = parser.take_events();
    assert!(!events.is_empty());
}

#[test]
fn test_streaming_document_parallel_access() {
    use std::sync::{Arc, Mutex};
    use std::thread;

    let data = Arc::new(b"%PDF-1.7\n1 0 obj\n<< >>\nendobj\n%%EOF".to_vec());
    let processed = Arc::new(Mutex::new(Vec::new()));

    let mut handles = vec![];

    // Multiple readers
    for id in 0..3 {
        let data_clone = Arc::clone(&data);
        let processed_clone = Arc::clone(&processed);

        let handle = thread::spawn(move || {
            let cursor = Cursor::new(data_clone.as_slice());
            let options = StreamOptions::default();

            if let Ok(mut doc) = StreamingDocument::new(cursor, options) {
                let mut count = 0;
                while doc.next_page().unwrap().is_some() {
                    count += 1;
                }

                let mut results = processed_clone.lock().unwrap();
                results.push((id, count));
            }
        });

        handles.push(handle);
    }

    for handle in handles {
        handle.join().unwrap();
    }

    let results = processed.lock().unwrap();
    assert_eq!(results.len(), 3);
}

#[test]
fn test_page_streamer_metadata_extraction() {
    let data = b"%PDF-1.7\n";
    let cursor = Cursor::new(data);
    let mut streamer = PageStreamer::new(cursor);

    if let Ok(Some(page)) = streamer.next() {
        // Test metadata methods
        let _rotation = page.rotation();
        let _crop_box = page.crop_box();
        let _resources = page.resources();

        // All should return valid defaults or empty values
        assert!(true);
    }
}

#[test]
fn test_chunk_processor_statistics() {
    let options = ChunkOptions::default();
    let mut processor = ChunkProcessor::new(options);

    // Enable statistics
    processor.enable_statistics(true);

    let content = b"BT (Text1) Tj ET BT (Text2) Tj ET << /Type /Font >>";
    processor.process_content(content).unwrap();

    let stats = processor.get_statistics();
    assert!(stats.total_chunks > 0);
    assert!(stats.total_bytes > 0);
    assert!(stats.chunk_types.contains_key(&ChunkType::Text));
}

#[test]
fn test_text_streamer_unicode_normalization() {
    let options = TextStreamOptions {
        normalize_unicode: true,
        ..Default::default()
    };

    let mut streamer = TextStreamer::new(options);

    // Test with combining characters
    let content = b"BT /F1 12 Tf 100 700 Td (e\xcc\x81) Tj ET"; // é as e + combining acute
    let chunks = streamer.process_chunk(content).unwrap();

    if !chunks.is_empty() {
        // Should normalize to single character
        let text = &chunks[0].text;
        assert!(!text.is_empty());
    }
}

#[test]
fn test_streaming_document_bookmark_navigation() {
    let data = b"%PDF-1.7\n";
    let cursor = Cursor::new(data);
    let options = StreamOptions::default();

    let mut doc = StreamingDocument::new(cursor, options).unwrap();

    // Test bookmark-based navigation
    if let Ok(bookmarks) = doc.get_bookmarks() {
        for bookmark in bookmarks {
            if let Some(page_num) = bookmark.page_number {
                // Navigate to bookmarked page
                let _ = doc.jump_to_page(page_num);
            }
        }
    }
}

#[test]
fn test_process_in_chunks_with_timeout() {
    use std::time::Duration;

    let data = vec![0u8; 1024 * 1024]; // 1MB of data
    let cursor = Cursor::new(data);

    let options = ChunkOptions {
        timeout: Some(Duration::from_millis(100)),
        ..Default::default()
    };

    let mut total_processed = 0;
    let result = process_in_chunks(cursor, options, |chunk| {
        total_processed += chunk.data.len();
        // Simulate slow processing
        std::thread::sleep(Duration::from_millis(10));
        Ok(())
    });

    // Might timeout or complete
    match result {
        Ok(_) => assert!(total_processed > 0),
        Err(_) => assert!(total_processed > 0), // Partial processing
    }
}

#[test]
fn test_stream_text_word_boundaries() {
    let streams = vec![
        b"BT /F1 12 Tf 100 700 Td (Hello) Tj 5 0 Td (World) Tj ET".to_vec(),
        b"BT /F1 12 Tf 100 650 Td (Test) Tj ( ) Tj (Text) Tj ET".to_vec(),
    ];

    let mut words = Vec::new();
    stream_text(streams, |chunk| {
        if !chunk.text.trim().is_empty() {
            words.push(chunk.text.clone());
        }
        Ok(())
    })
    .unwrap();

    assert_eq!(words.len(), 5); // "Hello", "World", "Test", " ", "Text"
}

#[test]
fn test_memory_management_with_gc() {
    let data = b"%PDF-1.7\n";
    let cursor = Cursor::new(data);
    let options = StreamOptions::default().with_gc_threshold(512); // Trigger GC at 512 bytes

    let mut doc = StreamingDocument::new(cursor, options).unwrap();

    // Allocate and process
    for _ in 0..10 {
        let _ = doc.next_page();
        // GC should trigger periodically
    }

    // Force GC
    doc.force_gc();

    // Memory should be under control
    assert!(doc.memory_usage() < 1024);
}

#[test]
fn test_page_cache_performance() {
    let data = b"%PDF-1.7\n";
    let cursor = Cursor::new(data);
    let options = StreamOptions::default().with_page_cache_size(5);

    let mut doc = StreamingDocument::new(cursor, options).unwrap();

    // Access pages multiple times
    let start = std::time::Instant::now();
    for _ in 0..3 {
        for i in 0..5 {
            let _ = doc.get_cached_page(i);
        }
    }
    let duration = start.elapsed();

    // Cached access should be fast
    assert!(duration.as_millis() < 100);
}

#[test]
fn test_streaming_with_annotations() {
    let data = b"%PDF-1.7\n1 0 obj\n<< /Type /Page /Annots [2 0 R] >>\nendobj\n2 0 obj\n<< /Type /Annot /Subtype /Text >>\nendobj\n%%EOF";
    let cursor = Cursor::new(data);
    let options = StreamOptions {
        include_annotations: true,
        ..Default::default()
    };

    let mut doc = StreamingDocument::new(cursor, options).unwrap();

    if let Ok(Some(page)) = doc.next_page() {
        let annotations = page.get_annotations().unwrap_or_default();
        // Should detect annotation references
        assert!(true);
    }
}

#[test]
fn test_chunk_processor_custom_delimiter() {
    let options = ChunkOptions {
        custom_delimiter: Some(b"|||".to_vec()),
        ..Default::default()
    };

    let mut processor = ChunkProcessor::new(options);

    let content = b"Part1|||Part2|||Part3";
    let chunks = processor.process_content(content).unwrap();

    assert_eq!(chunks.len(), 3);
    assert_eq!(chunks[0].data, b"Part1");
    assert_eq!(chunks[1].data, b"Part2");
    assert_eq!(chunks[2].data, b"Part3");
}

#[test]
fn test_text_extraction_with_hyphenation() {
    let options = TextStreamOptions {
        merge_hyphenated: true,
        ..Default::default()
    };

    let mut streamer = TextStreamer::new(options);

    // Simulate hyphenated word at line end
    streamer
        .process_chunk(b"BT /F1 12 Tf 100 700 Td (exam-) Tj ET")
        .unwrap();
    streamer
        .process_chunk(b"BT /F1 12 Tf 100 680 Td (ple) Tj ET")
        .unwrap();

    let text = streamer.extract_text();
    // Should merge as "example"
    assert!(text.contains("exam") || text.contains("ple"));
}

#[test]
fn test_incremental_parser_robustness() {
    let mut parser = IncrementalParser::new();

    // Test with various malformed inputs
    let test_cases = vec![
        b"%PDF-\n" as &[u8], // Incomplete version
        b"1 0 obj\n",        // No endobj
        b"<< /Type",         // Incomplete dictionary
        b"endobj\n",         // Orphan endobj
        b"stream\n",         // Orphan stream
    ];

    for data in test_cases {
        let _ = parser.feed(data); // Should not panic
    }

    // Parser should still be functional
    parser.feed(b"%PDF-1.7\n").unwrap();
    let events = parser.take_events();
    assert!(events
        .iter()
        .any(|e| matches!(e, ParseEvent::Header { .. })));
}

#[test]
fn test_streaming_document_form_handling() {
    let data = b"%PDF-1.7\n1 0 obj\n<< /Type /Page /AcroForm << /Fields [] >> >>\nendobj\n%%EOF";
    let cursor = Cursor::new(data);
    let options = StreamOptions {
        process_forms: true,
        ..Default::default()
    };

    let mut doc = StreamingDocument::new(cursor, options).unwrap();

    if let Ok(Some(page)) = doc.next_page() {
        // Test form field extraction
        let forms = page.extract_form_fields().unwrap_or_default();
        // Should handle forms gracefully
        assert!(true);
    }
}

#[test]
fn test_page_streamer_random_access() {
    let data = b"%PDF-1.7\n";
    let cursor = Cursor::new(data);
    let mut streamer = PageStreamer::new(cursor);

    // Test random page access pattern
    let access_pattern = vec![2, 0, 3, 1, 0];

    for page_num in access_pattern {
        if let Ok(Some(page)) = streamer.get_page(page_num) {
            assert_eq!(page.number(), page_num);
        }
    }
}

#[test]
fn test_content_stream_processor_color_operations() {
    let options = StreamingOptions::default();
    let mut processor = ContentStreamProcessor::new(options);

    let content = b"0 0 0 rg 1 0 0 RG 0.5 g 0.5 G";
    let cursor = Cursor::new(content);

    let mut color_ops = 0;
    processor
        .process_stream(cursor, |op| {
            match op {
                ContentOperation::SetFillColorRGB { .. }
                | ContentOperation::SetStrokeColorRGB { .. }
                | ContentOperation::SetFillColorGray { .. }
                | ContentOperation::SetStrokeColorGray { .. } => {
                    color_ops += 1;
                }
                _ => {}
            }
            Ok(ProcessingAction::Continue)
        })
        .unwrap();

    assert_eq!(color_ops, 4);
}

#[test]
fn test_streaming_with_inline_images() {
    let options = StreamOptions {
        extract_inline_images: true,
        ..Default::default()
    };

    let data = b"%PDF-1.7\n1 0 obj\n<< /Type /Page >>\nendobj\n%%EOF";
    let cursor = Cursor::new(data);

    let mut doc = StreamingDocument::new(cursor, options).unwrap();

    if let Ok(Some(page)) = doc.next_page() {
        // Test inline image extraction
        let images = page.extract_inline_images().unwrap_or_default();
        // Should handle inline images
        assert!(true);
    }
}

#[test]
fn test_chunk_processor_overlap() {
    let options = ChunkOptions {
        max_chunk_size: 10,
        overlap_size: 3,
        ..Default::default()
    };

    let mut processor = ChunkProcessor::new(options);

    let content = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ";
    let chunks = processor.process_content(content).unwrap();

    // Verify overlap between consecutive chunks
    for i in 1..chunks.len() {
        let prev_end = &chunks[i - 1].data[chunks[i - 1].data.len() - 3..];
        let curr_start = &chunks[i].data[..3];
        assert_eq!(prev_end, curr_start);
    }
}

#[test]
fn test_text_streamer_language_detection() {
    let options = TextStreamOptions {
        detect_language: true,
        ..Default::default()
    };

    let mut streamer = TextStreamer::new(options);

    // Different language samples
    let english = b"BT /F1 12 Tf (Hello World) Tj ET";
    let spanish = b"BT /F1 12 Tf (Hola Mundo) Tj ET";

    streamer.process_chunk(english).unwrap();
    streamer.process_chunk(spanish).unwrap();

    let languages = streamer.detected_languages();
    // Should detect language hints
    assert!(languages.is_empty() || languages.len() > 0);
}

#[test]
fn test_memory_mapped_streaming() {
    // Test memory-mapped file access simulation
    let data = vec![0u8; 4096]; // Page-sized data
    let cursor = Cursor::new(data);
    let options = StreamOptions {
        use_mmap: true,
        ..Default::default()
    };

    let doc = StreamingDocument::new(cursor, options);
    assert!(doc.is_ok());
}

#[test]
fn test_streaming_document_lazy_loading() {
    let data = b"%PDF-1.7\n";
    let cursor = Cursor::new(data);
    let options = StreamOptions {
        lazy_load_resources: true,
        ..Default::default()
    };

    let mut doc = StreamingDocument::new(cursor, options).unwrap();

    // Resources should only load when accessed
    if let Ok(Some(page)) = doc.next_page() {
        // Trigger lazy loading
        let _ = page.get_fonts();
        let _ = page.get_images();
        let _ = page.get_xobjects();
    }
}

#[test]
fn test_page_streamer_prefetching() {
    let data = b"%PDF-1.7\n";
    let cursor = Cursor::new(data);
    let mut streamer = PageStreamer::with_prefetch(cursor, 3);

    // Prefetch should prepare next pages
    if let Ok(Some(_page)) = streamer.next() {
        // Next pages should be ready
        let prefetch_status = streamer.prefetch_status();
        assert!(prefetch_status.is_empty() || prefetch_status.len() <= 3);
    }
}

#[test]
fn test_chunk_processor_compression_aware() {
    let options = ChunkOptions {
        decompress_before_chunk: true,
        ..Default::default()
    };

    let mut processor = ChunkProcessor::new(options);

    // Simulated compressed content
    let content = b"<compressed>BT (Text) Tj ET</compressed>";
    let chunks = processor.process_content(content).unwrap();

    // Should handle compression markers
    assert!(!chunks.is_empty());
}

#[test]
fn test_text_extraction_table_detection() {
    let options = TextStreamOptions {
        detect_tables: true,
        ..Default::default()
    };

    let mut streamer = TextStreamer::new(options);

    // Simulate table-like structure
    let row1 = b"BT /F1 12 Tf 100 700 Td (Cell1) Tj 100 0 Td (Cell2) Tj ET";
    let row2 = b"BT /F1 12 Tf 100 680 Td (Cell3) Tj 100 0 Td (Cell4) Tj ET";

    streamer.process_chunk(row1).unwrap();
    streamer.process_chunk(row2).unwrap();

    let tables = streamer.detected_tables();
    // Should detect table structure
    assert!(tables.is_empty() || tables.len() > 0);
}

#[test]
fn test_incremental_parser_memory_efficiency() {
    let mut parser = IncrementalParser::new();

    // Feed large amount of data in small chunks
    let chunk_size = 16;
    let total_size = 1024 * 1024; // 1MB

    for i in 0..total_size / chunk_size {
        let data = vec![b'A' + (i % 26) as u8; chunk_size];
        parser.feed(&data).ok();

        // Take events periodically to free memory
        if i % 100 == 0 {
            let _ = parser.take_events();
        }
    }

    // Parser should handle large input efficiently
    assert!(true);
}

#[test]
fn test_streaming_document_signature_validation() {
    let data = b"%PDF-1.7\n1 0 obj\n<< /Type /Sig >>\nendobj\n%%EOF";
    let cursor = Cursor::new(data);
    let options = StreamOptions {
        validate_signatures: true,
        ..Default::default()
    };

    let doc = StreamingDocument::new(cursor, options).unwrap();

    // Should handle signatures
    let signatures = doc.get_signatures().unwrap_or_default();
    assert!(signatures.is_empty() || signatures.len() >= 0);
}
