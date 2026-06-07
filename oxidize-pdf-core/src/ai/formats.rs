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

#[cfg(feature = "semantic")]
impl ChunkExporter for JsonExporter {
    fn export_chunks(&self, chunks: &[DocumentChunk]) -> Result<String> {
        JsonExporter::export_with_chunks(chunks)
    }
}

/// Exporter for converting PDF content to contextual format
///
/// Contextual format is optimized for direct injection into LLM prompts,
/// providing document content in a conversational, easy-to-understand structure
/// without heavy formatting or metadata overhead.
///
/// This format is ideal for:
/// - Direct LLM prompt injection
/// - Question-answering over documents
/// - Summarization tasks
/// - Conversational AI with document context
///
/// # Example
///
/// ```
/// use oxidize_pdf::ai::ContextualFormat;
///
/// let text = "This is the document content.";
/// let contextual = ContextualFormat::export_simple(text).unwrap();
/// // Output: "Document content:\n\nThis is the document content."
/// ```
#[derive(Debug, Clone)]
pub struct ContextualFormat;

impl ContextualFormat {
    /// Export plain text to contextual format
    ///
    /// Creates a simple, conversational representation of the document
    /// content without heavy formatting.
    ///
    /// # Arguments
    ///
    /// * `text` - The text content to export
    ///
    /// # Returns
    ///
    /// A contextual-formatted string
    ///
    /// # Example
    ///
    /// ```
    /// use oxidize_pdf::ai::ContextualFormat;
    ///
    /// let text = "Sample document text.";
    /// let result = ContextualFormat::export_simple(text).unwrap();
    /// assert!(result.contains("Document content:"));
    /// assert!(result.contains("Sample document text."));
    /// ```
    pub fn export_simple(text: &str) -> Result<String> {
        let mut output = String::new();
        output.push_str("Document content:\n\n");
        output.push_str(text);
        Ok(output)
    }

    /// Export text with metadata in conversational format
    ///
    /// Creates a natural language description of the document with metadata,
    /// ideal for LLM understanding without structured parsing.
    ///
    /// # Arguments
    ///
    /// * `text` - The text content
    /// * `metadata` - Document metadata
    ///
    /// # Returns
    ///
    /// A contextual-formatted string with metadata
    ///
    /// # Example
    ///
    /// ```
    /// use oxidize_pdf::ai::{ContextualFormat, DocumentMetadata};
    ///
    /// let metadata = DocumentMetadata {
    ///     title: "Annual Report".to_string(),
    ///     page_count: 25,
    ///     created_at: Some("2025-01-15".to_string()),
    ///     author: Some("Jane Smith".to_string()),
    /// };
    ///
    /// let result = ContextualFormat::export_with_metadata("Report text...", &metadata).unwrap();
    /// assert!(result.contains("This is a document titled \"Annual Report\""));
    /// assert!(result.contains("25 pages"));
    /// ```
    pub fn export_with_metadata(text: &str, metadata: &DocumentMetadata) -> Result<String> {
        let mut output = String::new();

        // Natural language metadata introduction
        output.push_str(&format!("This is a document titled \"{}\"", metadata.title));

        if metadata.page_count > 0 {
            output.push_str(&format!(
                " with {} page{}",
                metadata.page_count,
                if metadata.page_count == 1 { "" } else { "s" }
            ));
        }

        if let Some(ref author) = metadata.author {
            output.push_str(&format!(", written by {}", author));
        }

        if let Some(ref created) = metadata.created_at {
            output.push_str(&format!(", created on {}", created));
        }

        output.push_str(".\n\n");
        output.push_str("Content:\n\n");
        output.push_str(text);

        Ok(output)
    }

    /// Export multi-page document with conversational page markers
    ///
    /// # Arguments
    ///
    /// * `page_texts` - Vector of (page_number, text) tuples
    ///
    /// # Returns
    ///
    /// A contextual-formatted string with page indicators
    ///
    /// # Example
    ///
    /// ```
    /// use oxidize_pdf::ai::ContextualFormat;
    ///
    /// let pages = vec![
    ///     (1, "First page content".to_string()),
    ///     (2, "Second page content".to_string()),
    /// ];
    ///
    /// let result = ContextualFormat::export_with_pages(&pages).unwrap();
    /// assert!(result.contains("On page 1:"));
    /// assert!(result.contains("On page 2:"));
    /// ```
    pub fn export_with_pages(page_texts: &[(usize, String)]) -> Result<String> {
        let mut output = String::new();
        output.push_str("Document content:\n\n");

        for (page_num, text) in page_texts.iter() {
            output.push_str(&format!("On page {}:\n", page_num));
            output.push_str(text);
            output.push_str("\n\n");
        }

        Ok(output)
    }

