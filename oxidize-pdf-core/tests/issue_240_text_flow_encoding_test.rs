//! Regression tests for issue #240 — `TextFlowContext::write_wrapped` was
//! emitting raw UTF-8 bytes straight into `Op::ShowText`, bypassing the
//! `TextEncoding::WinAnsiEncoding` pipeline that `TextContext::write` uses
//! for builtin fonts. Result: `€`, `—`, smart quotes etc. surfaced in the
//! content stream as multi-byte UTF-8 runs that any viewer reinterpreted
//! as Windows-1252 — `€` → `â‚¬`, `—` → `â€"`, and so on.
//!
//! These tests are content-verifying (no smoke): they inspect the raw
//! content-stream bytes of the produced PDF and assert that each
//! special character lands at the WinAnsi octal escape `TextContext::write`
//! would emit (`\200` for `€`, `\227` for `—`, …) and that no raw UTF-8
//! multi-byte sequence appears.
//!
//! ISO 32000-1 references:
//! - §7.9.2 — string objects, literal escapes, `\NNN` octal form
//! - §9.4.3 — text-showing operators
//! - §9.10  — text encoding (WinAnsi as the default for builtin fonts)

use oxidize_pdf::parser::{PdfDocument, PdfReader};
use oxidize_pdf::text::{TextAlign, TextExtractor};
use oxidize_pdf::{Document, Font, Page};
use std::io::Cursor;

/// Extract the bytes of the FIRST literal-string show-text body in a
/// content stream — the bytes between `(` and the matching `) Tj`. Used
/// to compare what `TextContext::write` emits vs what
/// `TextFlowContext::write_wrapped` emits for the same input.
///
/// The parser honours PDF literal-string escape rules per ISO 32000-1
/// §7.9.2: a `)` preceded by a backslash does not close the literal,
/// and a `\\` consumes the backslash that follows it. Anything more
/// complex is irrelevant here — the show-text payloads we emit never
/// contain nested parentheses.
fn extract_first_show_text_literal(pdf: &[u8]) -> Vec<u8> {
    let mut i = 0;
    while i < pdf.len() {
        if pdf[i] == b'(' {
            // Walk forward, tracking backslash escapes, looking for the
            // matching `)` that immediately precedes ` Tj`.
            let mut j = i + 1;
            let mut body: Vec<u8> = Vec::new();
            while j < pdf.len() {
                match pdf[j] {
                    b'\\' if j + 1 < pdf.len() => {
                        body.push(pdf[j]);
                        body.push(pdf[j + 1]);
                        j += 2;
                    }
                    b')' if pdf[j..].starts_with(b") Tj") => {
                        return body;
                    }
                    b => {
                        body.push(b);
                        j += 1;
                    }
                }
            }
            // Unterminated literal — skip and continue scanning.
            i = j;
        } else {
            i += 1;
        }
    }
    panic!("no `(...) Tj` literal-string show-text found in content stream");
}

/// Render `text` through `text_flow().write_wrapped(...)` on a fresh page,
/// return the uncompressed PDF bytes. Compression is disabled so the
/// content stream bytes are directly observable.
fn render_via_text_flow(text: &str) -> Vec<u8> {
    let mut doc = Document::new();
    let mut page = Page::a4();
    page.set_margins(50.0, 50.0, 50.0, 50.0);
    {
        let mut flow = page.text_flow();
        flow.set_font(Font::Helvetica, 14.0);
        flow.at(50.0, 700.0);
        flow.set_alignment(TextAlign::Left);
        flow.write_wrapped(text)
            .expect("write_wrapped must succeed");
        page.add_text_flow(&flow);
    }
    doc.set_compress(false);
    doc.add_page(page);
    doc.to_bytes().expect("to_bytes must succeed")
}

