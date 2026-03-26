//! Integration tests for table functionality

use oxidize_pdf::advanced_tables::{CellData, RowData};
use oxidize_pdf::text::metrics::{
    get_custom_font_metrics, measure_text, register_custom_font_metrics, FontMetrics,
};
use oxidize_pdf::text::{HeaderStyle, Table, TableCell, TableOptions, TextAlign};
use oxidize_pdf::writer::WriterConfig;
use oxidize_pdf::{Color, Document, Font, Page, Result};
use std::fs;
use tempfile::TempDir;

#[test]
fn test_simple_table() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let file_path = temp_dir.path().join("simple_table.pdf");

    let mut doc = Document::new();
    doc.set_title("Simple Table Test");

    let mut page = Page::a4();

    // Create a simple table
    let mut table = Table::new(vec![150.0, 200.0, 150.0]);
    table.set_position(50.0, 700.0);

    // Add header row
    table.add_header_row(vec![
        "Product".to_string(),
        "Description".to_string(),
        "Price".to_string(),
    ])?;

    // Add data rows
    table.add_row(vec![
        "Widget A".to_string(),
        "High-quality widget for everyday use".to_string(),
        "$19.99".to_string(),
    ])?;

    table.add_row(vec![
        "Widget B".to_string(),
        "Premium widget with advanced features".to_string(),
        "$39.99".to_string(),
    ])?;

    table.add_row(vec![
        "Widget C".to_string(),
        "Budget-friendly widget option".to_string(),
        "$9.99".to_string(),
    ])?;

    // Render the table
    page.add_table(&table)?;

    doc.add_page(page);
    doc.save(&file_path)?;

    // Verify file was created
    assert!(file_path.exists());
    let file_size = fs::metadata(&file_path)?.len();
    assert!(file_size > 1000); // Should be larger than minimal PDF

    Ok(())
}

#[test]
fn test_table_with_custom_options() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let file_path = temp_dir.path().join("custom_table.pdf");

    let mut doc = Document::new();
    doc.set_title("Custom Table Test");

    let mut page = Page::a4();

    // Create table with custom options
    let mut table = Table::new(vec![100.0, 150.0, 100.0, 100.0]);
    table.set_position(50.0, 650.0);

    // Custom table options
    let mut options = TableOptions::default();
    options.border_width = 2.0;
    options.border_color = Color::rgb(0.2, 0.3, 0.5);
    options.cell_padding = 8.0;
    options.font = Font::TimesRoman;
    options.font_size = 11.0;
    options.text_color = Color::rgb(0.1, 0.1, 0.1);

    // Header style
    options.header_style = Some(HeaderStyle {
        background_color: Color::rgb(0.9, 0.9, 0.95),
        text_color: Color::rgb(0.0, 0.0, 0.5),
        font: Font::TimesBold,
        bold: true,
    });

    table.set_options(options);

    // Add header
    table.add_header_row(vec![
        "ID".to_string(),
        "Name".to_string(),
        "Status".to_string(),
        "Score".to_string(),
    ])?;

    // Add data rows
    table.add_row(vec![
        "001".to_string(),
        "Alice Johnson".to_string(),
        "Active".to_string(),
        "95".to_string(),
    ])?;

    table.add_row(vec![
        "002".to_string(),
        "Bob Smith".to_string(),
        "Pending".to_string(),
        "87".to_string(),
    ])?;

    page.add_table(&table)?;

    doc.add_page(page);
    doc.save(&file_path)?;

    assert!(file_path.exists());
    Ok(())
}

#[test]
fn test_table_alignment() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let file_path = temp_dir.path().join("aligned_table.pdf");

    let mut doc = Document::new();
    doc.set_title("Table Alignment Test");

    let mut page = Page::a4();

    // Create table with different alignments
    let mut table = Table::new(vec![120.0, 180.0, 120.0]);
    table.set_position(50.0, 700.0);

    // Header
    table.add_header_row(vec![
        "Left".to_string(),
        "Center".to_string(),
        "Right".to_string(),
    ])?;

    // Add rows with different alignments
    table.add_row_with_alignment(
        vec![
            "Left text".to_string(),
            "Center text".to_string(),
            "Right text".to_string(),
        ],
        TextAlign::Left,
    )?;

    // Custom cells with individual alignment
    let cells = vec![
        TableCell::with_align("Left aligned".to_string(), TextAlign::Left),
        TableCell::with_align("Center aligned".to_string(), TextAlign::Center),
        TableCell::with_align("Right aligned".to_string(), TextAlign::Right),
    ];
    table.add_custom_row(cells)?;

    page.add_table(&table)?;

    doc.add_page(page);
    doc.save(&file_path)?;

    assert!(file_path.exists());
    Ok(())
}

