use oxidize_pdf::pipeline::{Element, ElementData, ElementMetadata, PartitionConfig};
use oxidize_pdf::text::extraction::TextFragment;

fn frag(text: &str, x: f64, y: f64, font_size: f64) -> TextFragment {
    TextFragment {
        text: text.to_string(),
        x,
        y,
        width: text.len() as f64 * font_size * 0.5,
        height: font_size,
        font_size,
        font_name: None,
        is_bold: false,
        is_italic: false,
        color: None,
        space_decisions: Vec::new(),
    }
}

// Cycle 5.1
#[test]
fn test_element_metadata_has_parent_heading_field() {
    let meta = ElementMetadata::default();
    assert!(meta.parent_heading.is_none());
}

// Cycle 5.2
#[test]
fn test_element_set_parent_heading() {
    let mut elem = Element::Paragraph(ElementData {
        text: "Some paragraph.".to_string(),
        metadata: ElementMetadata::default(),
    });
    elem.set_parent_heading(Some("Introduction".to_string()));
    assert_eq!(
        elem.metadata().parent_heading.as_deref(),
        Some("Introduction")
    );
}

#[test]
fn test_element_set_parent_heading_none() {
    let mut elem = Element::Paragraph(ElementData {
        text: "Some paragraph.".to_string(),
        metadata: ElementMetadata {
            parent_heading: Some("Old heading".to_string()),
            ..Default::default()
        },
    });
    elem.set_parent_heading(None);
    assert!(elem.metadata().parent_heading.is_none());
}

// Cycle 5.3
#[test]
fn test_heading_assigned_after_title() {
    let fragments = vec![
        frag("Introduction", 50.0, 750.0, 24.0),
        frag("This is the intro paragraph text here.", 50.0, 700.0, 12.0),
        frag("Another sentence in the intro section.", 50.0, 680.0, 12.0),
    ];
    let elements = oxidize_pdf::pipeline::Partitioner::new(PartitionConfig::default())
        .partition_fragments(&fragments, 0, 842.0);

    let paragraphs: Vec<_> = elements
        .iter()
        .filter(|e| matches!(e, Element::Paragraph(_)))
        .collect();

    for para in &paragraphs {
        assert_eq!(
            para.metadata().parent_heading.as_deref(),
            Some("Introduction"),
            "Paragraphs despues del titulo deben tener parent_heading = 'Introduction'"
        );
    }
}

#[test]
fn test_heading_is_none_before_first_title() {
    let fragments = vec![
        frag(
            "Preamble paragraph text, no heading yet.",
            50.0,
            780.0,
            12.0,
        ),
        frag("Introduction", 50.0, 700.0, 24.0),
        frag("Post-heading paragraph text.", 50.0, 650.0, 12.0),
    ];
    let elements = oxidize_pdf::pipeline::Partitioner::new(PartitionConfig::default())
        .partition_fragments(&fragments, 0, 842.0);

    let first_para = elements
        .iter()
        .find(|e| matches!(e, Element::Paragraph(_)))
        .expect("debe haber al menos un parrafo");

    assert!(
        first_para.metadata().parent_heading.is_none(),
        "El primer parrafo (antes de cualquier titulo) no debe tener parent_heading"
    );
}

// Cycle 5.4
#[test]
fn test_parent_heading_updates_on_new_title() {
    let fragments = vec![
        frag("Chapter 1", 50.0, 780.0, 24.0),
        frag("Content of chapter one here.", 50.0, 730.0, 12.0),
        frag("Chapter 2", 50.0, 630.0, 24.0),
        frag("Content of chapter two here.", 50.0, 580.0, 12.0),
    ];
    let elements = oxidize_pdf::pipeline::Partitioner::new(PartitionConfig::default())
        .partition_fragments(&fragments, 0, 842.0);

    let paras: Vec<_> = elements
        .iter()
        .filter(|e| matches!(e, Element::Paragraph(_)))
        .collect();

    assert!(paras.len() >= 2);
    assert_eq!(
        paras[0].metadata().parent_heading.as_deref(),
        Some("Chapter 1")
    );
    assert_eq!(
        paras[1].metadata().parent_heading.as_deref(),
        Some("Chapter 2")
    );
}

// Cycle 5.5
#[test]
fn test_title_itself_has_own_text_as_parent_heading() {
    let fragments = vec![
        frag("Main Title", 50.0, 750.0, 24.0),
        frag("Content here.", 50.0, 700.0, 12.0),
    ];
    let elements = oxidize_pdf::pipeline::Partitioner::new(PartitionConfig::default())
        .partition_fragments(&fragments, 0, 842.0);

    let title = elements
        .iter()
        .find(|e| matches!(e, Element::Title(_)))
        .expect("debe haber un titulo");

    assert_eq!(
        title.metadata().parent_heading.as_deref(),
        Some("Main Title")
    );
}

// Cycle 5.6 (feature-gated)
#[cfg(feature = "semantic")]
#[test]
fn test_parent_heading_serializes_and_deserializes() {
    let elem = Element::Paragraph(ElementData {
        text: "Content".to_string(),
        metadata: ElementMetadata {
            parent_heading: Some("Section 1".to_string()),
            ..Default::default()
        },
    });

    let json = serde_json::to_string(&elem).unwrap();
    assert!(json.contains("parent_heading"));
    assert!(json.contains("Section 1"));

    let roundtripped: Element = serde_json::from_str(&json).unwrap();
    assert_eq!(
        roundtripped.metadata().parent_heading.as_deref(),
        Some("Section 1")
    );
}

// Cycle 5.7
#[test]
fn test_parent_heading_with_real_pdf_has_some_assigned() {
    let fixture = format!(
        "{}/examples/results/ai_ready_contract.pdf",
        env!("CARGO_MANIFEST_DIR")
    );
    let doc = oxidize_pdf::parser::PdfDocument::open(&fixture).unwrap();
    let elements = doc.partition().unwrap();

    for elem in &elements {
        let _ = elem.metadata().parent_heading.as_deref();
    }

    let has_titles = elements.iter().any(|e| matches!(e, Element::Title(_)));
    if has_titles {
        let paras_with_heading = elements
            .iter()
            .filter(|e| matches!(e, Element::Paragraph(_)) && e.metadata().parent_heading.is_some())
            .count();
        assert!(paras_with_heading > 0);
    }
}
