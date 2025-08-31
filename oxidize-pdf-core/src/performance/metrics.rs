//! Performance metrics collection and monitoring system
//!
//! This module provides comprehensive performance monitoring for all PDF operations,
//! enabling real-time performance analysis and optimization.
//!
//! # Key Features
//! - **Operation Tracking**: Monitor specific operations (parsing, generation, compression)
//! - **Real-time Metrics**: Live performance data collection
//! - **Histograms**: Detailed timing distribution analysis
//! - **Memory Tracking**: Monitor memory usage patterns
//! - **Alerting**: Configurable performance alerts
//! - **Export**: Export metrics for external monitoring systems
//!
//! # Example
//! ```rust
//! use oxidize_pdf::performance::{PerformanceMonitor, Operation};
//!
//! let monitor = PerformanceMonitor::new();
//!
//! // Start monitoring an operation
//! let token = monitor.start_operation(Operation::PdfGeneration);
//!
//! // ... perform PDF generation ...
//!
//! // End monitoring and get metrics
//! let duration = monitor.end_operation(token);
//! println!("PDF generation took: {:?}", duration);
//!
//! // Get comprehensive stats
//! let stats = monitor.get_stats();
//! println!("Average generation time: {:?}", stats.average_duration(Operation::PdfGeneration));
//! ```

use std::collections::HashMap;
use std::sync::{Arc, Mutex, RwLock};
use std::thread;
use std::time::{Duration, Instant, SystemTime};

/// Performance monitor for tracking PDF operations
pub struct PerformanceMonitor {
    active_operations: Arc<Mutex<HashMap<u64, ActiveOperation>>>,
    completed_operations: Arc<RwLock<Vec<CompletedOperation>>>,
    operation_stats: Arc<RwLock<HashMap<Operation, OperationStats>>>,
    next_token: Arc<Mutex<u64>>,
    start_time: SystemTime,
    enabled: bool,
}

/// Types of operations that can be monitored
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Operation {
    /// PDF document parsing
    PdfParsing,
    /// PDF document generation
    PdfGeneration,
    /// Page processing
    PageProcessing,
    /// Image processing
    ImageProcessing,
    /// Font processing
    FontProcessing,
    /// Content compression
    ContentCompression,
    /// Resource deduplication
    ResourceDeduplication,
    /// Memory allocation
    MemoryAllocation,
    /// File I/O operations
    FileIO,
    /// Parallel processing coordination
    ParallelProcessing,
    /// Custom operation
    Custom(String),
}

impl Operation {
    /// Get human-readable name for the operation
    pub fn name(&self) -> &str {
        match self {
            Operation::PdfParsing => "PDF Parsing",
            Operation::PdfGeneration => "PDF Generation",
            Operation::PageProcessing => "Page Processing",
            Operation::ImageProcessing => "Image Processing",
            Operation::FontProcessing => "Font Processing",
            Operation::ContentCompression => "Content Compression",
            Operation::ResourceDeduplication => "Resource Deduplication",
            Operation::MemoryAllocation => "Memory Allocation",
            Operation::FileIO => "File I/O",
            Operation::ParallelProcessing => "Parallel Processing",
            Operation::Custom(name) => name,
        }
    }

    /// Get the category of this operation
    pub fn category(&self) -> OperationCategory {
        match self {
            Operation::PdfParsing | Operation::PdfGeneration => OperationCategory::Core,
            Operation::PageProcessing | Operation::ImageProcessing | Operation::FontProcessing => {
                OperationCategory::Processing
            }
            Operation::ContentCompression | Operation::ResourceDeduplication => {
                OperationCategory::Optimization
            }
            Operation::MemoryAllocation | Operation::ParallelProcessing => {
                OperationCategory::System
            }
            Operation::FileIO => OperationCategory::IO,
            Operation::Custom(_) => OperationCategory::Custom,
        }
    }
}

/// Categories of operations
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OperationCategory {
    Core,
    Processing,
    Optimization,
    System,
    IO,
    Custom,
}

/// Token representing an active operation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OperationToken(u64);

/// Active operation being monitored
#[derive(Debug)]
struct ActiveOperation {
    operation: Operation,
    start_time: Instant,
    thread_id: thread::ThreadId,
    memory_start: usize,
}

