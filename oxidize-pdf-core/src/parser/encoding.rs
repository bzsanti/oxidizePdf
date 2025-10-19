//! Character Encoding Detection and Conversion Module
//!
//! This module provides robust character encoding detection and conversion capabilities
//! to handle the diverse encoding scenarios found in real-world PDF files.
//!
//! # Overview
//!
//! Many PDFs contain text encoded in various character sets beyond UTF-8, including:
//! - Latin-1 (ISO 8859-1) - Common in European documents
//! - Windows-1252 - Microsoft's extension of Latin-1
//! - MacRoman - Apple's legacy encoding
//! - Various PDF-specific encodings
//!
//! This module provides automatic detection and graceful conversion with fallback
//! handling for unrecognized characters.

use crate::error::PdfError;
use std::collections::HashMap;

/// Character encoding detection and conversion result
#[derive(Debug, Clone)]
pub struct EncodingResult {
    /// The decoded text
    pub text: String,
    /// Detected encoding (if any)
    pub detected_encoding: Option<EncodingType>,
    /// Number of replacement characters used
    pub replacement_count: usize,
    /// Confidence level (0.0 to 1.0)
    pub confidence: f64,
}

/// Supported encoding types for PDF text
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EncodingType {
    /// UTF-8 (modern standard)
    Utf8,
    /// Latin-1 / ISO 8859-1 (European)
    Latin1,
    /// Windows-1252 (Microsoft extension of Latin-1)
    Windows1252,
    /// MacRoman (Apple legacy)
    MacRoman,
    /// PDF built-in encoding
    PdfDocEncoding,
    /// Unknown/Mixed encoding
    Mixed,
}

impl EncodingType {
    /// Get human-readable name
    pub fn name(&self) -> &'static str {
        match self {
            EncodingType::Utf8 => "UTF-8",
            EncodingType::Latin1 => "ISO 8859-1 (Latin-1)",
            EncodingType::Windows1252 => "Windows-1252",
            EncodingType::MacRoman => "MacRoman",
            EncodingType::PdfDocEncoding => "PDFDocEncoding",
            EncodingType::Mixed => "Mixed/Unknown",
        }
    }
}

/// Configuration for character encoding processing
#[derive(Debug, Clone)]
pub struct EncodingOptions {
    /// Enable lenient mode (use replacement characters instead of failing)
    pub lenient_mode: bool,
    /// Prefer specific encoding for detection
    pub preferred_encoding: Option<EncodingType>,
    /// Maximum replacement characters before giving up
    pub max_replacements: usize,
    /// Log problematic characters for analysis
    pub log_issues: bool,
}

impl Default for EncodingOptions {
    fn default() -> Self {
        Self {
            lenient_mode: true,
            preferred_encoding: None,
            max_replacements: 100,
            log_issues: false,
        }
    }
}

/// Main character decoder trait
pub trait CharacterDecoder {
    /// Decode bytes to string with encoding detection
    fn decode(&self, bytes: &[u8], options: &EncodingOptions) -> Result<EncodingResult, PdfError>;

    /// Detect the most likely encoding for the given bytes
    fn detect_encoding(&self, bytes: &[u8]) -> Option<EncodingType>;

    /// Convert bytes using a specific encoding
    fn decode_with_encoding(
        &self,
        bytes: &[u8],
        encoding: EncodingType,
        lenient: bool,
    ) -> Result<String, PdfError>;
}

/// Enhanced character decoder implementation
pub struct EnhancedDecoder {
    /// Latin-1 to Unicode mapping
    latin1_map: HashMap<u8, char>,
    /// Windows-1252 to Unicode mapping
    windows1252_map: HashMap<u8, char>,
    /// MacRoman to Unicode mapping
    macroman_map: HashMap<u8, char>,
    /// Issue logger for analysis
    issue_log: Vec<EncodingIssue>,
}

/// Information about encoding issues encountered
#[derive(Debug, Clone)]
pub struct EncodingIssue {
    pub byte_value: u8,
    pub context: String,
    pub attempted_encodings: Vec<EncodingType>,
    pub resolution: IssueResolution,
}

#[derive(Debug, Clone)]
pub enum IssueResolution {
    ReplacementCharacter,
    SuccessfulConversion(char),
    Skipped,
}

