// Metadata module - ISO 32000-1 Section 14.3
//
// Provides support for document and object-level metadata,
// including XMP (Extensible Metadata Platform) streams.

pub mod xmp;

pub use xmp::{XmpMetadata, XmpNamespace, XmpProperty};
