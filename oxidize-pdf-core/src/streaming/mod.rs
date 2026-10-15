//! Streaming support for incremental PDF processing
//!
//! This module provides advanced streaming capabilities for processing PDFs
//! using seekable input. Page adapters materialize content/text one page at a time;
//! reader caches and caller-retained pages are outside the page cache budget.
//!
//! # Features
//!
//! - **Incremental Parsing**: Parse PDF objects as they're needed
//! - **Page Streaming**: Process pages one at a time
//! - **Content Stream Processing**: Handle content streams in chunks
//! - **Progressive Text Extraction**: Extract text as it's encountered
//! - **Page Cache Budget**: Configurable limits for retained page payloads
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
    /// Maximum combined decoded page content size
    pub max_content_stream_size: usize,
    /// Reserved for progressive rendering hints; currently unused
    pub progressive_hints: bool,
    /// Budget for retained page content/text in this adapter cache (bytes)
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

/// Page-at-a-time document access using the seekable PDF reader.
/// Content and text are materialized per page. `memory_usage` measures only this
/// adapter's retained page payloads, not reader caches or caller-owned pages.
pub struct StreamingDocument<R: Read + Seek> {
    streamer: PageStreamer<BufReader<R>>,
    options: StreamingOptions,
    page_cache: VecDeque<StreamingPage>,
    memory_used: usize,
}
impl<R: Read + Seek> StreamingDocument<R> {
    pub fn new(reader: R, options: StreamingOptions) -> Result<Self> {
        let mut streamer = PageStreamer::with_content_limit(
            BufReader::with_capacity(options.buffer_size.max(1), reader),
            options.max_content_stream_size,
        );
        streamer.initialize()?;
        Ok(Self {
            streamer,
            options,
            page_cache: VecDeque::new(),
            memory_used: 0,
        })
    }
    pub fn next_page(&mut self) -> Result<Option<StreamingPage>> {
        let Some(page) = self.streamer.next()? else {
            return Ok(None);
        };
        let size = page.retained_bytes();
        while !self.page_cache.is_empty()
            && (self.page_cache.len() >= self.options.page_cache_size
                || size > self.options.memory_limit.saturating_sub(self.memory_used))
        {
            if let Some(old) = self.page_cache.pop_front() {
                self.memory_used -= old.retained_bytes();
            }
        }
        if self.options.page_cache_size > 0
            && size <= self.options.memory_limit.saturating_sub(self.memory_used)
        {
            self.memory_used += size;
            self.page_cache.push_back(page.clone());
        }
        Ok(Some(page))
    }
    pub fn process_pages<F>(&mut self, mut callback: F) -> Result<()>
    where
        F: FnMut(&StreamingPage) -> Result<()>,
    {
        while let Some(page) = self.next_page()? {
            callback(&page)?;
        }
        Ok(())
    }
    /// Bytes of content/text retained in this adapter's page cache.
    pub fn memory_usage(&self) -> usize {
        self.memory_used
    }
    pub fn clear_cache(&mut self) {
        self.page_cache.clear();
        self.memory_used = 0;
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
