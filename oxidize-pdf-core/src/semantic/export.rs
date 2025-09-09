//! Export functionality for semantic entities

use super::Entity;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Map of entities organized by page
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityMap {
    /// Document-level metadata
    pub document_metadata: HashMap<String, String>,
    /// Entities organized by page number
    pub pages: HashMap<usize, Vec<Entity>>,
    /// Schema definitions used
    pub schemas: Vec<String>,
}

impl Default for EntityMap {
    fn default() -> Self {
        Self::new()
    }
}

impl EntityMap {
    pub fn new() -> Self {
        Self {
            document_metadata: HashMap::new(),
            pages: HashMap::new(),
            schemas: Vec::new(),
        }
    }

    /// Add an entity to the map
    pub fn add_entity(&mut self, entity: Entity) {
        self.pages.entry(entity.page).or_default().push(entity);
    }

    /// Export to JSON string (requires serde_json feature)
    #[cfg(feature = "semantic")]
    #[allow(unexpected_cfgs)]
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    /// Export to JSON with custom options (requires serde_json feature)
    #[cfg(feature = "semantic")]
    #[allow(unexpected_cfgs)]
    pub fn to_json_compact(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }

    /// Get all entities of a specific type
    pub fn entities_by_type(&self, entity_type: super::EntityType) -> Vec<&Entity> {
        self.pages
            .values()
            .flat_map(|entities| entities.iter())
            .filter(|e| e.entity_type == entity_type)
            .collect()
    }

    /// Get all entities on a specific page
    pub fn entities_on_page(&self, page: usize) -> Option<&Vec<Entity>> {
        self.pages.get(&page)
    }
}

/// Export format options
#[derive(Debug, Clone, Copy)]
pub enum ExportFormat {
    /// JSON format (default)
    Json,
    /// JSON-LD with schema.org context
    JsonLd,
    /// XML format
    Xml,
}

impl Default for ExportFormat {
    fn default() -> Self {
        Self::Json
    }
}
