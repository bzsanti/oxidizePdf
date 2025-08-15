//! Integration tests for compression functionality

use oxidize_pdf::compression::{compress, decompress};
use oxidize_pdf::{Document, Page};

#[test]
fn test_compress_decompress_pdf_content() {
    // Create PDF content stream
    let content = b"q\n\
        BT\n\
        /F1 12 Tf\n\
        100 700 Td\n\
        (Hello, World!) Tj\n\
        ET\n\
        Q";

    // Compress the content
    let compressed = compress(content).unwrap();
    assert!(compressed.len() > 0);
    assert!(compressed.len() < content.len()); // Should be smaller for this text

    // Decompress and verify
    let decompressed = decompress(&compressed).unwrap();
    assert_eq!(decompressed, content);
}

#[test]
fn test_compress_large_pdf_stream() {
    // Create a large repetitive stream (simulating PDF patterns)
    let mut large_stream = Vec::new();
    for i in 0..1000 {
        large_stream.extend_from_slice(b"0 0 m\n");
        large_stream.extend_from_slice(format!("{} {} l\n", i, i).as_bytes());
        large_stream.extend_from_slice(b"S\n");
    }

    let compressed = compress(&large_stream).unwrap();

    // Should achieve significant compression due to repetition
    assert!(compressed.len() < large_stream.len() / 2);

    // Verify round-trip
    let decompressed = decompress(&compressed).unwrap();
    assert_eq!(decompressed, large_stream);
}

#[test]
fn test_compress_binary_pdf_data() {
    // Simulate binary data that might appear in a PDF (e.g., image data)
    let mut binary_data = Vec::new();
    for i in 0..=255 {
        binary_data.push(i as u8);
        binary_data.push((255 - i) as u8);
    }

    let compressed = compress(&binary_data).unwrap();
    assert!(compressed.len() > 0);

    let decompressed = decompress(&compressed).unwrap();
    assert_eq!(decompressed, binary_data);
}

#[test]
fn test_pdf_with_compressed_streams() {
    let mut doc = Document::new();

    // Add a page with content that will be compressed
    let mut page = Page::new(595.0, 842.0);

    // Add multiple text elements (will be compressed in the content stream)
    let gc = page.graphics();
    gc.save_state();
    gc.begin_text();
    for i in 0..10 {
        let y = 700.0 - (i as f64 * 20.0);
        gc.set_text_position(100.0, y);
        gc.show_text(&format!("Line {}: This is a test of PDF compression", i));
    }
    gc.end_text();
    gc.restore_state();

    doc.add_page(page);

    // Save to bytes (streams should be compressed)
    let pdf_bytes = doc.to_bytes().unwrap();

    // Verify PDF contains FlateDecode filter
    let pdf_str = String::from_utf8_lossy(&pdf_bytes);
    assert!(pdf_str.contains("/FlateDecode") || pdf_str.contains("/Fl"));
}

#[test]
fn test_compress_empty_stream() {
    let empty = b"";
    let compressed = compress(empty).unwrap();

    // Even empty data produces some output (headers)
    assert!(compressed.len() > 0);

    let decompressed = decompress(&compressed).unwrap();
    assert_eq!(decompressed, empty);
}

#[test]
fn test_compress_single_byte_patterns() {
    // Test various single-byte patterns
    let patterns = vec![
        vec![0x00; 1000], // All zeros
        vec![0xFF; 1000], // All ones
        vec![0xAA; 1000], // Alternating bits
        vec![0x55; 1000], // Alternating bits (inverse)
    ];

    for pattern in patterns {
        let compressed = compress(&pattern).unwrap();

        // Should achieve excellent compression for repeated bytes
        assert!(compressed.len() < 100); // Much smaller than 1000

        let decompressed = decompress(&compressed).unwrap();
        assert_eq!(decompressed, pattern);
    }
}

#[test]
fn test_decompress_invalid_data_handling() {
    // Test with data that's not valid compressed data
    let invalid_data = b"This is not compressed data!";
    let result = decompress(invalid_data);
    assert!(result.is_err());

    // Test with truncated compressed data
    let valid = compress(b"Valid data").unwrap();
    if valid.len() > 5 {
        let truncated = &valid[..valid.len() / 2];
        let result = decompress(truncated);
        // Should either error or produce incorrect output
        assert!(result.is_err() || result.unwrap() != b"Valid data");
    }
}

#[test]
fn test_compress_unicode_pdf_text() {
    // Test compression of Unicode text that might appear in PDFs
    let unicode_text = "Hello 世界! 🎉 PDF Unicode test äöü €".as_bytes();

    let compressed = compress(unicode_text).unwrap();
    assert!(compressed.len() > 0);

    let decompressed = decompress(&compressed).unwrap();
    assert_eq!(decompressed, unicode_text);

    // Verify the text is preserved
    let restored = String::from_utf8(decompressed).unwrap();
    assert_eq!(restored, "Hello 世界! 🎉 PDF Unicode test äöü €");
}

#[test]
fn test_compress_mixed_content() {
    // Simulate mixed PDF content (text commands + binary data)
    let mut mixed = Vec::new();
    mixed.extend_from_slice(b"BT /F1 12 Tf ");
    mixed.extend_from_slice(&[0xFF, 0x00, 0xFF, 0x00]); // Binary
    mixed.extend_from_slice(b" 100 700 Td (Text) Tj ET");

    let compressed = compress(&mixed).unwrap();
    let decompressed = decompress(&compressed).unwrap();
    assert_eq!(decompressed, mixed);
}
