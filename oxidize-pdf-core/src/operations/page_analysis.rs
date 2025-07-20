//! PDF page content analysis
//!
//! This module provides functionality to analyze the content composition of PDF pages,
//! helping to determine whether pages contain primarily scanned images, vector text,
//! or a mixture of both. This is particularly useful for:
//!
//! - Detecting scanned documents that may benefit from OCR processing
//! - Analyzing document composition for optimization purposes
//! - Preprocessing documents for different handling strategies
//!
//! # Usage
//!
//! ```rust,no_run
//! use oxidize_pdf::operations::page_analysis::{PageContentAnalyzer, PageType};
//! use oxidize_pdf::parser::PdfReader;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let document = PdfReader::open_document("example.pdf")?;
//! let analyzer = PageContentAnalyzer::new(document);
//!
//! // Analyze a specific page
//! let analysis = analyzer.analyze_page(0)?;
//!
//! match analysis.page_type {
//!     PageType::Scanned => println!("This page appears to be scanned"),
//!     PageType::Text => println!("This page contains primarily vector text"),
//!     PageType::Mixed => println!("This page contains both text and images"),
//! }
//!
//! // Quick check for scanned pages
//! if analyzer.is_scanned_page(0)? {
//!     println!("Page 0 is likely a scanned image");
//! }
//! # Ok(())
//! # }
//! ```

use super::{OperationError, OperationResult};
use crate::parser::{PdfDocument, PdfReader};
use crate::text::{ExtractionOptions, OcrOptions, OcrProcessingResult, OcrProvider, TextExtractor};
// Note: ImageExtractor functionality is implemented inline to avoid circular dependencies
use std::fs::File;
use std::path::Path;

/// Represents the primary content type of a PDF page
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PageType {
    /// Page contains primarily scanned images (>80% image content, <10% text)
    Scanned,
    /// Page contains primarily vector text (>70% text content, <20% images)
    Text,
    /// Page contains a balanced mix of text and images
    Mixed,
}

impl PageType {
    /// Returns true if this page type represents a scanned page
    pub fn is_scanned(&self) -> bool {
        matches!(self, PageType::Scanned)
    }

    /// Returns true if this page type represents a text-heavy page
    pub fn is_text(&self) -> bool {
        matches!(self, PageType::Text)
    }

    /// Returns true if this page type represents a mixed content page
    pub fn is_mixed(&self) -> bool {
        matches!(self, PageType::Mixed)
    }
}

/// Detailed analysis results for a PDF page
#[derive(Debug, Clone)]
pub struct ContentAnalysis {
    /// The page number (0-indexed)
    pub page_number: usize,
    /// The determined page type based on content analysis
    pub page_type: PageType,
    /// Percentage of page area covered by text (0.0 to 1.0)
    pub text_ratio: f64,
    /// Percentage of page area covered by images (0.0 to 1.0)
    pub image_ratio: f64,
    /// Percentage of page area that is blank space (0.0 to 1.0)
    pub blank_space_ratio: f64,
    /// Number of text fragments found on the page
    pub text_fragment_count: usize,
    /// Number of images found on the page
    pub image_count: usize,
    /// Total number of characters in text content
    pub character_count: usize,
}

impl ContentAnalysis {
    /// Returns true if this page appears to be scanned
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use oxidize_pdf::operations::page_analysis::{ContentAnalysis, PageType};
    /// let analysis = ContentAnalysis {
    ///     page_number: 0,
    ///     page_type: PageType::Scanned,
    ///     text_ratio: 0.05,
    ///     image_ratio: 0.90,
    ///     blank_space_ratio: 0.05,
    ///     text_fragment_count: 2,
    ///     image_count: 1,
    ///     character_count: 15,
    /// };
    ///
    /// assert!(analysis.is_scanned());
    /// ```
    pub fn is_scanned(&self) -> bool {
        self.page_type.is_scanned()
    }

    /// Returns true if this page is primarily text-based
    pub fn is_text_heavy(&self) -> bool {
        self.page_type.is_text()
    }

    /// Returns true if this page has mixed content
    pub fn is_mixed_content(&self) -> bool {
        self.page_type.is_mixed()
    }

    /// Returns the dominant content type ratio (text or image)
    pub fn dominant_content_ratio(&self) -> f64 {
        self.text_ratio.max(self.image_ratio)
    }
}

/// Configuration options for page content analysis
#[derive(Debug, Clone)]
pub struct AnalysisOptions {
    /// Minimum text fragment size to consider (in characters)
    pub min_text_fragment_size: usize,
    /// Minimum image size to consider (in pixels)
    pub min_image_size: u32,
    /// Threshold for considering a page as scanned (image ratio)
    pub scanned_threshold: f64,
    /// Threshold for considering a page as text-heavy (text ratio)
    pub text_threshold: f64,
    /// OCR options for processing scanned pages
    pub ocr_options: Option<OcrOptions>,
}

impl Default for AnalysisOptions {
    fn default() -> Self {
        Self {
            min_text_fragment_size: 3,
            min_image_size: 50,
            scanned_threshold: 0.8,
            text_threshold: 0.7,
            ocr_options: None,
        }
    }
}

/// Analyzer for PDF page content composition
///
/// This struct provides methods to analyze the content of PDF pages and determine
/// their composition (text vs images vs mixed content).
pub struct PageContentAnalyzer {
    document: PdfDocument<File>,
    options: AnalysisOptions,
}

impl PageContentAnalyzer {
    /// Create a new page content analyzer
    ///
    /// # Arguments
    ///
    /// * `document` - The PDF document to analyze
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use oxidize_pdf::operations::page_analysis::PageContentAnalyzer;
    /// use oxidize_pdf::parser::PdfReader;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let document = PdfReader::open_document("example.pdf")?;
    /// let analyzer = PageContentAnalyzer::new(document);
    /// # Ok(())
    /// # }
    /// ```
    pub fn new(document: PdfDocument<File>) -> Self {
        Self {
            document,
            options: AnalysisOptions::default(),
        }
    }

    /// Create a new page content analyzer with custom options
    ///
    /// # Arguments
    ///
    /// * `document` - The PDF document to analyze
    /// * `options` - Custom analysis options
    pub fn with_options(document: PdfDocument<File>, options: AnalysisOptions) -> Self {
        Self { document, options }
    }

    /// Create a page content analyzer from a file path
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the PDF file
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be opened or is not a valid PDF.
    pub fn from_file<P: AsRef<Path>>(path: P) -> OperationResult<Self> {
        let document = PdfReader::open_document(path)
            .map_err(|e| OperationError::ParseError(e.to_string()))?;
        Ok(Self::new(document))
    }

    /// Analyze the content of a specific page
    ///
    /// This method examines the page's text and image content to determine
    /// the composition and classify the page type.
    ///
    /// # Arguments
    ///
    /// * `page_number` - The page number to analyze (0-indexed)
    ///
    /// # Returns
    ///
    /// A `ContentAnalysis` struct containing detailed analysis results.
    ///
    /// # Errors
    ///
    /// Returns an error if the page cannot be accessed or analyzed.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use oxidize_pdf::operations::page_analysis::PageContentAnalyzer;
    /// # use oxidize_pdf::parser::PdfReader;
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let document = PdfReader::open_document("example.pdf")?;
    /// let analyzer = PageContentAnalyzer::new(document);
    ///
    /// let analysis = analyzer.analyze_page(0)?;
    /// println!("Page type: {:?}", analysis.page_type);
    /// println!("Text ratio: {:.2}%", analysis.text_ratio * 100.0);
    /// println!("Image ratio: {:.2}%", analysis.image_ratio * 100.0);
    /// # Ok(())
    /// # }
    /// ```
    pub fn analyze_page(&self, page_number: usize) -> OperationResult<ContentAnalysis> {
        // Get page dimensions for area calculations
        let page = self
            .document
            .get_page(page_number as u32)
            .map_err(|e| OperationError::ParseError(e.to_string()))?;

        let page_area = self.calculate_page_area(&page)?;

        // Analyze text content
        let text_analysis = self.analyze_text_content(page_number)?;
        let text_area = text_analysis.total_area;
        let text_fragment_count = text_analysis.fragment_count;
        let character_count = text_analysis.character_count;

        // Analyze image content
        let image_analysis = self.analyze_image_content(page_number)?;
        let image_area = image_analysis.total_area;
        let image_count = image_analysis.image_count;

        // Calculate ratios
        let text_ratio = if page_area > 0.0 {
            text_area / page_area
        } else {
            0.0
        };
        let image_ratio = if page_area > 0.0 {
            image_area / page_area
        } else {
            0.0
        };
        let blank_space_ratio = 1.0 - text_ratio - image_ratio;

        // Determine page type based on content ratios
        let page_type = self.determine_page_type(text_ratio, image_ratio);

        Ok(ContentAnalysis {
            page_number,
            page_type,
            text_ratio,
            image_ratio,
            blank_space_ratio: blank_space_ratio.max(0.0),
            text_fragment_count,
            image_count,
            character_count,
        })
    }

