//! Tests for semantic marking functionality

#[cfg(test)]
mod entity_tests {
    use crate::semantic::{Entity, EntityMetadata, EntityType};

    #[test]
    fn test_entity_type_serialization() {
        // Test JSON serialization of entity types
        let entity_type = EntityType::Table;
        let json = serde_json::to_string(&entity_type).unwrap();
        assert_eq!(json, "\"table\"");

        let deserialized: EntityType = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, EntityType::Table);
    }

    #[test]
    fn test_all_entity_types() {
        let types = vec![
            (EntityType::Text, "\"text\""),
            (EntityType::Image, "\"image\""),
            (EntityType::Table, "\"table\""),
            (EntityType::Heading, "\"heading\""),
            (EntityType::Paragraph, "\"paragraph\""),
            (EntityType::List, "\"list\""),
            (EntityType::PageNumber, "\"pageNumber\""),
            (EntityType::Header, "\"header\""),
            (EntityType::Footer, "\"footer\""),
        ];

        for (entity_type, expected_json) in types {
            let json = serde_json::to_string(&entity_type).unwrap();
            assert_eq!(json, expected_json);
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
    fn test_entity_metadata_builder() {
        let metadata = EntityMetadata::new()
            .with_property("author", "John Doe")
            .with_property("date", "2023-07-15")
            .with_confidence(0.95)
            .with_schema("https://schema.org/Article");

        assert_eq!(metadata.properties.get("author").unwrap(), "John Doe");
        assert_eq!(metadata.properties.get("date").unwrap(), "2023-07-15");
        assert_eq!(metadata.confidence, Some(0.95));
        assert_eq!(
            metadata.schema.as_deref(),
            Some("https://schema.org/Article")
        );
    }

    #[test]
    fn test_entity_metadata_confidence_clamping() {
        // Test confidence is clamped between 0.0 and 1.0
        let metadata_high = EntityMetadata::new().with_confidence(1.5);
        assert_eq!(metadata_high.confidence, Some(1.0));

        let metadata_low = EntityMetadata::new().with_confidence(-0.5);
        assert_eq!(metadata_low.confidence, Some(0.0));

        let metadata_normal = EntityMetadata::new().with_confidence(0.75);
        assert_eq!(metadata_normal.confidence, Some(0.75));
    }

    #[test]
    fn test_entity_new() {
        let entity = Entity::new(
            "entity-1".to_string(),
            EntityType::Heading,
            (100.0, 200.0, 300.0, 50.0),
            0,
        );

        assert_eq!(entity.id, "entity-1");
        assert_eq!(entity.entity_type, EntityType::Heading);
        assert_eq!(entity.bounds, (100.0, 200.0, 300.0, 50.0));
        assert_eq!(entity.page, 0);
        assert!(entity.metadata.properties.is_empty());
    }

    #[test]
    fn test_entity_serialization() {
        let mut entity = Entity::new(
            "test-entity".to_string(),
            EntityType::Paragraph,
            (50.0, 100.0, 500.0, 200.0),
            2,
        );
        entity.metadata = EntityMetadata::new()
            .with_property("lang", "en")
            .with_confidence(0.88);

        let json = serde_json::to_string_pretty(&entity).unwrap();
        assert!(json.contains("\"id\": \"test-entity\""));
        assert!(json.contains("\"type\": \"paragraph\""));
        assert!(json.contains("\"page\": 2"));
        assert!(json.contains("\"confidence\": 0.88"));

        let deserialized: Entity = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.id, entity.id);
        assert_eq!(deserialized.entity_type, entity.entity_type);
        assert_eq!(deserialized.bounds, entity.bounds);
        assert_eq!(deserialized.page, entity.page);
        assert_eq!(deserialized.metadata.confidence, Some(0.88));
    }

    #[test]
    fn test_entity_metadata_multiple_properties() {
        let mut metadata = EntityMetadata::new();

        // Add properties manually
        metadata
            .properties
            .insert("key1".to_string(), "value1".to_string());
        metadata
            .properties
            .insert("key2".to_string(), "value2".to_string());
        metadata
            .properties
            .insert("key3".to_string(), "value3".to_string());

        assert_eq!(metadata.properties.len(), 3);
        assert_eq!(metadata.properties.get("key1").unwrap(), "value1");
        assert_eq!(metadata.properties.get("key2").unwrap(), "value2");
        assert_eq!(metadata.properties.get("key3").unwrap(), "value3");
    }

    #[test]
    fn test_entity_type_equality() {
        assert_eq!(EntityType::Text, EntityType::Text);
        assert_ne!(EntityType::Text, EntityType::Image);
        assert_ne!(EntityType::Table, EntityType::List);
    }

    #[test]
    fn test_entity_type_debug() {
        let entity_type = EntityType::Table;
        let debug_str = format!("{:?}", entity_type);
        assert_eq!(debug_str, "Table");
    }

    #[test]
    fn test_entity_clone() {
        let mut entity = Entity::new(
            "clone-test".to_string(),
            EntityType::Image,
            (0.0, 0.0, 100.0, 100.0),
            5,
        );
        entity.metadata = EntityMetadata::new()
            .with_property("alt", "Test image")
            .with_confidence(0.99);

        let cloned = entity.clone();
        assert_eq!(cloned.id, entity.id);
        assert_eq!(cloned.entity_type, entity.entity_type);
        assert_eq!(cloned.bounds, entity.bounds);
        assert_eq!(cloned.page, entity.page);
        assert_eq!(
            cloned.metadata.properties.get("alt"),
            Some(&"Test image".to_string())
        );
        assert_eq!(cloned.metadata.confidence, Some(0.99));
    }
}

