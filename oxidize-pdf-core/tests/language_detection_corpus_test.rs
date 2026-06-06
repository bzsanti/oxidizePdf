//! End-to-end language detection over the CJK multilingual corpus (#293).
//! Verifies that text extracted from real PDFs is detected at the document
//! level. RTL fixtures (Arabic/Hebrew) are excluded: they do not currently
//! extract to native-script Unicode (separate extraction gap), so RTL detection
//! is covered by synthetic strings in language_detection_test.rs.
#![cfg(all(feature = "language-detection", feature = "multilingual-fixtures"))]

use oxidize_pdf::ai::DocumentChunker;
use oxidize_pdf::parser::PdfReader;
use oxidize_pdf::text::TextExtractor;
use std::path::Path;

fn extract_pages(filename: &str) -> Vec<(usize, String)> {
    let path = Path::new("tests/fixtures/multilingual").join(filename);
    let doc = PdfReader::open_document(&path).unwrap_or_else(|e| panic!("open {filename}: {e:?}"));
    let n = doc.page_count().unwrap();
    let mut ex = TextExtractor::new();
    let mut pages = Vec::new();
    for i in 0..n {
        if let Ok(r) = ex.extract_from_page(&doc, i) {
            pages.push(((i + 1) as usize, r.text));
        }
    }
    pages
}

fn detect_corpus_language(filename: &str) -> String {
    let pages = extract_pages(filename);
    let chunker = DocumentChunker::new(512, 50).with_language_detection(true);
    let chunks = chunker.chunk_text_with_pages(&pages).unwrap();
    let lang = DocumentChunker::document_language(&chunks)
        .unwrap_or_else(|| panic!("no language detected for {filename}"));
    assert!(
        lang.reliable,
        "{filename}: detection should be reliable, got {lang:?}"
    );
    lang.code
}

#[test]
fn detects_chinese_corpus() {
    assert_eq!(detect_corpus_language("udhr_chinese.pdf"), "cmn");
}

#[test]
fn detects_japanese_corpus() {
    assert_eq!(detect_corpus_language("udhr_japanese.pdf"), "jpn");
}

#[test]
fn detects_korean_corpus() {
    assert_eq!(detect_corpus_language("udhr_korean.pdf"), "kor");
}