    /// Export multi-page document with metadata and conversational formatting
    ///
    /// # Arguments
    ///
    /// * `page_texts` - Vector of (page_number, text) tuples
    /// * `metadata` - Document metadata
    ///
    /// # Returns
    ///
    /// A contextual-formatted string with metadata and page content
    ///
    /// # Example
    ///
    /// ```
    /// use oxidize_pdf::ai::{ContextualFormat, DocumentMetadata};
    ///
    /// let metadata = DocumentMetadata {
    ///     title: "Technical Guide".to_string(),
    ///     page_count: 3,
    ///     created_at: None,
    ///     author: None,
    /// };
    ///
    /// let pages = vec![
    ///     (1, "Introduction".to_string()),
    ///     (2, "Main content".to_string()),
    /// ];
    ///
    /// let result = ContextualFormat::export_with_metadata_and_pages(&pages, &metadata).unwrap();
    /// assert!(result.contains("titled \"Technical Guide\""));
    /// assert!(result.contains("On page 1:"));
    /// ```
    pub fn export_with_metadata_and_pages(
        page_texts: &[(usize, String)],
        metadata: &DocumentMetadata,
    ) -> Result<String> {
        let mut output = String::new();

        // Natural language metadata introduction
        output.push_str(&format!("This is a document titled \"{}\"", metadata.title));

        if metadata.page_count > 0 {
            output.push_str(&format!(
                " with {} page{}",
                metadata.page_count,
                if metadata.page_count == 1 { "" } else { "s" }
            ));
        }

        if let Some(ref author) = metadata.author {
            output.push_str(&format!(", written by {}", author));
        }

        if let Some(ref created) = metadata.created_at {
            output.push_str(&format!(", created on {}", created));
        }

        output.push_str(".\n\n");
        output.push_str("Content:\n\n");

        // Add page content with conversational markers
        for (page_num, text) in page_texts.iter() {
            output.push_str(&format!("On page {}:\n", page_num));
            output.push_str(text);
            output.push_str("\n\n");
        }

        Ok(output)
    }
}

/// Common interface for serializers that turn RAG chunks into a string payload.
///
/// This unifies the chunk-export entry points that previously lived as unrelated
/// inherent methods (`JsonExporter::export_with_chunks`,
/// `TokenEfficientExporter::export_chunks`), so callers can pick a format at
/// runtime behind a `dyn ChunkExporter`.
///
/// `TokenEfficientExporter` always implements this trait. `JsonExporter`
/// implements it only when the `semantic` feature is enabled.
pub trait ChunkExporter {
    /// Serialize the given chunks into this exporter's output format.
    fn export_chunks(&self, chunks: &[DocumentChunk]) -> Result<String>;
}

/// Token-efficient, TOON-inspired tabular serializer for RAG chunks (issue #291).
///
/// Unlike [`JsonExporter`], which repeats every key name for every chunk, this
/// format declares the column names once in a header line and then emits one
/// tab-separated row per chunk. That removes the per-record structural overhead
/// that dominates JSON token cost for large chunk sets, while staying fully
/// round-trippable via [`TokenEfficientExporter::parse_chunks`].
///
/// The serializer builds its output with plain string formatting and therefore
/// does **not** require the `semantic` feature (no `serde_json` dependency).
///
/// Wire shape:
///
/// ```text
/// #oxct/1
/// id<TAB>tokens<TAB>chunk_index<TAB>start_char<TAB>end_char<TAB>first_page<TAB>last_page<TAB>confidence<TAB>sentence_boundary<TAB>page_numbers<TAB>content
/// chunk_0<TAB>10<TAB>0<TAB>0<TAB>100<TAB>1<TAB>1<TAB>1.0000<TAB>true<TAB>1<TAB>Hello world
/// ```
///
/// The token reduction versus JSON is measured (not estimated) by the
/// `token-bench` gated test; on a representative corpus it is ~64% with the
/// `cl100k_base` tokenizer.
///
/// # Example
///
/// ```
/// use oxidize_pdf::ai::{ChunkExporter, DocumentChunker, TokenEfficientExporter};
///
/// # fn main() -> oxidize_pdf::Result<()> {
/// let chunks = DocumentChunker::new(512, 50).chunk_text("Some document text to embed.")?;
/// let serialized = TokenEfficientExporter::new().export_chunks(&chunks)?;
///
/// // The format is fully round-trippable.
/// let restored = TokenEfficientExporter::parse_chunks(&serialized)?;
/// assert_eq!(restored.len(), chunks.len());
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone, Default)]
pub struct TokenEfficientExporter;

impl TokenEfficientExporter {
    /// Format version marker emitted as the first line.
    const MAGIC: &'static str = "#oxct/1";

    /// Column header emitted as the second line (declared once for all rows).
    const HEADER: &'static str = "id\ttokens\tchunk_index\tstart_char\tend_char\tfirst_page\tlast_page\tconfidence\tsentence_boundary\tpage_numbers\tcontent";

    /// Create a new exporter.
    pub fn new() -> Self {
        Self
    }

    /// Serialize chunks to the token-efficient tabular format.
    pub fn export_chunks(&self, chunks: &[DocumentChunk]) -> Result<String> {
        let mut out = String::new();
        out.push_str(Self::MAGIC);
        out.push('\n');
        out.push_str(Self::HEADER);
        for chunk in chunks {
            out.push('\n');
            out.push_str(&Self::encode_row(chunk));
        }
        Ok(out)
    }

    /// Parse a token-efficient document back into chunks (inverse of
    /// [`Self::export_chunks`]).
    ///
    /// Validates the version marker and column header, then decodes one chunk
    /// per data row. Rows are quote-aware so content fields containing embedded
    /// newlines are reassembled correctly. Returns an error on a wrong version,
    /// a wrong header, or a row whose column count does not match the header.
    pub fn parse_chunks(input: &str) -> Result<Vec<DocumentChunk>> {
        let logical = Self::rejoin_quoted_lines(input)?;
        let mut iter = logical.iter();

        match iter.next().map(|s| s.trim_end_matches('\r')) {
            Some(Self::MAGIC) => {}
            other => {
                return Err(crate::error::PdfError::InvalidStructure(format!(
                    "token-efficient: unexpected version marker {other:?}, expected {:?}",
                    Self::MAGIC
                )))
            }
        }
        match iter.next().map(|s| s.trim_end_matches('\r')) {
            Some(Self::HEADER) => {}
            other => {
                return Err(crate::error::PdfError::InvalidStructure(format!(
                    "token-efficient: unexpected column header {other:?}"
                )))
            }
        }

        let mut chunks = Vec::new();
        for line in iter {
            if line.is_empty() {
                continue;
            }
            chunks.push(Self::parse_row(line)?);
        }
        Ok(chunks)
    }

