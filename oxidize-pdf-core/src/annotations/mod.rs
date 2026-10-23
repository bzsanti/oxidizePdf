//! Basic PDF annotations support according to ISO 32000-1 Chapter 12.5
//!
//! This module provides basic annotation types including text annotations,
//! link annotations, and markup annotations.
//!
//! Polygon and polyline options use [`PolygonBorderStyle`],
//! [`PolygonBorderStyleType`] and [`PolylineLineEndingStyle`]. These are
//! distinct from the base annotation border and [`LineAnnotation`] endings.
//!
//! ```
//! use oxidize_pdf::annotations::{PolylineAnnotation, PolylineLineEndingStyle};
//! use oxidize_pdf::geometry::Point;
//! let line = PolylineAnnotation::new(vec![Point::new(10.0, 10.0), Point::new(90.0, 90.0)])
//!     .with_endings(PolylineLineEndingStyle::OpenArrow, PolylineLineEndingStyle::Circle);
//! assert_eq!(line.start_style.to_pdf_name(), "OpenArrow");
//! ```

mod annotation;
mod annotation_type;
mod geometric;
mod link;
mod markup;
mod polygon;
mod popup;
mod text;

pub use annotation::{
    Annotation, AnnotationFlags, AnnotationManager, AnnotationType, BorderStyle, BorderStyleType,
};
pub use annotation_type::{
    CircleAnnotation, FileAttachmentAnnotation, FileAttachmentIcon, FreeTextAnnotation,
    HighlightAnnotation, InkAnnotation, LineAnnotation, LineEndingStyle, SquareAnnotation,
    StampAnnotation, StampName,
};
pub use link::{HighlightMode, LinkAction, LinkAnnotation, LinkDestination};
pub use markup::{MarkupAnnotation, MarkupType, QuadPoints};
pub use polygon::{
    create_rectangle_polygon, create_regular_polygon, create_triangle,
    BorderStyle as PolygonBorderStyle, BorderStyleType as PolygonBorderStyleType,
    LineEndingStyle as PolylineLineEndingStyle, PolygonAnnotation, PolylineAnnotation,
};
pub use popup::{
    create_markup_popup, create_open_popup, create_text_popup, PopupAnnotation, PopupFlags,
};
pub use text::{Icon, TextAnnotation};
