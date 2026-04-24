//! Issue #212 (reframed) — `Document::fill_field` produces a corrupt `/AP/N`
//! content stream for **any** non-ASCII value because the default text
//! appearance generator emits the UTF-8 bytes of the value literally inside
//! a `( ... ) Tj` operator using Helvetica, which in PDF is a Type1 font with
//! WinAnsi encoding. Bytes above 0x7F are then interpreted as WinAnsi
//! codepoints, NOT as UTF-8.
//!
//! Minimal user-reachable reproduction (no custom fonts, no fixtures, no
//! exotic APIs — just `fill_field` with a Latin-1 value).
//!
//! Correct behavior (Helvetica path): the string `"café"` (U+0063 U+0061
//! U+0066 U+00E9) must be encoded to WinAnsi before being emitted inside the
//! string literal of the Tj operator. WinAnsi for `é` is byte 0xE9 — NOT the
//! two-byte UTF-8 sequence `C3 A9`. So the /AP/N content stream must contain
//! the four bytes `63 61 66 E9` (optionally with a leading octal escape for
//! the 0xE9 byte, `\351`), and must NOT contain the byte `C3`.
//!
//! When the fix lands, both assertions below go green and this test pins
//! the contract for every future non-ASCII fill.

use oxidize_pdf::forms::{FormManager, TextField, Widget, WidgetAppearance};
use oxidize_pdf::geometry::{Point, Rectangle};
use oxidize_pdf::parser::objects::PdfObject;
use oxidize_pdf::parser::PdfReader;
use oxidize_pdf::{Document, Page};
use std::io::Cursor;

/// Resolves the first page's first widget annotation /AP/N stream bytes.
fn extract_ap_n_bytes(pdf: &[u8]) -> Vec<u8> {
    let mut reader = PdfReader::new(Cursor::new(pdf)).expect("parse PDF bytes");

    // First page
    let pages = reader.pages().expect("/Pages").clone();
    let kids = pages
        .get("Kids")
        .and_then(|o| o.as_array())
        .expect("/Pages/Kids");
    let (page_n, page_g) = kids.0[0].as_reference().expect("page ref");
    let page_obj = reader.get_object(page_n, page_g).expect("page").clone();
    let page_dict = page_obj.as_dict().expect("page dict").clone();

    // First annotation
    let annots = page_dict
        .get("Annots")
        .and_then(|o| o.as_array())
        .expect("/Annots");
    let (annot_n, annot_g) = annots.0[0].as_reference().expect("annot ref");
    let annot_obj = reader.get_object(annot_n, annot_g).expect("annot").clone();
    let annot_dict = annot_obj.as_dict().expect("annot dict").clone();

    // /AP/N
    let ap = annot_dict
        .get("AP")
        .and_then(|o| o.as_dict())
        .expect("/AP")
        .clone();
    let normal = ap.get("N").expect("/AP/N").clone();

    match normal {
        PdfObject::Reference(n, g) => {
            let form_xobj = reader.get_object(n, g).expect("resolve /AP/N").clone();
            let stream = form_xobj.as_stream().expect("/AP/N stream");
            stream.decode(reader.options()).expect("decode /AP/N")
        }
        PdfObject::Stream(ref s) => s.decode(reader.options()).expect("decode inline /AP/N"),
        other => panic!("/AP/N must be a stream, got {:?}", other),
    }
}

/// Locates the first `( ... ) Tj` operator in the content stream and returns
/// the raw bytes inside the parentheses (after undoing PDF escape sequences
/// for `\\`, `\(`, `\)` and `\nnn` octal). Returns None if no Tj is found.
fn extract_first_tj_string(content: &[u8]) -> Option<Vec<u8>> {
    // Find the first `(` followed later by `) Tj` on the same logical op.
    // The content stream emitted by TextFieldAppearance is well-formed and
    // puts `) Tj\n` verbatim after the closing paren.
    let mut i = 0;
    while i < content.len() {
        if content[i] == b'(' {
            let start = i + 1;
            let mut j = start;
            let mut escape = false;
            while j < content.len() {
                if escape {
                    escape = false;
                    j += 1;
                    continue;
                }
                match content[j] {
                    b'\\' => {
                        escape = true;
                    }
                    b')' => break,
                    _ => {}
                }
                j += 1;
            }
            if j >= content.len() {
                return None;
            }
            // Require " Tj" (optionally with leading whitespace) right after.
            let tail = &content[j + 1..];
            let mut k = 0;
            while k < tail.len() && (tail[k] == b' ' || tail[k] == b'\t') {
                k += 1;
            }
            if tail.get(k..k + 2) == Some(b"Tj") {
                let raw = &content[start..j];
                return Some(decode_pdf_string_literal(raw));
            }
        }
        i += 1;
    }
    None
}

