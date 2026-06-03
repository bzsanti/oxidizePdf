//! Issue #287: custom TTF glyphs render as .notdef when the embedded font has
//! no glyph for them. This is correct PDF behaviour (the font genuinely lacks
//! the glyph), but the library used to map missing characters to GID 0
//! silently, giving no way to tell a font-coverage gap from a library bug.
//!
//! These tests cover the diagnostic API added to detect that gap up front.
//! Roboto-Regular covers Basic Latin, Latin-1 accents and general punctuation
//! (— •) but has no glyph for U+2713 (✓) / U+2717 (✗) — the exact profile of
//! the reporter's DMSans.

use oxidize_pdf::text::Font;
use oxidize_pdf::{Document, Page};
use std::sync::{Arc, Mutex};

fn roboto_bytes() -> Vec<u8> {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../test-pdfs/Roboto-Regular.ttf"
    );
    std::fs::read(path).expect("Roboto-Regular.ttf fixture")
}

/// `MakeWriter` that appends all log output into a shared buffer.
#[derive(Clone)]
struct BufWriter(Arc<Mutex<Vec<u8>>>);

impl std::io::Write for BufWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(buf);
        Ok(buf.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl<'a> tracing_subscriber::fmt::MakeWriter<'a> for BufWriter {
    type Writer = BufWriter;
    fn make_writer(&'a self) -> Self::Writer {
        self.clone()
    }
}

#[test]
fn test_font_missing_glyphs_reports_uncovered_characters() {
    let mut doc = Document::new();
    doc.add_font_from_bytes("Roboto", roboto_bytes()).unwrap();

    // ✓ and ✗ are absent from Roboto; A, É, — and • are present.
    let missing =
        doc.font_missing_glyphs("Roboto", "A \u{00C9} \u{2713} \u{2717} \u{2014} \u{2022}");
    assert_eq!(
        missing,
        vec!['\u{2713}', '\u{2717}'],
        "only the uncovered ✓/✗ must be reported, in first-seen order"
    );
}

#[test]
fn test_font_missing_glyphs_empty_when_all_covered() {
    let mut doc = Document::new();
    doc.add_font_from_bytes("Roboto", roboto_bytes()).unwrap();

    let missing = doc.font_missing_glyphs("Roboto", "Hello \u{00E9}\u{00F1} \u{2014}\u{2022}");
    assert!(
        missing.is_empty(),
        "fully covered text must report no missing glyphs, got {missing:?}"
    );
}

#[test]
fn test_font_missing_glyphs_deduplicates_and_ignores_controls() {
    let mut doc = Document::new();
    doc.add_font_from_bytes("Roboto", roboto_bytes()).unwrap();

    // Repeated ✓ and a newline (control) must not produce duplicates/entries.
    let missing = doc.font_missing_glyphs("Roboto", "\u{2713}\n\u{2713}\t\u{2713}");
    assert_eq!(missing, vec!['\u{2713}']);
}

#[test]
fn test_font_missing_glyphs_unknown_font_is_empty() {
    let doc = Document::new();
    assert!(doc
        .font_missing_glyphs("NotRegistered", "\u{2713}")
        .is_empty());
}

#[test]
fn test_font_has_glyph_reflects_cmap_coverage() {
    let mut doc = Document::new();
    doc.add_font_from_bytes("Roboto", roboto_bytes()).unwrap();
    let font = doc
        .custom_font("Roboto")
        .expect("registered font must be retrievable");

    assert!(font.has_glyph('A'));
    assert!(font.has_glyph('\u{00C9}')); // É
    assert!(font.has_glyph('\u{2014}')); // —
    assert!(!font.has_glyph('\u{2713}')); // ✓ absent
    assert!(!font.has_glyph('\u{2717}')); // ✗ absent
}

#[test]
fn test_saving_warns_about_missing_glyphs() {
    let buf = Arc::new(Mutex::new(Vec::<u8>::new()));
    let subscriber = tracing_subscriber::fmt()
        .with_writer(BufWriter(buf.clone()))
        .with_max_level(tracing::Level::WARN)
        .without_time()
        .finish();

    tracing::subscriber::with_default(subscriber, || {
        let mut doc = Document::new();
        doc.add_font_from_bytes("Roboto", roboto_bytes()).unwrap();
        let mut page = Page::a4();
        page.text()
            .set_font(Font::Custom("Roboto".to_string()), 12.0)
            .at(50.0, 700.0)
            .write("OK \u{2713}") // ✓ has no glyph in Roboto
            .unwrap();
        doc.add_page(page);
        let _ = doc.to_bytes().expect("save must succeed");
    });

    let logged = String::from_utf8(buf.lock().unwrap().clone()).unwrap();
    assert!(
        logged.contains("Roboto") && logged.contains("U+2713") && logged.contains(".notdef"),
        "save must warn, naming the font and the missing code point. Got:\n{logged}"
    );
}
