//! PDF Editor - High-level API for modifying existing PDF documents
//!
//! This module provides `PdfEditor`, the central abstraction for loading,
//! modifying, and saving PDF documents. It serves as the foundation for
//! all PDF modification operations (content injection, watermarking, form filling, etc.)
//!
//! # Example
//!
//! ```no_run
//! use oxidize_pdf::operations::PdfEditor;
//!
//! let mut editor = PdfEditor::open("input.pdf").unwrap();
//! println!("Document has {} pages", editor.page_count());
//! editor.save("output.pdf").unwrap();
//! ```

use std::fs;
use std::io::{Cursor, Read, Seek};
use std::path::Path;

use crate::parser::{PdfDocument, PdfReader};
use crate::writer::{PdfWriter, WriterConfig};
use crate::{Document, Page};

use super::annotation_injector::PendingAnnotation;
use super::content_injection::{
    CircleInjectionSpec, ImageFormat, ImageInjectionSpec, LineInjectionSpec, RectInjectionSpec,
    TextInjectionSpec,
};
use super::form_filler::FormFieldInfo;
use super::page_manipulator::CropBox;
use super::watermark::WatermarkSpec;

/// Error type for PDF modification operations
#[derive(Debug, thiserror::Error)]
pub enum ModificationError {
    /// Page index is out of bounds
    #[error("Page index {index} out of bounds (document has {total} pages)")]
    PageIndexOutOfBounds {
        /// The requested page index
        index: usize,
        /// Total number of pages in the document
        total: usize,
    },

    /// Error parsing the PDF document
    #[error("Parse error: {0}")]
    ParseError(String),

    /// Error writing the PDF document
    #[error("Write error: {0}")]
    WriteError(String),

    /// Invalid or unsupported content
    #[error("Invalid content: {0}")]
    InvalidContent(String),

    /// IO error
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// The PDF document has no pages
    #[error("Document has no pages")]
    EmptyDocument,
}

/// Result type for modification operations
pub type ModificationResult<T> = Result<T, ModificationError>;

/// Options for configuring PDF modification behavior
#[derive(Debug, Clone)]
pub struct PdfEditorOptions {
    /// Whether to compress streams in the output
    pub compress: bool,
    /// Whether to use incremental updates (appends to original)
    pub incremental: bool,
    /// PDF version for output (e.g., "1.7")
    pub pdf_version: String,
}

impl Default for PdfEditorOptions {
    fn default() -> Self {
        Self {
            compress: true,
            incremental: false,
            pdf_version: "1.7".to_string(),
        }
    }
}

impl PdfEditorOptions {
    /// Create options with incremental update mode enabled
    pub fn with_incremental(mut self) -> Self {
        self.incremental = true;
        self
    }

    /// Create options with compression disabled
    pub fn with_compress(mut self, compress: bool) -> Self {
        self.compress = compress;
        self
    }
}

/// High-level PDF editor for modifying existing PDF documents
///
/// `PdfEditor` provides a unified interface for:
/// - Loading existing PDFs from files or bytes
/// - Querying document properties (page count, page sizes)
/// - Applying modifications (via sub-modules)
/// - Saving the modified document
///
/// The editor uses a full-rewrite approach: the document is parsed,
/// modified in memory, and then completely re-serialized. This ensures
/// clean output without incremental update overhead.
pub struct PdfEditor {
    /// The writable PDF document
    document: Document,
    /// Original PDF bytes (for reference)
    #[allow(dead_code)]
    original_bytes: Vec<u8>,
    /// Editor options
    options: PdfEditorOptions,

    // Pending content injections (applied on save)
    /// Pending text injections: (page_index, spec)
    pub(crate) pending_text_injections: Vec<(usize, TextInjectionSpec)>,
    /// Pending image injections: (page_index, image_data, spec, format)
    pub(crate) pending_image_injections: Vec<(usize, Vec<u8>, ImageInjectionSpec, ImageFormat)>,
    /// Pending line injections: (page_index, spec)
    pub(crate) pending_line_injections: Vec<(usize, LineInjectionSpec)>,
    /// Pending rect injections: (page_index, spec)
    pub(crate) pending_rect_injections: Vec<(usize, RectInjectionSpec)>,
    /// Pending circle injections: (page_index, spec)
    pub(crate) pending_circle_injections: Vec<(usize, CircleInjectionSpec)>,
    /// Pending watermarks: (page_indices, spec)
    pub(crate) pending_watermarks: Vec<(Vec<usize>, WatermarkSpec)>,

