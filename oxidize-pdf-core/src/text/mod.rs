pub mod cmap;
mod encoding;
pub mod extraction;
mod extraction_cmap;
mod flow;
mod font;
pub mod font_manager;
pub mod fonts;
mod header_footer;
pub mod invoice;
mod layout;
mod list;
pub mod metrics;
pub mod ocr;
pub mod plaintext;
pub mod structured;
pub mod table;
pub mod table_detection;
pub mod validation;

#[cfg(test)]
mod cmap_tests;

#[cfg(feature = "ocr-tesseract")]
pub mod tesseract_provider;

pub use encoding::TextEncoding;
pub use extraction::{
    sanitize_extracted_text, ExtractedText, ExtractionOptions, TextExtractor, TextFragment,
};
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
    CharacterConfidence, CorrectionCandidate, CorrectionReason, CorrectionSuggestion,
    CorrectionType, FragmentType, ImagePreprocessing, MockOcrProvider, OcrEngine, OcrError,
    OcrOptions, OcrPostProcessor, OcrProcessingResult, OcrProvider, OcrRegion, OcrResult,
    OcrTextFragment, WordConfidence,
};
pub use plaintext::{LineBreakMode, PlainTextConfig, PlainTextExtractor, PlainTextResult};
pub use table::{HeaderStyle, Table, TableCell, TableOptions};
pub use validation::{MatchType, TextMatch, TextValidationResult, TextValidator};

#[cfg(feature = "ocr-tesseract")]
pub use tesseract_provider::{RustyTesseractConfig, RustyTesseractProvider};

