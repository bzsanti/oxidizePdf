//! Multi-column layout support for PDF documents
//!
//! This module provides basic column support for newsletter-style documents
//! with automatic text flow between columns.

use crate::error::PdfError;
use crate::graphics::{Color, GraphicsContext};
use crate::text::{Font, TextAlign};

/// Column layout configuration
#[derive(Debug, Clone)]
pub struct ColumnLayout {
    /// Number of columns
    column_count: usize,
    /// Width of each column (in points)
    column_widths: Vec<f64>,
    /// Gap between columns (in points)
    column_gap: f64,
    /// Total layout width (in points)
    total_width: f64,
    /// Layout options
    options: ColumnOptions,
}

/// Options for column layout
#[derive(Debug, Clone)]
pub struct ColumnOptions {
    /// Font for column text
    pub font: Font,
    /// Font size in points
    pub font_size: f64,
    /// Line height multiplier
    pub line_height: f64,
    /// Text color
    pub text_color: Color,
    /// Text alignment within columns
    pub text_align: TextAlign,
    /// Whether to balance columns (distribute content evenly)
    pub balance_columns: bool,
    /// Whether to draw column separators
    pub show_separators: bool,
    /// Separator color
    pub separator_color: Color,
    /// Separator width
    pub separator_width: f64,
}

impl Default for ColumnOptions {
    fn default() -> Self {
        Self {
            font: Font::Helvetica,
            font_size: 10.0,
            line_height: 1.2,
            text_color: Color::black(),
            text_align: TextAlign::Left,
            balance_columns: true,
            show_separators: false,
            separator_color: Color::gray(0.7),
            separator_width: 0.5,
        }
    }
}

/// Text content for column layout
#[derive(Debug, Clone)]
pub struct ColumnContent {
    /// Text content to flow across columns
    text: String,
    /// Formatting options
    formatting: Vec<TextFormat>,
}

/// Text formatting information
#[derive(Debug, Clone)]
pub struct TextFormat {
    /// Start position in text
    #[allow(dead_code)]
    start: usize,
    /// End position in text
    #[allow(dead_code)]
    end: usize,
    /// Font override
    font: Option<Font>,
    /// Font size override
    font_size: Option<f64>,
    /// Color override
    color: Option<Color>,
    /// Bold flag
    bold: bool,
    /// Italic flag
    italic: bool,
}

/// Column flow context for managing text across columns
#[derive(Debug)]
pub struct ColumnFlowContext {
    /// Current column being filled
    current_column: usize,
    /// Current Y position in each column
    column_positions: Vec<f64>,
    /// Height of each column
    column_heights: Vec<f64>,
    /// Content for each column
    column_contents: Vec<Vec<String>>,
}

impl ColumnLayout {
    /// Create a new column layout with equal column widths
    pub fn new(column_count: usize, total_width: f64, column_gap: f64) -> Self {
        if column_count == 0 {
            panic!("Column count must be greater than 0");
        }

        let available_width = total_width - (column_gap * (column_count - 1) as f64);
        let column_width = available_width / column_count as f64;
        let column_widths = vec![column_width; column_count];

        Self {
            column_count,
            column_widths,
            column_gap,
            total_width,
            options: ColumnOptions::default(),
        }
    }

    /// Create a new column layout with custom column widths
    pub fn with_custom_widths(column_widths: Vec<f64>, column_gap: f64) -> Self {
        let column_count = column_widths.len();
        if column_count == 0 {
            panic!("Must have at least one column");
        }

        let content_width: f64 = column_widths.iter().sum();
        let total_width = content_width + (column_gap * (column_count - 1) as f64);

        Self {
            column_count,
            column_widths,
            column_gap,
            total_width,
            options: ColumnOptions::default(),
        }
    }

    /// Set column options
    pub fn set_options(&mut self, options: ColumnOptions) -> &mut Self {
        self.options = options;
        self
    }

    /// Get the number of columns
    pub fn column_count(&self) -> usize {
        self.column_count
    }

    /// Get the total width
    pub fn total_width(&self) -> f64 {
        self.total_width
    }