/// Decodes the PDF string literal escape sequences (`\\`, `\(`, `\)`, `\nnn`
/// octal, plus `\n \r \t \b \f`) into a raw byte sequence. Parentheses
/// appearing unescaped inside are not handled because our content-stream
/// emitter always escapes them.
fn decode_pdf_string_literal(raw: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(raw.len());
    let mut i = 0;
    while i < raw.len() {
        if raw[i] != b'\\' {
            out.push(raw[i]);
            i += 1;
            continue;
        }
        // Escape sequence
        if i + 1 >= raw.len() {
            break;
        }
        let c = raw[i + 1];
        match c {
            b'n' => {
                out.push(b'\n');
                i += 2;
            }
            b'r' => {
                out.push(b'\r');
                i += 2;
            }
            b't' => {
                out.push(b'\t');
                i += 2;
            }
            b'b' => {
                out.push(0x08);
                i += 2;
            }
            b'f' => {
                out.push(0x0C);
                i += 2;
            }
            b'(' | b')' | b'\\' => {
                out.push(c);
                i += 2;
            }
            b'0'..=b'7' => {
                // Up to 3 octal digits
                let mut value: u32 = 0;
                let mut consumed = 0;
                let mut k = i + 1;
                while consumed < 3 && k < raw.len() && (b'0'..=b'7').contains(&raw[k]) {
                    value = value * 8 + (raw[k] - b'0') as u32;
                    k += 1;
                    consumed += 1;
                }
                out.push((value & 0xFF) as u8);
                i = k;
            }
            _ => {
                // Unknown escape: drop backslash, keep char (PDF spec behavior).
                out.push(c);
                i += 2;
            }
        }
    }
    out
}

/// Build a baseline single-page PDF with one text field named "name".
/// No custom fonts — exercises the default Helvetica path.
fn build_doc_with_text_field() -> Document {
    let mut doc = Document::new();
    let mut page = Page::a4();
    let mut fm = FormManager::new();

    let rect = Rectangle::new(Point::new(100.0, 700.0), Point::new(300.0, 720.0));
    let widget = Widget::new(rect).with_appearance(WidgetAppearance::default());
    let field = TextField::new("name");
    let field_ref = fm
        .add_text_field(field, widget.clone(), None)
        .expect("add_text_field");

    page.add_form_widget_with_ref(widget, field_ref)
        .expect("add_form_widget_with_ref");
    doc.add_page(page);
    doc.set_form_manager(fm);
    doc
}

/// The defect: `fill_field("name", "café")` writes a /AP/N content stream
/// whose Tj string literal contains the raw UTF-8 bytes of the value
/// (C3 A9 for `é`), not the correct WinAnsi encoding (0xE9 for Helvetica).
///
/// Concrete consequences for any viewer that honors /AP (i.e., when
/// /AcroForm/NeedAppearances is false or absent):
///   - "Pérez"    → renders as "PÃ©rez"
///   - "café"     → renders as "cafÃ©"
///   - "résumé"   → renders as "rÃ©sumÃ©"
///   - "Juan ñoño" → renders as "Juan Ã±oÃ±o"
///   - Every non-ASCII Spanish / French / German / Portuguese etc. fill.
///
/// Contract (post-fix):
///   1. The Tj string must NOT contain byte 0xC3 (the UTF-8 lead byte for
///      U+0080..U+00FF). If it does, the writer dumped UTF-8 verbatim.
///   2. The Tj string MUST contain byte 0xE9 (the WinAnsi code for `é`),
///      whether encoded literally or via octal escape `\351`.
///   3. Together these pin the WinAnsi encoding.
#[test]
fn fill_field_latin1_value_is_winansi_encoded_not_utf8() {
    let mut doc = build_doc_with_text_field();
    doc.fill_field("name", "café")
        .expect("fill_field must succeed for a Latin-1 value");

    let pdf = doc.to_bytes().expect("serialize");
    let ap = extract_ap_n_bytes(&pdf);

    let tj_bytes = extract_first_tj_string(&ap)
        .expect("/AP/N must carry a Tj operator showing the filled value");

    // Contract 1: no UTF-8 lead bytes (0xC3 specifically for Latin-1
    // supplement). The presence of 0xC3 means the writer dumped UTF-8.
    assert!(
        !tj_bytes.contains(&0xC3),
        "Tj string contains UTF-8 lead byte 0xC3 — fill_field dumped UTF-8 bytes instead of WinAnsi-encoding the value. bytes = {:02X?}",
        tj_bytes
    );

    // Contract 2: the WinAnsi code for `é` (0xE9) must appear as a literal
    // byte. A compliant WinAnsi encoder may emit the byte directly OR escape
    // it as `\351` — but escape sequences are already decoded by
    // `decode_pdf_string_literal`, so the decoded stream has the raw byte
    // either way.
    assert!(
        tj_bytes.contains(&0xE9),
        "Tj string does not contain WinAnsi byte 0xE9 for `é` — value was not encoded to WinAnsi. bytes = {:02X?}",
        tj_bytes
    );

    // Contract 3: full WinAnsi round-trip. The decoded bytes of "café" in
    // WinAnsi are `c a f é` → 63 61 66 E9.
    assert_eq!(
        tj_bytes,
        vec![0x63, 0x61, 0x66, 0xE9],
        "Tj string bytes must equal WinAnsi encoding of 'café'. got = {:02X?}",
        tj_bytes
    );
}

