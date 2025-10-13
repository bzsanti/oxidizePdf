//! LLM-optimized export formats for PDF documents
//!
//! This module provides utilities to export PDF content in formats optimized for
//! Large Language Model (LLM) processing, including Markdown, JSON, and contextual formats.
//!
//! # Example
//!
//! ```no_run
//! use oxidize_pdf::ai::{MarkdownExporter, MarkdownOptions};
//!
//! # fn main() -> oxidize_pdf::Result<()> {
//! let text = "Hello, world! This is a PDF document.";
//! let markdown = MarkdownExporter::export_text(text)?;
//! println!("{}", markdown);
//! # Ok(())
//! # }
//! ```

use crate::Result;

#[cfg(feature = "semantic")]
use serde_json::{json, Value};

#[cfg(feature = "semantic")]
use super::chunking::DocumentChunk;

/// Metadata about a PDF document for export
#[derive(Debug, Clone)]
pub struct DocumentMetadata {
    /// Document title
    pub title: String,

    /// Total number of pages
    pub page_count: usize,

    /// Creation date (ISO 8601 format recommended)
    pub created_at: Option<String>,

    /// Author name
    pub author: Option<String>,
}

impl Default for DocumentMetadata {
    fn default() -> Self {
        Self {
            title: "Untitled Document".to_string(),
            page_count: 0,
            created_at: None,
            author: None,
        }
    }
}

/// Configuration options for Markdown export
#[derive(Debug, Clone)]
pub struct MarkdownOptions {
    /// Whether to include metadata headers (YAML frontmatter)
    pub include_metadata: bool,

    /// Whether to include page number markers
    pub include_page_numbers: bool,
}

impl Default for MarkdownOptions {
    fn default() -> Self {
        Self {
            include_metadata: true,
            include_page_numbers: true,
        }
    }
}

/// Exporter for converting PDF content to Markdown format
///
/// Markdown is a lightweight markup language that's highly readable by both humans
/// and LLMs, making it ideal for document processing pipelines.
///
/// # Example
///
/// ```no_run
/// use oxidize_pdf::ai::{MarkdownExporter, MarkdownOptions};
///
/// # fn main() -> oxidize_pdf::Result<()> {
/// let exporter = MarkdownExporter::new(MarkdownOptions::default());
/// let markdown = MarkdownExporter::export_text("Hello, world!")?;
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone)]
pub struct MarkdownExporter {
    options: MarkdownOptions,
}

impl MarkdownExporter {
    /// Create a new Markdown exporter with the given options
    ///
    /// # Arguments
    ///
    /// * `options` - Configuration for the export process
    ///
    /// # Example
    ///
    /// ```
    /// use oxidize_pdf::ai::{MarkdownExporter, MarkdownOptions};
    ///
    /// let exporter = MarkdownExporter::new(MarkdownOptions {
    ///     include_metadata: true,
    ///     include_page_numbers: false,
    /// });
    /// ```
    pub fn new(options: MarkdownOptions) -> Self {
        Self { options }
    }

    /// Create a new Markdown exporter with default options
    pub fn default() -> Self {
        Self::new(MarkdownOptions::default())
    }

    /// Export text using the configured options
    ///
    /// This method respects the exporter's options for metadata and page numbers.
    ///
    /// # Arguments
    ///
    /// * `text` - The text content to export
    ///
    /// # Returns
    ///
    /// A Markdown-formatted string
    pub fn export(&self, text: &str) -> Result<String> {
        if self.options.include_metadata {
            // For now, export with basic header
            // Will be enhanced in future phases
            Self::export_text(text)
        } else {
            Ok(text.to_string())
        }
    }

    /// Export plain text to Markdown format
    ///
    /// This is the simplest export method, converting raw text to a basic
    /// Markdown document with a title header.
    ///
    /// # Arguments
    ///
    /// * `text` - The text content to export
    ///
    /// # Returns
    ///
    /// A Markdown-formatted string
    ///
    /// # Example
    ///
    /// ```
    /// use oxidize_pdf::ai::MarkdownExporter;
    ///
    /// let text = "This is a sample document.";
    /// let markdown = MarkdownExporter::export_text(text).unwrap();
    /// assert!(markdown.contains("# Document"));
    /// assert!(markdown.contains("This is a sample document."));
    /// ```
    pub fn export_text(text: &str) -> Result<String> {
        // Phase 1: Simple conversion with basic header
        let mut output = String::new();
        output.push_str("# Document\n\n");
        output.push_str(text);
        Ok(output)
    }

