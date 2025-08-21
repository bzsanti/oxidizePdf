//! Lazy loading implementation for PDF documents
//!
//! Provides on-demand loading of PDF objects and pages to minimize memory usage
//! when working with large documents.

use crate::error::{PdfError, Result};
use crate::memory::{MemoryManager, MemoryOptions};
use crate::objects::ObjectId;
use crate::parser::{ParsedPage, PdfObject, PdfReader};
use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Seek};
use std::sync::{Arc, RwLock};

/// A lazily-loaded PDF object that loads its content on first access
pub enum LazyObject {
    /// Not yet loaded - contains file offset
    NotLoaded { offset: u64 },
    /// Loaded and cached
    Loaded(Arc<PdfObject>),
    /// Currently being loaded (prevents recursive loading)
    Loading,
}

/// PDF document with lazy loading support
pub struct LazyDocument<R: Read + Seek> {
    #[allow(dead_code)]
    reader: Arc<RwLock<PdfReader<R>>>,
    memory_manager: Arc<MemoryManager>,
    object_map: Arc<RwLock<HashMap<ObjectId, LazyObject>>>,
    page_count: u32,
}

impl LazyDocument<File> {
    /// Open a PDF file with lazy loading
    pub fn open<P: AsRef<std::path::Path>>(path: P, options: MemoryOptions) -> Result<Self> {
        let reader = PdfReader::open(path).map_err(|e| PdfError::ParseError(e.to_string()))?;
        Self::new(reader, options)
    }
}

impl<R: Read + Seek> LazyDocument<R> {
    /// Create a new lazy document from a reader
    pub fn new(reader: PdfReader<R>, options: MemoryOptions) -> Result<Self> {
        let memory_manager = Arc::new(MemoryManager::new(options));

        // For now, use a fixed page count
        // In a real implementation, we would parse the catalog to get page count
        let page_count = 0;

        let reader = Arc::new(RwLock::new(reader));
        let object_map = Arc::new(RwLock::new(HashMap::new()));

        Ok(Self {
            reader,
            memory_manager,
            object_map,
            page_count,
        })
    }

    /// Get the total number of pages
    pub fn page_count(&self) -> u32 {
        self.page_count
    }

    /// Get a page (loads on demand)
    pub fn get_page(&self, index: u32) -> Result<ParsedPage> {
        if index >= self.page_count {
            return Err(PdfError::InvalidPageNumber(index));
        }

        self.memory_manager.record_cache_miss();

        // For now, return an error as we can't easily clone the reader
        // In a real implementation, we would need a different approach
        Err(PdfError::ParseError(
            "Lazy loading not fully implemented".to_string(),
        ))
    }

    /// Get an object by ID (loads on demand)
    pub fn get_object(&self, id: &ObjectId) -> Result<Arc<PdfObject>> {
        // Check cache first
        if let Some(cache) = self.memory_manager.cache() {
            if let Some(obj) = cache.get(id) {
                self.memory_manager.record_cache_hit();
                return Ok(obj);
            }
        }

        self.memory_manager.record_cache_miss();

        // Check if we have this object in our map
        if let Ok(mut map) = self.object_map.write() {
            match map.get_mut(id) {
                Some(LazyObject::Loaded(obj)) => {
                    return Ok(obj.clone());
                }
                Some(LazyObject::Loading) => {
                    return Err(PdfError::ParseError(
                        "Circular reference detected".to_string(),
                    ));
                }
                Some(LazyObject::NotLoaded { offset }) => {
                    let offset = *offset;
                    // Mark as loading to prevent recursion
                    map.insert(*id, LazyObject::Loading);

                    // Load the object
                    let obj = self.load_object_at_offset(offset)?;
                    let obj_arc = Arc::new(obj);

                    // Cache it
                    if let Some(cache) = self.memory_manager.cache() {
                        cache.put(*id, obj_arc.clone());
                    }

                    // Update map
                    map.insert(*id, LazyObject::Loaded(obj_arc.clone()));

                    return Ok(obj_arc);
                }
                None => {
                    // For now, return error as we can't easily access objects
                    // In a real implementation, we would need direct object access
                }
            }
        }

        Err(PdfError::InvalidObjectReference(
            id.number(),
            id.generation(),
        ))
    }

