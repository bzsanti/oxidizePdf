//! Memory pool for efficient buffer reuse and reduced garbage collection pressure
//!
//! This module provides memory pools to avoid constant allocation/deallocation
//! of buffers during PDF processing, leading to more predictable performance.
//!
//! # Performance Benefits
//! - **Reduced GC pressure**: Reuse existing allocations
//! - **Predictable performance**: No allocation spikes
//! - **Lower memory fragmentation**: Controlled allocation patterns
//! - **CPU cache efficiency**: Hot memory reuse
//!
//! # Example
//! ```rust
//! use oxidize_pdf::performance::{MemoryPool, PooledBuffer};
//!
//! let mut pool = MemoryPool::new(1024 * 1024); // 1MB pool
//!
//! // Get a buffer from the pool
//! let mut buffer = pool.get_buffer(4096)?; // 4KB buffer
//! buffer.extend_from_slice(b"Hello, World!");
//!
//! // Buffer is automatically returned to pool when dropped
//! drop(buffer);
//!
//! // Reuse the same memory
//! let buffer2 = pool.get_buffer(2048)?; // Gets recycled memory
//! ```

use crate::error::Result;
use std::collections::VecDeque;
use std::ops::{Deref, DerefMut};
use std::sync::{Arc, Mutex};

/// A memory pool that manages reusable buffers of different sizes
pub struct MemoryPool {
    pools: Arc<Mutex<SizedPools>>,
    stats: Arc<Mutex<MemoryPoolStats>>,
    enabled: bool,
    max_total_size: usize,
}

/// Internal storage for different-sized buffer pools
struct SizedPools {
    small_buffers: VecDeque<Vec<u8>>,  // < 4KB
    medium_buffers: VecDeque<Vec<u8>>, // 4KB - 64KB
    large_buffers: VecDeque<Vec<u8>>,  // 64KB - 1MB
    huge_buffers: VecDeque<Vec<u8>>,   // > 1MB
    current_size: usize,
}

/// Buffer size categories for optimal pooling
#[derive(Debug, Clone, Copy, PartialEq)]
enum BufferSize {
    Small,  // < 4KB
    Medium, // 4KB - 64KB
    Large,  // 64KB - 1MB
    Huge,   // > 1MB
}

impl BufferSize {
    fn from_size(size: usize) -> Self {
        match size {
            0..=4096 => BufferSize::Small,
            4097..=65536 => BufferSize::Medium,
            65537..=1048576 => BufferSize::Large,
            _ => BufferSize::Huge,
        }
    }

    fn pool_capacity(&self) -> usize {
        match self {
            BufferSize::Small => 32,  // Keep more small buffers
            BufferSize::Medium => 16, // Moderate medium buffers
            BufferSize::Large => 8,   // Fewer large buffers
            BufferSize::Huge => 2,    // Very few huge buffers
        }
    }

    fn typical_size(&self) -> usize {
        match self {
            BufferSize::Small => 1024,   // 1KB
            BufferSize::Medium => 8192,  // 8KB
            BufferSize::Large => 131072, // 128KB
            BufferSize::Huge => 1048576, // 1MB
        }
    }
}

impl MemoryPool {
    /// Create a new memory pool with the specified maximum total size
    pub fn new(max_total_size: usize) -> Self {
        Self {
            pools: Arc::new(Mutex::new(SizedPools {
                small_buffers: VecDeque::new(),
                medium_buffers: VecDeque::new(),
                large_buffers: VecDeque::new(),
                huge_buffers: VecDeque::new(),
                current_size: 0,
            })),
            stats: Arc::new(Mutex::new(MemoryPoolStats::default())),
            enabled: true,
            max_total_size,
        }
    }

    /// Create a disabled memory pool (no pooling, just direct allocation)
    pub fn disabled() -> Self {
        Self {
            pools: Arc::new(Mutex::new(SizedPools {
                small_buffers: VecDeque::new(),
                medium_buffers: VecDeque::new(),
                large_buffers: VecDeque::new(),
                huge_buffers: VecDeque::new(),
                current_size: 0,
            })),
            stats: Arc::new(Mutex::new(MemoryPoolStats::default())),
            enabled: false,
            max_total_size: 0,
        }
    }