/// Completed operation with full metrics
#[derive(Debug, Clone)]
pub struct CompletedOperation {
    pub operation: Operation,
    pub duration: Duration,
    pub thread_id: thread::ThreadId,
    pub memory_used: usize,
    pub timestamp: SystemTime,
}

impl PerformanceMonitor {
    /// Create a new performance monitor
    pub fn new() -> Self {
        Self {
            active_operations: Arc::new(Mutex::new(HashMap::new())),
            completed_operations: Arc::new(RwLock::new(Vec::new())),
            operation_stats: Arc::new(RwLock::new(HashMap::new())),
            next_token: Arc::new(Mutex::new(1)),
            start_time: SystemTime::now(),
            enabled: true,
        }
    }

    /// Create a disabled monitor (no overhead)
    pub fn disabled() -> Self {
        Self {
            active_operations: Arc::new(Mutex::new(HashMap::new())),
            completed_operations: Arc::new(RwLock::new(Vec::new())),
            operation_stats: Arc::new(RwLock::new(HashMap::new())),
            next_token: Arc::new(Mutex::new(1)),
            start_time: SystemTime::now(),
            enabled: false,
        }
    }

    /// Start monitoring an operation
    pub fn start_operation(&self, operation: Operation) -> OperationToken {
        if !self.enabled {
            return OperationToken(0);
        }

        let mut next_token = self.next_token.lock().unwrap();
        let token = OperationToken(*next_token);
        *next_token += 1;
        drop(next_token);

        let active_op = ActiveOperation {
            operation,
            start_time: Instant::now(),
            thread_id: thread::current().id(),
            memory_start: self.estimate_memory_usage(),
        };

        self.active_operations
            .lock()
            .unwrap()
            .insert(token.0, active_op);
        token
    }

    /// End monitoring an operation and return its duration
    pub fn end_operation(&self, token: OperationToken) -> Duration {
        if !self.enabled || token.0 == 0 {
            return Duration::ZERO;
        }

        let active_op = self.active_operations.lock().unwrap().remove(&token.0);

        if let Some(active_op) = active_op {
            let duration = active_op.start_time.elapsed();
            let memory_end = self.estimate_memory_usage();
            let memory_used = memory_end.saturating_sub(active_op.memory_start);

            let completed_op = CompletedOperation {
                operation: active_op.operation,
                duration,
                thread_id: active_op.thread_id,
                memory_used,
                timestamp: SystemTime::now(),
            };

            // Store completed operation
            self.completed_operations
                .write()
                .unwrap()
                .push(completed_op.clone());

            // Update operation statistics
            let mut stats = self.operation_stats.write().unwrap();
            let op_stats = stats
                .entry(active_op.operation)
                .or_insert_with(|| OperationStats::new(active_op.operation));
            op_stats.add_measurement(duration, memory_used);

            duration
        } else {
            Duration::ZERO
        }
    }

    /// Time a block of code
    pub fn time_operation<F, R>(&self, operation: Operation, f: F) -> (R, Duration)
    where
        F: FnOnce() -> R,
    {
        let token = self.start_operation(operation);
        let result = f();
        let duration = self.end_operation(token);
        (result, duration)
    }

    /// Get current performance statistics
    pub fn get_stats(&self) -> PerformanceMetrics {
        let operation_stats = self.operation_stats.read().unwrap().clone();
        let completed_ops = self.completed_operations.read().unwrap();

        let uptime = self.start_time.elapsed().unwrap_or(Duration::ZERO);
        let total_operations = completed_ops.len();
        let active_operations = self.active_operations.lock().unwrap().len();

        // Calculate category statistics
        let mut category_stats = HashMap::new();
        for (operation, stats) in &operation_stats {
            let category = operation.category();
            let cat_stats = category_stats
                .entry(category)
                .or_insert_with(CategoryStats::default);
            cat_stats.total_operations += stats.count;
            cat_stats.total_duration += stats.total_duration;
            cat_stats.total_memory += stats.total_memory;
        }

        PerformanceMetrics {
            uptime,
            total_operations,
            active_operations,
            operation_stats,
            category_stats,
            operations_per_second: if uptime.as_secs_f64() > 0.0 {
                total_operations as f64 / uptime.as_secs_f64()
            } else {
                0.0
            },
        }
    }

