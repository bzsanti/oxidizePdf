//! Issue #204 — per-font character tracking.
//!
//! When multiple custom fonts are registered on a `Document` but only a
//! subset are actually referenced from content streams, the output PDF
//! must NOT embed the unused fonts with the character set accumulated by
//! the used fonts.
//!
//! Prior to this fix (<2.5.7) `Document.used_characters` was a single
//! `HashSet<char>` shared across all fonts. The writer subsetted every
//! registered font with the same global set, so two CJK fonts in the
//! same family (e.g. `SourceHanSansTC-Regular` + `SourceHanSansTC-Bold`)
//! — where the second one was registered but never called via
//! `set_font` — both ended up with ~200-glyph subsets, doubling the
//! emitted size. Reported by @sparkyandrew in
//! https://github.com/bzsanti/oxidizePdf/issues/204
//!
//! The fix is in two parts:
//!   1. Track characters per-font (`HashMap<String, HashSet<char>>`) so
//!      each font's subset reflects only the characters actually drawn
//!      with that font.
//!   2. Skip emitting fonts whose per-font character set is empty (no
//!      content stream can reference them anyway, so embedding is waste).
//!
//! Assertions below target **wire format**: parse the emitted PDF and
//! inspect `/Resources/Font` + referenced font dictionaries. No file-size
//! smoke tests.

use oxidize_pdf::parser::objects::PdfObject;
use oxidize_pdf::parser::PdfReader;
use oxidize_pdf::{Document, Font, Page};
use std::io::Cursor;

const CJK_PATH: &str = "../test-pdfs/SourceHanSansSC-Regular.otf";
const LATIN_PATH: &str = "../test-pdfs/SourceSans3-Regular.otf";

fn load_fixture(path: &str) -> Option<Vec<u8>> {
    std::fs::read(path)
        .map_err(|_| eprintln!("SKIPPED: {} not found", path))
        .ok()
}

/// Walk the first page's `/Resources/Font` dict.
fn resolve_page0_fonts<R: std::io::Read + std::io::Seek>(
    reader: &mut PdfReader<R>,
) -> oxidize_pdf::parser::objects::PdfDictionary {
    let pages = reader.pages().expect("/Pages").clone();
    let kids = pages
        .get("Kids")
        .and_then(|o| o.as_array())
        .expect("/Pages/Kids");
    let (page_n, page_g) = kids.0[0].as_reference().expect("/Pages/Kids[0] ref");
    let page_obj = reader.get_object(page_n, page_g).expect("page").clone();
    let page_dict = page_obj.as_dict().expect("page dict").clone();
    let resources = match page_dict.get("Resources").expect("/Resources") {
        PdfObject::Dictionary(d) => d.clone(),
        PdfObject::Reference(n, g) => reader
            .get_object(*n, *g)
            .expect("resolve /Resources")
            .clone()
            .as_dict()
            .expect("/Resources is dict")
            .clone(),
        other => panic!("/Resources: unexpected {:?}", other),
    };
    match resources.get("Font").expect("/Resources/Font") {
        PdfObject::Dictionary(d) => d.clone(),
        PdfObject::Reference(n, g) => reader
            .get_object(*n, *g)
            .expect("resolve /Font")
            .clone()
            .as_dict()
            .expect("/Font dict")
            .clone(),
        other => panic!("/Font: unexpected {:?}", other),
    }
}

/// Primary regression for #204. Two large CJK fonts are registered under
/// different names (simulating Regular + Bold of the same family, which
/// is the user's exact scenario), but only one is referenced via
/// `set_font`. The unused font must NOT appear in any page's
/// `/Resources/Font` — it's unreferenced by any content stream and
/// embedding it wastes 5-20KB per font.
#[test]
fn unused_font_is_absent_from_resources() {
    let cjk_data = match load_fixture(CJK_PATH) {
        Some(d) => d,
        None => return,
    };

    let mut doc = Document::new();
    // Two logical fonts, same bytes — simulates Regular + Bold of the
    // same family. Both are 16.5 MB on disk.
    doc.add_font_from_bytes("CJK-Regular", cjk_data.clone())
        .expect("add CJK-Regular");
    doc.add_font_from_bytes("CJK-Bold", cjk_data)
        .expect("add CJK-Bold");

    let mut page = Page::a4();
    page.text()
        .set_font(Font::Custom("CJK-Regular".to_string()), 14.0)
        .at(30.0, 800.0)
        .write("高效能")
        .expect("write CJK text with Regular");
    doc.add_page(page);

    let pdf_bytes = doc.to_bytes().expect("serialize");
    let mut reader = PdfReader::new(Cursor::new(&pdf_bytes)).expect("parse");

    let fonts = resolve_page0_fonts(&mut reader);

    assert!(
        fonts.get("CJK-Regular").is_some(),
        "/Resources/Font must contain CJK-Regular (it was used); keys = {:?}",
        fonts.0.keys().map(|k| &k.0).collect::<Vec<_>>()
    );
    assert!(
        fonts.get("CJK-Bold").is_none(),
        "/Resources/Font must NOT contain CJK-Bold (no content stream references it); \
         keys = {:?}. Embedding unused fonts wastes space — see issue #204.",
        fonts.0.keys().map(|k| &k.0).collect::<Vec<_>>()
    );
}

