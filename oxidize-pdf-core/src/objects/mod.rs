//! PDF Object Types (Writer Module)
//!
//! # Migration Notice
//!
//! This module is being unified with the parser types in `crate::pdf_objects`.
//! The types here will be deprecated in v1.6.0 and removed in v2.0.0.
//!
//! **Migration Path**:
//! - `objects::Object` → `crate::pdf_objects::Object`
//! - `objects::Dictionary` → `crate::pdf_objects::Dictionary`
//! - `objects::ObjectId` → `crate::pdf_objects::ObjectId`
//! - `objects::Array` → `crate::pdf_objects::Array`
//! - `objects::Stream` → `crate::pdf_objects::Stream`
//!
//! The unified types in `pdf_objects` provide:
//! - Zero-overhead conversion between parser and writer
//! - Type-safe newtypes (Name, BinaryString)
//! - Consistent API across the library
//! - Better support for binary PDF strings

mod array;
mod dictionary;
mod primitive;
mod stream;

pub use array::Array;
pub use dictionary::Dictionary;
pub use primitive::{Object, ObjectId};
pub use stream::Stream;

// Type alias for compatibility
pub type ObjectReference = ObjectId;
