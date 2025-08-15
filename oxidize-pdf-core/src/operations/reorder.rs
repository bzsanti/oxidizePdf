//! PDF page reordering functionality
//!
//! This module provides functionality to reorder pages within a PDF document.

use super::{OperationError, OperationResult};
use crate::parser::page_tree::ParsedPage;
use crate::parser::{ContentOperation, ContentParser, PdfDocument, PdfReader};
use crate::{Document, Page};
use std::fs::File;
use std::path::Path;

/// Options for page reordering
#[derive(Debug, Clone)]
pub struct ReorderOptions {
    /// The new order of pages (0-based indices)
    pub page_order: Vec<usize>,
    /// Whether to preserve document metadata
    pub preserve_metadata: bool,
    /// Whether to optimize the output
    pub optimize: bool,
}

impl Default for ReorderOptions {
    fn default() -> Self {
        Self {
            page_order: Vec::new(),
            preserve_metadata: true,
            optimize: false,
        }
    }
}

/// Page reorderer
pub struct PageReorderer {
    document: PdfDocument<File>,
    options: ReorderOptions,
}

impl PageReorderer {
    /// Create a new page reorderer
    pub fn new(document: PdfDocument<File>, options: ReorderOptions) -> Self {
        Self { document, options }
    }

    /// Reorder pages according to the specified order
    pub fn reorder(&self) -> OperationResult<Document> {
        let total_pages =
            self.document
                .page_count()
                .map_err(|e| OperationError::ParseError(e.to_string()))? as usize;

        if total_pages == 0 {
            return Err(OperationError::NoPagesToProcess);
        }

        // Validate page order
        self.validate_page_order(total_pages)?;

        // Create new document
        let mut output_doc = Document::new();

        // Copy metadata if requested
        if self.options.preserve_metadata {
            self.copy_metadata(&mut output_doc)?;
        }

        // Add pages in the new order
        for &page_idx in &self.options.page_order {
            let parsed_page = self
                .document
                .get_page(page_idx as u32)
                .map_err(|e| OperationError::ParseError(e.to_string()))?;

            let page = self.convert_page(&parsed_page)?;
            output_doc.add_page(page);
        }

        Ok(output_doc)
    }

    /// Reorder pages and save to file
    pub fn reorder_to_file<P: AsRef<Path>>(&self, output_path: P) -> OperationResult<()> {
        let mut doc = self.reorder()?;
        doc.save(output_path)?;
        Ok(())
    }

    /// Validate that the page order is valid
    fn validate_page_order(&self, total_pages: usize) -> OperationResult<()> {
        if self.options.page_order.is_empty() {
            return Err(OperationError::InvalidPageRange(
                "Page order cannot be empty".to_string(),
            ));
        }

        // Check for out of bounds indices
        for &idx in &self.options.page_order {
            if idx >= total_pages {
                return Err(OperationError::InvalidPageRange(format!(
                    "Page index {idx} is out of bounds (document has {total_pages} pages)"
                )));
            }
        }

        Ok(())
    }

    /// Copy metadata from source to destination document
    fn copy_metadata(&self, doc: &mut Document) -> OperationResult<()> {
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
        Ok(())
    }

