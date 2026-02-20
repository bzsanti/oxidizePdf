//! Annotation Injection API for adding annotations to existing PDFs.
//!
//! This module provides operations for adding annotations to existing PDF documents:
//! - Text notes (sticky notes with comments)
//! - Highlights (text markup)
//! - Links (URL and internal page links)
//! - Stamps (predefined annotation stamps)
//!
//! # Example
//!
//! ```ignore
//! use oxidize_pdf::operations::{PdfEditor, AnnotationInjector, TextAnnotationSpec, LinkAnnotationSpec};
//!
//! let mut editor = PdfEditor::from_bytes(pdf_bytes)?;
//!
//! // Add a text note
//! AnnotationInjector::add_text_note(
//!     &mut editor,
//!     0,
//!     TextAnnotationSpec::new(100.0, 700.0, "Review this section")
//! )?;
//!
//! // Add a URL link
//! AnnotationInjector::add_link_url(
//!     &mut editor,
//!     0,
//!     AnnotationRect::new(50.0, 100.0, 200.0, 20.0),
//!     "https://example.com"
//! )?;
//!
//! editor.save_to_bytes()?;
//! ```

use std::fmt;
use thiserror::Error;

/// Error type for annotation injection operations.
#[derive(Debug, Clone, PartialEq, Error)]
pub enum AnnotationInjectorError {
    /// Page index is out of bounds.
    #[error("page index {index} is out of bounds (document has {page_count} pages)")]
    PageIndexOutOfBounds { index: usize, page_count: usize },

    /// Invalid annotation rectangle dimensions.
    #[error("invalid annotation rectangle: {reason}")]
    InvalidRect { reason: String },

    /// Invalid annotation content.
    #[error("invalid annotation content: {0}")]
    InvalidContent(String),

    /// Target page for internal link is out of bounds.
    #[error("link target page {target} is out of bounds (document has {page_count} pages)")]
    InvalidLinkTarget { target: usize, page_count: usize },

    /// Generic annotation error.
    #[error("annotation error: {0}")]
    AnnotationFailed(String),
}

/// Result type for annotation injection operations.
pub type AnnotationInjectorResult<T> = Result<T, AnnotationInjectorError>;

/// Annotation icon for text annotations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AnnotationIcon {
    /// Standard note icon (default).
    #[default]
    Note,
    /// Comment icon.
    Comment,
    /// Key icon.
    Key,
    /// Help icon.
    Help,
    /// New paragraph icon.
    NewParagraph,
    /// Paragraph icon.
    Paragraph,
    /// Insert icon.
    Insert,
}

impl AnnotationIcon {
    /// Returns the PDF name for this icon.
    pub fn as_pdf_name(&self) -> &'static str {
        match self {
            AnnotationIcon::Note => "Note",
            AnnotationIcon::Comment => "Comment",
            AnnotationIcon::Key => "Key",
            AnnotationIcon::Help => "Help",
            AnnotationIcon::NewParagraph => "NewParagraph",
            AnnotationIcon::Paragraph => "Paragraph",
            AnnotationIcon::Insert => "Insert",
        }
    }
}

/// Rectangle for annotation positioning.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AnnotationRect {
    /// X coordinate of lower-left corner.
    pub x: f64,
    /// Y coordinate of lower-left corner.
    pub y: f64,
    /// Width of the rectangle.
    pub width: f64,
    /// Height of the rectangle.
    pub height: f64,
}

impl AnnotationRect {
    /// Creates a new annotation rectangle.
    ///
    /// # Arguments
    ///
    /// * `x` - X coordinate of lower-left corner
    /// * `y` - Y coordinate of lower-left corner
    /// * `width` - Width of the rectangle
    /// * `height` - Height of the rectangle
    ///
    /// # Errors
    ///
    /// Returns `InvalidRect` if width or height is <= 0.
    pub fn new(x: f64, y: f64, width: f64, height: f64) -> AnnotationInjectorResult<Self> {
        if width <= 0.0 {
            return Err(AnnotationInjectorError::InvalidRect {
                reason: format!("width must be positive, got {}", width),
            });
        }
        if height <= 0.0 {
            return Err(AnnotationInjectorError::InvalidRect {
                reason: format!("height must be positive, got {}", height),
            });
        }
        Ok(Self {
            x,
            y,
            width,
            height,
        })
    }