impl EnhancedDecoder {
    /// Create a new enhanced decoder with all encoding tables loaded
    pub fn new() -> Self {
        let mut decoder = Self {
            latin1_map: HashMap::new(),
            windows1252_map: HashMap::new(),
            macroman_map: HashMap::new(),
            issue_log: Vec::new(),
        };

        decoder.initialize_encoding_tables();
        decoder
    }

    /// Initialize all encoding conversion tables
    fn initialize_encoding_tables(&mut self) {
        // Latin-1 (ISO 8859-1) mapping - direct 1:1 mapping for 0x80-0xFF
        for i in 0x80..=0xFF {
            // Safety: All values in 0x80-0xFF range are valid Unicode codepoints (ISO/IEC 8859-1)
            // This is guaranteed by the Unicode standard which includes Latin-1 supplement block
            if let Some(ch) = char::from_u32(i as u32) {
                self.latin1_map.insert(i, ch);
            }
        }

        // Windows-1252 mapping (extends Latin-1 for 0x80-0x9F range)
        let windows1252_extensions = [
            (0x80, '€'),        // Euro sign
            (0x82, '‚'),        // Single low-9 quotation mark
            (0x83, 'ƒ'),        // Latin small letter f with hook
            (0x84, '„'),        // Double low-9 quotation mark
            (0x85, '…'),        // Horizontal ellipsis
            (0x86, '†'),        // Dagger
            (0x87, '‡'),        // Double dagger
            (0x88, 'ˆ'),        // Modifier letter circumflex accent
            (0x89, '‰'),        // Per mille sign
            (0x8A, 'Š'),        // Latin capital letter S with caron
            (0x8B, '‹'),        // Single left-pointing angle quotation mark
            (0x8C, 'Œ'),        // Latin capital ligature OE
            (0x8E, 'Ž'),        // Latin capital letter Z with caron
            (0x91, '\u{2018}'), // Left single quotation mark
            (0x92, '\u{2019}'), // Right single quotation mark
            (0x93, '\u{201C}'), // Left double quotation mark
            (0x94, '\u{201D}'), // Right double quotation mark
            (0x95, '•'),        // Bullet
            (0x96, '–'),        // En dash
            (0x97, '—'),        // Em dash
            (0x98, '˜'),        // Small tilde
            (0x99, '™'),        // Trade mark sign
            (0x9A, 'š'),        // Latin small letter s with caron
            (0x9B, '›'),        // Single right-pointing angle quotation mark
            (0x9C, 'œ'),        // Latin small ligature oe
            (0x9E, 'ž'),        // Latin small letter z with caron
            (0x9F, 'Ÿ'),        // Latin capital letter Y with diaeresis
        ];

        // Copy Latin-1 base
        self.windows1252_map = self.latin1_map.clone();
        // Override with Windows-1252 extensions
        for (byte, ch) in windows1252_extensions.iter() {
            self.windows1252_map.insert(*byte, *ch);
        }

        // MacRoman mapping (partial - most common characters)
        let macroman_chars = [
            (0x80, 'Ä'),
            (0x81, 'Å'),
            (0x82, 'Ç'),
            (0x83, 'É'),
            (0x84, 'Ñ'),
            (0x85, 'Ö'),
            (0x86, 'Ü'),
            (0x87, 'á'),
            (0x88, 'à'),
            (0x89, 'â'),
            (0x8A, 'ä'),
            (0x8B, 'ã'),
            (0x8C, 'å'),
            (0x8D, 'ç'),
            (0x8E, 'é'),
            (0x8F, 'è'),
            (0x90, 'ê'),
            (0x91, 'ë'),
            (0x92, 'í'),
            (0x93, 'ì'),
            (0x94, 'î'),
            (0x95, 'ï'),
            (0x96, 'ñ'),
            (0x97, 'ó'),
            (0x98, 'ò'),
            (0x99, 'ô'),
            (0x9A, 'ö'),
            (0x9B, 'õ'),
            (0x9C, 'ú'),
            (0x9D, 'ù'),
            (0x9E, 'û'),
            (0x9F, 'ü'),
            (0xA0, '†'),
            (0xA1, '°'),
            (0xA2, '¢'),
            (0xA3, '£'),
            (0xA4, '§'),
            (0xA5, '•'),
            (0xA6, '¶'),
            (0xA7, 'ß'),
            (0xA8, '®'),
            (0xA9, '©'),
            (0xAA, '™'),
            (0xAB, '´'),
            (0xAC, '¨'),
            (0xAD, '≠'),
            (0xAE, 'Æ'),
            (0xAF, 'Ø'),
        ];

        for (byte, ch) in macroman_chars.iter() {
            self.macroman_map.insert(*byte, *ch);
        }
    }