#[test]
fn test_table_with_colspan() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let file_path = temp_dir.path().join("colspan_table.pdf");

    let mut doc = Document::new();
    doc.set_title("Table Colspan Test");

    let mut page = Page::a4();

    // Create table
    let mut table = Table::new(vec![100.0, 100.0, 100.0, 100.0]);
    table.set_position(50.0, 700.0);

    // Regular header
    table.add_header_row(vec![
        "Col 1".to_string(),
        "Col 2".to_string(),
        "Col 3".to_string(),
        "Col 4".to_string(),
    ])?;

    // Row with colspan
    let cells = vec![
        TableCell::new("Normal cell".to_string()),
        TableCell::with_colspan("Merged across 2 columns".to_string(), 2)
            .set_align(TextAlign::Center)
            .clone(),
        TableCell::new("Normal cell".to_string()),
    ];
    table.add_custom_row(cells)?;

    // Another colspan row
    let cells = vec![
        TableCell::with_colspan("Merged across 3 columns".to_string(), 3)
            .set_align(TextAlign::Center)
            .clone(),
        TableCell::new("Single".to_string()),
    ];
    table.add_custom_row(cells)?;

    // Full width cell
    let cells = vec![TableCell::with_colspan("Full width cell".to_string(), 4)
        .set_align(TextAlign::Center)
        .clone()];
    table.add_custom_row(cells)?;

    page.add_table(&table)?;

    doc.add_page(page);
    doc.save(&file_path)?;

    assert!(file_path.exists());
    Ok(())
}

#[test]
fn test_multiple_tables_on_page() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let file_path = temp_dir.path().join("multiple_tables.pdf");

    let mut doc = Document::new();
    doc.set_title("Multiple Tables Test");

    let mut page = Page::a4();

    // First table
    let mut table1 = Table::with_equal_columns(3, 300.0);
    table1.set_position(50.0, 750.0);
    table1.add_header_row(vec!["A".to_string(), "B".to_string(), "C".to_string()])?;
    table1.add_row(vec!["1".to_string(), "2".to_string(), "3".to_string()])?;

    page.add_table(&table1)?;

    // Add some text between tables
    page.text()
        .set_font(Font::Helvetica, 12.0)
        .at(50.0, 650.0)
        .write("Table comparison:")?;

    // Second table
    let mut table2 = Table::new(vec![80.0, 120.0, 100.0, 80.0]);
    table2.set_position(50.0, 600.0);

    let mut options = TableOptions::default();
    options.border_color = Color::rgb(0.8, 0.2, 0.2);
    options.font_size = 9.0;
    table2.set_options(options);

    table2.add_header_row(vec![
        "Type".to_string(),
        "Description".to_string(),
        "Value".to_string(),
        "Unit".to_string(),
    ])?;

    table2.add_row(vec![
        "Speed".to_string(),
        "Maximum velocity".to_string(),
        "150".to_string(),
        "km/h".to_string(),
    ])?;

    page.add_table(&table2)?;

    doc.add_page(page);
    doc.save(&file_path)?;

    assert!(file_path.exists());
    Ok(())
}

#[test]
fn test_table_error_handling() {
    // Test column count mismatch
    let mut table = Table::new(vec![100.0, 100.0]);
    let result = table.add_row(vec![
        "One".to_string(),
        "Two".to_string(),
        "Three".to_string(), // Too many cells
    ]);
    assert!(result.is_err());

    // Test invalid colspan
    let mut table = Table::new(vec![100.0, 100.0, 100.0]);
    let cells = vec![
        TableCell::new("Normal".to_string()),
        TableCell::with_colspan("Too wide".to_string(), 3), // Total would be 4
    ];
    let result = table.add_custom_row(cells);
    assert!(result.is_err());
}

#[test]
fn test_table_dimensions() -> Result<()> {
    let mut table = Table::new(vec![100.0, 150.0, 200.0]);

    // Test width calculation
    assert_eq!(table.get_width(), 450.0);

    // Add rows and test height calculation
    table.add_row(vec!["A".to_string(), "B".to_string(), "C".to_string()])?;
    table.add_row(vec!["D".to_string(), "E".to_string(), "F".to_string()])?;

    // With default font size 10 and padding 5, each row should be 20 points
    assert_eq!(table.get_height(), 40.0);

    // Test with custom row height
    let mut options = TableOptions::default();
    options.row_height = 30.0;
    table.set_options(options);

    assert_eq!(table.get_height(), 60.0);

    Ok(())
}

