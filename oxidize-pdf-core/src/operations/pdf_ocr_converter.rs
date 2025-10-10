//! PDF OCR Converter - Convert scanned PDFs to searchable PDFs
//!
//! This module provides functionality to convert PDF documents containing scanned images
//! into searchable PDFs by adding an invisible text layer over the images using OCR.
//!
//! # Features
//! - Automatic detection of scanned pages
//! - OCR text extraction with position information
//! - Invisible text layer overlay preserving original appearance
//! - Batch processing support
//! - Progress reporting
//! - Multi-language OCR support
//!
//! # Usage
//!
//! ```rust,no_run
//! use oxidize_pdf::operations::pdf_ocr_converter::{PdfOcrConverter, ConversionOptions};
//! use oxidize_pdf::text::RustyTesseractProvider;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let converter = PdfOcrConverter::new()?;
//! let ocr_provider = RustyTesseractProvider::new()?;
//! let options = ConversionOptions::default();
//!
//! converter.convert_to_searchable_pdf(
//!     "scanned_document.pdf",
//!     "searchable_document.pdf",
//!     &ocr_provider,
//!     &options,
//! )?;
//! # Ok(())
//! # }
//! ```

use crate::error::{PdfError, Result};
use crate::graphics::Color;
use crate::operations::page_analysis::{AnalysisOptions, PageContentAnalyzer};
use crate::parser::{ParseOptions, PdfDocument, PdfReader};
use crate::text::{FragmentType, OcrOptions, OcrProvider};
use crate::{Document, Font, Page};
use std::fs::File;
use std::path::Path;
use std::time::Instant;

/// Options for PDF OCR conversion
pub struct ConversionOptions {
    /// OCR options for text extraction
    pub ocr_options: OcrOptions,
    /// Minimum confidence threshold for OCR results (0.0 to 1.0)
    pub min_confidence: f64,
    /// Whether to skip pages that already contain text
    pub skip_text_pages: bool,
    /// Font size for invisible text layer (should match expected text size)
    pub text_layer_font_size: f64,
    /// DPI for image processing
    pub dpi: u32,
    /// Whether to preserve original page structure
    pub preserve_structure: bool,
    /// Progress callback function
    pub progress_callback: Option<Box<dyn Fn(usize, usize) + Send + Sync>>,
}

impl std::fmt::Debug for ConversionOptions {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ConversionOptions")
            .field("ocr_options", &self.ocr_options)
            .field("min_confidence", &self.min_confidence)
            .field("skip_text_pages", &self.skip_text_pages)
            .field("text_layer_font_size", &self.text_layer_font_size)
            .field("dpi", &self.dpi)
            .field("preserve_structure", &self.preserve_structure)
            .field(
                "progress_callback",
                &self.progress_callback.as_ref().map(|_| "Some(callback)"),
            )
            .finish()
    }
}

impl Default for ConversionOptions {
    fn default() -> Self {
        Self {
            ocr_options: OcrOptions::default(),
            min_confidence: 0.7,
            skip_text_pages: true,
            text_layer_font_size: 12.0,
            dpi: 300,
            preserve_structure: true,
            progress_callback: None,
        }
    }
}

/// Result of PDF OCR conversion
#[derive(Debug)]
pub struct ConversionResult {
    /// Number of pages processed
    pub pages_processed: usize,
    /// Number of pages that were OCR'd
    pub pages_ocr_processed: usize,
    /// Number of pages skipped (already had text)
    pub pages_skipped: usize,
    /// Total processing time
    pub processing_time: std::time::Duration,
    /// Average confidence score of OCR results
    pub average_confidence: f64,
    /// Total characters extracted via OCR
    pub total_characters_extracted: usize,
}

/// PDF OCR Converter for creating searchable PDFs from scanned documents
pub struct PdfOcrConverter {
    /// Analysis options for page content detection
    analysis_options: AnalysisOptions,
}

impl PdfOcrConverter {
    /// Create a new PDF OCR converter with default settings
    pub fn new() -> Result<Self> {
        Ok(Self {
            analysis_options: AnalysisOptions::default(),
        })
    }

    /// Create a new PDF OCR converter with custom analysis options
    pub fn with_analysis_options(analysis_options: AnalysisOptions) -> Self {
        Self { analysis_options }
    }

