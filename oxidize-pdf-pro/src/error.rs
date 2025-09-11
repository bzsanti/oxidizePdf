pub type Result<T> = std::result::Result<T, ProError>;

#[derive(Debug, thiserror::Error)]
pub enum ProError {
    #[error("XMP embedding error: {0}")]
    XmpEmbedding(String),

    #[error("XMP extraction error: {0}")]
    XmpExtraction(String),

    #[error("XMP parsing error: {0}")]
    XmpParsing(String),

    #[error("XMP serialization error: {0}")]
    XmpSerialization(String),

    #[error("Schema.org validation error: {0}")]
    SchemaValidation(String),

    #[error("License validation error: {0}")]
    LicenseValidation(String),

    #[error("License expired or invalid")]
    LicenseExpired,

    #[error("Feature not available in current license: {0}")]
    FeatureNotLicensed(String),

    #[error("Template error: {0}")]
    Template(String),

    #[error("Entity extraction error: {0}")]
    Extraction(String),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Date/time error: {0}")]
    DateTime(#[from] chrono::format::ParseError),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Core PDF error: {0}")]
    Core(String),

    #[error("HTTP request error: {0}")]
    Http(String),

    #[error("Base64 decode error: {0}")]
    Base64Decode(#[from] base64::DecodeError),

    #[error("UUID error: {0}")]
    Uuid(#[from] uuid::Error),
}

impl From<anyhow::Error> for ProError {
    fn from(err: anyhow::Error) -> Self {
        ProError::Core(err.to_string())
    }
}

#[cfg(feature = "license-validation")]
impl From<reqwest::Error> for ProError {
    fn from(err: reqwest::Error) -> Self {
        ProError::Http(err.to_string())
    }
}

// Convert from core PDF errors
impl From<Box<dyn std::error::Error + Send + Sync>> for ProError {
    fn from(err: Box<dyn std::error::Error + Send + Sync>) -> Self {
        ProError::Core(err.to_string())
    }
}

impl From<oxidize_pdf::PdfError> for ProError {
    fn from(err: oxidize_pdf::PdfError) -> Self {
        ProError::Core(err.to_string())
    }
}
