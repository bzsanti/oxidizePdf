//! Core curation logic - validation, classification, and consolidation
//!
//! This module implements the algorithms to:
//! 1. Validate if a text fragment is a real ISO requirement
//! 2. Classify requirements by type (mandatory/recommended/optional)
//! 3. Assign priority (P0/P1/P2/P3)
//! 4. Consolidate related fragments

mod validator;
mod classifier;
pub mod consolidator;
mod patterns;

pub use validator::{ValidationResult, is_valid_requirement, is_fragment, is_bibliographic_reference};
pub use classifier::{classify_type, assign_priority, detect_feature_area};
pub use consolidator::{ConsolidationGroup, consolidate_fragments, generate_semantic_id};
