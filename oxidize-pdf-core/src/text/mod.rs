pub mod cmap;
mod encoding;
mod extraction;
mod extraction_cmap;
mod flow;
mod font;
pub mod font_manager;
pub mod fonts;
mod header_footer;
pub mod header_footer_advanced;
mod layout;
mod list;
mod metrics;
pub mod ocr;
pub mod table;
mod table_advanced;

#[cfg(test)]
mod cmap_tests;

#[cfg(feature = "ocr-tesseract")]
pub mod tesseract_provider;

pub use encoding::TextEncoding;
pub use extraction::{ExtractedText, ExtractionOptions, TextExtractor, TextFragment};
pub use flow::{TextAlign, TextFlowContext};
pub use font::{Font, FontEncoding, FontFamily, FontWithEncoding};
pub use font_manager::{CustomFont, FontDescriptor, FontFlags, FontManager, FontMetrics, FontType};
pub use header_footer::{HeaderFooter, HeaderFooterOptions, HeaderFooterPosition};
pub use layout::{ColumnContent, ColumnLayout, ColumnOptions, TextFormat};
pub use list::{
    BulletStyle, ListElement, ListItem, ListOptions, ListStyle as ListStyleEnum, OrderedList,
    OrderedListStyle, UnorderedList,
};
pub use metrics::{measure_char, measure_text, split_into_words};
pub use ocr::{
    FragmentType, ImagePreprocessing, MockOcrProvider, OcrEngine, OcrError, OcrOptions,
    OcrProcessingResult, OcrProvider, OcrResult, OcrTextFragment,
};
pub use table::{HeaderStyle, Table, TableCell, TableOptions};
pub use table_advanced::{
    AdvancedTable, AdvancedTableCell, AdvancedTableOptions, AlternatingRowColors, BorderLine,
    BorderStyle, CellContent, CellPadding, ColumnDefinition, ColumnWidth, LineStyle, TableRow,
    VerticalAlign,
};

use crate::error::Result;
use std::fmt::Write;

/// Text rendering mode for PDF text operations
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextRenderingMode {
    /// Fill text (default)
    Fill = 0,
    /// Stroke text
    Stroke = 1,
    /// Fill and stroke text
    FillStroke = 2,
    /// Invisible text (for searchable text over images)
    Invisible = 3,
    /// Fill text and add to path for clipping
    FillClip = 4,
    /// Stroke text and add to path for clipping
    StrokeClip = 5,
    /// Fill and stroke text and add to path for clipping
    FillStrokeClip = 6,
    /// Add text to path for clipping (invisible)
    Clip = 7,
}

#[derive(Clone)]
pub struct TextContext {
    operations: String,
    current_font: Font,
    font_size: f64,
    text_matrix: [f64; 6],
    // Text state parameters
    character_spacing: Option<f64>,
    word_spacing: Option<f64>,
    horizontal_scaling: Option<f64>,
    leading: Option<f64>,
    text_rise: Option<f64>,
    rendering_mode: Option<TextRenderingMode>,
}

impl Default for TextContext {
    fn default() -> Self {
        Self::new()
    }
}

impl TextContext {
    pub fn new() -> Self {
        Self {
            operations: String::new(),
            current_font: Font::Helvetica,
            font_size: 12.0,
            text_matrix: [1.0, 0.0, 0.0, 1.0, 0.0, 0.0],
            character_spacing: None,
            word_spacing: None,
            horizontal_scaling: None,
            leading: None,
            text_rise: None,
            rendering_mode: None,
        }
    }

    pub fn set_font(&mut self, font: Font, size: f64) -> &mut Self {
        self.current_font = font;
        self.font_size = size;
        self
    }

    /// Get the current font
    #[allow(dead_code)]
    pub(crate) fn current_font(&self) -> &Font {
        &self.current_font
    }

    pub fn at(&mut self, x: f64, y: f64) -> &mut Self {
        self.text_matrix[4] = x;
        self.text_matrix[5] = y;
        self
    }

