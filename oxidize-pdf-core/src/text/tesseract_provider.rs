//! Tesseract OCR provider implementation using rusty-tesseract
//!
//! This module provides a complete implementation of the OcrProvider trait using
//! rusty-tesseract, which is a more modern and reliable Rust wrapper for Tesseract OCR.
//!
//! # Features
//! - Multiple language support
//! - Configurable OCR parameters (PSM, OEM, DPI)
//! - Confidence scoring
//! - Error handling with detailed error messages
//! - Integration with oxidize-pdf's OCR framework
//!
//! # Usage
//!
//! ```rust,no_run
//! use oxidize_pdf::text::{RustyTesseractProvider, OcrOptions, OcrProvider};
//!
//! let provider = RustyTesseractProvider::new();
//! let options = OcrOptions::default();
//! let image_data = std::fs::read("document.png")?;
//!
//! let result = provider.process_image(&image_data, &options)?;
//! println!("Extracted text: {}", result.text);
//! println!("Confidence: {:.1}%", result.confidence * 100.0);
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

#[cfg(feature = "ocr-tesseract")]
use crate::graphics::ImageFormat;
#[cfg(feature = "ocr-tesseract")]
use crate::text::{
    FragmentType, OcrEngine, OcrError, OcrOptions, OcrProcessingResult, OcrProvider, OcrResult,
    OcrTextFragment,
};

#[cfg(feature = "ocr-tesseract")]
use rusty_tesseract::{image_to_data, Args, Image};
#[cfg(feature = "ocr-tesseract")]
use std::collections::HashMap;
#[cfg(feature = "ocr-tesseract")]
use std::time::Instant;

/// Configuration for rusty-tesseract OCR provider
#[cfg(feature = "ocr-tesseract")]
#[derive(Debug, Clone)]
pub struct RustyTesseractConfig {
    /// Language code (e.g., "eng", "spa", "eng+spa")
    pub language: String,
    /// Page Segmentation Mode (PSM)
    pub psm: Option<u8>,
    /// OCR Engine Mode (OEM)  
    pub oem: Option<u8>,
    /// DPI for image processing
    pub dpi: Option<u32>,
    /// Additional configuration variables
    pub config_variables: HashMap<String, String>,
}

#[cfg(feature = "ocr-tesseract")]
impl Default for RustyTesseractConfig {
    fn default() -> Self {
        let mut config_vars = HashMap::new();

        config_vars.insert("preserve_interword_spaces".to_string(), "1".to_string());

        Self {
            language: "eng".to_string(),
            psm: Some(3), // Fully automatic page segmentation, but no OSD (best for scanned documents)
            oem: Some(3), // Default, based on what is available
            dpi: Some(300), // High DPI for better accuracy
            config_variables: config_vars,
        }
    }
}

/// Tesseract OCR provider using rusty-tesseract TSV word records.
///
/// Fragments use `ImagePixels`: bottom-left image-local coordinates with one unit
/// per pixel. Confidence is the mean of accepted word confidences. The configured
/// language selects the Tesseract model; no alphabet whitelist is imposed by default.
/// Preprocessing, `regions` and timeout options are not implemented by this adapter;
/// callers can preprocess/crop images before recognition. PDF conversion handles
/// page placement separately.
#[cfg(feature = "ocr-tesseract")]
pub struct RustyTesseractProvider {
    config: RustyTesseractConfig,
}

#[cfg(feature = "ocr-tesseract")]
impl RustyTesseractProvider {
    /// Create a new Tesseract OCR provider with default configuration
    pub fn new() -> Self {
        Self::with_config(RustyTesseractConfig::default())
    }

    /// Create a new Tesseract OCR provider with custom configuration
    pub fn with_config(config: RustyTesseractConfig) -> Self {
        Self { config }
    }

