//! Font subsystem modules

pub mod embedding;
pub mod standard;
pub mod truetype;
pub mod truetype_subsetter;
pub mod truetype_subsetting;

#[cfg(test)]
mod truetype_tests;

pub use embedding::{EmbeddedFontData, EmbeddingOptions, FontEmbedder};
pub use standard::{get_standard_font_metrics, StandardFontMetrics};
pub use truetype::{CmapSubtable, GlyphInfo, TrueTypeFont};
pub use truetype_subsetting::{SubsetStatistics, SubsettingOptions, TrueTypeSubsetter};