    /// Creates a rectangle without validation (for internal use).
    pub(crate) fn new_unchecked(x: f64, y: f64, width: f64, height: f64) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    /// Converts to PDF array format [x1, y1, x2, y2].
    pub fn to_pdf_array(&self) -> [f64; 4] {
        [self.x, self.y, self.x + self.width, self.y + self.height]
    }
}

impl fmt::Display for AnnotationRect {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Rect[{}, {}, {}x{}]",
            self.x, self.y, self.width, self.height
        )
    }
}

/// RGB color for annotations.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AnnotationColor {
    /// Red component (0.0 to 1.0).
    pub r: f64,
    /// Green component (0.0 to 1.0).
    pub g: f64,
    /// Blue component (0.0 to 1.0).
    pub b: f64,
}

impl AnnotationColor {
    /// Creates a new RGB color.
    pub fn rgb(r: f64, g: f64, b: f64) -> Self {
        Self {
            r: r.clamp(0.0, 1.0),
            g: g.clamp(0.0, 1.0),
            b: b.clamp(0.0, 1.0),
        }
    }

    /// Yellow color (default for highlights).
    pub fn yellow() -> Self {
        Self::rgb(1.0, 1.0, 0.0)
    }

    /// Red color.
    pub fn red() -> Self {
        Self::rgb(1.0, 0.0, 0.0)
    }

    /// Green color.
    pub fn green() -> Self {
        Self::rgb(0.0, 1.0, 0.0)
    }

    /// Blue color.
    pub fn blue() -> Self {
        Self::rgb(0.0, 0.0, 1.0)
    }

    /// Converts to PDF array format [r, g, b].
    pub fn to_pdf_array(&self) -> [f64; 3] {
        [self.r, self.g, self.b]
    }
}

impl Default for AnnotationColor {
    fn default() -> Self {
        Self::yellow()
    }
}

/// Specification for a text annotation (sticky note).
#[derive(Debug, Clone)]
pub struct TextAnnotationSpec {
    /// X coordinate of the annotation.
    pub x: f64,
    /// Y coordinate of the annotation.
    pub y: f64,
    /// Content of the annotation.
    pub content: String,
    /// Icon to display.
    pub icon: AnnotationIcon,
    /// Color of the annotation.
    pub color: Option<AnnotationColor>,
    /// Whether the annotation should be open by default.
    pub open: bool,
}

impl TextAnnotationSpec {
    /// Creates a new text annotation specification.
    pub fn new(x: f64, y: f64, content: impl Into<String>) -> Self {
        Self {
            x,
            y,
            content: content.into(),
            icon: AnnotationIcon::default(),
            color: None,
            open: false,
        }
    }

    /// Sets the annotation icon.
    pub fn with_icon(mut self, icon: AnnotationIcon) -> Self {
        self.icon = icon;
        self
    }

    /// Sets the annotation color.
    pub fn with_color(mut self, color: AnnotationColor) -> Self {
        self.color = Some(color);
        self
    }

    /// Sets whether the annotation should be open by default.
    pub fn with_open(mut self, open: bool) -> Self {
        self.open = open;
        self
    }
}

/// Specification for a highlight annotation.
#[derive(Debug, Clone)]
pub struct HighlightAnnotationSpec {
    /// QuadPoints defining the highlight region (8 points per quad).
    pub quad_points: Vec<f64>,
    /// Color of the highlight.
    pub color: AnnotationColor,
}

impl HighlightAnnotationSpec {
    /// Creates a new highlight annotation from quad points.
    ///
    /// QuadPoints must be a multiple of 8 (4 corners x 2 coordinates).
    pub fn new(quad_points: Vec<f64>) -> AnnotationInjectorResult<Self> {
        if quad_points.len() < 8 || quad_points.len() % 8 != 0 {
            return Err(AnnotationInjectorError::InvalidContent(format!(
                "quad_points must have 8*n elements, got {}",
                quad_points.len()
            )));
        }
        Ok(Self {
            quad_points,
            color: AnnotationColor::yellow(),
        })
    }

