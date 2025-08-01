//! Basic PDF annotations support according to ISO 32000-1 Chapter 12.5
//!
//! This module provides basic annotation types including text annotations,
//! link annotations, and markup annotations.

mod annotation;
mod annotation_type;
mod link;
mod markup;
mod text;

pub use annotation::{
    Annotation, AnnotationFlags, AnnotationManager, AnnotationType,
    BorderStyle as AnnotationBorderStyle,
};
pub use annotation_type::{
    FreeTextAnnotation, HighlightAnnotation, InkAnnotation, LineAnnotation, SquareAnnotation,
    StampAnnotation, StampName,
};
pub use link::{LinkAction, LinkAnnotation, LinkDestination};
pub use markup::{MarkupAnnotation, MarkupType, QuadPoints};
pub use text::{Icon, TextAnnotation};
