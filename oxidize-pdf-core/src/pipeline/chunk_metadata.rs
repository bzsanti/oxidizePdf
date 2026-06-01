//! Chunk-level metadata for RAG output.
//!
//! Surfaces data already computed by the partitioner (heading hierarchy, font,
//! style, confidence) plus new retrieval signals (content-type flags, counts,
//! stable IDs, language) and optional source-document metadata.

#[cfg(feature = "semantic")]
use serde::{Deserialize, Serialize};

use crate::pipeline::element::Element;
use crate::pipeline::hybrid_chunking::split_into_sentences;

/// Char-weighted aggregates over a chunk's elements.
pub(crate) struct Aggregates {
    pub dominant_font: Option<String>,
    pub dominant_font_size: Option<f64>,
    pub is_bold: bool,
    pub is_italic: bool,
    pub min_confidence: f32,
}

impl Aggregates {
    pub(crate) fn from_elements(elements: &[Element]) -> Self {
        let mut font_weight: Vec<(String, usize)> = Vec::new();
        let mut size_weight: Vec<(f64, usize)> = Vec::new();
        let mut bold_chars = 0usize;
        let mut italic_chars = 0usize;
        let mut total_chars = 0usize;
        let mut min_conf = 1.0f32;

        for e in elements {
            let w = e.text().chars().count();
            total_chars += w;
            let meta = e.metadata();
            if let Some(f) = &meta.font_name {
                match font_weight.iter_mut().find(|(name, _)| name == f) {
                    Some((_, c)) => *c += w,
                    None => font_weight.push((f.clone(), w)),
                }
            }
            if let Some(s) = meta.font_size {
                match size_weight.iter_mut().find(|(sz, _)| (*sz - s).abs() < 0.1) {
                    Some((_, c)) => *c += w,
                    None => size_weight.push((s, w)),
                }
            }
            if meta.is_bold {
                bold_chars += w;
            }
            if meta.is_italic {
                italic_chars += w;
            }
            min_conf = min_conf.min(meta.confidence as f32);
        }

        let dominant_font = font_weight
            .into_iter()
            .max_by_key(|(_, c)| *c)
            .map(|(name, _)| name);
        let dominant_font_size = size_weight
            .into_iter()
            .max_by_key(|(_, c)| *c)
            .map(|(sz, _)| sz);

        Self {
            dominant_font,
            dominant_font_size,
            is_bold: total_chars > 0 && bold_chars * 2 > total_chars,
            is_italic: total_chars > 0 && italic_chars * 2 > total_chars,
            min_confidence: if elements.is_empty() { 0.0 } else { min_conf },
        }
    }
}

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

use sha2::{Digest, Sha256};

#[allow(dead_code)] // wired into RagChunk::from_hybrid_chunk (Task 7)
impl ChunkMetadata {
    /// Build chunk metadata from the chunk's elements and text. `full_text` is
    /// used for the content-hash id; `doc_hash` (when `Some`) overrides it.
    /// Language and prev/next links are filled by later passes.
    pub(crate) fn from_elements(
        elements: &[Element],
        text: &str,
        full_text: &str,
        chunk_index: usize,
        doc_hash: Option<&str>,
    ) -> Self {
        let agg = Aggregates::from_elements(elements);
        let heading_path = elements
            .first()
            .map(|e| e.metadata().heading_path.clone())
            .unwrap_or_default();
        ChunkMetadata {
            heading_path,
            dominant_font: agg.dominant_font,
            dominant_font_size: agg.dominant_font_size,
            is_bold: agg.is_bold,
            is_italic: agg.is_italic,
            min_confidence: agg.min_confidence,
            content_types: content_type_flags(elements),
            char_count: char_count(text),
            word_count: word_count(text),
            sentence_count: sentence_count(text),
            language: None,
            chunk_id: content_chunk_id(doc_hash, chunk_index, full_text),
            prev_chunk_id: None,
            next_chunk_id: None,
            source: None,
        }
    }
}

/// Deterministic chunk id: `<doc_id>:<index>` where `doc_id` is the supplied
/// `doc_hash` or, absent that, the first 8 bytes of SHA-256(full_text) in hex.
pub(crate) fn content_chunk_id(doc_hash: Option<&str>, index: usize, full_text: &str) -> String {
    let doc_id = match doc_hash {
        Some(h) => h.to_string(),
        None => {
            let mut hasher = Sha256::new();
            hasher.update(full_text.as_bytes());
            let digest = hasher.finalize();
            digest[..8]
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect::<String>()
        }
    };
    format!("{doc_id}:{index}")
}

