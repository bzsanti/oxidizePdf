//! Issue #458 — a horizontal jump between two `TJ` operators on the same line
//! must produce a space in the extracted text.
//!
//! The `Tj` arm of the extractor turns a forward pen jump wider than
//! `space_threshold × font size` into a `U+0020`. The `TJ` arm computes the
//! same delta and used it only to decide newlines, so two `TJ` operators drawn
//! side by side on one line came out glued together.
//!
//! Two independent reports hit the same mechanism:
//!
//! - Issue #458: a multi-column contact-info table (three `"Tel: …"` phone
//!   numbers on one row, each drawn as its own `TJ` at a distinct X, identical
//!   Y) extracted as `Tel: …Tel: …Tel: …`, destroying every phone-number match
//!   past the first cell.
//! - The #448 reading-order probe on `preserve_027613.pdf` (an IBM manual, 429
//!   pages): every bulleted item is drawn as one `TJ` for the bullet glyph and
//!   another for the item text, 7.48pt apart at 10pt type (0.75 em, well past
//!   the 0.3 em threshold). The extractor emitted `vlarge`, `vquery`,
//!   `vmaintain` — the bullet welded onto the first word. Poppler splits them.
//!
//! The tests below pin both directions: the boundary between two `TJ`
//! operators must break, and kerning INSIDE one `TJ` array must not, because
//! that is how a single word is drawn as several positioned pieces.

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

/// The issue #458 case: three table cells on one row, each a separate `TJ` at
/// its own X and the same Y (a wide, unambiguous forward gap). Placeholder text
/// stands in for the confidential phone numbers of the original report; the
/// mechanism — separate `TJ` calls, same Y, forward X gap, no separator — is
/// identical.
const MULTI_COLUMN_TABLE_ROW: &str = concat!(
    "BT\n/F1 10 Tf\n",
    "1 0 0 1 218.27 554.25 Tm\n[(CellOne)] TJ\n",
    "1 0 0 1 286.65 554.25 Tm\n[(CellTwo)] TJ\n",
    "1 0 0 1 352.10 554.25 Tm\n[(CellThree)] TJ\n",
    "ET"
);

#[test]
fn issue_458_multi_column_table_cells_are_not_fused() {
    let text = extract(MULTI_COLUMN_TABLE_ROW);
    assert!(
        !text.contains("CellOneCellTwo"),
        "adjacent table cells were welded together: {text:?}"
    );
    assert!(
        text.contains("CellOne CellTwo CellThree"),
        "three cells drawn as three TJ calls on one row must read as three \
         separate words: {text:?}"
    );
}

/// The #448 bullet case, in pen-gap terms.
///
/// There the bullet is a 3.31pt glyph of a symbol font at x=113.75 and the item
/// text starts at x=124.54, both at 10pt: a pen gap of 7.48pt, 0.748 em. The
/// fixture cannot reuse those absolute coordinates, because its `/F1` is
/// Helvetica, whose `v` advances further and would leave a smaller gap than the
/// document has. What matters to the rule is the GAP, so the second run is
/// placed to reproduce it — and it lands just above `TJ_BOUNDARY_SPACE_EM`,
/// which is the narrowest verified word gap the constant is chosen to cover.
fn bullet_then_text(pen_gap_pt: f64) -> String {
    let bullet_advance = 5.0; // `v` in Helvetica: 500/1000 em at 10pt
    let x = 113.75;
    let text_x = x + bullet_advance + pen_gap_pt;
    format!(
        "BT\n/F1 10 Tf\n1 0 0 1 {x} 593.37 Tm\n[(v)] TJ\n\
         1 0 0 1 {text_x} 593.37 Tm\n[(large scale retrieval)] TJ\nET"
    )
}

#[test]
fn a_gap_between_two_tj_operators_on_one_line_becomes_a_space() {
    let text = extract(&bullet_then_text(7.48));
    assert!(
        !text.contains("vlarge"),
        "the bullet was welded onto the first word of the item: {text:?}"
    );
    assert!(
        text.contains("v large scale retrieval"),
        "a 0.75 em gap between two TJ operators must read as a word break: {text:?}"
    );
}

/// The same page draws body text as several `TJ` operators separated by
/// hairline gaps — `(Cr)` then `(eate new or select)` — because the producer
/// repositions mid-word. Those must NOT break, or the fix would split every
/// word it touches.
const WORD_SPLIT_ACROSS_TJ_OPERATORS: &str = concat!(
    "BT\n/F1 10 Tf\n",
    "1 0 0 1 124.54 593.37 Tm\n[(Cr)] TJ\n",
    "1 0 0 1 135.38 593.37 Tm\n[(eate new forms)] TJ\n",
    "ET"
);

#[test]
fn a_hairline_gap_between_two_tj_operators_does_not_break_the_word() {
    let text = extract(WORD_SPLIT_ACROSS_TJ_OPERATORS);
    assert!(
        text.contains("Create new forms"),
        "a sub-threshold reposition inside a word must stay welded: {text:?}"
    );
}

/// Kerning inside ONE `TJ` array is governed by the `Spacing` elements, which
/// synthesise their own space past `tj_space_threshold`. The boundary rule must
/// not fire there as well, or a kerned word gains a space it never had.
const KERNED_WORD_IN_ONE_ARRAY: &str = concat!(
    "BT\n/F1 10 Tf\n",
    "1 0 0 1 100 700 Tm\n[(Cr) -40 (eate) -40 (test) -300 (data)] TJ\n",
    "ET"
);

