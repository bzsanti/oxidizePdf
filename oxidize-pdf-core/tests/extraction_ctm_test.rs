//! Tests for issue #262: TextExtractor must apply CTM to font_size/width and
//! must implement the q/Q graphics state stack so multiple BT/ET blocks within
//! one page report consistent page-space geometry.
//!
//! Each test constructs a synthetic PDF via the writer API, interleaves
//! `q`/`cm`/`BT...ET`/`Q` operations on the page, re-parses the saved bytes,
//! and asserts on the extracted `TextFragment.font_size` and coordinates.

use oxidize_pdf::parser::{PdfDocument, PdfReader};
use oxidize_pdf::text::{ExtractionOptions, TextExtractor};
use oxidize_pdf::{Document, Font, Page};
use std::io::Cursor;

fn build_pdf_bytes(build: impl FnOnce(&mut Page)) -> Vec<u8> {
    let mut doc = Document::new();
    let mut page = Page::a4();
    build(&mut page);
    doc.add_page(page);
    let tmp = tempfile::NamedTempFile::new().unwrap();
    doc.save(tmp.path()).unwrap();
    std::fs::read(tmp.path()).unwrap()
}

fn extract_fragments(bytes: Vec<u8>) -> Vec<oxidize_pdf::text::TextFragment> {
    let cursor = Cursor::new(bytes);
    let reader = PdfReader::new(cursor).expect("must parse synthetic PDF");
    let pdf_doc = PdfDocument::new(reader);
    let mut extractor = TextExtractor::with_options(ExtractionOptions {
        preserve_layout: true,
        ..Default::default()
    });
    let pages = extractor
        .extract_from_document(&pdf_doc)
        .expect("must extract");
    pages.into_iter().flat_map(|p| p.fragments).collect()
}

#[test]
fn ctm_uniform_scale_is_applied_to_font_size() {
    // q 10 0 0 10 0 0 cm BT /F1 1 Tf 1 0 0 1 5 5 Tm (Hello) Tj ET Q
    let bytes = build_pdf_bytes(|page| {
        page.graphics()
            .save_state()
            .transform(10.0, 0.0, 0.0, 10.0, 0.0, 0.0);
        page.text()
            .set_font(Font::Helvetica, 1.0)
            .at(5.0, 5.0)
            .write("Hello")
            .unwrap();
        page.graphics().restore_state();
    });

    let fragments = extract_fragments(bytes);
    let f = fragments
        .iter()
        .find(|f| f.text.contains("Hello"))
        .expect("must find Hello fragment");
    // 1pt × 10× CTM scale = 10pt in page space
    assert!(
        (f.font_size - 10.0).abs() < 0.01,
        "font_size must reflect CTM scaling: expected ~10, got {}",
        f.font_size
    );
    assert!(
        (f.height - 10.0).abs() < 0.01,
        "height must reflect CTM scaling: expected ~10, got {}",
        f.height
    );
    // Origin: text_matrix at (5,5) × CTM 10× = (50, 50) in page space
    assert!(
        (f.x - 50.0).abs() < 0.1,
        "x must be CTM-transformed: expected ~50, got {}",
        f.x
    );
    assert!(
        (f.y - 50.0).abs() < 0.1,
        "y must be CTM-transformed: expected ~50, got {}",
        f.y
    );
}

#[test]
fn q_restore_pops_ctm_so_second_block_is_not_scaled() {
    // Block 1 (scaled 10×) writes "A". After Q, block 2 (no scaling) writes "B".
    let bytes = build_pdf_bytes(|page| {
        // Block 1: 10× CTM, fs=1
        page.graphics()
            .save_state()
            .transform(10.0, 0.0, 0.0, 10.0, 0.0, 0.0);
        page.text()
            .set_font(Font::Helvetica, 1.0)
            .at(0.0, 0.0)
            .write("A")
            .unwrap();
        page.graphics().restore_state();

        // Block 2: identity CTM (after pop), fs=12 directly
        page.text()
            .set_font(Font::Helvetica, 12.0)
            .at(100.0, 100.0)
            .write("B")
            .unwrap();
    });

    let fragments = extract_fragments(bytes);

    let frag_a = fragments
        .iter()
        .find(|f| f.text == "A")
        .expect("must find A fragment");
    let frag_b = fragments
        .iter()
        .find(|f| f.text == "B")
        .expect("must find B fragment");

    // A: 1pt × 10× = 10pt page-space font size
    assert!(
        (frag_a.font_size - 10.0).abs() < 0.01,
        "A font_size: expected ~10 (1×10 CTM), got {}",
        frag_a.font_size
    );
    // B: 12pt at identity CTM (after Q popped the 10× scale)
    assert!(
        (frag_b.font_size - 12.0).abs() < 0.01,
        "B font_size: expected ~12 (Q must have restored identity CTM), got {} \
         — if you see ~120, the q/Q stack is missing and CTM keeps accumulating",
        frag_b.font_size
    );
    // B's origin should NOT be scaled by the (popped) 10× CTM
    assert!(
        (frag_b.x - 100.0).abs() < 0.5,
        "B x: expected ~100 (identity CTM), got {}",
        frag_b.x
    );
    assert!(
        (frag_b.y - 100.0).abs() < 0.5,
        "B y: expected ~100 (identity CTM), got {}",
        frag_b.y
    );
}

