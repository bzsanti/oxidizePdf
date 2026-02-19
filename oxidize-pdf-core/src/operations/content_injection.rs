//! Content Injection Module for PDF Modification
//!
//! This module provides functionality to inject text, images, and graphics
//! onto existing PDF pages. Content is overlaid on top of existing page content.
//!
//! # Example
//! ```ignore
//! use oxidize_pdf::operations::{PdfEditor, TextInjectionSpec, ContentInjector};
//!
//! let mut editor = PdfEditor::open("document.pdf")?;
//! let spec = TextInjectionSpec::new(100.0, 200.0, "Hello World");
//! ContentInjector::add_text(&mut editor, 0, spec)?;
//! editor.save("modified.pdf")?;
//! ```

use crate::graphics::Color;
use crate::text::Font;
use std::fmt;

/// Error type for content injection operations
#[derive(Debug, Clone)]
pub enum ContentInjectionError {
    /// Page index is out of bounds
    PageIndexOutOfBounds { index: usize, total: usize },
    /// Image format is not supported
    UnsupportedImageFormat(String),
    /// Invalid image data
    InvalidImageData(String),
    /// Content stream modification failed
    ContentStreamError(String),
    /// Font not available
    FontNotAvailable(String),
}

impl fmt::Display for ContentInjectionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
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
            Self::InvalidImageData(msg) => write!(f, "invalid image data: {}", msg),
            Self::ContentStreamError(msg) => write!(f, "content stream error: {}", msg),
            Self::FontNotAvailable(name) => write!(f, "font not available: {}", name),
        }
    }
}

impl std::error::Error for ContentInjectionError {}

/// Result type for content injection operations
pub type ContentInjectionResult<T> = Result<T, ContentInjectionError>;

/// Specification for injecting text onto a PDF page
#[derive(Debug, Clone)]
pub struct TextInjectionSpec {
    /// X coordinate (from left edge of page)
    pub x: f64,
    /// Y coordinate (from bottom edge of page)
    pub y: f64,
    /// Text content to inject
    pub text: String,
    /// Font to use (default: Helvetica)
    pub font: Font,
    /// Font size in points (default: 12.0)
    pub font_size: f64,
    /// Text color (default: black)
    pub color: Color,
    /// Line spacing for multiline text (default: 1.2)
    pub line_spacing: f64,
}

impl TextInjectionSpec {
    /// Create a new text injection specification
    pub fn new(x: f64, y: f64, text: impl Into<String>) -> Self {
        Self {
            x,
            y,
            text: text.into(),
            font: Font::Helvetica,
            font_size: 12.0,
            color: Color::black(),
            line_spacing: 1.2,
        }
    }

    /// Set the font type
    pub fn with_font(mut self, font: Font) -> Self {
        self.font = font;
        self
    }

    /// Set the font size
    pub fn with_font_size(mut self, size: f64) -> Self {
        self.font_size = size;
        self
    }

    /// Set the text color
    pub fn with_color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }

    /// Set line spacing multiplier for multiline text
    pub fn with_line_spacing(mut self, spacing: f64) -> Self {
        self.line_spacing = spacing;
        self
    }
}

/// Specification for injecting an image onto a PDF page
#[derive(Debug, Clone)]
pub struct ImageInjectionSpec {
    /// X coordinate (from left edge of page)
    pub x: f64,
    /// Y coordinate (from bottom edge of page)
    pub y: f64,
    /// Width of the image on the page
    pub width: f64,
    /// Height of the image on the page
    pub height: f64,
    /// Whether to lock aspect ratio when scaling
    pub aspect_ratio_locked: bool,
}

impl ImageInjectionSpec {
    /// Create a new image injection specification
    pub fn new(x: f64, y: f64, width: f64, height: f64) -> Self {
        Self {
            x,
            y,
            width,
            height,
            aspect_ratio_locked: false,
        }
    }

    /// Lock the aspect ratio when scaling
    pub fn with_aspect_ratio_locked(mut self, locked: bool) -> Self {
        self.aspect_ratio_locked = locked;
        self
    }
}

