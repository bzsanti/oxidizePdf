use oxidize_pdf::pipeline::{
    Element, ElementBBox, ElementData, ElementMetadata, HybridChunkConfig, HybridChunker,
    MergePolicy, RagChunk,
};

fn make_para(text: &str, page: u32, x: f64, y: f64) -> Element {
    Element::Paragraph(ElementData {
        text: text.to_string(),
        metadata: ElementMetadata {
            page,
            bbox: ElementBBox::new(x, y, 400.0, 12.0),
            ..Default::default()
        },
    })
}

#[test]
fn test_rag_chunk_has_required_fields() {
    let chunk = RagChunk {
        chunk_index: 0,
        text: "Hello world.".to_string(),
        full_text: "Introduction\n\nHello world.".to_string(),
        page_numbers: vec![0],
        bounding_boxes: vec![ElementBBox::new(50.0, 700.0, 400.0, 12.0)],
        element_types: vec!["paragraph".to_string()],
        heading_context: Some("Introduction".to_string()),
        token_estimate: 2,
        is_oversized: false,
    };

    assert_eq!(chunk.chunk_index, 0);
    assert_eq!(chunk.text, "Hello world.");
    assert_eq!(chunk.full_text, "Introduction\n\nHello world.");
    assert_eq!(chunk.page_numbers, vec![0u32]);
    assert_eq!(chunk.bounding_boxes.len(), 1);
    assert_eq!(chunk.element_types, vec!["paragraph"]);
    assert_eq!(chunk.heading_context.as_deref(), Some("Introduction"));
    assert_eq!(chunk.token_estimate, 2);
    assert!(!chunk.is_oversized);
}

// ── Cycle 1.2: from_hybrid_chunk ──

fn para_at(text: &str, page: u32, y: f64) -> Element {
    Element::Paragraph(ElementData {
        text: text.to_string(),
        metadata: ElementMetadata {
            page,
            bbox: ElementBBox::new(50.0, y, 400.0, 12.0),
            parent_heading: Some("Section".to_string()),
            ..Default::default()
        },
    })
}

#[test]
fn test_rag_chunk_from_hybrid_chunk() {
    let elements = vec![
        para_at("First sentence.", 2, 700.0),
        para_at("Second sentence.", 2, 680.0),
    ];
    let chunker = HybridChunker::new(HybridChunkConfig {
        max_tokens: 512,
        overlap_tokens: 0,
        merge_adjacent: true,
        propagate_headings: true,
        merge_policy: MergePolicy::AnyInlineContent,
    });
    let hybrid_chunks = chunker.chunk(&elements);
    assert_eq!(hybrid_chunks.len(), 1);

    let rag = RagChunk::from_hybrid_chunk(0, &hybrid_chunks[0]);

    assert_eq!(rag.chunk_index, 0);
    assert!(rag.text.contains("First sentence."));
    assert!(rag.text.contains("Second sentence."));
    assert_eq!(rag.page_numbers, vec![2u32]);
    assert_eq!(rag.bounding_boxes.len(), 2);
    assert!(rag.element_types.iter().all(|t| t == "paragraph"));
    assert_eq!(rag.heading_context.as_deref(), Some("Section"));
    assert!(rag.token_estimate > 0);
    assert!(!rag.is_oversized);
    assert!(rag.full_text.contains("Section"));
    assert!(rag.full_text.contains("First sentence."));
}

// ── Cycle 1.3: multi-page deduplication ──

#[test]
fn test_rag_chunk_collects_pages_from_multi_page_elements() {
    let elements = vec![
        make_para("Page zero.", 0, 50.0, 700.0),
        make_para("Page one first.", 1, 50.0, 700.0),
        make_para("Page one second.", 1, 50.0, 680.0),
    ];
    let chunker = HybridChunker::new(HybridChunkConfig {
        max_tokens: 512,
        overlap_tokens: 0,
        merge_adjacent: true,
        merge_policy: MergePolicy::AnyInlineContent,
        propagate_headings: false,
    });
    let hybrid_chunks = chunker.chunk(&elements);
    let rag = RagChunk::from_hybrid_chunk(0, &hybrid_chunks[0]);

    assert_eq!(rag.page_numbers, vec![0u32, 1u32]);
    assert_eq!(rag.bounding_boxes.len(), 3);
}

// ── Cycle 2.2: JSON serialization (feature semantic) ──

