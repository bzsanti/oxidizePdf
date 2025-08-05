//! Advanced table rendering for PDF documents
//!
//! This module provides enhanced table functionality with additional features:
//! - Cell-specific styling (background color, borders, padding)
//! - Row and column spanning
//! - Automatic column width calculation
//! - Table headers and footers that repeat on page breaks
//! - Alternating row colors
//! - Cell content wrapping
//! - Vertical alignment in cells

use crate::error::PdfError;
use crate::graphics::{Color, GraphicsContext, LineDashPattern};
use crate::text::{Font, TextAlign};

/// Advanced table with enhanced features
#[derive(Debug, Clone)]
pub struct AdvancedTable {
    /// Table rows
    rows: Vec<TableRow>,
    /// Column definitions
    columns: Vec<ColumnDefinition>,
    /// Table position (x, y)
    position: (f64, f64),
    /// Table options
    options: AdvancedTableOptions,
    /// Header rows (repeated on each page)
    header_rows: Vec<TableRow>,
    /// Footer rows (repeated on each page)
    footer_rows: Vec<TableRow>,
}

/// Advanced options for table rendering
#[derive(Debug, Clone)]
pub struct AdvancedTableOptions {
    /// Default border style
    pub border_style: BorderStyle,
    /// Default cell padding
    pub cell_padding: CellPadding,
    /// Default row height (0 for auto)
    pub row_height: f64,
    /// Default font
    pub font: Font,
    /// Default font size
    pub font_size: f64,
    /// Default text color
    pub text_color: Color,
    /// Alternating row colors
    pub alternating_rows: Option<AlternatingRowColors>,
    /// Table-wide background color
    pub background_color: Option<Color>,
    /// Maximum table height before breaking
    pub max_height: Option<f64>,
    /// Spacing between cells
    pub cell_spacing: f64,
    /// Whether to draw outer border
    pub draw_outer_border: bool,
}

/// Column definition with width and default alignment
#[derive(Debug, Clone)]
pub struct ColumnDefinition {
    /// Column width (absolute or relative)
    pub width: ColumnWidth,
    /// Default alignment for cells in this column
    pub default_align: TextAlign,
    /// Minimum width (for auto-width columns)
    pub min_width: Option<f64>,
    /// Maximum width (for auto-width columns)
    pub max_width: Option<f64>,
}

/// Column width specification
#[derive(Debug, Clone)]
pub enum ColumnWidth {
    /// Fixed width in points
    Fixed(f64),
    /// Relative width (percentage of available space)
    Relative(f64),
    /// Automatic width based on content
    Auto,
}

/// Border style for cells
#[derive(Debug, Clone)]
pub struct BorderStyle {
    /// Top border
    pub top: Option<BorderLine>,
    /// Right border
    pub right: Option<BorderLine>,
    /// Bottom border
    pub bottom: Option<BorderLine>,
    /// Left border
    pub left: Option<BorderLine>,
}

/// Individual border line style
#[derive(Debug, Clone)]
pub struct BorderLine {
    /// Line width
    pub width: f64,
    /// Line color
    pub color: Color,
    /// Line style
    pub style: LineStyle,
}

/// Line style for borders
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LineStyle {
    /// Solid line
    Solid,
    /// Dashed line
    Dashed,
    /// Dotted line
    Dotted,
}

/// Cell padding specification
#[derive(Debug, Clone, Copy)]
pub struct CellPadding {
    pub top: f64,
    pub right: f64,
    pub bottom: f64,
    pub left: f64,
}

/// Alternating row color configuration
#[derive(Debug, Clone)]
pub struct AlternatingRowColors {
    /// Color for even rows
    pub even_color: Color,
    /// Color for odd rows
    pub odd_color: Color,
    /// Whether to include header rows in alternation
    pub include_header: bool,
}

/// Table row with advanced features
#[derive(Debug, Clone)]
pub struct TableRow {
    /// Cells in this row
    cells: Vec<AdvancedTableCell>,
    /// Row-specific height (overrides default)
    height: Option<f64>,
    /// Row-specific background color
    background_color: Option<Color>,
    /// Whether this row can be split across pages
    can_split: bool,
}

/// Advanced table cell with styling options
#[derive(Debug, Clone)]
pub struct AdvancedTableCell {
    /// Cell content
    content: CellContent,
    /// Text alignment
    align: TextAlign,
    /// Vertical alignment
    vertical_align: VerticalAlign,
    /// Column span
    colspan: usize,
    /// Row span
    rowspan: usize,
    /// Cell-specific background color
    background_color: Option<Color>,
    /// Cell-specific border style
    border_style: Option<BorderStyle>,
    /// Cell-specific padding
    padding: Option<CellPadding>,
    /// Cell-specific font
    font: Option<Font>,
    /// Cell-specific font size
    font_size: Option<f64>,
    /// Cell-specific text color
    text_color: Option<Color>,
}

