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
    let text = extract_text(content);
    assert!(
        text.contains("123 45"),
        "expected '123 45' without spaces between digits; got {:?}",
        text
    );
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
    let text = extract_text(content);
    assert!(
        text.contains("(11) 3030-7177"),
        "expected '(11) 3030-7177'; got {:?}",
        text
    );
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
    let text = extract_text(content);
    assert!(
        text.contains("Hello, World!"),
        "expected 'Hello, World!'; got {:?}",
        text
    );
    assert!(
        !text.contains("H e l l o"),
        "characters must not be split by spaces; got {:?}",
        text
    );
}