#[cfg(test)]
mod export_tests {
    use crate::semantic::{Entity, EntityMap, EntityMetadata, EntityType, ExportFormat};

    #[test]
    fn test_entity_map_new() {
        let map = EntityMap::new();
        assert!(map.pages.is_empty());
        assert!(map.schemas.is_empty());
        assert!(map.document_metadata.is_empty());
    }

    #[test]
    fn test_entity_map_add_entity() {
        let mut map = EntityMap::new();

        let entity = Entity::new(
            "test-1".to_string(),
            EntityType::Text,
            (0.0, 0.0, 100.0, 100.0),
            0,
        );

        map.add_entity(entity);
        assert_eq!(map.pages.len(), 1);
        assert_eq!(map.pages.get(&0).unwrap().len(), 1);
        assert_eq!(map.pages.get(&0).unwrap()[0].id, "test-1");
    }

    #[test]
    fn test_entity_map_add_schema() {
        let mut map = EntityMap::new();
        map.schemas.push("https://schema.org/Article".to_string());
        map.schemas.push("https://schema.org/Person".to_string());

        assert_eq!(map.schemas.len(), 2);
        assert!(map
            .schemas
            .contains(&"https://schema.org/Article".to_string()));
        assert!(map
            .schemas
            .contains(&"https://schema.org/Person".to_string()));
    }

    #[test]
    fn test_entity_map_to_json() {
        let mut map = EntityMap::new();
        map.schemas.push("https://schema.org/Document".to_string());

        let entity = Entity::new(
            "heading-1".to_string(),
            EntityType::Heading,
            (50.0, 50.0, 200.0, 30.0),
            0,
        );
        map.add_entity(entity);

        let json = map.to_json().unwrap();
        assert!(json.contains("\"pages\""));
        assert!(json.contains("\"schemas\""));
        assert!(json.contains("heading-1"));
        assert!(json.contains("https://schema.org/Document"));
    }

    #[test]
    fn test_entity_map_to_json_compact() {
        let mut map = EntityMap::new();

        let entity = Entity::new(
            "test".to_string(),
            EntityType::Text,
            (0.0, 0.0, 10.0, 10.0),
            0,
        );
        map.add_entity(entity);

        let json_pretty = map.to_json().unwrap();
        let json_compact = map.to_json_compact().unwrap();

        // Compact should be shorter (no pretty printing)
        assert!(json_compact.len() < json_pretty.len());
        assert!(!json_compact.contains("\n"));
    }