    /// Creates a highlight annotation from a rectangle.
    pub fn from_rect(rect: &AnnotationRect) -> Self {
        // QuadPoints order: x1,y1 (top-left), x2,y2 (top-right), x3,y3 (bottom-left), x4,y4 (bottom-right)
        let x1 = rect.x;
        let y1 = rect.y + rect.height;
        let x2 = rect.x + rect.width;
        let y2 = rect.y + rect.height;
        let x3 = rect.x;
        let y3 = rect.y;
        let x4 = rect.x + rect.width;
        let y4 = rect.y;

        Self {
            quad_points: vec![x1, y1, x2, y2, x3, y3, x4, y4],
            color: AnnotationColor::yellow(),
        }
    }

    /// Sets the highlight color.
    pub fn with_color(mut self, color: AnnotationColor) -> Self {
        self.color = color;
        self
    }

    /// Calculates the bounding rectangle from quad points.
    pub fn bounding_rect(&self) -> AnnotationRect {
        let mut min_x = f64::MAX;
        let mut min_y = f64::MAX;
        let mut max_x = f64::MIN;
        let mut max_y = f64::MIN;

        for chunk in self.quad_points.chunks(2) {
            if chunk.len() == 2 {
                min_x = min_x.min(chunk[0]);
                max_x = max_x.max(chunk[0]);
                min_y = min_y.min(chunk[1]);
                max_y = max_y.max(chunk[1]);
            }
        }

        AnnotationRect::new_unchecked(min_x, min_y, max_x - min_x, max_y - min_y)
    }
}

/// Link action type.
#[derive(Debug, Clone, PartialEq)]
pub enum LinkAction {
    /// Link to external URL.
    Uri(String),
    /// Link to internal page (0-based index).
    GoToPage(usize),
    /// Link to named destination.
    GoToNamed(String),
}

/// Specification for a link annotation.
#[derive(Debug, Clone)]
pub struct LinkAnnotationSpec {
    /// Rectangle defining the clickable area.
    pub rect: AnnotationRect,
    /// Action to perform when clicked.
    pub action: LinkAction,
    /// Border width (0 for no border).
    pub border_width: f64,
    /// Highlight mode when clicked.
    pub highlight_mode: LinkHighlightMode,
}

/// Highlight mode for link annotations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LinkHighlightMode {
    /// No highlighting.
    None,
    /// Invert the contents of the annotation rectangle.
    #[default]
    Invert,
    /// Invert the border of the annotation rectangle.
    Outline,
    /// Pressed appearance (push button effect).
    Push,
}

impl LinkHighlightMode {
    /// Returns the PDF name for this mode.
    pub fn as_pdf_name(&self) -> &'static str {
        match self {
            LinkHighlightMode::None => "N",
            LinkHighlightMode::Invert => "I",
            LinkHighlightMode::Outline => "O",
            LinkHighlightMode::Push => "P",
        }
    }
}

impl LinkAnnotationSpec {
    /// Creates a link to an external URL.
    pub fn url(rect: AnnotationRect, url: impl Into<String>) -> Self {
        Self {
            rect,
            action: LinkAction::Uri(url.into()),
            border_width: 0.0,
            highlight_mode: LinkHighlightMode::default(),
        }
    }

    /// Creates a link to an internal page.
    pub fn to_page(rect: AnnotationRect, page_index: usize) -> Self {
        Self {
            rect,
            action: LinkAction::GoToPage(page_index),
            border_width: 0.0,
            highlight_mode: LinkHighlightMode::default(),
        }
    }

    /// Creates a link to a named destination.
    pub fn to_named(rect: AnnotationRect, name: impl Into<String>) -> Self {
        Self {
            rect,
            action: LinkAction::GoToNamed(name.into()),
            border_width: 0.0,
            highlight_mode: LinkHighlightMode::default(),
        }
    }

    /// Sets the border width.
    pub fn with_border(mut self, width: f64) -> Self {
        self.border_width = width.max(0.0);
        self
    }

