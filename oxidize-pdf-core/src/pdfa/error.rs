//! Error types for PDF/A validation

use std::fmt;

/// Errors that can occur during PDF/A validation
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidationError {
    /// Document is encrypted (encryption forbidden in PDF/A)
    EncryptionForbidden,

    /// Font is not embedded in the document
    FontNotEmbedded {
        /// Name of the font that is not embedded
        font_name: String,
    },

    /// Font is missing required ToUnicode CMap
    FontMissingToUnicode {
        /// Name of the font missing ToUnicode
        font_name: String,
    },

    /// JavaScript is present (forbidden in PDF/A)
    JavaScriptForbidden {
        /// Location where JavaScript was found
        location: String,
    },

    /// XMP metadata is missing or invalid
    XmpMetadataMissing,

    /// XMP metadata is missing PDF/A identifier
    XmpMissingPdfAIdentifier,

    /// XMP metadata has invalid PDF/A identifier
    XmpInvalidPdfAIdentifier {
        /// Details about the invalid identifier
        details: String,
    },

    /// Invalid or device-dependent color space
    InvalidColorSpace {
        /// Name of the invalid color space
        color_space: String,
        /// Location where it was found
        location: String,
    },

    /// Missing output intent for device-dependent colors
    MissingOutputIntent,

    /// Transparency is forbidden (PDF/A-1b)
    TransparencyForbidden {
        /// Location where transparency was found
        location: String,
    },

    /// External reference found (forbidden in PDF/A)
    ExternalReferenceForbidden {
        /// Type of external reference
        reference_type: String,
    },

    /// LZW compression is forbidden (PDF/A-1b)
    LzwCompressionForbidden {
        /// Object ID where LZW was found
        object_id: String,
    },

    /// PDF version is incompatible with the requested PDF/A level
    IncompatiblePdfVersion {
        /// Actual PDF version
        actual: String,
        /// Required PDF version
        required: String,
    },

    /// Embedded file is not allowed (PDF/A-1b, PDF/A-2b)
    EmbeddedFileForbidden,

    /// Embedded file is missing required metadata (PDF/A-3b)
    EmbeddedFileMissingMetadata {
        /// Name of the file
        file_name: String,
        /// Missing field
        missing_field: String,
    },

    /// Actions are forbidden (certain action types)
    ActionForbidden {
        /// Type of action
        action_type: String,
    },
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EncryptionForbidden => {
                write!(f, "Encryption is forbidden in PDF/A documents")
            }
            Self::FontNotEmbedded { font_name } => {
                write!(f, "Font '{}' is not embedded in the document", font_name)
            }
            Self::FontMissingToUnicode { font_name } => {
                write!(f, "Font '{}' is missing required ToUnicode CMap", font_name)
            }
            Self::JavaScriptForbidden { location } => {
                write!(
                    f,
                    "JavaScript is forbidden in PDF/A (found at {})",
                    location
                )
            }
            Self::XmpMetadataMissing => {
                write!(f, "XMP metadata is required but missing")
            }
            Self::XmpMissingPdfAIdentifier => {
                write!(f, "XMP metadata is missing PDF/A identification")
            }
            Self::XmpInvalidPdfAIdentifier { details } => {
                write!(f, "Invalid PDF/A identifier in XMP metadata: {}", details)
            }
            Self::InvalidColorSpace {
                color_space,
                location,
            } => {
                write!(
                    f,
                    "Invalid color space '{}' at {} (device-independent color spaces required)",
                    color_space, location
                )
            }
            Self::MissingOutputIntent => {
                write!(
                    f,
                    "Output intent is required when using device-dependent color spaces"
                )
            }
            Self::TransparencyForbidden { location } => {
                write!(
                    f,
                    "Transparency is forbidden in PDF/A-1 (found at {})",
                    location
                )
            }
            Self::ExternalReferenceForbidden { reference_type } => {
                write!(
                    f,
                    "External references are forbidden in PDF/A (type: {})",
                    reference_type
                )
            }
            Self::LzwCompressionForbidden { object_id } => {
                write!(
                    f,
                    "LZW compression is forbidden in PDF/A-1 (object {})",
                    object_id
                )
            }
            Self::IncompatiblePdfVersion { actual, required } => {
                write!(
                    f,
                    "PDF version {} is incompatible (required: {})",
                    actual, required
                )
            }
            Self::EmbeddedFileForbidden => {
                write!(f, "Embedded files are forbidden in PDF/A-1 and PDF/A-2")
            }
            Self::EmbeddedFileMissingMetadata {
                file_name,
                missing_field,
            } => {
                write!(
                    f,
                    "Embedded file '{}' is missing required metadata: {}",
                    file_name, missing_field
                )
            }
            Self::ActionForbidden { action_type } => {
                write!(f, "Action type '{}' is forbidden in PDF/A", action_type)
            }
        }
    }
}