    /// Get column width by index
    pub fn column_width(&self, index: usize) -> Option<f64> {
        self.column_widths.get(index).copied()
    }

    /// Get the X position of a column
    pub fn column_x_position(&self, index: usize) -> f64 {
        let mut x = 0.0;
        for i in 0..index.min(self.column_count) {
            x += self.column_widths[i] + self.column_gap;
        }
        x
    }

    /// Create a flow context for managing text across columns
    pub fn create_flow_context(&self, start_y: f64, column_height: f64) -> ColumnFlowContext {
        ColumnFlowContext {
            current_column: 0,
            column_positions: vec![start_y; self.column_count],
            column_heights: vec![column_height; self.column_count],
            column_contents: vec![Vec::new(); self.column_count],
        }
    }

    /// Render column layout with content
    pub fn render(
        &self,
        graphics: &mut GraphicsContext,
        content: &ColumnContent,
        start_x: f64,
        start_y: f64,
        column_height: f64,
    ) -> Result<(), PdfError> {
        // Create flow context
        let mut flow_context = self.create_flow_context(start_y, column_height);

        // Split text into words for flowing
        let words = self.split_text_into_words(&content.text);

        // Flow text across columns
        self.flow_text_across_columns(&words, &mut flow_context)?;

        // Render each column
        for (col_index, column_content) in flow_context.column_contents.iter().enumerate() {
            let column_x = start_x + self.column_x_position(col_index);
            self.render_column(graphics, column_content, column_x, start_y)?;
        }

        // Draw column separators if enabled
        if self.options.show_separators {
            self.draw_separators(graphics, start_x, start_y, column_height)?;
        }

        Ok(())
    }

    /// Split text into words for flowing
    fn split_text_into_words(&self, text: &str) -> Vec<String> {
        text.split_whitespace()
            .map(|word| word.to_string())
            .collect()
    }

    /// Flow text across columns
    fn flow_text_across_columns(
        &self,
        words: &[String],
        flow_context: &mut ColumnFlowContext,
    ) -> Result<(), PdfError> {
        let mut current_line = String::new();
        let line_height = self.options.font_size * self.options.line_height;

        for word in words {
            // Check if adding this word would exceed column width
            let test_line = if current_line.is_empty() {
                word.clone()
            } else {
                format!("{current_line} {word}")
            };

            let line_width = self.estimate_text_width(&test_line);
            let column_width = self.column_widths[flow_context.current_column];

            if line_width <= column_width || current_line.is_empty() {
                // Word fits in current line
                current_line = test_line;
            } else {
                // Word doesn't fit, start new line
                if !current_line.is_empty() {
                    // Add current line to column
                    flow_context.column_contents[flow_context.current_column]
                        .push(current_line.clone());
                    flow_context.column_positions[flow_context.current_column] -= line_height;

                    // Check if we need to move to next column
                    if flow_context.column_positions[flow_context.current_column]
                        < flow_context.column_heights[flow_context.current_column] - line_height
                    {
                        // Move to next column if available
                        if flow_context.current_column + 1 < self.column_count {
                            flow_context.current_column += 1;
                        }
                    }
                }
                current_line = word.clone();
            }
        }

        // Add final line if not empty
        if !current_line.is_empty() {
            flow_context.column_contents[flow_context.current_column].push(current_line);
        }

        // Balance columns if enabled
        if self.options.balance_columns {
            self.balance_column_content(flow_context)?;
        }

        Ok(())
    }

    /// Estimate text width (simple approximation)
    fn estimate_text_width(&self, text: &str) -> f64 {
        // Simple approximation: character count * font size * 0.6
        text.len() as f64 * self.options.font_size * 0.6
    }

    /// Balance content across columns
    fn balance_column_content(&self, flow_context: &mut ColumnFlowContext) -> Result<(), PdfError> {
        // Collect all lines from all columns
        let mut all_lines = Vec::new();
        for column in &flow_context.column_contents {
            all_lines.extend(column.iter().cloned());
        }

        // Clear existing column contents
        for column in &mut flow_context.column_contents {
            column.clear();
        }

        // Redistribute lines evenly across columns
        let lines_per_column = all_lines.len().div_ceil(self.column_count);

        for (line_index, line) in all_lines.into_iter().enumerate() {
            let column_index = (line_index / lines_per_column).min(self.column_count - 1);
            flow_context.column_contents[column_index].push(line);
        }

        Ok(())
    }

