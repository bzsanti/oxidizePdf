//! Watermark Module for PDF Modification
//!
//! This module provides functionality to add watermarks (text or image stamps)
//! to PDF pages. Watermarks can be placed in foreground or background layers
//! with configurable opacity, rotation, and positioning.
//!
//! # Example
//! ```ignore
//! use oxidize_pdf::operations::{PdfEditor, WatermarkSpec, WatermarkPosition, PageRange};
//!
//! let mut editor = PdfEditor::open("document.pdf")?;
//! let watermark = WatermarkSpec::text("CONFIDENTIAL")
//!     .with_opacity(0.3)
//!     .with_rotation(45.0)
//!     .with_position(WatermarkPosition::Center);
//! Watermarker::apply(&mut editor, watermark, PageRange::All)?;
//! editor.save("watermarked.pdf")?;
//! ```

use crate::graphics::Color;
use crate::text::Font;
use std::fmt;

/// Error type for watermark operations
#[derive(Debug, Clone)]
pub enum WatermarkError {
    /// Opacity value is out of valid range (0.0 to 1.0)
    InvalidOpacity(f32),
    /// Rotation angle is invalid
    InvalidRotation(f32),
    /// Text content is empty
    EmptyText,
    /// Page not found in document
    PageNotFound(usize),
    /// Page index out of bounds
    PageIndexOutOfBounds { index: usize, total: usize },
    /// Image format not supported
    UnsupportedImageFormat(String),
}

impl fmt::Display for WatermarkError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidOpacity(value) => {
                write!(
                    f,
                    "invalid opacity value {}: must be between 0.0 and 1.0",
                    value
                )
            }
            Self::InvalidRotation(value) => {
                write!(f, "invalid rotation angle: {}", value)
            }
            Self::EmptyText => write!(f, "watermark text cannot be empty"),
            Self::PageNotFound(index) => write!(f, "page {} not found", index),
            Self::PageIndexOutOfBounds { index, total } => {
                write!(
                    f,
                    "page index {} is out of bounds (total pages: {})",
                    index, total
                )
            }
            Self::UnsupportedImageFormat(format) => {
                write!(f, "unsupported image format: {}", format)
            }
        }
    }
}

impl std::error::Error for WatermarkError {}

/// Result type for watermark operations
pub type WatermarkResult<T> = Result<T, WatermarkError>;

/// Content of a watermark - either text or image
#[derive(Debug, Clone)]
pub enum WatermarkContent {
    /// Text watermark with font and size
    Text {
        /// The text to display
        text: String,
        /// Font to use
        font: Font,
        /// Font size in points
        font_size: f32,
        /// Text color
        color: Color,
    },
    /// Image watermark with raw bytes
    Image {
        /// Image data (JPEG or PNG)
        data: Vec<u8>,
        /// Width of image on page
        width: f32,
        /// Height of image on page
        height: f32,
    },
}

/// Position of watermark on the page
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WatermarkPosition {
    /// Center of the page
    Center,
    /// Top-left corner
    TopLeft,
    /// Top-right corner
    TopRight,
    /// Bottom-left corner
    BottomLeft,
    /// Bottom-right corner
    BottomRight,
    /// Custom position (x, y from bottom-left)
    Custom(f64, f64),
}

impl WatermarkPosition {
    /// Calculate actual coordinates given page dimensions
    ///
    /// # Arguments
    /// * `page_width` - Width of the page in points
    /// * `page_height` - Height of the page in points
    /// * `content_width` - Width of the watermark content
    /// * `content_height` - Height of the watermark content
    ///
    /// # Returns
    /// (x, y) coordinates for the watermark position
    pub fn calculate_coordinates(
        &self,
        page_width: f64,
        page_height: f64,
        content_width: f64,
        content_height: f64,
    ) -> (f64, f64) {
        const MARGIN: f64 = 50.0;

        match self {
            Self::Center => (
                (page_width - content_width) / 2.0,
                (page_height - content_height) / 2.0,
            ),
            Self::TopLeft => (MARGIN, page_height - content_height - MARGIN),
            Self::TopRight => (
                page_width - content_width - MARGIN,
                page_height - content_height - MARGIN,
            ),
            Self::BottomLeft => (MARGIN, MARGIN),
            Self::BottomRight => (page_width - content_width - MARGIN, MARGIN),
            Self::Custom(x, y) => (*x, *y),
        }
    }
}

