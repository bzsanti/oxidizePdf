//! Issue #649: TJ negative kerning (adjustment > 0.2 em) injects spurious spaces
//! when glyph advance widths are encoded as kerning.
//!
//! Some PDF producers emit text where character advance widths in the font are 0,
//! and horizontal advance between characters is encoded via uniform negative kern numbers
//! in `TJ` (e.g. `[ (T) -1000 (e) -1000 (s) -1000 (t) ] TJ`).
//! Because `-(-1000) / 1000.0 * font_size = 1.0 * font_size > 0.2 * font_size`,
//! a space would be inserted between every single character: `T e s t`.
//! When explicit spaces (`\x20`) are present in the sequence, the negative kern before
//! them causes double spaces: `( 1 1 )   3 0 3 0 - 7 1 7 7`.

#[path = "common/mod.rs"]
mod common;
use std::io::Cursor;

use common::synthetic_pdf::build_pdf_with_content_stream;
use oxidize_pdf::parser::{PdfDocument, PdfReader};
use oxidize_pdf::text::{ExtractionOptions, TextExtractor};

fn extract_text(content: &[u8]) -> String {
    let pdf = build_pdf_with_content_stream(content);
    let reader = PdfReader::new(Cursor::new(pdf)).expect("synthetic PDF must parse");
    let document = PdfDocument::new(reader);
    let mut extractor = TextExtractor::with_options(ExtractionOptions::default());
    extractor
        .extract_from_page(&document, 0)
        .expect("extract page 0")
        .text
}

/// Test 1: Negative kerns as glyph advances with explicit spaces do not get spaces between characters.
/// e.g. "123 45" encoded as single-glyph strings separated by -1000 kerns and an explicit space.
#[test]
fn tj_zero_width_kerning_with_explicit_spaces_does_not_split_characters() {
    // Array with tracking kerns (-1000) between characters and an explicit space:
    // [ (1) -1000 (2) -1000 (3) -1000 ( ) -1000 (4) -1000 (5) ] TJ
    let content =
        b"BT\n/F1 12 Tf\n100 700 Td\n[(1)-1000(2)-1000(3)-1000( )-1000(4)-1000(5)] TJ\nET\n";
    let text = extract_zero_width_text(content);
    assert_eq!(text.trim(), "123 45");
    assert!(
        !text.contains("1 2 3"),
        "digits must not be separated by spaces; got {:?}",
        text
    );
    assert!(
        !text.contains("4 5"),
        "digits 4 5 must not be separated by spaces; got {:?}",
        text
    );
}

/// Test 2: Telephone number pattern with parentheses, spaces, and hyphens.
/// Verifies issue #649 description: `( 1 1 )   3 0 3 0 - 7 1 7 7` must be `(11) 3030-7177`.
#[test]
fn tj_zero_width_kerning_phone_number_formatting() {
    // [(()-1000(1)-1000(1)-1000())-1000( )-1000(3)-1000(0)-1000(3)-1000(0)-1000(-)-1000(7)-1000(1)-1000(7)-1000(7)] TJ
    let content = b"BT\n/F1 10 Tf\n100 700 Td\n[(\\()-1000(1)-1000(1)-1000(\\))-1000( )-1000(3)-1000(0)-1000(3)-1000(0)-1000(-)-1000(7)-1000(1)-1000(7)-1000(7)] TJ\nET\n";
    let text = extract_zero_width_text(content);
    assert_eq!(text.trim(), "(11) 3030-7177");
}

/// Test 3: Normal text with word breaks as negative kerns without explicit spaces
/// continues to split words properly (`Hello World`).
/// Condition 1 ensures tracking detection does NOT activate when there are no explicit spaces.
#[test]
fn tj_word_breaks_as_negative_kerns_without_explicit_spaces_still_split_words() {
    let content = b"BT\n/F1 12 Tf\n100 700 Td\n[(Hello)-300(World)] TJ\nET\n";
    let text = extract_text(content);
    assert!(
        text.contains("Hello World"),
        "negative kern without explicit space must still act as word break; got {:?}",
        text
    );
}

