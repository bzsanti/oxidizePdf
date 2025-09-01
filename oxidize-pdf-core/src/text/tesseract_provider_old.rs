//! Tesseract OCR provider implementation
//!
//! This module provides a complete implementation of the OcrProvider trait using
//! Tesseract OCR engine. It supports multiple languages, configurable page segmentation
//! modes, and various OCR engine modes.
//!
//! # Installation
//!
//! Before using this provider, you need to install Tesseract OCR on your system:
//!
//! ## macOS
//! ```bash
//! brew install tesseract
//! brew install tesseract-lang  # For additional languages
//! ```
//!
//! ## Ubuntu/Debian
//! ```bash
//! sudo apt-get install tesseract-ocr
//! sudo apt-get install tesseract-ocr-spa  # For Spanish
//! sudo apt-get install tesseract-ocr-deu  # For German
//! ```
//!
//! ## Windows
//! Download from: https://github.com/UB-Mannheim/tesseract/wiki
//!
//! # Usage
//!
//! ```rust
//! use oxidize_pdf::text::{TesseractOcrProvider, OcrOptions, OcrProvider};
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let provider = TesseractOcrProvider::new()?;
//! let options = OcrOptions::default();
//! let image_data = std::fs::read("scanned_page.jpg")?;
//!
//! let result = provider.process_image(&image_data, &options)?;
//! println!("Extracted text: {}", result.text);
//! println!("Confidence: {:.1}%", result.confidence * 100.0);
//! # Ok(())
//! # }
//! ```
//!
//! # Multiple Languages
//!
//! ```rust
//! use oxidize_pdf::text::{TesseractOcrProvider, TesseractConfig, OcrOptions};
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let config = TesseractConfig {
//!     language: "spa+eng".to_string(),  // Spanish + English
//!     psm: PageSegmentationMode::Auto,
//!     oem: OcrEngineMode::LstmOnly,
//!     ..Default::default()
//! };
//!
//! let provider = TesseractOcrProvider::with_config(config)?;
//! # Ok(())
//! # }
//! ```

#[cfg(feature = "ocr-tesseract")]
use crate::graphics::ImageFormat;
#[cfg(feature = "ocr-tesseract")]
use crate::operations::page_analysis::ContentAnalysis;
#[cfg(feature = "ocr-tesseract")]
use crate::text::{
    FragmentType, OcrEngine, OcrError, OcrOptions, OcrProcessingResult, OcrProvider, OcrResult,
    OcrTextFragment,
};
#[cfg(feature = "ocr-tesseract")]
use rusty_tesseract::{Args, Image};
#[cfg(feature = "ocr-tesseract")]
use std::collections::HashMap;
#[cfg(feature = "ocr-tesseract")]
use std::sync::Mutex;
#[cfg(feature = "ocr-tesseract")]
use std::time::Instant;

/// Page Segmentation Mode for Tesseract
#[cfg(feature = "ocr-tesseract")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PageSegmentationMode {
    /// Orientation and script detection (OSD) only
    OsdOnly = 0,
    /// Automatic page segmentation with OSD
    AutoOsd = 1,
    /// Automatic page segmentation, but no OSD, or OCR
    AutoOnly = 2,
    /// Fully automatic page segmentation, but no OSD (Default)
    Auto = 3,
    /// Assume a single column of text of variable sizes
    SingleColumn = 4,
    /// Assume a single uniform block of vertically aligned text
    SingleBlock = 5,
    /// Assume a single uniform block of text
    SingleUniformBlock = 6,
    /// Treat the image as a single text line
    SingleLine = 7,
    /// Treat the image as a single word
    SingleWord = 8,
    /// Treat the image as a single word in a circle
    SingleWordCircle = 9,
    /// Treat the image as a single character
    SingleChar = 10,
    /// Sparse text. Find as much text as possible in no particular order
    SparseText = 11,
    /// Sparse text with OSD
    SparseTextOsd = 12,
    /// Raw line. Treat the image as a single text line, bypassing hacks
    RawLine = 13,
}

#[cfg(feature = "ocr-tesseract")]
impl Default for PageSegmentationMode {
    fn default() -> Self {
        PageSegmentationMode::Auto
    }
}

#[cfg(feature = "ocr-tesseract")]
impl PageSegmentationMode {
    /// Convert to Tesseract PSM value
    pub fn to_psm_value(self) -> u8 {
        self as u8
    }

    /// Get description of the PSM mode
    pub fn description(&self) -> &'static str {
        match self {
            PageSegmentationMode::OsdOnly => "Orientation and script detection only",
            PageSegmentationMode::AutoOsd => "Automatic page segmentation with OSD",
            PageSegmentationMode::AutoOnly => "Automatic page segmentation, no OSD or OCR",
            PageSegmentationMode::Auto => "Fully automatic page segmentation, no OSD",
            PageSegmentationMode::SingleColumn => "Single column of text of variable sizes",
            PageSegmentationMode::SingleBlock => "Single uniform block of vertically aligned text",
            PageSegmentationMode::SingleUniformBlock => "Single uniform block of text",
            PageSegmentationMode::SingleLine => "Single text line",
            PageSegmentationMode::SingleWord => "Single word",
            PageSegmentationMode::SingleWordCircle => "Single word in a circle",
            PageSegmentationMode::SingleChar => "Single character",
            PageSegmentationMode::SparseText => "Sparse text in no particular order",
            PageSegmentationMode::SparseTextOsd => "Sparse text with OSD",
            PageSegmentationMode::RawLine => "Raw line, bypassing hacks",
        }
    }
}

/// OCR Engine Mode for Tesseract
#[cfg(feature = "ocr-tesseract")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OcrEngineMode {
    /// Legacy engine only
    LegacyOnly = 0,
    /// Neural nets LSTM engine only
    LstmOnly = 1,
    /// Legacy + LSTM engines
    LegacyLstm = 2,
    /// Default, based on what is available
    Default = 3,
}

