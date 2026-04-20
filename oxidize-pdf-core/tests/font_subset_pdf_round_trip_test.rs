//! End-to-end PDF round-trip tests for font subsetting.
//!
//! These tests generate a PDF with text using a real font fixture, parse the
//! PDF back, extract the text, and assert that what comes out matches what
//! went in. They exercise the full pipeline:
//!   - CFF or TTF subsetting (including desubroutinization, SID→CID, table
//!     stripping)
//!   - font embedding via the writer (CIDFontType0C for CFF, CIDFontType2
//!     for TTF, both wrapped in a Type0 font with Identity-H)
//!   - ToUnicode CMap generation
//!   - text reconstruction by the parser's text extraction
//!
//! Each test skips gracefully if its fixture is missing.

use oxidize_pdf::parser::{PdfDocument, PdfReader};
use oxidize_pdf::{Document, Font, Page};
use std::io::Cursor;

const SOURCE_SANS_PATH: &str = "../test-pdfs/SourceSans3-Regular.otf";
const ROBOTO_PATH: &str = "../test-pdfs/Roboto-Regular.ttf";

fn load_fixture(path: &str) -> Option<Vec<u8>> {
    std::fs::read(path)
        .map_err(|_| eprintln!("SKIPPED: {} not found", path))
        .ok()
}

/// Assert that every character in `expected` (ignoring whitespace) appears in
/// `extracted`. Text extraction may collapse or reorder whitespace; glyph
/// presence is what matters.
fn assert_all_chars_extracted(extracted: &str, expected: &str) {
    for ch in expected.chars() {
        if ch.is_whitespace() {
            continue;
        }
        assert!(
            extracted.contains(ch),
            "extracted text missing '{}' (U+{:04X}). Full extracted: {:?}",
            ch,
            ch as u32,
            extracted
        );
    }
}

/// Non-CID CFF (SID-keyed) end-to-end: after Task 7 the subsetter converts
/// these to CID-keyed raw CFF and the writer embeds them with
/// /CIDFontType0C. The PDF must round-trip plain ASCII + Latin accented
/// characters through that pipeline.
#[test]
fn test_non_cid_cff_pdf_round_trip_preserves_text() {
    let font_data = match load_fixture(SOURCE_SANS_PATH) {
        Some(d) => d,
        None => return,
    };

    // ASCII + Latin Extended (accented chars exercise 2-byte CID emission
    // under Identity-H; also validates that the ToUnicode CMap maps SMP-
    // adjacent codepoints back to the original char).
    let test_text = "café résumé naïve";

    let mut doc = Document::new();
    doc.add_font_from_bytes("SourceSans3", font_data)
        .expect("add_font_from_bytes should succeed");

    let mut page = Page::a4();
    page.text()
        .set_font(Font::Custom("SourceSans3".to_string()), 12.0)
        .at(50.0, 500.0)
        .write(test_text)
        .expect("writing accented Latin text should succeed");
    doc.add_page(page);

    let pdf_bytes = doc.to_bytes().expect("PDF generation should succeed");
    assert!(
        !pdf_bytes.is_empty(),
        "PDF generation produced empty output"
    );

    let reader =
        PdfReader::new(Cursor::new(&pdf_bytes)).expect("generated PDF must be re-parseable");
    let parsed = PdfDocument::new(reader);

    let extracted = parsed
        .extract_text_from_page(0)
        .expect("text extraction from generated PDF must succeed");

    assert_all_chars_extracted(&extracted.text, test_text);
}

/// TTF end-to-end: after Task 8 the subsetter strips cmap/OS/2/name from
/// the output font. The PDF writer embeds the result as /CIDFontType2
/// with CIDToGIDMap. Round-trip must preserve ASCII text even though the
/// embedded font has no cmap table to consult.
#[test]
fn test_ttf_pdf_round_trip_preserves_text() {
    let font_data = match load_fixture(ROBOTO_PATH) {
        Some(d) => d,
        None => return,
    };

    // Pangram: stresses a wide range of lowercase + uppercase glyphs with a
    // mix of common words. All characters are ASCII so the test isolates
    // the TTF subsetting / embedding path.
    let test_text = "The quick brown fox jumps over the lazy dog.";

    let mut doc = Document::new();
    doc.add_font_from_bytes("Roboto", font_data)
        .expect("add_font_from_bytes should succeed");

    let mut page = Page::a4();
    page.text()
        .set_font(Font::Custom("Roboto".to_string()), 12.0)
        .at(50.0, 500.0)
        .write(test_text)
        .expect("writing pangram should succeed");
    doc.add_page(page);

    let pdf_bytes = doc.to_bytes().expect("PDF generation should succeed");
    assert!(
        !pdf_bytes.is_empty(),
        "PDF generation produced empty output"
    );

    let reader =
        PdfReader::new(Cursor::new(&pdf_bytes)).expect("generated PDF must be re-parseable");
    let parsed = PdfDocument::new(reader);

    let extracted = parsed
        .extract_text_from_page(0)
        .expect("text extraction from generated PDF must succeed");

    assert_all_chars_extracted(&extracted.text, test_text);
}