/// Specification for injecting a line onto a PDF page
#[derive(Debug, Clone)]
pub struct LineInjectionSpec {
    /// Starting X coordinate
    pub x1: f64,
    /// Starting Y coordinate
    pub y1: f64,
    /// Ending X coordinate
    pub x2: f64,
    /// Ending Y coordinate
    pub y2: f64,
    /// Line width in points
    pub line_width: f64,
    /// Line color
    pub color: Color,
}

impl LineInjectionSpec {
    /// Create a new line injection specification
    pub fn new(x1: f64, y1: f64, x2: f64, y2: f64) -> Self {
        Self {
            x1,
            y1,
            x2,
            y2,
            line_width: 1.0,
            color: Color::black(),
        }
    }

    /// Set the line width
    pub fn with_line_width(mut self, width: f64) -> Self {
        self.line_width = width;
        self
    }

    /// Set the line color
    pub fn with_color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }
}

/// Specification for injecting a rectangle onto a PDF page
#[derive(Debug, Clone)]
pub struct RectInjectionSpec {
    /// X coordinate of lower-left corner
    pub x: f64,
    /// Y coordinate of lower-left corner
    pub y: f64,
    /// Width of rectangle
    pub width: f64,
    /// Height of rectangle
    pub height: f64,
    /// Stroke width (0 = no stroke)
    pub stroke_width: f64,
    /// Stroke color
    pub stroke_color: Color,
    /// Fill color (None = no fill)
    pub fill_color: Option<Color>,
}

impl RectInjectionSpec {
    /// Create a new rectangle injection specification
    pub fn new(x: f64, y: f64, width: f64, height: f64) -> Self {
        Self {
            x,
            y,
            width,
            height,
            stroke_width: 1.0,
            stroke_color: Color::black(),
            fill_color: None,
        }
    }

    /// Set the stroke width
    pub fn with_stroke_width(mut self, width: f64) -> Self {
        self.stroke_width = width;
        self
    }

    /// Set the stroke color
    pub fn with_stroke_color(mut self, color: Color) -> Self {
        self.stroke_color = color;
        self
    }

    /// Set the fill color
    pub fn with_fill_color(mut self, color: Color) -> Self {
        self.fill_color = Some(color);
        self
    }
}

/// Specification for injecting a circle onto a PDF page
#[derive(Debug, Clone)]
pub struct CircleInjectionSpec {
    /// X coordinate of center
    pub cx: f64,
    /// Y coordinate of center
    pub cy: f64,
    /// Radius of circle
    pub radius: f64,
    /// Stroke width (0 = no stroke)
    pub stroke_width: f64,
    /// Stroke color
    pub stroke_color: Color,
    /// Fill color (None = no fill)
    pub fill_color: Option<Color>,
}

impl CircleInjectionSpec {
    /// Create a new circle injection specification
    pub fn new(cx: f64, cy: f64, radius: f64) -> Self {
        Self {
            cx,
            cy,
            radius,
            stroke_width: 1.0,
            stroke_color: Color::black(),
            fill_color: None,
        }
    }

    /// Set the stroke width
    pub fn with_stroke_width(mut self, width: f64) -> Self {
        self.stroke_width = width;
        self
    }

    /// Set the stroke color
    pub fn with_stroke_color(mut self, color: Color) -> Self {
        self.stroke_color = color;
        self
    }

    /// Set the fill color
    pub fn with_fill_color(mut self, color: Color) -> Self {
        self.fill_color = Some(color);
        self
    }
}

/// Supported image formats for injection
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageFormat {
    /// JPEG image format
    Jpeg,
    /// PNG image format
    Png,
}

impl ImageFormat {
    /// Detect image format from bytes
    pub fn detect(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < 8 {
            return None;
        }

        // JPEG magic bytes: FF D8 FF
        if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
            return Some(Self::Jpeg);
        }

        // PNG magic bytes: 89 50 4E 47 0D 0A 1A 0A
        if bytes.starts_with(&[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]) {
            return Some(Self::Png);
        }

        None
    }
}

/// Content injector for adding content to PDF pages
///
/// This struct provides static methods for injecting various types of content
/// onto PDF pages through a PdfEditor instance.
pub struct ContentInjector;

