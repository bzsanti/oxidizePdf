use crate::text::metrics::{measure_text_with, FontMetricsStore};
use crate::text::{split_into_words, Font};

/// Result of measuring a text block before rendering.
///
/// Used to calculate how much vertical space a block of wrapped text
/// will occupy, enabling layout decisions (page breaks, element positioning)
/// before committing to rendering.
#[derive(Debug, Clone, PartialEq)]
pub struct TextBlockMetrics {
    /// Width of the longest line in points
    pub width: f64,
    /// Total height of the block in points (font_size × line_height × line_count)
    pub height: f64,
    /// Number of lines after word-wrapping
    pub line_count: usize,
}

/// Computes wrapped line widths for a given text, font, size, and max width.
///
/// Returns a vector of line widths (one per wrapped line). This is the shared
/// wrapping algorithm used by both `measure_text_block` and `TextFlowContext`.
///
/// Each "word" from `split_into_words` is placed on the current line if it fits;
/// otherwise a new line is started. A single word wider than `max_width` is placed
/// on its own line (it will exceed `max_width` but avoids infinite loops).
///
/// Back-compat shim; delegates to `compute_line_widths_with(..., None)`.
#[inline]
pub fn compute_line_widths(text: &str, font: &Font, font_size: f64, max_width: f64) -> Vec<f64> {
    compute_line_widths_with(text, font, font_size, max_width, None)
}

/// Scope-aware variant of `compute_line_widths`. Consults `store` (if Some)
/// before the legacy global registry for `Font::Custom` lookups via the
/// underlying `measure_text_with`.
pub(crate) fn compute_line_widths_with(
    text: &str,
    font: &Font,
    font_size: f64,
    max_width: f64,
    store: Option<&FontMetricsStore>,
) -> Vec<f64> {
    if text.is_empty() {
        return Vec::new();
    }

    let words = split_into_words(text);
    if words.is_empty() {
        return Vec::new();
    }

    let mut line_widths: Vec<f64> = Vec::new();
    let mut current_width = 0.0;

    for word in &words {
        let word_width = measure_text_with(word, font, font_size, store);

        if current_width > 0.0 && current_width + word_width > max_width {
            line_widths.push(current_width);
            current_width = word_width;
        } else {
            current_width += word_width;
        }
    }

    if current_width > 0.0 {
        line_widths.push(current_width);
    }

    line_widths
}

/// Measures a block of word-wrapped text without rendering it.
///
/// Given a text string, font, font size, line height multiplier, and maximum
/// width, computes how the text would be laid out with word wrapping and returns
/// the resulting dimensions.
///
/// # Arguments
///
/// * `text` - The text to measure
/// * `font` - The font to use for width calculations
/// * `font_size` - Font size in points
/// * `line_height` - Line height multiplier (e.g., 1.2 for 120% spacing)
/// * `max_width` - Maximum width available for text in points
///
/// # Returns
///
/// A `TextBlockMetrics` with the measured `width`, `height`, and `line_count`.
///
/// # Example
///
/// ```rust
/// use oxidize_pdf::text::text_block::measure_text_block;
/// use oxidize_pdf::Font;
///
/// let metrics = measure_text_block("Hello World", &Font::Helvetica, 12.0, 1.2, 200.0);
/// assert_eq!(metrics.line_count, 1);
/// assert!(metrics.width > 0.0);
/// assert!(metrics.height > 0.0);
/// ```
///
/// Back-compat shim; delegates to `measure_text_block_with(..., None)`.
#[inline]
pub fn measure_text_block(
    text: &str,
    font: &Font,
    font_size: f64,
    line_height: f64,
    max_width: f64,
) -> TextBlockMetrics {
    measure_text_block_with(text, font, font_size, line_height, max_width, None)
}

