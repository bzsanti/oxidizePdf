//! Tests for issue #265: adjacent visual rows with tight baseline gaps must not
//! be collapsed into a single logical line.
//!
//! When two rows share an almost-identical Y (typical of table cells or
//! tightly-spaced layouts), the previous Y-only tolerance in `merge_into_lines`
//! grouped them together and the subsequent build step concatenated their
//! contents by insertion order. The desired behaviour: distinct visual rows
//! must yield distinct logical lines / paragraphs even when their baselines
//! are within the historical `0.5 * height` tolerance, provided a structural
//! signal (X-position reset back toward the left margin) identifies them as
//! distinct rows.

use oxidize_pdf::text::{ExtractionOptions, TextExtractor, TextFragment};

fn frag(text: &str, x: f64, y: f64, width: f64, font_size: f64) -> TextFragment {
    TextFragment {
        text: text.to_string(),
        x,
        y,
        width,
        height: font_size,
        font_size,
        font_name: Some("Helvetica".to_string()),
        is_bold: false,
        is_italic: false,
        color: None,
        space_decisions: Vec::new(),
        mcid: None,
        struct_tag: None,
    }
}

#[test]
fn two_rows_with_2_5pt_baseline_gap_stay_separate() {
    // Reproduces the exact synthetic case from the issue acceptance criteria:
    // y_a = 400, y_b = 397.5 (gap = 2.5pt), both 9pt font. With the historical
    // tolerance of `0.5 * min(h)` = 4.5pt the two rows collapsed; the X-reset
    // post-pass must now split them.
    let opts = ExtractionOptions {
        reconstruct_paragraphs: true,
        ..ExtractionOptions::default()
    };
    let extractor = TextExtractor::with_options(opts);
    let input = vec![
        // Row-interleaved order, as a table renderer emits cell-by-cell
        frag("Row A first", 50.0, 400.0, 44.0, 9.0),
        frag("Row B first", 50.0, 397.5, 44.0, 9.0),
        frag("Row A second", 100.0, 400.0, 50.0, 9.0),
        frag("Row B second", 100.0, 397.5, 50.0, 9.0),
    ];

    let merged = extractor.merge_fragments_for_partition(&input);

    let texts: Vec<&String> = merged.iter().map(|f| &f.text).collect();
    assert_eq!(
        merged.len(),
        2,
        "two distinct visual rows must produce two paragraphs, got {} (texts: {:?})",
        merged.len(),
        texts
    );

    let a = &merged[0].text;
    let b = &merged[1].text;
    assert!(
        a.contains("Row A first") && a.contains("Row A second") && !a.contains("Row B"),
        "first paragraph must contain row A in order, no row B mixed: {:?}",
        a
    );
    assert!(
        b.contains("Row B first") && b.contains("Row B second") && !b.contains("Row A"),
        "second paragraph must contain row B in order, no row A mixed: {:?}",
        b
    );
}

#[test]
fn x_reset_within_y_tolerance_splits_into_two_lines() {
    // Y gap of 2.0pt with 9pt font (within historical 4.5pt tol). The structural
    // signal here is the X-reset: row B's first fragment is at x=50, well to the
    // left of row A's right edge at x≈232. The post-pass must detect that reset
    // and split.
    let opts = ExtractionOptions {
        reconstruct_paragraphs: true,
        ..ExtractionOptions::default()
    };
    let extractor = TextExtractor::with_options(opts);
    let input = vec![
        frag("Left A", 50.0, 400.0, 28.0, 9.0),
        frag("Right A", 200.0, 400.0, 32.0, 9.0),
        frag("Left B", 50.0, 398.0, 28.0, 9.0),
        frag("Right B", 200.0, 398.0, 32.0, 9.0),
    ];

    let merged = extractor.merge_fragments_for_partition(&input);

    let texts: Vec<&String> = merged.iter().map(|f| &f.text).collect();
    assert!(
        merged.iter().any(|f| f.text.contains("Left A")
            && f.text.contains("Right A")
            && !f.text.contains("Left B")
            && !f.text.contains("Right B")),
        "row A must stay coherent without B mixed: {:?}",
        texts
    );
    assert!(
        merged.iter().any(|f| f.text.contains("Left B")
            && f.text.contains("Right B")
            && !f.text.contains("Left A")
            && !f.text.contains("Right A")),
        "row B must stay coherent without A mixed: {:?}",
        texts
    );
}

#[test]
fn single_row_with_baseline_jitter_stays_one_line() {
    // Regression guard: the X-reset post-pass must NOT split a single visual
    // row whose fragments have tiny baseline jitter (sub-pixel from text-matrix
    // arithmetic) and monotonically increasing X.
    let opts = ExtractionOptions {
        reconstruct_paragraphs: true,
        ..ExtractionOptions::default()
    };
    let extractor = TextExtractor::with_options(opts);
    let input = vec![
        frag("Row", 50.0, 400.0, 18.0, 9.0),
        frag("with", 70.0, 400.1, 22.0, 9.0),
        frag("jitter", 95.0, 399.9, 30.0, 9.0),
        frag("here.", 130.0, 400.05, 28.0, 9.0),
    ];

    let merged = extractor.merge_fragments_for_partition(&input);

    let texts: Vec<&String> = merged.iter().map(|f| &f.text).collect();
    assert_eq!(
        merged.len(),
        1,
        "jittered same-row fragments must form a single line/paragraph, got {} ({:?})",
        merged.len(),
        texts
    );
    let t = &merged[0].text;
    assert!(
        t.contains("Row") && t.contains("with") && t.contains("jitter") && t.contains("here."),
        "all fragments present in order: {:?}",
        t
    );
}

#[test]
fn three_rows_collapsed_by_y_tolerance_all_split_by_x_reset() {
    // Pathological case: three rows packed tight (gap 2pt each, 9pt font).
    // All three would collapse into one "line" by the Y-tolerance alone.
    // The X-reset must trigger twice — at each row boundary — yielding three
    // distinct paragraphs.
    let opts = ExtractionOptions {
        reconstruct_paragraphs: true,
        ..ExtractionOptions::default()
    };
    let extractor = TextExtractor::with_options(opts);
    let input = vec![
        frag("First row only", 50.0, 400.0, 70.0, 9.0),
        frag("Second row only", 50.0, 398.0, 75.0, 9.0),
        frag("Third row only", 50.0, 396.0, 72.0, 9.0),
    ];

    let merged = extractor.merge_fragments_for_partition(&input);

    let texts: Vec<&String> = merged.iter().map(|f| &f.text).collect();
    assert_eq!(
        merged.len(),
        3,
        "three rows packed within Y-tolerance must split via X-reset, got {} ({:?})",
        merged.len(),
        texts
    );
    assert!(merged[0].text.contains("First row only"), "{:?}", texts);
    assert!(merged[1].text.contains("Second row only"), "{:?}", texts);
    assert!(merged[2].text.contains("Third row only"), "{:?}", texts);
}
