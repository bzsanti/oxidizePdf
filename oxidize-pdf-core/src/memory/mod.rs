//! Memory optimization module for efficient PDF handling
//!
//! This module provides memory-efficient strategies for working with large PDF files,
//! including lazy loading, streaming, and smart resource management.
//!
//! # Features
//!
//! - **Lazy Loading**: Load PDF objects only when accessed
//! - **Memory Mapping**: Use OS memory mapping for large files  
//! - **Smart Caching**: LRU cache for frequently accessed objects
//! - **Stream Processing**: Process content without loading entire file
//! - **Resource Pooling**: Reuse buffers and temporary objects
//!
//! # Example
//!
//! ```rust,no_run
//! use oxidize_pdf::memory::{LazyDocument, MemoryOptions};
//! use oxidize_pdf::parser::PdfReader;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! // Configure memory options
//! let options = MemoryOptions::default()
//!     .with_cache_size(100) // Cache up to 100 objects
//!     .with_lazy_loading(true)
//!     .with_memory_mapping(true);
//!
//! // Open document with lazy loading
//! let reader = PdfReader::open("large_document.pdf")?;
//! let document = LazyDocument::new(reader, options)?;
//!
//! // Objects are loaded only when accessed
//! let page = document.get_page(0)?; // Only loads this page
//! println!("Page loaded: {}x{}", page.width(), page.height());
//!
//! // Memory usage remains low even for large PDFs
//! # Ok(())
//! # }
//! ```

use std::sync::{Arc, RwLock};

pub mod cache;
pub mod lazy_loader;
pub mod memory_mapped;
pub mod stream_processor;

// Re-export main types
pub use cache::{LruCache, ObjectCache};
pub use lazy_loader::{LazyDocument, LazyObject};
pub use memory_mapped::{MappedReader, MemoryMappedFile};
pub use stream_processor::{ProcessingAction, ProcessingEvent, StreamProcessor, StreamingOptions};

/// Configuration options for memory optimization
#[derive(Debug, Clone)]
pub struct MemoryOptions {
    /// Enable lazy loading of objects
    pub lazy_loading: bool,
    /// Enable memory mapping for file access
    pub memory_mapping: bool,
    /// Maximum number of objects to cache
    pub cache_size: usize,
    /// Enable streaming mode for content
    pub streaming: bool,
    /// Buffer size for streaming operations
    pub buffer_size: usize,
    /// Threshold for using memory mapping (bytes)
    pub mmap_threshold: usize,
}

impl Default for MemoryOptions {
    fn default() -> Self {
        Self {
            lazy_loading: true,
            memory_mapping: true,
            cache_size: 1000,
            streaming: true,
            buffer_size: 64 * 1024,           // 64KB
            mmap_threshold: 10 * 1024 * 1024, // 10MB
        }
    }
}

impl MemoryOptions {
    /// Create options optimized for small PDFs
    pub fn small_file() -> Self {
        Self {
            lazy_loading: false,
            memory_mapping: false,
            cache_size: 0,
            streaming: false,
            buffer_size: 16 * 1024,
            mmap_threshold: usize::MAX,
        }
    }

    /// Create options optimized for large PDFs
    pub fn large_file() -> Self {
        Self {
            lazy_loading: true,
            memory_mapping: true,
            cache_size: 5000,
            streaming: true,
            buffer_size: 256 * 1024,
            mmap_threshold: 1024 * 1024, // 1MB
        }
    }

    /// Enable lazy loading
    pub fn with_lazy_loading(mut self, enabled: bool) -> Self {
        self.lazy_loading = enabled;
        self
    }

    /// Enable memory mapping
    pub fn with_memory_mapping(mut self, enabled: bool) -> Self {
        self.memory_mapping = enabled;
        self
    }

    /// Set cache size
    pub fn with_cache_size(mut self, size: usize) -> Self {
        self.cache_size = size;
        self
    }

    /// Enable streaming
    pub fn with_streaming(mut self, enabled: bool) -> Self {
        self.streaming = enabled;
        self
    }
}

