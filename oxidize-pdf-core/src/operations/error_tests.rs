#[cfg(test)]
mod tests {
    use super::super::{OperationError, PageRange};
    use crate::error::PdfError;
    use std::io;

    #[test]
    fn test_page_index_out_of_bounds() {
        let err = OperationError::PageIndexOutOfBounds(10, 5);
        assert_eq!(
            err.to_string(),
            "Page index 10 out of bounds (document has 5 pages)"
        );
    }

    #[test]
    fn test_invalid_page_range() {
        let err = OperationError::InvalidPageRange("invalid format".to_string());
        assert_eq!(err.to_string(), "Invalid page range: invalid format");
    }

    #[test]
    fn test_no_pages_to_process() {
        let err = OperationError::NoPagesToProcess;
        assert_eq!(err.to_string(), "No pages to process");
    }

    #[test]
    fn test_resource_conflict() {
        let err = OperationError::ResourceConflict("Font resource conflict".to_string());
        assert_eq!(err.to_string(), "Resource conflict: Font resource conflict");
    }

    #[test]
    fn test_invalid_rotation() {
        let err = OperationError::InvalidRotation(45);
        assert_eq!(
            err.to_string(),
            "Invalid rotation angle: 45 (must be 0, 90, 180, or 270)"
        );
    }

    #[test]
    fn test_parse_error() {
        let err = OperationError::ParseError("Invalid PDF structure".to_string());
        assert_eq!(err.to_string(), "Parse error: Invalid PDF structure");
    }

    #[test]
    fn test_io_error() {
        let io_err = io::Error::new(io::ErrorKind::NotFound, "File not found");
        let err = OperationError::Io(io_err);
        assert_eq!(err.to_string(), "IO error: File not found");
    }

    #[test]
    fn test_pdf_error() {
        let pdf_err = PdfError::InvalidStructure("Invalid header".to_string());
        let err = OperationError::PdfError(pdf_err);
        assert_eq!(
            err.to_string(),
            "PDF error: Invalid PDF structure: Invalid header"
        );
    }

    #[test]
    fn test_processing_error() {
        let err = OperationError::ProcessingError("Failed to process content".to_string());
        assert_eq!(
            err.to_string(),
            "Processing error: Failed to process content"
        );
    }

    #[test]
    fn test_from_io_error() {
        let io_err = io::Error::new(io::ErrorKind::PermissionDenied, "Access denied");
        let err: OperationError = io_err.into();
        assert!(matches!(err, OperationError::Io(_)));
        assert_eq!(err.to_string(), "IO error: Access denied");
    }

    #[test]
    fn test_from_pdf_error() {
        let pdf_err = PdfError::CompressionError("Invalid compression".to_string());
        let err: OperationError = pdf_err.into();
        assert!(matches!(err, OperationError::PdfError(_)));
        assert_eq!(
            err.to_string(),
            "PDF error: Compression error: Invalid compression"
        );
    }

    #[test]
    fn test_debug_format() {
        let err = OperationError::NoPagesToProcess;
        let debug_str = format!("{err:?}");
        assert!(debug_str.contains("NoPagesToProcess"));
    }

    #[test]
    fn test_error_chain() {
        // Test that errors can be properly chained
        let io_err = io::Error::other("Low level error");
        let op_err = OperationError::Io(io_err);

        // Verify we can access the source
        let source = std::error::Error::source(&op_err);
        assert!(source.is_some());
    }

    #[test]
    fn test_page_range_errors() {
        // Test page range parsing errors
        let result = PageRange::parse("0");
        assert!(result.is_err());
        if let Err(err) = result {
            assert!(matches!(err, OperationError::InvalidPageRange(_)));
            assert!(err.to_string().contains("Page numbers start at 1"));
        }

        let result = PageRange::parse("5-2");
        assert!(result.is_err());
        if let Err(err) = result {
            assert!(matches!(err, OperationError::InvalidPageRange(_)));
            assert!(err.to_string().contains("Start 5 is greater than end 2"));
        }
    }

    #[test]
    fn test_page_range_get_indices_errors() {
        let range = PageRange::Single(10);
        let result = range.get_indices(5);
        assert!(result.is_err());
        if let Err(err) = result {
            assert!(matches!(err, OperationError::PageIndexOutOfBounds(10, 5)));
        }

        let range = PageRange::Range(0, 10);
        let result = range.get_indices(5);
        assert!(result.is_err());
        if let Err(err) = result {
            assert!(matches!(err, OperationError::PageIndexOutOfBounds(10, 5)));
        }
    }

    #[test]
    fn test_error_send_sync() {
        // Ensure OperationError is Send + Sync for use across threads
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<OperationError>();
    }
}
