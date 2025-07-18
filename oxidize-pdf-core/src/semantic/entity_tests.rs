//! Tests for entity types and metadata

#[cfg(test)]
mod tests {
    use super::super::entity::*;
    use std::collections::HashMap;

    #[test]
    fn test_entity_type_serialization() {
        // Test JSON serialization of entity types
        let entity_type = EntityType::Heading;
        let json = serde_json::to_string(&entity_type).unwrap();
        assert_eq!(json, r#""heading""#);

        let entity_type = EntityType::PageNumber;
        let json = serde_json::to_string(&entity_type).unwrap();
        assert_eq!(json, r#""pageNumber""#);
    }

    #[test]
    fn test_entity_type_deserialization() {
        let entity_type: EntityType = serde_json::from_str(r#""paragraph""#).unwrap();
        assert_eq!(entity_type, EntityType::Paragraph);

        let entity_type: EntityType = serde_json::from_str(r#""table""#).unwrap();
        assert_eq!(entity_type, EntityType::Table);
    }

    #[test]
    fn test_entity_type_equality() {
        assert_eq!(EntityType::Text, EntityType::Text);
        assert_ne!(EntityType::Text, EntityType::Image);
        assert_eq!(EntityType::Header, EntityType::Header);
    }

    #[test]
    fn test_entity_metadata_new() {
        let metadata = EntityMetadata::new();
        assert!(metadata.properties.is_empty());
        assert_eq!(metadata.confidence, None);
        assert_eq!(metadata.schema, None);
    }

    #[test]
    fn test_entity_metadata_with_property() {
        let metadata = EntityMetadata::new()
            .with_property("author", "John Doe")
            .with_property("date", "2024-01-01");

        assert_eq!(
            metadata.properties.get("author"),
            Some(&"John Doe".to_string())
        );
        assert_eq!(
            metadata.properties.get("date"),
            Some(&"2024-01-01".to_string())
        );
    }

    #[test]
    fn test_entity_metadata_with_confidence() {
        let metadata = EntityMetadata::new().with_confidence(0.95);
        assert_eq!(metadata.confidence, Some(0.95));

        // Test clamping
        let metadata = EntityMetadata::new().with_confidence(1.5);
        assert_eq!(metadata.confidence, Some(1.0));

        let metadata = EntityMetadata::new().with_confidence(-0.5);
        assert_eq!(metadata.confidence, Some(0.0));
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
            .with_property("title", "Test Document")
            .with_property("version", "1.0")
            .with_confidence(0.85)
            .with_schema("https://example.com/schema");

        assert_eq!(metadata.properties.len(), 2);
        assert_eq!(metadata.confidence, Some(0.85));
        assert!(metadata.schema.is_some());
    }

    #[test]
    fn test_entity_metadata_serialization() {
        let mut metadata = EntityMetadata::new();
        metadata
            .properties
            .insert("key1".to_string(), "value1".to_string());
        metadata.confidence = Some(0.9);
        metadata.schema = Some("test_schema".to_string());

        let json = serde_json::to_string(&metadata).unwrap();
        let deserialized: EntityMetadata = serde_json::from_str(&json).unwrap();

        assert_eq!(
            deserialized.properties.get("key1"),
            Some(&"value1".to_string())
        );
        assert_eq!(deserialized.confidence, Some(0.9));
        assert_eq!(deserialized.schema, Some("test_schema".to_string()));
    }

    #[test]
    fn test_entity_new() {
        let entity = Entity::new(
            "entity1".to_string(),
            EntityType::Paragraph,
            (10.0, 20.0, 100.0, 50.0),
            0,
        );

        assert_eq!(entity.id, "entity1");
        assert_eq!(entity.entity_type, EntityType::Paragraph);
        assert_eq!(entity.bounds, (10.0, 20.0, 100.0, 50.0));
        assert_eq!(entity.page, 0);
        assert!(entity.metadata.properties.is_empty());
    }

    #[test]
    fn test_entity_with_metadata() {
        let mut entity = Entity::new(
            "table1".to_string(),
            EntityType::Table,
            (50.0, 100.0, 200.0, 150.0),
            2,
        );

        entity.metadata = EntityMetadata::new()
            .with_property("rows", "5")
            .with_property("columns", "3")
            .with_confidence(0.92);

        assert_eq!(
            entity.metadata.properties.get("rows"),
            Some(&"5".to_string())
        );
        assert_eq!(
            entity.metadata.properties.get("columns"),
            Some(&"3".to_string())
        );
        assert_eq!(entity.metadata.confidence, Some(0.92));
    }

    #[test]
    fn test_entity_serialization() {
        let mut entity = Entity::new(
            "heading1".to_string(),
            EntityType::Heading,
            (0.0, 0.0, 612.0, 72.0),
            0,
        );
        entity.metadata = EntityMetadata::new()
            .with_property("level", "1")
            .with_confidence(0.99);

        let json = serde_json::to_string(&entity).unwrap();
        assert!(json.contains(r#""id":"heading1""#));
        assert!(json.contains(r#""type":"heading""#)); // Note: renamed from entity_type
        assert!(json.contains(r#""bounds":[0.0,0.0,612.0,72.0]"#));
        assert!(json.contains(r#""page":0"#));
        assert!(json.contains(r#""confidence":0.99"#));
    }

    #[test]
    fn test_entity_deserialization() {
        let json = r#"{
            "id": "img1",
            "type": "image",
            "bounds": [100.0, 200.0, 300.0, 400.0],
            "page": 1,
            "metadata": {
                "properties": {"alt": "Logo", "format": "JPEG"},
                "confidence": 0.88,
                "schema": null
            }
        }"#;

        let entity: Entity = serde_json::from_str(json).unwrap();
        assert_eq!(entity.id, "img1");
        assert_eq!(entity.entity_type, EntityType::Image);
        assert_eq!(entity.bounds, (100.0, 200.0, 300.0, 400.0));
        assert_eq!(entity.page, 1);
        assert_eq!(
            entity.metadata.properties.get("alt"),
            Some(&"Logo".to_string())
        );
        assert_eq!(
            entity.metadata.properties.get("format"),
            Some(&"JPEG".to_string())
        );
        assert_eq!(entity.metadata.confidence, Some(0.88));
    }

    #[test]
    fn test_all_entity_types() {
        // Ensure all entity types can be created and compared
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

        for entity_type in &types {
            let entity = Entity::new(
                format!("test_{:?}", entity_type),
                *entity_type,
                (0.0, 0.0, 100.0, 100.0),
                0,
            );
            assert_eq!(entity.entity_type, *entity_type);
        }
    }

    #[test]
    fn test_entity_bounds_validation() {
        // Test various bound configurations
        let bounds_tests = vec![
            (0.0, 0.0, 100.0, 100.0),     // Normal bounds
            (-10.0, -10.0, 100.0, 100.0), // Negative position
            (0.0, 0.0, 0.0, 0.0),         // Zero size
            (500.0, 700.0, 50.0, 25.0),   // Off-page bounds
        ];

        for bounds in bounds_tests {
            let entity = Entity::new("test".to_string(), EntityType::Text, bounds, 0);
            assert_eq!(entity.bounds, bounds);
        }
    }

    #[test]
    fn test_entity_metadata_edge_cases() {
        // Empty property key/value
        let metadata = EntityMetadata::new()
            .with_property("", "empty_key")
            .with_property("empty_value", "");

        assert_eq!(metadata.properties.get(""), Some(&"empty_key".to_string()));
        assert_eq!(
            metadata.properties.get("empty_value"),
            Some(&"".to_string())
        );

        // Very long strings
        let long_key = "k".repeat(1000);
        let long_value = "v".repeat(10000);
        let metadata = EntityMetadata::new().with_property(long_key.clone(), long_value.clone());

        assert_eq!(metadata.properties.get(&long_key), Some(&long_value));
    }

    #[test]
    fn test_entity_clone() {
        let mut entity = Entity::new(
            "original".to_string(),
            EntityType::List,
            (10.0, 20.0, 30.0, 40.0),
            5,
        );
        entity.metadata = EntityMetadata::new()
            .with_property("items", "10")
            .with_confidence(0.75);

        let cloned = entity.clone();
        assert_eq!(cloned.id, entity.id);
        assert_eq!(cloned.entity_type, entity.entity_type);
        assert_eq!(cloned.bounds, entity.bounds);
        assert_eq!(cloned.page, entity.page);
        assert_eq!(cloned.metadata.properties, entity.metadata.properties);
        assert_eq!(cloned.metadata.confidence, entity.metadata.confidence);
    }
}