use crate::error::Result;
use crate::Color;
use std::collections::HashSet;
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
    // Pending position for next write operation
    pending_position: Option<(f64, f64)>,
    // Text state parameters
    character_spacing: Option<f64>,
    word_spacing: Option<f64>,
    horizontal_scaling: Option<f64>,
    leading: Option<f64>,
    text_rise: Option<f64>,
    rendering_mode: Option<TextRenderingMode>,
    // Color parameters
    fill_color: Option<Color>,
    stroke_color: Option<Color>,
    // Track used characters for font subsetting (fixes issue #97)
    used_characters: HashSet<char>,
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
            pending_position: None,
            character_spacing: None,
            word_spacing: None,
            horizontal_scaling: None,
            leading: None,
            text_rise: None,
            rendering_mode: None,
            fill_color: None,
            stroke_color: None,
            used_characters: HashSet::new(),
        }
    }

    /// Get the characters used in this text context for font subsetting.
    ///
    /// This is used to determine which glyphs need to be embedded when using
    /// custom fonts (especially CJK fonts).
    pub(crate) fn get_used_characters(&self) -> Option<HashSet<char>> {
        if self.used_characters.is_empty() {
            None
        } else {
            Some(self.used_characters.clone())
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
        // Update text_matrix immediately and store for write() operation
        self.text_matrix[4] = x;
        self.text_matrix[5] = y;
        self.pending_position = Some((x, y));
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
        .expect("Writing to String should never fail");

        // Apply text state parameters
        self.apply_text_state_parameters();

        // Set text position using pending_position if available, otherwise use text_matrix
        let (x, y) = if let Some((px, py)) = self.pending_position.take() {
            // Use and consume the pending position
            (px, py)
        } else {
            // Fallback to text_matrix values
            (self.text_matrix[4], self.text_matrix[5])
        };

        writeln!(&mut self.operations, "{:.2} {:.2} Td", x, y)
            .expect("Writing to String should never fail");

        // Choose encoding based on font type
        match &self.current_font {
            Font::Custom(_) => {
                // For custom fonts (CJK), use UTF-16BE encoding with hex strings
                let utf16_units: Vec<u16> = text.encode_utf16().collect();
                let mut utf16be_bytes = Vec::new();

                for unit in utf16_units {
                    utf16be_bytes.push((unit >> 8) as u8); // High byte
                    utf16be_bytes.push((unit & 0xFF) as u8); // Low byte
                }

                // Write as hex string for Type0 fonts
                self.operations.push('<');
                for &byte in &utf16be_bytes {
                    write!(&mut self.operations, "{:02X}", byte)
                        .expect("Writing to String should never fail");
                }
                self.operations.push_str("> Tj\n");
            }
            _ => {
                // For standard fonts, use WinAnsiEncoding with literal strings
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
                        _ => write!(&mut self.operations, "\\{byte:03o}")
                            .expect("Writing to String should never fail"),
                    }
                }
                self.operations.push_str(") Tj\n");
            }
        }

        // Track used characters for font subsetting (fixes issue #97)
        self.used_characters.extend(text.chars());

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

    /// Set the text fill color
    pub fn set_fill_color(&mut self, color: Color) -> &mut Self {
        self.fill_color = Some(color);
        self
    }

    /// Set the text stroke color
    pub fn set_stroke_color(&mut self, color: Color) -> &mut Self {
        self.stroke_color = Some(color);
        self
    }

    /// Apply text state parameters to the operations string
    fn apply_text_state_parameters(&mut self) {
        // Character spacing (Tc)
        if let Some(spacing) = self.character_spacing {
            writeln!(&mut self.operations, "{spacing:.2} Tc")
                .expect("Writing to String should never fail");
        }

        // Word spacing (Tw)
        if let Some(spacing) = self.word_spacing {
            writeln!(&mut self.operations, "{spacing:.2} Tw")
                .expect("Writing to String should never fail");
        }

        // Horizontal scaling (Tz)
        if let Some(scale) = self.horizontal_scaling {
            writeln!(&mut self.operations, "{:.2} Tz", scale * 100.0)
                .expect("Writing to String should never fail");
        }

        // Leading (TL)
        if let Some(leading) = self.leading {
            writeln!(&mut self.operations, "{leading:.2} TL")
                .expect("Writing to String should never fail");
        }

        // Text rise (Ts)
        if let Some(rise) = self.text_rise {
            writeln!(&mut self.operations, "{rise:.2} Ts")
                .expect("Writing to String should never fail");
        }

        // Text rendering mode (Tr)
        if let Some(mode) = self.rendering_mode {
            writeln!(&mut self.operations, "{} Tr", mode as u8)
                .expect("Writing to String should never fail");
        }

        // Fill color
        if let Some(color) = self.fill_color {
            match color {
                Color::Rgb(r, g, b) => {
                    writeln!(&mut self.operations, "{r:.3} {g:.3} {b:.3} rg")
                        .expect("Writing to String should never fail");
                }
                Color::Gray(gray) => {
                    writeln!(&mut self.operations, "{gray:.3} g")
                        .expect("Writing to String should never fail");
                }
                Color::Cmyk(c, m, y, k) => {
                    writeln!(&mut self.operations, "{c:.3} {m:.3} {y:.3} {k:.3} k")
                        .expect("Writing to String should never fail");
                }
            }
        }

        // Stroke color
        if let Some(color) = self.stroke_color {
            match color {
                Color::Rgb(r, g, b) => {
                    writeln!(&mut self.operations, "{r:.3} {g:.3} {b:.3} RG")
                        .expect("Writing to String should never fail");
                }
                Color::Gray(gray) => {
                    writeln!(&mut self.operations, "{gray:.3} G")
                        .expect("Writing to String should never fail");
                }
                Color::Cmyk(c, m, y, k) => {
                    writeln!(&mut self.operations, "{c:.3} {m:.3} {y:.3} {k:.3} K")
                        .expect("Writing to String should never fail");
                }
            }
        }
    }

    pub(crate) fn generate_operations(&self) -> Result<Vec<u8>> {
        Ok(self.operations.as_bytes().to_vec())
    }

    /// Appends a raw PDF operation to the text context
    ///
    /// This is used internally for marked content operators (BDC/EMC) and other
    /// low-level PDF operations that need to be interleaved with text operations.
    pub(crate) fn append_raw_operation(&mut self, operation: &str) {
        self.operations.push_str(operation);
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
        self.fill_color = None;
        self.stroke_color = None;
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
            writeln!(&mut ops, "{spacing:.2} Tc").unwrap();
        }

        // Word spacing (Tw)
        if let Some(spacing) = self.word_spacing {
            writeln!(&mut ops, "{spacing:.2} Tw").unwrap();
        }

        // Horizontal scaling (Tz)
        if let Some(scale) = self.horizontal_scaling {
            writeln!(&mut ops, "{:.2} Tz", scale * 100.0).unwrap();
        }

        // Leading (TL)
        if let Some(leading) = self.leading {
            writeln!(&mut ops, "{leading:.2} TL").unwrap();
        }

        // Text rise (Ts)
        if let Some(rise) = self.text_rise {
            writeln!(&mut ops, "{rise:.2} Ts").unwrap();
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

    #[test]
    fn test_text_color_operations() {
        use crate::Color;

        let mut context = TextContext::new();

        // Test RGB fill color
        context.set_fill_color(Color::rgb(1.0, 0.0, 0.0));
        context.apply_text_state_parameters();

        let ops = context.operations();
        assert!(
            ops.contains("1.000 0.000 0.000 rg"),
            "RGB fill color operator (rg) not found in: {ops}"
        );

        // Clear and test RGB stroke color
        context.clear();
        context.set_stroke_color(Color::rgb(0.0, 1.0, 0.0));
        context.apply_text_state_parameters();

        let ops = context.operations();
        assert!(
            ops.contains("0.000 1.000 0.000 RG"),
            "RGB stroke color operator (RG) not found in: {ops}"
        );

        // Clear and test grayscale fill color
        context.clear();
        context.set_fill_color(Color::gray(0.5));
        context.apply_text_state_parameters();

        let ops = context.operations();
        assert!(
            ops.contains("0.500 g"),
            "Gray fill color operator (g) not found in: {ops}"
        );

        // Clear and test CMYK stroke color
        context.clear();
        context.set_stroke_color(Color::cmyk(0.2, 0.3, 0.4, 0.1));
        context.apply_text_state_parameters();

        let ops = context.operations();
        assert!(
            ops.contains("0.200 0.300 0.400 0.100 K"),
            "CMYK stroke color operator (K) not found in: {ops}"
        );

        // Test both fill and stroke colors together
        context.clear();
        context.set_fill_color(Color::rgb(1.0, 0.0, 0.0));
        context.set_stroke_color(Color::rgb(0.0, 0.0, 1.0));
        context.apply_text_state_parameters();

        let ops = context.operations();
        assert!(
            ops.contains("1.000 0.000 0.000 rg") && ops.contains("0.000 0.000 1.000 RG"),
            "Both fill and stroke colors not found in: {ops}"
        );
    }

    // Issue #97: Test used_characters tracking
    #[test]
    fn test_used_characters_tracking_ascii() {
        let mut context = TextContext::new();
        context.write("Hello").unwrap();

        let chars = context.get_used_characters();
        assert!(chars.is_some());
        let chars = chars.unwrap();
        assert!(chars.contains(&'H'));
        assert!(chars.contains(&'e'));
        assert!(chars.contains(&'l'));
        assert!(chars.contains(&'o'));
        assert_eq!(chars.len(), 4); // H, e, l, o (l appears twice but HashSet dedupes)
    }

    #[test]
    fn test_used_characters_tracking_cjk() {
        let mut context = TextContext::new();
        context.set_font(Font::Custom("NotoSansCJK".to_string()), 12.0);
        context.write("中文测试").unwrap();

        let chars = context.get_used_characters();
        assert!(chars.is_some());
        let chars = chars.unwrap();
        assert!(chars.contains(&'中'));
        assert!(chars.contains(&'文'));
        assert!(chars.contains(&'测'));
        assert!(chars.contains(&'试'));
        assert_eq!(chars.len(), 4);
    }

    #[test]
    fn test_used_characters_empty_initially() {
        let context = TextContext::new();
        assert!(context.get_used_characters().is_none());
    }

    #[test]
    fn test_used_characters_multiple_writes() {
        let mut context = TextContext::new();
        context.write("AB").unwrap();
        context.write("CD").unwrap();

        let chars = context.get_used_characters();
        assert!(chars.is_some());
        let chars = chars.unwrap();
        assert!(chars.contains(&'A'));
        assert!(chars.contains(&'B'));
        assert!(chars.contains(&'C'));
        assert!(chars.contains(&'D'));
        assert_eq!(chars.len(), 4);
    }
}