/// Layer where watermark is rendered
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WatermarkLayer {
    /// Render watermark above page content
    #[default]
    Foreground,
    /// Render watermark below page content
    Background,
}

/// Specification for a watermark
#[derive(Debug, Clone)]
pub struct WatermarkSpec {
    /// Content of the watermark (text or image)
    pub content: WatermarkContent,
    /// Position on the page
    pub position: WatermarkPosition,
    /// Opacity (0.0 = invisible, 1.0 = fully opaque)
    pub opacity: f32,
    /// Rotation angle in degrees
    pub rotation: f32,
    /// Layer (foreground or background)
    pub layer: WatermarkLayer,
}

impl WatermarkSpec {
    /// Default opacity for watermarks (30% visible)
    pub const DEFAULT_OPACITY: f32 = 0.3;

    /// Create a text watermark with default settings
    pub fn text(text: impl Into<String>) -> Self {
        Self {
            content: WatermarkContent::Text {
                text: text.into(),
                font: Font::Helvetica,
                font_size: 72.0,
                color: Color::gray(0.5),
            },
            position: WatermarkPosition::Center,
            opacity: Self::DEFAULT_OPACITY,
            rotation: 0.0,
            layer: WatermarkLayer::Foreground,
        }
    }

    /// Create an image watermark
    pub fn image(data: Vec<u8>, width: f32, height: f32) -> Self {
        Self {
            content: WatermarkContent::Image {
                data,
                width,
                height,
            },
            position: WatermarkPosition::Center,
            opacity: Self::DEFAULT_OPACITY,
            rotation: 0.0,
            layer: WatermarkLayer::Foreground,
        }
    }

    /// Set the opacity (0.0 to 1.0)
    pub fn with_opacity(mut self, opacity: f32) -> Self {
        self.opacity = opacity;
        self
    }

    /// Set the rotation angle in degrees
    pub fn with_rotation(mut self, rotation: f32) -> Self {
        self.rotation = rotation;
        self
    }

    /// Set the position
    pub fn with_position(mut self, position: WatermarkPosition) -> Self {
        self.position = position;
        self
    }

    /// Set the layer (foreground or background)
    pub fn with_layer(mut self, layer: WatermarkLayer) -> Self {
        self.layer = layer;
        self
    }

    /// Set the font for text watermarks
    pub fn with_font(mut self, font: Font, size: f32) -> Self {
        if let WatermarkContent::Text {
            font: ref mut f,
            font_size: ref mut s,
            ..
        } = self.content
        {
            *f = font;
            *s = size;
        }
        self
    }

    /// Set the color for text watermarks
    pub fn with_color(mut self, color: Color) -> Self {
        if let WatermarkContent::Text {
            color: ref mut c, ..
        } = self.content
        {
            *c = color;
        }
        self
    }

    /// Validate the watermark specification
    pub fn validate(&self) -> WatermarkResult<()> {
        // Validate opacity
        if !(0.0..=1.0).contains(&self.opacity) {
            return Err(WatermarkError::InvalidOpacity(self.opacity));
        }

        // Validate text is not empty
        if let WatermarkContent::Text { ref text, .. } = self.content {
            if text.is_empty() {
                return Err(WatermarkError::EmptyText);
            }
        }

        Ok(())
    }
}

/// Range of pages to apply watermark to
#[derive(Debug, Clone)]
pub enum PageRange {
    /// All pages in the document
    All,
    /// Range of pages (inclusive, 0-indexed)
    Range { start: usize, end: usize },
    /// Specific list of page indices (0-indexed)
    Pages(Vec<usize>),
    /// Single page (0-indexed)
    Single(usize),
}

