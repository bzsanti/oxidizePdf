//! PDF operations module
//!
//! This module provides high-level operations for manipulating PDF documents
//! such as splitting, merging, rotating pages, and reordering.

pub mod extract_images;
pub mod merge;
pub mod page_analysis;
pub mod page_extraction;
pub mod reorder;
pub mod rotate;
pub mod split;

pub use extract_images::{
    extract_images_from_pages, extract_images_from_pdf, ExtractImagesOptions, ExtractedImage,
    ImageExtractor,
};
pub use merge::{merge_pdf_files, merge_pdfs, MergeInput, MergeOptions, PdfMerger};
pub use page_analysis::{AnalysisOptions, ContentAnalysis, PageContentAnalyzer, PageType};
pub use page_extraction::{
    extract_page, extract_page_range, extract_page_range_to_file, extract_page_to_file,
    extract_pages, extract_pages_to_file, PageExtractionOptions, PageExtractor,
};
pub use reorder::{
    move_pdf_page, reorder_pdf_pages, reverse_pdf_pages, swap_pdf_pages, PageReorderer,
    ReorderOptions,
};
pub use rotate::{rotate_all_pages, rotate_pdf_pages, PageRotator, RotateOptions, RotationAngle};
pub use split::{split_into_pages, split_pdf, PdfSplitter, SplitMode, SplitOptions};

use crate::error::PdfError;

/// Result type for operations
pub type OperationResult<T> = Result<T, OperationError>;

/// Operation-specific errors
#[derive(Debug, thiserror::Error)]
pub enum OperationError {
    /// Page index out of bounds
    #[error("Page index {0} out of bounds (document has {1} pages)")]
    PageIndexOutOfBounds(usize, usize),

    /// Invalid page range
    #[error("Invalid page range: {0}")]
    InvalidPageRange(String),

    /// No pages to process
    #[error("No pages to process")]
    NoPagesToProcess,

    /// Resource conflict during merge
    #[error("Resource conflict: {0}")]
    ResourceConflict(String),

    /// Invalid rotation angle
    #[error("Invalid rotation angle: {0} (must be 0, 90, 180, or 270)")]
    InvalidRotation(i32),

    /// Parse error
    #[error("Parse error: {0}")]
    ParseError(String),

    /// IO error
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Core PDF error
    #[error("PDF error: {0}")]
    PdfError(#[from] PdfError),

    /// General processing error
    #[error("Processing error: {0}")]
    ProcessingError(String),
}

/// Page range specification
#[derive(Debug, Clone)]
pub enum PageRange {
    /// All pages
    All,
    /// Single page (0-based index)
    Single(usize),
    /// Range of pages (inclusive, 0-based)
    Range(usize, usize),
    /// List of specific pages (0-based indices)
    List(Vec<usize>),
}

impl PageRange {
    /// Parse a page range from a string
    ///
    /// Examples:
    /// - "all" -> All pages
    /// - "1" -> Single page (converts to 0-based)
    /// - "1-5" -> Range of pages (converts to 0-based)
    /// - "1,3,5" -> List of pages (converts to 0-based)
    pub fn parse(s: &str) -> Result<Self, OperationError> {
        let s = s.trim();

        if s.eq_ignore_ascii_case("all") {
            return Ok(PageRange::All);
        }

        // Try single page
        if let Ok(page) = s.parse::<usize>() {
            if page == 0 {
                return Err(OperationError::InvalidPageRange(
                    "Page numbers start at 1".to_string(),
                ));
            }
            return Ok(PageRange::Single(page - 1));
        }

        // Try range (e.g., "1-5")
        if let Some((start, end)) = s.split_once('-') {
            let start = start
                .trim()
                .parse::<usize>()
                .map_err(|_| OperationError::InvalidPageRange(format!("Invalid start: {start}")))?;
            let end = end
                .trim()
                .parse::<usize>()
                .map_err(|_| OperationError::InvalidPageRange(format!("Invalid end: {end}")))?;

            if start == 0 || end == 0 {
                return Err(OperationError::InvalidPageRange(
                    "Page numbers start at 1".to_string(),
                ));
            }

            if start > end {
                return Err(OperationError::InvalidPageRange(format!(
                    "Start {start} is greater than end {end}"
                )));
            }

            return Ok(PageRange::Range(start - 1, end - 1));
        }

        // Try list (e.g., "1,3,5")
        if s.contains(',') {
            let pages: Result<Vec<usize>, _> = s
                .split(',')
                .map(|p| {
                    let page = p.trim().parse::<usize>().map_err(|_| {
                        OperationError::InvalidPageRange(format!("Invalid page: {p}"))
                    })?;
                    if page == 0 {
                        return Err(OperationError::InvalidPageRange(
                            "Page numbers start at 1".to_string(),
                        ));
                    }
                    Ok(page - 1)
                })
                .collect();

            return Ok(PageRange::List(pages?));
        }

        Err(OperationError::InvalidPageRange(format!(
            "Invalid format: {s}"
        )))
    }

