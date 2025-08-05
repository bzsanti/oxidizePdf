//! Page extension for table rendering
//!
//! This module provides traits and implementations to easily add tables to PDF pages.

use crate::error::PdfError;
use crate::graphics::Color;
use crate::page::Page;
use crate::text::{
    AdvancedTable, AdvancedTableCell, AdvancedTableOptions, AlternatingRowColors, BorderStyle,
    CellPadding, ColumnDefinition, ColumnWidth, Table, TableOptions,
};

/// Extension trait for adding tables to pages
pub trait PageTables {
    /// Add a simple table to the page
    fn add_simple_table(&mut self, table: &Table, x: f64, y: f64) -> Result<&mut Self, PdfError>;

    /// Add an advanced table to the page
    fn add_advanced_table(
        &mut self,
        table: &AdvancedTable,
        x: f64,
        y: f64,
        available_width: f64,
    ) -> Result<&mut Self, PdfError>;

    /// Create and add a quick table with equal columns
    fn add_quick_table(
        &mut self,
        data: Vec<Vec<String>>,
        x: f64,
        y: f64,
        width: f64,
        options: Option<TableOptions>,
    ) -> Result<&mut Self, PdfError>;

    /// Create and add an advanced table with custom styling
    fn add_styled_table(
        &mut self,
        headers: Vec<String>,
        data: Vec<Vec<String>>,
        x: f64,
        y: f64,
        width: f64,
        style: TableStyle,
    ) -> Result<&mut Self, PdfError>;
}

/// Predefined table styles
#[derive(Debug, Clone)]
pub struct TableStyle {
    /// Border style
    pub borders: BorderStyle,
    /// Cell padding
    pub padding: CellPadding,
    /// Header background color
    pub header_background: Option<Color>,
    /// Header text color
    pub header_text_color: Option<Color>,
    /// Alternating row colors
    pub alternating_rows: Option<AlternatingRowColors>,
    /// Default font size
    pub font_size: f64,
}

impl TableStyle {
    /// Create a minimal table style (no borders)
    pub fn minimal() -> Self {
        Self {
            borders: BorderStyle::none(),
            padding: CellPadding::uniform(5.0),
            header_background: None,
            header_text_color: None,
            alternating_rows: None,
            font_size: 10.0,
        }
    }

    /// Create a simple table style with borders
    pub fn simple() -> Self {
        Self {
            borders: BorderStyle::default(),
            padding: CellPadding::uniform(5.0),
            header_background: None,
            header_text_color: None,
            alternating_rows: None,
            font_size: 10.0,
        }
    }

    /// Create a professional table style
    pub fn professional() -> Self {
        Self {
            borders: BorderStyle::horizontal_only(1.0, Color::gray(0.7)),
            padding: CellPadding::symmetric(8.0, 6.0),
            header_background: Some(Color::gray(0.1)),
            header_text_color: Some(Color::white()),
            alternating_rows: Some(AlternatingRowColors {
                even_color: Color::white(),
                odd_color: Color::gray(0.95),
                include_header: false,
            }),
            font_size: 10.0,
        }
    }

    /// Create a colorful table style
    pub fn colorful() -> Self {
        Self {
            borders: BorderStyle::default(),
            padding: CellPadding::uniform(6.0),
            header_background: Some(Color::rgb(0.2, 0.4, 0.8)),
            header_text_color: Some(Color::white()),
            alternating_rows: Some(AlternatingRowColors {
                even_color: Color::rgb(0.95, 0.95, 1.0),
                odd_color: Color::white(),
                include_header: false,
            }),
            font_size: 10.0,
        }
    }
}

impl PageTables for Page {
    fn add_simple_table(&mut self, table: &Table, x: f64, y: f64) -> Result<&mut Self, PdfError> {
        let mut table_clone = table.clone();
        table_clone.set_position(x, y);
        table_clone.render(self.graphics())?;
        Ok(self)
    }

    fn add_advanced_table(
        &mut self,
        table: &AdvancedTable,
        x: f64,
        y: f64,
        available_width: f64,
    ) -> Result<&mut Self, PdfError> {
        let mut table_clone = table.clone();
        table_clone.set_position(x, y);
        table_clone.render(self.graphics(), available_width)?;
        Ok(self)
    }