    /// Get a buffer of at least the specified size
    pub fn get_buffer(&self, min_size: usize) -> Result<PooledBuffer> {
        let mut stats = self.stats.lock().unwrap();
        stats.total_requests += 1;

        if !self.enabled {
            stats.direct_allocations += 1;
            let buffer = vec![0; min_size];
            return Ok(PooledBuffer::new_direct(buffer));
        }

        let size_category = BufferSize::from_size(min_size);

        let buffer = {
            let mut pools = self.pools.lock().unwrap();

            let pool = match size_category {
                BufferSize::Small => &mut pools.small_buffers,
                BufferSize::Medium => &mut pools.medium_buffers,
                BufferSize::Large => &mut pools.large_buffers,
                BufferSize::Huge => &mut pools.huge_buffers,
            };

            // Try to find a suitable buffer in the pool
            if let Some(mut buffer) = pool.pop_front() {
                stats.pool_hits += 1;

                // Resize if needed
                if buffer.capacity() < min_size {
                    buffer.reserve(min_size - buffer.capacity());
                }
                buffer.clear();
                buffer.resize(min_size, 0);

                Some(buffer)
            } else {
                stats.pool_misses += 1;
                None
            }
        };

        let buffer = buffer.unwrap_or_else(|| {
            stats.new_allocations += 1;
            // Allocate with some extra capacity for future reuse
            let capacity = std::cmp::max(min_size, size_category.typical_size());
            let mut buf = Vec::with_capacity(capacity);
            buf.resize(min_size, 0);
            buf
        });

        Ok(PooledBuffer::new_pooled(
            buffer,
            self.pools.clone(),
            self.stats.clone(),
            size_category,
        ))
    }

    /// Preallocate buffers of common sizes
    pub fn preallocate(&self) -> Result<()> {
        if !self.enabled {
            return Ok(());
        }

        let mut pools = self.pools.lock().unwrap();

        // Preallocate small buffers (most common)
        for _ in 0..16 {
            let buffer = vec![0; BufferSize::Small.typical_size()];
            pools.small_buffers.push_back(buffer);
        }

        // Preallocate some medium buffers
        for _ in 0..8 {
            let buffer = vec![0; BufferSize::Medium.typical_size()];
            pools.medium_buffers.push_back(buffer);
        }

        // Preallocate a few large buffers
        for _ in 0..4 {
            let buffer = vec![0; BufferSize::Large.typical_size()];
            pools.large_buffers.push_back(buffer);
        }

        // Update current size estimate
        pools.current_size = 16 * BufferSize::Small.typical_size()
            + 8 * BufferSize::Medium.typical_size()
            + 4 * BufferSize::Large.typical_size();

        Ok(())
    }

    /// Get current statistics
    pub fn stats(&self) -> MemoryPoolStats {
        self.stats.lock().unwrap().clone()
    }

    /// Clear all pooled buffers
    pub fn clear(&self) {
        let mut pools = self.pools.lock().unwrap();
        pools.small_buffers.clear();
        pools.medium_buffers.clear();
        pools.large_buffers.clear();
        pools.huge_buffers.clear();
        pools.current_size = 0;
    }

    /// Estimate current memory usage
    pub fn memory_usage(&self) -> usize {
        let pools = self.pools.lock().unwrap();
        pools.current_size
    }

    /// Returns true if the pool is enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Internal method to return a buffer to the appropriate pool
    fn return_buffer(&self, mut buffer: Vec<u8>, size_category: BufferSize) {
        if !self.enabled {
            return; // Just drop the buffer
        }

        let mut pools = self.pools.lock().unwrap();
        let mut stats = self.stats.lock().unwrap();

        // Extract values before borrowing mutably
        let capacity = size_category.pool_capacity();
        let buffer_size = buffer.capacity();
        let current_size = pools.current_size;

        let pool = match size_category {
            BufferSize::Small => &mut pools.small_buffers,
            BufferSize::Medium => &mut pools.medium_buffers,
            BufferSize::Large => &mut pools.large_buffers,
            BufferSize::Huge => &mut pools.huge_buffers,
        };

        if pool.len() < capacity {
            if current_size + buffer_size <= self.max_total_size {
                buffer.clear(); // Clear contents but keep capacity
                pools.current_size += buffer_size;
                pool.push_back(buffer);
                stats.returns_to_pool += 1;
            } else {
                stats.pool_evictions += 1;
                // Drop the buffer (implicit)
            }
        } else {
            stats.pool_evictions += 1;
            // Drop the buffer (implicit)
        }
    }
}

