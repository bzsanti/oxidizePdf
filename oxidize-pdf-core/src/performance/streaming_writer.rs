//! Streaming PDF writer for minimal memory usage with large documents
//!
//! This module provides a streaming PDF writer that writes content incrementally
//! to avoid keeping the entire PDF in memory. Essential for processing very large
//! documents or in memory-constrained environments.
//!
//! Content is emitted one stream at a time. Object offsets and page references remain
//! in memory and grow with document size; compression also needs a temporary stream buffer.
//! Both classic xref tables and binary xref streams are supported. Font/image resource
//! registration, memory-mapped output, progress callbacks and partial-write recovery
//! are not supported. Use [`crate::Document`] when registered resources are required.
//!
//! # Example
//! ```no_run
//! use oxidize_pdf::performance::streaming_writer::{
//!     ContentStream, PageResources, StreamingOptions, StreamingPageContent, StreamingPdfWriter,
//! };
//! # fn main() -> oxidize_pdf::Result<()> {
//! let mut writer = StreamingPdfWriter::create("shapes.pdf", StreamingOptions::default())?;
//! writer.write_page_streaming(&StreamingPageContent {
//!     width: 612.0,
//!     height: 792.0,
//!     content_streams: vec![ContentStream::new(b"0 0 10 10 re f".to_vec())],
//!     resources: PageResources::default(),
//! })?;
//! writer.finalize()?;
//! # Ok(())
//! # }
//! ```

use crate::error::{PdfError, Result};
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;
use std::time::Instant;

/// Configuration for streaming PDF writer
#[derive(Debug, Clone)]
pub struct StreamingOptions {
    /// Buffer size before auto-flush (bytes)
    pub buffer_size: usize,
    /// Enable content stream compression
    pub compression: bool,
    /// Auto-flush when buffer is full
    pub auto_flush: bool,
    /// Compression level (1-9, higher = better compression but slower)
    pub compression_level: u32,
    /// Emit a binary cross-reference stream when true, or a classic xref table when false.
    pub xref_streaming: bool,
    /// Reserved: true is currently rejected because callbacks cannot be registered.
    pub progress_callbacks: bool,
    /// Write strategy to use
    pub write_strategy: WriteStrategy,
}

impl Default for StreamingOptions {
    fn default() -> Self {
        Self {
            buffer_size: 256 * 1024, // 256KB buffer
            compression: true,
            auto_flush: true,
            compression_level: 6, // Balanced compression
            xref_streaming: true,
            progress_callbacks: false,
            write_strategy: WriteStrategy::Buffered,
        }
    }
}

impl StreamingOptions {
    /// Create options optimized for minimal memory usage
    pub fn minimal_memory() -> Self {
        Self {
            buffer_size: 8 * 1024, // 8KB buffer
            compression: true,
            auto_flush: true,
            compression_level: 9, // Maximum compression
            xref_streaming: true,
            progress_callbacks: false,
            write_strategy: WriteStrategy::Direct,
        }
    }

    /// Create options optimized for maximum speed
    pub fn max_speed() -> Self {
        Self {
            buffer_size: 2 * 1024 * 1024, // 2MB buffer
            compression: false,           // Skip compression for speed
            auto_flush: false,            // Manual flush for control
            compression_level: 1,
            xref_streaming: false, // Build xref in memory
            progress_callbacks: false,
            write_strategy: WriteStrategy::Buffered,
        }
    }

    pub fn with_buffer_size(mut self, size: usize) -> Self {
        self.buffer_size = size;
        self
    }

    pub fn with_compression(mut self, enabled: bool) -> Self {
        self.compression = enabled;
        self
    }

    pub fn with_auto_flush(mut self, enabled: bool) -> Self {
        self.auto_flush = enabled;
        self
    }

    pub fn with_compression_level(mut self, level: u32) -> Self {
        self.compression_level = level.clamp(1, 9);
        self
    }

    pub fn with_write_strategy(mut self, strategy: WriteStrategy) -> Self {
        self.write_strategy = strategy;
        self
    }
}

/// Different strategies for writing data
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WriteStrategy {
    /// Buffer writes and flush periodically
    Buffered,
    /// Write directly to file with minimal buffering
    Direct,
    /// Reserved: currently rejected rather than silently using buffered writes.
    MemoryMapped,
}

/// Counts every byte accepted by the buffer, including formatted writes.
struct CountingWriter {
    inner: BufWriter<File>,
    position: u64,
}

