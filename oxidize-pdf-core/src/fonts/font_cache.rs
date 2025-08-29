//! Font caching for efficient font management

use super::Font;
use crate::{PdfError, Result};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// Thread-safe font cache
#[derive(Debug, Clone)]
pub struct FontCache {
    fonts: Arc<RwLock<HashMap<String, Arc<Font>>>>,
}

impl FontCache {
    /// Create a new font cache
    pub fn new() -> Self {
        FontCache {
            fonts: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Add a font to the cache
    pub fn add_font(&self, name: impl Into<String>, font: Font) -> Result<()> {
        let name = name.into();
        let mut fonts = self
            .fonts
            .write()
            .map_err(|_| PdfError::InvalidOperation("Font cache lock is poisoned".to_string()))?;
        fonts.insert(name, Arc::new(font));
        Ok(())
    }

    /// Get a font from the cache
    pub fn get_font(&self, name: &str) -> Option<Arc<Font>> {
        let fonts = self.fonts.read().ok()?;
        fonts.get(name).cloned()
    }

    /// Check if a font exists in the cache
    pub fn has_font(&self, name: &str) -> bool {
        let Ok(fonts) = self.fonts.read() else {
            return false;
        };
        fonts.contains_key(name)
    }

    /// Get all font names in the cache
    pub fn font_names(&self) -> Vec<String> {
        let Ok(fonts) = self.fonts.read() else {
            return Vec::new();
        };
        fonts.keys().cloned().collect()
    }

    /// Clear the cache
    pub fn clear(&self) {
        if let Ok(mut fonts) = self.fonts.write() {
            fonts.clear();
        }
        // Silently ignore if lock is poisoned
    }

    /// Get the number of cached fonts
    pub fn len(&self) -> usize {
        let Ok(fonts) = self.fonts.read() else {
            return 0;
        };
        fonts.len()
    }

    /// Check if the cache is empty
    pub fn is_empty(&self) -> bool {
        let Ok(fonts) = self.fonts.read() else {
            return true;
        };
        fonts.is_empty()
    }
}

impl Default for FontCache {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fonts::{FontDescriptor, FontFormat, FontMetrics, GlyphMapping};

    fn create_test_font(name: &str) -> Font {
        Font {
            name: name.to_string(),
            data: vec![0; 100],
            format: FontFormat::TrueType,
            metrics: FontMetrics {
                units_per_em: 1000,
                ascent: 800,
                descent: -200,
                line_gap: 200,
                cap_height: 700,
                x_height: 500,
            },
            descriptor: FontDescriptor::new(name),
            glyph_mapping: GlyphMapping::default(),
        }
    }

    #[test]
    fn test_font_cache_basic_operations() {
        let cache = FontCache::new();

        // Add fonts
        let font1 = create_test_font("Font1");
        let font2 = create_test_font("Font2");

        cache.add_font("Font1", font1).unwrap();
        cache.add_font("Font2", font2).unwrap();

        // Check cache state
        assert_eq!(cache.len(), 2);
        assert!(!cache.is_empty());
        assert!(cache.has_font("Font1"));
        assert!(cache.has_font("Font2"));
        assert!(!cache.has_font("Font3"));

        // Get fonts
        let retrieved = cache.get_font("Font1").unwrap();
        assert_eq!(retrieved.name, "Font1");

        // Get font names
        let mut names = cache.font_names();
        names.sort();
        assert_eq!(names, vec!["Font1", "Font2"]);

        // Clear cache
        cache.clear();
        assert_eq!(cache.len(), 0);
        assert!(cache.is_empty());
    }

    #[test]
    fn test_font_cache_thread_safety() {
        use std::thread;

        let cache = FontCache::new();
        let cache_clone = cache.clone();

        // Add font from another thread
        let handle = thread::spawn(move || {
            let font = create_test_font("ThreadFont");
            cache_clone.add_font("ThreadFont", font).unwrap();
        });

        handle.join().unwrap();

        // Check font was added
        assert!(cache.has_font("ThreadFont"));
    }

    #[test]
    fn test_font_cache_default() {
        let cache = FontCache::default();
        assert!(cache.is_empty());
        assert_eq!(cache.len(), 0);
    }

    #[test]
    fn test_get_nonexistent_font() {
        let cache = FontCache::new();
        assert!(cache.get_font("NonExistent").is_none());
    }

    #[test]
    fn test_replace_font() {
        let cache = FontCache::new();

        // Add original font
        let font1 = create_test_font("Original");
        cache.add_font("TestFont", font1).unwrap();

        // Replace with new font
        let mut font2 = create_test_font("Replacement");
        font2.metrics.units_per_em = 2048; // Different value
        cache.add_font("TestFont", font2).unwrap();

        // Verify replacement
        let retrieved = cache.get_font("TestFont").unwrap();
        assert_eq!(retrieved.name, "Replacement");
        assert_eq!(retrieved.metrics.units_per_em, 2048);
        assert_eq!(cache.len(), 1); // Still only one font
    }

    #[test]
    fn test_multiple_threads_reading() {
        use std::thread;

        let cache = FontCache::new();
        let font = create_test_font("SharedFont");
        cache.add_font("SharedFont", font).unwrap();

        let mut handles = vec![];

        // Spawn multiple reader threads
        for i in 0..5 {
            let cache_clone = cache.clone();
            let handle = thread::spawn(move || {
                for _ in 0..10 {
                    let font = cache_clone.get_font("SharedFont");
                    assert!(font.is_some());
                    assert_eq!(font.unwrap().name, "SharedFont");
                }
                i
            });
            handles.push(handle);
        }

        // Wait for all threads to complete
        for handle in handles {
            handle.join().unwrap();
        }
    }

    #[test]
    fn test_multiple_threads_writing() {
        use std::thread;

        let cache = FontCache::new();
        let mut handles = vec![];

        // Spawn multiple writer threads
        for i in 0..5 {
            let cache_clone = cache.clone();
            let handle = thread::spawn(move || {
                let font_name = format!("Font{}", i);
                let font = create_test_font(&font_name);
                cache_clone.add_font(&font_name, font).unwrap();
            });
            handles.push(handle);
        }

        // Wait for all threads to complete
        for handle in handles {
            handle.join().unwrap();
        }

        // Verify all fonts were added
        assert_eq!(cache.len(), 5);
        for i in 0..5 {
            assert!(cache.has_font(&format!("Font{}", i)));
        }
    }

    #[test]
    fn test_font_names_empty_cache() {
        let cache = FontCache::new();
        assert_eq!(cache.font_names(), Vec::<String>::new());
    }

    #[test]
    fn test_font_names_ordering() {
        let cache = FontCache::new();

        // Add fonts in non-alphabetical order
        cache.add_font("Zebra", create_test_font("Zebra")).unwrap();
        cache.add_font("Alpha", create_test_font("Alpha")).unwrap();
        cache
            .add_font("Middle", create_test_font("Middle"))
            .unwrap();

        let mut names = cache.font_names();
        names.sort(); // Sort for consistent testing
        assert_eq!(names, vec!["Alpha", "Middle", "Zebra"]);
    }

    #[test]
    fn test_clear_and_reuse() {
        let cache = FontCache::new();

        // Add fonts
        cache.add_font("Font1", create_test_font("Font1")).unwrap();
        cache.add_font("Font2", create_test_font("Font2")).unwrap();
        assert_eq!(cache.len(), 2);

        // Clear
        cache.clear();
        assert_eq!(cache.len(), 0);
        assert!(cache.is_empty());

        // Reuse cache
        cache.add_font("Font3", create_test_font("Font3")).unwrap();
        assert_eq!(cache.len(), 1);
        assert!(cache.has_font("Font3"));
        assert!(!cache.has_font("Font1"));
    }

    #[test]
    fn test_arc_sharing() {
        let cache = FontCache::new();
        let font = create_test_font("SharedFont");
        cache.add_font("SharedFont", font).unwrap();

        // Get multiple Arc references
        let arc1 = cache.get_font("SharedFont").unwrap();
        let arc2 = cache.get_font("SharedFont").unwrap();

        // Both should point to the same font
        assert!(Arc::ptr_eq(&arc1, &arc2));
    }

    #[test]
    fn test_cache_with_special_names() {
        let cache = FontCache::new();

        // Test with various special characters in names
        let special_names = vec![
            "Font-Name",
            "Font.Name",
            "Font Name",
            "Font_Name",
            "Font/Name",
            "Font@Name",
            "日本語",
            "😀Font",
        ];

        for name in &special_names {
            cache.add_font(*name, create_test_font(name)).unwrap();
        }

        assert_eq!(cache.len(), special_names.len());

        for name in &special_names {
            assert!(cache.has_font(name));
            let font = cache.get_font(name).unwrap();
            assert_eq!(font.name, *name);
        }
    }

    #[test]
    fn test_cache_memory_efficiency() {
        let cache = FontCache::new();

        // Add same font data with different names
        for i in 0..100 {
            let font = create_test_font("TestFont");
            cache.add_font(format!("Font{}", i), font).unwrap();
        }

        assert_eq!(cache.len(), 100);

        // Clear should free all references
        cache.clear();
        assert_eq!(cache.len(), 0);
    }
}