#[test]
fn test_table_with_custom_fonts() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let file_path = temp_dir.path().join("custom_font_table.pdf");

    let mut doc = Document::new();
    doc.set_title("Custom Font Table Test");

    // Load a custom font (if available)
    // For this test, we'll just use standard fonts
    let mut page = Page::a4();

    let mut table = Table::new(vec![150.0, 150.0, 150.0]);
    table.set_position(50.0, 700.0);

    // Use different fonts for header and content
    let mut options = TableOptions::default();
    options.font = Font::Courier;
    options.font_size = 10.0;

    options.header_style = Some(HeaderStyle {
        background_color: Color::gray(0.85),
        text_color: Color::black(),
        font: Font::CourierBold,
        bold: true,
    });

    table.set_options(options);

    table.add_header_row(vec![
        "Code".to_string(),
        "Function".to_string(),
        "Status".to_string(),
    ])?;

    table.add_row(vec![
        "FN001".to_string(),
        "initialize()".to_string(),
        "OK".to_string(),
    ])?;

    table.add_row(vec![
        "FN002".to_string(),
        "process()".to_string(),
        "PENDING".to_string(),
    ])?;

    page.add_table(&table)?;

    doc.add_page(page);
    doc.save(&file_path)?;

    assert!(file_path.exists());
    Ok(())
}

/// Issue #160: CJK font not displayed correctly in Table
/// Verifies that Table with Font::Custom uses hex-encoded CID strings
/// instead of literal PDF strings, which is required for Type0/CJK fonts.
#[test]
fn test_table_with_custom_font_uses_hex_encoding() -> Result<()> {
    let mut doc = Document::new();
    doc.set_title("CJK Table Font Test - Issue #160");

    let mut page = Page::a4();

    let mut table = Table::new(vec![200.0, 200.0]);
    table.set_position(50.0, 700.0);

    let mut options = TableOptions::default();
    options.font = Font::Custom("NotoSansCJK".to_string());
    options.font_size = 12.0;
    table.set_options(options);

    // Add row with CJK text: 你好 (U+4F60 U+597D)
    table.add_row(vec!["你好".to_string(), "世界".to_string()])?;

    page.add_table(&table)?;
    doc.add_page(page);

    // Disable stream compression so we can inspect raw content stream
    let config = WriterConfig {
        compress_streams: false,
        ..WriterConfig::default()
    };
    let pdf_bytes = doc.to_bytes_with_config(config)?;
    let pdf_content = String::from_utf8_lossy(&pdf_bytes);

    // The content stream must contain hex-encoded CIDs with Tj operator
    // 你=U+4F60, 好=U+597D → <4F60597D> Tj
    // 世=U+4E16, 界=U+754C → <4E16754C> Tj
    assert!(
        pdf_content.contains("<4F60597D> Tj"),
        "PDF should contain hex-encoded CJK text '你好' as <4F60597D> Tj operator"
    );
    assert!(
        pdf_content.contains("<4E16754C> Tj"),
        "PDF should contain hex-encoded CJK text '世界' as <4E16754C> Tj operator"
    );

    // Must NOT contain literal CJK characters in PDF string syntax
    assert!(
        !pdf_content.contains("(你好)"),
        "PDF must not contain literal CJK in parenthesized string"
    );

    Ok(())
}

/// Regression test: standard font tables must use literal encoding, not hex.
#[test]
fn test_table_with_standard_font_uses_literal_encoding() -> Result<()> {
    let mut doc = Document::new();
    let mut page = Page::a4();

    let mut table = Table::new(vec![200.0]);
    table.set_position(50.0, 700.0);
    // Default font is Helvetica (standard)
    table.add_row(vec!["Hello World".to_string()])?;

    page.add_table(&table)?;
    doc.add_page(page);

    let config = WriterConfig {
        compress_streams: false,
        ..WriterConfig::default()
    };
    let pdf_bytes = doc.to_bytes_with_config(config)?;
    let pdf_content = String::from_utf8_lossy(&pdf_bytes);

    assert!(
        pdf_content.contains("(Hello World) Tj"),
        "Standard font table must use literal string encoding"
    );

    Ok(())
}

/// Issue #162: CJK text not aligned center in table cell.
/// When custom font metrics are registered, measure_text should use actual widths
/// instead of Helvetica-like fallbacks, producing correct centering.
#[test]
fn test_measure_text_uses_registered_custom_font_metrics() {
    // Register a custom font with known CJK widths
    let mut widths = std::collections::HashMap::new();
    // CJK chars should be full-width (1000 units)
    for ch in "测试中文长文本居中对齐".chars() {
        widths.insert(ch, 1000u16);
    }
    // ASCII chars half-width
    widths.insert(' ', 500);
    for ch in 'a'..='z' {
        widths.insert(ch, 500);
    }
    let metrics = FontMetrics::from_char_map(widths, 500);
    register_custom_font_metrics("TestCJKFont162".to_string(), metrics);

    let font = Font::Custom("TestCJKFont162".to_string());

    // Measure CJK text: 11 chars × 1000 units × (10.5 / 1000) = 115.5
    let cjk_text = "测试中文长文本居中对齐";
    let cjk_width = measure_text(cjk_text, font.clone(), 10.5);
    let expected_cjk_width = 11.0 * 10.5; // 11 CJK chars × full-width
    assert!(
        (cjk_width - expected_cjk_width).abs() < 0.01,
        "CJK text width should be {expected_cjk_width}, got {cjk_width}"
    );

    // Measure ASCII text: 4 chars × 500 units × (10.5 / 1000) = 21.0
    let ascii_width = measure_text("test", font, 10.5);
    let expected_ascii_width = 4.0 * 500.0 * 10.5 / 1000.0;
    assert!(
        (ascii_width - expected_ascii_width).abs() < 0.01,
        "ASCII text width should be {expected_ascii_width}, got {ascii_width}"
    );

    // CJK text should be wider than ASCII text of same length
    assert!(
        cjk_width > ascii_width,
        "CJK text ({cjk_width}) should be wider than ASCII text ({ascii_width})"
    );
}