#[cfg(feature = "semantic")]
#[test]
fn test_rag_chunk_serializes_to_json() {
    let chunk = RagChunk {
        chunk_index: 3,
        text: "Some text content.".to_string(),
        full_text: "Heading\n\nSome text content.".to_string(),
        page_numbers: vec![0, 1],
        bounding_boxes: vec![ElementBBox::new(50.0, 700.0, 400.0, 12.0)],
        element_types: vec!["paragraph".to_string()],
        heading_context: Some("Heading".to_string()),
        token_estimate: 3,
        is_oversized: false,
    };

    let json = serde_json::to_string(&chunk).expect("serialization must succeed");

    assert!(json.contains("\"chunk_index\":3"));
    assert!(json.contains("\"text\":"));
    assert!(json.contains("\"full_text\":"));
    assert!(json.contains("\"page_numbers\":[0,1]"));
    assert!(json.contains("\"heading_context\":\"Heading\""));
    assert!(json.contains("\"token_estimate\":3"));
    assert!(json.contains("\"is_oversized\":false"));
}

#[cfg(feature = "semantic")]
#[test]
fn test_rag_chunk_to_json_method() {
    let chunk = RagChunk {
        chunk_index: 0,
        text: "Test.".to_string(),
        full_text: "Test.".to_string(),
        page_numbers: vec![0],
        bounding_boxes: vec![],
        element_types: vec!["paragraph".to_string()],
        heading_context: None,
        token_estimate: 1,
        is_oversized: false,
    };

    let json = chunk.to_json().expect("to_json must succeed");
    assert!(json.starts_with('{'));
    assert!(json.contains("\"chunk_index\":0"));
}

// ── Cycle 3.2: chunk_index is sequential ──

#[test]
fn test_rag_chunks_indices_are_sequential() {
    let elements: Vec<Element> = (0..5)
        .map(|i| {
            Element::Title(ElementData {
                text: format!("Section {}", i),
                metadata: ElementMetadata {
                    page: i as u32,
                    ..Default::default()
                },
            })
        })
        .collect();

    let chunker = HybridChunker::new(HybridChunkConfig {
        max_tokens: 5,
        overlap_tokens: 0,
        merge_adjacent: false,
        propagate_headings: false,
        merge_policy: MergePolicy::AnyInlineContent,
    });
    let hybrid_chunks = chunker.chunk(&elements);
    let rag_chunks: Vec<RagChunk> = hybrid_chunks
        .iter()
        .enumerate()
        .map(|(idx, hc)| RagChunk::from_hybrid_chunk(idx, hc))
        .collect();

    for (expected_idx, chunk) in rag_chunks.iter().enumerate() {
        assert_eq!(chunk.chunk_index, expected_idx);
    }
}

// ── Cycle 3.3: custom config produces different chunk counts ──

#[test]
fn test_rag_chunks_with_custom_config_produces_more_chunks() {
    let elements: Vec<Element> = (0..10)
        .map(|i| {
            Element::Paragraph(ElementData {
                text: format!("word word word word word word word word word {}", i),
                metadata: ElementMetadata {
                    page: 0,
                    ..Default::default()
                },
            })
        })
        .collect();

    let small_config = HybridChunkConfig {
        max_tokens: 15,
        overlap_tokens: 0,
        merge_adjacent: true,
        propagate_headings: false,
        merge_policy: MergePolicy::AnyInlineContent,
    };
    let large_config = HybridChunkConfig {
        max_tokens: 512,
        overlap_tokens: 0,
        merge_adjacent: true,
        propagate_headings: false,
        merge_policy: MergePolicy::AnyInlineContent,
    };

    let small_chunks: Vec<RagChunk> = HybridChunker::new(small_config)
        .chunk(&elements)
        .iter()
        .enumerate()
        .map(|(idx, hc)| RagChunk::from_hybrid_chunk(idx, hc))
        .collect();

    let large_chunks: Vec<RagChunk> = HybridChunker::new(large_config)
        .chunk(&elements)
        .iter()
        .enumerate()
        .map(|(idx, hc)| RagChunk::from_hybrid_chunk(idx, hc))
        .collect();

    assert!(
        small_chunks.len() > large_chunks.len(),
        "smaller max_tokens must produce more chunks: small={}, large={}",
        small_chunks.len(),
        large_chunks.len()
    );
}

