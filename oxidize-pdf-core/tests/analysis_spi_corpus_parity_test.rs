//! End-to-end regression guard: the analysis SPI's default path must be
//! byte-identical to the legacy `rag_chunks()` path on REAL documents, not just
//! a synthetic two-paragraph page. Closes spec §10.1 (corpus parity) with the
//! committed fixtures instead of a network corpus.
//!
//! If `AnalysisPipeline::new()` ever diverges from `rag_chunks()` — e.g. a
//! regression in the `HybridChunker → ChunkGroup → HybridChunk` round-trip
//! (`into_group`/`from_group`) — this fails on the first differing chunk.
#![cfg(feature = "unstable-spi")]

use oxidize_pdf::parser::{PdfDocument, PdfReader};
use oxidize_pdf::pipeline::{AnalysisPipeline, RagChunk};

/// Real, structurally diverse, git-tracked fixtures: English prose, a
/// two-column arXiv paper, a Spanish government bulletin, a SWE-bench RAG
/// document, and a generic poppler test PDF.
const FIXTURES: &[&str] = &[
    "tests/fixtures/Cold_Email_Hacks.pdf",
    "tests/fixtures/issue_272_higgs_arxiv_1207_7214.pdf",
    "tests/fixtures/issue_272_boe_sumario_2025_01_15.pdf",
    "tests/fixtures/issue_235_t.pdf",
    "tests/fixtures/poppler-85140-0.pdf",
];

fn chunks_via_legacy(path: &str) -> Vec<RagChunk> {
    let doc = PdfDocument::new(PdfReader::open(path).expect("open"));
    doc.rag_chunks().expect("rag_chunks")
}

fn chunks_via_spi(path: &str) -> Vec<RagChunk> {
    let doc = PdfDocument::new(PdfReader::open(path).expect("open"));
    doc.rag_chunks_with_pipeline(&AnalysisPipeline::new())
        .expect("rag_chunks_with_pipeline")
}

/// Assert every field of every chunk is identical between the two paths.
fn assert_chunks_identical(path: &str, legacy: &[RagChunk], spi: &[RagChunk]) {
    assert_eq!(
        spi.len(),
        legacy.len(),
        "{path}: chunk count differs (legacy {}, spi {})",
        legacy.len(),
        spi.len()
    );
    for (i, (l, s)) in legacy.iter().zip(spi.iter()).enumerate() {
        assert_eq!(s.chunk_index, l.chunk_index, "{path}#{i}: chunk_index");
        assert_eq!(s.text, l.text, "{path}#{i}: text");
        assert_eq!(s.full_text, l.full_text, "{path}#{i}: full_text");
        assert_eq!(s.page_numbers, l.page_numbers, "{path}#{i}: page_numbers");
        assert_eq!(
            s.bounding_boxes, l.bounding_boxes,
            "{path}#{i}: bounding_boxes"
        );
        assert_eq!(
            s.element_types, l.element_types,
            "{path}#{i}: element_types"
        );
        assert_eq!(
            s.heading_context, l.heading_context,
            "{path}#{i}: heading_context"
        );
        assert_eq!(
            s.token_estimate, l.token_estimate,
            "{path}#{i}: token_estimate"
        );
        assert_eq!(s.is_oversized, l.is_oversized, "{path}#{i}: is_oversized");
        // ChunkMetadata derives PartialEq → compares heading_path, fonts,
        // counts, chunk_id, prev/next links, source, citation anchor, table
        // dims (and `extra` under `semantic`) in one shot.
        assert_eq!(s.metadata, l.metadata, "{path}#{i}: metadata");
    }
}

#[test]
fn default_pipeline_is_byte_identical_to_rag_chunks_on_real_corpus() {
    let mut total_chunks = 0usize;
    let mut total_text_chunks = 0usize;
    for path in FIXTURES {
        let legacy = chunks_via_legacy(path);
        let spi = chunks_via_spi(path);

        // Parity holds per fixture, including a legitimately text-free PDF
        // (poppler-85140-0 yields zero chunks in BOTH paths — still parity).
        assert_chunks_identical(path, &legacy, &spi);
        total_chunks += legacy.len();
        total_text_chunks += legacy.iter().filter(|c| !c.text.trim().is_empty()).count();
    }
    // Guard against a vacuous pass: the corpus as a whole must produce a
    // substantial number of chunks carrying real text, otherwise "identical"
    // would prove nothing.
    assert!(
        total_chunks >= 100,
        "expected many chunks across the corpus, got {total_chunks}"
    );
    assert!(
        total_text_chunks >= 100,
        "expected many non-empty-text chunks, got {total_text_chunks}"
    );
}

#[test]
fn custom_config_via_pipeline_matches_rag_chunks_with() {
    use oxidize_pdf::pipeline::{HybridChunkConfig, HybridChunker};

    // A non-default budget through the SPI matches `rag_chunks_with(config)` of
    // the same budget — but `AnalysisPipeline::max_tokens` only drives the
    // `oversized` flag (spec §4/§6), so reproducing a custom-budget chunking
    // requires swapping in a `HybridChunker` built with that config AND setting
    // the matching oversized budget. (Calling only `with_max_tokens` would leave
    // the default 512-token chunker in place — a different grouping, by design.)
    let path = "tests/fixtures/Cold_Email_Hacks.pdf";
    let config = HybridChunkConfig {
        max_tokens: 128,
        ..HybridChunkConfig::default()
    };

    let legacy = {
        let doc = PdfDocument::new(PdfReader::open(path).expect("open"));
        doc.rag_chunks_with(config.clone())
            .expect("rag_chunks_with")
    };
    let spi = {
        let doc = PdfDocument::new(PdfReader::open(path).expect("open"));
        let pipeline = AnalysisPipeline::new()
            .with_chunking(Box::new(HybridChunker::new(config.clone())))
            .with_max_tokens(config.max_tokens);
        doc.rag_chunks_with_pipeline(&pipeline)
            .expect("rag_chunks_with_pipeline")
    };

    // The custom 128-token budget produces strictly more chunks than the default
    // 512 path would (≈100 vs ≈42) — proves the config actually took effect.
    assert!(
        legacy.len() > 50,
        "128-token budget should fragment more than the default; got {}",
        legacy.len()
    );
    assert_chunks_identical(path, &legacy, &spi);
}
