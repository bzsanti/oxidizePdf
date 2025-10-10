//! Advanced table builder for creating complex tables with professional styling

use super::cell_style::CellStyle;
use super::header_builder::HeaderBuilder;
use crate::graphics::Color;
use std::collections::HashMap;

/// Column definition for advanced tables
#[derive(Debug, Clone)]
pub struct Column {
    /// Column header text
    pub header: String,
    /// Column width in points
    pub width: f64,
    /// Default style for cells in this column
    pub default_style: Option<CellStyle>,
    /// Whether this column can resize automatically
    pub auto_resize: bool,
    /// Minimum width for auto-resizing columns
    pub min_width: Option<f64>,
    /// Maximum width for auto-resizing columns
    pub max_width: Option<f64>,
}

impl Column {
    /// Create a new column
    pub fn new<S: Into<String>>(header: S, width: f64) -> Self {
        Self {
            header: header.into(),
            width,
            default_style: None,
            auto_resize: false,
            min_width: None,
            max_width: None,
        }
    }

    /// Set default style for this column
    pub fn with_style(mut self, style: CellStyle) -> Self {
        self.default_style = Some(style);
        self
    }

    /// Enable auto-resizing for this column
    pub fn auto_resize(mut self, min_width: Option<f64>, max_width: Option<f64>) -> Self {
        self.auto_resize = true;
        self.min_width = min_width;
        self.max_width = max_width;
        self
    }
}

/// Cell data with optional styling and spanning
#[derive(Debug, Clone)]
pub struct CellData {
    /// Cell content
    pub content: String,
    /// Optional custom style for this cell
    pub style: Option<CellStyle>,
    /// Number of columns this cell spans (1 = no spanning)
    pub colspan: usize,
    /// Number of rows this cell spans (1 = no spanning)
    pub rowspan: usize,
}

impl CellData {
    /// Create a new cell with text content
    pub fn new<S: Into<String>>(content: S) -> Self {
        Self {
            content: content.into(),
            style: None,
            colspan: 1,
            rowspan: 1,
        }
    }

    /// Set custom style for this cell
    pub fn with_style(mut self, style: CellStyle) -> Self {
        self.style = Some(style);
        self
    }

    /// Set column span for this cell
    pub fn colspan(mut self, span: usize) -> Self {
        self.colspan = span.max(1);
        self
    }

    /// Set row span for this cell
    pub fn rowspan(mut self, span: usize) -> Self {
        self.rowspan = span.max(1);
        self
    }
}

/// Row data with optional styling
#[derive(Debug, Clone)]
pub struct RowData {
    /// Cells in this row
    pub cells: Vec<CellData>,
    /// Optional style for the entire row
    pub style: Option<CellStyle>,
    /// Minimum height for this row
    pub min_height: Option<f64>,
}

impl RowData {
    /// Create a new row from string content
    pub fn from_strings(content: Vec<&str>) -> Self {
        let cells = content.into_iter().map(CellData::new).collect();

        Self {
            cells,
            style: None,
            min_height: None,
        }
    }

    /// Create a new row from cell data
    pub fn from_cells(cells: Vec<CellData>) -> Self {
        Self {
            cells,
            style: None,
            min_height: None,
        }
    }

    /// Set style for the entire row
    pub fn with_style(mut self, style: CellStyle) -> Self {
        self.style = Some(style);
        self
    }

    /// Set minimum height for this row
    pub fn min_height(mut self, height: f64) -> Self {
        self.min_height = Some(height);
        self
    }
}

/// Complete advanced table configuration
#[derive(Debug, Clone)]
pub struct AdvancedTable {
    /// Table title
    pub title: Option<String>,
    /// X position on page
    pub x: f64,
    /// Y position on page
    pub y: f64,
    /// Table columns definition
    pub columns: Vec<Column>,
    /// Table rows data
    pub rows: Vec<RowData>,
    /// Header configuration
    pub header: Option<HeaderBuilder>,
    /// Whether to show the table header
    pub show_header: bool,
    /// Default cell style
    pub default_style: CellStyle,
    /// Header style
    pub header_style: CellStyle,
    /// Zebra striping configuration
    pub zebra_striping: Option<ZebraConfig>,
    /// Zebra stripe color
    pub zebra_color: Option<Color>,
    /// Zebra stripes enabled
    pub zebra_stripes: bool,
    /// Table-wide border style
    pub table_border: bool,
    /// Spacing between cells
    pub cell_spacing: f64,
    /// Total table width (auto-calculated if None)
    pub total_width: Option<f64>,
    /// Whether to repeat headers on page breaks
    pub repeat_headers: bool,
    /// Styles for specific cells (row, col) -> style
    pub cell_styles: HashMap<(usize, usize), CellStyle>,
}

/// Zebra striping configuration
#[derive(Debug, Clone)]
pub struct ZebraConfig {
    /// Background color for odd rows
    pub odd_color: Option<Color>,
    /// Background color for even rows
    pub even_color: Option<Color>,
    /// Start with odd or even row
    pub start_with_odd: bool,
}

