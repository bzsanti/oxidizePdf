//! Regression / feature tests for issue #382 — `extract_from_page` had no
//! per-page text-length limit, so a single page with a huge content stream
//! materialised the whole decoded string in RAM before the caller saw it.
//!
//! `ExtractionOptions::max_extracted_bytes` caps the bytes of decoded text
//! accumulated for one page. The cap is enforced *during* accumulation (not by
//! truncating the finished string), so it bounds peak RAM. When the cap cuts
//! extraction short, `ExtractedText::truncated` is `true`.
//!
//! Semantics are *undershoot*: extraction stops before the fragment that would
//! push the accumulated bytes over the limit, so `text.len() <= limit` and a
//! multi-byte UTF-8 character is never split.

use oxidize_pdf::parser::{ParseOptions, PdfReader};
use oxidize_pdf::text::{ExtractedText, ExtractionOptions, TextExtractor};

/// Build a minimal, valid single-page PDF whose content stream is `content`.
/// `/F1` maps to Helvetica (Type1) so decoding is trivial (1 byte → 1 char).
fn build_pdf(content: &str) -> Vec<u8> {
    let clen = content.len();
    let o1 = "<< /Type /Catalog /Pages 3 0 R >>";
    let o2 = "<< /Type /Page /Parent 3 0 R /MediaBox [0 0 612 792] \
              /Resources << /Font << /F1 4 0 R >> >> /Contents 5 0 R >>";
    let o3 = "<< /Type /Pages /Kids [2 0 R] /Count 1 >>";
    let o4 = "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>";

    let mut buf = Vec::<u8>::new();
    buf.extend_from_slice(b"%PDF-1.4\n");
    let mut offsets = [0usize; 6];
    let mut push = |buf: &mut Vec<u8>, n: usize, body: &str| {
        offsets[n] = buf.len();
        buf.extend_from_slice(format!("{n} 0 obj\n{body}\nendobj\n").as_bytes());
    };
    push(&mut buf, 1, o1);
    push(&mut buf, 2, o2);
    push(&mut buf, 3, o3);
    push(&mut buf, 4, o4);

    offsets[5] = buf.len();
    buf.extend_from_slice(
        format!("5 0 obj\n<< /Length {clen} >>\nstream\n{content}\nendstream\nendobj\n").as_bytes(),
    );

    let xref_pos = buf.len();
    buf.extend_from_slice(b"xref\n0 6\n0000000000 65535 f \n");
    for offset in offsets.iter().skip(1) {
        buf.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    buf.extend_from_slice(
        format!("trailer\n<< /Size 6 /Root 1 0 R >>\nstartxref\n{xref_pos}\n%%EOF\n").as_bytes(),
    );
    buf
}

fn extract(content: &str, opts: ExtractionOptions) -> ExtractedText {
    let doc = PdfReader::new_with_options(
        std::io::Cursor::new(build_pdf(content)),
        ParseOptions::lenient(),
    )
    .expect("PDF should parse")
    .into_document();
    TextExtractor::with_options(opts)
        .extract_from_page(&doc, 0)
        .expect("extraction should succeed")
}

/// A page with `lines` separate `Tj` show operators, each drawing `chunk`
/// (ASCII, so one byte of content stream = one byte of decoded text). Each
/// operator is its own text fragment, so the undershoot boundary is exercised
/// at fragment granularity.
fn build_repeated_text(chunk: &str, lines: usize) -> String {
    let mut c = String::new();
    let mut y = 760.0_f64;
    for _ in 0..lines {
        c.push_str(&format!(
            "BT\n/F1 12 Tf\n1 0 0 1 72 {y:.2} Tm\n({chunk}) Tj\nET\n"
        ));
        y -= 14.0;
        if y < 20.0 {
            y = 760.0;
        }
    }
    c
}

// ── Cycle 1: cap bounds the flat (default) path and sets the flag ────────────

#[test]
fn test_bounds_flat_text_and_sets_flag() {
    // 40 fragments × 50 chars ≈ 2 KB of decoded text (plus newlines the
    // extractor inserts between fragments on separate lines).
    let content = build_repeated_text(&"A".repeat(50), 40);

    // No limit: full text, not truncated.
    let full = extract(&content, ExtractionOptions::default());
    assert!(
        full.text.len() >= 40 * 50,
        "unbounded extraction should contain all {} chars, got {}",
        40 * 50,
        full.text.len()
    );
    assert!(
        !full.truncated,
        "unbounded extraction must not be truncated"
    );

    // With a 500-byte cap: bounded and flagged.
    let opts = ExtractionOptions {
        max_extracted_bytes: Some(500),
        ..Default::default()
    };
    let capped = extract(&content, opts);
    assert!(
        capped.text.len() <= 500,
        "capped text must be <= 500 bytes, got {}",
        capped.text.len()
    );
    assert!(
        capped.truncated,
        "capped extraction must set truncated = true"
    );
    // Content check (not just length): the kept text is a genuine prefix of the
    // full extraction — extraction stopped early, it did not reorder or mangle.
    assert!(
        full.text.starts_with(&capped.text),
        "capped text must be a prefix of the full text"
    );
    assert!(!capped.text.is_empty(), "a 500-byte cap keeps some text");
}

// ── Cycle 2: the cap flows through the reorder / preserve-layout paths ────────

#[test]
fn test_bounds_reorder_and_preserve_layout() {
    let content = build_repeated_text(&"B".repeat(40), 40);

    for (label, mut opts) in [
        ("reorder_columns", ExtractionOptions::default()),
        ("preserve_layout", ExtractionOptions::default()),
    ] {
        match label {
            "reorder_columns" => opts.reorder_columns = true,
            _ => opts.preserve_layout = true,
        }

        let full = extract(&content, opts.clone());
        assert!(!full.truncated, "{label}: unbounded must not be truncated");

        opts.max_extracted_bytes = Some(400);
        let capped = extract(&content, opts);

        // The flag is authoritative for every path.
        assert!(
            capped.truncated,
            "{label}: capped extraction must set truncated"
        );
        // These paths rebuild `.text` from the already-bounded fragment set and
        // insert their own separators, but the final `clamp_to_budget` safety
        // net guarantees the hard invariant on every path, not just the flat one.
        assert!(
            capped.text.len() <= 400,
            "{label}: capped text ({}) must respect the 400-byte cap",
            capped.text.len()
        );
        assert!(
            capped.text.len() < full.text.len(),
            "{label}: capped text ({}) must be shorter than full ({})",
            capped.text.len(),
            full.text.len()
        );
        assert!(
            !capped.text.is_empty(),
            "{label}: a 400-byte cap keeps text"
        );
    }
}

// ── Cycle 3: the cap never splits a multi-byte UTF-8 character ────────────────

/// `build_pdf`, but the content stream is raw bytes so we can embed WinAnsi
/// high bytes (e.g. `0xE9` → `é`) that decode to multi-byte UTF-8.
fn build_pdf_bytes(content: &[u8]) -> Vec<u8> {
    let clen = content.len();
    let o1 = "<< /Type /Catalog /Pages 3 0 R >>";
    let o2 = "<< /Type /Page /Parent 3 0 R /MediaBox [0 0 612 792] \
              /Resources << /Font << /F1 4 0 R >> >> /Contents 5 0 R >>";
    let o3 = "<< /Type /Pages /Kids [2 0 R] /Count 1 >>";
    let o4 = "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica \
              /Encoding /WinAnsiEncoding >>";

    let mut buf = Vec::<u8>::new();
    buf.extend_from_slice(b"%PDF-1.4\n");
    let mut offsets = [0usize; 6];
    let mut push = |buf: &mut Vec<u8>, n: usize, body: &str| {
        offsets[n] = buf.len();
        buf.extend_from_slice(format!("{n} 0 obj\n{body}\nendobj\n").as_bytes());
    };
    push(&mut buf, 1, o1);
    push(&mut buf, 2, o2);
    push(&mut buf, 3, o3);
    push(&mut buf, 4, o4);

    offsets[5] = buf.len();
    buf.extend_from_slice(format!("5 0 obj\n<< /Length {clen} >>\nstream\n").as_bytes());
    buf.extend_from_slice(content);
    buf.extend_from_slice(b"\nendstream\nendobj\n");

    let xref_pos = buf.len();
    buf.extend_from_slice(b"xref\n0 6\n0000000000 65535 f \n");
    for offset in offsets.iter().skip(1) {
        buf.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    buf.extend_from_slice(
        format!("trailer\n<< /Size 6 /Root 1 0 R >>\nstartxref\n{xref_pos}\n%%EOF\n").as_bytes(),
    );
    buf
}

fn extract_bytes(content: &[u8], opts: ExtractionOptions) -> ExtractedText {
    let doc = PdfReader::new_with_options(
        std::io::Cursor::new(build_pdf_bytes(content)),
        ParseOptions::lenient(),
    )
    .expect("PDF should parse")
    .into_document();
    TextExtractor::with_options(opts)
        .extract_from_page(&doc, 0)
        .expect("extraction should succeed")
}

#[test]
fn test_never_splits_utf8() {
    // Each `Tj` draws 10 × `0xE9` (WinAnsi 'é', 2 UTF-8 bytes each → 20 bytes of
    // decoded text per fragment). A 45-byte cap lands *inside* the third
    // fragment; undershoot must drop that whole fragment, never half a 'é'.
    let mut content = Vec::<u8>::new();
    let mut y = 760.0_f64;
    for _ in 0..20 {
        content.extend_from_slice(format!("BT\n/F1 12 Tf\n1 0 0 1 72 {y:.2} Tm\n(").as_bytes());
        content.extend(std::iter::repeat_n(0xE9u8, 10));
        content.extend_from_slice(b") Tj\nET\n");
        y -= 14.0;
    }

    // Sanity: unbounded decode really produced multi-byte 'é'.
    let full = extract_bytes(&content, ExtractionOptions::default());
    assert!(
        full.text.contains('é'),
        "fixture must decode WinAnsi 0xE9 to 'é', got {:?}",
        full.text.chars().take(8).collect::<String>()
    );
    assert!(!full.truncated);

    let opts = ExtractionOptions {
        max_extracted_bytes: Some(45),
        ..Default::default()
    };
    let capped = extract_bytes(&content, opts);

    // No panic, valid UTF-8 (String guarantees it), and bounded.
    assert!(
        capped.truncated,
        "45-byte cap on 20-byte fragments must truncate"
    );
    assert!(
        capped.text.len() <= 45,
        "flat path holds text.len() <= limit"
    );
    // Undershoot at fragment granularity: the kept text is a whole number of
    // fragments, so every 'é' is complete — assert no lone continuation byte.
    assert!(
        std::str::from_utf8(capped.text.as_bytes()).is_ok(),
        "text must be valid UTF-8"
    );
    assert!(
        full.text.starts_with(&capped.text),
        "capped text is a clean prefix, no split char"
    );
}

// ── Cycle 4: zero budget and empty page ──────────────────────────────────────

#[test]
fn test_zero_budget_and_empty_page() {
    // Some(0): a page with text yields empty output, flagged truncated.
    let content = build_repeated_text(&"C".repeat(20), 5);
    let zero = extract(
        &content,
        ExtractionOptions {
            max_extracted_bytes: Some(0),
            ..Default::default()
        },
    );
    assert!(
        zero.text.is_empty(),
        "Some(0) keeps no text, got {:?}",
        zero.text
    );
    assert!(zero.truncated, "Some(0) on a non-empty page is truncated");

    // Empty page (no show-text ops): never truncated, whatever the limit.
    let empty_page = "BT\n/F1 12 Tf\n1 0 0 1 72 700 Tm\nET\n";
    let empty = extract(
        empty_page,
        ExtractionOptions {
            max_extracted_bytes: Some(100),
            ..Default::default()
        },
    );
    assert!(empty.text.is_empty(), "no show-text → no text");
    assert!(!empty.truncated, "a page with no text is never truncated");
}

// ── Cycle 5: composes with a document-level output budget (the issue's loop) ──

#[test]
fn test_composes_with_document_budget() {
    // The issue's motivating loop: a per-page cap bounds each page's peak, while
    // the caller keeps its own document-level output budget. Here we drive one
    // page repeatedly (a stand-in for a multi-page document) and confirm every
    // page is individually bounded and correctly flagged.
    let content = build_repeated_text(&"D".repeat(60), 30);
    let per_page_cap = 300usize;

    let mut doc_budget = 1000usize;
    let mut out = String::new();
    let mut any_page_truncated = false;

    for _ in 0..5 {
        let page = extract(
            &content,
            ExtractionOptions {
                max_extracted_bytes: Some(per_page_cap),
                ..Default::default()
            },
        );
        // Per-page peak is bounded regardless of the document budget.
        assert!(
            page.text.len() <= per_page_cap,
            "each page must respect the per-page cap"
        );
        assert!(page.truncated, "each dense page hits the per-page cap");
        any_page_truncated |= page.truncated;

        // Caller's document-level truncation still composes on top.
        let take = page.text.len().min(doc_budget);
        out.push_str(&page.text[..take]);
        doc_budget -= take;
        if doc_budget == 0 {
            break;
        }
    }

    assert!(any_page_truncated);
    assert!(out.len() <= 1000, "document output budget is respected");
    assert!(!out.is_empty());
}

// ── Cycle 6: /ActualText override cannot bypass the budget (QR critical #1) ────

#[test]
fn test_actualtext_scope_respects_budget() {
    // A single marked-content scope declaring an inline `/ActualText` string of
    // 4000 bytes, populated by one tiny `Tj`. On the layout / reorder paths the
    // scope's fragment text is the *declared* string, and `.text` is rebuilt
    // from fragments — so without budgeting this leaks the whole 4000 bytes with
    // `truncated == false`. This is the adversarial single-operator case #382
    // exists to defend against.
    let big = "Z".repeat(4000);
    let content = format!(
        "BT\n/F1 12 Tf\n1 0 0 1 72 700 Tm\n\
         /Span << /ActualText ({big}) >> BDC\n(x) Tj\nEMC\nET\n"
    );

    for (label, base) in [
        ("preserve_layout", {
            ExtractionOptions {
                preserve_layout: true,
                ..ExtractionOptions::default()
            }
        }),
        ("reorder_columns", {
            ExtractionOptions {
                reorder_columns: true,
                ..ExtractionOptions::default()
            }
        }),
    ] {
        // Unbounded: the ActualText override does surface (sanity that the
        // fixture exercises the ActualText path at all).
        let full = extract(&content, base.clone());
        assert!(
            full.text.len() >= 4000,
            "{label}: fixture must exercise ActualText (got {} bytes)",
            full.text.len()
        );
        assert!(!full.truncated, "{label}: unbounded not truncated");

        // Bounded: the override must NOT escape the cap, and the flag must tell
        // the truth.
        let mut opts = base;
        opts.max_extracted_bytes = Some(50);
        let capped = extract(&content, opts);
        assert!(
            capped.text.len() <= 50,
            "{label}: ActualText must not bypass the 50-byte cap, got {}",
            capped.text.len()
        );
        assert!(
            capped.truncated,
            "{label}: a bypassed budget must still report truncated"
        );
    }
}