    #[test]
    fn test_entity_map_page_filter() {
        let mut map = EntityMap::new();

        // Add entities on different pages
        for i in 0..5 {
            let entity = Entity::new(
                format!("entity-{}", i),
                EntityType::Text,
                (0.0, 0.0, 100.0, 100.0),
                i % 3, // Pages 0, 1, 2, 0, 1
            );
            map.add_entity(entity);
        }

        let page_0_entities = map.entities_on_page(0).unwrap();
        assert_eq!(page_0_entities.len(), 2);

        let page_1_entities = map.entities_on_page(1).unwrap();
        assert_eq!(page_1_entities.len(), 2);

        let page_2_entities = map.entities_on_page(2).unwrap();
        assert_eq!(page_2_entities.len(), 1);

        let page_3_entities = map.entities_on_page(3);
        assert!(page_3_entities.is_none());
    }

    #[test]
    fn test_entity_map_type_filter() {
        let mut map = EntityMap::new();

        // Add entities of different types
        map.add_entity(Entity::new(
            "h1".to_string(),
            EntityType::Heading,
            (0.0, 0.0, 100.0, 30.0),
            0,
        ));
        map.add_entity(Entity::new(
            "h2".to_string(),
            EntityType::Heading,
            (0.0, 50.0, 100.0, 30.0),
            0,
        ));
        map.add_entity(Entity::new(
            "p1".to_string(),
            EntityType::Paragraph,
            (0.0, 100.0, 100.0, 50.0),
            0,
        ));
        map.add_entity(Entity::new(
            "img1".to_string(),
            EntityType::Image,
            (0.0, 200.0, 100.0, 100.0),
            0,
        ));

        let headings = map.entities_by_type(EntityType::Heading);
        assert_eq!(headings.len(), 2);

        let paragraphs = map.entities_by_type(EntityType::Paragraph);
        assert_eq!(paragraphs.len(), 1);

        let images = map.entities_by_type(EntityType::Image);
        assert_eq!(images.len(), 1);

        let tables = map.entities_by_type(EntityType::Table);
        assert_eq!(tables.len(), 0);
    }

    #[test]
    fn test_export_format_variants() {
        let formats = vec![ExportFormat::Json, ExportFormat::JsonLd, ExportFormat::Xml];

        for format in formats {
            // Just ensure they can be created and matched
            match format {
                ExportFormat::Json => assert!(true),
                ExportFormat::JsonLd => assert!(true),
                ExportFormat::Xml => assert!(true),
            }
        }
    }

    #[test]
    fn test_export_format_default() {
        let default_format = ExportFormat::default();
        match default_format {
            ExportFormat::Json => assert!(true),
            _ => panic!("Default format should be Json"),
        }
    }

    #[test]
    fn test_entity_map_document_metadata() {
        let mut map = EntityMap::new();

        map.document_metadata
            .insert("title".to_string(), "Test Document".to_string());
        map.document_metadata
            .insert("author".to_string(), "Test Author".to_string());

        assert_eq!(map.document_metadata.len(), 2);
        assert_eq!(map.document_metadata.get("title").unwrap(), "Test Document");
        assert_eq!(map.document_metadata.get("author").unwrap(), "Test Author");
    }

    // Additional tests for export.rs coverage

    #[test]
    fn test_entity_map_empty_json_export() {
        let map = EntityMap::new();
        let json = map.to_json().unwrap();

        assert!(json.contains("\"document_metadata\": {}"));
        assert!(json.contains("\"pages\": {}"));
        assert!(json.contains("\"schemas\": []"));
    }

    #[test]
    fn test_entity_map_multiple_pages() {
        let mut map = EntityMap::new();

        // Add entities to multiple pages
        for page in 0..5 {
            for i in 0..3 {
                let entity = Entity::new(
                    format!("entity-p{}-{}", page, i),
                    EntityType::Text,
                    (0.0, i as f64 * 50.0, 100.0, 40.0),
                    page,
                );
                map.add_entity(entity);
            }
        }

        assert_eq!(map.pages.len(), 5);
        for page in 0..5 {
            assert_eq!(map.entities_on_page(page).unwrap().len(), 3);
        }
    }

    #[test]
    fn test_entity_map_add_duplicate_schemas() {
        let mut map = EntityMap::new();

        map.schemas.push("https://schema.org/Article".to_string());
        map.schemas.push("https://schema.org/Person".to_string());
        map.schemas.push("https://schema.org/Article".to_string()); // Duplicate

        assert_eq!(map.schemas.len(), 3); // Allows duplicates
    }