    /// Get statistics for a specific operation
    pub fn get_operation_stats(&self, operation: Operation) -> Option<OperationStats> {
        self.operation_stats
            .read()
            .unwrap()
            .get(&operation)
            .cloned()
    }

    /// Get recent operations (last N)
    pub fn get_recent_operations(&self, limit: usize) -> Vec<CompletedOperation> {
        let completed_ops = self.completed_operations.read().unwrap();
        let start_idx = completed_ops.len().saturating_sub(limit);
        completed_ops[start_idx..].to_vec()
    }

    /// Clear all collected metrics
    pub fn clear(&self) {
        self.active_operations.lock().unwrap().clear();
        self.completed_operations.write().unwrap().clear();
        self.operation_stats.write().unwrap().clear();
    }

    /// Check if monitoring is enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Enable or disable monitoring
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    /// Export metrics in Prometheus format
    pub fn export_prometheus(&self) -> String {
        let stats = self.get_stats();
        let mut output = String::new();

        output.push_str("# HELP oxidize_pdf_operations_total Total number of operations\n");
        output.push_str("# TYPE oxidize_pdf_operations_total counter\n");

        for (operation, op_stats) in &stats.operation_stats {
            output.push_str(&format!(
                "oxidize_pdf_operations_total{{operation=\"{}\"}} {}\n",
                operation.name(),
                op_stats.count
            ));
        }

        output.push_str("\n# HELP oxidize_pdf_operation_duration_seconds Duration of operations\n");
        output.push_str("# TYPE oxidize_pdf_operation_duration_seconds histogram\n");

        for (operation, op_stats) in &stats.operation_stats {
            let avg_duration = op_stats.average_duration().as_secs_f64();
            output.push_str(&format!(
                "oxidize_pdf_operation_duration_seconds{{operation=\"{}\"}} {}\n",
                operation.name(),
                avg_duration
            ));
        }

        output
    }

    /// Estimate current memory usage (simplified implementation)
    fn estimate_memory_usage(&self) -> usize {
        // This is a simplified memory estimation
        // In a real implementation, you might use system APIs or memory profiling
        let active_count = self.active_operations.lock().unwrap().len();
        let completed_count = self.completed_operations.read().unwrap().len();

        // Rough estimate: 1KB per active operation, 100 bytes per completed
        active_count * 1024 + completed_count * 100
    }
}

impl Default for PerformanceMonitor {
    fn default() -> Self {
        Self::new()
    }
}

/// Statistics for a specific operation type
#[derive(Debug, Clone)]
pub struct OperationStats {
    pub operation: Operation,
    pub count: u64,
    pub total_duration: Duration,
    pub min_duration: Duration,
    pub max_duration: Duration,
    pub total_memory: usize,
    pub min_memory: usize,
    pub max_memory: usize,
    pub histogram: DurationHistogram,
}

impl OperationStats {
    fn new(operation: Operation) -> Self {
        Self {
            operation,
            count: 0,
            total_duration: Duration::ZERO,
            min_duration: Duration::MAX,
            max_duration: Duration::ZERO,
            total_memory: 0,
            min_memory: usize::MAX,
            max_memory: 0,
            histogram: DurationHistogram::new(),
        }
    }

    fn add_measurement(&mut self, duration: Duration, memory: usize) {
        self.count += 1;
        self.total_duration += duration;
        self.min_duration = self.min_duration.min(duration);
        self.max_duration = self.max_duration.max(duration);

        self.total_memory += memory;
        self.min_memory = self.min_memory.min(memory);
        self.max_memory = self.max_memory.max(memory);

        self.histogram.add_sample(duration);
    }

    /// Calculate average duration
    pub fn average_duration(&self) -> Duration {
        if self.count == 0 {
            Duration::ZERO
        } else {
            self.total_duration / self.count as u32
        }
    }

    /// Calculate average memory usage
    pub fn average_memory(&self) -> usize {
        if self.count == 0 {
            0
        } else {
            self.total_memory / self.count as usize
        }
    }

    /// Calculate operations per second
    pub fn operations_per_second(&self, uptime: Duration) -> f64 {
        if uptime.as_secs_f64() == 0.0 {
            0.0
        } else {
            self.count as f64 / uptime.as_secs_f64()
        }
    }