/// ASCII-only values must continue to work unchanged (regression fence for
/// the fix). The WinAnsi encoding of an ASCII string is byte-identical to
/// its UTF-8 encoding, so no behavior change expected on this path — but if
/// the fix accidentally breaks ASCII the suite catches it here.
#[test]
fn fill_field_ascii_value_still_emits_raw_ascii_in_tj() {
    let mut doc = build_doc_with_text_field();
    doc.fill_field("name", "John")
        .expect("fill_field ASCII must succeed");

    let pdf = doc.to_bytes().expect("serialize");
    let ap = extract_ap_n_bytes(&pdf);

    let tj_bytes = extract_first_tj_string(&ap).expect("Tj operator");

    assert_eq!(
        tj_bytes,
        b"John".to_vec(),
        "ASCII fill must emit raw ASCII bytes — regression fence"
    );
}

/// CJK reproduction. The expected post-fix behavior here is NOT WinAnsi
/// (Helvetica cannot render CJK) — the correct fix must either:
///   (a) pick a Type0/CID font registered in the document for the widget
///       (via /DA or a per-widget font selector), and emit hex CIDs;
///   (b) return an error at `fill_field` time refusing to silently render
///       garbage when the value contains codepoints unsupported by
///       Helvetica's WinAnsi.
///
/// This test asserts the minimum anti-garbage contract: the /AP/N content
/// stream MUST NOT contain the raw UTF-8 bytes of the CJK string (which
/// would render as `.notdef` triplets). Either (a) produces hex-CID Tj and
/// no literal UTF-8, or (b) returns Err and `to_bytes()` would not even
/// reach this assertion. Both paths are acceptable — silent UTF-8 dump is
/// not.
#[test]
fn fill_field_cjk_value_must_not_dump_utf8_bytes() {
    let mut doc = build_doc_with_text_field();
    let fill = doc.fill_field("name", "高效能");

    // Either fill_field returns Err (refusing to produce malformed output),
    // or the resulting /AP/N content stream does NOT contain the UTF-8
    // bytes of the CJK string.
    if fill.is_err() {
        // Acceptable — explicit refusal is better than silent corruption.
        return;
    }

    let pdf = doc.to_bytes().expect("serialize");
    let ap = extract_ap_n_bytes(&pdf);

    // UTF-8 bytes of "高效能":
    // 高 = E9 AB 98
    // 效 = E6 95 88
    // 能 = E8 83 BD
    let utf8_needle: &[u8] = "高效能".as_bytes();

    // Search the content stream (not just the Tj — the bytes must not be
    // present anywhere in the post-decode content, in any encoded form).
    assert!(
        !ap.windows(utf8_needle.len()).any(|w| w == utf8_needle),
        "/AP/N content stream contains the raw UTF-8 bytes of a CJK fill value — \
         fill_field silently produced a corrupt appearance instead of using a \
         CID-capable font or returning an error. content = {:02X?}",
        ap
    );
}