/// Cell content types
#[derive(Debug, Clone)]
pub enum CellContent {
    /// Simple text content
    Text(String),
    /// Multiple paragraphs
    Paragraphs(Vec<String>),
}

/// Vertical alignment in cells
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum VerticalAlign {
    Top,
    Middle,
    Bottom,
}

impl Default for AdvancedTableOptions {
    fn default() -> Self {
        Self {
            border_style: BorderStyle::default(),
            cell_padding: CellPadding::uniform(5.0),
            row_height: 0.0,
            font: Font::Helvetica,
            font_size: 10.0,
            text_color: Color::black(),
            alternating_rows: None,
            background_color: None,
            max_height: None,
            cell_spacing: 0.0,
            draw_outer_border: true,
        }
    }
}

impl Default for BorderStyle {
    fn default() -> Self {
        let default_line = BorderLine {
            width: 1.0,
            color: Color::black(),
            style: LineStyle::Solid,
        };
        Self {
            top: Some(default_line.clone()),
            right: Some(default_line.clone()),
            bottom: Some(default_line.clone()),
            left: Some(default_line),
        }
    }
}

impl BorderStyle {
    /// Create a border style with no borders
    pub fn none() -> Self {
        Self {
            top: None,
            right: None,
            bottom: None,
            left: None,
        }
    }

    /// Create a border style with only horizontal lines
    pub fn horizontal_only(width: f64, color: Color) -> Self {
        let line = BorderLine {
            width,
            color,
            style: LineStyle::Solid,
        };
        Self {
            top: Some(line.clone()),
            right: None,
            bottom: Some(line),
            left: None,
        }
    }

    /// Create a border style with only vertical lines
    pub fn vertical_only(width: f64, color: Color) -> Self {
        let line = BorderLine {
            width,
            color,
            style: LineStyle::Solid,
        };
        Self {
            top: None,
            right: Some(line.clone()),
            bottom: None,
            left: Some(line),
        }
    }
}

impl CellPadding {
    /// Create uniform padding on all sides
    pub fn uniform(padding: f64) -> Self {
        Self {
            top: padding,
            right: padding,
            bottom: padding,
            left: padding,
        }
    }

    /// Create padding with horizontal and vertical values
    pub fn symmetric(horizontal: f64, vertical: f64) -> Self {
        Self {
            top: vertical,
            right: horizontal,
            bottom: vertical,
            left: horizontal,
        }
    }
}

impl AdvancedTable {
    /// Create a new advanced table with column definitions
    pub fn new(columns: Vec<ColumnDefinition>) -> Self {
        Self {
            rows: Vec::new(),
            columns,
            position: (0.0, 0.0),
            options: AdvancedTableOptions::default(),
            header_rows: Vec::new(),
            footer_rows: Vec::new(),
        }
    }

    /// Create a table with equal-width columns
    pub fn with_equal_columns(num_columns: usize, total_width: f64) -> Self {
        let column_width = total_width / num_columns as f64;
        let columns = (0..num_columns)
            .map(|_| ColumnDefinition {
                width: ColumnWidth::Fixed(column_width),
                default_align: TextAlign::Left,
                min_width: None,
                max_width: None,
            })
            .collect();
        Self::new(columns)
    }

    /// Set table position
    pub fn set_position(&mut self, x: f64, y: f64) -> &mut Self {
        self.position = (x, y);
        self
    }

    /// Set table options
    pub fn set_options(&mut self, options: AdvancedTableOptions) -> &mut Self {
        self.options = options;
        self
    }

    /// Add a header row (will be repeated on each page)
    pub fn add_header_row(&mut self, cells: Vec<AdvancedTableCell>) -> Result<&mut Self, PdfError> {
        self.validate_row_cells(&cells)?;
        self.header_rows.push(TableRow {
            cells,
            height: None,
            background_color: None,
            can_split: false,
        });
        Ok(self)
    }

    /// Add a footer row (will be repeated on each page)
    pub fn add_footer_row(&mut self, cells: Vec<AdvancedTableCell>) -> Result<&mut Self, PdfError> {
        self.validate_row_cells(&cells)?;
        self.footer_rows.push(TableRow {
            cells,
            height: None,
            background_color: None,
            can_split: false,
        });
        Ok(self)
    }

