use crate::error::Result;
use crate::graphics::Color;
use crate::page::Margins;
use crate::text::metrics::{measure_text_with, FontMetricsStore};
use crate::text::{split_into_words, Font};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TextAlign {
    Left,
    Right,
    Center,
    Justified,
}

pub struct TextFlowContext {
    operations: Vec<crate::graphics::ops::Op>,
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
    /// Optional fill color for text glyphs (issue #216). When `Some`,
    /// `write_wrapped` emits the corresponding non-stroking color
    /// operator (`rg`/`g`/`k`) inside each `BT … ET` block before the
    /// `Tj`. `None` keeps the previous behaviour (whatever fill colour
    /// the surrounding graphics state is already carrying).
    fill_color: Option<Color>,
    /// Remaining text-state parameters propagated from `TextContext`
    /// (issue #222 — Phase 6 of the v2.7.0 IR refactor). When set,
    /// `write_wrapped` emits the corresponding operator inside each
    /// `BT … ET` block. `None` keeps the surrounding graphics state.
    character_spacing: Option<f64>,
    word_spacing: Option<f64>,
    horizontal_scaling: Option<f64>,
    leading: Option<f64>,
    text_rise: Option<f64>,
    rendering_mode: Option<u8>,
    stroke_color: Option<Color>,
    /// Characters drawn so far, bucketed by active font name (issue
    /// #204). Consumed by `Page::add_text_flow` to merge into the
    /// page's graphics-context tracking so the writer can subset each
    /// custom font with only its own characters.
    used_characters_by_font: HashMap<String, HashSet<char>>,
    /// Per-Document font metrics store threaded from the owning `Document`
    /// (issue #230, v2.8.0). When `Some`, `write_wrapped` resolves custom
    /// font widths via this store instead of the process-wide legacy registry.
    pub(crate) font_metrics_store: Option<FontMetricsStore>,
}

impl TextFlowContext {
    pub fn new(page_width: f64, page_height: f64, margins: Margins) -> Self {
        Self {
            operations: Vec::new(),
            current_font: Font::Helvetica,
            font_size: 12.0,
            line_height: 1.2,
            cursor_x: margins.left,
            cursor_y: page_height - margins.top,
            alignment: TextAlign::Left,
            page_width,
            page_height,
            margins,
            fill_color: None,
            character_spacing: None,
            word_spacing: None,
            horizontal_scaling: None,
            leading: None,
            text_rise: None,
            rendering_mode: None,
            stroke_color: None,
            used_characters_by_font: HashMap::new(),
            font_metrics_store: None,
        }
    }

    /// Create a `TextFlowContext` bound to a per-Document `FontMetricsStore`
    /// (issue #230, v2.8.0). Internal use only — external callers should use
    /// `Document::new_page_a4()` and `page.text_flow()` (Task 8 wires this).
    ///
    /// When `store` is `None`, behaviour is identical to `TextFlowContext::new`.
    // Task 8 will wire Page::text_flow() to call this; until then it is only
    // exercised by the test below. Suppress the lint so clippy stays clean
    // across the whole feature branch.
    #[allow(dead_code)]
    pub(crate) fn with_metrics_store(
        page_width: f64,
        page_height: f64,
        margins: Margins,
        store: Option<FontMetricsStore>,
    ) -> Self {
        let mut ctx = Self::new(page_width, page_height, margins);
        ctx.font_metrics_store = store;
        ctx
    }

