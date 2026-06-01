//! Chunk-level metadata for RAG output.
//!
//! Surfaces data already computed by the partitioner (heading hierarchy, font,
//! style, confidence) plus new retrieval signals (content-type flags, counts,
//! stable IDs, language) and optional source-document metadata.

#[cfg(feature = "semantic")]
use serde::{Deserialize, Serialize};

/// Boolean flags describing the kinds of content present in a chunk.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[cfg_attr(feature = "semantic", derive(Serialize, Deserialize))]
pub struct ContentTypeFlags {
    /// The chunk contains at least one table element.
    pub has_table: bool,
    /// The chunk contains at least one list item.
    pub has_list: bool,
    /// The chunk contains at least one code block.
    pub has_code: bool,
    /// The chunk is composed solely of heading (title) elements.
    pub heading_only: bool,
}

/// Metadata about the source document a chunk came from.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[cfg_attr(feature = "semantic", derive(Serialize, Deserialize))]
#[non_exhaustive]
pub struct DocumentSource {
    /// Document title from the info dictionary, if present.
    pub title: Option<String>,
    /// Document author from the info dictionary, if present.
    pub author: Option<String>,
    /// Creation date string from the info dictionary, if present.
    pub creation_date: Option<String>,
    /// Originating file name (caller-supplied — the pipeline does not know it).
    pub filename: Option<String>,
    /// Stable document hash (caller-supplied; used as the chunk_id prefix).
    pub doc_hash: Option<String>,
    /// Total page count of the source document.
    pub total_pages: Option<u32>,
}

/// Per-chunk metadata attached to every [`RagChunk`](crate::pipeline::RagChunk).
#[derive(Debug, Clone, Default, PartialEq)]
#[cfg_attr(feature = "semantic", derive(Serialize, Deserialize))]
#[non_exhaustive]
pub struct ChunkMetadata {
    /// Full section breadcrumb, root→leaf (e.g. `["1 Intro", "1.2 Scope"]`).
    pub heading_path: Vec<String>,
    /// Dominant font (char-weighted majority across the chunk's elements).
    pub dominant_font: Option<String>,
    /// Dominant font size (char-weighted majority).
    pub dominant_font_size: Option<f64>,
    /// True if the majority of characters are bold.
    pub is_bold: bool,
    /// True if the majority of characters are italic.
    pub is_italic: bool,
    /// Lowest classification confidence among the chunk's elements.
    pub min_confidence: f32,
    /// Content-type flags derived from element types.
    pub content_types: ContentTypeFlags,
    /// Character count of the chunk text.
    pub char_count: usize,
    /// Whitespace-separated word count.
    pub word_count: usize,
    /// Sentence count (uses the chunker's sentence splitter).
    pub sentence_count: usize,
    /// Detected language code (ISO 639-3, via `whatlang`); `None` if the
    /// `lang-detect` feature is off or detection is inconclusive.
    pub language: Option<String>,
    /// Deterministic, stable identifier for this chunk.
    pub chunk_id: String,
    /// Identifier of the previous chunk in the document, if any.
    pub prev_chunk_id: Option<String>,
    /// Identifier of the next chunk in the document, if any.
    pub next_chunk_id: Option<String>,
    /// Source-document metadata, if available.
    pub source: Option<DocumentSource>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chunk_metadata_default_is_empty() {
        let m = ChunkMetadata::default();
        assert!(m.heading_path.is_empty());
        assert_eq!(m.dominant_font, None);
        assert!(!m.is_bold);
        assert_eq!(m.min_confidence, 0.0);
        assert!(!m.content_types.has_table);
        assert_eq!(m.char_count, 0);
        assert_eq!(m.language, None);
        assert_eq!(m.chunk_id, "");
        assert!(m.source.is_none());
    }
}
