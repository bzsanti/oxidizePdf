use crate::pipeline::partition::ReadingOrderStrategy;
use crate::pipeline::PartitionConfig;
use crate::text::extraction::ExtractionOptions;

/// Pre-configured extraction profiles for different document types.
///
/// Each variant produces a [`ProfileConfig`] with tuned extraction and
/// partition settings. Pass directly to
/// [`PdfDocument::partition_with_profile`](crate::parser::PdfDocument::partition_with_profile)
/// or [`PdfDocument::rag_chunks_with_profile`](crate::parser::PdfDocument::rag_chunks_with_profile).
///
/// # Example
///
/// ```rust,no_run
/// use oxidize_pdf::parser::PdfDocument;
/// use oxidize_pdf::pipeline::ExtractionProfile;
///
/// let doc = PdfDocument::open("paper.pdf")?;
/// let chunks = doc.rag_chunks_with_profile(ExtractionProfile::Rag)?;
/// println!("{} chunks", chunks.len());
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone, Default)]
pub enum ExtractionProfile {
    /// General documents. Matches current defaults.
    #[default]
    Standard,
    /// Academic papers, multi-column layouts, narrow spacing.
    Academic,
    /// Forms and structured KV-heavy documents.
    Form,
    /// Government documents, official reports, slight scan tolerance.
    Government,
    /// Dense legal/technical text with tight spacing.
    Dense,
    /// Presentations and slides with large fonts.
    Presentation,
    /// Optimized for RAG chunking and vector store ingestion.
    ///
    /// Uses XY-Cut reading order for correct multi-column layout ordering
    /// and a slightly elevated table-confidence threshold (0.65) to reduce
    /// false table chunks that fragment prose.
    Rag,
}

/// Combined extraction configuration produced by a profile.
#[derive(Debug, Clone)]
pub struct ProfileConfig {
    pub extraction: ExtractionOptions,
    pub partition: PartitionConfig,
}

impl ExtractionProfile {
    /// Produce the combined configuration for this profile.
    pub fn config(&self) -> ProfileConfig {
        match self {
            ExtractionProfile::Standard => ProfileConfig {
                extraction: ExtractionOptions {
                    space_threshold: 0.3,
                    detect_columns: false,
                    ..ExtractionOptions::default()
                },
                partition: PartitionConfig {
                    title_min_font_ratio: 1.3,
                    header_zone: 0.05,
                    footer_zone: 0.05,
                    ..PartitionConfig::default()
                },
            },
            ExtractionProfile::Academic => ProfileConfig {
                extraction: ExtractionOptions {
                    space_threshold: 0.25,
                    detect_columns: true,
                    ..ExtractionOptions::default()
                },
                partition: PartitionConfig {
                    title_min_font_ratio: 1.4,
                    header_zone: 0.08,
                    footer_zone: 0.08,
                    ..PartitionConfig::default()
                },
            },
            ExtractionProfile::Form => ProfileConfig {
                extraction: ExtractionOptions {
                    space_threshold: 0.3,
                    detect_columns: false,
                    ..ExtractionOptions::default()
                },
                partition: PartitionConfig {
                    title_min_font_ratio: 1.5,
                    header_zone: 0.03,
                    footer_zone: 0.03,
                    ..PartitionConfig::default()
                },
            },
            ExtractionProfile::Government => ProfileConfig {
                extraction: ExtractionOptions {
                    space_threshold: 0.35,
                    detect_columns: false,
                    ..ExtractionOptions::default()
                },
                partition: PartitionConfig {
                    title_min_font_ratio: 1.3,
                    header_zone: 0.06,
                    footer_zone: 0.06,
                    ..PartitionConfig::default()
                },
            },
            ExtractionProfile::Dense => ProfileConfig {
                extraction: ExtractionOptions {
                    space_threshold: 0.2,
                    detect_columns: false,
                    ..ExtractionOptions::default()
                },
                partition: PartitionConfig {
                    title_min_font_ratio: 1.3,
                    header_zone: 0.05,
                    footer_zone: 0.05,
                    ..PartitionConfig::default()
                },
            },
            ExtractionProfile::Presentation => ProfileConfig {
                extraction: ExtractionOptions {
                    space_threshold: 0.4,
                    detect_columns: false,
                    ..ExtractionOptions::default()
                },
                partition: PartitionConfig {
                    title_min_font_ratio: 1.2,
                    header_zone: 0.10,
                    footer_zone: 0.10,
                    ..PartitionConfig::default()
                },
            },
            ExtractionProfile::Rag => ProfileConfig {
                extraction: ExtractionOptions {
                    space_threshold: 0.3,
                    detect_columns: false,
                    ..ExtractionOptions::default()
                },
                partition: PartitionConfig {
                    title_min_font_ratio: 1.3,
                    header_zone: 0.05,
                    footer_zone: 0.05,
                    reading_order: ReadingOrderStrategy::XYCut { min_gap: 20.0 },
                    min_table_confidence: 0.65,
                    ..PartitionConfig::default()
                },
            },
        }
    }
}
