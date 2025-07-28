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

    #[test]
    fn test_text_chunk_debug_clone() {
        let chunk = TextChunk {
            text: "Test".to_string(),
            x: 50.0,
            y: 100.0,
            font_size: 10.0,
            font_name: Some("Arial".to_string()),
        };

        let debug_str = format!("{:?}", chunk);
        assert!(debug_str.contains("TextChunk"));
        assert!(debug_str.contains("Test"));

        let cloned = chunk.clone();
        assert_eq!(cloned.text, chunk.text);
        assert_eq!(cloned.x, chunk.x);
        assert_eq!(cloned.y, chunk.y);
        assert_eq!(cloned.font_size, chunk.font_size);
        assert_eq!(cloned.font_name, chunk.font_name);
    }

    #[test]
    fn test_text_stream_options_custom() {
        let options = TextStreamOptions {
            min_font_size: 8.0,
            max_buffer_size: 2048,
            preserve_formatting: false,
            sort_by_position: false,
        };

        assert_eq!(options.min_font_size, 8.0);
        assert_eq!(options.max_buffer_size, 2048);
        assert!(!options.preserve_formatting);
        assert!(!options.sort_by_position);
    }

    #[test]
    fn test_text_stream_options_debug_clone() {
        let options = TextStreamOptions::default();

        let debug_str = format!("{:?}", options);
        assert!(debug_str.contains("TextStreamOptions"));

        let cloned = options.clone();
        assert_eq!(cloned.min_font_size, options.min_font_size);
        assert_eq!(cloned.max_buffer_size, options.max_buffer_size);
        assert_eq!(cloned.preserve_formatting, options.preserve_formatting);
        assert_eq!(cloned.sort_by_position, options.sort_by_position);
    }

    #[test]
    fn test_text_streamer_process_empty_chunk() {
        let mut streamer = TextStreamer::new(TextStreamOptions::default());
        let chunks = streamer.process_chunk(b"").unwrap();
        assert!(chunks.is_empty());
    }

    #[test]
    fn test_text_streamer_process_invalid_content() {
        let mut streamer = TextStreamer::new(TextStreamOptions::default());
        // Invalid PDF content should be handled gracefully
        let content = b"Not valid PDF content";
        let result = streamer.process_chunk(content);
        // Should either succeed with no chunks or return an error
        match result {
            Ok(chunks) => assert!(chunks.is_empty()),
            Err(_) => {} // Error is also acceptable
        }
    }

    #[test]
    fn test_text_streamer_font_tracking() {
        let mut streamer = TextStreamer::new(TextStreamOptions::default());

        // Set font operation
        let content = b"BT /Helvetica-Bold 16 Tf ET";
        let _ = streamer.process_chunk(content).unwrap();

        assert_eq!(streamer.current_font, Some("Helvetica-Bold".to_string()));
        assert_eq!(streamer.current_font_size, 16.0);
    }

    #[test]
    fn test_text_streamer_position_tracking() {
        let mut streamer = TextStreamer::new(TextStreamOptions::default());

        // Move text position
        let content = b"BT 50 100 Td ET";
        let _ = streamer.process_chunk(content).unwrap();

        assert_eq!(streamer.current_x, 50.0);
        assert_eq!(streamer.current_y, 100.0);
    }

    #[test]
    fn test_text_streamer_begin_text_resets_position() {
        let mut streamer = TextStreamer::new(TextStreamOptions::default());

        // Set position
        streamer.current_x = 100.0;
        streamer.current_y = 200.0;

        // BeginText should reset position
        let content = b"BT ET";
        let _ = streamer.process_chunk(content).unwrap();

        assert_eq!(streamer.current_x, 0.0);
        assert_eq!(streamer.current_y, 0.0);
    }

    #[test]
    fn test_text_streamer_clear_buffer() {
        let mut streamer = TextStreamer::new(TextStreamOptions::default());

        // Add some chunks
        streamer.buffer.push_back(TextChunk {
            text: "Chunk1".to_string(),
            x: 0.0,
            y: 0.0,
            font_size: 12.0,
            font_name: None,
        });
        streamer.buffer.push_back(TextChunk {
            text: "Chunk2".to_string(),
            x: 0.0,
            y: 0.0,
            font_size: 12.0,
            font_name: None,
        });

        assert_eq!(streamer.buffer.len(), 2);

        streamer.clear_buffer();
        assert!(streamer.buffer.is_empty());
    }

    #[test]
    fn test_text_streamer_get_buffered_chunks() {
        let mut streamer = TextStreamer::new(TextStreamOptions::default());

        let chunk1 = TextChunk {
            text: "First".to_string(),
            x: 10.0,
            y: 20.0,
            font_size: 14.0,
            font_name: Some("Times".to_string()),
        };
        let chunk2 = TextChunk {
            text: "Second".to_string(),
            x: 30.0,
            y: 40.0,
            font_size: 16.0,
            font_name: Some("Arial".to_string()),
        };

        streamer.buffer.push_back(chunk1.clone());
        streamer.buffer.push_back(chunk2.clone());

        let chunks = streamer.get_buffered_chunks();
        assert_eq!(chunks.len(), 2);
        assert_eq!(chunks[0].text, "First");
        assert_eq!(chunks[1].text, "Second");
    }

    #[test]
    fn test_extract_text_no_sorting() {
        let mut options = TextStreamOptions::default();
        options.sort_by_position = false;
        let mut streamer = TextStreamer::new(options);

        // Add text in specific order
        streamer.buffer.push_back(TextChunk {
            text: "First".to_string(),
            x: 200.0,
            y: 100.0,
            font_size: 12.0,
            font_name: None,
        });
        streamer.buffer.push_back(TextChunk {
            text: "Second".to_string(),
            x: 100.0,
            y: 200.0,
            font_size: 12.0,
            font_name: None,
        });

        let text = streamer.extract_text();
        assert_eq!(text, "First Second"); // Should maintain insertion order
    }

    #[test]
    fn test_extract_text_horizontal_sorting() {
        let mut streamer = TextStreamer::new(TextStreamOptions::default());

        // Add text on same line, different X positions
        streamer.buffer.push_back(TextChunk {
            text: "Right".to_string(),
            x: 300.0,
            y: 500.0,
            font_size: 12.0,
            font_name: None,
        });
        streamer.buffer.push_back(TextChunk {
            text: "Left".to_string(),
            x: 100.0,
            y: 500.0,
            font_size: 12.0,
            font_name: None,
        });
        streamer.buffer.push_back(TextChunk {
            text: "Middle".to_string(),
            x: 200.0,
            y: 500.0,
            font_size: 12.0,
            font_name: None,
        });

        let text = streamer.extract_text();
        assert_eq!(text, "Left Middle Right");
    }

    #[test]
    fn test_check_buffer_size_edge_cases() {
        let mut options = TextStreamOptions::default();
        options.max_buffer_size = 20;
        let mut streamer = TextStreamer::new(options);

        // Add chunk that exactly fills buffer
        streamer.buffer.push_back(TextChunk {
            text: "a".repeat(20),
            x: 0.0,
            y: 0.0,
            font_size: 12.0,
            font_name: None,
        });

        streamer.check_buffer_size();
        assert_eq!(streamer.buffer.len(), 1); // Should keep the chunk

        // Add another chunk to exceed limit
        streamer.buffer.push_back(TextChunk {
            text: "b".to_string(),
            x: 0.0,
            y: 0.0,
            font_size: 12.0,
            font_name: None,
        });

        streamer.check_buffer_size();
        // Should have removed the first chunk
        assert!(streamer.buffer.len() <= 1);
    }

    #[test]
    fn test_stream_text_with_error_callback() {
        let content = b"BT /F1 12 Tf 100 700 Td (Test) Tj ET".to_vec();
        let streams = vec![content];

        let result = stream_text(streams, |_chunk| {
            Err(crate::error::PdfError::ParseError("Test error".to_string()))
        });

        assert!(result.is_err());
    }

    #[test]
    fn test_stream_text_empty_streams() {
        let streams: Vec<Vec<u8>> = vec![];

        let mut collected = Vec::new();
        stream_text(streams, |chunk| {
            collected.push(chunk);
            Ok(())
        })
        .unwrap();

        assert!(collected.is_empty());
    }

    #[test]
    fn test_text_chunk_without_font_name() {
        let chunk = TextChunk {
            text: "No Font".to_string(),
            x: 0.0,
            y: 0.0,
            font_size: 12.0,
            font_name: None,
        };

        assert_eq!(chunk.font_name, None);
    }

    #[test]
    fn test_process_chunk_multiple_operations() {
        let mut streamer = TextStreamer::new(TextStreamOptions::default());

        // Content with multiple text operations
        let content = b"BT /F1 10 Tf 100 700 Td (First) Tj 50 0 Td (Second) Tj ET";
        let chunks = streamer.process_chunk(content).unwrap();

        assert_eq!(chunks.len(), 2);
        assert_eq!(chunks[0].text, "First");
        assert_eq!(chunks[1].text, "Second");
        assert_eq!(chunks[0].x, 100.0);
        assert_eq!(chunks[1].x, 150.0); // 100 + 50
    }

    #[test]
    fn test_buffer_size_calculation() {
        let mut options = TextStreamOptions::default();
        options.max_buffer_size = 100;
        let mut streamer = TextStreamer::new(options);

        // Add chunks with known sizes
        for _i in 0..10 {
            streamer.buffer.push_back(TextChunk {
                text: "1234567890".to_string(), // 10 bytes each
                x: 0.0,
                y: 0.0,
                font_size: 12.0,
                font_name: None,
            });
        }

        // Total size is 100 bytes
        streamer.check_buffer_size();

        // Add one more to exceed
        streamer.buffer.push_back(TextChunk {
            text: "x".to_string(),
            x: 0.0,
            y: 0.0,
            font_size: 12.0,
            font_name: None,
        });

        streamer.check_buffer_size();

        // Should have removed oldest chunks
        let total_size: usize = streamer.buffer.iter().map(|c| c.text.len()).sum();
        assert!(total_size <= 100);
    }
}
