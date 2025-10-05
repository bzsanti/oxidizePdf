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
        }
    }

    /// Create a default chunker with sensible defaults for most LLMs
    ///
    /// Uses 512 token chunks with 50 token overlap, which works well with
    /// GPT-3.5, GPT-4, and similar models.
    pub fn default() -> Self {
        Self::new(512, 50)
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

        self.chunk_text_internal(&full_text, &page_boundaries, page_numbers[0])
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
        // TODO: Use proper tokenizer (tiktoken) for accurate token counts
        let tokens: Vec<&str> = text.split_whitespace().collect();

        if tokens.is_empty() {
            return Ok(Vec::new());
        }

        let mut chunks = Vec::new();
        let mut start = 0;
        let mut chunk_idx = 0;
        let mut char_offset = 0;

        while start < tokens.len() {
            // Calculate end position for this chunk
            let mut end = (start + self.chunk_size).min(tokens.len());

            // Try to respect sentence boundaries
            let sentence_boundary_respected = if end < tokens.len() && end > start {
                // Look for sentence endings in the last few tokens
                let search_window = (end.saturating_sub(10)..end).rev();
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
                },
            };

            chunks.push(chunk);
            chunk_idx += 1;

            // Move start position with overlap
            if end < tokens.len() {
                // Not at end yet, apply overlap
                start = end.saturating_sub(self.overlap);

                // Ensure we make progress (avoid infinite loop)
                if start + self.chunk_size <= end {
                    start = end;
                }
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

        println!("Chunked 100 pages in {:?}", duration);
        println!("Created {} chunks", chunks.len());

        // Target: < 500ms for 100 pages (relaxed for debug builds)
        // In release mode this should be well under 100ms
        assert!(
            duration.as_millis() < 500,
            "Chunking 100 pages took {:?}, should be < 500ms",
            duration
        );
    }
}
