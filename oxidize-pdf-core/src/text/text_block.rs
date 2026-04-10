use crate::text::{measure_text, split_into_words, Font};

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
pub fn compute_line_widths(text: &str, font: &Font, font_size: f64, max_width: f64) -> Vec<f64> {
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
        let word_width = measure_text(word, font, font_size);

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
pub fn measure_text_block(
    text: &str,
    font: &Font,
    font_size: f64,
    line_height: f64,
    max_width: f64,
) -> TextBlockMetrics {
    let line_widths = compute_line_widths(text, font, font_size, max_width);

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
}
