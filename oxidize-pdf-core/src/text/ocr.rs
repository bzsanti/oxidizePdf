//! OCR (Optical Character Recognition) support for PDF processing
//!
//! This module provides a flexible, pluggable architecture for integrating OCR capabilities
//! into PDF processing workflows. It's designed to work seamlessly with the page analysis
//! module to process scanned pages and extract text from images.
//!
//! # Architecture
//!
//! The OCR system uses a trait-based approach that allows for multiple OCR providers:
//!
//! - **OcrProvider trait**: Generic interface for OCR engines
//! - **Pluggable implementations**: Support for local (Tesseract) and cloud (Azure, AWS) providers
//! - **Result standardization**: Consistent output format regardless of provider
//! - **Error handling**: Comprehensive error types for OCR operations
//!
//! # Usage
//!
//! ## Basic OCR Processing
//!
//! ```rust
//! use oxidize_pdf::text::{MockOcrProvider, OcrOptions, OcrProvider};
//! use oxidize_pdf::graphics::ImageFormat;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let provider = MockOcrProvider::new();
//! let options = OcrOptions::default();
//!
//! // Process image data directly - Mock JPEG data
//! let image_data = vec![
//!     0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10, 0x4A, 0x46, 0x49, 0x46, 0x00, 0x01,
//!     0x01, 0x01, 0x00, 0x48, 0x00, 0x48, 0x00, 0x00, 0xFF, 0xD9
//! ];
//! let result = provider.process_image(&image_data, &options)?;
//!
//! println!("Extracted text: {}", result.text);
//! println!("Confidence: {:.2}%", result.confidence * 100.0);
//!
//! for fragment in result.fragments {
//!     println!("Fragment: '{}' at ({}, {})", fragment.text, fragment.x, fragment.y);
//! }
//! # Ok(())
//! # }
//! ```
//!
//! ## Integration with Page Analysis
//!
//! ```rust,no_run
//! use oxidize_pdf::operations::page_analysis::PageContentAnalyzer;
//! use oxidize_pdf::text::{MockOcrProvider, OcrOptions};
//! use oxidize_pdf::parser::PdfReader;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let document = PdfReader::open_document("scanned.pdf")?;
//! let analyzer = PageContentAnalyzer::new(document);
//! let provider = MockOcrProvider::new();
//!
//! // Find scanned pages
//! let scanned_pages = analyzer.find_scanned_pages()?;
//!
//! for page_num in scanned_pages {
//!     let analysis = analyzer.analyze_page(page_num)?;
//!     if analysis.is_scanned() {
//!         println!("Processing scanned page {}", page_num);
//!         // OCR processing would happen here
//!     }
//! }
//! # Ok(())
//! # }
//! ```

use crate::graphics::ImageFormat;
use crate::operations::page_analysis::ContentAnalysis;
use std::fmt;

/// Result type for OCR operations
pub type OcrResult<T> = Result<T, OcrError>;

/// Errors that can occur during OCR processing
#[derive(Debug, thiserror::Error)]
pub enum OcrError {
    /// OCR provider is not available or not configured
    #[error("OCR provider not available: {0}")]
    ProviderNotAvailable(String),

    /// Unsupported image format for OCR processing
    #[error("Unsupported image format: {0:?}")]
    UnsupportedImageFormat(ImageFormat),

    /// Invalid or corrupted image data
    #[error("Invalid image data: {0}")]
    InvalidImageData(String),

    /// OCR processing failed
    #[error("OCR processing failed: {0}")]
    ProcessingFailed(String),

    /// Network error when using cloud OCR providers
    #[error("Network error: {0}")]
    NetworkError(String),

    /// API key or authentication error
    #[error("Authentication error: {0}")]
    AuthenticationError(String),

    /// Rate limiting or quota exceeded
    #[error("Rate limit exceeded: {0}")]
    RateLimitExceeded(String),

    /// OCR provider returned low confidence results
    #[error("Low confidence results: {0}")]
    LowConfidence(String),

    /// Generic IO error
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Configuration error
    #[error("Configuration error: {0}")]
    Configuration(String),
}

/// OCR processing options and configuration
#[derive(Debug, Clone)]
pub struct OcrOptions {
    /// Target language for OCR (ISO 639-1 code, e.g., "en", "es", "fr")
    pub language: String,

    /// Minimum confidence threshold (0.0 to 1.0)
    pub min_confidence: f64,

    /// Whether to preserve text layout and positioning
    pub preserve_layout: bool,

    /// Image preprocessing options
    pub preprocessing: ImagePreprocessing,

    /// OCR engine specific options
    pub engine_options: std::collections::HashMap<String, String>,

    /// Timeout for OCR operations (in seconds)
    pub timeout_seconds: u32,
}

impl Default for OcrOptions {
    fn default() -> Self {
        Self {
            language: "en".to_string(),
            min_confidence: 0.6,
            preserve_layout: true,
            preprocessing: ImagePreprocessing::default(),
            engine_options: std::collections::HashMap::new(),
            timeout_seconds: 30,
        }
    }
}

/// Image preprocessing options for OCR
#[derive(Debug, Clone)]
pub struct ImagePreprocessing {
    /// Whether to apply image denoising
    pub denoise: bool,

    /// Whether to apply image deskewing
    pub deskew: bool,

    /// Whether to enhance contrast
    pub enhance_contrast: bool,

    /// Whether to apply image sharpening
    pub sharpen: bool,

    /// Scale factor for image resizing (1.0 = no scaling)
    pub scale_factor: f64,
}

impl Default for ImagePreprocessing {
    fn default() -> Self {
        Self {
            denoise: true,
            deskew: true,
            enhance_contrast: true,
            sharpen: false,
            scale_factor: 1.0,
        }
    }
}

/// Text fragment extracted by OCR with position and confidence information
#[derive(Debug, Clone)]
pub struct OcrTextFragment {
    /// The extracted text content
    pub text: String,

    /// X position in page coordinates (points)
    pub x: f64,

    /// Y position in page coordinates (points)
    pub y: f64,

    /// Width of the text fragment (points)
    pub width: f64,

    /// Height of the text fragment (points)
    pub height: f64,

    /// Confidence score for this fragment (0.0 to 1.0)
    pub confidence: f64,

    /// Font size estimation (points)
    pub font_size: f64,

    /// Whether this fragment is part of a word or line
    pub fragment_type: FragmentType,
}

/// Type of text fragment
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FragmentType {
    /// Individual character
    Character,
    /// Complete word
    Word,
    /// Text line
    Line,
    /// Paragraph
    Paragraph,
}

/// Complete result of OCR processing
#[derive(Debug, Clone)]
pub struct OcrProcessingResult {
    /// The complete extracted text
    pub text: String,

    /// Overall confidence score (0.0 to 1.0)
    pub confidence: f64,

    /// Individual text fragments with position information
    pub fragments: Vec<OcrTextFragment>,

    /// Processing time in milliseconds
    pub processing_time_ms: u64,

    /// OCR engine used for processing
    pub engine_name: String,

    /// Language detected/used
    pub language: String,

    /// Image dimensions that were processed
    pub image_dimensions: (u32, u32),
}