    /// Get 95th percentile duration
    pub fn p95_duration(&self) -> Duration {
        self.histogram.percentile(0.95)
    }

    /// Get 99th percentile duration
    pub fn p99_duration(&self) -> Duration {
        self.histogram.percentile(0.99)
    }
}

/// Histogram for tracking duration distributions
#[derive(Debug, Clone)]
pub struct DurationHistogram {
    buckets: Vec<DurationBucket>,
}

#[derive(Debug, Clone)]
struct DurationBucket {
    upper_bound: Duration,
    count: u64,
}

impl DurationHistogram {
    fn new() -> Self {
        let buckets = vec![
            DurationBucket {
                upper_bound: Duration::from_micros(100),
                count: 0,
            }, // 0.1ms
            DurationBucket {
                upper_bound: Duration::from_micros(500),
                count: 0,
            }, // 0.5ms
            DurationBucket {
                upper_bound: Duration::from_millis(1),
                count: 0,
            }, // 1ms
            DurationBucket {
                upper_bound: Duration::from_millis(5),
                count: 0,
            }, // 5ms
            DurationBucket {
                upper_bound: Duration::from_millis(10),
                count: 0,
            }, // 10ms
            DurationBucket {
                upper_bound: Duration::from_millis(50),
                count: 0,
            }, // 50ms
            DurationBucket {
                upper_bound: Duration::from_millis(100),
                count: 0,
            }, // 100ms
            DurationBucket {
                upper_bound: Duration::from_millis(500),
                count: 0,
            }, // 500ms
            DurationBucket {
                upper_bound: Duration::from_secs(1),
                count: 0,
            }, // 1s
            DurationBucket {
                upper_bound: Duration::from_secs(5),
                count: 0,
            }, // 5s
            DurationBucket {
                upper_bound: Duration::MAX,
                count: 0,
            }, // >5s
        ];

        Self { buckets }
    }

    fn add_sample(&mut self, duration: Duration) {
        for bucket in &mut self.buckets {
            if duration <= bucket.upper_bound {
                bucket.count += 1;
                break;
            }
        }
    }

    fn percentile(&self, p: f64) -> Duration {
        let total_samples: u64 = self.buckets.iter().map(|b| b.count).sum();
        if total_samples == 0 {
            return Duration::ZERO;
        }

        let target_count = (total_samples as f64 * p) as u64;
        let mut cumulative = 0;

        for bucket in &self.buckets {
            cumulative += bucket.count;
            if cumulative >= target_count {
                return bucket.upper_bound;
            }
        }

        Duration::MAX
    }
}

/// Statistics for operation categories
#[derive(Debug, Clone, Default)]
pub struct CategoryStats {
    pub total_operations: u64,
    pub total_duration: Duration,
    pub total_memory: usize,
}

impl CategoryStats {
    pub fn average_duration(&self) -> Duration {
        if self.total_operations == 0 {
            Duration::ZERO
        } else {
            self.total_duration / self.total_operations as u32
        }
    }

    pub fn average_memory(&self) -> usize {
        if self.total_operations == 0 {
            0
        } else {
            self.total_memory / self.total_operations as usize
        }
    }
}

/// Comprehensive performance metrics
#[derive(Debug, Clone)]
pub struct PerformanceMetrics {
    pub uptime: Duration,
    pub total_operations: usize,
    pub active_operations: usize,
    pub operation_stats: HashMap<Operation, OperationStats>,
    pub category_stats: HashMap<OperationCategory, CategoryStats>,
    pub operations_per_second: f64,
}

impl PerformanceMetrics {
    /// Get the slowest operation type
    pub fn slowest_operation(&self) -> Option<(Operation, Duration)> {
        self.operation_stats
            .iter()
            .max_by_key(|(_, stats)| stats.average_duration())
            .map(|(&op, stats)| (op, stats.average_duration()))
    }

    /// Get the most memory-intensive operation type
    pub fn most_memory_intensive_operation(&self) -> Option<(Operation, usize)> {
        self.operation_stats
            .iter()
            .max_by_key(|(_, stats)| stats.average_memory())
            .map(|(&op, stats)| (op, stats.average_memory()))
    }

