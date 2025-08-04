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
}

#[cfg(test)]
#[path = "reorder_tests.rs"]
mod reorder_tests;