    /// Convert a parsed page to a new page
    fn convert_page(&self, parsed_page: &ParsedPage) -> OperationResult<Page> {
        // Create new page with same dimensions
        let width = parsed_page.width();
        let height = parsed_page.height();
        let mut page = Page::new(width, height);

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
                    self.process_operators(&mut page, &operators)?;
                    has_content = true;
                }
                Err(e) => {
                    eprintln!("Warning: Failed to parse content stream: {e}");
                }
            }
        }

        // If no content was successfully processed, add a placeholder
        if !has_content {
            page.text()
                .set_font(crate::text::Font::Helvetica, 10.0)
                .at(50.0, height - 50.0)
                .write("[Page reordered - content reconstruction in progress]")
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
                        _ => crate::text::Font::Helvetica,
                    };
                    current_font_size = *size;
                }
                ContentOperation::MoveText(tx, ty) => {
                    current_x += tx;
                    current_y += ty;
                }
                ContentOperation::ShowText(text) => {
                    if text_object && !text.is_empty() {
                        page.text()
                            .set_font(current_font.clone(), current_font_size as f64)
                            .at(current_x as f64, current_y as f64)
                            .write(&String::from_utf8_lossy(text))
                            .map_err(OperationError::PdfError)?;
                    }
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
                ContentOperation::Rectangle(x, y, w, h) => {
                    page.graphics()
                        .rectangle(*x as f64, *y as f64, *w as f64, *h as f64);
                }
                ContentOperation::SetLineWidth(width) => {
                    page.graphics().set_line_width(*width as f64);
                }
                _ => {
                    // Silently skip unimplemented operators for now
                }
            }
        }

        Ok(())
    }
}

/// Convenience function to reorder pages in a PDF
pub fn reorder_pdf_pages<P: AsRef<Path>, Q: AsRef<Path>>(
    input_path: P,
    output_path: Q,
    page_order: Vec<usize>,
) -> OperationResult<()> {
    let document = PdfReader::open_document(input_path)
        .map_err(|e| OperationError::ParseError(e.to_string()))?;

    let options = ReorderOptions {
        page_order,
        preserve_metadata: true,
        optimize: false,
    };

    let reorderer = PageReorderer::new(document, options);
    reorderer.reorder_to_file(output_path)
}

/// Reverse all pages in a PDF
pub fn reverse_pdf_pages<P: AsRef<Path>, Q: AsRef<Path>>(
    input_path: P,
    output_path: Q,
) -> OperationResult<()> {
    let document = PdfReader::open_document(&input_path)
        .map_err(|e| OperationError::ParseError(e.to_string()))?;

    let page_count = document
        .page_count()
        .map_err(|e| OperationError::ParseError(e.to_string()))? as usize;

    // Create reverse order
    let page_order: Vec<usize> = (0..page_count).rev().collect();

    reorder_pdf_pages(input_path, output_path, page_order)
}

/// Move a page to a new position
pub fn move_pdf_page<P: AsRef<Path>, Q: AsRef<Path>>(
    input_path: P,
    output_path: Q,
    from_index: usize,
    to_index: usize,
) -> OperationResult<()> {
    let document = PdfReader::open_document(&input_path)
        .map_err(|e| OperationError::ParseError(e.to_string()))?;

    let page_count = document
        .page_count()
        .map_err(|e| OperationError::ParseError(e.to_string()))? as usize;

    if from_index >= page_count || to_index >= page_count {
        return Err(OperationError::InvalidPageRange(
            "Page index out of bounds".to_string(),
        ));
    }

    // Create new order
    let mut page_order: Vec<usize> = (0..page_count).collect();
    let page = page_order.remove(from_index);
    page_order.insert(to_index, page);

    reorder_pdf_pages(input_path, output_path, page_order)
}

