//! Issue #448 — the flat-path reading-order option reorders line groups.
//!
//! The isolated ordering primitive (`text::flat_reading_order`) is wired into
//! the flat `.text` path behind an opt-in flag. With the flag OFF the output is
//! byte-identical to before (stream order). With it ON, line groups are permuted
//! into reading order: left column before right, top block before bottom.
//!
//! These tests pin both directions on a fully synthetic PDF whose content
//! stream deliberately draws blocks in the WRONG order, so a passing assertion
//! can only come from the reorder actually running (not from the stream already
//! being correct).

use oxidize_pdf::parser::{ParseOptions, PdfReader};
use oxidize_pdf::text::{ExtractionOptions, TextExtractor};

#[path = "common/synthetic_pdf.rs"]
mod synthetic_pdf;

fn extract_with(content: &str, reading_order: bool) -> String {
    let bytes = synthetic_pdf::build_pdf_with_content_stream(content.as_bytes());
    let doc = PdfReader::new_with_options(std::io::Cursor::new(bytes), ParseOptions::lenient())
        .expect("synthetic PDF must parse")
        .into_document();
    TextExtractor::with_options(ExtractionOptions::default())
        .with_reading_order(reading_order)
        .extract_from_page(&doc, 0)
        .expect("extraction must succeed")
        .text
}

/// Two columns, drawn right-column-first, each line its own `TJ` at its own Y.
const TWO_COLUMNS_RIGHT_FIRST: &str = concat!(
    "BT\n/F1 10 Tf\n",
    "1 0 0 1 350 700 Tm\n[(Right top)] TJ\n",
    "1 0 0 1 350 680 Tm\n[(Right bottom)] TJ\n",
    "1 0 0 1 50 700 Tm\n[(Left top)] TJ\n",
    "1 0 0 1 50 680 Tm\n[(Left bottom)] TJ\n",
    "ET"
);

#[test]
fn flag_off_leaves_stream_order_untouched() {
    let text = extract_with(TWO_COLUMNS_RIGHT_FIRST, false);
    assert_eq!(
        text, "Right top\nRight bottom\nLeft top\nLeft bottom",
        "with the flag off, output must be exactly stream order: {text:?}"
    );
}

#[test]
fn flag_on_reorders_columns_left_before_right() {
    let text = extract_with(TWO_COLUMNS_RIGHT_FIRST, true);
    assert_eq!(
        text, "Left top\nLeft bottom\nRight top\nRight bottom",
        "with the flag on, the left column must come out before the right: {text:?}"
    );
}

/// Two stacked single-column sections, drawn bottom-section-first.
const STACKED_SECTIONS_BOTTOM_FIRST: &str = concat!(
    "BT\n/F1 10 Tf\n",
    "1 0 0 1 50 120 Tm\n[(Bottom one)] TJ\n",
    "1 0 0 1 50 100 Tm\n[(Bottom two)] TJ\n",
    "1 0 0 1 50 700 Tm\n[(Top one)] TJ\n",
    "1 0 0 1 50 680 Tm\n[(Top two)] TJ\n",
    "ET"
);

#[test]
fn flag_on_reorders_sections_top_before_bottom() {
    let text = extract_with(STACKED_SECTIONS_BOTTOM_FIRST, true);
    assert_eq!(
        text, "Top one\nTop two\nBottom one\nBottom two",
        "with the flag on, the top section must come out before the bottom: {text:?}"
    );
}

/// A single column already in reading order: the permutation is the identity,
/// so the flag must not change a single byte (the §5.2 invariant).
const SINGLE_COLUMN_IN_ORDER: &str = concat!(
    "BT\n/F1 10 Tf\n",
    "1 0 0 1 50 700 Tm\n[(First line)] TJ\n",
    "1 0 0 1 50 680 Tm\n[(Second line)] TJ\n",
    "1 0 0 1 50 660 Tm\n[(Third line)] TJ\n",
    "ET"
);

#[test]
fn identity_permutation_is_byte_identical() {
    let on = extract_with(SINGLE_COLUMN_IN_ORDER, true);
    let off = extract_with(SINGLE_COLUMN_IN_ORDER, false);
    assert_eq!(on, off, "identity permutation must be byte-identical");
    assert_eq!(on, "First line\nSecond line\nThird line");
}
