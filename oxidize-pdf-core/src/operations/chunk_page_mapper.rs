//! Chunk-to-page mapper for RAG-aligned PDF editing
//!
//! Maps `DocumentChunk` page numbers to PDF page indices and extracts
//! only the pages relevant to retrieved chunks.

use std::io::Cursor;

use crate::ai::chunking::DocumentChunk;

use super::OperationError;
use super::OperationResult;

/// Maps RAG chunks to their corresponding PDF pages and extracts relevant pages.
pub struct ChunkPageMapper;

impl ChunkPageMapper {
    /// Get the 0-indexed page indices covered by the given chunks.
    ///
    /// Chunk `page_numbers` are 1-indexed (matching the chunker convention).
    /// Returns sorted, deduplicated, 0-indexed page indices.
    pub fn pages_for_chunks(chunks: &[&DocumentChunk]) -> Vec<usize> {
        let mut pages: Vec<usize> = chunks
            .iter()
            .flat_map(|c| c.page_numbers.iter())
            .filter(|&&p| p > 0)
            .map(|&p| p - 1) // 1-indexed → 0-indexed
            .collect();
        pages.sort();
        pages.dedup();
        pages
    }

    /// Extract only the pages referenced by the given chunks into a new PDF.
    ///
    /// # Arguments
    ///
    /// * `pdf_bytes` - The original PDF file bytes
    /// * `chunks` - Chunks whose pages should be extracted
    ///
    /// # Returns
    ///
    /// The new PDF bytes containing only the relevant pages.
    pub fn extract_pages_for_chunks(
        pdf_bytes: &[u8],
        chunks: &[&DocumentChunk],
    ) -> OperationResult<Vec<u8>> {
        let page_indices = Self::pages_for_chunks(chunks);

        if page_indices.is_empty() {
            return Err(OperationError::NoPagesToProcess);
        }

        let cursor = Cursor::new(pdf_bytes);
        let reader = crate::parser::PdfReader::new(cursor)
            .map_err(|e| OperationError::ParseError(e.to_string()))?;
        let document = reader.into_document();

        let page_count = document
            .page_count()
            .map_err(|e| OperationError::ParseError(e.to_string()))?
            as usize;

        // Validate indices
        for &idx in &page_indices {
            if idx >= page_count {
                return Err(OperationError::PageIndexOutOfBounds(idx, page_count));
            }
        }

        let mut output_doc = crate::document::Document::new();

        for &page_idx in &page_indices {
            let parsed_page = document
                .get_page(page_idx as u32)
                .map_err(|e| OperationError::ParseError(e.to_string()))?;

            let page = crate::page::Page::from_parsed_with_content(&parsed_page, &document)
                .map_err(|e| OperationError::ParseError(e.to_string()))?;

            output_doc.add_page(page);
        }

        output_doc.to_bytes().map_err(OperationError::PdfError)
    }
}
