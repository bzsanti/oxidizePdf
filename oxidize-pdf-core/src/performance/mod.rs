//! Optional PDF generation using the core page model and writer.
//!
//! `add_pdf_page` accepts real pages, including their fonts and images.
//! `generate_pages` builds pages through a caller callback, with bounded Rayon
//! workers when both requested and compiled; otherwise it runs serially.
//! Saving streams encoded output to a temporary file or buffers it in memory,
//! then publishes the completed PDF. Source pages remain resident in either mode.
//! No universal speedup or process-wide memory bound is promised.
//!
//! ```rust
//! use oxidize_pdf::{Page, text::Font};
//! use oxidize_pdf::performance::{HighPerformanceDocument, PerformanceOptions};
//! let mut doc = HighPerformanceDocument::new(PerformanceOptions::default())?;
//! let mut page = Page::a4();
//! page.text().set_font(Font::Helvetica, 12.0).at(40.0, 700.0).write("Real PDF content")?;
//! doc.add_pdf_page(page)?;
//! # let dir = tempfile::tempdir()?;
//! doc.save(dir.path().join("report.pdf"))?;
//! # Ok::<(), oxidize_pdf::PdfError>(())
//! ```

use crate::error::Result;
mod generation;

fn validate_page(page: &crate::Page) -> Result<()> {
    if !page.width().is_finite()
        || !page.height().is_finite()
        || page.width() <= 0.0
        || page.height() <= 0.0
    {
        return Err(crate::PdfError::InvalidOperation(
            "Page dimensions must be finite and positive".into(),
        ));
    }
    Ok(())
}

pub mod compression;
pub mod memory_pool;
pub mod metrics;
pub mod parallel_generation;
pub mod resource_pool;
pub mod streaming_writer;

// Re-export main types
pub use compression::{CompressionStats, CompressionStrategy, ContentType, IntelligentCompressor};
pub use memory_pool::{MemoryPool, MemoryPoolStats, PooledBuffer};
pub use metrics::{Operation, OperationStats, PerformanceMetrics, PerformanceMonitor};
pub use parallel_generation::{PageProcessor, ParallelGenerationOptions, ParallelPageGenerator};
pub use resource_pool::{FontResource, ImageResource, PatternResource, ResourceKey, ResourcePool};
pub use streaming_writer::{StreamingOptions, StreamingPdfWriter, StreamingStats, WriteStrategy};

/// High-performance PDF document optimized for speed and memory efficiency
pub struct HighPerformanceDocument {
    options: PerformanceOptions,
    resource_pool: ResourcePool,
    memory_pool: MemoryPool,
    metrics: PerformanceMonitor,
    pages: Vec<crate::Page>,
    font_data: std::collections::BTreeMap<String, Vec<u8>>,
}

/// Legacy page descriptor. Without content_refs it represents a blank page.
/// Use HighPerformanceDocument::add_pdf_page to supply real content and resources.
#[derive(Debug, Clone)]
pub struct PerformancePage {
    pub index: u32,
    pub width: f64,
    pub height: f64,
    pub content_refs: Vec<ResourceKey>,
    pub estimated_size: usize,
}

/// Options for high-performance operations
#[derive(Debug, Clone)]
pub struct PerformanceOptions {
    /// Enable parallel page generation
    pub parallel_generation: bool,
    /// Maximum number of worker threads
    pub max_threads: usize,
    /// Use the core writer's shared resources. Disabling this is unsupported.
    pub resource_deduplication: bool,
    /// Write encoded output directly to a buffered file instead of a full-PDF buffer.
    /// Source pages and writer resource tables remain resident.
    pub streaming_writer: bool,
    /// Buffer size for streaming (bytes)
    pub stream_buffer_size: usize,
    /// Enable core stream compression (legacy field name; not an adaptive codec).
    pub intelligent_compression: bool,
    /// Reuse full-PDF buffers in buffered output mode.
    pub memory_pooling: bool,
    /// Maximum retained buffer capacity in buffered mode; not a process memory cap.
    pub memory_pool_size: usize,
    /// Enable performance metrics collection
    pub collect_metrics: bool,
}

