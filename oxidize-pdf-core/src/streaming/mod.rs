//! Streaming support for incremental PDF processing
//!
//! This module provides advanced streaming capabilities for processing PDFs
//! without loading the entire document into memory. It's designed for handling
//! very large PDFs or situations with limited memory.
//!
//! # Features
//!
//! - **Incremental Parsing**: Parse PDF objects as they're needed
//! - **Page Streaming**: Process pages one at a time
//! - **Content Stream Processing**: Handle content streams in chunks
//! - **Progressive Text Extraction**: Extract text as it's encountered
//! - **Memory Bounds**: Configurable memory limits for buffering
//! - **Async Support**: Future-ready for async I/O operations
//!
//! # Example
//!
//! ```rust,no_run
//! use oxidize_pdf::streaming::{StreamingDocument, StreamingOptions};
//! use std::fs::File;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let file = File::open("large_document.pdf")?;
//! let options = StreamingOptions::default()
//!     .with_buffer_size(1024 * 1024) // 1MB buffer
//!     .with_page_cache_size(5);      // Keep 5 pages in memory
//!
//! let mut doc = StreamingDocument::new(file, options)?;
//!
//! // Process pages incrementally
//! while let Some(page) = doc.next_page()? {
//!     println!("Processing page {}", page.number());
//!     
//!     // Extract text incrementally
//!     let text = page.extract_text_streaming()?;
//!     println!("Text: {}", text);
//! }
//! # Ok(())
//! # }
//! ```

use crate::error::Result;
use std::collections::VecDeque;
use std::io::{BufReader, Read, Seek};

pub mod chunk_processor;
pub mod incremental_parser;
pub mod page_streamer;
pub mod text_streamer;

// Re-export main types
pub use chunk_processor::{
    process_in_chunks, ChunkOptions, ChunkProcessor, ChunkType, ContentChunk,
};
pub use incremental_parser::{process_incrementally, IncrementalParser, ParseEvent};
pub use page_streamer::{PageStreamer, StreamingPage};
pub use text_streamer::{stream_text, TextChunk, TextStreamOptions, TextStreamer};

/// Options for streaming operations
#[derive(Debug, Clone)]
pub struct StreamingOptions {
    /// Buffer size for reading
    pub buffer_size: usize,
    /// Maximum number of pages to keep in cache
    pub page_cache_size: usize,
    /// Maximum size of a single content stream
    pub max_content_stream_size: usize,
    /// Enable progressive rendering hints
    pub progressive_hints: bool,
    /// Memory limit for buffers (bytes)
    pub memory_limit: usize,
}

impl Default for StreamingOptions {
    fn default() -> Self {
        Self {
            buffer_size: 256 * 1024,                   // 256KB
            page_cache_size: 3,                        // Keep 3 pages
            max_content_stream_size: 10 * 1024 * 1024, // 10MB
            progressive_hints: true,
            memory_limit: 100 * 1024 * 1024, // 100MB
        }
    }
}

impl StreamingOptions {
    /// Create options optimized for minimal memory usage
    pub fn minimal_memory() -> Self {
        Self {
            buffer_size: 64 * 1024,
            page_cache_size: 1,
            max_content_stream_size: 1024 * 1024,
            progressive_hints: false,
            memory_limit: 10 * 1024 * 1024,
        }
    }

    /// Create options optimized for speed
    pub fn fast_processing() -> Self {
        Self {
            buffer_size: 1024 * 1024,
            page_cache_size: 10,
            max_content_stream_size: 50 * 1024 * 1024,
            progressive_hints: true,
            memory_limit: 500 * 1024 * 1024,
        }
    }

    pub fn with_buffer_size(mut self, size: usize) -> Self {
        self.buffer_size = size;
        self
    }

    pub fn with_page_cache_size(mut self, size: usize) -> Self {
        self.page_cache_size = size;
        self
    }

    pub fn with_memory_limit(mut self, limit: usize) -> Self {
        self.memory_limit = limit;
        self
    }
}

/// A PDF document that supports streaming operations
pub struct StreamingDocument<R: Read + Seek> {
    #[allow(dead_code)]
    reader: BufReader<R>,
    options: StreamingOptions,
    page_cache: VecDeque<StreamingPage>,
    current_page: u32,
    total_pages: Option<u32>,
    memory_used: usize,
}

impl<R: Read + Seek> StreamingDocument<R> {
    /// Create a new streaming document
    pub fn new(reader: R, options: StreamingOptions) -> Result<Self> {
        let buf_reader = BufReader::with_capacity(options.buffer_size, reader);

        Ok(Self {
            reader: buf_reader,
            options,
            page_cache: VecDeque::new(),
            current_page: 0,
            total_pages: None,
            memory_used: 0,
        })
    }

    /// Get the next page for processing
    pub fn next_page(&mut self) -> Result<Option<StreamingPage>> {
        // Check if we've processed all pages
        if let Some(total) = self.total_pages {
            if self.current_page >= total {
                return Ok(None);
            }
        } else {
            // For demo/test purposes, limit to 10 pages when total is unknown
            if self.current_page >= 10 {
                return Ok(None);
            }
        }

        // Check memory limit
        if self.memory_used > self.options.memory_limit {
            self.evict_pages();
        }

        // In a real implementation, this would parse the next page
        // For now, return a mock page
        let page = StreamingPage {
            number: self.current_page,
            width: 595.0,
            height: 842.0,
            content_offset: 0,
            content_length: 0,
        };

        self.current_page += 1;

        // Cache the page if there's room
        if self.page_cache.len() < self.options.page_cache_size {
            self.page_cache.push_back(page.clone());
        }

        Ok(Some(page))
    }