    // Form filling state
    /// Detected form fields from existing PDF
    pub(crate) pending_form_fields: Vec<FormFieldInfo>,
    /// Pending form field updates: (field_name, new_value)
    pub(crate) pending_form_updates: Vec<(String, String)>,
    /// Whether the form should be flattened on save
    pub(crate) form_flattened: bool,

    // Page manipulation state
    /// Pending crop box updates: (page_index, crop_box)
    pub(crate) pending_crop_boxes: Vec<(usize, CropBox)>,
    /// Pending resize updates: (page_index, new_width, new_height, scale_content)
    pub(crate) pending_resizes: Vec<(usize, f64, f64, bool)>,
    /// Pending page deletions (page indices)
    pub(crate) pending_deletions: Vec<usize>,

    // Annotation injection state
    /// Pending annotations: (page_index, annotation)
    pub(crate) pending_annotations: Vec<(usize, PendingAnnotation)>,
}

impl std::fmt::Debug for PdfEditor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PdfEditor")
            .field("page_count", &self.page_count())
            .field("options", &self.options)
            .finish()
    }
}

impl PdfEditor {
    /// Open a PDF file for editing
    ///
    /// # Arguments
    /// * `path` - Path to the PDF file
    ///
    /// # Returns
    /// A `PdfEditor` instance ready for modifications
    pub fn open<P: AsRef<Path>>(path: P) -> ModificationResult<Self> {
        let bytes = fs::read(path)?;
        Self::from_bytes(bytes)
    }

    /// Create a PDF editor from bytes in memory
    ///
    /// # Arguments
    /// * `bytes` - PDF file contents as bytes
    ///
    /// # Returns
    /// A `PdfEditor` instance ready for modifications
    pub fn from_bytes(bytes: Vec<u8>) -> ModificationResult<Self> {
        Self::from_bytes_with_options(bytes, PdfEditorOptions::default())
    }

    /// Create a PDF editor from bytes with custom options
    pub fn from_bytes_with_options(
        bytes: Vec<u8>,
        options: PdfEditorOptions,
    ) -> ModificationResult<Self> {
        let cursor = Cursor::new(&bytes);
        let reader =
            PdfReader::new(cursor).map_err(|e| ModificationError::ParseError(e.to_string()))?;

        // Convert to PdfDocument for page access
        let pdf_document = reader.into_document();

        // Convert parsed document to writable Document
        let document = Self::parsed_to_document(&pdf_document)?;

        Ok(Self {
            document,
            original_bytes: bytes,
            options,
            pending_text_injections: Vec::new(),
            pending_image_injections: Vec::new(),
            pending_line_injections: Vec::new(),
            pending_rect_injections: Vec::new(),
            pending_circle_injections: Vec::new(),
            pending_watermarks: Vec::new(),
            pending_form_fields: Vec::new(),
            pending_form_updates: Vec::new(),
            form_flattened: false,
            pending_crop_boxes: Vec::new(),
            pending_resizes: Vec::new(),
            pending_deletions: Vec::new(),
            pending_annotations: Vec::new(),
        })
    }

    /// Convert a parsed PDF to a writable Document
    fn parsed_to_document<R: Read + Seek>(
        pdf_document: &PdfDocument<R>,
    ) -> ModificationResult<Document> {
        let mut document = Document::new();

        let page_count = pdf_document
            .page_count()
            .map_err(|e| ModificationError::ParseError(e.to_string()))?;

        for page_idx in 0..page_count {
            let parsed_page = pdf_document
                .get_page(page_idx)
                .map_err(|e| ModificationError::ParseError(e.to_string()))?;

            let page = Page::from_parsed_with_content(&parsed_page, pdf_document)
                .map_err(|e| ModificationError::ParseError(e.to_string()))?;

            document.add_page(page);
        }

        Ok(document)
    }