    /// Add a data row
    pub fn add_row(&mut self, row: TableRow) -> Result<&mut Self, PdfError> {
        self.validate_row_cells(&row.cells)?;
        self.rows.push(row);
        Ok(self)
    }

    /// Add a simple text row
    pub fn add_text_row(&mut self, texts: Vec<String>) -> Result<&mut Self, PdfError> {
        if texts.len() != self.columns.len() {
            return Err(PdfError::InvalidStructure(
                "Text count doesn't match column count".to_string(),
            ));
        }

        let cells: Vec<AdvancedTableCell> = texts
            .into_iter()
            .enumerate()
            .map(|(i, text)| AdvancedTableCell {
                content: CellContent::Text(text),
                align: self.columns[i].default_align,
                vertical_align: VerticalAlign::Middle,
                colspan: 1,
                rowspan: 1,
                background_color: None,
                border_style: None,
                padding: None,
                font: None,
                font_size: None,
                text_color: None,
            })
            .collect();

        self.add_row(TableRow {
            cells,
            height: None,
            background_color: None,
            can_split: true,
        })
    }

    /// Validate that cells match column structure
    fn validate_row_cells(&self, cells: &[AdvancedTableCell]) -> Result<(), PdfError> {
        let total_colspan: usize = cells.iter().map(|c| c.colspan).sum();
        if total_colspan != self.columns.len() {
            return Err(PdfError::InvalidStructure(format!(
                "Total colspan {} doesn't match column count {}",
                total_colspan,
                self.columns.len()
            )));
        }
        Ok(())
    }

    /// Calculate actual column widths based on table width and content
    pub fn calculate_column_widths(&self, available_width: f64) -> Vec<f64> {
        let mut widths = vec![0.0; self.columns.len()];
        let mut fixed_width = 0.0;
        let mut relative_total = 0.0;
        let mut auto_columns = Vec::new();

        // First pass: calculate fixed widths and relative totals
        for (i, col) in self.columns.iter().enumerate() {
            match &col.width {
                ColumnWidth::Fixed(w) => {
                    widths[i] = *w;
                    fixed_width += *w;
                }
                ColumnWidth::Relative(pct) => {
                    relative_total += *pct;
                }
                ColumnWidth::Auto => {
                    auto_columns.push(i);
                }
            }
        }

        // Calculate remaining width for relative and auto columns
        let remaining_width = available_width - fixed_width;

        // Second pass: calculate relative widths
        for (i, col) in self.columns.iter().enumerate() {
            if let ColumnWidth::Relative(pct) = col.width {
                widths[i] = remaining_width * (pct / relative_total);
            }
        }

        // Third pass: distribute remaining width to auto columns
        if !auto_columns.is_empty() {
            let auto_width = remaining_width / auto_columns.len() as f64;
            for &i in &auto_columns {
                widths[i] = auto_width;

                // Apply min/max constraints
                if let Some(min) = self.columns[i].min_width {
                    widths[i] = widths[i].max(min);
                }
                if let Some(max) = self.columns[i].max_width {
                    widths[i] = widths[i].min(max);
                }
            }
        }

        widths
    }

    /// Get total table width
    pub fn get_width(&self, available_width: f64) -> f64 {
        self.calculate_column_widths(available_width).iter().sum()
    }

    /// Calculate row height based on content
    fn calculate_row_height(&self, row: &TableRow, _column_widths: &[f64]) -> f64 {
        if let Some(height) = row.height {
            return height;
        }

        if self.options.row_height > 0.0 {
            return self.options.row_height;
        }

        // Calculate based on content
        let padding = self.options.cell_padding;
        let font_size = self.options.font_size;

        // For now, simple calculation - can be enhanced with text wrapping
        font_size + padding.top + padding.bottom
    }

    /// Render the table to a graphics context
    pub fn render(
        &self,
        graphics: &mut GraphicsContext,
        available_width: f64,
    ) -> Result<(), PdfError> {
        let column_widths = self.calculate_column_widths(available_width);
        let (start_x, mut current_y) = self.position;

        // Draw table background if specified
        if let Some(bg_color) = self.options.background_color {
            let table_width: f64 = column_widths.iter().sum();
            let table_height = self.calculate_total_height(&column_widths);

            graphics.save_state();
            graphics.set_fill_color(bg_color);
            graphics.rectangle(start_x, current_y, table_width, table_height);
            graphics.fill();
            graphics.restore_state();
        }

        // Render header rows
        for header_row in &self.header_rows {
            self.render_row(
                graphics,
                header_row,
                &column_widths,
                start_x,
                &mut current_y,
                None,
            )?;
        }

        // Render data rows
        for (row_index, row) in self.rows.iter().enumerate() {
            let row_color = self.get_row_background_color(row, row_index);
            self.render_row(
                graphics,
                row,
                &column_widths,
                start_x,
                &mut current_y,
                row_color,
            )?;
        }

        // Render footer rows
        for footer_row in &self.footer_rows {
            self.render_row(
                graphics,
                footer_row,
                &column_widths,
                start_x,
                &mut current_y,
                None,
            )?;
        }

        Ok(())
    }