    /// Preload objects for a specific page
    pub fn preload_page(&self, index: u32) -> Result<()> {
        let page = self.get_page(index)?;

        // Preload common page resources
        if let Some(resources) = page.get_resources() {
            // Preload fonts
            if let Some(fonts) = resources.get("Font").and_then(|f| f.as_dict()) {
                for font_ref in fonts.0.values() {
                    if let PdfObject::Reference(num, gen) = font_ref {
                        let id = ObjectId::new(*num, *gen);
                        let _ = self.get_object(&id);
                    }
                }
            }

            // Preload XObjects (images)
            if let Some(xobjects) = resources.get("XObject").and_then(|x| x.as_dict()) {
                for xobj_ref in xobjects.0.values() {
                    if let PdfObject::Reference(num, gen) = xobj_ref {
                        let id = ObjectId::new(*num, *gen);
                        let _ = self.get_object(&id);
                    }
                }
            }
        }

        Ok(())
    }

    /// Get memory statistics
    pub fn memory_stats(&self) -> crate::memory::MemoryStats {
        self.memory_manager.stats()
    }

    /// Clear all caches
    pub fn clear_cache(&self) {
        if let Some(cache) = self.memory_manager.cache() {
            cache.clear();
        }

        if let Ok(mut map) = self.object_map.write() {
            // Keep NotLoaded entries, only clear Loaded ones
            map.retain(|_, obj| matches!(obj, LazyObject::NotLoaded { .. }));
        }
    }

    fn load_object_at_offset(&self, _offset: u64) -> Result<PdfObject> {
        // In a real implementation, this would seek to the offset and parse the object
        // For now, return a placeholder
        Ok(PdfObject::Null)
    }
}

/// Lazy page iterator
pub struct LazyPageIterator<R: Read + Seek> {
    document: Arc<LazyDocument<R>>,
    current: u32,
    total: u32,
}

impl<R: Read + Seek> LazyPageIterator<R> {
    pub fn new(document: Arc<LazyDocument<R>>) -> Self {
        let total = document.page_count();
        Self {
            document,
            current: 0,
            total,
        }
    }
}

impl<R: Read + Seek> Iterator for LazyPageIterator<R> {
    type Item = Result<ParsedPage>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current >= self.total {
            return None;
        }

