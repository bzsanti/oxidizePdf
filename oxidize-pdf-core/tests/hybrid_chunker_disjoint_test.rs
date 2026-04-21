//! Regression tests for `HybridChunker::chunk()` disjointness.
//!
//! The chunker MUST emit chunks whose element lists are pairwise disjoint:
//! the same source `Element` may not appear in two chunks, and the
//! concatenation of all chunks' element lists must equal the input element
//! list (modulo sentence-splitting of oversized paragraphs, which are
//! regenerated from source text and therefore do not share identity with
//! any input element).
//!
//! These tests exercise the bug that was observed on v2.5.4, where every
//! non-size-overflow flush re-injected the just-flushed elements back into
//! the working buffer via the "overlap" branch in `flush_buffer()`. The
//! consequence was quadratic content duplication: each chunk i+1 contained
//! a prefix of chunk i, making the output unusable for RAG ingestion.

use oxidize_pdf::parser::{PdfDocument, PdfReader};
use oxidize_pdf::pipeline::{
    Element, ElementBBox, ElementData, ElementMetadata, HybridChunkConfig, HybridChunker,
    MergePolicy,
};
use oxidize_pdf::text::Font;
use oxidize_pdf::{Document, Page};
use std::io::Cursor;

// ─── Helpers to build synthetic elements ─────────────────────────────────────

fn title(text: &str, page: u32, y: f64) -> Element {
    Element::Title(ElementData {
        text: text.to_string(),
        metadata: ElementMetadata {
            page,
            bbox: ElementBBox::new(50.0, y, 500.0, 18.0),
            parent_heading: Some(text.to_string()),
            ..Default::default()
        },
    })
}

fn paragraph(text: &str, page: u32, y: f64, heading: Option<&str>) -> Element {
    Element::Paragraph(ElementData {
        text: text.to_string(),
        metadata: ElementMetadata {
            page,
            bbox: ElementBBox::new(50.0, y, 500.0, 12.0),
            parent_heading: heading.map(String::from),
            ..Default::default()
        },
    })
}

// Return the textual "identity" of an element so we can detect duplicates
// without relying on object identity. For Paragraph/Title/Header/etc. this
// is the raw text; for tables/images it is a type-tag + bbox snapshot.
fn element_identity(e: &Element) -> String {
    let bbox = e.bbox();
    format!(
        "{}|p={}|x={:.1}|y={:.1}|w={:.1}|h={:.1}|t={}",
        e.type_name(),
        e.page(),
        bbox.x,
        bbox.y,
        bbox.width,
        bbox.height,
        e.display_text()
    )
}

fn assert_chunks_disjoint(chunks: &[oxidize_pdf::pipeline::HybridChunk]) {
    for i in 0..chunks.len() {
        for j in (i + 1)..chunks.len() {
            let ti = chunks[i].text();
            let tj = chunks[j].text();
            assert!(
                !ti.is_empty() && !tj.is_empty(),
                "chunks must have non-empty text"
            );
            assert!(
                !tj.contains(&ti),
                "chunk[{}].text() is a substring of chunk[{}].text():\n  i={:?}\n  j={:?}",
                i,
                j,
                ti,
                tj
            );
            assert!(
                !ti.contains(&tj),
                "chunk[{}].text() is a substring of chunk[{}].text():\n  i={:?}\n  j={:?}",
                j,
                i,
                tj,
                ti
            );
        }
    }
}

fn assert_coverage_equals_input(chunks: &[oxidize_pdf::pipeline::HybridChunk], input: &[Element]) {
    let mut input_ids: Vec<String> = input.iter().map(element_identity).collect();
    input_ids.sort();

    let mut chunk_ids: Vec<String> = chunks
        .iter()
        .flat_map(|c| c.elements().iter().map(element_identity))
        .collect();
    chunk_ids.sort();

    assert_eq!(
        chunk_ids, input_ids,
        "concat of chunk elements must equal input element list (no duplication, no loss)"
    );
}

// ─── Synthetic repro: Title + paragraphs, default config (overlap > 0) ───────

#[test]
fn synthetic_title_then_paragraphs_emits_disjoint_chunks() {
    // Default config: max_tokens=512, overlap_tokens=50, merge_adjacent=true,
    // merge_policy=AnyInlineContent. Title + three paragraphs trivially fit
    // under max_tokens, so the output must contain each source element
    // exactly once across all chunks.
    let elements = vec![
        title("HEAD ALPHA", 0, 750.0),
        paragraph("Para1 words words words.", 0, 720.0, Some("HEAD ALPHA")),
        paragraph("Para2 words words words.", 0, 700.0, Some("HEAD ALPHA")),
        paragraph("Para3 words words words.", 0, 680.0, Some("HEAD ALPHA")),
    ];

    let chunker = HybridChunker::new(HybridChunkConfig::default());
    let chunks = chunker.chunk(&elements);

    assert!(!chunks.is_empty(), "must produce at least one chunk");
    assert_chunks_disjoint(&chunks);
    assert_coverage_equals_input(&chunks, &elements);

    // Spec also accepts a single merged chunk as correct. Verify we never
    // produce more chunks than source elements (which would mean duplication).
    assert!(
        chunks.len() <= elements.len(),
        "chunk count ({}) must not exceed element count ({}) — excess indicates leaked overlap",
        chunks.len(),
        elements.len()
    );
}