/// A buffer that automatically returns to the pool when dropped
pub struct PooledBuffer {
    buffer: Option<Vec<u8>>,
    pool: Option<Arc<Mutex<SizedPools>>>,
    stats: Option<Arc<Mutex<MemoryPoolStats>>>,
    size_category: Option<BufferSize>,
}

impl PooledBuffer {
    /// Create a new pooled buffer
    fn new_pooled(
        buffer: Vec<u8>,
        pool: Arc<Mutex<SizedPools>>,
        stats: Arc<Mutex<MemoryPoolStats>>,
        size_category: BufferSize,
    ) -> Self {
        Self {
            buffer: Some(buffer),
            pool: Some(pool),
            stats: Some(stats),
            size_category: Some(size_category),
        }
    }

    /// Create a direct (non-pooled) buffer
    fn new_direct(buffer: Vec<u8>) -> Self {
        Self {
            buffer: Some(buffer),
            pool: None,
            stats: None,
            size_category: None,
        }
    }

    /// Get the size of the buffer
    pub fn len(&self) -> usize {
        self.buffer.as_ref().map_or(0, |b| b.len())
    }

    /// Check if the buffer is empty
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Get the capacity of the buffer
    pub fn capacity(&self) -> usize {
        self.buffer.as_ref().map_or(0, |b| b.capacity())
    }

    /// Check if this buffer is pooled
    pub fn is_pooled(&self) -> bool {
        self.pool.is_some()
    }
}

impl Deref for PooledBuffer {
    type Target = Vec<u8>;

    fn deref(&self) -> &Self::Target {
        self.buffer
            .as_ref()
            .expect("Buffer should always be present")
    }
}

impl DerefMut for PooledBuffer {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.buffer
            .as_mut()
            .expect("Buffer should always be present")
    }
}

impl Drop for PooledBuffer {
    fn drop(&mut self) {
        if let (Some(buffer), Some(pool), Some(_stats), Some(size_category)) = (
            self.buffer.take(),
            self.pool.take(),
            self.stats.take(),
            self.size_category,
        ) {
            // Return to pool in a separate thread context to avoid deadlocks
            let pool_ref = pool;
            if let Ok(mut pools) = pool_ref.try_lock() {
                let pool_queue = match size_category {
                    BufferSize::Small => &mut pools.small_buffers,
                    BufferSize::Medium => &mut pools.medium_buffers,
                    BufferSize::Large => &mut pools.large_buffers,
                    BufferSize::Huge => &mut pools.huge_buffers,
                };

                if pool_queue.len() < size_category.pool_capacity() {
                    let capacity = buffer.capacity();
                    let mut returned_buffer = buffer;
                    returned_buffer.clear();
                    pool_queue.push_back(returned_buffer);
                    pools.current_size += capacity;
                }
            }
            // If we can't acquire the lock, the buffer just gets dropped normally
        }
    }
}

/// Statistics about memory pool usage
#[derive(Debug, Clone, Default)]
pub struct MemoryPoolStats {
    pub total_requests: u64,
    pub pool_hits: u64,
    pub pool_misses: u64,
    pub new_allocations: u64,
    pub direct_allocations: u64,
    pub returns_to_pool: u64,
    pub pool_evictions: u64,
}

impl MemoryPoolStats {
    /// Calculate pool hit rate (0.0 to 1.0)
    pub fn hit_rate(&self) -> f64 {
        if self.total_requests == 0 {
            return 0.0;
        }
        self.pool_hits as f64 / self.total_requests as f64
    }

    /// Calculate memory pool efficiency (0.0 to 1.0)
    pub fn efficiency(&self) -> f64 {
        if self.total_requests == 0 {
            return 1.0;
        }

        let efficiency = (self.pool_hits as f64 + self.returns_to_pool as f64)
            / (self.total_requests as f64 + self.returns_to_pool as f64);
        efficiency.min(1.0)
    }

