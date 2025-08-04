//! Simple table rendering for PDF documents
//!
//! This module provides basic table functionality without CSS styling,
//! suitable for structured data presentation in PDF documents.

use crate::error::PdfError;
use crate::graphics::{Color, GraphicsContext};
use crate::text::{measure_text, Font, TextAlign};

/// Represents a simple table in a PDF document
#[derive(Debug, Clone)]
pub struct Table {
    /// Table rows
    rows: Vec<TableRow>,
    /// Column widths (in points)
    column_widths: Vec<f64>,
    /// Table position (x, y)
    position: (f64, f64),
    /// Table options
    options: TableOptions,
}

/// Options for table rendering
#[derive(Debug, Clone)]
pub struct TableOptions {
    /// Border width in points
    pub border_width: f64,
    /// Border color
    pub border_color: Color,
    /// Cell padding in points
    pub cell_padding: f64,
    /// Row height in points (0 for auto)
    pub row_height: f64,
    /// Font for table text
    pub font: Font,
    /// Font size in points
    pub font_size: f64,
    /// Text color
    pub text_color: Color,
    /// Header row styling
    pub header_style: Option<HeaderStyle>,
}

/// Header row styling options
#[derive(Debug, Clone)]
pub struct HeaderStyle {
    /// Background color for header cells
    pub background_color: Color,
    /// Text color for header cells
    pub text_color: Color,
    /// Font for header text
    pub font: Font,
    /// Make header text bold
    pub bold: bool,
}

/// Represents a row in the table
#[derive(Debug, Clone)]
pub struct TableRow {
    /// Cells in this row
    cells: Vec<TableCell>,
    /// Whether this is a header row
    is_header: bool,
}

/// Represents a cell in the table
#[derive(Debug, Clone)]
pub struct TableCell {
    /// Cell content
    content: String,
    /// Text alignment
    align: TextAlign,
    /// Column span (default 1)
    colspan: usize,
}

impl Default for TableOptions {
    fn default() -> Self {
        Self {
            border_width: 1.0,
            border_color: Color::black(),
            cell_padding: 5.0,
            row_height: 0.0, // Auto
            font: Font::Helvetica,
            font_size: 10.0,
            text_color: Color::black(),
            header_style: None,
        }
    }
}

impl Table {
    /// Create a new table with specified column widths
    pub fn new(column_widths: Vec<f64>) -> Self {
        Self {
            rows: Vec::new(),
            column_widths,
            position: (0.0, 0.0),
            options: TableOptions::default(),
        }
    }

    /// Create a table with equal column widths
    pub fn with_equal_columns(num_columns: usize, total_width: f64) -> Self {
        let column_width = total_width / num_columns as f64;
        let column_widths = vec![column_width; num_columns];
        Self::new(column_widths)
    }

    /// Set table position
    pub fn set_position(&mut self, x: f64, y: f64) -> &mut Self {
        self.position = (x, y);
        self
    }

    /// Set table options
    pub fn set_options(&mut self, options: TableOptions) -> &mut Self {
        self.options = options;
        self
    }

    /// Add a header row
    pub fn add_header_row(&mut self, cells: Vec<String>) -> Result<&mut Self, PdfError> {
        if cells.len() != self.column_widths.len() {
            return Err(PdfError::InvalidStructure(
                "Header cells count doesn't match column count".to_string(),
            ));
        }

        let row_cells: Vec<TableCell> = cells
            .into_iter()
            .map(|content| TableCell {
                content,
                align: TextAlign::Center,
                colspan: 1,
            })
            .collect();

        self.rows.push(TableRow {
            cells: row_cells,
            is_header: true,
        });

        Ok(self)
    }

    /// Add a data row
    pub fn add_row(&mut self, cells: Vec<String>) -> Result<&mut Self, PdfError> {
        self.add_row_with_alignment(cells, TextAlign::Left)
    }

    /// Add a data row with specific alignment
    pub fn add_row_with_alignment(
        &mut self,
        cells: Vec<String>,
        align: TextAlign,
    ) -> Result<&mut Self, PdfError> {
        if cells.len() != self.column_widths.len() {
            return Err(PdfError::InvalidStructure(
                "Row cells count doesn't match column count".to_string(),
            ));
        }

        let row_cells: Vec<TableCell> = cells
            .into_iter()
            .map(|content| TableCell {
                content,
                align,
                colspan: 1,
            })
            .collect();

        self.rows.push(TableRow {
            cells: row_cells,
            is_header: false,
        });

        Ok(self)
    }

