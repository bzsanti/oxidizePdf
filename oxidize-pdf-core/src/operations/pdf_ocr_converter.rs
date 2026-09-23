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
//! The example below uses `MockOcrProvider` so it compiles under the
//! default feature set. For real OCR, enable the `ocr-tesseract` feature
//! and substitute `RustyTesseractProvider::new()` (which requires the
//! Tesseract binary on `PATH`).
//!
//! ```rust,no_run
//! use oxidize_pdf::operations::pdf_ocr_converter::{PdfOcrConverter, ConversionOptions};
//! use oxidize_pdf::text::MockOcrProvider;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let converter = PdfOcrConverter::new()?;
//! let ocr_provider = MockOcrProvider::new();
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
use crate::operations::page_analysis::{AnalysisOptions, PageContentAnalyzer};
use crate::parser::{ParseOptions, PdfDocument, PdfReader};
use crate::text::{FragmentType, OcrOptions, OcrProvider};
use crate::writer::{IncrementalOcrLayerEditor, OcrLayerFragment, OcrLayerPage, OcrLayerPlan};
use std::fs::File;
use std::io::Write;
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
    /// Legacy preferred font size; positioned incremental layers use each OCR region's height.
    pub text_layer_font_size: f64,
    /// DPI for image processing
    pub dpi: u32,
    /// Legacy compatibility flag; incremental conversion always preserves page structure.
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

        let base_bytes = std::fs::read(input_path.as_ref()).map_err(PdfError::Io)?;
        let file = File::open(input_path.as_ref()).map_err(PdfError::Io)?;

        let reader = PdfReader::new_with_options(file, ParseOptions::tolerant())?;
        let document = PdfDocument::new(reader);
        let page_count = document.page_count()?;

        // Initialize analyzer
        let analyzer = PageContentAnalyzer::with_options(document, self.analysis_options.clone());

        let mut stats = ConversionStats::new();
        let mut layers = Vec::new();
        let mut accepted_results = Vec::new();

        // Process each page
        for page_num in 0..page_count {
            if let Some(ref callback) = options.progress_callback {
                callback(page_num as usize, page_count as usize);
            }

            stats.pages_processed += 1;
            let analysis = analyzer
                .analyze_page(page_num as usize)
                .map_err(|error| PdfError::ParseError(error.to_string()))?;
            if analysis.is_scanned() && (!options.skip_text_pages || analysis.character_count < 50)
            {
                let image_data = analyzer
                    .extract_page_image_data(page_num as usize)
                    .map_err(|error| {
                        PdfError::ParseError(format!(
                            "Failed to extract image from page {page_num}: {error}"
                        ))
                    })?;
                let result = ocr_provider
                    .process_image(&image_data, &options.ocr_options)
                    .map_err(|error| {
                        PdfError::InvalidStructure(format!(
                            "OCR failed for page {page_num}: {error}"
                        ))
                    })?;
                if result.confidence >= options.min_confidence {
                    let fragments = result
                        .fragments
                        .iter()
                        .filter(|fragment| fragment.fragment_type == FragmentType::Word)
                        .enumerate()
                        .map(|(reading_order, fragment)| OcrLayerFragment {
                            text: fragment.text.clone(),
                            region: [fragment.x, fragment.y, fragment.width, fragment.height],
                            confidence: fragment.confidence,
                            reading_order: reading_order as u32,
                        })
                        .collect();
                    layers.push(OcrLayerPage {
                        page_index: page_num,
                        language: result.language.clone(),
                        fragments,
                    });
                    accepted_results.push((page_num, result.confidence, result.text.len()));
                }
            } else if options.skip_text_pages {
                stats.pages_skipped += 1;
            }
        }
        let update = IncrementalOcrLayerEditor::new(&base_bytes).apply(&layers)?;
        stats.pages_ocr_processed = update.plan.pages.len();
        for (_, confidence, characters) in accepted_results
            .iter()
            .filter(|(page, _, _)| update.plan.pages.binary_search(page).is_ok())
        {
            stats.total_confidence += confidence;
            stats.total_characters += characters;
        }
        stats.pages_skipped += update.plan.pages_skipped_existing_ocr.len();
        atomic_write(output_path.as_ref(), &update.pdf_bytes)?;

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

    /// Validate positioned OCR results without modifying or publishing a PDF.
    ///
    /// # Errors
    ///
    /// Returns an error for malformed, encrypted, or DocMDP-certified input,
    /// invalid page requests, or unsupported page content/resource layouts.
    pub fn plan_incremental_layers(
        &self,
        pdf_bytes: &[u8],
        pages: &[OcrLayerPage],
    ) -> Result<OcrLayerPlan> {
        IncrementalOcrLayerEditor::new(pdf_bytes).plan(pages)
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
                    tracing::debug!(
                        "✅ Converted: {} ({} pages)",
                        input_path.display(),
                        result.pages_processed
                    );
                    results.push(result);
                }
                Err(e) => {
                    tracing::debug!("❌ Failed to convert {}: {}", input_path.display(), e);
                }
            }
        }

        Ok(results)
    }
}

fn atomic_write(path: &Path, bytes: &[u8]) -> Result<()> {
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let mut temporary = tempfile::NamedTempFile::new_in(parent).map_err(PdfError::Io)?;
    temporary.write_all(bytes).map_err(PdfError::Io)?;
    temporary.flush().map_err(PdfError::Io)?;
    temporary.as_file().sync_all().map_err(PdfError::Io)?;
    temporary
        .persist(path)
        .map_err(|error| PdfError::Io(error.error))?;
    Ok(())
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

    #[test]
    fn atomic_write_replaces_existing_destination() {
        let directory = tempfile::tempdir().unwrap();
        let output = directory.path().join("output.pdf");
        std::fs::write(&output, b"old").unwrap();
        atomic_write(&output, b"new").unwrap();
        assert_eq!(std::fs::read(output).unwrap(), b"new");
    }
}