    /// Get the number of pages in the document
    pub fn page_count(&self) -> usize {
        self.document.page_count()
    }

    /// Get the size (width, height) of a specific page in points
    ///
    /// # Arguments
    /// * `page_index` - Zero-based page index
    ///
    /// # Returns
    /// Tuple of (width, height) in points, or error if page doesn't exist
    pub fn get_page_size(&self, page_index: usize) -> ModificationResult<(f64, f64)> {
        let total = self.page_count();
        if page_index >= total {
            return Err(ModificationError::PageIndexOutOfBounds {
                index: page_index,
                total,
            });
        }

        // Access pages directly (pub(crate) field accessible within crate)
        let page = &self.document.pages[page_index];
        Ok((page.width(), page.height()))
    }

    /// Save the modified PDF to a file
    ///
    /// # Arguments
    /// * `path` - Output file path
    pub fn save<P: AsRef<Path>>(&mut self, path: P) -> ModificationResult<()> {
        let bytes = self.save_to_bytes()?;
        fs::write(path, bytes)?;
        Ok(())
    }

    /// Save the modified PDF to bytes in memory
    ///
    /// # Returns
    /// The complete PDF file as bytes
    pub fn save_to_bytes(&mut self) -> ModificationResult<Vec<u8>> {
        let config = WriterConfig {
            compress_streams: self.options.compress,
            incremental_update: self.options.incremental,
            ..WriterConfig::default()
        };

        let mut output = Vec::new();
        {
            let cursor = Cursor::new(&mut output);
            let mut writer = PdfWriter::with_config(cursor, config);

            writer
                .write_document(&mut self.document)
                .map_err(|e| ModificationError::WriteError(e.to_string()))?;
        }

        Ok(output)
    }

    /// Get a mutable reference to the internal document
    ///
    /// This is intended for internal use by sub-modules (content injection, etc.)
    #[allow(dead_code)]
    pub(crate) fn document_mut(&mut self) -> &mut Document {
        &mut self.document
    }

    /// Get a reference to the internal document
    #[allow(dead_code)]
    pub(crate) fn document(&self) -> &Document {
        &self.document
    }

    /// Get the editor options
    pub fn options(&self) -> &PdfEditorOptions {
        &self.options
    }

    // Form filling methods for testing

    /// Add a form field for testing purposes
    #[doc(hidden)]
    pub fn add_test_form_field(&mut self, field: super::form_filler::FormFieldInfo) {
        self.pending_form_fields.push(field);
    }

    /// Check if the form is marked for flattening
    pub fn is_form_flattened(&self) -> bool {
        self.form_flattened
    }

    /// Get the number of pending form updates
    pub fn pending_form_update_count(&self) -> usize {
        self.pending_form_updates.len()
    }

    // Page manipulation helper methods for testing

    /// Get the number of pending crop box updates
    pub fn pending_crop_count(&self) -> usize {
        self.pending_crop_boxes.len()
    }

    /// Get the number of pending resize operations
    pub fn pending_resize_count(&self) -> usize {
        self.pending_resizes.len()
    }

    /// Get the number of pending page deletions
    pub fn pending_deletion_count(&self) -> usize {
        self.pending_deletions.len()
    }

