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

    // Each body paragraph must appear in exactly one chunk — the split must
    // not duplicate content across chunks.
    for i in 0..20 {
        let needle = format!("paragraph {} with enough words", i);
        let hits: usize = chunks.iter().filter(|c| c.text().contains(&needle)).count();
        assert_eq!(
            hits, 1,
            "body paragraph {} must appear in exactly one sub-chunk, found {}",
            i, hits
        );
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

    assert!(!chunks.is_empty());

    // Preamble must appear in a chunk with no heading_context.
    let preamble_chunk = chunks
        .iter()
        .find(|c| c.text().contains("Preamble text before any heading."));
    assert!(preamble_chunk.is_some(), "preamble chunk must be present");
    assert!(preamble_chunk.unwrap().heading_context.is_none());

    // Section content must appear exactly once, in a chunk whose heading_context
    // is "Section One". The preamble must not appear in that section chunk.
    for marker in &["Preamble text before any heading.", "Section content here."] {
        let hits: usize = chunks.iter().filter(|c| c.text().contains(marker)).count();
        assert_eq!(hits, 1, "{:?} must appear in exactly one chunk", marker);
    }
    let section_chunk = chunks
        .iter()
        .find(|c| c.text().contains("Section content here."))
        .unwrap();
    assert_eq!(
        section_chunk.heading_context.as_deref(),
        Some("Section One")
    );
    assert!(!section_chunk.text().contains("Preamble text"));
}

#[test]
fn test_hybrid_chunk_with_graph_empty_input() {
    let graph = ElementGraph::build(&[]);
    let chunker = HybridChunker::default();
    let chunks = chunker.chunk_with_graph(&[], &graph);
    assert!(chunks.is_empty());
}
