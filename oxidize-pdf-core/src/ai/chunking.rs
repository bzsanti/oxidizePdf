//! Document chunking for RAG (Retrieval Augmented Generation)
//!
//! This module provides functionality to split PDF documents into smaller chunks
//! suitable for processing with Large Language Models (LLMs). LLMs have token limits,
//! so long documents need to be split into manageable pieces while preserving context.
//!
//! # Example
//!
//! ```no_run
//! use oxidize_pdf::ai::DocumentChunker;
//! use oxidize_pdf::parser::{PdfReader, PdfDocument};
//!
//! # fn main() -> oxidize_pdf::Result<()> {
//! let reader = PdfReader::open("large_document.pdf")?;
//! let pdf_doc = PdfDocument::new(reader);
//! let text_pages = pdf_doc.extract_text()?;
//!
//! let chunker = DocumentChunker::new(512, 50);  // 512 tokens, 50 overlap
//! let page_texts: Vec<(usize, String)> = text_pages.iter()
//!     .enumerate()
//!     .map(|(idx, page)| (idx + 1, page.text.clone()))
//!     .collect();
//! let chunks = chunker.chunk_text_with_pages(&page_texts)?;
//!
//! println!("Created {} chunks", chunks.len());
//! for chunk in &chunks {
//!     println!("Chunk {}: {} tokens", chunk.id, chunk.tokens);
//! }
//! # Ok(())
//! # }
//! ```

use crate::{Document, Result};
use std::collections::HashMap;

/// A chunk of a PDF document suitable for LLM processing
///
/// Each chunk represents a portion of the document's text with associated metadata
/// that helps maintain context during retrieval and generation.
#[derive(Debug, Clone)]
pub struct DocumentChunk {
    /// Unique identifier for this chunk (e.g., "chunk_0", "chunk_1")
    pub id: String,

    /// The text content of this chunk
    pub content: String,

    /// Estimated number of tokens in this chunk
    pub tokens: usize,

    /// Page numbers where this chunk's content appears (1-indexed)
    pub page_numbers: Vec<usize>,

    /// Index of this chunk in the sequence (0-indexed)
    pub chunk_index: usize,

    /// Additional metadata for this chunk
    pub metadata: ChunkMetadata,
}

/// A language detected for a chunk or aggregated over a document.
///
/// `code` is the ISO 639-3 code (e.g. `"eng"`, `"spa"`, `"cmn"`). The
/// underlying detector (`whatlang`) is an internal implementation detail and is
/// not part of this public API.
#[derive(Debug, Clone, PartialEq)]
pub struct DetectedLanguage {
    /// ISO 639-3 language code.
    pub code: String,
    /// Detector confidence in `[0.0, 1.0]`.
    pub confidence: f32,
    /// Whether the detector considers this detection reliable.
    pub reliable: bool,
}

/// Detect the language of `text` using `whatlang`. Returns `None` only when
/// `whatlang` cannot produce any detection (e.g. empty input). Detections are
/// surfaced as-is, including unreliable ones — callers decide whether to trust a
/// result using the `reliable` flag (and `confidence`). Unreliable detections on
/// short or ambiguous text carry effectively-random codes, so consumers should
/// gate routing on `reliable`.
#[cfg(feature = "language-detection")]
fn detect_chunk_language(text: &str) -> Option<DetectedLanguage> {
    whatlang::detect(text).map(|info| DetectedLanguage {
        code: info.lang().code().to_string(),
        confidence: info.confidence() as f32,
        reliable: info.is_reliable(),
    })
}

/// No-op when the `language-detection` feature is disabled.
#[cfg(not(feature = "language-detection"))]
fn detect_chunk_language(_text: &str) -> Option<DetectedLanguage> {
    None
}

/// Metadata for a document chunk
#[derive(Debug, Clone, Default)]
pub struct ChunkMetadata {
    /// Position information about where this chunk appears in the document
    pub position: ChunkPosition,

    /// Confidence score for text extraction quality (0.0-1.0)
    /// 1.0 = high confidence, 0.0 = low confidence
    pub confidence: f32,

    /// Whether this chunk respects sentence boundaries
    pub sentence_boundary_respected: bool,

    /// Detected language for this chunk, if language detection ran
    /// (`DocumentChunker::with_language_detection(true)` + the
    /// `language-detection` feature). `None` otherwise.
    pub language: Option<DetectedLanguage>,
}