/// Memory usage statistics
#[derive(Debug, Clone, Default)]
pub struct MemoryStats {
    /// Total memory allocated
    pub allocated_bytes: usize,
    /// Number of cached objects
    pub cached_objects: usize,
    /// Number of cache hits
    pub cache_hits: usize,
    /// Number of cache misses
    pub cache_misses: usize,
    /// Number of lazy loads
    pub lazy_loads: usize,
    /// Memory mapped regions
    pub mapped_regions: usize,
}

/// Memory manager for tracking and optimizing memory usage
pub struct MemoryManager {
    #[allow(dead_code)]
    options: MemoryOptions,
    stats: Arc<RwLock<MemoryStats>>,
    cache: Option<ObjectCache>,
}

impl MemoryManager {
    /// Create a new memory manager
    pub fn new(options: MemoryOptions) -> Self {
        let cache = if options.cache_size > 0 {
            Some(ObjectCache::new(options.cache_size))
        } else {
            None
        };

        Self {
            options,
            stats: Arc::new(RwLock::new(MemoryStats::default())),
            cache,
        }
    }

    /// Get memory statistics
    pub fn stats(&self) -> MemoryStats {
        self.stats.read().unwrap().clone()
    }

    /// Record a memory allocation
    pub fn record_allocation(&self, bytes: usize) {
        if let Ok(mut stats) = self.stats.write() {
            stats.allocated_bytes += bytes;
        }
    }

    /// Record a cache hit
    pub fn record_cache_hit(&self) {
        if let Ok(mut stats) = self.stats.write() {
            stats.cache_hits += 1;
        }
    }

    /// Record a cache miss
    pub fn record_cache_miss(&self) {
        if let Ok(mut stats) = self.stats.write() {
            stats.cache_misses += 1;
        }
    }

    /// Get the object cache
    pub fn cache(&self) -> Option<&ObjectCache> {
        self.cache.as_ref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_options_default() {
        let options = MemoryOptions::default();
        assert!(options.lazy_loading);
        assert!(options.memory_mapping);
        assert_eq!(options.cache_size, 1000);
        assert!(options.streaming);
        assert_eq!(options.buffer_size, 64 * 1024);
    }

    #[test]
    fn test_memory_options_small_file() {
        let options = MemoryOptions::small_file();
        assert!(!options.lazy_loading);
        assert!(!options.memory_mapping);
        assert_eq!(options.cache_size, 0);
        assert!(!options.streaming);
    }

    #[test]
    fn test_memory_options_large_file() {
        let options = MemoryOptions::large_file();
        assert!(options.lazy_loading);
        assert!(options.memory_mapping);
        assert_eq!(options.cache_size, 5000);
        assert!(options.streaming);
        assert_eq!(options.buffer_size, 256 * 1024);
    }

    #[test]
    fn test_memory_options_builder() {
        let options = MemoryOptions::default()
            .with_lazy_loading(false)
            .with_memory_mapping(false)
            .with_cache_size(500)
            .with_streaming(false);

        assert!(!options.lazy_loading);
        assert!(!options.memory_mapping);
        assert_eq!(options.cache_size, 500);
        assert!(!options.streaming);
    }

    #[test]
    fn test_memory_stats() {
        let stats = MemoryStats::default();
        assert_eq!(stats.allocated_bytes, 0);
        assert_eq!(stats.cached_objects, 0);
        assert_eq!(stats.cache_hits, 0);
        assert_eq!(stats.cache_misses, 0);
        assert_eq!(stats.lazy_loads, 0);
        assert_eq!(stats.mapped_regions, 0);
    }

    #[test]
    fn test_memory_manager() {
        let options = MemoryOptions::default();
        let manager = MemoryManager::new(options);

        // Test statistics recording
        manager.record_allocation(1024);
        manager.record_cache_hit();
        manager.record_cache_miss();

        let stats = manager.stats();
        assert_eq!(stats.allocated_bytes, 1024);
        assert_eq!(stats.cache_hits, 1);
        assert_eq!(stats.cache_misses, 1);

        // Test cache existence
        assert!(manager.cache().is_some());
    }

    #[test]
    fn test_memory_manager_no_cache() {
        let options = MemoryOptions::default().with_cache_size(0);
        let manager = MemoryManager::new(options);

        // Cache should not exist when size is 0
        assert!(manager.cache().is_none());
    }
}
