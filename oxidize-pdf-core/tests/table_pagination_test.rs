//! Integration tests for `Table` vertical-overflow handling — issue #218.
//!
//! Three layers under test:
//!   1. `Table::render_with_split(&mut GraphicsContext, bottom_y)` — primitive
//!   2. `Table::render_strict(&mut GraphicsContext, bottom_y)` — pre-flight Err
//!   3. `Document::add_paginated_table(...)` — batteries-included
//!
//! Tests verify content-stream output, never just absence of crash.

use oxidize_pdf::error::PdfError;
use oxidize_pdf::page_tables::{DocumentTables, PageTables};
use oxidize_pdf::text::{Table, TableOptions};
use oxidize_pdf::{Document, Page};

// ---------- helpers ----------

fn fixed_height_table(num_rows: usize, row_height: f64) -> Table {
    let mut table = Table::with_equal_columns(2, 200.0);
    let options = TableOptions {
        row_height,
        ..TableOptions::default()
    };
    table.set_options(options);
    for i in 0..num_rows {
        table
            .add_row(vec![format!("r{i}c0"), format!("r{i}c1")])
            .unwrap();
    }
    table
}

fn fixed_height_table_with_header(num_data_rows: usize, row_height: f64) -> Table {
    let mut table = Table::with_equal_columns(2, 200.0);
    let options = TableOptions {
        row_height,
        ..TableOptions::default()
    };
    table.set_options(options);
    table
        .add_header_row(vec!["H0".to_string(), "H1".to_string()])
        .unwrap();
    for i in 0..num_data_rows {
        table
            .add_row(vec![format!("r{i}c0"), format!("r{i}c1")])
            .unwrap();
    }
    table
}

fn count_tj(content: &str) -> usize {
    content.matches(") Tj\n").count() + content.matches("> Tj\n").count()
}

// ---------- tests ----------

#[test]
fn render_with_split_returns_tail_when_rows_overflow_bottom_y() {
    // 10 rows × 30pt = 300pt total. position.y = 800, bottom_y = 650.
    // Vertical room above floor = 150pt → exactly 5 rows fit, 5 are deferred.
    let mut table = fixed_height_table(10, 30.0);
    table.set_position(50.0, 800.0);

    let mut page = Page::a4();
    let result = table
        .render_with_split(page.graphics(), 650.0)
        .expect("render_with_split must succeed");

    let tail = result.expect("tail must be Some when rows overflow");
    assert_eq!(
        tail.row_count(),
        5,
        "tail must contain exactly the 5 unrendered rows"
    );

    let ops = page.graphics().get_operations();
    // Each row has 2 cells → 2 `Tj` per row × 5 rendered rows = 10
    assert_eq!(
        count_tj(&ops),
        10,
        "exactly 10 Tj operators expected (5 rendered rows × 2 cells), got: {}",
        count_tj(&ops)
    );

    for i in 5..10 {
        let needle = format!("r{i}c0");
        assert!(
            !ops.contains(&needle),
            "row {i} must NOT be drawn but content stream contains '{needle}'"
        );
    }
    for i in 0..5 {
        let needle = format!("r{i}c0");
        assert!(
            ops.contains(&needle),
            "row {i} must be drawn but content stream missing '{needle}'"
        );
    }
}

#[test]
fn render_with_split_returns_none_when_everything_fits() {
    let mut table = fixed_height_table(5, 30.0);
    table.set_position(50.0, 800.0);

    let mut page = Page::a4();
    // bottom_y far below — everything fits.
    let result = table
        .render_with_split(page.graphics(), 0.0)
        .expect("render_with_split must succeed");

    assert!(
        result.is_none(),
        "tail must be None when everything fits, got Some(_)"
    );

    let ops = page.graphics().get_operations();
    assert_eq!(
        count_tj(&ops),
        10,
        "all 5 rows × 2 cells = 10 Tj operators expected"
    );
    for i in 0..5 {
        assert!(ops.contains(&format!("r{i}c0")));
        assert!(ops.contains(&format!("r{i}c1")));
    }
}

#[test]
fn render_strict_returns_overflow_error_without_drawing_anything() {
    let mut table = fixed_height_table(10, 30.0);
    table.set_position(50.0, 800.0);

    let mut page = Page::a4();
    let err = table
        .render_strict(page.graphics(), 650.0)
        .expect_err("render_strict must Err on overflow");

    match err {
        PdfError::TableOverflow {
            rendered,
            dropped,
            bottom_y,
        } => {
            assert_eq!(rendered, 5, "5 rows would have fit");
            assert_eq!(dropped, 5, "5 rows would not have fit");
            assert!(
                (bottom_y - 650.0).abs() < f64::EPSILON,
                "bottom_y must echo back the requested floor"
            );
        }
        other => panic!("expected PdfError::TableOverflow, got {:?}", other),
    }

    // Critical invariant: pre-flight Err must NOT draw anything.
    let ops = page.graphics().get_operations();
    assert_eq!(
        count_tj(&ops),
        0,
        "render_strict must not emit any Tj before returning Err, got: {}",
        ops
    );
}