/// Position information for a chunk within the document
#[derive(Debug, Clone, Default)]
pub struct ChunkPosition {
    /// Character offset where this chunk starts in the full document text
    pub start_char: usize,

    /// Character offset where this chunk ends in the full document text
    pub end_char: usize,

    /// First page number where this chunk appears (1-indexed)
    pub first_page: usize,

    /// Last page number where this chunk appears (1-indexed)
    pub last_page: usize,
}

/// Configurable document chunker for splitting PDFs into LLM-friendly pieces
///
/// The chunker uses a simple fixed-size strategy with overlap to ensure context
/// is preserved between consecutive chunks.
///
/// # Example
///
/// ```no_run
/// use oxidize_pdf::ai::DocumentChunker;
///
/// // Create a chunker with 512 token chunks and 50 token overlap
/// let chunker = DocumentChunker::new(512, 50);
/// ```
#[derive(Debug, Clone)]
pub struct DocumentChunker {
    /// Target size for each chunk in tokens
    chunk_size: usize,

    /// Number of tokens to overlap between consecutive chunks
    overlap: usize,

    /// Whether to run per-chunk language detection
    detect_language: bool,
}

impl DocumentChunker {
    /// Create a new document chunker with specified chunk size and overlap
    ///
    /// # Arguments
    ///
    /// * `chunk_size` - Target number of tokens per chunk (typical: 256-1024)
    /// * `overlap` - Number of tokens to overlap between chunks (typical: 10-100)
    ///
    /// # Example
    ///
    /// ```
    /// use oxidize_pdf::ai::DocumentChunker;
    ///
    /// // For GPT-3.5/4 with 4K context, use smaller chunks
    /// let chunker = DocumentChunker::new(512, 50);
    ///
    /// // For Claude with 100K context, you can use larger chunks
    /// let chunker_large = DocumentChunker::new(2048, 200);
    /// ```
    pub fn new(chunk_size: usize, overlap: usize) -> Self {
        Self {
            chunk_size,
            overlap,
            detect_language: false,
        }
    }

    /// Create a default chunker with sensible defaults for most LLMs
    ///
    /// Uses 512 token chunks with 50 token overlap, which works well with
    /// GPT-3.5, GPT-4, and similar models.
    pub fn default() -> Self {
        Self::new(512, 50)
    }

    /// Enable per-chunk language detection.
    ///
    /// Requires the `language-detection` feature; without it this flag is a
    /// no-op and `ChunkMetadata::language` stays `None`. Disabled by default.
    pub fn with_language_detection(mut self, enabled: bool) -> Self {
        self.detect_language = enabled;
        self
    }

    /// Dominant language across chunks that already carry a detected language,
    /// weighted by chunk content length (chars). Returns `None` if no chunk has
    /// a language.
    ///
    /// `confidence` is the length-weighted mean of the winning code's chunk
    /// confidences; `reliable` is true if any contributing chunk for the winning
    /// code was reliable.
    pub fn document_language(chunks: &[DocumentChunk]) -> Option<DetectedLanguage> {
        // Per-code accumulators: (total_weight, weighted_confidence_sum, any_reliable)
        let mut acc: HashMap<String, (usize, f64, bool)> = HashMap::new();
        for chunk in chunks {
            if let Some(lang) = &chunk.metadata.language {
                let weight = chunk.content.chars().count().max(1);
                let entry = acc.entry(lang.code.clone()).or_insert((0, 0.0, false));
                entry.0 += weight;
                entry.1 += weight as f64 * lang.confidence as f64;
                entry.2 |= lang.reliable;
            }
        }

        // Winner = highest total weight; tie broken by code for determinism.
        let (code, (total_weight, conf_sum, reliable)) = acc
            .into_iter()
            .max_by(|a, b| a.1 .0.cmp(&b.1 .0).then_with(|| b.0.cmp(&a.0)))?;

        Some(DetectedLanguage {
            code,
            confidence: (conf_sum / total_weight as f64) as f32,
            reliable,
        })
    }

