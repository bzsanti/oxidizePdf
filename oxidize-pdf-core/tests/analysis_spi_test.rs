//! Integration tests for the unstable analysis SPI.
#![cfg(feature = "unstable-spi")]

use oxidize_pdf::pipeline::{ChunkGroup, ChunkingStrategy};
use oxidize_pdf::pipeline::{Element, ElementData, ElementMetadata};

/// A strategy that emits exactly one chunk per element.
struct OnePerElement;

impl ChunkingStrategy for OnePerElement {
    fn chunk(&self, elements: &[Element]) -> Vec<ChunkGroup> {
        elements
            .iter()
            .map(|e| ChunkGroup::new(vec![e.clone()], None))
            .collect()
    }
}

fn para(text: &str) -> Element {
    Element::Paragraph(ElementData {
        text: text.to_string(),
        metadata: ElementMetadata::default(),
    })
}

#[test]
fn custom_strategy_is_object_safe_and_groups_per_element() {
    let strategy: Box<dyn ChunkingStrategy> = Box::new(OnePerElement);
    let elements = vec![para("alpha"), para("bravo"), para("charlie")];
    let groups = strategy.chunk(&elements);
    assert_eq!(groups.len(), 3, "one chunk per element");
    assert_eq!(groups[0].elements.len(), 1);
    assert_eq!(groups[0].elements[0].text(), "alpha");
    assert_eq!(groups[2].elements[0].text(), "charlie");
}

use oxidize_pdf::pipeline::{HybridChunkConfig, HybridChunker, MergePolicy};

#[test]
fn hybrid_chunker_is_the_default_strategy() {
    let elements = vec![para("alpha one two three"), para("bravo four five six")];
    let chunker = HybridChunker::new(HybridChunkConfig {
        max_tokens: 4,
        overlap_tokens: 0,
        merge_adjacent: true,
        propagate_headings: true,
        merge_policy: MergePolicy::AnyInlineContent,
    });

    // Inherent API: Vec<HybridChunk>.
    let hybrid = HybridChunker::chunk(&chunker, &elements);
    // Trait API: Vec<ChunkGroup>, same grouping.
    let groups = ChunkingStrategy::chunk(&chunker, &elements);

    assert_eq!(groups.len(), hybrid.len(), "same number of chunks");
    for (g, h) in groups.iter().zip(hybrid.iter()) {
        let g_text: Vec<&str> = g.elements.iter().map(|e| e.text()).collect();
        let h_text: Vec<&str> = h.elements().iter().map(|e| e.text()).collect();
        assert_eq!(g_text, h_text, "same element grouping");
        assert_eq!(g.heading_context, h.heading_context);
    }
}

use oxidize_pdf::parser::{PdfDocument, PdfReader};
use oxidize_pdf::pipeline::AnalysisPipeline;
use oxidize_pdf::text::Font;
use oxidize_pdf::{Document, Page};
use std::io::Cursor;

fn build_two_section_doc() -> Vec<u8> {
    let mut doc = Document::new();
    let mut page = Page::a4();
    page.text()
        .set_font(Font::HelveticaBold, 16.0)
        .at(50.0, 760.0)
        .write("Section One")
        .unwrap();
    page.text()
        .set_font(Font::Helvetica, 11.0)
        .at(50.0, 730.0)
        .write("First body paragraph with enough words to chunk on its own line.")
        .unwrap();
    page.text()
        .set_font(Font::Helvetica, 11.0)
        .at(50.0, 700.0)
        .write("Second body paragraph also with several words to fill a bucket.")
        .unwrap();
    doc.add_page(page);
    doc.to_bytes().expect("pdf generation")
}

#[test]
fn default_pipeline_matches_rag_chunks() {
    let bytes = build_two_section_doc();

    let parsed_a = PdfDocument::new(PdfReader::new(Cursor::new(&bytes)).unwrap());
    let baseline = parsed_a.rag_chunks().expect("rag_chunks");

    let parsed_b = PdfDocument::new(PdfReader::new(Cursor::new(&bytes)).unwrap());
    let via_pipeline = parsed_b
        .rag_chunks_with_pipeline(&AnalysisPipeline::new())
        .expect("rag_chunks_with_pipeline");

    assert_eq!(
        via_pipeline.len(),
        baseline.len(),
        "default pipeline produces the same chunk count"
    );
    for (p, b) in via_pipeline.iter().zip(baseline.iter()) {
        assert_eq!(p.text, b.text, "same chunk text");
        assert_eq!(p.metadata.chunk_id, b.metadata.chunk_id, "same chunk_id");
        assert_eq!(p.is_oversized, b.is_oversized, "same oversized flag");
        assert_eq!(
            p.metadata.prev_chunk_id, b.metadata.prev_chunk_id,
            "same prev link"
        );
    }
}

