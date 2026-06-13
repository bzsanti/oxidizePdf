//! Shared RAG test helpers.

use oxidize_pdf::pipeline::{
    Element, ElementBBox, ElementData, ElementMetadata, HybridChunkConfig, HybridChunker,
    MergePolicy, RagChunk,
};

/// Build a `RagChunk` with caller-pinned public fields.
///
/// `RagChunk` is `#[non_exhaustive]`, so external crates cannot build it via a
/// struct literal. Construct one through the public `from_hybrid_chunk` API and
/// override the public fields each test needs to pin.
#[allow(clippy::too_many_arguments)]
pub fn make_rag_chunk(
    chunk_index: usize,
    text: &str,
    full_text: &str,
    page_numbers: Vec<u32>,
    bounding_boxes: Vec<ElementBBox>,
    element_types: Vec<String>,
    heading_context: Option<String>,
    token_estimate: usize,
    is_oversized: bool,
) -> RagChunk {
    let chunker = HybridChunker::new(HybridChunkConfig {
        max_tokens: 512,
        overlap_tokens: 0,
        merge_adjacent: false,
        propagate_headings: false,
        merge_policy: MergePolicy::AnyInlineContent,
    });
    let chunks = chunker.chunk(&[Element::Paragraph(ElementData {
        text: text.to_string(),
        metadata: ElementMetadata::default(),
    })]);
    let mut chunk = RagChunk::from_hybrid_chunk(chunk_index, &chunks[0]);
    chunk.chunk_index = chunk_index;
    chunk.text = text.to_string();
    chunk.full_text = full_text.to_string();
    chunk.page_numbers = page_numbers;
    chunk.bounding_boxes = bounding_boxes;
    chunk.element_types = element_types;
    chunk.heading_context = heading_context;
    chunk.token_estimate = token_estimate;
    chunk.is_oversized = is_oversized;
    chunk
}
