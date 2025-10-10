//! Table renderer for converting advanced tables to PDF content

use super::cell_style::{BorderConfiguration, BorderStyle, CellAlignment, CellStyle};
use super::header_builder::HeaderBuilder;
use super::table_builder::{AdvancedTable, CellData, RowData};
use crate::error::PdfError;
use crate::graphics::Color;
use crate::page::Page;
use crate::text::{measure_text, Font};

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

    /// Calculate the total height needed to render a table
    ///
    /// This is essential for intelligent positioning and layout management.
    /// Returns the height in points from bottom to top of the rendered table.
    pub fn calculate_table_height(&self, table: &AdvancedTable) -> f64 {
        let mut total_height = 0.0;

        // Calculate header height
        if table.hide_header == false {
            if let Some(header) = &table.header {
                // For complex headers, calculate based on levels and row spans
                total_height += header.calculate_height();
            } else if !table.columns.is_empty() {
                // Simple header from column definitions
                total_height += self.default_header_height;
            }
        }

        // Calculate rows height
        for row in &table.rows {
            let row_height = row.min_height.unwrap_or(self.default_row_height);

            // Account for row spans if needed
            let max_rowspan = row.cells.iter().map(|cell| cell.rowspan).max().unwrap_or(1);

            if max_rowspan > 1 {
                // For multi-row spanning cells, the height is distributed
                // This is a simplified calculation - full implementation would
                // need to track overlapping spans
                total_height += row_height * max_rowspan as f64;
            } else {
                total_height += row_height;
            }
        }

        // Add small buffer for table borders
        if table.table_border {
            total_height += 2.0; // Top and bottom borders
        }

        total_height
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
        table.validate().map_err(PdfError::InvalidOperation)?;

        let mut current_y = y;

        // Render header if present
        if table.hide_header == false {
            if let Some(header) = &table.header {
                current_y = self.render_header(page, table, header, x, current_y)?;
            } else if !table.columns.is_empty() {
                // Render simple header from column definitions
                current_y = self.render_simple_header(page, table, x, current_y)?;
            }
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

        for level in header.levels.iter() {
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
    #[allow(clippy::too_many_arguments)]
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

    /// Truncate text to fit within a specified width, adding ellipsis if needed
    fn truncate_text_to_width(
        &self,
        text: &str,
        max_width: f64,
        font: &Font,
        font_size: f64,
    ) -> String {
        // If text already fits, return as-is
        let full_width = measure_text(text, font.clone(), font_size);
        if full_width <= max_width {
            return text.to_string();
        }

        // If even ellipsis doesn't fit, return empty string
        let ellipsis = "...";
        let ellipsis_width = measure_text(ellipsis, font.clone(), font_size);
        if ellipsis_width > max_width {
            return String::new();
        }

        // If exactly ellipsis width, return ellipsis
        if ellipsis_width == max_width {
            return ellipsis.to_string();
        }

        // Binary search to find the maximum text that fits with ellipsis
        let available_width = max_width - ellipsis_width;
        let chars: Vec<char> = text.chars().collect();

        let mut left = 0;
        let mut right = chars.len();
        let mut best_length = 0;

        while left <= right {
            let mid = (left + right) / 2;
            if mid == 0 {
                break;
            }

            let substring: String = chars[..mid].iter().collect();
            let substring_width = measure_text(&substring, font.clone(), font_size);

            if substring_width <= available_width {
                best_length = mid;
                left = mid + 1;
            } else {
                if mid == 0 {
                    break;
                }
                right = mid - 1;
            }
        }

        if best_length == 0 {
            ellipsis.to_string()
        } else {
            let truncated: String = chars[..best_length].iter().collect();
            format!("{}{}", truncated, ellipsis)
        }
    }

    /// Render text within a cell
    #[allow(clippy::too_many_arguments)]
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
        let text_color = style.text_color.unwrap_or(Color::black());

        // Calculate available width for text (considering padding)
        let available_width = width - style.padding.left - style.padding.right;

        // Truncate text if it doesn't fit within the cell
        let display_text = if available_width > 0.0 {
            self.truncate_text_to_width(content, available_width, &font, font_size)
        } else {
            String::new()
        };

        // Calculate text position based on alignment and padding
        let text_x = match style.alignment {
            CellAlignment::Left => x + style.padding.left,
            CellAlignment::Center => {
                // For center alignment, we need to calculate based on actual text width
                let text_width = measure_text(&display_text, font.clone(), font_size);
                x + style.padding.left + (available_width - text_width) / 2.0
            }
            CellAlignment::Right => {
                let text_width = measure_text(&display_text, font.clone(), font_size);
                x + width - style.padding.right - text_width
            }
            CellAlignment::Justify => x + style.padding.left,
        };

        // Vertically center with padding applied
        let text_y = style
            .padding
            .pad_vertically(&page.coordinate_system(), y + height / 2.0);

        // Only render text if we have something to display
        if !display_text.is_empty() {
            let text_obj = page
                .text()
                .set_font(font, font_size)
                .set_fill_color(text_color);

            text_obj.at(text_x, text_y).write(&display_text)?;
        }

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::text::Font;

    #[test]
    fn test_truncate_text_to_width_no_truncation_needed() {
        let renderer = TableRenderer::new();
        let text = "Short";
        let max_width = 100.0;
        let font = Font::Helvetica;
        let font_size = 12.0;

        let result = renderer.truncate_text_to_width(text, max_width, &font, font_size);
        assert_eq!(result, "Short");
    }

    #[test]
    fn test_truncate_text_to_width_with_truncation() {
        let renderer = TableRenderer::new();
        let text = "This is a very long text that should be truncated";
        let max_width = 50.0; // Very narrow width
        let font = Font::Helvetica;
        let font_size = 12.0;

        let result = renderer.truncate_text_to_width(text, max_width, &font, font_size);
        assert!(result.ends_with("..."));
        assert!(result.len() < text.len());

        // Verify the truncated text fits within the width
        let truncated_width = measure_text(&result, font, font_size);
        assert!(truncated_width <= max_width);
    }

    #[test]
    fn test_truncate_text_to_width_empty_when_too_narrow() {
        let renderer = TableRenderer::new();
        let text = "Any text";
        let max_width = 5.0; // Too narrow even for ellipsis
        let font = Font::Helvetica;
        let font_size = 12.0;

        let result = renderer.truncate_text_to_width(text, max_width, &font, font_size);
        assert_eq!(result, "");
    }

    #[test]
    fn test_truncate_text_to_width_exactly_ellipsis_width() {
        let renderer = TableRenderer::new();
        let text = "Some text";
        let font = Font::Helvetica;
        let font_size = 12.0;

        // Calculate width that exactly fits ellipsis
        let ellipsis_width = measure_text("...", font.clone(), font_size);

        let result = renderer.truncate_text_to_width(text, ellipsis_width, &font, font_size);
        assert_eq!(result, "...");
    }

    #[test]
    fn test_truncate_text_to_width_single_character() {
        let renderer = TableRenderer::new();
        let text = "A";
        let max_width = 50.0;
        let font = Font::Helvetica;
        let font_size = 12.0;

        let result = renderer.truncate_text_to_width(text, max_width, &font, font_size);
        assert_eq!(result, "A");
    }

    #[test]
    fn test_truncate_text_to_width_different_fonts() {
        let renderer = TableRenderer::new();
        let text = "This text will be truncated";
        let max_width = 60.0;
        let font_size = 12.0;

        // Test with different fonts
        let helvetica_result =
            renderer.truncate_text_to_width(text, max_width, &Font::Helvetica, font_size);
        let courier_result =
            renderer.truncate_text_to_width(text, max_width, &Font::Courier, font_size);
        let times_result =
            renderer.truncate_text_to_width(text, max_width, &Font::TimesRoman, font_size);

        // All should be truncated and fit within width
        for result in [&helvetica_result, &courier_result, &times_result] {
            assert!(result.ends_with("..."));
            assert!(result.len() < text.len());
        }

        // Courier (monospace) might have different truncation point
        // All results should be valid and within width limits
        assert!(!helvetica_result.is_empty());
        assert!(!courier_result.is_empty());
        assert!(!times_result.is_empty());
    }

    #[test]
    fn test_truncate_text_to_width_empty_input() {
        let renderer = TableRenderer::new();
        let text = "";
        let max_width = 100.0;
        let font = Font::Helvetica;
        let font_size = 12.0;

        let result = renderer.truncate_text_to_width(text, max_width, &font, font_size);
        assert_eq!(result, "");
    }

    #[test]
    fn test_truncate_text_to_width_unicode_characters() {
        let renderer = TableRenderer::new();
        let text = "Héllö Wørld with ümlauts and émojis 🚀🎉";
        let max_width = 80.0;
        let font = Font::Helvetica;
        let font_size = 12.0;

        let result = renderer.truncate_text_to_width(text, max_width, &font, font_size);

        // Should handle unicode properly
        if result != text {
            assert!(result.ends_with("..."));
        }

        // Verify width constraint
        let result_width = measure_text(&result, font, font_size);
        assert!(result_width <= max_width);
    }
}