/// Mixed scenario: one CJK font used, one Latin font registered but
/// unused. Same assertion as the CJK+CJK case — the unused Latin font
/// must not surface in `/Resources/Font` even though its character
/// coverage partially overlaps with the accumulated global set (ASCII
/// characters that may appear in CJK text are present in both fonts).
#[test]
fn unused_latin_font_absent_when_cjk_used() {
    let cjk_data = match load_fixture(CJK_PATH) {
        Some(d) => d,
        None => return,
    };
    let latin_data = match load_fixture(LATIN_PATH) {
        Some(d) => d,
        None => return,
    };

    let mut doc = Document::new();
    doc.add_font_from_bytes("CJK", cjk_data).expect("add CJK");
    doc.add_font_from_bytes("Latin", latin_data)
        .expect("add Latin");

    let mut page = Page::a4();
    page.text()
        .set_font(Font::Custom("CJK".to_string()), 14.0)
        .at(30.0, 800.0)
        .write("可靠性")
        .expect("write CJK text");
    doc.add_page(page);

    let pdf_bytes = doc.to_bytes().expect("serialize");
    let mut reader = PdfReader::new(Cursor::new(&pdf_bytes)).expect("parse");
    let fonts = resolve_page0_fonts(&mut reader);

    assert!(fonts.get("CJK").is_some(), "CJK must be present (used)");
    assert!(
        fonts.get("Latin").is_none(),
        "Latin must be absent (registered but never referenced via set_font); \
         keys = {:?}",
        fonts.0.keys().map(|k| &k.0).collect::<Vec<_>>()
    );
}

/// Gap R1 from the post-fix analysis: `layout::rich_text::RichText` used
/// inside `FlowLayout` emits its content stream via `append_raw_content`
/// which historically bypassed char tracking. Under the issue #204 fix,
/// any custom font only referenced from RichText would silently disappear
/// from the output (no bucket → writer skips → widget references a
/// missing font).
///
/// This test enforces the fixed contract: the RichText-referenced font
/// MUST appear in `/Resources/Font` when the document is serialised.
/// The `pub(crate)` method `Page::append_raw_content` now requires a
/// `font_usage` map as a typed gate, so every future caller is
/// compile-errored into reporting what they draw.
#[test]
fn rich_text_with_custom_font_embeds_font() {
    use oxidize_pdf::layout::{FlowLayout, PageConfig, RichText, TextSpan};
    use oxidize_pdf::Color;

    let cjk_data = match load_fixture(CJK_PATH) {
        Some(d) => d,
        None => return,
    };

    let mut doc = Document::new();
    doc.add_font_from_bytes("MyCJK", cjk_data)
        .expect("register CJK");

    let rich = RichText::new(vec![TextSpan::new(
        "高效能",
        Font::Custom("MyCJK".to_string()),
        14.0,
        Color::black(),
    )]);

    let config = PageConfig::a4_with_margins(50.0, 50.0, 50.0, 50.0);
    let mut layout = FlowLayout::new(config);
    layout.add_rich_text(rich);
    layout.build_into(&mut doc).expect("build flow");

    let pdf_bytes = doc.to_bytes().expect("serialize");
    let mut reader = PdfReader::new(Cursor::new(&pdf_bytes)).expect("parse");
    let fonts = resolve_page0_fonts(&mut reader);

    assert!(
        fonts.get("MyCJK").is_some(),
        "/Resources/Font must contain MyCJK because RichText referenced it; \
         keys = {:?}. If this fires, `RichText::render_operations` or \
         its caller in `FlowLayout::build_into` is not reporting font \
         usage to the page (gap R1 from issue #204 analysis).",
        fonts.0.keys().map(|k| &k.0).collect::<Vec<_>>()
    );
}

