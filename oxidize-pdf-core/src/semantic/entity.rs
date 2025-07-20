//! Entity types and metadata for semantic marking

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Standard entity types available in all editions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum EntityType {
    /// Generic text region
    Text,
    /// Image or graphic
    Image,
    /// Table structure
    Table,
    /// Heading/Title
    Heading,
    /// Paragraph of text
    Paragraph,
    /// List (ordered or unordered)
    List,
    /// Page number
    PageNumber,
    /// Header region
    Header,
    /// Footer region
    Footer,
}

/// Metadata associated with an entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityMetadata {
    /// Key-value pairs of metadata
    pub properties: HashMap<String, String>,
    /// Confidence score (0.0 to 1.0)
    pub confidence: Option<f32>,
    /// Schema URL if applicable
    pub schema: Option<String>,
}

impl EntityMetadata {
    pub fn new() -> Self {
        Self {
            properties: HashMap::new(),
            confidence: None,
            schema: None,
        }
    }

    pub fn with_property(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.properties.insert(key.into(), value.into());
        self
    }

    pub fn with_confidence(mut self, confidence: f32) -> Self {
        self.confidence = Some(confidence.clamp(0.0, 1.0));
        self
    }

    pub fn with_schema(mut self, schema: impl Into<String>) -> Self {
        self.schema = Some(schema.into());
        self
    }
}

/// A marked entity in the PDF
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entity {
    /// Unique identifier for this entity
    pub id: String,
    /// Type of entity
    #[serde(rename = "type")]
    pub entity_type: EntityType,
    /// Bounding box (x, y, width, height)
    pub bounds: (f64, f64, f64, f64),
    /// Page number (0-indexed)
    pub page: usize,
    /// Associated metadata
    pub metadata: EntityMetadata,
}

impl Entity {
    pub fn new(
        id: String,
        entity_type: EntityType,
        bounds: (f64, f64, f64, f64),
        page: usize,
    ) -> Self {
        Self {
            id,
            entity_type,
            bounds,
            page,
            metadata: EntityMetadata::new(),
        }
    }
}
