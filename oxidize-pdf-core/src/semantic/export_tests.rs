//! Tests for semantic entity export functionality

#[cfg(test)]
mod tests {
    use super::super::export::*;
    use super::super::{Entity, EntityMetadata, EntityType};
    use std::collections::HashMap;

    #[test]
    fn test_entity_map_new() {
        let map = EntityMap::new();
        assert!(map.document_metadata.is_empty());
        assert!(map.pages.is_empty());
        assert!(map.schemas.is_empty());
    }

    #[test]
    fn test_entity_map_add_entity() {
        let mut map = EntityMap::new();

        let entity1 = Entity::new(
            "e1".to_string(),
            EntityType::Heading,
            (0.0, 0.0, 100.0, 20.0),
            0,
        );

        let entity2 = Entity::new(
            "e2".to_string(),
            EntityType::Paragraph,
            (0.0, 30.0, 100.0, 50.0),
            0,
        );

        let entity3 = Entity::new(
            "e3".to_string(),
            EntityType::Image,
            (0.0, 0.0, 200.0, 200.0),
            1,
        );

        map.add_entity(entity1);
        map.add_entity(entity2);
        map.add_entity(entity3);

        assert_eq!(map.pages.len(), 2);
        assert_eq!(map.pages.get(&0).unwrap().len(), 2);
        assert_eq!(map.pages.get(&1).unwrap().len(), 1);
    }

    #[test]
    fn test_entity_map_entities_on_page() {
        let mut map = EntityMap::new();

        let entity = Entity::new(
            "test".to_string(),
            EntityType::Text,
            (0.0, 0.0, 100.0, 100.0),
            5,
        );

        map.add_entity(entity);

        assert!(map.entities_on_page(5).is_some());
        assert_eq!(map.entities_on_page(5).unwrap().len(), 1);
        assert!(map.entities_on_page(0).is_none());
        assert!(map.entities_on_page(10).is_none());
    }

    #[test]
    fn test_entity_map_entities_by_type() {
        let mut map = EntityMap::new();

        // Add various entity types
        for i in 0..3 {
            map.add_entity(Entity::new(
                format!("heading{}", i),
                EntityType::Heading,
                (0.0, i as f64 * 30.0, 100.0, 20.0),
                i,
            ));
        }

        for i in 0..2 {
            map.add_entity(Entity::new(
                format!("para{}", i),
                EntityType::Paragraph,
                (0.0, i as f64 * 50.0, 100.0, 40.0),
                0,
            ));
        }

        map.add_entity(Entity::new(
            "table1".to_string(),
            EntityType::Table,
            (0.0, 100.0, 200.0, 150.0),
            1,
        ));

        let headings = map.entities_by_type(EntityType::Heading);
        assert_eq!(headings.len(), 3);

        let paragraphs = map.entities_by_type(EntityType::Paragraph);
        assert_eq!(paragraphs.len(), 2);

        let tables = map.entities_by_type(EntityType::Table);
        assert_eq!(tables.len(), 1);

        let images = map.entities_by_type(EntityType::Image);
        assert_eq!(images.len(), 0);
    }

    #[test]
    fn test_entity_map_to_json() {
        let mut map = EntityMap::new();
        map.document_metadata
            .insert("title".to_string(), "Test Document".to_string());
        map.document_metadata
            .insert("author".to_string(), "Test Author".to_string());
        map.schemas.push("https://schema.org".to_string());

        let mut entity = Entity::new(
            "entity1".to_string(),
            EntityType::Heading,
            (10.0, 20.0, 100.0, 30.0),
            0,
        );
        entity.metadata = EntityMetadata::new()
            .with_property("level", "1")
            .with_confidence(0.95);

        map.add_entity(entity);

        let json = map.to_json().unwrap();
        assert!(json.contains("\"title\": \"Test Document\""));
        assert!(json.contains("\"author\": \"Test Author\""));
        assert!(json.contains("\"entity1\""));
        assert!(json.contains("\"heading\""));
        assert!(json.contains("https://schema.org"));

        // Pretty print should have newlines
        assert!(json.contains('\n'));
    }

    #[test]
    fn test_entity_map_to_json_compact() {
        let mut map = EntityMap::new();
        map.add_entity(Entity::new(
            "e1".to_string(),
            EntityType::Text,
            (0.0, 0.0, 50.0, 50.0),
            0,
        ));

        let json = map.to_json_compact().unwrap();
        // Compact format should not have newlines
        assert!(!json.contains('\n'));
        assert!(json.contains("\"e1\""));
    }