    /// Render a single row
    fn render_row(
        &self,
        graphics: &mut GraphicsContext,
        row: &TableRow,
        column_widths: &[f64],
        start_x: f64,
        current_y: &mut f64,
        row_background: Option<Color>,
    ) -> Result<(), PdfError> {
        let row_height = self.calculate_row_height(row, column_widths);
        let mut current_x = start_x;

        // Draw row background
        if let Some(color) = row_background.or(row.background_color) {
            let row_width: f64 = column_widths.iter().sum();
            graphics.save_state();
            graphics.set_fill_color(color);
            graphics.rectangle(start_x, *current_y, row_width, row_height);
            graphics.fill();
            graphics.restore_state();
        }

        // Draw cells
        let mut col_index = 0;
        for cell in &row.cells {
            let mut cell_width = 0.0;
            for i in 0..cell.colspan {
                if col_index + i < column_widths.len() {
                    cell_width += column_widths[col_index + i];
                }
            }

            self.render_cell(
                graphics, cell, current_x, *current_y, cell_width, row_height,
            )?;

            current_x += cell_width;
            col_index += cell.colspan;
        }

        *current_y += row_height;
        Ok(())
    }

    /// Render a single cell
    fn render_cell(
        &self,
        graphics: &mut GraphicsContext,
        cell: &AdvancedTableCell,
        x: f64,
        y: f64,
        width: f64,
        height: f64,
    ) -> Result<(), PdfError> {
        // Draw cell background
        if let Some(bg_color) = cell.background_color {
            graphics.save_state();
            graphics.set_fill_color(bg_color);
            graphics.rectangle(x, y, width, height);
            graphics.fill();
            graphics.restore_state();
        }

        // Draw cell borders
        let border_style = cell
            .border_style
            .as_ref()
            .unwrap_or(&self.options.border_style);
        self.draw_cell_borders(graphics, border_style, x, y, width, height)?;

        // Draw cell content
        let padding = cell.padding.unwrap_or(self.options.cell_padding);
        let content_x = x + padding.left;
        let content_y = y + padding.bottom;
        let content_width = width - padding.left - padding.right;
        let content_height = height - padding.top - padding.bottom;

        match &cell.content {
            CellContent::Text(text) => {
                self.render_text_content(
                    graphics,
                    text,
                    cell,
                    content_x,
                    content_y,
                    content_width,
                    content_height,
                )?;
            }
            CellContent::Paragraphs(paragraphs) => {
                self.render_paragraphs_content(
                    graphics,
                    paragraphs,
                    cell,
                    content_x,
                    content_y,
                    content_width,
                    content_height,
                )?;
            }
        }

        Ok(())
    }

    /// Draw borders for a cell
    fn draw_cell_borders(
        &self,
        graphics: &mut GraphicsContext,
        border_style: &BorderStyle,
        x: f64,
        y: f64,
        width: f64,
        height: f64,
    ) -> Result<(), PdfError> {
        graphics.save_state();

        // Top border
        if let Some(border) = &border_style.top {
            self.draw_border_line(graphics, border, x, y, x + width, y)?;
        }

        // Right border
        if let Some(border) = &border_style.right {
            self.draw_border_line(graphics, border, x + width, y, x + width, y + height)?;
        }

        // Bottom border
        if let Some(border) = &border_style.bottom {
            self.draw_border_line(graphics, border, x, y + height, x + width, y + height)?;
        }

        // Left border
        if let Some(border) = &border_style.left {
            self.draw_border_line(graphics, border, x, y, x, y + height)?;
        }

        graphics.restore_state();
        Ok(())
    }