/// `€` must be encoded as a single WinAnsi byte `0x80`, escaped in the
/// PDF literal-string form as `\200` (octal three-digit). The raw UTF-8
/// representation of `€` is the three-byte sequence `0xE2 0x82 0xAC`,
/// which would escape as `\342\202\254` — that sequence MUST NOT appear.
#[test]
fn write_wrapped_euro_sign_encodes_as_winansi_not_utf8() {
    let bytes = render_via_text_flow("€100");

    // WinAnsi-correct escape for the euro sign followed by ASCII "100".
    let winansi = b"\\200100";
    assert!(
        bytes.windows(winansi.len()).any(|w| w == winansi),
        "Expected WinAnsi euro escape `\\200100` in the content stream — \
         `€` must be encoded as 0x80 per Windows-1252, not as the raw \
         UTF-8 sequence."
    );

    // The raw UTF-8 escape sequence would look like `\342\202\254`. If
    // this appears, `write_wrapped` is dumping UTF-8 bytes directly and
    // the encoding pipeline is being bypassed.
    let utf8_raw = b"\\342\\202\\254";
    assert!(
        !bytes.windows(utf8_raw.len()).any(|w| w == utf8_raw),
        "Raw UTF-8 octal escape `\\342\\202\\254` appeared in content stream — \
         `write_wrapped` is bypassing TextEncoding::WinAnsiEncoding."
    );
}

/// Cover every special character the issue body called out: each must
/// land at its Windows-1252 byte (encoded as `\NNN` octal in the PDF
/// literal-string body). One assertion per character so a missing entry
/// in the WinAnsi table or a divergence in the helper is pinpointed by
/// the test name, not buried inside a multi-char compare.
///
/// `ñ` is a Latin-1 character (`0xF1`), the rest live in the
/// Windows-1252 supplement (`0x80..=0x9F`). The combined coverage proves
/// both branches of the WinAnsi mapping are exercised.
#[test]
fn write_wrapped_special_chars_encode_at_windows_1252_codepoints() {
    let cases: &[(char, &[u8], &str)] = &[
        ('€', b"\\200", "euro U+20AC -> 0x80"),
        ('—', b"\\227", "em dash U+2014 -> 0x97"),
        ('–', b"\\226", "en dash U+2013 -> 0x96"),
        ('\u{2018}', b"\\221", "left single quote U+2018 -> 0x91"),
        ('\u{2019}', b"\\222", "right single quote U+2019 -> 0x92"),
        ('…', b"\\205", "horizontal ellipsis U+2026 -> 0x85"),
        ('±', b"\\261", "plus-minus U+00B1 -> 0xB1"),
        ('ñ', b"\\361", "n with tilde U+00F1 -> 0xF1"),
    ];

    for (ch, expected_escape, why) in cases {
        let input: String = std::iter::once(*ch).collect();
        let bytes = render_via_text_flow(&input);
        assert!(
            bytes
                .windows(expected_escape.len())
                .any(|w| w == *expected_escape),
            "Expected `{}` (escape `{}`) in content stream for character {ch:?} ({why}).",
            std::str::from_utf8(expected_escape).unwrap(),
            std::str::from_utf8(expected_escape).unwrap(),
        );
    }
}

/// Render `text` via `page.text().write(...)` (no flow), return bytes.
/// Mirror of `render_via_text_flow` so we can compare the show-text
/// payloads byte-for-byte.
fn render_via_text_at(text: &str) -> Vec<u8> {
    let mut doc = Document::new();
    let mut page = Page::a4();
    page.set_margins(50.0, 50.0, 50.0, 50.0);
    page.text()
        .set_font(Font::Helvetica, 14.0)
        .at(50.0, 700.0)
        .write(text)
        .expect("text().write must succeed");
    doc.set_compress(false);
    doc.add_page(page);
    doc.to_bytes().expect("to_bytes must succeed")
}

/// Count the number of `Tj` show-text operators in a content stream —
/// each one corresponds to a line emitted by `write_wrapped`.
fn count_tj_operators(pdf: &[u8]) -> usize {
    pdf.windows(4).filter(|w| *w == b") Tj").count()
}

