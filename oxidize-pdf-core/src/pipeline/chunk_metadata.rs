//! Chunk-level metadata for RAG output.
//!
//! Surfaces data already computed by the partitioner (heading hierarchy, font,
//! style, confidence) plus new retrieval signals (content-type flags, counts,
//! stable IDs, language) and optional source-document metadata.

#[cfg(feature = "semantic")]
use serde::{Deserialize, Serialize};
#[cfg(feature = "semantic")]
use std::collections::BTreeMap;

use crate::pipeline::element::{Element, ElementBBox};
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

impl DocumentSource {
    /// Construct a source from the two caller-supplied fields (`filename`,
    /// `doc_hash`); the rest (`title`/`author`/`creation_date`/`total_pages`)
    /// are left `None` for [`rag_chunks_with_source`](crate::parser::PdfDocument::rag_chunks_with_source)
    /// to auto-fill from the info dictionary. Provided because `DocumentSource`
    /// is `#[non_exhaustive]`, so external callers cannot use a struct literal.
    pub fn with_file(filename: Option<String>, doc_hash: Option<String>) -> Self {
        Self {
            filename,
            doc_hash,
            ..Default::default()
        }
    }
}

/// Citation anchor for a chunk on a single page: the axis-aligned union of all
/// the chunk's element bounding boxes that fall on that page. Lets a RAG
/// consumer cite back to an exact region of the source PDF.
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "semantic", derive(Serialize, Deserialize))]
#[non_exhaustive]
pub struct PageRegion {
    /// Page the region is on (as stored on the elements).
    pub page: u32,
    /// Union bounding box of the chunk's elements on this page.
    pub bbox: ElementBBox,
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
    /// `language-detection` feature is off or detection is inconclusive.
    pub language: Option<String>,
    /// Detection confidence in `(0, 1]` for [`language`](Self::language);
    /// `None` when no language was detected.
    pub language_confidence: Option<f32>,
    /// Whether `whatlang` considered the [`language`](Self::language)
    /// detection reliable. Consumers should gate language-based routing on
    /// this; `None` when no language was detected.
    pub language_reliable: Option<bool>,
    /// Deterministic, stable identifier for this chunk.
    pub chunk_id: String,
    /// Identifier of the previous chunk in the document, if any.
    pub prev_chunk_id: Option<String>,
    /// Identifier of the next chunk in the document, if any.
    pub next_chunk_id: Option<String>,
    /// Source-document metadata, if available.
    pub source: Option<DocumentSource>,
    /// First and last page the chunk's elements touch (inclusive), or `None`
    /// when the chunk has no positioned elements.
    pub page_span: Option<(u32, u32)>,
    /// Per-page citation regions (union bbox of the chunk's elements on each
    /// page), sorted ascending by page. Empty when the chunk has no elements.
    pub page_regions: Vec<PageRegion>,
    /// Row count of the chunk's largest table (by row count), or `None` if the
    /// chunk has no table. Lets a consumer filter/route table-bearing chunks.
    pub table_rows: Option<usize>,
    /// Column count (widest row) of the same table reported by
    /// [`table_rows`](Self::table_rows); `None` when the chunk has no table.
    pub table_cols: Option<usize>,
    /// Open extension bag for provider-supplied fields (e.g. a closed analyzer
    /// stamping `legal.clause_number`). Namespacing keys by provider avoids
    /// collisions. Serializes nested under `"extra"`; omitted when empty.
    #[cfg(feature = "semantic")]
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub extra: BTreeMap<String, serde_json::Value>,
}

use sha2::{Digest, Sha256};