/// Scope-aware variant of `measure_text_block`. Consults `store` (if Some)
/// before the legacy global registry for `Font::Custom` lookups via the
/// underlying `measure_text_with`.
pub fn measure_text_block_with(
    text: &str,
    font: &Font,
    font_size: f64,
    line_height: f64,
    max_width: f64,
    store: Option<&FontMetricsStore>,
) -> TextBlockMetrics {
    let line_widths = compute_line_widths_with(text, font, font_size, max_width, store);

    let line_count = line_widths.len();
    let width = line_widths.iter().copied().fold(0.0_f64, f64::max);
    let height = line_count as f64 * font_size * line_height;

    TextBlockMetrics {
        width,
        height,
        line_count,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_line_widths_empty() {
        let widths = compute_line_widths("", &Font::Helvetica, 12.0, 200.0);
        assert!(widths.is_empty());
    }

    #[test]
    fn test_compute_line_widths_single_word() {
        let widths = compute_line_widths("Hello", &Font::Helvetica, 12.0, 500.0);
        assert_eq!(widths.len(), 1);
        assert!(widths[0] > 0.0);
    }

    #[test]
    fn test_measure_text_block_empty() {
        let m = measure_text_block("", &Font::Helvetica, 12.0, 1.2, 300.0);
        assert_eq!(m.line_count, 0);
        assert_eq!(m.width, 0.0);
        assert_eq!(m.height, 0.0);
    }

    #[test]
    fn test_measure_text_block_with_uses_document_scope() {
        use crate::text::metrics::{FontMetrics, FontMetricsStore};
        let unique = format!("MeasureBlockTask5_{}", std::process::id());
        let store = FontMetricsStore::new();
        // Make every char width = 1000 (i.e., 1.0em per char). Word "AB" = 24 at 12pt.
        store.register(
            unique.clone(),
            FontMetrics::new(500).with_widths(&[('A', 1000), ('B', 1000)]),
        );

        let m = measure_text_block_with(
            "AB",
            &Font::Custom(unique.clone()),
            12.0,
            1.2,
            500.0,
            Some(&store),
        );
        // One line with width = 2 * 1000 / 1000 * 12 = 24
        assert!(
            (m.width - 24.0).abs() < 0.01,
            "expected scope-aware width 24, got {}",
            m.width
        );
    }

    #[test]
    fn test_measure_text_block_with_uses_store_across_wrap() {
        use crate::text::metrics::{FontMetrics, FontMetricsStore};
        let unique = format!("MeasureBlockWrapTask5_{}", std::process::id());
        let store = FontMetricsStore::new();
        // 'A' = 'B' = 1000 units → "AB" word width = 2000 units → 24.0 at 12pt.
        // Space ' ' = 1000 → 12.0 at 12pt.
        //
        // `split_into_words("AB AB")` yields three tokens: ["AB", " ", "AB"].
        // With max_width = 30.0 and per-store widths:
        //   token 1 "AB"  → current = 24.0               (fits)
        //   token 2 " "   → 24.0 + 12.0 = 36.0 > 30.0  → push 24.0, current = 12.0
        //   token 3 "AB"  → 12.0 + 24.0 = 36.0 > 30.0  → push 12.0, current = 24.0
        //   end            → push 24.0
        // → 3 lines: [24.0, 12.0, 24.0], width = 24.0.
        // This exercises all three iterations of the wrap loop, proving the store
        // is correctly threaded into compute_line_widths_with on every pass.
        store.register(
            unique.clone(),
            FontMetrics::new(500).with_widths(&[('A', 1000), ('B', 1000), (' ', 1000)]),
        );

        // max_width = 30.0 fits the first "AB" (24.0) but neither the space+AB
        // continuation nor the trailing "AB" fits, forcing two wraps (three lines).
        let m = measure_text_block_with(
            "AB AB",
            &Font::Custom(unique.clone()),
            12.0,
            1.2,
            30.0,
            Some(&store),
        );

        assert_eq!(
            m.line_count, 3,
            "split_into_words yields 3 tokens (\"AB\", \" \", \"AB\") and max_width=30 \
             forces a wrap after each; expected 3 lines"
        );
        // Block width is the max of [24.0, 12.0, 24.0] = 24.0, derived from
        // per-store widths. If the store were not threaded the widths would differ.
        assert!(
            (m.width - 24.0).abs() < 0.01,
            "wrapped block width must come from per-store widths (24.0); got {}",
            m.width
        );
    }
}