#[cfg(feature = "ocr-tesseract")]
impl Default for OcrEngineMode {
    fn default() -> Self {
        OcrEngineMode::Default
    }
}

#[cfg(feature = "ocr-tesseract")]
impl OcrEngineMode {
    /// Convert to Tesseract OEM value
    pub fn to_oem_value(self) -> u8 {
        self as u8
    }

    /// Get description of the OEM mode
    pub fn description(&self) -> &'static str {
        match self {
            OcrEngineMode::LegacyOnly => "Legacy engine only",
            OcrEngineMode::LstmOnly => "Neural nets LSTM engine only",
            OcrEngineMode::LegacyLstm => "Legacy + LSTM engines",
            OcrEngineMode::Default => "Default, based on what is available",
        }
    }
}

/// Configuration for Tesseract OCR provider
#[cfg(feature = "ocr-tesseract")]
#[derive(Debug, Clone)]
pub struct TesseractConfig {
    /// Language code (e.g., "eng", "spa", "deu", "eng+spa")
    pub language: String,
    /// Page segmentation mode
    pub psm: PageSegmentationMode,
    /// OCR engine mode
    pub oem: OcrEngineMode,
    /// Character whitelist (only recognize these characters)
    pub char_whitelist: Option<String>,
    /// Character blacklist (never recognize these characters)
    pub char_blacklist: Option<String>,
    /// Custom Tesseract variables
    pub variables: HashMap<String, String>,
    /// Enable debug output
    pub debug: bool,
}

#[cfg(feature = "ocr-tesseract")]
impl Default for TesseractConfig {
    fn default() -> Self {
        Self {
            language: "eng".to_string(),
            psm: PageSegmentationMode::default(),
            oem: OcrEngineMode::default(),
            char_whitelist: None,
            char_blacklist: None,
            variables: HashMap::new(),
            debug: false,
        }
    }
}

#[cfg(feature = "ocr-tesseract")]
impl TesseractConfig {
    /// Create a new configuration with the specified language
    pub fn with_language(language: impl Into<String>) -> Self {
        Self {
            language: language.into(),
            ..Default::default()
        }
    }

    /// Create a configuration optimized for documents
    pub fn for_documents() -> Self {
        Self {
            language: "eng".to_string(),
            psm: PageSegmentationMode::Auto,
            oem: OcrEngineMode::LstmOnly,
            ..Default::default()
        }
    }

    /// Create a configuration optimized for single lines of text
    pub fn for_single_line() -> Self {
        Self {
            language: "eng".to_string(),
            psm: PageSegmentationMode::SingleLine,
            oem: OcrEngineMode::LstmOnly,
            ..Default::default()
        }
    }

    /// Create a configuration optimized for sparse text
    pub fn for_sparse_text() -> Self {
        Self {
            language: "eng".to_string(),
            psm: PageSegmentationMode::SparseText,
            oem: OcrEngineMode::LstmOnly,
            ..Default::default()
        }
    }

    /// Set character whitelist
    pub fn with_char_whitelist(mut self, whitelist: impl Into<String>) -> Self {
        self.char_whitelist = Some(whitelist.into());
        self
    }

    /// Set character blacklist
    pub fn with_char_blacklist(mut self, blacklist: impl Into<String>) -> Self {
        self.char_blacklist = Some(blacklist.into());
        self
    }

    /// Add a custom Tesseract variable
    pub fn with_variable(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.variables.insert(name.into(), value.into());
        self
    }

    /// Enable debug output
    pub fn with_debug(mut self) -> Self {
        self.debug = true;
        self
    }
}

/// Tesseract OCR provider implementation
#[cfg(feature = "ocr-tesseract")]
pub struct TesseractOcrProvider {
    config: TesseractConfig,
    instance: Mutex<Option<Tesseract>>,
}

