//! Acceptance test for #334 item #5: PDF byte output must be deterministic
//! when multiple custom fonts are registered, regardless of the order the
//! caller registered them.
//!
//! Root cause: `FontCache::font_names()` (src/fonts/font_cache.rs:48-53)
//! returns `fonts.keys().cloned().collect()` from a
//! `HashMap<String, Arc<Font>>`. The order is randomized per HashMap
//! instance. `PdfWriter::write_fonts` iterates that order to allocate
//! ObjectIds (via `write_font_with_unicode_support` → `allocate_object_id`),
//! so two builds of the same document allocate different ObjectIds for the
//! same font. Every cross-reference downstream (xref table entries,
//! `/Resources /Font /Fn N 0 R` references, page content stream fingerprints)
//! changes accordingly.
//!
//! Test bypasses `Document::to_bytes()`/`save()` because both call
//! `update_modification_date()` which writes `Utc::now()` into the metadata
//! — that alone would make every output differ in the `/ModDate` and
//! `xmp:ModifyDate` bytes. Going through `PdfWriter::write_document`
//! directly keeps `creation_date` and `modification_date` pinned for full
//! byte-equality.

use chrono::{TimeZone, Utc};
use oxidize_pdf::writer::{PdfWriter, WriterConfig};
use oxidize_pdf::{Document, Font, Page};

const LATIN_FONT_PATH: &str = "../test-pdfs/Roboto-Regular.ttf";

fn load_font() -> Option<Vec<u8>> {
    if std::path::Path::new(LATIN_FONT_PATH).exists() {
        Some(std::fs::read(LATIN_FONT_PATH).expect("read font fixture"))
    } else {
        eprintln!("SKIPPED: font fixture not found at {LATIN_FONT_PATH}");
        None
    }
}

/// Build a document with three custom fonts whose names are lexically
/// non-monotonic relative to insertion order, then serialize via the writer
/// with pinned dates. Returns the PDF bytes.
fn build_pdf_bytes() -> Option<Vec<u8>> {
    let font_data = load_font()?;
    let mut doc = Document::new();
    // Pin both dates so the writer emits the same /CreationDate and
    // /ModDate every call — the only timing-dependent bytes in the writer
    // come from these two fields.
    let pinned = Utc.with_ymd_and_hms(2026, 6, 16, 0, 0, 0).unwrap();
    doc.set_creation_date(pinned);
    doc.set_modification_date(pinned);

    // Non-monotonic registration order: z, a, m. If FontCache::font_names()
    // returns its `HashMap<String, ...>::keys()` directly (the bug), the
    // iteration order varies between runs and cascades through ObjectId
    // allocation. With a sorted return, iteration is always
    // [alpha, malpha, zalpha] regardless of insertion order.
    doc.add_font_from_bytes("zalpha", font_data.clone())
        .expect("register zalpha");
    doc.add_font_from_bytes("alpha", font_data.clone())
        .expect("register alpha");
    doc.add_font_from_bytes("malpha", font_data)
        .expect("register malpha");

    // Use each font on a page so they all get tracked in
    // `document_used_chars_by_font` and pass the `has_usage` filter in
    // `write_fonts`. Without this, unused fonts are skipped and the bug
    // does not surface.
    let mut page = Page::new(612.0, 792.0);
    page.text()
        .set_font(Font::custom("zalpha"), 12.0)
        .at(50.0, 700.0)
        .write("Zalpha quick brown fox")
        .expect("zalpha write");
    page.text()
        .set_font(Font::custom("alpha"), 12.0)
        .at(50.0, 680.0)
        .write("Alpha lazy dog jumps")
        .expect("alpha write");
    page.text()
        .set_font(Font::custom("malpha"), 12.0)
        .at(50.0, 660.0)
        .write("Malpha cat naps softly")
        .expect("malpha write");
    doc.add_page(page);

    // Bypass Document::to_bytes(): it calls update_modification_date()
    // unconditionally. Go through PdfWriter directly with pinned dates.
    let mut buffer = Vec::new();
    let config = WriterConfig {
        use_xref_streams: false,
        use_object_streams: false,
        pdf_version: "1.7".to_string(),
        compress_streams: true,
        incremental_update: false,
    };
    let mut writer = PdfWriter::with_config(&mut buffer, config);
    writer
        .write_document(&mut doc)
        .expect("PdfWriter::write_document");
    Some(buffer)
}

/// Primary contract: building the same document N times in the same process
/// produces N byte-identical PDFs. Without the FontCache sort fix, each
/// build picks a different HashMap iteration order → different font
/// ObjectId assignments → diverging xref tables and resource refs.
#[test]
fn pdf_bytes_with_multiple_fonts_are_stable_across_builds() {
    let baseline = match build_pdf_bytes() {
        Some(b) => b,
        None => return,
    };
    let mut first_diverge: Option<(usize, usize)> = None;
    for i in 1..10 {
        let again = build_pdf_bytes().expect("font fixture available on first build but not later");
        if again != baseline {
            // Find the byte index where they first differ for a useful
            // failure message; capture both lengths so a wholesale size
            // mismatch surfaces clearly.
            let diff_idx = baseline
                .iter()
                .zip(again.iter())
                .position(|(a, b)| a != b)
                .unwrap_or_else(|| baseline.len().min(again.len()));
            first_diverge = Some((i, diff_idx));
            break;
        }
    }
    assert!(
        first_diverge.is_none(),
        "PDF bytes diverged on build #{} at byte offset {} — non-deterministic font ObjectId allocation (FontCache::font_names() returns HashMap order)",
        first_diverge.unwrap().0,
        first_diverge.unwrap().1,
    );
}

/// Anchor the contract to its semantics: `Document::custom_font_names()`
/// (which writes_fonts iterates) must return names in lexicographic order
/// regardless of registration order. Guards against future refactors that
/// "restore byte stability" via a non-canonical scheme.
#[test]
fn custom_font_names_returns_sorted() {
    let font_data = match load_font() {
        Some(d) => d,
        None => return,
    };
    let mut doc = Document::new();
    // Register in deliberately non-sorted order.
    doc.add_font_from_bytes("zalpha", font_data.clone())
        .unwrap();
    doc.add_font_from_bytes("alpha", font_data.clone()).unwrap();
    doc.add_font_from_bytes("malpha", font_data).unwrap();

    let names = doc.custom_font_names();
    let mut sorted = names.clone();
    sorted.sort();
    assert_eq!(
        names, sorted,
        "Document::custom_font_names() must return names in lexicographic order — \
         got {:?}, expected {:?}",
        names, sorted
    );
}