impl Write for CountingWriter {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.position
            .checked_add(bytes.len() as u64)
            .ok_or_else(|| std::io::Error::other("PDF byte offset overflow"))?;
        let written = self.inner.write(bytes)?;
        self.position += written as u64;
        Ok(written)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.inner.flush()
    }
}

/// Streaming PDF writer that retains object offsets and page IDs, not previous page payloads.
/// A write/flush failure makes the writer unusable; discard the partial output file.
/// Finalization is one-shot. Page resources cannot currently be registered through this API.
pub struct StreamingPdfWriter {
    writer: CountingWriter,
    options: StreamingOptions,
    object_positions: HashMap<u32, u64>,
    current_object_id: u32,
    page_ids: Vec<u32>,
    last_flush_position: u64,
    stats: StreamingStats,
    start_time: Instant,
    failed: bool,
    finalized: bool,
}

impl StreamingPdfWriter {
    /// Create a writer. Unsupported options are rejected before creating/truncating the file.
    pub fn create<P: AsRef<Path>>(path: P, options: StreamingOptions) -> Result<Self> {
        if options.buffer_size == 0 || !(1..=9).contains(&options.compression_level) {
            return Err(PdfError::InvalidOperation(
                "Invalid streaming buffer size or compression level".into(),
            ));
        }
        if options.write_strategy == WriteStrategy::MemoryMapped || options.progress_callbacks {
            return Err(PdfError::InvalidOperation(
                "Memory-mapped writing and progress callbacks are not supported".into(),
            ));
        }
        let capacity = if options.write_strategy == WriteStrategy::Direct {
            0
        } else {
            options.buffer_size
        };
        let mut instance = Self {
            writer: CountingWriter {
                inner: BufWriter::with_capacity(capacity, File::create(path)?),
                position: 0,
            },
            options,
            object_positions: HashMap::new(),
            // Reserve catalog 1 and page tree 2; pages/streams start at 3.
            current_object_id: 3,
            page_ids: Vec::new(),
            last_flush_position: 0,
            stats: StreamingStats::default(),
            start_time: Instant::now(),
            failed: false,
            finalized: false,
        };
        instance
            .writer
            .write_all(b"%PDF-1.7\n%\xE2\xE3\xCF\xD3\n")?;
        instance.stats.bytes_written = instance.writer.position;
        Ok(instance)
    }

    fn ensure_open(&self) -> Result<()> {
        if self.failed || self.finalized {
            return Err(PdfError::InvalidOperation(
                "Streaming writer has failed or is already finalized".into(),
            ));
        }
        Ok(())
    }

    /// Write streams as complete objects, followed by their page dictionary.
    /// Invalid dimensions and unsupported resource references are rejected before writing.
    /// Raw content must contain valid PDF operators; this API does not validate their semantics.
    pub fn write_page_streaming(&mut self, page: &StreamingPageContent) -> Result<()> {
        self.ensure_open()?;
        if !page.width.is_finite()
            || !page.height.is_finite()
            || page.width <= 0.0
            || page.height <= 0.0
        {
            return Err(PdfError::InvalidOperation(
                "Streaming page dimensions must be finite and positive".into(),
            ));
        }
        if !page.resources.fonts.is_empty() || !page.resources.images.is_empty() {
            return Err(PdfError::InvalidOperation("Streaming page resource references are unsupported: use Document to register fonts or images".into()));
        }
        let needed = u32::try_from(page.content_streams.len())
            .ok()
            .and_then(|n| n.checked_add(1));
        if needed
            .and_then(|n| self.current_object_id.checked_add(n))
            .is_none_or(|n| n == u32::MAX)
        {
            return Err(PdfError::InvalidOperation(
                "Streaming PDF object ID limit exceeded".into(),
            ));
        }
        let start = Instant::now();
        let result = self.write_page_inner(page);
        self.stats.bytes_written = self.writer.position;
        self.stats.total_write_time += start.elapsed();
        if result.is_err() {
            self.failed = true;
        }
        result
    }

    fn write_page_inner(&mut self, page: &StreamingPageContent) -> Result<()> {
        let mut streams = Vec::with_capacity(page.content_streams.len());
        for content in &page.content_streams {
            streams.push(self.write_content_stream(content)?);
        }
        let page_id = self.start_object()?;
        writeln!(
            self.writer,
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {} {}] /Resources << >> /Contents [",
            page.width, page.height
        )?;
        for id in streams {
            writeln!(self.writer, "{id} 0 R")?;
        }
        writeln!(self.writer, "] >>\nendobj")?;
        self.page_ids.push(page_id);
        self.stats.pages_written += 1;
        if self.options.auto_flush
            && self.writer.position - self.last_flush_position >= self.options.buffer_size as u64
        {
            self.flush()?;
        }
        Ok(())
    }