// ── Cycle 4.1: Vec<RagChunk> serializes as JSON array ──

#[cfg(feature = "semantic")]
#[test]
fn test_rag_chunks_json_serializes_vec_as_array() {
    let elements = vec![
        Element::Paragraph(ElementData {
            text: "First chunk.".to_string(),
            metadata: ElementMetadata {
                page: 0,
                ..Default::default()
            },
        }),
        Element::Title(ElementData {
            text: "Section Two".to_string(),
            metadata: ElementMetadata {
                page: 1,
                ..Default::default()
            },
        }),
    ];

    let chunker = HybridChunker::new(HybridChunkConfig {
        max_tokens: 5,
        overlap_tokens: 0,
        merge_adjacent: false,
        propagate_headings: false,
        merge_policy: MergePolicy::AnyInlineContent,
    });

    let rag_chunks: Vec<RagChunk> = chunker
        .chunk(&elements)
        .iter()
        .enumerate()
        .map(|(idx, hc)| RagChunk::from_hybrid_chunk(idx, hc))
        .collect();

    let json = serde_json::to_string(&rag_chunks).expect("must serialize");
    assert!(json.starts_with('['));
    assert!(json.ends_with(']'));
    assert!(json.contains("\"chunk_index\""));
}

// ── page_numbers sorted numerically ──

#[test]
fn test_rag_chunk_page_numbers_are_sorted() {
    let elements = vec![
        make_para("Third page.", 3, 50.0, 700.0),
        make_para("First page.", 1, 50.0, 700.0),
        make_para("Second page.", 2, 50.0, 700.0),
    ];
    let chunker = HybridChunker::new(HybridChunkConfig {
        max_tokens: 512,
        overlap_tokens: 0,
        merge_adjacent: true,
        merge_policy: MergePolicy::AnyInlineContent,
        propagate_headings: false,
    });
    let hybrid_chunks = chunker.chunk(&elements);
    let rag = RagChunk::from_hybrid_chunk(0, &hybrid_chunks[0]);

    assert_eq!(
        rag.page_numbers,
        vec![1u32, 2u32, 3u32],
        "page_numbers must be deduplicated and sorted numerically"
    );
}

// ── element_type_name covers all variants ──

#[test]
fn test_element_type_names_for_all_variants() {
    use oxidize_pdf::pipeline::{ImageElementData, KeyValueElementData, TableElementData};

    fn single_chunk(element: Element) -> RagChunk {
        let chunker = HybridChunker::new(HybridChunkConfig {
            max_tokens: 512,
            overlap_tokens: 0,
            merge_adjacent: false,
            propagate_headings: false,
            merge_policy: MergePolicy::AnyInlineContent,
        });
        let chunks = chunker.chunk(&[element]);
        RagChunk::from_hybrid_chunk(0, &chunks[0])
    }

    let md = ElementMetadata::default();

    let cases: Vec<(&str, Element)> = vec![
        (
            "title",
            Element::Title(ElementData {
                text: "T".into(),
                metadata: md.clone(),
            }),
        ),
        (
            "paragraph",
            Element::Paragraph(ElementData {
                text: "P".into(),
                metadata: md.clone(),
            }),
        ),
        (
            "header",
            Element::Header(ElementData {
                text: "H".into(),
                metadata: md.clone(),
            }),
        ),
        (
            "footer",
            Element::Footer(ElementData {
                text: "F".into(),
                metadata: md.clone(),
            }),
        ),
        (
            "list_item",
            Element::ListItem(ElementData {
                text: "L".into(),
                metadata: md.clone(),
            }),
        ),
        (
            "code_block",
            Element::CodeBlock(ElementData {
                text: "C".into(),
                metadata: md.clone(),
            }),
        ),
        (
            "image",
            Element::Image(ImageElementData {
                alt_text: Some("img".into()),
                metadata: md.clone(),
            }),
        ),
        (
            "key_value",
            Element::KeyValue(KeyValueElementData {
                key: "k".into(),
                value: "v".into(),
                metadata: md.clone(),
            }),
        ),
        (
            "table",
            Element::Table(TableElementData {
                rows: vec![vec!["a".into()]],
                metadata: md.clone(),
            }),
        ),
    ];

    for (expected_name, element) in cases {
        let chunk = single_chunk(element);
        assert_eq!(
            chunk.element_types[0], expected_name,
            "element_type for variant {expected_name}"
        );
    }
}
