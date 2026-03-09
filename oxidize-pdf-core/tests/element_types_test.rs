use oxidize_pdf::pipeline::{Element, ElementBBox, ElementData, ElementMetadata};

#[test]
fn test_element_title_creation() {
    let elem = Element::Title(ElementData {
        text: "Introduction".to_string(),
        metadata: ElementMetadata {
            page: 1,
            bbox: ElementBBox::new(10.0, 700.0, 200.0, 30.0),
            confidence: 0.95,
            ..Default::default()
        },
    });

    assert!(matches!(elem, Element::Title(_)));
    assert_eq!(elem.text(), "Introduction");
    assert_eq!(elem.page(), 1);
    assert_eq!(elem.bbox().x, 10.0);
    assert_eq!(elem.bbox().y, 700.0);
}

#[test]
fn test_element_paragraph_creation() {
    let elem = Element::Paragraph(ElementData {
        text: "This is a multi-line\nparagraph with content.".to_string(),
        metadata: ElementMetadata::default(),
    });

    assert!(matches!(elem, Element::Paragraph(_)));
    assert!(elem.text().contains("multi-line"));
    assert!(elem.text().contains("paragraph"));
}

#[test]
fn test_element_table_creation() {
    use oxidize_pdf::pipeline::TableElementData;

    let table = Element::Table(TableElementData {
        rows: vec![
            vec!["A".to_string(), "B".to_string(), "C".to_string()],
            vec!["1".to_string(), "2".to_string(), "3".to_string()],
            vec!["X".to_string(), "Y".to_string(), "Z".to_string()],
        ],
        metadata: ElementMetadata {
            page: 1,
            ..Default::default()
        },
    });

    assert!(matches!(table, Element::Table(_)));
    assert_eq!(table.row_count(), Some(3));
    assert_eq!(table.column_count(), Some(3));
    assert_eq!(table.cell(0, 1), Some("B"));
}

#[test]
fn test_element_variants_exhaustive() {
    use oxidize_pdf::pipeline::{ImageElementData, KeyValueElementData, TableElementData};

    let data = || ElementData {
        text: "test".to_string(),
        metadata: ElementMetadata::default(),
    };

    let elements = vec![
        Element::Title(data()),
        Element::Paragraph(data()),
        Element::Table(TableElementData {
            rows: vec![],
            metadata: ElementMetadata::default(),
        }),
        Element::Header(data()),
        Element::Footer(data()),
        Element::ListItem(data()),
        Element::Image(ImageElementData {
            alt_text: Some("logo".to_string()),
            metadata: ElementMetadata::default(),
        }),
        Element::CodeBlock(data()),
        Element::KeyValue(KeyValueElementData {
            key: "Name".to_string(),
            value: "John".to_string(),
            metadata: ElementMetadata::default(),
        }),
    ];

    assert!(matches!(elements[0], Element::Title(_)));
    assert!(matches!(elements[1], Element::Paragraph(_)));
    assert!(matches!(elements[2], Element::Table(_)));
    assert!(matches!(elements[3], Element::Header(_)));
    assert!(matches!(elements[4], Element::Footer(_)));
    assert!(matches!(elements[5], Element::ListItem(_)));
    assert!(matches!(elements[6], Element::Image(_)));
    assert!(matches!(elements[7], Element::CodeBlock(_)));
    assert!(matches!(elements[8], Element::KeyValue(_)));
}

#[test]
fn test_element_metadata_defaults() {
    let meta = ElementMetadata::default();

    assert_eq!(meta.page, 0);
    assert_eq!(meta.confidence, 1.0);
    assert_eq!(meta.bbox, ElementBBox::ZERO);
    assert!(meta.font_name.is_none());
    assert!(meta.font_size.is_none());
    assert!(!meta.is_bold);
    assert!(!meta.is_italic);
}

#[test]
fn test_element_metadata_with_font() {
    let elem = Element::Title(ElementData {
        text: "Chapter 1".to_string(),
        metadata: ElementMetadata {
            page: 1,
            font_name: Some("Helvetica-Bold".to_string()),
            font_size: Some(24.0),
            is_bold: true,
            ..Default::default()
        },
    });

    let meta = elem.metadata();
    assert_eq!(meta.font_name.as_deref(), Some("Helvetica-Bold"));
    assert_eq!(meta.font_size, Some(24.0));
    assert!(meta.is_bold);
    assert!(!meta.is_italic);
}

#[test]
fn test_element_equality_by_content_not_position() {
    // Two paragraphs with same text but different positions should be equal
    let a = Element::Paragraph(ElementData {
        text: "Same text".to_string(),
        metadata: ElementMetadata {
            page: 0,
            bbox: ElementBBox::new(10.0, 700.0, 100.0, 12.0),
            ..Default::default()
        },
    });
    let b = Element::Paragraph(ElementData {
        text: "Same text".to_string(),
        metadata: ElementMetadata {
            page: 1,
            bbox: ElementBBox::new(50.0, 300.0, 200.0, 20.0),
            ..Default::default()
        },
    });

    assert_eq!(
        a, b,
        "Elements with same variant+text should be equal regardless of position"
    );
}

