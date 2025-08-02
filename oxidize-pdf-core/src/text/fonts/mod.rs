//! Font subsystem modules

pub mod embedding;
pub mod truetype;

#[cfg(test)]
mod truetype_tests;

pub use embedding::{EmbeddedFontData, EmbeddingOptions, FontEmbedder};
pub use truetype::{CmapSubtable, GlyphInfo, TrueTypeFont};