#[test]
fn nested_q_blocks_restore_correctly() {
    // q cm(2x) q cm(3x) BT show(A) ET Q  q cm(5x) BT show(B) ET Q  Q
    // After the outer Q, total scale was 2× × (whatever survived inside)
    //   A is rendered at 2× × 3× = 6× scale
    //   B is rendered at 2× × 5× = 10× scale
    let bytes = build_pdf_bytes(|page| {
        page.graphics()
            .save_state()
            .transform(2.0, 0.0, 0.0, 2.0, 0.0, 0.0);

        // Inner block 1: extra 3× on top of outer 2×
        page.graphics()
            .save_state()
            .transform(3.0, 0.0, 0.0, 3.0, 0.0, 0.0);
        page.text()
            .set_font(Font::Helvetica, 1.0)
            .at(0.0, 0.0)
            .write("A")
            .unwrap();
        page.graphics().restore_state(); // pop inner 3×; outer 2× still active

        // Inner block 2: extra 5× on top of outer 2× (the 3× was popped)
        page.graphics()
            .save_state()
            .transform(5.0, 0.0, 0.0, 5.0, 0.0, 0.0);
        page.text()
            .set_font(Font::Helvetica, 1.0)
            .at(0.0, 0.0)
            .write("B")
            .unwrap();
        page.graphics().restore_state(); // pop inner 5×; outer 2× still active

        page.graphics().restore_state(); // pop outer 2×; back to identity
    });

    let fragments = extract_fragments(bytes);
    let frag_a = fragments
        .iter()
        .find(|f| f.text == "A")
        .expect("must find A");
    let frag_b = fragments
        .iter()
        .find(|f| f.text == "B")
        .expect("must find B");

    // A: 1pt × 2× × 3× = 6pt
    assert!(
        (frag_a.font_size - 6.0).abs() < 0.01,
        "A font_size: expected ~6 (2×3 stacked CTM), got {}",
        frag_a.font_size
    );
    // B: 1pt × 2× × 5× = 10pt
    assert!(
        (frag_b.font_size - 10.0).abs() < 0.01,
        "B font_size: expected ~10 (2×5 stacked CTM after popping 3×), got {}",
        frag_b.font_size
    );
}

#[test]
fn unbalanced_q_does_not_crash() {
    // Extra Q without matching q — must not panic, should be a no-op
    let bytes = build_pdf_bytes(|page| {
        // No save_state, just restore — extractor must handle gracefully
        page.graphics().restore_state();
        page.text()
            .set_font(Font::Helvetica, 12.0)
            .at(50.0, 50.0)
            .write("ok")
            .unwrap();
    });

    let fragments = extract_fragments(bytes);
    let frag = fragments.iter().find(|f| f.text == "ok");
    assert!(
        frag.is_some(),
        "extraction must succeed even with unbalanced Q"
    );
}

#[test]
fn no_q_no_cm_baseline_unchanged() {
    // Identity CTM throughout. Without my fix, this was already correct;
    // my fix must not break this case.
    let bytes = build_pdf_bytes(|page| {
        page.text()
            .set_font(Font::Helvetica, 14.0)
            .at(100.0, 200.0)
            .write("Baseline")
            .unwrap();
    });

    let fragments = extract_fragments(bytes);
    let f = fragments
        .iter()
        .find(|f| f.text.contains("Baseline"))
        .expect("must find Baseline fragment");

    assert!(
        (f.font_size - 14.0).abs() < 0.01,
        "identity CTM must leave font_size unchanged: expected 14, got {}",
        f.font_size
    );
    assert!(
        (f.x - 100.0).abs() < 0.5,
        "identity CTM must leave x unchanged: expected 100, got {}",
        f.x
    );
    assert!(
        (f.y - 200.0).abs() < 0.5,
        "identity CTM must leave y unchanged: expected 200, got {}",
        f.y
    );
}
