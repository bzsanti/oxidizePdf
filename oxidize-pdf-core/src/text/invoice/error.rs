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