impl ContentInjector {
    /// Add text to a specific page
    ///
    /// # Arguments
    /// * `editor` - The PdfEditor instance
    /// * `page_index` - Zero-based page index
    /// * `spec` - Text injection specification
    ///
    /// # Returns
    /// Ok(()) on success, or ContentInjectionError on failure
    pub fn add_text(
        editor: &mut super::PdfEditor,
        page_index: usize,
        spec: TextInjectionSpec,
    ) -> ContentInjectionResult<()> {
        let page_count = editor.page_count();
        if page_index >= page_count {
            return Err(ContentInjectionError::PageIndexOutOfBounds {
                index: page_index,
                total: page_count,
            });
        }

        // Store the injection spec for later application
        editor.pending_text_injections.push((page_index, spec));
        Ok(())
    }

    /// Add an image to a specific page
    ///
    /// # Arguments
    /// * `editor` - The PdfEditor instance
    /// * `page_index` - Zero-based page index
    /// * `image_data` - Raw image bytes (JPEG or PNG)
    /// * `spec` - Image injection specification
    ///
    /// # Returns
    /// Ok(()) on success, or ContentInjectionError on failure
    pub fn add_image(
        editor: &mut super::PdfEditor,
        page_index: usize,
        image_data: Vec<u8>,
        spec: ImageInjectionSpec,
    ) -> ContentInjectionResult<()> {
        let page_count = editor.page_count();
        if page_index >= page_count {
            return Err(ContentInjectionError::PageIndexOutOfBounds {
                index: page_index,
                total: page_count,
            });
        }

        // Detect and validate image format
        let format = ImageFormat::detect(&image_data).ok_or_else(|| {
            ContentInjectionError::UnsupportedImageFormat("unknown format".to_string())
        })?;

        // Store the injection spec for later application
        editor
            .pending_image_injections
            .push((page_index, image_data, spec, format));
        Ok(())
    }

    /// Add a line to a specific page
    pub fn add_line(
        editor: &mut super::PdfEditor,
        page_index: usize,
        spec: LineInjectionSpec,
    ) -> ContentInjectionResult<()> {
        let page_count = editor.page_count();
        if page_index >= page_count {
            return Err(ContentInjectionError::PageIndexOutOfBounds {
                index: page_index,
                total: page_count,
            });
        }

        editor.pending_line_injections.push((page_index, spec));
        Ok(())
    }

    /// Add a rectangle to a specific page
    pub fn add_rect(
        editor: &mut super::PdfEditor,
        page_index: usize,
        spec: RectInjectionSpec,
    ) -> ContentInjectionResult<()> {
        let page_count = editor.page_count();
        if page_index >= page_count {
            return Err(ContentInjectionError::PageIndexOutOfBounds {
                index: page_index,
                total: page_count,
            });
        }

        editor.pending_rect_injections.push((page_index, spec));
        Ok(())
    }

