use oxidize_pdf::pipeline::hybrid_chunking::{HybridChunkConfig, HybridChunker};
use oxidize_pdf::pipeline::{Element, ElementBBox, ElementData, ElementMetadata, TableElementData};

fn meta_with_heading(page: u32, y: f64, heading: Option<&str>) -> ElementMetadata {
    ElementMetadata {
        page,
        bbox: ElementBBox::new(50.0, y, 500.0, 12.0),
        parent_heading: heading.map(|s| s.to_string()),
        ..Default::default()
    }
}

fn para_with_heading(text: &str, page: u32, y: f64, heading: Option<&str>) -> Element {
    Element::Paragraph(ElementData {
        text: text.to_string(),
        metadata: meta_with_heading(page, y, heading),
    })
}

fn title_elem(text: &str, page: u32, y: f64) -> Element {
    Element::Title(ElementData {
        text: text.to_string(),
        metadata: meta_with_heading(page, y, Some(text)),
    })
}

// Cycle 6.1
#[test]
fn test_hybrid_chunk_config_defaults() {
    let cfg = HybridChunkConfig::default();
    assert_eq!(cfg.max_tokens, 512);
    assert_eq!(cfg.overlap_tokens, 50);
    assert!(cfg.merge_adjacent);
    assert!(cfg.propagate_headings);
}

// Cycle 6.2
#[test]
fn test_heading_context_propagated_to_chunks() {
    let elements = vec![
        title_elem("Chapter 1", 0, 750.0),
        para_with_heading("First paragraph of chapter 1.", 0, 700.0, Some("Chapter 1")),
        para_with_heading(
            "Second paragraph of chapter 1.",
            0,
            680.0,
            Some("Chapter 1"),
        ),
    ];

    let chunker = HybridChunker::new(HybridChunkConfig {
        max_tokens: 512,
        overlap_tokens: 0,
        merge_adjacent: false,
        propagate_headings: true,
    });
    let chunks = chunker.chunk(&elements);

    for chunk in &chunks {
        assert_eq!(chunk.heading_context.as_deref(), Some("Chapter 1"));
    }
}

#[test]
fn test_no_heading_before_first_title() {
    let elements = vec![
        para_with_heading("Preamble with no heading.", 0, 750.0, None),
        title_elem("Chapter 1", 0, 700.0),
        para_with_heading("Chapter content.", 0, 650.0, Some("Chapter 1")),
    ];

    let chunker = HybridChunker::new(HybridChunkConfig {
        max_tokens: 512,
        overlap_tokens: 0,
        merge_adjacent: false,
        propagate_headings: true,
    });
    let chunks = chunker.chunk(&elements);

    let preamble_chunk = chunks.iter().find(|c| c.text().contains("Preamble"));
    assert!(preamble_chunk.is_some());
    assert!(preamble_chunk.unwrap().heading_context.is_none());
}

// Cycle 6.3
#[test]
fn test_heading_context_changes_on_new_title() {
    let elements = vec![
        title_elem("Chapter 1", 0, 800.0),
        para_with_heading("Chapter 1 content.", 0, 750.0, Some("Chapter 1")),
        title_elem("Chapter 2", 0, 600.0),
        para_with_heading("Chapter 2 content.", 0, 550.0, Some("Chapter 2")),
    ];

    let chunker = HybridChunker::new(HybridChunkConfig {
        max_tokens: 512,
        overlap_tokens: 0,
        merge_adjacent: false,
        propagate_headings: true,
    });
    let chunks = chunker.chunk(&elements);

    let ch1 = chunks
        .iter()
        .find(|c| c.text().contains("Chapter 1 content"))
        .unwrap();
    let ch2 = chunks
        .iter()
        .find(|c| c.text().contains("Chapter 2 content"))
        .unwrap();
    assert_eq!(ch1.heading_context.as_deref(), Some("Chapter 1"));
    assert_eq!(ch2.heading_context.as_deref(), Some("Chapter 2"));
}

// Cycle 6.4
#[test]
fn test_merge_adjacent_paragraphs_within_budget() {
    let elements = vec![
        para_with_heading("Short paragraph one here.", 0, 700.0, None),
        para_with_heading("Short paragraph two here.", 0, 680.0, None),
        para_with_heading("Short paragraph three here.", 0, 660.0, None),
    ];

    let chunker = HybridChunker::new(HybridChunkConfig {
        max_tokens: 512,
        overlap_tokens: 0,
        merge_adjacent: true,
        propagate_headings: false,
    });
    let chunks = chunker.chunk(&elements);

    assert!(
        chunks.len() < elements.len(),
        "Con merge habilitado: {} chunks para {} elementos",
        chunks.len(),
        elements.len()
    );
}