    /// Draw a single border line
    fn draw_border_line(
        &self,
        graphics: &mut GraphicsContext,
        border: &BorderLine,
        x1: f64,
        y1: f64,
        x2: f64,
        y2: f64,
    ) -> Result<(), PdfError> {
        graphics.set_stroke_color(border.color);
        graphics.set_line_width(border.width);

        match border.style {
            LineStyle::Solid => {
                graphics.move_to(x1, y1);
                graphics.line_to(x2, y2);
                graphics.stroke();
            }
            LineStyle::Dashed => {
                graphics.set_line_dash_pattern(LineDashPattern::dashed(3.0, 3.0));
                graphics.move_to(x1, y1);
                graphics.line_to(x2, y2);
                graphics.stroke();
                graphics.set_line_solid(); // Reset
            }
            LineStyle::Dotted => {
                graphics.set_line_dash_pattern(LineDashPattern::dotted(1.0, 2.0));
                graphics.move_to(x1, y1);
                graphics.line_to(x2, y2);
                graphics.stroke();
                graphics.set_line_solid(); // Reset
            }
        }

        Ok(())
    }

    /// Render text content in a cell
    #[allow(clippy::too_many_arguments)]
    fn render_text_content(
        &self,
        graphics: &mut GraphicsContext,
        text: &str,
        cell: &AdvancedTableCell,
        x: f64,
        y: f64,
        width: f64,
        height: f64,
    ) -> Result<(), PdfError> {
        graphics.save_state();

        // Set font and color
        let font = cell.font.clone().unwrap_or(self.options.font.clone());
        let font_size = cell.font_size.unwrap_or(self.options.font_size);
        let text_color = cell.text_color.unwrap_or(self.options.text_color);

        graphics.set_font(font, font_size);
        graphics.set_fill_color(text_color);

        // Calculate text position based on alignment
        let text_y = match cell.vertical_align {
            VerticalAlign::Top => y + height - font_size,
            VerticalAlign::Middle => y + (height - font_size) / 2.0,
            VerticalAlign::Bottom => y + font_size,
        };

        // Draw text with horizontal alignment
        match cell.align {
            TextAlign::Left => {
                graphics.begin_text();
                graphics.set_text_position(x, text_y);
                graphics.show_text(text)?;
                graphics.end_text();
            }
            TextAlign::Center => {
                let text_width = text.len() as f64 * font_size * 0.5; // Approximate
                let centered_x = x + (width - text_width) / 2.0;
                graphics.begin_text();
                graphics.set_text_position(centered_x, text_y);
                graphics.show_text(text)?;
                graphics.end_text();
            }
            TextAlign::Right => {
                let text_width = text.len() as f64 * font_size * 0.5; // Approximate
                let right_x = x + width - text_width;
                graphics.begin_text();
                graphics.set_text_position(right_x, text_y);
                graphics.show_text(text)?;
                graphics.end_text();
            }
            TextAlign::Justified => {
                // For now, treat as left-aligned
                graphics.begin_text();
                graphics.set_text_position(x, text_y);
                graphics.show_text(text)?;
                graphics.end_text();
            }
        }

        graphics.restore_state();
        Ok(())
    }

    /// Render multiple paragraphs in a cell
    #[allow(clippy::too_many_arguments)]
    fn render_paragraphs_content(
        &self,
        graphics: &mut GraphicsContext,
        paragraphs: &[String],
        cell: &AdvancedTableCell,
        x: f64,
        y: f64,
        width: f64,
        height: f64,
    ) -> Result<(), PdfError> {
        let font_size = cell.font_size.unwrap_or(self.options.font_size);
        let line_height = font_size * 1.2;
        let mut current_y = y + height - font_size;

        for paragraph in paragraphs {
            if current_y < y {
                break; // No more space
            }

            self.render_text_content(
                graphics,
                paragraph,
                cell,
                x,
                current_y - (height - font_size),
                width,
                font_size,
            )?;

            current_y -= line_height;
        }

        Ok(())
    }

    /// Calculate total table height
    fn calculate_total_height(&self, column_widths: &[f64]) -> f64 {
        let mut height = 0.0;

        // Header rows
        for row in &self.header_rows {
            height += self.calculate_row_height(row, column_widths);
        }

        // Data rows
        for row in &self.rows {
            height += self.calculate_row_height(row, column_widths);
        }

        // Footer rows
        for row in &self.footer_rows {
            height += self.calculate_row_height(row, column_widths);
        }

        height
    }

    /// Get background color for a row considering alternating colors
    fn get_row_background_color(&self, row: &TableRow, row_index: usize) -> Option<Color> {
        if row.background_color.is_some() {
            return row.background_color;
        }

        if let Some(alt_colors) = &self.options.alternating_rows {
            let index = if alt_colors.include_header {
                row_index + self.header_rows.len()
            } else {
                row_index
            };

            if index % 2 == 0 {
                Some(alt_colors.even_color)
            } else {
                Some(alt_colors.odd_color)
            }
        } else {
            None
        }
    }
}

