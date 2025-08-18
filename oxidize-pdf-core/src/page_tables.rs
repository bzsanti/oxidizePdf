//! Page extension for table rendering
//!
//! This module provides traits and implementations to easily add tables to PDF pages.

use crate::error::PdfError;
use crate::graphics::Color;
use crate::page::Page;
use crate::text::{Font, HeaderStyle, Table, TableOptions};

/// Extension trait for adding tables to pages
pub trait PageTables {
    /// Add a simple table to the page
    fn add_simple_table(&mut self, table: &Table, x: f64, y: f64) -> Result<&mut Self, PdfError>;

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
    /// Header background color
    pub header_background: Option<Color>,
    /// Header text color
    pub header_text_color: Option<Color>,
    /// Default font size
    pub font_size: f64,
}

impl TableStyle {
    /// Create a minimal table style (no borders)
    pub fn minimal() -> Self {
        Self {
            header_background: None,
            header_text_color: None,
            font_size: 10.0,
        }
    }

    /// Create a simple table style with borders
    pub fn simple() -> Self {
        Self {
            header_background: None,
            header_text_color: None,
            font_size: 10.0,
        }
    }

    /// Create a professional table style
    pub fn professional() -> Self {
        Self {
            header_background: Some(Color::gray(0.1)),
            header_text_color: Some(Color::white()),
            font_size: 10.0,
        }
    }

    /// Create a colorful table style
    pub fn colorful() -> Self {
        Self {
            header_background: Some(Color::rgb(0.2, 0.4, 0.8)),
            header_text_color: Some(Color::white()),
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

        // Create a simple table with the given style
        let mut table = Table::with_equal_columns(num_columns, width);

        // Create table options based on style
        let header_style = if style.header_background.is_some() || style.header_text_color.is_some()
        {
            Some(HeaderStyle {
                background_color: style.header_background.unwrap_or(Color::white()),
                text_color: style.header_text_color.unwrap_or(Color::black()),
                font: Font::Helvetica,
                bold: true,
            })
        } else {
            None
        };

        let options = TableOptions {
            font_size: style.font_size,
            header_style,
            ..Default::default()
        };

        table.set_options(options);

        // Add header row
        table.add_row(headers)?;

        // Add data rows
        for row_data in data {
            table.add_row(row_data)?;
        }

        self.add_simple_table(&table, x, y)
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
        assert_eq!(minimal.font_size, 10.0);

        let simple = TableStyle::simple();
        assert_eq!(simple.font_size, 10.0);

        let professional = TableStyle::professional();
        assert!(professional.header_background.is_some());

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
}
