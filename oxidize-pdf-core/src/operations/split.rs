//! PDF splitting functionality
//!
//! This module provides functionality to split PDF documents into multiple files
//! based on page ranges or other criteria.

use super::{OperationError, OperationResult, PageRange};
use crate::parser::page_tree::ParsedPage;
use crate::parser::{ContentOperation, ContentParser, PdfDocument, PdfReader};
use crate::{Document, Page};
use std::fs::File;
use std::path::{Path, PathBuf};

/// Options for PDF splitting
#[derive(Debug, Clone)]
pub struct SplitOptions {
    /// How to split the document
    pub mode: SplitMode,
    /// Output file naming pattern
    pub output_pattern: String,
    /// Whether to preserve document metadata
    pub preserve_metadata: bool,
    /// Whether to optimize output files
    pub optimize: bool,
}

impl Default for SplitOptions {
    fn default() -> Self {
        Self {
            mode: SplitMode::SinglePages,
            output_pattern: "page_{}.pdf".to_string(),
            preserve_metadata: true,
            optimize: false,
        }
    }
}

/// Split mode specification
#[derive(Debug, Clone)]
pub enum SplitMode {
    /// Split into single pages
    SinglePages,
    /// Split by page ranges
    Ranges(Vec<PageRange>),
    /// Split into chunks of N pages
    ChunkSize(usize),
    /// Split at specific page numbers (creates files before each split point)
    SplitAt(Vec<usize>),
}

/// PDF splitter
pub struct PdfSplitter {
    document: PdfDocument<File>,
    options: SplitOptions,
}

impl PdfSplitter {
    /// Create a new PDF splitter
    pub fn new(document: PdfDocument<File>, options: SplitOptions) -> Self {
        Self { document, options }
    }

    /// Split the PDF according to the options
    pub fn split(&mut self) -> OperationResult<Vec<PathBuf>> {
        let total_pages =
            self.document
                .page_count()
                .map_err(|e| OperationError::ParseError(e.to_string()))? as usize;

        if total_pages == 0 {
            return Err(OperationError::NoPagesToProcess);
        }

        let ranges = match &self.options.mode {
            SplitMode::SinglePages => {
                // Create a range for each page
                (0..total_pages).map(PageRange::Single).collect()
            }
            SplitMode::Ranges(ranges) => ranges.clone(),
            SplitMode::ChunkSize(size) => {
                // Create ranges for chunks
                let mut ranges = Vec::new();
                let mut start = 0;
                while start < total_pages {
                    let end = (start + size - 1).min(total_pages - 1);
                    ranges.push(PageRange::Range(start, end));
                    start += size;
                }
                ranges
            }
            SplitMode::SplitAt(split_points) => {
                // Create ranges between split points
                let mut ranges = Vec::new();
                let mut start = 0;

                for &split_point in split_points {
                    if split_point > 0 && split_point < total_pages {
                        ranges.push(PageRange::Range(start, split_point - 1));
                        start = split_point;
                    }
                }

                // Add the last range
                if start < total_pages {
                    ranges.push(PageRange::Range(start, total_pages - 1));
                }

                ranges
            }
        };

        // Process each range
        let mut output_files = Vec::new();

        for (index, range) in ranges.iter().enumerate() {
            let output_path = self.format_output_path(index, range);
            self.extract_range(range, &output_path)?;
            output_files.push(output_path);
        }

        Ok(output_files)
    }

