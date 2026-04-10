//! Source highlighter for RAG-aligned PDF editing
//!
//! Provides `TextPositionIndex` to map character offsets from `DocumentChunk`
//! to PDF coordinates, and `SourceHighlighter` to highlight retrieved chunks
//! in the original PDF.

use std::collections::HashMap;
use std::io::Cursor;

use crate::ai::chunking::DocumentChunk;
use crate::annotations::MarkupAnnotation;
use crate::geometry::{Point, Rectangle};
use crate::graphics::Color;
use crate::text::extraction::{ExtractedText, ExtractionOptions, TextFragment};

/// PAGE_SEPARATOR matches the chunker's concatenation: pages joined with "\n\n"
const PAGE_SEPARATOR: &str = "\n\n";

/// Entry in the position index mapping a text fragment to its char offset
/// in the full concatenated document text.
#[derive(Debug, Clone)]
pub struct IndexedFragment {
    /// 0-indexed page number
    pub page: usize,
    /// Character offset where this fragment starts in the full document text
    pub start_char: usize,
    /// Character offset where this fragment ends (exclusive) in the full document text
    pub end_char: usize,
    /// X coordinate in PDF page coordinates
    pub x: f64,
    /// Y coordinate in PDF page coordinates
    pub y: f64,
    /// Width of the text fragment
    pub width: f64,
    /// Height of the text fragment
    pub height: f64,
}

impl IndexedFragment {
    /// Convert this fragment's position to a `Rectangle` suitable for annotations.
    pub fn to_rectangle(&self) -> Rectangle {
        Rectangle::from_position_and_size(self.x, self.y, self.width, self.height)
    }
}

/// Maps character offsets in concatenated document text to PDF coordinates.
///
/// Built from per-page `ExtractedText` (with `preserve_layout: true`),
/// this index allows mapping a character range (like those in `DocumentChunk`)
/// back to the physical locations on PDF pages.
#[derive(Debug)]
pub struct TextPositionIndex {
    /// Indexed fragments sorted by start_char
    entries: Vec<IndexedFragment>,
    /// Character offset where each page starts in the concatenated text
    page_offsets: Vec<usize>,
}

impl TextPositionIndex {
    /// Build an index from per-page extracted text.
    ///
    /// Pages are concatenated with `"\n\n"` separators (matching the chunker).
    /// For each page's `TextFragment`, we find its position in the page text
    /// and compute the global character offset.
    pub fn build(pages: &[ExtractedText]) -> Self {
        let mut entries = Vec::new();
        let mut page_offsets = Vec::new();
        let mut global_offset: usize = 0;

        for (page_idx, page) in pages.iter().enumerate() {
            page_offsets.push(global_offset);

            // Track position within the page text for incremental search
            let page_text = &page.text;
            let mut search_from: usize = 0;

            for fragment in &page.fragments {
                if fragment.text.is_empty() {
                    continue;
                }

                // Find this fragment's text within the page text, starting from
                // where the last fragment ended (incremental search)
                if let Some(pos_in_page) = page_text[search_from..].find(&fragment.text) {
                    let local_offset = search_from + pos_in_page;
                    let frag_len = fragment.text.len();

                    entries.push(IndexedFragment {
                        page: page_idx,
                        start_char: global_offset + local_offset,
                        end_char: global_offset + local_offset + frag_len,
                        x: fragment.x,
                        y: fragment.y,
                        width: fragment.width,
                        height: fragment.height,
                    });

                    // Advance search position past this fragment
                    search_from = local_offset + frag_len;
                }
            }

            // Advance global offset: page text length + separator
            global_offset += page_text.len();
            if page_idx < pages.len() - 1 {
                global_offset += PAGE_SEPARATOR.len();
            }
        }

        Self {
            entries,
            page_offsets,
        }
    }

    /// Find all fragments whose character range overlaps with `[start, end)`.
    pub fn fragments_for_range(&self, start: usize, end: usize) -> Vec<&IndexedFragment> {
        if start >= end {
            return Vec::new();
        }

        self.entries
            .iter()
            .filter(|e| e.start_char < end && e.end_char > start)
            .collect()
    }

    /// Get the character offset where a given page starts.
    pub fn page_offset(&self, page: usize) -> Option<usize> {
        self.page_offsets.get(page).copied()
    }

    /// Total number of indexed fragments.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the index is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// All indexed entries (for inspection/testing).
    pub fn entries(&self) -> &[IndexedFragment] {
        &self.entries
    }
}

