//! Table renderer for converting advanced tables to PDF content

use super::cell_style::{BorderConfiguration, BorderStyle, CellAlignment, CellStyle};
use super::header_builder::HeaderBuilder;
use super::table_builder::{AdvancedTable, CellData, RowData};
use crate::error::PdfError;
use crate::graphics::Color;
use crate::page::Page;
use crate::text::Font;

/// Renderer for advanced tables
pub struct TableRenderer {
    /// Default row height when not specified
    pub default_row_height: f64,
    /// Default header height
    pub default_header_height: f64,
    /// Whether to auto-calculate cell heights based on content
    pub auto_height: bool,
}

impl TableRenderer {
    /// Create a new table renderer
    pub fn new() -> Self {
        Self {
            default_row_height: 25.0,
            default_header_height: 30.0,
            auto_height: true,
        }
    }

    /// Render a table to a PDF page
    pub fn render_table(
        &self,
        page: &mut Page,
        table: &AdvancedTable,
        x: f64,
        y: f64,
    ) -> Result<f64, PdfError> {
        // Validate table structure
        table
            .validate()
            .map_err(|e| PdfError::InvalidOperation(e))?;

        let mut current_y = y;

        // Render header if present
        if let Some(header) = &table.header {
            current_y = self.render_header(page, table, header, x, current_y)?;
        } else if !table.columns.is_empty() {
            // Render simple header from column definitions
            current_y = self.render_simple_header(page, table, x, current_y)?;
        }

        // Render table rows
        current_y = self.render_rows(page, table, x, current_y)?;

        // Render table border if enabled
        if table.table_border {
            self.render_table_border(page, table, x, y, current_y)?;
        }

        Ok(current_y)
    }

    /// Render table headers
    fn render_header(
        &self,
        page: &mut Page,
        table: &AdvancedTable,
        header: &HeaderBuilder,
        x: f64,
        start_y: f64,
    ) -> Result<f64, PdfError> {
        let mut current_y = start_y;
        let column_positions = self.calculate_column_positions(table, x);

        for (_level_idx, level) in header.levels.iter().enumerate() {
            let row_height = self.default_header_height;

            for cell in level {
                let cell_x = column_positions[cell.start_col];
                let cell_width = self.calculate_span_width(table, cell.start_col, cell.colspan);
                let cell_height = row_height * cell.rowspan as f64;

                let style = cell.style.as_ref().unwrap_or(&table.header_style);

                self.render_cell(
                    page,
                    &cell.text,
                    cell_x,
                    current_y - cell_height,
                    cell_width,
                    cell_height,
                    style,
                )?;
            }

            current_y -= row_height;
        }

        Ok(current_y)
    }

    /// Render simple header from column definitions
    fn render_simple_header(
        &self,
        page: &mut Page,
        table: &AdvancedTable,
        x: f64,
        start_y: f64,
    ) -> Result<f64, PdfError> {
        let column_positions = self.calculate_column_positions(table, x);
        let header_height = self.default_header_height;

        for (col_idx, column) in table.columns.iter().enumerate() {
            let cell_x = column_positions[col_idx];
            let cell_width = column.width;

            self.render_cell(
                page,
                &column.header,
                cell_x,
                start_y - header_height,
                cell_width,
                header_height,
                &table.header_style,
            )?;
        }

        Ok(start_y - header_height)
    }

    /// Render table data rows
    fn render_rows(
        &self,
        page: &mut Page,
        table: &AdvancedTable,
        x: f64,
        start_y: f64,
    ) -> Result<f64, PdfError> {
        let mut current_y = start_y;
        let column_positions = self.calculate_column_positions(table, x);

        for (row_idx, row) in table.rows.iter().enumerate() {
            let row_height = row.min_height.unwrap_or(self.default_row_height);

            for (col_idx, cell) in row.cells.iter().enumerate() {
                let cell_x = column_positions[col_idx];
                let cell_width = self.calculate_span_width(table, col_idx, cell.colspan);
                let cell_height = row_height * cell.rowspan as f64;

                let style = self.resolve_cell_style(table, row, cell, row_idx, col_idx);

                self.render_cell(
                    page,
                    &cell.content,
                    cell_x,
                    current_y - cell_height,
                    cell_width,
                    cell_height,
                    &style,
                )?;
            }

            current_y -= row_height;
        }

        Ok(current_y)
    }

    /// Render an individual cell
    fn render_cell(
        &self,
        page: &mut Page,
        content: &str,
        x: f64,
        y: f64,
        width: f64,
        height: f64,
        style: &CellStyle,
    ) -> Result<(), PdfError> {
        // Draw background if specified
        if let Some(bg_color) = style.background_color {
            page.graphics()
                .save_state()
                .set_fill_color(bg_color)
                .rectangle(x, y, width, height)
                .fill()
                .restore_state();
        }

        // Draw borders
        self.render_cell_borders(page, x, y, width, height, &style.border)?;

        // Draw text content
        if !content.is_empty() {
            self.render_cell_text(page, content, x, y, width, height, style)?;
        }

        Ok(())
    }