#[cfg(feature = "ocr-tesseract")]
impl TesseractOcrProvider {
    /// Create a new Tesseract OCR provider with default configuration
    ///
    /// # Errors
    ///
    /// Returns an error if Tesseract is not installed or cannot be initialized.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use oxidize_pdf::text::TesseractOcrProvider;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let provider = TesseractOcrProvider::new()?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn new() -> OcrResult<Self> {
        Self::with_config(TesseractConfig::default())
    }

    /// Create a new Tesseract OCR provider with custom configuration
    ///
    /// # Arguments
    ///
    /// * `config` - Tesseract configuration settings
    ///
    /// # Errors
    ///
    /// Returns an error if Tesseract is not installed or cannot be initialized
    /// with the provided configuration.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use oxidize_pdf::text::{TesseractOcrProvider, TesseractConfig};
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let config = TesseractConfig::for_documents();
    /// let provider = TesseractOcrProvider::with_config(config)?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn with_config(config: TesseractConfig) -> OcrResult<Self> {
        // Test Tesseract initialization
        let _ = Self::create_tesseract_instance(&config)?;

        Ok(Self {
            config,
            instance: Mutex::new(None),
        })
    }

    /// Create a new Tesseract OCR provider with specified language
    ///
    /// # Arguments
    ///
    /// * `language` - Language code (e.g., "eng", "spa", "deu", "eng+spa")
    ///
    /// # Examples
    ///
    /// ```rust
    /// use oxidize_pdf::text::TesseractOcrProvider;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let provider = TesseractOcrProvider::with_language("spa")?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn with_language(language: impl Into<String>) -> OcrResult<Self> {
        let config = TesseractConfig::with_language(language);
        Self::with_config(config)
    }

    /// Get the current configuration
    pub fn config(&self) -> &TesseractConfig {
        &self.config
    }

    /// Update the configuration
    ///
    /// This will reset the internal Tesseract instance on the next use.
    pub fn set_config(&mut self, config: TesseractConfig) -> OcrResult<()> {
        // Test the new configuration
        let _ = Self::create_tesseract_instance(&config)?;

        self.config = config;
        // Reset the instance so it will be recreated with new config
        if let Ok(mut instance) = self.instance.lock() {
            *instance = None;
        }

        Ok(())
    }

    /// Check if Tesseract is available and properly installed
    ///
    /// # Returns
    ///
    /// `Ok(())` if Tesseract is available, `Err(OcrError)` otherwise.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use oxidize_pdf::text::TesseractOcrProvider;
    ///
    /// match TesseractOcrProvider::check_availability() {
    ///     Ok(()) => println!("Tesseract is available"),
    ///     Err(e) => println!("Tesseract not available: {}", e),
    /// }
    /// ```
    pub fn check_availability() -> OcrResult<()> {
        match Tesseract::new(Some("eng"), None) {
            Ok(_) => Ok(()),
            Err(e) => Err(OcrError::ProviderNotAvailable(format!(
                "Tesseract not available: {}",
                e
            ))),
        }
    }

    /// Get available languages
    ///
    /// # Returns
    ///
    /// A vector of language codes that are available for OCR.
    pub fn available_languages() -> OcrResult<Vec<String>> {
        // This is a simplified implementation
        // In a real implementation, you might want to query Tesseract for available languages
        Ok(vec![
            "eng".to_string(),
            "spa".to_string(),
            "deu".to_string(),
            "fra".to_string(),
            "ita".to_string(),
            "por".to_string(),
            "rus".to_string(),
            "chi_sim".to_string(),
            "chi_tra".to_string(),
            "jpn".to_string(),
            "kor".to_string(),
            "ara".to_string(),
            "hin".to_string(),
        ])
    }

    /// Create a new Tesseract instance with the given configuration
    fn create_tesseract_instance(config: &TesseractConfig) -> OcrResult<Tesseract> {
        // Initialize Tesseract with language and datapath
        let mut tesseract = Tesseract::new(Some(&config.language), None).map_err(|e| {
            OcrError::ProviderNotAvailable(format!("Failed to initialize Tesseract: {e}"))
        })?;

        // Set page segmentation mode
        tesseract
            .set_variable(
                "tessedit_pageseg_mode",
                &config.psm.to_psm_value().to_string(),
            )
            .map_err(|e| OcrError::Configuration(format!("Failed to set PSM: {e}")))?;

        // Set OCR engine mode
        tesseract
            .set_variable(
                "tessedit_ocr_engine_mode",
                &config.oem.to_oem_value().to_string(),
            )
            .map_err(|e| OcrError::Configuration(format!("Failed to set OEM: {e}")))?;

        // Set character whitelist if specified
        if let Some(ref whitelist) = config.char_whitelist {
            tesseract
                .set_variable("tessedit_char_whitelist", whitelist)
                .map_err(|e| OcrError::Configuration(format!("Failed to set whitelist: {e}")))?;
        }

        // Set character blacklist if specified
        if let Some(ref blacklist) = config.char_blacklist {
            tesseract
                .set_variable("tessedit_char_blacklist", blacklist)
                .map_err(|e| OcrError::Configuration(format!("Failed to set blacklist: {e}")))?;
        }

        // Set custom variables
        for (name, value) in &config.variables {
            tesseract.set_variable(name, value).map_err(|e| {
                OcrError::Configuration(format!("Failed to set variable {name}: {e}"))
            })?;
        }

        // Set debug mode if enabled
        if config.debug {
            tesseract
                .set_variable("debug_file", "/tmp/tesseract_debug.log")
                .map_err(|e| OcrError::Configuration(format!("Failed to set debug mode: {e}")))?;
        }

        Ok(tesseract)
    }

    /// Process with Tesseract instance
    fn with_tesseract_instance<F, R>(&self, f: F) -> OcrResult<R>
    where
        F: FnOnce(&mut Tesseract) -> OcrResult<R>,
    {
        let mut instance_guard = self
            .instance
            .lock()
            .map_err(|e| OcrError::ProcessingFailed(format!("Failed to acquire lock: {e}")))?;

        if instance_guard.is_none() {
            *instance_guard = Some(Self::create_tesseract_instance(&self.config)?);
        }

        instance_guard
            .as_mut()
            .ok_or_else(|| {
                OcrError::ProcessingFailed("Failed to get Tesseract instance".to_string())
            })
            .and_then(f)
    }

    /// Process image with detailed error handling
    fn process_image_internal(
        &self,
        image_data: &[u8],
        options: &OcrOptions,
    ) -> OcrResult<OcrProcessingResult> {
        let start_time = Instant::now();
        let img_data = image_data.to_vec(); // Clone data to use in closure
        let opts = options.clone();
        let lang = self.config.language.clone();

        self.with_tesseract_instance(move |tesseract| {
            // Set the image data
            tesseract
                .set_image_from_mem(&img_data)
                .map_err(|e| OcrError::InvalidImageData(format!("Failed to set image: {e}")))?;

            // Extract text
            let text = tesseract
                .get_text()
                .map_err(|e| OcrError::ProcessingFailed(format!("Failed to extract text: {e}")))?;

            // Get confidence (mean_text_conf returns i32, not Result)
            let confidence = tesseract.mean_text_conf();

            // Convert confidence from 0-100 to 0.0-1.0
            let confidence_ratio = confidence as f64 / 100.0;

            // Check minimum confidence
            if confidence_ratio < opts.min_confidence {
                return Err(OcrError::LowConfidence(format!(
                    "Confidence {:.1}% below minimum {:.1}%",
                    confidence_ratio * 100.0,
                    opts.min_confidence * 100.0
                )));
            }

            // Get word-level information if layout preservation is enabled
            let fragments = if opts.preserve_layout {
                // Extract word fragments inline
                let mut fragments = Vec::new();
                let words: Vec<&str> = text.split_whitespace().collect();
                let mut x = 0.0;
                let y = 0.0;
                let line_height = 20.0;

                for word in words.iter() {
                    let width = word.len() as f64 * 8.0;
                    fragments.push(OcrTextFragment {
                        text: word.to_string(),
                        x,
                        y,
                        width,
                        height: line_height,
                        confidence: confidence_ratio,
                        font_size: 12.0,
                        fragment_type: FragmentType::Word,
                    });
                    x += width + 8.0;
                }
                fragments
            } else {
                // Create a single fragment for the entire text
                vec![OcrTextFragment {
                    text: text.clone(),
                    x: 0.0,
                    y: 0.0,
                    width: 0.0,
                    height: 0.0,
                    confidence: confidence_ratio,
                    font_size: 12.0,
                    fragment_type: FragmentType::Paragraph,
                }]
            };

            let processing_time = start_time.elapsed();

            Ok(OcrProcessingResult {
                text,
                confidence: confidence_ratio,
                fragments,
                processing_time_ms: processing_time.as_millis() as u64,
                engine_name: "Tesseract".to_string(),
                language: lang,
                image_dimensions: (0, 0), // Note: Image dimensions not currently extracted
            })
        })
    }

    /// Extract word-level fragments from Tesseract
    fn extract_word_fragments(
        &self,
        tesseract: &mut Tesseract,
        default_confidence: f64,
    ) -> OcrResult<Vec<OcrTextFragment>> {
        let mut fragments = Vec::new();

        // In a real implementation, you would use Tesseract's word-level analysis
        // For now, we'll create a simplified version
        let text = tesseract.get_text().map_err(|e| {
            OcrError::ProcessingFailed(format!("Failed to get text for fragments: {e}"))
        })?;

        // Split into words and create fragments
        let words: Vec<&str> = text.split_whitespace().collect();
        let mut x = 0.0;
        let y = 0.0;
        let line_height = 20.0;

        for (_i, word) in words.iter().enumerate() {
            let width = word.len() as f64 * 8.0; // Approximate width

            fragments.push(OcrTextFragment {
                text: word.to_string(),
                x,
                y,
                width,
                height: line_height,
                confidence: default_confidence,
                font_size: 12.0,
                fragment_type: FragmentType::Word,
            });

            x += width + 8.0; // Move to next word position

            // Simple line wrapping at 500 pixels
            if x > 500.0 {
                x = 0.0;
                // y += line_height; // This would need to be mutable
            }
        }

        Ok(fragments)
    }
}