pub(crate) fn content_type_flags(elements: &[Element]) -> ContentTypeFlags {
    let mut flags = ContentTypeFlags::default();
    let mut all_titles = !elements.is_empty();
    for e in elements {
        match e {
            Element::Table(_) => flags.has_table = true,
            Element::ListItem(_) => flags.has_list = true,
            Element::CodeBlock(_) => flags.has_code = true,
            _ => {}
        }
        if !matches!(e, Element::Title(_)) {
            all_titles = false;
        }
    }
    flags.heading_only = all_titles;
    flags
}

pub(crate) fn char_count(text: &str) -> usize {
    text.chars().count()
}

pub(crate) fn word_count(text: &str) -> usize {
    text.split_whitespace().count()
}

pub(crate) fn sentence_count(text: &str) -> usize {
    if text.trim().is_empty() {
        return 0;
    }
    split_into_sentences(text).len()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pipeline::element::{Element, ElementData, ElementMetadata};

    fn table_el() -> Element {
        Element::Table(crate::pipeline::element::TableElementData {
            rows: vec![],
            metadata: crate::pipeline::element::ElementMetadata::default(),
        })
    }

    #[test]
    fn content_types_and_counts() {
        let els = vec![
            para("Hello world. Second sentence!", "F", 10.0, false, 1.0),
            table_el(),
        ];
        let flags = content_type_flags(&els);
        assert!(flags.has_table);
        assert!(!flags.has_list);
        assert!(!flags.heading_only);

        let text = "Hello world. Second sentence!";
        assert_eq!(char_count(text), text.chars().count());
        assert_eq!(word_count(text), 4);
        assert_eq!(sentence_count(text), 2);
    }

    #[test]
    fn heading_only_when_all_titles() {
        let d = crate::pipeline::element::ElementData {
            text: "Title".to_string(),
            metadata: crate::pipeline::element::ElementMetadata::default(),
        };
        let els = vec![Element::Title(d)];
        assert!(content_type_flags(&els).heading_only);
    }

    fn para(text: &str, font: &str, size: f64, bold: bool, conf: f64) -> Element {
        let metadata = ElementMetadata {
            font_name: Some(font.to_string()),
            font_size: Some(size),
            is_bold: bold,
            confidence: conf,
            ..ElementMetadata::default()
        };
        Element::Paragraph(ElementData {
            text: text.to_string(),
            metadata,
        })
    }

    #[test]
    fn aggregate_picks_char_weighted_dominant_font_and_min_confidence() {
        // "aaaa" (4 chars) Helvetica bold conf=0.9 ; "bb" (2) Times conf=0.5
        let els = vec![
            para("aaaa", "Helvetica", 12.0, true, 0.9),
            para("bb", "Times", 10.0, false, 0.5),
        ];
        let agg = Aggregates::from_elements(&els);
        assert_eq!(agg.dominant_font.as_deref(), Some("Helvetica"));
        assert_eq!(agg.dominant_font_size, Some(12.0));
        assert!(agg.is_bold, "4 bold chars vs 2 non-bold → bold majority");
        assert!((agg.min_confidence - 0.5).abs() < 1e-6);
    }

    #[test]
    fn chunk_id_is_deterministic_and_prefixed() {
        let a = content_chunk_id(None, 0, "the quick brown fox");
        let b = content_chunk_id(None, 0, "the quick brown fox");
        assert_eq!(a, b, "same text + index → same id");
        assert!(a.ends_with(":0"));

        let with_hash = content_chunk_id(Some("dochash123"), 7, "ignored when hash present");
        assert_eq!(with_hash, "dochash123:7");

        let other = content_chunk_id(None, 0, "different text");
        assert_ne!(a, other);
    }

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

    #[test]
    fn build_metadata_from_chunk_elements() {
        let els = vec![
            para("aaaa", "Helvetica", 12.0, true, 0.8),
            para("bb. cc.", "Helvetica", 12.0, false, 0.6),
        ];
        let text = "aaaa\nbb. cc.";
        let m = ChunkMetadata::from_elements(&els, text, text, 3, None);
        assert_eq!(m.dominant_font.as_deref(), Some("Helvetica"));
        assert!((m.min_confidence - 0.6).abs() < 1e-6);
        assert_eq!(m.char_count, text.chars().count());
        assert_eq!(m.chunk_id, content_chunk_id(None, 3, text));
        assert!(m.source.is_none());
        assert_eq!(m.language, None); // language filled separately in a later task
    }
}