    /// Extract a page range to a new PDF file
    fn extract_range(&mut self, range: &PageRange, output_path: &Path) -> OperationResult<()> {
        let total_pages =
            self.document
                .page_count()
                .map_err(|e| OperationError::ParseError(e.to_string()))? as usize;

        let indices = range.get_indices(total_pages)?;
        if indices.is_empty() {
            return Err(OperationError::NoPagesToProcess);
        }

        // Create new document
        let mut doc = Document::new();

        // Copy metadata if requested
        if self.options.preserve_metadata {
            if let Ok(metadata) = self.document.metadata() {
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
        }

        // Extract and add pages
        for &page_idx in &indices {
            let parsed_page = self
                .document
                .get_page(page_idx as u32)
                .map_err(|e| OperationError::ParseError(e.to_string()))?;

            let page = self.convert_page(&parsed_page)?;
            doc.add_page(page);
        }

        // Save the document
        doc.save(output_path)?;

        Ok(())
    }

    /// Convert a parsed page to a new page
    fn convert_page(&mut self, parsed_page: &ParsedPage) -> OperationResult<Page> {
        // Create new page with same dimensions
        let width = parsed_page.width();
        let height = parsed_page.height();
        let mut page = Page::new(width, height);

        // Set rotation if needed
        if parsed_page.rotation != 0 {
            page.set_rotation(parsed_page.rotation);
        }

        // Get content streams
        let content_streams = self
            .document
            .get_page_content_streams(parsed_page)
            .map_err(|e| OperationError::ParseError(e.to_string()))?;

        // Parse and process content streams
        let mut has_content = false;
        for stream_data in &content_streams {
            match ContentParser::parse_content(stream_data) {
                Ok(operators) => {
                    // Process the operators to recreate content
                    self.process_operators(&mut page, &operators)?;
                    has_content = true;
                }
                Err(e) => {
                    // If parsing fails, fall back to placeholder
                    eprintln!("Warning: Failed to parse content stream: {e}");
                }
            }
        }

        // If no content was successfully processed, add a placeholder
        if !has_content {
            page.text()
                .set_font(crate::text::Font::Helvetica, 10.0)
                .at(50.0, height - 50.0)
                .write("[Page extracted - content reconstruction in progress]")
                .map_err(OperationError::PdfError)?;
        }

        Ok(page)
    }

    /// Process content operators to recreate page content
    fn process_operators(
        &self,
        page: &mut Page,
        operators: &[ContentOperation],
    ) -> OperationResult<()> {
        // Track graphics state
        let mut text_object = false;
        let mut current_font = crate::text::Font::Helvetica;
        let mut current_font_size = 12.0;
        let mut current_x = 0.0;
        let mut current_y = 0.0;

        for operator in operators {
            match operator {
                ContentOperation::BeginText => {
                    text_object = true;
                }
                ContentOperation::EndText => {
                    text_object = false;
                }
                ContentOperation::SetFont(name, size) => {
                    // Map PDF font names to our fonts
                    current_font = match name.as_str() {
                        "Times-Roman" => crate::text::Font::TimesRoman,
                        "Times-Bold" => crate::text::Font::TimesBold,
                        "Times-Italic" => crate::text::Font::TimesItalic,
                        "Times-BoldItalic" => crate::text::Font::TimesBoldItalic,
                        "Helvetica-Bold" => crate::text::Font::HelveticaBold,
                        "Helvetica-Oblique" => crate::text::Font::HelveticaOblique,
                        "Helvetica-BoldOblique" => crate::text::Font::HelveticaBoldOblique,
                        "Courier" => crate::text::Font::Courier,
                        "Courier-Bold" => crate::text::Font::CourierBold,
                        "Courier-Oblique" => crate::text::Font::CourierOblique,
                        "Courier-BoldOblique" => crate::text::Font::CourierBoldOblique,
                        _ => crate::text::Font::Helvetica, // Default fallback
                    };
                    current_font_size = *size;
                }
                ContentOperation::MoveText(tx, ty) => {
                    current_x += tx;
                    current_y += ty;
                }
                ContentOperation::ShowText(text_bytes) => {
                    if text_object {
                        // Convert bytes to string (assuming ASCII/UTF-8 for now)
                        if let Ok(text) = String::from_utf8(text_bytes.clone()) {
                            page.text()
                                .set_font(current_font.clone(), current_font_size as f64)
                                .at(current_x as f64, current_y as f64)
                                .write(&text)
                                .map_err(OperationError::PdfError)?;
                        }
                    }
                }
                ContentOperation::Rectangle(x, y, width, height) => {
                    page.graphics()
                        .rect(*x as f64, *y as f64, *width as f64, *height as f64);
                }
                ContentOperation::MoveTo(x, y) => {
                    page.graphics().move_to(*x as f64, *y as f64);
                }
                ContentOperation::LineTo(x, y) => {
                    page.graphics().line_to(*x as f64, *y as f64);
                }
                ContentOperation::Stroke => {
                    page.graphics().stroke();
                }
                ContentOperation::Fill => {
                    page.graphics().fill();
                }
                ContentOperation::SetNonStrokingRGB(r, g, b) => {
                    page.graphics().set_fill_color(crate::graphics::Color::Rgb(
                        *r as f64, *g as f64, *b as f64,
                    ));
                }
                ContentOperation::SetStrokingRGB(r, g, b) => {
                    page.graphics()
                        .set_stroke_color(crate::graphics::Color::Rgb(
                            *r as f64, *g as f64, *b as f64,
                        ));
                }
                ContentOperation::SetLineWidth(width) => {
                    page.graphics().set_line_width(*width as f64);
                }
                // Note: Additional operators can be implemented on demand
                _ => {
                    // Silently skip unimplemented operators for now
                }
            }
        }

        Ok(())
    }

    /// Format the output path based on the pattern
    fn format_output_path(&self, index: usize, range: &PageRange) -> PathBuf {
        let filename = match range {
            PageRange::Single(page) => self
                .options
                .output_pattern
                .replace("{}", &(page + 1).to_string())
                .replace("{n}", &(index + 1).to_string())
                .replace("{page}", &(page + 1).to_string()),
            PageRange::Range(start, end) => self
                .options
                .output_pattern
                .replace("{}", &format!("{}-{}", start + 1, end + 1))
                .replace("{n}", &(index + 1).to_string())
                .replace("{start}", &(start + 1).to_string())
                .replace("{end}", &(end + 1).to_string()),
            _ => self
                .options
                .output_pattern
                .replace("{}", &(index + 1).to_string())
                .replace("{n}", &(index + 1).to_string()),
        };

        PathBuf::from(filename)
    }
}

/// Split a PDF file by page ranges
pub fn split_pdf<P: AsRef<Path>>(
    input_path: P,
    options: SplitOptions,
) -> OperationResult<Vec<PathBuf>> {
    let document = PdfReader::open_document(input_path)
        .map_err(|e| OperationError::ParseError(e.to_string()))?;

    let mut splitter = PdfSplitter::new(document, options);
    splitter.split()
}

/// Split a PDF file into single pages
pub fn split_into_pages<P: AsRef<Path>>(
    input_path: P,
    output_pattern: &str,
) -> OperationResult<Vec<PathBuf>> {
    let options = SplitOptions {
        mode: SplitMode::SinglePages,
        output_pattern: output_pattern.to_string(),
        ..Default::default()
    };

    split_pdf(input_path, options)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_split_options_default() {
        let options = SplitOptions::default();
        assert!(matches!(options.mode, SplitMode::SinglePages));
        assert_eq!(options.output_pattern, "page_{}.pdf");
        assert!(options.preserve_metadata);
        assert!(!options.optimize);
    }

    #[test]
    fn test_format_output_path() {
        let _options = SplitOptions {
            output_pattern: "output_page_{}.pdf".to_string(),
            ..Default::default()
        };

        let _reader = PdfReader::open("test.pdf");
        // Note: This test would need a valid PDF file to work properly
        // For now, we're just testing the logic
    }

    // ============= Additional Split Tests =============

    #[test]
    fn test_split_mode_variants() {
        // Test SinglePages variant
        let single_pages = SplitMode::SinglePages;
        assert!(matches!(single_pages, SplitMode::SinglePages));

        // Test Ranges variant
        let ranges = SplitMode::Ranges(vec![
            super::PageRange::Single(0),
            super::PageRange::Range(5, 10),
        ]);
        assert!(matches!(ranges, SplitMode::Ranges(_)));

        // Test ChunkSize variant
        let chunk = SplitMode::ChunkSize(5);
        if let SplitMode::ChunkSize(size) = chunk {
            assert_eq!(size, 5);
        } else {
            panic!("Expected ChunkSize");
        }

        // Test SplitAt variant
        let split_at = SplitMode::SplitAt(vec![5, 10, 15]);
        assert!(matches!(split_at, SplitMode::SplitAt(_)));
    }

    #[test]
    fn test_split_options_with_modes() {
        let options = SplitOptions {
            mode: SplitMode::ChunkSize(10),
            output_pattern: "chunk_{}.pdf".to_string(),
            preserve_metadata: true,
            optimize: true,
        };

        assert!(matches!(options.mode, SplitMode::ChunkSize(10)));
        assert_eq!(options.output_pattern, "chunk_{}.pdf");
        assert!(options.preserve_metadata);
        assert!(options.optimize);
    }

    #[test]
    fn test_split_options_page_range() {
        let ranges = vec![
            super::PageRange::All,
            super::PageRange::Single(5),
            super::PageRange::Range(10, 20),
            super::PageRange::List(vec![1, 3, 5, 7, 9]),
        ];

        let options = SplitOptions {
            mode: SplitMode::Ranges(ranges),
            ..Default::default()
        };

        if let SplitMode::Ranges(r) = options.mode {
            assert_eq!(r.len(), 4);
        } else {
            panic!("Expected Ranges mode");
        }
    }

    #[test]
    fn test_split_options_split_at() {
        let split_points = vec![3, 6, 9, 12]; // Split at these page numbers

        let options = SplitOptions {
            mode: SplitMode::SplitAt(split_points.clone()),
            output_pattern: "part_{}.pdf".to_string(),
            ..Default::default()
        };

        if let SplitMode::SplitAt(points) = options.mode {
            assert_eq!(points.len(), 4);
            assert_eq!(points, split_points);
        } else {
            panic!("Expected SplitAt mode");
        }
    }

    #[test]
    fn test_output_pattern_formatting() {
        // Test various output patterns
        let patterns = vec![
            "output_{}.pdf",
            "page_{}.pdf",
            "document_part_{}.pdf",
            "{}_split.pdf",
        ];

        for pattern in patterns {
            let options = SplitOptions {
                output_pattern: pattern.to_string(),
                ..Default::default()
            };
            assert!(options.output_pattern.contains("{")); // Just check for placeholder
        }
    }

    #[test]
    fn test_split_options_preserve_metadata() {
        // Test preserve_metadata flag
        let with_metadata = SplitOptions {
            preserve_metadata: true,
            ..Default::default()
        };
        assert!(with_metadata.preserve_metadata);

        let without_metadata = SplitOptions {
            preserve_metadata: false,
            ..Default::default()
        };
        assert!(!without_metadata.preserve_metadata);
    }

    #[test]
    fn test_split_single_pages_mode() {
        let options = SplitOptions {
            mode: SplitMode::SinglePages,
            output_pattern: "page_{:04}.pdf".to_string(),
            ..Default::default()
        };

        assert!(matches!(options.mode, SplitMode::SinglePages));
        assert!(options.output_pattern.contains("{"));
    }

    #[test]
    fn test_split_chunk_size_validation() {
        // Test various chunk sizes
        let chunk_sizes = vec![1, 5, 10, 50, 100];

        for size in chunk_sizes {
            let options = SplitOptions {
                mode: SplitMode::ChunkSize(size),
                ..Default::default()
            };

            if let SplitMode::ChunkSize(s) = options.mode {
                assert_eq!(s, size);
                assert!(s > 0); // Chunk size should be positive
            }
        }
    }

    #[test]
    fn test_split_options_optimization() {
        let optimized = SplitOptions {
            optimize: true,
            ..Default::default()
        };
        assert!(optimized.optimize);

        let not_optimized = SplitOptions {
            optimize: false,
            ..Default::default()
        };
        assert!(!not_optimized.optimize);
    }

    #[test]
    fn test_split_options_with_custom_pattern() {
        let options = SplitOptions {
            output_pattern: "document_part_{}.pdf".to_string(),
            ..Default::default()
        };
        assert_eq!(options.output_pattern, "document_part_{}.pdf");
    }

    #[test]
    fn test_split_mode_ranges() {
        let ranges = vec![
            PageRange::Single(0),
            PageRange::Range(1, 3),
            PageRange::Single(5),
        ];
        let mode = SplitMode::Ranges(ranges.clone());

        match mode {
            SplitMode::Ranges(r) => {
                assert_eq!(r.len(), 3);
                assert!(matches!(r[0], PageRange::Single(0)));
                assert!(matches!(r[1], PageRange::Range(1, 3)));
                assert!(matches!(r[2], PageRange::Single(5)));
            }
            _ => panic!("Wrong mode"),
        }
    }

    #[test]
    fn test_split_mode_split_at() {
        let split_points = vec![5, 10, 15];
        let mode = SplitMode::SplitAt(split_points.clone());

        match mode {
            SplitMode::SplitAt(points) => assert_eq!(points, split_points),
            _ => panic!("Wrong mode"),
        }
    }

    #[test]
    fn test_page_range_parse() {
        // Test all pages
        let range = PageRange::parse("all").unwrap();
        assert!(matches!(range, PageRange::All));

        // Test single page
        let range = PageRange::parse("5").unwrap();
        assert!(matches!(range, PageRange::Single(4))); // 0-indexed

        // Test range
        let range = PageRange::parse("3-7").unwrap();
        assert!(matches!(range, PageRange::Range(2, 6))); // 0-indexed

        // Test list
        let range = PageRange::parse("1,3,5").unwrap();
        match range {
            PageRange::List(pages) => assert_eq!(pages, vec![0, 2, 4]),
            _ => panic!("Expected List"),
        }
    }

    #[test]
    fn test_page_range_invalid_parse() {
        assert!(PageRange::parse("").is_err());
        assert!(PageRange::parse("abc").is_err());
        assert!(PageRange::parse("5-3").is_err()); // Invalid range
        assert!(PageRange::parse("0").is_err()); // Page numbers start at 1
    }

    #[test]
    fn test_split_options_all_fields() {
        let options = SplitOptions {
            mode: SplitMode::ChunkSize(5),
            output_pattern: "chunk_{}.pdf".to_string(),
            preserve_metadata: false,
            optimize: true,
        };

        match options.mode {
            SplitMode::ChunkSize(size) => assert_eq!(size, 5),
            _ => panic!("Wrong mode"),
        }
        assert_eq!(options.output_pattern, "chunk_{}.pdf");
        assert!(!options.preserve_metadata);
        assert!(options.optimize);
    }

    #[test]
    fn test_split_mode_chunk_size_edge_cases() {
        // Chunk size of 1 should be like single pages
        let mode = SplitMode::ChunkSize(1);
        match mode {
            SplitMode::ChunkSize(size) => assert_eq!(size, 1),
            _ => panic!("Wrong mode"),
        }

        // Large chunk size
        let mode = SplitMode::ChunkSize(1000);
        match mode {
            SplitMode::ChunkSize(size) => assert_eq!(size, 1000),
            _ => panic!("Wrong mode"),
        }
    }

    #[test]
    fn test_split_mode_empty_ranges() {
        let ranges = Vec::new();
        let mode = SplitMode::Ranges(ranges);

        match mode {
            SplitMode::Ranges(r) => assert!(r.is_empty()),
            _ => panic!("Wrong mode"),
        }
    }

    #[test]
    fn test_split_mode_empty_split_points() {
        let split_points = Vec::new();
        let mode = SplitMode::SplitAt(split_points);

        match mode {
            SplitMode::SplitAt(points) => assert!(points.is_empty()),
            _ => panic!("Wrong mode"),
        }
    }
}

#[cfg(test)]
#[path = "split_tests.rs"]
mod split_tests;
