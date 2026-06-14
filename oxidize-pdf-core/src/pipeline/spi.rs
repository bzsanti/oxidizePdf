//! Unstable analysis SPI — extension points for the chunking pipeline.
//!
//! Behind the `unstable-spi` feature. The trait surface is exempt from semver
//! while experimental and may change until promoted.

use crate::pipeline::element::Element;

/// A grouping of elements destined to become one chunk. The chunking strategy
/// decides the boundaries; the pipeline owns everything downstream (RagChunk,
/// chunk_id, links, metadata).
#[derive(Debug, Clone)]
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

/// An open class label for an element, assigned by an [`ElementClassifier`].
///
/// The label is an opaque string the core only transports — domain semantics
/// (what `"clause"` or `"definition"` mean) live entirely in the provider.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClassLabel(pub std::borrow::Cow<'static, str>);

impl ClassLabel {
    /// Construct a label from any string-like value.
    pub fn new(label: impl Into<std::borrow::Cow<'static, str>>) -> Self {
        Self(label.into())
    }

    /// The label as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Read-only context handed to an [`ElementClassifier`]: the full element slice
/// and the index of the element being classified, so a classifier can inspect
/// neighbours (preceding/following elements) to make its decision.
pub struct ClassifyContext<'a> {
    /// All elements of the document, in order.
    pub elements: &'a [Element],
    /// Index of the element currently being classified.
    pub index: usize,
}

/// Assigns an open [`ClassLabel`] to an element before chunking. Implement this
/// in a (possibly closed) crate to refine the core's classification with
/// domain-specific classes. Runs once per element; the label is stored on
/// [`ElementMetadata::class_label`](crate::pipeline::ElementMetadata::class_label)
/// and may be read by a [`ChunkingStrategy`] to decide chunk boundaries.
pub trait ElementClassifier: Send + Sync {
    /// Return a label for `element`, or `None` to leave it unlabelled.
    fn classify(&self, element: &Element, ctx: &ClassifyContext) -> Option<ClassLabel>;
}

use crate::pipeline::hybrid_chunking::{HybridChunkConfig, HybridChunker};
use crate::pipeline::DocumentSource;

/// Configures the analysis pipeline: which chunking strategy to run, the token
/// budget used to flag oversized chunks, and optional source-document metadata.
pub struct AnalysisPipeline {
    pub(crate) chunking: Box<dyn ChunkingStrategy>,
    pub(crate) max_tokens: usize,
    pub(crate) source: Option<DocumentSource>,
    pub(crate) classifier: Option<Box<dyn ElementClassifier>>,
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
            classifier: None,
        }
    }

    /// Replace the chunking strategy.
    ///
    /// `max_tokens` is independent of the strategy: it stays at whatever
    /// [`new`](Self::new) or [`with_max_tokens`](Self::with_max_tokens) set it
    /// to (default 512) and is used only to flag oversized chunks. A custom
    /// strategy that chunks to a different budget should also call
    /// [`with_max_tokens`](Self::with_max_tokens) so the oversized flag matches.
    #[must_use]
    pub fn with_chunking(mut self, strategy: Box<dyn ChunkingStrategy>) -> Self {
        self.chunking = strategy;
        self
    }

    /// Set the token budget used to flag oversized chunks.
    #[must_use]
    pub fn with_max_tokens(mut self, max_tokens: usize) -> Self {
        self.max_tokens = max_tokens;
        self
    }

    /// Stamp source-document metadata onto every chunk.
    #[must_use]
    pub fn with_source(mut self, source: DocumentSource) -> Self {
        self.source = Some(source);
        self
    }

    /// Set a classifier that labels elements (writing
    /// [`ElementMetadata::class_label`](crate::pipeline::ElementMetadata::class_label))
    /// after partitioning and before chunking. The chunking strategy may then
    /// read the labels to decide boundaries.
    #[must_use]
    pub fn with_classifier(mut self, classifier: Box<dyn ElementClassifier>) -> Self {
        self.classifier = Some(classifier);
        self
    }
}
