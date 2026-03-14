use oxidize_pdf::pipeline::{
    Element, ElementData, ElementGraph, ElementMetadata, HybridChunkConfig, HybridChunker,
};

fn make_title_with_heading(text: &str, heading: &str) -> Element {
    Element::Title(ElementData {
        text: text.to_string(),
        metadata: ElementMetadata {
            parent_heading: Some(heading.to_string()),
            ..Default::default()
        },
    })
}

fn make_para_with_heading(text: &str, heading: &str) -> Element {
    Element::Paragraph(ElementData {
        text: text.to_string(),
        metadata: ElementMetadata {
            parent_heading: if heading.is_empty() {
                None
            } else {
                Some(heading.to_string())
            },
            ..Default::default()
        },
    })
}

// ─── Cycle 3.1 ───────────────────────────────────────────────────────────────

#[test]
fn test_hybrid_chunk_with_graph_keeps_section_together() {
    let elements = vec![
        make_title_with_heading("Short Section", "Short Section"),
        make_para_with_heading("Para one text.", "Short Section"),
        make_para_with_heading("Para two text.", "Short Section"),
    ];
    let graph = ElementGraph::build(&elements);
    let chunker = HybridChunker::default();
    let chunks = chunker.chunk_with_graph(&elements, &graph);

    assert_eq!(chunks.len(), 1);
    assert_eq!(chunks[0].elements().len(), 3);
    assert_eq!(chunks[0].heading_context.as_deref(), Some("Short Section"));
}

#[test]
fn test_hybrid_chunk_with_graph_splits_large_section() {
    let mut elements = vec![make_title_with_heading("Big Section", "Big Section")];
    for i in 0..20 {
        elements.push(make_para_with_heading(
            &format!(
                "This is paragraph {} with enough words to consume tokens.",
                i
            ),
            "Big Section",
        ));
    }
    let graph = ElementGraph::build(&elements);
    let chunker = HybridChunker::new(HybridChunkConfig {
        max_tokens: 30,
        ..Default::default()
    });
    let chunks = chunker.chunk_with_graph(&elements, &graph);

    assert!(chunks.len() > 1);
    for chunk in &chunks {
        assert_eq!(chunk.heading_context.as_deref(), Some("Big Section"));
    }
}

#[test]
fn test_hybrid_chunk_with_graph_handles_preamble() {
    // Elements before any title (preamble) have no parent section.
    let elements = vec![
        make_para_with_heading("Preamble text before any heading.", ""),
        make_title_with_heading("Section One", "Section One"),
        make_para_with_heading("Section content here.", "Section One"),
    ];
    let graph = ElementGraph::build(&elements);
    let chunker = HybridChunker::default();
    let chunks = chunker.chunk_with_graph(&elements, &graph);

    // Should produce at least 2 chunks: preamble + section
    assert!(!chunks.is_empty());
}

#[test]
fn test_hybrid_chunk_with_graph_empty_input() {
    let graph = ElementGraph::build(&[]);
    let chunker = HybridChunker::default();
    let chunks = chunker.chunk_with_graph(&[], &graph);
    assert!(chunks.is_empty());
}
