//! Chunk-based content processing for streaming operations
//!
//! Processes PDF content in manageable chunks to maintain
//! memory efficiency while handling large documents.

use crate::error::Result;
use std::io::Read;

/// Type of content chunk
#[derive(Debug, Clone, PartialEq)]
pub enum ChunkType {
    /// Text content
    Text,
    /// Image data
    Image,
    /// Vector graphics
    Graphics,
    /// Form XObject
    Form,
    /// Unknown or mixed content
    Unknown,
}

/// A chunk of PDF content
#[derive(Debug, Clone)]
pub struct ContentChunk {
    /// Type of content in this chunk
    pub chunk_type: ChunkType,
    /// Raw data of the chunk
    pub data: Vec<u8>,
    /// Position in the document
    pub position: u64,
    /// Size of the chunk
    pub size: usize,
    /// Page number this chunk belongs to
    pub page_number: u32,
}

impl ContentChunk {
    /// Create a new content chunk
    pub fn new(chunk_type: ChunkType, data: Vec<u8>, position: u64, page_number: u32) -> Self {
        let size = data.len();
        Self {
            chunk_type,
            data,
            position,
            size,
            page_number,
        }
    }

    /// Check if this is a text chunk
    pub fn is_text(&self) -> bool {
        self.chunk_type == ChunkType::Text
    }

    /// Check if this is an image chunk
    pub fn is_image(&self) -> bool {
        self.chunk_type == ChunkType::Image
    }

    /// Get the chunk data as a string (for text chunks)
    pub fn as_text(&self) -> Option<String> {
        if self.is_text() {
            Some(String::from_utf8_lossy(&self.data).to_string())
        } else {
            None
        }
    }
}

/// Options for chunk processing
#[derive(Debug, Clone)]
pub struct ChunkOptions {
    /// Maximum size of a single chunk
    pub max_chunk_size: usize,
    /// Whether to split large objects
    pub split_large_objects: bool,
    /// Buffer size for reading
    pub buffer_size: usize,
    /// Types of chunks to process
    pub chunk_types: Vec<ChunkType>,
}

impl Default for ChunkOptions {
    fn default() -> Self {
        Self {
            max_chunk_size: 1024 * 1024, // 1MB
            split_large_objects: true,
            buffer_size: 64 * 1024, // 64KB
            chunk_types: vec![
                ChunkType::Text,
                ChunkType::Image,
                ChunkType::Graphics,
                ChunkType::Form,
            ],
        }
    }
}

/// Processes PDF content in chunks
pub struct ChunkProcessor {
    options: ChunkOptions,
    current_position: u64,
    current_page: u32,
}

impl ChunkProcessor {
    /// Create a new chunk processor
    pub fn new(options: ChunkOptions) -> Self {
        Self {
            options,
            current_position: 0,
            current_page: 0,
        }
    }

    /// Process content and yield chunks
    pub fn process_content(&mut self, content: &[u8]) -> Result<Vec<ContentChunk>> {
        let mut chunks = Vec::new();
        let mut offset = 0;

        while offset < content.len() {
            let remaining = content.len() - offset;
            let chunk_size = remaining.min(self.options.max_chunk_size);

            // Detect chunk type (simplified)
            let chunk_type = self.detect_chunk_type(&content[offset..offset + chunk_size]);

            // Skip if not in requested types
            if !self.options.chunk_types.contains(&chunk_type) {
                offset += chunk_size;
                continue;
            }

            let chunk = ContentChunk::new(
                chunk_type,
                content[offset..offset + chunk_size].to_vec(),
                self.current_position + offset as u64,
                self.current_page,
            );

            chunks.push(chunk);
            offset += chunk_size;
        }

        self.current_position += content.len() as u64;
        Ok(chunks)
    }

    /// Set the current page number
    pub fn set_page(&mut self, page_number: u32) {
        self.current_page = page_number;
    }

    /// Reset the processor state
    pub fn reset(&mut self) {
        self.current_position = 0;
        self.current_page = 0;
    }

    fn detect_chunk_type(&self, data: &[u8]) -> ChunkType {
        // Simple heuristic for chunk type detection
        if data.starts_with(b"BT") || data.contains(&b'T') && data.contains(&b'j') {
            ChunkType::Text
        } else if data.starts_with(b"\xFF\xD8") || data.starts_with(b"\x89PNG") {
            ChunkType::Image
        } else if data.contains(&b'm') || data.contains(&b'l') || data.contains(&b'c') {
            ChunkType::Graphics
        } else {
            ChunkType::Unknown
        }
    }
}