impl OcrProcessingResult {
    /// Filter fragments by minimum confidence
    pub fn filter_by_confidence(&self, min_confidence: f64) -> Vec<&OcrTextFragment> {
        self.fragments
            .iter()
            .filter(|fragment| fragment.confidence >= min_confidence)
            .collect()
    }

    /// Get text fragments within a specific region
    pub fn fragments_in_region(
        &self,
        x: f64,
        y: f64,
        width: f64,
        height: f64,
    ) -> Vec<&OcrTextFragment> {
        self.fragments
            .iter()
            .filter(|fragment| {
                fragment.x >= x
                    && fragment.y >= y
                    && fragment.x + fragment.width <= x + width
                    && fragment.y + fragment.height <= y + height
            })
            .collect()
    }

    /// Get fragments of a specific type
    pub fn fragments_of_type(&self, fragment_type: FragmentType) -> Vec<&OcrTextFragment> {
        self.fragments
            .iter()
            .filter(|fragment| fragment.fragment_type == fragment_type)
            .collect()
    }

    /// Calculate average confidence for all fragments
    pub fn average_confidence(&self) -> f64 {
        if self.fragments.is_empty() {
            return 0.0;
        }

        let sum: f64 = self.fragments.iter().map(|f| f.confidence).sum();
        sum / self.fragments.len() as f64
    }
}

/// Supported OCR engines
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OcrEngine {
    /// Mock OCR provider for testing
    Mock,
    /// Tesseract OCR (local processing)
    Tesseract,
    /// Azure Computer Vision OCR
    Azure,
    /// AWS Textract
    Aws,
    /// Google Cloud Vision OCR
    GoogleCloud,
}

impl OcrEngine {
    /// Get the name of the OCR engine
    pub fn name(&self) -> &'static str {
        match self {
            OcrEngine::Mock => "Mock OCR",
            OcrEngine::Tesseract => "Tesseract",
            OcrEngine::Azure => "Azure Computer Vision",
            OcrEngine::Aws => "AWS Textract",
            OcrEngine::GoogleCloud => "Google Cloud Vision",
        }
    }

    /// Check if this engine supports the given image format
    pub fn supports_format(&self, format: ImageFormat) -> bool {
        match self {
            OcrEngine::Mock => true, // Mock supports all formats
            OcrEngine::Tesseract => matches!(
                format,
                ImageFormat::Jpeg | ImageFormat::Png | ImageFormat::Tiff
            ),
            OcrEngine::Azure => matches!(format, ImageFormat::Jpeg | ImageFormat::Png),
            OcrEngine::Aws => matches!(format, ImageFormat::Jpeg | ImageFormat::Png),
            OcrEngine::GoogleCloud => matches!(format, ImageFormat::Jpeg | ImageFormat::Png),
        }
    }
}

impl fmt::Display for OcrEngine {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name())
    }
}

/// Trait for OCR providers
///
/// This trait defines the interface that all OCR providers must implement.
/// It provides methods for processing images and extracting text with position information.
///
/// # Implementation Notes
///
/// - Implementations should handle errors gracefully and return meaningful error messages
/// - The `process_image` method is the core functionality that all providers must implement
/// - The `process_page` method is a convenience method for working with page analysis results
/// - Providers should validate image formats and reject unsupported formats
///
/// # Examples
///
/// ```rust
/// use oxidize_pdf::text::{OcrProvider, OcrOptions, OcrProcessingResult, OcrError, OcrEngine};
/// use oxidize_pdf::graphics::ImageFormat;
///
/// struct MyOcrProvider;
///
/// impl OcrProvider for MyOcrProvider {
///     fn process_image(&self, image_data: &[u8], options: &OcrOptions) -> Result<OcrProcessingResult, OcrError> {
///         // Implementation here
///         # Ok(OcrProcessingResult {
///         #     text: "Sample text".to_string(),
///         #     confidence: 0.95,
///         #     fragments: vec![],
///         #     processing_time_ms: 100,
///         #     engine_name: "MyOCR".to_string(),
///         #     language: "en".to_string(),
///         #     image_dimensions: (800, 600),
///         # })
///     }
///
///     fn supported_formats(&self) -> Vec<ImageFormat> {
///         vec![ImageFormat::Jpeg, ImageFormat::Png]
///     }
///
///     fn engine_name(&self) -> &str {
///         "MyOCR"
///     }
///
///     fn engine_type(&self) -> OcrEngine {
///         OcrEngine::Mock
///     }
/// }
/// ```
pub trait OcrProvider: Send + Sync {
    /// Process an image and extract text using OCR
    ///
    /// This is the core method that all OCR providers must implement.
    /// It takes image data as bytes and returns structured text results.
    ///
    /// # Arguments
    ///
    /// * `image_data` - Raw image bytes (JPEG, PNG, or TIFF)
    /// * `options` - OCR processing options and configuration
    ///
    /// # Returns
    ///
    /// A `Result` containing the OCR results with text, confidence, and positioning information.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The image format is not supported
    /// - The image data is corrupted or invalid
    /// - OCR processing fails
    /// - Network errors occur (for cloud providers)
    /// - Authentication fails (for cloud providers)
    fn process_image(
        &self,
        image_data: &[u8],
        options: &OcrOptions,
    ) -> OcrResult<OcrProcessingResult>;

    /// Process a scanned page using content analysis information
    ///
    /// This method provides a higher-level interface that works with page analysis results.
    /// It's particularly useful when integrating with the page analysis module.
    ///
    /// # Arguments
    ///
    /// * `page_analysis` - Results from page content analysis
    /// * `page_data` - Raw page data or image data
    /// * `options` - OCR processing options
    ///
    /// # Returns
    ///
    /// OCR results optimized for the specific page content type.
    ///
    /// # Default Implementation
    ///
    /// The default implementation simply calls `process_image` with the page data.
    /// Providers can override this to provide specialized handling based on page analysis.
    fn process_page(
        &self,
        _page_analysis: &ContentAnalysis,
        page_data: &[u8],
        options: &OcrOptions,
    ) -> OcrResult<OcrProcessingResult> {
        self.process_image(page_data, options)
    }

    /// Get the list of supported image formats
    ///
    /// # Returns
    ///
    /// A vector of `ImageFormat` values that this provider can process.
    fn supported_formats(&self) -> Vec<ImageFormat>;

    /// Get the name of this OCR provider
    ///
    /// # Returns
    ///
    /// A string identifying this provider (e.g., "Tesseract", "Azure OCR").
    fn engine_name(&self) -> &str;

    /// Get the engine type for this provider
    ///
    /// # Returns
    ///
    /// The `OcrEngine` enum value corresponding to this provider.
    fn engine_type(&self) -> OcrEngine;

    /// Check if this provider supports the given image format
    ///
    /// # Arguments
    ///
    /// * `format` - The image format to check
    ///
    /// # Returns
    ///
    /// `true` if the format is supported, `false` otherwise.
    fn supports_format(&self, format: ImageFormat) -> bool {
        self.supported_formats().contains(&format)
    }

