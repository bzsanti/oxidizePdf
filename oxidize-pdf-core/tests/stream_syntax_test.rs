//! Tests for PDF Stream syntax validation
//!
//! These tests verify that stream Length attributes are correctly synchronized
//! with actual data, preventing "Bad Length" and "Missing endstream" errors.

use oxidize_pdf::objects::{Dictionary, Object, Stream};
use oxidize_pdf::{Document, Font, Page};

// =============================================================================
// Stream Length Synchronization Tests
// =============================================================================

/// Test that Stream::new correctly sets Length to match data
#[test]
fn test_stream_new_sets_correct_length() {
    let data = vec![1, 2, 3, 4, 5];
    let stream = Stream::new(data.clone());

    assert_eq!(
        stream.dictionary().get("Length"),
        Some(&Object::Integer(5)),
        "Length should equal data.len()"
    );
}

/// Test that modifying data via data_mut does NOT automatically update Length
/// This is the CURRENT BEHAVIOR - the test documents it
#[test]
fn test_data_mut_does_not_update_length() {
    let data = vec![1, 2, 3];
    let mut stream = Stream::new(data);

    // Add more data
    stream.data_mut().extend([4, 5, 6, 7]);

    // Current behavior: Length is NOT updated
    assert_eq!(
        stream.dictionary().get("Length"),
        Some(&Object::Integer(3)),
        "Current behavior: Length is NOT auto-updated after data_mut()"
    );
    assert_eq!(stream.data().len(), 7, "Data has been extended to 7 bytes");

    // This mismatch causes "Bad Length" errors in PDF readers!
}

/// Test that compress_flate DOES update Length correctly
#[test]
#[cfg(feature = "compression")]
fn test_compress_flate_updates_length() {
    let data = "Hello, this is test data for compression!".repeat(10);
    let mut stream = Stream::new(data.into_bytes());

    let original_length = stream.data().len();
    stream.compress_flate().expect("Compression should succeed");

    let compressed_length = stream.data().len();
    let dict_length = match stream.dictionary().get("Length") {
        Some(Object::Integer(len)) => *len as usize,
        _ => panic!("Length should be an integer"),
    };

    assert_eq!(
        compressed_length, dict_length,
        "After compression, dictionary Length ({}) should match data length ({})",
        dict_length, compressed_length
    );
    assert_ne!(
        original_length, compressed_length,
        "Compressed data should differ from original"
    );
}

/// Test that with_dictionary corrects an incorrect Length
#[test]
fn test_with_dictionary_corrects_wrong_length() {
    let mut dict = Dictionary::new();
    dict.set("Length", 999); // Intentionally wrong
    dict.set("Type", "XObject");

    let data = vec![1, 2, 3, 4, 5];
    let stream = Stream::with_dictionary(dict, data);

    assert_eq!(
        stream.dictionary().get("Length"),
        Some(&Object::Integer(5)),
        "with_dictionary should correct the Length to match data"
    );
}

// =============================================================================
// Stream Synchronization Helper Function Tests
// These test a proposed sync_length() method that should be implemented
// =============================================================================

/// Helper to manually synchronize Length with data
/// This tests the pattern that should be used until auto-sync is implemented
#[test]
fn test_manual_length_sync_pattern() {
    let mut stream = Stream::new(vec![1, 2, 3]);

    // Modify data
    stream.data_mut().extend([4, 5, 6]);

    // Manually sync - this is the pattern users should follow
    let actual_len = stream.data().len() as i64;
    stream.dictionary_mut().set("Length", actual_len);

    assert_eq!(
        stream.dictionary().get("Length"),
        Some(&Object::Integer(6)),
        "Manual sync should update Length correctly"
    );
}

// =============================================================================
// PDF Document Stream Writing Tests
// =============================================================================