impl AdvancedTableCell {
    /// Create a simple text cell
    pub fn text(content: String) -> Self {
        Self {
            content: CellContent::Text(content),
            align: TextAlign::Left,
            vertical_align: VerticalAlign::Middle,
            colspan: 1,
            rowspan: 1,
            background_color: None,
            border_style: None,
            padding: None,
            font: None,
            font_size: None,
            text_color: None,
        }
    }

    /// Create a cell with paragraphs
    pub fn paragraphs(paragraphs: Vec<String>) -> Self {
        Self {
            content: CellContent::Paragraphs(paragraphs),
            align: TextAlign::Left,
            vertical_align: VerticalAlign::Top,
            colspan: 1,
            rowspan: 1,
            background_color: None,
            border_style: None,
            padding: None,
            font: None,
            font_size: None,
            text_color: None,
        }
    }

    /// Set cell alignment
    pub fn with_align(mut self, align: TextAlign) -> Self {
        self.align = align;
        self
    }

    /// Set vertical alignment
    pub fn with_vertical_align(mut self, align: VerticalAlign) -> Self {
        self.vertical_align = align;
        self
    }

    /// Set column span
    pub fn with_colspan(mut self, colspan: usize) -> Self {
        self.colspan = colspan;
        self
    }

    /// Set row span
    pub fn with_rowspan(mut self, rowspan: usize) -> Self {
        self.rowspan = rowspan;
        self
    }

    /// Set background color
    pub fn with_background(mut self, color: Color) -> Self {
        self.background_color = Some(color);
        self
    }

    /// Set custom padding
    pub fn with_padding(mut self, padding: CellPadding) -> Self {
        self.padding = Some(padding);
        self
    }

    /// Set custom font
    pub fn with_font(mut self, font: Font, size: f64) -> Self {
        self.font = Some(font);
        self.font_size = Some(size);
        self
    }

    /// Set text color
    pub fn with_text_color(mut self, color: Color) -> Self {
        self.text_color = Some(color);
        self
    }
}

impl TableRow {
    /// Create a new row with cells
    pub fn new(cells: Vec<AdvancedTableCell>) -> Self {
        Self {
            cells,
            height: None,
            background_color: None,
            can_split: true,
        }
    }

    /// Set row height
    pub fn with_height(mut self, height: f64) -> Self {
        self.height = Some(height);
        self
    }

    /// Set row background color
    pub fn with_background(mut self, color: Color) -> Self {
        self.background_color = Some(color);
        self
    }