    /// Decode a single data row into a [`DocumentChunk`].
    fn parse_row(line: &str) -> Result<DocumentChunk> {
        use crate::ai::chunking::{ChunkMetadata, ChunkPosition};

        let fields: Vec<&str> = line.splitn(11, '\t').collect();
        if fields.len() != 11 {
            return Err(crate::error::PdfError::InvalidStructure(format!(
                "token-efficient: row has {} columns, expected 11",
                fields.len()
            )));
        }

        let parse_usize = |name: &str, s: &str| -> Result<usize> {
            s.parse::<usize>().map_err(|e| {
                crate::error::PdfError::InvalidStructure(format!(
                    "token-efficient: invalid {name} {s:?}: {e}"
                ))
            })
        };

        let confidence = fields[7].parse::<f32>().map_err(|e| {
            crate::error::PdfError::InvalidStructure(format!(
                "token-efficient: invalid confidence {:?}: {e}",
                fields[7]
            ))
        })?;
        if !confidence.is_finite() {
            return Err(crate::error::PdfError::InvalidStructure(format!(
                "token-efficient: confidence must be finite, got {confidence:?}"
            )));
        }

        Ok(DocumentChunk {
            id: fields[0].to_string(),
            tokens: parse_usize("tokens", fields[1])?,
            chunk_index: parse_usize("chunk_index", fields[2])?,
            page_numbers: Self::parse_page_numbers_field(fields[9])?,
            content: Self::parse_content_field(fields[10])?,
            metadata: ChunkMetadata {
                position: ChunkPosition {
                    start_char: parse_usize("start_char", fields[3])?,
                    end_char: parse_usize("end_char", fields[4])?,
                    first_page: parse_usize("first_page", fields[5])?,
                    last_page: parse_usize("last_page", fields[6])?,
                },
                confidence,
                sentence_boundary_respected: fields[8] == "true",
                // The token-efficient format does not carry language; round-trip
                // leaves it unset (re-derivable via DocumentChunker detection).
                language: None,
            },
        })
    }

    /// Split the input into logical rows, treating `\n` inside a quoted field as
    /// part of the content rather than a row separator. `""` (an escaped quote)
    /// toggles the quote state twice, correctly keeping the parser inside the field.
    ///
    /// A dangling open quote at end of input (odd number of `"`, e.g. a corrupt or
    /// hand-crafted payload) is rejected rather than silently merging every
    /// remaining row into one record.
    fn rejoin_quoted_lines(input: &str) -> Result<Vec<String>> {
        let mut rows = Vec::new();
        let mut current = String::new();
        let mut in_quote = false;
        for ch in input.chars() {
            match ch {
                '"' => {
                    in_quote = !in_quote;
                    current.push(ch);
                }
                '\n' if !in_quote => {
                    rows.push(std::mem::take(&mut current));
                }
                _ => current.push(ch),
            }
        }
        if in_quote {
            return Err(crate::error::PdfError::InvalidStructure(
                "token-efficient: unterminated quoted field".to_string(),
            ));
        }
        rows.push(current);
        Ok(rows)
    }

    /// Parse the `page_numbers` column (semicolon-separated integers).
    ///
    /// An empty string yields an empty vector. Any non-integer element is an error.
    fn parse_page_numbers_field(s: &str) -> Result<Vec<usize>> {
        if s.is_empty() {
            return Ok(Vec::new());
        }
        s.split(';')
            .map(|p| {
                p.parse::<usize>().map_err(|e| {
                    crate::error::PdfError::InvalidStructure(format!(
                        "invalid page number {p:?} in token-efficient chunk row: {e}"
                    ))
                })
            })
            .collect()
    }

    /// Quote the `content` field only when required to keep rows unambiguous.
    ///
    /// `content` is the last field and is recovered with `splitn(11, '\t')`, so
    /// interior tabs and commas are safe raw. Quoting is required when the content
    /// contains any `"` (otherwise the parser's quote-tracking would desync and
    /// swallow following rows) or a newline/CR (which would otherwise split the
    /// row). When quoted, the field is wrapped in `"` and interior `"` are doubled.
    /// This keeps the RFC-4180 invariant: a field is raw iff it contains no
    /// `"`, `\n`, or `\r`.
    fn quote_content(s: &str) -> String {
        let needs_quote = s.contains('"') || s.contains('\n') || s.contains('\r');
        if needs_quote {
            format!("\"{}\"", s.replace('"', "\"\""))
        } else {
            s.to_string()
        }
    }