#[test]
fn custom_strategy_drives_chunk_count_and_pipeline_owns_ids() {
    let bytes = build_two_section_doc();
    let parsed = PdfDocument::new(PdfReader::new(Cursor::new(&bytes)).unwrap());

    let pipeline = AnalysisPipeline::new().with_chunking(Box::new(OnePerElement));
    let chunks = parsed
        .rag_chunks_with_pipeline(&pipeline)
        .expect("rag_chunks_with_pipeline");

    // One element per chunk → at least as many chunks as the default merge.
    assert!(chunks.len() >= 3, "one-per-element yields >= 3 chunks");
    // The pipeline (not the strategy) derived ids and links.
    assert!(chunks[0].metadata.prev_chunk_id.is_none());
    assert_eq!(
        chunks[0].metadata.next_chunk_id.as_deref(),
        Some(chunks[1].metadata.chunk_id.as_str()),
        "pipeline wired prev/next"
    );
    for c in &chunks {
        assert!(!c.metadata.chunk_id.is_empty());
    }
}

/// A strategy that delegates to the default and then merges every pair of
/// adjacent groups — proving "delegate to the default and refine".
struct PairMerger {
    inner: HybridChunker,
}

impl ChunkingStrategy for PairMerger {
    fn chunk(&self, elements: &[Element]) -> Vec<ChunkGroup> {
        let base = ChunkingStrategy::chunk(&self.inner, elements);
        let mut out = Vec::new();
        let mut iter = base.into_iter();
        while let Some(mut a) = iter.next() {
            if let Some(b) = iter.next() {
                a.elements.extend(b.elements);
            }
            out.push(a);
        }
        out
    }
}

#[test]
fn decorator_wraps_default_and_refines() {
    let elements = vec![para("alpha"), para("bravo"), para("charlie"), para("delta")];

    let inner = HybridChunker::new(HybridChunkConfig {
        max_tokens: 1, // force one element per group from the default
        overlap_tokens: 0,
        merge_adjacent: false,
        propagate_headings: false,
        merge_policy: MergePolicy::AnyInlineContent,
    });
    let base_count = ChunkingStrategy::chunk(&inner, &elements).len();
    assert_eq!(base_count, 4, "default emits one group per element here");

    let decorated = PairMerger { inner };
    let groups = decorated.chunk(&elements);
    assert_eq!(groups.len(), 2, "pairs merged: 4 groups -> 2");
    assert_eq!(groups[0].elements.len(), 2);
    assert_eq!(groups[1].elements.len(), 2);
}

// --- ElementClassifier seam (§7) ---

use oxidize_pdf::pipeline::{ClassLabel, ClassifyContext, ElementClassifier};

/// Classifier that labels any element whose text contains "CLAUSE".
struct MarkClause;

impl ElementClassifier for MarkClause {
    fn classify(&self, element: &Element, ctx: &ClassifyContext) -> Option<ClassLabel> {
        // ctx must expose the surrounding elements and this element's index.
        assert_eq!(ctx.elements[ctx.index].text(), element.text());
        if element.text().contains("CLAUSE") {
            Some(ClassLabel::new("clause"))
        } else {
            None
        }
    }
}

/// Strategy that copies each element's `class_label` into the group's
/// heading_context — making the classifier's effect observable downstream.
struct ExposeLabel;

impl ChunkingStrategy for ExposeLabel {
    fn chunk(&self, elements: &[Element]) -> Vec<ChunkGroup> {
        elements
            .iter()
            .map(|e| ChunkGroup::new(vec![e.clone()], e.metadata().class_label.clone()))
            .collect()
    }
}

#[test]
fn classifier_runs_before_chunking_and_sets_class_label() {
    let mut doc = Document::new();
    let mut page = Page::a4();
    page.text()
        .set_font(Font::Helvetica, 11.0)
        .at(50.0, 760.0)
        .write("Intro paragraph without the marker word here.")
        .unwrap();
    page.text()
        .set_font(Font::Helvetica, 11.0)
        .at(50.0, 730.0)
        .write("CLAUSE 1 the parties hereby agree to the following terms.")
        .unwrap();
    doc.add_page(page);
    let bytes = doc.to_bytes().expect("pdf generation");

    let parsed = PdfDocument::new(PdfReader::new(Cursor::new(&bytes)).unwrap());
    let pipeline = AnalysisPipeline::new()
        .with_classifier(Box::new(MarkClause))
        .with_chunking(Box::new(ExposeLabel));
    let chunks = parsed
        .rag_chunks_with_pipeline(&pipeline)
        .expect("rag_chunks_with_pipeline");

    // Exactly the chunk(s) whose text carries "CLAUSE" inherit the label;
    // others have no heading_context from a label.
    let labeled: Vec<&str> = chunks
        .iter()
        .filter(|c| c.heading_context.as_deref() == Some("clause"))
        .map(|c| c.text.as_str())
        .collect();
    assert_eq!(
        labeled.len(),
        1,
        "exactly one element carries the CLAUSE label"
    );
    assert!(labeled[0].contains("CLAUSE"));

    let unlabeled = chunks
        .iter()
        .filter(|c| c.heading_context.is_none())
        .count();
    assert!(unlabeled >= 1, "the intro element is unlabeled");
}

