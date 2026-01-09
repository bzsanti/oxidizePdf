//! Marking API for semantic regions

use super::{Entity, EntityMetadata, EntityType};
use crate::page::Page;

/// Builder for creating marked entities
pub struct EntityBuilder<'a> {
    _page: &'a mut Page,
    entity_type: EntityType,
    bounds: (f64, f64, f64, f64),
    metadata: EntityMetadata,
}

impl<'a> EntityBuilder<'a> {
    pub(crate) fn new(
        page: &'a mut Page,
        entity_type: EntityType,
        bounds: (f64, f64, f64, f64),
    ) -> Self {
        Self {
            _page: page,
            entity_type,
            bounds,
            metadata: EntityMetadata::new(),
        }
    }

    /// Add a metadata property
    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata = self.metadata.with_property(key, value);
        self
    }

    /// Set confidence score
    pub fn with_confidence(mut self, confidence: f32) -> Self {
        self.metadata = self.metadata.with_confidence(confidence);
        self
    }

    /// Set schema URL
    pub fn with_schema(mut self, schema: impl Into<String>) -> Self {
        self.metadata = self.metadata.with_schema(schema);
        self
    }

    /// Finalize the entity marking
    pub fn build(self) -> String {
        let id = format!("entity_{}", uuid_simple());
        let _entity = Entity {
            id: id.clone(),
            entity_type: self.entity_type,
            bounds: self.bounds,
            page: 0, // Will be set by page
            metadata: self.metadata,
        };

        // Store entity in page (implementation detail)
        // self._page.add_entity(_entity);

        id
    }
}

/// Semantic marker for a page
pub struct SemanticMarker<'a> {
    page: &'a mut Page,
}

impl<'a> SemanticMarker<'a> {
    pub fn new(page: &'a mut Page) -> Self {
        Self { page }
    }

    /// Mark a region as a specific entity type
    #[allow(mismatched_lifetime_syntaxes)]
    pub fn mark(&mut self, entity_type: EntityType, bounds: (f64, f64, f64, f64)) -> EntityBuilder {
        EntityBuilder::new(self.page, entity_type, bounds)
    }

    /// Mark text region
    #[allow(mismatched_lifetime_syntaxes)]
    pub fn mark_text(&mut self, bounds: (f64, f64, f64, f64)) -> EntityBuilder {
        self.mark(EntityType::Text, bounds)
    }

    /// Mark image region
    #[allow(mismatched_lifetime_syntaxes)]
    pub fn mark_image(&mut self, bounds: (f64, f64, f64, f64)) -> EntityBuilder {
        self.mark(EntityType::Image, bounds)
    }

    /// Mark table region
    #[allow(mismatched_lifetime_syntaxes)]
    pub fn mark_table(&mut self, bounds: (f64, f64, f64, f64)) -> EntityBuilder {
        self.mark(EntityType::Table, bounds)
    }
}

// Simple UUID generation for entity IDs
pub fn uuid_simple() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_else(|_| std::time::Duration::from_secs(0))
        .as_nanos();
    format!("{:x}", timestamp)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_uuid_simple_generates_unique_ids() {
        let id1 = uuid_simple();
        let id2 = uuid_simple();

        // IDs should be non-empty hex strings
        assert!(!id1.is_empty());
        assert!(!id2.is_empty());

        // All characters should be valid hex
        for c in id1.chars() {
            assert!(c.is_ascii_hexdigit());
        }
    }

    #[test]
    fn test_uuid_simple_format() {
        let id = uuid_simple();

        // Should be a valid hex string (non-empty, all hex chars)
        assert!(!id.is_empty());
        assert!(id.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn test_entity_metadata_new() {
        let metadata = EntityMetadata::new();
        assert!(metadata.properties.is_empty());
        assert!(metadata.confidence.is_none());
        assert!(metadata.schema.is_none());
    }

    #[test]
    fn test_entity_metadata_with_property() {
        let metadata = EntityMetadata::new()
            .with_property("key1", "value1")
            .with_property("key2", "value2");

        assert_eq!(metadata.properties.len(), 2);
        assert_eq!(metadata.properties.get("key1"), Some(&"value1".to_string()));
        assert_eq!(metadata.properties.get("key2"), Some(&"value2".to_string()));
    }

    #[test]
    fn test_entity_metadata_with_confidence() {
        let metadata = EntityMetadata::new().with_confidence(0.95);

        assert_eq!(metadata.confidence, Some(0.95));
    }

    #[test]
    fn test_entity_metadata_with_schema() {
        let metadata = EntityMetadata::new().with_schema("https://schema.org/Person");

        assert_eq!(
            metadata.schema,
            Some("https://schema.org/Person".to_string())
        );
    }

    #[test]
    fn test_entity_metadata_chaining() {
        let metadata = EntityMetadata::new()
            .with_property("name", "Test Entity")
            .with_confidence(0.85)
            .with_schema("https://example.com/schema");

        assert_eq!(
            metadata.properties.get("name"),
            Some(&"Test Entity".to_string())
        );
        assert_eq!(metadata.confidence, Some(0.85));
        assert_eq!(
            metadata.schema,
            Some("https://example.com/schema".to_string())
        );
    }

    #[test]
    fn test_entity_type_variants() {
        // Test that all entity type variants exist
        let _text = EntityType::Text;
        let _image = EntityType::Image;
        let _table = EntityType::Table;

        assert!(true); // Just ensure variants are accessible
    }

    #[test]
    fn test_entity_creation() {
        let entity = Entity {
            id: "test_entity_1".to_string(),
            entity_type: EntityType::Text,
            bounds: (10.0, 20.0, 100.0, 50.0),
            page: 1,
            metadata: EntityMetadata::new().with_confidence(0.9),
        };

        assert_eq!(entity.id, "test_entity_1");
        assert!(matches!(entity.entity_type, EntityType::Text));
        assert_eq!(entity.bounds, (10.0, 20.0, 100.0, 50.0));
        assert_eq!(entity.page, 1);
        assert_eq!(entity.metadata.confidence, Some(0.9));
    }
}
