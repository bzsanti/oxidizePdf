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
}