    fn add_quick_table(
        &mut self,
        data: Vec<Vec<String>>,
        x: f64,
        y: f64,
        width: f64,
        options: Option<TableOptions>,
    ) -> Result<&mut Self, PdfError> {
        if data.is_empty() {
            return Ok(self);
        }

        let num_columns = data[0].len();
        let mut table = Table::with_equal_columns(num_columns, width);

        if let Some(opts) = options {
            table.set_options(opts);
        }

        for row in data {
            table.add_row(row)?;
        }

        self.add_simple_table(&table, x, y)
    }

    fn add_styled_table(
        &mut self,
        headers: Vec<String>,
        data: Vec<Vec<String>>,
        x: f64,
        y: f64,
        width: f64,
        style: TableStyle,
    ) -> Result<&mut Self, PdfError> {
        let num_columns = headers.len();
        if num_columns == 0 {
            return Ok(self);
        }

        // Create columns with equal width
        let columns: Vec<ColumnDefinition> = (0..num_columns)
            .map(|_| ColumnDefinition {
                width: ColumnWidth::Fixed(width / num_columns as f64),
                default_align: crate::text::TextAlign::Left,
                min_width: None,
                max_width: None,
            })
            .collect();

        let mut table = AdvancedTable::new(columns);

        // Apply style
        let options = AdvancedTableOptions {
            border_style: style.borders,
            cell_padding: style.padding,
            font_size: style.font_size,
            alternating_rows: style.alternating_rows,
            ..Default::default()
        };
        table.set_options(options);

        // Add header row
        let header_cells: Vec<AdvancedTableCell> = headers
            .into_iter()
            .map(|text| {
                let mut cell =
                    AdvancedTableCell::text(text).with_align(crate::text::TextAlign::Center);

                if let Some(bg) = style.header_background {
                    cell = cell.with_background(bg);
                }
                if let Some(color) = style.header_text_color {
                    cell = cell.with_text_color(color);
                }

                cell
            })
            .collect();

        table.add_header_row(header_cells)?;

        // Add data rows
        for row_data in data {
            table.add_text_row(row_data)?;
        }

        self.add_advanced_table(&table, x, y, width)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::page::Page;

    #[test]
    fn test_page_tables_trait() {
        let mut page = Page::a4();

        // Test quick table
        let data = vec![
            vec!["Name".to_string(), "Age".to_string()],
            vec!["John".to_string(), "30".to_string()],
        ];

        let result = page.add_quick_table(data, 50.0, 700.0, 400.0, None);
        assert!(result.is_ok());
    }

    #[test]
    fn test_table_styles() {
        let minimal = TableStyle::minimal();
        assert!(minimal.borders.top.is_none());

        let simple = TableStyle::simple();
        assert!(simple.borders.top.is_some());

        let professional = TableStyle::professional();
        assert!(professional.header_background.is_some());
        assert!(professional.alternating_rows.is_some());

        let colorful = TableStyle::colorful();
        assert!(colorful.header_background.is_some());
    }

    #[test]
    fn test_styled_table() {
        let mut page = Page::a4();

        let headers = vec!["Column 1".to_string(), "Column 2".to_string()];
        let data = vec![
            vec!["Data 1".to_string(), "Data 2".to_string()],
            vec!["Data 3".to_string(), "Data 4".to_string()],
        ];

        let result = page.add_styled_table(
            headers,
            data,
            50.0,
            700.0,
            500.0,
            TableStyle::professional(),
        );

        assert!(result.is_ok());
    }

    #[test]
    fn test_empty_table() {
        let mut page = Page::a4();

        let data: Vec<Vec<String>> = vec![];
        let result = page.add_quick_table(data, 50.0, 700.0, 400.0, None);
        assert!(result.is_ok());
    }

    #[test]
    fn test_advanced_table_integration() {
        let mut page = Page::a4();

        let columns = vec![
            ColumnDefinition {
                width: ColumnWidth::Fixed(100.0),
                default_align: crate::text::TextAlign::Left,
                min_width: None,
                max_width: None,
            },
            ColumnDefinition {
                width: ColumnWidth::Fixed(200.0),
                default_align: crate::text::TextAlign::Right,
                min_width: None,
                max_width: None,
            },
        ];

        let table = AdvancedTable::new(columns);
        let result = page.add_advanced_table(&table, 50.0, 700.0, 300.0);
        assert!(result.is_ok());
    }
}
