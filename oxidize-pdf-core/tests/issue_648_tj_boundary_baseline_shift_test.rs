//! Issue #648 — ShowTextArray boundary space threshold (0.7 em) suppresses
//! word separator when baseline shifts across text runs.
//!
//! When two `TJ` operators are drawn on the same line, issue #458 introduced a
//! 0.7 em boundary threshold to avoid fusing table cells while protecting
//! intra-word positioned fragments from being split.
//!
//! However, when a `TJ` operator begins after an explicit pen repositioning
//! (`Td`/`TD`/`Tm`) that shifts the baseline (`dy > vertical_shift_guard`),
//! the text runs are not intra-word fragments of a single horizontal word.
//! In this case, using the 0.7 em threshold erroneously suppressed the word
//! space for gaps between ~0.3 em and 0.7 em. The boundary threshold must drop
//! to `flat_space_gap_threshold`, matching the behavior of `ShowText` (`Tj`).
//!
//! At the same time:
//! 1. Small vertical jitter from text-matrix arithmetic (sub-pixel, <= 0.1 em)
//!    must continue to be treated as same-baseline to avoid splitting intra-word
//!    fragments (#458).
//! 2. Superscripts and subscripts (`dy > 0`, but `dx ≈ 0`) must not have an
//!    artificial space inserted.
//! 3. Same-baseline runs (`dy == 0`) must still respect the calibrated 0.7 em
//!    threshold.

use oxidize_pdf::parser::{ParseOptions, PdfReader};
use oxidize_pdf::text::{ExtractionOptions, TextExtractor};

#[path = "common/synthetic_pdf.rs"]
mod synthetic_pdf;

fn extract(content: &str) -> String {
    let bytes = synthetic_pdf::build_pdf_with_content_stream(content.as_bytes());
    let doc = PdfReader::new_with_options(std::io::Cursor::new(bytes), ParseOptions::lenient())
        .expect("synthetic PDF must parse")
        .into_document();
    TextExtractor::with_options(ExtractionOptions::default())
        .extract_from_page(&doc, 0)
        .expect("extraction must succeed")
        .text
}

/// Helper building two `TJ` runs with specified horizontal gap and vertical baseline delta.
/// `alpha` in 10pt Helvetica advances 24.45pt.
fn two_tj_runs(gap_em: f64, dy_pt: f64) -> String {
    let size = 10.0;
    let start_x = 100.0;
    let start_y = 700.0;
    let advance = 24.45;
    let second_x = start_x + advance + gap_em * size;
    let second_y = start_y - dy_pt;
    format!(
        "BT\n/F1 {size} Tf\n1 0 0 1 {start_x} {start_y} Tm\n[(alpha)] TJ\n\
         1 0 0 1 {second_x} {second_y} Tm\n[(beta)] TJ\nET"
    )
}

#[test]
fn issue_648_baseline_shift_emits_space_at_moderate_gap() {
    // 0.4 em gap (4.0pt at 10pt) with a 2.0pt baseline shift.
    // 0.4 em is below 0.7 em (which would suppress space if same baseline),
    // but above flat_space_gap_threshold (~0.14-0.28 em).
    let text = extract(&two_tj_runs(0.4, 2.0));
    assert!(
        text.contains("alpha beta"),
        "baseline shift across TJ runs must emit a space at 0.4 em gap, got: {text:?}"
    );
}

#[test]
fn issue_648_baseline_shift_via_td_operator() {
    // Same baseline shift and gap, but repositioned via Td instead of Tm.
    // In PDF (ISO 32000 §9.4.2), `tx ty Td` translates relative to the start
    // of the current line (text_line_matrix). Since `alpha` starts at 100.0 and
    // advances 24.45pt, placing `beta` with a 4.0pt forward gap means moving
    // tx = 24.45 + 4.0 = 28.45pt from the line start:
    let content = concat!(
        "BT\n/F1 10 Tf\n",
        "100 700 Td\n[(alpha)] TJ\n",
        "28.45 -2.0 Td\n[(beta)] TJ\n",
        "ET"
    );
    let text = extract(content);
    assert!(
        text.contains("alpha beta"),
        "baseline shift via Td must emit a space at 0.4 em gap, got: {text:?}"
    );
}

#[test]
fn issue_648_table_cells_with_shifted_baselines_are_not_fused() {
    // Adjacent table cells with slightly different baselines (e.g. 1.5pt shift due to cell padding)
    // and a 0.5 em gap.
    let content = concat!(
        "BT\n/F1 10 Tf\n",
        "1 0 0 1 100.0 550.0 Tm\n[(CellOne)] TJ\n",
        "1 0 0 1 145.0 548.5 Tm\n[(CellTwo)] TJ\n",
        "ET"
    );
    let text = extract(content);
    assert!(
        !text.contains("CellOneCellTwo"),
        "table cells with shifted baselines must not be fused: {text:?}"
    );
    assert!(
        text.contains("CellOne CellTwo"),
        "table cells with shifted baselines must read as separate words: {text:?}"
    );
}

