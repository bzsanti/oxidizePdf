use crate::error::Result;
use crate::page::Margins;
use crate::text::{measure_text, split_into_words, Font};
use std::fmt::Write;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TextAlign {
    Left,
    Right,
    Center,
    Justified,
}

pub struct TextFlowContext {
    operations: String,
    current_font: Font,
    font_size: f64,
    line_height: f64,
    cursor_x: f64,
    cursor_y: f64,
    alignment: TextAlign,
    page_width: f64,
    #[allow(dead_code)]
    page_height: f64,
    margins: Margins,
}

impl TextFlowContext {
    pub fn new(page_width: f64, page_height: f64, margins: Margins) -> Self {
        Self {
            operations: String::new(),
            current_font: Font::Helvetica,
            font_size: 12.0,
            line_height: 1.2,
            cursor_x: margins.left,
            cursor_y: page_height - margins.top,
            alignment: TextAlign::Left,
            page_width,
            page_height,
            margins,
        }
    }

    pub fn set_font(&mut self, font: Font, size: f64) -> &mut Self {
        self.current_font = font;
        self.font_size = size;
        self
    }

    pub fn set_line_height(&mut self, multiplier: f64) -> &mut Self {
        self.line_height = multiplier;
        self
    }

    pub fn set_alignment(&mut self, alignment: TextAlign) -> &mut Self {
        self.alignment = alignment;
        self
    }

    pub fn at(&mut self, x: f64, y: f64) -> &mut Self {
        self.cursor_x = x;
        self.cursor_y = y;
        self
    }

    pub fn content_width(&self) -> f64 {
        self.page_width - self.margins.left - self.margins.right
    }

    pub fn write_wrapped(&mut self, text: &str) -> Result<&mut Self> {
        let content_width = self.content_width();

        // Split text into words
        let words = split_into_words(text);
        let mut lines: Vec<Vec<&str>> = Vec::new();
        let mut current_line: Vec<&str> = Vec::new();
        let mut current_width = 0.0;

        // Build lines based on width constraints
        for word in words {
            let word_width = measure_text(word, self.current_font, self.font_size);

            // Check if we need to start a new line
            if !current_line.is_empty() && current_width + word_width > content_width {
                lines.push(current_line);
                current_line = vec![word];
                current_width = word_width;
            } else {
                current_line.push(word);
                current_width += word_width;
            }
        }

        if !current_line.is_empty() {
            lines.push(current_line);
        }

        // Render each line
        for (i, line) in lines.iter().enumerate() {
            let line_text = line.join("");
            let line_width = measure_text(&line_text, self.current_font, self.font_size);

            // Calculate x position based on alignment
            let x = match self.alignment {
                TextAlign::Left => self.margins.left,
                TextAlign::Right => self.page_width - self.margins.right - line_width,
                TextAlign::Center => self.margins.left + (content_width - line_width) / 2.0,
                TextAlign::Justified => {
                    if i < lines.len() - 1 && line.len() > 1 {
                        // We'll handle justification below
                        self.margins.left
                    } else {
                        self.margins.left
                    }
                }
            };

            // Begin text object
            self.operations.push_str("BT\n");

            // Set font
            writeln!(
                &mut self.operations,
                "/{} {} Tf",
                self.current_font.pdf_name(),
                self.font_size
            )
            .unwrap();

            // Set text position
            writeln!(&mut self.operations, "{:.2} {:.2} Td", x, self.cursor_y).unwrap();

            // Handle justification
            if self.alignment == TextAlign::Justified && i < lines.len() - 1 && line.len() > 1 {
                // Calculate extra space to distribute
                let spaces_count = line.iter().filter(|w| w.trim().is_empty()).count();
                if spaces_count > 0 {
                    let extra_space = content_width - line_width;
                    let space_adjustment = extra_space / spaces_count as f64;

                    // Set word spacing
                    writeln!(&mut self.operations, "{space_adjustment:.2} Tw").unwrap();
                }
            }

            // Show text
            self.operations.push('(');
            for ch in line_text.chars() {
                match ch {
                    '(' => self.operations.push_str("\\("),
                    ')' => self.operations.push_str("\\)"),
                    '\\' => self.operations.push_str("\\\\"),
                    '\n' => self.operations.push_str("\\n"),
                    '\r' => self.operations.push_str("\\r"),
                    '\t' => self.operations.push_str("\\t"),
                    _ => self.operations.push(ch),
                }
            }
            self.operations.push_str(") Tj\n");

            // Reset word spacing if it was set
            if self.alignment == TextAlign::Justified && i < lines.len() - 1 {
                self.operations.push_str("0 Tw\n");
            }

            // End text object
            self.operations.push_str("ET\n");

            // Move cursor down for next line
            self.cursor_y -= self.font_size * self.line_height;
        }

        Ok(self)
    }