#[test]
fn render_strict_succeeds_when_everything_fits() {
    let mut table = fixed_height_table(5, 30.0);
    table.set_position(50.0, 800.0);

    let mut page = Page::a4();
    table
        .render_strict(page.graphics(), 0.0)
        .expect("render_strict must succeed when everything fits");

    let ops = page.graphics().get_operations();
    assert_eq!(count_tj(&ops), 10);
}

#[test]
fn add_paginated_table_grows_pages_and_renders_every_row() {
    // 100 data rows + 1 header. Row height 30. Page A4 (842 tall).
    // Position y=800, bottom_margin=50 → ~25 rows per page; 100 rows → 4-5 pages.
    let mut doc = Document::new();
    let page = Page::a4();
    doc.add_page(page);

    let table = fixed_height_table_with_header(100, 30.0);
    let (final_page_idx, final_y) = doc
        .add_paginated_table(0, &table, 50.0, 800.0, 50.0, 800.0)
        .expect("add_paginated_table must succeed");

    assert!(
        doc.page_count() > 1,
        "should have allocated extra pages, got page_count={}",
        doc.page_count()
    );
    assert_eq!(
        final_page_idx,
        doc.page_count() - 1,
        "returned page index must point at the last page used"
    );
    assert!(
        final_y > 0.0 && final_y < 800.0,
        "final cursor y must be inside the last page below the start, got {final_y}"
    );

    // Sum data-row hits across all pages — each "rNcM" must appear exactly once.
    for i in 0..100 {
        let needle = format!("r{i}c0");
        let total: usize = (0..doc.page_count())
            .map(|p| {
                doc.page(p)
                    .unwrap()
                    .graphics_operations()
                    .matches(&needle)
                    .count()
            })
            .sum();
        assert_eq!(
            total, 1,
            "data row {i} must be drawn exactly once across all pages, got {total}"
        );
    }

    // With repeat_header_on_split = true (default), header must appear on EVERY page used.
    for p in 0..doc.page_count() {
        let ops = doc.page(p).unwrap().graphics_operations();
        assert!(
            ops.contains("H0") && ops.contains("H1"),
            "page {p} must contain repeated header (H0,H1)"
        );
    }
}

#[test]
fn add_paginated_table_does_not_repeat_header_when_disabled() {
    let mut doc = Document::new();
    doc.add_page(Page::a4());

    let mut table = fixed_height_table_with_header(100, 30.0);
    let mut options = TableOptions {
        row_height: 30.0,
        ..TableOptions::default()
    };
    options.repeat_header_on_split = false;
    table.set_options(options);
    // re-add header & rows since set_options after add_row keeps them; nothing to re-do here,
    // but defensively rebuild to ensure options propagate to existing data isn't stale.
    // (set_options only swaps the options struct, rows are untouched.)

    let _ = doc
        .add_paginated_table(0, &table, 50.0, 800.0, 50.0, 800.0)
        .expect("add_paginated_table must succeed");

    assert!(doc.page_count() > 1);

    // Header must appear ONLY on page 0.
    let page0 = doc.page(0).unwrap().graphics_operations().to_string();
    assert!(
        page0.contains("H0") && page0.contains("H1"),
        "page 0 must have the header"
    );
    for p in 1..doc.page_count() {
        let ops = doc.page(p).unwrap().graphics_operations();
        assert!(
            !ops.contains("H0") && !ops.contains("H1"),
            "page {p} must NOT contain header when repeat_header_on_split=false; ops:\n{ops}"
        );
    }
}

#[test]
fn add_paginated_table_errs_when_table_cannot_fit_on_a_blank_page() {
    // Single row of height 1000pt on A4 (842) — even a fresh page can't hold it.
    let mut doc = Document::new();
    doc.add_page(Page::a4());

    let table = fixed_height_table(1, 1000.0);
    let err = doc
        .add_paginated_table(0, &table, 50.0, 800.0, 50.0, 800.0)
        .expect_err("must Err when row is taller than any page");

    match err {
        PdfError::TableOverflow { .. } => {}
        other => panic!("expected TableOverflow, got {:?}", other),
    }
}

#[test]
fn render_back_compat_silently_overflows_unchanged() {
    // Lock the regression contract for issue #218: existing `render(...)` keeps its
    // original behaviour (silent vertical overflow) so 2.x consumers don't break.
    let mut table = fixed_height_table(50, 30.0);
    table.set_position(50.0, 800.0);

    let mut page = Page::a4();
    table
        .render(page.graphics())
        .expect("legacy render must still return Ok");

    let ops = page.graphics().get_operations();
    // All 50 rows must be drawn regardless of the page boundary.
    assert_eq!(
        count_tj(&ops),
        100,
        "legacy render must keep silently drawing past page bottom (50 rows × 2 cells)"
    );
    assert!(ops.contains("r49c0"), "even the last row must be emitted");
}

