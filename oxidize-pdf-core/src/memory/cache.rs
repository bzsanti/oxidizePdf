//! LRU cache implementation for PDF objects
//!
//! Provides efficient caching of frequently accessed PDF objects to reduce
//! repeated parsing and memory allocations.

use crate::objects::ObjectId;
use crate::parser::PdfObject;
use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, RwLock};

/// Generic LRU (Least Recently Used) cache
pub struct LruCache<K: Clone + Eq + std::hash::Hash, V> {
    capacity: usize,
    map: HashMap<K, V>,
    order: VecDeque<K>,
}

impl<K: Clone + Eq + std::hash::Hash, V> LruCache<K, V> {
    /// Create a new LRU cache with specified capacity
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity,
            map: HashMap::with_capacity(capacity),
            order: VecDeque::with_capacity(capacity),
        }
    }

    /// Get a value from the cache
    pub fn get(&mut self, key: &K) -> Option<&V> {
        if self.map.contains_key(key) {
            // Move to front (most recently used)
            self.order.retain(|k| k != key);
            self.order.push_front(key.clone());
            self.map.get(key)
        } else {
            None
        }
    }

    /// Put a value into the cache
    pub fn put(&mut self, key: K, value: V) {
        // Handle zero capacity
        if self.capacity == 0 {
            return;
        }

        if self.map.contains_key(&key) {
            // Update existing
            self.order.retain(|k| k != &key);
        } else if self.map.len() >= self.capacity {
            // Evict least recently used
            if let Some(lru_key) = self.order.pop_back() {
                self.map.remove(&lru_key);
            }
        }

        self.map.insert(key.clone(), value);
        self.order.push_front(key);
    }

    /// Clear the cache
    pub fn clear(&mut self) {
        self.map.clear();
        self.order.clear();
    }

    /// Get the current number of items in the cache
    pub fn len(&self) -> usize {
        self.map.len()
    }

    /// Check if the cache is empty
    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }
}

/// Thread-safe cache for PDF objects
pub struct ObjectCache {
    cache: Arc<RwLock<LruCache<ObjectId, Arc<PdfObject>>>>,
}

impl ObjectCache {
    /// Create a new object cache
    pub fn new(capacity: usize) -> Self {
        Self {
            cache: Arc::new(RwLock::new(LruCache::new(capacity))),
        }
    }

    /// Get an object from the cache
    pub fn get(&self, id: &ObjectId) -> Option<Arc<PdfObject>> {
        if let Ok(mut cache) = self.cache.write() {
            cache.get(id).cloned()
        } else {
            None
        }
    }

    /// Store an object in the cache
    pub fn put(&self, id: ObjectId, object: Arc<PdfObject>) {
        if let Ok(mut cache) = self.cache.write() {
            cache.put(id, object);
        }
    }

    /// Clear the cache
    pub fn clear(&self) {
        if let Ok(mut cache) = self.cache.write() {
            cache.clear();
        }
    }

    /// Get cache statistics
    pub fn stats(&self) -> CacheStats {
        if let Ok(cache) = self.cache.read() {
            CacheStats {
                size: cache.len(),
                capacity: cache.capacity,
            }
        } else {
            CacheStats::default()
        }
    }
}

/// Cache statistics
#[derive(Debug, Clone, Default)]
pub struct CacheStats {
    /// Current number of cached items
    pub size: usize,
    /// Maximum capacity
    pub capacity: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lru_cache_basic() {
        let mut cache = LruCache::new(3);

        // Test insertion
        cache.put(1, "one");
        cache.put(2, "two");
        cache.put(3, "three");

        assert_eq!(cache.len(), 3);
        assert_eq!(cache.get(&1), Some(&"one"));
        assert_eq!(cache.get(&2), Some(&"two"));
        assert_eq!(cache.get(&3), Some(&"three"));
    }

    #[test]
    fn test_lru_cache_eviction() {
        let mut cache = LruCache::new(3);

        cache.put(1, "one");
        cache.put(2, "two");
        cache.put(3, "three");

        // This should evict the least recently used (1)
        cache.put(4, "four");

        assert_eq!(cache.len(), 3);
        assert_eq!(cache.get(&1), None);
        assert_eq!(cache.get(&2), Some(&"two"));
        assert_eq!(cache.get(&3), Some(&"three"));
        assert_eq!(cache.get(&4), Some(&"four"));
    }

