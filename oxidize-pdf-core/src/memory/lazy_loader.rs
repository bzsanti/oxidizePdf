//! Lazy loading implementation for PDF documents
//!
//! Provides on-demand loading of PDF objects and pages to minimize memory usage
//! when working with large documents.

use crate::error::{PdfError, Result};
use crate::memory::{MemoryManager, MemoryOptions};
use crate::objects::ObjectId;
use crate::parser::{ParsedPage, PdfDocument, PdfObject, PdfReader};
use std::collections::HashSet;
use std::fs::File;
use std::io::{Read, Seek};
use std::sync::{Arc, Mutex};

/// Public object-loading state representation.
///
/// `LazyDocument` delegates loading to the parser and does not expose or use
/// these states internally.
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
    // Indirection preserves Unpin for readers that are not Unpin.
    document: Arc<Mutex<PdfDocument<R>>>,
    memory_manager: MemoryManager,
    object_references: HashSet<(u32, u16)>,
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
    /// Create a document from the reader's page tree and object index.
    ///
    /// Construction reads page-tree dictionaries to establish the actual page
    /// count. Content streams and unrelated objects are parsed on demand.
    /// Reader parsing options and its existing caches are preserved.
    ///
    /// `cache_size` bounds this adapter's object cache, not the parser's caches
    /// or total process memory. Other memory options do not reconfigure the
    /// supplied reader.
    pub fn new(reader: PdfReader<R>, options: MemoryOptions) -> Result<Self> {
        let object_references = reader.object_references().into_iter().collect();
        let document = PdfDocument::new(reader);
        let page_count = document
            .page_count()
            .map_err(|e| PdfError::ParseError(e.to_string()))?;
        Ok(Self {
            document: Arc::new(Mutex::new(document)),
            memory_manager: MemoryManager::new(options),
            object_references,
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

        self.document
            .lock()
            .map_err(|_| PdfError::ParseError("Lazy document lock poisoned".into()))?
            .get_page(index)
            .map_err(|e| PdfError::ParseError(e.to_string()))
    }

    /// Get an indexed object by ID, parsing it on first access.
    ///
    /// Missing, free and mismatched-generation references return
    /// `InvalidObjectReference`, even when the reader uses lenient syntax.
    pub fn get_object(&self, id: &ObjectId) -> Result<Arc<PdfObject>> {
        if !self
            .object_references
            .contains(&(id.number(), id.generation()))
        {
            return Err(PdfError::InvalidObjectReference(
                id.number(),
                id.generation(),
            ));
        }
        if let Some(cache) = self.memory_manager.cache() {
            if let Some(obj) = cache.get(id) {
                self.memory_manager.record_cache_hit();
                return Ok(obj);
            }
        }
        self.memory_manager.record_cache_miss();
        let obj = self
            .document
            .lock()
            .map_err(|_| PdfError::ParseError("Lazy document lock poisoned".into()))?
            .get_object(id.number(), id.generation())
            .map_err(|e| PdfError::ParseError(e.to_string()))?;
        let obj = Arc::new(obj);
        if let Some(cache) = self.memory_manager.cache() {
            cache.put(*id, obj.clone());
        }
        Ok(obj)
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
                        self.get_object(&id)?;
                    }
                }
            }

            // Preload XObjects (images)
            if let Some(xobjects) = resources.get("XObject").and_then(|x| x.as_dict()) {
                for xobj_ref in xobjects.0.values() {
                    if let PdfObject::Reference(num, gen) = xobj_ref {
                        let id = ObjectId::new(*num, *gen);
                        self.get_object(&id)?;
                    }
                }
            }
        }

        Ok(())
    }

    /// Get adapter cache-access counters. These do not measure allocations or
    /// caches retained by the underlying parser.
    pub fn memory_stats(&self) -> crate::memory::MemoryStats {
        self.memory_manager.stats()
    }

    /// Clear this adapter's object cache. Parsed values retained by the
    /// underlying parser and values held by callers are unaffected.
    /// References remain accessible after clearing or eviction.
    pub fn clear_cache(&self) {
        if let Some(cache) = self.memory_manager.cache() {
            cache.clear();
        }
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

        assert!(lazy_doc
            .get_object(&ObjectId::new(1, 0))
            .unwrap()
            .as_dict()
            .is_some());
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