    /// Get the most frequent operation type
    pub fn most_frequent_operation(&self) -> Option<(Operation, u64)> {
        self.operation_stats
            .iter()
            .max_by_key(|(_, stats)| stats.count)
            .map(|(&op, stats)| (op, stats.count))
    }

    /// Get human-readable summary
    pub fn summary(&self) -> String {
        let slowest = self
            .slowest_operation()
            .map(|(op, duration)| {
                format!("{} ({:.2}ms)", op.name(), duration.as_secs_f64() * 1000.0)
            })
            .unwrap_or_else(|| "None".to_string());

        let most_frequent = self
            .most_frequent_operation()
            .map(|(op, count)| format!("{} ({} ops)", op.name(), count))
            .unwrap_or_else(|| "None".to_string());

        format!(
            "Performance Summary:\n\
             - Uptime: {:.1}s\n\
             - Total Operations: {}\n\
             - Active Operations: {}\n\
             - Operations/Second: {:.2}\n\
             - Slowest Operation: {}\n\
             - Most Frequent: {}\n\
             - Operation Types: {}",
            self.uptime.as_secs_f64(),
            self.total_operations,
            self.active_operations,
            self.operations_per_second,
            slowest,
            most_frequent,
            self.operation_stats.len()
        )
    }
}

/// Performance alert configuration
#[derive(Debug, Clone)]
pub struct PerformanceAlert {
    pub operation: Operation,
    pub threshold_duration: Duration,
    pub threshold_memory: usize,
    pub enabled: bool,
}

impl PerformanceAlert {
    pub fn new(operation: Operation) -> Self {
        Self {
            operation,
            threshold_duration: Duration::from_millis(1000), // 1 second default
            threshold_memory: 10 * 1024 * 1024,              // 10MB default
            enabled: true,
        }
    }

    pub fn with_duration_threshold(mut self, threshold: Duration) -> Self {
        self.threshold_duration = threshold;
        self
    }

    pub fn with_memory_threshold(mut self, threshold: usize) -> Self {
        self.threshold_memory = threshold;
        self
    }

