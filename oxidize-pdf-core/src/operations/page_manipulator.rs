//! Page Manipulation API for PDF modification.
//!
//! This module provides operations for manipulating pages in existing PDFs:
//! - Cropping pages (logical viewport via CropBox)
//! - Resizing pages (MediaBox modification)
//! - Deleting pages
//!
//! # Example
//!
//! ```ignore
//! use oxidize_pdf::operations::{PdfEditor, PageManipulator, CropBox, ResizeMode};
//!
//! let mut editor = PdfEditor::from_bytes(pdf_bytes)?;
//!
//! // Crop page 0 with 50pt margins
//! PageManipulator::crop_page(&mut editor, 0, CropBox::from_margins(595.0, 842.0, 50.0)?)?;
//!
//! // Resize page 1 to Letter size
//! PageManipulator::resize_page(&mut editor, 1, ResizeMode::ToLetter)?;
//!
//! // Delete page 2
//! PageManipulator::delete_page(&mut editor, 2)?;
//!
//! editor.save_to_bytes()?;
//! ```

use std::fmt;
use thiserror::Error;

/// Error type for page manipulation operations.
#[derive(Debug, Clone, PartialEq, Error)]
pub enum PageManipulatorError {
    /// CropBox has invalid dimensions (left >= right or bottom >= top).
    #[error("invalid crop box: {reason}")]
    InvalidCropBox { reason: String },

    /// Cannot delete the last page of a document.
    #[error("cannot delete the last page of a document")]
    CannotDeleteLastPage,

    /// Page index is out of bounds.
    #[error("page index {index} is out of bounds (document has {page_count} pages)")]
    PageIndexOutOfBounds { index: usize, page_count: usize },

    /// Generic modification failure.
    #[error("page modification failed: {0}")]
    ModificationFailed(String),
}

/// Result type for page manipulation operations.
pub type PageManipulatorResult<T> = Result<T, PageManipulatorError>;

/// Represents a crop box for logical page cropping.
///
/// The CropBox defines the visible region of a page without modifying
/// the actual content. Coordinates are in PDF points (1/72 inch).
///
/// Per ISO 32000-1 §14.11.2, the CropBox defines the region to which
/// the contents of the page shall be clipped when displayed or printed.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CropBox {
    /// Left boundary (x minimum).
    pub left: f64,
    /// Bottom boundary (y minimum).
    pub bottom: f64,
    /// Right boundary (x maximum).
    pub right: f64,
    /// Top boundary (y maximum).
    pub top: f64,
}

impl CropBox {
    /// Creates a new CropBox with the specified boundaries.
    ///
    /// # Arguments
    ///
    /// * `left` - Left boundary (x minimum)
    /// * `bottom` - Bottom boundary (y minimum)
    /// * `right` - Right boundary (x maximum)
    /// * `top` - Top boundary (y maximum)
    ///
    /// # Errors
    ///
    /// Returns `InvalidCropBox` if left >= right or bottom >= top.
    pub fn new(left: f64, bottom: f64, right: f64, top: f64) -> PageManipulatorResult<Self> {
        if left >= right {
            return Err(PageManipulatorError::InvalidCropBox {
                reason: format!("left ({}) must be less than right ({})", left, right),
            });
        }
        if bottom >= top {
            return Err(PageManipulatorError::InvalidCropBox {
                reason: format!("bottom ({}) must be less than top ({})", bottom, top),
            });
        }
        Ok(Self {
            left,
            bottom,
            right,
            top,
        })
    }

    /// Creates a CropBox from page dimensions and uniform margins.
    ///
    /// # Arguments
    ///
    /// * `page_width` - Original page width in points
    /// * `page_height` - Original page height in points
    /// * `margin` - Uniform margin to apply on all sides
    ///
    /// # Errors
    ///
    /// Returns `InvalidCropBox` if margins would create an invalid box.
    pub fn from_margins(
        page_width: f64,
        page_height: f64,
        margin: f64,
    ) -> PageManipulatorResult<Self> {
        Self::new(margin, margin, page_width - margin, page_height - margin)
    }

