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
    pub fn mark(&mut self, entity_type: EntityType, bounds: (f64, f64, f64, f64)) -> EntityBuilder {
        EntityBuilder::new(self.page, entity_type, bounds)
    }

    /// Mark text region
    pub fn mark_text(&mut self, bounds: (f64, f64, f64, f64)) -> EntityBuilder {
        self.mark(EntityType::Text, bounds)
    }

    /// Mark image region
    pub fn mark_image(&mut self, bounds: (f64, f64, f64, f64)) -> EntityBuilder {
        self.mark(EntityType::Image, bounds)
    }

    /// Mark table region
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
