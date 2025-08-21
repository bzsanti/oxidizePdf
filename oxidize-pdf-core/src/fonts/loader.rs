//! Font loading utilities

use crate::error::PdfError;
use crate::Result;

/// Font format enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FontFormat {
    /// TrueType font format
    TrueType,
    /// OpenType font format  
    OpenType,
}

impl FontFormat {
    /// Detect font format from raw data
    pub fn detect(data: &[u8]) -> Result<Self> {
        if data.len() < 4 {
            return Err(PdfError::FontError("Font data too small".into()));
        }

        // Check magic bytes
        match &data[0..4] {
            // TTF magic: 0x00010000
            [0x00, 0x01, 0x00, 0x00] => Ok(FontFormat::TrueType),
            // OTF magic: "OTTO"
            [0x4F, 0x54, 0x54, 0x4F] => Ok(FontFormat::OpenType),
            // TTF with 'true' tag
            [0x74, 0x72, 0x75, 0x65] => Ok(FontFormat::TrueType),
            _ => Err(PdfError::FontError("Unknown font format".into())),
        }
    }
}

/// Raw font data container
#[derive(Debug, Clone)]
pub struct FontData {
    /// Raw font bytes
    pub bytes: Vec<u8>,
    /// Detected font format
    pub format: FontFormat,
}

/// Font loader for reading font files
pub struct FontLoader;

impl FontLoader {
    /// Load font data from file
    pub fn load_from_file(path: impl AsRef<std::path::Path>) -> Result<FontData> {
        let bytes = std::fs::read(path)?;
        Self::load_from_bytes(bytes)
    }

    /// Load font data from bytes
    pub fn load_from_bytes(bytes: Vec<u8>) -> Result<FontData> {
        let format = FontFormat::detect(&bytes)?;
        Ok(FontData { bytes, format })
    }

    /// Validate font data
    pub fn validate(data: &FontData) -> Result<()> {
        if data.bytes.len() < 12 {
            return Err(PdfError::FontError("Font file too small".into()));
        }

        // Basic validation based on format
        match data.format {
            FontFormat::TrueType => Self::validate_ttf(&data.bytes),
            FontFormat::OpenType => Self::validate_otf(&data.bytes),
        }
    }

    fn validate_ttf(data: &[u8]) -> Result<()> {
        // TTF should have table directory after header
        if data.len() < 12 {
            return Err(PdfError::FontError("Invalid TTF structure".into()));
        }
        Ok(())
    }

    fn validate_otf(data: &[u8]) -> Result<()> {
        // OTF should have CFF table
        if data.len() < 12 {
            return Err(PdfError::FontError("Invalid OTF structure".into()));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_font_format_detection() {
        // Test TTF detection
        let ttf_header = vec![0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
        let ttf_data = FontLoader::load_from_bytes(ttf_header).unwrap();
        assert_eq!(ttf_data.format, FontFormat::TrueType);

        // Test OTF detection
        let otf_header = vec![0x4F, 0x54, 0x54, 0x4F, 0x00, 0x00, 0x00, 0x00];
        let otf_data = FontLoader::load_from_bytes(otf_header).unwrap();
        assert_eq!(otf_data.format, FontFormat::OpenType);
    }

    #[test]
    fn test_invalid_font_data() {
        // Too small
        let small_data = vec![0x00, 0x01];
        assert!(FontLoader::load_from_bytes(small_data).is_err());

        // Invalid magic
        let invalid_data = vec![0xFF, 0xFF, 0xFF, 0xFF];
        assert!(FontLoader::load_from_bytes(invalid_data).is_err());
    }

    #[test]
    fn test_font_validation() {
        // Valid TTF with minimal data
        let mut ttf_data = vec![0x00, 0x01, 0x00, 0x00];
        ttf_data.extend_from_slice(&[0x00; 20]); // Add padding
        let font_data = FontLoader::load_from_bytes(ttf_data).unwrap();
        assert!(FontLoader::validate(&font_data).is_ok());

        // Invalid - too small
        let small_font = FontData {
            bytes: vec![0x00; 10],
            format: FontFormat::TrueType,
        };
        assert!(FontLoader::validate(&small_font).is_err());
    }

    #[test]
    fn test_ttf_true_tag_detection() {
        // Test TTF with 'true' tag
        let true_header = vec![0x74, 0x72, 0x75, 0x65, 0x00, 0x00, 0x00, 0x00];
        let true_data = FontLoader::load_from_bytes(true_header).unwrap();
        assert_eq!(true_data.format, FontFormat::TrueType);
    }

    #[test]
    fn test_font_data_clone() {
        let font_data = FontData {
            bytes: vec![1, 2, 3, 4],
            format: FontFormat::TrueType,
        };

        let cloned = font_data.clone();
        assert_eq!(cloned.bytes, font_data.bytes);
        assert_eq!(cloned.format, font_data.format);
    }

    #[test]
    fn test_font_format_equality() {
        assert_eq!(FontFormat::TrueType, FontFormat::TrueType);
        assert_eq!(FontFormat::OpenType, FontFormat::OpenType);
        assert_ne!(FontFormat::TrueType, FontFormat::OpenType);
    }

    #[test]
    fn test_otf_validation() {
        // Valid OTF with minimal data
        let mut otf_data = vec![0x4F, 0x54, 0x54, 0x4F];
        otf_data.extend_from_slice(&[0x00; 20]); // Add padding
        let font_data = FontLoader::load_from_bytes(otf_data).unwrap();
        assert!(FontLoader::validate(&font_data).is_ok());

        // Invalid OTF - too small
        let small_otf = FontData {
            bytes: vec![0x4F, 0x54, 0x54, 0x4F],
            format: FontFormat::OpenType,
        };
        assert!(FontLoader::validate(&small_otf).is_err());
    }

    #[test]
    fn test_empty_font_data() {
        let empty_data = vec![];
        let result = FontLoader::load_from_bytes(empty_data);
        assert!(result.is_err());
        if let Err(PdfError::FontError(msg)) = result {
            assert!(msg.contains("too small"));
        }
    }

    #[test]
    fn test_font_format_detect_edge_cases() {
        // Test with exactly 4 bytes
        let exact_ttf = vec![0x00, 0x01, 0x00, 0x00];
        assert_eq!(
            FontFormat::detect(&exact_ttf).unwrap(),
            FontFormat::TrueType
        );

        // Test with 3 bytes (too small)
        let too_small = vec![0x00, 0x01, 0x00];
        assert!(FontFormat::detect(&too_small).is_err());
    }
}
