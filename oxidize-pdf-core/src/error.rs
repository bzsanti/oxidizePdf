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

    #[error("Encryption error: {0}")]
    EncryptionError(String),

    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    #[error("Invalid operation: {0}")]
    InvalidOperation(String),

    #[error("Duplicate field: {0}")]
    DuplicateField(String),

    #[error("Field not found: {0}")]
    FieldNotFound(String),

    #[error("External validation error: {0}")]
    ExternalValidationError(String),

    #[error("Internal error: {0}")]
    Internal(String),

    #[error("Serialization error: {0}")]
    SerializationError(String),
}

pub type Result<T> = std::result::Result<T, PdfError>;

// Convert AesError to PdfError
impl From<crate::encryption::AesError> for PdfError {
    fn from(err: crate::encryption::AesError) -> Self {
        PdfError::EncryptionError(err.to_string())
    }
}

impl From<crate::parser::ParseError> for PdfError {
    fn from(err: crate::parser::ParseError) -> Self {
        PdfError::ParseError(err.to_string())
    }
}

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
        let debug_str = format!("{error:?}");
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
        let debug_str = format!("{error:?}");
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

    #[test]
    fn test_operation_cancelled_error() {
        // Test the OperationCancelled variant (line 44-45)
        let error = PdfError::OperationCancelled;
        assert_eq!(error.to_string(), "Operation cancelled");

        // Test in a Result context
        let result: Result<()> = Err(PdfError::OperationCancelled);
        assert!(result.is_err());
        if let Err(PdfError::OperationCancelled) = result {
            // Variant matched correctly
        } else {
            panic!("Expected OperationCancelled variant");
        }
    }

    #[test]
    fn test_encryption_error() {
        // Test the EncryptionError variant (line 47-48)
        let error = PdfError::EncryptionError("AES decryption failed".to_string());
        assert_eq!(error.to_string(), "Encryption error: AES decryption failed");

        // Test debug format
        let debug_str = format!("{:?}", error);
        assert!(debug_str.contains("EncryptionError"));
        assert!(debug_str.contains("AES decryption failed"));
    }

    #[test]
    fn test_permission_denied_error() {
        // Test the PermissionDenied variant (line 50-51)
        let error = PdfError::PermissionDenied("Cannot modify protected document".to_string());
        assert_eq!(
            error.to_string(),
            "Permission denied: Cannot modify protected document"
        );

        // Test that it's different from InvalidOperation
        let other_error = PdfError::InvalidOperation("Cannot modify".to_string());
        assert_ne!(error.to_string(), other_error.to_string());
    }

    #[test]
    fn test_invalid_operation_error() {
        // Test the InvalidOperation variant (line 53-54)
        let error =
            PdfError::InvalidOperation("Cannot perform operation on encrypted PDF".to_string());
        assert_eq!(
            error.to_string(),
            "Invalid operation: Cannot perform operation on encrypted PDF"
        );

        // Test in match expression
        match error {
            PdfError::InvalidOperation(msg) => {
                assert!(msg.contains("encrypted"));
            }
            _ => panic!("Expected InvalidOperation variant"),
        }
    }

    #[test]
    fn test_duplicate_field_error() {
        // Test the DuplicateField variant (line 56-57)
        let field_name = "email_address";
        let error = PdfError::DuplicateField(field_name.to_string());
        assert_eq!(error.to_string(), "Duplicate field: email_address");

        // Test that it handles empty field names
        let empty_error = PdfError::DuplicateField(String::new());
        assert_eq!(empty_error.to_string(), "Duplicate field: ");
    }

    #[test]
    fn test_field_not_found_error() {
        // Test the FieldNotFound variant (line 59-60)
        let field_name = "signature_field";
        let error = PdfError::FieldNotFound(field_name.to_string());
        assert_eq!(error.to_string(), "Field not found: signature_field");

        // Test with special characters
        let special_field = "field[0].subfield";
        let special_error = PdfError::FieldNotFound(special_field.to_string());
        assert_eq!(
            special_error.to_string(),
            "Field not found: field[0].subfield"
        );
    }

    #[test]
    fn test_aes_error_conversion() {
        // Test the From<AesError> conversion (line 66-70)
        // We need to simulate an AesError
        use crate::encryption::AesError;

        let aes_error = AesError::InvalidKeyLength {
            expected: 32,
            actual: 16,
        };
        let pdf_error: PdfError = aes_error.into();

        match pdf_error {
            PdfError::EncryptionError(msg) => {
                assert!(msg.contains("Invalid key length") || msg.contains("InvalidKeyLength"));
            }
            _ => panic!("Expected EncryptionError from AesError conversion"),
        }
    }

    #[test]
    fn test_parse_error_conversion() {
        // Test the From<ParseError> conversion (line 72-76)
        use crate::parser::ParseError;

        let parse_error = ParseError::InvalidXRef;
        let pdf_error: PdfError = parse_error.into();

        match pdf_error {
            PdfError::ParseError(msg) => {
                assert!(msg.contains("XRef") || msg.contains("Invalid"));
            }
            _ => panic!("Expected ParseError from ParseError conversion"),
        }
    }
}