impl PageRange {
    /// Get the list of page indices for a document with given page count
    pub fn to_indices(&self, total_pages: usize) -> Vec<usize> {
        match self {
            Self::All => (0..total_pages).collect(),
            Self::Range { start, end } => {
                let end = (*end).min(total_pages.saturating_sub(1));
                (*start..=end).collect()
            }
            Self::Pages(pages) => pages
                .iter()
                .filter(|&&p| p < total_pages)
                .copied()
                .collect(),
            Self::Single(page) => {
                if *page < total_pages {
                    vec![*page]
                } else {
                    vec![]
                }
            }
        }
    }
}

/// Watermarker for applying watermarks to PDF pages
pub struct Watermarker;

impl Watermarker {
    /// Apply a watermark to pages in a PDF
    ///
    /// # Arguments
    /// * `editor` - The PdfEditor instance
    /// * `spec` - Watermark specification
    /// * `page_range` - Which pages to apply the watermark to
    ///
    /// # Returns
    /// Ok(number of pages watermarked) on success, or WatermarkError on failure
    pub fn apply(
        editor: &mut super::PdfEditor,
        spec: WatermarkSpec,
        page_range: PageRange,
    ) -> WatermarkResult<usize> {
        // Validate the spec first
        spec.validate()?;

        let page_count = editor.page_count();
        let indices = page_range.to_indices(page_count);

        if indices.is_empty() {
            return Ok(0);
        }

        // Validate all page indices
        for &idx in &indices {
            if idx >= page_count {
                return Err(WatermarkError::PageIndexOutOfBounds {
                    index: idx,
                    total: page_count,
                });
            }
        }

        // Store the watermark for later application
        let count = indices.len();
        editor.pending_watermarks.push((indices, spec));

        Ok(count)
    }

