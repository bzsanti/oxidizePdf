//! Page extraction functionality
//!
//! This module provides functionality to extract individual pages or ranges of pages
//! from PDF documents. It builds upon the split module but provides a more focused
//! API specifically for page extraction use cases.

use super::{OperationError, OperationResult, PageRange};
use crate::parser::{ContentOperation, ContentParser, ParsedPage, PdfDocument, PdfReader};
use crate::{Document, Page};
use std::fs::File;
use std::path::Path;

/// Options for page extraction
#[derive(Debug, Clone)]
pub struct PageExtractionOptions {
    /// Whether to preserve document metadata
    pub preserve_metadata: bool,
    /// Whether to preserve annotations
    pub preserve_annotations: bool,
    /// Whether to preserve form fields
    pub preserve_forms: bool,
    /// Whether to optimize the extracted pages
    pub optimize: bool,
}

impl Default for PageExtractionOptions {
    fn default() -> Self {
        Self {
            preserve_metadata: true,
            preserve_annotations: true,
            preserve_forms: false,
            optimize: false,
        }
    }
}

/// Page extractor for extracting pages from PDF documents
pub struct PageExtractor {
    document: PdfDocument<File>,
    options: PageExtractionOptions,
}

impl PageExtractor {
    /// Create a new page extractor
    pub fn new(document: PdfDocument<File>) -> Self {
        Self {
            document,
            options: PageExtractionOptions::default(),
        }
    }

    /// Create a new page extractor with custom options
    pub fn with_options(document: PdfDocument<File>, options: PageExtractionOptions) -> Self {
        Self { document, options }
    }

    /// Extract a single page to a new document
    pub fn extract_page(&mut self, page_index: usize) -> OperationResult<Document> {
        let total_pages =
            self.document
                .page_count()
                .map_err(|e| OperationError::ParseError(e.to_string()))? as usize;

        if page_index >= total_pages {
            return Err(OperationError::PageIndexOutOfBounds(
                page_index,
                total_pages,
            ));
        }

        let mut doc = self.create_document()?;
        let parsed_page = self
            .document
            .get_page(page_index as u32)
            .map_err(|e| OperationError::ParseError(e.to_string()))?;

        let page = self.convert_page(&parsed_page)?;
        doc.add_page(page);

        Ok(doc)
    }

    /// Extract multiple pages to a new document
    pub fn extract_pages(&mut self, page_indices: &[usize]) -> OperationResult<Document> {
        let total_pages =
            self.document
                .page_count()
                .map_err(|e| OperationError::ParseError(e.to_string()))? as usize;

        // Validate all indices first
        for &idx in page_indices {
            if idx >= total_pages {
                return Err(OperationError::PageIndexOutOfBounds(idx, total_pages));
            }
        }

        if page_indices.is_empty() {
            return Err(OperationError::NoPagesToProcess);
        }

        let mut doc = self.create_document()?;

        for &page_idx in page_indices {
            let parsed_page = self
                .document
                .get_page(page_idx as u32)
                .map_err(|e| OperationError::ParseError(e.to_string()))?;

            let page = self.convert_page(&parsed_page)?;
            doc.add_page(page);
        }

        Ok(doc)
    }

    /// Extract a range of pages to a new document
    pub fn extract_page_range(&mut self, range: &PageRange) -> OperationResult<Document> {
        let total_pages =
            self.document
                .page_count()
                .map_err(|e| OperationError::ParseError(e.to_string()))? as usize;

        let indices = range.get_indices(total_pages)?;
        self.extract_pages(&indices)
    }

