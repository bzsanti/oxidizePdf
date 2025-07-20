//! Tests for semantic marking API

#[cfg(test)]
mod tests {
    use super::super::marking::*;
    use super::super::{EntityMetadata, EntityType};
    use crate::page::Page;

    #[test]
    fn test_uuid_simple() {
        let id1 = uuid_simple();
        let id2 = uuid_simple();

        // IDs should be unique
        assert_ne!(id1, id2);

        // IDs should be non-empty hex strings
        assert!(!id1.is_empty());
        assert!(!id2.is_empty());
        assert!(id1.chars().all(|c| c.is_ascii_hexdigit()));
        assert!(id2.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn test_entity_builder_new() {
        let mut page = Page::a4();
        let builder = EntityBuilder::new(&mut page, EntityType::Heading, (0.0, 0.0, 100.0, 20.0));

        assert_eq!(builder.entity_type, EntityType::Heading);
        assert_eq!(builder.bounds, (0.0, 0.0, 100.0, 20.0));
        // Metadata should be empty initially
        assert!(builder.metadata.properties.is_empty());
        assert_eq!(builder.metadata.confidence, None);
        assert_eq!(builder.metadata.schema, None);
    }

    #[test]
    fn test_entity_builder_with_metadata() {
        let mut page = Page::a4();
        let builder = EntityBuilder::new(&mut page, EntityType::Table, (10.0, 10.0, 200.0, 150.0))
            .with_metadata("rows", "5")
            .with_metadata("columns", "3")
            .with_metadata("header", "true");

        assert_eq!(
            builder.metadata.properties.get("rows"),
            Some(&"5".to_string())
        );
        assert_eq!(
            builder.metadata.properties.get("columns"),
            Some(&"3".to_string())
        );
        assert_eq!(
            builder.metadata.properties.get("header"),
            Some(&"true".to_string())
        );
    }

    #[test]
    fn test_entity_builder_with_confidence() {
        let mut page = Page::a4();
        let builder = EntityBuilder::new(&mut page, EntityType::Text, (0.0, 0.0, 100.0, 100.0))
            .with_confidence(0.85);

        assert_eq!(builder.metadata.confidence, Some(0.85));

        // Test confidence clamping
        let builder_high = EntityBuilder::new(&mut page, EntityType::Text, (0.0, 0.0, 10.0, 10.0))
            .with_confidence(1.5);
        assert_eq!(builder_high.metadata.confidence, Some(1.0));

        let builder_low = EntityBuilder::new(&mut page, EntityType::Text, (0.0, 0.0, 10.0, 10.0))
            .with_confidence(-0.5);
        assert_eq!(builder_low.metadata.confidence, Some(0.0));
    }

    #[test]
    fn test_entity_builder_with_schema() {
        let mut page = Page::a4();
        let builder =
            EntityBuilder::new(&mut page, EntityType::Paragraph, (0.0, 0.0, 300.0, 100.0))
                .with_schema("https://schema.org/Article");

        assert_eq!(
            builder.metadata.schema,
            Some("https://schema.org/Article".to_string())
        );
    }

    #[test]
    fn test_entity_builder_chain() {
        let mut page = Page::a4();
        let builder = EntityBuilder::new(&mut page, EntityType::Image, (50.0, 50.0, 150.0, 150.0))
            .with_metadata("alt", "Company Logo")
            .with_metadata("format", "PNG")
            .with_confidence(0.99)
            .with_schema("https://schema.org/ImageObject");

        assert_eq!(builder.metadata.properties.len(), 2);
        assert_eq!(builder.metadata.confidence, Some(0.99));
        assert!(builder.metadata.schema.is_some());
    }

    #[test]
    fn test_entity_builder_build() {
        let mut page = Page::a4();
        let id = EntityBuilder::new(&mut page, EntityType::Heading, (0.0, 0.0, 612.0, 72.0))
            .with_metadata("level", "1")
            .with_confidence(0.95)
            .build();

        // ID should start with entity_ prefix
        assert!(id.starts_with("entity_"));

        // ID should contain hex timestamp
        let timestamp_part = &id[7..]; // Skip "entity_"
        assert!(timestamp_part.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn test_semantic_marker_new() {
        let mut page = Page::a4();
        let _marker = SemanticMarker::new(&mut page);
        // Just ensure it can be created
    }

    #[test]
    fn test_semantic_marker_mark() {
        let mut page = Page::a4();
        let mut marker = SemanticMarker::new(&mut page);

        let builder = marker.mark(EntityType::List, (10.0, 100.0, 200.0, 150.0));
        assert_eq!(builder.entity_type, EntityType::List);
        assert_eq!(builder.bounds, (10.0, 100.0, 200.0, 150.0));
    }

    #[test]
    fn test_semantic_marker_mark_text() {
        let mut page = Page::a4();
        let mut marker = SemanticMarker::new(&mut page);

        let builder = marker.mark_text((0.0, 0.0, 100.0, 20.0));
        assert_eq!(builder.entity_type, EntityType::Text);
        assert_eq!(builder.bounds, (0.0, 0.0, 100.0, 20.0));
    }

    #[test]
    fn test_semantic_marker_mark_image() {
        let mut page = Page::a4();
        let mut marker = SemanticMarker::new(&mut page);

        let builder = marker.mark_image((50.0, 50.0, 200.0, 200.0));
        assert_eq!(builder.entity_type, EntityType::Image);
        assert_eq!(builder.bounds, (50.0, 50.0, 200.0, 200.0));
    }

    #[test]
    fn test_semantic_marker_mark_table() {
        let mut page = Page::a4();
        let mut marker = SemanticMarker::new(&mut page);

        let builder = marker.mark_table((10.0, 200.0, 400.0, 300.0));
        assert_eq!(builder.entity_type, EntityType::Table);
        assert_eq!(builder.bounds, (10.0, 200.0, 400.0, 300.0));
    }

    #[test]
    fn test_complete_marking_workflow() {
        let mut page = Page::a4();
        let mut marker = SemanticMarker::new(&mut page);

        // Mark a heading
        let heading_id = marker
            .mark_text((50.0, 50.0, 500.0, 72.0))
            .with_metadata("type", "heading")
            .with_metadata("level", "1")
            .with_confidence(0.98)
            .build();

        assert!(heading_id.starts_with("entity_"));

        // Mark a paragraph
        let para_id = marker
            .mark_text((50.0, 100.0, 500.0, 200.0))
            .with_metadata("type", "paragraph")
            .with_metadata("alignment", "justified")
            .build();

        assert!(para_id.starts_with("entity_"));
        assert_ne!(heading_id, para_id);

        // Mark an image
        let img_id = marker
            .mark_image((100.0, 250.0, 300.0, 200.0))
            .with_metadata("alt", "Diagram showing process flow")
            .with_schema("https://schema.org/ImageObject")
            .build();

        assert!(img_id.starts_with("entity_"));
        assert_ne!(img_id, heading_id);
        assert_ne!(img_id, para_id);
    }

    #[test]
    fn test_marking_different_page_sizes() {
        let pages = vec![
            Page::a4(),
            Page::letter(),
            Page::legal(),
            Page::new(200.0, 300.0),
        ];

        for mut page in pages {
            let mut marker = SemanticMarker::new(&mut page);

            // Should work with any page size
            let id = marker
                .mark_text((0.0, 0.0, 100.0, 50.0))
                .with_metadata("test", "true")
                .build();

            assert!(id.starts_with("entity_"));
        }
    }

    #[test]
    fn test_marking_edge_case_bounds() {
        let mut page = Page::a4();
        let mut marker = SemanticMarker::new(&mut page);

        // Zero-sized region
        let id1 = marker.mark_text((100.0, 100.0, 0.0, 0.0)).build();
        assert!(id1.starts_with("entity_"));

        // Negative position
        let id2 = marker.mark_text((-10.0, -10.0, 50.0, 50.0)).build();
        assert!(id2.starts_with("entity_"));

        // Large bounds exceeding page
        let id3 = marker.mark_text((0.0, 0.0, 10000.0, 10000.0)).build();
        assert!(id3.starts_with("entity_"));
    }

    #[test]
    fn test_marking_all_entity_types() {
        let mut page = Page::a4();
        let mut marker = SemanticMarker::new(&mut page);

        let types = vec![
            (EntityType::Text, (0.0, 0.0, 100.0, 20.0)),
            (EntityType::Image, (0.0, 30.0, 100.0, 100.0)),
            (EntityType::Table, (0.0, 140.0, 300.0, 200.0)),
            (EntityType::Heading, (0.0, 350.0, 300.0, 30.0)),
            (EntityType::Paragraph, (0.0, 390.0, 300.0, 100.0)),
            (EntityType::List, (320.0, 140.0, 200.0, 150.0)),
            (EntityType::PageNumber, (250.0, 750.0, 50.0, 20.0)),
            (EntityType::Header, (0.0, 0.0, 612.0, 50.0)),
            (EntityType::Footer, (0.0, 750.0, 612.0, 42.0)),
        ];

        for (entity_type, bounds) in types {
            let id = marker
                .mark(entity_type, bounds)
                .with_metadata("entity_type", format!("{:?}", entity_type))
                .build();
            assert!(id.starts_with("entity_"));
        }
    }
}