impl ZebraConfig {
    /// Create zebra striping with alternating colors
    pub fn new(odd_color: Option<Color>, even_color: Option<Color>) -> Self {
        Self {
            odd_color,
            even_color,
            start_with_odd: true,
        }
    }

    /// Simple zebra striping with one alternating color
    pub fn simple(color: Color) -> Self {
        Self::new(Some(color), None)
    }

    /// Get color for a specific row
    pub fn get_color_for_row(&self, row_index: usize) -> Option<Color> {
        let is_odd = (row_index % 2) == (if self.start_with_odd { 1 } else { 0 });
        if is_odd {
            self.odd_color
        } else {
            self.even_color
        }
    }
}

/// Builder for creating advanced tables with fluent API
pub struct AdvancedTableBuilder {
    table: AdvancedTable,
}

impl AdvancedTableBuilder {
    /// Create a new table builder
    pub fn new() -> Self {
        Self {
            table: AdvancedTable {
                title: None,
                x: 0.0,
                y: 0.0,
                columns: Vec::new(),
                rows: Vec::new(),
                header: None,
                show_header: true,
                default_style: CellStyle::data(),
                header_style: CellStyle::header(),
                zebra_striping: None,
                zebra_color: None,
                zebra_stripes: false,
                table_border: true,
                cell_spacing: 0.0,
                total_width: None,
                repeat_headers: false,
                cell_styles: HashMap::new(),
            },
        }
    }

    /// Add a column to the table
    pub fn add_column<S: Into<String>>(mut self, header: S, width: f64) -> Self {
        self.table.columns.push(Column::new(header, width));
        self
    }

    /// Add a column with custom styling
    pub fn add_styled_column<S: Into<String>>(
        mut self,
        header: S,
        width: f64,
        style: CellStyle,
    ) -> Self {
        self.table
            .columns
            .push(Column::new(header, width).with_style(style));
        self
    }

    /// Set columns from a list of headers with equal widths
    pub fn columns_equal_width(mut self, headers: Vec<&str>, total_width: f64) -> Self {
        let column_width = total_width / headers.len() as f64;
        self.table.columns = headers
            .into_iter()
            .map(|header| Column::new(header, column_width))
            .collect();
        self.table.total_width = Some(total_width);
        self
    }

    /// Add a simple row from string content
    pub fn add_row(mut self, content: Vec<&str>) -> Self {
        self.table.rows.push(RowData::from_strings(content));
        self
    }

    pub fn add_row_with_min_height(mut self, content: Vec<&str>, min_height: f64) -> Self {
        self.table
            .rows
            .push(RowData::from_strings(content).min_height(min_height));
        self
    }

    /// Add a row with cell data
    pub fn add_row_cells(mut self, cells: Vec<CellData>) -> Self {
        self.table.rows.push(RowData::from_cells(cells));
        self
    }

    /// Add a styled row
    pub fn add_styled_row(mut self, content: Vec<&str>, style: CellStyle) -> Self {
        self.table
            .rows
            .push(RowData::from_strings(content).with_style(style));
        self
    }

    /// Set default cell style
    pub fn default_style(mut self, style: CellStyle) -> Self {
        self.table.default_style = style;
        self
    }

    /// Set data cell style (alias for default_style)
    pub fn data_style(mut self, style: CellStyle) -> Self {
        self.table.default_style = style;
        self
    }

    /// Set header style
    pub fn header_style(mut self, style: CellStyle) -> Self {
        self.table.header_style = style;
        self
    }

    /// Control header visibility
    pub fn show_header(mut self, show: bool) -> Self {
        self.table.show_header = show;
        self
    }

    /// Set table title
    pub fn title<S: Into<String>>(mut self, title: S) -> Self {
        self.table.title = Some(title.into());
        self
    }

    /// Set table columns from (header, width) tuples
    pub fn columns(mut self, column_specs: Vec<(&str, f64)>) -> Self {
        self.table.columns = column_specs
            .into_iter()
            .map(|(header, width)| Column::new(header, width))
            .collect();
        self
    }

    /// Set table position on page
    pub fn position(mut self, x: f64, y: f64) -> Self {
        self.table.x = x;
        self.table.y = y;
        self
    }

    /// Add a complex header using HeaderBuilder
    pub fn complex_header(mut self, header: HeaderBuilder) -> Self {
        // Auto-generate columns from header if table has no columns
        if self.table.columns.is_empty() {
            let column_count = header.total_columns;
            for i in 0..column_count {
                self.table.columns.push(Column::new(
                    format!("Column {}", i + 1),
                    100.0, // Default width
                ));
            }
        }
        self.table.header = Some(header);
        self
    }

    /// Enable zebra stripes
    pub fn zebra_stripes(mut self, enabled: bool, color: Color) -> Self {
        self.table.zebra_stripes = enabled;
        self.table.zebra_color = Some(color);
        if enabled {
            self.table.zebra_striping = Some(ZebraConfig::simple(color));
        } else {
            self.table.zebra_striping = None;
        }
        self
    }