#[test]
fn kerning_inside_one_tj_array_is_left_to_the_spacing_rule() {
    let text = extract(KERNED_WORD_IN_ONE_ARRAY);
    assert!(
        text.contains("Createtest data") || text.contains("Create test data"),
        "tight kerns must not become spaces and the wide one must: {text:?}"
    );
    assert!(
        !text.contains("Cr eate"),
        "a 40/1000 em kern is not a word break: {text:?}"
    );
    assert!(
        !text.contains("test  data"),
        "the wide kern must produce exactly one space, not two: {text:?}"
    );
}

/// A `TJ` array whose FIRST element is a kern: the pen jump the boundary rule
/// would see is that kern, which the `Spacing` rule already owns. Applying both
/// would double the space.
const ARRAY_STARTING_WITH_A_KERN: &str = concat!(
    "BT\n/F1 10 Tf\n",
    "1 0 0 1 100 700 Tm\n[(alpha)] TJ\n",
    "[-400 (beta)] TJ\n",
    "ET"
);

/// Both sides of the calibrated threshold, pinned at the unit level so a change
/// to the constant cannot pass unnoticed. The corpus sweep behind the value is
/// in the doc comment of `TJ_BOUNDARY_SPACE_EM`; what these two tests add is
/// that the rule has teeth in both directions rather than firing everywhere or
/// nowhere.
fn two_runs_with_gap(gap_em: f64) -> String {
    let size = 10.0;
    // `alpha` in 10pt Helvetica advances 24.45pt (widths 556+278+556+278+500 of
    // 1000/em). The second run starts that far along, plus the requested gap.
    let start = 100.0;
    let advance = 24.45;
    let second = start + advance + gap_em * size;
    format!(
        "BT\n/F1 {size} Tf\n1 0 0 1 {start} 700 Tm\n[(alpha)] TJ\n\
         1 0 0 1 {second} 700 Tm\n[(beta)] TJ\nET"
    )
}

#[test]
fn a_gap_below_the_threshold_leaves_the_runs_welded() {
    let text = extract(&two_runs_with_gap(0.5));
    assert!(
        text.contains("alphabeta"),
        "half an em is inside the range where a producer just repositions \
         mid-word, so it must not split: {text:?}"
    );
}

#[test]
fn a_gap_above_the_threshold_splits_the_runs() {
    let text = extract(&two_runs_with_gap(0.9));
    assert!(
        text.contains("alpha beta"),
        "nine tenths of an em between two separately positioned runs is a word \
         break: {text:?}"
    );
}

/// The mirror case: an array that ENDS with a kern, followed by another `TJ`
/// that does not reposition. The kern already produced the space and the pen
/// jump the boundary rule sees is that same kern.
const TRAILING_KERN_THEN_NEW_ARRAY: &str = concat!(
    "BT\n/F1 10 Tf\n",
    "1 0 0 1 100 700 Tm\n[(alpha) -400] TJ\n",
    "[(beta)] TJ\n",
    "ET"
);

#[test]
fn a_trailing_kern_followed_by_another_array_produces_one_space_not_two() {
    let text = extract(TRAILING_KERN_THEN_NEW_ARRAY);
    assert!(
        text.contains("alpha beta"),
        "the trailing kern must read as one word break: {text:?}"
    );
    assert!(
        !text.contains("alpha  beta"),
        "the boundary rule counted the kern the spacing rule had already counted: {text:?}"
    );
}

#[test]
fn a_leading_kern_produces_one_space_not_two() {
    let text = extract(ARRAY_STARTING_WITH_A_KERN);
    assert!(
        text.contains("alpha beta"),
        "the leading kern must read as one word break: {text:?}"
    );
    assert!(
        !text.contains("alpha  beta"),
        "the boundary rule and the kern rule both fired: {text:?}"
    );
}

/// A `TJ` that opens with a SMALL intra-word kern (below `tj_space_threshold`,
/// so the kern itself synthesises nothing) while a real `Tm`-driven column gap
/// sits on its first text element. The boundary space must still fire — the
/// leading kern must not consume `at_array_start` and mask the jump, or issue
/// #458 recurs for arrays shaped `Tm … [-N (word)…] TJ`.
fn column_gap_then_array_with_small_leading_kern(gap_em: f64) -> String {
    let size = 10.0;
    let start = 100.0;
    let advance = 24.45; // `alpha` in 10pt Helvetica
    let second = start + advance + gap_em * size;
    format!(
        "BT\n/F1 {size} Tf\n1 0 0 1 {start} 700 Tm\n[(alpha)] TJ\n\
         1 0 0 1 {second} 700 Tm\n[-20 (beta)] TJ\nET"
    )
}

#[test]
fn a_small_leading_kern_does_not_mask_a_real_boundary_gap() {
    let text = extract(&column_gap_then_array_with_small_leading_kern(0.9));
    assert!(
        text.contains("alpha beta"),
        "a 0.2/1000 em leading kern must not suppress the 0.9 em column boundary: {text:?}"
    );
    assert!(
        !text.contains("alpha  beta"),
        "a sub-threshold leading kern must not add a second space: {text:?}"
    );
}
