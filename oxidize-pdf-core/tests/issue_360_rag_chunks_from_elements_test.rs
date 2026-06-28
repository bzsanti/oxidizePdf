//! Issue #360: expose a public way to run a custom `AnalysisPipeline`
//! (classifier → chunking → enrichers) over caller-provided `Element`s, instead
//! of forcing the element source to be the document's own `partition_with`.
//!
//! This lets a consumer feed externally-recovered elements (e.g. list items a
//! two-column layout scrambles past the partitioner) into the same enriched
//! chunk flow as the rest of the document — without the `RagChunk`
//! metadata-stamping workaround, which bypasses the classifier and enrichers.
//!
//! Contract under test:
//! - `rag_chunks_from_elements(elements, pipeline)` runs the classify→chunk→
//!   enrich stages over exactly the caller's `elements`.
//! - `rag_chunks_with_pipeline(pipeline)` is behaviourally
//!   `rag_chunks_from_elements(partition_with(pipeline.partition_config), pipeline)`,
//!   i.e. the existing path is preserved.
#![cfg(feature = "unstable-spi")]

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use oxidize_pdf::parser::{PdfDocument, PdfReader};
use oxidize_pdf::pipeline::{
    AnalysisPipeline, ClassLabel, ClassifyContext, Element, ElementClassifier, ElementData,
    ElementMetadata, PartitionConfig, RagChunk,
};

const FIXTURE: &str = "tests/fixtures/issue_272_boe_sumario_2025_01_15.pdf";

fn open() -> PdfDocument<std::fs::File> {
    PdfDocument::new(PdfReader::open(FIXTURE).expect("open fixture"))
}

fn texts(chunks: &[RagChunk]) -> Vec<String> {
    chunks.iter().map(|c| c.text.clone()).collect()
}

/// A classifier that records how many elements it was asked to classify.
struct CountingClassifier(Arc<AtomicUsize>);

impl ElementClassifier for CountingClassifier {
    fn classify(&self, _element: &Element, _ctx: &ClassifyContext) -> Option<ClassLabel> {
        self.0.fetch_add(1, Ordering::Relaxed);
        None
    }
}

#[test]
fn rag_chunks_from_elements_matches_pipeline_over_the_same_partition() {
    let doc = open();
    let pipeline = AnalysisPipeline::new();

    // The existing entry point: it partitions internally with the default config.
    let via_partition = doc
        .rag_chunks_with_pipeline(&pipeline)
        .expect("rag_chunks_with_pipeline");

    // The new entry point fed the *same* elements the existing path would derive.
    let elements = doc
        .partition_with(PartitionConfig::default())
        .expect("partition_with(default)");
    let via_elements = doc
        .rag_chunks_from_elements(elements, &pipeline)
        .expect("rag_chunks_from_elements");

    assert!(
        !via_partition.is_empty(),
        "partition path produced no chunks"
    );
    assert_eq!(
        texts(&via_partition),
        texts(&via_elements),
        "rag_chunks_from_elements over the default partition must reproduce \
         rag_chunks_with_pipeline exactly"
    );
}

#[test]
fn rag_chunks_from_elements_runs_over_caller_elements_not_the_partition() {
    let doc = open();

    // Caller provides a deliberately small, hand-trimmed element set — far fewer
    // than the document's full partition.
    let all = doc
        .partition_with(PartitionConfig::default())
        .expect("partition_with(default)");
    let full_len = all.len();
    let provided: Vec<Element> = all.into_iter().take(3).collect();
    assert!(
        full_len > provided.len(),
        "fixture must have more than {} partitioned elements (got {})",
        provided.len(),
        full_len
    );

    // The classifier stage must see exactly the caller's elements, proving the
    // element source is the provided vector and not the document partition.
    let counter = Arc::new(AtomicUsize::new(0));
    let pipeline =
        AnalysisPipeline::new().with_classifier(Box::new(CountingClassifier(counter.clone())));

    let chunks = doc
        .rag_chunks_from_elements(provided, &pipeline)
        .expect("rag_chunks_from_elements");

    assert_eq!(
        counter.load(Ordering::Relaxed),
        3,
        "classifier ran over {} elements but the caller provided 3 — the \
         partition was used instead of the provided elements",
        counter.load(Ordering::Relaxed)
    );
    assert!(
        !chunks.is_empty(),
        "three real elements should still yield at least one chunk"
    );
}

#[test]
fn rag_chunks_from_elements_carries_synthetic_recovered_content() {
    // The motivating use case (#360): mix the document's partitioned elements
    // with an externally-recovered element the partitioner never produced (e.g.
    // a list item a two-column layout scrambled), and get it into the same
    // enriched chunk flow. The recovered text must reach the output chunks.
    let doc = open();

    let mut elements: Vec<Element> = doc
        .partition_with(PartitionConfig::default())
        .expect("partition_with(default)")
        .into_iter()
        .take(2)
        .collect();

    const RECOVERED: &str = "RECOVERED_LIST_ITEM_scrambled_by_two_column_layout";
    elements.push(Element::Paragraph(ElementData {
        text: RECOVERED.to_string(),
        metadata: ElementMetadata::default(),
    }));

    let chunks = doc
        .rag_chunks_from_elements(elements, &AnalysisPipeline::new())
        .expect("rag_chunks_from_elements");

    let joined: String = chunks
        .iter()
        .map(|c| c.text.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        joined.contains(RECOVERED),
        "the externally-recovered element must flow through the pipeline into \
         the chunks; got: {joined:?}"
    );
}

#[test]
fn rag_chunks_from_elements_with_no_elements_yields_no_chunks() {
    // A caller may legitimately hand over an empty recovered set; that must be
    // a clean empty result, not an error or panic.
    let doc = open();
    let chunks = doc
        .rag_chunks_from_elements(Vec::new(), &AnalysisPipeline::new())
        .expect("rag_chunks_from_elements over an empty element set");
    assert!(
        chunks.is_empty(),
        "no elements must yield no chunks, got {}",
        chunks.len()
    );
}