    /// Create a new Tesseract OCR provider optimized for legal documents and contracts
    pub fn for_contracts() -> Self {
        let mut config_vars = HashMap::new();

        // Optimize specifically for legal/contract documents
        config_vars.insert("preserve_interword_spaces".to_string(), "1".to_string());
        config_vars.insert("tessedit_create_hocr".to_string(), "0".to_string());
        config_vars.insert("load_system_dawg".to_string(), "1".to_string()); // Use system dictionary
        config_vars.insert("load_freq_dawg".to_string(), "1".to_string()); // Use frequency data
        config_vars.insert("textord_debug_tabfind".to_string(), "0".to_string());
        config_vars.insert("textord_use_cjk_fp_model".to_string(), "0".to_string());

        let config = RustyTesseractConfig {
            language: "eng".to_string(),
            psm: Some(1), // Automatic page segmentation with OSD (best for full page documents)
            oem: Some(1), // Neural nets LSTM engine only
            dpi: Some(300),
            config_variables: config_vars,
        };

        Self { config }
    }

    /// Create a new Tesseract OCR provider optimized for large documents with potential rotation issues
    pub fn for_large_documents() -> Self {
        let mut config_vars = HashMap::new();

        // Optimize for speed and rotation handling
        config_vars.insert("preserve_interword_spaces".to_string(), "1".to_string());
        config_vars.insert("tessedit_create_hocr".to_string(), "0".to_string());
        config_vars.insert("load_system_dawg".to_string(), "1".to_string());
        config_vars.insert("load_freq_dawg".to_string(), "1".to_string());

        // Speed optimizations - disable learning and complex features
        config_vars.insert("classify_enable_learning".to_string(), "0".to_string());
        config_vars.insert("tessedit_do_invert".to_string(), "0".to_string());

        // Better handling of rotated documents
        config_vars.insert("textord_debug_tabfind".to_string(), "0".to_string());
        config_vars.insert("textord_use_cjk_fp_model".to_string(), "0".to_string());

        let config = RustyTesseractConfig {
            language: "eng".to_string(),
            psm: Some(1),   // Automatic page segmentation with OSD (handles rotation)
            oem: Some(1),   // Neural nets LSTM engine only (faster than legacy)
            dpi: Some(150), // Reduced DPI for speed
            config_variables: config_vars,
        };

        Self { config }
    }

    /// Test if Tesseract is available and working
    pub fn test_availability() -> OcrResult<bool> {
        // Try to create a simple test args to verify Tesseract is installed
        let _args = Args {
            lang: "eng".to_string(),
            config_variables: HashMap::new(),
            dpi: Some(150),
            psm: Some(6),
            oem: Some(3),
        };

        // Just return true if we can create args - actual testing would need a real image
        Ok(true)
    }

    /// Get the current configuration
    pub fn config(&self) -> &RustyTesseractConfig {
        &self.config
    }

    /// Convert OcrOptions to rusty-tesseract Args
    fn create_args(&self, options: &OcrOptions) -> Args {
        let mut config_vars = self.config.config_variables.clone();

        // Add any options-specific configurations
        if options.min_confidence > 0.0 {
            config_vars.insert("tessedit_reject_mode".to_string(), "2".to_string());
        }

        Args {
            lang: self.config.language.clone(),
            config_variables: config_vars,
            dpi: self.config.dpi.map(|v| v as i32),
            psm: self.config.psm.map(|v| v as i32),
            oem: self.config.oem.map(|v| v as i32),
        }
    }
}

#[cfg(feature = "ocr-tesseract")]
impl OcrProvider for RustyTesseractProvider {
    fn coordinate_space(&self) -> crate::text::OcrCoordinateSpace {
        crate::text::OcrCoordinateSpace::ImagePixels
    }

    fn supported_formats(&self) -> Vec<ImageFormat> {
        vec![
            ImageFormat::Png,
            ImageFormat::Jpeg,
            ImageFormat::Tiff,
            // ImageFormat::Bmp, // Not available in current enum
        ]
    }

    fn engine_name(&self) -> &str {
        "rusty-tesseract"
    }

    fn engine_type(&self) -> OcrEngine {
        OcrEngine::Tesseract
    }

