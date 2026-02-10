//! Error types for digital signature operations

use std::fmt;

/// Result type for signature operations
pub type SignatureResult<T> = Result<T, SignatureError>;

/// Errors that can occur during signature operations
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SignatureError {
    /// Missing required field in signature dictionary
    MissingField {
        /// Name of the missing field
        field: String,
    },

    /// Invalid ByteRange format
    InvalidByteRange {
        /// Description of what's wrong
        details: String,
    },

    /// Invalid signature dictionary structure
    InvalidSignatureDict {
        /// Description of the issue
        details: String,
    },

    /// Signature contents extraction failed
    ContentsExtractionFailed {
        /// Description of the failure
        details: String,
    },

    /// AcroForm not found in document
    AcroFormNotFound,

    /// No signature fields in document
    NoSignatureFields,

    /// PDF parsing error during signature extraction
    ParseError {
        /// The underlying error message
        message: String,
    },

    /// CMS/PKCS#7 structure parsing failed
    CmsParsingFailed {
        /// Description of the parsing failure
        details: String,
    },

    /// Unsupported cryptographic algorithm
    UnsupportedAlgorithm {
        /// The algorithm that is not supported
        algorithm: String,
    },
}

impl fmt::Display for SignatureError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingField { field } => {
                write!(f, "Missing required signature field: {}", field)
            }
            Self::InvalidByteRange { details } => {
                write!(f, "Invalid ByteRange format: {}", details)
            }
            Self::InvalidSignatureDict { details } => {
                write!(f, "Invalid signature dictionary: {}", details)
            }
            Self::ContentsExtractionFailed { details } => {
                write!(f, "Failed to extract signature contents: {}", details)
            }
            Self::AcroFormNotFound => {
                write!(f, "Document does not contain an AcroForm dictionary")
            }
            Self::NoSignatureFields => {
                write!(f, "No signature fields found in document")
            }
            Self::ParseError { message } => {
                write!(f, "PDF parsing error: {}", message)
            }
            Self::CmsParsingFailed { details } => {
                write!(f, "CMS/PKCS#7 parsing failed: {}", details)
            }
            Self::UnsupportedAlgorithm { algorithm } => {
                write!(f, "Unsupported algorithm: {}", algorithm)
            }
        }
    }
}

impl std::error::Error for SignatureError {}

impl From<crate::error::PdfError> for SignatureError {
    fn from(err: crate::error::PdfError) -> Self {
        SignatureError::ParseError {
            message: err.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_missing_field_error_display() {
        let err = SignatureError::MissingField {
            field: "Filter".to_string(),
        };
        assert!(err.to_string().contains("Filter"));
        assert!(err.to_string().contains("Missing"));
    }

    #[test]
    fn test_invalid_byterange_error_display() {
        let err = SignatureError::InvalidByteRange {
            details: "expected 4 elements".to_string(),
        };
        assert!(err.to_string().contains("ByteRange"));
        assert!(err.to_string().contains("4 elements"));
    }

    #[test]
    fn test_acroform_not_found_error_display() {
        let err = SignatureError::AcroFormNotFound;
        assert!(err.to_string().contains("AcroForm"));
    }

    #[test]
    fn test_error_is_std_error() {
        fn assert_error<E: std::error::Error>() {}
        assert_error::<SignatureError>();
    }

    #[test]
    fn test_error_clone_eq() {
        let err1 = SignatureError::NoSignatureFields;
        let err2 = err1.clone();
        assert_eq!(err1, err2);
    }

    #[test]
    fn test_all_error_variants_display() {
        let errors = vec![
            SignatureError::MissingField {
                field: "SubFilter".to_string(),
            },
            SignatureError::InvalidByteRange {
                details: "negative value".to_string(),
            },
            SignatureError::InvalidSignatureDict {
                details: "not a dictionary".to_string(),
            },
            SignatureError::ContentsExtractionFailed {
                details: "hex decode failed".to_string(),
            },
            SignatureError::AcroFormNotFound,
            SignatureError::NoSignatureFields,
            SignatureError::ParseError {
                message: "unexpected EOF".to_string(),
            },
            SignatureError::CmsParsingFailed {
                details: "invalid DER".to_string(),
            },
            SignatureError::UnsupportedAlgorithm {
                algorithm: "MD5".to_string(),
            },
        ];

        for err in errors {
            let display = err.to_string();
            assert!(!display.is_empty(), "Error display should not be empty");
        }
    }

    #[test]
    fn test_cms_parsing_failed_error_display() {
        let err = SignatureError::CmsParsingFailed {
            details: "invalid ContentInfo".to_string(),
        };
        assert!(err.to_string().contains("CMS"));
        assert!(err.to_string().contains("invalid ContentInfo"));
    }

    #[test]
    fn test_unsupported_algorithm_error_display() {
        let err = SignatureError::UnsupportedAlgorithm {
            algorithm: "MD5".to_string(),
        };
        assert!(err.to_string().contains("algorithm"));
        assert!(err.to_string().contains("MD5"));
    }
}
