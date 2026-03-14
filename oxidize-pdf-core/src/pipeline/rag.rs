use std::collections::HashSet;

use crate::pipeline::element::Element;
use crate::pipeline::hybrid_chunking::HybridChunk;
use crate::pipeline::ElementBBox;

#[cfg(feature = "semantic")]
use serde::{Deserialize, Serialize};

/// A RAG-ready chunk with full metadata for vector store ingestion.
///
/// Each `RagChunk` carries everything a vector store needs: text for embedding,
/// heading context for retrieval, and structural metadata (pages, bounding boxes,
/// element types) for citation and filtering.
///
/// Construct via [`RagChunk::from_hybrid_chunk`] or [`PdfDocument::rag_chunks()`].
#[derive(Debug, Clone)]
#[cfg_attr(feature = "semantic", derive(Serialize, Deserialize))]
pub struct RagChunk {
    /// Sequential index of this chunk in the document (0-based).
    pub chunk_index: usize,
    /// Chunk text content (elements joined by newlines).
    pub text: String,
    /// Text with heading context prepended — use this for embedding generation.
    pub full_text: String,
    /// Page numbers where this chunk's elements appear (deduplicated, ordered).
    pub page_numbers: Vec<u32>,
    /// Bounding boxes of each element in the chunk.
    pub bounding_boxes: Vec<ElementBBox>,
    /// Type names of each element (e.g. "title", "paragraph", "table").
    pub element_types: Vec<String>,
    /// Heading context inherited from the nearest parent heading.
    pub heading_context: Option<String>,
    /// Approximate token count (word-count proxy).
    pub token_estimate: usize,
    /// Whether the chunk exceeds the configured `max_tokens`.
    pub is_oversized: bool,
}

impl RagChunk {
    /// Build a `RagChunk` from a [`HybridChunk`], extracting all metadata from its elements.
    pub fn from_hybrid_chunk(chunk_index: usize, chunk: &HybridChunk) -> Self {
        let elements = chunk.elements();
        let page_numbers = collect_pages(elements);
        let bounding_boxes = elements.iter().map(|e| *e.bbox()).collect();
        let element_types = elements.iter().map(element_type_name).collect();

        Self {
            chunk_index,
            text: chunk.text(),
            full_text: chunk.full_text(),
            page_numbers,
            bounding_boxes,
            element_types,
            heading_context: chunk.heading_context.clone(),
            token_estimate: chunk.token_estimate(),
            is_oversized: chunk.is_oversized(),
        }
    }

    /// Serialize this chunk to a JSON string (requires `semantic` feature).
    #[cfg(feature = "semantic")]
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }
}

/// Collect unique page numbers from elements, preserving first-occurrence order.
fn collect_pages(elements: &[Element]) -> Vec<u32> {
    let mut seen = HashSet::new();
    let mut pages = Vec::new();
    for e in elements {
        let p = e.page();
        if seen.insert(p) {
            pages.push(p);
        }
    }
    pages
}

/// Map an `Element` variant to its snake_case type name.
fn element_type_name(element: &Element) -> String {
    match element {
        Element::Title(_) => "title",
        Element::Paragraph(_) => "paragraph",
        Element::Table(_) => "table",
        Element::Header(_) => "header",
        Element::Footer(_) => "footer",
        Element::ListItem(_) => "list_item",
        Element::Image(_) => "image",
        Element::CodeBlock(_) => "code_block",
        Element::KeyValue(_) => "key_value",
    }
    .to_string()
}
