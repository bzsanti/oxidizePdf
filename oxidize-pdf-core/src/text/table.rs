//! Simple table rendering for PDF documents
//!
//! This module provides basic table functionality without CSS styling,
//! suitable for structured data presentation in PDF documents.

use crate::error::{ensure_finite, PdfError};
use crate::graphics::{Color, GraphicsContext, LineDashPattern};
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
    /// Grid layout options
    pub grid_style: GridStyle,
    /// Cell border style
    pub cell_border_style: CellBorderStyle,
    /// Alternating row colors
    pub alternating_row_colors: Option<(Color, Color)>,
    /// Table background color
    pub background_color: Option<Color>,
    /// When the table is split across pages by `Document::add_paginated_table`,
    /// repeat header rows at the top of every continuation page. Defaults to `true`.
    pub repeat_header_on_split: bool,
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
    /// Optional per-row height (overrides global row_height)
    row_height: Option<f64>,
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
    /// Row span (default 1)
    rowspan: usize,
    /// Cell background color (overrides row color)
    background_color: Option<Color>,
    /// Cell border style (overrides table default)
    border_style: Option<CellBorderStyle>,
}

/// Grid layout style for tables
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GridStyle {
    /// No grid lines
    None,
    /// Only horizontal lines
    Horizontal,
    /// Only vertical lines
    Vertical,
    /// Full grid with all lines
    Full,
    /// Only outer borders
    Outline,
}

/// Cell border style options
#[derive(Debug, Clone)]
pub struct CellBorderStyle {
    /// Border width
    pub width: f64,
    /// Border color
    pub color: Color,
    /// Dash pattern (None for solid)
    pub dash_pattern: Option<LineDashPattern>,
}

impl Default for CellBorderStyle {
    fn default() -> Self {
        Self {
            width: 1.0,
            color: Color::black(),
            dash_pattern: None,
        }
    }
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
            grid_style: GridStyle::Full,
            cell_border_style: CellBorderStyle::default(),
            alternating_row_colors: None,
            background_color: None,
            repeat_header_on_split: true,
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