    /// Creates a CropBox with different margins for each side.
    ///
    /// # Arguments
    ///
    /// * `page_width` - Original page width in points
    /// * `page_height` - Original page height in points
    /// * `left` - Left margin
    /// * `bottom` - Bottom margin
    /// * `right` - Right margin
    /// * `top` - Top margin
    pub fn from_margins_lbrt(
        page_width: f64,
        page_height: f64,
        left: f64,
        bottom: f64,
        right: f64,
        top: f64,
    ) -> PageManipulatorResult<Self> {
        Self::new(left, bottom, page_width - right, page_height - top)
    }

    /// Returns the width of the crop region.
    #[inline]
    pub fn width(&self) -> f64 {
        self.right - self.left
    }

    /// Returns the height of the crop region.
    #[inline]
    pub fn height(&self) -> f64 {
        self.top - self.bottom
    }

    /// Converts the CropBox to an array suitable for PDF output.
    #[inline]
    pub fn to_array(&self) -> [f64; 4] {
        [self.left, self.bottom, self.right, self.top]
    }
}

impl fmt::Display for CropBox {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "CropBox[{}, {}, {}, {}]",
            self.left, self.bottom, self.right, self.top
        )
    }
}

/// Resize mode for page resizing operations.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ResizeMode {
    /// Scale the page by a factor (e.g., 2.0 doubles the size).
    Scale(f64),
    /// Resize to specific dimensions in points.
    ToSize(f64, f64),
    /// Resize to A4 dimensions (595.28 x 841.89 points).
    ToA4,
    /// Resize to US Letter dimensions (612 x 792 points).
    ToLetter,
}

impl ResizeMode {
    /// A4 page width in points.
    pub const A4_WIDTH: f64 = 595.28;
    /// A4 page height in points.
    pub const A4_HEIGHT: f64 = 841.89;
    /// US Letter page width in points.
    pub const LETTER_WIDTH: f64 = 612.0;
    /// US Letter page height in points.
    pub const LETTER_HEIGHT: f64 = 792.0;

    /// Calculates the target dimensions for the given source dimensions.
    ///
    /// # Arguments
    ///
    /// * `source_width` - Original page width
    /// * `source_height` - Original page height
    ///
    /// # Returns
    ///
    /// Tuple of (target_width, target_height)
    pub fn target_dimensions(&self, source_width: f64, source_height: f64) -> (f64, f64) {
        match self {
            ResizeMode::Scale(factor) => (source_width * factor, source_height * factor),
            ResizeMode::ToSize(width, height) => (*width, *height),
            ResizeMode::ToA4 => (Self::A4_WIDTH, Self::A4_HEIGHT),
            ResizeMode::ToLetter => (Self::LETTER_WIDTH, Self::LETTER_HEIGHT),
        }
    }
}

/// Options for resize operations.
#[derive(Debug, Clone)]
pub struct ResizeOptions {
    /// The resize mode to apply.
    pub mode: ResizeMode,
    /// Whether to preserve aspect ratio (for ToSize mode).
    pub preserve_aspect_ratio: bool,
    /// Whether to scale content along with MediaBox.
    pub scale_content: bool,
}

impl Default for ResizeOptions {
    fn default() -> Self {
        Self {
            mode: ResizeMode::Scale(1.0),
            preserve_aspect_ratio: false,
            scale_content: false,
        }
    }
}

impl ResizeOptions {
    /// Creates resize options with scaling factor.
    pub fn scale(factor: f64) -> Self {
        Self {
            mode: ResizeMode::Scale(factor),
            ..Default::default()
        }
    }

    /// Creates resize options for specific target dimensions.
    pub fn to_size(width: f64, height: f64) -> Self {
        Self {
            mode: ResizeMode::ToSize(width, height),
            ..Default::default()
        }
    }

    /// Creates resize options for A4 dimensions.
    pub fn to_a4() -> Self {
        Self {
            mode: ResizeMode::ToA4,
            ..Default::default()
        }
    }

