//! PDF writing functionality

mod content_stream_utils;
mod incremental_annotations;
mod incremental_form_fill;
mod incremental_highlights;
mod incremental_text_notes;
mod incremental_update;
mod object_streams;
mod pdf_writer;
mod signature;
mod xref_stream_writer;

// Phase 2 utilities for font preservation
pub(crate) use content_stream_utils::{
    apply_font_rename_map, collision_font_mapping, rewrite_font_references, INJECTED_BASE_FONT_KEYS,
};
pub use incremental_form_fill::IncrementalFormFiller;
pub use incremental_highlights::{
    Highlight, HighlightColor, HighlightId, HighlightMutation, HighlightOpacity, HighlightQuad,
    HighlightUpdate, IncrementalHighlightEditor,
};
pub use incremental_text_notes::{
    IncrementalTextNoteEditor, TextNote, TextNoteId, TextNoteMutation, TextNoteUpdate,
};
pub use object_streams::{ObjectStream, ObjectStreamConfig, ObjectStreamStats, ObjectStreamWriter};
pub use pdf_writer::{PdfWriter, WriterConfig};
pub(crate) use signature::{Edition, PdfSignature};
pub use xref_stream_writer::XRefStreamWriter;
