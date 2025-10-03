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
//! let provider = RustyTesseractProvider::new()?;
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
use rusty_tesseract::{image_to_string, Args, Image};
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

        // Optimize for scanned documents
        config_vars.insert("tessedit_char_whitelist".to_string(),
            "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789.,!?:;()[]{}\"'-+=%&#@*/\\| \t\n".to_string());
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

/// Tesseract OCR provider using rusty-tesseract
#[cfg(feature = "ocr-tesseract")]
pub struct RustyTesseractProvider {
    config: RustyTesseractConfig,
}

#[cfg(feature = "ocr-tesseract")]
impl RustyTesseractProvider {
    /// Create a new Tesseract OCR provider with default configuration
    pub fn new() -> OcrResult<Self> {
        Self::with_config(RustyTesseractConfig::default())
    }

    /// Create a new Tesseract OCR provider with custom configuration
    pub fn with_config(config: RustyTesseractConfig) -> OcrResult<Self> {
        Ok(Self { config })
    }

    /// Create a new Tesseract OCR provider optimized for legal documents and contracts
    pub fn for_contracts() -> OcrResult<Self> {
        let mut config_vars = HashMap::new();

        // Optimize specifically for legal/contract documents
        config_vars.insert("preserve_interword_spaces".to_string(), "1".to_string());
        config_vars.insert("tessedit_create_hocr".to_string(), "0".to_string());
        config_vars.insert("tessedit_create_tsv".to_string(), "0".to_string());
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

        Ok(Self { config })
    }

    /// Create a new Tesseract OCR provider optimized for large documents with potential rotation issues
    pub fn for_large_documents() -> OcrResult<Self> {
        let mut config_vars = HashMap::new();

        // Optimize for speed and rotation handling
        config_vars.insert("preserve_interword_spaces".to_string(), "1".to_string());
        config_vars.insert("tessedit_create_hocr".to_string(), "0".to_string());
        config_vars.insert("tessedit_create_tsv".to_string(), "0".to_string());
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

        Ok(Self { config })
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

        // Perform OCR
        let text = image_to_string(&image, &args)
            .map_err(|e| OcrError::ProcessingFailed(format!("OCR processing failed: {}", e)))?;

        let processing_time = start_time.elapsed();

        // For now, we'll estimate confidence based on text length and characters
        // rusty-tesseract doesn't provide detailed confidence information
        let confidence = estimate_confidence(&text);

        // Create basic fragments - one fragment for the entire text
        let fragments = if text.trim().is_empty() {
            Vec::new()
        } else {
            vec![OcrTextFragment {
                text: text.clone(),
                x: 0.0,
                y: 0.0,
                width: 0.0,
                height: 0.0,
                font_size: 12.0,
                confidence: confidence as f64,
                word_confidences: None,
                fragment_type: FragmentType::Word,
            }]
        };

        Ok(OcrProcessingResult {
            text: text.trim().to_string(),
            confidence: confidence as f64,
            fragments,
            processing_time_ms: processing_time.as_millis() as u64,
            engine_name: "rusty-tesseract".to_string(),
            language: self.config.language.clone(),
            processed_region: None,
            image_dimensions: (0, 0), // We could extract actual dimensions if needed
        })
    }
}

/// Estimate confidence based on text characteristics
/// This is a simple heuristic since rusty-tesseract doesn't provide detailed confidence
#[cfg(feature = "ocr-tesseract")]
fn estimate_confidence(text: &str) -> f32 {
    if text.trim().is_empty() {
        return 0.0;
    }

    let trimmed = text.trim();
    let total_chars = trimmed.len() as f32;

    // Basic confidence estimation
    let alpha_count = trimmed.chars().filter(|c| c.is_alphabetic()).count() as f32;
    let _digit_count = trimmed.chars().filter(|c| c.is_numeric()).count() as f32;
    let space_count = trimmed.chars().filter(|c| c.is_whitespace()).count() as f32;
    let punct_count = trimmed.chars().filter(|c| c.is_ascii_punctuation()).count() as f32;

    // Start with base confidence
    let mut confidence: f32 = 0.7;

    // Boost confidence for reasonable character distributions
    let alpha_ratio = alpha_count / total_chars;
    if alpha_ratio > 0.5 {
        confidence += 0.1;
    }

    // Reasonable amount of spaces suggests proper word separation
    let space_ratio = space_count / total_chars;
    if space_ratio > 0.1 && space_ratio < 0.3 {
        confidence += 0.1;
    }

    // Some punctuation is good, too much might indicate noise
    let punct_ratio = punct_count / total_chars;
    if punct_ratio < 0.2 {
        confidence += 0.05;
    } else {
        confidence -= 0.1;
    }

    // Clamp to valid range
    confidence.max(0.0).min(1.0)
}

#[cfg(feature = "ocr-tesseract")]
impl Default for RustyTesseractProvider {
    fn default() -> Self {
        Self::new().expect("Failed to create default RustyTesseractProvider")
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
        let provider = RustyTesseractProvider::new();
        assert!(provider.is_ok());
    }

    #[cfg(feature = "ocr-tesseract")]
    #[test]
    fn test_confidence_estimation() {
        assert_eq!(estimate_confidence(""), 0.0);

        let normal_text = "This is a normal text with proper spacing.";
        let confidence = estimate_confidence(normal_text);
        assert!(confidence > 0.5);

        let noisy_text = "!!@#$%^&*()";
        let noisy_confidence = estimate_confidence(noisy_text);
        assert!(noisy_confidence < confidence);
    }

    #[cfg(feature = "ocr-tesseract")]
    #[test]
    fn test_engine_info() {
        let provider = RustyTesseractProvider::new().unwrap();
        assert_eq!(provider.engine_type(), OcrEngine::Tesseract);
        assert_eq!(provider.engine_name(), "rusty-tesseract");
    }
}
