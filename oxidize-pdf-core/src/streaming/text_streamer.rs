//! Text streaming for incremental text extraction
//!
//! Extracts text from PDF content streams incrementally, processing
//! text operations as they are encountered.

use crate::error::Result;
use crate::parser::content::{ContentOperation, ContentParser};
use std::collections::VecDeque;

/// A chunk of extracted text with position information
#[derive(Debug, Clone)]
pub struct TextChunk {
    /// The extracted text
    pub text: String,
    /// X position on the page
    pub x: f64,
    /// Y position on the page
    pub y: f64,
    /// Font size
    pub font_size: f64,
    /// Font name (if known)
    pub font_name: Option<String>,
}

/// Options for text streaming
#[derive(Debug, Clone)]
pub struct TextStreamOptions {
    /// Minimum text size to include
    pub min_font_size: f64,
    /// Maximum buffer size for text chunks
    pub max_buffer_size: usize,
    /// Whether to preserve formatting
    pub preserve_formatting: bool,
    /// Whether to sort by position
    pub sort_by_position: bool,
}

impl Default for TextStreamOptions {
    fn default() -> Self {
        Self {
            min_font_size: 0.0,
            max_buffer_size: 1024 * 1024, // 1MB
            preserve_formatting: true,
            sort_by_position: true,
        }
    }
}

/// Streams text from PDF content
pub struct TextStreamer {
    options: TextStreamOptions,
    buffer: VecDeque<TextChunk>,
    current_font: Option<String>,
    current_font_size: f64,
    current_x: f64,
    current_y: f64,
}

impl TextStreamer {
    /// Create a new text streamer
    pub fn new(options: TextStreamOptions) -> Self {
        Self {
            options,
            buffer: VecDeque::new(),
            current_font: None,
            current_font_size: 12.0,
            current_x: 0.0,
            current_y: 0.0,
        }
    }

    /// Process a content stream chunk
    pub fn process_chunk(&mut self, data: &[u8]) -> Result<Vec<TextChunk>> {
        let operations = ContentParser::parse(data)
            .map_err(|e| crate::error::PdfError::ParseError(e.to_string()))?;

        let mut chunks = Vec::new();

        for op in operations {
            match op {
                ContentOperation::SetFont(name, size) => {
                    self.current_font = Some(name);
                    self.current_font_size = size as f64;
                }
                ContentOperation::MoveText(x, y) => {
                    self.current_x += x as f64;
                    self.current_y += y as f64;
                }
                ContentOperation::ShowText(bytes) => {
                    if self.current_font_size >= self.options.min_font_size {
                        let text = String::from_utf8_lossy(&bytes).to_string();
                        let chunk = TextChunk {
                            text,
                            x: self.current_x,
                            y: self.current_y,
                            font_size: self.current_font_size,
                            font_name: self.current_font.clone(),
                        };
                        chunks.push(chunk);
                    }
                }
                ContentOperation::BeginText => {
                    self.current_x = 0.0;
                    self.current_y = 0.0;
                }
                _ => {} // Ignore other operations
            }
        }

        // Add to buffer if needed
        for chunk in &chunks {
            self.buffer.push_back(chunk.clone());
        }

        // Check buffer size
        self.check_buffer_size();

        Ok(chunks)
    }

    /// Get all buffered text chunks
    pub fn get_buffered_chunks(&self) -> Vec<TextChunk> {
        self.buffer.iter().cloned().collect()
    }

    /// Clear the buffer
    pub fn clear_buffer(&mut self) {
        self.buffer.clear();
    }

    /// Extract text as a single string
    pub fn extract_text(&self) -> String {
        let mut chunks = self.get_buffered_chunks();

        if self.options.sort_by_position {
            // Sort by Y position (top to bottom), then X (left to right)
            chunks.sort_by(|a, b| {
                b.y.partial_cmp(&a.y)
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .then(a.x.partial_cmp(&b.x).unwrap_or(std::cmp::Ordering::Equal))
            });
        }

        chunks
            .into_iter()
            .map(|chunk| chunk.text)
            .collect::<Vec<_>>()
            .join(" ")
    }

    fn check_buffer_size(&mut self) {
        let total_size: usize = self.buffer.iter().map(|chunk| chunk.text.len()).sum();

        // Remove oldest chunks if buffer is too large
        while total_size > self.options.max_buffer_size && !self.buffer.is_empty() {
            self.buffer.pop_front();
        }
    }
}

