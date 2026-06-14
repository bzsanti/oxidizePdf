//! Unstable analysis SPI — extension points for the chunking pipeline.
//!
//! Behind the `unstable-spi` feature. The trait surface is exempt from semver
//! while experimental and may change until promoted.

use crate::pipeline::element::Element;

/// A grouping of elements destined to become one chunk. The chunking strategy
/// decides the boundaries; the pipeline owns everything downstream (RagChunk,
/// chunk_id, links, metadata).
#[non_exhaustive]
pub struct ChunkGroup {
    /// The elements that form this chunk, in order.
    pub elements: Vec<Element>,
    /// Optional heading context to prepend for embedding.
    pub heading_context: Option<String>,
}

impl ChunkGroup {
    /// Construct a group from elements and an optional heading context.
    pub fn new(elements: Vec<Element>, heading_context: Option<String>) -> Self {
        Self {
            elements,
            heading_context,
        }
    }
}

/// Decides which elements group into a chunk. Implement this in a (possibly
/// closed) crate to override how the pipeline forms chunks. The pipeline
/// computes `oversized`, `chunk_id`, prev/next links, and `ChunkMetadata`.
pub trait ChunkingStrategy: Send + Sync {
    /// Group `elements` into chunks. Called once per document.
    fn chunk(&self, elements: &[Element]) -> Vec<ChunkGroup>;
}