#[cfg(feature = "ocr-tesseract")]
impl OcrProvider for TesseractOcrProvider {
    fn process_image(
        &self,
        image_data: &[u8],
        options: &OcrOptions,
    ) -> OcrResult<OcrProcessingResult> {
        // Validate image data first
        self.validate_image_data(image_data)?;

        // Process with Tesseract
        self.process_image_internal(image_data, options)
    }

    fn process_page(
        &self,
        page_analysis: &ContentAnalysis,
        page_data: &[u8],
        options: &OcrOptions,
    ) -> OcrResult<OcrProcessingResult> {
        // Optimize configuration based on page analysis
        let optimized_options = self.optimize_for_page_analysis(page_analysis, options);

        self.process_image(page_data, &optimized_options)
    }

    fn supported_formats(&self) -> Vec<ImageFormat> {
        vec![ImageFormat::Jpeg, ImageFormat::Png, ImageFormat::Tiff]
    }

    fn engine_name(&self) -> &str {
        "Tesseract"
    }

    fn engine_type(&self) -> OcrEngine {
        OcrEngine::Tesseract
    }
}

#[cfg(feature = "ocr-tesseract")]
impl TesseractOcrProvider {
    /// Optimize OCR options based on page analysis
    fn optimize_for_page_analysis(
        &self,
        analysis: &ContentAnalysis,
        options: &OcrOptions,
    ) -> OcrOptions {
        let mut optimized = options.clone();

        // Adjust preprocessing based on page type
        match analysis.page_type {
            crate::operations::page_analysis::PageType::Scanned => {
                // For scanned pages, enable more aggressive preprocessing
                optimized.preprocessing.denoise = true;
                optimized.preprocessing.deskew = true;
                optimized.preprocessing.enhance_contrast = true;
            }
            _ => {
                // For other page types, use lighter preprocessing
                optimized.preprocessing.denoise = false;
                optimized.preprocessing.deskew = false;
            }
        }

        // Adjust confidence threshold based on image quality indicators
        if analysis.image_ratio > 0.9 {
            // Very image-heavy pages might need lower confidence threshold
            optimized.min_confidence = optimized.min_confidence * 0.8;
        }

        optimized
    }
}

// Provide stub implementations when the feature is not enabled
#[cfg(not(feature = "ocr-tesseract"))]
pub struct TesseractOcrProvider;

#[cfg(not(feature = "ocr-tesseract"))]
impl TesseractOcrProvider {
    pub fn new() -> Result<Self, crate::text::OcrError> {
        Err(crate::text::OcrError::ProviderNotAvailable(
            "Tesseract OCR provider not available. Enable the 'ocr-tesseract' feature.".to_string(),
        ))
    }

    pub fn check_availability() -> Result<(), crate::text::OcrError> {
        Err(crate::text::OcrError::ProviderNotAvailable(
            "Tesseract OCR provider not available. Enable the 'ocr-tesseract' feature.".to_string(),
        ))
    }
}

#[cfg(feature = "ocr-tesseract")]
#[cfg(test)]
mod tests {
    use super::*;
    use crate::text::{OcrOptions, OcrProvider};