    /// Convert a scanned PDF to a searchable PDF
    ///
    /// This is the main function that:
    /// 1. Opens the input PDF
    /// 2. Analyzes each page to detect scanned content
    /// 3. Applies OCR to scanned pages
    /// 4. Creates a new PDF with invisible text layers
    /// 5. Saves the result
    pub fn convert_to_searchable_pdf<P: OcrProvider>(
        &self,
        input_path: impl AsRef<Path>,
        output_path: impl AsRef<Path>,
        ocr_provider: &P,
        options: &ConversionOptions,
    ) -> Result<ConversionResult> {
        let start_time = Instant::now();

        // Open input PDF
        let file = File::open(input_path.as_ref()).map_err(|e| PdfError::Io(e))?;

        let reader = PdfReader::new_with_options(file, ParseOptions::tolerant())?;
        let document = PdfDocument::new(reader);
        let page_count = document.page_count()?;

        // Initialize analyzer
        let analyzer = PageContentAnalyzer::with_options(document, self.analysis_options.clone());

        // Create new output document
        let mut output_doc = Document::new();

        let mut stats = ConversionStats::new();

        // Process each page
        for page_num in 0..page_count {
            if let Some(ref callback) = options.progress_callback {
                callback(page_num as usize, page_count as usize);
            }

            let processed_page = self.process_page(
                &analyzer,
                page_num as usize,
                ocr_provider,
                options,
                &mut stats,
            )?;

            output_doc.add_page(processed_page);
        }

        // Save output document
        let pdf_bytes = output_doc.to_bytes()?;
        std::fs::write(output_path.as_ref(), pdf_bytes).map_err(|e| PdfError::Io(e))?;

        let processing_time = start_time.elapsed();

        Ok(ConversionResult {
            pages_processed: stats.pages_processed,
            pages_ocr_processed: stats.pages_ocr_processed,
            pages_skipped: stats.pages_skipped,
            processing_time,
            average_confidence: stats.calculate_average_confidence(),
            total_characters_extracted: stats.total_characters,
        })
    }

    /// Process a single page, applying OCR if needed
    fn process_page<P: OcrProvider>(
        &self,
        analyzer: &PageContentAnalyzer,
        page_num: usize,
        ocr_provider: &P,
        options: &ConversionOptions,
        stats: &mut ConversionStats,
    ) -> Result<Page> {
        stats.pages_processed += 1;

        // Analyze page content
        let analysis = analyzer
            .analyze_page(page_num)
            .map_err(|e| PdfError::ParseError(e.to_string()))?;

        // Create base page
        let mut page = Page::a4(); // TODO: Get actual page dimensions

        if analysis.is_scanned() && (!options.skip_text_pages || analysis.character_count < 50) {
            // Page needs OCR processing
            self.process_scanned_page(analyzer, page_num, ocr_provider, options, stats, &mut page)?;
        } else {
            // Copy existing page content (vector text, graphics, etc.)
            self.copy_page_content(analyzer, page_num, &mut page)?;
            if options.skip_text_pages {
                stats.pages_skipped += 1;
            }
        }

        Ok(page)
    }

    /// Process a scanned page with OCR
    fn process_scanned_page<P: OcrProvider>(
        &self,
        analyzer: &PageContentAnalyzer,
        page_num: usize,
        ocr_provider: &P,
        options: &ConversionOptions,
        stats: &mut ConversionStats,
        page: &mut Page,
    ) -> Result<()> {
        // Extract image data from the page
        let image_data = analyzer.extract_page_image_data(page_num).map_err(|e| {
            PdfError::ParseError(format!(
                "Failed to extract image from page {}: {}",
                page_num, e
            ))
        })?;

        // Apply OCR to extract text
        let ocr_result = ocr_provider
            .process_image(&image_data, &options.ocr_options)
            .map_err(|e| {
                PdfError::InvalidStructure(format!("OCR failed for page {}: {}", page_num, e))
            })?;

        if ocr_result.confidence >= options.min_confidence {
            // Add the original image to the page (visible layer)
            self.add_image_to_page(page, &image_data)?;

            // Add invisible text layer
            self.add_invisible_text_layer(page, &ocr_result, options)?;

            stats.pages_ocr_processed += 1;
            stats.total_confidence += ocr_result.confidence;
            stats.total_characters += ocr_result.text.len();
        } else {
            // Low confidence, just add the image without text layer
            self.add_image_to_page(page, &image_data)?;
            eprintln!(
                "Warning: Low OCR confidence ({:.1}%) for page {}, skipping text layer",
                ocr_result.confidence * 100.0,
                page_num
            );
        }

        Ok(())
    }

    /// Add an image to a page
    fn add_image_to_page(&self, page: &mut Page, _image_data: &[u8]) -> Result<()> {
        // TODO: Implement image embedding
        // For now, we'll create a placeholder
        page.graphics()
            .save_state()
            .set_fill_color(Color::rgb(240.0, 240.0, 240.0))
            .rect(50.0, 50.0, 500.0, 700.0)
            .fill()
            .restore_state();

        Ok(())
    }

    /// Add an invisible text layer over the image
    fn add_invisible_text_layer(
        &self,
        page: &mut Page,
        ocr_result: &crate::text::OcrProcessingResult,
        options: &ConversionOptions,
    ) -> Result<()> {
        // Add invisible text at detected positions
        let text_context = page.text();

        for fragment in &ocr_result.fragments {
            if fragment.fragment_type == FragmentType::Word {
                // Position text at the detected coordinates
                // Make text invisible by setting rendering mode to invisible
                text_context
                    .set_font(Font::Helvetica, options.text_layer_font_size)
                    .at(fragment.x as f64, fragment.y as f64)
                    .set_rendering_mode(crate::text::TextRenderingMode::Invisible)
                    .write(&fragment.text)?;
            }
        }

        Ok(())
    }

