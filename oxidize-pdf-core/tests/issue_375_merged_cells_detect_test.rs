//! Issue #375 Task 7: detect merged cells from a drawn grid by retaining which
//! divider line-segments are actually present.
//!
//! Fixture: a bordered 2-column table whose TOP row omits the middle vertical
//! divider (a single spanning header cell) while the BOTTOM row keeps it (two
//! cells). Detection must yield a top-left cell with `col_span == 2` and two
//! `col_span == 1` cells in the bottom row.

use oxidize_pdf::graphics::extraction::{ExtractedGraphics, VectorLine};
use oxidize_pdf::text::extraction::TextFragment;
use oxidize_pdf::text::table_detection::TableDetector;

/// Horizontal line at height `y` from `x1` to `x2`.
fn hline(x1: f64, x2: f64, y: f64) -> VectorLine {
    VectorLine::new(x1, y, x2, y, 1.0, true, None)
}

/// Vertical line at `x` from `y1` to `y2`.
fn vline(x: f64, y1: f64, y2: f64) -> VectorLine {
    VectorLine::new(x, y1, x, y2, 1.0, true, None)
}

fn frag(t: &str, x: f64, y: f64) -> TextFragment {
    TextFragment::new(t, x, y, 30.0, 10.0, 10.0)
}

fn build_graphics(h: Vec<VectorLine>, v: Vec<VectorLine>) -> ExtractedGraphics {
    let mut g = ExtractedGraphics::new();
    for line in h.into_iter().chain(v) {
        g.add_line(line);
    }
    g
}

#[test]
fn merged_header_cell_detected_with_col_span_2() {
    // Grid X at 100,200,300 ; Y at 100 (bottom),150 (mid),200 (top).
    // Row 0 (top band, y 150..200) has NO middle vertical => one spanning cell.
    // Row 1 (y 100..150) HAS the middle vertical => two cells.
    let h = vec![
        hline(100.0, 300.0, 100.0),
        hline(100.0, 300.0, 150.0),
        hline(100.0, 300.0, 200.0),
    ];
    let v = vec![
        vline(100.0, 100.0, 200.0), // left border (full height)
        vline(300.0, 100.0, 200.0), // right border (full height)
        vline(200.0, 100.0, 150.0), // MIDDLE divider only in row 1
    ];
    let graphics = build_graphics(h, v);
    let frags = vec![
        frag("Header", 150.0, 170.0),
        frag("A", 130.0, 120.0),
        frag("B", 230.0, 120.0),
    ];

    let det = TableDetector::default();
    let tables = det.detect(&graphics, &frags).expect("detect");
    let table = tables.first().expect("one table");

    // Base grid dimensions stay 2x2.
    assert_eq!(table.rows, 2, "base grid rows");
    assert_eq!(table.columns, 2, "base grid columns");

    let top_left = table
        .cells
        .iter()
        .find(|c| c.row == 0 && c.column == 0)
        .expect("cell 0,0");
    assert_eq!(top_left.col_span, 2, "merged header should span 2 columns");
    assert_eq!(top_left.row_span, 1, "merged header spans a single row");

    // The merged header must NOT leave a separate cell at (0,1).
    assert!(
        !table.cells.iter().any(|c| c.row == 0 && c.column == 1),
        "interior position of a merged cell must be omitted"
    );

    // The non-merged bottom row keeps two single cells.
    assert!(
        table
            .cells
            .iter()
            .any(|c| c.row == 1 && c.column == 0 && c.col_span == 1 && c.row_span == 1),
        "bottom-left single cell"
    );
    assert!(
        table
            .cells
            .iter()
            .any(|c| c.row == 1 && c.column == 1 && c.col_span == 1 && c.row_span == 1),
        "bottom-right single cell"
    );
}

/// Same fragment as `frag`, but tagged as a table-header structure element.
fn th(t: &str, x: f64, y: f64) -> TextFragment {
    let mut f = frag(t, x, y);
    f.struct_tag = Some("TH".to_string());
    f
}

