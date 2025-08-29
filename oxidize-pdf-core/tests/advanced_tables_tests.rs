//! Unit tests for advanced tables functionality

use oxidize_pdf::advanced_tables::{
    AdvancedTableBuilder, BorderStyle, CellAlignment, CellStyle, HeaderBuilder, Padding,
};
use oxidize_pdf::graphics::Color;
use oxidize_pdf::text::Font;

#[test]
fn test_table_builder_basic() {
    let table = AdvancedTableBuilder::new()
        .title("Test Table")
        .columns(vec![("Column 1", 100.0), ("Column 2", 150.0)])
        .add_row(vec!["Cell 1", "Cell 2"])
        .build()
        .unwrap();

    assert_eq!(table.title, Some("Test Table".to_string()));
    assert_eq!(table.columns.len(), 2);
    assert_eq!(table.rows.len(), 1);
}

#[test]
fn test_cell_style_creation() {
    let style = CellStyle::new()
        .background_color(Color::blue())
        .text_color(Color::white())
        .font(Font::HelveticaBold)
        .font_size(14.0)
        .alignment(CellAlignment::Center)
        .padding(Padding::uniform(10.0))
        .border(BorderStyle::Double, 2.0, Color::black());

    assert_eq!(style.background_color, Some(Color::blue()));
    assert_eq!(style.text_color, Some(Color::white()));
    assert_eq!(style.font, Some(Font::HelveticaBold));
    assert_eq!(style.font_size, Some(14.0));
    assert_eq!(style.alignment, CellAlignment::Center);
    assert_eq!(style.padding.top, 10.0);
    assert_eq!(style.border_style, BorderStyle::Double);
}

#[test]
fn test_padding_creation() {
    let uniform = Padding::uniform(5.0);
    assert_eq!(uniform.top, 5.0);
    assert_eq!(uniform.right, 5.0);
    assert_eq!(uniform.bottom, 5.0);
    assert_eq!(uniform.left, 5.0);

    let custom = Padding::new(1.0, 2.0, 3.0, 4.0);
    assert_eq!(custom.top, 1.0);
    assert_eq!(custom.right, 2.0);
    assert_eq!(custom.bottom, 3.0);
    assert_eq!(custom.left, 4.0);
}

#[test]
fn test_table_with_zebra_stripes() {
    let table = AdvancedTableBuilder::new()
        .columns(vec![("A", 50.0), ("B", 50.0)])
        .zebra_stripes(true, Color::gray(0.9))
        .add_row(vec!["1", "2"])
        .add_row(vec!["3", "4"])
        .add_row(vec!["5", "6"])
        .build()
        .unwrap();

    assert!(table.zebra_stripes);
    assert_eq!(table.zebra_color, Some(Color::gray(0.9)));
    assert_eq!(table.rows.len(), 3);
}

#[test]
fn test_complex_header_builder() {
    let header = HeaderBuilder::auto()
        .add_level(vec![
            ("Group 1", 2), // Spans 2 columns
            ("Group 2", 3), // Spans 3 columns
        ])
        .add_level(vec![
            ("Col A", 1),
            ("Col B", 1),
            ("Col C", 1),
            ("Col D", 1),
            ("Col E", 1),
        ]);

    assert_eq!(header.levels.len(), 2);
    assert_eq!(header.levels[0].len(), 2);
    assert_eq!(header.levels[1].len(), 5);
}

#[test]
fn test_table_with_custom_row_styles() {
    let header_style = CellStyle::new()
        .background_color(Color::blue())
        .text_color(Color::white());

    let data_style = CellStyle::new().font(Font::Helvetica).font_size(10.0);

    let table = AdvancedTableBuilder::new()
        .columns(vec![("Name", 100.0), ("Value", 80.0)])
        .header_style(header_style.clone())
        .data_style(data_style.clone())
        .add_row(vec!["Item 1", "100"])
        .add_row_with_style(vec!["Total", "100"], header_style.clone())
        .build()
        .unwrap();

    assert_eq!(table.header_style.background_color, Some(Color::blue()));
    assert_eq!(table.default_style.font_size, Some(10.0));
    assert_eq!(table.rows.len(), 2);
}

#[test]
fn test_table_positioning() {
    let table = AdvancedTableBuilder::new()
        .position(100.0, 200.0)
        .columns(vec![("Test", 50.0)])
        .build()
        .unwrap();

    assert_eq!(table.x, 100.0);
    assert_eq!(table.y, 200.0);
}

#[test]
fn test_table_with_mixed_styles() {
    let style1 = CellStyle::new().text_color(Color::red());
    let style2 = CellStyle::new().text_color(Color::blue());

    let table = AdvancedTableBuilder::new()
        .columns(vec![("A", 50.0), ("B", 50.0)])
        .add_row_with_mixed_styles(vec![(style1.clone(), "Red"), (style2.clone(), "Blue")])
        .build()
        .unwrap();

    assert_eq!(table.rows.len(), 1);
    assert_eq!(table.rows[0].cells.len(), 2);
}

#[test]
fn test_border_styles() {
    assert_eq!(BorderStyle::None as i32, 0);
    assert_eq!(BorderStyle::Solid as i32, 1);
    assert_eq!(BorderStyle::Dashed as i32, 2);
    assert_eq!(BorderStyle::Dotted as i32, 3);
    assert_eq!(BorderStyle::Double as i32, 4);
}

#[test]
fn test_cell_alignments() {
    assert_eq!(CellAlignment::Left as i32, 0);
    assert_eq!(CellAlignment::Center as i32, 1);
    assert_eq!(CellAlignment::Right as i32, 2);
    assert_eq!(CellAlignment::Justify as i32, 3);
}

#[test]
fn test_table_width_calculation() {
    let table = AdvancedTableBuilder::new()
        .columns(vec![("Col1", 100.0), ("Col2", 150.0), ("Col3", 200.0)])
        .build()
        .unwrap();

    let total_width: f64 = table.columns.iter().map(|c| c.width).sum();
    assert_eq!(total_width, 450.0);
}

#[test]
fn test_empty_table() {
    let result = AdvancedTableBuilder::new().title("Empty Table").build();

    // Should fail without columns
    assert!(result.is_err());
}

#[test]
fn test_table_with_title_only() {
    let table = AdvancedTableBuilder::new()
        .title("Title Only")
        .columns(vec![("Column", 100.0)])
        .build()
        .unwrap();

    assert_eq!(table.title, Some("Title Only".to_string()));
    assert_eq!(table.rows.len(), 0); // No data rows
}