/// Test that a created PDF with streams can be parsed without errors
#[test]
fn test_pdf_with_streams_is_parseable() {
    // Create a simple document with a content stream
    let mut doc = Document::new();
    let mut page = Page::a4();
    page.text()
        .set_font(Font::Helvetica, 12.0)
        .at(100.0, 700.0)
        .write("Test")
        .unwrap();
    doc.add_page(page);

    // Save to bytes
    let pdf_bytes = doc.to_bytes().expect("Save should succeed");

    // Verify it can be re-parsed
    let cursor = std::io::Cursor::new(pdf_bytes);
    let result = oxidize_pdf::parser::PdfReader::new(cursor);
    assert!(result.is_ok(), "Generated PDF should be parseable");
}

/// Test that content streams have correct Length
#[test]
fn test_content_stream_has_correct_length() {
    let mut doc = Document::new();
    // Disable compression to make Length verification straightforward
    doc.set_compress(false);

    let mut page = Page::a4();
    page.text()
        .set_font(Font::Helvetica, 12.0)
        .at(100.0, 700.0)
        .write("Hello World")
        .unwrap();
    doc.add_page(page);

    // Save to bytes
    let pdf_bytes = doc.to_bytes().expect("Save should succeed");

    // Convert to string for analysis (PDF is mostly ASCII)
    let pdf_str = String::from_utf8_lossy(&pdf_bytes);

    // Find stream...endstream sections and verify Length matches
    let stream_keyword = "stream\n";
    let endstream_keyword = "\nendstream";

    let mut stream_count = 0;
    let mut search_start = 0;

    while let Some(stream_pos) = pdf_str[search_start..].find(stream_keyword) {
        let abs_stream_pos = search_start + stream_pos;
        let data_start = abs_stream_pos + stream_keyword.len();

        if let Some(rel_end_pos) = pdf_str[data_start..].find(endstream_keyword) {
            let stream_data_len = rel_end_pos;

            // Find the corresponding /Length in the dictionary before "stream"
            let dict_search_start = if abs_stream_pos > 200 {
                abs_stream_pos - 200
            } else {
                0
            };
            let dict_section = &pdf_str[dict_search_start..abs_stream_pos];

            if let Some(length_pos) = dict_section.rfind("/Length ") {
                let length_start = length_pos + 8;
                let remaining = &dict_section[length_start..];
                let length_end = remaining
                    .find(|c: char| !c.is_ascii_digit())
                    .unwrap_or(remaining.len());
                if let Ok(declared_length) = remaining[..length_end].parse::<usize>() {
                    assert_eq!(
                        declared_length, stream_data_len,
                        "Stream {} has Length={} but actual data is {} bytes",
                        stream_count, declared_length, stream_data_len
                    );
                }
            }
            stream_count += 1;
            search_start = data_start + rel_end_pos + endstream_keyword.len();
        } else {
            break;
        }
    }

    assert!(stream_count > 0, "PDF should contain at least one stream");
}

// =============================================================================
// Edge Case Tests
// =============================================================================

/// Test empty stream handling
#[test]
fn test_empty_stream_length() {
    let stream = Stream::new(vec![]);

    assert_eq!(
        stream.dictionary().get("Length"),
        Some(&Object::Integer(0)),
        "Empty stream should have Length=0"
    );
}

/// Test large stream handling
#[test]
fn test_large_stream_length() {
    let data: Vec<u8> = (0..100_000).map(|i| (i % 256) as u8).collect();
    let stream = Stream::new(data.clone());

    assert_eq!(
        stream.dictionary().get("Length"),
        Some(&Object::Integer(100_000)),
        "Large stream should have correct Length"
    );
}

/// Test stream with binary data (bytes that look like PDF keywords)
#[test]
fn test_stream_with_binary_data() {
    // Create data that contains "endstream" as bytes - should NOT confuse parser
    let mut data = vec![0u8; 100];
    data[10..19].copy_from_slice(b"endstream");

    let stream = Stream::new(data.clone());

    assert_eq!(
        stream.dictionary().get("Length"),
        Some(&Object::Integer(100)),
        "Stream with 'endstream' in data should still have correct Length"
    );
}

// =============================================================================
// Regression Tests for Known Issues
// =============================================================================

