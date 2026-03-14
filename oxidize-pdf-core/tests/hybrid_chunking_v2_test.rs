use oxidize_pdf::pipeline::hybrid_chunking::{HybridChunkConfig, HybridChunker};
use oxidize_pdf::pipeline::{
    Element, ElementBBox, ElementData, ElementGraph, ElementMetadata, MergePolicy, PartitionConfig,
    Partitioner, TableElementData,
};
use oxidize_pdf::text::extraction::TextFragment;

// ── Shared helpers ────────────────────────────────────────────────────────────

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

fn list_item_with_heading(text: &str, page: u32, y: f64, heading: Option<&str>) -> Element {
    Element::ListItem(ElementData {
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

fn make_small_table() -> Element {
    Element::Table(TableElementData {
        rows: vec![
            vec!["Header A".to_string(), "Header B".to_string()],
            vec!["Row 1A".to_string(), "Row 1B".to_string()],
            vec!["Row 2A".to_string(), "Row 2B".to_string()],
        ],
        metadata: ElementMetadata {
            page: 0,
            bbox: ElementBBox::new(50.0, 400.0, 400.0, 100.0),
            ..Default::default()
        },
    })
}

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

// ── Phase 1: Agnostic merge (Cycles 1.1 – 1.3) ───────────────────────────────

// Cycle 1.1 — RED (confirmed) → GREEN after implementing AnyInlineContent.
// With SameTypeOnly, Paragraph+ListItem produce 3 chunks.
// With AnyInlineContent (new default), they merge into 1 chunk.
#[test]
fn test_merge_paragraph_and_list_item_under_same_heading() {
    let elements = vec![
        para_with_heading("Introduction text here.", 0, 700.0, Some("Section")),
        list_item_with_heading("- First bullet point.", 0, 680.0, Some("Section")),
        list_item_with_heading("- Second bullet point.", 0, 660.0, Some("Section")),
    ];
    let chunker = HybridChunker::new(HybridChunkConfig {
        max_tokens: 512,
        overlap_tokens: 0,
        merge_adjacent: true,
        propagate_headings: false,
        merge_policy: MergePolicy::AnyInlineContent,
    });
    let chunks = chunker.chunk(&elements);

    assert_eq!(
        chunks.len(),
        1,
        "Paragraph + ListItems under same heading should merge, got {} chunks",
        chunks.len()
    );
}

// Cycle 1.3 — Additional merge policy tests.

#[test]
fn test_same_type_only_policy_preserves_legacy_behavior() {
    let elements = vec![
        para_with_heading("Intro.", 0, 700.0, None),
        list_item_with_heading("- Item.", 0, 680.0, None),
    ];
    let chunker = HybridChunker::new(HybridChunkConfig {
        max_tokens: 512,
        overlap_tokens: 0,
        merge_adjacent: true,
        propagate_headings: false,
        merge_policy: MergePolicy::SameTypeOnly,
    });
    let chunks = chunker.chunk(&elements);
    assert_eq!(
        chunks.len(),
        2,
        "SameTypeOnly: Paragraph+ListItem must NOT merge"
    );
}

#[test]
fn test_title_never_merges_with_adjacent_paragraph() {
    let elements = vec![
        title_elem("Chapter", 0, 750.0),
        para_with_heading("Content.", 0, 700.0, Some("Chapter")),
    ];
    let chunker = HybridChunker::new(HybridChunkConfig {
        max_tokens: 512,
        overlap_tokens: 0,
        merge_adjacent: true,
        propagate_headings: false,
        merge_policy: MergePolicy::AnyInlineContent,
    });
    let chunks = chunker.chunk(&elements);
    // Title must be its own chunk — it is structural, not inline.
    assert!(
        chunks
            .iter()
            .any(|c| c.elements().iter().any(|e| matches!(e, Element::Title(_)))),
        "Title must appear in at least one chunk"
    );
    assert!(
        chunks.len() >= 2,
        "Title and Paragraph must produce at least 2 chunks"
    );
}

#[test]
fn test_table_never_merges_with_adjacent_paragraph() {
    let table = make_small_table();
    let elements = vec![
        para_with_heading("Before table.", 0, 750.0, None),
        table,
        para_with_heading("After table.", 0, 200.0, None),
    ];
    let chunker = HybridChunker::new(HybridChunkConfig {
        max_tokens: 512,
        overlap_tokens: 0,
        merge_adjacent: true,
        propagate_headings: false,
        merge_policy: MergePolicy::AnyInlineContent,
    });
    let chunks = chunker.chunk(&elements);
    // Table must be isolated in its own chunk.
    let table_chunks: Vec<_> = chunks
        .iter()
        .filter(|c| c.elements().iter().any(|e| matches!(e, Element::Table(_))))
        .collect();
    assert_eq!(table_chunks.len(), 1, "Table must be in exactly one chunk");
    assert_eq!(
        table_chunks[0].elements().len(),
        1,
        "Table chunk must contain only the table element"
    );
}

// ── Phase 2: Sentence splitting (Cycles 2.1 – 2.3) ───────────────────────────

// Cycle 2.1 — RED (confirmed) → GREEN after implementing split_by_sentences.
#[test]
fn test_oversized_paragraph_splits_at_sentence_boundary() {
    let long_text = "First sentence here. Second sentence follows. \
                     Third sentence is also present. Fourth sentence added. \
                     Fifth sentence completes the paragraph. Sixth one too.";
    let elements = vec![para_with_heading(long_text, 0, 700.0, Some("Section A"))];
    let chunker = HybridChunker::new(HybridChunkConfig {
        max_tokens: 10,
        overlap_tokens: 0,
        merge_adjacent: false,
        propagate_headings: true,
        merge_policy: MergePolicy::AnyInlineContent,
    });
    let chunks = chunker.chunk(&elements);

    assert!(
        chunks.len() > 1,
        "Oversized paragraph must split at sentence boundaries, got {} chunk(s)",
        chunks.len()
    );

    // Split text chunks must NOT be marked oversized.
    let text_oversized = chunks.iter().filter(|c| c.is_oversized()).count();
    assert_eq!(
        text_oversized, 0,
        "Split text chunks must not be marked oversized"
    );

    // Every split chunk must inherit the heading context.
    for chunk in &chunks {
        assert_eq!(
            chunk.heading_context.as_deref(),
            Some("Section A"),
            "All split chunks must inherit heading context"
        );
    }
}

// Cycle 2.3 — Complementary sentence split tests.

#[test]
fn test_table_oversized_remains_atomic_not_split() {
    let big_rows: Vec<Vec<String>> = (0..100)
        .map(|i| vec![format!("key_{}", i), format!("value_{}", i)])
        .collect();
    let elements = vec![Element::Table(TableElementData {
        rows: big_rows,
        metadata: ElementMetadata {
            page: 0,
            bbox: ElementBBox::new(50.0, 300.0, 400.0, 400.0),
            ..Default::default()
        },
    })];
    let chunker = HybridChunker::new(HybridChunkConfig {
        max_tokens: 20,
        overlap_tokens: 0,
        merge_adjacent: false,
        propagate_headings: false,
        merge_policy: MergePolicy::AnyInlineContent,
    });
    let chunks = chunker.chunk(&elements);

    assert_eq!(
        chunks.len(),
        1,
        "Oversized table must stay as one atomic chunk"
    );
    assert!(
        chunks[0].is_oversized(),
        "Table chunk must be marked oversized"
    );
}

#[test]
fn test_single_very_long_sentence_stays_in_one_chunk() {
    // 200 words with no sentence-ending punctuation — cannot be split further.
    let long_sentence = "word ".repeat(200);
    let long_sentence = long_sentence.trim();
    let elements = vec![para_with_heading(long_sentence, 0, 700.0, None)];
    let chunker = HybridChunker::new(HybridChunkConfig {
        max_tokens: 50,
        overlap_tokens: 0,
        merge_adjacent: false,
        propagate_headings: false,
        merge_policy: MergePolicy::AnyInlineContent,
    });
    let chunks = chunker.chunk(&elements);
    // No sentence boundaries → entire text is one "sentence" → one chunk.
    assert_eq!(
        chunks.len(),
        1,
        "A single unsplittable sentence must remain in one chunk"
    );
}

#[test]
fn test_sentence_split_preserves_parent_heading_metadata() {
    let text = "Sentence one. Sentence two. Sentence three. Sentence four.";
    let elements = vec![para_with_heading(text, 0, 700.0, Some("My Heading"))];
    let chunker = HybridChunker::new(HybridChunkConfig {
        max_tokens: 4,
        overlap_tokens: 0,
        merge_adjacent: false,
        propagate_headings: true,
        merge_policy: MergePolicy::AnyInlineContent,
    });
    let chunks = chunker.chunk(&elements);

    assert!(chunks.len() > 1, "Must produce multiple split chunks");
    for chunk in &chunks {
        assert_eq!(
            chunk.heading_context.as_deref(),
            Some("My Heading"),
            "Every split chunk must inherit heading context"
        );
    }
}

// ── Phase 3: full_text() (Cycle 3.1) ─────────────────────────────────────────

#[test]
fn test_full_text_prepends_heading_context() {
    let elements = vec![para_with_heading(
        "Content of section.",
        0,
        700.0,
        Some("Introduction"),
    )];
    let chunker = HybridChunker::new(HybridChunkConfig {
        propagate_headings: true,
        merge_adjacent: false,
        ..Default::default()
    });
    let chunks = chunker.chunk(&elements);
    assert!(!chunks.is_empty());

    let full = chunks[0].full_text();
    assert!(
        full.starts_with("Introduction"),
        "full_text must start with the heading context, got: {:?}",
        full
    );
    assert!(
        full.contains("Content of section."),
        "full_text must contain the chunk text"
    );
}

#[test]
fn test_full_text_equals_text_when_no_heading() {
    let elements = vec![para_with_heading("Just content.", 0, 700.0, None)];
    let chunker = HybridChunker::default();
    let chunks = chunker.chunk(&elements);
    assert!(!chunks.is_empty());
    assert_eq!(
        chunks[0].full_text(),
        chunks[0].text(),
        "full_text without heading must equal text()"
    );
}

// ── Phase 5: End-to-end with Partitioner + ElementGraph (Cycle 5.1) ──────────

#[test]
fn test_hybrid_chunker_v2_end_to_end_with_partition() {
    let fragments = vec![
        frag("Abstract", 50.0, 800.0, 20.0),
        frag("This paper presents a novel approach.", 50.0, 760.0, 12.0),
        frag(
            "Background and motivation are described below.",
            50.0,
            740.0,
            12.0,
        ),
        frag("Introduction", 50.0, 700.0, 20.0),
        frag(
            "The problem statement. This is a detailed description.",
            50.0,
            660.0,
            12.0,
        ),
        frag("- First key contribution of this work.", 50.0, 640.0, 12.0),
        frag(
            "- Second key contribution also important.",
            50.0,
            620.0,
            12.0,
        ),
    ];
    let elements =
        Partitioner::new(PartitionConfig::default()).partition_fragments(&fragments, 0, 842.0);

    let chunker = HybridChunker::new(HybridChunkConfig {
        max_tokens: 20,
        overlap_tokens: 5,
        merge_adjacent: true,
        propagate_headings: true,
        merge_policy: MergePolicy::AnyInlineContent,
    });

    let graph = ElementGraph::build(&elements);
    let chunks = chunker.chunk_with_graph(&elements, &graph);

    assert!(
        !chunks.is_empty(),
        "End-to-end chunking must produce chunks"
    );

    // Chunks for the Introduction section must carry heading context.
    let intro_chunks: Vec<_> = chunks
        .iter()
        .filter(|c| c.heading_context.as_deref() == Some("Introduction"))
        .collect();
    assert!(
        !intro_chunks.is_empty(),
        "Introduction section chunks must carry heading context"
    );

    // full_text must include the heading for intro chunks.
    for chunk in &intro_chunks {
        assert!(
            chunk.full_text().contains("Introduction"),
            "full_text must include the heading"
        );
    }
}
