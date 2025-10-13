//! PDF writing functionality

mod content_stream_utils;
mod object_streams;
mod pdf_writer;
mod signature;
mod xref_stream_writer;

// Phase 2 utilities for font preservation
pub(crate) use content_stream_utils::{rename_preserved_fonts, rewrite_font_references};
pub use object_streams::{ObjectStream, ObjectStreamConfig, ObjectStreamStats, ObjectStreamWriter};
pub use pdf_writer::{PdfWriter, WriterConfig};
pub(crate) use signature::{Edition, PdfSignature};
pub use xref_stream_writer::XRefStreamWriter;