    /// Get the page indices for this range
    pub fn get_indices(&self, total_pages: usize) -> Result<Vec<usize>, OperationError> {
        match self {
            PageRange::All => Ok((0..total_pages).collect()),
            PageRange::Single(idx) => {
                if *idx >= total_pages {
                    Err(OperationError::PageIndexOutOfBounds(*idx, total_pages))
                } else {
                    Ok(vec![*idx])
                }
            }
            PageRange::Range(start, end) => {
                if *start >= total_pages {
                    Err(OperationError::PageIndexOutOfBounds(*start, total_pages))
                } else if *end >= total_pages {
                    Err(OperationError::PageIndexOutOfBounds(*end, total_pages))
                } else {
                    Ok((*start..=*end).collect())
                }
            }
            PageRange::List(pages) => {
                for &page in pages {
                    if page >= total_pages {
                        return Err(OperationError::PageIndexOutOfBounds(page, total_pages));
                    }
                }
                Ok(pages.clone())
            }
        }
    }
}

#[cfg(test)]
mod error_tests;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_page_range_parsing() {
        assert!(matches!(PageRange::parse("all").unwrap(), PageRange::All));
        assert!(matches!(PageRange::parse("ALL").unwrap(), PageRange::All));

        match PageRange::parse("5").unwrap() {
            PageRange::Single(idx) => assert_eq!(idx, 4),
            _ => panic!("Expected Single"),
        }

        match PageRange::parse("2-5").unwrap() {
            PageRange::Range(start, end) => {
                assert_eq!(start, 1);
                assert_eq!(end, 4);
            }
            _ => panic!("Expected Range"),
        }

        match PageRange::parse("1,3,5,7").unwrap() {
            PageRange::List(pages) => {
                assert_eq!(pages, vec![0, 2, 4, 6]);
            }
            _ => panic!("Expected List"),
        }

        assert!(PageRange::parse("0").is_err());
        assert!(PageRange::parse("5-2").is_err());
        assert!(PageRange::parse("invalid").is_err());
    }

    #[test]
    fn test_page_range_indices() {
        let total = 10;

        assert_eq!(
            PageRange::All.get_indices(total).unwrap(),
            vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9]
        );

        assert_eq!(PageRange::Single(5).get_indices(total).unwrap(), vec![5]);

        assert_eq!(
            PageRange::Range(2, 5).get_indices(total).unwrap(),
            vec![2, 3, 4, 5]
        );

        assert_eq!(
            PageRange::List(vec![1, 3, 5]).get_indices(total).unwrap(),
            vec![1, 3, 5]
        );