    /// Clear the issue log
    pub fn clear_log(&mut self) {
        self.issue_log.clear();
    }

    /// Get the current issue log
    pub fn get_issues(&self) -> &[EncodingIssue] {
        &self.issue_log
    }

    /// Log an encoding issue
    #[allow(dead_code)]
    fn log_issue(&mut self, issue: EncodingIssue) {
        self.issue_log.push(issue);
    }

    /// Analyze bytes to detect most likely encoding
    fn analyze_encoding_indicators(&self, bytes: &[u8]) -> Vec<(EncodingType, f64)> {
        let mut scores = vec![
            (EncodingType::Utf8, 0.0),
            (EncodingType::Latin1, 0.0),
            (EncodingType::Windows1252, 0.0),
            (EncodingType::MacRoman, 0.0),
        ];

        // UTF-8 validity check
        if std::str::from_utf8(bytes).is_ok() {
            scores[0].1 = 0.9; // High confidence for valid UTF-8
        }

        // Check for Windows-1252 specific characters
        let mut windows1252_indicators = 0;
        let mut latin1_indicators = 0;
        let mut macroman_indicators = 0;

        for &byte in bytes {
            if byte >= 0x80 {
                // Count high-bit characters
                if self.windows1252_map.contains_key(&byte) {
                    windows1252_indicators += 1;
                    // Special boost for Windows-1252 specific chars
                    if matches!(byte, 0x80 | 0x82..=0x8C | 0x8E | 0x91..=0x9C | 0x9E | 0x9F) {
                        scores[2].1 += 0.1;
                    }
                }
                if self.latin1_map.contains_key(&byte) {
                    latin1_indicators += 1;
                }
                if self.macroman_map.contains_key(&byte) {
                    macroman_indicators += 1;
                }
            }
        }

        // Adjust scores based on indicators
        if windows1252_indicators > 0 {
            scores[2].1 += 0.3;
        }
        if latin1_indicators > 0 {
            scores[1].1 += 0.2;
        }
        if macroman_indicators > 0 {
            scores[3].1 += 0.1;
        }

        // Sort by confidence score
        scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scores
    }
}

impl Default for EnhancedDecoder {
    fn default() -> Self {
        Self::new()
    }
}

impl CharacterDecoder for EnhancedDecoder {
    fn decode(&self, bytes: &[u8], options: &EncodingOptions) -> Result<EncodingResult, PdfError> {
        // Try preferred encoding first
        if let Some(preferred) = options.preferred_encoding {
            if let Ok(text) = self.decode_with_encoding(bytes, preferred, options.lenient_mode) {
                let replacement_count = text.chars().filter(|&c| c == '\u{FFFD}').count();
                return Ok(EncodingResult {
                    text,
                    detected_encoding: Some(preferred),
                    replacement_count,
                    confidence: 0.8,
                });
            }
        }

        // Auto-detect encoding
        let encoding_candidates = self.analyze_encoding_indicators(bytes);

        for (encoding, confidence) in encoding_candidates {
            if confidence > 0.1 {
                match self.decode_with_encoding(bytes, encoding, options.lenient_mode) {
                    Ok(text) => {
                        let replacement_count = text.chars().filter(|&c| c == '\u{FFFD}').count();

                        if replacement_count <= options.max_replacements {
                            return Ok(EncodingResult {
                                text,
                                detected_encoding: Some(encoding),
                                replacement_count,
                                confidence,
                            });
                        }
                    }
                    Err(_) => continue,
                }
            }
        }

        // Last resort: UTF-8 with replacement
        if options.lenient_mode {
            let text = String::from_utf8_lossy(bytes).to_string();
            let replacement_count = text.chars().filter(|&c| c == '\u{FFFD}').count();

            Ok(EncodingResult {
                text,
                detected_encoding: Some(EncodingType::Mixed),
                replacement_count,
                confidence: 0.1,
            })
        } else {
            Err(PdfError::EncodingError(
                "Failed to decode text with any supported encoding".to_string(),
            ))
        }
    }

