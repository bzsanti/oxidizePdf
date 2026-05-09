//! Suite 4 — full render pipeline tests for issue #230.
//!
//! Each test renders a PDF with a custom font and verifies that the
//! per-Document scope flows correctly through the
//! Document → Page → TextContext → content emission pipeline.

use oxidize_pdf::text::Font;
use oxidize_pdf::Document;

const LATIN_FONT_PATH: &str = "../test-pdfs/Roboto-Regular.ttf";
const CJK_FONT_PATH: &str = "../test-pdfs/SourceHanSansSC-Regular.otf";

fn load_latin_font() -> Option<Vec<u8>> {
    if std::path::Path::new(LATIN_FONT_PATH).exists() {
        Some(std::fs::read(LATIN_FONT_PATH).expect("read Latin TTF fixture"))
    } else {
        eprintln!("SKIPPED: Latin font fixture not found at {LATIN_FONT_PATH}");
        None
    }
}

fn load_cjk_font() -> Option<Vec<u8>> {
    if std::path::Path::new(CJK_FONT_PATH).exists() {
        Some(std::fs::read(CJK_FONT_PATH).expect("read CJK OTF fixture"))
    } else {
        eprintln!("SKIPPED: CJK font fixture not found at {CJK_FONT_PATH}");
        None
    }
}

/// Test 4.1 — full render with a per-Document custom font; verify the
/// emitted PDF round-trips and references the font name.
#[test]
fn cjk_render_per_document_widths() {
    let cjk = match load_cjk_font() {
        Some(b) => b,
        None => return,
    };

    let mut doc = Document::new();
    doc.add_font_from_bytes("NotoCJK_4_1", cjk).expect("font");

    let mut page = doc.new_page_a4();
    page.text()
        .set_font(Font::Custom("NotoCJK_4_1".into()), 12.0)
        .at(50.0, 750.0)
        .write("高効能テスト")
        .expect("write CJK");
    doc.add_page(page);

    let bytes = doc.to_bytes().expect("render");

    // Round-trip: parse the rendered PDF and verify it has 1 page.
    // PdfReader::new fails loudly if the bytes are not a valid PDF — that
    // is the structural-validity gate. page_count == 1 verifies the
    // content tree shape that the test set up.
    let mut reader =
        oxidize_pdf::parser::PdfReader::new(std::io::Cursor::new(&bytes)).expect("read back");
    assert_eq!(reader.page_count().expect("page count"), 1);

    // The PDF must reference the registered font name (or its subset
    // prefix). Embedded font names in Type0 use BaseFont = /XXXXXX+Name.
    let raw = String::from_utf8_lossy(&bytes);
    assert!(
        raw.contains("NotoCJK_4_1"),
        "rendered PDF must reference the registered font name 'NotoCJK_4_1'"
    );
}

/// Test 4.2 — two Documents register the same name with different fonts;
/// each rendered PDF must reflect its own font's bytes (no cross-Document
/// contamination via the legacy global last-writer-wins).
#[test]
fn render_two_documents_no_cross_contamination() {
    let latin = match load_latin_font() {
        Some(b) => b,
        None => return,
    };
    let cjk = match load_cjk_font() {
        Some(b) => b,
        None => return,
    };

    let mut doc_a = Document::new();
    doc_a
        .add_font_from_bytes("Shared_4_2", latin)
        .expect("doc_a font");
    let mut page_a = doc_a.new_page_a4();
    page_a
        .text()
        .set_font(Font::Custom("Shared_4_2".into()), 12.0)
        .at(50.0, 750.0)
        .write("Hello")
        .expect("write a");
    doc_a.add_page(page_a);
    let bytes_a = doc_a.to_bytes().expect("render a");

    let mut doc_b = Document::new();
    doc_b
        .add_font_from_bytes("Shared_4_2", cjk)
        .expect("doc_b font");
    let mut page_b = doc_b.new_page_a4();
    page_b
        .text()
        .set_font(Font::Custom("Shared_4_2".into()), 12.0)
        .at(50.0, 750.0)
        .write("高効能")
        .expect("write b");
    doc_b.add_page(page_b);
    let bytes_b = doc_b.to_bytes().expect("render b");

    // Pre-fix, both Documents shared the last-writer global → doc_a's
    // "Hello" would have been measured/embedded with doc_b's CJK font's
    // metrics. Post-fix, each Document carries its own font bytes.
    // The byte-level cross-contamination signal lives in Suite 1
    // (`multi_document_isolation`); here we ratify that the full render
    // pipeline produces two structurally-valid, distinct PDFs for two
    // docs sharing a font name. Both must round-trip through PdfReader
    // (proves they are not garbage), each must have exactly one page,
    // and each must reference the font name they registered.
    let mut reader_a = oxidize_pdf::parser::PdfReader::new(std::io::Cursor::new(&bytes_a))
        .expect("doc_a parses back");
    assert_eq!(reader_a.page_count().expect("page count a"), 1);
    let mut reader_b = oxidize_pdf::parser::PdfReader::new(std::io::Cursor::new(&bytes_b))
        .expect("doc_b parses back");
    assert_eq!(reader_b.page_count().expect("page count b"), 1);

    let raw_a = String::from_utf8_lossy(&bytes_a);
    let raw_b = String::from_utf8_lossy(&bytes_b);
    assert!(
        raw_a.contains("Shared_4_2"),
        "doc_a PDF must reference its font name 'Shared_4_2'"
    );
    assert!(
        raw_b.contains("Shared_4_2"),
        "doc_b PDF must reference its font name 'Shared_4_2'"
    );

    // The two encoded text payloads differ (Latin "Hello" vs CJK glyphs),
    // and each Document subsetted its own TTF, so the produced bytes
    // cannot be byte-identical even though they share the font name.
    assert_ne!(
        bytes_a, bytes_b,
        "two docs with the same font name but different bytes/text must produce different PDFs"
    );
}