    #[test]
    fn test_entity_map_serialization_deserialization() {
        let mut map = EntityMap::new();
        map.document_metadata
            .insert("version".to_string(), "1.0".to_string());
        map.schemas.push("custom-schema".to_string());

        for i in 0..3 {
            let mut entity = Entity::new(
                format!("entity{}", i),
                EntityType::Paragraph,
                (0.0, i as f64 * 100.0, 200.0, 80.0),
                i / 2,
            );
            entity.metadata = EntityMetadata::new().with_property("index", i.to_string());
            map.add_entity(entity);
        }

        let json = map.to_json().unwrap();
        let deserialized: EntityMap = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.document_metadata, map.document_metadata);
        assert_eq!(deserialized.schemas, map.schemas);
        assert_eq!(deserialized.pages.len(), map.pages.len());

        for (page, entities) in &map.pages {
            let deser_entities = deserialized.pages.get(page).unwrap();
            assert_eq!(entities.len(), deser_entities.len());
        }
    }

    #[test]
    fn test_entity_map_multiple_pages() {
        let mut map = EntityMap::new();

        // Add entities across multiple pages
        for page in 0..5 {
            for i in 0..3 {
                map.add_entity(Entity::new(
                    format!("p{}e{}", page, i),
                    EntityType::Text,
                    (0.0, i as f64 * 30.0, 100.0, 25.0),
                    page,
                ));
            }
        }

        assert_eq!(map.pages.len(), 5);
        for page in 0..5 {
            assert_eq!(map.entities_on_page(page).unwrap().len(), 3);
        }
    }

    #[test]
    fn test_entity_map_empty_pages() {
        let mut map = EntityMap::new();

        // Add entities only to pages 0, 2, and 5
        map.add_entity(Entity::new(
            "e0".to_string(),
            EntityType::Text,
            (0.0, 0.0, 10.0, 10.0),
            0,
        ));
        map.add_entity(Entity::new(
            "e2".to_string(),
            EntityType::Text,
            (0.0, 0.0, 10.0, 10.0),
            2,
        ));
        map.add_entity(Entity::new(
            "e5".to_string(),
            EntityType::Text,
            (0.0, 0.0, 10.0, 10.0),
            5,
        ));

        assert_eq!(map.pages.len(), 3);
        assert!(map.entities_on_page(0).is_some());
        assert!(map.entities_on_page(1).is_none());
        assert!(map.entities_on_page(2).is_some());
        assert!(map.entities_on_page(3).is_none());
        assert!(map.entities_on_page(4).is_none());
        assert!(map.entities_on_page(5).is_some());
    }

    #[test]
    fn test_export_format_default() {
        let format = ExportFormat::default();
        match format {
            ExportFormat::Json => {} // Expected
            _ => panic!("Default format should be JSON"),
        }
    }

    #[test]
    fn test_export_format_variants() {
        // Just ensure all variants can be created
        let _json = ExportFormat::Json;
        let _jsonld = ExportFormat::JsonLd;
        let _xml = ExportFormat::Xml;
    }

    #[test]
    fn test_entity_map_with_all_entity_types() {
        let mut map = EntityMap::new();

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

        for (i, entity_type) in types.iter().enumerate() {
            map.add_entity(Entity::new(
                format!("entity_{:?}", entity_type),
                *entity_type,
                (i as f64 * 10.0, i as f64 * 10.0, 50.0, 50.0),
                0,
            ));
        }

        assert_eq!(map.entities_on_page(0).unwrap().len(), types.len());

        // Check each type can be filtered
        for entity_type in types {
            let filtered = map.entities_by_type(entity_type);
            assert_eq!(filtered.len(), 1);
            assert_eq!(filtered[0].entity_type, entity_type);
        }
    }

    #[test]
    fn test_entity_map_large_dataset() {
        let mut map = EntityMap::new();

        // Add a large number of entities
        for page in 0..100 {
            for i in 0..50 {
                map.add_entity(Entity::new(
                    format!("p{}e{}", page, i),
                    if i % 2 == 0 {
                        EntityType::Text
                    } else {
                        EntityType::Image
                    },
                    (i as f64, i as f64, 10.0, 10.0),
                    page,
                ));
            }
        }

        assert_eq!(map.pages.len(), 100);
        assert_eq!(map.entities_by_type(EntityType::Text).len(), 2500);
        assert_eq!(map.entities_by_type(EntityType::Image).len(), 2500);

        // Ensure JSON serialization works with large dataset
        let json = map.to_json_compact().unwrap();
        assert!(!json.is_empty());
    }
}
