//! Regression tests for issue #235 — `rag_pipeline` returned 0 chunks for
//! TJ-heavy academic PDFs because the `preserve_layout` text extractor only
//! emitted `TextFragment` for `Tj` (`ShowText`), silently dropping `TJ`
//! (`ShowTextArray`), `'` (`NextLineShowText`), and `"`
//! (`SetSpacingNextLineShowText`).
//!
//! Each test asserts on **content** of the extracted fragments, never on
//! "shape" (count > 0, !is_empty, etc.) — see CLAUDE.local.md.

use oxidize_pdf::parser::{PdfDocument, PdfReader};
use oxidize_pdf::text::{ExtractionOptions, TextExtractor};
use std::io::{BufReader, Cursor};
use std::path::PathBuf;

fn fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/issue_235_t.pdf")
}

fn open_fixture() -> PdfDocument<std::fs::File> {
    let reader = PdfReader::open(fixture_path()).expect("fixture must open");
    PdfDocument::new(reader)
}

/// Write a PDF object body to `bytes`, recording its absolute offset in
/// `offset` for the xref table. Used by `build_pdf_with_content_stream`
/// to lay out objects 1..5 sequentially.
fn write_obj(bytes: &mut Vec<u8>, offset: &mut usize, body: &str) {
    *offset = bytes.len();
    bytes.extend_from_slice(body.as_bytes());
}

/// Build a minimal valid 1-page PDF whose Contents stream is the supplied
/// raw byte sequence (typically a hand-crafted sequence of text operators).
/// Resources expose a single Type1 Helvetica font as `/F1`.
fn build_pdf_with_content_stream(content: &[u8]) -> Vec<u8> {
    let mut bytes: Vec<u8> = Vec::with_capacity(1024 + content.len());
    let mut offsets: Vec<usize> = vec![0; 6]; // index by object id (1..=5)

    bytes.extend_from_slice(b"%PDF-1.4\n%\xE2\xE3\xCF\xD3\n");

    write_obj(
        &mut bytes,
        &mut offsets[1],
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n",
    );
    write_obj(
        &mut bytes,
        &mut offsets[2],
        "2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n",
    );
    write_obj(
        &mut bytes,
        &mut offsets[3],
        "3 0 obj\n<< /Type /Page /Parent 2 0 R /Resources << /Font << /F1 4 0 R >> >> /Contents 5 0 R /MediaBox [0 0 612 792] >>\nendobj\n",
    );
    write_obj(
        &mut bytes,
        &mut offsets[4],
        "4 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>\nendobj\n",
    );

    offsets[5] = bytes.len();
    bytes.extend_from_slice(
        format!("5 0 obj\n<< /Length {} >>\nstream\n", content.len()).as_bytes(),
    );
    bytes.extend_from_slice(content);
    bytes.extend_from_slice(b"\nendstream\nendobj\n");

    let xref_off = bytes.len();
    bytes.extend_from_slice(b"xref\n0 6\n0000000000 65535 f \n");
    for off in offsets.iter().skip(1) {
        bytes.extend_from_slice(format!("{:010} 00000 n \n", off).as_bytes());
    }
    bytes.extend_from_slice(
        format!(
            "trailer\n<< /Size 6 /Root 1 0 R >>\nstartxref\n{}\n%%EOF\n",
            xref_off
        )
        .as_bytes(),
    );

    bytes
}

fn extract_synthetic(content: &[u8]) -> oxidize_pdf::text::ExtractedText {
    let pdf = build_pdf_with_content_stream(content);
    let reader =
        PdfReader::new(BufReader::new(Cursor::new(pdf))).expect("synthetic PDF must parse");
    let doc = PdfDocument::new(reader);
    let opts = ExtractionOptions {
        preserve_layout: true,
        sort_by_position: false,
        ..Default::default()
    };
    let mut extractor = TextExtractor::with_options(opts);
    extractor
        .extract_from_page(&doc, 0)
        .expect("synthetic page must extract")
}

#[test]
fn tj_operator_emits_fragments_for_every_body_page() {
    // t.pdf is a 52-page LaTeX academic paper (SWE-bench, arXiv 2310.06770v3).
    // Body pages (index 1..=51) render text exclusively with the TJ operator.
    // Pre-fix: pages 1..=51 each return zero fragments.
    // Post-fix: each must yield text fragments whose content reproduces the
    // page header that LaTeX emits at the top of every page.
    let doc = open_fixture();

    let opts = ExtractionOptions {
        preserve_layout: true,
        sort_by_position: true,
        ..Default::default()
    };
    let mut extractor = TextExtractor::with_options(opts);
    let pages = extractor
        .extract_from_document(&doc)
        .expect("extraction must succeed");

    assert_eq!(
        pages.len(),
        52,
        "fixture must yield 52 pages, got {}",
        pages.len()
    );

    // Every body page contains the conference header verbatim.
    // We verify content reconstruction (not just fragment count) so the
    // test cannot pass with empty-shape fragments.
    let mut pages_missing_header = Vec::new();
    let header_substrings = ["Published", "ICLR", "2024"];

    for (i, page) in pages.iter().enumerate().skip(1) {
        let joined: String = page
            .fragments
            .iter()
            .map(|f| f.text.as_str())
            .collect::<Vec<_>>()
            .join(" ");
        let has_all = header_substrings.iter().all(|s| joined.contains(s));
        if !has_all {
            pages_missing_header.push((
                i,
                page.fragments.len(),
                joined.chars().take(120).collect::<String>(),
            ));
        }
    }
    assert!(
        pages_missing_header.is_empty(),
        "pages whose joined fragments do not contain the LaTeX header \"Published as a conference paper at ICLR 2024\": {:#?}",
        pages_missing_header
    );
}