    /// Add a row with custom cells (allows colspan)
    pub fn add_custom_row(&mut self, cells: Vec<TableCell>) -> Result<&mut Self, PdfError> {
        // Validate total colspan matches column count
        let total_colspan: usize = cells.iter().map(|c| c.colspan).sum();
        if total_colspan != self.column_widths.len() {
            return Err(PdfError::InvalidStructure(
                "Total colspan doesn't match column count".to_string(),
            ));
        }

        self.rows.push(TableRow {
            cells,
            is_header: false,
        });

        Ok(self)
    }

    /// Calculate the height of a row
    fn calculate_row_height(&self, _row: &TableRow) -> f64 {
        if self.options.row_height > 0.0 {
            self.options.row_height
        } else {
            // Auto height: font size + padding
            self.options.font_size + (self.options.cell_padding * 2.0)
        }
    }

    /// Get total table height
    pub fn get_height(&self) -> f64 {
        self.rows
            .iter()
            .map(|row| self.calculate_row_height(row))
            .sum()
    }

    /// Get total table width
    pub fn get_width(&self) -> f64 {
        self.column_widths.iter().sum()
    }

    /// Render the table to a graphics context
    pub fn render(&self, graphics: &mut GraphicsContext) -> Result<(), PdfError> {
        let (start_x, start_y) = self.position;
        let mut current_y = start_y;

        // Draw each row
        for row in self.rows.iter() {
            let row_height = self.calculate_row_height(row);
            let mut current_x = start_x;

            // Determine if we should use header styling
            let use_header_style = row.is_header && self.options.header_style.is_some();
            let header_style = self.options.header_style.as_ref();

            // Draw cells
            let mut col_index = 0;
            for cell in &row.cells {
                // Calculate cell width (considering colspan)
                let mut cell_width = 0.0;
                for i in 0..cell.colspan {
                    if col_index + i < self.column_widths.len() {
                        cell_width += self.column_widths[col_index + i];
                    }
                }

                // Draw cell background if header
                if use_header_style {
                    if let Some(style) = header_style {
                        graphics.save_state();
                        graphics.set_fill_color(style.background_color);
                        graphics.rectangle(current_x, current_y, cell_width, row_height);
                        graphics.fill();
                        graphics.restore_state();
                    }
                }

                // Draw cell border
                graphics.save_state();
                graphics.set_stroke_color(self.options.border_color);
                graphics.set_line_width(self.options.border_width);
                graphics.rectangle(current_x, current_y, cell_width, row_height);
                graphics.stroke();
                graphics.restore_state();

                // Draw cell text
                let text_x = current_x + self.options.cell_padding;
                let text_y =
                    current_y + row_height - self.options.cell_padding - self.options.font_size;
                let text_width = cell_width - (2.0 * self.options.cell_padding);

                graphics.save_state();

                // Set font and color
                if use_header_style {
                    if let Some(style) = header_style {
                        let font = if style.bold {
                            match style.font {
                                Font::Helvetica => Font::HelveticaBold,
                                Font::TimesRoman => Font::TimesBold,
                                Font::Courier => Font::CourierBold,
                                _ => style.font.clone(),
                            }
                        } else {
                            style.font.clone()
                        };
                        graphics.set_font(font, self.options.font_size);
                        graphics.set_fill_color(style.text_color);
                    }
                } else {
                    graphics.set_font(self.options.font.clone(), self.options.font_size);
                    graphics.set_fill_color(self.options.text_color);
                }

                // Draw text with alignment
                match cell.align {
                    TextAlign::Left => {
                        graphics.begin_text();
                        graphics.set_text_position(text_x, text_y);
                        graphics.show_text(&cell.content)?;
                        graphics.end_text();
                    }
                    TextAlign::Center => {
                        // Determine which font to use based on header style
                        let font_to_measure = if use_header_style {
                            if let Some(style) = header_style {
                                if style.bold {
                                    match style.font {
                                        Font::Helvetica => Font::HelveticaBold,
                                        Font::TimesRoman => Font::TimesBold,
                                        Font::Courier => Font::CourierBold,
                                        _ => style.font.clone(),
                                    }
                                } else {
                                    style.font.clone()
                                }
                            } else {
                                self.options.font.clone()
                            }
                        } else {
                            self.options.font.clone()
                        };
                        
                        let text_width_measured = measure_text(&cell.content, font_to_measure, self.options.font_size);
                        let centered_x = text_x + (text_width - text_width_measured) / 2.0;
                        graphics.begin_text();
                        graphics.set_text_position(centered_x, text_y);
                        graphics.show_text(&cell.content)?;
                        graphics.end_text();
                    }
                    TextAlign::Right => {
                        // Determine which font to use based on header style
                        let font_to_measure = if use_header_style {
                            if let Some(style) = header_style {
                                if style.bold {
                                    match style.font {
                                        Font::Helvetica => Font::HelveticaBold,
                                        Font::TimesRoman => Font::TimesBold,
                                        Font::Courier => Font::CourierBold,
                                        _ => style.font.clone(),
                                    }
                                } else {
                                    style.font.clone()
                                }
                            } else {
                                self.options.font.clone()
                            }
                        } else {
                            self.options.font.clone()
                        };
                        
                        let text_width_measured = measure_text(&cell.content, font_to_measure, self.options.font_size);
                        let right_x = text_x + text_width - text_width_measured;
                        graphics.begin_text();
                        graphics.set_text_position(right_x, text_y);
                        graphics.show_text(&cell.content)?;
                        graphics.end_text();
                    }
                    TextAlign::Justified => {
                        // For simple tables, treat justified as left-aligned
                        graphics.begin_text();
                        graphics.set_text_position(text_x, text_y);
                        graphics.show_text(&cell.content)?;
                        graphics.end_text();
                    }
                }

                graphics.restore_state();

                current_x += cell_width;
                col_index += cell.colspan;
            }

            current_y += row_height;
        }

        Ok(())
    }
}