    #[test]
    fn test_entity_map_entities_by_type_empty() {
        let map = EntityMap::new();
        let tables = map.entities_by_type(EntityType::Table);
        assert_eq!(tables.len(), 0);
    }

    #[test]
    fn test_entity_map_mixed_entity_types() {
        let mut map = EntityMap::new();

        let types = vec![
            EntityType::Heading,
            EntityType::Paragraph,
            EntityType::Image,
            EntityType::Table,
            EntityType::List,
            EntityType::Heading, // Another heading
        ];

        for (i, entity_type) in types.iter().enumerate() {
            let entity = Entity::new(
                format!("entity-{}", i),
                *entity_type,
                (0.0, 0.0, 100.0, 50.0),
                0,
            );
            map.add_entity(entity);
        }

        assert_eq!(map.entities_by_type(EntityType::Heading).len(), 2);
        assert_eq!(map.entities_by_type(EntityType::Paragraph).len(), 1);
        assert_eq!(map.entities_by_type(EntityType::Image).len(), 1);
        assert_eq!(map.entities_by_type(EntityType::Table).len(), 1);
        assert_eq!(map.entities_by_type(EntityType::List).len(), 1);
        assert_eq!(map.entities_by_type(EntityType::Footer).len(), 0);
    }

    #[test]
    fn test_entity_map_json_serialization_with_metadata() {
        let mut map = EntityMap::new();

        map.document_metadata
            .insert("version".to_string(), "1.0".to_string());
        map.document_metadata
            .insert("created".to_string(), "2024-01-15".to_string());
        map.schemas.push("https://schema.org/Document".to_string());

        let mut entity = Entity::new(
            "meta-entity".to_string(),
            EntityType::Heading,
            (10.0, 20.0, 300.0, 40.0),
            0,
        );
        entity.metadata = EntityMetadata::new()
            .with_property("level", "1")
            .with_confidence(0.95);

        map.add_entity(entity);

        let json = map.to_json().unwrap();
        assert!(json.contains("\"version\": \"1.0\""));
        assert!(json.contains("\"created\": \"2024-01-15\""));
        assert!(json.contains("\"level\": \"1\""));
        assert!(json.contains("\"confidence\": 0.95"));
    }

    #[test]
    fn test_entity_map_compact_vs_pretty_json() {
        let mut map = EntityMap::new();

        // Add some content to make the difference noticeable
        for i in 0..3 {
            map.add_entity(Entity::new(
                format!("e{}", i),
                EntityType::Text,
                (0.0, 0.0, 100.0, 20.0),
                0,
            ));
        }

        let pretty = map.to_json().unwrap();
        let compact = map.to_json_compact().unwrap();

        // Pretty should have newlines and indentation
        assert!(pretty.contains("\n"));
        assert!(pretty.contains("  "));

        // Compact should not
        assert!(!compact.contains("\n"));
        assert!(compact.len() < pretty.len());
    }

    #[test]
    fn test_entity_map_large_document() {
        let mut map = EntityMap::new();

        // Simulate a large document with many pages and entities
        for page in 0..100 {
            for i in 0..10 {
                let entity = Entity::new(
                    format!("entity-{}-{}", page, i),
                    EntityType::Paragraph,
                    (0.0, i as f64 * 50.0, 500.0, 40.0),
                    page,
                );
                map.add_entity(entity);
            }
        }

        assert_eq!(map.pages.len(), 100);
        let all_paragraphs = map.entities_by_type(EntityType::Paragraph);
        assert_eq!(all_paragraphs.len(), 1000);
    }

    #[test]
    fn test_entity_map_unicode_document_metadata() {
        let mut map = EntityMap::new();

        map.document_metadata
            .insert("标题".to_string(), "测试文档".to_string());
        map.document_metadata
            .insert("المؤلف".to_string(), "مؤلف الاختبار".to_string());
        map.document_metadata
            .insert("автор".to_string(), "Тестовый автор".to_string());

        assert_eq!(map.document_metadata.get("标题").unwrap(), "测试文档");
        assert_eq!(map.document_metadata.get("المؤلف").unwrap(), "مؤلف الاختبار");
        assert_eq!(
            map.document_metadata.get("автор").unwrap(),
            "Тестовый автор"
        );
    }