    fn write_content_stream(&mut self, content: &ContentStream) -> Result<u32> {
        let compressed;
        let data = if self.options.compression {
            compressed = self.compress_data(&content.data)?;
            &compressed[..]
        } else {
            &content.data[..]
        };
        let id = self.start_object()?;
        write!(self.writer, "<< /Length {}", data.len())?;
        if self.options.compression {
            write!(self.writer, " /Filter /FlateDecode")?;
        }
        writeln!(self.writer, " >>\nstream")?;
        self.writer.write_all(data)?;
        writeln!(self.writer, "\nendstream\nendobj")?;
        Ok(id)
    }

    fn start_object(&mut self) -> Result<u32> {
        let id = self.current_object_id;
        self.current_object_id = id.checked_add(1).ok_or_else(|| {
            PdfError::InvalidOperation("Streaming PDF object ID limit exceeded".into())
        })?;
        self.start_numbered_object(id)?;
        Ok(id)
    }

    fn start_numbered_object(&mut self, id: u32) -> Result<()> {
        self.object_positions.insert(id, self.writer.position);
        writeln!(self.writer, "{id} 0 obj")?;
        self.stats.objects_written += 1;
        Ok(())
    }

    fn compress_data(&self, data: &[u8]) -> Result<Vec<u8>> {
        use flate2::{write::ZlibEncoder, Compression};
        let mut encoder =
            ZlibEncoder::new(Vec::new(), Compression::new(self.options.compression_level));
        encoder.write_all(data)?;
        Ok(encoder.finish()?)
    }

    /// Flush buffered bytes. An I/O failure permanently invalidates this writer.
    pub fn flush(&mut self) -> Result<()> {
        if self.failed {
            return self.ensure_open();
        }
        let start = Instant::now();
        if let Err(error) = self.writer.flush() {
            self.failed = true;
            return Err(error.into());
        }
        self.last_flush_position = self.writer.position;
        self.stats.bytes_written = self.writer.position;
        self.stats.flushes += 1;
        self.stats.total_flush_time += start.elapsed();
        Ok(())
    }

    /// Write the complete page tree, catalog, cross-reference section and EOF, then flush.
    /// Repeated finalization or writing pages after success returns an error without writing.
    /// On failure the file may be partial and must be discarded; this is not atomic publication.
    pub fn finalize(&mut self) -> Result<()> {
        self.ensure_open()?;
        let start = Instant::now();
        let result = self.finalize_inner();
        self.stats.bytes_written = self.writer.position;
        self.stats.total_time = self.start_time.elapsed();
        self.stats.finalization_time = start.elapsed();
        if result.is_ok() {
            self.finalized = true;
        } else {
            self.failed = true;
        }
        result
    }

    fn finalize_inner(&mut self) -> Result<()> {
        self.start_numbered_object(2)?;
        writeln!(
            self.writer,
            "<< /Type /Pages /Count {} /Kids [",
            self.page_ids.len()
        )?;
        for id in &self.page_ids {
            writeln!(self.writer, "{id} 0 R")?;
        }
        writeln!(self.writer, "] >>\nendobj")?;
        self.start_numbered_object(1)?;
        writeln!(self.writer, "<< /Type /Catalog /Pages 2 0 R >>\nendobj")?;
        let xref_position = self.writer.position;
        if self.options.xref_streaming {
            self.write_xref_stream()?;
        } else {
            self.write_xref_table()?;
        }
        writeln!(self.writer, "startxref\n{xref_position}\n%%EOF")?;
        self.flush()
    }

    fn object_offset(&self, id: u32) -> Result<u64> {
        self.object_positions
            .get(&id)
            .copied()
            .ok_or_else(|| PdfError::InvalidOperation(format!("Missing streaming PDF object {id}")))
    }

    fn write_xref_table(&mut self) -> Result<()> {
        writeln!(
            self.writer,
            "xref\n0 {}\n0000000000 65535 f ",
            self.current_object_id
        )?;
        for id in 1..self.current_object_id {
            let offset = self.object_offset(id)?;
            if offset > 9_999_999_999 {
                return Err(PdfError::InvalidOperation(
                    "Classic xref offset exceeds ten digits; enable xref_streaming".into(),
                ));
            }
            writeln!(self.writer, "{offset:010} 00000 n ")?;
        }
        writeln!(
            self.writer,
            "trailer\n<< /Size {} /Root 1 0 R >>",
            self.current_object_id
        )?;
        Ok(())
    }

