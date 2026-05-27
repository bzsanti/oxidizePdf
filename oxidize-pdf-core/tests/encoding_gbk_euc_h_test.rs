//! Issue #272 follow-up (Task 7): vendored GBK-EUC-H predefined CMap.
//!
//! issue2128r.pdf (pdfjs corpus) is a GBK-EUC-H / Adobe-GB1 PDF containing
//! Chinese text (浅谈校长的魅力, a school name, and an author name). Before the
//! fix, GBK codes were treated as CIDs via the Identity fallback, decoded to
//! garbage (0 CJK ideographs). After: `resolve_predefined("GBK-EUC-H")` returns
//! the vendored CMap, codes are mapped to real GB1 CIDs, and the CID-to-Unicode
//! table produces readable Chinese text.

use oxidize_pdf::parser::{PdfDocument, PdfReader};
use oxidize_pdf::text::{ExtractionOptions, TextExtractor};
use std::path::PathBuf;

const FIXTURE: &str = "tests/fixtures/issue_272_gbk_euc_h.pdf";

fn extract_text() -> String {
    let pdf_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(FIXTURE);
    let reader = PdfReader::open(&pdf_path).expect("GBK-EUC-H fixture must be readable");
    let document = PdfDocument::new(reader);
    let mut extractor = TextExtractor::with_options(ExtractionOptions::default());
    let page_count = document.page_count().unwrap_or(1).min(3);
    let mut text = String::new();
    for p in 0..page_count {
        if let Ok(result) = extractor.extract_from_page(&document, p) {
            text.push_str(&result.text);
        }
    }
    text
}

/// A real GBK-EUC-H PDF (Adobe-GB1). Before the fix, GBK codes were treated
/// as CIDs (Identity) and decoded to garbage (0 CJK). After: real CJK
/// ideographs extracted via the vendored predefined CMap.
#[test]
fn gbk_euc_h_extracts_real_cjk() {
    let text = extract_text();
    let cjk = text
        .chars()
        .filter(|&c| ('\u{4E00}'..='\u{9FFF}').contains(&c))
        .count();
    assert!(
        cjk >= 5,
        "expected real CJK ideographs, got: {:?}",
        text.chars().take(80).collect::<String>()
    );
}