/// Swap two pages in a PDF
pub fn swap_pdf_pages<P: AsRef<Path>, Q: AsRef<Path>>(
    input_path: P,
    output_path: Q,
    page1: usize,
    page2: usize,
) -> OperationResult<()> {
    let document = PdfReader::open_document(&input_path)
        .map_err(|e| OperationError::ParseError(e.to_string()))?;

    let page_count = document
        .page_count()
        .map_err(|e| OperationError::ParseError(e.to_string()))? as usize;

    if page1 >= page_count || page2 >= page_count {
        return Err(OperationError::InvalidPageRange(
            "Page index out of bounds".to_string(),
        ));
    }

    // Create new order with swapped pages
    let mut page_order: Vec<usize> = (0..page_count).collect();
    page_order.swap(page1, page2);

    reorder_pdf_pages(input_path, output_path, page_order)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reorder_options_default() {
        let options = ReorderOptions::default();
        assert!(options.page_order.is_empty());
        assert!(options.preserve_metadata);
        assert!(!options.optimize);
    }

    #[test]
    fn test_reorder_options_custom() {
        let options = ReorderOptions {
            page_order: vec![2, 0, 1],
            preserve_metadata: false,
            optimize: true,
        };
        assert_eq!(options.page_order, vec![2, 0, 1]);
        assert!(!options.preserve_metadata);
        assert!(options.optimize);
    }

    #[test]
    fn test_validate_page_order_empty() {
        use crate::{Document, Page};
        use tempfile::NamedTempFile;

        // Create a test PDF
        let mut doc = Document::new();
        doc.add_page(Page::a4());

        let temp_file = NamedTempFile::new().unwrap();
        doc.save(temp_file.path()).unwrap();

        // Create reorderer with empty page order
        let pdf_doc = PdfReader::open_document(temp_file.path()).unwrap();
        let options = ReorderOptions {
            page_order: vec![],
            preserve_metadata: true,
            optimize: false,
        };

        let reorderer = PageReorderer::new(pdf_doc, options);
        let result = reorderer.reorder();

        assert!(result.is_err());
        if let Err(OperationError::InvalidPageRange(msg)) = result {
            assert!(msg.contains("empty"));
        } else {
            panic!("Expected InvalidPageRange error");
        }
    }

    #[test]
    fn test_validate_page_order_out_of_bounds() {
        use crate::{Document, Page};
        use tempfile::NamedTempFile;

        // Create a test PDF with 2 pages
        let mut doc = Document::new();
        doc.add_page(Page::a4());
        doc.add_page(Page::letter());

        let temp_file = NamedTempFile::new().unwrap();
        doc.save(temp_file.path()).unwrap();

        // Try to reorder with invalid index
        let pdf_doc = PdfReader::open_document(temp_file.path()).unwrap();
        let options = ReorderOptions {
            page_order: vec![0, 5], // Index 5 is out of bounds
            preserve_metadata: true,
            optimize: false,
        };

        let reorderer = PageReorderer::new(pdf_doc, options);
        let result = reorderer.reorder();

        assert!(result.is_err());
        if let Err(OperationError::InvalidPageRange(msg)) = result {
            assert!(msg.contains("out of bounds"));
        } else {
            panic!("Expected InvalidPageRange error");
        }
    }

    #[test]
    fn test_reorder_pages_simple() {
        use crate::{Document, Page};
        use tempfile::NamedTempFile;

        // Create a test PDF with 3 pages
        let mut doc = Document::new();
        let mut page1 = Page::a4();
        page1.graphics().begin_text();
        page1.graphics().set_text_position(100.0, 700.0);
        page1.graphics().show_text("Page 1");
        page1.graphics().end_text();
        doc.add_page(page1);

        let mut page2 = Page::a4();
        page2.graphics().begin_text();
        page2.graphics().set_text_position(100.0, 700.0);
        page2.graphics().show_text("Page 2");
        page2.graphics().end_text();
        doc.add_page(page2);

        let mut page3 = Page::a4();
        page3.graphics().begin_text();
        page3.graphics().set_text_position(100.0, 700.0);
        page3.graphics().show_text("Page 3");
        page3.graphics().end_text();
        doc.add_page(page3);

        let temp_file = NamedTempFile::new().unwrap();
        doc.save(temp_file.path()).unwrap();

        // Reorder pages: [2, 0, 1]
        let pdf_doc = PdfReader::open_document(temp_file.path()).unwrap();
        let options = ReorderOptions {
            page_order: vec![2, 0, 1],
            preserve_metadata: true,
            optimize: false,
        };

        let reorderer = PageReorderer::new(pdf_doc, options);
        let result = reorderer.reorder();

        assert!(result.is_ok());
        let reordered_doc = result.unwrap();
        assert_eq!(reordered_doc.page_count(), 3);
    }

    #[test]
    fn test_reverse_pages() {
        use crate::{Document, Page};
        use tempfile::NamedTempFile;

        // Create a test PDF with 4 pages
        let mut doc = Document::new();
        for i in 1..=4 {
            let mut page = Page::a4();
            page.graphics().begin_text();
            page.graphics().set_text_position(100.0, 700.0);
            page.graphics().show_text(&format!("Page {}", i));
            page.graphics().end_text();
            doc.add_page(page);
        }

        let temp_input = NamedTempFile::new().unwrap();
        doc.save(temp_input.path()).unwrap();

        let temp_output = NamedTempFile::new().unwrap();

        // Reverse the pages
        let result = reverse_pdf_pages(temp_input.path(), temp_output.path());
        assert!(result.is_ok());

        // Verify the output file exists
        assert!(temp_output.path().exists());
    }

    #[test]
    fn test_swap_pages() {
        use crate::{Document, Page};
        use tempfile::NamedTempFile;

        // Create a test PDF
        let mut doc = Document::new();
        doc.add_page(Page::a4());
        doc.add_page(Page::letter());
        doc.add_page(Page::legal());

        let temp_input = NamedTempFile::new().unwrap();
        doc.save(temp_input.path()).unwrap();

        let temp_output = NamedTempFile::new().unwrap();

        // Swap pages 0 and 2
        let result = swap_pdf_pages(temp_input.path(), temp_output.path(), 0, 2);
        assert!(result.is_ok());

        // Test invalid swap (out of bounds)
        let result = swap_pdf_pages(temp_input.path(), temp_output.path(), 0, 10);
        assert!(result.is_err());
    }

    #[test]
    fn test_move_page() {
        use crate::{Document, Page};
        use tempfile::NamedTempFile;

        // Create a test PDF
        let mut doc = Document::new();
        for _ in 0..5 {
            doc.add_page(Page::a4());
        }

        let temp_input = NamedTempFile::new().unwrap();
        doc.save(temp_input.path()).unwrap();

        let temp_output = NamedTempFile::new().unwrap();

        // Move page from index 0 to index 3
        let result = move_pdf_page(temp_input.path(), temp_output.path(), 0, 3);
        assert!(result.is_ok());

        // Test invalid move (out of bounds)
        let result = move_pdf_page(temp_input.path(), temp_output.path(), 10, 2);
        assert!(result.is_err());
    }

    #[test]
    fn test_duplicate_pages_in_order() {
        use crate::{Document, Page};
        use tempfile::NamedTempFile;

        // Create a test PDF with 2 pages
        let mut doc = Document::new();
        doc.add_page(Page::a4());
        doc.add_page(Page::letter());

        let temp_file = NamedTempFile::new().unwrap();
        doc.save(temp_file.path()).unwrap();

        // Create order with duplicates [0, 1, 0, 1]
        let pdf_doc = PdfReader::open_document(temp_file.path()).unwrap();
        let options = ReorderOptions {
            page_order: vec![0, 1, 0, 1],
            preserve_metadata: true,
            optimize: false,
        };

        let reorderer = PageReorderer::new(pdf_doc, options);
        let result = reorderer.reorder();

        assert!(result.is_ok());
        let reordered_doc = result.unwrap();
        assert_eq!(reordered_doc.page_count(), 4); // Should have 4 pages now
    }

    #[test]
    fn test_single_page_reorder() {
        use crate::{Document, Page};
        use tempfile::NamedTempFile;

        // Create a test PDF with 1 page
        let mut doc = Document::new();
        doc.add_page(Page::a4());

        let temp_file = NamedTempFile::new().unwrap();
        doc.save(temp_file.path()).unwrap();

        // Reorder single page
        let pdf_doc = PdfReader::open_document(temp_file.path()).unwrap();
        let options = ReorderOptions {
            page_order: vec![0],
            preserve_metadata: true,
            optimize: false,
        };

        let reorderer = PageReorderer::new(pdf_doc, options);
        let result = reorderer.reorder();

        assert!(result.is_ok());
        let reordered_doc = result.unwrap();
        assert_eq!(reordered_doc.page_count(), 1);
    }
}

#[cfg(test)]
#[path = "reorder_tests.rs"]
mod reorder_tests;
