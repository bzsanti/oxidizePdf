//! Issue #269 Phase 1 — defensive paths for unbalanced marked-content
//! operators. Real PDFs (especially those produced by buggy generators or
//! after incremental updates) sometimes emit dangling EMC or unmatched
//! BDC. The extractor must not panic.

#[path = "common/mod.rs"]
mod common;
use common::synthetic_pdf::build_pdf_with_content_stream;

use oxidize_pdf::parser::{PdfDocument, PdfReader};
use oxidize_pdf::text::{ExtractionOptions, TextExtractor};
use std::io::Cursor;

fn extract(content: &[u8]) -> Vec<oxidize_pdf::text::TextFragment> {
    let pdf = build_pdf_with_content_stream(content);
    let reader = PdfReader::new(Cursor::new(pdf)).expect("reader");
    let document = PdfDocument::new(reader);
    let opts = ExtractionOptions {
        preserve_layout: true,
        ..Default::default()
    };
    let mut extractor = TextExtractor::with_options(opts);
    extractor
        .extract_from_page(&document, 0)
        .expect("extract")
        .fragments
}

#[test]
fn extra_emc_does_not_panic_and_text_still_extracts() {
    // Three EMCs but only one BDC. Extractor must extract the text and
    // silently drop the extra EMCs.
    let content = b"BT\n/F1 12 Tf\n100 700 Td\n\
                    EMC\n\
                    EMC\n\
                    /P << /MCID 0 >> BDC\n\
                    (hello) Tj\n\
                    EMC\n\
                    EMC\n\
                    ET\n";
    let frags = extract(content);
    let texts: Vec<&str> = frags.iter().map(|f| f.text.as_str()).collect();
    assert!(
        texts.contains(&"hello"),
        "text must survive extra EMC; got {:?}",
        texts
    );
}

#[test]
fn dangling_bdc_at_eof_does_not_panic_and_text_still_extracts() {
    // BDC with no EMC. Stack is non-empty at end of stream — must not
    // panic, must flush content (even if mcid attribution is degraded).
    let content = b"BT\n/F1 12 Tf\n100 700 Td\n\
                    /P << /MCID 0 >> BDC\n\
                    (hello) Tj\n\
                    ET\n";
    let frags = extract(content);
    let texts: Vec<&str> = frags.iter().map(|f| f.text.as_str()).collect();
    assert!(
        texts.contains(&"hello"),
        "text under dangling BDC must still extract; got {:?}",
        texts
    );
    // The single fragment must carry mcid=0 (the BDC was opened).
    let f = frags.iter().find(|f| f.text == "hello").unwrap();
    assert_eq!(f.mcid, Some(0));
}
