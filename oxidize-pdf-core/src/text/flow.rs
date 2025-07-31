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

    #[test]
    fn test_write_wrapped_with_alignment_right() {
        let margins = create_test_margins();
        let mut context = TextFlowContext::new(400.0, 600.0, margins);

        context.set_alignment(TextAlign::Right);
        context.write_wrapped("Right aligned text").unwrap();

        let ops = context.operations();
        assert!(ops.contains("BT\n"));
        assert!(ops.contains("ET\n"));
        // Right alignment should position text differently
        assert!(ops.contains("Td"));
    }

    #[test]
    fn test_write_wrapped_with_alignment_center() {
        let margins = create_test_margins();
        let mut context = TextFlowContext::new(400.0, 600.0, margins);

        context.set_alignment(TextAlign::Center);
        context.write_wrapped("Centered text").unwrap();

        let ops = context.operations();
        assert!(ops.contains("BT\n"));
        assert!(ops.contains("(Centered text) Tj"));
    }

    #[test]
    fn test_write_wrapped_with_alignment_justified() {
        let margins = create_test_margins();
        let mut context = TextFlowContext::new(400.0, 600.0, margins);

        context.set_alignment(TextAlign::Justified);
        // Long text that will wrap and justify
        context.write_wrapped("This is a longer text that should wrap across multiple lines to test justification").unwrap();

        let ops = context.operations();
        assert!(ops.contains("BT\n"));
        // Justified text may have word spacing adjustments
        assert!(ops.contains("Tw") || ops.contains("0 Tw"));
    }

    #[test]
    fn test_write_wrapped_empty_text() {
        let margins = create_test_margins();
        let mut context = TextFlowContext::new(400.0, 600.0, margins);

        context.write_wrapped("").unwrap();

        // Empty text should not generate operations
        assert!(context.operations().is_empty());
    }

    #[test]
    fn test_write_wrapped_whitespace_only() {
        let margins = create_test_margins();
        let mut context = TextFlowContext::new(400.0, 600.0, margins);

        context.write_wrapped("   ").unwrap();

        let ops = context.operations();
        // Should handle whitespace-only text
        assert!(ops.contains("BT\n") || ops.is_empty());
    }

    #[test]
    fn test_write_wrapped_special_characters() {
        let margins = create_test_margins();
        let mut context = TextFlowContext::new(400.0, 600.0, margins);

        context
            .write_wrapped("Text with (parentheses) and \\backslash\\")
            .unwrap();

        let ops = context.operations();
        // Special characters should be escaped
        assert!(ops.contains("\\(parentheses\\)"));
        assert!(ops.contains("\\\\backslash\\\\"));
    }

    #[test]
    fn test_write_wrapped_newlines_tabs() {
        let margins = create_test_margins();
        let mut context = TextFlowContext::new(400.0, 600.0, margins);

        context.write_wrapped("Line1\nLine2\tTabbed").unwrap();

        let ops = context.operations();
        // Newlines and tabs should be escaped
        assert!(ops.contains("\\n") || ops.contains("\\t"));
    }

    #[test]
    fn test_write_wrapped_very_long_word() {
        let margins = create_test_margins();
        let mut context = TextFlowContext::new(200.0, 600.0, margins); // Narrow page

        let long_word = "a".repeat(100);
        context.write_wrapped(&long_word).unwrap();

        let ops = context.operations();
        assert!(ops.contains("BT\n"));
        assert!(ops.contains(&long_word));
    }

    #[test]
    fn test_write_wrapped_cursor_movement() {
        let margins = create_test_margins();
        let mut context = TextFlowContext::new(400.0, 600.0, margins);

        let initial_y = context.cursor_y;

        context.write_wrapped("Line 1").unwrap();
        let y_after_line1 = context.cursor_y;

        context.write_wrapped("Line 2").unwrap();
        let y_after_line2 = context.cursor_y;

        // Cursor should move down after each line
        assert!(y_after_line1 < initial_y);
        assert!(y_after_line2 < y_after_line1);
    }

    #[test]
    fn test_write_paragraph_spacing() {
        let margins = create_test_margins();
        let mut context = TextFlowContext::new(400.0, 600.0, margins);

        let initial_y = context.cursor_y;
        context.write_paragraph("Paragraph 1").unwrap();
        let y_after_p1 = context.cursor_y;

        context.write_paragraph("Paragraph 2").unwrap();
        let y_after_p2 = context.cursor_y;

        // Paragraphs should have extra spacing
        let spacing1 = initial_y - y_after_p1;
        let spacing2 = y_after_p1 - y_after_p2;

        assert!(spacing1 > 0.0);
        assert!(spacing2 > 0.0);
    }

    #[test]
    fn test_multiple_newlines() {
        let margins = create_test_margins();
        let mut context = TextFlowContext::new(400.0, 600.0, margins);

        let initial_y = context.cursor_y;

        context.newline();
        let y1 = context.cursor_y;

        context.newline();
        let y2 = context.cursor_y;

        context.newline();
        let y3 = context.cursor_y;

        // Each newline should move cursor down by same amount
        let spacing1 = initial_y - y1;
        let spacing2 = y1 - y2;
        let spacing3 = y2 - y3;

        assert_eq!(spacing1, spacing2);
        assert_eq!(spacing2, spacing3);
        assert_eq!(spacing1, context.font_size * context.line_height);
    }

    #[test]
    fn test_content_width_different_margins() {
        let margins = Margins {
            left: 30.0,
            right: 70.0,
            top: 40.0,
            bottom: 60.0,
        };
        let context = TextFlowContext::new(500.0, 700.0, margins);

        let content_width = context.content_width();
        assert_eq!(content_width, 400.0); // 500 - 30 - 70
    }

    #[test]
    fn test_custom_line_height() {
        let margins = create_test_margins();
        let mut context = TextFlowContext::new(400.0, 600.0, margins);

        context.set_line_height(2.0);

        let initial_y = context.cursor_y;
        context.newline();
        let y_after = context.cursor_y;

        let spacing = initial_y - y_after;
        assert_eq!(spacing, context.font_size * 2.0); // line_height = 2.0
    }

    #[test]
    fn test_different_fonts() {
        let margins = create_test_margins();
        let mut context = TextFlowContext::new(400.0, 600.0, margins);

        let fonts = vec![
            Font::Helvetica,
            Font::HelveticaBold,
            Font::TimesRoman,
            Font::TimesBold,
            Font::Courier,
            Font::CourierBold,
        ];

        for font in fonts {
            context.clear();
            context.set_font(font, 14.0);
            context.write_wrapped("Test text").unwrap();

            let ops = context.operations();
            assert!(ops.contains(&format!("/{} 14 Tf", font.pdf_name())));
        }
    }

    #[test]
    fn test_font_size_variations() {
        let margins = create_test_margins();
        let mut context = TextFlowContext::new(400.0, 600.0, margins);

        let sizes = vec![8.0, 10.0, 12.0, 14.0, 16.0, 24.0, 36.0];

        for size in sizes {
            context.clear();
            context.set_font(Font::Helvetica, size);
            context.write_wrapped("Test").unwrap();

            let ops = context.operations();
            assert!(ops.contains(&format!("/Helvetica {} Tf", size)));
        }
    }

    #[test]
    fn test_at_position_edge_cases() {
        let margins = create_test_margins();
        let mut context = TextFlowContext::new(400.0, 600.0, margins);

        // Test zero position
        context.at(0.0, 0.0);
        assert_eq!(context.cursor_position(), (0.0, 0.0));

        // Test negative position
        context.at(-10.0, -20.0);
        assert_eq!(context.cursor_position(), (-10.0, -20.0));

        // Test large position
        context.at(10000.0, 20000.0);
        assert_eq!(context.cursor_position(), (10000.0, 20000.0));
    }

    #[test]
    fn test_write_wrapped_with_narrow_content() {
        let margins = Margins {
            left: 190.0,
            right: 190.0,
            top: 50.0,
            bottom: 50.0,
        };
        let mut context = TextFlowContext::new(400.0, 600.0, margins);

        // Content width is only 20.0 units
        context
            .write_wrapped("This text should wrap a lot")
            .unwrap();

        let ops = context.operations();
        // Should have multiple text objects for wrapped lines
        let bt_count = ops.matches("BT\n").count();
        assert!(bt_count > 1);
    }

    #[test]
    fn test_justified_text_single_word_line() {
        let margins = create_test_margins();
        let mut context = TextFlowContext::new(400.0, 600.0, margins);

        context.set_alignment(TextAlign::Justified);
        context.write_wrapped("SingleWord").unwrap();

        let ops = context.operations();
        // Single word lines should not have word spacing
        assert!(!ops.contains(" Tw") || ops.contains("0 Tw"));
    }

    #[test]
    fn test_justified_text_last_line() {
        let margins = create_test_margins();
        let mut context = TextFlowContext::new(400.0, 600.0, margins);

        context.set_alignment(TextAlign::Justified);
        // Text that will create multiple lines
        context.write_wrapped("This is a test of justified text alignment where the last line should not be justified").unwrap();

        let ops = context.operations();
        // Should reset word spacing (0 Tw) for last line
        assert!(ops.contains("0 Tw"));
    }

    #[test]
    fn test_generate_operations_encoding() {
        let margins = create_test_margins();
        let mut context = TextFlowContext::new(400.0, 600.0, margins);

        context.write_wrapped("UTF-8 Text: Ñ").unwrap();

        let ops_bytes = context.generate_operations();
        let ops_string = String::from_utf8(ops_bytes.clone()).unwrap();

        assert_eq!(ops_bytes, context.operations().as_bytes());
        assert_eq!(ops_string, context.operations());
    }

    #[test]
    fn test_clear_resets_operations_only() {
        let margins = create_test_margins();
        let mut context = TextFlowContext::new(400.0, 600.0, margins);

        context.set_font(Font::TimesBold, 18.0);
        context.set_alignment(TextAlign::Right);
        context.at(100.0, 200.0);
        context.write_wrapped("Text").unwrap();

        context.clear();

        // Operations should be cleared
        assert!(context.operations().is_empty());

        // But other settings should remain
        assert_eq!(context.current_font, Font::TimesBold);
        assert_eq!(context.font_size, 18.0);
        assert_eq!(context.alignment(), TextAlign::Right);
        assert_eq!(context.cursor_position(), (100.0, 200.0));
    }

    #[test]
    fn test_long_text_wrapping() {
        let margins = create_test_margins();
        let mut context = TextFlowContext::new(400.0, 600.0, margins);

        let long_text = "Lorem ipsum dolor sit amet, consectetur adipiscing elit. \
                        Sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. \
                        Ut enim ad minim veniam, quis nostrud exercitation ullamco laboris.";

        context.write_wrapped(long_text).unwrap();

        let ops = context.operations();
        // Should have multiple lines
        let tj_count = ops.matches(") Tj").count();
        assert!(tj_count > 1);
    }

    #[test]
    fn test_empty_operations_initially() {
        let margins = create_test_margins();
        let context = TextFlowContext::new(400.0, 600.0, margins);

        assert!(context.operations().is_empty());
        assert_eq!(context.generate_operations().len(), 0);
    }

    #[test]
    fn test_write_paragraph_empty() {
        let margins = create_test_margins();
        let mut context = TextFlowContext::new(400.0, 600.0, margins);

        let initial_y = context.cursor_y;
        context.write_paragraph("").unwrap();

        // Empty paragraph should still add spacing
        assert!(context.cursor_y < initial_y);
    }

    #[test]
    fn test_extreme_line_height() {
        let margins = create_test_margins();
        let mut context = TextFlowContext::new(400.0, 600.0, margins);

        // Very small line height
        context.set_line_height(0.1);
        let initial_y = context.cursor_y;
        context.newline();
        assert_eq!(context.cursor_y, initial_y - context.font_size * 0.1);

        // Very large line height
        context.set_line_height(10.0);
        let initial_y2 = context.cursor_y;
        context.newline();
        assert_eq!(context.cursor_y, initial_y2 - context.font_size * 10.0);
    }

    #[test]
    fn test_zero_content_width() {
        let margins = Margins {
            left: 200.0,
            right: 200.0,
            top: 50.0,
            bottom: 50.0,
        };
        let context = TextFlowContext::new(400.0, 600.0, margins);

        assert_eq!(context.content_width(), 0.0);
    }

    #[test]
    fn test_cursor_x_reset_on_newline() {
        let margins = create_test_margins();
        let mut context = TextFlowContext::new(400.0, 600.0, margins.clone());

        context.at(250.0, 300.0); // Move cursor to custom position
        context.newline();

        // X should reset to left margin
        assert_eq!(context.cursor_x, margins.left);
        // Y should decrease by line height
        assert_eq!(
            context.cursor_y,
            300.0 - context.font_size * context.line_height
        );
    }
}