    /// Sets the highlight mode.
    pub fn with_highlight_mode(mut self, mode: LinkHighlightMode) -> Self {
        self.highlight_mode = mode;
        self
    }
}

/// Standard stamp names per ISO 32000-1.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StampName {
    /// "Approved" stamp.
    Approved,
    /// "Experimental" stamp.
    Experimental,
    /// "NotApproved" stamp.
    NotApproved,
    /// "AsIs" stamp.
    AsIs,
    /// "Expired" stamp.
    Expired,
    /// "NotForPublicRelease" stamp.
    NotForPublicRelease,
    /// "Confidential" stamp.
    Confidential,
    /// "Final" stamp.
    Final,
    /// "Sold" stamp.
    Sold,
    /// "Departmental" stamp.
    Departmental,
    /// "ForComment" stamp.
    ForComment,
    /// "TopSecret" stamp.
    TopSecret,
    /// "Draft" stamp.
    Draft,
    /// "ForPublicRelease" stamp.
    ForPublicRelease,
}

impl StampName {
    /// Returns the PDF name for this stamp.
    pub fn as_pdf_name(&self) -> &'static str {
        match self {
            StampName::Approved => "Approved",
            StampName::Experimental => "Experimental",
            StampName::NotApproved => "NotApproved",
            StampName::AsIs => "AsIs",
            StampName::Expired => "Expired",
            StampName::NotForPublicRelease => "NotForPublicRelease",
            StampName::Confidential => "Confidential",
            StampName::Final => "Final",
            StampName::Sold => "Sold",
            StampName::Departmental => "Departmental",
            StampName::ForComment => "ForComment",
            StampName::TopSecret => "TopSecret",
            StampName::Draft => "Draft",
            StampName::ForPublicRelease => "ForPublicRelease",
        }
    }
}

/// Specification for a stamp annotation.
#[derive(Debug, Clone)]
pub struct StampAnnotationSpec {
    /// Rectangle defining the stamp area.
    pub rect: AnnotationRect,
    /// Stamp name.
    pub name: StampName,
    /// Optional contents/subject.
    pub contents: Option<String>,
}

impl StampAnnotationSpec {
    /// Creates a new stamp annotation.
    pub fn new(rect: AnnotationRect, name: StampName) -> Self {
        Self {
            rect,
            name,
            contents: None,
        }
    }

    /// Sets the stamp contents.
    pub fn with_contents(mut self, contents: impl Into<String>) -> Self {
        self.contents = Some(contents.into());
        self
    }
}

/// Pending annotation to be added to the document.
#[derive(Debug, Clone)]
pub enum PendingAnnotation {
    /// Text note annotation.
    TextNote(TextAnnotationSpec),
    /// Highlight annotation.
    Highlight(HighlightAnnotationSpec),
    /// Link annotation.
    Link(LinkAnnotationSpec),
    /// Stamp annotation.
    Stamp(StampAnnotationSpec),
}

/// Annotation injection operations for existing PDFs.
pub struct AnnotationInjector;

impl AnnotationInjector {
    /// Adds a text note annotation to a page.
    pub fn add_text_note(
        editor: &mut super::PdfEditor,
        page_index: usize,
        spec: TextAnnotationSpec,
    ) -> AnnotationInjectorResult<()> {
        let page_count = editor.page_count();
        if page_index >= page_count {
            return Err(AnnotationInjectorError::PageIndexOutOfBounds {
                index: page_index,
                page_count,
            });
        }

        editor
            .pending_annotations
            .push((page_index, PendingAnnotation::TextNote(spec)));
        Ok(())
    }

    /// Adds a highlight annotation to a page.
    pub fn add_highlight(
        editor: &mut super::PdfEditor,
        page_index: usize,
        spec: HighlightAnnotationSpec,
    ) -> AnnotationInjectorResult<()> {
        let page_count = editor.page_count();
        if page_index >= page_count {
            return Err(AnnotationInjectorError::PageIndexOutOfBounds {
                index: page_index,
                page_count,
            });
        }

        editor
            .pending_annotations
            .push((page_index, PendingAnnotation::Highlight(spec)));
        Ok(())
    }