    /// Apply a watermark to all pages in a PDF
    pub fn apply_to_all(
        editor: &mut super::PdfEditor,
        spec: WatermarkSpec,
    ) -> WatermarkResult<usize> {
        Self::apply(editor, spec, PageRange::All)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ==================== WatermarkSpec Tests ====================

    // T3.1 - WatermarkSpec texto con opacidad default
    #[test]
    fn test_watermark_spec_text_default_opacity() {
        let spec = WatermarkSpec::text("CONFIDENTIAL");
        assert!((spec.opacity - 0.3).abs() < f32::EPSILON);
    }

    // T3.2 - WatermarkSpec texto con opacidad custom
    #[test]
    fn test_watermark_spec_text_custom_opacity() {
        let spec = WatermarkSpec::text("DRAFT").with_opacity(0.5);
        assert!((spec.opacity - 0.5).abs() < f32::EPSILON);
    }

    // T3.3 - WatermarkSpec texto diagonal (rotation = 45.0)
    #[test]
    fn test_watermark_spec_text_diagonal() {
        let spec = WatermarkSpec::text("SAMPLE").with_rotation(45.0);
        assert!((spec.rotation - 45.0).abs() < f32::EPSILON);
    }

    // T3.4 - WatermarkSpec imagen (bytes + posicion)
    #[test]
    fn test_watermark_spec_image() {
        let image_data = vec![0xFF, 0xD8, 0xFF, 0xE0]; // Fake JPEG header
        let spec = WatermarkSpec::image(image_data.clone(), 100.0, 50.0);

        match &spec.content {
            WatermarkContent::Image {
                data,
                width,
                height,
            } => {
                assert_eq!(data, &image_data);
                assert!((width - 100.0).abs() < f32::EPSILON);
                assert!((height - 50.0).abs() < f32::EPSILON);
            }
            _ => panic!("Expected Image content"),
        }
    }

    // T3.5 - WatermarkPosition::Center calcula coordenadas correctas
    #[test]
    fn test_watermark_position_center() {
        let pos = WatermarkPosition::Center;
        let (x, y) = pos.calculate_coordinates(595.0, 842.0, 100.0, 50.0);

        // Center: (595 - 100) / 2 = 247.5, (842 - 50) / 2 = 396.0
        assert!((x - 247.5).abs() < 0.01);
        assert!((y - 396.0).abs() < 0.01);
    }

    // T3.6 - WatermarkPosition::TopLeft
    #[test]
    fn test_watermark_position_top_left() {
        let pos = WatermarkPosition::TopLeft;
        let (x, y) = pos.calculate_coordinates(595.0, 842.0, 100.0, 50.0);

        // TopLeft: margin=50, y = 842 - 50 - 50 = 742
        assert!((x - 50.0).abs() < 0.01);
        assert!((y - 742.0).abs() < 0.01);
    }

    // T3.7 - WatermarkPosition::BottomRight
    #[test]
    fn test_watermark_position_bottom_right() {
        let pos = WatermarkPosition::BottomRight;
        let (x, y) = pos.calculate_coordinates(595.0, 842.0, 100.0, 50.0);

        // BottomRight: x = 595 - 100 - 50 = 445, y = 50
        assert!((x - 445.0).abs() < 0.01);
        assert!((y - 50.0).abs() < 0.01);
    }

    // T3.8 - WatermarkPosition::Custom(x, y)
    #[test]
    fn test_watermark_position_custom() {
        let pos = WatermarkPosition::Custom(123.0, 456.0);
        let (x, y) = pos.calculate_coordinates(595.0, 842.0, 100.0, 50.0);

        assert!((x - 123.0).abs() < 0.01);
        assert!((y - 456.0).abs() < 0.01);
    }

    // T3.9 - WatermarkLayer::Foreground (sobre contenido)
    #[test]
    fn test_watermark_layer_foreground() {
        let spec = WatermarkSpec::text("TEST").with_layer(WatermarkLayer::Foreground);
        assert_eq!(spec.layer, WatermarkLayer::Foreground);
    }

    // T3.10 - WatermarkLayer::Background (bajo contenido)
    #[test]
    fn test_watermark_layer_background() {
        let spec = WatermarkSpec::text("TEST").with_layer(WatermarkLayer::Background);
        assert_eq!(spec.layer, WatermarkLayer::Background);
    }

    // T3.11 - Watermarker aplica a todas las paginas
    #[test]
    fn test_watermarker_all_pages() {
        let range = PageRange::All;
        let indices = range.to_indices(5);
        assert_eq!(indices, vec![0, 1, 2, 3, 4]);
    }

    // T3.12 - Watermarker aplica a rango de paginas
    #[test]
    fn test_watermarker_page_range() {
        let range = PageRange::Range { start: 1, end: 3 };
        let indices = range.to_indices(5);
        assert_eq!(indices, vec![1, 2, 3]);
    }

    // T3.13 - Watermarker aplica a lista de paginas especificas
    #[test]
    fn test_watermarker_specific_pages() {
        let range = PageRange::Pages(vec![0, 2, 4]);
        let indices = range.to_indices(5);
        assert_eq!(indices, vec![0, 2, 4]);
    }

    // T3.14 - WatermarkSpec implementa Clone
    #[test]
    fn test_watermark_spec_clone() {
        let spec = WatermarkSpec::text("CLONE TEST").with_opacity(0.7);
        let cloned = spec.clone();
        assert!((cloned.opacity - 0.7).abs() < f32::EPSILON);

        match cloned.content {
            WatermarkContent::Text { text, .. } => assert_eq!(text, "CLONE TEST"),
            _ => panic!("Expected Text content"),
        }
    }

    // T3.15 - WatermarkSpec implementa Debug
    #[test]
    fn test_watermark_spec_debug() {
        let spec = WatermarkSpec::text("DEBUG");
        let debug_str = format!("{:?}", spec);
        assert!(debug_str.contains("WatermarkSpec"));
        assert!(debug_str.contains("DEBUG"));
    }

    // T3.16 - Opacity 0.0 es valida (invisible)
    #[test]
    fn test_watermark_spec_zero_opacity() {
        let spec = WatermarkSpec::text("INVISIBLE").with_opacity(0.0);
        assert!(spec.validate().is_ok());
        assert!((spec.opacity - 0.0).abs() < f32::EPSILON);
    }

    // T3.17 - Opacity > 1.0 es error
    #[test]
    fn test_watermark_spec_opacity_out_of_range() {
        let spec = WatermarkSpec::text("TOO OPAQUE").with_opacity(1.5);
        let result = spec.validate();
        assert!(result.is_err());

        match result {
            Err(WatermarkError::InvalidOpacity(v)) => assert!((v - 1.5).abs() < f32::EPSILON),
            _ => panic!("Expected InvalidOpacity error"),
        }
    }

    // T3.18 - WatermarkError variants tienen Display correcto
    #[test]
    fn test_watermark_error_display() {
        let err = WatermarkError::InvalidOpacity(1.5);
        assert!(err.to_string().contains("1.5"));
        assert!(err.to_string().contains("opacity"));

        let err = WatermarkError::InvalidRotation(f32::NAN);
        assert!(err.to_string().contains("rotation"));

        let err = WatermarkError::EmptyText;
        assert!(err.to_string().contains("empty"));

        let err = WatermarkError::PageNotFound(5);
        assert!(err.to_string().contains("5"));

        let err = WatermarkError::PageIndexOutOfBounds {
            index: 10,
            total: 5,
        };
        assert!(err.to_string().contains("10"));
        assert!(err.to_string().contains("5"));

        let err = WatermarkError::UnsupportedImageFormat("BMP".to_string());
        assert!(err.to_string().contains("BMP"));
    }

    // ==================== Additional Tests ====================

    // Test negative opacity validation
    #[test]
    fn test_watermark_spec_negative_opacity() {
        let spec = WatermarkSpec::text("NEGATIVE").with_opacity(-0.5);
        let result = spec.validate();
        assert!(result.is_err());

        match result {
            Err(WatermarkError::InvalidOpacity(v)) => assert!((v - (-0.5)).abs() < f32::EPSILON),
            _ => panic!("Expected InvalidOpacity error"),
        }
    }

    // Test empty text validation
    #[test]
    fn test_watermark_spec_empty_text() {
        let spec = WatermarkSpec::text("");
        let result = spec.validate();
        assert!(result.is_err());

        match result {
            Err(WatermarkError::EmptyText) => {}
            _ => panic!("Expected EmptyText error"),
        }
    }

    // Test WatermarkPosition::TopRight
    #[test]
    fn test_watermark_position_top_right() {
        let pos = WatermarkPosition::TopRight;
        let (x, y) = pos.calculate_coordinates(595.0, 842.0, 100.0, 50.0);

        // TopRight: x = 595 - 100 - 50 = 445, y = 842 - 50 - 50 = 742
        assert!((x - 445.0).abs() < 0.01);
        assert!((y - 742.0).abs() < 0.01);
    }

    // Test WatermarkPosition::BottomLeft
    #[test]
    fn test_watermark_position_bottom_left() {
        let pos = WatermarkPosition::BottomLeft;
        let (x, y) = pos.calculate_coordinates(595.0, 842.0, 100.0, 50.0);

        // BottomLeft: x = 50, y = 50
        assert!((x - 50.0).abs() < 0.01);
        assert!((y - 50.0).abs() < 0.01);
    }

    // Test PageRange::Single
    #[test]
    fn test_page_range_single() {
        let range = PageRange::Single(2);
        let indices = range.to_indices(5);
        assert_eq!(indices, vec![2]);
    }

    // Test PageRange::Single out of bounds
    #[test]
    fn test_page_range_single_out_of_bounds() {
        let range = PageRange::Single(10);
        let indices = range.to_indices(5);
        assert!(indices.is_empty());
    }

    // Test PageRange::Range clamps to total
    #[test]
    fn test_page_range_clamps() {
        let range = PageRange::Range { start: 3, end: 10 };
        let indices = range.to_indices(5);
        assert_eq!(indices, vec![3, 4]);
    }

    // Test PageRange::Pages filters out of bounds
    #[test]
    fn test_page_range_pages_filters() {
        let range = PageRange::Pages(vec![0, 5, 2, 10, 4]);
        let indices = range.to_indices(5);
        assert_eq!(indices, vec![0, 2, 4]);
    }

    // Test WatermarkSpec with_font
    #[test]
    fn test_watermark_spec_with_font() {
        let spec = WatermarkSpec::text("FONTED").with_font(Font::TimesRoman, 48.0);

        match &spec.content {
            WatermarkContent::Text {
                font, font_size, ..
            } => {
                assert!(matches!(font, Font::TimesRoman));
                assert!((font_size - 48.0).abs() < f32::EPSILON);
            }
            _ => panic!("Expected Text content"),
        }
    }

    // Test WatermarkSpec with_color
    #[test]
    fn test_watermark_spec_with_color() {
        let spec = WatermarkSpec::text("COLORED").with_color(Color::rgb(1.0, 0.0, 0.0));

        match &spec.content {
            WatermarkContent::Text { color, .. } => {
                assert!((color.r() - 1.0).abs() < 0.01);
                assert!((color.g() - 0.0).abs() < 0.01);
                assert!((color.b() - 0.0).abs() < 0.01);
            }
            _ => panic!("Expected Text content"),
        }
    }

    // Test default layer is Foreground
    #[test]
    fn test_watermark_spec_default_layer() {
        let spec = WatermarkSpec::text("DEFAULT");
        assert_eq!(spec.layer, WatermarkLayer::Foreground);
    }

    // Test default position is Center
    #[test]
    fn test_watermark_spec_default_position() {
        let spec = WatermarkSpec::text("DEFAULT");
        assert_eq!(spec.position, WatermarkPosition::Center);
    }

    // Test WatermarkLayer Default trait
    #[test]
    fn test_watermark_layer_default() {
        let layer: WatermarkLayer = Default::default();
        assert_eq!(layer, WatermarkLayer::Foreground);
    }

    // Test WatermarkPosition PartialEq
    #[test]
    fn test_watermark_position_eq() {
        assert_eq!(WatermarkPosition::Center, WatermarkPosition::Center);
        assert_ne!(WatermarkPosition::Center, WatermarkPosition::TopLeft);
        assert_eq!(
            WatermarkPosition::Custom(10.0, 20.0),
            WatermarkPosition::Custom(10.0, 20.0)
        );
    }

    // Test WatermarkError Clone
    #[test]
    fn test_watermark_error_clone() {
        let err = WatermarkError::PageIndexOutOfBounds { index: 5, total: 3 };
        let cloned = err.clone();
        assert_eq!(err.to_string(), cloned.to_string());
    }

    // Test WatermarkError Debug
    #[test]
    fn test_watermark_error_debug() {
        let err = WatermarkError::EmptyText;
        let debug_str = format!("{:?}", err);
        assert!(debug_str.contains("EmptyText"));
    }

    // Test std::error::Error impl
    #[test]
    fn test_watermark_error_is_error() {
        let err: Box<dyn std::error::Error> = Box::new(WatermarkError::EmptyText);
        assert!(!err.to_string().is_empty());
    }

    // Test valid opacity 1.0
    #[test]
    fn test_watermark_spec_full_opacity() {
        let spec = WatermarkSpec::text("SOLID").with_opacity(1.0);
        assert!(spec.validate().is_ok());
    }

    // Test image watermark validation (always valid if opacity ok)
    #[test]
    fn test_watermark_spec_image_validation() {
        let spec = WatermarkSpec::image(vec![0xFF, 0xD8], 100.0, 50.0);
        assert!(spec.validate().is_ok());
    }

    // Test rotation can be any value (no validation on rotation currently)
    #[test]
    fn test_watermark_spec_large_rotation() {
        let spec = WatermarkSpec::text("ROTATED").with_rotation(720.0);
        assert!(spec.validate().is_ok());
        assert!((spec.rotation - 720.0).abs() < f32::EPSILON);
    }

    // Test negative rotation
    #[test]
    fn test_watermark_spec_negative_rotation() {
        let spec = WatermarkSpec::text("NEGATIVE").with_rotation(-45.0);
        assert!(spec.validate().is_ok());
        assert!((spec.rotation - (-45.0)).abs() < f32::EPSILON);
    }
}