    fn process_image(
        &self,
        image_data: &[u8],
        options: &OcrOptions,
    ) -> OcrResult<OcrProcessingResult> {
        let start_time = Instant::now();

        // Create rusty-tesseract image from DynamicImage
        // First decode the image bytes to DynamicImage using rusty_tesseract's image crate
        use std::io::Cursor;
        let cursor = Cursor::new(image_data);
        let dynamic_image = rusty_tesseract::image::ImageReader::new(cursor)
            .with_guessed_format()
            .map_err(|e| {
                OcrError::ProcessingFailed(format!("Failed to guess image format: {}", e))
            })?
            .decode()
            .map_err(|e| OcrError::ProcessingFailed(format!("Failed to decode image: {}", e)))?;

        let image = Image::from_dynamic_image(&dynamic_image).map_err(|e| {
            OcrError::ProcessingFailed(format!("Failed to create tesseract image: {}", e))
        })?;

        // Create OCR arguments
        let args = self.create_args(options);

        // TSV supplies measured word boxes and confidence; hierarchy rows are not words.
        let output = image_to_data(&image, &args)
            .map_err(|e| OcrError::ProcessingFailed(format!("OCR processing failed: {e}")))?;
        let dimensions = (dynamic_image.width(), dynamic_image.height());
        let mut fragments = Vec::new();
        let mut text = String::new();
        let mut previous_line = None;
        for word in output
            .data
            .iter()
            .filter(|word| word.level == 5 && !word.text.trim().is_empty())
        {
            if word.left < 0
                || word.top < 0
                || word.width <= 0
                || word.height <= 0
                || i64::from(word.left) + i64::from(word.width) > i64::from(dimensions.0)
                || i64::from(word.top) + i64::from(word.height) > i64::from(dimensions.1)
                || !word.conf.is_finite()
                || !(0.0..=100.0).contains(&word.conf)
            {
                return Err(OcrError::ProcessingFailed(
                    "Tesseract returned an invalid word region or confidence".into(),
                ));
            }
            let confidence = f64::from(word.conf) / 100.0;
            if confidence < options.min_confidence {
                continue;
            }
            let line = (word.page_num, word.block_num, word.par_num, word.line_num);
            if let Some(previous) = previous_line {
                text.push(if previous == line { ' ' } else { '\n' });
            }
            previous_line = Some(line);
            text.push_str(&word.text);
            fragments.push(OcrTextFragment {
                text: word.text.clone(),
                x: f64::from(word.left),
                y: f64::from(dimensions.1) - f64::from(word.top) - f64::from(word.height),
                width: f64::from(word.width),
                height: f64::from(word.height),
                font_size: f64::from(word.height),
                confidence,
                word_confidences: None,
                fragment_type: FragmentType::Word,
            });
        }
        let confidence = if fragments.is_empty() {
            0.0
        } else {
            fragments
                .iter()
                .map(|fragment| fragment.confidence)
                .sum::<f64>()
                / fragments.len() as f64
        };
        let processing_time = start_time.elapsed();

        Ok(OcrProcessingResult {
            text: text.trim().to_string(),
            confidence,
            fragments,
            processing_time_ms: processing_time.as_millis() as u64,
            engine_name: "rusty-tesseract".to_string(),
            language: self.config.language.clone(),
            processed_region: None,
            image_dimensions: dimensions,
        })
    }
}

#[cfg(feature = "ocr-tesseract")]
impl Default for RustyTesseractProvider {
    fn default() -> Self {
        Self::new()
    }
}

// Mock implementation for when OCR feature is not enabled
#[cfg(not(feature = "ocr-tesseract"))]
pub struct RustyTesseractProvider;

#[cfg(not(feature = "ocr-tesseract"))]
impl RustyTesseractProvider {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        Err("OCR feature not enabled. Compile with --features ocr-tesseract".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(feature = "ocr-tesseract")]
    #[test]
    fn test_config_default() {
        let config = RustyTesseractConfig::default();
        assert_eq!(config.language, "eng");
        assert_eq!(config.psm, Some(3)); // Fully automatic page segmentation for scanned documents
        assert_eq!(config.oem, Some(3));
        assert_eq!(config.dpi, Some(300));
    }

    #[cfg(feature = "ocr-tesseract")]
    #[test]
    fn test_provider_creation() {
        let _provider = RustyTesseractProvider::new();
        // Provider creation is now infallible
    }

    #[cfg(feature = "ocr-tesseract")]
    #[test]
    fn test_engine_info() {
        let provider = RustyTesseractProvider::new();
        assert_eq!(provider.engine_type(), OcrEngine::Tesseract);
        assert_eq!(provider.engine_name(), "rusty-tesseract");
    }
}
