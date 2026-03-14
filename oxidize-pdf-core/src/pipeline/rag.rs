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
/// Construct via [`PdfDocument::rag_chunks()`](crate::parser::PdfDocument::rag_chunks)
/// or [`PdfDocument::rag_chunks_with_profile()`](crate::parser::PdfDocument::rag_chunks_with_profile).
///
/// # Field guide
///
/// - `text`: raw chunk text for display or keyword search
/// - `full_text`: heading context + text — **use this for embedding generation**
/// - `token_estimate`: word-count proxy (multiply by ~1.5 for subword tokens)
/// - `is_oversized`: true when a single element exceeds `max_tokens`
///
/// # Example
///
/// ```rust,no_run
/// use oxidize_pdf::parser::PdfDocument;
/// use oxidize_pdf::pipeline::ExtractionProfile;
///
/// let doc = PdfDocument::open("paper.pdf")?;
/// let chunks = doc.rag_chunks_with_profile(ExtractionProfile::Rag)?;
///
/// for chunk in &chunks {
///     println!(
///         "[chunk {}] pages={:?} tokens~{} types={:?}",
///         chunk.chunk_index, chunk.page_numbers,
///         chunk.token_estimate, chunk.element_types,
///     );
/// }
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone)]
#[cfg_attr(feature = "semantic", derive(Serialize, Deserialize))]
pub struct RagChunk {
    /// Sequential index of this chunk in the document (0-based).
    pub chunk_index: usize,
    /// Chunk text content (elements joined by newlines).
    pub text: String,
    /// Text with heading context prepended — use this for embedding generation.
    pub full_text: String,
    /// Page numbers where this chunk's elements appear (deduplicated, sorted numerically).
    pub page_numbers: Vec<u32>,
    /// Bounding boxes of each element in the chunk.
    pub bounding_boxes: Vec<ElementBBox>,
    /// Type names of each element (e.g. "title", "paragraph", "table").
    pub element_types: Vec<String>,
    /// Heading context inherited from the nearest parent heading.
    pub heading_context: Option<String>,
    /// Approximate token count (word-count proxy).
    ///
    /// Computed as the number of whitespace-separated words. LLM subword
    /// tokenizers (BPE/WordPiece) typically produce 1.3–1.7x more tokens
    /// than the raw word count. Size your chunk budget accordingly: a
    /// `max_tokens: 512` setting corresponds to roughly 300–390 actual
    /// subword tokens for typical English prose.
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
        let element_types: Vec<String> = elements
            .iter()
            .map(|e| element_type_name(e).to_string())
            .collect();

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

/// Collect unique page numbers from elements, deduplicated and sorted numerically.
fn collect_pages(elements: &[Element]) -> Vec<u32> {
    let mut seen = HashSet::new();
    let mut pages = Vec::new();
    for e in elements {
        let p = e.page();
        if seen.insert(p) {
            pages.push(p);
        }
    }
    pages.sort_unstable();
    pages
}

/// Map an `Element` variant to its snake_case type name.
fn element_type_name(element: &Element) -> &'static str {
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
}