#[test]
fn issue_648_subpixel_vertical_jitter_leaves_same_baseline_runs_welded() {
    // Sub-pixel vertical jitter (0.05pt at 10pt type = 0.005 em) with 0.4 em gap.
    // This is within the jitter guard (0.1 * font_size = 1.0pt), so it is treated
    // as same-baseline text: 0.4 em < 0.7 em threshold leaves runs welded.
    let text = extract(&two_tj_runs(0.4, 0.05));
    assert!(
        text.contains("alphabeta"),
        "sub-pixel vertical jitter must not trigger baseline-shift threshold: {text:?}"
    );
}

#[test]
fn issue_648_superscript_with_baseline_shift_does_not_insert_artificial_space() {
    // `x` at 10pt advances 5.0pt. Raised `2` at 703.0 (dy = 3.0pt), starting
    // immediately at x=105.0 (dx = 0.0).
    let content = concat!(
        "BT\n/F1 10 Tf\n",
        "1 0 0 1 100.0 700.0 Tm\n[(x)] TJ\n",
        "1 0 0 1 105.0 703.0 Tm\n[(2)] TJ\n",
        "ET"
    );
    let text = extract(content);
    assert_eq!(
        text.trim(),
        "x2",
        "superscript with zero horizontal gap must not gain an artificial space: {text:?}"
    );
}

#[test]
fn issue_648_superscript_citation_following_word_stays_attached() {
    // `word` in 10pt Helvetica advances ~22.78pt. Citation `1` raised by 3.5pt,
    // immediately adjacent at x=122.78.
    let content = concat!(
        "BT\n/F1 10 Tf\n",
        "1 0 0 1 100.0 700.0 Tm\n[(word)] TJ\n",
        "1 0 0 1 122.78 703.5 Tm\n[(1)] TJ\n",
        "ET"
    );
    let text = extract(content);
    assert_eq!(
        text.trim(),
        "word1",
        "citation superscript adjacent to word must stay attached: {text:?}"
    );
}

#[test]
fn issue_648_same_baseline_runs_retain_conservative_threshold() {
    // At dy == 0, 0.5 em stays welded, 0.9 em splits (issue #458).
    let below = extract(&two_tj_runs(0.5, 0.0));
    assert!(
        below.contains("alphabeta"),
        "same-baseline 0.5 em gap must remain welded: {below:?}"
    );

    let above = extract(&two_tj_runs(0.9, 0.0));
    assert!(
        above.contains("alpha beta"),
        "same-baseline 0.9 em gap must split: {above:?}"
    );
}

#[test]
fn issue_648_large_vertical_shift_still_emits_newline() {
    // dy = 12.0pt > default newline_threshold (10.0pt) must emit newline.
    let text = extract(&two_tj_runs(0.4, 12.0));
    assert_eq!(
        text.trim(),
        "alpha\nbeta",
        "large vertical jump past newline_threshold must produce a newline: {text:?}"
    );
}

#[test]
fn shear_keeps_perpendicular_baseline_threshold() {
    // Both cases have page-space gap 4pt and perpendicular baseline displacement 1.05pt.
    let normal = extract(
        "BT /F1 10 Tf 1 0 0 1 100 100 Tm [(alpha)] TJ 1 0 0 1 128.45 101.05 Tm [(beta)] TJ ET",
    );
    let shear=extract("1 0 0.5 1 0 0 cm BT /F1 10 Tf 1 0 0 1 50 100 Tm [(alpha)] TJ 1 0 0 1 77.925 101.05 Tm [(beta)] TJ ET");
    assert_eq!(normal.trim(), "alpha beta");
    assert_eq!(shear.trim(), "alpha beta");
}
#[test]
fn rotation_preserves_baseline_shift() {
    let text=extract("0 1 -1 0 800 0 cm BT /F1 10 Tf 1 0 0 1 100 100 Tm [(alpha)] TJ 1 0 0 1 128.45 102 Tm [(beta)] TJ ET");
    assert_eq!(text.trim(), "alpha beta");
}
#[test]
fn vertical_scale_preserves_baseline_shift() {
    let text=extract("1 0 0 2 0 0 cm BT /F1 10 Tf 1 0 0 1 100 100 Tm [(alpha)] TJ 1 0 0 1 128.45 102 Tm [(beta)] TJ ET");
    assert_eq!(text.trim(), "alpha beta");
}