    /// Add row with custom style
    pub fn add_row_with_style(mut self, content: Vec<&str>, style: CellStyle) -> Self {
        let mut row = RowData::from_strings(content);
        row = row.with_style(style);
        self.table.rows.push(row);
        self
    }

    /// Add row with mixed cell styles
    pub fn add_row_with_mixed_styles(mut self, cells: Vec<(CellStyle, &str)>) -> Self {
        let cell_data: Vec<CellData> = cells
            .into_iter()
            .map(|(style, content)| CellData::new(content.to_string()).with_style(style))
            .collect();
        self.table.rows.push(RowData::from_cells(cell_data));
        self
    }

    /// Build with error handling (for compatibility with tests)
    pub fn build(self) -> Result<AdvancedTable, String> {
        if self.table.columns.is_empty() {
            return Err("Table must have at least one column".to_string());
        }
        Ok(self.table)
    }

    /// Enable zebra striping
    pub fn zebra_striping(mut self, color: Color) -> Self {
        self.table.zebra_striping = Some(ZebraConfig::simple(color));
        self
    }

    /// Enable custom zebra striping
    pub fn zebra_striping_custom(mut self, config: ZebraConfig) -> Self {
        self.table.zebra_striping = Some(config);
        self
    }

    /// Enable or disable table border
    pub fn table_border(mut self, enabled: bool) -> Self {
        self.table.table_border = enabled;
        self
    }

    /// Set cell spacing
    pub fn cell_spacing(mut self, spacing: f64) -> Self {
        self.table.cell_spacing = spacing;
        self
    }

    /// Set total table width
    pub fn total_width(mut self, width: f64) -> Self {
        self.table.total_width = Some(width);
        self
    }

    /// Enable header repetition on page breaks
    pub fn repeat_headers(mut self, repeat: bool) -> Self {
        self.table.repeat_headers = repeat;
        self
    }

    /// Set style for a specific cell
    pub fn set_cell_style(mut self, row: usize, col: usize, style: CellStyle) -> Self {
        self.table.cell_styles.insert((row, col), style);
        self
    }

    /// Add bulk data from a 2D vector
    pub fn add_data(mut self, data: Vec<Vec<&str>>) -> Self {
        for row in data {
            self = self.add_row(row);
        }
        self
    }

    /// Create a financial table with common styling
    pub fn financial_table(self) -> Self {
        self.header_style(
            CellStyle::header()
                .background_color(Color::rgb(0.2, 0.4, 0.8))
                .text_color(Color::white()),
        )
        .default_style(CellStyle::data())
        .zebra_striping(Color::rgb(0.97, 0.97, 0.97))
        .table_border(true)
    }

    /// Create a data table with minimal styling
    pub fn minimal_table(self) -> Self {
        self.header_style(
            CellStyle::new()
                .font_size(12.0)
                .background_color(Color::rgb(0.95, 0.95, 0.95)),
        )
        .default_style(CellStyle::data())
        .table_border(false)
        .cell_spacing(2.0)
    }
}

impl Default for AdvancedTableBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl AdvancedTable {
    /// Get the total calculated width of the table
    pub fn calculate_width(&self) -> f64 {
        if let Some(width) = self.total_width {
            width
        } else {
            self.columns.iter().map(|col| col.width).sum()
        }
    }

    /// Get the number of rows (excluding headers)
    pub fn row_count(&self) -> usize {
        self.rows.len()
    }

    /// Get the number of columns
    pub fn column_count(&self) -> usize {
        self.columns.len()
    }

    /// Get style for a specific cell, considering row/column defaults and zebra striping
    pub fn get_cell_style(&self, row: usize, col: usize) -> CellStyle {
        // Priority: specific cell style > row style > column style > zebra striping > default

        // Check specific cell style
        if let Some(cell_style) = self.cell_styles.get(&(row, col)) {
            return cell_style.clone();
        }

        // Check row style
        if let Some(row_data) = self.rows.get(row) {
            if let Some(row_style) = &row_data.style {
                return row_style.clone();
            }
        }

        // Check column style
        if let Some(column) = self.columns.get(col) {
            if let Some(column_style) = &column.default_style {
                let mut style = column_style.clone();

                // Apply zebra striping if configured
                if let Some(zebra) = &self.zebra_striping {
                    if let Some(color) = zebra.get_color_for_row(row) {
                        style.background_color = Some(color);
                    }
                }

                return style;
            }
        }

        // Apply zebra striping to default style
        let mut style = self.default_style.clone();
        if let Some(zebra) = &self.zebra_striping {
            if let Some(color) = zebra.get_color_for_row(row) {
                style.background_color = Some(color);
            }
        }

        style
    }

    /// Validate table structure (e.g., consistent column counts)
    pub fn validate(&self) -> Result<(), String> {
        let expected_cols = self.columns.len();

        for (row_idx, row) in self.rows.iter().enumerate() {
            if row.cells.len() != expected_cols {
                return Err(format!(
                    "Row {} has {} cells but expected {} columns",
                    row_idx,
                    row.cells.len(),
                    expected_cols
                ));
            }
        }

        Ok(())
    }
}