    /// Render a single column
    fn render_column(
        &self,
        graphics: &mut GraphicsContext,
        lines: &[String],
        column_x: f64,
        start_y: f64,
    ) -> Result<(), PdfError> {
        let line_height = self.options.font_size * self.options.line_height;
        let mut current_y = start_y;

        graphics.save_state();
        graphics.set_font(self.options.font.clone(), self.options.font_size);
        graphics.set_fill_color(self.options.text_color);

        for line in lines {
            graphics.begin_text();

            match self.options.text_align {
                TextAlign::Left => {
                    graphics.set_text_position(column_x, current_y);
                    graphics.show_text(line)?;
                }
                TextAlign::Center => {
                    let line_width = self.estimate_text_width(line);
                    let column_width = self.column_widths[0]; // Simplified for now
                    let text_x = column_x + (column_width - line_width) / 2.0;
                    graphics.set_text_position(text_x, current_y);
                    graphics.show_text(line)?;
                }
                TextAlign::Right => {
                    let line_width = self.estimate_text_width(line);
                    let column_width = self.column_widths[0]; // Simplified for now
                    let text_x = column_x + column_width - line_width;
                    graphics.set_text_position(text_x, current_y);
                    graphics.show_text(line)?;
                }
                TextAlign::Justified => {
                    let column_width = self.column_widths[0]; // Simplified for now
                    graphics.set_text_position(column_x, current_y);
                    graphics.show_justified_text(line, column_width)?;
                }
            };
            graphics.end_text();

            current_y -= line_height;
        }

        graphics.restore_state();
        Ok(())
    }

    /// Draw column separators
    fn draw_separators(
        &self,
        graphics: &mut GraphicsContext,
        start_x: f64,
        start_y: f64,
        column_height: f64,
    ) -> Result<(), PdfError> {
        if self.column_count <= 1 {
            return Ok(());
        }

        graphics.save_state();
        graphics.set_stroke_color(self.options.separator_color);
        graphics.set_line_width(self.options.separator_width);

        for i in 0..self.column_count - 1 {
            let separator_x = start_x
                + self.column_x_position(i)
                + self.column_widths[i]
                + (self.column_gap / 2.0);

            graphics.move_to(separator_x, start_y);
            graphics.line_to(separator_x, start_y - column_height);
            graphics.stroke();
        }

        graphics.restore_state();
        Ok(())
    }
}

impl ColumnContent {
    /// Create new column content
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            formatting: Vec::new(),
        }
    }

    /// Add text formatting
    pub fn add_format(&mut self, format: TextFormat) -> &mut Self {
        self.formatting.push(format);
        self
    }

    /// Get the text content
    pub fn text(&self) -> &str {
        &self.text
    }
}

impl TextFormat {
    /// Create a new text format
    pub fn new(start: usize, end: usize) -> Self {
        Self {
            start,
            end,
            font: None,
            font_size: None,
            color: None,
            bold: false,
            italic: false,
        }
    }

    /// Set font override
    pub fn with_font(mut self, font: Font) -> Self {
        self.font = Some(font);
        self
    }

    /// Set font size override
    pub fn with_font_size(mut self, size: f64) -> Self {
        self.font_size = Some(size);
        self
    }