    /// Export text with metadata as YAML frontmatter
    ///
    /// This adds a YAML header to the Markdown document with metadata like
    /// title, page count, author, and creation date.
    ///
    /// # Arguments
    ///
    /// * `text` - The text content to export
    /// * `metadata` - Document metadata to include
    ///
    /// # Returns
    ///
    /// A Markdown-formatted string with YAML frontmatter
    ///
    /// # Example
    ///
    /// ```
    /// use oxidize_pdf::ai::{MarkdownExporter, DocumentMetadata};
    ///
    /// let metadata = DocumentMetadata {
    ///     title: "My Document".to_string(),
    ///     page_count: 5,
    ///     created_at: Some("2025-10-13".to_string()),
    ///     author: Some("John Doe".to_string()),
    /// };
    ///
    /// let markdown = MarkdownExporter::export_with_metadata("Content here", &metadata).unwrap();
    /// assert!(markdown.contains("title: My Document"));
    /// assert!(markdown.contains("pages: 5"));
    /// ```
    pub fn export_with_metadata(text: &str, metadata: &DocumentMetadata) -> Result<String> {
        let mut output = String::new();

        // YAML frontmatter
        output.push_str("---\n");

        // Escape title if it contains special characters
        let escaped_title = if metadata.title.contains(':') || metadata.title.contains('#') {
            format!("\"{}\"", metadata.title.replace('"', "\\\""))
        } else {
            metadata.title.clone()
        };

        output.push_str(&format!("title: {}\n", escaped_title));
        output.push_str(&format!("pages: {}\n", metadata.page_count));

        if let Some(ref created) = metadata.created_at {
            output.push_str(&format!("created: {}\n", created));
        }

        if let Some(ref author) = metadata.author {
            let escaped_author = if author.contains(':') {
                format!("\"{}\"", author.replace('"', "\\\""))
            } else {
                author.clone()
            };
            output.push_str(&format!("author: {}\n", escaped_author));
        }

        output.push_str("---\n\n");

        // Content
        output.push_str(&format!("# {}\n\n", metadata.title));
        output.push_str(text);

        Ok(output)
    }

    /// Export multi-page text with page break markers
    ///
    /// This method creates a Markdown document with clear page boundaries,
    /// making it easy for LLMs to understand document structure.
    ///
    /// # Arguments
    ///
    /// * `page_texts` - Vector of (page_number, text) tuples (1-indexed)
    ///
    /// # Returns
    ///
    /// A Markdown-formatted string with page markers
    ///
    /// # Example
    ///
    /// ```
    /// use oxidize_pdf::ai::MarkdownExporter;
    ///
    /// let pages = vec![
    ///     (1, "Page 1 content".to_string()),
    ///     (2, "Page 2 content".to_string()),
    ///     (3, "Page 3 content".to_string()),
    /// ];
    ///
    /// let markdown = MarkdownExporter::export_with_pages(&pages).unwrap();
    /// assert!(markdown.contains("**Page 1**"));
    /// assert!(markdown.contains("**Page 2**"));
    /// ```
    pub fn export_with_pages(page_texts: &[(usize, String)]) -> Result<String> {
        let mut output = String::new();
        output.push_str("# Document\n\n");

        for (i, (page_num, text)) in page_texts.iter().enumerate() {
            if i > 0 {
                // Add page break separator
                output.push_str("\n\n---\n\n");
            }

            output.push_str(&format!("**Page {}**\n\n", page_num));
            output.push_str(text);
        }

        Ok(output)
    }