impl std::error::Error for ValidationError {}

/// General errors for PDF/A operations
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PdfAError {
    /// Validation error
    Validation(ValidationError),

    /// XMP parsing error
    XmpParseError(String),

    /// Invalid PDF/A level string
    InvalidLevel(String),

    /// Document parsing error
    ParseError(String),

    /// IO error (as string for Clone)
    IoError(String),
}

impl fmt::Display for PdfAError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Validation(err) => write!(f, "PDF/A validation error: {}", err),
            Self::XmpParseError(msg) => write!(f, "XMP parsing error: {}", msg),
            Self::InvalidLevel(level) => write!(f, "Invalid PDF/A level: '{}'", level),
            Self::ParseError(msg) => write!(f, "PDF parsing error: {}", msg),
            Self::IoError(msg) => write!(f, "IO error: {}", msg),
        }
    }
}

impl std::error::Error for PdfAError {}

impl From<ValidationError> for PdfAError {
    fn from(err: ValidationError) -> Self {
        PdfAError::Validation(err)
    }
}

/// Result type for PDF/A operations
pub type PdfAResult<T> = std::result::Result<T, PdfAError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validation_error_display_encryption() {
        let err = ValidationError::EncryptionForbidden;
        assert!(err.to_string().contains("Encryption"));
        assert!(err.to_string().contains("forbidden"));
    }

    #[test]
    fn test_validation_error_display_font_not_embedded() {
        let err = ValidationError::FontNotEmbedded {
            font_name: "Arial".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("Arial"));
        assert!(msg.contains("not embedded"));
    }

    #[test]
    fn test_validation_error_display_javascript() {
        let err = ValidationError::JavaScriptForbidden {
            location: "Page 1".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("JavaScript"));
        assert!(msg.contains("Page 1"));
    }

    #[test]
    fn test_validation_error_display_xmp_missing() {
        let err = ValidationError::XmpMetadataMissing;
        assert!(err.to_string().contains("XMP"));
        assert!(err.to_string().contains("missing"));
    }

    #[test]
    fn test_validation_error_display_invalid_colorspace() {
        let err = ValidationError::InvalidColorSpace {
            color_space: "DeviceRGB".to_string(),
            location: "Image XObject".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("DeviceRGB"));
        assert!(msg.contains("Image XObject"));
    }

    #[test]
    fn test_validation_error_display_transparency() {
        let err = ValidationError::TransparencyForbidden {
            location: "Page 3".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("Transparency"));
        assert!(msg.contains("Page 3"));
    }

    #[test]
    fn test_validation_error_display_lzw() {
        let err = ValidationError::LzwCompressionForbidden {
            object_id: "15 0".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("LZW"));
        assert!(msg.contains("15 0"));
    }

    #[test]
    fn test_validation_error_display_pdf_version() {
        let err = ValidationError::IncompatiblePdfVersion {
            actual: "1.7".to_string(),
            required: "1.4".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("1.7"));
        assert!(msg.contains("1.4"));
    }

    #[test]
    fn test_pdfa_error_from_validation_error() {
        let validation_err = ValidationError::EncryptionForbidden;
        let pdfa_err: PdfAError = validation_err.into();
        assert!(matches!(pdfa_err, PdfAError::Validation(_)));
    }

    #[test]
    fn test_pdfa_error_display() {
        let err = PdfAError::InvalidLevel("PDF/A-4".to_string());
        assert!(err.to_string().contains("PDF/A-4"));
    }

    #[test]
    fn test_validation_error_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<ValidationError>();
        assert_send_sync::<PdfAError>();
    }

    #[test]
    fn test_validation_error_clone() {
        let err = ValidationError::FontNotEmbedded {
            font_name: "Times".to_string(),
        };
        let cloned = err.clone();
        assert_eq!(err, cloned);
    }

    #[test]
    fn test_validation_error_debug() {
        let err = ValidationError::XmpMetadataMissing;
        let debug_str = format!("{:?}", err);
        assert!(debug_str.contains("XmpMetadataMissing"));
    }
}