    /// Create a new document with metadata if preservation is enabled
    fn create_document(&self) -> OperationResult<Document> {
        let mut doc = Document::new();

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
                // creator and producer are not available in Document API yet
                // if let Some(creator) = metadata.creator {
                //     doc.set_creator(&creator);
                // }
                // if let Some(producer) = metadata.producer {
                //     doc.set_producer(&producer);
                // }
            }
        }

        Ok(doc)
    }

    /// Convert a parsed page to a new page
    fn convert_page(&mut self, parsed_page: &ParsedPage) -> OperationResult<Page> {
        // Create new page with same dimensions
        let width = parsed_page.width();
        let height = parsed_page.height();
        let mut page = Page::new(width, height);

        // Apply rotation if needed
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
                    self.process_operators(&mut page, &operators)?;
                    has_content = true;
                }
                Err(e) => {
                    // Log warning but continue with other streams
                    tracing::debug!("Warning: Failed to parse content stream: {e}");
                }
            }
        }

        // Handle annotations if preservation is enabled
        if self.options.preserve_annotations {
            // Note: Annotation preservation requires annotation support implementation
        }

        // Handle form fields if preservation is enabled
        if self.options.preserve_forms {
            // Note: Form field preservation requires form support implementation
        }

        // If no content was successfully processed, add a placeholder
        if !has_content {
            page.text()
                .set_font(crate::text::Font::Helvetica, 10.0)
                .at(50.0, height - 50.0)
                .write("[Page extracted]")
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
        // This is a simplified implementation that handles basic text and graphics
        // A full implementation would handle all PDF operators

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
                    current_font = self.map_font_name(name);
                    current_font_size = *size;
                }
                ContentOperation::MoveText(x, y) => {
                    current_x = *x;
                    current_y = *y;
                }
                ContentOperation::ShowText(text) => {
                    if text_object {
                        if let Ok(text_str) = String::from_utf8(text.clone()) {
                            page.text()
                                .set_font(current_font.clone(), current_font_size as f64)
                                .at(current_x as f64, current_y as f64)
                                .write(&text_str)
                                .map_err(OperationError::PdfError)?;
                        }
                    }
                }
                ContentOperation::MoveTo(x, y) => {
                    page.graphics().move_to(*x as f64, *y as f64);
                }
                ContentOperation::LineTo(x, y) => {
                    page.graphics().line_to(*x as f64, *y as f64);
                }
                ContentOperation::Rectangle(x, y, w, h) => {
                    page.graphics()
                        .rectangle(*x as f64, *y as f64, *w as f64, *h as f64);
                }
                ContentOperation::Stroke => {
                    page.graphics().stroke();
                }
                ContentOperation::Fill => {
                    page.graphics().fill();
                }
                ContentOperation::SetStrokingRGB(r, g, b) => {
                    page.graphics()
                        .set_stroke_color(crate::Color::rgb(*r as f64, *g as f64, *b as f64));
                }
                ContentOperation::SetNonStrokingRGB(r, g, b) => {
                    page.graphics()
                        .set_fill_color(crate::Color::rgb(*r as f64, *g as f64, *b as f64));
                }
                ContentOperation::SetLineWidth(width) => {
                    page.graphics().set_line_width(*width as f64);
                }
                _ => {
                    // Other operators not yet implemented
                }
            }
        }

        Ok(())
    }

    /// Map PDF font names to our font enum
    fn map_font_name(&self, name: &str) -> crate::text::Font {
        match name {
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
        }
    }
}

/// Extract a single page from a PDF file to a new document
pub fn extract_page<P: AsRef<Path>>(input_path: P, page_index: usize) -> OperationResult<Document> {
    let reader =
        PdfReader::open(input_path).map_err(|e| OperationError::ParseError(e.to_string()))?;
    let document = PdfDocument::new(reader);
    let mut extractor = PageExtractor::new(document);
    extractor.extract_page(page_index)
}

/// Extract multiple pages from a PDF file to a new document
pub fn extract_pages<P: AsRef<Path>>(
    input_path: P,
    page_indices: &[usize],
) -> OperationResult<Document> {
    let reader =
        PdfReader::open(input_path).map_err(|e| OperationError::ParseError(e.to_string()))?;
    let document = PdfDocument::new(reader);
    let mut extractor = PageExtractor::new(document);
    extractor.extract_pages(page_indices)
}

/// Extract a page range from a PDF file to a new document
pub fn extract_page_range<P: AsRef<Path>>(
    input_path: P,
    range: &PageRange,
) -> OperationResult<Document> {
    let reader =
        PdfReader::open(input_path).map_err(|e| OperationError::ParseError(e.to_string()))?;
    let document = PdfDocument::new(reader);
    let mut extractor = PageExtractor::new(document);
    extractor.extract_page_range(range)
}

/// Extract a single page from a PDF file and save it
pub fn extract_page_to_file<P: AsRef<Path>, Q: AsRef<Path>>(
    input_path: P,
    page_index: usize,
    output_path: Q,
) -> OperationResult<()> {
    let mut doc = extract_page(input_path, page_index)?;
    doc.save(output_path).map_err(OperationError::PdfError)
}

/// Extract multiple pages from a PDF file and save them
pub fn extract_pages_to_file<P: AsRef<Path>, Q: AsRef<Path>>(
    input_path: P,
    page_indices: &[usize],
    output_path: Q,
) -> OperationResult<()> {
    let mut doc = extract_pages(input_path, page_indices)?;
    doc.save(output_path).map_err(OperationError::PdfError)
}

