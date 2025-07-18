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
}