    /// Add a circle to a specific page
    pub fn add_circle(
        editor: &mut super::PdfEditor,
        page_index: usize,
        spec: CircleInjectionSpec,
    ) -> ContentInjectionResult<()> {
        let page_count = editor.page_count();
        if page_index >= page_count {
            return Err(ContentInjectionError::PageIndexOutOfBounds {
                index: page_index,
                total: page_count,
            });
        }

        editor.pending_circle_injections.push((page_index, spec));
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // T2.1 - TextInjectionSpec se construye con posicion y texto
    #[test]
    fn test_text_injection_spec_new() {
        let spec = TextInjectionSpec::new(100.0, 200.0, "Hello");
        assert!((spec.x - 100.0).abs() < f64::EPSILON);
        assert!((spec.y - 200.0).abs() < f64::EPSILON);
        assert_eq!(spec.text, "Hello");
    }

    // T2.2 - TextInjectionSpec con fuente y tamano
    #[test]
    fn test_text_injection_spec_with_font() {
        let spec = TextInjectionSpec::new(0.0, 0.0, "Test")
            .with_font(Font::TimesRoman)
            .with_font_size(24.0);
        assert!(matches!(spec.font, Font::TimesRoman));
        assert!((spec.font_size - 24.0).abs() < f64::EPSILON);
    }

    // T2.3 - TextInjectionSpec con color
    #[test]
    fn test_text_injection_spec_with_color() {
        let spec = TextInjectionSpec::new(0.0, 0.0, "Test").with_color(Color::rgb(1.0, 0.0, 0.0));
        assert!((spec.color.r() - 1.0).abs() < f64::EPSILON);
        assert!((spec.color.g() - 0.0).abs() < f64::EPSILON);
        assert!((spec.color.b() - 0.0).abs() < f64::EPSILON);
    }

    // T2.4 - ImageInjectionSpec se construye con posicion y dimensiones
    #[test]
    fn test_image_injection_spec_new() {
        let spec = ImageInjectionSpec::new(50.0, 50.0, 200.0, 100.0);
        assert!((spec.x - 50.0).abs() < f64::EPSILON);
        assert!((spec.y - 50.0).abs() < f64::EPSILON);
        assert!((spec.width - 200.0).abs() < f64::EPSILON);
        assert!((spec.height - 100.0).abs() < f64::EPSILON);
    }

    // T2.5 - ImageInjectionSpec con aspect_ratio_locked
    #[test]
    fn test_image_injection_spec_aspect_ratio() {
        let spec = ImageInjectionSpec::new(0.0, 0.0, 100.0, 100.0).with_aspect_ratio_locked(true);
        assert!(spec.aspect_ratio_locked);
    }

    // T2.11 - TextInjectionSpec default font es Helvetica
    #[test]
    fn test_text_injection_default_font() {
        let spec = TextInjectionSpec::new(0.0, 0.0, "Test");
        assert!(matches!(spec.font, Font::Helvetica));
    }

    // T2.12 - TextInjectionSpec default size es 12.0
    #[test]
    fn test_text_injection_default_size() {
        let spec = TextInjectionSpec::new(0.0, 0.0, "Test");
        assert!((spec.font_size - 12.0).abs() < f64::EPSILON);
    }

    // T2.16 - GraphicInjectionSpec para rectangulos rellenos
    #[test]
    fn test_graphic_injection_spec_filled_rect() {
        let spec = RectInjectionSpec::new(10.0, 10.0, 50.0, 50.0)
            .with_fill_color(Color::rgb(0.5, 0.5, 0.5));
        assert!(spec.fill_color.is_some());
        let fill = spec.fill_color.unwrap();
        assert!((fill.r() - 0.5).abs() < f64::EPSILON);
    }

    // T2.17 - GraphicInjectionSpec para circulos
    #[test]
    fn test_graphic_injection_spec_circle() {
        let spec = CircleInjectionSpec::new(100.0, 100.0, 50.0)
            .with_stroke_width(2.0)
            .with_fill_color(Color::rgb(0.0, 0.0, 1.0));
        assert!((spec.cx - 100.0).abs() < f64::EPSILON);
        assert!((spec.cy - 100.0).abs() < f64::EPSILON);
        assert!((spec.radius - 50.0).abs() < f64::EPSILON);
        assert!((spec.stroke_width - 2.0).abs() < f64::EPSILON);
        assert!(spec.fill_color.is_some());
    }

    // Test LineInjectionSpec
    #[test]
    fn test_line_injection_spec_new() {
        let spec = LineInjectionSpec::new(0.0, 0.0, 100.0, 100.0);
        assert!((spec.x1 - 0.0).abs() < f64::EPSILON);
        assert!((spec.y1 - 0.0).abs() < f64::EPSILON);
        assert!((spec.x2 - 100.0).abs() < f64::EPSILON);
        assert!((spec.y2 - 100.0).abs() < f64::EPSILON);
        assert!((spec.line_width - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_line_injection_spec_with_properties() {
        let spec = LineInjectionSpec::new(0.0, 0.0, 50.0, 50.0)
            .with_line_width(3.0)
            .with_color(Color::rgb(1.0, 0.0, 0.0));
        assert!((spec.line_width - 3.0).abs() < f64::EPSILON);
        assert!((spec.color.r() - 1.0).abs() < f64::EPSILON);
    }

    // Test RectInjectionSpec
    #[test]
    fn test_rect_injection_spec_new() {
        let spec = RectInjectionSpec::new(10.0, 20.0, 100.0, 50.0);
        assert!((spec.x - 10.0).abs() < f64::EPSILON);
        assert!((spec.y - 20.0).abs() < f64::EPSILON);
        assert!((spec.width - 100.0).abs() < f64::EPSILON);
        assert!((spec.height - 50.0).abs() < f64::EPSILON);
        assert!(spec.fill_color.is_none());
    }

    #[test]
    fn test_rect_injection_spec_with_stroke() {
        let spec = RectInjectionSpec::new(0.0, 0.0, 50.0, 50.0)
            .with_stroke_width(2.0)
            .with_stroke_color(Color::rgb(0.0, 1.0, 0.0));
        assert!((spec.stroke_width - 2.0).abs() < f64::EPSILON);
        assert!((spec.stroke_color.g() - 1.0).abs() < f64::EPSILON);
    }

    // Test ImageFormat detection
    #[test]
    fn test_image_format_detect_jpeg() {
        let jpeg_header = [0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10, 0x4A, 0x46];
        assert_eq!(ImageFormat::detect(&jpeg_header), Some(ImageFormat::Jpeg));
    }

    #[test]
    fn test_image_format_detect_png() {
        let png_header = [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
        assert_eq!(ImageFormat::detect(&png_header), Some(ImageFormat::Png));
    }

    #[test]
    fn test_image_format_detect_unknown() {
        let unknown = [0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07];
        assert_eq!(ImageFormat::detect(&unknown), None);
    }

    #[test]
    fn test_image_format_detect_too_short() {
        let short = [0xFF, 0xD8];
        assert_eq!(ImageFormat::detect(&short), None);
    }

    // Test ContentInjectionError Display
    #[test]
    fn test_content_injection_error_display() {
        let err = ContentInjectionError::PageIndexOutOfBounds { index: 5, total: 3 };
        assert_eq!(
            err.to_string(),
            "page index 5 is out of bounds (total pages: 3)"
        );

        let err = ContentInjectionError::UnsupportedImageFormat("BMP".to_string());
        assert_eq!(err.to_string(), "unsupported image format: BMP");

        let err = ContentInjectionError::InvalidImageData("corrupted".to_string());
        assert_eq!(err.to_string(), "invalid image data: corrupted");

        let err = ContentInjectionError::ContentStreamError("parse failed".to_string());
        assert_eq!(err.to_string(), "content stream error: parse failed");

        let err = ContentInjectionError::FontNotAvailable("CustomFont".to_string());
        assert_eq!(err.to_string(), "font not available: CustomFont");
    }

    // Test TextInjectionSpec multiline support
    #[test]
    fn test_text_injection_spec_line_spacing() {
        let spec = TextInjectionSpec::new(0.0, 0.0, "Line 1\nLine 2").with_line_spacing(1.5);
        assert!((spec.line_spacing - 1.5).abs() < f64::EPSILON);
        assert!(spec.text.contains('\n'));
    }

    // Test default line spacing
    #[test]
    fn test_text_injection_default_line_spacing() {
        let spec = TextInjectionSpec::new(0.0, 0.0, "Test");
        assert!((spec.line_spacing - 1.2).abs() < f64::EPSILON);
    }

    // Test CircleInjectionSpec defaults
    #[test]
    fn test_circle_injection_spec_defaults() {
        let spec = CircleInjectionSpec::new(50.0, 50.0, 25.0);
        assert!((spec.stroke_width - 1.0).abs() < f64::EPSILON);
        assert!(spec.fill_color.is_none());
    }

    // Test Debug implementations
    #[test]
    fn test_text_injection_spec_debug() {
        let spec = TextInjectionSpec::new(10.0, 20.0, "Test");
        let debug_str = format!("{:?}", spec);
        assert!(debug_str.contains("TextInjectionSpec"));
        assert!(debug_str.contains("10"));
        assert!(debug_str.contains("20"));
    }

    #[test]
    fn test_image_injection_spec_debug() {
        let spec = ImageInjectionSpec::new(0.0, 0.0, 100.0, 100.0);
        let debug_str = format!("{:?}", spec);
        assert!(debug_str.contains("ImageInjectionSpec"));
    }

    #[test]
    fn test_content_injection_error_clone() {
        let err = ContentInjectionError::PageIndexOutOfBounds { index: 1, total: 1 };
        let cloned = err.clone();
        assert_eq!(err.to_string(), cloned.to_string());
    }
}