    pub fn write(&mut self, text: &str) -> Result<&mut Self> {
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

        // Apply text state parameters
        self.apply_text_state_parameters();

        // Set text position
        writeln!(
            &mut self.operations,
            "{:.2} {:.2} Td",
            self.text_matrix[4], self.text_matrix[5]
        )
        .unwrap();

        // Encode text using WinAnsiEncoding
        let encoding = TextEncoding::WinAnsiEncoding;
        let encoded_bytes = encoding.encode(text);

        // Show text as a literal string
        self.operations.push('(');
        for &byte in &encoded_bytes {
            match byte {
                b'(' => self.operations.push_str("\\("),
                b')' => self.operations.push_str("\\)"),
                b'\\' => self.operations.push_str("\\\\"),
                b'\n' => self.operations.push_str("\\n"),
                b'\r' => self.operations.push_str("\\r"),
                b'\t' => self.operations.push_str("\\t"),
                // For bytes in the printable ASCII range, write as is
                0x20..=0x7E => self.operations.push(byte as char),
                // For other bytes, write as octal escape sequences
                _ => write!(&mut self.operations, "\\{byte:03o}").unwrap(),
            }
        }
        self.operations.push_str(") Tj\n");

        // End text object
        self.operations.push_str("ET\n");

        Ok(self)
    }

    pub fn write_line(&mut self, text: &str) -> Result<&mut Self> {
        self.write(text)?;
        self.text_matrix[5] -= self.font_size * 1.2; // Move down for next line
        Ok(self)
    }

    pub fn set_character_spacing(&mut self, spacing: f64) -> &mut Self {
        self.character_spacing = Some(spacing);
        self
    }

    pub fn set_word_spacing(&mut self, spacing: f64) -> &mut Self {
        self.word_spacing = Some(spacing);
        self
    }

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

    /// Set the text rendering mode
    pub fn set_rendering_mode(&mut self, mode: TextRenderingMode) -> &mut Self {
        self.rendering_mode = Some(mode);
        self
    }

    /// Apply text state parameters to the operations string
    fn apply_text_state_parameters(&mut self) {
        // Character spacing (Tc)
        if let Some(spacing) = self.character_spacing {
            writeln!(&mut self.operations, "{:.2} Tc", spacing).unwrap();
        }

        // Word spacing (Tw)
        if let Some(spacing) = self.word_spacing {
            writeln!(&mut self.operations, "{:.2} Tw", spacing).unwrap();
        }

        // Horizontal scaling (Tz)
        if let Some(scale) = self.horizontal_scaling {
            writeln!(&mut self.operations, "{:.2} Tz", scale * 100.0).unwrap();
        }

        // Leading (TL)
        if let Some(leading) = self.leading {
            writeln!(&mut self.operations, "{:.2} TL", leading).unwrap();
        }

        // Text rise (Ts)
        if let Some(rise) = self.text_rise {
            writeln!(&mut self.operations, "{:.2} Ts", rise).unwrap();
        }

        // Text rendering mode (Tr)
        if let Some(mode) = self.rendering_mode {
            writeln!(&mut self.operations, "{} Tr", mode as u8).unwrap();
        }
    }

    pub(crate) fn generate_operations(&self) -> Result<Vec<u8>> {
        Ok(self.operations.as_bytes().to_vec())
    }

    /// Get the current font size
    pub fn font_size(&self) -> f64 {
        self.font_size
    }

    /// Get the current text matrix
    pub fn text_matrix(&self) -> [f64; 6] {
        self.text_matrix
    }

    /// Get the current position
    pub fn position(&self) -> (f64, f64) {
        (self.text_matrix[4], self.text_matrix[5])
    }

    /// Clear all operations and reset text state parameters
    pub fn clear(&mut self) {
        self.operations.clear();
        self.character_spacing = None;
        self.word_spacing = None;
        self.horizontal_scaling = None;
        self.leading = None;
        self.text_rise = None;
        self.rendering_mode = None;
    }

    /// Get the raw operations string
    pub fn operations(&self) -> &str {
        &self.operations
    }