    /// Render cell borders
    fn render_cell_borders(
        &self,
        page: &mut Page,
        x: f64,
        y: f64,
        width: f64,
        height: f64,
        border_config: &BorderConfiguration,
    ) -> Result<(), PdfError> {
        let graphics = page.graphics();

        // Top border
        if border_config.top.style != BorderStyle::None {
            graphics
                .save_state()
                .set_stroke_color(border_config.top.color)
                .set_line_width(border_config.top.width);

            self.apply_line_style(graphics, border_config.top.style);

            graphics
                .move_to(x, y + height)
                .line_to(x + width, y + height)
                .stroke()
                .restore_state();
        }

        // Bottom border
        if border_config.bottom.style != BorderStyle::None {
            graphics
                .save_state()
                .set_stroke_color(border_config.bottom.color)
                .set_line_width(border_config.bottom.width);

            self.apply_line_style(graphics, border_config.bottom.style);

            graphics
                .move_to(x, y)
                .line_to(x + width, y)
                .stroke()
                .restore_state();
        }

        // Left border
        if border_config.left.style != BorderStyle::None {
            graphics
                .save_state()
                .set_stroke_color(border_config.left.color)
                .set_line_width(border_config.left.width);

            self.apply_line_style(graphics, border_config.left.style);

            graphics
                .move_to(x, y)
                .line_to(x, y + height)
                .stroke()
                .restore_state();
        }

        // Right border
        if border_config.right.style != BorderStyle::None {
            graphics
                .save_state()
                .set_stroke_color(border_config.right.color)
                .set_line_width(border_config.right.width);

            self.apply_line_style(graphics, border_config.right.style);

            graphics
                .move_to(x + width, y)
                .line_to(x + width, y + height)
                .stroke()
                .restore_state();
        }

        Ok(())
    }

    /// Apply line style for borders
    fn apply_line_style(
        &self,
        _graphics: &mut crate::graphics::GraphicsContext,
        _style: BorderStyle,
    ) {
        // TODO: Implement line styles when GraphicsContext supports dash patterns
        // For now, all borders will be solid
    }

    /// Render text within a cell
    fn render_cell_text(
        &self,
        page: &mut Page,
        content: &str,
        x: f64,
        y: f64,
        width: f64,
        height: f64,
        style: &CellStyle,
    ) -> Result<(), PdfError> {
        let font = style.font.clone().unwrap_or(Font::Helvetica);
        let font_size = style.font_size.unwrap_or(12.0);
        let text_color = style.text_color.clone().unwrap_or(Color::black());

        // Calculate text position based on alignment and padding
        let text_x = match style.alignment {
            CellAlignment::Left => x + style.padding.left,
            CellAlignment::Center => x + width / 2.0,
            CellAlignment::Right => x + width - style.padding.right,
            CellAlignment::Justify => x + style.padding.left,
        };

        let text_y = y + height / 2.0; // Vertically center for now

        let text_obj = page
            .text()
            .set_font(font, font_size)
            .set_fill_color(text_color);

        // Simplified text rendering - position and write
        // TODO: Implement proper alignment when TextContext supports it
        text_obj.at(text_x, text_y).write(content)?;

        Ok(())
    }

    /// Calculate column positions based on widths
    fn calculate_column_positions(&self, table: &AdvancedTable, start_x: f64) -> Vec<f64> {
        let mut positions = Vec::new();
        let mut current_x = start_x;

        for column in &table.columns {
            positions.push(current_x);
            current_x += column.width + table.cell_spacing;
        }

        positions
    }

    /// Calculate width for a cell that spans multiple columns
    fn calculate_span_width(&self, table: &AdvancedTable, start_col: usize, colspan: usize) -> f64 {
        let mut total_width = 0.0;

        for i in 0..colspan {
            if let Some(column) = table.columns.get(start_col + i) {
                total_width += column.width;
                if i > 0 {
                    total_width += table.cell_spacing;
                }
            }
        }

        total_width
    }

    /// Resolve the effective style for a cell
    fn resolve_cell_style(
        &self,
        table: &AdvancedTable,
        _row: &RowData,
        cell: &CellData,
        row_idx: usize,
        col_idx: usize,
    ) -> CellStyle {
        // Priority: cell style > specific cell style > row style > column style > table default

        if let Some(cell_style) = &cell.style {
            return cell_style.clone();
        }

        table.get_cell_style(row_idx, col_idx)
    }

    /// Render table border
    fn render_table_border(
        &self,
        page: &mut Page,
        table: &AdvancedTable,
        x: f64,
        start_y: f64,
        end_y: f64,
    ) -> Result<(), PdfError> {
        let total_width = table.calculate_width();
        let height = start_y - end_y;

        page.graphics()
            .save_state()
            .set_stroke_color(Color::black())
            .set_line_width(1.0)
            .rectangle(x, end_y, total_width, height)
            .stroke()
            .restore_state();

        Ok(())
    }
}

impl Default for TableRenderer {
    fn default() -> Self {
        Self::new()
    }
}