/// Gap R4: `DocumentBuilder::add_text(text, Font::Custom(...), size)`
/// (and more broadly any caller of `Page::add_text_flow` whose
/// `TextFlowContext` uses a custom font) emits `Tj` operators into the
/// page content stream without going through `GraphicsContext::show_text`
/// or `TextContext::write`. The pre-fix global `used_characters` was
/// also not updated, but the writer embedded every registered font
/// anyway. Post-fix, the unused font is skipped → PDF has `/CJK 14 Tf`
/// but no `/CJK` resource.
///
/// The typed-gate plumbing now extends to `TextFlowContext` so the
/// page tracks chars drawn via `write_wrapped`.
#[test]
fn text_flow_with_custom_font_embeds_font() {
    use oxidize_pdf::layout::{FlowLayout, PageConfig};

    let cjk_data = match load_fixture(CJK_PATH) {
        Some(d) => d,
        None => return,
    };

    // DocumentBuilder::build() creates a new Document, so we use
    // build_into on a Document that has the custom font pre-registered.
    let mut doc = Document::new();
    doc.add_font_from_bytes("MyCJK", cjk_data)
        .expect("register CJK");

    let config = PageConfig::a4_with_margins(50.0, 50.0, 50.0, 50.0);
    let mut layout = FlowLayout::new(config);
    layout.add_text("高效能", Font::Custom("MyCJK".to_string()), 14.0);
    layout.build_into(&mut doc).expect("build flow");

    let pdf_bytes = doc.to_bytes().expect("serialize");
    let mut reader = PdfReader::new(Cursor::new(&pdf_bytes)).expect("parse");
    let fonts = resolve_page0_fonts(&mut reader);

    assert!(
        fonts.get("MyCJK").is_some(),
        "/Resources/Font must contain MyCJK: `DocumentBuilder::add_text` \
         with a Custom font goes through `TextFlowContext` + \
         `page.add_text_flow` which, gap R4, did not track chars. keys = {:?}",
        fonts.0.keys().map(|k| &k.0).collect::<Vec<_>>()
    );
}

/// Gap R5: a page whose ONLY reference to a custom font is inside a
/// header (or footer) must still embed that font. Pre-fix the header's
/// chars were never tracked anywhere — the writer fell back to an
/// ASCII character set and subsetted the font with digits + letters
/// by accident, so `"Page 1 of 10"` rendered. Post-issue-#204, the
/// per-font bucket for that font is empty → writer skips it → header
/// references a missing font resource.
///
/// Fix: eagerly register header/footer font + sampled template chars
/// at `Page::set_header` / `set_footer` time, so the page's
/// graphics-context accumulator contains enough characters for the
/// writer to embed a usable subset.
#[test]
fn header_only_custom_font_embeds_font() {
    use oxidize_pdf::text::{HeaderFooter, HeaderFooterOptions};

    let cjk_data = match load_fixture(CJK_PATH) {
        Some(d) => d,
        None => return,
    };

    let mut doc = Document::new();
    doc.add_font_from_bytes("MyCJK", cjk_data)
        .expect("register CJK");

    let mut page = Page::a4();
    // No text drawn on the page body. Only the header references MyCJK.
    let options = HeaderFooterOptions {
        font: Font::Custom("MyCJK".to_string()),
        font_size: 10.0,
        ..Default::default()
    };
    let header =
        HeaderFooter::new_header("Page {{page_number}} of {{total_pages}}").with_options(options);
    page.set_header(header);
    doc.add_page(page);

    let pdf_bytes = doc.to_bytes().expect("serialize");
    let mut reader = PdfReader::new(Cursor::new(&pdf_bytes)).expect("parse");
    let fonts = resolve_page0_fonts(&mut reader);

    assert!(
        fonts.get("MyCJK").is_some(),
        "/Resources/Font must contain MyCJK even when only a header \
         references it; keys = {:?}. See gap R5 in issue #204 analysis.",
        fonts.0.keys().map(|k| &k.0).collect::<Vec<_>>()
    );
}

/// Regression: when both fonts ARE used, each must remain independently
/// registered. Guards against an over-aggressive fix that skips fonts
/// incorrectly.
#[test]
fn both_fonts_present_when_both_used() {
    let cjk_data = match load_fixture(CJK_PATH) {
        Some(d) => d,
        None => return,
    };
    let latin_data = match load_fixture(LATIN_PATH) {
        Some(d) => d,
        None => return,
    };

    let mut doc = Document::new();
    doc.add_font_from_bytes("CJK", cjk_data).expect("add CJK");
    doc.add_font_from_bytes("Latin", latin_data)
        .expect("add Latin");

    let mut page = Page::a4();
    page.text()
        .set_font(Font::Custom("CJK".to_string()), 14.0)
        .at(30.0, 800.0)
        .write("高效能")
        .expect("CJK text");
    page.text()
        .set_font(Font::Custom("Latin".to_string()), 12.0)
        .at(30.0, 780.0)
        .write("Hello world")
        .expect("Latin text");
    doc.add_page(page);

    let pdf_bytes = doc.to_bytes().expect("serialize");
    let mut reader = PdfReader::new(Cursor::new(&pdf_bytes)).expect("parse");
    let fonts = resolve_page0_fonts(&mut reader);

    assert!(
        fonts.get("CJK").is_some(),
        "CJK must be present when used; keys = {:?}",
        fonts.0.keys().map(|k| &k.0).collect::<Vec<_>>()
    );
    assert!(
        fonts.get("Latin").is_some(),
        "Latin must be present when used; keys = {:?}",
        fonts.0.keys().map(|k| &k.0).collect::<Vec<_>>()
    );
}