    fn write_xref_stream(&mut self) -> Result<()> {
        self.start_object()?; // The xref stream includes its own recorded offset.
        let count = self.current_object_id;
        writeln!(
            self.writer,
            "<< /Type /XRef /Size {count} /Root 1 0 R /W [1 8 2] /Length {} >>\nstream",
            u64::from(count) * 11
        )?;
        self.writer
            .write_all(&[0, 0, 0, 0, 0, 0, 0, 0, 0, 255, 255])?;
        for id in 1..count {
            let offset = self.object_offset(id)?;
            self.writer.write_all(&[1])?;
            self.writer.write_all(&offset.to_be_bytes())?;
            self.writer.write_all(&[0, 0])?;
        }
        writeln!(self.writer, "\nendstream\nendobj")?;
        Ok(())
    }

    /// Statistics for completed operations. bytes_written counts bytes accepted by the buffer.
    /// Successful finalization flushes those bytes; it does not promise fsync durability.
    pub fn stats(&self) -> &StreamingStats {
        &self.stats
    }

    /// Actual buffer occupancy; direct writing uses no buffer.
    pub fn buffer_usage_percent(&self) -> f64 {
        let capacity = self.writer.inner.capacity();
        if capacity == 0 {
            0.0
        } else {
            self.writer.inner.buffer().len() as f64 / capacity as f64 * 100.0
        }
    }

    /// Approximate buffer and index payload, excluding allocator overhead, caller data and compression scratch.
    /// Indices grow with object/page count; this is not a constant-memory or RSS bound.
    pub fn memory_usage(&self) -> usize {
        self.writer.inner.capacity()
            + self.object_positions.len()
                * (std::mem::size_of::<u32>() + std::mem::size_of::<u64>())
            + self.page_ids.capacity() * std::mem::size_of::<u32>()
            + std::mem::size_of::<StreamingStats>()
    }
}

/// Content for a page to be written
#[derive(Debug, Clone)]
pub struct StreamingPageContent {
    pub width: f64,
    pub height: f64,
    pub content_streams: Vec<ContentStream>,
    pub resources: PageResources,
}

/// A content stream with its data
#[derive(Debug, Clone)]
pub struct ContentStream {
    pub data: Vec<u8>,
}

impl ContentStream {
    pub fn new(data: Vec<u8>) -> Self {
        Self { data }
    }

    pub fn from_text(text: &str, x: f64, y: f64, font: &str, size: f64) -> Self {
        let content = format!(
            "BT\n/{} {} Tf\n{} {} Td\n({}) Tj\nET\n",
            font, size, x, y, text
        );
        Self::new(content.into_bytes())
    }
}

/// Resources referenced by a page. Nonempty maps are currently rejected by this writer;
/// object IDs cannot be registered through this API. Use Document for fonts/images.
#[derive(Debug, Clone, Default)]
pub struct PageResources {
    pub fonts: HashMap<String, u32>,
    pub images: HashMap<String, u32>,
}

/// Statistics about streaming operations
#[derive(Debug, Clone, Default)]
pub struct StreamingStats {
    pub pages_written: u32,
    pub objects_written: u32,
    pub bytes_written: u64,
    pub flushes: u32,
    pub total_time: std::time::Duration,
    pub total_write_time: std::time::Duration,
    pub total_flush_time: std::time::Duration,
    pub finalization_time: std::time::Duration,
}

impl StreamingStats {
    /// Calculate pages per second
    pub fn pages_per_second(&self) -> f64 {
        if self.total_time.as_secs_f64() == 0.0 {
            return 0.0;
        }
        self.pages_written as f64 / self.total_time.as_secs_f64()
    }

    /// Calculate average write time per page
    pub fn average_write_time_per_page(&self) -> std::time::Duration {
        if self.pages_written == 0 {
            return std::time::Duration::ZERO;
        }
        self.total_write_time / self.pages_written
    }

    /// Calculate write throughput (MB/s)
    pub fn write_throughput_mbps(&self) -> f64 {
        if self.total_time.as_secs_f64() == 0.0 {
            return 0.0;
        }
        let mb = self.bytes_written as f64 / (1024.0 * 1024.0);
        mb / self.total_time.as_secs_f64()
    }

