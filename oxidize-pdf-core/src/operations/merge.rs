//! PDF merging functionality
//!
//! This module provides functionality to merge multiple PDF documents into a single file.

use super::{OperationError, OperationResult, PageRange};
use crate::parser::{PdfDocument, PdfReader};
use crate::{Document, Page};
use std::fs::File;
use std::path::{Path, PathBuf};

/// Options for PDF merging
#[derive(Debug, Clone)]
pub struct MergeOptions {
    /// Page ranges to include from each input file
    pub page_ranges: Option<Vec<PageRange>>,
    /// Whether to preserve bookmarks/outlines
    pub preserve_bookmarks: bool,
    /// Whether to preserve form fields
    pub preserve_forms: bool,
    /// Whether to optimize the output
    pub optimize: bool,
    /// How to handle metadata
    pub metadata_mode: MetadataMode,
}

impl Default for MergeOptions {
    fn default() -> Self {
        Self {
            page_ranges: None,
            preserve_bookmarks: true,
            preserve_forms: false,
            optimize: false,
            metadata_mode: MetadataMode::FromFirst,
        }
    }
}

/// How to handle metadata when merging
#[derive(Debug, Clone)]
pub enum MetadataMode {
    /// Use metadata from the first document
    FromFirst,
    /// Use metadata from a specific document (by index)
    FromDocument(usize),
    /// Use custom metadata
    Custom {
        title: Option<String>,
        author: Option<String>,
        subject: Option<String>,
        keywords: Option<String>,
    },
    /// Don't set any metadata
    None,
}

/// Input specification for merging
#[derive(Debug)]
pub struct MergeInput {
    /// Path to the PDF file
    pub path: PathBuf,
    /// Optional page range to include
    pub pages: Option<PageRange>,
}

impl MergeInput {
    /// Create a new merge input that includes all pages
    pub fn new<P: Into<PathBuf>>(path: P) -> Self {
        Self {
            path: path.into(),
            pages: None,
        }
    }

    /// Create a merge input with specific pages
    pub fn with_pages<P: Into<PathBuf>>(path: P, pages: PageRange) -> Self {
        Self {
            path: path.into(),
            pages: Some(pages),
        }
    }
}

/// PDF merger
pub struct PdfMerger {
    inputs: Vec<MergeInput>,
    options: MergeOptions,
}

impl PdfMerger {
    /// Create a new PDF merger
    pub fn new(options: MergeOptions) -> Self {
        Self {
            inputs: Vec::new(),
            options,
        }
    }

    /// Add an input file to merge
    pub fn add_input(&mut self, input: MergeInput) {
        self.inputs.push(input);
    }

    /// Add multiple input files
    pub fn add_inputs(&mut self, inputs: impl IntoIterator<Item = MergeInput>) {
        self.inputs.extend(inputs);
    }