    /// Chunk a PDF document into pieces suitable for LLM processing
    ///
    /// # Arguments
    ///
    /// * `doc` - The PDF document to chunk
    ///
    /// # Returns
    ///
    /// A vector of `DocumentChunk` objects, each containing a portion of the document.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use oxidize_pdf::{Document, ai::DocumentChunker};
    ///
    /// # fn main() -> oxidize_pdf::Result<()> {
    /// let doc = Document::new();
    /// // Add pages to doc...
    /// let chunker = DocumentChunker::new(512, 50);
    /// let chunks = chunker.chunk_document(&doc)?;
    ///
    /// for chunk in chunks {
    ///     println!("Processing chunk {}: {} tokens", chunk.id, chunk.tokens);
    ///     // Send to LLM for processing...
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn chunk_document(&self, doc: &Document) -> Result<Vec<DocumentChunk>> {
        // Extract all text from the document
        let full_text = doc.extract_text()?;

        // Chunk the text
        self.chunk_text(&full_text)
    }

    /// Chunk a text string into fixed-size pieces with overlap
    ///
    /// This is the core chunking algorithm that:
    /// 1. Tokenizes the text (simple whitespace split)
    /// 2. Creates chunks of `chunk_size` tokens
    /// 3. Applies `overlap` tokens between consecutive chunks
    /// 4. Respects sentence boundaries when possible
    ///
    /// # Arguments
    ///
    /// * `text` - The text to chunk
    ///
    /// # Returns
    ///
    /// A vector of `DocumentChunk` objects
    ///
    /// # Example
    ///
    /// ```
    /// use oxidize_pdf::ai::DocumentChunker;
    ///
    /// let chunker = DocumentChunker::new(10, 2);
    /// let text = "This is some sample text that will be chunked into smaller pieces";
    /// let chunks = chunker.chunk_text(text).unwrap();
    /// println!("Created {} chunks", chunks.len());
    /// ```
    pub fn chunk_text(&self, text: &str) -> Result<Vec<DocumentChunk>> {
        // For simple text chunking, we don't have page information
        self.chunk_text_internal(text, &[], 0)
    }

    /// Chunk text with page information for accurate page tracking
    ///
    /// # Arguments
    ///
    /// * `page_texts` - Vector of (page_number, text) tuples (1-indexed page numbers)
    ///
    /// # Returns
    ///
    /// A vector of `DocumentChunk` objects with page tracking
    pub fn chunk_text_with_pages(
        &self,
        page_texts: &[(usize, String)],
    ) -> Result<Vec<DocumentChunk>> {
        // Combine all page texts with page markers
        let mut full_text = String::new();
        let mut page_boundaries = vec![0]; // Character positions where pages start

        for (_page_num, text) in page_texts {
            if !full_text.is_empty() {
                full_text.push_str("\n\n"); // Page separator
            }
            full_text.push_str(text);
            page_boundaries.push(full_text.len());
        }

        let page_numbers: Vec<usize> = page_texts.iter().map(|(num, _)| *num).collect();

        // `page_texts` may be empty (e.g. a PDF from which no text extracted);
        // fall back to page 0 rather than indexing into an empty vec.
        let first_page = page_numbers.first().copied().unwrap_or(0);
        self.chunk_text_internal(&full_text, &page_boundaries, first_page)
    }