    #[test]
    fn test_export_format_copy_trait() {
        let format1 = ExportFormat::Json;
        let format2 = format1; // Copy

        match format1 {
            ExportFormat::Json => assert!(true),
            _ => assert!(false),
        }

        match format2 {
            ExportFormat::Json => assert!(true),
            _ => assert!(false),
        }
    }

    #[test]
    fn test_entity_map_json_deserialize() {
        let mut map = EntityMap::new();
        map.schemas.push("https://schema.org/Thing".to_string());
        map.add_entity(Entity::new(
            "test-id".to_string(),
            EntityType::Text,
            (1.0, 2.0, 3.0, 4.0),
            5,
        ));

        let json = map.to_json().unwrap();
        let deserialized: EntityMap = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.schemas.len(), 1);
        assert_eq!(deserialized.pages.len(), 1);
        assert_eq!(deserialized.entities_on_page(5).unwrap().len(), 1);
    }

    #[test]
    fn test_entity_map_entities_on_invalid_page() {
        let map = EntityMap::new();
        assert!(map.entities_on_page(999).is_none());
    }

    #[test]
    fn test_entity_map_add_entity_preserves_metadata() {
        let mut map = EntityMap::new();

        let mut entity = Entity::new(
            "preserve-meta".to_string(),
            EntityType::Image,
            (0.0, 0.0, 200.0, 150.0),
            2,
        );

        entity.metadata = EntityMetadata::new()
            .with_property("alt", "Test image")
            .with_property("source", "scanner")
            .with_confidence(0.88)
            .with_schema("https://schema.org/ImageObject");

        map.add_entity(entity);

        let retrieved = &map.entities_on_page(2).unwrap()[0];
        assert_eq!(retrieved.metadata.properties.len(), 2);
        assert_eq!(retrieved.metadata.confidence, Some(0.88));
        assert_eq!(
            retrieved.metadata.schema,
            Some("https://schema.org/ImageObject".to_string())
        );
    }
}

#[cfg(test)]
mod marking_tests {
    use crate::semantic::{Entity, EntityMetadata, EntityType};

    #[test]
    fn test_entity_builder_metadata() {
        // Since EntityBuilder requires a mutable Page reference,
        // we'll test the metadata builder functionality through EntityMetadata
        use crate::semantic::EntityMetadata;

        let metadata = EntityMetadata::new()
            .with_property("level", "1")
            .with_property("text", "Introduction")
            .with_confidence(0.92)
            .with_schema("https://schema.org/Article");

        assert_eq!(metadata.properties.get("level"), Some(&"1".to_string()));
        assert_eq!(
            metadata.properties.get("text"),
            Some(&"Introduction".to_string())
        );
        assert_eq!(metadata.confidence, Some(0.92));
        assert_eq!(
            metadata.schema,
            Some("https://schema.org/Article".to_string())
        );
    }