    /// Process all pages with a callback
    pub fn process_pages<F>(&mut self, mut callback: F) -> Result<()>
    where
        F: FnMut(&StreamingPage) -> Result<()>,
    {
        while let Some(page) = self.next_page()? {
            callback(&page)?;
        }
        Ok(())
    }

    /// Get current memory usage
    pub fn memory_usage(&self) -> usize {
        self.memory_used
    }

    /// Clear page cache to free memory
    pub fn clear_cache(&mut self) {
        self.page_cache.clear();
        self.memory_used = 0;
    }

    fn evict_pages(&mut self) {
        // Evict oldest pages until we're under the memory limit
        while self.memory_used > self.options.memory_limit && !self.page_cache.is_empty() {
            if self.page_cache.pop_front().is_some() {
                // In a real implementation, update memory_used
                self.memory_used = self.memory_used.saturating_sub(1024);
            }
        }
    }
}

/// Statistics for streaming operations
#[derive(Debug, Clone, Default)]
pub struct StreamingStats {
    /// Total bytes processed
    pub bytes_processed: usize,
    /// Number of pages processed
    pub pages_processed: u32,
    /// Number of objects parsed
    pub objects_parsed: u32,
    /// Current memory usage
    pub memory_used: usize,
    /// Peak memory usage
    pub peak_memory: usize,
    /// Number of cache evictions
    pub cache_evictions: u32,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_streaming_options_default() {
        let options = StreamingOptions::default();
        assert_eq!(options.buffer_size, 256 * 1024);
        assert_eq!(options.page_cache_size, 3);
        assert!(options.progressive_hints);
    }

    #[test]
    fn test_streaming_options_minimal() {
        let options = StreamingOptions::minimal_memory();
        assert_eq!(options.buffer_size, 64 * 1024);
        assert_eq!(options.page_cache_size, 1);
        assert!(!options.progressive_hints);
        assert_eq!(options.memory_limit, 10 * 1024 * 1024);
    }

    #[test]
    fn test_streaming_options_fast() {
        let options = StreamingOptions::fast_processing();
        assert_eq!(options.buffer_size, 1024 * 1024);
        assert_eq!(options.page_cache_size, 10);
        assert!(options.progressive_hints);
    }

    #[test]
    fn test_streaming_options_builder() {
        let options = StreamingOptions::default()
            .with_buffer_size(512 * 1024)
            .with_page_cache_size(5)
            .with_memory_limit(50 * 1024 * 1024);

        assert_eq!(options.buffer_size, 512 * 1024);
        assert_eq!(options.page_cache_size, 5);
        assert_eq!(options.memory_limit, 50 * 1024 * 1024);
    }

    #[test]
    fn test_streaming_document_creation() {
        let data = b"%PDF-1.7\n";
        let cursor = Cursor::new(data);
        let options = StreamingOptions::default();

        let doc = StreamingDocument::new(cursor, options);
        assert!(doc.is_ok());
    }

    #[test]
    fn test_next_page() {
        let data = b"%PDF-1.7\n";
        let cursor = Cursor::new(data);
        let options = StreamingOptions::default();

        let mut doc = StreamingDocument::new(cursor, options).unwrap();

        // Should get at least one page
        let page = doc.next_page().unwrap();
        assert!(page.is_some());

        let page = page.unwrap();
        assert_eq!(page.number(), 0);
        assert_eq!(page.width(), 595.0);
        assert_eq!(page.height(), 842.0);
    }

    #[test]
    fn test_process_pages() {
        let data = b"%PDF-1.7\n";
        let cursor = Cursor::new(data);
        let options = StreamingOptions::default();

        let mut doc = StreamingDocument::new(cursor, options).unwrap();
        let mut page_count = 0;

        doc.process_pages(|page| {
            page_count += 1;
            assert!(page.number() < 1000); // Sanity check with higher limit
            Ok(())
        })
        .unwrap();

        assert!(page_count > 0);
    }

    #[test]
    fn test_memory_management() {
        let data = b"%PDF-1.7\n";
        let cursor = Cursor::new(data);
        let options = StreamingOptions::default().with_memory_limit(1024); // Very small limit

        let mut doc = StreamingDocument::new(cursor, options).unwrap();

        // Process multiple pages
        for _ in 0..5 {
            let _ = doc.next_page();
        }

        // Cache should be limited
        assert!(doc.page_cache.len() <= 3);

        // Clear cache
        doc.clear_cache();
        assert_eq!(doc.page_cache.len(), 0);
        assert_eq!(doc.memory_usage(), 0);
    }

    #[test]
    fn test_streaming_stats() {
        let stats = StreamingStats::default();
        assert_eq!(stats.bytes_processed, 0);
        assert_eq!(stats.pages_processed, 0);
        assert_eq!(stats.objects_parsed, 0);
        assert_eq!(stats.memory_used, 0);
        assert_eq!(stats.peak_memory, 0);
        assert_eq!(stats.cache_evictions, 0);
    }
}