    #[test]
    fn test_lru_cache_access_order() {
        let mut cache = LruCache::new(3);

        cache.put(1, "one");
        cache.put(2, "two");
        cache.put(3, "three");

        // Access 1, making it recently used
        assert_eq!(cache.get(&1), Some(&"one"));

        // Add 4, should evict 2 (least recently used)
        cache.put(4, "four");

        assert_eq!(cache.get(&1), Some(&"one"));
        assert_eq!(cache.get(&2), None);
        assert_eq!(cache.get(&3), Some(&"three"));
        assert_eq!(cache.get(&4), Some(&"four"));
    }

    #[test]
    fn test_lru_cache_update() {
        let mut cache = LruCache::new(3);

        cache.put(1, "one");
        cache.put(2, "two");
        cache.put(3, "three");

        // Update existing key
        cache.put(2, "two-updated");

        assert_eq!(cache.len(), 3);
        assert_eq!(cache.get(&2), Some(&"two-updated"));

        // 2 should now be most recently used
        cache.put(4, "four");

        assert_eq!(cache.get(&1), None); // Evicted
        assert_eq!(cache.get(&2), Some(&"two-updated"));
    }

    #[test]
    fn test_lru_cache_clear() {
        let mut cache = LruCache::new(3);

        cache.put(1, "one");
        cache.put(2, "two");
        cache.put(3, "three");

        cache.clear();

        assert_eq!(cache.len(), 0);
        assert!(cache.is_empty());
        assert_eq!(cache.get(&1), None);
    }

    #[test]
    fn test_object_cache() {
        let cache = ObjectCache::new(10);

        let obj1 = Arc::new(PdfObject::Integer(42));
        let obj2 = Arc::new(PdfObject::String(crate::parser::PdfString::new(
            b"test".to_vec(),
        )));

        let id1 = ObjectId::new(1, 0);
        let id2 = ObjectId::new(2, 0);

        cache.put(id1, obj1.clone());
        cache.put(id2, obj2.clone());

        assert_eq!(cache.get(&id1), Some(obj1));
        assert_eq!(cache.get(&id2), Some(obj2));

        let stats = cache.stats();
        assert_eq!(stats.size, 2);
        assert_eq!(stats.capacity, 10);
    }

    #[test]
    fn test_object_cache_clear() {
        let cache = ObjectCache::new(5);

        let obj = Arc::new(PdfObject::Boolean(true));
        let id = ObjectId::new(1, 0);

        cache.put(id, obj.clone());
        assert_eq!(cache.get(&id), Some(obj));

        cache.clear();
        assert_eq!(cache.get(&id), None);

        let stats = cache.stats();
        assert_eq!(stats.size, 0);
    }

    #[test]
    fn test_lru_cache_zero_capacity() {
        let mut cache = LruCache::new(0);

        cache.put(1, "one");
        assert_eq!(cache.len(), 0);
        assert!(cache.is_empty());
        assert_eq!(cache.get(&1), None);
    }

    #[test]
    fn test_lru_cache_single_capacity() {
        let mut cache = LruCache::new(1);

        cache.put(1, "one");
        assert_eq!(cache.len(), 1);
        assert_eq!(cache.get(&1), Some(&"one"));

        cache.put(2, "two");
        assert_eq!(cache.len(), 1);
        assert_eq!(cache.get(&1), None);
        assert_eq!(cache.get(&2), Some(&"two"));
    }

    #[test]
    fn test_lru_cache_repeated_access() {
        let mut cache = LruCache::new(3);

        cache.put(1, "one");
        cache.put(2, "two");
        cache.put(3, "three");

        // Access key 1 multiple times
        for _ in 0..5 {
            assert_eq!(cache.get(&1), Some(&"one"));
        }

        // Add new item, should evict key 2 (least recently used)
        cache.put(4, "four");

        assert_eq!(cache.get(&1), Some(&"one"));
        assert_eq!(cache.get(&2), None);
        assert_eq!(cache.get(&3), Some(&"three"));
        assert_eq!(cache.get(&4), Some(&"four"));
    }

    #[test]
    fn test_lru_cache_get_nonexistent() {
        let mut cache = LruCache::new(3);

        cache.put(1, "one");
        cache.put(2, "two");

        assert_eq!(cache.get(&3), None);
        assert_eq!(cache.get(&99), None);

        // Cache should remain unchanged
        assert_eq!(cache.len(), 2);
        assert_eq!(cache.get(&1), Some(&"one"));
        assert_eq!(cache.get(&2), Some(&"two"));
    }

    #[test]
    fn test_lru_cache_large_capacity() {
        let mut cache = LruCache::new(1000);

        // Fill cache with items
        for i in 0..500 {
            cache.put(i, format!("value_{i}"));
        }

        assert_eq!(cache.len(), 500);

        // Access all items to verify they're all there
        for i in 0..500 {
            assert_eq!(cache.get(&i), Some(&format!("value_{i}")));
        }
    }