    /// Check if an operation exceeds thresholds
    pub fn check(&self, duration: Duration, memory: usize) -> bool {
        self.enabled && (duration > self.threshold_duration || memory > self.threshold_memory)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread::sleep;

    #[test]
    fn test_performance_monitor_creation() {
        let monitor = PerformanceMonitor::new();
        assert!(monitor.is_enabled());

        let disabled_monitor = PerformanceMonitor::disabled();
        assert!(!disabled_monitor.is_enabled());
    }

    #[test]
    fn test_operation_tracking() {
        let monitor = PerformanceMonitor::new();

        let token = monitor.start_operation(Operation::PdfGeneration);
        assert_ne!(token, OperationToken(0));

        // Simulate some work
        sleep(Duration::from_millis(1));

        let duration = monitor.end_operation(token);
        assert!(duration > Duration::ZERO);
    }

    #[test]
    fn test_operation_timing() {
        let monitor = PerformanceMonitor::new();

        let (result, duration) = monitor.time_operation(Operation::PdfParsing, || {
            sleep(Duration::from_millis(1));
            42
        });

        assert_eq!(result, 42);
        assert!(duration > Duration::ZERO);
    }

    #[test]
    fn test_statistics_collection() {
        let monitor = PerformanceMonitor::new();

        // Perform multiple operations
        for _ in 0..3 {
            let token = monitor.start_operation(Operation::PageProcessing);
            sleep(Duration::from_micros(100));
            monitor.end_operation(token);
        }

        let stats = monitor.get_stats();
        assert_eq!(stats.total_operations, 3);

        let page_stats = monitor
            .get_operation_stats(Operation::PageProcessing)
            .unwrap();
        assert_eq!(page_stats.count, 3);
        assert!(page_stats.average_duration() > Duration::ZERO);
    }

    #[test]
    fn test_operation_categories() {
        assert_eq!(Operation::PdfParsing.category(), OperationCategory::Core);
        assert_eq!(
            Operation::ImageProcessing.category(),
            OperationCategory::Processing
        );
        assert_eq!(
            Operation::ContentCompression.category(),
            OperationCategory::Optimization
        );
        assert_eq!(Operation::FileIO.category(), OperationCategory::IO);
    }

    #[test]
    fn test_duration_histogram() {
        let mut histogram = DurationHistogram::new();

        histogram.add_sample(Duration::from_micros(50)); // Should go in first bucket
        histogram.add_sample(Duration::from_millis(2)); // Should go in 5ms bucket
        histogram.add_sample(Duration::from_millis(200)); // Should go in 500ms bucket

        let p50 = histogram.percentile(0.5);
        let p95 = histogram.percentile(0.95);

        assert!(p50 > Duration::ZERO);
        assert!(p95 >= p50);
    }

    #[test]
    fn test_operation_stats() {
        let mut stats = OperationStats::new(Operation::PdfGeneration);

        stats.add_measurement(Duration::from_millis(100), 1024);
        stats.add_measurement(Duration::from_millis(200), 2048);

        assert_eq!(stats.count, 2);
        assert_eq!(stats.average_duration(), Duration::from_millis(150));
        assert_eq!(stats.average_memory(), 1536); // (1024 + 2048) / 2
        assert_eq!(stats.min_duration, Duration::from_millis(100));
        assert_eq!(stats.max_duration, Duration::from_millis(200));
    }

    #[test]
    fn test_performance_metrics_analysis() {
        let monitor = PerformanceMonitor::new();

        // Add different operations with different characteristics
        let _ =
            monitor.time_operation(Operation::PdfGeneration, || sleep(Duration::from_millis(2)));
        let _ = monitor.time_operation(Operation::ImageProcessing, || {
            sleep(Duration::from_millis(1))
        });
        let _ =
            monitor.time_operation(Operation::PdfGeneration, || sleep(Duration::from_millis(3)));

        let metrics = monitor.get_stats();

        // PDF Generation should be slowest (average 2.5ms)
        let slowest = metrics.slowest_operation();
        assert!(slowest.is_some());

        let most_frequent = metrics.most_frequent_operation();
        assert!(most_frequent.is_some());
        assert_eq!(most_frequent.unwrap().0, Operation::PdfGeneration); // 2 operations vs 1
    }

    #[test]
    fn test_prometheus_export() {
        let monitor = PerformanceMonitor::new();

        let _ = monitor.time_operation(Operation::PdfParsing, || {});

        let prometheus_output = monitor.export_prometheus();
        assert!(prometheus_output.contains("oxidize_pdf_operations_total"));
        assert!(prometheus_output.contains("oxidize_pdf_operation_duration_seconds"));
        assert!(prometheus_output.contains("PDF Parsing"));
    }

    #[test]
    fn test_performance_alerts() {
        let alert = PerformanceAlert::new(Operation::PdfGeneration)
            .with_duration_threshold(Duration::from_millis(100))
            .with_memory_threshold(1024);

        // Should not trigger
        assert!(!alert.check(Duration::from_millis(50), 512));

        // Should trigger on duration
        assert!(alert.check(Duration::from_millis(150), 512));

        // Should trigger on memory
        assert!(alert.check(Duration::from_millis(50), 2048));
    }

    #[test]
    fn test_recent_operations() {
        let monitor = PerformanceMonitor::new();

        // Perform several operations
        for i in 0..5 {
            let _ = monitor.time_operation(Operation::Custom(format!("test_{}", i)), || {});
        }

        let recent = monitor.get_recent_operations(3);
        assert_eq!(recent.len(), 3);

        // Should be the most recent operations
        let all_recent = monitor.get_recent_operations(10);
        assert_eq!(all_recent.len(), 5);
    }

    #[test]
    fn test_monitor_clear() {
        let monitor = PerformanceMonitor::new();

        let _ = monitor.time_operation(Operation::PdfGeneration, || {});

        let stats_before = monitor.get_stats();
        assert_eq!(stats_before.total_operations, 1);

        monitor.clear();

        let stats_after = monitor.get_stats();
        assert_eq!(stats_after.total_operations, 0);
    }

    #[test]
    fn test_disabled_monitor() {
        let monitor = PerformanceMonitor::disabled();

        let token = monitor.start_operation(Operation::PdfGeneration);
        assert_eq!(token, OperationToken(0));

        let duration = monitor.end_operation(token);
        assert_eq!(duration, Duration::ZERO);

        let stats = monitor.get_stats();
        assert_eq!(stats.total_operations, 0);
    }
}