impl TableRow {
    /// Create a new row with cells
    #[allow(dead_code)]
    pub fn new(cells: Vec<TableCell>) -> Self {
        Self {
            cells,
            is_header: false,
        }
    }

    /// Create a header row
    #[allow(dead_code)]
    pub fn header(cells: Vec<TableCell>) -> Self {
        Self {
            cells,
            is_header: true,
        }
    }
}

impl TableCell {
    /// Create a new cell with content
    pub fn new(content: String) -> Self {
        Self {
            content,
            align: TextAlign::Left,
            colspan: 1,
        }
    }

    /// Create a cell with specific alignment
    pub fn with_align(content: String, align: TextAlign) -> Self {
        Self {
            content,
            align,
            colspan: 1,
        }
    }

    /// Create a cell with colspan
    pub fn with_colspan(content: String, colspan: usize) -> Self {
        Self {
            content,
            align: TextAlign::Left,
            colspan,
        }
    }

    /// Set cell alignment
    pub fn set_align(&mut self, align: TextAlign) -> &mut Self {
        self.align = align;
        self
    }

    /// Set cell colspan
    pub fn set_colspan(&mut self, colspan: usize) -> &mut Self {
        self.colspan = colspan;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_table_creation() {
        let table = Table::new(vec![100.0, 150.0, 200.0]);
        assert_eq!(table.column_widths.len(), 3);
        assert_eq!(table.rows.len(), 0);
    }

    #[test]
    fn test_table_equal_columns() {
        let table = Table::with_equal_columns(4, 400.0);
        assert_eq!(table.column_widths.len(), 4);
        assert_eq!(table.column_widths[0], 100.0);
        assert_eq!(table.get_width(), 400.0);
    }

    #[test]
    fn test_add_header_row() {
        let mut table = Table::new(vec![100.0, 100.0, 100.0]);
        let result = table.add_header_row(vec![
            "Name".to_string(),
            "Age".to_string(),
            "City".to_string(),
        ]);
        assert!(result.is_ok());
        assert_eq!(table.rows.len(), 1);
        assert!(table.rows[0].is_header);
    }

    #[test]
    fn test_add_row_mismatch() {
        let mut table = Table::new(vec![100.0, 100.0]);
        let result = table.add_row(vec![
            "John".to_string(),
            "25".to_string(),
            "NYC".to_string(),
        ]);
        assert!(result.is_err());
    }

    #[test]
    fn test_table_cell_creation() {
        let cell = TableCell::new("Test".to_string());
        assert_eq!(cell.content, "Test");
        assert_eq!(cell.align, TextAlign::Left);
        assert_eq!(cell.colspan, 1);
    }

    #[test]
    fn test_table_cell_with_colspan() {
        let cell = TableCell::with_colspan("Merged".to_string(), 3);
        assert_eq!(cell.content, "Merged");
        assert_eq!(cell.colspan, 3);
    }

    #[test]
    fn test_custom_row_colspan_validation() {
        let mut table = Table::new(vec![100.0, 100.0, 100.0]);
        let cells = vec![
            TableCell::new("Normal".to_string()),
            TableCell::with_colspan("Merged".to_string(), 2),
        ];
        let result = table.add_custom_row(cells);
        assert!(result.is_ok());
        assert_eq!(table.rows.len(), 1);
    }

    #[test]
    fn test_custom_row_invalid_colspan() {
        let mut table = Table::new(vec![100.0, 100.0, 100.0]);
        let cells = vec![
            TableCell::new("Normal".to_string()),
            TableCell::with_colspan("Merged".to_string(), 3), // Total would be 4
        ];
        let result = table.add_custom_row(cells);
        assert!(result.is_err());
    }

    #[test]
    fn test_table_options_default() {
        let options = TableOptions::default();
        assert_eq!(options.border_width, 1.0);
        assert_eq!(options.border_color, Color::black());
        assert_eq!(options.cell_padding, 5.0);
        assert_eq!(options.font_size, 10.0);
    }

    #[test]
    fn test_header_style() {
        let style = HeaderStyle {
            background_color: Color::gray(0.9),
            text_color: Color::black(),
            font: Font::HelveticaBold,
            bold: true,
        };
        assert_eq!(style.background_color, Color::gray(0.9));
        assert!(style.bold);
    }

    #[test]
    fn test_table_dimensions() {
        let mut table = Table::new(vec![100.0, 150.0, 200.0]);
        table.options.row_height = 20.0;

        table
            .add_row(vec!["A".to_string(), "B".to_string(), "C".to_string()])
            .unwrap();
        table
            .add_row(vec!["D".to_string(), "E".to_string(), "F".to_string()])
            .unwrap();

        assert_eq!(table.get_width(), 450.0);
        assert_eq!(table.get_height(), 40.0);
    }

    #[test]
    fn test_table_position() {
        let mut table = Table::new(vec![100.0]);
        table.set_position(50.0, 100.0);
        assert_eq!(table.position, (50.0, 100.0));
    }

    #[test]
    fn test_row_with_alignment() {
        let mut table = Table::new(vec![100.0, 100.0]);
        let result = table.add_row_with_alignment(
            vec!["Left".to_string(), "Right".to_string()],
            TextAlign::Right,
        );
        assert!(result.is_ok());
        assert_eq!(table.rows[0].cells[0].align, TextAlign::Right);
    }

    #[test]
    fn test_table_cell_setters() {
        let mut cell = TableCell::new("Test".to_string());
        cell.set_align(TextAlign::Center).set_colspan(2);
        assert_eq!(cell.align, TextAlign::Center);
        assert_eq!(cell.colspan, 2);
    }

    #[test]
    fn test_auto_row_height() {
        let table = Table::new(vec![100.0]);
        let row = TableRow::new(vec![TableCell::new("Test".to_string())]);
        let height = table.calculate_row_height(&row);
        assert_eq!(height, 20.0); // font_size (10) + padding*2 (5*2)
    }

    #[test]
    fn test_fixed_row_height() {
        let mut table = Table::new(vec![100.0]);
        table.options.row_height = 30.0;
        let row = TableRow::new(vec![TableCell::new("Test".to_string())]);
        let height = table.calculate_row_height(&row);
        assert_eq!(height, 30.0);
    }
}