    pub fn write_paragraph(&mut self, text: &str) -> Result<&mut Self> {
        self.write_wrapped(text)?;
        // Add extra space after paragraph
        self.cursor_y -= self.font_size * self.line_height * 0.5;
        Ok(self)
    }

    pub fn newline(&mut self) -> &mut Self {
        self.cursor_y -= self.font_size * self.line_height;
        self.cursor_x = self.margins.left;
        self
    }

    pub fn cursor_position(&self) -> (f64, f64) {
        (self.cursor_x, self.cursor_y)
    }

    pub fn generate_operations(&self) -> Vec<u8> {
        self.operations.as_bytes().to_vec()
    }

    /// Get the current alignment
    pub fn alignment(&self) -> TextAlign {
        self.alignment
    }

    /// Get the page dimensions
    pub fn page_dimensions(&self) -> (f64, f64) {
        (self.page_width, self.page_height)
    }

    /// Get the margins
    pub fn margins(&self) -> &Margins {
        &self.margins
    }

    /// Get current line height multiplier
    pub fn line_height(&self) -> f64 {
        self.line_height
    }

    /// Get the operations string
    pub fn operations(&self) -> &str {
        &self.operations
    }

    /// Clear all operations
    pub fn clear(&mut self) {
        self.operations.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::page::Margins;

    fn create_test_margins() -> Margins {
        Margins {
            left: 50.0,
            right: 50.0,
            top: 50.0,
            bottom: 50.0,
        }
    }

    #[test]
    fn test_text_flow_context_new() {
        let margins = create_test_margins();
        let context = TextFlowContext::new(400.0, 600.0, margins.clone());

        assert_eq!(context.current_font, Font::Helvetica);
        assert_eq!(context.font_size, 12.0);
        assert_eq!(context.line_height, 1.2);
        assert_eq!(context.alignment, TextAlign::Left);
        assert_eq!(context.page_width, 400.0);
        assert_eq!(context.page_height, 600.0);
        assert_eq!(context.cursor_x, 50.0); // margins.left
        assert_eq!(context.cursor_y, 550.0); // page_height - margins.top
    }

    #[test]
    fn test_set_font() {
        let margins = create_test_margins();
        let mut context = TextFlowContext::new(400.0, 600.0, margins.clone());

        context.set_font(Font::TimesBold, 16.0);
        assert_eq!(context.current_font, Font::TimesBold);
        assert_eq!(context.font_size, 16.0);
    }

    #[test]
    fn test_set_line_height() {
        let margins = create_test_margins();
        let mut context = TextFlowContext::new(400.0, 600.0, margins.clone());

        context.set_line_height(1.5);
        assert_eq!(context.line_height(), 1.5);
    }

    #[test]
    fn test_set_alignment() {
        let margins = create_test_margins();
        let mut context = TextFlowContext::new(400.0, 600.0, margins.clone());

        context.set_alignment(TextAlign::Center);
        assert_eq!(context.alignment(), TextAlign::Center);
    }

    #[test]
    fn test_at_position() {
        let margins = create_test_margins();
        let mut context = TextFlowContext::new(400.0, 600.0, margins.clone());

        context.at(100.0, 200.0);
        let (x, y) = context.cursor_position();
        assert_eq!(x, 100.0);
        assert_eq!(y, 200.0);
    }

    #[test]
    fn test_content_width() {
        let margins = create_test_margins();
        let context = TextFlowContext::new(400.0, 600.0, margins.clone());

        let content_width = context.content_width();
        assert_eq!(content_width, 300.0); // 400 - 50 - 50
    }

    #[test]
    fn test_text_align_variants() {
        assert_eq!(TextAlign::Left, TextAlign::Left);
        assert_eq!(TextAlign::Right, TextAlign::Right);
        assert_eq!(TextAlign::Center, TextAlign::Center);
        assert_eq!(TextAlign::Justified, TextAlign::Justified);

        assert_ne!(TextAlign::Left, TextAlign::Right);
    }

    #[test]
    fn test_write_wrapped_simple() {
        let margins = create_test_margins();
        let mut context = TextFlowContext::new(400.0, 600.0, margins.clone());

        context.write_wrapped("Hello World").unwrap();

        let ops = context.operations();
        assert!(ops.contains("BT\n"));
        assert!(ops.contains("ET\n"));
        assert!(ops.contains("/Helvetica 12 Tf"));
        assert!(ops.contains("(Hello World) Tj"));
    }

    #[test]
    fn test_write_paragraph() {
        let margins = create_test_margins();
        let mut context = TextFlowContext::new(400.0, 600.0, margins.clone());

        let initial_y = context.cursor_y;
        context.write_paragraph("Test paragraph").unwrap();

        // Y position should have moved down more than just line height
        assert!(context.cursor_y < initial_y);
    }

    #[test]
    fn test_newline() {
        let margins = create_test_margins();
        let mut context = TextFlowContext::new(400.0, 600.0, margins.clone());

        let initial_y = context.cursor_y;
        context.newline();

        assert_eq!(context.cursor_x, margins.left);
        assert!(context.cursor_y < initial_y);
        assert_eq!(
            context.cursor_y,
            initial_y - context.font_size * context.line_height
        );
    }

    #[test]
    fn test_cursor_position() {
        let margins = create_test_margins();
        let mut context = TextFlowContext::new(400.0, 600.0, margins.clone());

        context.at(75.0, 125.0);
        let (x, y) = context.cursor_position();
        assert_eq!(x, 75.0);
        assert_eq!(y, 125.0);
    }

    #[test]
    fn test_generate_operations() {
        let margins = create_test_margins();
        let mut context = TextFlowContext::new(400.0, 600.0, margins.clone());

        context.write_wrapped("Test").unwrap();
        let ops_bytes = context.generate_operations();
        let ops_string = String::from_utf8(ops_bytes).unwrap();

        assert_eq!(ops_string, context.operations());
    }

    #[test]
    fn test_clear_operations() {
        let margins = create_test_margins();
        let mut context = TextFlowContext::new(400.0, 600.0, margins.clone());

        context.write_wrapped("Test").unwrap();
        assert!(!context.operations().is_empty());

        context.clear();
        assert!(context.operations().is_empty());
    }

    #[test]
    fn test_page_dimensions() {
        let margins = create_test_margins();
        let context = TextFlowContext::new(400.0, 600.0, margins.clone());

        let (width, height) = context.page_dimensions();
        assert_eq!(width, 400.0);
        assert_eq!(height, 600.0);
    }

    #[test]
    fn test_margins_access() {
        let margins = create_test_margins();
        let context = TextFlowContext::new(400.0, 600.0, margins.clone());

        let ctx_margins = context.margins();
        assert_eq!(ctx_margins.left, 50.0);
        assert_eq!(ctx_margins.right, 50.0);
        assert_eq!(ctx_margins.top, 50.0);
        assert_eq!(ctx_margins.bottom, 50.0);
    }

    #[test]
    fn test_method_chaining() {
        let margins = create_test_margins();
        let mut context = TextFlowContext::new(400.0, 600.0, margins.clone());

        context
            .set_font(Font::Courier, 10.0)
            .set_line_height(1.5)
            .set_alignment(TextAlign::Center)
            .at(100.0, 200.0);

        assert_eq!(context.current_font, Font::Courier);
        assert_eq!(context.font_size, 10.0);
        assert_eq!(context.line_height(), 1.5);
        assert_eq!(context.alignment(), TextAlign::Center);
        let (x, y) = context.cursor_position();
        assert_eq!(x, 100.0);
        assert_eq!(y, 200.0);
    }

    #[test]
    fn test_text_align_debug() {
        let align = TextAlign::Center;
        let debug_str = format!("{:?}", align);
        assert_eq!(debug_str, "Center");
    }

    #[test]
    fn test_text_align_clone() {
        let align1 = TextAlign::Justified;
        let align2 = align1;
        assert_eq!(align1, align2);
    }

    #[test]
    fn test_text_align_copy() {
        let align1 = TextAlign::Right;
        let align2 = align1; // Copy semantics
        assert_eq!(align1, align2);

        // Both variables should still be usable
        assert_eq!(align1, TextAlign::Right);
        assert_eq!(align2, TextAlign::Right);
    }
}