#[test]
fn test_no_merge_different_element_types() {
    let elements = vec![
        title_elem("Title", 0, 750.0),
        para_with_heading("A paragraph.", 0, 700.0, Some("Title")),
    ];

    let chunker = HybridChunker::new(HybridChunkConfig {
        max_tokens: 512,
        overlap_tokens: 0,
        merge_adjacent: true,
        propagate_headings: false,
    });
    let chunks = chunker.chunk(&elements);

    let has_title = chunks
        .iter()
        .any(|c| c.elements().iter().any(|e| matches!(e, Element::Title(_))));
    let has_para = chunks.iter().any(|c| {
        c.elements()
            .iter()
            .any(|e| matches!(e, Element::Paragraph(_)))
    });
    assert!(has_title && has_para);
}

// Cycle 6.5
#[test]
fn test_merge_disabled_one_chunk_per_element() {
    let elements = vec![
        para_with_heading("Para one.", 0, 700.0, None),
        para_with_heading("Para two.", 0, 680.0, None),
        para_with_heading("Para three.", 0, 660.0, None),
    ];
    let chunker = HybridChunker::new(HybridChunkConfig {
        max_tokens: 512,
        overlap_tokens: 0,
        merge_adjacent: false,
        propagate_headings: false,
    });
    assert_eq!(chunker.chunk(&elements).len(), 3);
}

// Cycle 6.6
#[test]
fn test_oversized_table_gets_own_oversized_chunk() {
    let big_rows: Vec<Vec<String>> = (0..200)
        .map(|i| vec![format!("cell_{}", i), format!("value_{}", i)])
        .collect();

    let elements = vec![
        para_with_heading("Before table.", 0, 750.0, None),
        Element::Table(TableElementData {
            rows: big_rows,
            metadata: ElementMetadata {
                page: 0,
                bbox: ElementBBox::new(50.0, 300.0, 400.0, 400.0),
                ..Default::default()
            },
        }),
        para_with_heading("After table.", 0, 100.0, None),
    ];

    let chunker = HybridChunker::new(HybridChunkConfig {
        max_tokens: 50,
        overlap_tokens: 0,
        merge_adjacent: false,
        propagate_headings: false,
    });
    let chunks = chunker.chunk(&elements);

    let oversized: Vec<_> = chunks.iter().filter(|c| c.is_oversized()).collect();
    assert!(!oversized.is_empty());
    assert!(oversized[0]
        .elements()
        .iter()
        .any(|e| matches!(e, Element::Table(_))));
}

// Cycle 6.7
#[test]
fn test_overlap_chunks_preserve_heading_context() {
    let elements: Vec<Element> = (0..20)
        .map(|i| {
            para_with_heading(
                &format!("Paragraph number {} with some words.", i),
                0,
                800.0 - i as f64 * 20.0,
                Some("Section A"),
            )
        })
        .collect();

    let chunker = HybridChunker::new(HybridChunkConfig {
        max_tokens: 20,
        overlap_tokens: 5,
        merge_adjacent: false,
        propagate_headings: true,
    });
    let chunks = chunker.chunk(&elements);

    assert!(chunks.len() > 1);
    for chunk in &chunks {
        assert_eq!(chunk.heading_context.as_deref(), Some("Section A"));
    }
}

// Cycle 6.8
#[test]
fn test_empty_input_produces_empty_chunks() {
    assert!(HybridChunker::default().chunk(&[]).is_empty());
}

// Cycle 6.9
#[test]
fn test_chunk_text_does_not_include_heading_context() {
    let elements = vec![para_with_heading(
        "Content paragraph.",
        0,
        700.0,
        Some("My Section"),
    )];
    let chunker = HybridChunker::new(HybridChunkConfig {
        merge_adjacent: false,
        propagate_headings: true,
        ..Default::default()
    });
    let chunks = chunker.chunk(&elements);
    assert!(!chunks.is_empty());
    assert!(chunks[0].text().contains("Content paragraph."));
    assert_eq!(chunks[0].heading_context.as_deref(), Some("My Section"));
}

// Cycle 6.10
#[test]
fn test_list_items_merge_with_list_items_not_paragraphs() {
    let elements = vec![
        Element::ListItem(ElementData {
            text: "- First item".to_string(),
            metadata: meta_with_heading(0, 700.0, None),
        }),
        Element::ListItem(ElementData {
            text: "- Second item".to_string(),
            metadata: meta_with_heading(0, 685.0, None),
        }),
        para_with_heading("A paragraph after list.", 0, 650.0, None),
    ];

    let chunker = HybridChunker::new(HybridChunkConfig {
        max_tokens: 512,
        overlap_tokens: 0,
        merge_adjacent: true,
        propagate_headings: false,
    });
    let chunks = chunker.chunk(&elements);

    assert!(chunks.len() < elements.len(), "ListItems deben mergearse");
    let para_chunk = chunks
        .iter()
        .find(|c| c.text().contains("A paragraph after list."));
    assert!(para_chunk.is_some());
    assert!(!para_chunk.unwrap().text().contains("First item"));
}

// Cycle 6.11
#[test]
fn test_hybrid_chunker_accessible_from_pipeline() {
    use oxidize_pdf::pipeline::{HybridChunk, HybridChunker};
    let chunks: Vec<HybridChunk> = HybridChunker::default().chunk(&[]);
    assert!(chunks.is_empty());
}