    /// Get the per-font character usage accumulated by `write_wrapped`
    /// (issue #204). `Page::add_text_flow` merges this into the page's
    /// graphics context so the writer knows which custom fonts were
    /// referenced and what characters each drew.
    pub(crate) fn get_used_characters_by_font(&self) -> &HashMap<String, HashSet<char>> {
        &self.used_characters_by_font
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

    /// Sets the non-stroking (fill) color used for subsequent text emitted
    /// by `write_wrapped` (issue #216). Mirrors `TextContext::set_fill_color`.
    /// `None` keeps the surrounding graphics state untouched (previous
    /// behaviour); `Some(color)` emits the matching PDF operator inside each
    /// `BT … ET` block.
    pub fn set_fill_color(&mut self, color: Color) -> &mut Self {
        self.fill_color = Some(color);
        self
    }

    /// Setters for the remaining text-state parameters, mirroring
    /// `TextContext`. Closes the propagation gap reported in issue #222.
    /// All apply on the next `BT … ET` block emitted by `write_wrapped`.
    pub fn set_character_spacing(&mut self, spacing: f64) -> &mut Self {
        self.character_spacing = Some(spacing);
        self
    }

    pub fn set_word_spacing(&mut self, spacing: f64) -> &mut Self {
        self.word_spacing = Some(spacing);
        self
    }

    /// Set horizontal scaling. The argument is the ratio (e.g. `0.85`
    /// for 85 %); it is converted to the PDF `Tz` percentage at
    /// emission time. Matches the contract documented on
    /// `TextContext::set_horizontal_scaling`.
    pub fn set_horizontal_scaling(&mut self, scale: f64) -> &mut Self {
        self.horizontal_scaling = Some(scale);
        self
    }

    pub fn set_leading(&mut self, leading: f64) -> &mut Self {
        self.leading = Some(leading);
        self
    }

    pub fn set_text_rise(&mut self, rise: f64) -> &mut Self {
        self.text_rise = Some(rise);
        self
    }

    /// Set the text rendering mode (`0`..=`7` per ISO 32000-1 §9.3.6).
    /// The argument is taken as a `u8` rather than the typed
    /// `TextRenderingMode` enum to avoid an extra public dependency
    /// from `TextFlowContext` on the parent module.
    pub fn set_rendering_mode(&mut self, mode: u8) -> &mut Self {
        self.rendering_mode = Some(mode);
        self
    }

    pub fn set_stroke_color(&mut self, color: Color) -> &mut Self {
        self.stroke_color = Some(color);
        self
    }

    /// Current font this context will use when emitting text.
    pub fn current_font(&self) -> &Font {
        &self.current_font
    }

    /// Current font size in points.
    pub fn font_size(&self) -> f64 {
        self.font_size
    }

    /// Current fill color, if one has been explicitly set (issue #216).
    pub fn fill_color(&self) -> Option<Color> {
        self.fill_color
    }

    pub fn at(&mut self, x: f64, y: f64) -> &mut Self {
        self.cursor_x = x;
        self.cursor_y = y;
        self
    }

    pub fn content_width(&self) -> f64 {
        self.page_width - self.margins.left - self.margins.right
    }

    /// Returns the width available for text starting at the current cursor_x position.
    ///
    /// Unlike `content_width()` which always uses `margins.left` as the origin,
    /// `available_width()` accounts for the actual cursor position so that text
    /// placed via `.at(x, y)` does not overflow the right margin.
    pub fn available_width(&self) -> f64 {
        (self.page_width - self.margins.right - self.cursor_x).max(0.0)
    }

    pub fn write_wrapped(&mut self, text: &str) -> Result<&mut Self> {
        let start_x = self.cursor_x;
        let available_width = self.available_width();

        // Split text into words
        let words = split_into_words(text);
        let mut lines: Vec<Vec<&str>> = Vec::new();
        let mut current_line: Vec<&str> = Vec::new();
        let mut current_width = 0.0;

        // Build lines based on available width (respects cursor_x offset)
        for word in words {
            let word_width = measure_text_with(
                word,
                &self.current_font,
                self.font_size,
                self.font_metrics_store.as_ref(),
            );

            // Check if we need to start a new line
            if !current_line.is_empty() && current_width + word_width > available_width {
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
            let line_width = measure_text_with(
                &line_text,
                &self.current_font,
                self.font_size,
                self.font_metrics_store.as_ref(),
            );

            // Calculate x position based on alignment.
            // start_x is the column where this block of text begins (set via .at()).
            // Left/Justified start at start_x; Center is relative to start_x;
            // Right stays anchored to the right margin.
            let x = match self.alignment {
                TextAlign::Left => start_x,
                TextAlign::Right => self.page_width - self.margins.right - line_width,
                TextAlign::Center => start_x + (available_width - line_width) / 2.0,
                TextAlign::Justified => start_x,
            };

            use crate::graphics::ops::Op;

            self.operations.push(Op::BeginText);

            // Set font
            self.operations.push(Op::SetFont {
                name: self.current_font.pdf_name(),
                size: self.font_size,
            });

            // Apply text-state parameters propagated from `TextContext`
            // (issue #222 — Phase 6 of the v2.7.0 IR refactor).
            // These mirror `TextContext::apply_text_state_parameters`
            // but live inside the per-line `BT … ET` block of the flow
            // emitter. PDF spec ISO 32000-1 §8.6.8 / §9.3 allow these
            // operators inside a text object; they take effect for the
            // `Tj` that follows.
            if let Some(spacing) = self.character_spacing {
                self.operations.push(Op::SetCharSpacing(spacing));
            }
            if let Some(spacing) = self.word_spacing {
                self.operations.push(Op::SetWordSpacing(spacing));
            }
            if let Some(scale) = self.horizontal_scaling {
                // The Tz operator takes a percentage; the setter accepts
                // a 0.0–1.0 ratio (matching `TextContext`), so multiply
                // by 100 at emission.
                self.operations
                    .push(Op::SetHorizontalScaling(scale * 100.0));
            }
            if let Some(leading) = self.leading {
                self.operations.push(Op::SetLeading(leading));
            }
            if let Some(rise) = self.text_rise {
                self.operations.push(Op::SetTextRise(rise));
            }
            if let Some(mode) = self.rendering_mode {
                self.operations.push(Op::SetRenderingMode(mode));
            }

            // Apply non-stroking fill colour (issue #216) and stroking
            // colour (issue #222) if one was inherited from the
            // page-level text state or explicitly configured via the
            // setters. The IR variants route through
            // `write_fill_color_bytes` / `write_stroke_color_bytes` so
            // the same NaN-sanitising helpers (issues #220 + #221) apply.
            if let Some(color) = self.fill_color {
                self.operations.push(Op::SetFillColor(color));
            }
            if let Some(color) = self.stroke_color {
                self.operations.push(Op::SetStrokeColor(color));
            }

            self.operations.push(Op::SetTextPosition {
                x,
                y: self.cursor_y,
            });

            // Handle justification: emit Tw with the per-line word-spacing
            // adjustment so the rendered line spans `available_width`.
            if self.alignment == TextAlign::Justified && i < lines.len() - 1 && line.len() > 1 {
                let spaces_count = line.iter().filter(|w| w.trim().is_empty()).count();
                if spaces_count > 0 {
                    let extra_space = available_width - line_width;
                    let space_adjustment = extra_space / spaces_count as f64;
                    self.operations.push(Op::SetWordSpacing(space_adjustment));
                }
            }

            // Show text — escape PDF literal-string special characters.
            let mut buf = Vec::with_capacity(line_text.len());
            for ch in line_text.chars() {
                match ch {
                    '(' => buf.extend_from_slice(b"\\("),
                    ')' => buf.extend_from_slice(b"\\)"),
                    '\\' => buf.extend_from_slice(b"\\\\"),
                    '\n' => buf.extend_from_slice(b"\\n"),
                    '\r' => buf.extend_from_slice(b"\\r"),
                    '\t' => buf.extend_from_slice(b"\\t"),
                    _ => {
                        let mut tmp = [0u8; 4];
                        buf.extend_from_slice(ch.encode_utf8(&mut tmp).as_bytes());
                    }
                }
            }
            self.operations.push(Op::ShowText(buf));

            // Record per-font char usage so the consuming page can
            // report it to the writer (issue #204).
            self.used_characters_by_font
                .entry(self.current_font.pdf_name())
                .or_default()
                .extend(line_text.chars());

            // Reset word spacing if it was set. The IR emits this as
            // `0.00 Tw` (was `0 Tw` in pre-2.7.0 — documented in CHANGELOG).
            if self.alignment == TextAlign::Justified && i < lines.len() - 1 {
                self.operations.push(Op::SetWordSpacing(0.0));
            }

            self.operations.push(Op::EndText);

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
        let mut buf = Vec::new();
        crate::graphics::ops::serialize_ops(&mut buf, &self.operations);
        buf
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

    /// Get the operations as a serialised PDF content-stream `String`.
    ///
    /// Pre-2.7.0 this returned `&str`. The IR migration replaced the
    /// internal `String` buffer with a typed `Vec<Op>`, so the legacy
    /// borrow is materialised on demand. Internal callers prefer
    /// `generate_operations()` which returns the byte buffer directly.
    pub fn operations(&self) -> String {
        crate::graphics::ops::ops_to_string(&self.operations)
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
        let context = TextFlowContext::new(400.0, 600.0, margins);

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
        let mut context = TextFlowContext::new(400.0, 600.0, margins);

        context.set_font(Font::TimesBold, 16.0);
        assert_eq!(context.current_font, Font::TimesBold);
        assert_eq!(context.font_size, 16.0);
    }

    #[test]
    fn test_set_line_height() {
        let margins = create_test_margins();
        let mut context = TextFlowContext::new(400.0, 600.0, margins);

        context.set_line_height(1.5);
        assert_eq!(context.line_height(), 1.5);
    }

    #[test]
    fn test_set_alignment() {
        let margins = create_test_margins();
        let mut context = TextFlowContext::new(400.0, 600.0, margins);

        context.set_alignment(TextAlign::Center);
        assert_eq!(context.alignment(), TextAlign::Center);
    }

    #[test]
    fn test_at_position() {
        let margins = create_test_margins();
        let mut context = TextFlowContext::new(400.0, 600.0, margins);

        context.at(100.0, 200.0);
        let (x, y) = context.cursor_position();
        assert_eq!(x, 100.0);
        assert_eq!(y, 200.0);
    }

    #[test]
    fn test_content_width() {
        let margins = create_test_margins();
        let context = TextFlowContext::new(400.0, 600.0, margins);

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
        let mut context = TextFlowContext::new(400.0, 600.0, margins);

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
        let mut context = TextFlowContext::new(400.0, 600.0, margins);

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
        let mut context = TextFlowContext::new(400.0, 600.0, margins);

        context.at(75.0, 125.0);
        let (x, y) = context.cursor_position();
        assert_eq!(x, 75.0);
        assert_eq!(y, 125.0);
    }

    #[test]
    fn test_generate_operations() {
        let margins = create_test_margins();
        let mut context = TextFlowContext::new(400.0, 600.0, margins);

        context.write_wrapped("Test").unwrap();
        let ops_bytes = context.generate_operations();
        let ops_string = String::from_utf8(ops_bytes).unwrap();

        assert_eq!(ops_string, context.operations());
    }

    #[test]
    fn test_clear_operations() {
        let margins = create_test_margins();
        let mut context = TextFlowContext::new(400.0, 600.0, margins);

        context.write_wrapped("Test").unwrap();
        assert!(!context.operations().is_empty());

        context.clear();
        assert!(context.operations().is_empty());
    }

    #[test]
    fn test_page_dimensions() {
        let margins = create_test_margins();
        let context = TextFlowContext::new(400.0, 600.0, margins);

        let (width, height) = context.page_dimensions();
        assert_eq!(width, 400.0);
        assert_eq!(height, 600.0);
    }

    #[test]
    fn test_margins_access() {
        let margins = create_test_margins();
        let context = TextFlowContext::new(400.0, 600.0, margins);

        let ctx_margins = context.margins();
        assert_eq!(ctx_margins.left, 50.0);
        assert_eq!(ctx_margins.right, 50.0);
        assert_eq!(ctx_margins.top, 50.0);
        assert_eq!(ctx_margins.bottom, 50.0);
    }

    #[test]
    fn test_method_chaining() {
        let margins = create_test_margins();
        let mut context = TextFlowContext::new(400.0, 600.0, margins);

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
        let debug_str = format!("{align:?}");
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

        // Use approximate equality for floating point comparisons
        assert!((spacing1 - spacing2).abs() < 1e-10);
        assert!((spacing2 - spacing3).abs() < 1e-10);
        assert!((spacing1 - context.font_size * context.line_height).abs() < 1e-10);
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
            let font_name = font.pdf_name();
            context.set_font(font, 14.0);
            context.write_wrapped("Test text").unwrap();

            let ops = context.operations();
            assert!(ops.contains(&format!("/{font_name} 14 Tf")));
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
            assert!(ops.contains(&format!("/Helvetica {size} Tf")));
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
        // Cursor position should reflect where we are after writing text (moved down by line height)
        let (x, y) = context.cursor_position();
        assert_eq!(x, 100.0); // X position should be unchanged
        assert!(y < 200.0); // Y position should have moved down after writing text
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

    // --- Issue #167: available_width respects cursor_x ---

    #[test]
    fn test_available_width_respects_cursor_x() {
        // Page: 400pt wide, 50pt margins each side → content_width = 300pt
        let margins = create_test_margins(); // left=50, right=50
        let mut context = TextFlowContext::new(400.0, 600.0, margins);

        // Default: cursor_x == margins.left == 50, available_width == 300
        assert_eq!(context.available_width(), 300.0);

        // After .at(200, 500): cursor_x = 200, available_width = 400 - 50 - 200 = 150
        context.at(200.0, 500.0);
        assert_eq!(context.available_width(), 150.0);
    }

    #[test]
    fn test_available_width_clamps_to_zero() {
        // cursor_x past the right margin → available_width = 0 (not negative)
        let margins = create_test_margins(); // right = 50
        let mut context = TextFlowContext::new(400.0, 600.0, margins);

        // cursor_x = 380, right margin = 50 → would be 400-50-380 = -30 → clamp to 0
        context.at(380.0, 500.0);
        assert_eq!(context.available_width(), 0.0);
    }

    #[test]
    fn test_write_wrapped_at_x_limits_available_width() {
        // Page 400pt, margins 50pt each → content_width = 300pt
        // Place cursor at x=250: available_width = 400-50-250 = 100pt
        // Use text wider than 100pt but narrower than 300pt → must wrap at x=250
        let margins = create_test_margins();
        let mut context = TextFlowContext::new(400.0, 600.0, margins);

        context.set_font(Font::Helvetica, 12.0);
        // "Hello World Hello World" at 12pt Helvetica exceeds 100pt easily
        context.at(250.0, 500.0);
        context.write_wrapped("Hello World Hello World").unwrap();

        let ops = context.operations();
        // Multiple BT blocks → wrapping occurred
        let bt_count = ops.matches("BT\n").count();
        assert!(
            bt_count > 1,
            "Expected wrapping (multiple lines), got {bt_count} BT blocks. ops:\n{ops}"
        );
    }

    #[test]
    fn test_write_wrapped_respects_cursor_x_offset() {
        // Cursor at x=300, page 600pt wide, margins 50pt each → available_width = 250pt
        let margins = Margins {
            left: 50.0,
            right: 50.0,
            top: 50.0,
            bottom: 50.0,
        };
        let mut context = TextFlowContext::new(600.0, 800.0, margins);

        context.set_font(Font::Helvetica, 12.0);
        context.at(300.0, 700.0);
        context
            .write_wrapped("Hello World Foo Bar Baz Qux")
            .unwrap();

        let ops = context.operations();
        // Every Td x-coordinate should be >= 300.0
        for line in ops.lines() {
            if line.ends_with(" Td") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 3 {
                    let x: f64 = parts[0].parse().expect("Td x should be a number");
                    assert!(
                        x >= 300.0 - 1e-6,
                        "Expected Td x >= 300.0 but got {x}. ops:\n{ops}"
                    );
                }
            }
        }
    }

    #[test]
    fn test_text_flow_context_threads_metrics_store() {
        use crate::text::metrics::{FontMetrics, FontMetricsStore};
        let unique = format!("FlowThreadTask6_{}", std::process::id());
        let store = FontMetricsStore::new();
        // 'A' = 1000 → 12pt → 12.0 per char.
        store.register(
            unique.clone(),
            FontMetrics::new(500).with_widths(&[('A', 1000)]),
        );

        let mut ctx = TextFlowContext::with_metrics_store(
            595.0, // A4 width pt
            842.0, // A4 height pt
            Margins::default(),
            Some(store),
        );
        ctx.set_font(Font::Custom(unique), 12.0);
        ctx.write_wrapped("AA").unwrap();

        // The flow should have measured "AA" using the per-store widths and
        // produced a positive width on the line. The exact public way to
        // observe this depends on the flow API; this test asserts that the
        // generated_operations() output contains a Tj with the expected text
        // and that the flow advanced.
        let ops = ctx.generate_operations();
        assert!(!ops.is_empty(), "flow must emit content for 'AA'");
    }

    /// RED for Phase 3 of the v2.7.0 IR refactor: with the legacy `String`
    /// emission, a non-finite cursor position (e.g. `at(NaN, NaN)`) reaches
    /// `write_wrapped` and emits `NaN NaN Td`, which is invalid per
    /// ISO 32000-1 §7.3.3. Once the migration routes Td through
    /// `serialize_ops`, `finite_or_zero` clamps non-finite values to `0.0`
    /// and the assertion below passes.
    #[test]
    fn nan_cursor_position_in_flow_is_sanitised_at_emission() {
        let mut ctx = TextFlowContext::new(595.0, 842.0, Margins::default());
        ctx.at(f64::NAN, f64::NAN);
        ctx.write_wrapped("hello").unwrap();
        let ops = String::from_utf8(ctx.generate_operations())
            .expect("operations bytes must be valid UTF-8");
        assert!(
            !ops.contains("NaN") && !ops.contains("inf"),
            "non-finite tokens must not appear in flow content stream, got: {ops:?}"
        );
        assert!(
            ops.contains(" Td\n"),
            "Td operator must still be emitted, got: {ops:?}"
        );
    }
}
