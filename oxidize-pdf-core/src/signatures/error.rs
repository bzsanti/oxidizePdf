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

    /// ByteRange exceeds document size
    ByteRangeExceedsDocument {
        /// The byte range that's out of bounds
        offset: u64,
        /// The requested length
        length: u64,
        /// The document size
        document_size: u64,
    },

    /// Hash verification failed
    HashVerificationFailed {
        /// Description of the failure
        details: String,
    },

    /// Signature verification failed
    SignatureVerificationFailed {
        /// Description of the failure
        details: String,
    },

    /// Certificate extraction failed
    CertificateExtractionFailed {
        /// Description of the failure
        details: String,
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
            Self::ByteRangeExceedsDocument {
                offset,
                length,
                document_size,
            } => {
                write!(
                    f,
                    "ByteRange exceeds document: offset {} + length {} > document size {}",
                    offset, length, document_size
                )
            }
            Self::HashVerificationFailed { details } => {
                write!(f, "Hash verification failed: {}", details)
            }
            Self::SignatureVerificationFailed { details } => {
                write!(f, "Signature verification failed: {}", details)
            }
            Self::CertificateExtractionFailed { details } => {
                write!(f, "Certificate extraction failed: {}", details)
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
            SignatureError::ByteRangeExceedsDocument {
                offset: 1000,
                length: 500,
                document_size: 800,
            },
            SignatureError::HashVerificationFailed {
                details: "hash mismatch".to_string(),
            },
            SignatureError::SignatureVerificationFailed {
                details: "invalid signature".to_string(),
            },
            SignatureError::CertificateExtractionFailed {
                details: "no certificate".to_string(),
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

    #[test]
    fn test_byterange_exceeds_document_error_display() {
        let err = SignatureError::ByteRangeExceedsDocument {
            offset: 1000,
            length: 500,
            document_size: 800,
        };
        let display = err.to_string();
        assert!(display.contains("1000"));
        assert!(display.contains("500"));
        assert!(display.contains("800"));
        assert!(display.contains("exceeds"));
    }

    #[test]
    fn test_hash_verification_failed_error_display() {
        let err = SignatureError::HashVerificationFailed {
            details: "hash mismatch".to_string(),
        };
        assert!(err.to_string().contains("Hash verification failed"));
        assert!(err.to_string().contains("hash mismatch"));
    }

    #[test]
    fn test_signature_verification_failed_error_display() {
        let err = SignatureError::SignatureVerificationFailed {
            details: "invalid RSA signature".to_string(),
        };
        assert!(err.to_string().contains("Signature verification failed"));
        assert!(err.to_string().contains("invalid RSA signature"));
    }

    #[test]
    fn test_certificate_extraction_failed_error_display() {
        let err = SignatureError::CertificateExtractionFailed {
            details: "no certificate found".to_string(),
        };
        assert!(err.to_string().contains("Certificate extraction failed"));
        assert!(err.to_string().contains("no certificate found"));
    }
}