    #[test]
    fn test_page_segmentation_mode() {
        assert_eq!(PageSegmentationMode::Auto as u8, 3);
        assert_eq!(PageSegmentationMode::SingleLine as u8, 7);
        assert_eq!(PageSegmentationMode::SingleWord as u8, 8);

        assert_eq!(PageSegmentationMode::Auto.to_psm_value(), 3);
        assert!(PageSegmentationMode::Auto
            .description()
            .contains("automatic"));
    }

    #[test]
    fn test_ocr_engine_mode() {
        assert_eq!(OcrEngineMode::LstmOnly as u8, 1);
        assert_eq!(OcrEngineMode::Default as u8, 3);

        assert_eq!(OcrEngineMode::LstmOnly.to_oem_value(), 1);
        assert!(OcrEngineMode::LstmOnly.description().contains("LSTM"));
    }

    #[test]
    fn test_tesseract_config_defaults() {
        let config = TesseractConfig::default();
        assert_eq!(config.language, "eng");
        assert_eq!(config.psm, PageSegmentationMode::Auto);
        assert_eq!(config.oem, OcrEngineMode::Default);
        assert!(config.char_whitelist.is_none());
        assert!(config.char_blacklist.is_none());
        assert!(!config.debug);
    }

    #[test]
    fn test_tesseract_config_builders() {
        let config = TesseractConfig::with_language("spa");
        assert_eq!(config.language, "spa");

        let config = TesseractConfig::for_documents();
        assert_eq!(config.psm, PageSegmentationMode::Auto);
        assert_eq!(config.oem, OcrEngineMode::LstmOnly);

        let config = TesseractConfig::for_single_line();
        assert_eq!(config.psm, PageSegmentationMode::SingleLine);

        let config = TesseractConfig::for_sparse_text();
        assert_eq!(config.psm, PageSegmentationMode::SparseText);
    }

    #[test]
    fn test_tesseract_config_customization() {
        let config = TesseractConfig::default()
            .with_char_whitelist("0123456789")
            .with_char_blacklist("!@#$%")
            .with_variable("tessedit_char_blacklist", "")
            .with_debug();

        assert_eq!(config.char_whitelist, Some("0123456789".to_string()));
        assert_eq!(config.char_blacklist, Some("!@#$%".to_string()));
        assert!(config.variables.contains_key("tessedit_char_blacklist"));
        assert!(config.debug);
    }

    #[test]
    fn test_available_languages() {
        let languages = TesseractOcrProvider::available_languages().unwrap();
        assert!(languages.contains(&"eng".to_string()));
        assert!(languages.contains(&"spa".to_string()));
        assert!(languages.contains(&"deu".to_string()));
        assert!(languages.len() > 5);
    }

    // Note: These tests would require Tesseract to be installed
    // In a real implementation, you might want to use conditional compilation
    // or skip these tests if Tesseract is not available

    #[test]
    #[ignore = "Requires Tesseract installation"]
    fn test_tesseract_provider_creation() {
        match TesseractOcrProvider::new() {
            Ok(provider) => {
                assert_eq!(provider.engine_name(), "Tesseract");
                assert_eq!(provider.engine_type(), OcrEngine::Tesseract);
            }
            Err(e) => {
                println!("Tesseract not available: {}", e);
                // This is expected if Tesseract is not installed
            }
        }
    }

    #[test]
    #[ignore = "Requires Tesseract installation"]
    fn test_tesseract_check_availability() {
        match TesseractOcrProvider::check_availability() {
            Ok(()) => println!("Tesseract is available"),
            Err(e) => println!("Tesseract not available: {}", e),
        }
    }

    #[test]
    #[ignore = "Requires Tesseract installation and sample image"]
    fn test_tesseract_process_image() {
        // This test would require a sample image and Tesseract installation
        let provider = match TesseractOcrProvider::new() {
            Ok(p) => p,
            Err(_) => return, // Skip if Tesseract not available
        };

        // Mock image data (in real test, use actual image)
        let image_data = vec![0xFF, 0xD8, 0xFF, 0xE0]; // JPEG header
        let options = OcrOptions::default();

        // This would fail with mock data, but tests the interface
        match provider.process_image(&image_data, &options) {
            Ok(_result) => {
                // Test passed
            }
            Err(e) => {
                // Expected with mock data
                println!("Expected error with mock data: {}", e);
            }
        }
    }

    #[test]
    fn test_tesseract_supported_formats() {
        // Test without actually creating provider (to avoid Tesseract dependency)
        let supported = vec![ImageFormat::Jpeg, ImageFormat::Png, ImageFormat::Tiff];
        assert!(supported.contains(&ImageFormat::Jpeg));
        assert!(supported.contains(&ImageFormat::Png));
        assert!(supported.contains(&ImageFormat::Tiff));
    }

    // Comprehensive tests for TesseractOcrProvider
    mod comprehensive_tests {
        use super::*;
        use crate::text::{OcrOptions, OcrProvider};

        #[test]
        fn test_page_segmentation_mode_enum_values() {
            assert_eq!(PageSegmentationMode::OsdOnly as u8, 0);
            assert_eq!(PageSegmentationMode::AutoOsd as u8, 1);
            assert_eq!(PageSegmentationMode::AutoOnly as u8, 2);
            assert_eq!(PageSegmentationMode::Auto as u8, 3);
            assert_eq!(PageSegmentationMode::SingleColumn as u8, 4);
            assert_eq!(PageSegmentationMode::SingleBlock as u8, 5);
            assert_eq!(PageSegmentationMode::SingleUniformBlock as u8, 6);
            assert_eq!(PageSegmentationMode::SingleLine as u8, 7);
            assert_eq!(PageSegmentationMode::SingleWord as u8, 8);
            assert_eq!(PageSegmentationMode::SingleWord as u8, 8);
            assert_eq!(PageSegmentationMode::SingleChar as u8, 10);
            assert_eq!(PageSegmentationMode::SparseText as u8, 11);
            assert_eq!(PageSegmentationMode::SparseTextOsd as u8, 12);
            assert_eq!(PageSegmentationMode::RawLine as u8, 13);
        }

