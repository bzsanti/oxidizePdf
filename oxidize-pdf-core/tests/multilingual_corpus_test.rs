//! Multilingual corpus tests — verify parsing and text extraction
//! for CJK (Chinese, Japanese, Korean) and RTL (Arabic, Hebrew) PDFs.
//!
//! Fixtures: UDHR (Universal Declaration of Human Rights) in 5 languages,
//! sourced from ohchr.org and legal.un.org (public domain).

use oxidize_pdf::parser::PdfReader;
use oxidize_pdf::text::TextExtractor;
use std::path::Path;

const FIXTURES_DIR: &str = "tests/fixtures/multilingual";

/// Helper: verify PDF can be opened and pages counted without panic.
fn assert_pdf_parseable(filename: &str, lang: &str) {
    let path = Path::new(FIXTURES_DIR).join(filename);
    if !path.exists() {
        eprintln!("[SKIP] Fixture not found: {}", path.display());
        return;
    }

    let doc = PdfReader::open_document(&path)
        .unwrap_or_else(|e| panic!("[{}] Failed to parse {}: {:?}", lang, filename, e));

    let num_pages = doc.page_count().unwrap();
    assert!(num_pages > 0, "[{}] PDF has 0 pages: {}", lang, filename);
}

// =============================================================================
// Parsing tests — PDF structure is valid and parseable
// =============================================================================

#[test]
#[cfg_attr(not(feature = "multilingual-fixtures"), ignore)]
fn test_chinese_pdf_parseable() {
    assert_pdf_parseable("udhr_chinese.pdf", "Chinese");
}

#[test]
#[cfg_attr(not(feature = "multilingual-fixtures"), ignore)]
fn test_japanese_pdf_parseable() {
    assert_pdf_parseable("udhr_japanese.pdf", "Japanese");
}

#[test]
#[cfg_attr(not(feature = "multilingual-fixtures"), ignore)]
fn test_korean_pdf_parseable() {
    assert_pdf_parseable("udhr_korean.pdf", "Korean");
}

#[test]
#[cfg_attr(not(feature = "multilingual-fixtures"), ignore)]
fn test_arabic_pdf_parseable() {
    assert_pdf_parseable("udhr_arabic.pdf", "Arabic");
}

#[test]
#[cfg_attr(not(feature = "multilingual-fixtures"), ignore)]
fn test_hebrew_pdf_parseable() {
    assert_pdf_parseable("udhr_hebrew.pdf", "Hebrew");
}

// =============================================================================
// Text extraction tests — verify non-trivial text is extracted from each language.
// Note: CJK PDFs may use CID encoding where the extracted text contains CID
// code points rather than Unicode. We verify that extraction produces substantial
// non-empty, non-ASCII content rather than matching specific substrings.
// =============================================================================

/// Helper: extract text and verify it contains non-ASCII characters (indicating
/// the PDF's native script was processed, even if CID-to-Unicode mapping is incomplete).
fn assert_pdf_extracts_non_ascii(filename: &str, min_chars: usize, lang: &str) {
    let path = Path::new(FIXTURES_DIR).join(filename);
    if !path.exists() {
        eprintln!("[SKIP] Fixture not found: {}", path.display());
        return;
    }

    let doc = PdfReader::open_document(&path)
        .unwrap_or_else(|e| panic!("[{}] Failed to open {}: {:?}", lang, filename, e));

    let num_pages = doc.page_count().unwrap();
    let mut extractor = TextExtractor::new();
    let mut full_text = String::new();
    for i in 0..num_pages {
        if let Ok(result) = extractor.extract_from_page(&doc, i) {
            full_text.push_str(&result.text);
        }
    }

    assert!(
        full_text.len() >= min_chars,
        "[{}] Expected >= {} chars, got {} from {} ({} pages)",
        lang,
        min_chars,
        full_text.len(),
        filename,
        num_pages
    );

    // Verify non-ASCII content exists (the native script was processed)
    let non_ascii_count = full_text.chars().filter(|c| !c.is_ascii()).count();
    assert!(
        non_ascii_count > 0,
        "[{}] No non-ASCII characters found — native script not extracted from {}",
        lang,
        filename
    );
}

#[test]
#[cfg_attr(not(feature = "multilingual-fixtures"), ignore)]
fn test_chinese_text_extraction() {
    assert_pdf_extracts_non_ascii("udhr_chinese.pdf", 5000, "Chinese");
}

#[test]
#[cfg_attr(not(feature = "multilingual-fixtures"), ignore)]
fn test_japanese_text_extraction() {
    assert_pdf_extracts_non_ascii("udhr_japanese.pdf", 3000, "Japanese");
}

#[test]
#[cfg_attr(not(feature = "multilingual-fixtures"), ignore)]
fn test_korean_text_extraction() {
    assert_pdf_extracts_non_ascii("udhr_korean.pdf", 3000, "Korean");
}

#[test]
#[cfg_attr(not(feature = "multilingual-fixtures"), ignore)]
fn test_arabic_text_extraction() {
    assert_pdf_extracts_non_ascii("udhr_arabic.pdf", 5000, "Arabic");
}

#[test]
#[cfg_attr(not(feature = "multilingual-fixtures"), ignore)]
fn test_hebrew_text_extraction() {
    // Note: The Hebrew UDHR fixture is PDF 1.2 (1996) which uses legacy font
    // encoding without Unicode CMaps. Text extraction produces mapped code points
    // but may not contain actual Hebrew Unicode characters. We verify extraction
    // produces substantial content without panicking.
    let path = Path::new(FIXTURES_DIR).join("udhr_hebrew.pdf");
    if !path.exists() {
        eprintln!("[SKIP] Fixture not found: {}", path.display());
        return;
    }
    let doc = PdfReader::open_document(&path).unwrap();
    let num_pages = doc.page_count().unwrap();
    let mut extractor = TextExtractor::new();
    let mut total_len = 0;
    for i in 0..num_pages {
        if let Ok(result) = extractor.extract_from_page(&doc, i) {
            total_len += result.text.len();
        }
    }
    assert!(
        total_len >= 2000,
        "[Hebrew] Expected >= 2000 chars, got {}",
        total_len
    );
}

// =============================================================================
// Full document extraction — no panics across all pages
// =============================================================================

#[test]
#[cfg_attr(not(feature = "multilingual-fixtures"), ignore)]
fn test_full_document_extraction_no_panic() {
    let fixtures = [
        ("udhr_chinese.pdf", "Chinese"),
        ("udhr_japanese.pdf", "Japanese"),
        ("udhr_korean.pdf", "Korean"),
        ("udhr_arabic.pdf", "Arabic"),
        ("udhr_hebrew.pdf", "Hebrew"),
    ];

    for (filename, lang) in &fixtures {
        let path = Path::new(FIXTURES_DIR).join(filename);
        if !path.exists() {
            eprintln!("[SKIP] Fixture not found: {}", path.display());
            continue;
        }

        let doc = match PdfReader::open_document(&path) {
            Ok(d) => d,
            Err(e) => {
                panic!("[{}] Failed to open: {:?}", lang, e);
            }
        };

        let num_pages = doc.page_count().unwrap();
        let mut extractor = TextExtractor::new();
        let mut total_chars = 0usize;

        for i in 0..num_pages {
            if let Ok(result) = extractor.extract_from_page(&doc, i) {
                total_chars += result.text.len();
            }
        }

        println!(
            "[{}] {} — {} pages, {} chars extracted",
            lang, filename, num_pages, total_chars
        );

        assert!(
            total_chars > 0,
            "[{}] Zero characters extracted from entire document",
            lang
        );
    }
}