// ─── Synthetic repro: size overflow with merge enabled ───────────────────────

#[test]
fn synthetic_overflow_flushes_emit_disjoint_chunks() {
    // Small max_tokens forces multiple overflow flushes. Each paragraph is
    // ~3 tokens; max_tokens=5 means at most one paragraph per chunk. Prior
    // to the fix, the overlap branch re-injected the last flushed paragraph
    // back into the buffer, causing each chunk to contain the previous
    // paragraph as well.
    let elements: Vec<Element> = (0..6)
        .map(|i| {
            paragraph(
                &format!("Para{} three tokens.", i),
                0,
                700.0 - i as f64 * 20.0,
                None,
            )
        })
        .collect();

    let chunker = HybridChunker::new(HybridChunkConfig {
        max_tokens: 5,
        overlap_tokens: 3,
        merge_adjacent: true,
        propagate_headings: false,
        merge_policy: MergePolicy::AnyInlineContent,
    });
    let chunks = chunker.chunk(&elements);

    assert!(!chunks.is_empty());
    assert_chunks_disjoint(&chunks);
    assert_coverage_equals_input(&chunks, &elements);
    assert_eq!(
        chunks.len(),
        elements.len(),
        "each 3-token paragraph must become its own chunk under max_tokens=5"
    );
}

// ─── Synthetic repro: merge_adjacent=false + overlap > 0 ─────────────────────

#[test]
fn synthetic_merge_disabled_with_overlap_still_disjoint() {
    // merge_adjacent=false was previously the worst-case: every element
    // triggered a flush, and the overlap branch copied the just-flushed
    // element back into the buffer, so every chunk contained the prior
    // element too.
    let elements: Vec<Element> = (0..5)
        .map(|i| {
            paragraph(
                &format!("Paragraph number {} with some words.", i),
                0,
                800.0 - i as f64 * 20.0,
                Some("Section A"),
            )
        })
        .collect();

    let chunker = HybridChunker::new(HybridChunkConfig {
        max_tokens: 20,
        overlap_tokens: 5,
        merge_adjacent: false,
        propagate_headings: true,
        merge_policy: MergePolicy::AnyInlineContent,
    });
    let chunks = chunker.chunk(&elements);

    assert_eq!(
        chunks.len(),
        elements.len(),
        "with merge disabled, chunker must emit exactly one chunk per element"
    );
    assert_chunks_disjoint(&chunks);
    assert_coverage_equals_input(&chunks, &elements);
    for chunk in &chunks {
        assert_eq!(
            chunk.elements().len(),
            1,
            "with merge disabled, each chunk must contain exactly one element"
        );
        assert_eq!(chunk.heading_context.as_deref(), Some("Section A"));
    }
}

// ─── End-to-end: build a PDF, parse it, chunk it ─────────────────────────────

#[test]
fn end_to_end_pdf_produces_disjoint_chunks() {
    // Build a PDF: one page with a 16pt bold title + three 11pt body
    // paragraphs. Parse it back via PdfDocument::rag_chunks() and assert
    // disjointness on the resulting RagChunks.
    let mut doc = Document::new();
    let mut page = Page::a4();

    page.text()
        .set_font(Font::HelveticaBold, 16.0)
        .at(50.0, 750.0)
        .write("HEAD ALPHA")
        .unwrap();

    page.text()
        .set_font(Font::Helvetica, 11.0)
        .at(50.0, 700.0)
        .write("Para1 body paragraph alpha content line.")
        .unwrap();
    page.text()
        .set_font(Font::Helvetica, 11.0)
        .at(50.0, 680.0)
        .write("Para2 body paragraph bravo content line.")
        .unwrap();
    page.text()
        .set_font(Font::Helvetica, 11.0)
        .at(50.0, 660.0)
        .write("Para3 body paragraph charlie content line.")
        .unwrap();

    doc.add_page(page);
    let pdf_bytes = doc.to_bytes().expect("pdf generation should succeed");

    let reader = PdfReader::new(Cursor::new(&pdf_bytes)).expect("parse generated PDF");
    let parsed = PdfDocument::new(reader);

    let chunks = parsed.rag_chunks().expect("rag_chunks must succeed");
    assert!(!chunks.is_empty(), "must emit at least one chunk");

    // Check disjointness pairwise on the RagChunk texts.
    for i in 0..chunks.len() {
        for j in (i + 1)..chunks.len() {
            let ti = &chunks[i].text;
            let tj = &chunks[j].text;
            assert!(
                !tj.contains(ti.as_str()),
                "rag chunk[{}].text is substring of rag chunk[{}].text:\n  i={:?}\n  j={:?}",
                i,
                j,
                ti,
                tj
            );
            assert!(
                !ti.contains(tj.as_str()),
                "rag chunk[{}].text is substring of rag chunk[{}].text:\n  i={:?}\n  j={:?}",
                j,
                i,
                tj,
                ti
            );
        }
    }

    // Each body paragraph's unique marker must appear in exactly one chunk.
    for marker in &["alpha content", "bravo content", "charlie content"] {
        let occurrences: usize = chunks.iter().filter(|c| c.text.contains(marker)).count();
        assert_eq!(
            occurrences, 1,
            "marker {:?} must appear in exactly one chunk, found in {}",
            marker, occurrences
        );
    }
}
