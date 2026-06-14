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

use crate::pipeline::hybrid_chunking::{HybridChunkConfig, HybridChunker};
use crate::pipeline::DocumentSource;

/// Configures the analysis pipeline: which chunking strategy to run, the token
/// budget used to flag oversized chunks, and optional source-document metadata.
pub struct AnalysisPipeline {
    pub(crate) chunking: Box<dyn ChunkingStrategy>,
    pub(crate) max_tokens: usize,
    pub(crate) source: Option<DocumentSource>,
}

impl Default for AnalysisPipeline {
    fn default() -> Self {
        Self::new()
    }
}

impl AnalysisPipeline {
    /// Default pipeline: the built-in `HybridChunker`, default token budget, no
    /// source. Reproduces `PdfDocument::rag_chunks()` exactly.
    pub fn new() -> Self {
        let config = HybridChunkConfig::default();
        Self {
            max_tokens: config.max_tokens,
            chunking: Box::new(HybridChunker::new(config)),
            source: None,
        }
    }

    /// Replace the chunking strategy.
    pub fn with_chunking(mut self, strategy: Box<dyn ChunkingStrategy>) -> Self {
        self.chunking = strategy;
        self
    }

    /// Set the token budget used to flag oversized chunks.
    pub fn with_max_tokens(mut self, max_tokens: usize) -> Self {
        self.max_tokens = max_tokens;
        self
    }

    /// Stamp source-document metadata onto every chunk.
    pub fn with_source(mut self, source: DocumentSource) -> Self {
        self.source = Some(source);
        self
    }
}
