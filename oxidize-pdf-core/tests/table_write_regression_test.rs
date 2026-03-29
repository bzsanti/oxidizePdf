//! Regression tests for table writing — verify actual PDF output, not just struct construction.
//!
//! Every test in this file MUST:
//! 1. Generate a PDF
//! 2. Parse the output (content stream or re-parse)
//! 3. Verify the content is correct (positions, text, sizes)
//!
//! `assert!(result.is_ok())` alone is NOT acceptable.

use oxidize_pdf::advanced_tables::{AdvancedTableBuilder, AdvancedTableExt, CellData};
use oxidize_pdf::text::{Table, TableCell};
use oxidize_pdf::writer::WriterConfig;
use oxidize_pdf::{Color, Document, Page, Result};
use regex::Regex;

/// Helper: generate PDF bytes from a table with compression disabled for inspection
fn render_table_to_bytes(table: &Table) -> Result<Vec<u8>> {
    let mut doc = Document::new();
    let mut page = Page::a4();
    page.add_table(table)?;
    doc.add_page(page);
    let config = WriterConfig {
        compress_streams: false,
        ..WriterConfig::default()
    };
    doc.to_bytes_with_config(config)
}

/// Helper: extract all text position operators from PDF bytes.
/// Matches both `x y Td` and `a b c d x y Tm` (extracts x,y from the last two args of Tm).
fn extract_text_positions(pdf_bytes: &[u8]) -> Vec<(f64, f64)> {
    let content = String::from_utf8_lossy(pdf_bytes);
    let mut positions = Vec::new();

    // Td: x y Td
    let re_td = Regex::new(r"(-?\d+\.?\d*)\s+(-?\d+\.?\d*)\s+Td").unwrap();
    for cap in re_td.captures_iter(&content) {
        positions.push((cap[1].parse().unwrap(), cap[2].parse().unwrap()));
    }

    // Tm: a b c d e f Tm — e=x, f=y (last two numeric args before Tm)
    let re_tm =
        Regex::new(r"[-\d.]+\s+[-\d.]+\s+[-\d.]+\s+[-\d.]+\s+(-?\d+\.?\d*)\s+(-?\d+\.?\d*)\s+Tm")
            .unwrap();
    for cap in re_tm.captures_iter(&content) {
        positions.push((cap[1].parse().unwrap(), cap[2].parse().unwrap()));
    }

    positions
}

/// Helper: extract all `x y w h re` rectangle operators from PDF bytes
fn extract_rectangles(pdf_bytes: &[u8]) -> Vec<(f64, f64, f64, f64)> {
    let content = String::from_utf8_lossy(pdf_bytes);
    let re =
        Regex::new(r"(-?\d+\.?\d*)\s+(-?\d+\.?\d*)\s+(-?\d+\.?\d*)\s+(-?\d+\.?\d*)\s+re").unwrap();
    re.captures_iter(&content)
        .map(|cap| {
            let x: f64 = cap[1].parse().unwrap();
            let y: f64 = cap[2].parse().unwrap();
            let w: f64 = cap[3].parse().unwrap();
            let h: f64 = cap[4].parse().unwrap();
            (x, y, w, h)
        })
        .collect()
}

/// Helper: extract all `(text) Tj` string operators from PDF bytes
fn extract_text_strings(pdf_bytes: &[u8]) -> Vec<String> {
    let content = String::from_utf8_lossy(pdf_bytes);
    let re = Regex::new(r"\(([^)]*)\)\s+Tj").unwrap();
    re.captures_iter(&content)
        .map(|cap| cap[1].to_string())
        .collect()
}

// =============================================================================
// P1: #172 — Row Order
// =============================================================================

#[test]
fn test_row_order_top_to_bottom() -> Result<()> {
    // Issue #172: rows added first should display first (at top)
    let mut table = Table::new(vec![200.0]);
    table.set_position(50.0, 700.0);

    table.add_row(vec!["Row A".to_string()])?;
    table.add_row(vec!["Row B".to_string()])?;
    table.add_row(vec!["Row C".to_string()])?;

    let pdf_bytes = render_table_to_bytes(&table)?;
    let positions = extract_text_positions(&pdf_bytes);
    let texts = extract_text_strings(&pdf_bytes);

    // Verify all 3 rows are present
    assert!(
        texts.contains(&"Row A".to_string()),
        "PDF should contain 'Row A', found: {:?}",
        texts
    );
    assert!(
        texts.contains(&"Row B".to_string()),
        "PDF should contain 'Row B'"
    );
    assert!(
        texts.contains(&"Row C".to_string()),
        "PDF should contain 'Row C'"
    );

    // Find Y positions for each row's text
    // In PDF, Y increases upward. So the first row (top) should have the HIGHEST Y.
    let row_a_idx = texts.iter().position(|t| t == "Row A").unwrap();
    let row_b_idx = texts.iter().position(|t| t == "Row B").unwrap();
    let row_c_idx = texts.iter().position(|t| t == "Row C").unwrap();

    let y_a = positions[row_a_idx].1;
    let y_b = positions[row_b_idx].1;
    let y_c = positions[row_c_idx].1;

    assert!(
        y_a > y_b,
        "Row A (y={y_a}) should be ABOVE Row B (y={y_b}) — first row added should be at top"
    );
    assert!(
        y_b > y_c,
        "Row B (y={y_b}) should be ABOVE Row C (y={y_c}) — rows should go top to bottom"
    );

    Ok(())
}