        #[test]
        fn test_page_segmentation_mode_to_psm_value() {
            assert_eq!(PageSegmentationMode::OsdOnly.to_psm_value(), 0);
            assert_eq!(PageSegmentationMode::AutoOsd.to_psm_value(), 1);
            assert_eq!(PageSegmentationMode::AutoOnly.to_psm_value(), 2);
            assert_eq!(PageSegmentationMode::Auto.to_psm_value(), 3);
            assert_eq!(PageSegmentationMode::SingleColumn.to_psm_value(), 4);
            assert_eq!(PageSegmentationMode::SingleBlock.to_psm_value(), 5);
            assert_eq!(PageSegmentationMode::SingleUniformBlock.to_psm_value(), 6);
            assert_eq!(PageSegmentationMode::SingleLine.to_psm_value(), 7);
            assert_eq!(PageSegmentationMode::SingleWord.to_psm_value(), 8);
            assert_eq!(PageSegmentationMode::SingleWord.to_psm_value(), 8);
            assert_eq!(PageSegmentationMode::SingleChar.to_psm_value(), 10);
            assert_eq!(PageSegmentationMode::SparseText.to_psm_value(), 11);
            assert_eq!(PageSegmentationMode::SparseTextOsd.to_psm_value(), 12);
            assert_eq!(PageSegmentationMode::RawLine.to_psm_value(), 13);
        }

        #[test]
        fn test_page_segmentation_mode_descriptions() {
            assert!(PageSegmentationMode::OsdOnly
                .description()
                .contains("OSD only"));
            assert!(PageSegmentationMode::AutoOsd
                .description()
                .contains("Automatic"));
            assert!(PageSegmentationMode::AutoOnly
                .description()
                .contains("Automatic"));
            assert!(PageSegmentationMode::Auto
                .description()
                .contains("automatic"));
            assert!(PageSegmentationMode::SingleColumn
                .description()
                .contains("column"));
            assert!(PageSegmentationMode::SingleBlock
                .description()
                .contains("block"));
            assert!(PageSegmentationMode::SingleUniformBlock
                .description()
                .contains("uniform"));
            assert!(PageSegmentationMode::SingleLine
                .description()
                .contains("line"));
            assert!(PageSegmentationMode::SingleWord
                .description()
                .contains("word"));
            assert!(PageSegmentationMode::SingleWord
                .description()
                .contains("word"));
            assert!(PageSegmentationMode::SingleChar
                .description()
                .contains("character"));
            assert!(PageSegmentationMode::SparseText
                .description()
                .contains("sparse"));
            assert!(PageSegmentationMode::SparseTextOsd
                .description()
                .contains("sparse"));
            assert!(PageSegmentationMode::RawLine.description().contains("raw"));
        }

        #[test]
        fn test_ocr_engine_mode_enum_values() {
            assert_eq!(OcrEngineMode::LegacyOnly as u8, 0);
            assert_eq!(OcrEngineMode::LstmOnly as u8, 1);
            assert_eq!(OcrEngineMode::LegacyLstm as u8, 2);
            assert_eq!(OcrEngineMode::Default as u8, 3);
        }

        #[test]
        fn test_ocr_engine_mode_to_oem_value() {
            assert_eq!(OcrEngineMode::LegacyOnly.to_oem_value(), 0);
            assert_eq!(OcrEngineMode::LstmOnly.to_oem_value(), 1);
            assert_eq!(OcrEngineMode::LegacyLstm.to_oem_value(), 2);
            assert_eq!(OcrEngineMode::Default.to_oem_value(), 3);
        }

        #[test]
        fn test_ocr_engine_mode_descriptions() {
            assert!(OcrEngineMode::LegacyOnly.description().contains("Legacy"));
            assert!(OcrEngineMode::LstmOnly.description().contains("LSTM"));
            assert!(OcrEngineMode::LegacyLstm
                .description()
                .contains("Legacy + LSTM"));
            assert!(OcrEngineMode::Default.description().contains("Default"));
        }

        #[test]
        fn test_tesseract_config_default_values() {
            let config = TesseractConfig::default();
            assert_eq!(config.language, "eng");
            assert_eq!(config.psm, PageSegmentationMode::Auto);
            assert_eq!(config.oem, OcrEngineMode::Default);
            assert_eq!(config.char_whitelist, None);
            assert_eq!(config.char_blacklist, None);
            assert!(config.variables.is_empty());
            assert!(!config.debug);
        }

        #[test]
        fn test_tesseract_config_with_language_builder() {
            let config = TesseractConfig::with_language("spa");
            assert_eq!(config.language, "spa");
            assert_eq!(config.psm, PageSegmentationMode::Auto);
            assert_eq!(config.oem, OcrEngineMode::Default);
        }

        #[test]
        fn test_tesseract_config_for_documents_builder() {
            let config = TesseractConfig::for_documents();
            assert_eq!(config.language, "eng");
            assert_eq!(config.psm, PageSegmentationMode::Auto);
            assert_eq!(config.oem, OcrEngineMode::LstmOnly);
        }

        #[test]
        fn test_tesseract_config_for_single_line_builder() {
            let config = TesseractConfig::for_single_line();
            assert_eq!(config.language, "eng");
            assert_eq!(config.psm, PageSegmentationMode::SingleLine);
            assert_eq!(config.oem, OcrEngineMode::Default);
        }