#[test]
fn default_pipeline_has_no_classifier_and_leaves_labels_unset() {
    let bytes = build_two_section_doc();
    let parsed = PdfDocument::new(PdfReader::new(Cursor::new(&bytes)).unwrap());
    // No classifier configured → ExposeLabel sees only None labels.
    let pipeline = AnalysisPipeline::new().with_chunking(Box::new(ExposeLabel));
    let chunks = parsed
        .rag_chunks_with_pipeline(&pipeline)
        .expect("rag_chunks_with_pipeline");
    assert!(
        chunks.iter().all(|c| c.heading_context.is_none()),
        "without a classifier, no element carries a class label"
    );
}

// --- MetadataEnricher seam (§7), gated on `semantic` (writes `extra`) ---

#[cfg(feature = "semantic")]
mod enricher {
    use super::*;
    use oxidize_pdf::pipeline::{ChunkMetadata, EnrichContext, MetadataEnricher};

    /// Enricher that stamps the chunk's word count and element count into the
    /// open `extra` bag, proving it can read text + elements and write `extra`.
    struct CountEnricher;

    impl MetadataEnricher for CountEnricher {
        fn enrich(&self, ctx: &EnrichContext, meta: &mut ChunkMetadata) {
            let words = ctx.text.split_whitespace().count();
            meta.extra
                .insert("test.word_count".to_string(), serde_json::json!(words));
            meta.extra.insert(
                "test.element_count".to_string(),
                serde_json::json!(ctx.elements.len()),
            );
            // heading_path is reachable too.
            meta.extra.insert(
                "test.heading_depth".to_string(),
                serde_json::json!(ctx.heading_path.len()),
            );
        }
    }

    #[test]
    fn enricher_writes_namespaced_keys_into_extra() {
        let bytes = build_two_section_doc();
        let parsed = PdfDocument::new(PdfReader::new(Cursor::new(&bytes)).unwrap());
        let pipeline = AnalysisPipeline::new().with_enricher(Box::new(CountEnricher));
        let chunks = parsed
            .rag_chunks_with_pipeline(&pipeline)
            .expect("rag_chunks_with_pipeline");

        assert!(!chunks.is_empty());
        for c in &chunks {
            let wc = c.metadata.extra.get("test.word_count").expect("word_count");
            assert_eq!(
                wc.as_u64().unwrap() as usize,
                c.text.split_whitespace().count(),
                "enricher saw the real chunk text"
            );
            let ec = c
                .metadata
                .extra
                .get("test.element_count")
                .expect("element_count");
            assert!(ec.as_u64().unwrap() >= 1, "chunk has >= 1 element");
            assert!(c.metadata.extra.contains_key("test.heading_depth"));
        }
    }

    #[test]
    fn enrichers_run_in_order_last_writer_wins_per_key() {
        struct StampA;
        struct StampB;
        impl MetadataEnricher for StampA {
            fn enrich(&self, _ctx: &EnrichContext, meta: &mut ChunkMetadata) {
                meta.extra
                    .insert("ns.order".to_string(), serde_json::json!("A"));
            }
        }
        impl MetadataEnricher for StampB {
            fn enrich(&self, _ctx: &EnrichContext, meta: &mut ChunkMetadata) {
                meta.extra
                    .insert("ns.order".to_string(), serde_json::json!("B"));
            }
        }

        let bytes = build_two_section_doc();
        let parsed = PdfDocument::new(PdfReader::new(Cursor::new(&bytes)).unwrap());
        let pipeline = AnalysisPipeline::new()
            .with_enricher(Box::new(StampA))
            .with_enricher(Box::new(StampB));
        let chunks = parsed
            .rag_chunks_with_pipeline(&pipeline)
            .expect("rag_chunks_with_pipeline");

        for c in &chunks {
            assert_eq!(
                c.metadata.extra.get("ns.order").unwrap(),
                &serde_json::json!("B"),
                "enrichers apply in insertion order; B runs after A"
            );
        }
    }

    #[test]
    fn no_enricher_leaves_extra_empty() {
        let bytes = build_two_section_doc();
        let parsed = PdfDocument::new(PdfReader::new(Cursor::new(&bytes)).unwrap());
        let chunks = parsed
            .rag_chunks_with_pipeline(&AnalysisPipeline::new())
            .expect("rag_chunks_with_pipeline");
        assert!(
            chunks.iter().all(|c| c.metadata.extra.is_empty()),
            "default pipeline does not touch extra"
        );
    }
}