#[test]
fn test_element_inequality_different_text_same_position() {
    let meta = ElementMetadata {
        page: 0,
        bbox: ElementBBox::new(10.0, 700.0, 100.0, 12.0),
        ..Default::default()
    };
    let a = Element::Paragraph(ElementData {
        text: "Alpha".to_string(),
        metadata: meta.clone(),
    });
    let b = Element::Paragraph(ElementData {
        text: "Beta".to_string(),
        metadata: meta,
    });

    assert_ne!(
        a, b,
        "Elements with different text should not be equal even at same position"
    );
}

#[test]
fn test_element_sort_by_reading_order() {
    use oxidize_pdf::pipeline::element_reading_order;

    let make = |page: u32, y: f64| {
        Element::Paragraph(ElementData {
            text: format!("p{}y{}", page, y),
            metadata: ElementMetadata {
                page,
                bbox: ElementBBox::new(0.0, y, 100.0, 12.0),
                ..Default::default()
            },
        })
    };

    let mut elements = vec![make(2, 500.0), make(1, 300.0), make(1, 700.0)];

    elements.sort_by(element_reading_order);

    // Page 1 first, then within page: higher Y first (reading order top-to-bottom in PDF coords)
    assert_eq!(elements[0].text(), "p1y700");
    assert_eq!(elements[1].text(), "p1y300");
    assert_eq!(elements[2].text(), "p2y500");
}

#[test]
fn test_element_display_text_table() {
    use oxidize_pdf::pipeline::TableElementData;

    let table = Element::Table(TableElementData {
        rows: vec![
            vec!["Name".to_string(), "Age".to_string()],
            vec!["Alice".to_string(), "30".to_string()],
        ],
        metadata: ElementMetadata::default(),
    });

    let display = table.display_text();
    assert!(display.contains("Name"));
    assert!(display.contains("Age"));
    assert!(display.contains("Alice"));
    assert!(display.contains("30"));
    assert!(
        display.contains(" | "),
        "Table display_text should use pipe separators"
    );
}

#[test]
fn test_element_display_text_key_value() {
    use oxidize_pdf::pipeline::KeyValueElementData;

    let kv = Element::KeyValue(KeyValueElementData {
        key: "Status".to_string(),
        value: "Active".to_string(),
        metadata: ElementMetadata::default(),
    });

    let display = kv.display_text();
    assert_eq!(display, "Status: Active");
}

#[test]
fn test_element_display_text_paragraph() {
    let para = Element::Paragraph(ElementData {
        text: "Hello world".to_string(),
        metadata: ElementMetadata::default(),
    });

    // For simple text elements, display_text() == text()
    assert_eq!(para.display_text(), para.text());
}

#[test]
fn test_element_debug_display() {
    let elem = Element::Title(ElementData {
        text: "Hello".to_string(),
        metadata: ElementMetadata::default(),
    });

    let debug = format!("{:?}", elem);
    assert!(debug.contains("Title"));
}

#[test]
fn test_element_clone() {
    let elem = Element::Paragraph(ElementData {
        text: "Original".to_string(),
        metadata: ElementMetadata {
            page: 3,
            confidence: 0.88,
            ..Default::default()
        },
    });

    let cloned = elem.clone();
    assert_eq!(elem.text(), cloned.text());
    assert_eq!(elem.page(), cloned.page());
    assert_eq!(elem.metadata().confidence, cloned.metadata().confidence);
}

#[test]
fn test_element_bbox_accessors() {
    let bbox = ElementBBox::new(10.0, 20.0, 100.0, 50.0);

    assert_eq!(bbox.x, 10.0);
    assert_eq!(bbox.y, 20.0);
    assert_eq!(bbox.width, 100.0);
    assert_eq!(bbox.height, 50.0);
    assert_eq!(bbox.right(), 110.0);
    assert_eq!(bbox.top(), 70.0);
}

#[cfg(feature = "semantic")]
mod serialization {
    use super::*;

    #[test]
    fn test_element_serialize_json() {
        let elem = Element::Title(ElementData {
            text: "Introduction".to_string(),
            metadata: ElementMetadata {
                page: 1,
                ..Default::default()
            },
        });

        let json = serde_json::to_string(&elem).unwrap();
        assert!(json.contains("\"type\":\"title\""));
        assert!(json.contains("\"text\":\"Introduction\""));
        assert!(json.contains("\"page\":1"));
    }

    #[test]
    fn test_element_deserialize_json() {
        let elem = Element::Paragraph(ElementData {
            text: "Test paragraph".to_string(),
            metadata: ElementMetadata::default(),
        });

        let json = serde_json::to_string(&elem).unwrap();
        let deserialized: Element = serde_json::from_str(&json).unwrap();

        assert!(matches!(deserialized, Element::Paragraph(_)));
        assert_eq!(deserialized.text(), "Test paragraph");
    }

    #[test]
    fn test_elements_vec_serialize() {
        let elements = vec![
            Element::Title(ElementData {
                text: "Title".to_string(),
                metadata: ElementMetadata::default(),
            }),
            Element::Paragraph(ElementData {
                text: "Body".to_string(),
                metadata: ElementMetadata::default(),
            }),
            Element::Footer(ElementData {
                text: "Page 1".to_string(),
                metadata: ElementMetadata::default(),
            }),
        ];

        let json = serde_json::to_string(&elements).unwrap();
        let deserialized: Vec<Element> = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.len(), 3);
    }
}