    #[test]
    fn test_lru_cache_complex_eviction_pattern() {
        let mut cache = LruCache::new(4);

        // Fill cache
        cache.put(1, "one");
        cache.put(2, "two");
        cache.put(3, "three");
        cache.put(4, "four");

        // Access keys in specific order
        assert_eq!(cache.get(&2), Some(&"two"));
        assert_eq!(cache.get(&4), Some(&"four"));

        // Add new items - should evict 1 and 3
        cache.put(5, "five");
        cache.put(6, "six");

        assert_eq!(cache.get(&1), None);
        assert_eq!(cache.get(&2), Some(&"two"));
        assert_eq!(cache.get(&3), None);
        assert_eq!(cache.get(&4), Some(&"four"));
        assert_eq!(cache.get(&5), Some(&"five"));
        assert_eq!(cache.get(&6), Some(&"six"));
    }

    #[test]
    fn test_lru_cache_update_preserves_capacity() {
        let mut cache = LruCache::new(2);

        cache.put(1, "one");
        cache.put(2, "two");

        // Update key 1 - this should move it to the front
        cache.put(1, "one_updated");

        assert_eq!(cache.len(), 2);

        // Add new key - since we updated key 1 most recently, key 2 should be evicted
        cache.put(3, "three");

        // Check that key 1 (most recently updated) is still there
        assert_eq!(cache.get(&1), Some(&"one_updated"));
        // Check that key 2 (least recently used) was evicted
        assert_eq!(cache.get(&2), None);
        // Check that key 3 (newly added) is there
        assert_eq!(cache.get(&3), Some(&"three"));
    }

    #[test]
    fn test_lru_cache_with_string_keys() {
        let mut cache = LruCache::new(3);

        cache.put("key1".to_string(), 1);
        cache.put("key2".to_string(), 2);
        cache.put("key3".to_string(), 3);

        assert_eq!(cache.get(&"key1".to_string()), Some(&1));
        assert_eq!(cache.get(&"key2".to_string()), Some(&2));
        assert_eq!(cache.get(&"key3".to_string()), Some(&3));

        cache.put("key4".to_string(), 4);

        assert_eq!(cache.get(&"key1".to_string()), None);
        assert_eq!(cache.get(&"key4".to_string()), Some(&4));
    }

    #[test]
    fn test_object_cache_with_different_object_types() {
        let cache = ObjectCache::new(10);

        let int_obj = Arc::new(PdfObject::Integer(42));
        let bool_obj = Arc::new(PdfObject::Boolean(false));
        let null_obj = Arc::new(PdfObject::Null);
        let real_obj = Arc::new(PdfObject::Real(3.14));

        let id1 = ObjectId::new(1, 0);
        let id2 = ObjectId::new(2, 0);
        let id3 = ObjectId::new(3, 0);
        let id4 = ObjectId::new(4, 0);

        cache.put(id1, int_obj.clone());
        cache.put(id2, bool_obj.clone());
        cache.put(id3, null_obj.clone());
        cache.put(id4, real_obj.clone());

        assert_eq!(cache.get(&id1), Some(int_obj));
        assert_eq!(cache.get(&id2), Some(bool_obj));
        assert_eq!(cache.get(&id3), Some(null_obj));
        assert_eq!(cache.get(&id4), Some(real_obj));

        let stats = cache.stats();
        assert_eq!(stats.size, 4);
    }

    #[test]
    fn test_object_cache_eviction() {
        let cache = ObjectCache::new(2);

        let obj1 = Arc::new(PdfObject::Integer(1));
        let obj2 = Arc::new(PdfObject::Integer(2));
        let obj3 = Arc::new(PdfObject::Integer(3));

        let id1 = ObjectId::new(1, 0);
        let id2 = ObjectId::new(2, 0);
        let id3 = ObjectId::new(3, 0);

        cache.put(id1, obj1);
        cache.put(id2, obj2.clone());

        let stats = cache.stats();
        assert_eq!(stats.size, 2);

        // This should evict the first object
        cache.put(id3, obj3.clone());

        let stats = cache.stats();
        assert_eq!(stats.size, 2);
        assert_eq!(cache.get(&id1), None);
        assert_eq!(cache.get(&id2), Some(obj2));
        assert_eq!(cache.get(&id3), Some(obj3));
    }