    /// Validate image data before processing
    ///
    /// This method can be used to perform basic validation of image data
    /// before attempting OCR processing.
    ///
    /// # Arguments
    ///
    /// * `image_data` - Raw image bytes to validate
    ///
    /// # Returns
    ///
    /// `Ok(())` if the image data is valid, `Err(OcrError)` otherwise.
    ///
    /// # Default Implementation
    ///
    /// The default implementation performs basic format detection based on magic bytes.
    fn validate_image_data(&self, image_data: &[u8]) -> OcrResult<()> {
        if image_data.len() < 8 {
            return Err(OcrError::InvalidImageData(
                "Image data too short".to_string(),
            ));
        }

        // Check for common image format signatures
        let format = if image_data.starts_with(b"\xFF\xD8\xFF") {
            ImageFormat::Jpeg
        } else if image_data.starts_with(b"\x89PNG\r\n\x1a\n") {
            ImageFormat::Png
        } else if image_data.starts_with(b"II\x2A\x00") || image_data.starts_with(b"MM\x00\x2A") {
            ImageFormat::Tiff
        } else {
            return Err(OcrError::InvalidImageData(
                "Unrecognized image format".to_string(),
            ));
        };

        if !self.supports_format(format) {
            return Err(OcrError::UnsupportedImageFormat(format));
        }

        Ok(())
    }
}

/// Mock OCR provider for testing and development
///
/// This provider simulates OCR processing without actually performing text recognition.
/// It's useful for testing OCR workflows and developing OCR-dependent functionality.
///
/// # Examples
///
/// ```rust
/// use oxidize_pdf::text::{MockOcrProvider, OcrOptions, OcrProvider};
///
/// let provider = MockOcrProvider::new();
/// let options = OcrOptions::default();
/// let image_data = vec![0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10, 0x4A, 0x46, 0x49, 0x46]; // Mock JPEG data
///
/// let result = provider.process_image(&image_data, &options).unwrap();
/// assert!(result.text.contains("Mock OCR"));
/// ```
#[derive(Clone)]
pub struct MockOcrProvider {
    /// Mock confidence level to return
    confidence: f64,
    /// Mock text to return
    mock_text: String,
    /// Simulated processing delay (milliseconds)
    processing_delay_ms: u64,
}

impl MockOcrProvider {
    /// Create a new mock OCR provider with default settings
    pub fn new() -> Self {
        Self {
            confidence: 0.85,
            mock_text: "Mock OCR extracted text from scanned image".to_string(),
            processing_delay_ms: 100,
        }
    }

    /// Create a mock provider with custom text and confidence
    pub fn with_text_and_confidence(text: String, confidence: f64) -> Self {
        Self {
            confidence,
            mock_text: text,
            processing_delay_ms: 100,
        }
    }

    /// Set the mock text to return
    pub fn set_mock_text(&mut self, text: String) {
        self.mock_text = text;
    }

    /// Set the confidence level to return
    pub fn set_confidence(&mut self, confidence: f64) {
        self.confidence = confidence.clamp(0.0, 1.0);
    }

    /// Set the simulated processing delay
    pub fn set_processing_delay(&mut self, delay_ms: u64) {
        self.processing_delay_ms = delay_ms;
    }
}