    /// Creates resize options for US Letter dimensions.
    pub fn to_letter() -> Self {
        Self {
            mode: ResizeMode::ToLetter,
            ..Default::default()
        }
    }

    /// Sets whether to preserve aspect ratio.
    pub fn with_preserve_aspect_ratio(mut self, preserve: bool) -> Self {
        self.preserve_aspect_ratio = preserve;
        self
    }

    /// Sets whether to scale content along with the page.
    pub fn with_scale_content(mut self, scale: bool) -> Self {
        self.scale_content = scale;
        self
    }
}

/// Page manipulation operations for existing PDFs.
///
/// This struct provides static methods for manipulating pages:
/// - Cropping (logical viewport via CropBox)
/// - Resizing (MediaBox modification)
/// - Deleting pages
pub struct PageManipulator;

impl PageManipulator {
    /// Crops a page to the specified CropBox.
    ///
    /// This sets the CropBox entry in the page dictionary, which defines
    /// the visible region of the page without modifying actual content.
    ///
    /// # Arguments
    ///
    /// * `editor` - The PdfEditor containing the document
    /// * `page_index` - Zero-based page index
    /// * `crop_box` - The crop box to apply
    ///
    /// # Errors
    ///
    /// Returns `PageIndexOutOfBounds` if page_index is invalid.
    pub fn crop_page(
        editor: &mut super::PdfEditor,
        page_index: usize,
        crop_box: CropBox,
    ) -> PageManipulatorResult<()> {
        let page_count = editor.page_count();
        if page_index >= page_count {
            return Err(PageManipulatorError::PageIndexOutOfBounds {
                index: page_index,
                page_count,
            });
        }

        // Store the crop box update
        editor.pending_crop_boxes.push((page_index, crop_box));
        Ok(())
    }

    /// Resizes a page according to the specified options.
    ///
    /// This modifies the MediaBox entry in the page dictionary.
    ///
    /// # Arguments
    ///
    /// * `editor` - The PdfEditor containing the document
    /// * `page_index` - Zero-based page index
    /// * `options` - Resize options including mode and flags
    ///
    /// # Errors
    ///
    /// Returns `PageIndexOutOfBounds` if page_index is invalid.
    pub fn resize_page(
        editor: &mut super::PdfEditor,
        page_index: usize,
        options: ResizeOptions,
    ) -> PageManipulatorResult<()> {
        let page_count = editor.page_count();
        if page_index >= page_count {
            return Err(PageManipulatorError::PageIndexOutOfBounds {
                index: page_index,
                page_count,
            });
        }

        // Get current page size
        let (current_width, current_height) = editor
            .get_page_size(page_index)
            .map_err(|e| PageManipulatorError::ModificationFailed(e.to_string()))?;

        // Calculate target dimensions
        let (target_width, target_height) = options
            .mode
            .target_dimensions(current_width, current_height);

        // Store the resize update
        editor.pending_resizes.push((
            page_index,
            target_width,
            target_height,
            options.scale_content,
        ));
        Ok(())
    }

    /// Deletes a page from the document.
    ///
    /// # Arguments
    ///
    /// * `editor` - The PdfEditor containing the document
    /// * `page_index` - Zero-based page index to delete
    ///
    /// # Errors
    ///
    /// Returns `CannotDeleteLastPage` if the document has only one page.
    /// Returns `PageIndexOutOfBounds` if page_index is invalid.
    pub fn delete_page(
        editor: &mut super::PdfEditor,
        page_index: usize,
    ) -> PageManipulatorResult<()> {
        let page_count = editor.page_count();

        // Check if it's the last page
        if page_count <= 1 {
            return Err(PageManipulatorError::CannotDeleteLastPage);
        }

        if page_index >= page_count {
            return Err(PageManipulatorError::PageIndexOutOfBounds {
                index: page_index,
                page_count,
            });
        }

        // Store the deletion
        editor.pending_deletions.push(page_index);
        Ok(())
    }

