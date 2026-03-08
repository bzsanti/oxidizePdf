use oxidize_pdf::pipeline::{
    Element, ElementBBox, ElementData, ElementMetadata, SemanticChunkConfig, SemanticChunker,
    TableElementData,
};

fn para(text: &str, page: u32, y: f64) -> Element {
    Element::Paragraph(ElementData {
        text: text.to_string(),
        metadata: ElementMetadata {
            page,
            bbox: ElementBBox::new(50.0, y, 500.0, 12.0),
            ..Default::default()
        },
    })
}

fn title(text: &str, page: u32, y: f64) -> Element {
    Element::Title(ElementData {
        text: text.to_string(),
        metadata: ElementMetadata {
            page,
            bbox: ElementBBox::new(50.0, y, 500.0, 24.0),
            ..Default::default()
        },
    })
}

// --- Step 5.1: Config ---

#[test]
fn test_semantic_chunk_config_defaults() {
    let cfg = SemanticChunkConfig::default();
    assert_eq!(cfg.max_tokens, 512);
    assert_eq!(cfg.overlap_tokens, 50);
    assert!(cfg.respect_element_boundaries);
}

#[test]
fn test_semantic_chunk_config_builder() {
    let cfg = SemanticChunkConfig::new(1024).with_overlap(100);
    assert_eq!(cfg.max_tokens, 1024);
    assert_eq!(cfg.overlap_tokens, 100);
}

// --- Step 5.2: Chunking respects boundaries ---

#[test]
fn test_semantic_chunk_title_not_split() {
    let elements = vec![
        title("Introduction", 0, 750.0),
        para(&"word ".repeat(200), 0, 700.0), // ~200 tokens
    ];

    let chunks =
        SemanticChunker::new(SemanticChunkConfig::new(100).with_overlap(0)).chunk(&elements);

    // Title should be entirely in first chunk, not split
    assert!(!chunks.is_empty());
    assert!(chunks[0].text().contains("Introduction"));
}

#[test]
fn test_semantic_chunk_table_stays_whole() {
    let elements = vec![
        para("Before table.", 0, 750.0),
        Element::Table(TableElementData {
            rows: vec![
                vec!["A".into(), "B".into()],
                vec!["1".into(), "2".into()],
                vec!["3".into(), "4".into()],
            ],
            metadata: ElementMetadata {
                page: 0,
                bbox: ElementBBox::new(50.0, 600.0, 400.0, 100.0),
                ..Default::default()
            },
        }),
        para("After table.", 0, 450.0),
    ];

    let chunks =
        SemanticChunker::new(SemanticChunkConfig::new(50).with_overlap(0)).chunk(&elements);

    // Table should appear in exactly one chunk
    let table_chunks: Vec<_> = chunks
        .iter()
        .filter(|c| c.elements().iter().any(|e| matches!(e, Element::Table(_))))
        .collect();
    assert_eq!(table_chunks.len(), 1);
}

#[test]
fn test_semantic_chunk_large_table_allowed_to_overflow() {
    let big_table = Element::Table(TableElementData {
        rows: (0..50)
            .map(|i| vec![format!("cell_{}_0", i), format!("cell_{}_1", i)])
            .collect(),
        metadata: ElementMetadata {
            page: 0,
            bbox: ElementBBox::new(50.0, 100.0, 400.0, 500.0),
            ..Default::default()
        },
    });

    let chunks =
        SemanticChunker::new(SemanticChunkConfig::new(20).with_overlap(0)).chunk(&[big_table]);

    // Table gets its own chunk even though > max_tokens
    assert_eq!(chunks.len(), 1);
    assert!(chunks[0].is_oversized());
}

#[test]
fn test_semantic_chunk_paragraph_split_at_sentence() {
    let long_text = "First sentence here. Second sentence here. Third sentence here. Fourth sentence here. Fifth sentence here.";
    let elements = vec![para(long_text, 0, 700.0)];

    let chunks = SemanticChunker::new(SemanticChunkConfig::new(5).with_overlap(0)).chunk(&elements);

    // Should split into multiple chunks
    assert!(chunks.len() > 1);
    // Each chunk text should end at a sentence boundary (period + space or end of string)
    for chunk in &chunks {
        let text = chunk.text();
        let trimmed = text.trim();
        if !trimmed.is_empty() {
            assert!(
                trimmed.ends_with('.') || trimmed.ends_with('!') || trimmed.ends_with('?'),
                "Chunk should end at sentence boundary: {:?}",
                trimmed
            );
        }
    }
}

#[test]
fn test_semantic_chunk_preserves_metadata() {
    let elements = vec![
        title("Chapter 1", 0, 750.0),
        para("Body text on page 0.", 0, 700.0),
        para("Body text on page 1.", 1, 700.0),
    ];

    let chunks =
        SemanticChunker::new(SemanticChunkConfig::new(512).with_overlap(0)).chunk(&elements);

    // Single chunk should span pages 0 and 1
    assert_eq!(chunks.len(), 1);
    let pages = chunks[0].page_numbers();
    assert!(pages.contains(&0));
    assert!(pages.contains(&1));
}

// --- Step 5.3: SemanticChunk output ---

#[test]
fn test_semantic_chunk_has_elements() {
    let elements = vec![
        title("Heading", 0, 750.0),
        para("Paragraph text.", 0, 700.0),
    ];

    let chunks = SemanticChunker::default().chunk(&elements);

    assert!(!chunks.is_empty());
    for chunk in &chunks {
        assert!(!chunk.elements().is_empty());
        assert!(!chunk.text().is_empty());
        assert!(chunk.token_estimate() > 0);
    }
}

#[test]
fn test_semantic_chunk_text_concatenation() {
    let elements = vec![title("Title", 0, 750.0), para("Body.", 0, 700.0)];

    let chunks =
        SemanticChunker::new(SemanticChunkConfig::new(512).with_overlap(0)).chunk(&elements);

    assert_eq!(chunks.len(), 1);
    let text = chunks[0].text();
    assert!(text.contains("Title"));
    assert!(text.contains("Body."));
}