        let result = self.document.get_page(self.current);
        self.current += 1;
        Some(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::test_helpers;
    use std::io::Cursor;

    #[test]
    fn test_lazy_object_states() {
        let not_loaded = LazyObject::NotLoaded { offset: 1234 };
        match not_loaded {
            LazyObject::NotLoaded { offset } => assert_eq!(offset, 1234),
            _ => panic!("Wrong state"),
        }

        let loaded = LazyObject::Loaded(Arc::new(PdfObject::Integer(42)));
        match loaded {
            LazyObject::Loaded(obj) => {
                assert_eq!(*obj, PdfObject::Integer(42));
            }
            _ => panic!("Wrong state"),
        }

        let loading = LazyObject::Loading;
        match loading {
            LazyObject::Loading => assert!(true),
            _ => panic!("Wrong state"),
        }
    }

    #[test]
    fn test_lazy_document_creation() {
        let pdf_data = test_helpers::create_minimal_pdf();
        let cursor = Cursor::new(pdf_data);
        let reader = PdfReader::new(cursor).unwrap();
        let options = MemoryOptions::default();

        let lazy_doc = LazyDocument::new(reader, options).unwrap();
        assert_eq!(lazy_doc.page_count(), 0); // Minimal PDF has no pages
    }

    #[test]
    fn test_memory_stats() {
        let pdf_data = test_helpers::create_minimal_pdf();
        let cursor = Cursor::new(pdf_data);
        let reader = PdfReader::new(cursor).unwrap();
        let options = MemoryOptions::default();

        let lazy_doc = LazyDocument::new(reader, options).unwrap();
        let stats = lazy_doc.memory_stats();

        assert_eq!(stats.cache_hits, 0);
        assert_eq!(stats.cache_misses, 0);
    }

    #[test]
    fn test_clear_cache() {
        let pdf_data = test_helpers::create_minimal_pdf();
        let cursor = Cursor::new(pdf_data);
        let reader = PdfReader::new(cursor).unwrap();
        let options = MemoryOptions::default().with_cache_size(10);

        let lazy_doc = LazyDocument::new(reader, options).unwrap();

        // Clear cache should not panic
        lazy_doc.clear_cache();
    }

    #[test]
    fn test_page_iterator() {
        let pdf_data = test_helpers::create_minimal_pdf();
        let cursor = Cursor::new(pdf_data);
        let reader = PdfReader::new(cursor).unwrap();
        let options = MemoryOptions::default();

        let lazy_doc = Arc::new(LazyDocument::new(reader, options).unwrap());
        let mut iterator = LazyPageIterator::new(lazy_doc);

        // Should have no pages for minimal PDF
        assert!(iterator.next().is_none());
    }

    #[test]
    fn test_lazy_object_not_loaded() {
        let obj = LazyObject::NotLoaded { offset: 42 };

        match obj {
            LazyObject::NotLoaded { offset } => {
                assert_eq!(offset, 42);
            }
            _ => panic!("Expected NotLoaded variant"),
        }
    }

    #[test]
    fn test_lazy_object_loaded() {
        let pdf_obj = Arc::new(PdfObject::Boolean(true));
        let obj = LazyObject::Loaded(pdf_obj.clone());

        match obj {
            LazyObject::Loaded(arc_obj) => {
                assert_eq!(*arc_obj, PdfObject::Boolean(true));
                assert!(Arc::ptr_eq(&arc_obj, &pdf_obj));
            }
            _ => panic!("Expected Loaded variant"),
        }
    }

    #[test]
    fn test_lazy_object_loading() {
        let obj = LazyObject::Loading;

        match obj {
            LazyObject::Loading => {
                // Success - no additional assertions needed
            }
            _ => panic!("Expected Loading variant"),
        }
    }

    #[test]
    fn test_lazy_document_page_count() {
        let pdf_data = test_helpers::create_minimal_pdf();
        let cursor = Cursor::new(pdf_data);
        let reader = PdfReader::new(cursor).unwrap();
        let options = MemoryOptions::default();

        let lazy_doc = LazyDocument::new(reader, options).unwrap();

        assert_eq!(lazy_doc.page_count(), 0);
    }

    #[test]
    fn test_lazy_document_get_page_invalid_index() {
        let pdf_data = test_helpers::create_minimal_pdf();
        let cursor = Cursor::new(pdf_data);
        let reader = PdfReader::new(cursor).unwrap();
        let options = MemoryOptions::default();

        let lazy_doc = LazyDocument::new(reader, options).unwrap();

        // Try to get page 0 when document has 0 pages
        let result = lazy_doc.get_page(0);
        assert!(result.is_err());

        match result {
            Err(PdfError::InvalidPageNumber(num)) => {
                assert_eq!(num, 0);
            }
            _ => panic!("Expected InvalidPageNumber error"),
        }

        // Try to get page 5 when document has 0 pages
        let result = lazy_doc.get_page(5);
        assert!(result.is_err());

        match result {
            Err(PdfError::InvalidPageNumber(num)) => {
                assert_eq!(num, 5);
            }
            _ => panic!("Expected InvalidPageNumber error"),
        }
    }

    #[test]
    fn test_lazy_document_get_object_not_found() {
        let pdf_data = test_helpers::create_minimal_pdf();
        let cursor = Cursor::new(pdf_data);
        let reader = PdfReader::new(cursor).unwrap();
        let options = MemoryOptions::default();

        let lazy_doc = LazyDocument::new(reader, options).unwrap();
        let obj_id = ObjectId::new(999, 0);

        let result = lazy_doc.get_object(&obj_id);
        assert!(result.is_err());

        match result {
            Err(PdfError::InvalidObjectReference(num, gen)) => {
                assert_eq!(num, 999);
                assert_eq!(gen, 0);
            }
            _ => panic!("Expected InvalidObjectReference error"),
        }
    }

    #[test]
    fn test_lazy_document_get_object_circular_reference() {
        let pdf_data = test_helpers::create_minimal_pdf();
        let cursor = Cursor::new(pdf_data);
        let reader = PdfReader::new(cursor).unwrap();
        let options = MemoryOptions::default();

        let lazy_doc = LazyDocument::new(reader, options).unwrap();
        let obj_id = ObjectId::new(1, 0);

        // Manually insert a Loading object to simulate circular reference
        {
            let mut map = lazy_doc.object_map.write().unwrap();
            map.insert(obj_id, LazyObject::Loading);
        }

        let result = lazy_doc.get_object(&obj_id);
        assert!(result.is_err());

        match result {
            Err(PdfError::ParseError(msg)) => {
                assert!(msg.contains("Circular reference"));
            }
            _ => panic!("Expected ParseError for circular reference"),
        }
    }

    #[test]
    fn test_lazy_document_get_object_not_loaded_then_loaded() {
        let pdf_data = test_helpers::create_minimal_pdf();
        let cursor = Cursor::new(pdf_data);
        let reader = PdfReader::new(cursor).unwrap();
        let options = MemoryOptions::default();

        let lazy_doc = LazyDocument::new(reader, options).unwrap();
        let obj_id = ObjectId::new(1, 0);

        // Manually insert a NotLoaded object
        {
            let mut map = lazy_doc.object_map.write().unwrap();
            map.insert(obj_id, LazyObject::NotLoaded { offset: 100 });
        }

        let result = lazy_doc.get_object(&obj_id);
        assert!(result.is_ok());

        let obj = result.unwrap();
        assert_eq!(*obj, PdfObject::Null); // load_object_at_offset returns Null

        // Object should now be cached as Loaded
        {
            let map = lazy_doc.object_map.read().unwrap();
            match map.get(&obj_id) {
                Some(LazyObject::Loaded(_)) => {}
                _ => panic!("Expected object to be cached as Loaded"),
            }
        }
    }

    #[test]
    fn test_lazy_document_get_object_already_loaded() {
        let pdf_data = test_helpers::create_minimal_pdf();
        let cursor = Cursor::new(pdf_data);
        let reader = PdfReader::new(cursor).unwrap();
        let options = MemoryOptions::default();

        let lazy_doc = LazyDocument::new(reader, options).unwrap();
        let obj_id = ObjectId::new(1, 0);
        let test_obj = Arc::new(PdfObject::Boolean(true));

        // Manually insert a Loaded object
        {
            let mut map = lazy_doc.object_map.write().unwrap();
            map.insert(obj_id, LazyObject::Loaded(test_obj.clone()));
        }

        let result = lazy_doc.get_object(&obj_id);
        assert!(result.is_ok());

        let obj = result.unwrap();
        assert_eq!(*obj, PdfObject::Boolean(true));
        assert!(Arc::ptr_eq(&obj, &test_obj));
    }

    #[test]
    fn test_lazy_document_preload_page_invalid_index() {
        let pdf_data = test_helpers::create_minimal_pdf();
        let cursor = Cursor::new(pdf_data);
        let reader = PdfReader::new(cursor).unwrap();
        let options = MemoryOptions::default();

        let lazy_doc = LazyDocument::new(reader, options).unwrap();

        let result = lazy_doc.preload_page(0);
        assert!(result.is_err());

        match result {
            Err(PdfError::InvalidPageNumber(num)) => {
                assert_eq!(num, 0);
            }
            _ => panic!("Expected InvalidPageNumber error"),
        }
    }

    #[test]
    fn test_lazy_document_memory_stats_initial() {
        let pdf_data = test_helpers::create_minimal_pdf();
        let cursor = Cursor::new(pdf_data);
        let reader = PdfReader::new(cursor).unwrap();
        let options = MemoryOptions::default();

        let lazy_doc = LazyDocument::new(reader, options).unwrap();
        let stats = lazy_doc.memory_stats();

        // Initial stats should be zero
        assert_eq!(stats.cache_hits, 0);
        assert_eq!(stats.cache_misses, 0);
        assert_eq!(stats.allocated_bytes, 0);
    }

    #[test]
    fn test_lazy_document_memory_stats_after_operations() {
        let pdf_data = test_helpers::create_minimal_pdf();
        let cursor = Cursor::new(pdf_data);
        let reader = PdfReader::new(cursor).unwrap();
        let options = MemoryOptions::default().with_cache_size(10);

        let lazy_doc = LazyDocument::new(reader, options).unwrap();
        let obj_id = ObjectId::new(1, 0);

        // Try to get an object (should result in cache miss)
        let _ = lazy_doc.get_object(&obj_id);

        let stats = lazy_doc.memory_stats();
        assert!(stats.cache_misses > 0);
    }

    #[test]
    fn test_lazy_document_clear_cache_empty() {
        let pdf_data = test_helpers::create_minimal_pdf();
        let cursor = Cursor::new(pdf_data);
        let reader = PdfReader::new(cursor).unwrap();
        let options = MemoryOptions::default();

        let lazy_doc = LazyDocument::new(reader, options).unwrap();

        // Clear cache on empty document should not panic
        lazy_doc.clear_cache();

        // Verify object map is still accessible
        let map = lazy_doc.object_map.read().unwrap();
        assert!(map.is_empty());
    }

    #[test]
    fn test_lazy_document_clear_cache_with_objects() {
        let pdf_data = test_helpers::create_minimal_pdf();
        let cursor = Cursor::new(pdf_data);
        let reader = PdfReader::new(cursor).unwrap();
        let options = MemoryOptions::default().with_cache_size(10);

        let lazy_doc = LazyDocument::new(reader, options).unwrap();
        let obj_id1 = ObjectId::new(1, 0);
        let obj_id2 = ObjectId::new(2, 0);
        let obj_id3 = ObjectId::new(3, 0);

        // Add different types of objects
        {
            let mut map = lazy_doc.object_map.write().unwrap();
            map.insert(obj_id1, LazyObject::NotLoaded { offset: 100 });
            map.insert(
                obj_id2,
                LazyObject::Loaded(Arc::new(PdfObject::Integer(42))),
            );
            map.insert(obj_id3, LazyObject::Loading);
        }

        lazy_doc.clear_cache();

        // Verify that only NotLoaded objects remain
        {
            let map = lazy_doc.object_map.read().unwrap();
            assert_eq!(map.len(), 1); // Only NotLoaded should remain
            assert!(matches!(
                map.get(&obj_id1),
                Some(LazyObject::NotLoaded { .. })
            ));
            assert!(!map.contains_key(&obj_id2));
            assert!(!map.contains_key(&obj_id3));
        }
    }

    #[test]
    fn test_lazy_page_iterator_creation() {
        let pdf_data = test_helpers::create_minimal_pdf();
        let cursor = Cursor::new(pdf_data);
        let reader = PdfReader::new(cursor).unwrap();
        let options = MemoryOptions::default();

        let lazy_doc = Arc::new(LazyDocument::new(reader, options).unwrap());
        let iterator = LazyPageIterator::new(lazy_doc.clone());

        assert_eq!(iterator.current, 0);
        assert_eq!(iterator.total, 0); // Minimal PDF has 0 pages
        assert!(Arc::ptr_eq(&iterator.document, &lazy_doc));
    }

    #[test]
    fn test_lazy_page_iterator_empty_document() {
        let pdf_data = test_helpers::create_minimal_pdf();
        let cursor = Cursor::new(pdf_data);
        let reader = PdfReader::new(cursor).unwrap();
        let options = MemoryOptions::default();

        let lazy_doc = Arc::new(LazyDocument::new(reader, options).unwrap());
        let mut iterator = LazyPageIterator::new(lazy_doc);

        // Should immediately return None for empty document
        assert!(iterator.next().is_none());
        assert!(iterator.next().is_none()); // Multiple calls should be safe
    }

    #[test]
    fn test_lazy_page_iterator_multiple_calls() {
        let pdf_data = test_helpers::create_minimal_pdf();
        let cursor = Cursor::new(pdf_data);
        let reader = PdfReader::new(cursor).unwrap();
        let options = MemoryOptions::default();

        let lazy_doc = Arc::new(LazyDocument::new(reader, options).unwrap());
        let mut iterator = LazyPageIterator::new(lazy_doc);

        // Multiple calls on empty iterator should return None
        for _ in 0..5 {
            assert!(iterator.next().is_none());
        }
    }

    #[test]
    fn test_lazy_document_with_different_memory_options() {
        let pdf_data = test_helpers::create_minimal_pdf();
        let cursor = Cursor::new(pdf_data);
        let reader = PdfReader::new(cursor).unwrap();

        // Test with custom memory options
        let options = MemoryOptions::default().with_cache_size(100);

        let lazy_doc = LazyDocument::new(reader, options).unwrap();

        assert_eq!(lazy_doc.page_count(), 0);

        let stats = lazy_doc.memory_stats();
        assert_eq!(stats.cache_hits, 0);
        assert_eq!(stats.cache_misses, 0);
    }

    #[test]
    fn test_lazy_document_load_object_at_offset() {
        let pdf_data = test_helpers::create_minimal_pdf();
        let cursor = Cursor::new(pdf_data);
        let reader = PdfReader::new(cursor).unwrap();
        let options = MemoryOptions::default();

        let lazy_doc = LazyDocument::new(reader, options).unwrap();

        // Test the private method through public interface
        let obj_id = ObjectId::new(1, 0);

        // Add NotLoaded object to trigger load_object_at_offset
        {
            let mut map = lazy_doc.object_map.write().unwrap();
            map.insert(obj_id, LazyObject::NotLoaded { offset: 1234 });
        }

        let result = lazy_doc.get_object(&obj_id);
        assert!(result.is_ok());

        let obj = result.unwrap();
        assert_eq!(*obj, PdfObject::Null); // Current implementation returns Null
    }

    #[test]
    fn test_lazy_document_cache_hit_path() {
        let pdf_data = test_helpers::create_minimal_pdf();
        let cursor = Cursor::new(pdf_data);
        let reader = PdfReader::new(cursor).unwrap();
        let options = MemoryOptions::default().with_cache_size(10);

        let lazy_doc = LazyDocument::new(reader, options).unwrap();
        let obj_id = ObjectId::new(1, 0);
        let test_obj = Arc::new(PdfObject::Real(3.14));

        // Manually add object to cache
        if let Some(cache) = lazy_doc.memory_manager.cache() {
            cache.put(obj_id, test_obj.clone());
        }

        let result = lazy_doc.get_object(&obj_id);
        assert!(result.is_ok());

        let obj = result.unwrap();
        assert_eq!(*obj, PdfObject::Real(3.14));
        assert!(Arc::ptr_eq(&obj, &test_obj));

        // Should have recorded a cache hit
        let stats = lazy_doc.memory_stats();
        assert!(stats.cache_hits > 0);
    }

    #[test]
    fn test_lazy_document_object_map_locking() {
        let pdf_data = test_helpers::create_minimal_pdf();
        let cursor = Cursor::new(pdf_data);
        let reader = PdfReader::new(cursor).unwrap();
        let options = MemoryOptions::default();

        let lazy_doc = LazyDocument::new(reader, options).unwrap();
        let obj_id = ObjectId::new(1, 0);

        // Test that we can acquire read and write locks
        {
            let _read_lock = lazy_doc.object_map.read().unwrap();
        }

        {
            let mut write_lock = lazy_doc.object_map.write().unwrap();
            write_lock.insert(obj_id, LazyObject::NotLoaded { offset: 500 });
        }

        // Verify the insertion worked
        {
            let read_lock = lazy_doc.object_map.read().unwrap();
            assert!(read_lock.contains_key(&obj_id));
        }
    }

    #[test]
    fn test_lazy_document_open_nonexistent_file() {
        let nonexistent_path = "/path/that/does/not/exist.pdf";
        let options = MemoryOptions::default();

        let result = LazyDocument::open(nonexistent_path, options);
        assert!(result.is_err());

        match result {
            Err(PdfError::ParseError(_)) => {}
            _ => panic!("Expected ParseError for nonexistent file"),
        }
    }

    #[test]
    fn test_lazy_object_enum_all_variants() {
        // Test all variants of LazyObject enum
        let variants = vec![
            LazyObject::NotLoaded { offset: 12345 },
            LazyObject::Loaded(Arc::new(PdfObject::String(crate::parser::PdfString::new(
                b"test".to_vec(),
            )))),
            LazyObject::Loading,
        ];

        for (i, variant) in variants.into_iter().enumerate() {
            match variant {
                LazyObject::NotLoaded { offset } if i == 0 => {
                    assert_eq!(offset, 12345);
                }
                LazyObject::Loaded(obj) if i == 1 => match &*obj {
                    PdfObject::String(_) => {}
                    _ => panic!("Expected String object"),
                },
                LazyObject::Loading if i == 2 => {
                    // Success
                }
                _ => panic!("Unexpected variant at index {i}"),
            }
        }
    }

    #[test]
    fn test_lazy_page_iterator_with_document_reference() {
        let pdf_data = test_helpers::create_minimal_pdf();
        let cursor = Cursor::new(pdf_data);
        let reader = PdfReader::new(cursor).unwrap();
        let options = MemoryOptions::default();

        let lazy_doc = Arc::new(LazyDocument::new(reader, options).unwrap());
        let _iterator = LazyPageIterator::new(lazy_doc.clone());

        // Verify the document is still accessible after creating iterator
        assert_eq!(lazy_doc.page_count(), 0);
    }

    #[test]
    fn test_lazy_document_concurrent_access_simulation() {
        use std::sync::Arc;
        use std::thread;

        let pdf_data = test_helpers::create_minimal_pdf();
        let cursor = Cursor::new(pdf_data);
        let reader = PdfReader::new(cursor).unwrap();
        let options = MemoryOptions::default().with_cache_size(10);

        let lazy_doc = Arc::new(LazyDocument::new(reader, options).unwrap());
        let mut handles = vec![];

        // Simulate concurrent access
        for i in 0..3 {
            let doc_clone = lazy_doc.clone();
            let handle = thread::spawn(move || {
                let obj_id = ObjectId::new(i + 1, 0);

                // Try to get object (will fail but shouldn't panic)
                let _result = doc_clone.get_object(&obj_id);

                // Get memory stats
                let _stats = doc_clone.memory_stats();

                // Clear cache
                doc_clone.clear_cache();
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.join().unwrap();
        }

        // Document should still be accessible
        assert_eq!(lazy_doc.page_count(), 0);
    }
}
