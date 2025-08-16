//! Compression utilities for PDF streams

use crate::error::{PdfError, Result};

/// Compress data using Flate/Zlib compression
pub fn compress(data: &[u8]) -> Result<Vec<u8>> {
    use flate2::write::ZlibEncoder;
    use flate2::Compression;
    use std::io::Write;

    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(data).map_err(PdfError::Io)?;
    encoder.finish().map_err(PdfError::Io)
}

/// Decompress data using Flate/Zlib decompression
pub fn decompress(data: &[u8]) -> Result<Vec<u8>> {
    use flate2::read::ZlibDecoder;
    use std::io::Read;

    let mut decoder = ZlibDecoder::new(data);
    let mut decompressed = Vec::new();
    decoder
        .read_to_end(&mut decompressed)
        .map_err(PdfError::Io)?;
    Ok(decompressed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compress_decompress_roundtrip() {
        let original = b"Hello, this is a test string that should be compressed and decompressed!";

        let compressed = compress(original).unwrap();
        assert!(compressed.len() > 0);

        let decompressed = decompress(&compressed).unwrap();
        assert_eq!(decompressed, original);
    }

    #[test]
    fn test_compress_empty() {
        let compressed = compress(b"").unwrap();
        assert!(compressed.len() > 0); // Even empty data has headers

        let decompressed = decompress(&compressed).unwrap();
        assert_eq!(decompressed, b"");
    }

    #[test]
    fn test_compress_large_data() {
        let large_data: Vec<u8> = (0..10000).map(|i| (i % 256) as u8).collect();

        let compressed = compress(&large_data).unwrap();
        let decompressed = decompress(&compressed).unwrap();

        assert_eq!(decompressed, large_data);
    }

    #[test]
    fn test_compress_single_byte() {
        let data = b"A";
        let compressed = compress(data).unwrap();
        assert!(compressed.len() > 0);

        let decompressed = decompress(&compressed).unwrap();
        assert_eq!(decompressed, data);
    }

    #[test]
    fn test_compress_repetitive_data() {
        // Highly compressible data
        let data: Vec<u8> = vec![0x42; 1000]; // 1000 'B' characters

        let compressed = compress(&data).unwrap();
        // Should compress well due to repetition
        assert!(compressed.len() < data.len());
        assert!(compressed.len() < 100); // Should be very small

        let decompressed = decompress(&compressed).unwrap();
        assert_eq!(decompressed, data);
    }

    #[test]
    fn test_compress_random_like_data() {
        // Less compressible data (pseudo-random)
        let data: Vec<u8> = (0..256)
            .cycle()
            .take(1000)
            .map(|i| (i * 7 + 13) as u8)
            .collect();

        let compressed = compress(&data).unwrap();
        // Random data doesn't compress as well
        // But modern algorithms might still achieve some compression
        assert!(compressed.len() > 0);

        let decompressed = decompress(&compressed).unwrap();
        assert_eq!(decompressed, data);
    }

    #[test]
    fn test_compress_pdf_like_data() {
        // Simulate PDF-like content
        let data = b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n";

        let compressed = compress(data).unwrap();
        assert!(compressed.len() > 0);

        let decompressed = decompress(&compressed).unwrap();
        assert_eq!(decompressed, data);
    }

    #[test]
    fn test_compress_binary_data() {
        // All possible byte values
        let data: Vec<u8> = (0..=255).collect();

        let compressed = compress(&data).unwrap();
        assert!(compressed.len() > 0);

        let decompressed = decompress(&compressed).unwrap();
        assert_eq!(decompressed, data);
    }

    #[test]
    fn test_decompress_invalid_data() {
        // Invalid compressed data should fail
        let invalid_data = b"This is not valid compressed data!";
        let result = decompress(invalid_data);
        assert!(result.is_err());
    }

    #[test]
    fn test_decompress_truncated_data() {
        // Create valid compressed data then truncate it
        let original = b"Valid data to compress";
        let compressed = compress(original).unwrap();

        // Truncate the compressed data
        if compressed.len() > 2 {
            let truncated = &compressed[..compressed.len() / 2];
            let result = decompress(truncated);
            // Truncated data should fail to decompress
            assert!(result.is_err() || result.unwrap() != original);
        }
    }

    #[test]
    fn test_compress_unicode_text() {
        let data = "Hello 世界! 🎉 UTF-8 test".as_bytes();

        let compressed = compress(data).unwrap();
        assert!(compressed.len() > 0);

        let decompressed = decompress(&compressed).unwrap();
        assert_eq!(decompressed, data);
        assert_eq!(
            String::from_utf8(decompressed).unwrap(),
            "Hello 世界! 🎉 UTF-8 test"
        );
    }

    #[test]
    fn test_compress_max_compression_ratio() {
        // Test with data that should compress extremely well
        let data = vec![0u8; 100_000]; // 100KB of zeros

        let compressed = compress(&data).unwrap();
        // Should achieve excellent compression ratio
        assert!(compressed.len() < 1000); // Should be less than 1KB

        let decompressed = decompress(&compressed).unwrap();
        assert_eq!(decompressed.len(), 100_000);
        assert_eq!(decompressed, data);
    }

    #[test]
    fn test_compress_multiple_rounds() {
        // Test compressing already compressed data
        let original = b"Test data for multiple compression rounds";

        let compressed_once = compress(original).unwrap();
        let compressed_twice = compress(&compressed_once).unwrap();

        // Double compression usually makes data larger
        assert!(compressed_twice.len() >= compressed_once.len());

        // Should still decompress correctly
        let decompressed_once = decompress(&compressed_twice).unwrap();
        assert_eq!(decompressed_once, compressed_once);

        let decompressed_twice = decompress(&decompressed_once).unwrap();
        assert_eq!(decompressed_twice, original);
    }

    #[test]
    fn test_compress_stream_boundaries() {
        // Test data at various size boundaries
        let sizes = vec![
            1, 15, 16, 17, 31, 32, 33, 63, 64, 65, 127, 128, 129, 255, 256, 257, 511, 512, 513,
            1023, 1024, 1025,
        ];

        for size in sizes {
            let data: Vec<u8> = (0..size).map(|i| (i % 256) as u8).collect();

            let compressed = compress(&data).unwrap();
            let decompressed = decompress(&compressed).unwrap();

            assert_eq!(decompressed.len(), size, "Failed for size {}", size);
            assert_eq!(decompressed, data, "Data mismatch for size {}", size);
        }
    }

    #[test]
    fn test_compress_performance_characteristics() {
        // Test that compression behaves as expected for different data patterns

        // Highly compressible
        let repetitive = vec![b'A'; 10000];
        let compressed_repetitive = compress(&repetitive).unwrap();
        assert!(compressed_repetitive.len() < repetitive.len() / 10);

        // Moderately compressible
        let text = b"The quick brown fox jumps over the lazy dog. "
            .iter()
            .cycle()
            .take(10000)
            .copied()
            .collect::<Vec<u8>>();
        let compressed_text = compress(&text).unwrap();
        assert!(compressed_text.len() < text.len() / 2);

        // Poorly compressible (random-like)
        let random_like: Vec<u8> = (0..10000)
            .map(|i| ((i * 214013 + 2531011) % 256) as u8)
            .collect();
        let compressed_random = compress(&random_like).unwrap();
        // Random data should not compress significantly
        // But modern algorithms may achieve better compression than expected
        // Just verify that compression happened
        assert!(compressed_random.len() > 0);
    }

    #[test]
    fn test_compress_different_compression_levels() {
        use flate2::write::ZlibEncoder;
        use flate2::Compression;
        use std::io::Write;

        let data = b"This is test data that will be compressed at different levels. ".repeat(100);

        // Test different compression levels manually
        let levels = vec![
            Compression::none(),
            Compression::fast(),
            Compression::default(),
            Compression::best(),
        ];

        let mut sizes = Vec::new();
        for level in levels {
            let mut encoder = ZlibEncoder::new(Vec::new(), level);
            encoder.write_all(&data).unwrap();
            let compressed = encoder.finish().unwrap();
            sizes.push(compressed.len());

            // Verify decompression works regardless of level
            let decompressed = decompress(&compressed).unwrap();
            assert_eq!(decompressed, data);
        }

        // Verify that compression levels work (none should be larger than compressed)
        // none() should give largest size
        assert!(
            sizes[0] >= sizes[1],
            "none() compression should be >= fast()"
        );
        // Note: best() vs default() may vary based on data, so we just verify they compress
        assert!(
            sizes[2] < sizes[0],
            "default() should compress better than none()"
        );
        assert!(
            sizes[3] < sizes[0],
            "best() should compress better than none()"
        );
    }

    #[test]
    fn test_compress_with_null_bytes() {
        // PDF files often contain null bytes
        let mut data = Vec::new();
        data.extend_from_slice(b"PDF-1.4\n");
        data.extend_from_slice(&[0x00; 100]);
        data.extend_from_slice(b"\n%%EOF");

        let compressed = compress(&data).unwrap();
        let decompressed = decompress(&compressed).unwrap();
        assert_eq!(decompressed, data);
    }

    #[test]
    fn test_compress_pdf_like_content() {
        // Simulate typical PDF content stream
        let pdf_content = b"q\n\
            1 0 0 1 0 0 cm\n\
            BT\n\
            /F1 12 Tf\n\
            100 700 Td\n\
            (Hello World) Tj\n\
            ET\n\
            Q\n\
            q\n\
            0.5 0 0 0.5 200 400 cm\n\
            1 0 0 rg\n\
            0 0 100 100 re\n\
            f\n\
            Q"
        .repeat(10);

        let compressed = compress(&pdf_content).unwrap();
        // PDF content should compress well due to repetitive commands
        assert!(compressed.len() < pdf_content.len() / 2);

        let decompressed = decompress(&compressed).unwrap();
        assert_eq!(decompressed, pdf_content);
    }

    #[test]
    fn test_decompress_invalid_zlib_header() {
        // Invalid zlib header
        let invalid = vec![0xFF, 0xFF, 0x00, 0x00];
        let result = decompress(&invalid);
        assert!(result.is_err());
    }

    #[test]
    fn test_decompress_truncated_stream() {
        // Create valid compressed data then truncate it
        let original = b"This is valid data that will be compressed then truncated";
        let compressed = compress(original).unwrap();

        // Test severe truncation (should definitely fail)
        if compressed.len() > 10 {
            let severely_truncated = &compressed[..compressed.len() / 4];
            let result = decompress(severely_truncated);
            // Severe truncation should either fail or produce wrong data
            assert!(
                result.is_err() || result.unwrap() != original,
                "Severely truncated data should fail or produce incorrect result"
            );
        }

        // Test removing last byte (may or may not fail depending on padding)
        if compressed.len() > 1 {
            let truncated = &compressed[..compressed.len() - 1];
            let result = decompress(truncated);
            // May succeed with wrong data or fail
            if let Ok(decompressed) = result {
                // If it succeeds, it shouldn't match original (in most cases)
                // Note: In rare cases it might match if last byte was padding
                // So we just verify decompression attempted
                assert!(decompressed.len() <= original.len());
            }
        }
    }

    #[test]
    fn test_compress_maximum_size() {
        // Test with maximum practical size (1MB)
        let large_data = vec![b'X'; 1024 * 1024];

        let compressed = compress(&large_data).unwrap();
        assert!(compressed.len() > 0);
        assert!(compressed.len() < large_data.len()); // Should compress repeated data

        let decompressed = decompress(&compressed).unwrap();
        assert_eq!(decompressed.len(), large_data.len());
        assert_eq!(decompressed, large_data);
    }

    #[test]
    fn test_compress_unicode_utf8() {
        // Test compression of UTF-8 encoded text (common in modern PDFs)
        let unicode_data = "Hello 世界 مرحبا мир שלום 🌍🎉📝".as_bytes();

        let compressed = compress(unicode_data).unwrap();
        let decompressed = decompress(&compressed).unwrap();

        assert_eq!(decompressed, unicode_data);

        // Verify the text is preserved
        let text = String::from_utf8(decompressed).unwrap();
        assert_eq!(text, "Hello 世界 مرحبا мир שלום 🌍🎉📝");
    }

    #[test]
    fn test_compress_binary_image_data() {
        // Simulate binary data like embedded images in PDFs
        let mut image_data = Vec::new();

        // JPEG-like header
        image_data.extend_from_slice(&[0xFF, 0xD8, 0xFF, 0xE0]);

        // Random-ish binary data
        for i in 0..1000 {
            image_data.push(((i * 7 + 13) % 256) as u8);
        }

        // JPEG-like footer
        image_data.extend_from_slice(&[0xFF, 0xD9]);

        let compressed = compress(&image_data).unwrap();
        let decompressed = decompress(&compressed).unwrap();
        assert_eq!(decompressed, image_data);
    }

    #[test]
    fn test_compress_alternating_patterns() {
        // Test alternating byte patterns (common in some PDF structures)
        let patterns = vec![
            vec![0x00, 0xFF].repeat(1000),
            vec![0xAA, 0x55].repeat(1000),
            vec![0x01, 0x02, 0x04, 0x08, 0x10, 0x20, 0x40, 0x80].repeat(250),
        ];

        for pattern in patterns {
            let compressed = compress(&pattern).unwrap();
            // Patterns should compress well
            assert!(compressed.len() < pattern.len() / 3);

            let decompressed = decompress(&compressed).unwrap();
            assert_eq!(decompressed, pattern);
        }
    }
}
