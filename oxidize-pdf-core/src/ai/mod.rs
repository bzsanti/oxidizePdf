//! AI/ML integration utilities for PDF processing
//!
//! This module provides tools for integrating PDF documents into AI/ML pipelines,
//! including document chunking for RAG (Retrieval Augmented Generation), LLM-optimized
//! formats, and vector store preparation.
//!
//! # Quick Start
//!
//! ```rust,no_run
//! use oxidize_pdf::parser::{PdfDocument, PdfReader};
//! use oxidize_pdf::ai;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! // Load a PDF
//! let reader = PdfReader::open("document.pdf")?;
//! let document = PdfDocument::new(reader);
//!
//! // Export to Markdown
//! let markdown = ai::export_to_markdown(&document)?;
//! println!("{}", markdown);
//!
//! // Export to Contextual format for LLM prompts
//! let contextual = ai::export_to_contextual(&document)?;
//! println!("{}", contextual);
//!
//! # Ok(())
//! # }
//! ```

pub mod chunking;
pub mod formats;

pub use chunking::{ChunkMetadata, ChunkPosition, DocumentChunk, DocumentChunker};
pub use formats::{ContextualFormat, DocumentMetadata, MarkdownExporter, MarkdownOptions};

#[cfg(feature = "semantic")]
pub use formats::{JsonExporter, JsonOptions};

use crate::error::Result;
use std::io::{Read, Seek};

/// Export a parsed PDF document to Markdown format.
///
/// This function extracts text from all pages and exports it with
/// page markers and YAML frontmatter metadata.
///
/// # Arguments
///
/// * `document` - A parsed PDF document
///
/// # Returns
///
/// A Markdown-formatted string with document metadata and page content.
///
/// # Example
///
/// ```rust,no_run
/// use oxidize_pdf::parser::{PdfDocument, PdfReader};
/// use oxidize_pdf::ai;
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let reader = PdfReader::open("report.pdf")?;
/// let document = PdfDocument::new(reader);
///
/// let markdown = ai::export_to_markdown(&document)?;
/// std::fs::write("output.md", markdown)?;
/// # Ok(())
/// # }
/// ```
pub fn export_to_markdown<R: Read + Seek>(
    document: &crate::parser::PdfDocument<R>,
) -> Result<String> {
    // Extract text from all pages
    let extracted = document.extract_text()?;

    // Collect pages as (page_num, text) tuples
    let pages: Vec<(usize, String)> = extracted
        .iter()
        .enumerate()
        .map(|(i, page_text)| (i + 1, page_text.text.clone()))
        .collect();

    // Get document metadata
    let parsed_metadata = document.metadata()?;
    let ai_metadata = DocumentMetadata {
        title: parsed_metadata
            .title
            .unwrap_or_else(|| "Untitled Document".to_string()),
        page_count: pages.len(),
        created_at: parsed_metadata.creation_date.clone(),
        author: parsed_metadata.author.clone(),
    };

    // Export with metadata and pages
    MarkdownExporter::export_with_metadata_and_pages(&pages, &ai_metadata)
}

/// Export a parsed PDF document to Contextual format for LLM prompt injection.
///
/// This function extracts text and formats it in natural language,
/// ideal for feeding directly into LLM prompts.
///
/// # Arguments
///
/// * `document` - A parsed PDF document
///
/// # Returns
///
/// A natural language formatted string describing the document and its content.
///
/// # Example
///
/// ```rust,no_run
/// use oxidize_pdf::parser::{PdfDocument, PdfReader};
/// use oxidize_pdf::ai;
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let reader = PdfReader::open("contract.pdf")?;
/// let document = PdfDocument::new(reader);
///
/// let contextual = ai::export_to_contextual(&document)?;
///
/// // Use in LLM prompt
/// let prompt = format!(
///     "You are a legal assistant. Below is a contract.\n\n{}\n\nQuestion: What is the termination clause?",
///     contextual
/// );
/// # Ok(())
/// # }
/// ```
pub fn export_to_contextual<R: Read + Seek>(
    document: &crate::parser::PdfDocument<R>,
) -> Result<String> {
    // Extract text from all pages
    let extracted = document.extract_text()?;

    // Collect pages as (page_num, text) tuples
    let pages: Vec<(usize, String)> = extracted
        .iter()
        .enumerate()
        .map(|(i, page_text)| (i + 1, page_text.text.clone()))
        .collect();

    // Get document metadata
    let parsed_metadata = document.metadata()?;
    let ai_metadata = DocumentMetadata {
        title: parsed_metadata
            .title
            .unwrap_or_else(|| "Untitled Document".to_string()),
        page_count: pages.len(),
        created_at: parsed_metadata.creation_date.clone(),
        author: parsed_metadata.author.clone(),
    };

    // Export with metadata and pages
    ContextualFormat::export_with_metadata_and_pages(&pages, &ai_metadata)
}

