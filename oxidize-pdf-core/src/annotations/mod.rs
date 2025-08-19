//! Basic PDF annotations support according to ISO 32000-1 Chapter 12.5
//!
//! This module provides basic annotation types including text annotations,
//! link annotations, and markup annotations.

mod annotation;
mod annotation_type;
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
    create_rectangle_polygon, create_regular_polygon, create_triangle, PolygonAnnotation,
    PolylineAnnotation,
};
pub use popup::{
    create_markup_popup, create_open_popup, create_text_popup, PopupAnnotation, PopupFlags,
};
pub use text::{Icon, TextAnnotation};
