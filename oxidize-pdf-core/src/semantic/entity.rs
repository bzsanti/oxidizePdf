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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_entity_type_variants() {
        let types = vec![
            EntityType::Text,
            EntityType::Image,
            EntityType::Table,
            EntityType::Heading,
            EntityType::Paragraph,
            EntityType::List,
            EntityType::PageNumber,
            EntityType::Header,
            EntityType::Footer,
        ];

        for entity_type in types {
            match entity_type {
                EntityType::Text => assert_eq!(entity_type, EntityType::Text),
                EntityType::Image => assert_eq!(entity_type, EntityType::Image),
                EntityType::Table => assert_eq!(entity_type, EntityType::Table),
                EntityType::Heading => assert_eq!(entity_type, EntityType::Heading),
                EntityType::Paragraph => assert_eq!(entity_type, EntityType::Paragraph),
                EntityType::List => assert_eq!(entity_type, EntityType::List),
                EntityType::PageNumber => assert_eq!(entity_type, EntityType::PageNumber),
                EntityType::Header => assert_eq!(entity_type, EntityType::Header),
                EntityType::Footer => assert_eq!(entity_type, EntityType::Footer),
            }
        }
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
            .with_property("author", "John Doe")
            .with_property("title", "Test Document");

        assert_eq!(metadata.properties.len(), 2);
        assert_eq!(
            metadata.properties.get("author"),
            Some(&"John Doe".to_string())
        );
        assert_eq!(
            metadata.properties.get("title"),
            Some(&"Test Document".to_string())
        );
    }

    #[test]
    fn test_entity_metadata_with_confidence() {
        let metadata = EntityMetadata::new().with_confidence(0.95);
        assert_eq!(metadata.confidence, Some(0.95));

        // Test clamping
        let metadata_high = EntityMetadata::new().with_confidence(1.5);
        assert_eq!(metadata_high.confidence, Some(1.0));

        let metadata_low = EntityMetadata::new().with_confidence(-0.5);
        assert_eq!(metadata_low.confidence, Some(0.0));
    }

    #[test]
    fn test_entity_metadata_with_schema() {
        let metadata = EntityMetadata::new().with_schema("https://schema.org/Article");
        assert_eq!(
            metadata.schema,
            Some("https://schema.org/Article".to_string())
        );
    }

    #[test]
    fn test_entity_metadata_builder_chain() {
        let metadata = EntityMetadata::new()
            .with_property("lang", "en")
            .with_property("version", "1.0")
            .with_confidence(0.85)
            .with_schema("https://example.com/schema");

        assert_eq!(metadata.properties.len(), 2);
        assert_eq!(metadata.confidence, Some(0.85));
        assert!(metadata.schema.is_some());
    }

    #[test]
    fn test_entity_new() {
        let entity = Entity::new(
            "entity-1".to_string(),
            EntityType::Paragraph,
            (10.0, 20.0, 100.0, 50.0),
            0,
        );

        assert_eq!(entity.id, "entity-1");
        assert_eq!(entity.entity_type, EntityType::Paragraph);
        assert_eq!(entity.bounds, (10.0, 20.0, 100.0, 50.0));
        assert_eq!(entity.page, 0);
        assert!(entity.metadata.properties.is_empty());
    }

    #[test]
    fn test_entity_with_metadata() {
        let mut entity = Entity::new(
            "heading-1".to_string(),
            EntityType::Heading,
            (0.0, 0.0, 200.0, 30.0),
            1,
        );

        entity.metadata = EntityMetadata::new()
            .with_property("level", "1")
            .with_property("text", "Introduction")
            .with_confidence(0.98);

        assert_eq!(
            entity.metadata.properties.get("level"),
            Some(&"1".to_string())
        );
        assert_eq!(
            entity.metadata.properties.get("text"),
            Some(&"Introduction".to_string())
        );
        assert_eq!(entity.metadata.confidence, Some(0.98));
    }

    #[test]
    fn test_entity_serialization() {
        let entity = Entity::new(
            "test-entity".to_string(),
            EntityType::Image,
            (50.0, 50.0, 150.0, 100.0),
            2,
        );

        // Test that entity can be serialized
        let json = serde_json::to_string(&entity).unwrap();
        assert!(json.contains("\"id\":\"test-entity\""));
        assert!(json.contains("\"type\":\"image\""));

        // Test deserialization
        let deserialized: Entity = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.id, entity.id);
        assert_eq!(deserialized.entity_type, entity.entity_type);
    }

    #[test]
    fn test_entity_type_serialization() {
        // Test that EntityType serializes to camelCase
        let entity_type = EntityType::PageNumber;
        let json = serde_json::to_string(&entity_type).unwrap();
        assert_eq!(json, "\"pageNumber\"");

        // Test deserialization
        let deserialized: EntityType = serde_json::from_str("\"pageNumber\"").unwrap();
        assert_eq!(deserialized, EntityType::PageNumber);
    }

    #[test]
    fn test_multiple_entities() {
        let entities = vec![
            Entity::new(
                "e1".to_string(),
                EntityType::Header,
                (0.0, 0.0, 100.0, 20.0),
                0,
            ),
            Entity::new(
                "e2".to_string(),
                EntityType::Paragraph,
                (0.0, 20.0, 100.0, 80.0),
                0,
            ),
            Entity::new(
                "e3".to_string(),
                EntityType::Footer,
                (0.0, 100.0, 100.0, 20.0),
                0,
            ),
        ];

        assert_eq!(entities.len(), 3);
        assert_eq!(entities[0].entity_type, EntityType::Header);
        assert_eq!(entities[1].entity_type, EntityType::Paragraph);
        assert_eq!(entities[2].entity_type, EntityType::Footer);
    }

    #[test]
    fn test_entity_bounds() {
        let entity = Entity::new(
            "table-1".to_string(),
            EntityType::Table,
            (25.5, 30.75, 200.25, 150.5),
            5,
        );

        let (x, y, width, height) = entity.bounds;
        assert_eq!(x, 25.5);
        assert_eq!(y, 30.75);
        assert_eq!(width, 200.25);
        assert_eq!(height, 150.5);
    }

    #[test]
    fn test_metadata_multiple_properties() {
        let mut metadata = EntityMetadata::new();

        // Add properties one by one
        for i in 0..10 {
            metadata
                .properties
                .insert(format!("key{}", i), format!("value{}", i));
        }

        assert_eq!(metadata.properties.len(), 10);
        assert_eq!(metadata.properties.get("key5"), Some(&"value5".to_string()));
    }

    #[test]
    fn test_entity_list_type() {
        let list_entity = Entity::new(
            "list-1".to_string(),
            EntityType::List,
            (10.0, 10.0, 180.0, 100.0),
            0,
        );

        // Add list-specific metadata
        let mut entity = list_entity;
        entity.metadata = EntityMetadata::new()
            .with_property("list_type", "ordered")
            .with_property("item_count", "5");

        assert_eq!(entity.entity_type, EntityType::List);
        assert_eq!(
            entity.metadata.properties.get("list_type"),
            Some(&"ordered".to_string())
        );
    }

    #[test]
    fn test_confidence_edge_cases() {
        // Test exact boundaries
        let metadata1 = EntityMetadata::new().with_confidence(0.0);
        assert_eq!(metadata1.confidence, Some(0.0));

        let metadata2 = EntityMetadata::new().with_confidence(1.0);
        assert_eq!(metadata2.confidence, Some(1.0));

        // Test normal value
        let metadata3 = EntityMetadata::new().with_confidence(0.5);
        assert_eq!(metadata3.confidence, Some(0.5));
    }
}