    /// Set color override
    pub fn with_color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }

    /// Set bold
    pub fn bold(mut self) -> Self {
        self.bold = true;
        self
    }

    /// Set italic
    pub fn italic(mut self) -> Self {
        self.italic = true;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_column_layout_creation() {
        let layout = ColumnLayout::new(3, 600.0, 20.0);
        assert_eq!(layout.column_count(), 3);
        assert_eq!(layout.total_width(), 600.0);

        // Each column should be (600 - 2*20) / 3 = 186.67 points wide
        assert!((layout.column_width(0).unwrap() - 186.67).abs() < 0.01);
    }

    #[test]
    fn test_custom_column_widths() {
        let layout = ColumnLayout::with_custom_widths(vec![200.0, 150.0, 250.0], 15.0);
        assert_eq!(layout.column_count(), 3);
        assert_eq!(layout.total_width(), 630.0); // 200 + 150 + 250 + 2*15
        assert_eq!(layout.column_width(0), Some(200.0));
        assert_eq!(layout.column_width(1), Some(150.0));
        assert_eq!(layout.column_width(2), Some(250.0));
    }

    #[test]
    fn test_column_x_positions() {
        let layout = ColumnLayout::with_custom_widths(vec![100.0, 200.0, 150.0], 20.0);
        assert_eq!(layout.column_x_position(0), 0.0);
        assert_eq!(layout.column_x_position(1), 120.0); // 100 + 20
        assert_eq!(layout.column_x_position(2), 340.0); // 100 + 20 + 200 + 20
    }

    #[test]
    fn test_column_options_default() {
        let options = ColumnOptions::default();
        assert_eq!(options.font, Font::Helvetica);
        assert_eq!(options.font_size, 10.0);
        assert_eq!(options.line_height, 1.2);
        assert!(options.balance_columns);
        assert!(!options.show_separators);
    }

    #[test]
    fn test_column_content() {
        let mut content = ColumnContent::new("Hello world");
        assert_eq!(content.text(), "Hello world");

        content.add_format(TextFormat::new(0, 5).bold());
        assert_eq!(content.formatting.len(), 1);
        assert!(content.formatting[0].bold);
    }

    #[test]
    fn test_text_format() {
        let format = TextFormat::new(0, 10)
            .with_font(Font::HelveticaBold)
            .with_font_size(14.0)
            .with_color(Color::red())
            .bold()
            .italic();

        assert_eq!(format.start, 0);
        assert_eq!(format.end, 10);
        assert_eq!(format.font, Some(Font::HelveticaBold));
        assert_eq!(format.font_size, Some(14.0));
        assert_eq!(format.color, Some(Color::red()));
        assert!(format.bold);
        assert!(format.italic);
    }

    #[test]
    fn test_flow_context_creation() {
        let layout = ColumnLayout::new(2, 400.0, 20.0);
        let context = layout.create_flow_context(100.0, 500.0);

        assert_eq!(context.current_column, 0);
        assert_eq!(context.column_positions.len(), 2);
        assert_eq!(context.column_heights.len(), 2);
        assert_eq!(context.column_contents.len(), 2);
        assert_eq!(context.column_positions[0], 100.0);
        assert_eq!(context.column_heights[0], 500.0);
    }

    #[test]
    fn test_text_width_estimation() {
        let layout = ColumnLayout::new(1, 100.0, 0.0);
        let width = layout.estimate_text_width("Hello");
        assert_eq!(width, 5.0 * 10.0 * 0.6); // 5 chars * 10pt font * 0.6 factor
    }

    #[test]
    fn test_split_text_into_words() {
        let layout = ColumnLayout::new(1, 100.0, 0.0);
        let words = layout.split_text_into_words("Hello world, this is a test");
        assert_eq!(words, vec!["Hello", "world,", "this", "is", "a", "test"]);
    }

    #[test]
    fn test_column_layout_with_options() {
        let mut layout = ColumnLayout::new(2, 400.0, 20.0);
        let options = ColumnOptions {
            font: Font::TimesBold,
            font_size: 12.0,
            show_separators: true,
            ..Default::default()
        };

        layout.set_options(options);
        assert_eq!(layout.options.font, Font::TimesBold);
        assert_eq!(layout.options.font_size, 12.0);
        assert!(layout.options.show_separators);
    }

    #[test]
    #[should_panic(expected = "Column count must be greater than 0")]
    fn test_zero_columns_panic() {
        ColumnLayout::new(0, 100.0, 10.0);
    }

    #[test]
    #[should_panic(expected = "Must have at least one column")]
    fn test_empty_custom_widths_panic() {
        ColumnLayout::with_custom_widths(vec![], 10.0);
    }

    #[test]
    fn test_column_width_out_of_bounds() {
        let layout = ColumnLayout::new(2, 400.0, 20.0);
        assert_eq!(layout.column_width(5), None);
    }
}