        assert!(PageRange::Single(10).get_indices(total).is_err());
        assert!(PageRange::Range(8, 15).get_indices(total).is_err());
    }

    #[test]
    fn test_module_exports() {
        // Verify that all operation types are exported correctly
        // This test just ensures the module structure is correct

        // We can create these types through their modules
        use super::extract_images::ExtractImagesOptions;
        use super::merge::MergeOptions;
        use super::page_analysis::{AnalysisOptions, PageType};
        use super::page_extraction::PageExtractionOptions;
        use super::rotate::{RotateOptions, RotationAngle};
        use super::split::{SplitMode, SplitOptions};

        // Just verify we can access these types
        let _extract: ExtractImagesOptions;
        let _merge: MergeOptions;
        let _analysis: AnalysisOptions;
        let _extraction: PageExtractionOptions;
        let _rotate: RotateOptions;
        let _split: SplitOptions;
        let _angle: RotationAngle;
        let _page_type: PageType;
        let _mode: SplitMode;
    }

    #[test]
    fn test_operation_error_variants() {
        let errors = vec![
            OperationError::PageIndexOutOfBounds(5, 3),
            OperationError::InvalidPageRange("test".to_string()),
            OperationError::NoPagesToProcess,
            OperationError::ResourceConflict("test".to_string()),
            OperationError::InvalidRotation(45),
            OperationError::ParseError("test".to_string()),
            OperationError::ProcessingError("test".to_string()),
        ];

        for error in errors {
            let message = error.to_string();
            assert!(!message.is_empty());
        }
    }

    #[test]
    fn test_page_range_edge_cases() {
        // Test whitespace handling
        assert!(matches!(
            PageRange::parse("  all  ").unwrap(),
            PageRange::All
        ));
        assert!(matches!(
            PageRange::parse(" 5 ").unwrap(),
            PageRange::Single(4)
        ));

        // Test various list formats
        match PageRange::parse(" 1 , 3 , 5 ").unwrap() {
            PageRange::List(pages) => assert_eq!(pages, vec![0, 2, 4]),
            _ => panic!("Expected List"),
        }

        // Test range with spaces
        match PageRange::parse(" 2 - 5 ").unwrap() {
            PageRange::Range(start, end) => {
                assert_eq!(start, 1);
                assert_eq!(end, 4);
            }
            _ => panic!("Expected Range"),
        }
    }

    #[test]
    fn test_page_range_invalid_formats() {
        // Test various invalid formats
        assert!(PageRange::parse("").is_err());
        assert!(PageRange::parse("abc").is_err());
        assert!(PageRange::parse("1-").is_err());
        assert!(PageRange::parse("-5").is_err());
        assert!(PageRange::parse("1-2-3").is_err());
        assert!(PageRange::parse("1,0,3").is_err());
        assert!(PageRange::parse("0-5").is_err());
        assert!(PageRange::parse("5-0").is_err());
        assert!(PageRange::parse("1,,3").is_err());
        assert!(PageRange::parse("1.5").is_err());
    }

    #[test]
    fn test_page_range_get_indices_empty_document() {
        let total = 0;

        assert_eq!(PageRange::All.get_indices(total).unwrap(), vec![]);
        assert!(PageRange::Single(0).get_indices(total).is_err());
        assert!(PageRange::Range(0, 1).get_indices(total).is_err());
        assert!(PageRange::List(vec![0]).get_indices(total).is_err());
    }

    #[test]
    fn test_page_range_get_indices_single_page_document() {
        let total = 1;

        assert_eq!(PageRange::All.get_indices(total).unwrap(), vec![0]);
        assert_eq!(PageRange::Single(0).get_indices(total).unwrap(), vec![0]);
        assert!(PageRange::Single(1).get_indices(total).is_err());
        assert_eq!(PageRange::Range(0, 0).get_indices(total).unwrap(), vec![0]);
        assert!(PageRange::Range(0, 1).get_indices(total).is_err());
    }

    #[test]
    fn test_page_range_list_duplicates() {
        // Lists can have duplicates in our implementation
        match PageRange::parse("1,1,2,2,3").unwrap() {
            PageRange::List(pages) => {
                assert_eq!(pages, vec![0, 0, 1, 1, 2]);
            }
            _ => panic!("Expected List"),
        }
    }

    #[test]
    fn test_page_range_list_unordered() {
        // Lists don't need to be ordered
        match PageRange::parse("5,2,8,1,3").unwrap() {
            PageRange::List(pages) => {
                assert_eq!(pages, vec![4, 1, 7, 0, 2]);
            }
            _ => panic!("Expected List"),
        }
    }

    #[test]
    fn test_operation_error_display() {
        let error = OperationError::PageIndexOutOfBounds(10, 5);
        assert_eq!(
            error.to_string(),
            "Page index 10 out of bounds (document has 5 pages)"
        );

        let error = OperationError::InvalidRotation(45);
        assert_eq!(
            error.to_string(),
            "Invalid rotation angle: 45 (must be 0, 90, 180, or 270)"
        );

        let error = OperationError::NoPagesToProcess;
        assert_eq!(error.to_string(), "No pages to process");
    }

    #[test]
    fn test_page_range_large_document() {
        let total = 1000;

        // Test all pages
        let indices = PageRange::All.get_indices(total).unwrap();
        assert_eq!(indices.len(), 1000);
        assert_eq!(indices[0], 0);
        assert_eq!(indices[999], 999);

        // Test large range
        let indices = PageRange::Range(100, 200).get_indices(total).unwrap();
        assert_eq!(indices.len(), 101);
        assert_eq!(indices[0], 100);
        assert_eq!(indices[100], 200);
    }

    #[test]
    fn test_page_range_parse_case_insensitive() {
        assert!(matches!(PageRange::parse("all").unwrap(), PageRange::All));
        assert!(matches!(PageRange::parse("ALL").unwrap(), PageRange::All));
        assert!(matches!(PageRange::parse("All").unwrap(), PageRange::All));
        assert!(matches!(PageRange::parse("aLL").unwrap(), PageRange::All));
    }

    #[test]
    fn test_operation_result_type() {
        // Test that OperationResult works correctly
        fn test_function() -> OperationResult<usize> {
            Ok(42)
        }

        fn test_error_function() -> OperationResult<usize> {
            Err(OperationError::NoPagesToProcess)
        }

        assert_eq!(test_function().unwrap(), 42);
        assert!(test_error_function().is_err());
    }

    #[test]
    fn test_page_range_boundary_values() {
        // Test maximum safe values
        let large_page = usize::MAX / 2;

        match PageRange::parse(&large_page.to_string()).unwrap() {
            PageRange::Single(idx) => assert_eq!(idx, large_page - 1),
            _ => panic!("Expected Single"),
        }

        // Test with actual document
        let indices = PageRange::Single(5).get_indices(10).unwrap();
        assert_eq!(indices, vec![5]);

        // Test range boundary
        let indices = PageRange::Range(0, 9).get_indices(10).unwrap();
        assert_eq!(indices.len(), 10);
    }

    #[test]
    fn test_error_from_io() {
        use std::io;

        let io_error = io::Error::new(io::ErrorKind::NotFound, "File not found");
        let op_error: OperationError = io_error.into();

        match op_error {
            OperationError::Io(_) => {}
            _ => panic!("Expected Io variant"),
        }
    }

    #[test]
    fn test_page_range_fmt_debug() {
        // Test Debug implementation
        let range = PageRange::All;
        let debug_str = format!("{:?}", range);
        assert!(debug_str.contains("All"));

        let range = PageRange::Single(5);
        let debug_str = format!("{:?}", range);
        assert!(debug_str.contains("Single"));
        assert!(debug_str.contains("5"));

        let range = PageRange::Range(1, 10);
        let debug_str = format!("{:?}", range);
        assert!(debug_str.contains("Range"));

        let range = PageRange::List(vec![1, 2, 3]);
        let debug_str = format!("{:?}", range);
        assert!(debug_str.contains("List"));
    }

    #[test]
    fn test_page_range_clone() {
        let original = PageRange::List(vec![1, 2, 3]);
        let cloned = original.clone();

        match (original, cloned) {
            (PageRange::List(orig), PageRange::List(clone)) => {
                assert_eq!(orig, clone);
            }
            _ => panic!("Clone failed"),
        }
    }
}
