//! Font caching for efficient font management

use super::Font;
use crate::Result;
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
        let mut fonts = self.fonts.write().unwrap();
        fonts.insert(name, Arc::new(font));
        Ok(())
    }

    /// Get a font from the cache
    pub fn get_font(&self, name: &str) -> Option<Arc<Font>> {
        let fonts = self.fonts.read().unwrap();
        fonts.get(name).cloned()
    }

    /// Check if a font exists in the cache
    pub fn has_font(&self, name: &str) -> bool {
        let fonts = self.fonts.read().unwrap();
        fonts.contains_key(name)
    }

    /// Get all font names in the cache
    pub fn font_names(&self) -> Vec<String> {
        let fonts = self.fonts.read().unwrap();
        fonts.keys().cloned().collect()
    }

    /// Clear the cache
    pub fn clear(&self) {
        let mut fonts = self.fonts.write().unwrap();
        fonts.clear();
    }

    /// Get the number of cached fonts
    pub fn len(&self) -> usize {
        let fonts = self.fonts.read().unwrap();
        fonts.len()
    }

    /// Check if the cache is empty
    pub fn is_empty(&self) -> bool {
        let fonts = self.fonts.read().unwrap();
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
}