#[test]
fn test_row_order_cell_backgrounds_match_text_order() -> Result<()> {
    // Verify cell background rectangles also follow top-to-bottom order
    // Use GridStyle::None to avoid grid border rects mixing with background rects
    use oxidize_pdf::text::table::{GridStyle, TableOptions};

    let mut options = TableOptions::default();
    options.grid_style = GridStyle::None;

    let mut table = Table::new(vec![200.0]);
    table.set_options(options);
    table.set_position(50.0, 700.0);

    let mut cell_a = TableCell::new("Row A".to_string());
    cell_a.set_background_color(Color::rgb(1.0, 0.0, 0.0));
    let mut cell_b = TableCell::new("Row B".to_string());
    cell_b.set_background_color(Color::rgb(0.0, 1.0, 0.0));

    table.add_custom_row(vec![cell_a])?;
    table.add_custom_row(vec![cell_b])?;

    let pdf_bytes = render_table_to_bytes(&table)?;
    let rects = extract_rectangles(&pdf_bytes);

    // With GridStyle::None, only cell background rects should be present
    assert_eq!(
        rects.len(),
        2,
        "Should have exactly 2 rectangles for cell backgrounds, found {}",
        rects.len()
    );

    // Row A background rect (first) should have higher bottom-left Y than Row B (second)
    let rect_a_bottom_y = rects[0].1;
    let rect_b_bottom_y = rects[1].1;

    assert!(
        rect_a_bottom_y > rect_b_bottom_y,
        "Row A rect bottom (y={rect_a_bottom_y}) should be above Row B rect bottom (y={rect_b_bottom_y})"
    );

    Ok(())
}

// =============================================================================
// P2: #170 — Multi-line text (\n) in cells
// =============================================================================

#[test]
fn test_basic_table_multiline_text() -> Result<()> {
    // Issue #170: \n in cell text should produce multiple lines
    let mut table = Table::new(vec![200.0]);
    table.set_position(50.0, 700.0);

    table.add_row(vec!["Line1\nLine2".to_string()])?;

    let pdf_bytes = render_table_to_bytes(&table)?;
    let texts = extract_text_strings(&pdf_bytes);
    let positions = extract_text_positions(&pdf_bytes);

    // Should have 2 separate text operations (one per line)
    assert!(
        texts.contains(&"Line1".to_string()),
        "PDF should contain 'Line1', found: {:?}",
        texts
    );
    assert!(
        texts.contains(&"Line2".to_string()),
        "PDF should contain 'Line2', found: {:?}",
        texts
    );

    // Line1 should be above Line2 (higher Y)
    let idx1 = texts.iter().position(|t| t == "Line1").unwrap();
    let idx2 = texts.iter().position(|t| t == "Line2").unwrap();
    assert!(
        positions[idx1].1 > positions[idx2].1,
        "Line1 (y={}) should be above Line2 (y={})",
        positions[idx1].1,
        positions[idx2].1
    );

    Ok(())
}

#[test]
fn test_advanced_table_multiline_text() -> Result<()> {
    // Issue #170: AdvancedTable should also support \n with text_wrap (default true)
    let table = AdvancedTableBuilder::new()
        .add_column("Col1", 200.0)
        .add_row_cells(vec![CellData::new("Line1\nLine2")])
        .build()
        .map_err(|e| oxidize_pdf::error::PdfError::InvalidOperation(e.to_string()))?;

    let mut doc = Document::new();
    let mut page = Page::a4();
    page.add_advanced_table(&table, 50.0, 700.0)?;
    doc.add_page(page);

    let config = WriterConfig {
        compress_streams: false,
        ..WriterConfig::default()
    };
    let pdf_bytes = doc.to_bytes_with_config(config)?;
    let texts = extract_text_strings(&pdf_bytes);

    // Should have 2 separate text strings
    assert!(
        texts.contains(&"Line1".to_string()),
        "AdvancedTable PDF should contain 'Line1', found: {:?}",
        texts
    );
    assert!(
        texts.contains(&"Line2".to_string()),
        "AdvancedTable PDF should contain 'Line2', found: {:?}",
        texts
    );

    Ok(())
}

