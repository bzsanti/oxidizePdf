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
