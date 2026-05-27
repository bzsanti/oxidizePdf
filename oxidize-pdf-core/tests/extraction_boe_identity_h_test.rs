//! Issue #272 (Bug A) — Identity-H ToUnicode decoding for minified CMaps.
//!
//! BOE (Boletín Oficial del Estado) PDFs ship Type0 / Identity-H fonts
//! whose `/ToUnicode` CMap puts `begincodespacerange ... endcodespacerange`
//! on a single line. The pre-fix CMap parser (line-based state machine)
//! got stuck and produced an empty CMap; the extractor then fell through
//! to PdfDocEncoding and emitted the second byte of each 2-byte CID as
//! raw Latin-1, prefixed with a sanitized null:
//!
//!   `MINISTERIO DE ECONOMÍA` → ` 0 , 1 , 6 7 ( 5 , 2 ' ( ( & 2 1 2 0 ...`
//!
//! After the fix the CMap parser tokenises the PostScript content, the
//! BOE font lands in `font_cache` with a real ToUnicode, and the 2-byte
//! CIDs decode via bfchar/bfrange to readable Spanish text.

use oxidize_pdf::parser::{PdfDocument, PdfReader};
use oxidize_pdf::text::{ExtractionOptions, TextExtractor};
use std::path::PathBuf;

const BOE_FIXTURE: &str = "tests/fixtures/issue_272_boe_sumario_2025_01_15.pdf";

fn extract_page1_text() -> String {
    let pdf_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(BOE_FIXTURE);
    let reader = PdfReader::open(&pdf_path).expect("BOE fixture must be readable");
    let document = PdfDocument::new(reader);
    let mut extractor = TextExtractor::with_options(ExtractionOptions::default());
    extractor
        .extract_from_page(&document, 0)
        .expect("extract page 0")
        .text
}

/// BOE sumario 2025-01-15 page 1 contains the heading
/// "MINISTERIO DE ECONOMÍA, COMERCIO Y EMPRESA". Before the fix this
/// came out as ` 0 , 1 , 6 7 ( 5 , 2 ' ( ( & 2 1 2 0 ...`. After
/// the fix the Identity-H ToUnicode CMap decodes correctly.
#[test]
fn boe_page1_decodes_ministerio_heading() {
    let text = extract_page1_text();
    let head: String = text.chars().take(600).collect();
    assert!(
        text.contains("MINISTERIO"),
        "page 1 must contain 'MINISTERIO'; first 600 chars were:\n{}",
        head
    );
}

/// Additional structural assertion. Page 1 of any BOE sumario carries
/// "DISPOSICIONES GENERALES" as a section header. Asserting two
/// independent Spanish keywords guards against accidentally passing
/// the first assertion via a partial decode.
#[test]
fn boe_page1_decodes_disposiciones_section() {
    let text = extract_page1_text();
    let head: String = text.chars().take(600).collect();
    assert!(
        text.contains("DISPOSICIONES"),
        "page 1 must contain 'DISPOSICIONES'; first 600 chars were:\n{}",
        head
    );
}

/// Anti-regression: the pre-fix output had the literal byte-with-space
/// pattern `\u{0020}0\u{0020},\u{0020}1\u{0020},\u{0020}6` (every
/// second byte of a 2-byte CID prefixed by sanitised NUL → space).
/// If the parser regresses and CMap returns 0 mappings again, this
/// exact sequence reappears. The presence of `" 0 , 1 ,"` anywhere in
/// the page-1 text would mean the bug is back.
#[test]
fn boe_page1_does_not_contain_glyph_index_garbage() {
    let text = extract_page1_text();
    // The "M" / "I" / "N" / "I" / "S" / "T" CIDs of Arimo-Bold in this
    // font are 0x0030/0x002C/0x0031/0x002C/0x0036/0x0037 respectively.
    // Pre-fix output is `" 0 , 1 , 6 7"`; post-fix this sequence
    // (with single spaces between literal ASCII digits) must NOT appear.
    assert!(
        !text.contains(" 0 , 1 , 6 7"),
        "pre-fix glyph-index garbage ` 0 , 1 , 6 7` must not appear; got first 200 chars:\n{}",
        text.chars().take(200).collect::<String>()
    );
}