/// Convert a `TextFragment` position to a `Rectangle` for highlight annotations.
pub fn fragment_to_highlight_rect(frag: &TextFragment) -> Rectangle {
    Rectangle::new(
        Point::new(frag.x, frag.y),
        Point::new(frag.x + frag.width, frag.y + frag.height),
    )
}

// =============================================================================
// SourceHighlighter API
// =============================================================================

/// Style configuration for highlight annotations.
#[derive(Debug, Clone)]
pub struct HighlightStyle {
    /// Color of the highlight (default: yellow)
    pub color: Color,
    /// Opacity of the highlight (0.0 = transparent, 1.0 = opaque; default: 0.5)
    pub opacity: f64,
}

impl Default for HighlightStyle {
    fn default() -> Self {
        Self {
            color: Color::Rgb(1.0, 1.0, 0.0), // Yellow
            opacity: 0.5,
        }
    }
}

impl HighlightStyle {
    /// Create a new HighlightStyle with default values.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the highlight color.
    pub fn with_color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }

    /// Set the highlight opacity.
    pub fn with_opacity(mut self, opacity: f64) -> Self {
        self.opacity = opacity;
        self
    }
}

/// Errors that can occur during source highlighting.
#[derive(Debug, thiserror::Error)]
pub enum SourceHighlighterError {
    /// Failed to extract text from the PDF
    #[error("text extraction failed: {0}")]
    TextExtractionFailed(String),

    /// Failed to reconstruct a page from the parsed PDF
    #[error("page reconstruction failed: {0}")]
    PageReconstructionFailed(String),

    /// Failed to write the output PDF
    #[error("write failed: {0}")]
    WriteFailed(String),
}

/// Result type for source highlighter operations.
pub type SourceHighlighterResult<T> = Result<T, SourceHighlighterError>;

/// Highlights text regions in a PDF corresponding to retrieved RAG chunks.
///
/// Given PDF bytes and a set of `DocumentChunk`s (from the chunker), this
/// produces a new PDF with highlight annotations over the text regions
/// that correspond to each chunk.
pub struct SourceHighlighter;

