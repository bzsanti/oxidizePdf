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

/// A rectangular region for selective OCR processing
#[derive(Debug, Clone, PartialEq)]
pub struct OcrRegion {
    /// X coordinate of the top-left corner (pixels)
    pub x: u32,

    /// Y coordinate of the top-left corner (pixels)  
    pub y: u32,

    /// Width of the region (pixels)
    pub width: u32,

    /// Height of the region (pixels)
    pub height: u32,

    /// Optional label for this region (e.g., "header", "table", "paragraph")
    pub label: Option<String>,
}

impl OcrRegion {
    /// Create a new OCR region
    pub fn new(x: u32, y: u32, width: u32, height: u32) -> Self {
        Self {
            x,
            y,
            width,
            height,
            label: None,
        }
    }

    /// Create a new OCR region with a label
    pub fn with_label(x: u32, y: u32, width: u32, height: u32, label: impl Into<String>) -> Self {
        Self {
            x,
            y,
            width,
            height,
            label: Some(label.into()),
        }
    }

    /// Check if this region contains a point
    pub fn contains_point(&self, x: u32, y: u32) -> bool {
        x >= self.x && x < self.x + self.width && y >= self.y && y < self.y + self.height
    }

    /// Check if this region overlaps with another region
    pub fn overlaps_with(&self, other: &OcrRegion) -> bool {
        !(self.x + self.width <= other.x
            || other.x + other.width <= self.x
            || self.y + self.height <= other.y
            || other.y + other.height <= self.y)
    }
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

    /// Specific regions to process (None = process entire image)
    pub regions: Option<Vec<OcrRegion>>,

    /// Whether to save extracted images for debug purposes
    pub debug_output: bool,
}

