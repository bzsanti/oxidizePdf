//! PDF writing functionality

mod object_streams;
mod pdf_writer;
mod signature;
mod xref_stream_writer;

pub use object_streams::{ObjectStream, ObjectStreamConfig, ObjectStreamStats, ObjectStreamWriter};
pub use pdf_writer::{PdfWriter, WriterConfig};
pub(crate) use signature::{Edition, PdfSignature};
pub use xref_stream_writer::XRefStreamWriter;