    /// Export multi-page text with metadata and page breaks
    ///
    /// Combines metadata frontmatter with page-by-page content.
    ///
    /// # Arguments
    ///
    /// * `page_texts` - Vector of (page_number, text) tuples (1-indexed)
    /// * `metadata` - Document metadata
    ///
    /// # Returns
    ///
    /// A Markdown-formatted string with YAML frontmatter and page markers
    ///
    /// # Example
    ///
    /// ```
    /// use oxidize_pdf::ai::{MarkdownExporter, DocumentMetadata};
    ///
    /// let metadata = DocumentMetadata {
    ///     title: "Multi-Page Doc".to_string(),
    ///     page_count: 2,
    ///     created_at: None,
    ///     author: None,
    /// };
    ///
    /// let pages = vec![
    ///     (1, "First page".to_string()),
    ///     (2, "Second page".to_string()),
    /// ];
    ///
    /// let markdown = MarkdownExporter::export_with_metadata_and_pages(&pages, &metadata).unwrap();
    /// assert!(markdown.contains("pages: 2"));
    /// assert!(markdown.contains("**Page 1**"));
    /// ```
    pub fn export_with_metadata_and_pages(
        page_texts: &[(usize, String)],
        metadata: &DocumentMetadata,
    ) -> Result<String> {
        let mut output = String::new();

        // YAML frontmatter
        output.push_str("---\n");

        let escaped_title = if metadata.title.contains(':') || metadata.title.contains('#') {
            format!("\"{}\"", metadata.title.replace('"', "\\\""))
        } else {
            metadata.title.clone()
        };

        output.push_str(&format!("title: {}\n", escaped_title));
        output.push_str(&format!("pages: {}\n", metadata.page_count));

        if let Some(ref created) = metadata.created_at {
            output.push_str(&format!("created: {}\n", created));
        }

        if let Some(ref author) = metadata.author {
            let escaped_author = if author.contains(':') {
                format!("\"{}\"", author.replace('"', "\\\""))
            } else {
                author.clone()
            };
            output.push_str(&format!("author: {}\n", escaped_author));
        }

        output.push_str("---\n\n");

        // Title
        output.push_str(&format!("# {}\n\n", metadata.title));

        // Pages
        for (i, (page_num, text)) in page_texts.iter().enumerate() {
            if i > 0 {
                output.push_str("\n\n---\n\n");
            }

            output.push_str(&format!("**Page {}**\n\n", page_num));
            output.push_str(text);
        }

        Ok(output)
    }
}

/// Configuration options for JSON export
#[cfg(feature = "semantic")]
#[derive(Debug, Clone)]
pub struct JsonOptions {
    /// Whether to pretty-print the JSON output
    pub pretty_print: bool,

    /// Whether to include chunk information
    pub include_chunks: bool,
}

#[cfg(feature = "semantic")]
impl Default for JsonOptions {
    fn default() -> Self {
        Self {
            pretty_print: true,
            include_chunks: false,
        }
    }
}

/// Exporter for converting PDF content to JSON format
///
/// JSON is a structured format that's easy to parse programmatically,
/// making it ideal for feeding PDF content into data pipelines and LLM APIs.
///
/// # Example
///
/// ```no_run
/// use oxidize_pdf::ai::{JsonExporter, JsonOptions};
///
/// # fn main() -> oxidize_pdf::Result<()> {
/// let exporter = JsonExporter::new(JsonOptions::default());
/// let json = JsonExporter::export_simple("Hello, world!")?;
/// println!("{}", json);
/// # Ok(())
/// # }
/// ```
#[cfg(feature = "semantic")]
#[derive(Debug, Clone)]
pub struct JsonExporter {
    options: JsonOptions,
}

#[cfg(feature = "semantic")]
impl JsonExporter {
    /// Create a new JSON exporter with the given options
    ///
    /// # Arguments
    ///
    /// * `options` - Configuration for the export process
    ///
    /// # Example
    ///
    /// ```
    /// use oxidize_pdf::ai::{JsonExporter, JsonOptions};
    ///
    /// let exporter = JsonExporter::new(JsonOptions {
    ///     pretty_print: true,
    ///     include_chunks: false,
    /// });
    /// ```
    pub fn new(options: JsonOptions) -> Self {
        Self { options }
    }

    /// Create a new JSON exporter with default options
    pub fn default() -> Self {
        Self::new(JsonOptions::default())
    }

    /// Export text using the configured options
    ///
    /// This method respects the exporter's options for formatting.
    ///
    /// # Arguments
    ///
    /// * `text` - The text content to export
    ///
    /// # Returns
    ///
    /// A JSON-formatted string
    pub fn export(&self, text: &str) -> Result<String> {
        let doc = json!({
            "type": "document",
            "content": text
        });

        if self.options.pretty_print {
            serde_json::to_string_pretty(&doc)
                .map_err(|e| crate::error::PdfError::SerializationError(e.to_string()))
        } else {
            serde_json::to_string(&doc)
                .map_err(|e| crate::error::PdfError::SerializationError(e.to_string()))
        }
    }

    /// Export plain text to simple JSON format
    ///
    /// This is the simplest export method, converting raw text to a basic
    /// JSON document structure.
    ///
    /// # Arguments
    ///
    /// * `text` - The text content to export
    ///
    /// # Returns
    ///
    /// A pretty-printed JSON string
    ///
    /// # Example
    ///
    /// ```
    /// use oxidize_pdf::ai::JsonExporter;
    ///
    /// let text = "This is a sample document.";
    /// let json = JsonExporter::export_simple(text).unwrap();
    /// assert!(json.contains("\"type\": \"document\""));
    /// assert!(json.contains("This is a sample document."));
    /// ```
    pub fn export_simple(text: &str) -> Result<String> {
        let doc = json!({
            "type": "document",
            "content": text
        });
        serde_json::to_string_pretty(&doc)
            .map_err(|e| crate::error::PdfError::SerializationError(e.to_string()))
    }