#[test]
fn add_paginated_table_rejects_header_heavy_dos_input() {
    // Regression for F1 (security review): a table whose leading headers
    // consume an entire page must NOT trigger unbounded page allocation.
    // If headers fit but no data row advances, the previous progress check
    // (`tail.row_count() < current_table.row_count()`) was satisfied because
    // re-prepending headers on the next iteration inflated the count without
    // making real progress. A correct implementation compares *data* rows.
    let mut doc = Document::new();
    doc.add_page(Page::a4());

    // 30 header rows × 30pt = 900pt of headers, taller than A4 (842pt).
    // So no header row even fits below a 50pt floor on a 842pt page either:
    // even on a fresh page only some headers fit and zero data rows do.
    let mut table = Table::with_equal_columns(2, 200.0);
    let options = TableOptions {
        row_height: 30.0,
        ..TableOptions::default()
    };
    table.set_options(options);
    for i in 0..30 {
        table
            .add_header_row(vec![format!("H{i}A"), format!("H{i}B")])
            .unwrap();
    }
    table
        .add_row(vec!["data".to_string(), "row".to_string()])
        .unwrap();

    let initial_page_count = doc.page_count();
    let err = doc
        .add_paginated_table(0, &table, 50.0, 800.0, 50.0, 800.0)
        .expect_err("must Err — no data row can advance");

    match err {
        PdfError::TableOverflow { .. } => {}
        other => panic!("expected TableOverflow, got {:?}", other),
    }

    // Critical bound: the implementation must not have allocated dozens of
    // pages before returning. A small bounded growth (≤ 3 pages) is acceptable
    // because the first page renders some headers; the failure must be
    // detected on the next iteration.
    let pages_added = doc.page_count() - initial_page_count;
    assert!(
        pages_added <= 3,
        "DoS guard failed: {pages_added} pages allocated before TableOverflow returned (cap=3)"
    );
}

#[test]
fn render_with_split_rejects_non_finite_bottom_y() {
    // Regression for F2 (security review): NaN/±∞ for `bottom_y` must NOT
    // bypass overflow checks. NaN comparisons silently return `false`, so
    // an unchecked NaN would make `fit_count` declare every row as fitting
    // and silently render off-page — defeating the entire #218 control.
    let mut table = fixed_height_table(5, 30.0);
    table.set_position(50.0, 800.0);

    let mut page = Page::a4();
    for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        let result = table.render_with_split(page.graphics(), bad);
        match result {
            Err(PdfError::InvalidStructure(msg)) => {
                assert!(
                    msg.contains("bottom_y"),
                    "error must name the bad parameter, got: {msg}"
                );
            }
            other => panic!(
                "expected InvalidStructure for bottom_y={bad}, got {:?}",
                other
            ),
        }
    }

    // Critical: no Tj operators emitted for any of the rejected calls.
    assert_eq!(
        count_tj(&page.graphics().get_operations()),
        0,
        "non-finite bottom_y must not draw anything"
    );
}

#[test]
fn render_strict_rejects_non_finite_bottom_y() {
    let mut table = fixed_height_table(5, 30.0);
    table.set_position(50.0, 800.0);

    let mut page = Page::a4();
    let err = table
        .render_strict(page.graphics(), f64::NAN)
        .expect_err("NaN bottom_y must be rejected");
    assert!(matches!(err, PdfError::InvalidStructure(_)));
    assert_eq!(count_tj(&page.graphics().get_operations()), 0);
}

#[test]
fn add_paginated_table_rejects_non_finite_floats() {
    let mut doc = Document::new();
    doc.add_page(Page::a4());
    let table = fixed_height_table(5, 30.0);

    let cases = [
        (f64::NAN, 800.0, 50.0, 800.0, "x"),
        (50.0, f64::NAN, 50.0, 800.0, "y"),
        (50.0, 800.0, f64::INFINITY, 800.0, "bottom_y"),
        (50.0, 800.0, 50.0, f64::NEG_INFINITY, "next_page_y"),
    ];
    for (x, y, bottom_y, next_y, name) in cases {
        let err = doc
            .add_paginated_table(0, &table, x, y, bottom_y, next_y)
            .expect_err(&format!("{name} must be rejected"));
        match err {
            PdfError::InvalidStructure(msg) => assert!(
                msg.contains(name),
                "error message must name the bad parameter `{name}`, got: {msg}"
            ),
            other => panic!("expected InvalidStructure for bad {name}, got {:?}", other),
        }
    }
}

#[test]
fn page_tables_add_simple_table_unchanged() {
    // Smoke regression on the unchanged trait method to ensure the new API
    // additions don't break the existing `add_simple_table` integration.
    let mut page = Page::a4();
    let mut table = fixed_height_table(3, 30.0);
    table.set_position(50.0, 800.0);

    page.add_simple_table(&table, 50.0, 800.0)
        .expect("add_simple_table must still succeed");

    let ops = page.graphics().get_operations();
    assert_eq!(count_tj(&ops), 6, "3 rows × 2 cells");
}