    /// Adds a URL link annotation to a page.
    pub fn add_link_url(
        editor: &mut super::PdfEditor,
        page_index: usize,
        rect: AnnotationRect,
        url: impl Into<String>,
    ) -> AnnotationInjectorResult<()> {
        let page_count = editor.page_count();
        if page_index >= page_count {
            return Err(AnnotationInjectorError::PageIndexOutOfBounds {
                index: page_index,
                page_count,
            });
        }

        let spec = LinkAnnotationSpec::url(rect, url);
        editor
            .pending_annotations
            .push((page_index, PendingAnnotation::Link(spec)));
        Ok(())
    }

    /// Adds an internal page link annotation.
    pub fn add_link_page(
        editor: &mut super::PdfEditor,
        page_index: usize,
        rect: AnnotationRect,
        target_page: usize,
    ) -> AnnotationInjectorResult<()> {
        let page_count = editor.page_count();
        if page_index >= page_count {
            return Err(AnnotationInjectorError::PageIndexOutOfBounds {
                index: page_index,
                page_count,
            });
        }
        if target_page >= page_count {
            return Err(AnnotationInjectorError::InvalidLinkTarget {
                target: target_page,
                page_count,
            });
        }

        let spec = LinkAnnotationSpec::to_page(rect, target_page);
        editor
            .pending_annotations
            .push((page_index, PendingAnnotation::Link(spec)));
        Ok(())
    }

    /// Adds a stamp annotation to a page.
    pub fn add_stamp(
        editor: &mut super::PdfEditor,
        page_index: usize,
        spec: StampAnnotationSpec,
    ) -> AnnotationInjectorResult<()> {
        let page_count = editor.page_count();
        if page_index >= page_count {
            return Err(AnnotationInjectorError::PageIndexOutOfBounds {
                index: page_index,
                page_count,
            });
        }

        editor
            .pending_annotations
            .push((page_index, PendingAnnotation::Stamp(spec)));
        Ok(())
    }

