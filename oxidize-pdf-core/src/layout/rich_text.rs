use crate::text::{measure_text, Font};
use crate::Color;
use std::collections::{HashMap, HashSet};
use std::fmt::Write;

/// A styled text segment with its own font, size, and color.
#[derive(Debug, Clone)]
pub struct TextSpan {
    pub text: String,
    pub font: Font,
    pub font_size: f64,
    pub color: Color,
}

impl TextSpan {
    /// Create a new text span.
    pub fn new(text: &str, font: Font, font_size: f64, color: Color) -> Self {
        Self {
            text: text.to_string(),
            font,
            font_size,
            color,
        }
    }

    /// Measure the width of this span in points.
    pub fn measure_width(&self) -> f64 {
        measure_text(&self.text, &self.font, self.font_size)
    }
}

/// A line of mixed-style text composed of multiple [`TextSpan`]s.
///
/// Each span can have a different font, size, and color. The entire
/// RichText renders as a single line (no word-wrapping).
///
/// # Example
///
/// ```rust
/// use oxidize_pdf::layout::{RichText, TextSpan};
/// use oxidize_pdf::{Color, Font};
///
/// let rich = RichText::new(vec![
///     TextSpan::new("Total: ", Font::HelveticaBold, 14.0, Color::black()),
///     TextSpan::new("$1,234.56", Font::Helvetica, 14.0, Color::gray(0.3)),
/// ]);
/// assert_eq!(rich.spans().len(), 2);
/// assert!(rich.total_width() > 0.0);
/// ```
#[derive(Debug)]
pub struct RichText {
    spans: Vec<TextSpan>,
}

impl RichText {
    /// Create a RichText from a list of spans.
    pub fn new(spans: Vec<TextSpan>) -> Self {
        Self { spans }
    }

    /// Total width of all spans combined.
    pub fn total_width(&self) -> f64 {
        self.spans.iter().map(|s| s.measure_width()).sum()
    }

    /// Maximum font size across all spans (determines line height).
    pub fn max_font_size(&self) -> f64 {
        self.spans
            .iter()
            .map(|s| s.font_size)
            .fold(0.0_f64, f64::max)
    }

    /// Access the spans.
    pub fn spans(&self) -> &[TextSpan] {
        &self.spans
    }

    /// Generate PDF operators to render this rich text at position (x, y).
    ///
    /// Produces a single BT/ET block with per-span font/color/text changes.
    /// Render this rich-text block to a content-stream fragment plus a
    /// per-font character usage map (issue #204).
    ///
    /// The caller is responsible for splicing `ops` into the target
    /// page's content stream and reporting `font_usage` via
    /// [`crate::Page::append_raw_content`] — both go together so the
    /// writer knows which fonts this fragment referenced and what
    /// characters it drew with each. Returning the usage map is the
    /// type-gated replacement for scattering `record_used_chars` calls
    /// through every content builder; future builders cannot forget
    /// tracking because `append_raw_content` won't compile without it.
    pub(crate) fn render_operations(
        &self,
        x: f64,
        y: f64,
    ) -> (String, HashMap<String, HashSet<char>>) {
        let mut font_usage: HashMap<String, HashSet<char>> = HashMap::new();
        if self.spans.is_empty() {
            return (String::new(), font_usage);
        }

        let mut ops = String::new();
        ops.push_str("BT\n");
        writeln!(&mut ops, "{x:.2} {y:.2} Td").expect("write to String");

        for span in &self.spans {
            // Set color
            match span.color {
                Color::Rgb(r, g, b) => {
                    writeln!(&mut ops, "{r:.3} {g:.3} {b:.3} rg").expect("write to String");
                }
                Color::Gray(gray) => {
                    writeln!(&mut ops, "{gray:.3} g").expect("write to String");
                }
                Color::Cmyk(c, m, y, k) => {
                    writeln!(&mut ops, "{c:.3} {m:.3} {y:.3} {k:.3} k").expect("write to String");
                }
            }

            // Set font
            let font_name = span.font.pdf_name();
            writeln!(&mut ops, "/{} {:.2} Tf", font_name, span.font_size).expect("write to String");

            // Show text with escaping
            ops.push('(');
            for ch in span.text.chars() {
                match ch {
                    '(' => ops.push_str("\\("),
                    ')' => ops.push_str("\\)"),
                    '\\' => ops.push_str("\\\\"),
                    '\n' => ops.push_str("\\n"),
                    '\r' => ops.push_str("\\r"),
                    '\t' => ops.push_str("\\t"),
                    _ => ops.push(ch),
                }
            }
            ops.push_str(") Tj\n");

            // Report the characters drawn with this span's font so the
            // writer can subset the font accurately (issue #204).
            font_usage
                .entry(font_name)
                .or_default()
                .extend(span.text.chars());
        }

        ops.push_str("ET\n");
        (ops, font_usage)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_rich_text() {
        let rt = RichText::new(vec![]);
        assert_eq!(rt.total_width(), 0.0);
        assert_eq!(rt.max_font_size(), 0.0);
        let (ops, font_usage) = rt.render_operations(0.0, 0.0);
        assert!(ops.is_empty());
        assert!(font_usage.is_empty(), "no spans → no font usage reported");
    }

    #[test]
    fn test_render_operations_contains_bt_et() {
        let rt = RichText::new(vec![TextSpan::new(
            "Hello",
            Font::Helvetica,
            12.0,
            Color::black(),
        )]);
        let (ops, font_usage) = rt.render_operations(50.0, 700.0);
        assert!(ops.starts_with("BT\n"));
        assert!(ops.ends_with("ET\n"));
        assert!(ops.contains("(Hello) Tj"));
        assert!(ops.contains("/Helvetica 12.00 Tf"));

        // PR for issue #204: render_operations must also report per-font
        // char usage so the caller can feed it into the page tracker
        // via `Page::append_raw_content`.
        let chars = font_usage
            .get("Helvetica")
            .expect("Helvetica span must produce a bucket");
        assert!(chars.contains(&'H'));
        assert!(chars.contains(&'e'));
        assert!(chars.contains(&'l'));
        assert!(chars.contains(&'o'));
    }
}