impl Default for PerformanceOptions {
    fn default() -> Self {
        Self {
            parallel_generation: true,
            max_threads: num_cpus::get().min(8),
            resource_deduplication: true,
            streaming_writer: true,
            stream_buffer_size: 1024 * 1024, // 1MB
            intelligent_compression: true,
            memory_pooling: true,
            memory_pool_size: 16 * 1024 * 1024, // 16MB
            collect_metrics: true,
        }
    }
}

impl PerformanceOptions {
    /// Create options optimized for maximum speed (uses more memory)
    pub fn max_speed() -> Self {
        Self {
            parallel_generation: true,
            max_threads: num_cpus::get(),
            resource_deduplication: true,
            streaming_writer: false,             // Keep in memory for speed
            stream_buffer_size: 4 * 1024 * 1024, // 4MB
            intelligent_compression: false,      // Skip compression for speed
            memory_pooling: true,
            memory_pool_size: 64 * 1024 * 1024, // 64MB
            collect_metrics: false,             // Skip metrics for speed
        }
    }

    /// Create options optimized for minimum memory usage
    pub fn min_memory() -> Self {
        Self {
            parallel_generation: false, // Less parallelism = less memory
            max_threads: 2,
            resource_deduplication: true,
            streaming_writer: true,
            stream_buffer_size: 64 * 1024, // 64KB
            intelligent_compression: true,
            memory_pooling: false, // Avoid memory pooling overhead
            memory_pool_size: 0,
            collect_metrics: false,
        }
    }

    /// Create options balanced between speed and memory
    pub fn balanced() -> Self {
        Self::default()
    }

    pub fn with_parallel_generation(mut self, enabled: bool) -> Self {
        self.parallel_generation = enabled;
        self
    }

    pub fn with_resource_deduplication(mut self, enabled: bool) -> Self {
        self.resource_deduplication = enabled;
        self
    }

    pub fn with_streaming_writer(mut self, enabled: bool) -> Self {
        self.streaming_writer = enabled;
        self
    }

    pub fn with_max_threads(mut self, threads: usize) -> Self {
        self.max_threads = threads.max(1);
        self
    }

    pub fn with_stream_buffer_size(mut self, size: usize) -> Self {
        self.stream_buffer_size = size;
        self
    }

    pub fn with_memory_pool_size(mut self, size: usize) -> Self {
        self.memory_pool_size = size;
        self
    }
}

impl HighPerformanceDocument {
    /// Create a new high-performance document
    pub fn new(options: PerformanceOptions) -> Result<Self> {
        if options.max_threads == 0 || options.stream_buffer_size == 0 {
            return Err(crate::PdfError::InvalidOperation(
                "Generation requires a positive thread count and a positive output buffer size"
                    .into(),
            ));
        }
        if !options.resource_deduplication {
            return Err(crate::PdfError::InvalidOperation(
                "Disabling core writer resource sharing is unsupported".into(),
            ));
        }
        let resource_pool = ResourcePool::new();
        let memory_pool = if options.memory_pooling {
            MemoryPool::new(options.memory_pool_size)
        } else {
            MemoryPool::disabled()
        };
        let metrics = if options.collect_metrics {
            PerformanceMonitor::new()
        } else {
            PerformanceMonitor::disabled()
        };

        Ok(Self {
            options,
            resource_pool,
            memory_pool,
            metrics,
            pages: Vec::new(),
            font_data: std::collections::BTreeMap::new(),
        })
    }

    /// Add a legacy blank-page descriptor in insertion order. Unresolved resource
    /// references are rejected; descriptors do not contain rendering instructions.
    pub fn add_page(&mut self, page: PerformancePage) -> Result<()> {
        if !page.content_refs.is_empty() {
            return Err(crate::PdfError::InvalidOperation("Page descriptors cannot resolve content_refs; use add_pdf_page with owned content/resources".into()));
        }
        self.add_pdf_page(crate::Page::new(page.width, page.height))
    }

    /// Get performance statistics
    pub fn performance_stats(&self) -> PerformanceStats {
        PerformanceStats {
            total_pages: self.pages.len(),
            resource_pool_stats: self.resource_pool.stats(),
            memory_pool_stats: self.memory_pool.stats(),
            compression_stats: CompressionStats::default(),
            operation_stats: self
                .metrics
                .get_stats()
                .operation_stats
                .into_values()
                .collect(),
        }
    }
}