/// Stream text from multiple content streams
pub fn stream_text<F>(content_streams: Vec<Vec<u8>>, mut callback: F) -> Result<()>
where
    F: FnMut(TextChunk) -> Result<()>,
{
    let mut streamer = TextStreamer::new(TextStreamOptions::default());

    for stream in content_streams {
        let chunks = streamer.process_chunk(&stream)?;
        for chunk in chunks {
            callback(chunk)?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_text_chunk() {
        let chunk = TextChunk {
            text: "Hello".to_string(),
            x: 100.0,
            y: 700.0,
            font_size: 12.0,
            font_name: Some("Helvetica".to_string()),
        };

        assert_eq!(chunk.text, "Hello");
        assert_eq!(chunk.x, 100.0);
        assert_eq!(chunk.y, 700.0);
        assert_eq!(chunk.font_size, 12.0);
        assert_eq!(chunk.font_name, Some("Helvetica".to_string()));
    }

    #[test]
    fn test_text_stream_options_default() {
        let options = TextStreamOptions::default();
        assert_eq!(options.min_font_size, 0.0);
        assert_eq!(options.max_buffer_size, 1024 * 1024);
        assert!(options.preserve_formatting);
        assert!(options.sort_by_position);
    }

    #[test]
    fn test_text_streamer_creation() {
        let options = TextStreamOptions::default();
        let streamer = TextStreamer::new(options);

        assert!(streamer.buffer.is_empty());
        assert_eq!(streamer.current_font_size, 12.0);
        assert_eq!(streamer.current_x, 0.0);
        assert_eq!(streamer.current_y, 0.0);
    }

    #[test]
    fn test_process_chunk_text() {
        let mut streamer = TextStreamer::new(TextStreamOptions::default());

        // Simple text showing operation
        let content = b"BT /F1 14 Tf 100 700 Td (Hello World) Tj ET";
        let chunks = streamer.process_chunk(content).unwrap();

        assert!(!chunks.is_empty());
        assert_eq!(chunks[0].text, "Hello World");
        assert_eq!(chunks[0].font_size, 14.0);
    }

    #[test]
    fn test_min_font_size_filter() {
        let mut options = TextStreamOptions::default();
        options.min_font_size = 10.0;
        let mut streamer = TextStreamer::new(options);

        // Text with small font (8pt) - should be filtered out
        let content = b"BT /F1 8 Tf 100 700 Td (Small Text) Tj ET";
        let chunks = streamer.process_chunk(content).unwrap();
        assert!(chunks.is_empty());

        // Text with large font (12pt) - should be included
        let content = b"BT /F1 12 Tf 100 650 Td (Large Text) Tj ET";
        let chunks = streamer.process_chunk(content).unwrap();
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].text, "Large Text");
    }

    #[test]
    fn test_extract_text_sorted() {
        let mut streamer = TextStreamer::new(TextStreamOptions::default());

        // Add text in random order
        streamer.buffer.push_back(TextChunk {
            text: "Bottom".to_string(),
            x: 100.0,
            y: 100.0,
            font_size: 12.0,
            font_name: None,
        });

        streamer.buffer.push_back(TextChunk {
            text: "Top".to_string(),
            x: 100.0,
            y: 700.0,
            font_size: 12.0,
            font_name: None,
        });

        streamer.buffer.push_back(TextChunk {
            text: "Middle".to_string(),
            x: 100.0,
            y: 400.0,
            font_size: 12.0,
            font_name: None,
        });

        let text = streamer.extract_text();
        assert_eq!(text, "Top Middle Bottom");
    }

    #[test]
    fn test_buffer_management() {
        let mut options = TextStreamOptions::default();
        options.max_buffer_size = 10; // Very small buffer
        let mut streamer = TextStreamer::new(options);

        // Add chunks that exceed buffer size
        for i in 0..5 {
            streamer.buffer.push_back(TextChunk {
                text: format!("Text{}", i),
                x: 0.0,
                y: 0.0,
                font_size: 12.0,
                font_name: None,
            });
        }

        streamer.check_buffer_size();

        // Buffer should be limited
        assert!(streamer.buffer.len() < 5);
    }

    #[test]
    fn test_stream_text_function() {
        let content1 = b"BT /F1 12 Tf 100 700 Td (Page 1) Tj ET".to_vec();
        let content2 = b"BT /F1 12 Tf 100 650 Td (Page 2) Tj ET".to_vec();
        let streams = vec![content1, content2];

        let mut collected = Vec::new();
        stream_text(streams, |chunk| {
            collected.push(chunk.text);
            Ok(())
        })
        .unwrap();

        assert_eq!(collected.len(), 2);
        assert_eq!(collected[0], "Page 1");
        assert_eq!(collected[1], "Page 2");
    }
}
