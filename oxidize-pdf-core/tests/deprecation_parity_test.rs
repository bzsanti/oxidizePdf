#![allow(deprecated)]

use oxidize_pdf::parser::PdfDocument;

fn fixture(name: &str) -> String {
    format!("{}/tests/fixtures/{}", env!("CARGO_MANIFEST_DIR"), name)
}

#[test]
fn test_to_markdown_parity() {
    let doc = PdfDocument::open(fixture("Cold_Email_Hacks.pdf")).unwrap();
    let old = oxidize_pdf::ai::export_to_markdown(&doc).unwrap();
    let new = doc.to_markdown().unwrap();
    assert_eq!(old, new);
}

#[test]
fn test_to_contextual_parity() {
    let doc = PdfDocument::open(fixture("Cold_Email_Hacks.pdf")).unwrap();
    let old = oxidize_pdf::ai::export_to_contextual(&doc).unwrap();
    let new = doc.to_contextual().unwrap();
    assert_eq!(old, new);
}

#[cfg(feature = "semantic")]
#[test]
fn test_to_json_parity() {
    let doc = PdfDocument::open(fixture("Cold_Email_Hacks.pdf")).unwrap();
    let old = oxidize_pdf::ai::export_to_json(&doc).unwrap();
    let new = doc.to_json().unwrap();
    assert_eq!(old, new);
}