    /// Get human-readable summary
    pub fn summary(&self) -> String {
        format!(
            "Streaming Writer Stats:\n\
             - Pages Written: {}\n\
             - Objects Written: {}\n\
             - Bytes Written: {:.1} MB\n\
             - Flushes: {}\n\
             - Total Time: {:.2}s\n\
             - Write Time: {:.2}s\n\
             - Flush Time: {:.2}s\n\
             - Finalization Time: {:.2}s\n\
             - Pages/Second: {:.1}\n\
             - Throughput: {:.1} MB/s\n\
             - Avg Time/Page: {:.2}ms",
            self.pages_written,
            self.objects_written,
            self.bytes_written as f64 / (1024.0 * 1024.0),
            self.flushes,
            self.total_time.as_secs_f64(),
            self.total_write_time.as_secs_f64(),
            self.total_flush_time.as_secs_f64(),
            self.finalization_time.as_secs_f64(),
            self.pages_per_second(),
            self.write_throughput_mbps(),
            self.average_write_time_per_page().as_secs_f64() * 1000.0
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_streaming_options_default() {
        let options = StreamingOptions::default();
        assert_eq!(options.buffer_size, 256 * 1024);
        assert!(options.compression);
        assert!(options.auto_flush);
    }

    #[test]
    fn test_streaming_options_minimal_memory() {
        let options = StreamingOptions::minimal_memory();
        assert_eq!(options.buffer_size, 8 * 1024);
        assert!(options.compression);
        assert_eq!(options.compression_level, 9);
        assert_eq!(options.write_strategy, WriteStrategy::Direct);
    }

    #[test]
    fn test_streaming_options_max_speed() {
        let options = StreamingOptions::max_speed();
        assert_eq!(options.buffer_size, 2 * 1024 * 1024);
        assert!(!options.compression);
        assert!(!options.auto_flush);
    }

    #[test]
    fn test_streaming_writer_creation() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.pdf");

        let options = StreamingOptions::default();
        let writer = StreamingPdfWriter::create(&file_path, options);

        assert!(writer.is_ok());
        assert!(file_path.exists());
    }

    #[test]
    fn test_content_stream_creation() {
        let stream = ContentStream::new(b"Hello".to_vec());
        assert_eq!(stream.data, b"Hello");

        let text_stream = ContentStream::from_text("Hello", 100.0, 200.0, "F1", 12.0);
        assert!(String::from_utf8_lossy(&text_stream.data).contains("Hello"));
        assert!(String::from_utf8_lossy(&text_stream.data).contains("100 200 Td"));
    }

    #[test]
    fn test_page_content() {
        let mut resources = PageResources::default();
        resources.fonts.insert("F1".to_string(), 1);

        let content = StreamingPageContent {
            width: 595.0,
            height: 842.0,
            content_streams: vec![ContentStream::from_text("Hello", 100.0, 200.0, "F1", 12.0)],
            resources,
        };

        assert_eq!(content.width, 595.0);
        assert_eq!(content.content_streams.len(), 1);
        assert!(content.resources.fonts.contains_key("F1"));
    }

    #[test]
    fn test_streaming_stats() {
        let mut stats = StreamingStats::default();
        stats.pages_written = 10;
        stats.total_time = std::time::Duration::from_secs(2);
        stats.bytes_written = 1024 * 1024; // 1MB

        assert_eq!(stats.pages_per_second(), 5.0);
        assert_eq!(stats.write_throughput_mbps(), 0.5);
    }

    #[test]
    fn test_buffer_usage_calculation() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.pdf");

        let options = StreamingOptions::default().with_buffer_size(1000);
        let writer = StreamingPdfWriter::create(&file_path, options).unwrap();

        // Initially should be low usage
        assert!(writer.buffer_usage_percent() < 10.0);
    }

    #[test]
    fn test_write_strategy_enum() {
        assert_eq!(WriteStrategy::Buffered, WriteStrategy::Buffered);
        assert_ne!(WriteStrategy::Buffered, WriteStrategy::Direct);
        assert_ne!(WriteStrategy::Direct, WriteStrategy::MemoryMapped);
    }

    #[test]
    fn test_streaming_options_builder() {
        let options = StreamingOptions::default()
            .with_buffer_size(128 * 1024)
            .with_compression(false)
            .with_auto_flush(false)
            .with_compression_level(3)
            .with_write_strategy(WriteStrategy::Direct);

        assert_eq!(options.buffer_size, 128 * 1024);
        assert!(!options.compression);
        assert!(!options.auto_flush);
        assert_eq!(options.compression_level, 3);
        assert_eq!(options.write_strategy, WriteStrategy::Direct);
    }

    #[test]
    fn test_compression_level_clamping() {
        let options = StreamingOptions::default().with_compression_level(15);
        assert_eq!(options.compression_level, 9); // Should be clamped to 9

        let options = StreamingOptions::default().with_compression_level(0);
        assert_eq!(options.compression_level, 1); // Should be clamped to 1
    }
}