    /// Generate text state operations for testing purposes
    #[cfg(test)]
    pub fn generate_text_state_operations(&self) -> String {
        let mut ops = String::new();

        // Character spacing (Tc)
        if let Some(spacing) = self.character_spacing {
            writeln!(&mut ops, "{:.2} Tc", spacing).unwrap();
        }

        // Word spacing (Tw)
        if let Some(spacing) = self.word_spacing {
            writeln!(&mut ops, "{:.2} Tw", spacing).unwrap();
        }

        // Horizontal scaling (Tz)
        if let Some(scale) = self.horizontal_scaling {
            writeln!(&mut ops, "{:.2} Tz", scale * 100.0).unwrap();
        }

        // Leading (TL)
        if let Some(leading) = self.leading {
            writeln!(&mut ops, "{:.2} TL", leading).unwrap();
        }

        // Text rise (Ts)
        if let Some(rise) = self.text_rise {
            writeln!(&mut ops, "{:.2} Ts", rise).unwrap();
        }

        // Text rendering mode (Tr)
        if let Some(mode) = self.rendering_mode {
            writeln!(&mut ops, "{} Tr", mode as u8).unwrap();
        }

        ops
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_text_context_new() {
        let context = TextContext::new();
        assert_eq!(context.current_font, Font::Helvetica);
        assert_eq!(context.font_size, 12.0);
        assert_eq!(context.text_matrix, [1.0, 0.0, 0.0, 1.0, 0.0, 0.0]);
        assert!(context.operations.is_empty());
    }

    #[test]
    fn test_text_context_default() {
        let context = TextContext::default();
        assert_eq!(context.current_font, Font::Helvetica);
        assert_eq!(context.font_size, 12.0);
    }

    #[test]
    fn test_set_font() {
        let mut context = TextContext::new();
        context.set_font(Font::TimesBold, 14.0);
        assert_eq!(context.current_font, Font::TimesBold);
        assert_eq!(context.font_size, 14.0);
    }

    #[test]
    fn test_position() {
        let mut context = TextContext::new();
        context.at(100.0, 200.0);
        let (x, y) = context.position();
        assert_eq!(x, 100.0);
        assert_eq!(y, 200.0);
        assert_eq!(context.text_matrix[4], 100.0);
        assert_eq!(context.text_matrix[5], 200.0);
    }

    #[test]
    fn test_write_simple_text() {
        let mut context = TextContext::new();
        context.write("Hello").unwrap();

        let ops = context.operations();
        assert!(ops.contains("BT\n"));
        assert!(ops.contains("ET\n"));
        assert!(ops.contains("/Helvetica 12 Tf"));
        assert!(ops.contains("(Hello) Tj"));
    }

    #[test]
    fn test_write_text_with_escaping() {
        let mut context = TextContext::new();
        context.write("(Hello)").unwrap();

        let ops = context.operations();
        assert!(ops.contains("(\\(Hello\\)) Tj"));
    }

    #[test]
    fn test_write_line() {
        let mut context = TextContext::new();
        let initial_y = context.text_matrix[5];
        context.write_line("Line 1").unwrap();

        // Y position should have moved down
        let new_y = context.text_matrix[5];
        assert!(new_y < initial_y);
        assert_eq!(new_y, initial_y - 12.0 * 1.2); // font_size * 1.2
    }

    #[test]
    fn test_character_spacing() {
        let mut context = TextContext::new();
        context.set_character_spacing(2.5);

        let ops = context.generate_text_state_operations();
        assert!(ops.contains("2.50 Tc"));
    }

    #[test]
    fn test_word_spacing() {
        let mut context = TextContext::new();
        context.set_word_spacing(1.5);

        let ops = context.generate_text_state_operations();
        assert!(ops.contains("1.50 Tw"));
    }

    #[test]
    fn test_horizontal_scaling() {
        let mut context = TextContext::new();
        context.set_horizontal_scaling(1.25);

        let ops = context.generate_text_state_operations();
        assert!(ops.contains("125.00 Tz")); // 1.25 * 100
    }

    #[test]
    fn test_leading() {
        let mut context = TextContext::new();
        context.set_leading(15.0);

        let ops = context.generate_text_state_operations();
        assert!(ops.contains("15.00 TL"));
    }

    #[test]
    fn test_text_rise() {
        let mut context = TextContext::new();
        context.set_text_rise(3.0);

        let ops = context.generate_text_state_operations();
        assert!(ops.contains("3.00 Ts"));
    }

    #[test]
    fn test_clear() {
        let mut context = TextContext::new();
        context.write("Hello").unwrap();
        assert!(!context.operations().is_empty());

        context.clear();
        assert!(context.operations().is_empty());
    }

    #[test]
    fn test_generate_operations() {
        let mut context = TextContext::new();
        context.write("Test").unwrap();

        let ops_bytes = context.generate_operations().unwrap();
        let ops_string = String::from_utf8(ops_bytes).unwrap();
        assert_eq!(ops_string, context.operations());
    }

    #[test]
    fn test_method_chaining() {
        let mut context = TextContext::new();
        context
            .set_font(Font::Courier, 10.0)
            .at(50.0, 100.0)
            .set_character_spacing(1.0)
            .set_word_spacing(2.0);

        assert_eq!(context.current_font(), &Font::Courier);
        assert_eq!(context.font_size(), 10.0);
        let (x, y) = context.position();
        assert_eq!(x, 50.0);
        assert_eq!(y, 100.0);
    }

    #[test]
    fn test_text_matrix_access() {
        let mut context = TextContext::new();
        context.at(25.0, 75.0);

        let matrix = context.text_matrix();
        assert_eq!(matrix, [1.0, 0.0, 0.0, 1.0, 25.0, 75.0]);
    }

    #[test]
    fn test_special_characters_encoding() {
        let mut context = TextContext::new();
        context.write("Test\nLine\tTab").unwrap();

        let ops = context.operations();
        assert!(ops.contains("\\n"));
        assert!(ops.contains("\\t"));
    }

    #[test]
    fn test_rendering_mode_fill() {
        let mut context = TextContext::new();
        context.set_rendering_mode(TextRenderingMode::Fill);

        let ops = context.generate_text_state_operations();
        assert!(ops.contains("0 Tr"));
    }

    #[test]
    fn test_rendering_mode_stroke() {
        let mut context = TextContext::new();
        context.set_rendering_mode(TextRenderingMode::Stroke);

        let ops = context.generate_text_state_operations();
        assert!(ops.contains("1 Tr"));
    }

    #[test]
    fn test_rendering_mode_fill_stroke() {
        let mut context = TextContext::new();
        context.set_rendering_mode(TextRenderingMode::FillStroke);

        let ops = context.generate_text_state_operations();
        assert!(ops.contains("2 Tr"));
    }

    #[test]
    fn test_rendering_mode_invisible() {
        let mut context = TextContext::new();
        context.set_rendering_mode(TextRenderingMode::Invisible);

        let ops = context.generate_text_state_operations();
        assert!(ops.contains("3 Tr"));
    }

    #[test]
    fn test_rendering_mode_fill_clip() {
        let mut context = TextContext::new();
        context.set_rendering_mode(TextRenderingMode::FillClip);

        let ops = context.generate_text_state_operations();
        assert!(ops.contains("4 Tr"));
    }

    #[test]
    fn test_rendering_mode_stroke_clip() {
        let mut context = TextContext::new();
        context.set_rendering_mode(TextRenderingMode::StrokeClip);

        let ops = context.generate_text_state_operations();
        assert!(ops.contains("5 Tr"));
    }

    #[test]
    fn test_rendering_mode_fill_stroke_clip() {
        let mut context = TextContext::new();
        context.set_rendering_mode(TextRenderingMode::FillStrokeClip);

        let ops = context.generate_text_state_operations();
        assert!(ops.contains("6 Tr"));
    }

    #[test]
    fn test_rendering_mode_clip() {
        let mut context = TextContext::new();
        context.set_rendering_mode(TextRenderingMode::Clip);

        let ops = context.generate_text_state_operations();
        assert!(ops.contains("7 Tr"));
    }

    #[test]
    fn test_text_state_parameters_chaining() {
        let mut context = TextContext::new();
        context
            .set_character_spacing(1.5)
            .set_word_spacing(2.0)
            .set_horizontal_scaling(1.1)
            .set_leading(14.0)
            .set_text_rise(0.5)
            .set_rendering_mode(TextRenderingMode::FillStroke);

        let ops = context.generate_text_state_operations();
        assert!(ops.contains("1.50 Tc"));
        assert!(ops.contains("2.00 Tw"));
        assert!(ops.contains("110.00 Tz"));
        assert!(ops.contains("14.00 TL"));
        assert!(ops.contains("0.50 Ts"));
        assert!(ops.contains("2 Tr"));
    }

    #[test]
    fn test_all_text_state_operators_generated() {
        let mut context = TextContext::new();

        // Test all operators in sequence
        context.set_character_spacing(1.0); // Tc
        context.set_word_spacing(2.0); // Tw
        context.set_horizontal_scaling(1.2); // Tz
        context.set_leading(15.0); // TL
        context.set_text_rise(1.0); // Ts
        context.set_rendering_mode(TextRenderingMode::Stroke); // Tr

        let ops = context.generate_text_state_operations();

        // Verify all PDF text state operators are present
        assert!(
            ops.contains("Tc"),
            "Character spacing operator (Tc) not found"
        );
        assert!(ops.contains("Tw"), "Word spacing operator (Tw) not found");
        assert!(
            ops.contains("Tz"),
            "Horizontal scaling operator (Tz) not found"
        );
        assert!(ops.contains("TL"), "Leading operator (TL) not found");
        assert!(ops.contains("Ts"), "Text rise operator (Ts) not found");
        assert!(
            ops.contains("Tr"),
            "Text rendering mode operator (Tr) not found"
        );
    }
}