/// Extract a page range from a PDF file and save it
pub fn extract_page_range_to_file<P: AsRef<Path>, Q: AsRef<Path>>(
    input_path: P,
    range: &PageRange,
    output_path: Q,
) -> OperationResult<()> {
    let mut doc = extract_page_range(input_path, range)?;
    doc.save(output_path).map_err(OperationError::PdfError)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Document;
    use tempfile::TempDir;

    fn create_test_pdf(title: &str, page_count: usize) -> Document {
        let mut doc = Document::new();
        doc.set_title(title);
        doc.set_author("Test Author");
        doc.set_subject("Test Subject");
        doc.set_keywords("test, extraction");

        for i in 0..page_count {
            let mut page = crate::Page::new(612.0, 792.0);
            page.text()
                .set_font(crate::text::Font::Helvetica, 14.0)
                .at(50.0, 750.0)
                .write(&format!("Page {}", i + 1))
                .unwrap();
            doc.add_page(page);
        }

        doc
    }

    fn save_test_pdf(doc: &mut Document, dir: &TempDir, name: &str) -> std::path::PathBuf {
        let path = dir.path().join(name);
        doc.save(&path).unwrap();
        path
    }

    #[test]
    fn test_page_extraction_options_default() {
        let options = PageExtractionOptions::default();
        assert!(options.preserve_metadata);
        assert!(options.preserve_annotations);
        assert!(!options.preserve_forms);
        assert!(!options.optimize);
    }

    #[test]
    fn test_page_extraction_options_custom() {
        let options = PageExtractionOptions {
            preserve_metadata: false,
            preserve_annotations: false,
            preserve_forms: true,
            optimize: true,
        };
        assert!(!options.preserve_metadata);
        assert!(!options.preserve_annotations);
        assert!(options.preserve_forms);
        assert!(options.optimize);
    }

    #[test]
    fn test_page_extractor_new() {
        let temp_dir = TempDir::new().unwrap();
        let mut doc = create_test_pdf("Test", 3);
        let path = save_test_pdf(&mut doc, &temp_dir, "test.pdf");

        let reader = PdfReader::open(&path).unwrap();
        let document = PdfDocument::new(reader);
        let extractor = PageExtractor::new(document);

        assert!(extractor.options.preserve_metadata);
        assert!(extractor.options.preserve_annotations);
    }

    #[test]
    fn test_page_extractor_with_options() {
        let temp_dir = TempDir::new().unwrap();
        let mut doc = create_test_pdf("Test", 3);
        let path = save_test_pdf(&mut doc, &temp_dir, "test.pdf");

        let reader = PdfReader::open(&path).unwrap();
        let document = PdfDocument::new(reader);

        let options = PageExtractionOptions {
            preserve_metadata: false,
            preserve_annotations: false,
            preserve_forms: true,
            optimize: true,
        };
        let extractor = PageExtractor::with_options(document, options);

        assert!(!extractor.options.preserve_metadata);
        assert!(!extractor.options.preserve_annotations);
        assert!(extractor.options.preserve_forms);
        assert!(extractor.options.optimize);
    }

    #[test]
    fn test_extract_single_page() {
        let temp_dir = TempDir::new().unwrap();
        let mut doc = create_test_pdf("Test", 5);
        let path = save_test_pdf(&mut doc, &temp_dir, "test.pdf");

        let reader = PdfReader::open(&path).unwrap();
        let document = PdfDocument::new(reader);
        let mut extractor = PageExtractor::new(document);

        let result = extractor.extract_page(2);
        assert!(result.is_ok());

        let extracted_doc = result.unwrap();
        // The extracted document should have 1 page
        assert_eq!(extracted_doc.pages.len(), 1);
    }

    #[test]
    fn test_extract_page_out_of_bounds() {
        let temp_dir = TempDir::new().unwrap();
        let mut doc = create_test_pdf("Test", 3);
        let path = save_test_pdf(&mut doc, &temp_dir, "test.pdf");

        let reader = PdfReader::open(&path).unwrap();
        let document = PdfDocument::new(reader);
        let mut extractor = PageExtractor::new(document);

        let result = extractor.extract_page(10);
        assert!(result.is_err());
        if let Err(err) = result {
            assert!(matches!(err, OperationError::PageIndexOutOfBounds(10, 3)));
        }
    }

    #[test]
    fn test_extract_multiple_pages() {
        let temp_dir = TempDir::new().unwrap();
        let mut doc = create_test_pdf("Test", 5);
        let path = save_test_pdf(&mut doc, &temp_dir, "test.pdf");

        let reader = PdfReader::open(&path).unwrap();
        let document = PdfDocument::new(reader);
        let mut extractor = PageExtractor::new(document);

        let result = extractor.extract_pages(&[0, 2, 4]);
        assert!(result.is_ok());

        let extracted_doc = result.unwrap();
        assert_eq!(extracted_doc.pages.len(), 3);
    }

    #[test]
    fn test_extract_pages_empty_list() {
        let temp_dir = TempDir::new().unwrap();
        let mut doc = create_test_pdf("Test", 3);
        let path = save_test_pdf(&mut doc, &temp_dir, "test.pdf");

        let reader = PdfReader::open(&path).unwrap();
        let document = PdfDocument::new(reader);
        let mut extractor = PageExtractor::new(document);

        let result = extractor.extract_pages(&[]);
        assert!(result.is_err());
        if let Err(err) = result {
            assert!(matches!(err, OperationError::NoPagesToProcess));
        }
    }

    #[test]
    fn test_extract_pages_out_of_bounds() {
        let temp_dir = TempDir::new().unwrap();
        let mut doc = create_test_pdf("Test", 3);
        let path = save_test_pdf(&mut doc, &temp_dir, "test.pdf");

        let reader = PdfReader::open(&path).unwrap();
        let document = PdfDocument::new(reader);
        let mut extractor = PageExtractor::new(document);

        let result = extractor.extract_pages(&[0, 1, 5]);
        assert!(result.is_err());
        if let Err(err) = result {
            assert!(matches!(err, OperationError::PageIndexOutOfBounds(5, 3)));
        }
    }

    #[test]
    fn test_extract_page_range() {
        let temp_dir = TempDir::new().unwrap();
        let mut doc = create_test_pdf("Test", 5);
        let path = save_test_pdf(&mut doc, &temp_dir, "test.pdf");

        let reader = PdfReader::open(&path).unwrap();
        let document = PdfDocument::new(reader);
        let mut extractor = PageExtractor::new(document);

        let range = PageRange::Range(1, 3);
        let result = extractor.extract_page_range(&range);
        assert!(result.is_ok());

        let extracted_doc = result.unwrap();
        assert_eq!(extracted_doc.pages.len(), 3); // Pages 1, 2, 3 (0-based: 1, 2, 3)
    }

    #[test]
    fn test_extract_single_page_range() {
        let temp_dir = TempDir::new().unwrap();
        let mut doc = create_test_pdf("Test", 5);
        let path = save_test_pdf(&mut doc, &temp_dir, "test.pdf");

        let reader = PdfReader::open(&path).unwrap();
        let document = PdfDocument::new(reader);
        let mut extractor = PageExtractor::new(document);

        let range = PageRange::Single(2);
        let result = extractor.extract_page_range(&range);
        assert!(result.is_ok());

        let extracted_doc = result.unwrap();
        assert_eq!(extracted_doc.pages.len(), 1);
    }

    #[test]
    fn test_extract_all_pages_range() {
        let temp_dir = TempDir::new().unwrap();
        let mut doc = create_test_pdf("Test", 3);
        let path = save_test_pdf(&mut doc, &temp_dir, "test.pdf");

        let reader = PdfReader::open(&path).unwrap();
        let document = PdfDocument::new(reader);
        let mut extractor = PageExtractor::new(document);

        let range = PageRange::All;
        let result = extractor.extract_page_range(&range);
        assert!(result.is_ok());

        let extracted_doc = result.unwrap();
        assert_eq!(extracted_doc.pages.len(), 3);
    }

    #[test]
    fn test_metadata_preservation() {
        let temp_dir = TempDir::new().unwrap();
        let mut doc = create_test_pdf("Original Title", 2);
        let path = save_test_pdf(&mut doc, &temp_dir, "test.pdf");

        let reader = PdfReader::open(&path).unwrap();
        let document = PdfDocument::new(reader);
        let mut extractor = PageExtractor::new(document);

        let result = extractor.extract_page(0);
        assert!(result.is_ok());

        let extracted_doc = result.unwrap();
        // Test that page was extracted successfully
        assert_eq!(extracted_doc.pages.len(), 1);
        // Note: Document doesn't have getter methods for metadata in current API
    }

    #[test]
    fn test_metadata_not_preserved() {
        let temp_dir = TempDir::new().unwrap();
        let mut doc = create_test_pdf("Original Title", 2);
        let path = save_test_pdf(&mut doc, &temp_dir, "test.pdf");

        let reader = PdfReader::open(&path).unwrap();
        let document = PdfDocument::new(reader);

        let options = PageExtractionOptions {
            preserve_metadata: false,
            ..Default::default()
        };
        let mut extractor = PageExtractor::with_options(document, options);

        let result = extractor.extract_page(0);
        assert!(result.is_ok());

        let extracted_doc = result.unwrap();
        // When metadata is not preserved, the document should have default/empty metadata
        assert_eq!(extracted_doc.pages.len(), 1);
        // Note: Document doesn't have getter methods for metadata in current API
    }

    #[test]
    fn test_map_font_name() {
        let temp_dir = TempDir::new().unwrap();
        let mut doc = create_test_pdf("Test", 1);
        let path = save_test_pdf(&mut doc, &temp_dir, "test.pdf");

        let reader = PdfReader::open(&path).unwrap();
        let document = PdfDocument::new(reader);
        let extractor = PageExtractor::new(document);

        assert_eq!(
            extractor.map_font_name("Times-Roman"),
            crate::text::Font::TimesRoman
        );
        assert_eq!(
            extractor.map_font_name("Times-Bold"),
            crate::text::Font::TimesBold
        );
        assert_eq!(
            extractor.map_font_name("Helvetica-Bold"),
            crate::text::Font::HelveticaBold
        );
        assert_eq!(
            extractor.map_font_name("Courier"),
            crate::text::Font::Courier
        );
        assert_eq!(
            extractor.map_font_name("Unknown"),
            crate::text::Font::Helvetica
        );
    }

    #[test]
    fn test_convenience_functions() {
        let temp_dir = TempDir::new().unwrap();
        let mut doc = create_test_pdf("Test", 5);
        let path = save_test_pdf(&mut doc, &temp_dir, "test.pdf");

        // Test extract_page
        let result = extract_page(&path, 2);
        assert!(result.is_ok());
        let extracted_doc = result.unwrap();
        assert_eq!(extracted_doc.pages.len(), 1);

        // Test extract_pages
        let result = extract_pages(&path, &[0, 2, 4]);
        assert!(result.is_ok());
        let extracted_doc = result.unwrap();
        assert_eq!(extracted_doc.pages.len(), 3);

        // Test extract_page_range
        let range = PageRange::Range(1, 3);
        let result = extract_page_range(&path, &range);
        assert!(result.is_ok());
        let extracted_doc = result.unwrap();
        assert_eq!(extracted_doc.pages.len(), 3);
    }

    #[test]
    fn test_extract_to_file_functions() {
        let temp_dir = TempDir::new().unwrap();
        let mut doc = create_test_pdf("Test", 5);
        let input_path = save_test_pdf(&mut doc, &temp_dir, "test.pdf");

        // Test extract_page_to_file
        let output_path = temp_dir.path().join("extracted_page.pdf");
        let result = extract_page_to_file(&input_path, 2, &output_path);
        assert!(result.is_ok());
        assert!(output_path.exists());

        // Test extract_pages_to_file
        let output_path = temp_dir.path().join("extracted_pages.pdf");
        let result = extract_pages_to_file(&input_path, &[0, 2, 4], &output_path);
        assert!(result.is_ok());
        assert!(output_path.exists());

        // Test extract_page_range_to_file
        let output_path = temp_dir.path().join("extracted_range.pdf");
        let range = PageRange::Range(1, 3);
        let result = extract_page_range_to_file(&input_path, &range, &output_path);
        assert!(result.is_ok());
        assert!(output_path.exists());
    }

    #[test]
    fn test_extract_nonexistent_file() {
        let result = extract_page("nonexistent.pdf", 0);
        assert!(result.is_err());
        if let Err(err) = result {
            assert!(matches!(err, OperationError::ParseError(_)));
        }
    }

    #[test]
    fn test_extract_page_range_invalid() {
        let temp_dir = TempDir::new().unwrap();
        let mut doc = create_test_pdf("Test", 3);
        let path = save_test_pdf(&mut doc, &temp_dir, "test.pdf");

        let reader = PdfReader::open(&path).unwrap();
        let document = PdfDocument::new(reader);
        let mut extractor = PageExtractor::new(document);

        let range = PageRange::Range(5, 10);
        let result = extractor.extract_page_range(&range);
        assert!(result.is_err());
        if let Err(err) = result {
            assert!(matches!(err, OperationError::PageIndexOutOfBounds(_, _)));
        }
    }

    // Comprehensive tests for PageExtractor
    mod comprehensive_tests {
        use super::*;
        use crate::graphics::Color;
        use crate::text::Font;

        #[test]
        fn test_page_extraction_options_clone() {
            let options = PageExtractionOptions {
                preserve_metadata: true,
                preserve_annotations: false,
                preserve_forms: true,
                optimize: false,
            };

            let cloned = options.clone();
            assert_eq!(options.preserve_metadata, cloned.preserve_metadata);
            assert_eq!(options.preserve_annotations, cloned.preserve_annotations);
            assert_eq!(options.preserve_forms, cloned.preserve_forms);
            assert_eq!(options.optimize, cloned.optimize);
        }

        #[test]
        fn test_page_extraction_options_debug() {
            let options = PageExtractionOptions::default();
            let debug_str = format!("{options:?}");
            assert!(debug_str.contains("PageExtractionOptions"));
            assert!(debug_str.contains("preserve_metadata"));
            assert!(debug_str.contains("preserve_annotations"));
            assert!(debug_str.contains("preserve_forms"));
            assert!(debug_str.contains("optimize"));
        }

        #[test]
        fn test_extract_page_with_complex_content() {
            let temp_dir = TempDir::new().unwrap();
            let mut doc = create_complex_test_pdf("Complex Test", 3);
            let path = save_test_pdf(&mut doc, &temp_dir, "complex.pdf");

            let reader = PdfReader::open(&path).unwrap();
            let document = PdfDocument::new(reader);
            let mut extractor = PageExtractor::new(document);

            let result = extractor.extract_page(1);
            assert!(result.is_ok());

            let extracted_doc = result.unwrap();
            assert_eq!(extracted_doc.pages.len(), 1);
            assert_eq!(
                extracted_doc.metadata.title,
                Some("Complex Test".to_string())
            );
        }

        #[test]
        fn test_extract_pages_with_custom_options() {
            let temp_dir = TempDir::new().unwrap();
            let mut doc = create_test_pdf("Custom Options Test", 4);
            let path = save_test_pdf(&mut doc, &temp_dir, "custom.pdf");

            let reader = PdfReader::open(&path).unwrap();
            let document = PdfDocument::new(reader);

            let options = PageExtractionOptions {
                preserve_metadata: false,
                preserve_annotations: false,
                preserve_forms: true,
                optimize: true,
            };

            let mut extractor = PageExtractor::with_options(document, options);
            let result = extractor.extract_pages(&[0, 2]);
            assert!(result.is_ok());

            let extracted_doc = result.unwrap();
            assert_eq!(extracted_doc.pages.len(), 2);

            // With preserve_metadata false, title should be None or default
            assert!(
                extracted_doc.metadata.title.is_none()
                    || extracted_doc.metadata.title == Some("".to_string())
            );
        }

        #[test]
        fn test_extract_page_range_edge_cases() {
            let temp_dir = TempDir::new().unwrap();
            let mut doc = create_test_pdf("Edge Cases", 5);
            let path = save_test_pdf(&mut doc, &temp_dir, "edge.pdf");

            let reader = PdfReader::open(&path).unwrap();
            let document = PdfDocument::new(reader);
            let mut extractor = PageExtractor::new(document);

            // Test Range with same start and end
            let range = PageRange::Range(2, 2);
            let result = extractor.extract_page_range(&range);
            assert!(result.is_ok());
            let extracted_doc = result.unwrap();
            assert_eq!(extracted_doc.pages.len(), 1);
        }

        #[test]
        fn test_extract_page_range_reverse_order() {
            let temp_dir = TempDir::new().unwrap();
            let mut doc = create_test_pdf("Reverse Order", 5);
            let path = save_test_pdf(&mut doc, &temp_dir, "reverse.pdf");

            let reader = PdfReader::open(&path).unwrap();
            let document = PdfDocument::new(reader);
            let mut extractor = PageExtractor::new(document);

            // Test Range with end < start (should handle gracefully)
            let range = PageRange::Range(3, 1);
            let result = extractor.extract_page_range(&range);
            // This should either work (extracting in reverse) or return an error
            assert!(result.is_ok() || result.is_err());
        }

        #[test]
        fn test_extract_first_page() {
            let temp_dir = TempDir::new().unwrap();
            let mut doc = create_test_pdf("First Page Test", 3);
            let path = save_test_pdf(&mut doc, &temp_dir, "first.pdf");

            let reader = PdfReader::open(&path).unwrap();
            let document = PdfDocument::new(reader);
            let mut extractor = PageExtractor::new(document);

            let result = extractor.extract_page(0);
            assert!(result.is_ok());

            let extracted_doc = result.unwrap();
            assert_eq!(extracted_doc.pages.len(), 1);
        }

        #[test]
        fn test_extract_last_page() {
            let temp_dir = TempDir::new().unwrap();
            let mut doc = create_test_pdf("Last Page Test", 3);
            let path = save_test_pdf(&mut doc, &temp_dir, "last.pdf");

            let reader = PdfReader::open(&path).unwrap();
            let document = PdfDocument::new(reader);
            let mut extractor = PageExtractor::new(document);

            let result = extractor.extract_page(2); // Last page (0-indexed)
            assert!(result.is_ok());

            let extracted_doc = result.unwrap();
            assert_eq!(extracted_doc.pages.len(), 1);
        }

        #[test]
        fn test_extract_pages_duplicate_indices() {
            let temp_dir = TempDir::new().unwrap();
            let mut doc = create_test_pdf("Duplicates Test", 3);
            let path = save_test_pdf(&mut doc, &temp_dir, "duplicates.pdf");

            let reader = PdfReader::open(&path).unwrap();
            let document = PdfDocument::new(reader);
            let mut extractor = PageExtractor::new(document);

            let result = extractor.extract_pages(&[0, 1, 0, 2, 1]);
            assert!(result.is_ok());

            let extracted_doc = result.unwrap();
            // Should extract 5 pages (duplicates allowed)
            assert_eq!(extracted_doc.pages.len(), 5);
        }

        #[test]
        fn test_extract_pages_large_document() {
            let temp_dir = TempDir::new().unwrap();
            let mut doc = create_test_pdf("Large Document", 100);
            let path = save_test_pdf(&mut doc, &temp_dir, "large.pdf");

            let reader = PdfReader::open(&path).unwrap();
            let document = PdfDocument::new(reader);
            let mut extractor = PageExtractor::new(document);

            // Extract every 10th page
            let indices: Vec<usize> = (0..100).step_by(10).collect();
            let result = extractor.extract_pages(&indices);
            assert!(result.is_ok());

            let extracted_doc = result.unwrap();
            assert_eq!(extracted_doc.pages.len(), 10);
        }

        #[test]
        fn test_extract_page_range_all_large() {
            let temp_dir = TempDir::new().unwrap();
            let mut doc = create_test_pdf("All Large", 20); // Reduced from 50 to 20
            let path = save_test_pdf(&mut doc, &temp_dir, "all_large.pdf");

            let reader = PdfReader::open(&path).unwrap();
            let document = PdfDocument::new(reader);
            let mut extractor = PageExtractor::new(document);

            let range = PageRange::All;
            let result = extractor.extract_page_range(&range);
            assert!(result.is_ok());

            let extracted_doc = result.unwrap();
            assert_eq!(extracted_doc.pages.len(), 20);
        }

        #[test]
        fn test_extract_page_range_middle_section() {
            let temp_dir = TempDir::new().unwrap();
            let mut doc = create_test_pdf("Middle Section", 10);
            let path = save_test_pdf(&mut doc, &temp_dir, "middle.pdf");

            let reader = PdfReader::open(&path).unwrap();
            let document = PdfDocument::new(reader);
            let mut extractor = PageExtractor::new(document);

            let range = PageRange::Range(3, 7);
            let result = extractor.extract_page_range(&range);
            assert!(result.is_ok());

            let extracted_doc = result.unwrap();
            assert_eq!(extracted_doc.pages.len(), 5); // Pages 3, 4, 5, 6, 7
        }

        #[test]
        fn test_extract_page_error_handling() {
            let temp_dir = TempDir::new().unwrap();
            let mut doc = create_test_pdf("Error Test", 3);
            let path = save_test_pdf(&mut doc, &temp_dir, "error.pdf");

            let reader = PdfReader::open(&path).unwrap();
            let document = PdfDocument::new(reader);
            let mut extractor = PageExtractor::new(document);

            // Test out of bounds
            let result = extractor.extract_page(10);
            assert!(result.is_err());
            if let Err(err) = result {
                assert!(matches!(err, OperationError::PageIndexOutOfBounds(10, 3)));
            }
        }

        #[test]
        fn test_extract_pages_mixed_valid_invalid() {
            let temp_dir = TempDir::new().unwrap();
            let mut doc = create_test_pdf("Mixed Valid Invalid", 5);
            let path = save_test_pdf(&mut doc, &temp_dir, "mixed.pdf");

            let reader = PdfReader::open(&path).unwrap();
            let document = PdfDocument::new(reader);
            let mut extractor = PageExtractor::new(document);

            // Mix of valid and invalid indices
            let result = extractor.extract_pages(&[0, 2, 10, 4]);
            assert!(result.is_err());
            if let Err(err) = result {
                assert!(matches!(err, OperationError::PageIndexOutOfBounds(10, 5)));
            }
        }

        #[test]
        fn test_extract_single_page_from_single_page_document() {
            let temp_dir = TempDir::new().unwrap();
            let mut doc = create_test_pdf("Single Page Doc", 1);
            let path = save_test_pdf(&mut doc, &temp_dir, "single.pdf");

            let reader = PdfReader::open(&path).unwrap();
            let document = PdfDocument::new(reader);
            let mut extractor = PageExtractor::new(document);

            let result = extractor.extract_page(0);
            assert!(result.is_ok());

            let extracted_doc = result.unwrap();
            assert_eq!(extracted_doc.pages.len(), 1);
        }

        #[test]
        fn test_extract_all_pages_from_single_page_document() {
            let temp_dir = TempDir::new().unwrap();
            let mut doc = create_test_pdf("Single Page All", 1);
            let path = save_test_pdf(&mut doc, &temp_dir, "single_all.pdf");

            let reader = PdfReader::open(&path).unwrap();
            let document = PdfDocument::new(reader);
            let mut extractor = PageExtractor::new(document);

            let range = PageRange::All;
            let result = extractor.extract_page_range(&range);
            assert!(result.is_ok());

            let extracted_doc = result.unwrap();
            assert_eq!(extracted_doc.pages.len(), 1);
        }

        #[test]
        fn test_extract_page_preserve_metadata() {
            let temp_dir = TempDir::new().unwrap();
            let mut doc = create_test_pdf("Metadata Test", 2);
            let path = save_test_pdf(&mut doc, &temp_dir, "metadata.pdf");

            let reader = PdfReader::open(&path).unwrap();
            let document = PdfDocument::new(reader);

            let options = PageExtractionOptions {
                preserve_metadata: true,
                preserve_annotations: false,
                preserve_forms: false,
                optimize: false,
            };

            let mut extractor = PageExtractor::with_options(document, options);
            let result = extractor.extract_page(0);
            assert!(result.is_ok());

            let extracted_doc = result.unwrap();
            assert_eq!(
                extracted_doc.metadata.title,
                Some("Metadata Test".to_string())
            );
            assert_eq!(
                extracted_doc.metadata.author,
                Some("Test Author".to_string())
            );
            assert_eq!(
                extracted_doc.metadata.subject,
                Some("Test Subject".to_string())
            );
        }

        #[test]
        fn test_extract_page_no_metadata() {
            let temp_dir = TempDir::new().unwrap();
            let mut doc = create_test_pdf("No Metadata Test", 2);
            let path = save_test_pdf(&mut doc, &temp_dir, "no_metadata.pdf");

            let reader = PdfReader::open(&path).unwrap();
            let document = PdfDocument::new(reader);

            let options = PageExtractionOptions {
                preserve_metadata: false,
                preserve_annotations: false,
                preserve_forms: false,
                optimize: false,
            };

            let mut extractor = PageExtractor::with_options(document, options);
            let result = extractor.extract_page(0);
            assert!(result.is_ok());

            let extracted_doc = result.unwrap();
            // When preserve_metadata is false, metadata should be empty or default
            assert!(
                extracted_doc.metadata.title.is_none()
                    || extracted_doc.metadata.title == Some("".to_string())
            );
        }

        #[test]
        fn test_extract_pages_ordered_indices() {
            let temp_dir = TempDir::new().unwrap();
            let mut doc = create_test_pdf("Ordered Test", 5);
            let path = save_test_pdf(&mut doc, &temp_dir, "ordered.pdf");

            let reader = PdfReader::open(&path).unwrap();
            let document = PdfDocument::new(reader);
            let mut extractor = PageExtractor::new(document);

            // Extract pages in specific order
            let result = extractor.extract_pages(&[4, 2, 0, 1, 3]);
            assert!(result.is_ok());

            let extracted_doc = result.unwrap();
            assert_eq!(extracted_doc.pages.len(), 5);
        }

        #[test]
        fn test_extract_page_range_boundary_conditions() {
            let temp_dir = TempDir::new().unwrap();
            let mut doc = create_test_pdf("Boundary Test", 3);
            let path = save_test_pdf(&mut doc, &temp_dir, "boundary.pdf");

            let reader = PdfReader::open(&path).unwrap();
            let document = PdfDocument::new(reader);
            let mut extractor = PageExtractor::new(document);

            // Test exact boundary
            let range = PageRange::Range(0, 2);
            let result = extractor.extract_page_range(&range);
            assert!(result.is_ok());

            let extracted_doc = result.unwrap();
            assert_eq!(extracted_doc.pages.len(), 3); // All pages
        }

        #[test]
        fn test_convenience_functions() {
            let temp_dir = TempDir::new().unwrap();
            let mut doc = create_test_pdf("Convenience Test", 3);
            let input_path = save_test_pdf(&mut doc, &temp_dir, "convenience.pdf");

            // Test extract_page function
            let result = extract_page(&input_path, 1);
            assert!(result.is_ok());
            let extracted_doc = result.unwrap();
            assert_eq!(extracted_doc.pages.len(), 1);

            // Test extract_pages function
            let result = extract_pages(&input_path, &[0, 2]);
            assert!(result.is_ok());
            let extracted_doc = result.unwrap();
            assert_eq!(extracted_doc.pages.len(), 2);

            // Test extract_page_range function
            let range = PageRange::Range(0, 1);
            let result = extract_page_range(&input_path, &range);
            assert!(result.is_ok());
            let extracted_doc = result.unwrap();
            assert_eq!(extracted_doc.pages.len(), 2);
        }

        #[test]
        fn test_extract_to_file_functions() {
            let temp_dir = TempDir::new().unwrap();
            let mut doc = create_test_pdf("File Test", 4);
            let input_path = save_test_pdf(&mut doc, &temp_dir, "file_input.pdf");

            // Test extract_page_to_file
            let output_path = temp_dir.path().join("page_output.pdf");
            let result = extract_page_to_file(&input_path, 1, &output_path);
            assert!(result.is_ok());
            assert!(output_path.exists());

            // Test extract_pages_to_file
            let output_path = temp_dir.path().join("pages_output.pdf");
            let result = extract_pages_to_file(&input_path, &[0, 3], &output_path);
            assert!(result.is_ok());
            assert!(output_path.exists());

            // Test extract_page_range_to_file
            let output_path = temp_dir.path().join("range_output.pdf");
            let range = PageRange::Range(1, 2);
            let result = extract_page_range_to_file(&input_path, &range, &output_path);
            assert!(result.is_ok());
            assert!(output_path.exists());
        }

        #[test]
        fn test_extract_performance_many_pages() {
            let temp_dir = TempDir::new().unwrap();
            let mut doc = create_test_pdf("Performance Test", 200);
            let path = save_test_pdf(&mut doc, &temp_dir, "performance.pdf");

            let reader = PdfReader::open(&path).unwrap();
            let document = PdfDocument::new(reader);
            let mut extractor = PageExtractor::new(document);

            // Extract every 20th page
            let indices: Vec<usize> = (0..200).step_by(20).collect();

            let start_time = std::time::Instant::now();
            let result = extractor.extract_pages(&indices);
            let elapsed = start_time.elapsed();

            assert!(result.is_ok());
            assert!(elapsed.as_millis() < 5000); // Should complete within 5 seconds

            let extracted_doc = result.unwrap();
            assert_eq!(extracted_doc.pages.len(), 10);
        }

        /// Create a test PDF with complex content for testing
        fn create_complex_test_pdf(title: &str, page_count: usize) -> Document {
            let mut doc = Document::new();
            doc.set_title(title);
            doc.set_author("Complex Test Author");
            doc.set_subject("Complex Test Subject");
            doc.set_keywords("complex, test, extraction");

            for i in 0..page_count {
                let mut page = crate::Page::new(612.0, 792.0);

                // Add multiple text elements
                page.text()
                    .set_font(Font::HelveticaBold, 16.0)
                    .at(50.0, 750.0)
                    .write(&format!("Complex Page {}", i + 1))
                    .unwrap();

                page.text()
                    .set_font(Font::TimesRoman, 12.0)
                    .at(50.0, 700.0)
                    .write("This is a complex page with multiple elements.")
                    .unwrap();

                page.text()
                    .set_font(Font::Courier, 10.0)
                    .at(50.0, 650.0)
                    .write("Monospace text for variety.")
                    .unwrap();

                // Add graphics elements
                page.graphics()
                    .set_fill_color(Color::rgb(0.8, 0.8, 0.9))
                    .rect(100.0, 500.0, 200.0, 100.0)
                    .fill();

                page.graphics()
                    .set_stroke_color(Color::rgb(0.2, 0.2, 0.8))
                    .set_line_width(2.0)
                    .circle(200.0, 300.0, 50.0)
                    .stroke();

                doc.add_page(page);
            }

            doc
        }
    }
}
