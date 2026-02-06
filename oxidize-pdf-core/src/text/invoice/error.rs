//! Error types for invoice extraction

use thiserror::Error;

/// Errors that can occur during invoice extraction
#[derive(Debug, Error)]
pub enum ExtractionError {
    /// No text found on the specified page
    #[error("No text found on page {0}")]
    NoTextFound(u32),

    /// Pattern matching failed with details
    #[error("Pattern matching failed: {0}")]
    PatternError(String),

    /// Invalid configuration provided
    #[error("Invalid configuration: {0}")]
    ConfigError(String),

    /// Unsupported language specified
    #[error("Unsupported language: {0}")]
    UnsupportedLanguage(String),

    /// Invalid confidence threshold (must be 0.0-1.0)
    #[error("Invalid confidence threshold: {0} (must be between 0.0 and 1.0)")]
    InvalidThreshold(f64),

    /// Regex compilation error
    #[error("Regex compilation error: {0}")]
    RegexError(String),

    /// Generic extraction error with context
    #[error("Extraction error: {0}")]
    Generic(String),
}

/// Result type for invoice extraction operations
pub type Result<T> = std::result::Result<T, ExtractionError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_no_text_found_error() {
        let err = ExtractionError::NoTextFound(5);
        assert_eq!(format!("{}", err), "No text found on page 5");
    }

    #[test]
    fn test_no_text_found_page_zero() {
        let err = ExtractionError::NoTextFound(0);
        assert_eq!(format!("{}", err), "No text found on page 0");
    }

    #[test]
    fn test_pattern_error() {
        let err = ExtractionError::PatternError("Invalid regex".to_string());
        assert_eq!(format!("{}", err), "Pattern matching failed: Invalid regex");
    }

    #[test]
    fn test_config_error() {
        let err = ExtractionError::ConfigError("Missing required field".to_string());
        assert_eq!(
            format!("{}", err),
            "Invalid configuration: Missing required field"
        );
    }

    #[test]
    fn test_unsupported_language() {
        let err = ExtractionError::UnsupportedLanguage("Klingon".to_string());
        assert_eq!(format!("{}", err), "Unsupported language: Klingon");
    }

    #[test]
    fn test_invalid_threshold_negative() {
        let err = ExtractionError::InvalidThreshold(-0.5);
        assert_eq!(
            format!("{}", err),
            "Invalid confidence threshold: -0.5 (must be between 0.0 and 1.0)"
        );
    }

    #[test]
    fn test_invalid_threshold_too_high() {
        let err = ExtractionError::InvalidThreshold(1.5);
        assert_eq!(
            format!("{}", err),
            "Invalid confidence threshold: 1.5 (must be between 0.0 and 1.0)"
        );
    }

    #[test]
    fn test_regex_error() {
        let err = ExtractionError::RegexError("Unmatched parenthesis".to_string());
        assert_eq!(
            format!("{}", err),
            "Regex compilation error: Unmatched parenthesis"
        );
    }

    #[test]
    fn test_generic_error() {
        let err = ExtractionError::Generic("Something went wrong".to_string());
        assert_eq!(format!("{}", err), "Extraction error: Something went wrong");
    }

    #[test]
    fn test_error_debug_impl() {
        let err = ExtractionError::NoTextFound(3);
        // Debug trait should be implemented
        let debug_str = format!("{:?}", err);
        assert!(debug_str.contains("NoTextFound"));
        assert!(debug_str.contains("3"));
    }

    #[test]
    fn test_error_is_send_sync() {
        // Verify ExtractionError implements Send + Sync for thread safety
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<ExtractionError>();
    }

    #[test]
    fn test_result_type_alias() {
        // Test that Result type alias works correctly
        fn returns_ok() -> Result<i32> {
            Ok(42)
        }

        fn returns_err() -> Result<i32> {
            Err(ExtractionError::Generic("test".to_string()))
        }

        assert!(returns_ok().is_ok());
        assert!(returns_err().is_err());
    }
}