/// Issue #375 Task 8: a fully-ruled 2x2 table whose top row is tagged `TH`
/// must report `header_rows == 1`; the untagged bottom row must not count.
#[test]
fn header_row_detected_from_th_tag() {
    let h = vec![
        hline(100.0, 300.0, 100.0),
        hline(100.0, 300.0, 150.0),
        hline(100.0, 300.0, 200.0),
    ];
    let v = vec![
        vline(100.0, 100.0, 200.0),
        vline(200.0, 100.0, 200.0),
        vline(300.0, 100.0, 200.0),
    ];
    let graphics = build_graphics(h, v);
    let frags = vec![
        th("H1", 130.0, 170.0),
        th("H2", 230.0, 170.0),
        frag("a", 130.0, 120.0),
        frag("b", 230.0, 120.0),
    ];

    let tables = TableDetector::default()
        .detect(&graphics, &frags)
        .expect("detect");
    let table = tables.first().expect("one table");
    assert_eq!(table.header_rows, 1, "top row tagged TH must be the header");
}

/// Issue #375 Task 8: with no structure tags at all, a >=2-row bordered
/// table still defaults `header_rows` to 1 (top row fallback).
#[test]
fn header_row_defaults_to_top_row_without_tags() {
    let h = vec![
        hline(100.0, 300.0, 100.0),
        hline(100.0, 300.0, 150.0),
        hline(100.0, 300.0, 200.0),
    ];
    let v = vec![
        vline(100.0, 100.0, 200.0),
        vline(200.0, 100.0, 200.0),
        vline(300.0, 100.0, 200.0),
    ];
    let graphics = build_graphics(h, v);
    let frags = vec![
        frag("H1", 130.0, 170.0),
        frag("H2", 230.0, 170.0),
        frag("a", 130.0, 120.0),
        frag("b", 230.0, 120.0),
    ];

    let tables = TableDetector::default()
        .detect(&graphics, &frags)
        .expect("detect");
    let table = tables.first().expect("one table");
    assert_eq!(
        table.header_rows, 1,
        "no tags present: must default to top-row fallback"
    );
}

/// Issue #375 Task 8: a fully-ruled 3x2 table with the TOP TWO rows tagged
/// `TH` must report `header_rows == 2`. This is discriminating from the
/// untagged `rows >= 2 -> 1` fallback: a broken tag-matching path can only
/// ever produce 1 here, never 2.
#[test]
fn header_rows_two_when_top_two_of_three_rows_tagged() {
    let h = vec![
        hline(100.0, 300.0, 100.0),
        hline(100.0, 300.0, 140.0),
        hline(100.0, 300.0, 180.0),
        hline(100.0, 300.0, 220.0),
    ];
    let v = vec![
        vline(100.0, 100.0, 220.0),
        vline(200.0, 100.0, 220.0),
        vline(300.0, 100.0, 220.0),
    ];
    let graphics = build_graphics(h, v);
    let frags = vec![
        // Top row (row 0), tagged.
        th("H1", 130.0, 200.0),
        th("H2", 230.0, 200.0),
        // Middle row (row 1), tagged.
        th("H3", 130.0, 160.0),
        th("H4", 230.0, 160.0),
        // Bottom row (row 2), plain.
        frag("a", 130.0, 120.0),
        frag("b", 230.0, 120.0),
    ];

    let tables = TableDetector::default()
        .detect(&graphics, &frags)
        .expect("detect");
    let table = tables.first().expect("one table");
    assert_eq!(table.rows, 3, "base grid rows");
    assert_eq!(
        table.header_rows, 2,
        "top two tagged rows must both count as header rows"
    );
}