    /// Export text with metadata
    ///
    /// # Arguments
    ///
    /// * `text` - The text content
    /// * `metadata` - Document metadata
    ///
    /// # Returns
    ///
    /// A JSON string with metadata and content
    ///
    /// # Example
    ///
    /// ```
    /// use oxidize_pdf::ai::{JsonExporter, DocumentMetadata};
    ///
    /// let metadata = DocumentMetadata {
    ///     title: "My Document".to_string(),
    ///     page_count: 5,
    ///     created_at: Some("2025-10-13".to_string()),
    ///     author: Some("John Doe".to_string()),
    /// };
    ///
    /// let json = JsonExporter::export_with_metadata("Content here", &metadata).unwrap();
    /// assert!(json.contains("\"title\": \"My Document\""));
    /// ```
    pub fn export_with_metadata(text: &str, metadata: &DocumentMetadata) -> Result<String> {
        let mut meta_obj = json!({
            "title": metadata.title,
            "page_count": metadata.page_count
        });

        if let Some(ref created) = metadata.created_at {
            meta_obj["created_at"] = json!(created);
        }

        if let Some(ref author) = metadata.author {
            meta_obj["author"] = json!(author);
        }

        let doc = json!({
            "type": "document",
            "metadata": meta_obj,
            "content": text
        });

        serde_json::to_string_pretty(&doc)
            .map_err(|e| crate::error::PdfError::SerializationError(e.to_string()))
    }

    /// Export multi-page document to JSON
    ///
    /// # Arguments
    ///
    /// * `page_texts` - Vector of (page_number, text) tuples
    ///
    /// # Returns
    ///
    /// A JSON string with page-by-page structure
    ///
    /// # Example
    ///
    /// ```
    /// use oxidize_pdf::ai::JsonExporter;
    ///
    /// let pages = vec![
    ///     (1, "Page 1 content".to_string()),
    ///     (2, "Page 2 content".to_string()),
    /// ];
    ///
    /// let json = JsonExporter::export_pages(&pages).unwrap();
    /// assert!(json.contains("\"page_number\": 1"));
    /// assert!(json.contains("\"page_number\": 2"));
    /// ```
    pub fn export_pages(page_texts: &[(usize, String)]) -> Result<String> {
        let pages: Vec<Value> = page_texts
            .iter()
            .map(|(page_num, text)| {
                json!({
                    "page_number": page_num,
                    "content": text
                })
            })
            .collect();

        let doc = json!({
            "type": "document",
            "page_count": page_texts.len(),
            "pages": pages
        });

        serde_json::to_string_pretty(&doc)
            .map_err(|e| crate::error::PdfError::SerializationError(e.to_string()))
    }