    /// Get a reference to the table's current options.
    pub fn options(&self) -> &TableOptions {
        &self.options
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
                rowspan: 1,
                background_color: None,
                border_style: None,
            })
            .collect();

        self.rows.push(TableRow {
            cells: row_cells,
            is_header: true,
            row_height: None,
        });

        Ok(self)
    }

    /// Set the height of the last added row
    pub fn set_last_row_height(&mut self, height: f64) -> &mut Self {
        if let Some(row) = self.rows.last_mut() {
            row.row_height = Some(height);
        }
        self
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
                rowspan: 1,
                background_color: None,
                border_style: None,
            })
            .collect();

        self.rows.push(TableRow {
            cells: row_cells,
            is_header: false,
            row_height: None,
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
            row_height: None,
        });

        Ok(self)
    }

    /// Calculate the height of a row
    fn calculate_row_height(&self, row: &TableRow) -> f64 {
        // Priority: per-row height > global options height > auto
        if let Some(h) = row.row_height {
            return h;
        }
        if self.options.row_height > 0.0 {
            return self.options.row_height;
        }

        // Auto height: consider multi-line content
        let line_height = self.options.font_size * 1.2;
        let max_lines = row
            .cells
            .iter()
            .map(|cell| cell.content.split('\n').count())
            .max()
            .unwrap_or(1);

        if max_lines <= 1 {
            // Single line: font size + padding
            self.options.font_size + (self.options.cell_padding * 2.0)
        } else {
            // Multi-line: first line height + additional lines + padding
            self.options.font_size
                + ((max_lines - 1) as f64 * line_height)
                + (self.options.cell_padding * 2.0)
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

    /// Number of rows in this table (header + data).
    pub fn row_count(&self) -> usize {
        self.rows.len()
    }

    /// Number of leading header rows in this table.
    pub fn header_count(&self) -> usize {
        self.rows.iter().take_while(|r| r.is_header).count()
    }

    /// Current top-left position of the table, `(x, y)`.
    pub fn position(&self) -> (f64, f64) {
        self.position
    }

    /// Prepend a clone of the leading header rows from `source` to this table.
    ///
    /// Used by `Document::add_paginated_table` to repeat headers on continuation
    /// pages when `TableOptions::repeat_header_on_split` is true. No-op when
    /// `source` has zero header rows.
    ///
    /// Crate-private: callers outside this crate have no use case for this and
    /// passing a `source` with mismatched `column_widths` would yield malformed
    /// rendering.
    pub(crate) fn prepend_headers_from(&mut self, source: &Table) {
        let header_count = source.header_count();
        if header_count == 0 {
            return;
        }
        let mut new_rows: Vec<TableRow> = source.rows[..header_count].to_vec();
        new_rows.extend(self.rows.drain(..));
        self.rows = new_rows;
    }

    /// Render the table to a graphics context.
    ///
    /// Back-compat note: this method is **vertical-overflow-unaware** by design.
    /// Rows past the page boundary are still drawn off-page (silent overflow).
    /// Callers concerned with overflow should use [`Table::render_with_split`]
    /// or [`Table::render_strict`], or [`crate::page_tables::DocumentTables::add_paginated_table`].
    pub fn render(&self, graphics: &mut GraphicsContext) -> Result<(), PdfError> {
        // Direct call to the row-drawing helper with all rows — preserves the
        // pre-#218 silent-overflow behaviour callers depend on, and skips the
        // boundary `ensure_finite` check (which the safe APIs apply).
        self.render_rows_slice(graphics, &self.rows, self.get_height())
    }

    /// Render as many leading rows as fully fit above `bottom_y`; return the
    /// unrendered tail as a fresh [`Table`] (with the same `column_widths` and
    /// `options`), or `None` when everything fit.
    ///
    /// **Tail position is a sentinel `(start_x, 0.0)`** — the caller MUST call
    /// [`Table::set_position`] on the returned tail before using it for
    /// rendering or fit checks. Calling `render_with_split` on a tail without
    /// repositioning will report 0 rows fitting (since `start_y - row_height < bottom_y`
    /// for any positive `bottom_y`) and yield a tail identical to the input.
    /// For batteries-included pagination, see
    /// [`crate::page_tables::DocumentTables::add_paginated_table`].
    ///
    /// # Errors
    ///
    /// Returns [`PdfError::InvalidStructure`] when `bottom_y` is not a finite
    /// value (NaN or ±∞).
    pub fn render_with_split(
        &self,
        graphics: &mut GraphicsContext,
        bottom_y: f64,
    ) -> Result<Option<Table>, PdfError> {
        ensure_finite("bottom_y", bottom_y)?;
        let (start_x, _start_y) = self.position;

        // Pre-flight: how many leading rows fully fit above the floor?
        let rendered_count = self.fit_count(bottom_y);
        let rendered_height = self.rows[..rendered_count]
            .iter()
            .map(|r| self.calculate_row_height(r))
            .sum::<f64>();

        if rendered_count > 0 {
            self.render_rows_slice(graphics, &self.rows[..rendered_count], rendered_height)?;
        }

        if rendered_count == self.rows.len() {
            Ok(None)
        } else {
            let mut tail = self.clone();
            tail.rows = self.rows[rendered_count..].to_vec();
            tail.position = (start_x, 0.0);
            Ok(Some(tail))
        }
    }

    /// Strict variant: pre-flight check the table against `bottom_y`. If any
    /// row would overflow, return [`PdfError::TableOverflow`] **without
    /// drawing anything**; otherwise render normally.
    ///
    /// # Errors
    ///
    /// Returns [`PdfError::InvalidStructure`] when `bottom_y` is not finite.
    pub fn render_strict(
        &self,
        graphics: &mut GraphicsContext,
        bottom_y: f64,
    ) -> Result<(), PdfError> {
        ensure_finite("bottom_y", bottom_y)?;
        let rendered = self.fit_count(bottom_y);
        if rendered < self.rows.len() {
            return Err(PdfError::TableOverflow {
                rendered,
                dropped: self.rows.len() - rendered,
                bottom_y,
            });
        }
        // Skip the redundant fit_count inside render() by going straight to
        // the helper with the full row set.
        self.render_rows_slice(graphics, &self.rows, self.get_height())
    }

    /// Count the number of leading rows that fully fit above `bottom_y`.
    fn fit_count(&self, bottom_y: f64) -> usize {
        let (_start_x, start_y) = self.position;
        let mut current_y = start_y;
        let mut count = 0usize;
        for row in &self.rows {
            let row_height = self.calculate_row_height(row);
            let next_y = current_y - row_height;
            if next_y < bottom_y {
                break;
            }
            count += 1;
            current_y = next_y;
        }
        count
    }

    /// Internal: draw a slice of rows starting at `self.position`. The
    /// `rendered_height` parameter sizes the optional table-wide background.
    fn render_rows_slice(
        &self,
        graphics: &mut GraphicsContext,
        rows: &[TableRow],
        rendered_height: f64,
    ) -> Result<(), PdfError> {
        let (start_x, start_y) = self.position;
        let mut current_y = start_y;

        // Draw table background if specified, sized to the rendered subset.
        if let Some(bg_color) = self.options.background_color {
            graphics.save_state();
            graphics.set_fill_color(bg_color);
            graphics.rectangle(
                start_x,
                start_y - rendered_height,
                self.get_width(),
                rendered_height,
            );
            graphics.fill();
            graphics.restore_state();
        }

        // Draw each row
        let mut data_row_index: usize = 0; // Counts only non-header rows (for zebra stripes)
        for (row_index, row) in rows.iter().enumerate() {
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

                // Cell rectangle bottom-left Y (table grows downward)
                let cell_rect_y = current_y - row_height;

                // Draw cell background
                // First priority: cell-specific background
                if let Some(cell_bg) = cell.background_color {
                    graphics.save_state();
                    graphics.set_fill_color(cell_bg);
                    graphics.rectangle(current_x, cell_rect_y, cell_width, row_height);
                    graphics.fill();
                    graphics.restore_state();
                }
                // Second priority: header style background
                else if use_header_style {
                    if let Some(style) = header_style {
                        graphics.save_state();
                        graphics.set_fill_color(style.background_color);
                        graphics.rectangle(current_x, cell_rect_y, cell_width, row_height);
                        graphics.fill();
                        graphics.restore_state();
                    }
                }
                // Third priority: alternating row colors
                else if let Some((even_color, odd_color)) = self.options.alternating_row_colors {
                    if !row.is_header {
                        let color = if data_row_index % 2 == 0 {
                            even_color
                        } else {
                            odd_color
                        };
                        graphics.save_state();
                        graphics.set_fill_color(color);
                        graphics.rectangle(current_x, cell_rect_y, cell_width, row_height);
                        graphics.fill();
                        graphics.restore_state();
                    }
                }

                // Draw cell border based on grid style
                let should_draw_border = match self.options.grid_style {
                    GridStyle::None => false,
                    GridStyle::Full => true,
                    GridStyle::Horizontal => {
                        // Draw top and bottom borders only
                        true
                    }
                    GridStyle::Vertical => {
                        // Draw left and right borders only
                        true
                    }
                    GridStyle::Outline => {
                        // Only draw if it's an edge cell.
                        // For partial renders (page splits), the outline tracks the
                        // boundary of the rendered slice, not the original full table.
                        col_index == 0
                            || col_index + cell.colspan >= self.column_widths.len()
                            || row_index == 0
                            || row_index == rows.len() - 1
                    }
                };

                if should_draw_border {
                    graphics.save_state();

                    // Use cell-specific border style if available
                    let border_style = cell
                        .border_style
                        .as_ref()
                        .unwrap_or(&self.options.cell_border_style);

                    graphics.set_stroke_color(border_style.color);
                    graphics.set_line_width(border_style.width);

                    // Apply dash pattern if specified
                    if let Some(dash_pattern) = &border_style.dash_pattern {
                        graphics.set_line_dash_pattern(dash_pattern.clone());
                    }

                    // Draw borders based on grid style
                    match self.options.grid_style {
                        GridStyle::Full | GridStyle::Outline => {
                            graphics.rectangle(current_x, cell_rect_y, cell_width, row_height);
                            graphics.stroke();
                        }
                        GridStyle::Horizontal => {
                            // Top border
                            graphics.move_to(current_x, current_y);
                            graphics.line_to(current_x + cell_width, current_y);
                            // Bottom border
                            graphics.move_to(current_x, cell_rect_y);
                            graphics.line_to(current_x + cell_width, cell_rect_y);
                            graphics.stroke();
                        }
                        GridStyle::Vertical => {
                            // Left border
                            graphics.move_to(current_x, current_y);
                            graphics.line_to(current_x, cell_rect_y);
                            // Right border
                            graphics.move_to(current_x + cell_width, current_y);
                            graphics.line_to(current_x + cell_width, cell_rect_y);
                            graphics.stroke();
                        }
                        GridStyle::None => {}
                    }

                    graphics.restore_state();
                }

                // Draw cell text
                // Text baseline: near top of cell, offset by padding and font size
                let text_x = current_x + self.options.cell_padding;
                let text_y = current_y - self.options.cell_padding - self.options.font_size;
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

                // Split content by newlines for multi-line support
                let lines: Vec<&str> = cell.content.split('\n').collect();
                let line_height = self.options.font_size * 1.2;

                // Determine font for measurement (needed for Center/Right alignment)
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

                // Draw each line with alignment
                for (line_idx, line) in lines.iter().enumerate() {
                    let line_y = text_y - (line_idx as f64 * line_height);

                    let line_x = match cell.align {
                        TextAlign::Center => {
                            let measured =
                                measure_text(line, &font_to_measure, self.options.font_size);
                            text_x + (text_width - measured) / 2.0
                        }
                        TextAlign::Right => {
                            let measured =
                                measure_text(line, &font_to_measure, self.options.font_size);
                            text_x + text_width - measured
                        }
                        TextAlign::Left | TextAlign::Justified => text_x,
                    };

                    graphics.begin_text();
                    graphics.set_text_position(line_x, line_y);
                    graphics.show_text(line)?;
                    graphics.end_text();
                }

                graphics.restore_state();

                current_x += cell_width;
                col_index += cell.colspan;
            }

            if !row.is_header {
                data_row_index += 1;
            }
            current_y -= row_height;
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
            row_height: None,
        }
    }

    /// Create a header row
    #[allow(dead_code)]
    pub fn header(cells: Vec<TableCell>) -> Self {
        Self {
            cells,
            is_header: true,
            row_height: None,
        }
    }

    /// Set the height for this specific row (overrides global row_height)
    pub fn set_row_height(&mut self, height: f64) -> &mut Self {
        self.row_height = Some(height);
        self
    }
}