/// Test 4: Ordinary intra-word kerning adjustments (-20, +15, -50) remain unaffected.
#[test]
fn tj_ordinary_intra_word_kerning_unaffected() {
    let content = b"BT\n/F1 12 Tf\n100 700 Td\n[(A)15(W)-20(A)-50(Y)] TJ\nET\n";
    let text = extract_text(content);
    assert!(
        text.contains("AWAY"),
        "ordinary intra-word kerns must collapse to 'AWAY'; got {:?}",
        text
    );
}

/// Test 5: Punctuation and symbols in uniform tracking sequences are handled cleanly.
#[test]
fn tj_punctuation_and_symbols_in_tracking_sequence() {
    // "Hello, World!" with uniform tracking (-600) and explicit space:
    let content = b"BT\n/F1 12 Tf\n100 700 Td\n[(H)-600(e)-600(l)-600(l)-600(o)-600(,)-600( )-600(W)-600(o)-600(r)-600(l)-600(d)-600(!)] TJ\nET\n";
    let text = extract_zero_width_text(content);
    assert_eq!(text.trim(), "Hello, World!");
    assert!(
        !text.contains("H e l l o"),
        "characters must not be split by spaces; got {:?}",
        text
    );
}

fn font_pdf(content: &[u8], font: String, extra: Vec<String>) -> Vec<u8> {
    let mut objects = vec![
        "<< /Type /Catalog /Pages 2 0 R >>".to_owned(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Resources << /Font << /F1 4 0 R >> >> /Contents 5 0 R >>".to_owned(),
        font,
        format!("<< /Length {} >>\nstream\n{}\nendstream", content.len(), std::str::from_utf8(content).unwrap()),
    ];
    objects.extend(extra);
    let mut bytes = b"%PDF-1.4\n".to_vec();
    let mut offsets = Vec::new();
    for (i, obj) in objects.iter().enumerate() {
        offsets.push(bytes.len());
        bytes.extend_from_slice(format!("{} 0 obj\n{obj}\nendobj\n", i + 1).as_bytes());
    }
    let xref = bytes.len();
    bytes.extend_from_slice(
        format!("xref\n0 {}\n0000000000 65535 f \n", objects.len() + 1).as_bytes(),
    );
    for offset in offsets {
        bytes.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    bytes.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n",
            objects.len() + 1
        )
        .as_bytes(),
    );
    bytes
}

fn extract_pdf(pdf: Vec<u8>) -> String {
    let doc = PdfReader::new(Cursor::new(pdf)).unwrap().into_document();
    TextExtractor::with_options(ExtractionOptions::default())
        .extract_from_page(&doc, 0)
        .unwrap()
        .text
}

fn extract_zero_width_text(content: &[u8]) -> String {
    let font = format!("<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /FirstChar 0 /LastChar 255 /Widths [{}] >>", "0 ".repeat(256));
    extract_pdf(font_pdf(content, font, vec![]))
}

#[test]
fn ordinary_word_gaps_are_not_tracking() {
    assert_eq!(
        extract_text(b"BT /F1 12 Tf 100 700 Td [(I)-300(a)-300(I)-300( )] TJ ET").trim(),
        "I a I"
    );
}

#[test]
fn zero_width_single_word_without_literal_space() {
    assert_eq!(
        extract_zero_width_text(b"BT /F1 12 Tf 100 700 Td [(T)-600(e)-600(s)-600(t)] TJ ET").trim(),
        "Test"
    );
}

#[test]
fn zero_width_tracking_preserves_larger_word_gap() {
    assert_eq!(
        extract_zero_width_text(
            b"BT /F1 12 Tf 100 700 Td [(A)-600(B)-1000(C)-600(D)-600( )-600(E)] TJ ET"
        )
        .trim(),
        "AB CD E"
    );
}

fn stream(body: &str) -> String {
    format!("<< /Length {} >>\nstream\n{body}\nendstream", body.len())
}