impl SourceHighlighter {
    /// Highlight the given chunks in the PDF, returning the modified PDF bytes.
    ///
    /// # Arguments
    ///
    /// * `pdf_bytes` - The original PDF file bytes
    /// * `chunks` - Chunks to highlight (with position metadata from the chunker)
    /// * `style` - Visual style for the highlights
    ///
    /// # Returns
    ///
    /// The modified PDF bytes with highlight annotations added.
    pub fn highlight_chunks(
        pdf_bytes: &[u8],
        chunks: &[&DocumentChunk],
        style: HighlightStyle,
    ) -> SourceHighlighterResult<Vec<u8>> {
        if chunks.is_empty() {
            return Ok(pdf_bytes.to_vec());
        }

        // 1. Parse the PDF
        let cursor = Cursor::new(pdf_bytes);
        let reader = crate::parser::PdfReader::new(cursor)
            .map_err(|e| SourceHighlighterError::TextExtractionFailed(e.to_string()))?;
        let document = reader.into_document();

        // 2. Extract text with position information
        let options = ExtractionOptions {
            preserve_layout: true,
            ..Default::default()
        };
        let extracted_pages = document
            .extract_text_with_options(options)
            .map_err(|e| SourceHighlighterError::TextExtractionFailed(e.to_string()))?;

        // 3. Build the position index
        let index = TextPositionIndex::build(&extracted_pages);

        // 4. For each chunk, find matching fragments and group by page
        let mut annotations_by_page: HashMap<usize, Vec<Rectangle>> = HashMap::new();

        for chunk in chunks {
            let start = chunk.metadata.position.start_char;
            let end = chunk.metadata.position.end_char;

            for frag in index.fragments_for_range(start, end) {
                annotations_by_page
                    .entry(frag.page)
                    .or_default()
                    .push(frag.to_rectangle());
            }
        }

        // 5. Reconstruct the document with annotations
        let page_count = document
            .page_count()
            .map_err(|e| SourceHighlighterError::PageReconstructionFailed(e.to_string()))?;

        let mut output_doc = crate::document::Document::new();

        for page_idx in 0..page_count {
            let parsed_page = document
                .get_page(page_idx)
                .map_err(|e| SourceHighlighterError::PageReconstructionFailed(e.to_string()))?;

            let mut page = crate::page::Page::from_parsed_with_content(&parsed_page, &document)
                .map_err(|e| SourceHighlighterError::PageReconstructionFailed(e.to_string()))?;

            // Add highlight annotations for this page
            if let Some(rects) = annotations_by_page.get(&(page_idx as usize)) {
                for rect in rects {
                    let highlight =
                        MarkupAnnotation::highlight(*rect).with_color(style.color.clone());
                    page.add_annotation(highlight.to_annotation());
                }
            }

            output_doc.add_page(page);
        }

        // 6. Write to bytes
        output_doc
            .to_bytes()
            .map_err(|e| SourceHighlighterError::WriteFailed(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: create a TextFragment with given text and position
    fn make_fragment(text: &str, x: f64, y: f64, width: f64, height: f64) -> TextFragment {
        TextFragment {
            text: text.to_string(),
            x,
            y,
            width,
            height,
            font_size: 12.0,
            font_name: None,
            is_bold: false,
            is_italic: false,
            color: None,
            space_decisions: Vec::new(),
        }
    }

    /// Helper: create ExtractedText from fragments, building text by joining fragment texts
    fn make_extracted(fragments: Vec<TextFragment>) -> ExtractedText {
        let text = fragments
            .iter()
            .map(|f| f.text.as_str())
            .collect::<Vec<_>>()
            .join(" ");
        ExtractedText { text, fragments }
    }

    #[test]
    fn test_index_single_fragment() {
        let page = ExtractedText {
            text: "Hello".to_string(),
            fragments: vec![make_fragment("Hello", 100.0, 700.0, 50.0, 12.0)],
        };
        let index = TextPositionIndex::build(&[page]);

        assert_eq!(index.len(), 1);
        let results = index.fragments_for_range(0, 5);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].start_char, 0);
        assert_eq!(results[0].end_char, 5);
        assert!((results[0].x - 100.0).abs() < 0.01);
    }

    #[test]
    fn test_index_multiple_fragments() {
        let page = ExtractedText {
            text: "Hello World Test".to_string(),
            fragments: vec![
                make_fragment("Hello", 100.0, 700.0, 50.0, 12.0),
                make_fragment("World", 160.0, 700.0, 55.0, 12.0),
                make_fragment("Test", 225.0, 700.0, 40.0, 12.0),
            ],
        };
        let index = TextPositionIndex::build(&[page]);

        assert_eq!(index.len(), 3);

        // Query that overlaps only "World" (chars 6-11)
        let results = index.fragments_for_range(6, 11);
        assert_eq!(results.len(), 1);
        assert!((results[0].x - 160.0).abs() < 0.01);
    }

    #[test]
    fn test_index_cross_page() {
        let page1 = ExtractedText {
            text: "Page one".to_string(),
            fragments: vec![make_fragment("Page one", 72.0, 700.0, 80.0, 12.0)],
        };
        let page2 = ExtractedText {
            text: "Page two".to_string(),
            fragments: vec![make_fragment("Page two", 72.0, 700.0, 80.0, 12.0)],
        };
        let index = TextPositionIndex::build(&[page1, page2]);

        assert_eq!(index.len(), 2);

        // Page 1: "Page one" (0..8), separator "\n\n" (8..10), Page 2: "Page two" (10..18)
        // Query that spans both pages
        let results = index.fragments_for_range(5, 15);
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].page, 0);
        assert_eq!(results[1].page, 1);
    }

    #[test]
    fn test_index_empty_range() {
        let page = ExtractedText {
            text: "Hello".to_string(),
            fragments: vec![make_fragment("Hello", 100.0, 700.0, 50.0, 12.0)],
        };
        let index = TextPositionIndex::build(&[page]);

        let results = index.fragments_for_range(2, 2);
        assert!(results.is_empty(), "Empty range should return no results");
    }

    #[test]
    fn test_index_exact_boundary() {
        let page = ExtractedText {
            text: "Hello".to_string(),
            fragments: vec![make_fragment("Hello", 100.0, 700.0, 50.0, 12.0)],
        };
        let index = TextPositionIndex::build(&[page]);

        // Exact boundaries [0, 5) should include the fragment
        let results = index.fragments_for_range(0, 5);
        assert_eq!(results.len(), 1);

        // Range [5, 10) should NOT include it (fragment ends at 5)
        let results = index.fragments_for_range(5, 10);
        assert!(results.is_empty());
    }

    #[test]
    fn test_index_no_overlap() {
        let page = ExtractedText {
            text: "Hello".to_string(),
            fragments: vec![make_fragment("Hello", 100.0, 700.0, 50.0, 12.0)],
        };
        let index = TextPositionIndex::build(&[page]);

        let results = index.fragments_for_range(100, 200);
        assert!(
            results.is_empty(),
            "Query far beyond text should return nothing"
        );
    }

    #[test]
    fn test_fragment_to_highlight_rect_conversion() {
        let frag = make_fragment("Test", 100.0, 500.0, 200.0, 15.0);
        let rect = fragment_to_highlight_rect(&frag);

        assert!((rect.lower_left.x - 100.0).abs() < 0.01);
        assert!((rect.lower_left.y - 500.0).abs() < 0.01);
        assert!((rect.upper_right.x - 300.0).abs() < 0.01);
        assert!((rect.upper_right.y - 515.0).abs() < 0.01);
    }

    #[test]
    fn test_index_build_from_extracted() {
        let fragments = vec![
            make_fragment("Alpha", 72.0, 750.0, 45.0, 12.0),
            make_fragment("Beta", 130.0, 750.0, 35.0, 12.0),
            make_fragment("Gamma", 180.0, 750.0, 50.0, 12.0),
        ];
        let page = make_extracted(fragments);
        let index = TextPositionIndex::build(&[page]);

        assert_eq!(index.len(), 3);
        // All entries should have page 0
        for entry in index.entries() {
            assert_eq!(entry.page, 0);
        }
    }

    #[test]
    fn test_index_fragments_grouped_by_page() {
        let page1 = ExtractedText {
            text: "AAA".to_string(),
            fragments: vec![make_fragment("AAA", 72.0, 700.0, 30.0, 12.0)],
        };
        let page2 = ExtractedText {
            text: "BBB CCC".to_string(),
            fragments: vec![
                make_fragment("BBB", 72.0, 700.0, 30.0, 12.0),
                make_fragment("CCC", 110.0, 700.0, 30.0, 12.0),
            ],
        };
        let index = TextPositionIndex::build(&[page1, page2]);

        // Query all
        let all = index.fragments_for_range(0, 100);
        assert_eq!(all.len(), 3);

        let page0_frags: Vec<_> = all.iter().filter(|f| f.page == 0).collect();
        let page1_frags: Vec<_> = all.iter().filter(|f| f.page == 1).collect();
        assert_eq!(page0_frags.len(), 1);
        assert_eq!(page1_frags.len(), 2);
    }

    #[test]
    fn test_index_page_offsets() {
        let page1 = ExtractedText {
            text: "ABCDE".to_string(), // 5 chars
            fragments: vec![make_fragment("ABCDE", 72.0, 700.0, 50.0, 12.0)],
        };
        let page2 = ExtractedText {
            text: "FGHIJ".to_string(), // 5 chars
            fragments: vec![make_fragment("FGHIJ", 72.0, 700.0, 50.0, 12.0)],
        };
        let page3 = ExtractedText {
            text: "KLMNO".to_string(), // 5 chars
            fragments: vec![make_fragment("KLMNO", 72.0, 700.0, 50.0, 12.0)],
        };
        let index = TextPositionIndex::build(&[page1, page2, page3]);

        // Page 0 starts at 0
        assert_eq!(index.page_offset(0), Some(0));
        // Page 1 starts at 5 + 2 ("\n\n") = 7
        assert_eq!(index.page_offset(1), Some(7));
        // Page 2 starts at 7 + 5 + 2 = 14
        assert_eq!(index.page_offset(2), Some(14));
        // Page 3 doesn't exist
        assert_eq!(index.page_offset(3), None);
    }

    #[test]
    fn test_index_whitespace_only_fragments() {
        let page = ExtractedText {
            text: "Hello World".to_string(),
            fragments: vec![
                make_fragment("Hello", 72.0, 700.0, 50.0, 12.0),
                make_fragment("", 130.0, 700.0, 10.0, 12.0), // empty fragment
                make_fragment("World", 145.0, 700.0, 55.0, 12.0),
            ],
        };
        let index = TextPositionIndex::build(&[page]);

        // Empty fragment should be skipped
        assert_eq!(index.len(), 2);
        let results = index.fragments_for_range(0, 20);
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_index_preserves_order() {
        let page = ExtractedText {
            text: "AAA BBB CCC DDD".to_string(),
            fragments: vec![
                make_fragment("AAA", 72.0, 700.0, 30.0, 12.0),
                make_fragment("BBB", 110.0, 700.0, 30.0, 12.0),
                make_fragment("CCC", 150.0, 700.0, 30.0, 12.0),
                make_fragment("DDD", 190.0, 700.0, 30.0, 12.0),
            ],
        };
        let index = TextPositionIndex::build(&[page]);

        let entries = index.entries();
        for i in 1..entries.len() {
            assert!(
                entries[i].start_char >= entries[i - 1].start_char,
                "Entries should be ordered by start_char"
            );
        }
    }
}