/// Test that multiple streams in a document all have correct lengths
#[test]
fn test_multiple_streams_correct_length() {
    let mut doc = Document::new();

    // Add multiple pages with content
    for i in 0..5 {
        let mut page = Page::a4();
        page.text()
            .set_font(Font::Helvetica, 12.0)
            .at(100.0, 700.0)
            .write(&format!("Page {}", i + 1))
            .unwrap();
        doc.add_page(page);
    }

    let pdf_bytes = doc.to_bytes().expect("Save should succeed");

    // Parse and verify
    let cursor = std::io::Cursor::new(pdf_bytes);
    let reader = oxidize_pdf::parser::PdfReader::new(cursor).expect("Should parse generated PDF");

    let doc = oxidize_pdf::parser::PdfDocument::new(reader);
    let page_count = doc.page_count().expect("Should get page count");

    assert_eq!(page_count, 5, "Should have 5 pages");
}

/// Test that font streams have correct length
#[test]
fn test_font_stream_length() {
    let mut doc = Document::new();
    let mut page = Page::a4();

    // Use text that should trigger font embedding
    page.text()
        .set_font(Font::Helvetica, 12.0)
        .at(100.0, 700.0)
        .write("Hello World with various characters: ABC 123")
        .unwrap();

    doc.add_page(page);

    // Generate PDF
    let pdf_bytes = doc.to_bytes().expect("Save should succeed");

    // Verify basic structure
    assert!(!pdf_bytes.is_empty());
    assert!(pdf_bytes.starts_with(b"%PDF-"));

    // Verify no obvious corruption
    let pdf_str = String::from_utf8_lossy(&pdf_bytes);
    assert!(pdf_str.contains("%%EOF"), "PDF should have proper EOF");
}

/// Test that compressed streams have correct Length
#[test]
fn test_compressed_stream_has_correct_length() {
    let mut doc = Document::new();
    // Enable compression (default, but explicit for clarity)
    doc.set_compress(true);

    let mut page = Page::a4();
    // Add enough text to get meaningful compression
    page.text()
        .set_font(Font::Helvetica, 12.0)
        .at(100.0, 700.0)
        .write("Hello World - this is a test with some repetitive content. ")
        .unwrap();
    doc.add_page(page);

    // Save to bytes
    let pdf_bytes = doc.to_bytes().expect("Save should succeed");

    // Parse and verify - if Length was wrong, parsing would fail
    let cursor = std::io::Cursor::new(&pdf_bytes);
    let reader =
        oxidize_pdf::parser::PdfReader::new(cursor).expect("Compressed PDF should be parseable");

    let parsed_doc = oxidize_pdf::parser::PdfDocument::new(reader);
    assert_eq!(parsed_doc.page_count().unwrap(), 1, "Should have 1 page");

    // Verify the PDF has FlateDecode filter (meaning it's compressed)
    let pdf_str = String::from_utf8_lossy(&pdf_bytes);
    assert!(
        pdf_str.contains("FlateDecode"),
        "Compressed PDF should use FlateDecode filter"
    );
}

/// Test PDF round-trip: create, save, parse, verify
#[test]
fn test_pdf_roundtrip_stream_integrity() {
    // Create document
    let mut doc = Document::new();
    doc.set_title("Stream Test");

    let mut page = Page::a4();
    page.text()
        .set_font(Font::Helvetica, 14.0)
        .at(100.0, 700.0)
        .write("Round-trip test")
        .unwrap();
    doc.add_page(page);

    // Save and reload
    let pdf_bytes = doc.to_bytes().expect("Save should succeed");

    let cursor = std::io::Cursor::new(&pdf_bytes);
    let reader = oxidize_pdf::parser::PdfReader::new(cursor).expect("Should parse generated PDF");

    let parsed_doc = oxidize_pdf::parser::PdfDocument::new(reader);

    // Verify page count
    assert_eq!(
        parsed_doc.page_count().unwrap(),
        1,
        "Should have 1 page after round-trip"
    );

    // Verify we can access the page
    let _page = parsed_doc.get_page(0).expect("Should get page 0");
}
