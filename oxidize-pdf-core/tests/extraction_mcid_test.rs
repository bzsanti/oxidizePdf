//! Issue #269 Phase 1 — `TextFragment.mcid` and `struct_tag` carry the
//! innermost BDC ancestor's identity; nested BDCs are resolved to the
//! innermost MCID-bearing entry.

#[path = "common/mod.rs"]
mod common;
use common::synthetic_pdf::build_pdf_with_content_stream;

use oxidize_pdf::parser::{PdfDocument, PdfReader};
use oxidize_pdf::text::{ExtractionOptions, TextExtractor};
use std::io::Cursor;

fn extract_fragments(
    content: &[u8],
    options: ExtractionOptions,
) -> Vec<oxidize_pdf::text::TextFragment> {
    let pdf = build_pdf_with_content_stream(content);
    let reader = PdfReader::new(Cursor::new(pdf)).expect("reader");
    let document = PdfDocument::new(reader);
    let mut extractor = TextExtractor::with_options(options);
    let extracted = extractor
        .extract_from_page(&document, 0)
        .expect("extract page 0");
    extracted.fragments
}

#[test]
fn nested_bdc_innermost_mcid_and_tag_win() {
    // /P <</MCID 0>> BDC /Span BMC (x) Tj EMC EMC
    // Expected: fragment.mcid = 0 (from /P; /Span has no MCID), struct_tag = "P".
    let content = b"BT\n/F1 12 Tf\n100 700 Td\n\
                    /P << /MCID 0 >> BDC\n\
                    /Span BMC\n\
                    (x) Tj\n\
                    EMC\n\
                    EMC\n\
                    ET\n";
    let opts = ExtractionOptions {
        preserve_layout: true,
        ..ExtractionOptions::default()
    };
    let frags = extract_fragments(content, opts);

    let frag = frags
        .iter()
        .find(|f| f.text == "x")
        .expect("fragment for 'x' present");
    assert_eq!(frag.mcid, Some(0));
    assert_eq!(frag.struct_tag.as_deref(), Some("P"));
}

#[test]
fn overlaid_baselines_distinct_lines_when_mcid_differs() {
    // Two BDC blocks at the same Y (700 pt) but different MCIDs.
    // After Phase 1 + Task 11 grouping fix: two distinct lines.
    // (This test is expected to still fail after Task 8; Task 11 makes it pass.)
    let content = b"BT\n/F1 12 Tf\n\
                    /P << /MCID 0 >> BDC\n\
                    100 700 Td (Hello) Tj\n\
                    EMC\n\
                    /P << /MCID 1 >> BDC\n\
                    1 0 0 1 200 700 Tm (World) Tj\n\
                    EMC\n\
                    ET\n";
    let opts = ExtractionOptions {
        preserve_layout: true,
        reconstruct_paragraphs: true,
        ..ExtractionOptions::default()
    };
    let frags = extract_fragments(content, opts);

    let texts: Vec<String> = frags.iter().map(|f| f.text.clone()).collect();
    assert!(
        texts.iter().any(|t| t == "Hello"),
        "MCID 0 fragment 'Hello' must survive as its own group; got {:?}",
        texts
    );
    assert!(
        texts.iter().any(|t| t == "World"),
        "MCID 1 fragment 'World' must survive as its own group; got {:?}",
        texts
    );
    assert!(
        !texts.iter().any(|t| t.contains("HW") || t.contains("ld o")),
        "fragments must not be merged across mcid boundaries; got {:?}",
        texts
    );
}
