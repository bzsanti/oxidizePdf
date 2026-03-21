//! Integration tests for CID-keyed font text extraction (Issue #157)
//!
//! Tests that PDFs with CID-keyed fonts and no ToUnicode CMap
//! correctly extract text using Adobe CID→Unicode mapping tables.

use oxidize_pdf::parser::{PdfDocument, PdfReader};
use oxidize_pdf::text::cid_to_unicode::CidCollection;

// ─── Unit tests for CidCollection ───

#[test]
fn test_cid_collection_from_ordering() {
    assert_eq!(
        CidCollection::from_ordering("CNS1"),
        Some(CidCollection::Cns1)
    );
    assert_eq!(
        CidCollection::from_ordering("GB1"),
        Some(CidCollection::Gb1)
    );
    assert_eq!(
        CidCollection::from_ordering("Japan1"),
        Some(CidCollection::Japan1)
    );
    assert_eq!(
        CidCollection::from_ordering("Korea1"),
        Some(CidCollection::Korea1)
    );
    assert_eq!(
        CidCollection::from_ordering("KR"),
        Some(CidCollection::Korea1)
    );
    assert_eq!(CidCollection::from_ordering("Unknown"), None);
    assert_eq!(CidCollection::from_ordering(""), None);
}

#[test]
fn test_cns1_known_cid_mappings() {
    let cns1 = CidCollection::Cns1;

    // Verified mappings from the issue #157 PDF:
    // CID 0x0830 → 香 (U+9999)
    assert_eq!(cns1.cid_to_unicode(0x0830), Some('香'));
    // CID 0x0CA5 → 港 (U+6E2F)
    assert_eq!(cns1.cid_to_unicode(0x0CA5), Some('港'));
    // CID 0x0374 → 交 (U+4EA4)
    assert_eq!(cns1.cid_to_unicode(0x0374), Some('交'));
    // CID 0x05F8 → 易 (U+6613)
    assert_eq!(cns1.cid_to_unicode(0x05F8), Some('易'));
    // CID 0x02BF → 及 (U+53CA)
    assert_eq!(cns1.cid_to_unicode(0x02BF), Some('及'));
    // CID 0x0D10 → 結 (U+7D50)
    assert_eq!(cns1.cid_to_unicode(0x0D10), Some('結'));
    // CID 0x02AE → 公 (U+516C)
    assert_eq!(cns1.cid_to_unicode(0x02AE), Some('公'));
    // CID 0x0321 → 司 (U+53F8)
    assert_eq!(cns1.cid_to_unicode(0x0321), Some('司'));
}

#[test]
fn test_cns1_basic_latin_cids() {
    let cns1 = CidCollection::Cns1;
    // CID 1 → space (U+0020) — typical for CID 1
    // CID 0 is .notdef — should return None or a notdef char
    assert_eq!(cns1.cid_to_unicode(0), None);
}

#[test]
fn test_gb1_exists_and_has_entries() {
    let gb1 = CidCollection::Gb1;
    // CID 2 in GB1 should map to a character (basic Latin '!')
    assert!(gb1.cid_to_unicode(2).is_some());
}

#[test]
fn test_japan1_exists_and_has_entries() {
    let japan1 = CidCollection::Japan1;
    assert!(japan1.cid_to_unicode(2).is_some());
}

#[test]
fn test_korea1_exists_and_has_entries() {
    let korea1 = CidCollection::Korea1;
    assert!(korea1.cid_to_unicode(2).is_some());
}

#[test]
fn test_unknown_cid_returns_none() {
    let cns1 = CidCollection::Cns1;
    // Very high CID unlikely to be mapped
    assert_eq!(cns1.cid_to_unicode(u16::MAX), None);
}

// ─── Integration test with actual PDF ───

#[test]
fn test_issue_157_cid_keyed_font_text_extraction() {
    let pdf_path = "examples/results/issue_157_cid_cmap.pdf";
    let reader = PdfReader::open(pdf_path).expect("Failed to open issue 157 PDF");
    let document = PdfDocument::new(reader);

    assert_eq!(document.page_count().unwrap(), 1);

    let text_pages = document.extract_text().expect("Failed to extract text");
    assert_eq!(text_pages.len(), 1);

    let text = &text_pages[0].text;

    // Verify key Traditional Chinese phrases from the document
    assert!(
        text.contains("香港交易及結算所有限公司"),
        "Should contain '香港交易及結算所有限公司', got: {}",
        &text[..text.len().min(200)]
    );
    assert!(
        text.contains("綠色動"),
        "Should contain '綠色動' (Dynagreen)"
    );
    assert!(
        text.contains("Dynagreen Environmental Protection Group"),
        "Should contain English company name"
    );
    assert!(text.contains("1330"), "Should contain stock code 1330");
    assert!(
        text.contains("董事會召開日期"),
        "Should contain '董事會召開日期' (board meeting date)"
    );
    assert!(
        text.contains("成蘇寧"),
        "Should contain chairman name '成蘇寧'"
    );

    // Verify NO garbage characters (the original bug symptom)
    assert!(
        !text.contains("«þÏçY"),
        "Should not contain garbage characters from WinAnsi fallback"
    );
}
