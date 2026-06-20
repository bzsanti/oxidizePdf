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

use oxidize_pdf::parser::{PdfDocument, PdfReader};
use oxidize_pdf::pipeline::{AnalysisPipeline, PartitionConfig, RagChunk};

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
    // elements, so the chunk set is materially different from the default.
    // If the config is ignored (the #345 bug), both calls run the default
    // partition and these vectors are identical.
    assert_ne!(
        texts(&default_chunks),
        texts(&custom_chunks),
        "partition_config was ignored: detect_tables=false produced chunks \
         identical to the default partition"
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