impl Default for MockOcrProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl OcrProvider for MockOcrProvider {
    fn process_image(
        &self,
        image_data: &[u8],
        options: &OcrOptions,
    ) -> OcrResult<OcrProcessingResult> {
        // Validate image data
        self.validate_image_data(image_data)?;

        // Simulate processing time
        std::thread::sleep(std::time::Duration::from_millis(self.processing_delay_ms));

        // Create mock text fragments
        let fragments = vec![
            OcrTextFragment {
                text: self.mock_text.clone(),
                x: 50.0,
                y: 700.0,
                width: 200.0,
                height: 20.0,
                confidence: self.confidence,
                font_size: 12.0,
                fragment_type: FragmentType::Line,
            },
            OcrTextFragment {
                text: "Additional mock text".to_string(),
                x: 50.0,
                y: 680.0,
                width: 150.0,
                height: 20.0,
                confidence: self.confidence * 0.9,
                font_size: 12.0,
                fragment_type: FragmentType::Line,
            },
        ];

        Ok(OcrProcessingResult {
            text: format!("{}\nAdditional mock text", self.mock_text),
            confidence: self.confidence,
            fragments,
            processing_time_ms: self.processing_delay_ms,
            engine_name: "Mock OCR".to_string(),
            language: options.language.clone(),
            image_dimensions: (800, 600), // Mock dimensions
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ocr_options_default() {
        let options = OcrOptions::default();
        assert_eq!(options.language, "en");
        assert_eq!(options.min_confidence, 0.6);
        assert!(options.preserve_layout);
        assert_eq!(options.timeout_seconds, 30);
    }

    #[test]
    fn test_image_preprocessing_default() {
        let preprocessing = ImagePreprocessing::default();
        assert!(preprocessing.denoise);
        assert!(preprocessing.deskew);
        assert!(preprocessing.enhance_contrast);
        assert!(!preprocessing.sharpen);
        assert_eq!(preprocessing.scale_factor, 1.0);
    }

    #[test]
    fn test_ocr_engine_name() {
        assert_eq!(OcrEngine::Mock.name(), "Mock OCR");
        assert_eq!(OcrEngine::Tesseract.name(), "Tesseract");
        assert_eq!(OcrEngine::Azure.name(), "Azure Computer Vision");
    }

    #[test]
    fn test_ocr_engine_supports_format() {
        assert!(OcrEngine::Mock.supports_format(ImageFormat::Jpeg));
        assert!(OcrEngine::Mock.supports_format(ImageFormat::Png));
        assert!(OcrEngine::Mock.supports_format(ImageFormat::Tiff));

        assert!(OcrEngine::Tesseract.supports_format(ImageFormat::Jpeg));
        assert!(OcrEngine::Tesseract.supports_format(ImageFormat::Png));
        assert!(OcrEngine::Tesseract.supports_format(ImageFormat::Tiff));

        assert!(OcrEngine::Azure.supports_format(ImageFormat::Jpeg));
        assert!(OcrEngine::Azure.supports_format(ImageFormat::Png));
        assert!(!OcrEngine::Azure.supports_format(ImageFormat::Tiff));
    }

    #[test]
    fn test_fragment_type_equality() {
        assert_eq!(FragmentType::Word, FragmentType::Word);
        assert_ne!(FragmentType::Word, FragmentType::Line);
        assert_ne!(FragmentType::Character, FragmentType::Paragraph);
    }

    #[test]
    fn test_mock_ocr_provider_creation() {
        let provider = MockOcrProvider::new();
        assert_eq!(provider.confidence, 0.85);
        assert!(provider.mock_text.contains("Mock OCR"));
        assert_eq!(provider.processing_delay_ms, 100);
    }

    #[test]
    fn test_mock_ocr_provider_with_custom_text() {
        let custom_text = "Custom mock text".to_string();
        let provider = MockOcrProvider::with_text_and_confidence(custom_text.clone(), 0.95);
        assert_eq!(provider.mock_text, custom_text);
        assert_eq!(provider.confidence, 0.95);
    }

    #[test]
    fn test_mock_ocr_provider_process_image() {
        let provider = MockOcrProvider::new();
        let options = OcrOptions::default();

        // Mock JPEG data
        let jpeg_data = vec![0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10, 0x4A, 0x46, 0x49, 0x46];

        let result = provider.process_image(&jpeg_data, &options).unwrap();
        assert!(result.text.contains("Mock OCR"));
        assert_eq!(result.confidence, 0.85);
        assert!(!result.fragments.is_empty());
        assert_eq!(result.engine_name, "Mock OCR");
        assert_eq!(result.language, "en");
    }

    #[test]
    fn test_mock_ocr_provider_supported_formats() {
        let provider = MockOcrProvider::new();
        let formats = provider.supported_formats();
        assert!(formats.contains(&ImageFormat::Jpeg));
        assert!(formats.contains(&ImageFormat::Png));
        assert!(formats.contains(&ImageFormat::Tiff));
    }

    #[test]
    fn test_mock_ocr_provider_engine_info() {
        let provider = MockOcrProvider::new();
        assert_eq!(provider.engine_name(), "Mock OCR");
        assert_eq!(provider.engine_type(), OcrEngine::Mock);
    }

    #[test]
    fn test_mock_ocr_provider_supports_format() {
        let provider = MockOcrProvider::new();
        assert!(provider.supports_format(ImageFormat::Jpeg));
        assert!(provider.supports_format(ImageFormat::Png));
        assert!(provider.supports_format(ImageFormat::Tiff));
    }

    #[test]
    fn test_mock_ocr_provider_validate_image_data() {
        let provider = MockOcrProvider::new();

        // Valid JPEG data
        let jpeg_data = vec![0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10, 0x4A, 0x46, 0x49, 0x46];
        assert!(provider.validate_image_data(&jpeg_data).is_ok());

        // Invalid data (too short)
        let short_data = vec![0xFF, 0xD8];
        assert!(provider.validate_image_data(&short_data).is_err());

        // Invalid format
        let invalid_data = vec![0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09];
        assert!(provider.validate_image_data(&invalid_data).is_err());
    }

    #[test]
    fn test_ocr_processing_result_filter_by_confidence() {
        let result = OcrProcessingResult {
            text: "Test text".to_string(),
            confidence: 0.8,
            fragments: vec![
                OcrTextFragment {
                    text: "High confidence".to_string(),
                    x: 0.0,
                    y: 0.0,
                    width: 100.0,
                    height: 20.0,
                    confidence: 0.9,
                    font_size: 12.0,
                    fragment_type: FragmentType::Word,
                },
                OcrTextFragment {
                    text: "Low confidence".to_string(),
                    x: 0.0,
                    y: 20.0,
                    width: 100.0,
                    height: 20.0,
                    confidence: 0.5,
                    font_size: 12.0,
                    fragment_type: FragmentType::Word,
                },
            ],
            processing_time_ms: 100,
            engine_name: "Test".to_string(),
            language: "en".to_string(),
            image_dimensions: (800, 600),
        };

        let high_confidence = result.filter_by_confidence(0.8);
        assert_eq!(high_confidence.len(), 1);
        assert_eq!(high_confidence[0].text, "High confidence");
    }

    #[test]
    fn test_ocr_processing_result_fragments_in_region() {
        let result = OcrProcessingResult {
            text: "Test text".to_string(),
            confidence: 0.8,
            fragments: vec![
                OcrTextFragment {
                    text: "Inside region".to_string(),
                    x: 10.0,
                    y: 10.0,
                    width: 80.0,
                    height: 20.0,
                    confidence: 0.9,
                    font_size: 12.0,
                    fragment_type: FragmentType::Word,
                },
                OcrTextFragment {
                    text: "Outside region".to_string(),
                    x: 200.0,
                    y: 200.0,
                    width: 80.0,
                    height: 20.0,
                    confidence: 0.9,
                    font_size: 12.0,
                    fragment_type: FragmentType::Word,
                },
            ],
            processing_time_ms: 100,
            engine_name: "Test".to_string(),
            language: "en".to_string(),
            image_dimensions: (800, 600),
        };

        let in_region = result.fragments_in_region(0.0, 0.0, 100.0, 100.0);
        assert_eq!(in_region.len(), 1);
        assert_eq!(in_region[0].text, "Inside region");
    }

    #[test]
    fn test_ocr_processing_result_fragments_of_type() {
        let result = OcrProcessingResult {
            text: "Test text".to_string(),
            confidence: 0.8,
            fragments: vec![
                OcrTextFragment {
                    text: "Word fragment".to_string(),
                    x: 0.0,
                    y: 0.0,
                    width: 100.0,
                    height: 20.0,
                    confidence: 0.9,
                    font_size: 12.0,
                    fragment_type: FragmentType::Word,
                },
                OcrTextFragment {
                    text: "Line fragment".to_string(),
                    x: 0.0,
                    y: 20.0,
                    width: 200.0,
                    height: 20.0,
                    confidence: 0.9,
                    font_size: 12.0,
                    fragment_type: FragmentType::Line,
                },
            ],
            processing_time_ms: 100,
            engine_name: "Test".to_string(),
            language: "en".to_string(),
            image_dimensions: (800, 600),
        };

        let words = result.fragments_of_type(FragmentType::Word);
        assert_eq!(words.len(), 1);
        assert_eq!(words[0].text, "Word fragment");

        let lines = result.fragments_of_type(FragmentType::Line);
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].text, "Line fragment");
    }

    #[test]
    fn test_ocr_processing_result_average_confidence() {
        let result = OcrProcessingResult {
            text: "Test text".to_string(),
            confidence: 0.8,
            fragments: vec![
                OcrTextFragment {
                    text: "Fragment 1".to_string(),
                    x: 0.0,
                    y: 0.0,
                    width: 100.0,
                    height: 20.0,
                    confidence: 0.8,
                    font_size: 12.0,
                    fragment_type: FragmentType::Word,
                },
                OcrTextFragment {
                    text: "Fragment 2".to_string(),
                    x: 0.0,
                    y: 20.0,
                    width: 100.0,
                    height: 20.0,
                    confidence: 0.6,
                    font_size: 12.0,
                    fragment_type: FragmentType::Word,
                },
            ],
            processing_time_ms: 100,
            engine_name: "Test".to_string(),
            language: "en".to_string(),
            image_dimensions: (800, 600),
        };

        let avg_confidence = result.average_confidence();
        assert_eq!(avg_confidence, 0.7);
    }

    #[test]
    fn test_ocr_processing_result_average_confidence_empty() {
        let result = OcrProcessingResult {
            text: "Test text".to_string(),
            confidence: 0.8,
            fragments: vec![],
            processing_time_ms: 100,
            engine_name: "Test".to_string(),
            language: "en".to_string(),
            image_dimensions: (800, 600),
        };

        let avg_confidence = result.average_confidence();
        assert_eq!(avg_confidence, 0.0);
    }

    // Comprehensive tests for OCR module
    mod comprehensive_tests {
        use super::*;
        use std::collections::HashMap;

        // Error Type Tests
        #[test]
        fn test_ocr_error_display() {
            let errors = vec![
                OcrError::ProviderNotAvailable("Tesseract not installed".to_string()),
                OcrError::UnsupportedImageFormat(ImageFormat::Tiff),
                OcrError::InvalidImageData("Corrupted header".to_string()),
                OcrError::ProcessingFailed("OCR engine crashed".to_string()),
                OcrError::NetworkError("Connection timeout".to_string()),
                OcrError::AuthenticationError("Invalid API key".to_string()),
                OcrError::RateLimitExceeded("429 Too Many Requests".to_string()),
                OcrError::LowConfidence("Confidence below threshold".to_string()),
                OcrError::Configuration("Missing language pack".to_string()),
            ];

            for error in errors {
                let display = format!("{error}");
                assert!(!display.is_empty());

                // Verify error messages contain expected content
                match &error {
                    OcrError::ProviderNotAvailable(msg) => assert!(display.contains(msg)),
                    OcrError::UnsupportedImageFormat(_) => {
                        assert!(display.contains("Unsupported image format"))
                    }
                    OcrError::InvalidImageData(msg) => assert!(display.contains(msg)),
                    OcrError::ProcessingFailed(msg) => assert!(display.contains(msg)),
                    OcrError::NetworkError(msg) => assert!(display.contains(msg)),
                    OcrError::AuthenticationError(msg) => assert!(display.contains(msg)),
                    OcrError::RateLimitExceeded(msg) => assert!(display.contains(msg)),
                    OcrError::LowConfidence(msg) => assert!(display.contains(msg)),
                    OcrError::Configuration(msg) => assert!(display.contains(msg)),
                    _ => {}
                }
            }
        }

        #[test]
        fn test_ocr_error_from_io_error() {
            use std::io::{Error as IoError, ErrorKind};

            let io_error = IoError::new(ErrorKind::NotFound, "File not found");
            let ocr_error: OcrError = io_error.into();

            match ocr_error {
                OcrError::Io(_) => {
                    let display = format!("{ocr_error}");
                    assert!(display.contains("IO error"));
                }
                _ => panic!("Expected OcrError::Io variant"),
            }
        }

        #[test]
        fn test_ocr_error_debug_format() {
            let error = OcrError::ProcessingFailed("Test error".to_string());
            let debug_str = format!("{error:?}");
            assert!(debug_str.contains("ProcessingFailed"));
            assert!(debug_str.contains("Test error"));
        }

        // OcrOptions Tests
        #[test]
        fn test_ocr_options_custom_language() {
            let mut options = OcrOptions::default();
            assert_eq!(options.language, "en");

            options.language = "spa+eng".to_string();
            assert_eq!(options.language, "spa+eng");

            options.language = "jpn".to_string();
            assert_eq!(options.language, "jpn");
        }

        #[test]
        fn test_ocr_options_confidence_threshold() {
            let mut options = OcrOptions::default();
            assert_eq!(options.min_confidence, 0.6);

            // Test various thresholds
            options.min_confidence = 0.0;
            assert_eq!(options.min_confidence, 0.0);

            options.min_confidence = 1.0;
            assert_eq!(options.min_confidence, 1.0);

            options.min_confidence = 0.85;
            assert_eq!(options.min_confidence, 0.85);
        }

        #[test]
        fn test_ocr_options_engine_specific() {
            let mut options = OcrOptions::default();
            assert!(options.engine_options.is_empty());

            // Add engine-specific options
            options.engine_options.insert(
                "tessedit_char_whitelist".to_string(),
                "0123456789".to_string(),
            );
            options
                .engine_options
                .insert("tessedit_ocr_engine_mode".to_string(), "3".to_string());

            assert_eq!(options.engine_options.len(), 2);
            assert_eq!(
                options.engine_options.get("tessedit_char_whitelist"),
                Some(&"0123456789".to_string())
            );
        }

        #[test]
        fn test_ocr_options_clone() {
            let mut options = OcrOptions {
                language: "fra".to_string(),
                min_confidence: 0.75,
                preserve_layout: false,
                preprocessing: ImagePreprocessing {
                    denoise: false,
                    deskew: true,
                    enhance_contrast: false,
                    sharpen: true,
                    scale_factor: 1.5,
                },
                engine_options: HashMap::new(),
                timeout_seconds: 60,
            };

            options
                .engine_options
                .insert("key".to_string(), "value".to_string());

            let cloned = options.clone();
            assert_eq!(cloned.language, options.language);
            assert_eq!(cloned.min_confidence, options.min_confidence);
            assert_eq!(cloned.preserve_layout, options.preserve_layout);
            assert_eq!(
                cloned.preprocessing.scale_factor,
                options.preprocessing.scale_factor
            );
            assert_eq!(cloned.engine_options.get("key"), Some(&"value".to_string()));
            assert_eq!(cloned.timeout_seconds, options.timeout_seconds);
        }

        #[test]
        fn test_ocr_options_timeout_configuration() {
            let mut options = OcrOptions::default();
            assert_eq!(options.timeout_seconds, 30);

            options.timeout_seconds = 0; // No timeout
            assert_eq!(options.timeout_seconds, 0);

            options.timeout_seconds = 300; // 5 minutes
            assert_eq!(options.timeout_seconds, 300);
        }

        // ImagePreprocessing Tests
        #[test]
        fn test_image_preprocessing_combinations() {
            let test_cases = vec![
                (true, true, true, true),
                (false, false, false, false),
                (true, false, true, false),
                (false, true, false, true),
            ];

            for (denoise, deskew, enhance, sharpen) in test_cases {
                let preprocessing = ImagePreprocessing {
                    denoise,
                    deskew,
                    enhance_contrast: enhance,
                    sharpen,
                    scale_factor: 1.0,
                };

                assert_eq!(preprocessing.denoise, denoise);
                assert_eq!(preprocessing.deskew, deskew);
                assert_eq!(preprocessing.enhance_contrast, enhance);
                assert_eq!(preprocessing.sharpen, sharpen);
            }
        }

        #[test]
        fn test_image_preprocessing_scale_factor() {
            let mut preprocessing = ImagePreprocessing::default();
            assert_eq!(preprocessing.scale_factor, 1.0);

            // Test various scale factors
            preprocessing.scale_factor = 0.5;
            assert_eq!(preprocessing.scale_factor, 0.5);

            preprocessing.scale_factor = 2.0;
            assert_eq!(preprocessing.scale_factor, 2.0);

            preprocessing.scale_factor = 1.25;
            assert_eq!(preprocessing.scale_factor, 1.25);
        }

        #[test]
        fn test_image_preprocessing_clone() {
            let preprocessing = ImagePreprocessing {
                denoise: false,
                deskew: true,
                enhance_contrast: false,
                sharpen: true,
                scale_factor: 1.5,
            };

            let cloned = preprocessing.clone();
            assert_eq!(cloned.denoise, preprocessing.denoise);
            assert_eq!(cloned.deskew, preprocessing.deskew);
            assert_eq!(cloned.enhance_contrast, preprocessing.enhance_contrast);
            assert_eq!(cloned.sharpen, preprocessing.sharpen);
            assert_eq!(cloned.scale_factor, preprocessing.scale_factor);
        }

        // OcrTextFragment Tests
        #[test]
        fn test_ocr_text_fragment_creation() {
            let fragment = OcrTextFragment {
                text: "Hello World".to_string(),
                x: 100.0,
                y: 200.0,
                width: 150.0,
                height: 25.0,
                confidence: 0.92,
                font_size: 14.0,
                fragment_type: FragmentType::Line,
            };

            assert_eq!(fragment.text, "Hello World");
            assert_eq!(fragment.x, 100.0);
            assert_eq!(fragment.y, 200.0);
            assert_eq!(fragment.width, 150.0);
            assert_eq!(fragment.height, 25.0);
            assert_eq!(fragment.confidence, 0.92);
            assert_eq!(fragment.font_size, 14.0);
            assert_eq!(fragment.fragment_type, FragmentType::Line);
        }

        #[test]
        fn test_ocr_text_fragment_clone() {
            let fragment = OcrTextFragment {
                text: "Test".to_string(),
                x: 50.0,
                y: 100.0,
                width: 40.0,
                height: 15.0,
                confidence: 0.88,
                font_size: 11.0,
                fragment_type: FragmentType::Word,
            };

            let cloned = fragment.clone();
            assert_eq!(cloned.text, fragment.text);
            assert_eq!(cloned.x, fragment.x);
            assert_eq!(cloned.confidence, fragment.confidence);
            assert_eq!(cloned.fragment_type, fragment.fragment_type);
        }

        #[test]
        fn test_fragment_type_copy() {
            let ft1 = FragmentType::Character;
            let ft2 = ft1; // Copy
            assert_eq!(ft1, ft2);
            assert_eq!(ft1, FragmentType::Character);
        }

        #[test]
        fn test_fragment_position_calculations() {
            let fragment = OcrTextFragment {
                text: "Test".to_string(),
                x: 100.0,
                y: 200.0,
                width: 50.0,
                height: 20.0,
                confidence: 0.9,
                font_size: 12.0,
                fragment_type: FragmentType::Word,
            };

            // Calculate bounding box
            let right = fragment.x + fragment.width;
            let bottom = fragment.y + fragment.height;

            assert_eq!(right, 150.0);
            assert_eq!(bottom, 220.0);
        }

        // OcrProcessingResult Advanced Tests
        #[test]
        fn test_ocr_result_complex_region_filtering() {
            let fragments = vec![
                OcrTextFragment {
                    text: "A".to_string(),
                    x: 10.0,
                    y: 10.0,
                    width: 20.0,
                    height: 20.0,
                    confidence: 0.9,
                    font_size: 12.0,
                    fragment_type: FragmentType::Character,
                },
                OcrTextFragment {
                    text: "B".to_string(),
                    x: 25.0,
                    y: 10.0,
                    width: 20.0,
                    height: 20.0,
                    confidence: 0.9,
                    font_size: 12.0,
                    fragment_type: FragmentType::Character,
                },
                OcrTextFragment {
                    text: "C".to_string(),
                    x: 10.0,
                    y: 35.0,
                    width: 20.0,
                    height: 20.0,
                    confidence: 0.9,
                    font_size: 12.0,
                    fragment_type: FragmentType::Character,
                },
                OcrTextFragment {
                    text: "D".to_string(),
                    x: 100.0,
                    y: 100.0,
                    width: 20.0,
                    height: 20.0,
                    confidence: 0.9,
                    font_size: 12.0,
                    fragment_type: FragmentType::Character,
                },
            ];

            let result = OcrProcessingResult {
                text: "ABCD".to_string(),
                confidence: 0.9,
                fragments,
                processing_time_ms: 50,
                engine_name: "Test".to_string(),
                language: "en".to_string(),
                image_dimensions: (200, 200),
            };

            // Test overlapping region - B is partially outside (starts at x=25, width=20, so ends at x=45)
            let region1 = result.fragments_in_region(0.0, 0.0, 50.0, 50.0);
            assert_eq!(region1.len(), 2); // A and C (B extends beyond the region)

            // Test exact fit
            let region2 = result.fragments_in_region(10.0, 10.0, 20.0, 20.0);
            assert_eq!(region2.len(), 1); // Only A

            // Test empty region
            let region3 = result.fragments_in_region(200.0, 200.0, 50.0, 50.0);
            assert_eq!(region3.len(), 0);
        }

        #[test]
        fn test_ocr_result_confidence_edge_cases() {
            let fragments = vec![
                OcrTextFragment {
                    text: "Perfect".to_string(),
                    x: 0.0,
                    y: 0.0,
                    width: 100.0,
                    height: 20.0,
                    confidence: 1.0,
                    font_size: 12.0,
                    fragment_type: FragmentType::Word,
                },
                OcrTextFragment {
                    text: "Zero".to_string(),
                    x: 0.0,
                    y: 25.0,
                    width: 50.0,
                    height: 20.0,
                    confidence: 0.0,
                    font_size: 12.0,
                    fragment_type: FragmentType::Word,
                },
                OcrTextFragment {
                    text: "Mid".to_string(),
                    x: 0.0,
                    y: 50.0,
                    width: 30.0,
                    height: 20.0,
                    confidence: 0.5,
                    font_size: 12.0,
                    fragment_type: FragmentType::Word,
                },
            ];

            let result = OcrProcessingResult {
                text: "Perfect Zero Mid".to_string(),
                confidence: 0.5,
                fragments,
                processing_time_ms: 50,
                engine_name: "Test".to_string(),
                language: "en".to_string(),
                image_dimensions: (200, 200),
            };

            // Test boundary conditions
            assert_eq!(result.filter_by_confidence(0.0).len(), 3);
            assert_eq!(result.filter_by_confidence(0.5).len(), 2);
            assert_eq!(result.filter_by_confidence(1.0).len(), 1);
            assert_eq!(result.filter_by_confidence(1.1).len(), 0);
        }

        #[test]
        fn test_ocr_result_fragment_type_combinations() {
            let fragments = vec![
                OcrTextFragment {
                    text: "A".to_string(),
                    x: 0.0,
                    y: 0.0,
                    width: 10.0,
                    height: 20.0,
                    confidence: 0.9,
                    font_size: 12.0,
                    fragment_type: FragmentType::Character,
                },
                OcrTextFragment {
                    text: "Word".to_string(),
                    x: 20.0,
                    y: 0.0,
                    width: 40.0,
                    height: 20.0,
                    confidence: 0.9,
                    font_size: 12.0,
                    fragment_type: FragmentType::Word,
                },
                OcrTextFragment {
                    text: "Line of text".to_string(),
                    x: 0.0,
                    y: 25.0,
                    width: 100.0,
                    height: 20.0,
                    confidence: 0.9,
                    font_size: 12.0,
                    fragment_type: FragmentType::Line,
                },
                OcrTextFragment {
                    text: "Paragraph text...".to_string(),
                    x: 0.0,
                    y: 50.0,
                    width: 200.0,
                    height: 100.0,
                    confidence: 0.9,
                    font_size: 12.0,
                    fragment_type: FragmentType::Paragraph,
                },
            ];

            let result = OcrProcessingResult {
                text: "Combined".to_string(),
                confidence: 0.9,
                fragments,
                processing_time_ms: 50,
                engine_name: "Test".to_string(),
                language: "en".to_string(),
                image_dimensions: (300, 300),
            };

            assert_eq!(result.fragments_of_type(FragmentType::Character).len(), 1);
            assert_eq!(result.fragments_of_type(FragmentType::Word).len(), 1);
            assert_eq!(result.fragments_of_type(FragmentType::Line).len(), 1);
            assert_eq!(result.fragments_of_type(FragmentType::Paragraph).len(), 1);
        }

        #[test]
        fn test_ocr_result_large_fragment_set() {
            // Test with many fragments (performance test)
            let mut fragments = Vec::new();
            for i in 0..1000 {
                fragments.push(OcrTextFragment {
                    text: format!("Fragment{i}"),
                    x: (i % 10) as f64 * 50.0,
                    y: (i / 10) as f64 * 20.0,
                    width: 45.0,
                    height: 18.0,
                    confidence: 0.5 + (i as f64 % 50.0) / 100.0,
                    font_size: 12.0,
                    fragment_type: if i % 4 == 0 {
                        FragmentType::Line
                    } else {
                        FragmentType::Word
                    },
                });
            }

            let result = OcrProcessingResult {
                text: "Large document".to_string(),
                confidence: 0.75,
                fragments,
                processing_time_ms: 500,
                engine_name: "Test".to_string(),
                language: "en".to_string(),
                image_dimensions: (500, 2000),
            };

            // Test various operations on large set
            let high_conf = result.filter_by_confidence(0.8);
            assert!(high_conf.len() < 1000);

            let lines = result.fragments_of_type(FragmentType::Line);
            assert_eq!(lines.len(), 250); // 1/4 of fragments

            let region = result.fragments_in_region(0.0, 0.0, 200.0, 200.0);
            assert!(!region.is_empty());

            let avg = result.average_confidence();
            assert!(avg > 0.5 && avg < 1.0);
        }

        #[test]
        fn test_ocr_result_empty_handling() {
            let result = OcrProcessingResult {
                text: String::new(),
                confidence: 0.0,
                fragments: vec![],
                processing_time_ms: 10,
                engine_name: "Test".to_string(),
                language: "en".to_string(),
                image_dimensions: (0, 0),
            };

            assert_eq!(result.filter_by_confidence(0.5).len(), 0);
            assert_eq!(result.fragments_in_region(0.0, 0.0, 100.0, 100.0).len(), 0);
            assert_eq!(result.fragments_of_type(FragmentType::Word).len(), 0);
            assert_eq!(result.average_confidence(), 0.0);
        }

        // MockOcrProvider Advanced Tests
        #[test]
        fn test_mock_provider_configuration_mutations() {
            let mut provider = MockOcrProvider::new();

            // Test text mutation
            provider.set_mock_text("Custom mock text".to_string());

            // Test confidence mutation
            provider.set_confidence(0.95);

            // Test delay mutation
            provider.set_processing_delay(200);

            let options = OcrOptions::default();
            let jpeg_data = vec![0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10, 0x4A, 0x46, 0x49, 0x46];

            let result = provider.process_image(&jpeg_data, &options).unwrap();
            assert!(result.text.contains("Custom mock text"));
            assert_eq!(result.confidence, 0.95);
            assert_eq!(result.processing_time_ms, 200);
        }

        #[test]
        fn test_mock_provider_confidence_clamping() {
            let mut provider = MockOcrProvider::new();

            // Test clamping above 1.0
            provider.set_confidence(1.5);
            assert_eq!(provider.confidence, 1.0);

            // Test clamping below 0.0
            provider.set_confidence(-0.5);
            assert_eq!(provider.confidence, 0.0);

            // Test normal values
            provider.set_confidence(0.75);
            assert_eq!(provider.confidence, 0.75);
        }

        #[test]
        fn test_mock_provider_validate_png() {
            let provider = MockOcrProvider::new();

            // Valid PNG signature
            let png_data = vec![0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
            assert!(provider.validate_image_data(&png_data).is_ok());

            // Invalid PNG (corrupted signature)
            let bad_png = vec![0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0B];
            assert!(provider.validate_image_data(&bad_png).is_err());
        }

        #[test]
        fn test_mock_provider_validate_tiff() {
            let provider = MockOcrProvider::new();

            // Valid TIFF (little endian)
            let tiff_le = vec![0x49, 0x49, 0x2A, 0x00, 0x00, 0x00, 0x00, 0x00];
            assert!(provider.validate_image_data(&tiff_le).is_ok());

            // Valid TIFF (big endian)
            let tiff_be = vec![0x4D, 0x4D, 0x00, 0x2A, 0x00, 0x00, 0x00, 0x00];
            assert!(provider.validate_image_data(&tiff_be).is_ok());
        }

        #[test]
        fn test_mock_provider_process_page() {
            let provider = MockOcrProvider::new();
            let options = OcrOptions::default();

            // Create a mock ContentAnalysis
            let analysis = ContentAnalysis {
                page_number: 0,
                page_type: crate::operations::page_analysis::PageType::Scanned,
                text_ratio: 0.0,
                image_ratio: 1.0,
                blank_space_ratio: 0.0,
                text_fragment_count: 0,
                image_count: 1,
                character_count: 0,
            };

            let jpeg_data = vec![0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10, 0x4A, 0x46, 0x49, 0x46];

            // Test that process_page works (default implementation calls process_image)
            let result = provider
                .process_page(&analysis, &jpeg_data, &options)
                .unwrap();
            assert!(result.text.contains("Mock OCR"));
        }

        #[test]
        fn test_mock_provider_thread_safety() {
            use std::sync::Arc;
            use std::thread;

            let provider = Arc::new(MockOcrProvider::new());
            let options = Arc::new(OcrOptions::default());

            let mut handles = vec![];

            // Spawn multiple threads to test Send + Sync
            for i in 0..5 {
                let provider_clone = Arc::clone(&provider);
                let options_clone = Arc::clone(&options);

                let handle = thread::spawn(move || {
                    let jpeg_data =
                        vec![0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10, 0x4A, 0x46, 0x49, 0x46];
                    let result = provider_clone
                        .process_image(&jpeg_data, &options_clone)
                        .unwrap();
                    assert!(result.text.contains("Mock OCR"));
                    i
                });

                handles.push(handle);
            }

            // Wait for all threads
            for handle in handles {
                let thread_id = handle.join().unwrap();
                assert!(thread_id < 5);
            }
        }

        // OcrEngine Tests
        #[test]
        fn test_ocr_engine_display() {
            assert_eq!(format!("{}", OcrEngine::Mock), "Mock OCR");
            assert_eq!(format!("{}", OcrEngine::Tesseract), "Tesseract");
            assert_eq!(format!("{}", OcrEngine::Azure), "Azure Computer Vision");
            assert_eq!(format!("{}", OcrEngine::Aws), "AWS Textract");
            assert_eq!(format!("{}", OcrEngine::GoogleCloud), "Google Cloud Vision");
        }

        #[test]
        fn test_ocr_engine_equality() {
            assert_eq!(OcrEngine::Mock, OcrEngine::Mock);
            assert_ne!(OcrEngine::Mock, OcrEngine::Tesseract);

            // Test Copy trait
            let engine1 = OcrEngine::Azure;
            let engine2 = engine1;
            assert_eq!(engine1, engine2);
        }

        #[test]
        fn test_ocr_engine_format_support_matrix() {
            // Test complete format support matrix
            let _engines = [
                OcrEngine::Mock,
                OcrEngine::Tesseract,
                OcrEngine::Azure,
                OcrEngine::Aws,
                OcrEngine::GoogleCloud,
            ];

            let formats = [ImageFormat::Jpeg, ImageFormat::Png, ImageFormat::Tiff];

            // Expected support matrix
            let expected = vec![
                (OcrEngine::Mock, vec![true, true, true]),
                (OcrEngine::Tesseract, vec![true, true, true]),
                (OcrEngine::Azure, vec![true, true, false]),
                (OcrEngine::Aws, vec![true, true, false]),
                (OcrEngine::GoogleCloud, vec![true, true, false]),
            ];

            for (engine, expected_support) in expected {
                for (i, format) in formats.iter().enumerate() {
                    assert_eq!(
                        engine.supports_format(*format),
                        expected_support[i],
                        "Engine {engine:?} format {format:?} support mismatch"
                    );
                }
            }
        }

        // Integration Tests
        #[test]
        fn test_validate_image_data_all_formats() {
            let provider = MockOcrProvider::new();

            // Test all supported formats
            let test_cases = vec![
                // JPEG with JFIF marker
                (vec![0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10, 0x4A, 0x46], true),
                // JPEG with EXIF marker
                (vec![0xFF, 0xD8, 0xFF, 0xE1, 0x00, 0x10, 0x45, 0x78], true),
                // PNG
                (vec![0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A], true),
                // TIFF LE
                (vec![0x49, 0x49, 0x2A, 0x00, 0x00, 0x00, 0x00, 0x00], true),
                // TIFF BE
                (vec![0x4D, 0x4D, 0x00, 0x2A, 0x00, 0x00, 0x00, 0x00], true),
                // GIF (not supported)
                (vec![0x47, 0x49, 0x46, 0x38, 0x39, 0x61, 0x00, 0x00], false),
                // BMP (not supported)
                (vec![0x42, 0x4D, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00], false),
                // Too short
                (vec![0xFF, 0xD8], false),
                // Empty
                (vec![], false),
            ];

            for (data, should_succeed) in test_cases {
                let result = provider.validate_image_data(&data);
                assert_eq!(
                    result.is_ok(),
                    should_succeed,
                    "Failed for data: {:?}",
                    &data[..data.len().min(8)]
                );
            }
        }

        #[test]
        fn test_ocr_options_with_all_preprocessing() {
            let options = OcrOptions {
                language: "deu+eng+fra".to_string(),
                min_confidence: 0.85,
                preserve_layout: true,
                preprocessing: ImagePreprocessing {
                    denoise: true,
                    deskew: true,
                    enhance_contrast: true,
                    sharpen: true,
                    scale_factor: 1.5,
                },
                engine_options: {
                    let mut map = HashMap::new();
                    map.insert("param1".to_string(), "value1".to_string());
                    map.insert("param2".to_string(), "value2".to_string());
                    map
                },
                timeout_seconds: 120,
            };

            // Verify all fields
            assert_eq!(options.language, "deu+eng+fra");
            assert_eq!(options.min_confidence, 0.85);
            assert!(options.preserve_layout);
            assert!(options.preprocessing.denoise);
            assert!(options.preprocessing.deskew);
            assert!(options.preprocessing.enhance_contrast);
            assert!(options.preprocessing.sharpen);
            assert_eq!(options.preprocessing.scale_factor, 1.5);
            assert_eq!(options.engine_options.len(), 2);
            assert_eq!(options.timeout_seconds, 120);
        }

        #[test]
        fn test_fragment_boundary_calculations() {
            let fragments = [
                OcrTextFragment {
                    text: "TopLeft".to_string(),
                    x: 0.0,
                    y: 0.0,
                    width: 50.0,
                    height: 20.0,
                    confidence: 0.9,
                    font_size: 12.0,
                    fragment_type: FragmentType::Word,
                },
                OcrTextFragment {
                    text: "BottomRight".to_string(),
                    x: 550.0,
                    y: 770.0,
                    width: 60.0,
                    height: 20.0,
                    confidence: 0.9,
                    font_size: 12.0,
                    fragment_type: FragmentType::Word,
                },
            ];

            // Calculate document bounds
            let min_x = fragments.iter().map(|f| f.x).fold(f64::INFINITY, f64::min);
            let min_y = fragments.iter().map(|f| f.y).fold(f64::INFINITY, f64::min);
            let max_x = fragments
                .iter()
                .map(|f| f.x + f.width)
                .fold(f64::NEG_INFINITY, f64::max);
            let max_y = fragments
                .iter()
                .map(|f| f.y + f.height)
                .fold(f64::NEG_INFINITY, f64::max);

            assert_eq!(min_x, 0.0);
            assert_eq!(min_y, 0.0);
            assert_eq!(max_x, 610.0);
            assert_eq!(max_y, 790.0);
        }

        #[test]
        fn test_error_chain_context() {
            use std::io::{Error as IoError, ErrorKind};

            // Test error context preservation
            let io_error = IoError::new(ErrorKind::PermissionDenied, "Access denied to image file");
            let ocr_error: OcrError = io_error.into();

            let error_chain = format!("{ocr_error}");
            assert!(error_chain.contains("IO error"));

            // Test custom error with context
            let processing_error = OcrError::ProcessingFailed(
                "Failed to process page 5: insufficient memory".to_string(),
            );
            let error_msg = format!("{processing_error}");
            assert!(error_msg.contains("page 5"));
            assert!(error_msg.contains("insufficient memory"));
        }

        #[test]
        fn test_concurrent_result_processing() {
            use std::sync::{Arc, Mutex};
            use std::thread;

            // Create shared result
            let result = Arc::new(OcrProcessingResult {
                text: "Concurrent test".to_string(),
                confidence: 0.85,
                fragments: vec![
                    OcrTextFragment {
                        text: "Fragment1".to_string(),
                        x: 0.0,
                        y: 0.0,
                        width: 100.0,
                        height: 20.0,
                        confidence: 0.9,
                        font_size: 12.0,
                        fragment_type: FragmentType::Word,
                    },
                    OcrTextFragment {
                        text: "Fragment2".to_string(),
                        x: 0.0,
                        y: 25.0,
                        width: 100.0,
                        height: 20.0,
                        confidence: 0.8,
                        font_size: 12.0,
                        fragment_type: FragmentType::Word,
                    },
                ],
                processing_time_ms: 100,
                engine_name: "Test".to_string(),
                language: "en".to_string(),
                image_dimensions: (200, 100),
            });

            let counter = Arc::new(Mutex::new(0));
            let mut handles = vec![];

            // Spawn threads to process result concurrently
            for _ in 0..10 {
                let result_clone = Arc::clone(&result);
                let counter_clone = Arc::clone(&counter);

                let handle = thread::spawn(move || {
                    // Perform various read operations
                    let _ = result_clone.filter_by_confidence(0.85);
                    let _ = result_clone.fragments_in_region(0.0, 0.0, 200.0, 100.0);
                    let _ = result_clone.average_confidence();

                    let mut count = counter_clone.lock().unwrap();
                    *count += 1;
                });

                handles.push(handle);
            }

            // Wait for all threads
            for handle in handles {
                handle.join().unwrap();
            }

            assert_eq!(*counter.lock().unwrap(), 10);
        }
    }
}