    /// Set whether row can be split across pages
    pub fn with_can_split(mut self, can_split: bool) -> Self {
        self.can_split = can_split;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_advanced_table_creation() {
        let columns = vec![
            ColumnDefinition {
                width: ColumnWidth::Fixed(100.0),
                default_align: TextAlign::Left,
                min_width: None,
                max_width: None,
            },
            ColumnDefinition {
                width: ColumnWidth::Relative(0.5),
                default_align: TextAlign::Center,
                min_width: None,
                max_width: None,
            },
            ColumnDefinition {
                width: ColumnWidth::Auto,
                default_align: TextAlign::Right,
                min_width: Some(50.0),
                max_width: Some(200.0),
            },
        ];

        let table = AdvancedTable::new(columns);
        assert_eq!(table.columns.len(), 3);
        assert_eq!(table.rows.len(), 0);
    }

    #[test]
    fn test_column_width_calculation() {
        let columns = vec![
            ColumnDefinition {
                width: ColumnWidth::Fixed(100.0),
                default_align: TextAlign::Left,
                min_width: None,
                max_width: None,
            },
            ColumnDefinition {
                width: ColumnWidth::Relative(0.6),
                default_align: TextAlign::Center,
                min_width: None,
                max_width: None,
            },
            ColumnDefinition {
                width: ColumnWidth::Relative(0.4),
                default_align: TextAlign::Right,
                min_width: None,
                max_width: None,
            },
        ];

        let table = AdvancedTable::new(columns);
        let widths = table.calculate_column_widths(500.0);

        assert_eq!(widths[0], 100.0); // Fixed
        assert_eq!(widths[1], 240.0); // 60% of remaining 400
        assert_eq!(widths[2], 160.0); // 40% of remaining 400
    }

    #[test]
    fn test_cell_padding() {
        let uniform = CellPadding::uniform(10.0);
        assert_eq!(uniform.top, 10.0);
        assert_eq!(uniform.right, 10.0);
        assert_eq!(uniform.bottom, 10.0);
        assert_eq!(uniform.left, 10.0);

        let symmetric = CellPadding::symmetric(5.0, 15.0);
        assert_eq!(symmetric.top, 15.0);
        assert_eq!(symmetric.right, 5.0);
        assert_eq!(symmetric.bottom, 15.0);
        assert_eq!(symmetric.left, 5.0);
    }

    #[test]
    fn test_border_styles() {
        let none = BorderStyle::none();
        assert!(none.top.is_none());
        assert!(none.right.is_none());
        assert!(none.bottom.is_none());
        assert!(none.left.is_none());

        let horizontal = BorderStyle::horizontal_only(2.0, Color::red());
        assert!(horizontal.top.is_some());
        assert!(horizontal.right.is_none());
        assert!(horizontal.bottom.is_some());
        assert!(horizontal.left.is_none());

        let vertical = BorderStyle::vertical_only(1.5, Color::blue());
        assert!(vertical.top.is_none());
        assert!(vertical.right.is_some());
        assert!(vertical.bottom.is_none());
        assert!(vertical.left.is_some());
    }

    #[test]
    fn test_advanced_cell_creation() {
        let cell = AdvancedTableCell::text("Hello".to_string())
            .with_align(TextAlign::Center)
            .with_vertical_align(VerticalAlign::Top)
            .with_colspan(2)
            .with_background(Color::gray(0.9))
            .with_font(Font::HelveticaBold, 12.0)
            .with_text_color(Color::blue());

        match &cell.content {
            CellContent::Text(text) => assert_eq!(text, "Hello"),
            _ => panic!("Expected text content"),
        }
        assert_eq!(cell.align, TextAlign::Center);
        assert_eq!(cell.vertical_align, VerticalAlign::Top);
        assert_eq!(cell.colspan, 2);
        assert!(cell.background_color.is_some());
        assert!(cell.font.is_some());
        assert!(cell.font_size.is_some());
        assert!(cell.text_color.is_some());
    }

    #[test]
    fn test_table_row_creation() {
        let cells = vec![
            AdvancedTableCell::text("Cell 1".to_string()),
            AdvancedTableCell::text("Cell 2".to_string()),
        ];

        let row = TableRow::new(cells)
            .with_height(30.0)
            .with_background(Color::yellow())
            .with_can_split(false);

        assert_eq!(row.cells.len(), 2);
        assert_eq!(row.height, Some(30.0));
        assert!(row.background_color.is_some());
        assert!(!row.can_split);
    }

    #[test]
    fn test_add_text_row() {
        let mut table = AdvancedTable::with_equal_columns(3, 300.0);
        let result = table.add_text_row(vec![
            "Name".to_string(),
            "Age".to_string(),
            "City".to_string(),
        ]);

        assert!(result.is_ok());
        assert_eq!(table.rows.len(), 1);
    }

    #[test]
    fn test_add_text_row_mismatch() {
        let mut table = AdvancedTable::with_equal_columns(2, 200.0);
        let result = table.add_text_row(vec![
            "One".to_string(),
            "Two".to_string(),
            "Three".to_string(),
        ]);

        assert!(result.is_err());
    }

    #[test]
    fn test_validate_row_cells() {
        let table = AdvancedTable::with_equal_columns(3, 300.0);

        // Valid: 3 cells with colspan 1 each
        let cells1 = vec![
            AdvancedTableCell::text("A".to_string()),
            AdvancedTableCell::text("B".to_string()),
            AdvancedTableCell::text("C".to_string()),
        ];
        assert!(table.validate_row_cells(&cells1).is_ok());

        // Valid: 1 cell with colspan 2, 1 cell with colspan 1
        let cells2 = vec![
            AdvancedTableCell::text("AB".to_string()).with_colspan(2),
            AdvancedTableCell::text("C".to_string()),
        ];
        assert!(table.validate_row_cells(&cells2).is_ok());

        // Invalid: total colspan is 4
        let cells3 = vec![
            AdvancedTableCell::text("A".to_string()).with_colspan(2),
            AdvancedTableCell::text("B".to_string()).with_colspan(2),
        ];
        assert!(table.validate_row_cells(&cells3).is_err());
    }

    #[test]
    fn test_alternating_row_colors() {
        let alt_colors = AlternatingRowColors {
            even_color: Color::gray(0.95),
            odd_color: Color::white(),
            include_header: false,
        };

        let mut options = AdvancedTableOptions::default();
        options.alternating_rows = Some(alt_colors);

        let mut table = AdvancedTable::with_equal_columns(2, 200.0);
        table.set_options(options);

        // Add some rows
        table
            .add_text_row(vec!["Row 0".to_string(), "Data".to_string()])
            .unwrap();
        table
            .add_text_row(vec!["Row 1".to_string(), "Data".to_string()])
            .unwrap();

        // Test color calculation
        let row = &table.rows[0];
        let color = table.get_row_background_color(row, 0);
        assert!(color.is_some());
        assert_eq!(color.unwrap(), Color::gray(0.95)); // Even row

        let row = &table.rows[1];
        let color = table.get_row_background_color(row, 1);
        assert!(color.is_some());
        assert_eq!(color.unwrap(), Color::white()); // Odd row
    }

    #[test]
    fn test_cell_content_types() {
        // Text content
        let text_cell = AdvancedTableCell::text("Simple text".to_string());
        match text_cell.content {
            CellContent::Text(ref t) => assert_eq!(t, "Simple text"),
            _ => panic!("Expected text content"),
        }

        // Paragraphs content
        let para_cell =
            AdvancedTableCell::paragraphs(vec!["Line 1".to_string(), "Line 2".to_string()]);
        match para_cell.content {
            CellContent::Paragraphs(ref p) => assert_eq!(p.len(), 2),
            _ => panic!("Expected paragraphs content"),
        }
    }

    #[test]
    fn test_line_styles() {
        assert_eq!(LineStyle::Solid, LineStyle::Solid);
        assert_ne!(LineStyle::Solid, LineStyle::Dashed);
        assert_ne!(LineStyle::Dashed, LineStyle::Dotted);
    }

    #[test]
    fn test_vertical_alignment() {
        assert_eq!(VerticalAlign::Top, VerticalAlign::Top);
        assert_ne!(VerticalAlign::Top, VerticalAlign::Middle);
        assert_ne!(VerticalAlign::Middle, VerticalAlign::Bottom);
    }

    #[test]
    fn test_table_dimensions() {
        let mut table = AdvancedTable::with_equal_columns(3, 300.0);

        // Set fixed row height
        table.options.row_height = 25.0;

        // Add some rows
        table
            .add_text_row(vec!["A".to_string(), "B".to_string(), "C".to_string()])
            .unwrap();
        table
            .add_text_row(vec!["D".to_string(), "E".to_string(), "F".to_string()])
            .unwrap();

        let widths = table.calculate_column_widths(300.0);
        let total_width: f64 = widths.iter().sum();
        assert_eq!(total_width, 300.0);

        let total_height = table.calculate_total_height(&widths);
        assert_eq!(total_height, 50.0); // 2 rows * 25.0 height
    }

    #[test]
    fn test_auto_column_constraints() {
        let columns = vec![
            ColumnDefinition {
                width: ColumnWidth::Fixed(100.0),
                default_align: TextAlign::Left,
                min_width: None,
                max_width: None,
            },
            ColumnDefinition {
                width: ColumnWidth::Auto,
                default_align: TextAlign::Center,
                min_width: Some(80.0),
                max_width: Some(120.0),
            },
            ColumnDefinition {
                width: ColumnWidth::Auto,
                default_align: TextAlign::Right,
                min_width: Some(50.0),
                max_width: None,
            },
        ];

        let table = AdvancedTable::new(columns);
        let widths = table.calculate_column_widths(400.0);

        assert_eq!(widths[0], 100.0); // Fixed
        assert!(widths[1] >= 80.0 && widths[1] <= 120.0); // Auto with constraints
        assert!(widths[2] >= 50.0); // Auto with min constraint
    }

    #[test]
    fn test_header_footer_rows() {
        let mut table = AdvancedTable::with_equal_columns(2, 200.0);

        // Add header
        let header_cells = vec![
            AdvancedTableCell::text("Header 1".to_string()),
            AdvancedTableCell::text("Header 2".to_string()),
        ];
        assert!(table.add_header_row(header_cells).is_ok());
        assert_eq!(table.header_rows.len(), 1);

        // Add footer
        let footer_cells = vec![
            AdvancedTableCell::text("Footer 1".to_string()),
            AdvancedTableCell::text("Footer 2".to_string()),
        ];
        assert!(table.add_footer_row(footer_cells).is_ok());
        assert_eq!(table.footer_rows.len(), 1);

        // Add data row
        assert!(table
            .add_text_row(vec!["Data 1".to_string(), "Data 2".to_string()])
            .is_ok());
        assert_eq!(table.rows.len(), 1);
    }
}