    fn detect_encoding(&self, bytes: &[u8]) -> Option<EncodingType> {
        let candidates = self.analyze_encoding_indicators(bytes);
        candidates.first().map(|(encoding, _)| *encoding)
    }

    fn decode_with_encoding(
        &self,
        bytes: &[u8],
        encoding: EncodingType,
        lenient: bool,
    ) -> Result<String, PdfError> {
        match encoding {
            EncodingType::Utf8 => {
                if lenient {
                    Ok(String::from_utf8_lossy(bytes).to_string())
                } else {
                    String::from_utf8(bytes.to_vec())
                        .map_err(|e| PdfError::EncodingError(format!("UTF-8 decoding failed: {e}")))
                }
            }

            EncodingType::Latin1 => {
                let mut result = String::with_capacity(bytes.len());
                for &byte in bytes {
                    if byte < 0x80 {
                        result.push(byte as char);
                    } else if let Some(&ch) = self.latin1_map.get(&byte) {
                        result.push(ch);
                    } else if lenient {
                        result.push('\u{FFFD}');
                    } else {
                        return Err(PdfError::EncodingError(format!(
                            "Invalid Latin-1 character: 0x{byte:02X}"
                        )));
                    }
                }
                Ok(result)
            }

            EncodingType::Windows1252 => {
                let mut result = String::with_capacity(bytes.len());
                for &byte in bytes {
                    if byte < 0x80 {
                        result.push(byte as char);
                    } else if let Some(&ch) = self.windows1252_map.get(&byte) {
                        result.push(ch);
                    } else if lenient {
                        result.push('\u{FFFD}');
                    } else {
                        return Err(PdfError::EncodingError(format!(
                            "Invalid Windows-1252 character: 0x{byte:02X}"
                        )));
                    }
                }
                Ok(result)
            }

            EncodingType::MacRoman => {
                let mut result = String::with_capacity(bytes.len());
                for &byte in bytes {
                    if byte < 0x80 {
                        result.push(byte as char);
                    } else if let Some(&ch) = self.macroman_map.get(&byte) {
                        result.push(ch);
                    } else if lenient {
                        result.push('\u{FFFD}');
                    } else {
                        return Err(PdfError::EncodingError(format!(
                            "Invalid MacRoman character: 0x{byte:02X}"
                        )));
                    }
                }
                Ok(result)
            }

            EncodingType::PdfDocEncoding => {
                // PDFDocEncoding is identical to Latin-1 for now
                self.decode_with_encoding(bytes, EncodingType::Latin1, lenient)
            }

            EncodingType::Mixed => {
                // Try multiple encodings and pick the best result
                let candidates = [
                    EncodingType::Utf8,
                    EncodingType::Windows1252,
                    EncodingType::Latin1,
                    EncodingType::MacRoman,
                ];

                for candidate in &candidates {
                    if let Ok(result) = self.decode_with_encoding(bytes, *candidate, true) {
                        let replacement_count = result.chars().filter(|&c| c == '\u{FFFD}').count();
                        if replacement_count < bytes.len() / 4 {
                            // Less than 25% replacement chars
                            return Ok(result);
                        }
                    }
                }

                // Fallback to UTF-8 lossy
                Ok(String::from_utf8_lossy(bytes).to_string())
            }
        }
    }
}

/// Convenience function to decode bytes with default settings
pub fn decode_text(bytes: &[u8]) -> Result<String, PdfError> {
    let decoder = EnhancedDecoder::new();
    let options = EncodingOptions::default();
    let result = decoder.decode(bytes, &options)?;
    Ok(result.text)
}