    #[test]
    fn test_object_cache_get_nonexistent() {
        let cache = ObjectCache::new(5);

        let obj = Arc::new(PdfObject::Integer(42));
        let id1 = ObjectId::new(1, 0);
        let id2 = ObjectId::new(2, 0);

        cache.put(id1, obj.clone());

        assert_eq!(cache.get(&id1), Some(obj));
        assert_eq!(cache.get(&id2), None);

        let stats = cache.stats();
        assert_eq!(stats.size, 1);
    }

    #[test]
    fn test_object_cache_update_existing() {
        let cache = ObjectCache::new(3);

        let obj1 = Arc::new(PdfObject::Integer(42));
        let obj2 = Arc::new(PdfObject::Integer(100));
        let id = ObjectId::new(1, 0);

        cache.put(id, obj1);
        cache.put(id, obj2.clone());

        assert_eq!(cache.get(&id), Some(obj2));

        let stats = cache.stats();
        assert_eq!(stats.size, 1);
    }

    #[test]
    fn test_object_cache_with_generation_numbers() {
        let cache = ObjectCache::new(5);

        let obj1 = Arc::new(PdfObject::Integer(1));
        let obj2 = Arc::new(PdfObject::Integer(2));

        let id1_gen0 = ObjectId::new(1, 0);
        let id1_gen1 = ObjectId::new(1, 1);

        cache.put(id1_gen0, obj1.clone());
        cache.put(id1_gen1, obj2.clone());

        // Different generations should be treated as different keys
        assert_eq!(cache.get(&id1_gen0), Some(obj1));
        assert_eq!(cache.get(&id1_gen1), Some(obj2));

        let stats = cache.stats();
        assert_eq!(stats.size, 2);
    }

    #[test]
    fn test_cache_stats_debug_clone_default() {
        let stats = CacheStats {
            size: 5,
            capacity: 10,
        };

        let debug_str = format!("{stats:?}");
        assert!(debug_str.contains("CacheStats"));
        assert!(debug_str.contains("5"));
        assert!(debug_str.contains("10"));

        let cloned = stats;
        assert_eq!(cloned.size, 5);
        assert_eq!(cloned.capacity, 10);

        let default_stats = CacheStats::default();
        assert_eq!(default_stats.size, 0);
        assert_eq!(default_stats.capacity, 0);
    }

    #[test]
    fn test_object_cache_stats_after_operations() {
        let cache = ObjectCache::new(3);

        // Initially empty
        let stats = cache.stats();
        assert_eq!(stats.size, 0);
        assert_eq!(stats.capacity, 3);

        // Add one item
        let obj1 = Arc::new(PdfObject::Integer(1));
        let id1 = ObjectId::new(1, 0);
        cache.put(id1, obj1);

        let stats = cache.stats();
        assert_eq!(stats.size, 1);
        assert_eq!(stats.capacity, 3);

        // Fill to capacity
        let obj2 = Arc::new(PdfObject::Integer(2));
        let obj3 = Arc::new(PdfObject::Integer(3));
        let id2 = ObjectId::new(2, 0);
        let id3 = ObjectId::new(3, 0);

        cache.put(id2, obj2);
        cache.put(id3, obj3);

        let stats = cache.stats();
        assert_eq!(stats.size, 3);
        assert_eq!(stats.capacity, 3);

        // Clear cache
        cache.clear();

        let stats = cache.stats();
        assert_eq!(stats.size, 0);
        assert_eq!(stats.capacity, 3);
    }

    #[test]
    fn test_lru_cache_stress_test() {
        let mut cache = LruCache::new(100);

        // Fill cache beyond capacity
        for i in 0..200 {
            cache.put(i, format!("value_{i}"));
        }

        // Should only contain last 100 items
        assert_eq!(cache.len(), 100);

        for i in 0..100 {
            assert_eq!(cache.get(&i), None);
        }

        for i in 100..200 {
            assert_eq!(cache.get(&i), Some(&format!("value_{i}")));
        }
    }

    #[test]
    fn test_object_cache_concurrent_access_simulation() {
        use std::sync::Arc as StdArc;
        use std::thread;

        let cache = StdArc::new(ObjectCache::new(10));
        let mut handles = vec![];

        // Simulate concurrent access (though not truly concurrent in test)
        for i in 0..5 {
            let cache_clone = cache.clone();
            let handle = thread::spawn(move || {
                let obj = Arc::new(PdfObject::Integer(i));
                let id = ObjectId::new(i as u32, 0);

                cache_clone.put(id, obj.clone());
                assert_eq!(cache_clone.get(&id), Some(obj));
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.join().unwrap();
        }

        let stats = cache.stats();
        assert_eq!(stats.size, 5);
    }
}
