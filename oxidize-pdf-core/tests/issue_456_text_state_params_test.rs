//! Issue #456 — `Tc` (char spacing), `Tw` (word spacing) and `Ts` (text rise)
//! were parsed and stored but never read, so they had no effect on extraction
//! (ISO 32000-1 §9.3.2, §9.3.3, §9.4.4).
//!
//! The glyph-displacement formula is, per §9.4.4:
//!
//! ```text
//! tx = ((w0/1000 - Tj/1000) * Tfs + Tc + Tw) * Th
//! ```
//!
//! `Tc` is added once per glyph shown, `Tw` once per *single-byte* space
//! (code 32, §9.3.3), both in unscaled text-space units and both then scaled by
//! `Th`. `Ts` shifts the glyph origin up the y-axis by an unscaled text-space
//! amount before the text/CTM transform.
//!
//! Oracle: fragment coordinates under `preserve_layout` — the only extraction
//! mode that exposes the pen position, and therefore the only place the pen
//! advance and the baseline offset are observable. Each test measures the shift
//! of a *following* run between spacing = 0 and spacing = k, which cancels the
//! (font-dependent) base glyph widths and isolates the parameter under test.
//!
//! Scope note: these apply to `TextExtractor` only. `PlainTextExtractor` decides
//! separators from the explicit pen positions the producer writes (Td/TD/TJ),
//! never from accumulated glyph advances (see its `_char_space`/`_word_space`
//! fields), so `Tc`/`Tw` are correctly unobservable there.

#[path = "common/mod.rs"]
mod common;
use common::synthetic_pdf::build_pdf_with_content_stream;

use oxidize_pdf::parser::{PdfDocument, PdfReader};
use oxidize_pdf::text::{ExtractionOptions, TextExtractor};
use std::io::Cursor;

/// Extract every fragment under `preserve_layout`.
fn fragments(content: &[u8]) -> Vec<oxidize_pdf::text::TextFragment> {
    let pdf = build_pdf_with_content_stream(content);
    let reader = PdfReader::new(Cursor::new(pdf)).expect("synthetic PDF must parse");
    let document = PdfDocument::new(reader);
    let options = ExtractionOptions {
        preserve_layout: true,
        ..Default::default()
    };
    let mut extractor = TextExtractor::with_options(options);
    extractor
        .extract_from_page(&document, 0)
        .expect("extract page 0")
        .fragments
}

/// `(x, y)` of the first fragment whose trimmed text equals `needle`.
fn frag_xy(content: &[u8], needle: &str) -> (f64, f64) {
    fragments(content)
        .into_iter()
        .find(|f| f.text.trim() == needle)
        .map(|f| (f.x, f.y))
        .unwrap_or_else(|| panic!("no fragment with text {needle:?}"))
}

/// Width of the first fragment whose trimmed text equals `needle`. The fragment
/// width is the run's pen advance (`text_width`, scaled by the CTM x-scale,
/// which is 1 here), the same quantity that drives the flat path's separator
/// deltas — so it is the faithful oracle for `Tc`/`Tw` in the advance.
fn frag_width(content: &[u8], needle: &str) -> f64 {
    fragments(content)
        .into_iter()
        .find(|f| f.text.trim() == needle)
        .map(|f| f.width)
        .unwrap_or_else(|| panic!("no fragment with text {needle:?}"))
}

const EPS: f64 = 1e-6;

/// `Tc` advances the pen once per glyph shown. "ABCD" is four glyphs, so `Tc = 5`
/// must widen its advance by `4 * 5 = 20` relative to `Tc = 0`. The base glyph
/// widths cancel between the two runs.
#[test]
fn char_spacing_advances_pen_once_per_glyph() {
    let base = b"BT\n/F1 12 Tf\n100 700 Td\n(ABCD)Tj\nET\n";
    let with_tc = b"BT\n/F1 12 Tf\n5 Tc\n100 700 Td\n(ABCD)Tj\nET\n";
    let delta = frag_width(with_tc, "ABCD") - frag_width(base, "ABCD");
    assert!(
        (delta - 20.0).abs() < EPS,
        "Tc=5 over 4 glyphs must add 20.0 of advance; got {delta}"
    );
}

/// `Tw` advances the pen once per single-byte space. The run "A A A" holds two
/// spaces, so `Tw = 7` must widen its advance by `2 * 7 = 14`.
#[test]
fn word_spacing_advances_pen_once_per_space() {
    let base = b"BT\n/F1 12 Tf\n100 700 Td\n(A A A)Tj\nET\n";
    let with_tw = b"BT\n/F1 12 Tf\n7 Tw\n100 700 Td\n(A A A)Tj\nET\n";
    let delta = frag_width(with_tw, "A A A") - frag_width(base, "A A A");
    assert!(
        (delta - 14.0).abs() < EPS,
        "Tw=7 over 2 spaces must add 14.0 of advance; got {delta}"
    );
}

/// `Tw` applies to spaces only. A run with no spaces must be unaffected by it —
/// this is what separates `Tw` from `Tc` (§9.3.3).
#[test]
fn word_spacing_does_not_advance_on_non_space_glyphs() {
    let base = b"BT\n/F1 12 Tf\n100 700 Td\n(ABCD)Tj\nET\n";
    let with_tw = b"BT\n/F1 12 Tf\n50 Tw\n100 700 Td\n(ABCD)Tj\nET\n";
    let delta = frag_width(with_tw, "ABCD") - frag_width(base, "ABCD");
    assert!(
        delta.abs() < EPS,
        "Tw must not change the advance of a run without spaces; got {delta}"
    );
}

/// `Tc` must also fold into the `TJ` (`ShowTextArray`) advance, not just `Tj` —
/// real documents draw almost everything through `TJ`.
#[test]
fn char_spacing_advances_pen_in_show_text_array() {
    let base = b"BT\n/F1 12 Tf\n100 700 Td\n[(ABCD)]TJ\nET\n";
    let with_tc = b"BT\n/F1 12 Tf\n5 Tc\n100 700 Td\n[(ABCD)]TJ\nET\n";
    let delta = frag_width(with_tc, "ABCD") - frag_width(base, "ABCD");
    assert!(
        (delta - 20.0).abs() < EPS,
        "Tc=5 over 4 glyphs must add 20.0 of advance through TJ; got {delta}"
    );
}

/// `Ts` (text rise) offsets the glyph baseline up the y-axis by an unscaled
/// text-space amount. With an axis-aligned matrix the user-space y shifts by
/// exactly `Ts`.
#[test]
fn text_rise_offsets_the_baseline_y() {
    let base = b"BT\n/F1 12 Tf\n100 700 Td\n(A)Tj\nET\n";
    let with_ts = b"BT\n/F1 12 Tf\n5 Ts\n100 700 Td\n(A)Tj\nET\n";
    let delta = frag_xy(with_ts, "A").1 - frag_xy(base, "A").1;
    assert!(
        (delta - 5.0).abs() < EPS,
        "Ts=5 must raise the baseline by 5.0; got {delta}"
    );
}