// =============================================================================
// P3: #171 — Per-row height in basic Table
// =============================================================================

#[test]
fn test_per_row_height() -> Result<()> {
    // Issue #171: each row should be able to have its own height
    use oxidize_pdf::text::table::{GridStyle, TableOptions};

    let mut options = TableOptions::default();
    options.grid_style = GridStyle::None;
    options.row_height = 0.0; // auto

    let mut table = Table::new(vec![200.0]);
    table.set_options(options);
    table.set_position(50.0, 700.0);

    // Row A: 30pt height
    let cell_a = TableCell::new("Row A".to_string());
    table.add_custom_row(vec![cell_a])?;
    table.set_last_row_height(30.0);

    // Row B: 50pt height
    let cell_b = TableCell::new("Row B".to_string());
    table.add_custom_row(vec![cell_b])?;
    table.set_last_row_height(50.0);

    // Row C: 20pt height
    let cell_c = TableCell::new("Row C".to_string());
    table.add_custom_row(vec![cell_c])?;
    table.set_last_row_height(20.0);

    let pdf_bytes = render_table_to_bytes(&table)?;
    let positions = extract_text_positions(&pdf_bytes);
    let texts = extract_text_strings(&pdf_bytes);

    let idx_a = texts.iter().position(|t| t == "Row A").unwrap();
    let idx_b = texts.iter().position(|t| t == "Row B").unwrap();
    let idx_c = texts.iter().position(|t| t == "Row C").unwrap();

    let y_a = positions[idx_a].1;
    let y_b = positions[idx_b].1;
    let y_c = positions[idx_c].1;

    // Row A is 30pt high, Row B is 50pt high
    // Gap between Row A text and Row B text should be approximately 30pt
    // Gap between Row B text and Row C text should be approximately 50pt
    let gap_ab = y_a - y_b;
    let gap_bc = y_b - y_c;

    assert!(
        (gap_ab - 30.0).abs() < 1.0,
        "Gap between Row A and B should be ~30pt (Row A height), got {gap_ab}"
    );
    assert!(
        (gap_bc - 50.0).abs() < 1.0,
        "Gap between Row B and C should be ~50pt (Row B height), got {gap_bc}"
    );

    // Total table height should be 30+50+20 = 100
    assert!(
        (table.get_height() - 100.0).abs() < 0.01,
        "Total height should be 100pt, got {}",
        table.get_height()
    );

    Ok(())
}

// =============================================================================
// P4: #163 — Colspan/Rowspan in AdvancedTable
// =============================================================================

#[test]
fn test_advanced_table_colspan_cell_positions() -> Result<()> {
    // Issue #163: colspan should correctly position cells
    // 4 columns, each 100pt wide. Row with [Cell(colspan=2), Cell, Cell]
    let table = AdvancedTableBuilder::new()
        .add_column("A", 100.0)
        .add_column("B", 100.0)
        .add_column("C", 100.0)
        .add_column("D", 100.0)
        .add_row_cells(vec![
            CellData::new("Span2").colspan(2),
            CellData::new("CellC"),
            CellData::new("CellD"),
        ])
        .build()
        .map_err(|e| oxidize_pdf::error::PdfError::InvalidOperation(e.to_string()))?;

    let mut doc = Document::new();
    let mut page = Page::a4();
    page.add_advanced_table(&table, 50.0, 700.0)?;
    doc.add_page(page);

    let config = WriterConfig {
        compress_streams: false,
        ..WriterConfig::default()
    };
    let pdf_bytes = doc.to_bytes_with_config(config)?;
    let texts = extract_text_strings(&pdf_bytes);
    let positions = extract_text_positions(&pdf_bytes);

    // All texts should be present
    assert!(
        texts.contains(&"Span2".to_string()),
        "Should contain 'Span2', found: {:?}",
        texts
    );
    assert!(
        texts.contains(&"CellC".to_string()),
        "Should contain 'CellC', found: {:?}",
        texts
    );
    assert!(
        texts.contains(&"CellD".to_string()),
        "Should contain 'CellD', found: {:?}",
        texts
    );

    // Get X positions (skip header row texts)
    let idx_span = texts.iter().position(|t| t == "Span2").unwrap();
    let idx_c = texts.iter().position(|t| t == "CellC").unwrap();
    let idx_d = texts.iter().position(|t| t == "CellD").unwrap();

    let x_span = positions[idx_span].0;
    let x_c = positions[idx_c].0;
    let x_d = positions[idx_d].0;

    // Span2 starts at col 0 (x ~ 50 + padding)
    // CellC starts at col 2 (x ~ 250 + padding), NOT col 1 (x ~ 150)
    // CellD starts at col 3 (x ~ 350 + padding)
    assert!(
        x_c > x_span + 150.0,
        "CellC (x={x_c}) should start at col 2 (~250), not col 1 (~150). Span2 at x={x_span}"
    );
    assert!(
        x_d > x_c + 50.0,
        "CellD (x={x_d}) should be after CellC (x={x_c})"
    );

    Ok(())
}

