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
        assert!(!compressed.is_empty());

        let decompressed = decompress(&compressed).unwrap();
        assert_eq!(decompressed, original);
    }

    #[test]
    fn test_compress_empty() {
        let compressed = compress(b"").unwrap();
        assert!(!compressed.is_empty()); // Even empty data has headers

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
}