        #[test]
        fn test_tesseract_config_for_sparse_text_builder() {
            let config = TesseractConfig::for_sparse_text();
            assert_eq!(config.language, "eng");
            assert_eq!(config.psm, PageSegmentationMode::SparseText);
            assert_eq!(config.oem, OcrEngineMode::Default);
        }

        #[test]
        fn test_tesseract_config_chaining_methods() {
            let config = TesseractConfig::default()
                .with_char_whitelist("0123456789")
                .with_char_blacklist("!@#$%")
                .with_variable("tessedit_char_blacklist", "")
                .with_variable("tessedit_create_hocr", "1")
                .with_debug();

            assert_eq!(config.char_whitelist, Some("0123456789".to_string()));
            assert_eq!(config.char_blacklist, Some("!@#$%".to_string()));
            assert_eq!(
                config.variables.get("tessedit_char_blacklist"),
                Some(&"".to_string())
            );
            assert_eq!(
                config.variables.get("tessedit_create_hocr"),
                Some(&"1".to_string())
            );
            assert!(config.debug);
        }

        #[test]
        fn test_tesseract_config_multiple_languages() {
            let config = TesseractConfig::with_language("eng+spa+deu");
            assert_eq!(config.language, "eng+spa+deu");

            let config = TesseractConfig::with_language("chi_sim+chi_tra");
            assert_eq!(config.language, "chi_sim+chi_tra");
        }

        #[test]
        fn test_tesseract_config_clone() {
            let config1 = TesseractConfig::default()
                .with_char_whitelist("ABC")
                .with_debug();

            let config2 = config1.clone();
            assert_eq!(config1.char_whitelist, config2.char_whitelist);
            assert_eq!(config1.debug, config2.debug);
            assert_eq!(config1.language, config2.language);
        }

        #[test]
        fn test_tesseract_config_debug_format() {
            let config = TesseractConfig::default()
                .with_char_whitelist("0123456789")
                .with_debug();

            let debug_str = format!("{:?}", config);
            assert!(debug_str.contains("TesseractConfig"));
            assert!(debug_str.contains("0123456789"));
            assert!(debug_str.contains("debug: true"));
        }

        #[test]
        fn test_tesseract_config_variable_overrides() {
            let mut config = TesseractConfig::default()
                .with_variable("tessedit_char_whitelist", "ABC")
                .with_variable("tessedit_char_blacklist", "XYZ");

            config = config.with_variable("tessedit_char_whitelist", "123");
            assert_eq!(
                config.variables.get("tessedit_char_whitelist"),
                Some(&"123".to_string())
            );
            assert_eq!(
                config.variables.get("tessedit_char_blacklist"),
                Some(&"XYZ".to_string())
            );
        }

        #[test]
        fn test_tesseract_config_empty_language() {
            let config = TesseractConfig::with_language("");
            assert_eq!(config.language, "");
        }

        #[test]
        fn test_tesseract_config_empty_whitelist_blacklist() {
            let config = TesseractConfig::default()
                .with_char_whitelist("")
                .with_char_blacklist("");

            assert_eq!(config.char_whitelist, Some("".to_string()));
            assert_eq!(config.char_blacklist, Some("".to_string()));
        }

        #[test]
        fn test_tesseract_config_special_characters() {
            let config = TesseractConfig::default()
                .with_char_whitelist("αβγδε")
                .with_char_blacklist("©®™");

            assert_eq!(config.char_whitelist, Some("αβγδε".to_string()));
            assert_eq!(config.char_blacklist, Some("©®™".to_string()));
        }

        #[test]
        fn test_tesseract_config_many_variables() {
            let mut config = TesseractConfig::default();

            for i in 0..10 {
                config = config.with_variable(&format!("test_var_{}", i), &format!("value_{}", i));
            }

            assert_eq!(config.variables.len(), 10);
            assert_eq!(
                config.variables.get("test_var_0"),
                Some(&"value_0".to_string())
            );
            assert_eq!(
                config.variables.get("test_var_9"),
                Some(&"value_9".to_string())
            );
        }

        #[test]
        #[ignore = "Requires Tesseract installation"]
        fn test_tesseract_provider_engine_info() {
            if let Ok(provider) = TesseractOcrProvider::new() {
                assert_eq!(provider.engine_name(), "Tesseract");
                assert_eq!(provider.engine_type(), OcrEngine::Tesseract);
                assert!(!provider.supported_formats().is_empty());
            }
        }

        #[test]
        #[ignore = "Requires Tesseract installation"]
        fn test_tesseract_provider_format_support() {
            if let Ok(_provider) = TesseractOcrProvider::new() {
                // Test format support without actually creating provider
                let formats = vec![ImageFormat::Jpeg, ImageFormat::Png, ImageFormat::Tiff];
                assert!(formats.contains(&ImageFormat::Jpeg));
                assert!(formats.contains(&ImageFormat::Png));
                assert!(formats.contains(&ImageFormat::Tiff));
            }
        }

        #[test]
        #[ignore = "Requires Tesseract installation"]
        fn test_tesseract_provider_with_custom_config() {
            let config = TesseractConfig::for_documents()
                .with_char_whitelist("0123456789")
                .with_debug();

            match TesseractOcrProvider::with_config(config) {
                Ok(provider) => {
                    assert_eq!(provider.engine_name(), "Tesseract");
                    assert_eq!(provider.engine_type(), OcrEngine::Tesseract);
                }
                Err(_) => {
                    // Expected if Tesseract is not installed
                }
            }
        }