    /// Deletes multiple pages from the document.
    ///
    /// Pages are processed in reverse order to maintain correct indices.
    ///
    /// # Arguments
    ///
    /// * `editor` - The PdfEditor containing the document
    /// * `page_indices` - Zero-based page indices to delete
    ///
    /// # Errors
    ///
    /// Returns `CannotDeleteLastPage` if deleting all pages would leave none.
    /// Returns `PageIndexOutOfBounds` if any page_index is invalid.
    pub fn delete_pages(
        editor: &mut super::PdfEditor,
        page_indices: &[usize],
    ) -> PageManipulatorResult<usize> {
        let page_count = editor.page_count();
        let unique_indices: std::collections::HashSet<_> = page_indices.iter().copied().collect();

        // Check if we'd delete all pages
        if unique_indices.len() >= page_count {
            return Err(PageManipulatorError::CannotDeleteLastPage);
        }

        // Validate all indices
        for &index in &unique_indices {
            if index >= page_count {
                return Err(PageManipulatorError::PageIndexOutOfBounds { index, page_count });
            }
        }

        // Store all deletions
        let count = unique_indices.len();
        for index in unique_indices {
            editor.pending_deletions.push(index);
        }

        Ok(count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ==========================================================
    // T5.1 - CropBox::new() with valid values
    // ==========================================================
    #[test]
    fn test_crop_box_new() {
        let crop = CropBox::new(50.0, 50.0, 545.0, 792.0).unwrap();
        assert_eq!(crop.left, 50.0);
        assert_eq!(crop.bottom, 50.0);
        assert_eq!(crop.right, 545.0);
        assert_eq!(crop.top, 792.0);
    }

    // ==========================================================
    // T5.2 - CropBox::width() calculates correctly
    // ==========================================================
    #[test]
    fn test_crop_box_width() {
        let crop = CropBox::new(50.0, 50.0, 545.0, 792.0).unwrap();
        assert!((crop.width() - 495.0).abs() < f64::EPSILON);
    }

    // ==========================================================
    // T5.3 - CropBox::height() calculates correctly
    // ==========================================================
    #[test]
    fn test_crop_box_height() {
        let crop = CropBox::new(50.0, 50.0, 545.0, 792.0).unwrap();
        assert!((crop.height() - 742.0).abs() < f64::EPSILON);
    }

    // ==========================================================
    // T5.4 - CropBox invalid (left >= right)
    // ==========================================================
    #[test]
    fn test_crop_box_invalid_left_right() {
        let result = CropBox::new(545.0, 50.0, 50.0, 792.0);
        assert!(result.is_err());
        match result.unwrap_err() {
            PageManipulatorError::InvalidCropBox { reason } => {
                assert!(reason.contains("left"));
                assert!(reason.contains("right"));
            }
            e => panic!("Expected InvalidCropBox, got {:?}", e),
        }
    }

    #[test]
    fn test_crop_box_invalid_left_equals_right() {
        let result = CropBox::new(100.0, 50.0, 100.0, 792.0);
        assert!(result.is_err());
    }

    // ==========================================================
    // T5.5 - CropBox invalid (bottom >= top)
    // ==========================================================
    #[test]
    fn test_crop_box_invalid_bottom_top() {
        let result = CropBox::new(50.0, 792.0, 545.0, 50.0);
        assert!(result.is_err());
        match result.unwrap_err() {
            PageManipulatorError::InvalidCropBox { reason } => {
                assert!(reason.contains("bottom"));
                assert!(reason.contains("top"));
            }
            e => panic!("Expected InvalidCropBox, got {:?}", e),
        }
    }

    #[test]
    fn test_crop_box_invalid_bottom_equals_top() {
        let result = CropBox::new(50.0, 400.0, 545.0, 400.0);
        assert!(result.is_err());
    }

    // ==========================================================
    // T5.6 - CropBox::from_margins()
    // ==========================================================
    #[test]
    fn test_crop_box_from_margins() {
        // A4 page with 50pt uniform margins
        let crop = CropBox::from_margins(595.0, 842.0, 50.0).unwrap();
        assert_eq!(crop.left, 50.0);
        assert_eq!(crop.bottom, 50.0);
        assert_eq!(crop.right, 545.0);
        assert_eq!(crop.top, 792.0);
    }

    #[test]
    fn test_crop_box_from_margins_too_large() {
        // Margins larger than half the page
        let result = CropBox::from_margins(100.0, 100.0, 60.0);
        assert!(result.is_err());
    }

    #[test]
    fn test_crop_box_from_margins_lbrt() {
        let crop = CropBox::from_margins_lbrt(595.0, 842.0, 20.0, 30.0, 40.0, 50.0).unwrap();
        assert_eq!(crop.left, 20.0);
        assert_eq!(crop.bottom, 30.0);
        assert_eq!(crop.right, 555.0); // 595 - 40
        assert_eq!(crop.top, 792.0); // 842 - 50
    }

    // ==========================================================
    // T5.7 - ResizeOptions::scale() with factor 2.0
    // ==========================================================
    #[test]
    fn test_resize_options_scale() {
        let mode = ResizeMode::Scale(2.0);
        let (width, height) = mode.target_dimensions(595.0, 842.0);
        assert!((width - 1190.0).abs() < f64::EPSILON);
        assert!((height - 1684.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_resize_options_scale_factor_half() {
        let mode = ResizeMode::Scale(0.5);
        let (width, height) = mode.target_dimensions(595.0, 842.0);
        assert!((width - 297.5).abs() < f64::EPSILON);
        assert!((height - 421.0).abs() < f64::EPSILON);
    }

    // ==========================================================
    // T5.8 - ResizeOptions::to_size(width, height)
    // ==========================================================
    #[test]
    fn test_resize_options_to_size() {
        let mode = ResizeMode::ToSize(800.0, 600.0);
        let (width, height) = mode.target_dimensions(595.0, 842.0);
        assert!((width - 800.0).abs() < f64::EPSILON);
        assert!((height - 600.0).abs() < f64::EPSILON);
    }

    // ==========================================================
    // T5.9 - ResizeOptions::to_a4() uses standard A4 dimensions
    // ==========================================================
    #[test]
    fn test_resize_options_to_a4() {
        let mode = ResizeMode::ToA4;
        let (width, height) = mode.target_dimensions(612.0, 792.0); // Letter to A4
        assert!((width - 595.28).abs() < f64::EPSILON);
        assert!((height - 841.89).abs() < f64::EPSILON);
    }

    #[test]
    fn test_resize_options_a4_constants() {
        assert!((ResizeMode::A4_WIDTH - 595.28).abs() < f64::EPSILON);
        assert!((ResizeMode::A4_HEIGHT - 841.89).abs() < f64::EPSILON);
    }

    // ==========================================================
    // T5.10 - ResizeOptions::to_letter() uses Letter dimensions
    // ==========================================================
    #[test]
    fn test_resize_options_to_letter() {
        let mode = ResizeMode::ToLetter;
        let (width, height) = mode.target_dimensions(595.0, 842.0); // A4 to Letter
        assert!((width - 612.0).abs() < f64::EPSILON);
        assert!((height - 792.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_resize_options_letter_constants() {
        assert!((ResizeMode::LETTER_WIDTH - 612.0).abs() < f64::EPSILON);
        assert!((ResizeMode::LETTER_HEIGHT - 792.0).abs() < f64::EPSILON);
    }

    // ==========================================================
    // T5.18 - CropBox implements Clone, Debug, PartialEq
    // ==========================================================
    #[test]
    fn test_crop_box_clone_debug_eq() {
        let crop1 = CropBox::new(50.0, 50.0, 545.0, 792.0).unwrap();
        let crop2 = crop1;
        assert_eq!(crop1, crop2);

        // Debug
        let debug_str = format!("{:?}", crop1);
        assert!(debug_str.contains("CropBox"));
        assert!(debug_str.contains("50"));

        // Display
        let display_str = format!("{}", crop1);
        assert!(display_str.contains("CropBox"));
    }

    #[test]
    fn test_crop_box_not_equal() {
        let crop1 = CropBox::new(50.0, 50.0, 545.0, 792.0).unwrap();
        let crop2 = CropBox::new(60.0, 50.0, 545.0, 792.0).unwrap();
        assert_ne!(crop1, crop2);
    }

    #[test]
    fn test_crop_box_to_array() {
        let crop = CropBox::new(50.0, 60.0, 545.0, 792.0).unwrap();
        let arr = crop.to_array();
        assert_eq!(arr, [50.0, 60.0, 545.0, 792.0]);
    }

    // ==========================================================
    // T5.19 - PageManipulatorError variants Display correctly
    // ==========================================================
    #[test]
    fn test_page_manipulator_error_display_invalid_crop_box() {
        let err = PageManipulatorError::InvalidCropBox {
            reason: "test reason".to_string(),
        };
        let msg = format!("{}", err);
        assert!(msg.contains("invalid crop box"));
        assert!(msg.contains("test reason"));
    }

    #[test]
    fn test_page_manipulator_error_display_cannot_delete_last_page() {
        let err = PageManipulatorError::CannotDeleteLastPage;
        let msg = format!("{}", err);
        assert!(msg.contains("cannot delete the last page"));
    }

    #[test]
    fn test_page_manipulator_error_display_page_out_of_bounds() {
        let err = PageManipulatorError::PageIndexOutOfBounds {
            index: 5,
            page_count: 3,
        };
        let msg = format!("{}", err);
        assert!(msg.contains("5"));
        assert!(msg.contains("3"));
        assert!(msg.contains("out of bounds"));
    }

    #[test]
    fn test_page_manipulator_error_display_modification_failed() {
        let err = PageManipulatorError::ModificationFailed("write error".to_string());
        let msg = format!("{}", err);
        assert!(msg.contains("modification failed"));
        assert!(msg.contains("write error"));
    }

    #[test]
    fn test_page_manipulator_error_clone() {
        let err = PageManipulatorError::CannotDeleteLastPage;
        let cloned = err.clone();
        assert_eq!(err, cloned);
    }

    // ==========================================================
    // T5.20 - ResizeOptions implements Clone and Debug
    // ==========================================================
    #[test]
    fn test_resize_options_clone_debug() {
        let opts = ResizeOptions::scale(2.0);
        let cloned = opts.clone();
        assert!(matches!(cloned.mode, ResizeMode::Scale(f) if (f - 2.0).abs() < f64::EPSILON));

        let debug_str = format!("{:?}", opts);
        assert!(debug_str.contains("ResizeOptions"));
    }

    #[test]
    fn test_resize_options_default() {
        let opts = ResizeOptions::default();
        assert!(matches!(opts.mode, ResizeMode::Scale(f) if (f - 1.0).abs() < f64::EPSILON));
        assert!(!opts.preserve_aspect_ratio);
        assert!(!opts.scale_content);
    }

    #[test]
    fn test_resize_options_builders() {
        let opts = ResizeOptions::to_a4()
            .with_preserve_aspect_ratio(true)
            .with_scale_content(true);
        assert!(matches!(opts.mode, ResizeMode::ToA4));
        assert!(opts.preserve_aspect_ratio);
        assert!(opts.scale_content);
    }

    #[test]
    fn test_resize_mode_clone_eq() {
        let mode1 = ResizeMode::Scale(1.5);
        let mode2 = mode1;
        assert_eq!(mode1, mode2);
    }

    #[test]
    fn test_resize_mode_to_size_eq() {
        let mode1 = ResizeMode::ToSize(100.0, 200.0);
        let mode2 = ResizeMode::ToSize(100.0, 200.0);
        assert_eq!(mode1, mode2);

        let mode3 = ResizeMode::ToSize(100.0, 300.0);
        assert_ne!(mode1, mode3);
    }
}