/// Process a reader in chunks
pub fn process_in_chunks<R, F>(mut reader: R, options: ChunkOptions, mut callback: F) -> Result<()>
where
    R: Read,
    F: FnMut(ContentChunk) -> Result<()>,
{
    let mut processor = ChunkProcessor::new(options.clone());
    let mut buffer = vec![0u8; options.buffer_size];
    let mut _position = 0u64;

    loop {
        match reader.read(&mut buffer) {
            Ok(0) => break, // EOF
            Ok(n) => {
                let chunks = processor.process_content(&buffer[..n])?;
                for chunk in chunks {
                    callback(chunk)?;
                }
                _position += n as u64;
            }
            Err(e) => return Err(crate::error::PdfError::Io(e)),
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_content_chunk() {
        let chunk = ContentChunk::new(ChunkType::Text, b"Hello World".to_vec(), 1024, 0);

        assert_eq!(chunk.chunk_type, ChunkType::Text);
        assert_eq!(chunk.size, 11);
        assert_eq!(chunk.position, 1024);
        assert_eq!(chunk.page_number, 0);
        assert!(chunk.is_text());
        assert!(!chunk.is_image());
        assert_eq!(chunk.as_text(), Some("Hello World".to_string()));
    }

    #[test]
    fn test_chunk_options_default() {
        let options = ChunkOptions::default();
        assert_eq!(options.max_chunk_size, 1024 * 1024);
        assert!(options.split_large_objects);
        assert_eq!(options.buffer_size, 64 * 1024);
        assert_eq!(options.chunk_types.len(), 4);
    }

    #[test]
    fn test_chunk_processor() {
        let options = ChunkOptions::default();
        let mut processor = ChunkProcessor::new(options);

        let content = b"BT /F1 12 Tf 100 700 Td (Hello) Tj ET";
        let chunks = processor.process_content(content).unwrap();

        assert!(!chunks.is_empty());
        assert_eq!(chunks[0].chunk_type, ChunkType::Text);
        assert_eq!(chunks[0].data, content);
    }

    #[test]
    fn test_chunk_type_detection() {
        let processor = ChunkProcessor::new(ChunkOptions::default());

        // Text content
        let text = b"BT /F1 12 Tf (text) Tj ET";
        assert_eq!(processor.detect_chunk_type(text), ChunkType::Text);

        // JPEG image
        let jpeg = b"\xFF\xD8\xFF\xE0";
        assert_eq!(processor.detect_chunk_type(jpeg), ChunkType::Image);

        // PNG image
        let png = b"\x89PNG\r\n\x1a\n";
        assert_eq!(processor.detect_chunk_type(png), ChunkType::Image);

        // Graphics
        let graphics = b"100 200 m 300 400 l S";
        assert_eq!(processor.detect_chunk_type(graphics), ChunkType::Graphics);
    }

    #[test]
    fn test_large_content_splitting() {
        let mut options = ChunkOptions::default();
        options.max_chunk_size = 10; // Very small chunks

        let mut processor = ChunkProcessor::new(options);
        let content = b"This is a much longer content that should be split into multiple chunks";

        let chunks = processor.process_content(content).unwrap();

        assert!(chunks.len() > 1);
        assert!(chunks.iter().all(|c| c.size <= 10));
    }

    #[test]
    fn test_chunk_filtering() {
        let mut options = ChunkOptions::default();
        options.chunk_types = vec![ChunkType::Text]; // Only process text

        let mut processor = ChunkProcessor::new(options);

        // Mix of content types
        let text_content = b"BT (text) Tj ET";
        let image_content = b"\xFF\xD8\xFF\xE0 image data";

        let text_chunks = processor.process_content(text_content).unwrap();
        assert_eq!(text_chunks.len(), 1);

        let image_chunks = processor.process_content(image_content).unwrap();
        assert_eq!(image_chunks.len(), 0); // Filtered out
    }

    #[test]
    fn test_process_in_chunks() {
        use std::io::Cursor;

        let data = b"BT /F1 12 Tf 100 700 Td (Hello World) Tj ET";
        let cursor = Cursor::new(data);
        let options = ChunkOptions {
            buffer_size: 10,
            ..Default::default()
        };

        let mut chunks_received = Vec::new();
        process_in_chunks(cursor, options, |chunk| {
            chunks_received.push(chunk);
            Ok(())
        })
        .unwrap();

        assert!(!chunks_received.is_empty());
    }

    #[test]
    fn test_page_tracking() {
        let mut processor = ChunkProcessor::new(ChunkOptions::default());

        processor.set_page(5);
        let content = b"Page 5 content";
        let chunks = processor.process_content(content).unwrap();

        assert!(!chunks.is_empty());
        assert_eq!(chunks[0].page_number, 5);
    }

    #[test]
    fn test_processor_reset() {
        let mut processor = ChunkProcessor::new(ChunkOptions::default());

        processor.current_position = 1000;
        processor.current_page = 10;

        processor.reset();

        assert_eq!(processor.current_position, 0);
        assert_eq!(processor.current_page, 0);
    }
}