/// `write_wrapped` must wrap based on character widths, not on UTF-8
/// byte length. Two paragraphs with the same character count but
/// different UTF-8 byte length (one ASCII, one with a 2-byte Latin-1
/// supplement char per word) must wrap to nearly the same number of
/// lines — the character widths of `i` and `ï` are essentially equal in
/// Helvetica, so any wrap-count divergence implies the measurement
/// pipeline is summing bytes instead of glyph widths.
#[test]
fn write_wrapped_wraps_by_char_width_not_by_utf8_byte_length() {
    let n_words = 30;
    // "naïve" = 5 chars, 6 bytes UTF-8 (`ï` = 0xC3 0xAF in UTF-8).
    let unicode_line: String = std::iter::repeat("naïve")
        .take(n_words)
        .collect::<Vec<_>>()
        .join(" ");
    // "naive" = 5 chars, 5 bytes UTF-8.
    let ascii_line: String = std::iter::repeat("naive")
        .take(n_words)
        .collect::<Vec<_>>()
        .join(" ");

    let lines_unicode = count_tj_operators(&render_via_text_flow(&unicode_line));
    let lines_ascii = count_tj_operators(&render_via_text_flow(&ascii_line));

    assert!(
        lines_unicode.abs_diff(lines_ascii) <= 1,
        "wrap line count differs more than 1 between unicode={lines_unicode} \
         and ascii={lines_ascii} — same char count, different UTF-8 byte \
         length. measure_text_with is summing bytes instead of glyph widths."
    );
}

/// Paridad de emisión: `text()` y `text_flow()` deben producir
/// EXACTAMENTE los mismos bytes en el body de `Op::ShowText` para el
/// mismo `(text, font)`. Antes del fix de #240 divergían: la ruta de
/// flow emitía UTF-8 raw; la ruta de text emitía WinAnsi escapado.
#[test]
fn text_at_and_text_flow_at_emit_identical_show_text_bytes() {
    let inputs = [
        "ASCII only",
        "€100",
        "café — naïve",
        "Hello \u{2018}quoted\u{2019} world",
    ];

    for input in inputs {
        let via_text = render_via_text_at(input);
        let via_flow = render_via_text_flow(input);

        let body_text = extract_first_show_text_literal(&via_text);
        let body_flow = extract_first_show_text_literal(&via_flow);

        assert_eq!(
            body_flow, body_text,
            "show-text body diverges between text() and text_flow() for input {input:?}.\n\
             text()      bytes: {body_text:?}\n\
             text_flow() bytes: {body_flow:?}",
        );
    }
}

/// Integración E2E #240: round-trip a través del parser. Emitir `€ — ±`
/// vía `text_flow().write_wrapped(...)`, parsear el PDF con `PdfReader`
/// + `TextExtractor` y verificar que cada character especial sobrevive
/// la decodificación WinAnsi → UTF-8. Pre-fix, el extractor leía los
/// bytes UTF-8 raw como tres chars Windows-1252 separados produciendo
/// mojibake (`€` → `â‚¬`).
#[test]
fn issue_240_reproducer_round_trip_special_chars_via_extractor() {
    let input = "precio: €100 — oferta ±5%";
    let pdf = render_via_text_flow(input);

    let reader = PdfReader::new(Cursor::new(pdf)).expect("PdfReader must open in-memory PDF");
    let doc = PdfDocument::new(reader);
    let mut extractor = TextExtractor::new();
    let extracted = extractor
        .extract_from_document(&doc)
        .expect("extract_from_document must succeed");
    let all_text: String = extracted
        .iter()
        .map(|page_text| page_text.text.as_str())
        .collect::<Vec<_>>()
        .join("\n");

    for needle in ['€', '—', '±'] {
        assert!(
            all_text.contains(needle),
            "round-trip extracted text must contain `{needle}`; got: {all_text:?}",
        );
    }

    // Negative-space check: the UTF-8 mojibake the pre-fix produced
    // (`â‚¬` for €) must NOT appear — that would indicate the extractor
    // read raw UTF-8 bytes through the WinAnsi table.
    assert!(
        !all_text.contains("â‚¬"),
        "extracted text contains WinAnsi mojibake `â‚¬` — write_wrapped \
         is still emitting raw UTF-8. got: {all_text:?}",
    );
}