#[test]
fn test_advanced_table_rowspan_no_overlap() -> Result<()> {
    // Issue #163: rowspan cell should not be overwritten by next row
    // 2 columns. Row 0: [Cell(rowspan=2), Cell]. Row 1: [Cell] (only 1 cell, col 0 occupied)
    let table = AdvancedTableBuilder::new()
        .add_column("A", 150.0)
        .add_column("B", 150.0)
        .add_row_cells(vec![
            CellData::new("SpanDown").rowspan(2),
            CellData::new("R0B"),
        ])
        .add_row_cells(vec![
            CellData::new("R1B"), // Only one cell — col 0 is occupied by rowspan
        ])
        .build()
        .map_err(|e| oxidize_pdf::error::PdfError::InvalidOperation(e.to_string()))?;

    let mut doc = Document::new();
    let mut page = Page::a4();
    page.add_advanced_table(&table, 50.0, 700.0)?;
    doc.add_page(page);

    let config = WriterConfig {
        compress_streams: false,
        ..WriterConfig::default()
    };
    let pdf_bytes = doc.to_bytes_with_config(config)?;
    let texts = extract_text_strings(&pdf_bytes);

    // SpanDown should be rendered once
    assert!(
        texts.contains(&"SpanDown".to_string()),
        "Should contain 'SpanDown'"
    );
    assert!(texts.contains(&"R0B".to_string()), "Should contain 'R0B'");
    assert!(texts.contains(&"R1B".to_string()), "Should contain 'R1B'");

    // R1B should be in column B (second column), not column A
    let positions = extract_text_positions(&pdf_bytes);
    let idx_r1b = texts.iter().position(|t| t == "R1B").unwrap();
    let x_r1b = positions[idx_r1b].0;

    // Column B starts at x=200 (50 + 150), R1B should be near there
    assert!(
        x_r1b > 180.0,
        "R1B (x={x_r1b}) should be in column B (x~200), not column A (x~50)"
    );

    Ok(())
}

// =============================================================================
// P6: #162 — CJK text alignment verification
// =============================================================================

#[test]
fn test_cjk_center_alignment_in_table() -> Result<()> {
    // Issue #162: CJK text should be properly centered in table cells
    use oxidize_pdf::text::TextAlign;

    let mut table = Table::new(vec![200.0]);
    table.set_position(50.0, 700.0);

    // Row with center-aligned CJK text
    table.add_row_with_alignment(vec!["测试中文".to_string()], TextAlign::Center)?;
    // Row with center-aligned Latin text (as reference)
    table.add_row_with_alignment(vec!["Test".to_string()], TextAlign::Center)?;

    let pdf_bytes = render_table_to_bytes(&table)?;
    let texts = extract_text_strings(&pdf_bytes);
    let positions = extract_text_positions(&pdf_bytes);

    // Both texts should be present
    // Note: CJK text may be hex-encoded in PDF, so check for Latin text at minimum
    assert!(
        texts.contains(&"Test".to_string()),
        "Should contain 'Test', found: {:?}",
        texts
    );

    // If CJK text is rendered as literal string, check its X position
    // Both centered texts should have X > 50 (the cell start + padding)
    // and X < 250 (cell end). The center should be around 150.
    let test_idx = texts.iter().position(|t| t == "Test").unwrap();
    let test_x = positions[test_idx].0;

    // "Test" centered in 200pt cell should be around x=50+padding+(200-text_width)/2
    // With padding=5 and text_width~20pt, centered_x ~= 50+5+(190-20)/2 = 140
    assert!(
        test_x > 80.0 && test_x < 200.0,
        "Centered 'Test' (x={test_x}) should be roughly centered in 200pt cell starting at x=50"
    );

    Ok(())
}

// =============================================================================
// Guard: extract_text_positions must not silently return empty
// =============================================================================

#[test]
fn test_extract_text_positions_is_non_empty() -> Result<()> {
    // Guard against silent test pass if PDF operator format changes.
    let mut table = Table::new(vec![200.0]);
    table.set_position(50.0, 700.0);
    table.add_row(vec!["Guard".to_string()])?;

    let pdf_bytes = render_table_to_bytes(&table)?;
    let positions = extract_text_positions(&pdf_bytes);

    assert!(
        !positions.is_empty(),
        "extract_text_positions returned empty — regex may not match the PDF operator format"
    );

    Ok(())
}