    /// Export document chunks to JSON format
    ///
    /// This method is ideal for RAG (Retrieval Augmented Generation) pipelines
    /// where you need structured chunks with metadata for embedding and retrieval.
    ///
    /// Each chunk includes:
    /// - Content text
    /// - Token count
    /// - Page numbers where the chunk appears
    /// - Position metadata (character offsets, page range)
    /// - Confidence score
    /// - Whether sentence boundaries were respected
    ///
    /// # Arguments
    ///
    /// * `chunks` - Vector of document chunks from DocumentChunker
    ///
    /// # Returns
    ///
    /// A JSON string with structured chunk data
    ///
    /// # Example
    ///
    /// ```no_run
    /// use oxidize_pdf::ai::{DocumentChunker, JsonExporter};
    ///
    /// # fn main() -> oxidize_pdf::Result<()> {
    /// let chunker = DocumentChunker::new(512, 50);
    /// let chunks = chunker.chunk_text("Long document text...")?;
    /// let json = JsonExporter::export_with_chunks(&chunks)?;
    /// println!("{}", json);
    /// # Ok(())
    /// # }
    /// ```
    pub fn export_with_chunks(chunks: &[DocumentChunk]) -> Result<String> {
        let chunk_objects: Vec<Value> = chunks
            .iter()
            .map(|chunk| {
                json!({
                    "id": chunk.id,
                    "content": chunk.content,
                    "tokens": chunk.tokens,
                    "page_numbers": chunk.page_numbers,
                    "chunk_index": chunk.chunk_index,
                    "metadata": {
                        "position": {
                            "start_char": chunk.metadata.position.start_char,
                            "end_char": chunk.metadata.position.end_char,
                            "first_page": chunk.metadata.position.first_page,
                            "last_page": chunk.metadata.position.last_page
                        },
                        "confidence": chunk.metadata.confidence,
                        "sentence_boundary_respected": chunk.metadata.sentence_boundary_respected
                    }
                })
            })
            .collect();

        let doc = json!({
            "type": "chunked_document",
            "chunk_count": chunks.len(),
            "chunks": chunk_objects
        });

        serde_json::to_string_pretty(&doc)
            .map_err(|e| crate::error::PdfError::SerializationError(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_text_to_markdown() {
        let text = "hello world";
        let result = MarkdownExporter::export_text(text).unwrap();

        assert!(result.contains("# Document"), "Should have document header");
        assert!(
            result.contains("hello world"),
            "Should contain original text"
        );

        // Verify structure: header, blank line, content
        let lines: Vec<&str> = result.lines().collect();
        assert_eq!(lines[0], "# Document");
        assert_eq!(lines[1], "");
        assert_eq!(lines[2], "hello world");
    }

    #[test]
    fn test_empty_text() {
        let result = MarkdownExporter::export_text("").unwrap();
        assert!(
            result.contains("# Document"),
            "Should still have header for empty text"
        );
        assert_eq!(result, "# Document\n\n");
    }

    #[test]
    fn test_multiline_text() {
        let text = "First line\nSecond line\nThird line";
        let result = MarkdownExporter::export_text(text).unwrap();

        assert!(result.contains("First line"));
        assert!(result.contains("Second line"));
        assert!(result.contains("Third line"));
    }

    #[test]
    fn test_text_with_special_characters() {
        let text = "Text with # hash and * asterisk";
        let result = MarkdownExporter::export_text(text).unwrap();

        // Special characters should be preserved in content
        assert!(result.contains("# hash"));
        assert!(result.contains("* asterisk"));
    }

    #[test]
    fn test_markdown_exporter_creation() {
        let exporter = MarkdownExporter::new(MarkdownOptions {
            include_metadata: true,
            include_page_numbers: false,
        });

        assert!(exporter.options.include_metadata);
        assert!(!exporter.options.include_page_numbers);
    }

    #[test]
    fn test_markdown_exporter_default() {
        let exporter = MarkdownExporter::default();

        assert!(exporter.options.include_metadata);
        assert!(exporter.options.include_page_numbers);
    }

    #[test]
    fn test_markdown_with_metadata() {
        let metadata = DocumentMetadata {
            title: "Test Document".to_string(),
            page_count: 10,
            created_at: Some("2025-10-13".to_string()),
            author: Some("John Doe".to_string()),
        };

        let result = MarkdownExporter::export_with_metadata("Sample content", &metadata).unwrap();

        // Check YAML frontmatter structure
        assert!(result.starts_with("---\n"), "Should start with YAML marker");
        assert!(result.contains("title: Test Document"));
        assert!(result.contains("pages: 10"));
        assert!(result.contains("created: 2025-10-13"));
        assert!(result.contains("author: John Doe"));

        // Check content section
        assert!(result.contains("# Test Document"));
        assert!(result.contains("Sample content"));
    }

    #[test]
    fn test_metadata_with_special_characters() {
        let metadata = DocumentMetadata {
            title: "Test: Document #1".to_string(),
            page_count: 5,
            created_at: None,
            author: None,
        };

        let result = MarkdownExporter::export_with_metadata("Content", &metadata).unwrap();

        // Title with special characters should be quoted
        assert!(result.contains("title: \"Test: Document #1\""));
    }

    #[test]
    fn test_metadata_minimal() {
        let metadata = DocumentMetadata {
            title: "Simple".to_string(),
            page_count: 1,
            created_at: None,
            author: None,
        };

        let result = MarkdownExporter::export_with_metadata("Text", &metadata).unwrap();

        assert!(result.contains("title: Simple"));
        assert!(result.contains("pages: 1"));
        assert!(!result.contains("created:"));
        assert!(!result.contains("author:"));
    }

    #[test]
    fn test_document_metadata_default() {
        let metadata = DocumentMetadata::default();

        assert_eq!(metadata.title, "Untitled Document");
        assert_eq!(metadata.page_count, 0);
        assert!(metadata.created_at.is_none());
        assert!(metadata.author.is_none());
    }

    #[test]
    fn test_multipage_markdown() {
        let pages = vec![
            (1, "Content of page 1".to_string()),
            (2, "Content of page 2".to_string()),
            (3, "Content of page 3".to_string()),
        ];

        let result = MarkdownExporter::export_with_pages(&pages).unwrap();

        // Check document header
        assert!(result.starts_with("# Document\n\n"));

        // Check all page markers
        assert!(result.contains("**Page 1**"));
        assert!(result.contains("**Page 2**"));
        assert!(result.contains("**Page 3**"));

        // Check page content
        assert!(result.contains("Content of page 1"));
        assert!(result.contains("Content of page 2"));
        assert!(result.contains("Content of page 3"));

        // Check page separators (---)
        let separator_count = result.matches("\n---\n").count();
        assert_eq!(separator_count, 2, "Should have 2 separators for 3 pages");
    }

    #[test]
    fn test_page_numbers_correct() {
        let pages = vec![(1, "First".to_string()), (2, "Second".to_string())];

        let result = MarkdownExporter::export_with_pages(&pages).unwrap();

        // Verify page numbers appear in order
        let page1_pos = result.find("**Page 1**").unwrap();
        let page2_pos = result.find("**Page 2**").unwrap();
        assert!(page1_pos < page2_pos, "Page 1 should appear before Page 2");
    }

    #[test]
    fn test_single_page_no_separator() {
        let pages = vec![(1, "Single page content".to_string())];

        let result = MarkdownExporter::export_with_pages(&pages).unwrap();

        // Should not have separator for single page
        assert!(
            !result.contains("---"),
            "Single page should not have separator"
        );
        assert!(result.contains("**Page 1**"));
        assert!(result.contains("Single page content"));
    }

    #[test]
    fn test_empty_pages_list() {
        let pages: Vec<(usize, String)> = vec![];
        let result = MarkdownExporter::export_with_pages(&pages).unwrap();

        // Should just have document header
        assert_eq!(result, "# Document\n\n");
    }

    #[test]
    fn test_metadata_and_pages_combined() {
        let metadata = DocumentMetadata {
            title: "Test Document".to_string(),
            page_count: 2,
            created_at: Some("2025-10-13".to_string()),
            author: Some("John Doe".to_string()),
        };

        let pages = vec![
            (1, "Page one text".to_string()),
            (2, "Page two text".to_string()),
        ];

        let result = MarkdownExporter::export_with_metadata_and_pages(&pages, &metadata).unwrap();

        // Check YAML frontmatter
        assert!(result.starts_with("---\n"));
        assert!(result.contains("title: Test Document"));
        assert!(result.contains("pages: 2"));
        assert!(result.contains("created: 2025-10-13"));
        assert!(result.contains("author: John Doe"));

        // Check title header
        assert!(result.contains("# Test Document"));

        // Check pages
        assert!(result.contains("**Page 1**"));
        assert!(result.contains("**Page 2**"));
        assert!(result.contains("Page one text"));
        assert!(result.contains("Page two text"));
    }

    #[test]
    fn test_page_separator_format() {
        let pages = vec![(1, "A".to_string()), (2, "B".to_string())];

        let result = MarkdownExporter::export_with_pages(&pages).unwrap();

        // Check separator format is exactly "\n\n---\n\n"
        assert!(result.contains("\n\n---\n\n"));
    }

    // JSON Exporter Tests
    #[cfg(feature = "semantic")]
    #[test]
    fn test_basic_json_export() {
        let text = "Hello, world!";
        let result = JsonExporter::export_simple(text).unwrap();

        // Should be valid JSON
        let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();

        // Check structure
        assert_eq!(parsed["type"], "document");
        assert_eq!(parsed["content"], "Hello, world!");
    }

    #[cfg(feature = "semantic")]
    #[test]
    fn test_json_parsing() {
        let text = "Sample content";
        let json = JsonExporter::export_simple(text).unwrap();

        // Parse back the JSON
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

        assert!(parsed.is_object());
        assert_eq!(parsed["type"], "document");
        assert_eq!(parsed["content"], "Sample content");
    }

    #[cfg(feature = "semantic")]
    #[test]
    fn test_json_with_metadata() {
        let metadata = DocumentMetadata {
            title: "Test Doc".to_string(),
            page_count: 10,
            created_at: Some("2025-10-13".to_string()),
            author: Some("Jane Doe".to_string()),
        };

        let json = JsonExporter::export_with_metadata("Content", &metadata).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

        // Check metadata structure
        assert_eq!(parsed["metadata"]["title"], "Test Doc");
        assert_eq!(parsed["metadata"]["page_count"], 10);
        assert_eq!(parsed["metadata"]["created_at"], "2025-10-13");
        assert_eq!(parsed["metadata"]["author"], "Jane Doe");
        assert_eq!(parsed["content"], "Content");
    }

    #[cfg(feature = "semantic")]
    #[test]
    fn test_json_pages_export() {
        let pages = vec![
            (1, "Page 1 text".to_string()),
            (2, "Page 2 text".to_string()),
            (3, "Page 3 text".to_string()),
        ];

        let json = JsonExporter::export_pages(&pages).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

        // Check structure
        assert_eq!(parsed["type"], "document");
        assert_eq!(parsed["page_count"], 3);

        // Check pages array
        let pages_array = parsed["pages"].as_array().unwrap();
        assert_eq!(pages_array.len(), 3);

        assert_eq!(pages_array[0]["page_number"], 1);
        assert_eq!(pages_array[0]["content"], "Page 1 text");

        assert_eq!(pages_array[1]["page_number"], 2);
        assert_eq!(pages_array[1]["content"], "Page 2 text");

        assert_eq!(pages_array[2]["page_number"], 3);
        assert_eq!(pages_array[2]["content"], "Page 3 text");
    }

    #[cfg(feature = "semantic")]
    #[test]
    fn test_json_exporter_options() {
        let exporter = JsonExporter::new(JsonOptions {
            pretty_print: false,
            include_chunks: false,
        });

        let result = exporter.export("test").unwrap();

        // Non-pretty print should not have newlines
        assert!(!result.contains('\n'));
    }

    #[cfg(feature = "semantic")]
    #[test]
    fn test_json_pretty_print() {
        let exporter = JsonExporter::new(JsonOptions {
            pretty_print: true,
            include_chunks: false,
        });

        let result = exporter.export("test").unwrap();

        // Pretty print should have newlines and indentation
        assert!(result.contains('\n'));
        assert!(result.contains("  ")); // Indentation
    }

    #[cfg(feature = "semantic")]
    #[test]
    fn test_json_empty_pages() {
        let pages: Vec<(usize, String)> = vec![];
        let json = JsonExporter::export_pages(&pages).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed["page_count"], 0);
        assert_eq!(parsed["pages"].as_array().unwrap().len(), 0);
    }

    // JSON Chunks Export Tests
    #[cfg(feature = "semantic")]
    #[test]
    fn test_export_with_chunks_basic() {
        use crate::ai::chunking::{ChunkMetadata, ChunkPosition};

        let chunks = vec![
            DocumentChunk {
                id: "chunk_0".to_string(),
                content: "First chunk content".to_string(),
                tokens: 10,
                page_numbers: vec![1],
                chunk_index: 0,
                metadata: ChunkMetadata {
                    position: ChunkPosition {
                        start_char: 0,
                        end_char: 100,
                        first_page: 1,
                        last_page: 1,
                    },
                    confidence: 1.0,
                    sentence_boundary_respected: true,
                },
            },
            DocumentChunk {
                id: "chunk_1".to_string(),
                content: "Second chunk content".to_string(),
                tokens: 12,
                page_numbers: vec![1, 2],
                chunk_index: 1,
                metadata: ChunkMetadata {
                    position: ChunkPosition {
                        start_char: 90,
                        end_char: 200,
                        first_page: 1,
                        last_page: 2,
                    },
                    confidence: 0.95,
                    sentence_boundary_respected: false,
                },
            },
        ];

        let json = JsonExporter::export_with_chunks(&chunks).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

        // Validate structure
        assert_eq!(parsed["type"], "chunked_document");
        assert_eq!(parsed["chunk_count"], 2);

        let chunks_array = parsed["chunks"].as_array().unwrap();
        assert_eq!(chunks_array.len(), 2);

        // Validate first chunk
        assert_eq!(chunks_array[0]["id"], "chunk_0");
        assert_eq!(chunks_array[0]["tokens"], 10);
        assert_eq!(chunks_array[0]["content"], "First chunk content");
        assert_eq!(chunks_array[0]["page_numbers"][0], 1);
        assert_eq!(chunks_array[0]["chunk_index"], 0);
        assert_eq!(chunks_array[0]["metadata"]["confidence"], 1.0);
        assert_eq!(
            chunks_array[0]["metadata"]["sentence_boundary_respected"],
            true
        );
        assert_eq!(chunks_array[0]["metadata"]["position"]["start_char"], 0);
        assert_eq!(chunks_array[0]["metadata"]["position"]["end_char"], 100);
        assert_eq!(chunks_array[0]["metadata"]["position"]["first_page"], 1);
        assert_eq!(chunks_array[0]["metadata"]["position"]["last_page"], 1);

        // Validate second chunk
        assert_eq!(chunks_array[1]["id"], "chunk_1");
        assert_eq!(chunks_array[1]["chunk_index"], 1);
        assert_eq!(chunks_array[1]["tokens"], 12);
        assert_eq!(chunks_array[1]["page_numbers"].as_array().unwrap().len(), 2);
        // Use approximate comparison for f32 values serialized through JSON
        let confidence = chunks_array[1]["metadata"]["confidence"].as_f64().unwrap();
        assert!(
            (confidence - 0.95).abs() < 0.01,
            "Confidence should be approximately 0.95, got {}",
            confidence
        );
        assert_eq!(
            chunks_array[1]["metadata"]["sentence_boundary_respected"],
            false
        );
    }

    #[cfg(feature = "semantic")]
    #[test]
    fn test_export_with_chunks_empty() {
        let chunks: Vec<DocumentChunk> = vec![];
        let json = JsonExporter::export_with_chunks(&chunks).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed["type"], "chunked_document");
        assert_eq!(parsed["chunk_count"], 0);
        assert_eq!(parsed["chunks"].as_array().unwrap().len(), 0);
    }

    #[cfg(feature = "semantic")]
    #[test]
    fn test_export_with_chunks_position_metadata() {
        use crate::ai::chunking::{ChunkMetadata, ChunkPosition};

        // Test that all position metadata is correctly serialized
        let chunk = DocumentChunk {
            id: "test_chunk".to_string(),
            content: "Test content for position tracking".to_string(),
            tokens: 5,
            page_numbers: vec![5, 6, 7],
            chunk_index: 10,
            metadata: ChunkMetadata {
                position: ChunkPosition {
                    start_char: 1000,
                    end_char: 2000,
                    first_page: 5,
                    last_page: 7,
                },
                confidence: 0.85,
                sentence_boundary_respected: false,
            },
        };

        let json = JsonExporter::export_with_chunks(&[chunk]).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed["chunk_count"], 1);

        let chunk_obj = &parsed["chunks"][0];
        assert_eq!(chunk_obj["id"], "test_chunk");
        assert_eq!(chunk_obj["tokens"], 5);
        assert_eq!(chunk_obj["chunk_index"], 10);
        assert_eq!(chunk_obj["content"], "Test content for position tracking");

        // Validate page numbers array
        let pages = chunk_obj["page_numbers"].as_array().unwrap();
        assert_eq!(pages.len(), 3);
        assert_eq!(pages[0], 5);
        assert_eq!(pages[1], 6);
        assert_eq!(pages[2], 7);

        // Validate position metadata
        let pos = &chunk_obj["metadata"]["position"];
        assert_eq!(pos["start_char"], 1000);
        assert_eq!(pos["end_char"], 2000);
        assert_eq!(pos["first_page"], 5);
        assert_eq!(pos["last_page"], 7);

        // Validate other metadata (use approximate comparison for f32)
        let confidence = chunk_obj["metadata"]["confidence"].as_f64().unwrap();
        assert!(
            (confidence - 0.85).abs() < 0.01,
            "Confidence should be approximately 0.85, got {}",
            confidence
        );
        assert_eq!(chunk_obj["metadata"]["sentence_boundary_respected"], false);
    }

    #[cfg(feature = "semantic")]
    #[test]
    fn test_export_with_chunks_multiple_pages() {
        use crate::ai::chunking::{ChunkMetadata, ChunkPosition};

        // Test chunk spanning multiple pages
        let chunk = DocumentChunk {
            id: "multipage".to_string(),
            content: "Content spanning pages".to_string(),
            tokens: 20,
            page_numbers: vec![2, 3, 4],
            chunk_index: 0,
            metadata: ChunkMetadata {
                position: ChunkPosition {
                    start_char: 500,
                    end_char: 1500,
                    first_page: 2,
                    last_page: 4,
                },
                confidence: 1.0,
                sentence_boundary_respected: true,
            },
        };

        let json = JsonExporter::export_with_chunks(&[chunk]).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

        let chunk_obj = &parsed["chunks"][0];
        let pages = chunk_obj["page_numbers"].as_array().unwrap();

        assert_eq!(pages.len(), 3);
        assert_eq!(chunk_obj["metadata"]["position"]["first_page"], 2);
        assert_eq!(chunk_obj["metadata"]["position"]["last_page"], 4);
    }
}