    /// Internal chunking implementation with page tracking
    fn chunk_text_internal(
        &self,
        text: &str,
        page_boundaries: &[usize],
        first_page: usize,
    ) -> Result<Vec<DocumentChunk>> {
        if text.is_empty() {
            return Ok(Vec::new());
        }

        // Tokenize: simple whitespace split for now
        // Enhancement: Use proper tokenizer (tiktoken) for accurate token counts
        // Priority: MEDIUM - Current whitespace split provides estimates
        // Accurate tokenization would require tiktoken-rs external dependency
        // Target: v1.7.0 for LLM integration improvements
        let tokens: Vec<&str> = text.split_whitespace().collect();

        if tokens.is_empty() {
            return Ok(Vec::new());
        }

        // Normalize degenerate configuration without changing the public
        // constructor: a chunk must hold at least one token, and the overlap must
        // leave room for `start` to advance by at least one token between chunks.
        let chunk_size = self.chunk_size.max(1);
        let overlap = self.overlap.min(chunk_size - 1);

        let mut chunks = Vec::new();
        let mut start = 0;
        let mut chunk_idx = 0;
        let mut char_offset = 0;

        while start < tokens.len() {
            // Calculate end position for this chunk
            let mut end = (start + chunk_size).min(tokens.len());

            // Try to respect sentence boundaries
            let sentence_boundary_respected = if end < tokens.len() && end > start {
                // Look for sentence endings in the last few tokens, but never
                // before this chunk's own start: backtracking past `start` would
                // collapse `end` to <= start, panicking on tokens[start..end] and
                // stalling forward progress (#308).
                let window_start = end.saturating_sub(10).max(start + 1);
                let search_window = (window_start..end).rev();
                let mut found_boundary = false;

                for i in search_window {
                    let token = tokens[i];
                    if token.ends_with('.') || token.ends_with('!') || token.ends_with('?') {
                        end = i + 1; // Include the sentence-ending token
                        found_boundary = true;
                        break;
                    }
                }
                found_boundary
            } else {
                false
            };

            // Extract chunk tokens
            let chunk_tokens = &tokens[start..end];

            // Join tokens back into text
            let content = chunk_tokens.join(" ");

            // Detect language for this chunk (no-op unless enabled + feature on)
            let language = if self.detect_language {
                detect_chunk_language(&content)
            } else {
                None
            };

            // Calculate character positions
            let start_char = char_offset;
            let end_char = char_offset + content.len();
            char_offset = end_char;

            // Determine page numbers for this chunk
            let (page_nums, first_pg, last_pg) = if page_boundaries.is_empty() {
                (Vec::new(), 0, 0)
            } else {
                let mut pages = Vec::new();
                let mut first = first_page;
                let mut last = first_page;

                for (idx, &boundary) in page_boundaries.iter().enumerate().skip(1) {
                    if start_char < boundary && end_char > page_boundaries[idx - 1] {
                        let page_num = first_page + idx - 1;
                        pages.push(page_num);
                        if pages.len() == 1 {
                            first = page_num;
                        }
                        last = page_num;
                    }
                }

                if pages.is_empty() {
                    // Chunk is beyond all tracked pages
                    pages.push(first_page);
                    first = first_page;
                    last = first_page;
                }

                (pages, first, last)
            };

            // Create chunk
            let chunk = DocumentChunk {
                id: format!("chunk_{}", chunk_idx),
                content,
                tokens: chunk_tokens.len(),
                page_numbers: page_nums.clone(),
                chunk_index: chunk_idx,
                metadata: ChunkMetadata {
                    position: ChunkPosition {
                        start_char,
                        end_char,
                        first_page: first_pg,
                        last_page: last_pg,
                    },
                    confidence: 1.0, // Default high confidence for text-based chunking
                    sentence_boundary_respected,
                    language,
                },
            };

            chunks.push(chunk);
            chunk_idx += 1;

            // Move start position with overlap
            if end < tokens.len() {
                // Apply overlap, but guarantee strict forward progress regardless
                // of how far sentence-boundary backtracking pulled `end` back or
                // how `overlap` compares to the chunk size (#308). `end > start`
                // always holds here, so falling back to `start = end` advances.
                let next_start = end.saturating_sub(overlap);
                start = if next_start > start { next_start } else { end };
            } else {
                // Reached the end
                break;
            }
        }

        Ok(chunks)
    }