    /// Get the number of pending annotations
    pub fn pending_annotation_count(&self) -> usize {
        self.pending_annotations.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Helper function to create a minimal valid PDF for testing
    fn create_test_pdf(page_count: usize) -> Vec<u8> {
        let mut doc = Document::new();
        for _ in 0..page_count {
            // Create A4 page (595 x 842 points)
            let page = Page::new(595.0, 842.0);
            doc.add_page(page);
        }

        let config = WriterConfig::default();
        let mut output = Vec::new();
        {
            let cursor = Cursor::new(&mut output);
            let mut writer = PdfWriter::with_config(cursor, config);
            writer
                .write_document(&mut doc)
                .expect("Failed to create test PDF");
        }
        output
    }

    // T1.1 - PdfEditor constructs from bytes
    #[test]
    fn test_pdf_editor_from_bytes() {
        let pdf_bytes = create_test_pdf(1);
        let result = PdfEditor::from_bytes(pdf_bytes);
        assert!(
            result.is_ok(),
            "Should construct PdfEditor from valid PDF bytes"
        );
    }

    // T1.2 - PdfEditor constructs from file
    #[test]
    fn test_pdf_editor_from_file() {
        let pdf_bytes = create_test_pdf(1);
        let temp_dir = tempfile::tempdir().unwrap();
        let path = temp_dir.path().join("test.pdf");
        std::fs::write(&path, &pdf_bytes).unwrap();

        let result = PdfEditor::open(&path);
        assert!(result.is_ok(), "Should construct PdfEditor from file");
    }

    // T1.3 - page_count returns correct number
    #[test]
    fn test_pdf_editor_page_count() {
        let pdf_bytes = create_test_pdf(3);
        let editor = PdfEditor::from_bytes(pdf_bytes).unwrap();
        assert_eq!(editor.page_count(), 3, "Should report 3 pages");
    }

    // T1.4 - save_to_bytes produces valid PDF bytes
    #[test]
    fn test_pdf_editor_save_to_bytes() {
        let pdf_bytes = create_test_pdf(1);
        let mut editor = PdfEditor::from_bytes(pdf_bytes).unwrap();

        let output = editor.save_to_bytes().unwrap();
        assert!(!output.is_empty(), "Output should not be empty");
        assert!(
            output.starts_with(b"%PDF-"),
            "Output should start with PDF header"
        );
    }

    // T1.5 - save_to_file writes to disk
    #[test]
    fn test_pdf_editor_save_to_file() {
        let pdf_bytes = create_test_pdf(1);
        let mut editor = PdfEditor::from_bytes(pdf_bytes).unwrap();

        let temp_dir = tempfile::tempdir().unwrap();
        let output_path = temp_dir.path().join("output.pdf");

        editor.save(&output_path).unwrap();

        assert!(output_path.exists(), "Output file should exist");
        let file_size = std::fs::metadata(&output_path).unwrap().len();
        assert!(file_size > 0, "Output file should not be empty");
    }

    // T1.6 - Default options don't modify document
    #[test]
    fn test_pdf_editor_options_default() {
        let pdf_bytes = create_test_pdf(5);
        let options = PdfEditorOptions::default();
        let editor = PdfEditor::from_bytes_with_options(pdf_bytes, options).unwrap();

        assert_eq!(editor.page_count(), 5, "Page count should be unchanged");
    }

    // T1.7 - Single page PDF
    #[test]
    fn test_pdf_editor_single_page_count() {
        let pdf_bytes = create_test_pdf(1);
        let editor = PdfEditor::from_bytes(pdf_bytes).unwrap();
        assert_eq!(editor.page_count(), 1, "Should report 1 page");
    }

    // T1.8 - Error on page out of bounds
    #[test]
    fn test_pdf_editor_page_out_of_bounds_error() {
        let pdf_bytes = create_test_pdf(2);
        let editor = PdfEditor::from_bytes(pdf_bytes).unwrap();

        let result = editor.get_page_size(5);
        assert!(result.is_err(), "Should error on out of bounds page");

        match result.unwrap_err() {
            ModificationError::PageIndexOutOfBounds { index, total } => {
                assert_eq!(index, 5);
                assert_eq!(total, 2);
            }
            e => panic!("Expected PageIndexOutOfBounds, got {:?}", e),
        }
    }

    // T1.9 - Debug implementation
    #[test]
    fn test_pdf_editor_debug_impl() {
        let pdf_bytes = create_test_pdf(2);
        let editor = PdfEditor::from_bytes(pdf_bytes).unwrap();

        let debug_str = format!("{:?}", editor);
        assert!(
            debug_str.contains("PdfEditor"),
            "Debug should contain type name"
        );
        assert!(
            debug_str.contains("page_count"),
            "Debug should contain page_count"
        );
    }

    // T1.10 - ModificationError Display variants
    #[test]
    fn test_modification_error_display() {
        let errors = vec![
            ModificationError::PageIndexOutOfBounds { index: 5, total: 3 },
            ModificationError::ParseError("invalid header".to_string()),
            ModificationError::WriteError("disk full".to_string()),
            ModificationError::InvalidContent("corrupt stream".to_string()),
            ModificationError::EmptyDocument,
        ];

        for error in errors {
            let message = error.to_string();
            assert!(!message.is_empty(), "Error message should not be empty");
        }

        // Check specific messages
        let error = ModificationError::PageIndexOutOfBounds { index: 5, total: 3 };
        assert!(error.to_string().contains("5"));
        assert!(error.to_string().contains("3"));

        let error = ModificationError::ParseError("invalid".to_string());
        assert!(error.to_string().contains("invalid"));
    }

    // T1.11 - Incremental mode option
    #[test]
    fn test_pdf_editor_options_incremental() {
        let options = PdfEditorOptions::default().with_incremental();
        assert!(options.incremental, "Incremental should be enabled");
    }

    // T1.12 - get_page_size returns correct dimensions
    #[test]
    fn test_pdf_editor_get_page_size() {
        let pdf_bytes = create_test_pdf(1);
        let editor = PdfEditor::from_bytes(pdf_bytes).unwrap();

        let (width, height) = editor.get_page_size(0).unwrap();
        // A4 dimensions: 595 x 842 points (approximately)
        assert!((width - 595.0).abs() < 1.0, "Width should be ~595");
        assert!((height - 842.0).abs() < 1.0, "Height should be ~842");
    }

    // T1.13 - Roundtrip preserves page count
    #[test]
    fn test_pdf_editor_roundtrip_page_count() {
        let pdf_bytes = create_test_pdf(3);
        let mut editor = PdfEditor::from_bytes(pdf_bytes).unwrap();

        // Save and reload
        let output_bytes = editor.save_to_bytes().unwrap();
        let editor2 = PdfEditor::from_bytes(output_bytes).unwrap();

        assert_eq!(
            editor.page_count(),
            editor2.page_count(),
            "Page count should be preserved after roundtrip"
        );
    }

    // T1.14 - No compression option
    #[test]
    fn test_pdf_editor_options_no_compression() {
        let options = PdfEditorOptions::default().with_compress(false);
        assert!(!options.compress, "Compression should be disabled");

        let pdf_bytes = create_test_pdf(1);
        let mut editor = PdfEditor::from_bytes_with_options(pdf_bytes, options).unwrap();

        // Should still produce valid output
        let output = editor.save_to_bytes().unwrap();
        assert!(output.starts_with(b"%PDF-"));
    }

    // T1.15 - Output PDF version
    #[test]
    fn test_pdf_editor_output_pdf_version() {
        let pdf_bytes = create_test_pdf(1);
        let mut editor = PdfEditor::from_bytes(pdf_bytes).unwrap();

        let output = editor.save_to_bytes().unwrap();

        // Check that output starts with %PDF-1.x header
        let header = String::from_utf8_lossy(&output[0..8.min(output.len())]);
        assert!(
            header.starts_with("%PDF-1."),
            "Output should have PDF version header, got: {}",
            header
        );
    }

    // Additional tests for edge cases

    #[test]
    fn test_modification_error_io() {
        let io_error = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let mod_error: ModificationError = io_error.into();

        match mod_error {
            ModificationError::Io(_) => {}
            _ => panic!("Expected Io variant"),
        }
    }

    #[test]
    fn test_pdf_editor_options_clone() {
        let options = PdfEditorOptions::default();
        let cloned = options.clone();

        assert_eq!(options.compress, cloned.compress);
        assert_eq!(options.incremental, cloned.incremental);
        assert_eq!(options.pdf_version, cloned.pdf_version);
    }

    #[test]
    fn test_pdf_editor_options_debug() {
        let options = PdfEditorOptions::default();
        let debug_str = format!("{:?}", options);

        assert!(debug_str.contains("compress"));
        assert!(debug_str.contains("incremental"));
    }
}