impl Default for OcrOptions {
    fn default() -> Self {
        Self {
            language: "en".to_string(),
            min_confidence: 0.6,
            preserve_layout: true,
            preprocessing: ImagePreprocessing::default(),
            engine_options: std::collections::HashMap::new(),
            timeout_seconds: 60, // Aumentado para documentos complejos
            regions: None,
            debug_output: false,
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

/// Word-level confidence information for detailed OCR analysis
#[derive(Debug, Clone)]
pub struct WordConfidence {
    /// The word text
    pub word: String,

    /// Confidence score for this specific word (0.0 to 1.0)
    pub confidence: f64,

    /// X position of the word within the fragment (relative to fragment start)
    pub x_offset: f64,

    /// Width of the word in points
    pub width: f64,

    /// Optional character-level confidences (for ultimate granularity)
    pub character_confidences: Option<Vec<CharacterConfidence>>,
}

impl WordConfidence {
    /// Create a new word confidence
    pub fn new(word: String, confidence: f64, x_offset: f64, width: f64) -> Self {
        Self {
            word,
            confidence,
            x_offset,
            width,
            character_confidences: None,
        }
    }

    /// Create a word confidence with character-level details
    pub fn with_characters(
        word: String,
        confidence: f64,
        x_offset: f64,
        width: f64,
        character_confidences: Vec<CharacterConfidence>,
    ) -> Self {
        Self {
            word,
            confidence,
            x_offset,
            width,
            character_confidences: Some(character_confidences),
        }
    }

    /// Get the average character confidence if available
    pub fn average_character_confidence(&self) -> Option<f64> {
        self.character_confidences.as_ref().map(|chars| {
            let sum: f64 = chars.iter().map(|c| c.confidence).sum();
            sum / chars.len() as f64
        })
    }

    /// Check if this word has low confidence (below threshold)
    pub fn is_low_confidence(&self, threshold: f64) -> bool {
        self.confidence < threshold
    }
}

/// Character-level confidence information for ultimate OCR granularity
#[derive(Debug, Clone)]
pub struct CharacterConfidence {
    /// The character
    pub character: char,

    /// Confidence score for this character (0.0 to 1.0)  
    pub confidence: f64,

    /// X position relative to word start
    pub x_offset: f64,

    /// Character width in points
    pub width: f64,
}

impl CharacterConfidence {
    /// Create a new character confidence
    pub fn new(character: char, confidence: f64, x_offset: f64, width: f64) -> Self {
        Self {
            character,
            confidence,
            x_offset,
            width,
        }
    }
}

/// Candidate for OCR post-processing correction
#[derive(Debug, Clone)]
pub struct CorrectionCandidate {
    /// The original word with low confidence or errors
    pub word: String,

    /// Original confidence score
    pub confidence: f64,

    /// Position within the text fragment
    pub position_in_fragment: usize,

    /// Suggested corrections ranked by likelihood
    pub suggested_corrections: Vec<CorrectionSuggestion>,

    /// Reason why this word needs correction
    pub correction_reason: CorrectionReason,
}

/// A suggested correction for an OCR error
#[derive(Debug, Clone)]
pub struct CorrectionSuggestion {
    /// The corrected word
    pub corrected_word: String,

    /// Confidence in this correction (0.0 to 1.0)
    pub correction_confidence: f64,

    /// Type of correction applied
    pub correction_type: CorrectionType,

    /// Explanation of why this correction was suggested
    pub explanation: Option<String>,
}

/// Reasons why a word might need correction
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CorrectionReason {
    /// Word has low OCR confidence
    LowConfidence,

    /// Word contains common OCR confusion patterns
    ConfusionPattern,

    /// Word not found in dictionary
    NotInDictionary,

    /// Word doesn't fit context
    ContextualError,

    /// Word has suspicious character combinations
    SuspiciousPattern,
}

/// Types of corrections that can be applied
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum CorrectionType {
    /// Character substitution (e.g., "0" -> "O")
    CharacterSubstitution,

    /// Dictionary lookup and replacement
    DictionaryCorrection,

    /// Contextual correction based on surrounding words
    ContextualCorrection,

    /// Pattern-based correction (e.g., "rn" -> "m")
    PatternCorrection,

    /// Manual review suggested
    ManualReview,
}

/// OCR post-processor for automatic text correction
#[derive(Debug, Clone)]
pub struct OcrPostProcessor {
    /// Common OCR character confusions
    pub character_corrections: std::collections::HashMap<char, Vec<char>>,

    /// Dictionary of valid words (optional)
    pub dictionary: Option<std::collections::HashSet<String>>,

    /// Common pattern corrections
    pub pattern_corrections: std::collections::HashMap<String, String>,

    /// Confidence threshold for correction
    pub correction_threshold: f64,

    /// Maximum edit distance for corrections
    pub max_edit_distance: usize,
}

impl OcrPostProcessor {
    /// Create a new post-processor with common OCR corrections
    pub fn new() -> Self {
        let mut character_corrections = std::collections::HashMap::new();

        // Common OCR character confusions
        character_corrections.insert('0', vec!['O', 'o', 'Q']);
        character_corrections.insert('O', vec!['0', 'Q', 'o']);
        character_corrections.insert('1', vec!['l', 'I', '|']);
        character_corrections.insert('l', vec!['1', 'I', '|']);
        character_corrections.insert('I', vec!['1', 'l', '|']);
        character_corrections.insert('S', vec!['5', '$']);
        character_corrections.insert('5', vec!['S', '$']);
        character_corrections.insert('2', vec!['Z', 'z']);
        character_corrections.insert('Z', vec!['2', 'z']);

        let mut pattern_corrections = std::collections::HashMap::new();
        pattern_corrections.insert("rn".to_string(), "m".to_string());
        pattern_corrections.insert("cl".to_string(), "d".to_string());
        pattern_corrections.insert("fi".to_string(), "fi".to_string()); // ligature
        pattern_corrections.insert("fl".to_string(), "fl".to_string()); // ligature

        Self {
            character_corrections,
            dictionary: None,
            pattern_corrections,
            correction_threshold: 0.7,
            max_edit_distance: 2,
        }
    }

    /// Add a dictionary for word validation
    pub fn with_dictionary(mut self, dictionary: std::collections::HashSet<String>) -> Self {
        self.dictionary = Some(dictionary);
        self
    }

    /// Process a fragment and suggest corrections
    pub fn process_fragment(&self, fragment: &OcrTextFragment) -> Vec<CorrectionCandidate> {
        let mut candidates = fragment.get_correction_candidates(self.correction_threshold);

        // Enhance candidates with suggestions
        for candidate in &mut candidates {
            candidate.suggested_corrections = self.generate_suggestions(&candidate.word);
        }

        candidates
    }

    /// Generate correction suggestions for a word
    pub fn generate_suggestions(&self, word: &str) -> Vec<CorrectionSuggestion> {
        let mut suggestions = Vec::new();

        // Character substitution corrections
        suggestions.extend(self.character_substitution_corrections(word));

        // Pattern-based corrections
        suggestions.extend(self.pattern_corrections(word));

        // Dictionary corrections (if available)
        if let Some(dict) = &self.dictionary {
            suggestions.extend(self.dictionary_corrections(word, dict));
        }

        // Sort by confidence and limit results
        suggestions.sort_by(|a, b| {
            b.correction_confidence
                .partial_cmp(&a.correction_confidence)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        suggestions.truncate(5); // Limit to top 5 suggestions

        suggestions
    }

    /// Generate character substitution corrections
    fn character_substitution_corrections(&self, word: &str) -> Vec<CorrectionSuggestion> {
        let mut suggestions = Vec::new();
        let chars: Vec<char> = word.chars().collect();

        for (i, &ch) in chars.iter().enumerate() {
            if let Some(alternatives) = self.character_corrections.get(&ch) {
                for &alt_ch in alternatives {
                    let mut corrected_chars = chars.clone();
                    corrected_chars[i] = alt_ch;
                    let corrected_word: String = corrected_chars.into_iter().collect();

                    suggestions.push(CorrectionSuggestion {
                        corrected_word,
                        correction_confidence: 0.8,
                        correction_type: CorrectionType::CharacterSubstitution,
                        explanation: Some(format!("'{}' -> '{}' substitution", ch, alt_ch)),
                    });
                }
            }
        }

        suggestions
    }

    /// Generate pattern-based corrections
    fn pattern_corrections(&self, word: &str) -> Vec<CorrectionSuggestion> {
        let mut suggestions = Vec::new();

        for (pattern, replacement) in &self.pattern_corrections {
            if word.contains(pattern) {
                let corrected_word = word.replace(pattern, replacement);
                suggestions.push(CorrectionSuggestion {
                    corrected_word,
                    correction_confidence: 0.85,
                    correction_type: CorrectionType::PatternCorrection,
                    explanation: Some(format!(
                        "Pattern '{}' -> '{}' correction",
                        pattern, replacement
                    )),
                });
            }
        }

        suggestions
    }

    /// Generate dictionary-based corrections
    fn dictionary_corrections(
        &self,
        word: &str,
        dictionary: &std::collections::HashSet<String>,
    ) -> Vec<CorrectionSuggestion> {
        let mut suggestions = Vec::new();

        // Check if word is already valid
        if dictionary.contains(word) {
            return suggestions;
        }

        // Find similar words using simple edit distance
        for dict_word in dictionary {
            if self.edit_distance(word, dict_word) <= self.max_edit_distance {
                let confidence = 1.0
                    - (self.edit_distance(word, dict_word) as f64
                        / word.len().max(dict_word.len()) as f64);
                suggestions.push(CorrectionSuggestion {
                    corrected_word: dict_word.clone(),
                    correction_confidence: confidence * 0.9, // Slightly lower than pattern corrections
                    correction_type: CorrectionType::DictionaryCorrection,
                    explanation: Some(format!(
                        "Dictionary match with edit distance {}",
                        self.edit_distance(word, dict_word)
                    )),
                });
            }
        }

        suggestions
    }

    /// Calculate simple edit distance (Levenshtein distance)
    fn edit_distance(&self, s1: &str, s2: &str) -> usize {
        let len1 = s1.len();
        let len2 = s2.len();

        let mut dp = vec![vec![0; len2 + 1]; len1 + 1];

        #[allow(clippy::needless_range_loop)]
        for i in 0..=len1 {
            dp[i][0] = i;
        }
        for j in 0..=len2 {
            dp[0][j] = j;
        }

        let s1_chars: Vec<char> = s1.chars().collect();
        let s2_chars: Vec<char> = s2.chars().collect();

        for i in 1..=len1 {
            for j in 1..=len2 {
                if s1_chars[i - 1] == s2_chars[j - 1] {
                    dp[i][j] = dp[i - 1][j - 1];
                } else {
                    dp[i][j] = 1 + dp[i - 1][j].min(dp[i][j - 1]).min(dp[i - 1][j - 1]);
                }
            }
        }

        dp[len1][len2]
    }
}

impl Default for OcrPostProcessor {
    fn default() -> Self {
        Self::new()
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

    /// Word-level confidence scores (optional, for advanced OCR engines)
    pub word_confidences: Option<Vec<WordConfidence>>,

    /// Font size estimation (points)
    pub font_size: f64,

    /// Whether this fragment is part of a word or line
    pub fragment_type: FragmentType,
}

impl OcrTextFragment {
    /// Create a new OCR text fragment
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        text: String,
        x: f64,
        y: f64,
        width: f64,
        height: f64,
        confidence: f64,
        font_size: f64,
        fragment_type: FragmentType,
    ) -> Self {
        Self {
            text,
            x,
            y,
            width,
            height,
            confidence,
            word_confidences: None,
            font_size,
            fragment_type,
        }
    }

    /// Create a fragment with word-level confidence scores
    #[allow(clippy::too_many_arguments)]
    pub fn with_word_confidences(
        text: String,
        x: f64,
        y: f64,
        width: f64,
        height: f64,
        confidence: f64,
        font_size: f64,
        fragment_type: FragmentType,
        word_confidences: Vec<WordConfidence>,
    ) -> Self {
        Self {
            text,
            x,
            y,
            width,
            height,
            confidence,
            word_confidences: Some(word_confidences),
            font_size,
            fragment_type,
        }
    }

    /// Get words with confidence below the threshold
    pub fn get_low_confidence_words(&self, threshold: f64) -> Vec<&WordConfidence> {
        self.word_confidences
            .as_ref()
            .map(|words| words.iter().filter(|w| w.confidence < threshold).collect())
            .unwrap_or_default()
    }

    /// Get the average word confidence if available
    pub fn average_word_confidence(&self) -> Option<f64> {
        self.word_confidences.as_ref().map(|words| {
            if words.is_empty() {
                return 0.0;
            }
            let sum: f64 = words.iter().map(|w| w.confidence).sum();
            sum / words.len() as f64
        })
    }

    /// Get words sorted by confidence (lowest first)
    pub fn words_by_confidence(&self) -> Vec<&WordConfidence> {
        self.word_confidences
            .as_ref()
            .map(|words| {
                let mut sorted_words: Vec<_> = words.iter().collect();
                sorted_words.sort_by(|a, b| {
                    a.confidence
                        .partial_cmp(&b.confidence)
                        .unwrap_or(std::cmp::Ordering::Equal)
                });
                sorted_words
            })
            .unwrap_or_default()
    }

    /// Check if this fragment has any low-confidence words
    pub fn has_low_confidence_words(&self, threshold: f64) -> bool {
        self.word_confidences
            .as_ref()
            .map(|words| words.iter().any(|w| w.confidence < threshold))
            .unwrap_or(false)
    }

    /// Get words that are candidates for correction (low confidence + patterns)
    pub fn get_correction_candidates(&self, threshold: f64) -> Vec<CorrectionCandidate> {
        self.word_confidences
            .as_ref()
            .map(|words| {
                words
                    .iter()
                    .enumerate()
                    .filter(|(_, w)| w.confidence < threshold)
                    .map(|(index, word)| CorrectionCandidate {
                        word: word.word.clone(),
                        confidence: word.confidence,
                        position_in_fragment: index,
                        suggested_corrections: vec![], // Will be filled by post-processor
                        correction_reason: CorrectionReason::LowConfidence,
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Generate a confidence report for this fragment
    pub fn confidence_report(&self) -> String {
        let mut report = format!(
            "Fragment confidence: {:.1}% - \"{}\"\n",
            self.confidence * 100.0,
            self.text.trim()
        );

        if let Some(words) = &self.word_confidences {
            report.push_str(&format!(
                "  Word-level breakdown ({} words):\n",
                words.len()
            ));
            for (i, word) in words.iter().enumerate() {
                report.push_str(&format!(
                    "    {}: \"{}\" - {:.1}%\n",
                    i + 1,
                    word.word,
                    word.confidence * 100.0
                ));

                if let Some(chars) = &word.character_confidences {
                    report.push_str("      Characters: ");
                    for ch in chars {
                        report.push_str(&format!(
                            "'{}'({:.0}%) ",
                            ch.character,
                            ch.confidence * 100.0
                        ));
                    }
                    report.push('\n');
                }
            }
        } else {
            report.push_str("  (No word-level data available)\n");
        }

        report
    }
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

    /// Region that was processed (None if entire image was processed)
    pub processed_region: Option<OcrRegion>,

    /// Image dimensions that were processed
    pub image_dimensions: (u32, u32),
}

impl OcrProcessingResult {
    /// Create a new OCR processing result
    pub fn new(
        text: String,
        confidence: f64,
        fragments: Vec<OcrTextFragment>,
        processing_time_ms: u64,
        engine_name: String,
        language: String,
        image_dimensions: (u32, u32),
    ) -> Self {
        Self {
            text,
            confidence,
            fragments,
            processing_time_ms,
            engine_name,
            language,
            processed_region: None,
            image_dimensions,
        }
    }

    /// Create a new OCR processing result for a specific region
    #[allow(clippy::too_many_arguments)]
    pub fn with_region(
        text: String,
        confidence: f64,
        fragments: Vec<OcrTextFragment>,
        processing_time_ms: u64,
        engine_name: String,
        language: String,
        image_dimensions: (u32, u32),
        region: OcrRegion,
    ) -> Self {
        Self {
            text,
            confidence,
            fragments,
            processing_time_ms,
            engine_name,
            language,
            processed_region: Some(region),
            image_dimensions,
        }
    }

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
///         #     processed_region: None,
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

    /// Process multiple images with region information
    ///
    /// This method allows for selective OCR processing where each image corresponds
    /// to a specific region. This is useful for:
    /// - Processing pre-cropped regions of a document  
    /// - Batch processing of multiple regions with different OCR settings
    /// - Optimizing performance by avoiding full-image processing
    ///
    /// # Arguments
    ///
    /// * `image_region_pairs` - Vector of (image_data, region) pairs
    /// * `options` - OCR processing options (applies to all regions)
    ///
    /// # Returns
    ///
    /// A vector of `OcrProcessingResult`, one for each processed region.
    /// The order matches the input pairs vector.
    ///
    /// # Default Implementation
    ///
    /// The default implementation processes each image separately and sets
    /// the region information in the result.
    fn process_image_regions(
        &self,
        image_region_pairs: &[(&[u8], &OcrRegion)],
        options: &OcrOptions,
    ) -> OcrResult<Vec<OcrProcessingResult>> {
        let mut results = Vec::with_capacity(image_region_pairs.len());

        for (image_data, region) in image_region_pairs {
            let mut result = self.process_image(image_data, options)?;

            // Adjust fragment coordinates to match original image coordinates
            // (assuming the input image_data is already cropped to the region)
            for fragment in &mut result.fragments {
                fragment.x += region.x as f64;
                fragment.y += region.y as f64;
            }

            result.processed_region = Some((*region).clone());
            results.push(result);
        }

        Ok(results)
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
                word_confidences: None,
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
                word_confidences: None,
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
            processed_region: None,
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
mod tests;

#[cfg(test)]
mod postprocessor_tests;