    /// Merge all input files into a single document
    pub fn merge(&mut self) -> OperationResult<Document> {
        if self.inputs.is_empty() {
            return Err(OperationError::NoPagesToProcess);
        }

        let mut output_doc = Document::new();

        // Process each input file
        for input_idx in 0..self.inputs.len() {
            let input_path = self.inputs[input_idx].path.clone();
            let input_pages = self.inputs[input_idx].pages.clone();

            let document = PdfReader::open_document(&input_path).map_err(|e| {
                OperationError::ParseError(format!(
                    "Failed to open {}: {}",
                    input_path.display(),
                    e
                ))
            })?;

            // Get page range
            let total_pages = document
                .page_count()
                .map_err(|e| OperationError::ParseError(e.to_string()))?
                as usize;

            let page_range = input_pages.as_ref().unwrap_or(&PageRange::All);

            let page_indices = page_range.get_indices(total_pages)?;

            // Extract and add pages
            for page_idx in page_indices {
                let parsed_page = document
                    .get_page(page_idx as u32)
                    .map_err(|e| OperationError::ParseError(e.to_string()))?;

                // Use Page::from_parsed_with_content to preserve original content streams
                // and resources (fonts, images, XObjects) instead of reconstructing pages
                let page = Page::from_parsed_with_content(&parsed_page, &document)
                    .map_err(|e| OperationError::ParseError(e.to_string()))?;
                output_doc.add_page(page);
            }

            // Handle metadata for the first document or specified document
            match &self.options.metadata_mode {
                MetadataMode::FromFirst if input_idx == 0 => {
                    self.copy_metadata(&document, &mut output_doc)?;
                }
                MetadataMode::FromDocument(idx) if input_idx == *idx => {
                    self.copy_metadata(&document, &mut output_doc)?;
                }
                _ => {}
            }
        }

        // Apply custom metadata if specified
        if let MetadataMode::Custom {
            title,
            author,
            subject,
            keywords,
        } = &self.options.metadata_mode
        {
            if let Some(title) = title {
                output_doc.set_title(title);
            }
            if let Some(author) = author {
                output_doc.set_author(author);
            }
            if let Some(subject) = subject {
                output_doc.set_subject(subject);
            }
            if let Some(keywords) = keywords {
                output_doc.set_keywords(keywords);
            }
        }

        Ok(output_doc)
    }

    /// Merge files and save to output path
    pub fn merge_to_file<P: AsRef<Path>>(&mut self, output_path: P) -> OperationResult<()> {
        let mut doc = self.merge()?;
        doc.save(output_path)?;
        Ok(())
    }

    /// Copy metadata from source to destination document
    fn copy_metadata(
        &self,
        document: &PdfDocument<File>,
        doc: &mut Document,
    ) -> OperationResult<()> {
        if let Ok(metadata) = document.metadata() {
            if let Some(title) = metadata.title {
                doc.set_title(&title);
            }
            if let Some(author) = metadata.author {
                doc.set_author(&author);
            }
            if let Some(subject) = metadata.subject {
                doc.set_subject(&subject);
            }
            if let Some(keywords) = metadata.keywords {
                doc.set_keywords(&keywords);
            }
        }
        Ok(())
    }
}

/// Merge multiple PDF files into one
pub fn merge_pdfs<P: AsRef<Path>>(
    inputs: Vec<MergeInput>,
    output_path: P,
    options: MergeOptions,
) -> OperationResult<()> {
    let mut merger = PdfMerger::new(options);
    merger.add_inputs(inputs);
    merger.merge_to_file(output_path)
}