#[test]
fn type0_identity_h_zero_widths_without_literal_space() {
    let unicode = stream("begincmap\n1 begincodespacerange\n<0000> <FFFF>\nendcodespacerange\n4 beginbfchar\n<0054> <0054>\n<0065> <0065>\n<0073> <0073>\n<0074> <0074>\nendbfchar\nendcmap");
    let font = "<< /Type /Font /Subtype /Type0 /BaseFont /ReviewFont /Encoding /Identity-H /DescendantFonts [6 0 R] /ToUnicode 7 0 R >>".into();
    let cid = "<< /Type /Font /Subtype /CIDFontType2 /BaseFont /ReviewFont /CIDSystemInfo << /Registry (Adobe) /Ordering (Identity) /Supplement 0 >> /DW 500 /W [1 255 0] >>".into();
    assert_eq!(
        extract_pdf(font_pdf(
            b"BT /F1 12 Tf 100 700 Td [<0054>-600<0065>-600<0073>-600<0074>] TJ ET",
            font,
            vec![cid, unicode]
        ))
        .trim(),
        "Test"
    );
}

#[test]
fn type0_tracking_uses_cids_not_unicode_or_source_codes() {
    let unicode = stream("begincmap\n1 begincodespacerange\n<0000> <FFFF>\nendcodespacerange\n4 beginbfchar\n<0001> <0054>\n<0002> <0065>\n<0003> <0073>\n<0004> <0074>\nendbfchar\nendcmap");
    let encoding = stream("begincmap\n/WMode 0 def\n1 begincodespacerange\n<0000> <FFFF>\nendcodespacerange\n1 begincidrange\n<0001> <0004> 201\nendcidrange\nendcmap");
    let font = "<< /Type /Font /Subtype /Type0 /BaseFont /ReviewFont /Encoding 8 0 R /DescendantFonts [6 0 R] /ToUnicode 7 0 R >>".into();
    let cid = "<< /Type /Font /Subtype /CIDFontType2 /BaseFont /ReviewFont /CIDSystemInfo << /Registry (Adobe) /Ordering (Identity) /Supplement 0 >> /DW 500 /W [201 204 0] >>".into();
    assert_eq!(
        extract_pdf(font_pdf(
            b"BT /F1 12 Tf 100 700 Td [<0001>-600<0002>-600<0003>-600<0004>] TJ ET",
            font,
            vec![cid, unicode, encoding]
        ))
        .trim(),
        "Test"
    );
}

#[test]
fn char_spacing_does_not_get_discounted_as_missing_advance() {
    assert_eq!(
        extract_zero_width_text(b"BT /F1 12 Tf 2 Tc 100 700 Td [(I)-300(a)-300(I)-300( )] TJ ET")
            .trim(),
        "I a I"
    );
}

#[test]
fn nonuniform_zero_width_advances_remain_conservative() {
    assert_eq!(
        extract_zero_width_text(b"BT /F1 12 Tf 100 700 Td [(A)-300(B)-600(C)-900(D)] TJ ET").trim(),
        "A B C D"
    );
}

#[test]
fn one_transition_cannot_establish_tracking() {
    assert_eq!(
        extract_zero_width_text(b"BT /F1 12 Tf 100 700 Td [(I)-600(a)] TJ ET").trim(),
        "I a"
    );
}

#[test]
fn positive_declared_widths_do_not_activate_tracking() {
    let font = format!("<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /FirstChar 0 /LastChar 255 /Widths [{}] >>", "500 ".repeat(256));
    assert_eq!(
        extract_pdf(font_pdf(
            b"BT /F1 12 Tf 100 700 Td [(I)-300(a)-300(I)-300( )] TJ ET",
            font,
            vec![]
        ))
        .trim(),
        "I a I"
    );
}

#[test]
fn leading_and_trailing_kerns_do_not_establish_tracking() {
    assert_eq!(
        extract_zero_width_text(b"BT /F1 12 Tf 100 700 Td [-600(A)-600(B)-600(C)-600] TJ ET")
            .trim(),
        "A B C"
    );
}

#[test]
fn word_spacing_does_not_get_discounted_as_missing_advance() {
    assert_eq!(
        extract_zero_width_text(b"BT /F1 12 Tf 2 Tw 100 700 Td [(I)-300(a)-300(I)-300( )] TJ ET")
            .trim(),
        "I a I"
    );
}