    /// Estimate the number of tokens in a text string
    ///
    /// Uses a simple approximation: 1 token ≈ 0.75 words (or ~1.33 tokens per word).
    /// This is reasonably accurate for English text with GPT models.
    ///
    /// # Arguments
    ///
    /// * `text` - The text to estimate tokens for
    ///
    /// # Returns
    ///
    /// Estimated number of tokens
    ///
    /// # Note
    ///
    /// This is an approximation. For exact token counts, integrate with
    /// a proper tokenizer like tiktoken.
    pub fn estimate_tokens(text: &str) -> usize {
        // Simple approximation: count words
        // 1 token ≈ 0.75 words for English text
        let words = text.split_whitespace().count();
        ((words as f32) * 1.33) as usize
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_chunking() {
        let chunker = DocumentChunker::new(10, 2);

        // Create text with exactly 25 words
        let text = (0..25)
            .map(|i| format!("word{}", i))
            .collect::<Vec<_>>()
            .join(" ");

        let chunks = chunker.chunk_text(&text).unwrap();

        // Should create 3 chunks:
        // Chunk 0: words 0-9 (10 tokens)
        // Chunk 1: words 8-17 (10 tokens, overlap of 2)
        // Chunk 2: words 16-24 (9 tokens, overlap of 2)
        assert_eq!(chunks.len(), 3, "Should create 3 chunks");

        // Check first chunk
        assert_eq!(chunks[0].tokens, 10);
        assert_eq!(chunks[0].chunk_index, 0);
        assert_eq!(chunks[0].id, "chunk_0");
        assert_eq!(chunks[0].metadata.position.start_char, 0);

        // Check second chunk
        assert_eq!(chunks[1].tokens, 10);
        assert_eq!(chunks[1].chunk_index, 1);

        // Check third chunk
        assert_eq!(chunks[2].tokens, 9);
        assert_eq!(chunks[2].chunk_index, 2);
    }

    #[test]
    fn test_overlap_preserves_context() {
        let chunker = DocumentChunker::new(5, 2);

        // Text: "a b c d e f g h i j"
        let text = "a b c d e f g h i j";

        let chunks = chunker.chunk_text(&text).unwrap();

        // Chunk 0: a b c d e (positions 0-4)
        // Chunk 1: d e f g h (positions 3-7, overlap of 2: d e)
        // Chunk 2: g h i j (positions 6-9, overlap of 2: g h)

        // Check overlap between chunk 0 and 1
        let chunk0_end = chunks[0]
            .content
            .split_whitespace()
            .rev()
            .take(2)
            .collect::<Vec<_>>();
        let chunk1_start = chunks[1]
            .content
            .split_whitespace()
            .take(2)
            .collect::<Vec<_>>();

        assert_eq!(chunk0_end, vec!["e", "d"]);
        assert_eq!(chunk1_start, vec!["d", "e"]);
    }

    #[test]
    fn test_empty_text() {
        let chunker = DocumentChunker::new(10, 2);
        let chunks = chunker.chunk_text("").unwrap();
        assert_eq!(chunks.len(), 0);
    }

    #[test]
    fn test_text_smaller_than_chunk_size() {
        let chunker = DocumentChunker::new(100, 10);
        let text = "just a few words";

        let chunks = chunker.chunk_text(&text).unwrap();

        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].tokens, 4);
    }

    #[test]
    fn test_token_estimation() {
        // "hello world" = 2 words ≈ 2.66 tokens
        let tokens = DocumentChunker::estimate_tokens("hello world");
        assert!(
            tokens >= 2 && tokens <= 3,
            "Expected ~2-3 tokens, got {}",
            tokens
        );

        // Empty text
        assert_eq!(DocumentChunker::estimate_tokens(""), 0);

        // Longer text: 100 words ≈ 133 tokens
        let long_text = (0..100)
            .map(|i| format!("word{}", i))
            .collect::<Vec<_>>()
            .join(" ");
        let tokens_long = DocumentChunker::estimate_tokens(&long_text);
        assert!(
            tokens_long >= 120 && tokens_long <= 140,
            "Expected ~133 tokens, got {}",
            tokens_long
        );
    }

    #[test]
    fn test_chunk_ids_are_unique() {
        let chunker = DocumentChunker::new(5, 1);
        let text = (0..20)
            .map(|i| format!("word{}", i))
            .collect::<Vec<_>>()
            .join(" ");

        let chunks = chunker.chunk_text(&text).unwrap();

        let ids: Vec<String> = chunks.iter().map(|c| c.id.clone()).collect();
        let unique_ids: std::collections::HashSet<_> = ids.iter().collect();

        assert_eq!(
            ids.len(),
            unique_ids.len(),
            "All chunk IDs should be unique"
        );
    }

    #[test]
    fn test_sentence_boundary_detection() {
        let chunker = DocumentChunker::new(10, 2);

        let text = "This is the first sentence. This is the second sentence. This is the third sentence. And here is a fourth one.";

        let chunks = chunker.chunk_text(&text).unwrap();

        // At least some chunks should respect sentence boundaries
        let has_boundary_respect = chunks
            .iter()
            .any(|c| c.metadata.sentence_boundary_respected);
        assert!(
            has_boundary_respect,
            "At least some chunks should respect sentence boundaries"
        );

        // Check that sentences aren't broken in the middle (chunks should end with punctuation or be the last chunk)
        for (i, chunk) in chunks.iter().enumerate() {
            if i < chunks.len() - 1 && chunk.metadata.sentence_boundary_respected {
                assert!(
                    chunk.content.ends_with('.')
                        || chunk.content.ends_with('!')
                        || chunk.content.ends_with('?'),
                    "Chunk {} should end with sentence punctuation",
                    i
                );
            }
        }
    }

    #[test]
    fn test_page_tracking() {
        let chunker = DocumentChunker::new(10, 2);

        let page_texts = vec![
            (1, "This is page one content.".to_string()),
            (2, "This is page two content.".to_string()),
            (3, "This is page three content.".to_string()),
        ];

        let chunks = chunker.chunk_text_with_pages(&page_texts).unwrap();

        // All chunks should have page information
        for chunk in &chunks {
            assert!(
                !chunk.page_numbers.is_empty(),
                "Chunk should have page numbers"
            );
            assert!(
                chunk.metadata.position.first_page > 0,
                "First page should be > 0"
            );
            assert!(
                chunk.metadata.position.last_page > 0,
                "Last page should be > 0"
            );
        }

        // First chunk should start at page 1
        assert_eq!(
            chunks[0].metadata.position.first_page, 1,
            "First chunk should start at page 1"
        );
    }

    #[test]
    fn test_metadata_position_tracking() {
        let chunker = DocumentChunker::new(5, 1);

        let text = "word1 word2 word3 word4 word5 word6 word7 word8 word9 word10";

        let chunks = chunker.chunk_text(&text).unwrap();

        // Check that positions are sequential and non-overlapping in character space
        for i in 0..chunks.len() - 1 {
            assert!(
                chunks[i].metadata.position.end_char
                    <= chunks[i + 1].metadata.position.start_char + 10,
                "Chunks should have reasonable character positions"
            );
        }

        // First chunk should start at position 0
        assert_eq!(chunks[0].metadata.position.start_char, 0);

        // Each chunk should have a meaningful character range
        for chunk in &chunks {
            assert!(
                chunk.metadata.position.end_char > chunk.metadata.position.start_char,
                "End char should be greater than start char"
            );
        }
    }

    #[test]
    fn test_confidence_scores() {
        let chunker = DocumentChunker::new(10, 2);

        let text = "This is a test document with multiple sentences.";

        let chunks = chunker.chunk_text(&text).unwrap();

        // All chunks should have confidence scores
        for chunk in &chunks {
            assert!(
                chunk.metadata.confidence >= 0.0 && chunk.metadata.confidence <= 1.0,
                "Confidence should be between 0.0 and 1.0"
            );
        }
    }

    #[test]
    fn test_performance_100_pages() {
        use std::time::Instant;

        let chunker = DocumentChunker::new(512, 50);

        // Generate 100 pages with ~200 words each (typical PDF page)
        let page_texts: Vec<(usize, String)> = (1..=100)
            .map(|page_num| {
                let words: Vec<String> = (0..200).map(|i| format!("word{}", i)).collect();
                (page_num, words.join(" "))
            })
            .collect();

        let start = Instant::now();
        let chunks = chunker.chunk_text_with_pages(&page_texts).unwrap();
        let duration = start.elapsed();

        tracing::debug!("Chunked 100 pages in {:?}", duration);
        tracing::debug!("Created {} chunks", chunks.len());

        // Target: < 500ms for 100 pages (relaxed for debug builds)
        // In release mode this should be well under 100ms
        assert!(
            duration.as_millis() < 500,
            "Chunking 100 pages took {:?}, should be < 500ms",
            duration
        );
    }

    /// Run `chunk_text` on a worker thread so a reintroduced infinite loop fails
    /// the test with a timeout instead of hanging the whole test runner.
    fn chunk_text_bounded(chunker: DocumentChunker, text: &str) -> Vec<DocumentChunk> {
        use std::sync::mpsc;
        use std::time::Duration;

        let (tx, rx) = mpsc::channel();
        let owned = text.to_string();
        std::thread::spawn(move || {
            let result = chunker.chunk_text(&owned);
            // Ignore send errors: the receiver may have already timed out.
            let _ = tx.send(result);
        });

        match rx.recv_timeout(Duration::from_secs(10)) {
            Ok(result) => result.expect("chunk_text returned an error"),
            Err(_) => panic!("chunk_text did not terminate within 10s (infinite loop, #308)"),
        }
    }

    /// Reconstruct the set of source tokens covered by the chunks. Every input
    /// token must appear in at least one chunk (coverage), proving the loop both
    /// terminated and did not silently drop content.
    fn assert_full_token_coverage(text: &str, chunks: &[DocumentChunk]) {
        let mut covered = std::collections::HashSet::new();
        for chunk in chunks {
            for tok in chunk.content.split_whitespace() {
                covered.insert(tok.to_string());
            }
        }
        for tok in text.split_whitespace() {
            assert!(
                covered.contains(tok),
                "token {:?} from the source is missing from all chunks",
                tok
            );
        }
    }

    #[test]
    fn test_no_infinite_loop_when_sentence_boundary_at_chunk_start() {
        // Exact reproduction from issue #308: chunk_size=10, overlap=2, and the
        // only sentence-ending token in the first window is at index 0 ("Hi.").
        // Pre-fix, sentence-boundary backtracking set end=1, start=0 stayed put,
        // and the loop never advanced.
        let chunker = DocumentChunker::new(10, 2);
        let text = "Hi. word word word word word word word word word word word word";

        let chunks = chunk_text_bounded(chunker, text);

        assert!(!chunks.is_empty(), "must produce at least one chunk");
        assert_full_token_coverage(text, &chunks);
        // Chunk indices must be strictly sequential (no repeats from a stalled loop).
        for (i, chunk) in chunks.iter().enumerate() {
            assert_eq!(chunk.chunk_index, i, "chunk indices must be sequential");
        }
    }

    #[test]
    fn test_no_infinite_loop_when_overlap_meets_or_exceeds_chunk_size() {
        // overlap >= chunk_size leaves no room for the overlap to advance `start`.
        // The loop must still make forward progress via the strict-advance guard.
        let text = (0..30)
            .map(|i| format!("word{}", i))
            .collect::<Vec<_>>()
            .join(" ");

        for (chunk_size, overlap) in [(3usize, 5usize), (4, 4), (1, 10)] {
            let chunker = DocumentChunker::new(chunk_size, overlap);
            let chunks = chunk_text_bounded(chunker, &text);
            assert!(
                !chunks.is_empty(),
                "chunk_size={chunk_size}, overlap={overlap}: must produce chunks"
            );
            assert_full_token_coverage(&text, &chunks);
        }
    }

    #[test]
    fn test_no_panic_when_chunk_size_below_search_window() {
        // chunk_size < 10 means the raw sentence-boundary search window
        // (end-10 .. end) can reach below `start`. The window lower bound must be
        // clamped so `end` never collapses to <= start (would panic on the slice
        // tokens[start..end]).
        let text =
            "first. second third fourth. fifth sixth seventh eighth. ninth tenth eleventh twelfth";
        let chunker = DocumentChunker::new(4, 1);

        let chunks = chunk_text_bounded(chunker, text);

        assert!(!chunks.is_empty());
        assert_full_token_coverage(text, &chunks);
        // No empty chunks: every chunk holds at least one token.
        for chunk in &chunks {
            assert!(chunk.tokens >= 1, "chunk {} is empty", chunk.chunk_index);
        }
    }

    #[test]
    fn test_zero_chunk_size_terminates_with_coverage() {
        // Degenerate chunk_size=0 must not loop forever; it is normalized to a
        // minimum of one token per chunk.
        let text = "alpha beta gamma delta epsilon";
        let chunker = DocumentChunker::new(0, 0);

        let chunks = chunk_text_bounded(chunker, text);

        assert!(!chunks.is_empty());
        assert_full_token_coverage(text, &chunks);
    }

    #[test]
    fn test_sentence_boundary_still_respected_after_loop_fix() {
        // Regression guard: the loop fix must not disable the sentence-boundary
        // feature. With a period mid-window, the first chunk should end at the
        // sentence boundary, not at the raw chunk_size cut.
        let chunker = DocumentChunker::new(10, 2);
        let text = "one two three four five. six seven eight nine ten eleven twelve thirteen";

        let chunks = chunk_text_bounded(chunker, text);

        assert!(chunks[0].metadata.sentence_boundary_respected);
        assert!(
            chunks[0].content.ends_with("five."),
            "first chunk should end at the sentence boundary, got: {:?}",
            chunks[0].content
        );
        assert_full_token_coverage(text, &chunks);
    }
}