impl TableCell {
    /// Create a new cell with content
    pub fn new(content: String) -> Self {
        Self {
            content,
            align: TextAlign::Left,
            colspan: 1,
            rowspan: 1,
            background_color: None,
            border_style: None,
        }
    }

    /// Create a cell with specific alignment
    pub fn with_align(content: String, align: TextAlign) -> Self {
        Self {
            content,
            align,
            colspan: 1,
            rowspan: 1,
            background_color: None,
            border_style: None,
        }
    }

    /// Create a cell with colspan
    pub fn with_colspan(content: String, colspan: usize) -> Self {
        Self {
            content,
            align: TextAlign::Left,
            colspan,
            rowspan: 1,
            background_color: None,
            border_style: None,
        }
    }

    /// Set cell background color
    pub fn set_background_color(&mut self, color: Color) -> &mut Self {
        self.background_color = Some(color);
        self
    }

    /// Set cell border style
    pub fn set_border_style(&mut self, style: CellBorderStyle) -> &mut Self {
        self.border_style = Some(style);
        self
    }

    /// Set rowspan
    pub fn set_rowspan(&mut self, rowspan: usize) -> &mut Self {
        self.rowspan = rowspan;
        self
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
        assert_eq!(options.grid_style, GridStyle::Full);
        assert!(options.alternating_row_colors.is_none());
        assert!(options.background_color.is_none());
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

    #[test]
    fn test_grid_styles() {
        let mut options = TableOptions::default();

        options.grid_style = GridStyle::None;
        assert_eq!(options.grid_style, GridStyle::None);

        options.grid_style = GridStyle::Horizontal;
        assert_eq!(options.grid_style, GridStyle::Horizontal);

        options.grid_style = GridStyle::Vertical;
        assert_eq!(options.grid_style, GridStyle::Vertical);

        options.grid_style = GridStyle::Outline;
        assert_eq!(options.grid_style, GridStyle::Outline);
    }

    #[test]
    fn test_cell_border_style() {
        let style = CellBorderStyle::default();
        assert_eq!(style.width, 1.0);
        assert_eq!(style.color, Color::black());
        assert!(style.dash_pattern.is_none());

        let custom_style = CellBorderStyle {
            width: 2.0,
            color: Color::rgb(1.0, 0.0, 0.0),
            dash_pattern: Some(LineDashPattern::new(vec![5.0, 3.0], 0.0)),
        };
        assert_eq!(custom_style.width, 2.0);
        assert!(custom_style.dash_pattern.is_some());
    }

    #[test]
    fn test_table_with_alternating_colors() {
        let mut table = Table::new(vec![100.0, 100.0]);
        table.options.alternating_row_colors = Some((Color::gray(0.95), Color::gray(0.9)));

        table
            .add_row(vec!["Row 1".to_string(), "Data 1".to_string()])
            .unwrap();
        table
            .add_row(vec!["Row 2".to_string(), "Data 2".to_string()])
            .unwrap();

        assert_eq!(table.rows.len(), 2);
        assert!(table.options.alternating_row_colors.is_some());
    }

    #[test]
    fn test_cell_with_background() {
        let mut cell = TableCell::new("Test".to_string());
        cell.set_background_color(Color::rgb(0.0, 1.0, 0.0));

        assert!(cell.background_color.is_some());
        assert_eq!(cell.background_color.unwrap(), Color::rgb(0.0, 1.0, 0.0));
    }

    #[test]
    fn test_cell_with_custom_border() {
        let mut cell = TableCell::new("Test".to_string());
        let border_style = CellBorderStyle {
            width: 2.0,
            color: Color::rgb(0.0, 0.0, 1.0),
            dash_pattern: None,
        };
        cell.set_border_style(border_style);

        assert!(cell.border_style.is_some());
        let style = cell.border_style.as_ref().unwrap();
        assert_eq!(style.width, 2.0);
        assert_eq!(style.color, Color::rgb(0.0, 0.0, 1.0));
    }
}