    #[test]
    fn test_entity_type_variants() {
        // Test all entity type variants can be created
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
                EntityType::Text => assert!(true),
                EntityType::Image => assert!(true),
                EntityType::Table => assert!(true),
                EntityType::Heading => assert!(true),
                EntityType::Paragraph => assert!(true),
                EntityType::List => assert!(true),
                EntityType::PageNumber => assert!(true),
                EntityType::Header => assert!(true),
                EntityType::Footer => assert!(true),
            }
        }
    }

    #[test]
    fn test_uuid_generation() {
        // Test that entity IDs would be unique
        use std::collections::HashSet;
        use std::time::{SystemTime, UNIX_EPOCH};

        let mut ids = HashSet::new();

        for _ in 0..100 {
            let timestamp = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let id = format!("entity_{:x}", timestamp);

            // Small delay to ensure different timestamps
            std::thread::sleep(std::time::Duration::from_nanos(1));

            assert!(ids.insert(id), "Duplicate ID generated");
        }

        assert_eq!(ids.len(), 100);
    }

    // Additional tests for entity.rs coverage

    #[test]
    fn test_entity_bounds_validation() {
        let entity = Entity::new(
            "test-bounds".to_string(),
            EntityType::Text,
            (50.0, 100.0, 200.0, 300.0),
            0,
        );

        assert_eq!(entity.bounds.0, 50.0);
        assert_eq!(entity.bounds.1, 100.0);
        assert_eq!(entity.bounds.2, 200.0);
        assert_eq!(entity.bounds.3, 300.0);
    }

    #[test]
    fn test_entity_negative_bounds() {
        // Test that entities can have negative coordinates (for transformed content)
        let entity = Entity::new(
            "negative-bounds".to_string(),
            EntityType::Image,
            (-10.0, -20.0, 100.0, 50.0),
            1,
        );

        assert_eq!(entity.bounds.0, -10.0);
        assert_eq!(entity.bounds.1, -20.0);
    }

    #[test]
    fn test_entity_metadata_empty_property() {
        let metadata = EntityMetadata::new()
            .with_property("", "empty_key")
            .with_property("empty_value", "");

        assert_eq!(metadata.properties.get(""), Some(&"empty_key".to_string()));
        assert_eq!(
            metadata.properties.get("empty_value"),
            Some(&"".to_string())
        );
    }

    #[test]
    fn test_entity_metadata_unicode_properties() {
        let metadata = EntityMetadata::new()
            .with_property("título", "Introducción")
            .with_property("作者", "张三")
            .with_property("emoji", "🎉 Party!");

        assert_eq!(
            metadata.properties.get("título"),
            Some(&"Introducción".to_string())
        );
        assert_eq!(metadata.properties.get("作者"), Some(&"张三".to_string()));
        assert_eq!(
            metadata.properties.get("emoji"),
            Some(&"🎉 Party!".to_string())
        );
    }

    #[test]
    fn test_entity_metadata_overwrite_property() {
        let metadata = EntityMetadata::new()
            .with_property("key", "value1")
            .with_property("key", "value2");

        assert_eq!(metadata.properties.get("key"), Some(&"value2".to_string()));
        assert_eq!(metadata.properties.len(), 1);
    }

    #[test]
    fn test_entity_type_display_formatting() {
        // Test debug output for all entity types
        let types_and_debug = vec![
            (EntityType::Text, "Text"),
            (EntityType::Image, "Image"),
            (EntityType::Table, "Table"),
            (EntityType::Heading, "Heading"),
            (EntityType::Paragraph, "Paragraph"),
            (EntityType::List, "List"),
            (EntityType::PageNumber, "PageNumber"),
            (EntityType::Header, "Header"),
            (EntityType::Footer, "Footer"),
        ];

        for (entity_type, expected_debug) in types_and_debug {
            assert_eq!(format!("{:?}", entity_type), expected_debug);
        }
    }

    #[test]
    fn test_entity_with_all_metadata_fields() {
        let mut entity = Entity::new(
            "full-metadata".to_string(),
            EntityType::Table,
            (0.0, 0.0, 500.0, 300.0),
            3,
        );

        entity.metadata = EntityMetadata::new()
            .with_property("rows", "5")
            .with_property("columns", "3")
            .with_property("has_header", "true")
            .with_confidence(0.87)
            .with_schema("https://schema.org/Table");

        assert_eq!(entity.metadata.properties.len(), 3);
        assert_eq!(entity.metadata.confidence, Some(0.87));
        assert!(entity.metadata.schema.is_some());
    }

    #[test]
    fn test_entity_metadata_confidence_edge_cases() {
        // Test exact boundaries
        let metadata_zero = EntityMetadata::new().with_confidence(0.0);
        assert_eq!(metadata_zero.confidence, Some(0.0));

        let metadata_one = EntityMetadata::new().with_confidence(1.0);
        assert_eq!(metadata_one.confidence, Some(1.0));

        // Test very small positive number
        let metadata_tiny = EntityMetadata::new().with_confidence(0.0001);
        assert_eq!(metadata_tiny.confidence, Some(0.0001));
    }

    #[test]
    fn test_entity_metadata_chain_builder() {
        // Test that builder pattern can be chained multiple times
        let metadata = EntityMetadata::new()
            .with_property("a", "1")
            .with_property("b", "2")
            .with_property("c", "3")
            .with_confidence(0.5)
            .with_property("d", "4")
            .with_confidence(0.6) // Should overwrite
            .with_schema("schema1")
            .with_property("e", "5")
            .with_schema("schema2"); // Should overwrite

        assert_eq!(metadata.properties.len(), 5);
        assert_eq!(metadata.confidence, Some(0.6));
        assert_eq!(metadata.schema, Some("schema2".to_string()));
    }

    #[test]
    fn test_entity_zero_size_bounds() {
        // Test entities with zero width/height (valid for markers)
        let point_entity = Entity::new(
            "point".to_string(),
            EntityType::PageNumber,
            (100.0, 200.0, 0.0, 0.0),
            5,
        );

        assert_eq!(point_entity.bounds.2, 0.0);
        assert_eq!(point_entity.bounds.3, 0.0);
    }

    #[test]
    fn test_entity_large_page_numbers() {
        let entity = Entity::new(
            "large-page".to_string(),
            EntityType::Footer,
            (0.0, 0.0, 100.0, 20.0),
            9999,
        );

        assert_eq!(entity.page, 9999);
    }

    #[test]
    fn test_entity_metadata_long_strings() {
        let long_key = "k".repeat(1000);
        let long_value = "v".repeat(10000);

        let metadata = EntityMetadata::new().with_property(long_key.clone(), long_value.clone());

        assert_eq!(metadata.properties.get(&long_key), Some(&long_value));
    }

    #[test]
    fn test_entity_id_format() {
        let entity = Entity::new(
            "custom-id-123-ABC_xyz".to_string(),
            EntityType::Heading,
            (0.0, 0.0, 100.0, 30.0),
            0,
        );

        assert_eq!(entity.id, "custom-id-123-ABC_xyz");
    }

    #[test]
    fn test_entity_metadata_special_characters_in_schema() {
        let metadata = EntityMetadata::new()
            .with_schema("https://example.com/schema?type=article&lang=en#section");

        assert_eq!(
            metadata.schema,
            Some("https://example.com/schema?type=article&lang=en#section".to_string())
        );
    }

    // Additional marking.rs tests

    #[test]
    fn test_marking_different_entity_types() {
        // Test all entity type marking combinations
        let test_cases = vec![
            (EntityType::Text, (0.0, 0.0, 100.0, 20.0)),
            (EntityType::Image, (0.0, 30.0, 200.0, 150.0)),
            (EntityType::Table, (0.0, 190.0, 400.0, 200.0)),
            (EntityType::Heading, (0.0, 400.0, 300.0, 30.0)),
            (EntityType::Paragraph, (0.0, 440.0, 500.0, 100.0)),
            (EntityType::List, (0.0, 550.0, 250.0, 80.0)),
            (EntityType::PageNumber, (250.0, 750.0, 50.0, 20.0)),
            (EntityType::Header, (0.0, 780.0, 600.0, 20.0)),
            (EntityType::Footer, (0.0, 0.0, 600.0, 20.0)),
        ];

        for (entity_type, bounds) in test_cases {
            let entity = Entity::new(format!("mark-{:?}", entity_type), entity_type, bounds, 0);
            assert_eq!(entity.entity_type, entity_type);
            assert_eq!(entity.bounds, bounds);
        }
    }

    #[test]
    fn test_entity_builder_with_all_methods() {
        // Test using all builder methods
        let mut entity = Entity::new(
            "all-methods".to_string(),
            EntityType::Table,
            (50.0, 100.0, 400.0, 300.0),
            2,
        );

        entity.metadata = EntityMetadata::new()
            .with_property("key1", "value1")
            .with_confidence(0.85)
            .with_schema("https://schema.org/Table")
            .with_property("key2", "value2")
            .with_property("key3", "value3");

        assert_eq!(entity.metadata.properties.len(), 3);
        assert_eq!(entity.metadata.confidence, Some(0.85));
        assert!(entity.metadata.schema.is_some());
    }

    #[test]
    fn test_marking_edge_case_bounds() {
        // Test edge cases for bounds
        let edge_bounds = vec![
            (f64::MIN_POSITIVE, f64::MIN_POSITIVE, 100.0, 100.0),
            (0.0, 0.0, f64::MIN_POSITIVE, f64::MIN_POSITIVE),
            (1e-10, 1e-10, 1e-10, 1e-10),
            (1e10, 1e10, 100.0, 100.0),
        ];

        for (i, bounds) in edge_bounds.iter().enumerate() {
            let entity = Entity::new(format!("edge-bounds-{}", i), EntityType::Text, *bounds, 0);
            assert_eq!(entity.bounds, *bounds);
        }
    }
}
