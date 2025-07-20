use thiserror::Error;

#[derive(Error, Debug)]
pub enum PdfError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Invalid PDF structure: {0}")]
    InvalidStructure(String),

    #[error("Invalid object reference: {0}")]
    InvalidReference(String),

    #[error("Encoding error: {0}")]
    EncodingError(String),

    #[error("Font error: {0}")]
    FontError(String),

    #[error("Compression error: {0}")]
    CompressionError(String),

    #[error("Invalid image: {0}")]
    InvalidImage(String),

    #[error("Invalid object reference: {0} {1} R")]
    InvalidObjectReference(u32, u16),

    #[error("Parse error: {0}")]
    ParseError(String),

    #[error("Invalid page number: {0}")]
    InvalidPageNumber(u32),

    #[error("Invalid format: {0}")]
    InvalidFormat(String),

    #[error("Invalid header")]
    InvalidHeader,

    #[error("Content stream too large: {0} bytes")]
    ContentStreamTooLarge(usize),

    #[error("Operation cancelled")]
    OperationCancelled,
}

pub type Result<T> = std::result::Result<T, PdfError>;

// Separate error type for oxidize-pdf-core
#[derive(Error, Debug)]
pub enum OxidizePdfError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Parse error: {0}")]
    ParseError(String),

    #[error("Invalid PDF structure: {0}")]
    InvalidStructure(String),

    #[error("Encoding error: {0}")]
    EncodingError(String),

    #[error("Other error: {0}")]
    Other(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Error as IoError, ErrorKind};

    #[test]
    fn test_pdf_error_display() {
        let error = PdfError::InvalidStructure("test message".to_string());
        assert_eq!(error.to_string(), "Invalid PDF structure: test message");
    }

    #[test]
    fn test_pdf_error_debug() {
        let error = PdfError::InvalidReference("object 1 0".to_string());
        let debug_str = format!("{:?}", error);
        assert!(debug_str.contains("InvalidReference"));
        assert!(debug_str.contains("object 1 0"));
    }

    #[test]
    fn test_pdf_error_from_io_error() {
        let io_error = IoError::new(ErrorKind::NotFound, "file not found");
        let pdf_error = PdfError::from(io_error);

        match pdf_error {
            PdfError::Io(ref err) => {
                assert_eq!(err.kind(), ErrorKind::NotFound);
            }
            _ => panic!("Expected IO error variant"),
        }
    }

    #[test]
    fn test_all_pdf_error_variants() {
        let errors = vec![
            PdfError::InvalidStructure("structure error".to_string()),
            PdfError::InvalidObjectReference(1, 0),
            PdfError::EncodingError("encoding error".to_string()),
            PdfError::FontError("font error".to_string()),
            PdfError::CompressionError("compression error".to_string()),
            PdfError::InvalidImage("image error".to_string()),
            PdfError::ParseError("parse error".to_string()),
            PdfError::InvalidPageNumber(999),
            PdfError::InvalidFormat("format error".to_string()),
            PdfError::InvalidHeader,
            PdfError::ContentStreamTooLarge(1024 * 1024),
        ];

        // Test that all variants can be created and displayed
        for error in errors {
            let error_string = error.to_string();
            assert!(!error_string.is_empty());
        }
    }

    #[test]
    fn test_oxidize_pdf_error_display() {
        let error = OxidizePdfError::ParseError("parsing failed".to_string());
        assert_eq!(error.to_string(), "Parse error: parsing failed");
    }

    #[test]
    fn test_oxidize_pdf_error_debug() {
        let error = OxidizePdfError::InvalidStructure("malformed PDF".to_string());
        let debug_str = format!("{:?}", error);
        assert!(debug_str.contains("InvalidStructure"));
        assert!(debug_str.contains("malformed PDF"));
    }

    #[test]
    fn test_oxidize_pdf_error_from_io_error() {
        let io_error = IoError::new(ErrorKind::PermissionDenied, "access denied");
        let pdf_error = OxidizePdfError::from(io_error);

        match pdf_error {
            OxidizePdfError::Io(ref err) => {
                assert_eq!(err.kind(), ErrorKind::PermissionDenied);
            }
            _ => panic!("Expected IO error variant"),
        }
    }

    #[test]
    fn test_all_oxidize_pdf_error_variants() {
        let errors = vec![
            OxidizePdfError::ParseError("parse error".to_string()),
            OxidizePdfError::InvalidStructure("structure error".to_string()),
            OxidizePdfError::EncodingError("encoding error".to_string()),
            OxidizePdfError::Other("other error".to_string()),
        ];

        // Test that all variants can be created and displayed
        for error in errors {
            let error_string = error.to_string();
            assert!(!error_string.is_empty());
            assert!(error_string.contains("error"));
        }
    }

    #[test]
    fn test_result_type_ok() {
        let result: Result<i32> = Ok(42);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
    }

    #[test]
    fn test_result_type_err() {
        let result: Result<i32> = Err(PdfError::InvalidStructure("test".to_string()));
        assert!(result.is_err());

        let error = result.unwrap_err();
        match error {
            PdfError::InvalidStructure(msg) => assert_eq!(msg, "test"),
            _ => panic!("Expected InvalidStructure variant"),
        }
    }

    #[test]
    fn test_error_chain_display() {
        // Test that error messages are properly formatted
        let errors = [
            (
                "Invalid PDF structure: corrupted header",
                PdfError::InvalidStructure("corrupted header".to_string()),
            ),
            (
                "Invalid object reference: 999 0 R",
                PdfError::InvalidObjectReference(999, 0),
            ),
            (
                "Encoding error: unsupported encoding",
                PdfError::EncodingError("unsupported encoding".to_string()),
            ),
            (
                "Font error: missing font",
                PdfError::FontError("missing font".to_string()),
            ),
            (
                "Compression error: deflate failed",
                PdfError::CompressionError("deflate failed".to_string()),
            ),
            (
                "Invalid image: corrupt JPEG",
                PdfError::InvalidImage("corrupt JPEG".to_string()),
            ),
        ];

        for (expected, error) in errors {
            assert_eq!(error.to_string(), expected);
        }
    }

    #[test]
    fn test_oxidize_pdf_error_chain_display() {
        // Test that OxidizePdfError messages are properly formatted
        let errors = [
            (
                "Parse error: unexpected token",
                OxidizePdfError::ParseError("unexpected token".to_string()),
            ),
            (
                "Invalid PDF structure: missing xref",
                OxidizePdfError::InvalidStructure("missing xref".to_string()),
            ),
            (
                "Encoding error: invalid UTF-8",
                OxidizePdfError::EncodingError("invalid UTF-8".to_string()),
            ),
            (
                "Other error: unknown issue",
                OxidizePdfError::Other("unknown issue".to_string()),
            ),
        ];

        for (expected, error) in errors {
            assert_eq!(error.to_string(), expected);
        }
    }

    #[test]
    fn test_error_send_sync() {
        // Ensure error types implement Send + Sync for thread safety
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<PdfError>();
        assert_send_sync::<OxidizePdfError>();
    }

    #[test]
    fn test_error_struct_creation() {
        // Test creating errors with string messages
        let errors = vec![
            PdfError::InvalidStructure("test".to_string()),
            PdfError::InvalidObjectReference(1, 0),
            PdfError::EncodingError("encoding".to_string()),
            PdfError::FontError("font".to_string()),
            PdfError::CompressionError("compression".to_string()),
            PdfError::InvalidImage("image".to_string()),
            PdfError::ParseError("parse".to_string()),
            PdfError::InvalidPageNumber(1),
            PdfError::InvalidFormat("format".to_string()),
            PdfError::InvalidHeader,
            PdfError::ContentStreamTooLarge(1024),
            PdfError::OperationCancelled,
        ];

        // Verify each error can be created and has the expected message structure
        for error in errors {
            let msg = error.to_string();
            assert!(!msg.is_empty(), "Error message should not be empty");

            // Check that the message makes sense for the error type
            match &error {
                PdfError::OperationCancelled => assert!(msg.contains("cancelled")),
                PdfError::ContentStreamTooLarge(_) => assert!(msg.contains("too large")),
                _ => assert!(msg.contains("error") || msg.contains("Invalid")),
            }
        }
    }

    #[test]
    fn test_oxidize_pdf_error_struct_creation() {
        // Test creating OxidizePdfError with string messages
        let errors = vec![
            OxidizePdfError::ParseError("test".to_string()),
            OxidizePdfError::InvalidStructure("structure".to_string()),
            OxidizePdfError::EncodingError("encoding".to_string()),
            OxidizePdfError::Other("other".to_string()),
        ];

        // Verify each error can be created and has the expected message structure
        for error in errors {
            let msg = error.to_string();
            assert!(msg.contains("error") || msg.contains("Invalid"));
        }
    }

    #[test]
    fn test_error_equality() {
        let error1 = PdfError::InvalidStructure("test".to_string());
        let error2 = PdfError::InvalidStructure("test".to_string());
        let error3 = PdfError::InvalidStructure("different".to_string());

        // Note: thiserror doesn't automatically derive PartialEq, so we test the display output
        assert_eq!(error1.to_string(), error2.to_string());
        assert_ne!(error1.to_string(), error3.to_string());
    }

    #[test]
    fn test_io_error_preservation() {
        // Test that IO error details are preserved through conversion
        let original_io_error = IoError::new(ErrorKind::UnexpectedEof, "sudden EOF");
        let pdf_error = PdfError::from(original_io_error);

        if let PdfError::Io(io_err) = pdf_error {
            assert_eq!(io_err.kind(), ErrorKind::UnexpectedEof);
            assert_eq!(io_err.to_string(), "sudden EOF");
        } else {
            panic!("IO error should be preserved as PdfError::Io");
        }
    }

    #[test]
    fn test_oxidize_pdf_error_io_error_preservation() {
        // Test that IO error details are preserved through conversion
        let original_io_error = IoError::new(ErrorKind::InvalidData, "corrupted data");
        let oxidize_error = OxidizePdfError::from(original_io_error);

        if let OxidizePdfError::Io(io_err) = oxidize_error {
            assert_eq!(io_err.kind(), ErrorKind::InvalidData);
            assert_eq!(io_err.to_string(), "corrupted data");
        } else {
            panic!("IO error should be preserved as OxidizePdfError::Io");
        }
    }
}