/// Simple merge of multiple PDF files with default options
pub fn merge_pdf_files<P: AsRef<Path>, Q: AsRef<Path>>(
    input_paths: &[P],
    output_path: Q,
) -> OperationResult<()> {
    let inputs: Vec<MergeInput> = input_paths
        .iter()
        .map(|p| MergeInput::new(p.as_ref()))
        .collect();

    merge_pdfs(inputs, output_path, MergeOptions::default())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_merge_options_default() {
        let options = MergeOptions::default();
        assert!(options.page_ranges.is_none());
        assert!(options.preserve_bookmarks);
        assert!(!options.preserve_forms);
        assert!(!options.optimize);
        assert!(matches!(options.metadata_mode, MetadataMode::FromFirst));
    }

    #[test]
    fn test_merge_input_creation() {
        let input = MergeInput::new("test.pdf");
        assert_eq!(input.path, PathBuf::from("test.pdf"));
        assert!(input.pages.is_none());

        let input_with_pages = MergeInput::with_pages("test.pdf", PageRange::Range(0, 4));
        assert!(input_with_pages.pages.is_some());
    }

    // ============= Additional MergeOptions Tests =============

    #[test]
    fn test_merge_options_with_custom_metadata() {
        let options = MergeOptions {
            page_ranges: Some(vec![PageRange::All]),
            preserve_bookmarks: false,
            preserve_forms: true,
            optimize: true,
            metadata_mode: MetadataMode::Custom {
                title: Some("Merged Document".to_string()),
                author: Some("PDF Merger".to_string()),
                subject: Some("Combined PDFs".to_string()),
                keywords: Some("merge, pdf".to_string()),
            },
        };

        assert!(options.page_ranges.is_some());
        assert!(!options.preserve_bookmarks);
        assert!(options.preserve_forms);
        assert!(options.optimize);

        if let MetadataMode::Custom { title, .. } = options.metadata_mode {
            assert_eq!(title, Some("Merged Document".to_string()));
        } else {
            panic!("Expected Custom metadata mode");
        }
    }

    #[test]
    fn test_merge_options_from_document() {
        let options = MergeOptions {
            metadata_mode: MetadataMode::FromDocument(2),
            ..Default::default()
        };

        if let MetadataMode::FromDocument(idx) = options.metadata_mode {
            assert_eq!(idx, 2);
        } else {
            panic!("Expected FromDocument metadata mode");
        }
    }

    #[test]
    fn test_page_range_variants() {
        // Test All variant
        let all_pages = PageRange::All;
        assert!(matches!(all_pages, PageRange::All));

        // Test Single page
        let single = PageRange::Single(5);
        if let PageRange::Single(page) = single {
            assert_eq!(page, 5);
        } else {
            panic!("Expected Single page range");
        }

        // Test Range
        let range = PageRange::Range(1, 10);
        if let PageRange::Range(start, end) = range {
            assert_eq!(start, 1);
            assert_eq!(end, 10);
        } else {
            panic!("Expected Range");
        }

        // Test List
        let list = PageRange::List(vec![1, 3, 5, 7]);
        if let PageRange::List(pages) = list {
            assert_eq!(pages, vec![1, 3, 5, 7]);
        } else {
            panic!("Expected List");
        }
    }

    #[test]
    fn test_merge_input_with_all_pages() {
        let input = MergeInput::with_pages("document.pdf", PageRange::All);
        assert_eq!(input.path, PathBuf::from("document.pdf"));
        assert!(input.pages.is_some()); // Can't compare PageRange directly
    }

    #[test]
    fn test_merge_input_with_single_page() {
        let input = MergeInput::with_pages("document.pdf", PageRange::Single(0));
        assert_eq!(input.path, PathBuf::from("document.pdf"));
        assert!(input.pages.is_some()); // Can't compare PageRange directly
    }

    #[test]
    fn test_merge_input_with_page_list() {
        let pages = vec![0, 2, 4, 6];
        let input = MergeInput::with_pages("document.pdf", PageRange::List(pages));
        assert_eq!(input.path, PathBuf::from("document.pdf"));
        assert!(input.pages.is_some());
    }

    // Tests removed for PdfMerger methods that don't exist or have different signatures

    #[test]
    fn test_metadata_mode_all_variants() {
        // Test all MetadataMode variants
        let from_first = MetadataMode::FromFirst;
        assert!(matches!(from_first, MetadataMode::FromFirst));

        let from_doc = MetadataMode::FromDocument(3);
        assert!(matches!(from_doc, MetadataMode::FromDocument(3)));

        let custom = MetadataMode::Custom {
            title: Some("Title".to_string()),
            author: None,
            subject: None,
            keywords: None,
        };
        assert!(matches!(custom, MetadataMode::Custom { .. }));
    }

    // Removed test_merge_builder_pattern - PdfMerger methods not matching

    #[test]
    fn test_merge_options_builder() {
        let options = MergeOptions {
            page_ranges: Some(vec![
                PageRange::All,
                PageRange::Range(0, 5),
                PageRange::Single(10),
            ]),
            preserve_bookmarks: true,
            preserve_forms: true,
            optimize: true,
            metadata_mode: MetadataMode::FromFirst,
        };

        assert!(options.page_ranges.is_some());
        let ranges = options.page_ranges.unwrap();
        assert_eq!(ranges.len(), 3);
    }
}

#[cfg(test)]
#[path = "merge_tests.rs"]
mod merge_tests;