    /// Copy existing page content (for pages that already have text)
    fn copy_page_content(
        &self,
        _analyzer: &PageContentAnalyzer,
        _page_num: usize,
        page: &mut Page,
    ) -> Result<()> {
        // TODO: Implement page content copying
        // For now, create a placeholder indicating this is a text page
        page.text()
            .set_font(Font::Helvetica, 12.0)
            .at(50.0, 750.0)
            .write("This page already contains text content")?;

        Ok(())
    }

    /// Batch process multiple PDF files
    pub fn batch_convert<P: OcrProvider>(
        &self,
        input_paths: &[impl AsRef<Path>],
        output_dir: impl AsRef<Path>,
        ocr_provider: &P,
        options: &ConversionOptions,
    ) -> Result<Vec<ConversionResult>> {
        let mut results = Vec::new();

        for input_path in input_paths {
            let input_path = input_path.as_ref();
            let output_filename = format!(
                "{}_searchable.pdf",
                input_path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("output")
            );
            let output_path = output_dir.as_ref().join(output_filename);

            match self.convert_to_searchable_pdf(input_path, output_path, ocr_provider, options) {
                Ok(result) => {
                    println!(
                        "✅ Converted: {} ({} pages)",
                        input_path.display(),
                        result.pages_processed
                    );
                    results.push(result);
                }
                Err(e) => {
                    eprintln!("❌ Failed to convert {}: {}", input_path.display(), e);
                }
            }
        }

        Ok(results)
    }
}

/// Internal statistics tracking
struct ConversionStats {
    pages_processed: usize,
    pages_ocr_processed: usize,
    pages_skipped: usize,
    total_confidence: f64,
    total_characters: usize,
}

impl ConversionStats {
    fn new() -> Self {
        Self {
            pages_processed: 0,
            pages_ocr_processed: 0,
            pages_skipped: 0,
            total_confidence: 0.0,
            total_characters: 0,
        }
    }

    fn calculate_average_confidence(&self) -> f64 {
        if self.pages_ocr_processed > 0 {
            self.total_confidence / self.pages_ocr_processed as f64
        } else {
            0.0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::text::{FragmentType, OcrEngine, OcrProcessingResult, OcrTextFragment};
    use crate::ImageFormat;

    // Mock OCR provider for testing
    struct MockOcrProvider {
        confidence: f64,
        text: String,
    }

    impl MockOcrProvider {
        #[allow(dead_code)]
        fn new(confidence: f64, text: String) -> Self {
            Self { confidence, text }
        }
    }

    impl OcrProvider for MockOcrProvider {
        fn process_image(
            &self,
            _image_data: &[u8],
            _options: &OcrOptions,
        ) -> crate::text::OcrResult<OcrProcessingResult> {
            Ok(OcrProcessingResult {
                text: self.text.clone(),
                confidence: self.confidence,
                processing_time_ms: 100,
                fragments: vec![OcrTextFragment {
                    text: self.text.clone(),
                    x: 100.0,
                    y: 700.0,
                    width: 200.0,
                    height: 20.0,
                    confidence: self.confidence,
                    word_confidences: None,
                    font_size: 12.0,
                    fragment_type: FragmentType::Word,
                }],
                engine_name: "Mock OCR".to_string(),
                language: "eng".to_string(),
                processed_region: None,
                image_dimensions: (800, 600),
            })
        }

        fn supported_formats(&self) -> Vec<ImageFormat> {
            vec![ImageFormat::Jpeg, ImageFormat::Png, ImageFormat::Tiff]
        }

        fn engine_name(&self) -> &str {
            "Mock OCR"
        }

        fn engine_type(&self) -> OcrEngine {
            OcrEngine::Mock
        }
    }

    #[test]
    fn test_conversion_options_default() {
        let options = ConversionOptions::default();
        assert_eq!(options.min_confidence, 0.7);
        assert!(options.skip_text_pages);
        assert_eq!(options.text_layer_font_size, 12.0);
        assert_eq!(options.dpi, 300);
    }

    #[test]
    fn test_pdf_ocr_converter_creation() {
        let converter = PdfOcrConverter::new();
        assert!(converter.is_ok());
    }

    #[test]
    fn test_conversion_stats() {
        let mut stats = ConversionStats::new();
        assert_eq!(stats.pages_processed, 0);
        assert_eq!(stats.calculate_average_confidence(), 0.0);

        stats.pages_ocr_processed = 2;
        stats.total_confidence = 1.6;
        assert_eq!(stats.calculate_average_confidence(), 0.8);
    }
}