#[test]
fn tj_fragments_carry_real_glyph_metrics() {
    // Guards against a regression where Spacing array elements (numbers,
    // not text) emit zero-width / empty-text fragments. Also guards
    // against fragments inheriting font_size = 0 or losing the font name.
    let doc = open_fixture();
    let opts = ExtractionOptions {
        preserve_layout: true,
        sort_by_position: false, // raw, unmerged so we observe each push
        ..Default::default()
    };
    let mut extractor = TextExtractor::with_options(opts);
    let pages = extractor
        .extract_from_document(&doc)
        .expect("extraction must succeed");

    let mut empty_text = 0usize;
    let mut zero_width = 0usize;
    let mut zero_font_size = 0usize;
    let mut missing_font_name = 0usize;
    let mut samples: Vec<String> = Vec::new();

    for page in &pages {
        for f in &page.fragments {
            if f.text.is_empty() {
                empty_text += 1;
            }
            if !f.text.is_empty() && f.width <= 0.0 {
                zero_width += 1;
                if samples.len() < 3 {
                    samples.push(format!(
                        "zero_width: {:?} font_size={}",
                        f.text, f.font_size
                    ));
                }
            }
            if f.font_size <= 0.0 {
                zero_font_size += 1;
            }
            if f.font_name.is_none() {
                missing_font_name += 1;
            }
        }
    }

    let total: usize = pages.iter().map(|p| p.fragments.len()).sum();
    assert!(
        total >= 1000,
        "extractor must emit a meaningful corpus; got {}",
        total
    );

    assert_eq!(
        empty_text, 0,
        "Spacing array elements must not produce empty-text fragments"
    );
    assert_eq!(
        zero_font_size, 0,
        "every fragment must inherit a positive font_size from the text state"
    );
    assert_eq!(
        missing_font_name, 0,
        "every fragment must record the font_name active when it was emitted"
    );
    // `calculate_text_width` has a known fallback path that returns 0 for
    // fonts whose Widths array is present but empty/zeroed (e.g. some Type 3
    // and tightly-encoded CID fonts). This is a pre-existing parser
    // limitation independent of fragment emission. Allow it to leak through
    // at a rate ≤ 0.5 % so it cannot mask a wholesale regression.
    let zw_rate = zero_width as f64 / total as f64;
    assert!(
        zw_rate <= 0.005,
        "zero-width fragments must be a rare edge case (≤ 0.5 %); got {}/{} = {:.4} %, samples: {:?}",
        zero_width,
        total,
        100.0 * zw_rate,
        samples
    );
}

#[test]
fn tj_only_page_zero_watermark_still_extractable() {
    // Page 0 of t.pdf has the rotated arXiv submission watermark drawn with
    // a single `Tj` call. This already produced a fragment BEFORE the fix.
    // Guard the refactor (Cycle 5) does not break the canonical Tj path.
    let doc = open_fixture();
    let opts = ExtractionOptions {
        preserve_layout: true,
        sort_by_position: false,
        ..Default::default()
    };
    let mut extractor = TextExtractor::with_options(opts);
    let page0 = extractor
        .extract_from_page(&doc, 0)
        .expect("page 0 must extract");

    let watermark = page0.fragments.iter().find(|f| f.text.contains("arXiv"));
    let wm = watermark.unwrap_or_else(|| {
        panic!(
            "page 0 must still emit the arXiv watermark fragment; got fragments: {:?}",
            page0.fragments.iter().map(|f| &f.text).collect::<Vec<_>>()
        )
    });
    assert!(
        wm.text.contains("2310.06770"),
        "watermark text must contain the arXiv id; got {:?}",
        wm.text
    );
    assert!(
        wm.font_size > 0.0,
        "watermark font_size must be positive; got {}",
        wm.font_size
    );
}