impl ChunkMetadata {
    /// Build chunk metadata from the chunk's elements and text. `full_text` is
    /// used for the content-hash id; `doc_hash` (when `Some`) overrides it.
    /// Language is detected here when the `language-detection` feature is on;
    /// prev/next links are filled by a later pass ([`link_chunks`]).
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
        let (page_span, page_regions) = page_anchor(elements);
        let (table_rows, table_cols) = table_dims(elements);
        // Detect once; fill code + confidence + reliability together.
        #[cfg(feature = "language-detection")]
        let (language, language_confidence, language_reliable) = match detect_language_full(text) {
            Some((code, conf, reliable)) => (Some(code), Some(conf), Some(reliable)),
            None => (None, None, None),
        };
        #[cfg(not(feature = "language-detection"))]
        let (language, language_confidence, language_reliable): (
            Option<String>,
            Option<f32>,
            Option<bool>,
        ) = (None, None, None);
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
            language,
            language_confidence,
            language_reliable,
            chunk_id: content_chunk_id(doc_hash, chunk_index, full_text),
            prev_chunk_id: None,
            next_chunk_id: None,
            source: None,
            page_span,
            page_regions,
            table_rows,
            table_cols,
            #[cfg(feature = "semantic")]
            extra: BTreeMap::new(),
        }
    }
}

/// Dimensions of the chunk's largest table (by row count): `(rows, widest row)`.
/// `(None, None)` when the chunk contains no table element.
fn table_dims(elements: &[Element]) -> (Option<usize>, Option<usize>) {
    elements
        .iter()
        .filter_map(|e| match e {
            Element::Table(t) => Some(&t.rows),
            _ => None,
        })
        .max_by_key(|rows| rows.len())
        .map(|rows| {
            let cols = rows.iter().map(|r| r.len()).max().unwrap_or(0);
            (Some(rows.len()), Some(cols))
        })
        .unwrap_or((None, None))
}

/// Union of two axis-aligned bounding boxes.
fn union_bbox(a: ElementBBox, b: ElementBBox) -> ElementBBox {
    let x = a.x.min(b.x);
    let y = a.y.min(b.y);
    let right = a.right().max(b.right());
    let top = a.top().max(b.top());
    ElementBBox::new(x, y, right - x, top - y)
}

/// Compute the chunk's citation anchor: `(page_span, page_regions)`. Groups the
/// elements by page, unions their bboxes per page, and sorts the regions by
/// page ascending. Returns `(None, vec![])` for an element-less chunk.
fn page_anchor(elements: &[Element]) -> (Option<(u32, u32)>, Vec<PageRegion>) {
    let mut by_page: Vec<(u32, ElementBBox)> = Vec::new();
    for e in elements {
        let page = e.metadata().page;
        let bbox = *e.bbox();
        match by_page.iter_mut().find(|(p, _)| *p == page) {
            Some(slot) => slot.1 = union_bbox(slot.1, bbox),
            None => by_page.push((page, bbox)),
        }
    }
    if by_page.is_empty() {
        return (None, Vec::new());
    }
    by_page.sort_by_key(|(p, _)| *p);
    let span = (by_page.first().unwrap().0, by_page.last().unwrap().0);
    let regions = by_page
        .into_iter()
        .map(|(page, bbox)| PageRegion { page, bbox })
        .collect();
    (Some(span), regions)
}

/// Fill `prev_chunk_id` / `next_chunk_id` on each chunk from its neighbours' ids.
pub(crate) fn link_chunks(chunks: &mut [crate::pipeline::RagChunk]) {
    let ids: Vec<String> = chunks.iter().map(|c| c.metadata.chunk_id.clone()).collect();
    for (i, c) in chunks.iter_mut().enumerate() {
        c.metadata.prev_chunk_id = if i > 0 {
            Some(ids[i - 1].clone())
        } else {
            None
        };
        c.metadata.next_chunk_id = ids.get(i + 1).cloned();
    }
}

/// Detect the dominant language of `text` as an ISO 639-3 code (e.g. `"eng"`,
/// `"spa"`), via `whatlang`. Returns `None` for empty/whitespace-only input or
/// when `whatlang` produces no detection. The detection is best-effort: on
/// short or ambiguous text the code may be unreliable.
///
/// Requires the `language-detection` feature.
#[cfg(feature = "language-detection")]
pub fn detect_language(text: &str) -> Option<String> {
    detect_language_full(text).map(|(code, _, _)| code)
}

