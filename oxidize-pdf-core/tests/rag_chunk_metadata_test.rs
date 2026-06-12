//! Integration test for Task 7: ChunkMetadata wired into RagChunk, with
//! prev/next linkage (Task 5b `link_chunks`) populated end-to-end through the
//! `PdfDocument::rag_chunks*` entry points.
//!
//! This is a content-verifying test: it builds a real two-section PDF, parses
//! it back, runs the actual chunking pipeline, and asserts that the resulting
//! `RagChunk`s carry populated metadata and a correctly chained prev/next id
//! list. It does NOT assert mere absence of crash.

use oxidize_pdf::parser::{PdfDocument, PdfReader};
use oxidize_pdf::pipeline::{HybridChunkConfig, MergePolicy, RagChunk};
use oxidize_pdf::text::Font;
use oxidize_pdf::{Document, Page};
use std::io::Cursor;

/// Build a small PDF whose body, chunked with a tight token budget, yields at
/// least two chunks. Each paragraph carries a unique marker so we can verify
/// the chunks are distinct content (not duplicated).
fn build_chunks() -> Vec<RagChunk> {
    let mut doc = Document::new();
    let mut page = Page::a4();

    page.text()
        .set_font(Font::HelveticaBold, 16.0)
        .at(50.0, 760.0)
        .write("SECTION ALPHA HEADING")
        .unwrap();

    // Several body paragraphs, each with enough words that a small max_tokens
    // budget forces a flush between them.
    let body_lines = [
        (720.0, "Alpha marker paragraph with several words to fill the first token budget bucket completely."),
        (700.0, "Bravo marker paragraph with several words to fill the second token budget bucket completely."),
        (680.0, "Charlie marker paragraph with several words to fill the third token budget bucket completely."),
        (660.0, "Delta marker paragraph with several words to fill the fourth token budget bucket completely."),
    ];
    for (y, line) in body_lines {
        page.text()
            .set_font(Font::Helvetica, 11.0)
            .at(50.0, y)
            .write(line)
            .unwrap();
    }

    doc.add_page(page);
    let pdf_bytes = doc.to_bytes().expect("pdf generation should succeed");

    let reader = PdfReader::new(Cursor::new(&pdf_bytes)).expect("parse generated PDF");
    let parsed = PdfDocument::new(reader);

    // Tight token budget to force multiple chunks; merge adjacent so paragraphs
    // accrete into a bucket until the budget overflows.
    let config = HybridChunkConfig {
        max_tokens: 12,
        overlap_tokens: 0,
        merge_adjacent: true,
        propagate_headings: true,
        merge_policy: MergePolicy::AnyInlineContent,
    };
    parsed
        .rag_chunks_with(config)
        .expect("rag_chunks_with must succeed")
}

#[test]
fn rag_chunks_have_linked_ids_and_metadata() {
    let chunks = build_chunks();
    assert!(
        chunks.len() >= 2,
        "expected at least two chunks to verify linkage, got {}",
        chunks.len()
    );

    // First chunk has no predecessor; last has no successor.
    assert!(
        chunks[0].metadata.prev_chunk_id.is_none(),
        "first chunk must have prev_chunk_id == None"
    );
    let last = chunks.len() - 1;
    assert!(
        chunks[last].metadata.next_chunk_id.is_none(),
        "last chunk must have next_chunk_id == None"
    );

    // Adjacent chunks' ids chain forward and backward.
    for i in 0..chunks.len() {
        if i > 0 {
            assert_eq!(
                chunks[i].metadata.prev_chunk_id.as_deref(),
                Some(chunks[i - 1].metadata.chunk_id.as_str()),
                "chunk[{i}].prev_chunk_id must equal chunk[{}].chunk_id",
                i - 1
            );
        }
        if i + 1 < chunks.len() {
            assert_eq!(
                chunks[i].metadata.next_chunk_id.as_deref(),
                Some(chunks[i + 1].metadata.chunk_id.as_str()),
                "chunk[{i}].next_chunk_id must equal chunk[{}].chunk_id",
                i + 1
            );
        }
    }

    // chunk_id is non-empty and unique per chunk.
    for (i, c) in chunks.iter().enumerate() {
        assert!(
            !c.metadata.chunk_id.is_empty(),
            "chunk[{i}].chunk_id must be non-empty"
        );
        assert!(
            c.metadata.chunk_id.ends_with(&format!(":{i}")),
            "chunk[{i}].chunk_id must end with :{i}, got {:?}",
            c.metadata.chunk_id
        );
    }

    // Metadata is genuinely populated (not the Default::default() placeholder):
    // every chunk has real character/word/sentence counts derived from its text.
    for (i, c) in chunks.iter().enumerate() {
        assert!(
            c.metadata.char_count > 0,
            "chunk[{i}].metadata.char_count must be > 0"
        );
        assert_eq!(
            c.metadata.char_count,
            c.text.chars().count(),
            "chunk[{i}].metadata.char_count must match its text length"
        );
        assert!(
            c.metadata.word_count > 0,
            "chunk[{i}].metadata.word_count must be > 0"
        );
    }

    // Source is not stamped through the plain (non-source) entry point.
    assert!(
        chunks[0].metadata.source.is_none(),
        "rag_chunks_with must not stamp a DocumentSource"
    );
}