/// Export a parsed PDF document to JSON format (requires `semantic` feature).
///
/// This function extracts text from all pages and exports it as structured JSON,
/// ideal for API responses and data pipelines.
///
/// # Arguments
///
/// * `document` - A parsed PDF document
///
/// # Returns
///
/// A JSON-formatted string with document metadata and page content.
///
/// # Example
///
/// ```rust,no_run
/// use oxidize_pdf::parser::{PdfDocument, PdfReader};
/// use oxidize_pdf::ai;
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let reader = PdfReader::open("data.pdf")?;
/// let document = PdfDocument::new(reader);
///
/// let json = ai::export_to_json(&document)?;
/// std::fs::write("output.json", json)?;
/// # Ok(())
/// # }
/// ```
#[cfg(feature = "semantic")]
pub fn export_to_json<R: Read + Seek>(document: &crate::parser::PdfDocument<R>) -> Result<String> {
    // Extract text from all pages
    let extracted = document.extract_text()?;

    // Collect pages as (page_num, text) tuples
    let pages: Vec<(usize, String)> = extracted
        .iter()
        .enumerate()
        .map(|(i, page_text)| (i + 1, page_text.text.clone()))
        .collect();

    // Export pages as JSON
    JsonExporter::export_pages(&pages)
}

/// Export a parsed PDF document with chunking for RAG pipelines (requires `semantic` feature).
///
/// This function extracts text, chunks it into manageable pieces with overlap,
/// and exports as JSON with metadata for each chunk.
///
/// # Arguments
///
/// * `document` - A parsed PDF document
/// * `chunk_size` - Target size for each chunk in characters (default: 1000)
/// * `overlap` - Number of characters to overlap between chunks (default: 200)
///
/// # Returns
///
/// A JSON-formatted string with chunked document data ready for embedding.
///
/// # Example
///
/// ```rust,no_run
/// use oxidize_pdf::parser::{PdfDocument, PdfReader};
/// use oxidize_pdf::ai;
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let reader = PdfReader::open("knowledge_base.pdf")?;
/// let document = PdfDocument::new(reader);
///
/// // Chunk for embedding (1000 chars per chunk, 200 char overlap)
/// let chunks = ai::export_to_chunks(&document, 1000, 200)?;
/// std::fs::write("chunks.json", chunks)?;
/// # Ok(())
/// # }
/// ```
#[cfg(feature = "semantic")]
pub fn export_to_chunks<R: Read + Seek>(
    document: &crate::parser::PdfDocument<R>,
    chunk_size: usize,
    overlap: usize,
) -> Result<String> {
    // Extract text from all pages
    let extracted = document.extract_text()?;

    // Concatenate all page text
    let full_text: String = extracted
        .iter()
        .map(|page_text| page_text.text.as_str())
        .collect::<Vec<&str>>()
        .join("\n\n");

    // Create chunker with specified parameters
    let chunker = DocumentChunker::new(chunk_size, overlap);

    // Chunk the document
    let chunks = chunker.chunk_text(&full_text)?;

    // Export as JSON
    JsonExporter::export_with_chunks(&chunks)
}