/// Comprehensive performance statistics
#[derive(Debug, Clone)]
pub struct PerformanceStats {
    pub total_pages: usize,
    pub resource_pool_stats: resource_pool::ResourcePoolStats,
    pub memory_pool_stats: MemoryPoolStats,
    pub compression_stats: CompressionStats,
    pub operation_stats: Vec<OperationStats>,
}

impl PerformanceStats {
    /// Calculate overall performance score (0-100)
    pub fn performance_score(&self) -> f64 {
        let mut score = 100.0;

        // Penalize for low resource deduplication
        let dedup_ratio = self.resource_pool_stats.deduplication_ratio();
        if dedup_ratio < 0.3 {
            score -= (0.3 - dedup_ratio) * 50.0;
        }

        // Bonus for high compression ratio
        let compression_ratio = self.compression_stats.compression_ratio();
        if compression_ratio > 0.5 {
            score += (compression_ratio - 0.5) * 20.0;
        }

        // Penalize for high memory usage
        let memory_efficiency = self.memory_pool_stats.efficiency();
        if memory_efficiency < 0.8 {
            score -= (0.8 - memory_efficiency) * 30.0;
        }

        score.max(0.0).min(100.0)
    }

    /// Get human-readable performance summary
    pub fn summary(&self) -> String {
        format!(
            "Performance Summary:\n\
             - Total Pages: {}\n\
             - Resource Deduplication: {:.1}%\n\
             - Compression Ratio: {:.1}%\n\
             - Memory Pool Efficiency: {:.1}%\n\
             - Overall Score: {:.1}/100",
            self.total_pages,
            self.resource_pool_stats.deduplication_ratio() * 100.0,
            self.compression_stats.compression_ratio() * 100.0,
            self.memory_pool_stats.efficiency() * 100.0,
            self.performance_score()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_performance_options_default() {
        let options = PerformanceOptions::default();
        assert!(options.parallel_generation);
        assert!(options.resource_deduplication);
        assert!(options.streaming_writer);
        assert!(options.intelligent_compression);
        assert!(options.memory_pooling);
    }

    #[test]
    fn test_performance_options_max_speed() {
        let options = PerformanceOptions::max_speed();
        assert!(options.parallel_generation);
        assert!(!options.streaming_writer); // Keep in memory for speed
        assert!(!options.intelligent_compression); // Skip compression
        assert!(!options.collect_metrics); // Skip metrics
    }

    #[test]
    fn test_performance_options_min_memory() {
        let options = PerformanceOptions::min_memory();
        assert!(!options.parallel_generation);
        assert!(options.streaming_writer);
        assert!(options.intelligent_compression);
        assert!(!options.memory_pooling);
    }

    #[test]
    fn test_performance_options_builder() {
        let options = PerformanceOptions::default()
            .with_parallel_generation(false)
            .with_max_threads(4)
            .with_stream_buffer_size(512 * 1024);

        assert!(!options.parallel_generation);
        assert_eq!(options.max_threads, 4);
        assert_eq!(options.stream_buffer_size, 512 * 1024);
    }

    #[test]
    fn test_high_performance_document_creation() {
        let options = PerformanceOptions::default();
        let doc = HighPerformanceDocument::new(options);
        assert!(doc.is_ok());
    }

    #[test]
    fn test_performance_page() {
        let page = PerformancePage {
            index: 0,
            width: 595.0,
            height: 842.0,
            content_refs: vec![],
            estimated_size: 1024,
        };

        assert_eq!(page.index, 0);
        assert_eq!(page.width, 595.0);
        assert_eq!(page.height, 842.0);
        assert_eq!(page.estimated_size, 1024);
    }

    #[test]
    fn test_add_page() {
        let options = PerformanceOptions::default();
        let mut doc = HighPerformanceDocument::new(options).unwrap();

        let page = PerformancePage {
            index: 0,
            width: 595.0,
            height: 842.0,
            content_refs: vec![],
            estimated_size: 1024,
        };

        let result = doc.add_page(page);
        assert!(result.is_ok());
        assert_eq!(doc.pages.len(), 1);
    }
}