    /// Gets the count of pending annotations.
    pub fn pending_count(editor: &super::PdfEditor) -> usize {
        editor.pending_annotations.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ==========================================================
    // T6.1 - TextAnnotationSpec construction
    // ==========================================================
    #[test]
    fn test_text_annotation_spec_new() {
        let spec = TextAnnotationSpec::new(100.0, 200.0, "Test content");
        assert!((spec.x - 100.0).abs() < f64::EPSILON);
        assert!((spec.y - 200.0).abs() < f64::EPSILON);
        assert_eq!(spec.content, "Test content");
        assert_eq!(spec.icon, AnnotationIcon::Note);
        assert!(spec.color.is_none());
        assert!(!spec.open);
    }

    // ==========================================================
    // T6.2 - TextAnnotationSpec with custom icon
    // ==========================================================
    #[test]
    fn test_text_annotation_spec_custom_icon() {
        let spec = TextAnnotationSpec::new(0.0, 0.0, "").with_icon(AnnotationIcon::Comment);
        assert_eq!(spec.icon, AnnotationIcon::Comment);
        assert_eq!(spec.icon.as_pdf_name(), "Comment");
    }

    #[test]
    fn test_annotation_icon_all_variants() {
        assert_eq!(AnnotationIcon::Note.as_pdf_name(), "Note");
        assert_eq!(AnnotationIcon::Comment.as_pdf_name(), "Comment");
        assert_eq!(AnnotationIcon::Key.as_pdf_name(), "Key");
        assert_eq!(AnnotationIcon::Help.as_pdf_name(), "Help");
        assert_eq!(AnnotationIcon::NewParagraph.as_pdf_name(), "NewParagraph");
        assert_eq!(AnnotationIcon::Paragraph.as_pdf_name(), "Paragraph");
        assert_eq!(AnnotationIcon::Insert.as_pdf_name(), "Insert");
    }

    // ==========================================================
    // T6.3 - HighlightAnnotationSpec with QuadPoints
    // ==========================================================
    #[test]
    fn test_highlight_annotation_spec() {
        let quad_points = vec![0.0, 0.0, 100.0, 0.0, 0.0, 20.0, 100.0, 20.0];
        let spec = HighlightAnnotationSpec::new(quad_points.clone()).unwrap();
        assert_eq!(spec.quad_points, quad_points);
        assert_eq!(spec.color, AnnotationColor::yellow());
    }

    #[test]
    fn test_highlight_annotation_invalid_quad_points() {
        // Less than 8 points
        let result = HighlightAnnotationSpec::new(vec![0.0, 0.0, 100.0, 0.0]);
        assert!(result.is_err());

        // Not a multiple of 8
        let result = HighlightAnnotationSpec::new(vec![0.0; 10]);
        assert!(result.is_err());
    }

    #[test]
    fn test_highlight_annotation_from_rect() {
        let rect = AnnotationRect::new(50.0, 100.0, 200.0, 30.0).unwrap();
        let spec = HighlightAnnotationSpec::from_rect(&rect);
        assert_eq!(spec.quad_points.len(), 8);
    }

    #[test]
    fn test_highlight_bounding_rect() {
        let quad_points = vec![10.0, 20.0, 110.0, 20.0, 10.0, 50.0, 110.0, 50.0];
        let spec = HighlightAnnotationSpec::new(quad_points).unwrap();
        let bounds = spec.bounding_rect();
        assert!((bounds.x - 10.0).abs() < f64::EPSILON);
        assert!((bounds.y - 20.0).abs() < f64::EPSILON);
        assert!((bounds.width - 100.0).abs() < f64::EPSILON);
        assert!((bounds.height - 30.0).abs() < f64::EPSILON);
    }

    // ==========================================================
    // T6.4 - LinkAnnotationSpec to URL
    // ==========================================================
    #[test]
    fn test_link_annotation_spec_url() {
        let rect = AnnotationRect::new(100.0, 200.0, 150.0, 20.0).unwrap();
        let spec = LinkAnnotationSpec::url(rect, "https://example.com");

        match &spec.action {
            LinkAction::Uri(url) => assert_eq!(url, "https://example.com"),
            _ => panic!("Expected Uri action"),
        }
        assert!((spec.border_width - 0.0).abs() < f64::EPSILON);
    }

    // ==========================================================
    // T6.5 - LinkAnnotationSpec to internal page
    // ==========================================================
    #[test]
    fn test_link_annotation_spec_internal_page() {
        let rect = AnnotationRect::new(0.0, 0.0, 100.0, 50.0).unwrap();
        let spec = LinkAnnotationSpec::to_page(rect, 5);

        match &spec.action {
            LinkAction::GoToPage(page) => assert_eq!(*page, 5),
            _ => panic!("Expected GoToPage action"),
        }
    }

    #[test]
    fn test_link_annotation_to_named() {
        let rect = AnnotationRect::new(0.0, 0.0, 100.0, 50.0).unwrap();
        let spec = LinkAnnotationSpec::to_named(rect, "chapter1");

        match &spec.action {
            LinkAction::GoToNamed(name) => assert_eq!(name, "chapter1"),
            _ => panic!("Expected GoToNamed action"),
        }
    }

    // ==========================================================
    // T6.10 - StampAnnotation with predefined names
    // ==========================================================
    #[test]
    fn test_stamp_annotation_spec() {
        let rect = AnnotationRect::new(100.0, 100.0, 150.0, 50.0).unwrap();
        let spec = StampAnnotationSpec::new(rect, StampName::Approved);

        assert_eq!(spec.name, StampName::Approved);
        assert_eq!(spec.name.as_pdf_name(), "Approved");
        assert!(spec.contents.is_none());
    }

    #[test]
    fn test_stamp_annotation_with_contents() {
        let rect = AnnotationRect::new(0.0, 0.0, 100.0, 50.0).unwrap();
        let spec = StampAnnotationSpec::new(rect, StampName::Draft).with_contents("Review needed");

        assert_eq!(spec.contents, Some("Review needed".to_string()));
    }

    #[test]
    fn test_stamp_names_all() {
        // Verify all stamp names have proper PDF names
        let stamps = [
            (StampName::Approved, "Approved"),
            (StampName::Experimental, "Experimental"),
            (StampName::NotApproved, "NotApproved"),
            (StampName::AsIs, "AsIs"),
            (StampName::Expired, "Expired"),
            (StampName::NotForPublicRelease, "NotForPublicRelease"),
            (StampName::Confidential, "Confidential"),
            (StampName::Final, "Final"),
            (StampName::Sold, "Sold"),
            (StampName::Departmental, "Departmental"),
            (StampName::ForComment, "ForComment"),
            (StampName::TopSecret, "TopSecret"),
            (StampName::Draft, "Draft"),
            (StampName::ForPublicRelease, "ForPublicRelease"),
        ];

        for (stamp, expected_name) in stamps {
            assert_eq!(stamp.as_pdf_name(), expected_name);
        }
    }

    // ==========================================================
    // T6.12 - AnnotationRect validation
    // ==========================================================
    #[test]
    fn test_annotation_rect_valid() {
        let rect = AnnotationRect::new(50.0, 100.0, 200.0, 30.0).unwrap();
        assert!((rect.x - 50.0).abs() < f64::EPSILON);
        assert!((rect.y - 100.0).abs() < f64::EPSILON);
        assert!((rect.width - 200.0).abs() < f64::EPSILON);
        assert!((rect.height - 30.0).abs() < f64::EPSILON);
    }

    // ==========================================================
    // T6.13 - AnnotationRect invalid width
    // ==========================================================
    #[test]
    fn test_annotation_rect_invalid_width() {
        let result = AnnotationRect::new(0.0, 0.0, 0.0, 100.0);
        assert!(result.is_err());

        let result = AnnotationRect::new(0.0, 0.0, -10.0, 100.0);
        assert!(result.is_err());
    }

    // ==========================================================
    // T6.14 - AnnotationRect invalid height
    // ==========================================================
    #[test]
    fn test_annotation_rect_invalid_height() {
        let result = AnnotationRect::new(0.0, 0.0, 100.0, 0.0);
        assert!(result.is_err());

        let result = AnnotationRect::new(0.0, 0.0, 100.0, -5.0);
        assert!(result.is_err());
    }

    #[test]
    fn test_annotation_rect_to_pdf_array() {
        let rect = AnnotationRect::new(10.0, 20.0, 100.0, 50.0).unwrap();
        let arr = rect.to_pdf_array();
        assert_eq!(arr, [10.0, 20.0, 110.0, 70.0]);
    }

    #[test]
    fn test_annotation_rect_display() {
        let rect = AnnotationRect::new(10.0, 20.0, 100.0, 50.0).unwrap();
        let display = format!("{}", rect);
        assert!(display.contains("Rect"));
        assert!(display.contains("10"));
    }

    // ==========================================================
    // T6.17 - AnnotationInjectorError Display
    // ==========================================================
    #[test]
    fn test_annotation_injector_error_display() {
        let err = AnnotationInjectorError::PageIndexOutOfBounds {
            index: 5,
            page_count: 3,
        };
        let msg = format!("{}", err);
        assert!(msg.contains("5"));
        assert!(msg.contains("3"));
        assert!(msg.contains("out of bounds"));
    }

    #[test]
    fn test_annotation_injector_error_invalid_rect() {
        let err = AnnotationInjectorError::InvalidRect {
            reason: "width is zero".to_string(),
        };
        let msg = format!("{}", err);
        assert!(msg.contains("width is zero"));
    }

    #[test]
    fn test_annotation_injector_error_invalid_content() {
        let err = AnnotationInjectorError::InvalidContent("empty quad points".to_string());
        let msg = format!("{}", err);
        assert!(msg.contains("empty quad points"));
    }

    #[test]
    fn test_annotation_injector_error_invalid_link_target() {
        let err = AnnotationInjectorError::InvalidLinkTarget {
            target: 10,
            page_count: 5,
        };
        let msg = format!("{}", err);
        assert!(msg.contains("10"));
        assert!(msg.contains("5"));
    }

    #[test]
    fn test_annotation_injector_error_clone() {
        let err = AnnotationInjectorError::AnnotationFailed("test".to_string());
        let cloned = err.clone();
        assert_eq!(err, cloned);
    }

    // ==========================================================
    // Additional tests
    // ==========================================================

    #[test]
    fn test_annotation_color_rgb() {
        let color = AnnotationColor::rgb(0.5, 0.3, 0.8);
        assert!((color.r - 0.5).abs() < f64::EPSILON);
        assert!((color.g - 0.3).abs() < f64::EPSILON);
        assert!((color.b - 0.8).abs() < f64::EPSILON);
    }

    #[test]
    fn test_annotation_color_clamping() {
        let color = AnnotationColor::rgb(1.5, -0.2, 0.5);
        assert!((color.r - 1.0).abs() < f64::EPSILON);
        assert!((color.g - 0.0).abs() < f64::EPSILON);
        assert!((color.b - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn test_annotation_color_presets() {
        let yellow = AnnotationColor::yellow();
        assert!((yellow.r - 1.0).abs() < f64::EPSILON);
        assert!((yellow.g - 1.0).abs() < f64::EPSILON);
        assert!((yellow.b - 0.0).abs() < f64::EPSILON);

        let red = AnnotationColor::red();
        assert!((red.r - 1.0).abs() < f64::EPSILON);
        assert!((red.g - 0.0).abs() < f64::EPSILON);

        let green = AnnotationColor::green();
        assert!((green.g - 1.0).abs() < f64::EPSILON);

        let blue = AnnotationColor::blue();
        assert!((blue.b - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_annotation_color_to_pdf_array() {
        let color = AnnotationColor::rgb(0.1, 0.2, 0.3);
        let arr = color.to_pdf_array();
        assert!((arr[0] - 0.1).abs() < f64::EPSILON);
        assert!((arr[1] - 0.2).abs() < f64::EPSILON);
        assert!((arr[2] - 0.3).abs() < f64::EPSILON);
    }

    #[test]
    fn test_text_annotation_with_color() {
        let color = AnnotationColor::red();
        let spec = TextAnnotationSpec::new(0.0, 0.0, "Test").with_color(color);
        assert!(spec.color.is_some());
        assert_eq!(spec.color.unwrap(), AnnotationColor::red());
    }

    #[test]
    fn test_text_annotation_open() {
        let spec = TextAnnotationSpec::new(0.0, 0.0, "").with_open(true);
        assert!(spec.open);
    }

    #[test]
    fn test_link_highlight_modes() {
        assert_eq!(LinkHighlightMode::None.as_pdf_name(), "N");
        assert_eq!(LinkHighlightMode::Invert.as_pdf_name(), "I");
        assert_eq!(LinkHighlightMode::Outline.as_pdf_name(), "O");
        assert_eq!(LinkHighlightMode::Push.as_pdf_name(), "P");
    }

    #[test]
    fn test_link_annotation_with_border() {
        let rect = AnnotationRect::new(0.0, 0.0, 100.0, 50.0).unwrap();
        let spec = LinkAnnotationSpec::url(rect, "http://test.com").with_border(2.0);
        assert!((spec.border_width - 2.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_link_annotation_with_highlight_mode() {
        let rect = AnnotationRect::new(0.0, 0.0, 100.0, 50.0).unwrap();
        let spec = LinkAnnotationSpec::url(rect, "http://test.com")
            .with_highlight_mode(LinkHighlightMode::Push);
        assert_eq!(spec.highlight_mode, LinkHighlightMode::Push);
    }

    #[test]
    fn test_highlight_with_color() {
        let spec = HighlightAnnotationSpec::new(vec![0.0; 8])
            .unwrap()
            .with_color(AnnotationColor::green());
        assert_eq!(spec.color, AnnotationColor::green());
    }

    #[test]
    fn test_pending_annotation_enum() {
        let text_spec = TextAnnotationSpec::new(0.0, 0.0, "Test");
        let pending = PendingAnnotation::TextNote(text_spec.clone());

        match pending {
            PendingAnnotation::TextNote(s) => assert_eq!(s.content, "Test"),
            _ => panic!("Expected TextNote"),
        }
    }
}