/// Run `whatlang` once and return `(code, confidence, reliable)`; `None` for
/// empty/whitespace-only input or when no detection is produced. Single call
/// site so [`ChunkMetadata`] can fill code + confidence + reliability without
/// detecting twice.
#[cfg(feature = "language-detection")]
pub(crate) fn detect_language_full(text: &str) -> Option<(String, f32, bool)> {
    if text.trim().is_empty() {
        return None;
    }
    whatlang::detect(text).map(|info| {
        (
            info.lang().code().to_string(),
            info.confidence() as f32,
            info.is_reliable(),
        )
    })
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
        // Hashless prefix is exactly 8 bytes of SHA-256 → 16 hex chars; pin the
        // width so a change to the digest slice can't silently shrink the id.
        assert_eq!(
            a.split(':').next().unwrap().len(),
            16,
            "hashless chunk_id prefix must be 16 hex chars (8 bytes)"
        );

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
        assert_eq!(m.language_confidence, None);
        assert_eq!(m.language_reliable, None);
        assert_eq!(m.chunk_id, "");
        assert!(m.source.is_none());
        assert_eq!(m.page_span, None);
        assert!(m.page_regions.is_empty());
        assert_eq!(m.table_rows, None);
        assert_eq!(m.table_cols, None);
    }

    #[test]
    fn document_source_with_file_sets_only_supplied_fields() {
        let s = DocumentSource::with_file(Some("doc.pdf".to_string()), Some("h7".to_string()));
        assert_eq!(s.filename.as_deref(), Some("doc.pdf"));
        assert_eq!(s.doc_hash.as_deref(), Some("h7"));
        // Everything the caller did not supply stays None for the info-dict
        // auto-fill pass to populate.
        assert_eq!(s.title, None);
        assert_eq!(s.author, None);
        assert_eq!(s.creation_date, None);
        assert_eq!(s.total_pages, None);

        let empty = DocumentSource::with_file(None, None);
        assert_eq!(empty, DocumentSource::default());
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
        // Without the feature the field stays None; with it, detection runs on
        // the chunk text (the exact code is whatlang's call, not asserted here).
        #[cfg(not(feature = "language-detection"))]
        assert_eq!(m.language, None);
    }

    fn el_at(text: &str, page: u32, x: f64, y: f64, w: f64, h: f64) -> Element {
        Element::Paragraph(ElementData {
            text: text.to_string(),
            metadata: ElementMetadata {
                page,
                bbox: crate::pipeline::element::ElementBBox::new(x, y, w, h),
                ..ElementMetadata::default()
            },
        })
    }

    #[test]
    fn citation_anchor_page_span_and_per_page_union_bbox() {
        let els = vec![
            el_at("a", 1, 10.0, 700.0, 100.0, 20.0), // page1: x[10,110] y[700,720]
            el_at("b", 1, 50.0, 600.0, 200.0, 10.0), // page1: x[50,250] y[600,610]
            el_at("c", 2, 30.0, 500.0, 40.0, 40.0),  // page2: x[30,70]  y[500,540]
        ];
        let text = "a\nb\nc";
        let m = ChunkMetadata::from_elements(&els, text, text, 0, None);

        assert_eq!(m.page_span, Some((1, 2)));
        assert_eq!(m.page_regions.len(), 2);
        // Sorted ascending by page.
        assert_eq!(m.page_regions[0].page, 1);
        assert_eq!(m.page_regions[1].page, 2);

        // Page 1 region = union of its two element bboxes.
        let p1 = &m.page_regions[0].bbox;
        assert_eq!(p1.x, 10.0);
        assert_eq!(p1.y, 600.0);
        assert_eq!(p1.right(), 250.0);
        assert_eq!(p1.top(), 720.0);

        // Page 2 region = the single element's bbox.
        let p2 = &m.page_regions[1].bbox;
        assert_eq!(p2.x, 30.0);
        assert_eq!(p2.right(), 70.0);
        assert_eq!(p2.top(), 540.0);
    }

    #[test]
    fn citation_anchor_empty_for_no_elements() {
        let m = ChunkMetadata::from_elements(&[], "", "", 0, None);
        assert_eq!(m.page_span, None);
        assert!(m.page_regions.is_empty());
    }

    #[cfg(feature = "language-detection")]
    #[test]
    fn language_reliability_populated_alongside_code() {
        let els = vec![para("x", "F", 10.0, false, 1.0)];
        let text =
            "The annual report summarizes the financial performance of the company over the year.";
        let m = ChunkMetadata::from_elements(&els, text, text, 0, None);
        assert_eq!(m.language.as_deref(), Some("eng"));
        let conf = m
            .language_confidence
            .expect("confidence present when a language is detected");
        assert!(
            conf > 0.0 && conf <= 1.0,
            "confidence must be in (0, 1], got {conf}"
        );
        assert_eq!(
            m.language_reliable,
            Some(true),
            "a full English sentence must be a reliable detection"
        );
    }

    #[cfg(feature = "language-detection")]
    #[test]
    fn language_reliability_none_for_empty_text() {
        let m = ChunkMetadata::from_elements(&[], "", "", 0, None);
        assert_eq!(m.language, None);
        assert_eq!(m.language_confidence, None);
        assert_eq!(m.language_reliable, None);
    }

    fn table_with(rows: Vec<Vec<&str>>) -> Element {
        Element::Table(crate::pipeline::element::TableElementData {
            rows: rows
                .into_iter()
                .map(|r| r.into_iter().map(String::from).collect())
                .collect(),
            metadata: ElementMetadata::default(),
        })
    }

    #[test]
    fn table_dims_from_largest_table() {
        let small = table_with(vec![vec!["a", "b"]]); // 1 row x 2 cols
        let big = table_with(vec![vec!["a"], vec!["b"], vec!["c"]]); // 3 rows x 1 col
        let els = vec![para("x", "F", 10.0, false, 1.0), small, big];
        let text = "x";
        let m = ChunkMetadata::from_elements(&els, text, text, 0, None);
        // Largest by row count wins.
        assert_eq!(m.table_rows, Some(3));
        assert_eq!(m.table_cols, Some(1));
    }

    #[test]
    fn table_cols_uses_widest_row() {
        let ragged = table_with(vec![vec!["a", "b"], vec!["c", "d", "e", "f"]]);
        let m = ChunkMetadata::from_elements(&[ragged], "t", "t", 0, None);
        assert_eq!(m.table_rows, Some(2));
        assert_eq!(m.table_cols, Some(4));
    }

    #[test]
    fn table_dims_none_without_table() {
        let els = vec![para("just prose", "F", 10.0, false, 1.0)];
        let m = ChunkMetadata::from_elements(&els, "just prose", "just prose", 0, None);
        assert_eq!(m.table_rows, None);
        assert_eq!(m.table_cols, None);
    }

    #[cfg(feature = "semantic")]
    #[test]
    fn extra_bag_defaults_empty_and_roundtrips() {
        let mut m = ChunkMetadata::default();
        assert!(m.extra.is_empty(), "extra defaults to empty");

        // Empty extra is omitted from the serialized output.
        let json_empty = serde_json::to_string(&m).unwrap();
        assert!(
            !json_empty.contains("\"extra\""),
            "empty extra must be skipped in JSON"
        );

        // Populated extra survives a deterministic round-trip.
        m.extra
            .insert("legal.clause_number".to_string(), serde_json::json!("3.2"));
        m.extra.insert(
            "legal.defined_terms".to_string(),
            serde_json::json!(["Party", "Agreement"]),
        );
        let json = serde_json::to_string(&m).unwrap();
        assert!(json.contains("\"extra\""));
        let back: ChunkMetadata = serde_json::from_str(&json).unwrap();
        assert_eq!(back.extra, m.extra, "extra survives round-trip");
        assert_eq!(
            back.extra.get("legal.clause_number").unwrap(),
            &serde_json::json!("3.2")
        );
    }
}