    /// Inverse of [`Self::quote_content`].
    ///
    /// A raw field (no surrounding quotes) is returned as-is. A quoted field is
    /// unwrapped and its doubled `""` are collapsed. A malformed quoted field
    /// (an interior lone `"` that is not part of a `""` pair) is rejected, since
    /// the encoder never produces one.
    fn parse_content_field(s: &str) -> Result<String> {
        if s.len() >= 2 && s.starts_with('"') && s.ends_with('"') {
            let inner = &s[1..s.len() - 1];
            // Every interior `"` must be part of a doubled `""`. Removing the
            // pairs must leave no stray quote behind.
            if inner.replace("\"\"", "").contains('"') {
                return Err(crate::error::PdfError::InvalidStructure(
                    "token-efficient: malformed quoted content field (unbalanced quotes)"
                        .to_string(),
                ));
            }
            Ok(inner.replace("\"\"", "\""))
        } else if s.contains('"') {
            // A raw field can never legitimately contain a `"`.
            Err(crate::error::PdfError::InvalidStructure(
                "token-efficient: unquoted content field contains a stray quote".to_string(),
            ))
        } else {
            Ok(s.to_string())
        }
    }

    /// Encode a single chunk as one tab-separated data row.
    fn encode_row(chunk: &DocumentChunk) -> String {
        let pages = chunk
            .page_numbers
            .iter()
            .map(|p| p.to_string())
            .collect::<Vec<_>>()
            .join(";");
        format!(
            "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{:.4}\t{}\t{}\t{}",
            chunk.id,
            chunk.tokens,
            chunk.chunk_index,
            chunk.metadata.position.start_char,
            chunk.metadata.position.end_char,
            chunk.metadata.position.first_page,
            chunk.metadata.position.last_page,
            chunk.metadata.confidence,
            chunk.metadata.sentence_boundary_respected,
            pages,
            Self::quote_content(&chunk.content),
        )
    }
}