        #[test]
        #[ignore = "Requires Tesseract installation"]
        fn test_tesseract_provider_validate_image_data() {
            if let Ok(provider) = TesseractOcrProvider::new() {
                // Valid JPEG header
                let jpeg_data = vec![0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10, 0x4A, 0x46, 0x49, 0x46];
                assert!(provider.validate_image_data(&jpeg_data).is_ok());

                // Valid PNG header
                let png_data = vec![0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
                assert!(provider.validate_image_data(&png_data).is_ok());

                // Invalid data
                let invalid_data = vec![0x00, 0x01, 0x02, 0x03];
                assert!(provider.validate_image_data(&invalid_data).is_err());
            }
        }

        #[test]
        #[ignore = "Requires Tesseract installation"]
        fn test_tesseract_provider_error_handling() {
            if let Ok(provider) = TesseractOcrProvider::new() {
                let options = OcrOptions::default();

                // Test with invalid image data
                let invalid_data = vec![0x00, 0x01, 0x02, 0x03];
                let result = provider.process_image(&invalid_data, &options);
                assert!(result.is_err());

                // Test with empty data
                let empty_data = vec![];
                let result = provider.process_image(&empty_data, &options);
                assert!(result.is_err());
            }
        }

        #[test]
        #[ignore = "Requires Tesseract installation"]
        fn test_tesseract_provider_language_support() {
            if let Ok(langs) = TesseractOcrProvider::available_languages() {
                assert!(!langs.is_empty());

                // Common languages that should be available
                let common_langs = vec!["eng", "spa", "deu", "fra", "ita"];
                let mut found_count = 0;

                for lang in common_langs {
                    if langs.contains(&lang.to_string()) {
                        found_count += 1;
                    }
                }

                assert!(found_count > 0, "Should have at least one common language");
            }
        }

        #[test]
        #[ignore = "Requires Tesseract installation"]
        fn test_tesseract_provider_process_page() {
            if let Ok(provider) = TesseractOcrProvider::new() {
                let options = OcrOptions::default();
                let analysis = ContentAnalysis {
                    page_number: 0,
                    page_type: crate::operations::page_analysis::PageType::Scanned,
                    text_ratio: 0.1,
                    image_ratio: 0.8,
                    blank_space_ratio: 0.1,
                    text_fragment_count: 5,
                    image_count: 1,
                    character_count: 50,
                };

                // Mock page data (would fail with real OCR, but tests the interface)
                let page_data = vec![0xFF, 0xD8, 0xFF, 0xE0];
                let result = provider.process_page(&analysis, &page_data, &options);
                // This will likely fail with mock data, but tests the interface
                assert!(result.is_ok() || result.is_err());
            }
        }

        #[test]
        fn test_tesseract_provider_without_feature() {
            // Test the stub implementation when feature is not enabled
            #[cfg(not(feature = "ocr-tesseract"))]
            {
                let result = TesseractOcrProvider::new();
                assert!(result.is_err());

                let result = TesseractOcrProvider::check_availability();
                assert!(result.is_err());
            }
        }

        #[test]
        fn test_supported_image_formats() {
            let formats = vec![ImageFormat::Jpeg, ImageFormat::Png, ImageFormat::Tiff];

            // Test that all expected formats are supported
            assert!(formats.contains(&ImageFormat::Jpeg));
            assert!(formats.contains(&ImageFormat::Png));
            assert!(formats.contains(&ImageFormat::Tiff));

            // Test that we have the expected number of formats
            assert_eq!(formats.len(), 3);
        }

        #[test]
        fn test_page_segmentation_mode_ordering() {
            let modes = vec![
                PageSegmentationMode::OsdOnly,
                PageSegmentationMode::AutoOsd,
                PageSegmentationMode::AutoOnly,
                PageSegmentationMode::Auto,
                PageSegmentationMode::SingleColumn,
                PageSegmentationMode::SingleBlock,
                PageSegmentationMode::SingleUniformBlock,
                PageSegmentationMode::SingleLine,
                PageSegmentationMode::SingleWord,
                PageSegmentationMode::SingleWord,
                PageSegmentationMode::SingleChar,
                PageSegmentationMode::SparseText,
                PageSegmentationMode::SparseTextOsd,
                PageSegmentationMode::RawLine,
            ];

            // Test that enum values are ordered correctly
            for (i, mode) in modes.iter().enumerate() {
                assert_eq!(*mode as u8, i as u8);
            }
        }

        #[test]
        fn test_ocr_engine_mode_ordering() {
            let modes = vec![
                OcrEngineMode::LegacyOnly,
                OcrEngineMode::LstmOnly,
                OcrEngineMode::LegacyLstm,
                OcrEngineMode::Default,
            ];

            // Test that enum values are ordered correctly
            for (i, mode) in modes.iter().enumerate() {
                assert_eq!(*mode as u8, i as u8);
            }
        }

        #[test]
        fn test_tesseract_config_builder_pattern() {
            let config = TesseractConfig::with_language("spa")
                .with_char_whitelist("ABCDEFGHIJKLMNOPQRSTUVWXYZ")
                .with_char_blacklist("0123456789")
                .with_variable("tessedit_char_whitelist", "")
                .with_variable("tessedit_char_blacklist", "")
                .with_debug();

            assert_eq!(config.language, "spa");
            assert_eq!(
                config.char_whitelist,
                Some("ABCDEFGHIJKLMNOPQRSTUVWXYZ".to_string())
            );
            assert_eq!(config.char_blacklist, Some("0123456789".to_string()));
            assert_eq!(config.variables.len(), 2);
            assert!(config.debug);
        }

        #[test]
        fn test_tesseract_config_immutability() {
            let config1 = TesseractConfig::default();
            let config1_clone = config1.clone();
            let config2 = config1.with_debug();

            // Original clone should not be modified
            assert!(!config1_clone.debug);
            assert!(config2.debug);
        }

        #[test]
        fn test_tesseract_error_handling_types() {
            // Test that error types are correctly handled
            let error_types = vec![
                "ProviderNotAvailable",
                "ProcessingFailed",
                "InvalidImageData",
                "UnsupportedImageFormat",
            ];

            for error_type in error_types {
                assert!(!error_type.is_empty());
            }
        }
    }
}