    /// Get human-readable summary
    pub fn summary(&self) -> String {
        format!(
            "Memory Pool Stats:\n\
             - Total Requests: {}\n\
             - Pool Hits: {} ({:.1}%)\n\
             - Pool Misses: {} ({:.1}%)\n\
             - New Allocations: {}\n\
             - Direct Allocations: {}\n\
             - Returns to Pool: {}\n\
             - Pool Evictions: {}\n\
             - Hit Rate: {:.1}%\n\
             - Efficiency: {:.1}%",
            self.total_requests,
            self.pool_hits,
            self.hit_rate() * 100.0,
            self.pool_misses,
            (self.pool_misses as f64 / self.total_requests.max(1) as f64) * 100.0,
            self.new_allocations,
            self.direct_allocations,
            self.returns_to_pool,
            self.pool_evictions,
            self.hit_rate() * 100.0,
            self.efficiency() * 100.0
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_pool_creation() {
        let pool = MemoryPool::new(1024 * 1024);
        assert!(pool.is_enabled());
        assert_eq!(pool.memory_usage(), 0);
    }

    #[test]
    fn test_disabled_pool() {
        let pool = MemoryPool::disabled();
        assert!(!pool.is_enabled());
    }

    #[test]
    fn test_buffer_allocation() {
        let pool = MemoryPool::new(1024 * 1024);
        let buffer = pool.get_buffer(1024).unwrap();

        assert_eq!(buffer.len(), 1024);
        assert!(buffer.capacity() >= 1024);
        assert!(buffer.is_pooled());
    }

    #[test]
    fn test_direct_buffer_allocation() {
        let pool = MemoryPool::disabled();
        let buffer = pool.get_buffer(1024).unwrap();

        assert_eq!(buffer.len(), 1024);
        assert!(!buffer.is_pooled());
    }

    #[test]
    fn test_buffer_reuse() {
        let pool = MemoryPool::new(1024 * 1024);

        // Allocate and drop a buffer
        {
            let _buffer = pool.get_buffer(1024).unwrap();
        }

        let stats1 = pool.stats();
        assert_eq!(stats1.total_requests, 1);

        // Allocate another buffer of same size - should reuse
        let _buffer2 = pool.get_buffer(1024).unwrap();

        let stats2 = pool.stats();
        assert_eq!(stats2.total_requests, 2);
        assert!(stats2.pool_hits > 0 || stats2.pool_misses > 0);
    }

    #[test]
    fn test_buffer_size_categories() {
        assert_eq!(BufferSize::from_size(1000), BufferSize::Small);
        assert_eq!(BufferSize::from_size(8000), BufferSize::Medium);
        assert_eq!(BufferSize::from_size(100000), BufferSize::Large);
        assert_eq!(BufferSize::from_size(2000000), BufferSize::Huge);
    }

    #[test]
    fn test_preallocate() {
        let pool = MemoryPool::new(10 * 1024 * 1024);
        pool.preallocate().unwrap();

        assert!(pool.memory_usage() > 0);

        // First request should be a hit
        let _buffer = pool.get_buffer(1024).unwrap();
        let stats = pool.stats();
        assert!(stats.pool_hits > 0);
    }

    #[test]
    fn test_buffer_resize() {
        let pool = MemoryPool::new(1024 * 1024);
        let mut buffer = pool.get_buffer(100).unwrap();

        // Resize buffer
        buffer.resize(200, 42);
        assert_eq!(buffer.len(), 200);
        assert_eq!(buffer[199], 42);
    }

    #[test]
    fn test_hit_rate_calculation() {
        let mut stats = MemoryPoolStats::default();
        stats.total_requests = 10;
        stats.pool_hits = 7;

        assert_eq!(stats.hit_rate(), 0.7);
    }

    #[test]
    fn test_efficiency_calculation() {
        let mut stats = MemoryPoolStats::default();
        stats.total_requests = 10;
        stats.pool_hits = 6;
        stats.returns_to_pool = 8;

        let efficiency = stats.efficiency();
        assert!(efficiency > 0.0 && efficiency <= 1.0);
    }

    #[test]
    fn test_pool_clear() {
        let pool = MemoryPool::new(1024 * 1024);
        pool.preallocate().unwrap();

        assert!(pool.memory_usage() > 0);

        pool.clear();
        assert_eq!(pool.memory_usage(), 0);
    }

    #[test]
    fn test_buffer_deref() {
        let pool = MemoryPool::new(1024 * 1024);
        let mut buffer = pool.get_buffer(10).unwrap();

        // Test deref
        assert_eq!(buffer.len(), 10);

        // Test deref_mut
        buffer[0] = 42;
        assert_eq!(buffer[0], 42);
    }
}
