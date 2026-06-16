//! Acceptance tests for issue #330: `extract_from_page().text` and
//! `extract_from_page().fragments` must be CONSISTENT — pages with text
//! content must not produce zero fragments (silent-drop in RAG pipeline)
//! and pages with no content must not produce ghost text without fragments.
//!
//! Reproducer in #330: a closing legal-disclaimer page in a slide deck
//! returned `text-len: 2434, fragments: 0` while body pages had `text-len:
//! 1373, fragments: 834`. Such pages then vanish from `partition_with(...)`
//! and `rag_chunks(...)` because the partitioner operates on `fragments`,
//! while the user sees the text in `.text` — silent inconsistency.
//!
//! Root cause being verified: `ShowText` / `ShowTextArray` /
//! `NextLineShowText` / `SetSpacingNextLineShowText` push to
//! `extracted_text` unconditionally (extraction.rs:879, 927, 1033, 1090)
//! but `emit_text_fragment` (extraction.rs:1846-1848) returns early when
//! the marked-content stack contains an Artifact entry and
//! `include_artifacts` is false (the default). Result: a page entirely
//! wrapped in an `/Artifact BMC … EMC` (a common pattern for screen-reader
//! "skip" sections like legal disclaimers and slide footers) produces
//! text-with-no-fragments.

#[path = "common/mod.rs"]
mod common;
use common::synthetic_pdf::build_pdf_with_content_stream;

use oxidize_pdf::parser::{PdfDocument, PdfReader};
use oxidize_pdf::text::{ExtractedText, ExtractionOptions, TextExtractor};
use std::io::Cursor;

fn extract(content: &[u8], include_artifacts: bool) -> ExtractedText {
    let pdf = build_pdf_with_content_stream(content);
    let reader = PdfReader::new(Cursor::new(pdf)).expect("reader");
    let document = PdfDocument::new(reader);
    let opts = ExtractionOptions {
        preserve_layout: true,
        include_artifacts,
        ..ExtractionOptions::default()
    };
    let mut extractor = TextExtractor::with_options(opts);
    extractor.extract_from_page(&document, 0).expect("extract")
}

/// Primary RED contract from #330: a page whose content is entirely inside
/// an `/Artifact BMC … EMC` scope must produce a consistent state —
/// either text AND fragments are empty (correct for `include_artifacts =
/// false`), or both are non-empty. The current bug is the asymmetric state
/// where text > 0 and fragments = 0.
#[test]
fn artifact_only_page_has_consistent_text_and_fragments() {
    // Whole-page Artifact wrap (common pattern for PowerPoint/Keynote
    // closing-disclaimer slides exported via accessibility-aware tooling).
    let content = b"BT\n/F1 12 Tf\n100 700 Td\n\
                    /Artifact BMC\n\
                    (Disclaimer line one of legal boilerplate.) Tj\n\
                    0 -14 Td\n\
                    (Disclaimer line two more boilerplate.) Tj\n\
                    EMC\n\
                    ET\n";

    let extracted = extract(content, false);

    let text_has_content = !extracted.text.trim().is_empty();
    let frags_has_content = !extracted.fragments.is_empty();
    assert_eq!(
        text_has_content,
        frags_has_content,
        ".text and .fragments must agree on whether the page has content. \
         text-len={} ({:?}); fragments={}",
        extracted.text.len(),
        extracted.text,
        extracted.fragments.len(),
    );
}

/// Generalised invariant from #330's Acceptance section: for any page where
/// `.text` has characters, `.fragments` must be non-empty. Same fixture as
/// above but the assertion is phrased per the acceptance criterion.
#[test]
fn extracted_text_non_empty_implies_fragments_non_empty() {
    let content = b"BT\n/F1 12 Tf\n100 700 Td\n\
                    /Artifact BMC\n\
                    (Footer text that would land in .text but never in .fragments.) Tj\n\
                    EMC\n\
                    ET\n";

    let extracted = extract(content, false);

    if !extracted.text.trim().is_empty() {
        assert!(
            !extracted.fragments.is_empty(),
            "Acceptance #330: text contains {} chars but fragments is empty — \
             silent-drop in the RAG pipeline. text={:?}",
            extracted.text.len(),
            extracted.text,
        );
    }
}

/// Opt-in symmetry: with `include_artifacts = true` both surfaces must
/// carry the artifact content. Guards against an over-correction where the
/// fix gates `.text` too aggressively and loses the opt-in capability.
#[test]
fn include_artifacts_true_emits_both_text_and_fragments() {
    let content = b"BT\n/F1 12 Tf\n100 700 Td\n\
                    /Artifact BMC\n\
                    (Artifact body opted in.) Tj\n\
                    EMC\n\
                    ET\n";

    let extracted = extract(content, true);

    assert!(
        extracted.text.contains("Artifact body opted in"),
        "with include_artifacts=true, .text must include the artifact content; \
         got text={:?}",
        extracted.text
    );
    assert!(
        extracted
            .fragments
            .iter()
            .any(|f| f.text.contains("Artifact body opted in")),
        "with include_artifacts=true, .fragments must include the artifact content; \
         got fragments={:?}",
        extracted
            .fragments
            .iter()
            .map(|f| &f.text)
            .collect::<Vec<_>>()
    );
}

/// Control: non-artifact text on the same fixture must surface as today —
/// the fix must not strip ordinary text.
#[test]
fn ordinary_text_surfaces_in_both_text_and_fragments() {
    let content = b"BT\n/F1 12 Tf\n100 700 Td\n\
                    (Ordinary body text.) Tj\n\
                    ET\n";

    let extracted = extract(content, false);

    assert!(
        extracted.text.contains("Ordinary body text"),
        ".text must carry ordinary (non-artifact) content"
    );
    assert!(
        !extracted.fragments.is_empty(),
        ".fragments must carry ordinary (non-artifact) content"
    );
}
