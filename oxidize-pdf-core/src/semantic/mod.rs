//! Semantic marking for AI-Ready PDFs (Community Edition)
//!
//! This module provides basic functionality to mark PDF regions with semantic meaning,
//! making PDFs more accessible to AI/ML processing pipelines.
//!
//! For advanced features like invoice detection, form field marking, and ML-ready
//! exports, please see the PRO edition.

mod entity;
mod export;
mod marking;

pub use entity::{
    BoundingBox, Entity, EntityMetadata, EntityRelation, EntityType, RelationType, SemanticEntity,
};
pub use export::{EntityMap, ExportFormat};
pub use marking::{EntityBuilder, SemanticMarker};

/// Trait for types that support semantic marking
pub trait SemanticMarking {
    /// Mark a region with semantic meaning
    /// bounds is (x, y, width, height)
    #[allow(mismatched_lifetime_syntaxes)]
    fn mark_region(&mut self, bounds: (f64, f64, f64, f64)) -> EntityBuilder;

    /// Add a schema definition to the document
    fn add_schema(&mut self, schema_url: &str);

    /// Export all marked entities
    fn export_entities(&self) -> EntityMap;
}

mod tests;
