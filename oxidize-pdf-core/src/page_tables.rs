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

    // ==================== TableStyle Tests ====================

    #[test]
    fn test_table_style_minimal() {
        let style = TableStyle::minimal();
        assert_eq!(style.header_background, None);
        assert_eq!(style.header_text_color, None);
        assert_eq!(style.font_size, 10.0);
    }

    #[test]
    fn test_table_style_simple() {
        let style = TableStyle::simple();
        assert_eq!(style.header_background, None);
        assert_eq!(style.header_text_color, None);
        assert_eq!(style.font_size, 10.0);
    }

    #[test]
    fn test_table_style_professional() {
        let style = TableStyle::professional();
        assert!(style.header_background.is_some());
        assert!(style.header_text_color.is_some());
        assert_eq!(style.font_size, 10.0);

        // Verify dark header background
        if let Some(bg) = style.header_background {
            assert!(bg.r() < 0.2, "Professional header should be dark");
        }

        // Verify white text
        if let Some(text) = style.header_text_color {
            assert_eq!(text, Color::white());
        }
    }

    #[test]
    fn test_table_style_colorful() {
        let style = TableStyle::colorful();
        assert!(style.header_background.is_some());
        assert!(style.header_text_color.is_some());
        assert_eq!(style.font_size, 10.0);

        // Verify blue-ish header background (0.2, 0.4, 0.8)
        if let Some(bg) = style.header_background {
            assert!(bg.b() > bg.r(), "Colorful header should be blue-ish");
            assert!(bg.b() > bg.g(), "Colorful header should be blue-ish");
        }

        // Verify white text
        if let Some(text) = style.header_text_color {
            assert_eq!(text, Color::white());
        }
    }

    #[test]
    fn test_table_style_clone() {
        let original = TableStyle::professional();
        let cloned = original.clone();

        assert_eq!(cloned.header_background, original.header_background);
        assert_eq!(cloned.header_text_color, original.header_text_color);
        assert_eq!(cloned.font_size, original.font_size);
    }

    #[test]
    fn test_table_style_debug() {
        let style = TableStyle::minimal();
        let debug_str = format!("{:?}", style);
        assert!(debug_str.contains("TableStyle"));
    }

    #[test]
    fn test_table_style_mutability() {
        let mut style = TableStyle::minimal();

        style.header_background = Some(Color::red());
        style.header_text_color = Some(Color::blue());
        style.font_size = 14.0;

        assert_eq!(style.header_background, Some(Color::red()));
        assert_eq!(style.header_text_color, Some(Color::blue()));
        assert_eq!(style.font_size, 14.0);
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

    // ==================== Page Integration Tests ====================

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
    fn test_quick_table_with_options() {
        let mut page = Page::a4();

        let data = vec![
            vec!["A".to_string(), "B".to_string()],
            vec!["C".to_string(), "D".to_string()],
        ];

        let options = TableOptions {
            font_size: 12.0,
            ..Default::default()
        };

        let result = page.add_quick_table(data, 50.0, 700.0, 400.0, Some(options));
        assert!(result.is_ok());
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
    fn test_styled_table_minimal() {
        let mut page = Page::a4();

        let headers = vec!["H1".to_string(), "H2".to_string()];
        let data = vec![vec!["V1".to_string(), "V2".to_string()]];

        let result =
            page.add_styled_table(headers, data, 50.0, 700.0, 400.0, TableStyle::minimal());
        assert!(result.is_ok());
    }

    #[test]
    fn test_styled_table_colorful() {
        let mut page = Page::a4();

        let headers = vec!["Header".to_string()];
        let data = vec![vec!["Value".to_string()]];

        let result =
            page.add_styled_table(headers, data, 50.0, 700.0, 300.0, TableStyle::colorful());
        assert!(result.is_ok());
    }

    #[test]
    fn test_styled_table_empty_headers() {
        let mut page = Page::a4();

        let headers: Vec<String> = vec![];
        let data = vec![vec!["Data".to_string()]];

        // Empty headers should return Ok (early return)
        let result = page.add_styled_table(headers, data, 50.0, 700.0, 400.0, TableStyle::simple());
        assert!(result.is_ok());
    }

    #[test]
    fn test_styled_table_empty_data() {
        let mut page = Page::a4();

        let headers = vec!["H1".to_string(), "H2".to_string()];
        let data: Vec<Vec<String>> = vec![];

        // Headers only, no data rows
        let result = page.add_styled_table(
            headers,
            data,
            50.0,
            700.0,
            400.0,
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
    fn test_single_cell_table() {
        let mut page = Page::a4();

        let data = vec![vec!["Single".to_string()]];
        let result = page.add_quick_table(data, 50.0, 700.0, 200.0, None);
        assert!(result.is_ok());
    }

    #[test]
    fn test_single_row_table() {
        let mut page = Page::a4();

        let data = vec![vec![
            "A".to_string(),
            "B".to_string(),
            "C".to_string(),
            "D".to_string(),
        ]];
        let result = page.add_quick_table(data, 50.0, 700.0, 500.0, None);
        assert!(result.is_ok());
    }

    #[test]
    fn test_single_column_table() {
        let mut page = Page::a4();

        let data = vec![
            vec!["Row 1".to_string()],
            vec!["Row 2".to_string()],
            vec!["Row 3".to_string()],
        ];
        let result = page.add_quick_table(data, 50.0, 700.0, 150.0, None);
        assert!(result.is_ok());
    }

    #[test]
    fn test_many_rows_table() {
        let mut page = Page::a4();

        let data: Vec<Vec<String>> = (0..50)
            .map(|i| vec![format!("Row {}", i), format!("Value {}", i)])
            .collect();

        let result = page.add_quick_table(data, 50.0, 700.0, 400.0, None);
        assert!(result.is_ok());
    }

    #[test]
    fn test_many_columns_table() {
        let mut page = Page::a4();

        let headers: Vec<String> = (0..10).map(|i| format!("Col {}", i)).collect();
        let data = vec![(0..10).map(|i| format!("V{}", i)).collect()];

        let result = page.add_styled_table(headers, data, 50.0, 700.0, 550.0, TableStyle::simple());
        assert!(result.is_ok());
    }

    #[test]
    fn test_table_at_different_positions() {
        let mut page = Page::a4();

        let data = vec![vec!["Test".to_string()]];

        // Top-left
        let result = page.add_quick_table(data.clone(), 0.0, 800.0, 100.0, None);
        assert!(result.is_ok());

        // Center-ish
        let result = page.add_quick_table(data.clone(), 200.0, 400.0, 100.0, None);
        assert!(result.is_ok());

        // Bottom-right area
        let result = page.add_quick_table(data, 400.0, 100.0, 100.0, None);
        assert!(result.is_ok());
    }

    #[test]
    fn test_styled_table_with_only_header_background() {
        let mut page = Page::a4();

        let mut style = TableStyle::minimal();
        style.header_background = Some(Color::green());
        // header_text_color remains None

        let headers = vec!["Test".to_string()];
        let data = vec![vec!["Data".to_string()]];

        let result = page.add_styled_table(headers, data, 50.0, 700.0, 200.0, style);
        assert!(result.is_ok());
    }

    #[test]
    fn test_styled_table_with_only_header_text_color() {
        let mut page = Page::a4();

        let mut style = TableStyle::minimal();
        style.header_text_color = Some(Color::red());
        // header_background remains None

        let headers = vec!["Test".to_string()];
        let data = vec![vec!["Data".to_string()]];

        let result = page.add_styled_table(headers, data, 50.0, 700.0, 200.0, style);
        assert!(result.is_ok());
    }

    #[test]
    fn test_styled_table_custom_font_size() {
        let mut page = Page::a4();

        let mut style = TableStyle::professional();
        style.font_size = 16.0;

        let headers = vec!["Big".to_string(), "Text".to_string()];
        let data = vec![vec!["Large".to_string(), "Font".to_string()]];

        let result = page.add_styled_table(headers, data, 50.0, 700.0, 300.0, style);
        assert!(result.is_ok());
    }

    #[test]
    fn test_all_styles_integration() {
        let mut page = Page::a4();

        let headers = vec!["A".to_string(), "B".to_string()];
        let data = vec![vec!["1".to_string(), "2".to_string()]];

        let styles = vec![
            TableStyle::minimal(),
            TableStyle::simple(),
            TableStyle::professional(),
            TableStyle::colorful(),
        ];

        for (i, style) in styles.into_iter().enumerate() {
            let y = 700.0 - (i as f64 * 100.0);
            let result =
                page.add_styled_table(headers.clone(), data.clone(), 50.0, y, 200.0, style);
            assert!(result.is_ok(), "Failed for style index {}", i);
        }
    }
}
