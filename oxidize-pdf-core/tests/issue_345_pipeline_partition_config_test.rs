//! Issue #345: a custom `AnalysisPipeline` must be able to run over a
//! configurable partition. Before the fix, `rag_chunks_with_pipeline`
//! hard-coded `self.partition()` (default `PartitionConfig`), so a downstream
//! consumer could not avoid the table-detector behaviour that turns
//! single-column prose pages into page-spanning (empty) `table` elements.
//!
//! Contract under test: `AnalysisPipeline::with_partition_config` threads a
//! `PartitionConfig` into `rag_chunks_with_pipeline`, and the pipeline honours
//! it. The default round-trips to the unconfigured behaviour exactly.
//!
//! Fixture: `issue_272_boe_sumario_2025_01_15.pdf` — a Spanish government
//! bulletin where the default table detector emits 29 empty `table` elements
//! (34 elements total), while `detect_tables = false` partitions the same
//! pages into 338 prose/list elements. That structural delta is the observable
//! signal that the partition config reached the partitioner.
#![cfg(feature = "unstable-spi")]

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use oxidize_pdf::parser::{PdfDocument, PdfReader};
use oxidize_pdf::pipeline::{
    AnalysisPipeline, ClassLabel, ClassifyContext, Element, ElementClassifier, PartitionConfig,
    RagChunk,
};

const FIXTURE: &str = "tests/fixtures/issue_272_boe_sumario_2025_01_15.pdf";

fn open() -> PdfDocument<std::fs::File> {
    PdfDocument::new(PdfReader::open(FIXTURE).expect("open fixture"))
}

fn texts(chunks: &[RagChunk]) -> Vec<String> {
    chunks.iter().map(|c| c.text.clone()).collect()
}

#[test]
fn rag_chunks_with_pipeline_honors_custom_partition_config() {
    let doc = open();

    let default_chunks = doc
        .rag_chunks_with_pipeline(&AnalysisPipeline::new())
        .expect("default pipeline");

    let cfg = PartitionConfig {
        detect_tables: false,
        ..Default::default()
    };
    let custom_chunks = doc
        .rag_chunks_with_pipeline(&AnalysisPipeline::new().with_partition_config(cfg))
        .expect("custom pipeline");

    // Neither path is degenerate.
    assert!(!default_chunks.is_empty(), "default produced no chunks");
    assert!(!custom_chunks.is_empty(), "custom produced no chunks");

    // The partition config must reach the partitioner: with detect_tables=false
    // the 29 page-spanning empty tables explode into their constituent prose
    // elements, which the HybridChunker then re-groups — so the chunk set is
    // materially different from the default. If the config is ignored (the #345
    // bug), both calls run the default partition and these vectors are identical.
    //
    // Note: the direction is not asserted — detect_tables=false yields *fewer*,
    // larger prose chunks than the table-fragmented default on this fixture, so
    // a `custom.len() > default.len()` guard would be a false failure.
    assert_ne!(
        texts(&default_chunks),
        texts(&custom_chunks),
        "partition_config was ignored: detect_tables=false ({} chunks) produced \
         chunks identical to the default partition ({} chunks)",
        custom_chunks.len(),
        default_chunks.len()
    );
}

/// A classifier that records how many elements it was asked to classify,
/// without changing any output.
struct CountingClassifier(Arc<AtomicUsize>);

impl ElementClassifier for CountingClassifier {
    fn classify(&self, _element: &Element, _ctx: &ClassifyContext) -> Option<ClassLabel> {
        self.0.fetch_add(1, Ordering::Relaxed);
        None
    }
}

#[test]
fn partition_config_governs_the_elements_seen_by_the_classifier() {
    let doc = open();

    let cfg = PartitionConfig {
        detect_tables: false,
        ..Default::default()
    };

    // Ground truth: the elements the configured partition produces, computed
    // independently of the pipeline.
    let expected = doc
        .partition_with(cfg.clone())
        .expect("partition_with(cfg)")
        .len();

    // The classifier stage must run over exactly those elements — proving the
    // pipeline's PartitionConfig governs the *mechanism* (which elements reach
    // the rest of the pipeline), not merely some downstream chunk-text delta.
    let counter = Arc::new(AtomicUsize::new(0));
    let pipeline = AnalysisPipeline::new()
        .with_partition_config(cfg)
        .with_classifier(Box::new(CountingClassifier(counter.clone())));
    let _ = doc
        .rag_chunks_with_pipeline(&pipeline)
        .expect("rag_chunks_with_pipeline");

    assert_eq!(
        counter.load(Ordering::Relaxed),
        expected,
        "classifier ran over {} elements but the configured partition yields {}",
        counter.load(Ordering::Relaxed),
        expected
    );

    // Guard against a vacuous match: the configured partition must genuinely
    // differ from the default, otherwise this would pass even if the config
    // were ignored.
    let default_len = doc.partition().expect("partition").len();
    assert_ne!(
        expected, default_len,
        "fixture no longer distinguishes detect_tables on/off ({} vs {}); \
         the test can no longer detect an ignored partition_config",
        expected, default_len
    );
}

#[test]
fn explicit_default_partition_config_matches_unconfigured_pipeline() {
    let doc = open();

    let unconfigured = doc
        .rag_chunks_with_pipeline(&AnalysisPipeline::new())
        .expect("unconfigured pipeline");
    let explicit_default = doc
        .rag_chunks_with_pipeline(
            &AnalysisPipeline::new().with_partition_config(PartitionConfig::default()),
        )
        .expect("explicit-default pipeline");

    // Setting the default config explicitly must not change anything.
    assert_eq!(
        texts(&unconfigured),
        texts(&explicit_default),
        "explicit PartitionConfig::default() diverged from the unconfigured pipeline"
    );
}