/// Issue #162: Default custom font metrics should treat CJK chars as full-width (1000).
#[test]
fn test_default_custom_metrics_cjk_width() {
    // Use an unregistered font name to trigger default metrics creation
    let font = Font::Custom("UnregisteredFontForCJKTest".to_string());

    // CJK character '中' (U+4E2D) should use 1000 width, not 556 (Helvetica default)
    let cjk_width = measure_text("中", font.clone(), 10.0);
    let expected = 1000.0 * 10.0 / 1000.0; // 10.0
    assert!(
        (cjk_width - expected).abs() < 0.01,
        "CJK char '中' should measure {expected}, got {cjk_width}"
    );

    // ASCII 'A' should still use Helvetica-like width (667)
    let ascii_width = measure_text("A", font, 10.0);
    let expected_ascii = 667.0 * 10.0 / 1000.0; // 6.67
    assert!(
        (ascii_width - expected_ascii).abs() < 0.01,
        "ASCII 'A' should measure {expected_ascii}, got {ascii_width}"
    );
}

/// Issue #163: CellData and RowData must be accessible from public API.
#[test]
fn test_celldata_accessible_and_span_works() {
    // Verify CellData can be constructed and used
    let cell = CellData::new("Hello").colspan(3).rowspan(2);

    assert_eq!(cell.content, "Hello");
    assert_eq!(cell.colspan, 3);
    assert_eq!(cell.rowspan, 2);

    // colspan(0) should clamp to 1 (minimum, not maximum)
    let cell_zero = CellData::new("Zero span").colspan(0);
    assert_eq!(cell_zero.colspan, 1, "colspan(0) should clamp to minimum 1");

    // rowspan(0) should clamp to 1
    let cell_zero_row = CellData::new("Zero row span").rowspan(0);
    assert_eq!(
        cell_zero_row.rowspan, 1,
        "rowspan(0) should clamp to minimum 1"
    );
}

/// Issue #163: RowData must be accessible and constructable from CellData.
#[test]
fn test_rowdata_from_cells() {
    let cells = vec![
        CellData::new("Cell 1").colspan(2),
        CellData::new("Cell 2"),
        CellData::new("Cell 3").rowspan(3),
    ];

    let row = RowData::from_cells(cells);
    assert_eq!(row.cells.len(), 3);
    assert_eq!(row.cells[0].colspan, 2);
    assert_eq!(row.cells[1].colspan, 1); // default
    assert_eq!(row.cells[2].rowspan, 3);
}

/// Quality item #4: default_width should be computed as average, not hardcoded 500.
#[test]
fn test_from_char_map_default_width_is_average() {
    let mut widths = std::collections::HashMap::new();
    widths.insert('A', 700u16);
    widths.insert('B', 600u16);
    widths.insert('C', 500u16);
    // Average = (700 + 600 + 500) / 3 = 600
    let avg: u16 = 600;
    let metrics = FontMetrics::from_char_map(widths, avg);
    // Unknown char 'Z' should use the average as default
    assert_eq!(
        metrics.char_width('Z'),
        600,
        "default_width should reflect the font's average width, not a hardcoded value"
    );
}

/// Quality item #1: add_font_from_bytes must not leave metrics registered on failure.
#[test]
fn test_add_font_from_bytes_no_metrics_on_failure() {
    let font_name = "FailTestFont_unique_162_163";
    assert!(
        get_custom_font_metrics(font_name).is_none(),
        "metrics must not exist before the call"
    );

    let mut doc = Document::new();
    // Invalid font data — parsing will fail
    let result = doc.add_font_from_bytes(font_name, vec![0u8; 16]);
    assert!(result.is_err(), "invalid font data should produce an error");
    assert!(
        get_custom_font_metrics(font_name).is_none(),
        "metrics must NOT be registered when add_font_from_bytes fails"
    );
}