/// Issue #375 Task 7 (final-review I2/T7d): vertical merge coverage.
///
/// Grid: X gridlines 100/200/300 (2 cols); Y gridlines 100/150/200 (2 row
/// bands, row 0 = top). The middle horizontal divider at y=150 is drawn only
/// over the RIGHT column (x 200..300); it is absent over the LEFT column
/// (x 100..200). `divider_present_horizontal` therefore reports the divider
/// missing on the left, so the two left-column base cells merge vertically
/// into one `row_span == 2` cell, while the right column — where the divider
/// is present — keeps two separate `row_span == 1` cells.
#[test]
fn merged_cell_detected_with_row_span_2() {
    let h = vec![
        hline(100.0, 300.0, 100.0), // bottom border
        hline(200.0, 300.0, 150.0), // middle divider, RIGHT column only
        hline(100.0, 300.0, 200.0), // top border
    ];
    let v = vec![
        vline(100.0, 100.0, 200.0), // left border (full height)
        vline(200.0, 100.0, 200.0), // middle vertical (full height, both rows)
        vline(300.0, 100.0, 200.0), // right border (full height)
    ];
    let graphics = build_graphics(h, v);
    let frags = vec![
        frag("Left", 130.0, 150.0),
        frag("TopRight", 230.0, 170.0),
        frag("BottomRight", 230.0, 120.0),
    ];

    let det = TableDetector::default();
    let tables = det.detect(&graphics, &frags).expect("detect");
    let table = tables.first().expect("one table");

    // Base grid dimensions stay 2x2.
    assert_eq!(table.rows, 2, "base grid rows");
    assert_eq!(table.columns, 2, "base grid columns");

    let left = table
        .cells
        .iter()
        .find(|c| c.row == 0 && c.column == 0)
        .expect("cell 0,0");
    assert_eq!(left.row_span, 2, "left column should span 2 rows");
    assert_eq!(left.col_span, 1, "left merged cell spans a single column");

    // The vertical merge must not leave a separate cell at (1,0).
    assert!(
        !table.cells.iter().any(|c| c.row == 1 && c.column == 0),
        "interior position of a vertically merged cell must be omitted"
    );

    // The right column keeps two single (unmerged) cells.
    assert!(
        table
            .cells
            .iter()
            .any(|c| c.row == 0 && c.column == 1 && c.row_span == 1 && c.col_span == 1),
        "top-right single cell"
    );
    assert!(
        table
            .cells
            .iter()
            .any(|c| c.row == 1 && c.column == 1 && c.row_span == 1 && c.col_span == 1),
        "bottom-right single cell"
    );
}

/// Issue #375 Task 7 (final-review I2/T7d): transitive multi-cell merge
/// coverage.
///
/// Grid: X gridlines 100/200/300/400 (3 cols); Y gridlines 100/150/200 (2 row
/// bands). In the TOP row band (y 150..200) BOTH interior verticals (x=200
/// and x=300) are absent — each is drawn only over the BOTTOM band
/// (y 100..150) — so all three top-row base cells merge transitively
/// (0-1 absent, 1-2 absent -> union-find connects all three) into one
/// `col_span == 3` cell. The bottom row keeps both interior verticals, so it
/// stays as three separate `col_span == 1` cells.
#[test]
fn transitive_merge_across_three_columns_detected() {
    let h = vec![
        hline(100.0, 400.0, 100.0), // bottom border
        hline(100.0, 400.0, 150.0), // middle divider, full width
        hline(100.0, 400.0, 200.0), // top border
    ];
    let v = vec![
        vline(100.0, 100.0, 200.0), // left border (full height)
        vline(400.0, 100.0, 200.0), // right border (full height)
        vline(200.0, 100.0, 150.0), // interior divider 1, BOTTOM band only
        vline(300.0, 100.0, 150.0), // interior divider 2, BOTTOM band only
    ];
    let graphics = build_graphics(h, v);
    let frags = vec![
        frag("Top", 250.0, 170.0),
        frag("A", 130.0, 120.0),
        frag("B", 230.0, 120.0),
        frag("C", 330.0, 120.0),
    ];

    let det = TableDetector::default();
    let tables = det.detect(&graphics, &frags).expect("detect");
    let table = tables.first().expect("one table");

    // Base grid dimensions stay 2x3.
    assert_eq!(table.rows, 2, "base grid rows");
    assert_eq!(table.columns, 3, "base grid columns");

    let top = table
        .cells
        .iter()
        .find(|c| c.row == 0 && c.column == 0)
        .expect("cell 0,0");
    assert_eq!(
        top.col_span, 3,
        "top row should transitively merge 3 columns"
    );
    assert_eq!(top.row_span, 1, "top merged cell spans a single row");

    // The transitive merge must not leave separate cells at (0,1) or (0,2).
    assert!(
        !table.cells.iter().any(|c| c.row == 0 && c.column == 1),
        "interior position (0,1) of the merged cell must be omitted"
    );
    assert!(
        !table.cells.iter().any(|c| c.row == 0 && c.column == 2),
        "interior position (0,2) of the merged cell must be omitted"
    );

    // The bottom row keeps three single (unmerged) cells.
    for col in 0..3 {
        assert!(
            table
                .cells
                .iter()
                .any(|c| { c.row == 1 && c.column == col && c.row_span == 1 && c.col_span == 1 }),
            "bottom row cell at column {col} should remain unmerged"
        );
    }
}