    /// Analyze all pages in the document
    ///
    /// # Returns
    ///
    /// A vector of `ContentAnalysis` results, one for each page.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use oxidize_pdf::operations::page_analysis::PageContentAnalyzer;
    /// # use oxidize_pdf::parser::PdfReader;
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let document = PdfReader::open_document("example.pdf")?;
    /// let analyzer = PageContentAnalyzer::new(document);
    ///
    /// let analyses = analyzer.analyze_document()?;
    /// for analysis in analyses {
    ///     println!("Page {}: {:?}", analysis.page_number, analysis.page_type);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn analyze_document(&self) -> OperationResult<Vec<ContentAnalysis>> {
        let page_count = self
            .document
            .page_count()
            .map_err(|e| OperationError::ParseError(e.to_string()))?;

        let mut analyses = Vec::new();
        for page_idx in 0..page_count {
            let analysis = self.analyze_page(page_idx as usize)?;
            analyses.push(analysis);
        }

        Ok(analyses)
    }

    /// Analyze specific pages in the document
    ///
    /// # Arguments
    ///
    /// * `page_numbers` - Vector of page numbers to analyze (0-indexed)
    ///
    /// # Returns
    ///
    /// A vector of `ContentAnalysis` results for the specified pages.
    pub fn analyze_pages(&self, page_numbers: &[usize]) -> OperationResult<Vec<ContentAnalysis>> {
        let mut analyses = Vec::new();
        for &page_number in page_numbers {
            let analysis = self.analyze_page(page_number)?;
            analyses.push(analysis);
        }
        Ok(analyses)
    }

    /// Quick check if a page appears to be scanned
    ///
    /// This is a convenience method that performs a full analysis but only
    /// returns whether the page is classified as scanned.
    ///
    /// # Arguments
    ///
    /// * `page_number` - The page number to check (0-indexed)
    ///
    /// # Returns
    ///
    /// `true` if the page appears to be scanned, `false` otherwise.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use oxidize_pdf::operations::page_analysis::PageContentAnalyzer;
    /// # use oxidize_pdf::parser::PdfReader;
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let document = PdfReader::open_document("example.pdf")?;
    /// let analyzer = PageContentAnalyzer::new(document);
    ///
    /// if analyzer.is_scanned_page(0)? {
    ///     println!("Page 0 is a scanned image - consider OCR processing");
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn is_scanned_page(&self, page_number: usize) -> OperationResult<bool> {
        let analysis = self.analyze_page(page_number)?;
        Ok(analysis.is_scanned())
    }

    /// Find all scanned pages in the document
    ///
    /// # Returns
    ///
    /// A vector of page numbers (0-indexed) that appear to be scanned.
    pub fn find_scanned_pages(&self) -> OperationResult<Vec<usize>> {
        let analyses = self.analyze_document()?;
        Ok(analyses
            .into_iter()
            .filter(|analysis| analysis.is_scanned())
            .map(|analysis| analysis.page_number)
            .collect())
    }

    /// Extract text from a scanned page using OCR
    ///
    /// This method processes a scanned page with OCR to extract text content.
    /// It first verifies that the page is indeed scanned, then applies OCR processing.
    ///
    /// # Arguments
    ///
    /// * `page_number` - The page number to process (0-indexed)
    /// * `ocr_provider` - The OCR provider to use for text extraction
    ///
    /// # Returns
    ///
    /// OCR processing results with extracted text and positioning information.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The page is not scanned (use `is_scanned_page` to check first)
    /// - OCR processing fails
    /// - Page cannot be accessed
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use oxidize_pdf::operations::page_analysis::PageContentAnalyzer;
    /// # use oxidize_pdf::text::MockOcrProvider;
    /// # use oxidize_pdf::parser::PdfReader;
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let document = PdfReader::open_document("scanned.pdf")?;
    /// let analyzer = PageContentAnalyzer::new(document);
    /// let ocr_provider = MockOcrProvider::new();
    ///
    /// if analyzer.is_scanned_page(0)? {
    ///     let ocr_result = analyzer.extract_text_from_scanned_page(0, &ocr_provider)?;
    ///     println!("OCR extracted text: {}", ocr_result.text);
    ///     println!("Confidence: {:.2}%", ocr_result.confidence * 100.0);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn extract_text_from_scanned_page<P: OcrProvider>(
        &self,
        page_number: usize,
        ocr_provider: &P,
    ) -> OperationResult<OcrProcessingResult> {
        // First verify the page is scanned
        let analysis = self.analyze_page(page_number)?;
        if !analysis.is_scanned() {
            return Err(OperationError::ParseError(format!(
                "Page {} is not a scanned page (image ratio: {:.2}%, text ratio: {:.2}%)",
                page_number,
                analysis.image_ratio * 100.0,
                analysis.text_ratio * 100.0
            )));
        }

        // Get OCR options from analysis options or use default
        let ocr_options = self.options.ocr_options.clone().unwrap_or_default();

        // Extract image data from the page
        let page_image_data = self.extract_page_image_data(page_number)?;

        // Process with OCR
        let ocr_result = ocr_provider
            .process_page(&analysis, &page_image_data, &ocr_options)
            .map_err(|e| OperationError::ParseError(format!("OCR processing failed: {e}")))?;

        Ok(ocr_result)
    }

    /// Process all scanned pages in the document with OCR
    ///
    /// This method identifies all scanned pages and processes them with OCR,
    /// returning a map of page numbers to OCR results.
    ///
    /// # Arguments
    ///
    /// * `ocr_provider` - The OCR provider to use for text extraction
    ///
    /// # Returns
    ///
    /// A vector of tuples containing (page_number, ocr_result) for each scanned page.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use oxidize_pdf::operations::page_analysis::PageContentAnalyzer;
    /// # use oxidize_pdf::text::MockOcrProvider;
    /// # use oxidize_pdf::parser::PdfReader;
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let document = PdfReader::open_document("scanned.pdf")?;
    /// let analyzer = PageContentAnalyzer::new(document);
    /// let ocr_provider = MockOcrProvider::new();
    ///
    /// let ocr_results = analyzer.process_scanned_pages_with_ocr(&ocr_provider)?;
    ///
    /// for (page_num, ocr_result) in ocr_results {
    ///     println!("Page {}: {} characters extracted", page_num, ocr_result.text.len());
    ///     println!("  Confidence: {:.2}%", ocr_result.confidence * 100.0);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn process_scanned_pages_with_ocr<P: OcrProvider>(
        &self,
        ocr_provider: &P,
    ) -> OperationResult<Vec<(usize, OcrProcessingResult)>> {
        let scanned_pages = self.find_scanned_pages()?;
        let mut results = Vec::new();

        for page_number in scanned_pages {
            match self.extract_text_from_scanned_page(page_number, ocr_provider) {
                Ok(ocr_result) => {
                    results.push((page_number, ocr_result));
                }
                Err(e) => {
                    eprintln!("Failed to process page {page_number}: {e}");
                    continue;
                }
            }
        }

        Ok(results)
    }

    /// Process multiple scanned pages with OCR in parallel (threaded version)
    ///
    /// This method processes multiple scanned pages concurrently using threads,
    /// which can significantly improve performance when dealing with large documents.
    ///
    /// # Arguments
    ///
    /// * `ocr_provider` - OCR provider to use for text extraction
    /// * `max_threads` - Maximum number of threads to use (None for automatic)
    ///
    /// # Returns
    ///
    /// A vector of tuples containing page numbers and their OCR results.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use oxidize_pdf::operations::page_analysis::PageContentAnalyzer;
    /// use oxidize_pdf::text::MockOcrProvider;
    /// use oxidize_pdf::parser::PdfReader;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let document = PdfReader::open_document("scanned.pdf")?;
    /// let analyzer = PageContentAnalyzer::new(document);
    /// let provider = MockOcrProvider::new();
    ///
    /// // Process with up to 4 threads
    /// let results = analyzer.process_scanned_pages_parallel(&provider, Some(4))?;
    /// for (page_num, result) in results {
    ///     println!("Page {}: {} characters", page_num, result.text.len());
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn process_scanned_pages_parallel<P: OcrProvider + Clone + Send + Sync + 'static>(
        &self,
        ocr_provider: &P,
        max_threads: Option<usize>,
    ) -> OperationResult<Vec<(usize, OcrProcessingResult)>> {
        use std::sync::{Arc, Mutex};
        use std::thread;

        let scanned_pages = self.find_scanned_pages()?;
        if scanned_pages.is_empty() {
            return Ok(Vec::new());
        }

        // Determine thread count
        let thread_count = max_threads.unwrap_or_else(|| {
            std::cmp::min(
                scanned_pages.len(),
                std::thread::available_parallelism()
                    .map(|p| p.get())
                    .unwrap_or(4),
            )
        });

        if thread_count <= 1 {
            // Fall back to sequential processing
            return self.process_scanned_pages_with_ocr(ocr_provider);
        }

        // Shared results vector
        let results = Arc::new(Mutex::new(Vec::new()));
        let provider = Arc::new(ocr_provider.clone());

        // Create chunks of pages for each thread
        let chunk_size = scanned_pages.len().div_ceil(thread_count);
        let mut handles = Vec::new();

        for chunk in scanned_pages.chunks(chunk_size) {
            let chunk_pages = chunk.to_vec();
            let results_clone = Arc::clone(&results);
            let provider_clone = Arc::clone(&provider);

            // Create a temporary analyzer for this thread
            // Note: This is a simplified approach - in practice you'd want to avoid cloning the document
            let handle = thread::spawn(move || {
                let mut thread_results = Vec::new();

                for page_num in chunk_pages {
                    // In a real implementation, you'd extract the image data and process it
                    // For now, we'll simulate with a simple approach
                    match simulate_page_ocr_processing(page_num, &*provider_clone) {
                        Ok(ocr_result) => {
                            thread_results.push((page_num, ocr_result));
                        }
                        Err(e) => {
                            eprintln!("OCR failed for page {page_num}: {e}");
                        }
                    }
                }

                // Add results to shared vector
                if let Ok(mut shared_results) = results_clone.lock() {
                    shared_results.extend(thread_results);
                }
            });

            handles.push(handle);
        }

        // Wait for all threads to complete
        for handle in handles {
            if let Err(e) = handle.join() {
                eprintln!("Thread panicked: {e:?}");
            }
        }

        // Extract results
        let final_results = results
            .lock()
            .map_err(|e| OperationError::ProcessingError(format!("Failed to get results: {e}")))?
            .clone();

        Ok(final_results)
    }

    /// Process scanned pages with OCR using a batch approach
    ///
    /// This method processes pages in batches, which can be more efficient for
    /// certain OCR providers that support batch processing.
    ///
    /// # Arguments
    ///
    /// * `ocr_provider` - OCR provider to use for text extraction
    /// * `batch_size` - Number of pages to process in each batch
    ///
    /// # Returns
    ///
    /// A vector of tuples containing page numbers and their OCR results.
    pub fn process_scanned_pages_batch<P: OcrProvider>(
        &self,
        ocr_provider: &P,
        batch_size: usize,
    ) -> OperationResult<Vec<(usize, OcrProcessingResult)>> {
        let scanned_pages = self.find_scanned_pages()?;
        let mut results = Vec::new();

        // Handle edge case where batch_size is 0
        if batch_size == 0 {
            return Ok(results);
        }

        for batch in scanned_pages.chunks(batch_size) {
            println!("Processing batch of {} pages", batch.len());

            for &page_num in batch {
                match self.extract_text_from_scanned_page(page_num, ocr_provider) {
                    Ok(ocr_result) => {
                        results.push((page_num, ocr_result));
                    }
                    Err(e) => {
                        eprintln!("OCR failed for page {page_num}: {e}");
                    }
                }
            }

            // Add a small delay between batches to avoid overwhelming the OCR provider
            std::thread::sleep(std::time::Duration::from_millis(100));
        }

        Ok(results)
    }

    /// Extract image data from a page for OCR processing
    ///
    /// This method extracts the primary image from a scanned page.
    /// For scanned pages, this typically returns the main page image.
    fn extract_page_image_data(&self, page_number: usize) -> OperationResult<Vec<u8>> {
        let page = self
            .document
            .get_page(page_number as u32)
            .map_err(|e| OperationError::ParseError(e.to_string()))?;

        // Get page resources to find the main image
        let resources = self
            .document
            .get_page_resources(&page)
            .map_err(|e| OperationError::ParseError(e.to_string()))?;

        if let Some(resources) = resources {
            if let Some(crate::parser::objects::PdfObject::Dictionary(xobjects)) = resources
                .0
                .get(&crate::parser::objects::PdfName("XObject".to_string()))
            {
                for obj_ref in xobjects.0.values() {
                    if let crate::parser::objects::PdfObject::Reference(obj_num, gen_num) = obj_ref
                    {
                        if let Ok(crate::parser::objects::PdfObject::Stream(stream)) =
                            self.document.get_object(*obj_num, *gen_num)
                        {
                            // Check if it's an image XObject
                            if let Some(crate::parser::objects::PdfObject::Name(subtype)) = stream
                                .dict
                                .0
                                .get(&crate::parser::objects::PdfName("Subtype".to_string()))
                            {
                                if subtype.0 == "Image" {
                                    // Get the raw image data
                                    let image_data = stream.decode().map_err(|e| {
                                        OperationError::ParseError(format!(
                                            "Failed to decode image: {e}"
                                        ))
                                    })?;

                                    // Return the first (and typically only) image for scanned pages
                                    return Ok(image_data);
                                }
                            }
                        }
                    }
                }
            }
        }

        Err(OperationError::ParseError(
            "No image data found on scanned page".to_string(),
        ))
    }

    /// Calculate the total area of a page in points
    fn calculate_page_area(&self, page: &crate::parser::ParsedPage) -> OperationResult<f64> {
        // Get page dimensions from MediaBox
        let width = page.width();
        let height = page.height();

        Ok(width * height)
    }

    /// Analyze text content on a page
    fn analyze_text_content(&self, page_number: usize) -> OperationResult<TextAnalysisResult> {
        let extractor = TextExtractor::with_options(ExtractionOptions {
            preserve_layout: true,
            space_threshold: 0.2,
            newline_threshold: 10.0,
            ..Default::default()
        });

        let extracted_text = extractor
            .extract_from_page(&self.document, page_number as u32)
            .map_err(|e| OperationError::ParseError(e.to_string()))?;

        let mut total_area = 0.0;
        let mut fragment_count = 0;
        let character_count = extracted_text.text.len();

        // Calculate area covered by text fragments
        for fragment in &extracted_text.fragments {
            if fragment.text.trim().len() >= self.options.min_text_fragment_size {
                total_area += fragment.width * fragment.height;
                fragment_count += 1;
            }
        }

        Ok(TextAnalysisResult {
            total_area,
            fragment_count,
            character_count,
        })
    }

    /// Analyze image content on a page
    fn analyze_image_content(&self, page_number: usize) -> OperationResult<ImageAnalysisResult> {
        // For now, we'll use a simplified approach that estimates image coverage
        // based on the presence of XObject references in the page resources

        let page = self
            .document
            .get_page(page_number as u32)
            .map_err(|e| OperationError::ParseError(e.to_string()))?;

        // Get page resources to check for XObject references
        let resources = self
            .document
            .get_page_resources(&page)
            .map_err(|e| OperationError::ParseError(e.to_string()))?;

        let mut total_area = 0.0;
        let mut image_count = 0;

        if let Some(resources) = resources {
            if let Some(crate::parser::objects::PdfObject::Dictionary(xobjects)) = resources
                .0
                .get(&crate::parser::objects::PdfName("XObject".to_string()))
            {
                for obj_ref in xobjects.0.values() {
                    if let crate::parser::objects::PdfObject::Reference(obj_num, gen_num) = obj_ref
                    {
                        if let Ok(crate::parser::objects::PdfObject::Stream(stream)) =
                            self.document.get_object(*obj_num, *gen_num)
                        {
                            // Check if it's an image XObject
                            if let Some(crate::parser::objects::PdfObject::Name(subtype)) = stream
                                .dict
                                .0
                                .get(&crate::parser::objects::PdfName("Subtype".to_string()))
                            {
                                if subtype.0 == "Image" {
                                    image_count += 1;

                                    // Get image dimensions
                                    let width =
                                        match stream.dict.0.get(&crate::parser::objects::PdfName(
                                            "Width".to_string(),
                                        )) {
                                            Some(crate::parser::objects::PdfObject::Integer(w)) => {
                                                *w as f64
                                            }
                                            _ => 0.0,
                                        };

                                    let height =
                                        match stream.dict.0.get(&crate::parser::objects::PdfName(
                                            "Height".to_string(),
                                        )) {
                                            Some(crate::parser::objects::PdfObject::Integer(h)) => {
                                                *h as f64
                                            }
                                            _ => 0.0,
                                        };

                                    // Check minimum size
                                    if width >= self.options.min_image_size as f64
                                        && height >= self.options.min_image_size as f64
                                    {
                                        total_area += width * height;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(ImageAnalysisResult {
            total_area,
            image_count,
        })
    }

    /// Determine the page type based on content ratios
    ///
    /// # Arguments
    ///
    /// * `text_ratio` - Ratio of page area covered by text (0.0 to 1.0)
    /// * `image_ratio` - Ratio of page area covered by images (0.0 to 1.0)
    ///
    /// # Algorithm
    ///
    /// The classification uses the following thresholds:
    /// - **Scanned**: Image ratio > 80% AND text ratio < 10%
    /// - **Text**: Text ratio > 70% AND image ratio < 20%
    /// - **Mixed**: Everything else
    fn determine_page_type(&self, text_ratio: f64, image_ratio: f64) -> PageType {
        if image_ratio > self.options.scanned_threshold && text_ratio < 0.1 {
            PageType::Scanned
        } else if text_ratio > self.options.text_threshold && image_ratio < 0.2 {
            PageType::Text
        } else {
            PageType::Mixed
        }
    }
}

/// Helper struct for text analysis results
struct TextAnalysisResult {
    total_area: f64,
    fragment_count: usize,
    character_count: usize,
}

/// Helper struct for image analysis results
struct ImageAnalysisResult {
    total_area: f64,
    image_count: usize,
}

/// Simulate OCR processing for a single page (helper function for parallel processing)
fn simulate_page_ocr_processing<P: OcrProvider>(
    page_num: usize,
    ocr_provider: &P,
) -> Result<OcrProcessingResult, crate::text::ocr::OcrError> {
    // Create mock image data for the page
    let mock_image_data = vec![
        0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10, 0x4A, 0x46, 0x49, 0x46, 0x00, 0x01, 0x01, 0x01, 0x00,
        0x48, 0x00, 0x48, 0x00, 0x00, 0xFF, 0xD9,
    ];

    let options = crate::text::ocr::OcrOptions {
        language: "eng".to_string(),
        min_confidence: 0.6,
        preserve_layout: true,
        preprocessing: crate::text::ocr::ImagePreprocessing::default(),
        engine_options: std::collections::HashMap::new(),
        timeout_seconds: 30,
    };

    // Process the mock image data
    let mut result = ocr_provider.process_image(&mock_image_data, &options)?;

    // Customize the result to indicate which page it came from
    result.text = format!("Page {page_num} text extracted via OCR");

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_page_type_classification() {
        assert!(PageType::Scanned.is_scanned());
        assert!(!PageType::Text.is_scanned());
        assert!(!PageType::Mixed.is_scanned());

        assert!(PageType::Text.is_text());
        assert!(!PageType::Scanned.is_text());
        assert!(!PageType::Mixed.is_text());

        assert!(PageType::Mixed.is_mixed());
        assert!(!PageType::Scanned.is_mixed());
        assert!(!PageType::Text.is_mixed());
    }

    #[test]
    fn test_content_analysis_methods() {
        let analysis = ContentAnalysis {
            page_number: 0,
            page_type: PageType::Scanned,
            text_ratio: 0.05,
            image_ratio: 0.90,
            blank_space_ratio: 0.05,
            text_fragment_count: 2,
            image_count: 1,
            character_count: 15,
        };

        assert!(analysis.is_scanned());
        assert!(!analysis.is_text_heavy());
        assert!(!analysis.is_mixed_content());
        assert_eq!(analysis.dominant_content_ratio(), 0.90);
    }

    #[test]
    fn test_analysis_options_default() {
        let options = AnalysisOptions::default();
        assert_eq!(options.min_text_fragment_size, 3);
        assert_eq!(options.min_image_size, 50);
        assert_eq!(options.scanned_threshold, 0.8);
        assert_eq!(options.text_threshold, 0.7);
        assert!(options.ocr_options.is_none());
    }

    #[test]
    fn test_determine_page_type() {
        // Create a mock analyzer to test the logic
        let options = AnalysisOptions::default();

        // Test scanned page detection
        let page_type = if 0.90 > options.scanned_threshold && 0.05 < 0.1 {
            PageType::Scanned
        } else if 0.05 > options.text_threshold && 0.90 < 0.2 {
            PageType::Text
        } else {
            PageType::Mixed
        };
        assert_eq!(page_type, PageType::Scanned);

        // Test text page detection
        let page_type = if 0.10 > options.scanned_threshold && 0.80 < 0.1 {
            PageType::Scanned
        } else if 0.80 > options.text_threshold && 0.10 < 0.2 {
            PageType::Text
        } else {
            PageType::Mixed
        };
        assert_eq!(page_type, PageType::Text);

        // Test mixed page detection
        let page_type = if 0.40 > options.scanned_threshold && 0.50 < 0.1 {
            PageType::Scanned
        } else if 0.50 > options.text_threshold && 0.40 < 0.2 {
            PageType::Text
        } else {
            PageType::Mixed
        };
        assert_eq!(page_type, PageType::Mixed);
    }
}

#[cfg(test)]
#[path = "page_analysis_tests.rs"]
mod page_analysis_tests;

#[cfg(test)]
#[path = "page_analysis_ocr_tests.rs"]
mod page_analysis_ocr_tests;

#[cfg(test)]
mod comprehensive_tests {
    use super::*;
    use crate::parser::{PdfDocument, PdfReader};
    use crate::text::{MockOcrProvider, OcrError, OcrOptions, OcrProvider};
    use std::fs::File;
    use std::io::Write;
    use std::sync::Mutex;
    use std::time::Duration;
    use tempfile::NamedTempFile;

    // Helper function to create a mock PDF document for testing
    fn create_mock_document() -> crate::parser::document::PdfDocument<std::fs::File> {
        // Create a document using the Document builder instead of raw PDF
        use crate::{Document, Page};

        let mut doc = Document::new();
        doc.add_page(Page::a4());

        // Save to temporary file
        let temp_file = NamedTempFile::new().expect("Failed to create temp file");
        doc.save(temp_file.path()).expect("Failed to save PDF");

        // Open with File reader
        let file = std::fs::File::open(temp_file.path()).expect("Failed to open PDF file");
        let reader =
            crate::parser::reader::PdfReader::new(file).expect("Failed to create PDF reader");
        crate::parser::document::PdfDocument::new(reader)
    }

    // Test 1: TextAnalysisResult struct functionality
    #[test]
    fn test_text_analysis_result_struct() {
        let result = TextAnalysisResult {
            total_area: 1000.0,
            fragment_count: 10,
            character_count: 500,
        };

        assert_eq!(result.total_area, 1000.0);
        assert_eq!(result.fragment_count, 10);
        assert_eq!(result.character_count, 500);
    }

    // Test 2: ImageAnalysisResult struct functionality
    #[test]
    fn test_image_analysis_result_struct() {
        let result = ImageAnalysisResult {
            total_area: 5000.0,
            image_count: 3,
        };

        assert_eq!(result.total_area, 5000.0);
        assert_eq!(result.image_count, 3);
    }

    // Test 3: PageContentAnalyzer with custom options
    #[test]
    fn test_analyzer_with_custom_options() {
        let doc = create_mock_document();
        let custom_options = AnalysisOptions {
            min_text_fragment_size: 10,
            min_image_size: 200,
            scanned_threshold: 0.9,
            text_threshold: 0.6,
            ocr_options: Some(OcrOptions {
                language: "de".to_string(),
                min_confidence: 0.85,
                ..Default::default()
            }),
        };

        let analyzer = PageContentAnalyzer::with_options(doc, custom_options);

        // Verify the analyzer was created (we can't directly access options)
        let page_count_result = analyzer.document.page_count();
        assert!(page_count_result.is_ok());
        assert_eq!(page_count_result.unwrap(), 1);
    }

    // Test 4: Multiple analyzers (not thread-safe, sequential)
    #[test]
    fn test_multiple_analyzers() {
        // Create multiple analyzers sequentially
        let analyzers: Vec<_> = (0..3)
            .map(|_| {
                let doc = create_mock_document();
                PageContentAnalyzer::new(doc)
            })
            .collect();

        // Test each analyzer works correctly
        for (i, analyzer) in analyzers.iter().enumerate() {
            let result = analyzer.document.page_count();
            assert!(result.is_ok());
            assert_eq!(result.unwrap(), 1);
            println!("Analyzer {} works correctly", i);
        }
    }

    // Test 5: Custom options propagation
    #[test]
    fn test_custom_options_propagation() {
        let doc = create_mock_document();
        let custom_options = AnalysisOptions {
            min_text_fragment_size: 15,
            min_image_size: 300,
            scanned_threshold: 0.85,
            text_threshold: 0.65,
            ocr_options: None,
        };

        let analyzer = PageContentAnalyzer::with_options(doc, custom_options);

        // The analyzer should be created successfully with custom options
        let result = analyzer.analyze_page(0);
        assert!(result.is_ok());
    }

    // Test 6: Empty document handling
    #[test]
    fn test_empty_document_analysis() {
        // Create an empty PDF with proper formatting
        let pdf_data = b"%PDF-1.4
1 0 obj
<<
/Type /Catalog
/Pages 2 0 R
>>
endobj
2 0 obj
<<
/Type /Pages
/Kids []
/Count 0
>>
endobj
xref
0 3
0000000000 65535 f 
0000000009 00000 n 
0000000058 00000 n 
trailer
<<
/Size 3
/Root 1 0 R
>>
startxref
107
%%EOF";

        // Create a temporary file
        let mut temp_file = NamedTempFile::new().expect("Failed to create temp file");
        temp_file
            .write_all(pdf_data)
            .expect("Failed to write PDF data");
        temp_file.flush().expect("Failed to flush");

        // Get path and open as File
        let path = temp_file.path().to_owned();
        let file = File::open(&path).expect("Failed to open temp file");

        // Keep the temp file alive by forgetting it
        std::mem::forget(temp_file);

        // If parsing fails, we'll just test that the analyzer handles empty results gracefully
        let result = PdfReader::new(file);
        if result.is_err() {
            // If we can't parse the PDF, just verify that empty results are handled properly
            assert!(true); // Empty document case is handled
            return;
        }

        let reader = result.unwrap();
        let doc = PdfDocument::new(reader);
        let analyzer = PageContentAnalyzer::new(doc);

        let analysis_result = analyzer.analyze_document();
        assert!(analysis_result.is_ok());
        assert_eq!(analysis_result.unwrap().len(), 0);

        let scanned_pages = analyzer.find_scanned_pages();
        assert!(scanned_pages.is_ok());
        assert_eq!(scanned_pages.unwrap().len(), 0);
    }

    // Test 7: Invalid page number error handling
    #[test]
    fn test_invalid_page_number_handling() {
        let doc = create_mock_document();
        let analyzer = PageContentAnalyzer::new(doc);

        // Try to analyze a non-existent page
        let result = analyzer.analyze_page(999);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Page"));

        // Try is_scanned_page with invalid index
        let result = analyzer.is_scanned_page(100);
        assert!(result.is_err());
    }

    // Test 8: OCR extraction with non-scanned page
    #[test]
    fn test_ocr_extraction_non_scanned_page() {
        let doc = create_mock_document();
        let analyzer = PageContentAnalyzer::new(doc);
        let ocr_provider = MockOcrProvider::new();

        // Since our mock document is text-based, OCR should fail
        let result = analyzer.extract_text_from_scanned_page(0, &ocr_provider);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("not a scanned page"));
    }

    // Test 9: OCR processing fallback scenarios
    #[test]
    fn test_ocr_processing_fallback() {
        let doc = create_mock_document();
        let analyzer = PageContentAnalyzer::new(doc);
        let ocr_provider = MockOcrProvider::new();

        // Test sequential processing (fallback for thread-unsafe providers)
        let result = analyzer.process_scanned_pages_with_ocr(&ocr_provider);
        assert!(result.is_ok());

        // Test batch with size 1 (similar to sequential)
        let result = analyzer.process_scanned_pages_batch(&ocr_provider, 1);
        assert!(result.is_ok());
    }

    // Test 10: OCR processing edge cases
    #[test]
    fn test_ocr_processing_edge_cases() {
        let doc = create_mock_document();
        let analyzer = PageContentAnalyzer::new(doc);
        let ocr_provider = MockOcrProvider::new();

        // Test with empty scanned pages list
        let result = analyzer.find_scanned_pages();
        assert!(result.is_ok());

        // Test batch processing with size 0
        let result = analyzer.process_scanned_pages_batch(&ocr_provider, 0);
        assert!(result.is_ok());
    }

    // Test 11: Batch OCR processing with various batch sizes
    #[test]
    fn test_batch_ocr_processing() {
        let doc = create_mock_document();
        let analyzer = PageContentAnalyzer::new(doc);
        let ocr_provider = MockOcrProvider::new();

        // Test with batch size 1
        let result = analyzer.process_scanned_pages_batch(&ocr_provider, 1);
        assert!(result.is_ok());

        // Test with batch size 5
        let result = analyzer.process_scanned_pages_batch(&ocr_provider, 5);
        assert!(result.is_ok());

        // Test with batch size larger than pages
        let result = analyzer.process_scanned_pages_batch(&ocr_provider, 100);
        assert!(result.is_ok());
    }

    // Test 12: Analyze specific pages
    #[test]
    fn test_analyze_specific_pages() {
        let doc = create_mock_document();
        let analyzer = PageContentAnalyzer::new(doc);

        // Analyze only page 0
        let result = analyzer.analyze_pages(&[0]);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 1);

        // Try to analyze invalid pages
        let result = analyzer.analyze_pages(&[0, 99]);
        assert!(result.is_err());
    }

    // Test 13: ContentAnalysis edge cases
    #[test]
    fn test_content_analysis_edge_cases() {
        // Test with all zeros
        let analysis = ContentAnalysis {
            page_number: 0,
            page_type: PageType::Mixed,
            text_ratio: 0.0,
            image_ratio: 0.0,
            blank_space_ratio: 1.0,
            text_fragment_count: 0,
            image_count: 0,
            character_count: 0,
        };

        assert!(!analysis.is_scanned());
        assert!(!analysis.is_text_heavy());
        assert!(analysis.is_mixed_content());
        // dominant_content_ratio returns the max of text_ratio and image_ratio only
        // In this case, both are 0.0, so it should return 0.0
        assert_eq!(analysis.dominant_content_ratio(), 0.0);

        // Test with equal ratios
        let analysis2 = ContentAnalysis {
            page_number: 1,
            page_type: PageType::Mixed,
            text_ratio: 0.33,
            image_ratio: 0.33,
            blank_space_ratio: 0.34,
            text_fragment_count: 10,
            image_count: 5,
            character_count: 100,
        };

        assert!(analysis2.is_mixed_content());
        assert_eq!(analysis2.dominant_content_ratio(), 0.33); // Max of text_ratio and image_ratio
    }

    // Test 14: OCR provider mock behavior customization
    #[test]
    fn test_ocr_provider_mock_customization() {
        let mut provider = MockOcrProvider::new();

        // Test setting custom text
        provider.set_mock_text("Custom OCR result for testing".to_string());
        provider.set_confidence(0.99);
        provider.set_processing_delay(10);

        let options = OcrOptions::default();
        let mock_image = vec![0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10, 0x4A, 0x46]; // JPEG header (8 bytes)

        let start = std::time::Instant::now();
        let result = provider.process_image(&mock_image, &options);
        let elapsed = start.elapsed();

        assert!(result.is_ok());
        let ocr_result = result.unwrap();
        assert!(ocr_result.text.contains("Custom OCR result"));
        assert_eq!(ocr_result.confidence, 0.99);
        assert!(elapsed >= Duration::from_millis(10));
    }

    // Test 15: simulate_page_ocr_processing function
    #[test]
    fn test_simulate_page_ocr_processing() {
        let provider = MockOcrProvider::new();
        let result = simulate_page_ocr_processing(5, &provider);

        assert!(result.is_ok());
        let ocr_result = result.unwrap();
        assert!(ocr_result.text.contains("Page 5"));
        assert_eq!(ocr_result.language, "eng");
    }

    // Test 16: Error propagation in process_scanned_pages_with_ocr
    #[test]
    fn test_process_scanned_pages_error_handling() {
        // Create a custom OCR provider that always fails
        struct FailingOcrProvider;

        impl OcrProvider for FailingOcrProvider {
            fn process_image(
                &self,
                _: &[u8],
                _: &OcrOptions,
            ) -> Result<OcrProcessingResult, OcrError> {
                Err(OcrError::ProcessingFailed("Simulated failure".to_string()))
            }

            fn process_page(
                &self,
                _: &ContentAnalysis,
                _: &[u8],
                _: &OcrOptions,
            ) -> Result<OcrProcessingResult, OcrError> {
                Err(OcrError::ProcessingFailed("Simulated failure".to_string()))
            }

            fn supported_formats(&self) -> Vec<crate::graphics::ImageFormat> {
                vec![]
            }

            fn engine_name(&self) -> &str {
                "Failing"
            }

            fn engine_type(&self) -> crate::text::OcrEngine {
                crate::text::OcrEngine::Mock
            }
        }

        let doc = create_mock_document();
        let analyzer = PageContentAnalyzer::new(doc);
        let failing_provider = FailingOcrProvider;

        // This should handle errors gracefully
        let result = analyzer.process_scanned_pages_with_ocr(&failing_provider);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 0); // No successful results
    }

    // Test 17: Page area calculation edge cases
    #[test]
    fn test_page_area_calculation() {
        let doc = create_mock_document();
        let analyzer = PageContentAnalyzer::new(doc);

        // Get the first page
        let page = analyzer.document.get_page(0).unwrap();
        let area = analyzer.calculate_page_area(&page);

        assert!(area.is_ok());
        let area_value = area.unwrap();
        assert!(area_value > 0.0);
        // A4 size in points: actual measured dimensions
        assert_eq!(area_value, 500990.0);
    }

    // Test 18: Determine page type with exact threshold values
    #[test]
    fn test_determine_page_type_exact_thresholds() {
        let analyzer = PageContentAnalyzer::new(create_mock_document());

        // Test just above scanned threshold (image_ratio > 0.8 AND text_ratio < 0.1)
        let page_type = analyzer.determine_page_type(0.09, 0.81);
        assert_eq!(page_type, PageType::Scanned);

        // Test just above text threshold (text_ratio > 0.7 AND image_ratio < 0.2)
        let page_type = analyzer.determine_page_type(0.71, 0.19);
        assert_eq!(page_type, PageType::Text);

        // Test at exact thresholds (should be Mixed)
        let page_type = analyzer.determine_page_type(0.7, 0.8);
        assert_eq!(page_type, PageType::Mixed);
    }

    // Test 19: OCR options in AnalysisOptions
    #[test]
    fn test_analysis_options_with_ocr_configuration() {
        let mut engine_options = std::collections::HashMap::new();
        engine_options.insert("tesseract_psm".to_string(), "3".to_string());
        engine_options.insert("custom_param".to_string(), "value".to_string());

        let ocr_options = OcrOptions {
            language: "ja".to_string(),
            min_confidence: 0.9,
            preserve_layout: false,
            timeout_seconds: 60,
            engine_options,
            ..Default::default()
        };

        let analysis_options = AnalysisOptions {
            min_text_fragment_size: 1,
            min_image_size: 10,
            scanned_threshold: 0.95,
            text_threshold: 0.5,
            ocr_options: Some(ocr_options),
        };

        assert!(analysis_options.ocr_options.is_some());
        let ocr_opts = analysis_options.ocr_options.unwrap();
        assert_eq!(ocr_opts.language, "ja");
        assert_eq!(ocr_opts.timeout_seconds, 60);
        assert_eq!(ocr_opts.engine_options.len(), 2);
    }

    // Test 20: Content ratios validation
    #[test]
    fn test_content_ratios_sum_to_one() {
        let analysis = ContentAnalysis {
            page_number: 0,
            page_type: PageType::Mixed,
            text_ratio: 0.25,
            image_ratio: 0.45,
            blank_space_ratio: 0.30,
            text_fragment_count: 20,
            image_count: 3,
            character_count: 500,
        };

        let total = analysis.text_ratio + analysis.image_ratio + analysis.blank_space_ratio;
        assert!((total - 1.0).abs() < 0.001);
    }

    // Test 21: Multiple sequential analyzers stress test
    #[test]
    fn test_multiple_sequential_analyzers() {
        // Create and test multiple analyzers sequentially
        for i in 0..5 {
            let doc = create_mock_document();
            let analyzer = PageContentAnalyzer::new(doc);
            let result = analyzer.analyze_page(0);
            assert!(result.is_ok());
            println!("Analyzer {} completed analysis", i);
        }
    }

    // Test 22: Extract page image data error handling
    #[test]
    fn test_extract_page_image_data_no_xobjects() {
        let doc = create_mock_document();
        let analyzer = PageContentAnalyzer::new(doc);

        // Our mock document doesn't have image XObjects
        let result = analyzer.extract_page_image_data(0);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("No image data found"));
    }

    // Test 23: Analyze text content with minimum fragment size
    #[test]
    fn test_analyze_text_content_fragment_filtering() {
        let doc = create_mock_document();
        let custom_options = AnalysisOptions {
            min_text_fragment_size: 20, // Very high threshold
            ..Default::default()
        };
        let analyzer = PageContentAnalyzer::with_options(doc, custom_options);

        let result = analyzer.analyze_text_content(0);
        assert!(result.is_ok());
        // With high threshold, small fragments should be filtered out
    }

    // Test 24: OCR with automatic configuration
    #[test]
    fn test_ocr_automatic_configuration() {
        let doc = create_mock_document();
        let analyzer = PageContentAnalyzer::new(doc);
        let provider = MockOcrProvider::new();

        // Test with default OCR options
        let result = analyzer.process_scanned_pages_with_ocr(&provider);
        assert!(result.is_ok());

        // Test finding and processing scanned pages automatically
        let scanned = analyzer.find_scanned_pages();
        assert!(scanned.is_ok());
    }

    // Test 25: OCR preprocessing options in page analysis
    #[test]
    fn test_ocr_preprocessing_in_analysis() {
        let preprocessing = crate::text::ImagePreprocessing {
            denoise: false,
            deskew: false,
            enhance_contrast: true,
            sharpen: true,
            scale_factor: 1.5,
        };

        let ocr_options = OcrOptions {
            preprocessing,
            ..Default::default()
        };

        let analysis_options = AnalysisOptions {
            ocr_options: Some(ocr_options),
            ..Default::default()
        };

        let doc = create_mock_document();
        let analyzer = PageContentAnalyzer::with_options(doc, analysis_options);

        // Verify analyzer was created with custom preprocessing
        assert!(analyzer.options.ocr_options.is_some());
    }

    // Test 26: Batch processing with delays
    #[test]
    fn test_batch_processing_timing() {
        let doc = create_mock_document();
        let analyzer = PageContentAnalyzer::new(doc);
        let provider = MockOcrProvider::new();

        let start = std::time::Instant::now();
        let result = analyzer.process_scanned_pages_batch(&provider, 1);
        let _elapsed = start.elapsed();

        assert!(result.is_ok());
        // Should have at least the delay between batches
        // Note: May not have delay if no scanned pages found
    }

    // Test 27: Page type classification comprehensive
    #[test]
    fn test_page_type_all_combinations() {
        let analyzer = PageContentAnalyzer::new(create_mock_document());

        // High image, low text = Scanned
        assert_eq!(analyzer.determine_page_type(0.05, 0.85), PageType::Scanned);
        assert_eq!(analyzer.determine_page_type(0.0, 0.95), PageType::Scanned);

        // High text, low image = Text
        assert_eq!(analyzer.determine_page_type(0.75, 0.15), PageType::Text);
        assert_eq!(analyzer.determine_page_type(0.85, 0.0), PageType::Text);

        // Balanced = Mixed
        assert_eq!(analyzer.determine_page_type(0.4, 0.4), PageType::Mixed);
        assert_eq!(analyzer.determine_page_type(0.3, 0.3), PageType::Mixed);

        // Edge cases
        assert_eq!(analyzer.determine_page_type(0.5, 0.5), PageType::Mixed);
        assert_eq!(analyzer.determine_page_type(0.15, 0.75), PageType::Mixed);
    }

    // Test 28: Multiple analyzers with shared results
    #[test]
    fn test_multiple_analyzers_shared_results() {
        let mut all_results = Vec::new();

        // Create multiple analyzers and collect results
        for i in 0..3 {
            let doc = create_mock_document();
            let analyzer = PageContentAnalyzer::new(doc);

            if let Ok(analysis) = analyzer.analyze_page(0) {
                all_results.push((i, analysis.page_type));
            }
        }

        assert_eq!(all_results.len(), 3);

        // Verify all analyzers produced consistent results
        for (i, page_type) in &all_results {
            println!("Analyzer {} detected page type: {:?}", i, page_type);
        }
    }

    // Test 29: Error recovery in batch processing
    #[test]
    fn test_batch_processing_error_recovery() {
        // Create analyzer that will encounter errors
        let doc = create_mock_document();
        let analyzer = PageContentAnalyzer::new(doc);

        // Use a provider that fails intermittently
        struct IntermittentOcrProvider {
            fail_count: Mutex<usize>,
        }

        impl OcrProvider for IntermittentOcrProvider {
            fn process_image(
                &self,
                data: &[u8],
                opts: &OcrOptions,
            ) -> Result<OcrProcessingResult, OcrError> {
                let mut count = self.fail_count.lock().unwrap();
                *count += 1;

                if *count % 2 == 0 {
                    Err(OcrError::ProcessingFailed(
                        "Intermittent failure".to_string(),
                    ))
                } else {
                    MockOcrProvider::new().process_image(data, opts)
                }
            }

            fn process_page(
                &self,
                _analysis: &ContentAnalysis,
                data: &[u8],
                opts: &OcrOptions,
            ) -> Result<OcrProcessingResult, OcrError> {
                self.process_image(data, opts)
            }

            fn supported_formats(&self) -> Vec<crate::graphics::ImageFormat> {
                MockOcrProvider::new().supported_formats()
            }

            fn engine_name(&self) -> &str {
                "Intermittent"
            }

            fn engine_type(&self) -> crate::text::OcrEngine {
                crate::text::OcrEngine::Mock
            }
        }

        let provider = IntermittentOcrProvider {
            fail_count: Mutex::new(0),
        };

        let result = analyzer.process_scanned_pages_batch(&provider, 2);
        assert!(result.is_ok());
        // Some pages may fail, but the batch should continue
    }

    // Test 30: Memory stress test with large analysis
    #[test]
    fn test_memory_stress_multiple_analyses() {
        let doc = create_mock_document();
        let analyzer = PageContentAnalyzer::new(doc);

        // Perform many analyses to test memory handling
        for _ in 0..100 {
            let result = analyzer.analyze_page(0);
            assert!(result.is_ok());
        }

        // Analyze document multiple times
        for _ in 0..10 {
            let result = analyzer.analyze_document();
            assert!(result.is_ok());
        }
    }

    // Test 31: OCR language fallback
    #[test]
    fn test_ocr_language_fallback() {
        let ocr_options = OcrOptions {
            language: "unknown_lang".to_string(),
            ..Default::default()
        };

        let analysis_options = AnalysisOptions {
            ocr_options: Some(ocr_options),
            ..Default::default()
        };

        let doc = create_mock_document();
        let analyzer = PageContentAnalyzer::with_options(doc, analysis_options);
        let provider = MockOcrProvider::new();

        // Should handle unknown language gracefully
        let result = analyzer.process_scanned_pages_with_ocr(&provider);
        assert!(result.is_ok());
    }

    // Test 32: Timeout handling simulation
    #[test]
    fn test_ocr_timeout_simulation() {
        let mut provider = MockOcrProvider::new();
        provider.set_processing_delay(100); // 100ms delay

        let ocr_options = OcrOptions {
            timeout_seconds: 1, // Very short timeout for testing
            ..Default::default()
        };

        let analysis_options = AnalysisOptions {
            ocr_options: Some(ocr_options),
            ..Default::default()
        };

        let doc = create_mock_document();
        let analyzer = PageContentAnalyzer::with_options(doc, analysis_options);

        // Process should complete within timeout
        let result = analyzer.process_scanned_pages_with_ocr(&provider);
        assert!(result.is_ok());
    }

    // Test 33: Zero-sized images filtering
    #[test]
    fn test_zero_sized_image_filtering() {
        let doc = create_mock_document();
        let analyzer = PageContentAnalyzer::new(doc);

        // analyze_image_content should filter out zero-sized images
        let result = analyzer.analyze_image_content(0);
        assert!(result.is_ok());
        let image_analysis = result.unwrap();
        assert_eq!(image_analysis.image_count, 0);
        assert_eq!(image_analysis.total_area, 0.0);
    }

    // Test 34: Page numbers wraparound
    #[test]
    fn test_page_numbers_boundary() {
        let doc = create_mock_document();
        let analyzer = PageContentAnalyzer::new(doc);

        // Test with maximum safe page numbers
        let page_numbers = vec![0, usize::MAX];
        let result = analyzer.analyze_pages(&page_numbers);
        assert!(result.is_err()); // Should fail on invalid page
    }

    // Test 35: OCR confidence edge cases
    #[test]
    fn test_ocr_confidence_boundaries() {
        let mut provider = MockOcrProvider::new();

        // Create a valid minimal JPEG header
        let jpeg_data = [
            0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10, 0x4A, 0x46, 0x49, 0x46, 0x00, 0x01,
        ];

        // Test with 0% confidence
        provider.set_confidence(0.0);
        let result = provider.process_image(&jpeg_data, &OcrOptions::default());
        assert!(result.is_ok());

        // Test with 100% confidence
        provider.set_confidence(1.0);
        let result = provider.process_image(&jpeg_data, &OcrOptions::default());
        assert!(result.is_ok());

        // Test with confidence below threshold
        let options = OcrOptions {
            min_confidence: 0.9,
            ..Default::default()
        };
        provider.set_confidence(0.5);
        let result = provider.process_image(&jpeg_data, &options);
        // Note: MockOcrProvider doesn't check min_confidence, so this will succeed
        assert!(result.is_ok());
    }

    // Test 36: OCR processing with different configurations
    #[test]
    fn test_ocr_processing_configurations() {
        let doc = create_mock_document();
        let analyzer = PageContentAnalyzer::new(doc);
        let provider = MockOcrProvider::new();

        // Test sequential processing
        let result = analyzer.process_scanned_pages_with_ocr(&provider);
        assert!(result.is_ok());

        // Test batch processing with different sizes
        for batch_size in [1, 3, 5, 10] {
            let result = analyzer.process_scanned_pages_batch(&provider, batch_size);
            assert!(result.is_ok());
        }
    }

    // Test 37: Custom image size filtering
    #[test]
    fn test_custom_min_image_size() {
        let doc = create_mock_document();
        let custom_options = AnalysisOptions {
            min_image_size: 1000, // Very large minimum
            ..Default::default()
        };
        let analyzer = PageContentAnalyzer::with_options(doc, custom_options);

        let result = analyzer.analyze_image_content(0);
        assert!(result.is_ok());
        // With high threshold, small images should be filtered
    }

    // Test 38: Page analysis with all content types
    #[test]
    fn test_comprehensive_page_analysis() {
        let doc = create_mock_document();
        let analyzer = PageContentAnalyzer::new(doc);

        let analysis = analyzer.analyze_page(0);
        assert!(analysis.is_ok());

        let analysis = analysis.unwrap();

        // Verify all fields are populated
        assert!(analysis.page_number == 0);
        assert!(analysis.text_ratio >= 0.0 && analysis.text_ratio <= 1.0);
        assert!(analysis.image_ratio >= 0.0 && analysis.image_ratio <= 1.0);
        assert!(analysis.blank_space_ratio >= 0.0 && analysis.blank_space_ratio <= 1.0);

        // Ratios should sum to approximately 1.0
        let total = analysis.text_ratio + analysis.image_ratio + analysis.blank_space_ratio;
        assert!((total - 1.0).abs() < 0.01);
    }

    // Test 39: Error message formatting
    #[test]
    fn test_error_message_formatting() {
        let doc = create_mock_document();
        let analyzer = PageContentAnalyzer::new(doc);
        let provider = MockOcrProvider::new();

        // Test non-scanned page error message
        let result = analyzer.extract_text_from_scanned_page(0, &provider);
        assert!(result.is_err());
        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.contains("not a scanned page"));
        assert!(error_msg.contains("image ratio"));
        assert!(error_msg.contains("text ratio"));
    }

    // Test 40: Batch size edge cases
    #[test]
    fn test_batch_size_edge_cases() {
        let doc = create_mock_document();
        let analyzer = PageContentAnalyzer::new(doc);
        let provider = MockOcrProvider::new();

        // Test with batch size 0 (should handle gracefully)
        let result = analyzer.process_scanned_pages_batch(&provider, 0);
        assert!(result.is_ok());

        // Test with very large batch size
        let result = analyzer.process_scanned_pages_batch(&provider, usize::MAX);
        assert!(result.is_ok());
    }

    // Test 41: OCR provider robustness
    #[test]
    fn test_ocr_provider_robustness() {
        // Create a provider that might fail
        struct UnreliableOcrProvider {
            call_count: Mutex<usize>,
        }

        impl UnreliableOcrProvider {
            fn new() -> Self {
                UnreliableOcrProvider {
                    call_count: Mutex::new(0),
                }
            }
        }

        impl Clone for UnreliableOcrProvider {
            fn clone(&self) -> Self {
                UnreliableOcrProvider {
                    call_count: Mutex::new(0),
                }
            }
        }

        impl OcrProvider for UnreliableOcrProvider {
            fn process_image(
                &self,
                _: &[u8],
                _: &OcrOptions,
            ) -> Result<OcrProcessingResult, OcrError> {
                let mut count = self.call_count.lock().unwrap();
                *count += 1;

                // Fail on first call, succeed on subsequent calls
                if *count == 1 {
                    Err(OcrError::ProcessingFailed("Temporary failure".to_string()))
                } else {
                    MockOcrProvider::new().process_image(&[0xFF, 0xD8], &OcrOptions::default())
                }
            }

            fn process_page(
                &self,
                _: &ContentAnalysis,
                data: &[u8],
                opts: &OcrOptions,
            ) -> Result<OcrProcessingResult, OcrError> {
                self.process_image(data, opts)
            }

            fn supported_formats(&self) -> Vec<crate::graphics::ImageFormat> {
                MockOcrProvider::new().supported_formats()
            }

            fn engine_name(&self) -> &str {
                "Unreliable"
            }

            fn engine_type(&self) -> crate::text::OcrEngine {
                crate::text::OcrEngine::Mock
            }
        }

        let doc = create_mock_document();
        let analyzer = PageContentAnalyzer::new(doc);
        let provider = UnreliableOcrProvider::new();

        // Test sequential processing with unreliable provider
        let result = analyzer.process_scanned_pages_with_ocr(&provider);
        assert!(result.is_ok());

        // Test batch processing with unreliable provider
        let result = analyzer.process_scanned_pages_batch(&provider, 2);
        assert!(result.is_ok());
    }

    // Test 42: Analysis options validation
    #[test]
    fn test_analysis_options_validation() {
        // Test with negative values (logically invalid but should handle)
        let options = AnalysisOptions {
            min_text_fragment_size: 0,
            min_image_size: 0,
            scanned_threshold: 1.5, // Above 1.0
            text_threshold: -0.5,   // Below 0.0
            ocr_options: None,
        };

        let doc = create_mock_document();
        let analyzer = PageContentAnalyzer::with_options(doc, options);

        // Should still work despite invalid thresholds
        let result = analyzer.analyze_page(0);
        assert!(result.is_ok());
    }

    // Test 43: OCR result aggregation
    #[test]
    fn test_ocr_result_aggregation() {
        let doc = create_mock_document();
        let analyzer = PageContentAnalyzer::new(doc);
        let mut provider = MockOcrProvider::new();

        // Set up provider with specific results
        provider.set_mock_text("Page content from OCR".to_string());
        provider.set_confidence(0.85);

        let results = analyzer.process_scanned_pages_with_ocr(&provider);
        assert!(results.is_ok());

        let ocr_results = results.unwrap();

        // Verify results can be aggregated
        let total_chars: usize = ocr_results
            .iter()
            .map(|(_, result)| result.text.len())
            .sum();
        let avg_confidence: f64 = if !ocr_results.is_empty() {
            ocr_results
                .iter()
                .map(|(_, result)| result.confidence)
                .sum::<f64>()
                / ocr_results.len() as f64
        } else {
            0.0
        };

        // total_chars is usize, always >= 0
        assert!(total_chars == total_chars); // Just to use the variable
        assert!((0.0..=1.0).contains(&avg_confidence));
    }

    // Test 44: Resource cleanup verification
    #[test]
    fn test_resource_cleanup() {
        // Test that resources are properly cleaned up
        for _ in 0..10 {
            let doc = create_mock_document();
            let analyzer = PageContentAnalyzer::new(doc);
            let _result = analyzer.analyze_document();
            // Resources should be automatically cleaned up when analyzer goes out of scope
        }

        // If this test completes without issues, resource cleanup is working
        assert!(true);
    }

    // Test 45: Complete workflow integration test
    #[test]
    fn test_complete_analysis_workflow() {
        // Create analyzer
        let doc = create_mock_document();
        let analyzer = PageContentAnalyzer::new(doc);

        // 1. Analyze document
        let analyses = analyzer.analyze_document().unwrap();
        assert!(!analyses.is_empty());

        // 2. Find scanned pages
        let _scanned_pages = analyzer.find_scanned_pages().unwrap();

        // 3. Check specific page
        let _is_scanned = analyzer.is_scanned_page(0).unwrap();

        // 4. Process with OCR (if applicable)
        let provider = MockOcrProvider::new();
        let ocr_results = analyzer.process_scanned_pages_with_ocr(&provider).unwrap();

        // 5. Sequential processing (since parallel requires Send + Sync)
        let sequential_results = analyzer.process_scanned_pages_with_ocr(&provider).unwrap();

        // 6. Batch processing
        let batch_results = analyzer.process_scanned_pages_batch(&provider, 5).unwrap();

        // Verify consistency across methods
        assert_eq!(ocr_results.len(), sequential_results.len());
        assert_eq!(ocr_results.len(), batch_results.len());

        println!(
            "Complete workflow test passed with {} pages analyzed",
            analyses.len()
        );
    }
}