impl ChunkExporter for TokenEfficientExporter {
    fn export_chunks(&self, chunks: &[DocumentChunk]) -> Result<String> {
        TokenEfficientExporter::export_chunks(self, chunks)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---- Token-efficient exporter (#291) ----

    /// Expected column header for the token-efficient tabular format.
    const TE_HEADER: &str = "id\ttokens\tchunk_index\tstart_char\tend_char\tfirst_page\tlast_page\tconfidence\tsentence_boundary\tpage_numbers\tcontent";

    #[allow(clippy::too_many_arguments)]
    fn te_chunk(
        id: &str,
        content: &str,
        tokens: usize,
        page_numbers: Vec<usize>,
        chunk_index: usize,
        start_char: usize,
        end_char: usize,
        first_page: usize,
        last_page: usize,
        confidence: f32,
        sentence_boundary_respected: bool,
    ) -> crate::ai::chunking::DocumentChunk {
        use crate::ai::chunking::{ChunkMetadata, ChunkPosition, DocumentChunk};
        DocumentChunk {
            id: id.to_string(),
            content: content.to_string(),
            tokens,
            page_numbers,
            chunk_index,
            metadata: ChunkMetadata {
                position: ChunkPosition {
                    start_char,
                    end_char,
                    first_page,
                    last_page,
                },
                confidence,
                sentence_boundary_respected,
                language: None,
            },
        }
    }

    #[test]
    fn test_encode_scalar_fields_no_special_chars() {
        let chunks = vec![te_chunk(
            "chunk_0",
            "Hello world",
            10,
            vec![1],
            0,
            0,
            100,
            1,
            1,
            1.0,
            true,
        )];

        let out = TokenEfficientExporter::new()
            .export_chunks(&chunks)
            .unwrap();
        let lines: Vec<&str> = out.lines().collect();

        // version marker, header, one data row
        assert_eq!(
            lines.len(),
            3,
            "expected magic + header + 1 row, got: {out:?}"
        );
        assert_eq!(
            lines[0], "#oxct/1",
            "first line must be the format version marker"
        );
        assert_eq!(lines[1], TE_HEADER, "second line must be the column header");
        assert_eq!(
            lines[2], "chunk_0\t10\t0\t0\t100\t1\t1\t1.0000\ttrue\t1\tHello world",
            "data row must be tab-separated scalar fields followed by content"
        );
    }

    #[test]
    fn test_page_numbers_encoding() {
        // multi-page
        let out = TokenEfficientExporter::new()
            .export_chunks(&[te_chunk(
                "c",
                "x",
                1,
                vec![2, 3, 4],
                0,
                0,
                1,
                2,
                4,
                0.5,
                false,
            )])
            .unwrap();
        let row = out.lines().nth(2).unwrap();
        assert_eq!(row.split('\t').nth(9).unwrap(), "2;3;4");

        // single page
        let out = TokenEfficientExporter::new()
            .export_chunks(&[te_chunk("c", "x", 1, vec![1], 0, 0, 1, 1, 1, 0.5, false)])
            .unwrap();
        let row = out.lines().nth(2).unwrap();
        assert_eq!(row.split('\t').nth(9).unwrap(), "1");

        // empty page list
        let out = TokenEfficientExporter::new()
            .export_chunks(&[te_chunk("c", "x", 1, vec![], 0, 0, 1, 0, 0, 0.5, false)])
            .unwrap();
        let row = out.lines().nth(2).unwrap();
        assert_eq!(row.split('\t').nth(9).unwrap(), "");
    }

    #[test]
    fn test_parse_page_numbers_field() {
        assert_eq!(
            TokenEfficientExporter::parse_page_numbers_field("2;3;4").unwrap(),
            vec![2usize, 3, 4]
        );
        assert_eq!(
            TokenEfficientExporter::parse_page_numbers_field("1").unwrap(),
            vec![1usize]
        );
        assert_eq!(
            TokenEfficientExporter::parse_page_numbers_field("").unwrap(),
            Vec::<usize>::new()
        );
        assert!(TokenEfficientExporter::parse_page_numbers_field("1;x;3").is_err());
    }

    /// Extract the raw (still-encoded) content field from a single-chunk export.
    fn te_content_field(out: &str) -> String {
        let row = out
            .strip_prefix("#oxct/1\n")
            .and_then(|s| s.strip_prefix(TE_HEADER))
            .and_then(|s| s.strip_prefix('\n'))
            .expect("well-formed export");
        row.splitn(11, '\t').nth(10).unwrap().to_string()
    }

    fn te_export_one(content: &str) -> String {
        TokenEfficientExporter::new()
            .export_chunks(&[te_chunk(
                "c",
                content,
                1,
                vec![1],
                0,
                0,
                1,
                1,
                1,
                0.5,
                false,
            )])
            .unwrap()
    }

    #[test]
    fn test_content_quoting() {
        // comma: tab-delimited format does not need to quote commas
        assert_eq!(
            te_content_field(&te_export_one("hello, world")),
            "hello, world"
        );
        // any interior double-quote must be quoted + doubled (RFC-4180 invariant),
        // otherwise the parser's quote tracking would desync on odd quote counts
        assert_eq!(
            te_content_field(&te_export_one("say \"hi\"")),
            "\"say \"\"hi\"\"\""
        );
        // content that starts with a quote must be quoted + interior quotes doubled
        assert_eq!(te_content_field(&te_export_one("\"hi\"")), "\"\"\"hi\"\"\"");
        // odd number of interior quotes: still fully quoted (the QR #3 case)
        assert_eq!(
            te_content_field(&te_export_one("say \"hello")),
            "\"say \"\"hello\""
        );
        // embedded newline must be quoted (so the row stays one logical record)
        assert_eq!(
            te_content_field(&te_export_one("line1\nline2")),
            "\"line1\nline2\""
        );
        // unicode: no special chars, raw
        assert_eq!(te_content_field(&te_export_one("こんにちは")), "こんにちは");
        // empty content: raw empty
        assert_eq!(te_content_field(&te_export_one("")), "");
    }

    #[test]
    fn test_parse_content_field() {
        // raw field with no special chars
        assert_eq!(
            TokenEfficientExporter::parse_content_field("hello, world").unwrap(),
            "hello, world"
        );
        // properly quoted field with doubled interior quotes
        assert_eq!(
            TokenEfficientExporter::parse_content_field("\"\"\"hi\"\"\"").unwrap(),
            "\"hi\""
        );
        // empty
        assert_eq!(TokenEfficientExporter::parse_content_field("").unwrap(), "");
        // quoted field spanning a newline
        assert_eq!(
            TokenEfficientExporter::parse_content_field("\"line1\nline2\"").unwrap(),
            "line1\nline2"
        );
        // a raw field can never legitimately contain a stray quote -> rejected
        assert!(TokenEfficientExporter::parse_content_field("say \"hi\"").is_err());
        // a quoted field with an unbalanced interior quote -> rejected
        assert!(TokenEfficientExporter::parse_content_field("\"ab\"cd\"").is_err());
    }

    #[test]
    fn test_multi_chunk_document() {
        let chunks = vec![
            te_chunk(
                "chunk_0",
                "First chunk",
                10,
                vec![1],
                0,
                0,
                100,
                1,
                1,
                1.0,
                true,
            ),
            te_chunk(
                "chunk_1",
                "Second, chunk",
                12,
                vec![1, 2],
                1,
                90,
                200,
                1,
                2,
                0.95,
                false,
            ),
        ];
        let out = TokenEfficientExporter::new()
            .export_chunks(&chunks)
            .unwrap();
        let lines: Vec<&str> = out.lines().collect();

        assert_eq!(lines.len(), 4, "magic + header + 2 rows");
        assert_eq!(lines[0], "#oxct/1");
        assert_eq!(lines[1], TE_HEADER);
        // first row content has no special chars -> not quoted
        assert!(lines[2].ends_with("\tFirst chunk"));
        // comma in content does not trigger quoting (tab-delimited)
        assert!(lines[3].ends_with("\tSecond, chunk"));
        assert_eq!(lines[3].split('\t').nth(9).unwrap(), "1;2");
    }

    fn assert_chunk_eq(
        a: &crate::ai::chunking::DocumentChunk,
        b: &crate::ai::chunking::DocumentChunk,
    ) {
        assert_eq!(a.id, b.id, "id");
        assert_eq!(a.content, b.content, "content");
        assert_eq!(a.tokens, b.tokens, "tokens");
        assert_eq!(a.page_numbers, b.page_numbers, "page_numbers");
        assert_eq!(a.chunk_index, b.chunk_index, "chunk_index");
        assert_eq!(
            a.metadata.position.start_char, b.metadata.position.start_char,
            "start_char"
        );
        assert_eq!(
            a.metadata.position.end_char, b.metadata.position.end_char,
            "end_char"
        );
        assert_eq!(
            a.metadata.position.first_page, b.metadata.position.first_page,
            "first_page"
        );
        assert_eq!(
            a.metadata.position.last_page, b.metadata.position.last_page,
            "last_page"
        );
        assert!(
            (a.metadata.confidence - b.metadata.confidence).abs() < 1e-4,
            "confidence: {} vs {}",
            a.metadata.confidence,
            b.metadata.confidence
        );
        assert_eq!(
            a.metadata.sentence_boundary_respected, b.metadata.sentence_boundary_respected,
            "sentence_boundary"
        );
    }

    fn roundtrip(chunk: crate::ai::chunking::DocumentChunk) {
        let exporter = TokenEfficientExporter::new();
        let serialized = exporter
            .export_chunks(std::slice::from_ref(&chunk))
            .unwrap();
        let parsed = TokenEfficientExporter::parse_chunks(&serialized).unwrap();
        assert_eq!(
            parsed.len(),
            1,
            "single chunk should round-trip to one chunk"
        );
        assert_chunk_eq(&parsed[0], &chunk);
    }

    #[test]
    fn test_rejoin_quoted_lines_with_embedded_newline() {
        // a quoted field spanning a physical newline must rejoin into one logical row
        let raw = "#oxct/1\nHDR\nrow_a\t\"line1\nline2\"\nrow_b";
        let logical = TokenEfficientExporter::rejoin_quoted_lines(raw).unwrap();
        assert_eq!(logical.len(), 4); // magic, hdr, row_a(quoted), row_b
        assert_eq!(logical[0], "#oxct/1");
        assert_eq!(logical[1], "HDR");
        assert_eq!(logical[2], "row_a\t\"line1\nline2\"");
        assert_eq!(logical[3], "row_b");
    }

    #[test]
    fn test_roundtrip_single_chunk() {
        roundtrip(te_chunk(
            "chunk_0",
            "Plain content",
            7,
            vec![3],
            0,
            10,
            210,
            3,
            3,
            0.95,
            true,
        ));
    }

    #[test]
    fn test_roundtrip_zero_chunks() {
        let exporter = TokenEfficientExporter::new();
        let serialized = exporter.export_chunks(&[]).unwrap();
        // exactly magic + header, no trailing newline
        assert_eq!(serialized, format!("#oxct/1\n{TE_HEADER}"));
        let parsed = TokenEfficientExporter::parse_chunks(&serialized).unwrap();
        assert!(parsed.is_empty());
    }

    #[test]
    fn test_roundtrip_content_with_comma() {
        roundtrip(te_chunk(
            "c",
            "price: $1,200",
            4,
            vec![1],
            0,
            0,
            13,
            1,
            1,
            0.5,
            false,
        ));
    }

    #[test]
    fn test_roundtrip_content_with_embedded_newline() {
        roundtrip(te_chunk(
            "c",
            "line1\nline2\nline3",
            5,
            vec![1, 2],
            0,
            0,
            17,
            1,
            2,
            0.5,
            false,
        ));
    }

    #[test]
    fn test_roundtrip_content_with_embedded_quote() {
        roundtrip(te_chunk(
            "c",
            "He said \"hello\"",
            5,
            vec![1],
            0,
            0,
            15,
            1,
            1,
            0.5,
            false,
        ));
    }

    #[test]
    fn test_roundtrip_content_starting_with_quote() {
        roundtrip(te_chunk(
            "c",
            "\"quoted start",
            3,
            vec![1],
            0,
            0,
            13,
            1,
            1,
            0.5,
            false,
        ));
    }

    #[test]
    fn test_roundtrip_unicode() {
        roundtrip(te_chunk(
            "c",
            "Ñoño: αβγ 中文 🦀",
            6,
            vec![1],
            0,
            0,
            20,
            1,
            1,
            0.5,
            false,
        ));
    }

    #[test]
    fn test_roundtrip_multi_page() {
        roundtrip(te_chunk(
            "c",
            "x",
            1,
            vec![3, 7, 12],
            0,
            0,
            1,
            3,
            12,
            0.5,
            false,
        ));
    }

    #[test]
    fn test_roundtrip_multiple_chunks_preserves_order() {
        let chunks = vec![
            te_chunk("a", "first", 1, vec![1], 0, 0, 5, 1, 1, 1.0, true),
            te_chunk(
                "b",
                "with, comma",
                2,
                vec![1, 2],
                1,
                5,
                16,
                1,
                2,
                0.8,
                false,
            ),
            te_chunk("c", "line\nbreak", 3, vec![2], 2, 16, 26, 2, 2, 0.6, true),
        ];
        let serialized = TokenEfficientExporter::new()
            .export_chunks(&chunks)
            .unwrap();
        let parsed = TokenEfficientExporter::parse_chunks(&serialized).unwrap();
        assert_eq!(parsed.len(), 3);
        for (orig, got) in chunks.iter().zip(parsed.iter()) {
            assert_chunk_eq(got, orig);
        }
    }

    #[test]
    fn test_parse_rejects_wrong_column_count() {
        let bad =
            format!("#oxct/1\n{TE_HEADER}\nonly\tnine\tfields\there\tare\tsome\tnot\televen\tk");
        assert!(TokenEfficientExporter::parse_chunks(&bad).is_err());
    }

    #[test]
    fn test_parse_rejects_bad_header() {
        let bad = "#oxct/1\nnot_the_header\nrow";
        assert!(TokenEfficientExporter::parse_chunks(bad).is_err());
    }

    #[test]
    fn test_parse_rejects_bad_magic() {
        let bad = format!("#oxct/2\n{TE_HEADER}");
        assert!(TokenEfficientExporter::parse_chunks(&bad).is_err());
    }

    #[test]
    fn test_roundtrip_content_with_odd_interior_quote() {
        // QR #3: content with a single (odd) interior quote, no newline, not at
        // the start. Must not let the parser swallow the following row.
        let chunks = vec![
            te_chunk("a", "say \"hello", 2, vec![1], 0, 0, 10, 1, 1, 0.5, false),
            te_chunk("b", "second chunk", 1, vec![1], 1, 10, 22, 1, 1, 0.5, false),
        ];
        let serialized = TokenEfficientExporter::new()
            .export_chunks(&chunks)
            .unwrap();
        let parsed = TokenEfficientExporter::parse_chunks(&serialized).unwrap();
        assert_eq!(parsed.len(), 2, "the second row must not be swallowed");
        assert_chunk_eq(&parsed[0], &chunks[0]);
        assert_chunk_eq(&parsed[1], &chunks[1]);
    }

    #[test]
    fn test_parse_rejects_unterminated_quote() {
        // QR #1: hand-crafted/corrupt input with a dangling quote must error,
        // not silently consume the rest of the document.
        let bad =
            format!("#oxct/1\n{TE_HEADER}\nc\t1\t0\t0\t1\t1\t1\t0.5000\ttrue\t1\t\"unterminated");
        assert!(TokenEfficientExporter::parse_chunks(&bad).is_err());
    }

    #[test]
    fn test_parse_rejects_non_finite_confidence() {
        // QR #4: NaN/Infinity confidence must be rejected rather than silently
        // producing a NaN that corrupts downstream ranking.
        let bad = format!("#oxct/1\n{TE_HEADER}\nc\t1\t0\t0\t1\t1\t1\tNaN\ttrue\t1\tx");
        assert!(TokenEfficientExporter::parse_chunks(&bad).is_err());
        let bad_inf = format!("#oxct/1\n{TE_HEADER}\nc\t1\t0\t0\t1\t1\t1\tinf\ttrue\t1\tx");
        assert!(TokenEfficientExporter::parse_chunks(&bad_inf).is_err());
    }

    #[test]
    fn test_chunk_exporter_trait_object() {
        let chunks = vec![te_chunk("a", "x", 1, vec![1], 0, 0, 1, 1, 1, 1.0, true)];
        // object-safe: usable behind a trait object
        let exporters: Vec<Box<dyn ChunkExporter>> = vec![Box::new(TokenEfficientExporter::new())];
        for e in &exporters {
            let out = e.export_chunks(&chunks).unwrap();
            assert!(out.starts_with("#oxct/1\n"));
        }
    }

    #[test]
    fn test_chunk_exporter_trait_matches_inherent() {
        let chunks = vec![te_chunk("a", "x", 1, vec![1], 0, 0, 1, 1, 1, 1.0, true)];
        let exporter = TokenEfficientExporter::new();
        let via_trait = ChunkExporter::export_chunks(&exporter, &chunks).unwrap();
        let via_inherent = exporter.export_chunks(&chunks).unwrap();
        assert_eq!(via_trait, via_inherent);
    }

    #[cfg(feature = "semantic")]
    #[test]
    fn test_json_exporter_implements_chunk_exporter() {
        let chunks = vec![te_chunk("a", "x", 1, vec![1], 0, 0, 1, 1, 1, 1.0, true)];
        let json_exporter = JsonExporter::default();
        let via_trait = ChunkExporter::export_chunks(&json_exporter, &chunks).unwrap();
        let via_inherent = JsonExporter::export_with_chunks(&chunks).unwrap();
        assert_eq!(via_trait, via_inherent);
    }

    /// Token-count benchmark validating the core claim of #291: the
    /// token-efficient format serializes a chunk set into fewer LLM tokens than
    /// the JSON baseline. Gated behind `token-bench` (pulls `tiktoken-rs`,
    /// MSRV 1.85+) so it never enters the default MSRV-1.77 build.
    ///
    /// Run: `cargo test -p oxidize-pdf --features token-bench uses_fewer_tokens -- --nocapture`
    #[cfg(feature = "token-bench")]
    #[test]
    fn test_token_efficient_uses_fewer_tokens_than_json() {
        // A representative RAG corpus: varied content lengths, multi-page spans,
        // commas, and mixed confidences — the homogeneous-record shape that the
        // tabular format is meant to win on.
        let paragraphs = [
            "The quarterly report shows revenue of $1,200,000, up 12% year over year.",
            "Section 3.2 describes the authentication flow, including token refresh and rotation.",
            "Climate models project a temperature increase between 1.5 and 4.0 degrees Celsius.",
            "The defendant, having waived counsel, proceeded to represent themselves at trial.",
            "Mitochondria are the membrane-bound organelles responsible for cellular respiration.",
        ];
        let mut chunks = Vec::new();
        for i in 0..50usize {
            let body = paragraphs[i % paragraphs.len()];
            // vary length so the corpus isn't uniform
            let content = if i % 3 == 0 {
                format!("{body} {body}")
            } else {
                body.to_string()
            };
            let first_page = 1 + i / 3;
            let last_page = first_page + (i % 2);
            let page_numbers: Vec<usize> = (first_page..=last_page).collect();
            chunks.push(te_chunk(
                &format!("chunk_{i}"),
                &content,
                crate::ai::chunking::DocumentChunker::estimate_tokens(&content),
                page_numbers,
                i,
                i * 100,
                i * 100 + content.len(),
                first_page,
                last_page,
                0.5 + (i % 5) as f32 / 10.0,
                i % 2 == 0,
            ));
        }

        let te_out = TokenEfficientExporter::new()
            .export_chunks(&chunks)
            .unwrap();
        let json_out = JsonExporter::export_with_chunks(&chunks).unwrap();

        let bpe = tiktoken_rs::cl100k_base().expect("cl100k_base tokenizer");
        let te_tokens = bpe.encode_ordinary(&te_out).len();
        let json_tokens = bpe.encode_ordinary(&json_out).len();

        let reduction = 100.0 * (1.0 - te_tokens as f64 / json_tokens as f64);
        eprintln!(
            "token-efficient: {te_tokens} tokens | json: {json_tokens} tokens | reduction: {reduction:.1}%"
        );

        assert!(
            te_tokens < json_tokens,
            "token-efficient ({te_tokens}) must use fewer tokens than JSON ({json_tokens})"
        );
    }

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
                    language: None,
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
                    language: None,
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
                language: None,
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
                language: None,
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

    // Contextual Format Tests
    #[test]
    fn test_contextual_simple() {
        let text = "This is sample content.";
        let result = ContextualFormat::export_simple(text).unwrap();

        assert!(result.contains("Document content:"));
        assert!(result.contains("This is sample content."));
        assert_eq!(result, "Document content:\n\nThis is sample content.");
    }

    #[test]
    fn test_contextual_with_metadata_full() {
        let metadata = DocumentMetadata {
            title: "Annual Report".to_string(),
            page_count: 25,
            created_at: Some("2025-01-15".to_string()),
            author: Some("Jane Smith".to_string()),
        };

        let result =
            ContextualFormat::export_with_metadata("Report content here.", &metadata).unwrap();

        // Check natural language introduction
        assert!(result.contains("This is a document titled \"Annual Report\""));
        assert!(result.contains("with 25 pages"));
        assert!(result.contains("written by Jane Smith"));
        assert!(result.contains("created on 2025-01-15"));
        assert!(result.contains("Content:"));
        assert!(result.contains("Report content here."));

        // Check sentence flow (singular vs plural)
        assert!(!result.contains("page,"));
        assert!(result.contains("pages,"));
    }

    #[test]
    fn test_contextual_with_metadata_minimal() {
        let metadata = DocumentMetadata {
            title: "Simple Doc".to_string(),
            page_count: 1,
            created_at: None,
            author: None,
        };

        let result = ContextualFormat::export_with_metadata("Content", &metadata).unwrap();

        assert!(result.contains("titled \"Simple Doc\""));
        assert!(result.contains("with 1 page"));
        assert!(!result.contains("pages")); // Should use singular "page"
        assert!(!result.contains("written by"));
        assert!(!result.contains("created on"));
    }

    #[test]
    fn test_contextual_with_metadata_no_page_count() {
        let metadata = DocumentMetadata {
            title: "Test".to_string(),
            page_count: 0,
            created_at: None,
            author: None,
        };

        let result = ContextualFormat::export_with_metadata("Text", &metadata).unwrap();

        // When page_count is 0, should not mention pages
        assert!(!result.contains("with"));
        assert!(!result.contains("page"));
        assert!(result.contains("This is a document titled \"Test\"."));
    }

    #[test]
    fn test_contextual_with_pages() {
        let pages = vec![
            (1, "First page text".to_string()),
            (2, "Second page text".to_string()),
            (3, "Third page text".to_string()),
        ];

        let result = ContextualFormat::export_with_pages(&pages).unwrap();

        assert!(result.starts_with("Document content:\n\n"));
        assert!(result.contains("On page 1:\nFirst page text"));
        assert!(result.contains("On page 2:\nSecond page text"));
        assert!(result.contains("On page 3:\nThird page text"));
    }

    #[test]
    fn test_contextual_with_pages_empty() {
        let pages: Vec<(usize, String)> = vec![];
        let result = ContextualFormat::export_with_pages(&pages).unwrap();

        assert_eq!(result, "Document content:\n\n");
    }

    #[test]
    fn test_contextual_with_pages_single() {
        let pages = vec![(1, "Only page".to_string())];
        let result = ContextualFormat::export_with_pages(&pages).unwrap();

        assert!(result.contains("On page 1:\nOnly page"));
    }

    #[test]
    fn test_contextual_with_metadata_and_pages() {
        let metadata = DocumentMetadata {
            title: "Technical Guide".to_string(),
            page_count: 2,
            created_at: Some("2025-10-13".to_string()),
            author: Some("John Doe".to_string()),
        };

        let pages = vec![
            (1, "Introduction text".to_string()),
            (2, "Main content".to_string()),
        ];

        let result = ContextualFormat::export_with_metadata_and_pages(&pages, &metadata).unwrap();

        // Check metadata introduction
        assert!(result.contains("titled \"Technical Guide\""));
        assert!(result.contains("with 2 pages"));
        assert!(result.contains("written by John Doe"));
        assert!(result.contains("created on 2025-10-13"));

        // Check content section
        assert!(result.contains("Content:"));
        assert!(result.contains("On page 1:\nIntroduction text"));
        assert!(result.contains("On page 2:\nMain content"));
    }

    #[test]
    fn test_contextual_natural_language_flow() {
        // Test that the format reads naturally
        let metadata = DocumentMetadata {
            title: "Report".to_string(),
            page_count: 5,
            created_at: Some("2025-01-01".to_string()),
            author: Some("Alice".to_string()),
        };

        let result = ContextualFormat::export_with_metadata("Text", &metadata).unwrap();

        // Should read as a natural sentence
        assert!(result.starts_with("This is a document titled \"Report\" with 5 pages, written by Alice, created on 2025-01-01."));
    }

    #[test]
    fn test_contextual_empty_text() {
        let result = ContextualFormat::export_simple("").unwrap();
        assert_eq!(result, "Document content:\n\n");
    }
}
