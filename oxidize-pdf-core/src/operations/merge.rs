//! PDF merging functionality
//!
//! This module provides functionality to merge multiple PDF documents into a single file.

use super::{OperationError, OperationResult, PageRange};
use crate::parser::page_tree::ParsedPage;
use crate::parser::{ContentOperation, ContentParser, PdfDocument, PdfReader};
use crate::{Document, Page};
use std::collections::HashMap;
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
    /// Object number mapping for each input document
    object_mappings: Vec<HashMap<u32, u32>>,
    /// Font name mapping for each input document (original name -> new unique name)
    font_mappings: Vec<HashMap<String, String>>,
    /// XObject name mapping for each input document (original name -> new unique name)
    xobject_mappings: Vec<HashMap<String, String>>,
    /// Next available object number
    #[allow(dead_code)]
    next_object_num: u32,
    /// Next available font index for unique naming
    next_font_index: u32,
    /// Next available XObject index for unique naming
    next_xobject_index: u32,
}

impl PdfMerger {
    /// Create a new PDF merger
    pub fn new(options: MergeOptions) -> Self {
        Self {
            inputs: Vec::new(),
            options,
            object_mappings: Vec::new(),
            font_mappings: Vec::new(),
            xobject_mappings: Vec::new(),
            next_object_num: 1,
            next_font_index: 1,
            next_xobject_index: 1,
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

        // Initialize font and XObject mappings for each input
        self.font_mappings.clear();
        self.xobject_mappings.clear();
        for _ in &self.inputs {
            self.font_mappings.push(HashMap::new());
            self.xobject_mappings.push(HashMap::new());
        }

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

            // Initialize object mapping for this document
            self.object_mappings.push(HashMap::new());

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

                let page = self.convert_page_for_merge(&parsed_page, &document, input_idx)?;
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

    /// Convert a page for merging, handling object renumbering
    fn convert_page_for_merge(
        &mut self,
        parsed_page: &ParsedPage,
        document: &PdfDocument<File>,
        input_idx: usize,
    ) -> OperationResult<Page> {
        // Create new page with same dimensions
        let width = parsed_page.width();
        let height = parsed_page.height();
        let mut page = Page::new(width, height);

        // Extract and map fonts from page resources
        if let Some(resources) = parsed_page.get_resources() {
            // Map fonts
            if let Some(fonts_dict) = resources.get("Font").and_then(|f| f.as_dict()) {
                for (font_name, _font_obj) in fonts_dict.0.iter() {
                    // Map font names to unique names for this merge
                    let font_name_str = font_name.0.as_str();
                    if !self.font_mappings[input_idx].contains_key(font_name_str) {
                        let new_font_name = format!("MF{}", self.next_font_index);
                        self.font_mappings[input_idx].insert(font_name_str.to_string(), new_font_name);
                        self.next_font_index += 1;
                    }
                }
            }
            
            // Map XObjects (images, forms)
            if let Some(xobjects_dict) = resources.get("XObject").and_then(|x| x.as_dict()) {
                for (xobject_name, _xobject_obj) in xobjects_dict.0.iter() {
                    // Map XObject names to unique names for this merge
                    let xobject_name_str = xobject_name.0.as_str();
                    if !self.xobject_mappings[input_idx].contains_key(xobject_name_str) {
                        let new_xobject_name = format!("MX{}", self.next_xobject_index);
                        self.xobject_mappings[input_idx].insert(xobject_name_str.to_string(), new_xobject_name);
                        self.next_xobject_index += 1;
                    }
                }
            }
        }

        // Get content streams
        let content_streams = document
            .get_page_content_streams(parsed_page)
            .map_err(|e| OperationError::ParseError(e.to_string()))?;

        // Parse and process content streams
        let mut has_content = false;
        for stream_data in &content_streams {
            match ContentParser::parse_content(stream_data) {
                Ok(operators) => {
                    // Process the operators to recreate content
                    // Note: In a full implementation, we would need to handle object
                    // reference renumbering for resources like fonts and images
                    self.process_operators_for_merge(&mut page, &operators, input_idx)?;
                    has_content = true;
                }
                Err(e) => {
                    eprintln!(
                        "Warning: Failed to parse content stream from document {}: {}",
                        input_idx + 1,
                        e
                    );
                }
            }
        }

        // If no content was successfully processed, add a placeholder
        if !has_content {
            page.text()
                .set_font(crate::text::Font::Helvetica, 10.0)
                .at(50.0, height - 50.0)
                .write(&format!(
                    "[Page from document {} - content reconstruction in progress]",
                    input_idx + 1
                ))
                .map_err(OperationError::PdfError)?;
        }

        Ok(page)
    }

    /// Process content operators for merge, handling resource remapping
    fn process_operators_for_merge(
        &self,
        page: &mut Page,
        operators: &[ContentOperation],
        input_idx: usize,
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
                    // Use font mapping if available
                    let mapped_name = self.font_mappings.get(input_idx)
                        .and_then(|mapping| mapping.get(name))
                        .unwrap_or(name);
                    
                    // Map PDF font names to our standard fonts
                    // Note: In a full implementation, we would preserve custom fonts
                    // by copying font resources and updating references
                    current_font = match mapped_name.as_str() {
                        "Times-Roman" => crate::text::Font::TimesRoman,
                        "Times-Bold" => crate::text::Font::TimesBold,
                        "Times-Italic" => crate::text::Font::TimesItalic,
                        "Times-BoldItalic" => crate::text::Font::TimesBoldItalic,
                        "Helvetica" => crate::text::Font::Helvetica,
                        "Helvetica-Bold" => crate::text::Font::HelveticaBold,
                        "Helvetica-Oblique" => crate::text::Font::HelveticaOblique,
                        "Helvetica-BoldOblique" => crate::text::Font::HelveticaBoldOblique,
                        "Courier" => crate::text::Font::Courier,
                        "Courier-Bold" => crate::text::Font::CourierBold,
                        "Courier-Oblique" => crate::text::Font::CourierOblique,
                        "Courier-BoldOblique" => crate::text::Font::CourierBoldOblique,
                        _ => {
                            // For non-standard fonts, default to Helvetica
                            // A complete implementation would preserve the original font
                            eprintln!("Warning: Font '{}' not supported, using Helvetica", mapped_name);
                            crate::text::Font::Helvetica
                        }
                    };
                    current_font_size = *size;
                }
                ContentOperation::MoveText(tx, ty) => {
                    current_x += tx;
                    current_y += ty;
                }
                ContentOperation::ShowText(text_bytes) => {
                    if text_object {
                        if let Ok(text) = String::from_utf8(text_bytes.clone()) {
                            page.text()
                                .set_font(current_font, current_font_size as f64)
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
                ContentOperation::PaintXObject(name) => {
                    // Use XObject mapping if available
                    let mapped_name = self.xobject_mappings.get(input_idx)
                        .and_then(|mapping| mapping.get(name))
                        .unwrap_or(name);
                    
                    // Note: In a complete implementation, we would copy the XObject
                    // resource (image or form) and paint it with the new reference.
                    // For now, we'll add a placeholder comment
                    eprintln!(
                        "Info: XObject '{}' (mapped to '{}') would be painted here. \
                         Full XObject support requires resource copying.",
                        name, mapped_name
                    );
                    
                    // Add a visual placeholder for missing XObjects
                    page.graphics()
                        .set_fill_color(crate::graphics::Color::Rgb(0.9, 0.9, 0.9))
                        .rect(current_x as f64, current_y as f64, 100.0, 100.0)
                        .fill();
                    
                    page.text()
                        .set_font(crate::text::Font::Helvetica, 10.0)
                        .at(current_x as f64 + 10.0, current_y as f64 + 50.0)
                        .write(&format!("[Image: {}]", name))
                        .map_err(OperationError::PdfError)?;
                }
                _ => {
                    // Silently skip unimplemented operators for now
                }
            }
        }

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

    /// Get the next available object number and increment
    #[allow(dead_code)]
    fn allocate_object_number(&mut self) -> u32 {
        let num = self.next_object_num;
        self.next_object_num += 1;
        num
    }

    /// Map an object number from an input document to the merged document
    #[allow(dead_code)]
    fn map_object_number(&mut self, input_idx: usize, old_num: u32) -> u32 {
        // Check if already mapped
        if let Some(&new_num) = self.object_mappings[input_idx].get(&old_num) {
            return new_num;
        }

        // Allocate new number
        let new_num = self.allocate_object_number();
        self.object_mappings[input_idx].insert(old_num, new_num);
        new_num
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
}

#[cfg(test)]
#[path = "merge_tests.rs"]
mod merge_tests;