/// Two fonts (CFF + TTF) in the same document on the same page. Exercises
/// the writer's font-dict collision handling and confirms that neither
/// subsetting path corrupts the other when both are used simultaneously.
#[test]
fn test_mixed_cff_and_ttf_pdf_round_trip() {
    let cff = match load_fixture(SOURCE_SANS_PATH) {
        Some(d) => d,
        None => return,
    };
    let ttf = match load_fixture(ROBOTO_PATH) {
        Some(d) => d,
        None => return,
    };

    let cff_text = "hello";
    let ttf_text = "world";

    let mut doc = Document::new();
    doc.add_font_from_bytes("SourceSans3", cff)
        .expect("add_font_from_bytes SourceSans3");
    doc.add_font_from_bytes("Roboto", ttf)
        .expect("add_font_from_bytes Roboto");

    let mut page = Page::a4();
    page.text()
        .set_font(Font::Custom("SourceSans3".to_string()), 12.0)
        .at(50.0, 500.0)
        .write(cff_text)
        .expect("writing CFF text");
    page.text()
        .set_font(Font::Custom("Roboto".to_string()), 12.0)
        .at(50.0, 460.0)
        .write(ttf_text)
        .expect("writing TTF text");
    doc.add_page(page);

    let pdf_bytes = doc.to_bytes().expect("PDF generation should succeed");
    let reader =
        PdfReader::new(Cursor::new(&pdf_bytes)).expect("generated PDF must be re-parseable");
    let parsed = PdfDocument::new(reader);

    let extracted = parsed
        .extract_text_from_page(0)
        .expect("text extraction must succeed");

    // Both strings must be present; their relative order depends on the
    // extractor's line-reading strategy.
    assert_all_chars_extracted(&extracted.text, cff_text);
    assert_all_chars_extracted(&extracted.text, ttf_text);
}

// =============================================================================
// FlateDecode compression tests for font-file streams (Cycles E and F)
//
// A stream-dictionary block is of the form `<< ... >> stream ... endstream`.
// We locate a dictionary containing a given `signature` key (e.g. `/Length1`
// for TTF FontFile2 or `/Subtype /CIDFontType0C` for OTF FontFile3) and
// verify the same dictionary declares `/Filter /FlateDecode`.
// =============================================================================

/// Return true if the PDF bytes contain at least one stream dictionary whose
/// body includes both `signature` and `/Filter /FlateDecode`.
fn dictionary_with_signature_has_flate_filter(pdf_bytes: &[u8], signature: &[u8]) -> bool {
    let mut cursor = 0;
    while let Some(sig_rel) = find_subslice(&pdf_bytes[cursor..], signature) {
        let sig_abs = cursor + sig_rel;
        // Back up to the nearest "<<" before the signature
        let dict_start = match rfind_subslice(&pdf_bytes[..sig_abs], b"<<") {
            Some(p) => p,
            None => {
                cursor = sig_abs + signature.len();
                continue;
            }
        };
        // Forward to the nearest ">>" after the signature
        let dict_end_rel = match find_subslice(&pdf_bytes[sig_abs..], b">>") {
            Some(p) => p,
            None => break,
        };
        let dict_end = sig_abs + dict_end_rel;
        let dict_body = &pdf_bytes[dict_start..dict_end];
        if find_subslice(dict_body, b"/Filter").is_some()
            && find_subslice(dict_body, b"/FlateDecode").is_some()
        {
            return true;
        }
        cursor = dict_end;
    }
    false
}

fn find_subslice(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() || needle.len() > haystack.len() {
        return None;
    }
    (0..=haystack.len() - needle.len()).find(|&i| &haystack[i..i + needle.len()] == needle)
}

fn rfind_subslice(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() || needle.len() > haystack.len() {
        return None;
    }
    (0..=haystack.len() - needle.len())
        .rev()
        .find(|&i| &haystack[i..i + needle.len()] == needle)
}

#[cfg(feature = "compression")]
#[test]
fn test_ttf_fontfile2_stream_has_flatedecode_filter() {
    let font_data = match load_fixture(ROBOTO_PATH) {
        Some(d) => d,
        None => return,
    };

    let mut doc = Document::new();
    doc.add_font_from_bytes("Roboto", font_data)
        .expect("add_font_from_bytes must succeed");
    let mut page = Page::a4();
    page.text()
        .set_font(Font::Custom("Roboto".to_string()), 12.0)
        .at(50.0, 500.0)
        .write("AB")
        .expect("writing ASCII text must succeed");
    doc.add_page(page);

    let pdf_bytes = doc.to_bytes().expect("PDF generation must succeed");

    // A TTF FontFile2 stream dictionary always carries /Length1.
    assert!(
        dictionary_with_signature_has_flate_filter(&pdf_bytes, b"/Length1"),
        "FontFile2 (TTF) stream dictionary must declare /Filter /FlateDecode"
    );

    // Sanity: a 2-char Roboto subset must fit comfortably under 100 KB.
    assert!(
        pdf_bytes.len() < 100_000,
        "PDF with 2-char Roboto subset must be under 100 KB, got {} bytes",
        pdf_bytes.len()
    );
}

#[cfg(feature = "compression")]
#[test]
fn test_otf_fontfile3_stream_has_flatedecode_filter() {
    let font_data = match load_fixture(SOURCE_SANS_PATH) {
        Some(d) => d,
        None => return,
    };

    let mut doc = Document::new();
    doc.add_font_from_bytes("SourceSans3", font_data)
        .expect("add_font_from_bytes must succeed");
    let mut page = Page::a4();
    page.text()
        .set_font(Font::Custom("SourceSans3".to_string()), 12.0)
        .at(50.0, 500.0)
        .write("AB")
        .expect("writing ASCII text must succeed");
    doc.add_page(page);

    let pdf_bytes = doc.to_bytes().expect("PDF generation must succeed");

    // An OTF FontFile3 stream dictionary carries /Subtype /CIDFontType0C.
    assert!(
        dictionary_with_signature_has_flate_filter(&pdf_bytes, b"/CIDFontType0C"),
        "FontFile3 (CIDFontType0C) stream dictionary must declare /Filter /FlateDecode"
    );

    assert!(
        pdf_bytes.len() < 50_000,
        "PDF with 2-char SourceSans3 subset must be under 50 KB, got {} bytes",
        pdf_bytes.len()
    );
}
