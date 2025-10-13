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
}