/// Convenience function to decode bytes with specific encoding
pub fn decode_text_with_encoding(bytes: &[u8], encoding: EncodingType) -> Result<String, PdfError> {
    let decoder = EnhancedDecoder::new();
    decoder.decode_with_encoding(bytes, encoding, true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_utf8_decoding() {
        let decoder = EnhancedDecoder::new();
        let options = EncodingOptions::default();

        let utf8_text = "Hello, 世界!";
        let bytes = utf8_text.as_bytes();

        let result = decoder.decode(bytes, &options).unwrap();
        assert_eq!(result.text, utf8_text);
        assert_eq!(result.detected_encoding, Some(EncodingType::Utf8));
        assert_eq!(result.replacement_count, 0);
    }

    #[test]
    fn test_latin1_decoding() {
        let decoder = EnhancedDecoder::new();
        let options = EncodingOptions::default();

        // Latin-1 text with accented characters
        let bytes = vec![0x48, 0x65, 0x6C, 0x6C, 0x6F, 0x2C, 0x20, 0xE9, 0xE8, 0xE7]; // "Hello, éèç"

        let result = decoder.decode(&bytes, &options).unwrap();
        assert!(result.text.contains("éèç"));
    }

    #[test]
    fn test_windows1252_decoding() {
        let decoder = EnhancedDecoder::new();
        let options = EncodingOptions::default();

        // Windows-1252 text with Euro sign and smart quotes
        let bytes = vec![0x80, 0x20, 0x91, 0x48, 0x65, 0x6C, 0x6C, 0x6F, 0x92]; // "€ 'Hello'"

        let result = decoder.decode(&bytes, &options).unwrap();
        assert!(result.text.contains("€"));
        assert!(result.text.contains('\u{2018}')); // Left single quote
        assert!(result.text.contains('\u{2019}')); // Right single quote
    }

    #[test]
    fn test_lenient_mode() {
        let decoder = EnhancedDecoder::new();
        let mut options = EncodingOptions::default();
        options.lenient_mode = true;
        options.preferred_encoding = Some(EncodingType::Utf8); // Force UTF-8 to get replacement chars

        // Invalid UTF-8 sequence (will cause replacement chars in UTF-8 mode)
        let bytes = vec![0xFF, 0xFE, 0x48, 0x65, 0x6C, 0x6C, 0x6F]; // Invalid UTF-8 + "Hello"

        let result = decoder.decode(&bytes, &options).unwrap();
        assert!(
            result.replacement_count > 0,
            "Expected replacement chars, got {}",
            result.replacement_count
        );
        assert!(result.text.contains("Hello"));
    }

    #[test]
    fn test_encoding_detection() {
        let decoder = EnhancedDecoder::new();

        // UTF-8
        let utf8_bytes = "Hello, 世界!".as_bytes();
        assert_eq!(
            decoder.detect_encoding(utf8_bytes),
            Some(EncodingType::Utf8)
        );

        // Windows-1252 with Euro sign
        let win1252_bytes = vec![0x80, 0x20, 0x48, 0x65, 0x6C, 0x6C, 0x6F];
        let detected = decoder.detect_encoding(&win1252_bytes);
        assert!(matches!(
            detected,
            Some(EncodingType::Windows1252) | Some(EncodingType::Latin1)
        ));
    }

    #[test]
    fn test_specific_encoding() {
        let decoder = EnhancedDecoder::new();

        let bytes = vec![0xC9]; // É in Latin-1

        let latin1_result = decoder
            .decode_with_encoding(&bytes, EncodingType::Latin1, false)
            .unwrap();
        assert_eq!(latin1_result, "É");

        let win1252_result = decoder
            .decode_with_encoding(&bytes, EncodingType::Windows1252, false)
            .unwrap();
        assert_eq!(win1252_result, "É");
    }

    #[test]
    fn test_convenience_functions() {
        let utf8_text = "Hello, world!";
        let bytes = utf8_text.as_bytes();

        let decoded = decode_text(bytes).unwrap();
        assert_eq!(decoded, utf8_text);

        let latin1_bytes = vec![0x48, 0x65, 0x6C, 0x6C, 0x6F, 0xE9]; // "Hellé"
        let decoded = decode_text_with_encoding(&latin1_bytes, EncodingType::Latin1).unwrap();
        assert!(decoded.contains("é"));
    }
}