#[test]
fn rag_pipeline_recovers_body_chunks_for_tj_pdf() {
    // End-to-end: the bug surfaces as `rag_chunks()` returning ≤ 1 chunks.
    // After the fix, the 52-page paper must yield at least 10 chunks and
    // every chunk must contain non-whitespace text that includes recognisable
    // English content from the paper (verifying we are not just emitting
    // garbage to inflate the count).
    let doc = open_fixture();
    let chunks = doc.rag_chunks().expect("rag_chunks must succeed");

    assert!(
        chunks.len() >= 10,
        "rag_chunks() returned {} chunks for a 52-page LaTeX paper; expected >= 10",
        chunks.len()
    );

    let empty_chunks: Vec<usize> = chunks
        .iter()
        .enumerate()
        .filter(|(_, c)| c.text.trim().is_empty())
        .map(|(i, _)| i)
        .collect();
    assert!(
        empty_chunks.is_empty(),
        "chunks {:?} have empty/whitespace text",
        empty_chunks
    );

    let corpus: String = chunks
        .iter()
        .map(|c| c.text.as_str())
        .collect::<Vec<_>>()
        .join(" ");
    // Phrases that appear verbatim in the SWE-bench abstract / body.
    // Each chosen as a distinctive string unlikely to collide with PDF
    // metadata or noise. Post-#262 the partitioner classifies the paper
    // title as a table, so the multi-word "language model" phrase becomes
    // "LANGUAGE | MODELS" with column separators; we use the single
    // distinctive word "language" instead (49 occurrences in the corpus).
    let signature_phrases = ["SWE-bench", "ICLR", "language"];
    let missing: Vec<&str> = signature_phrases
        .iter()
        .copied()
        .filter(|phrase| !corpus.to_lowercase().contains(&phrase.to_lowercase()))
        .collect();
    assert!(
        missing.is_empty(),
        "rag chunks corpus missing expected SWE-bench phrases: {:?}",
        missing
    );
}

#[test]
fn apostrophe_operator_advances_line_and_emits_fragment() {
    // `'` operator (NextLineShowText): equivalent to T* then Tj.
    // Pre-fix: falls through to the `_ => {}` catch-all, no fragment.
    // Post-fix: emits the text fragment AND advances y by `-leading`.
    let content = b"BT\n\
        /F1 12 Tf\n\
        14 TL\n\
        100 700 Td\n\
        (Hello) Tj\n\
        (World) '\n\
        ET\n";
    let extracted = extract_synthetic(content);

    let frags = &extracted.fragments;
    assert_eq!(
        frags.len(),
        2,
        "expected 2 fragments (Tj + '), got {}: {:?}",
        frags.len(),
        frags.iter().map(|f| &f.text).collect::<Vec<_>>()
    );
    assert_eq!(
        frags[0].text, "Hello",
        "first fragment must be the Tj string"
    );
    assert_eq!(
        frags[1].text, "World",
        "second fragment must be emitted by the `'` operator"
    );
    // `'` invokes T* which moves the line matrix down by `leading` (14 pt).
    // PDF Y axis: smaller Y = further down the page.
    let dy = frags[0].y - frags[1].y;
    assert!(
        (dy - 14.0).abs() < 0.01,
        "`'` must drop y by exactly the leading (14 pt); got dy = {} ({} → {})",
        dy,
        frags[0].y,
        frags[1].y
    );
    // X must reset to the line-matrix origin (100 pt from the Td above).
    assert!(
        (frags[1].x - 100.0).abs() < 0.01,
        "`'` must reset x to the line-matrix origin (100 pt); got x = {}",
        frags[1].x
    );
}

#[test]
fn double_quote_operator_sets_spacing_advances_line_and_emits_fragment() {
    // `"` operator (SetSpacingNextLineShowText): equivalent to
    //   aw Tw   ac Tc   T*   Tj
    // Pre-fix: falls through to the `_ => {}` catch-all, no fragment.
    // Post-fix: emits the fragment, advances y by `-leading`, AND
    //           subsequent Tj show calls reflect the new word/char spacing.
    let content = b"BT\n\
        /F1 12 Tf\n\
        14 TL\n\
        100 700 Td\n\
        (Base) Tj\n\
        2.5 1.5 (Spaced) \"\n\
        (After) Tj\n\
        ET\n";
    let extracted = extract_synthetic(content);

    let frags = &extracted.fragments;
    // `merge_close_fragments` collapses adjacent same-line runs, so the
    // trailing Tj at the same y as the `"` line merges with it. The bug
    // being fixed is `"` silently swallowing its text; observing
    // "Spaced" AND "After" in the merged second fragment confirms both
    // (a) `"` emitted its text and (b) the trailing Tj inherited the
    // post-`"` line position.
    assert_eq!(
        frags.len(),
        2,
        "expected 2 fragments after merge (Tj + (\" merged with trailing Tj)), got {}: {:?}",
        frags.len(),
        frags.iter().map(|f| &f.text).collect::<Vec<_>>()
    );
    assert_eq!(frags[0].text, "Base");
    assert!(
        frags[1].text.contains("Spaced"),
        "merged second fragment must contain `\"`'s string \"Spaced\"; got {:?}",
        frags[1].text
    );
    assert!(
        frags[1].text.contains("After"),
        "merged second fragment must include the trailing Tj \"After\"; got {:?}",
        frags[1].text
    );
    // Line advance: `"` must have moved y down by `leading` (14 pt).
    let dy = frags[0].y - frags[1].y;
    assert!(
        (dy - 14.0).abs() < 0.01,
        "`\"` must drop y by exactly the leading (14 pt); got dy = {} ({} → {})",
        dy,
        frags[0].y,
        frags[1].y
    );
}
