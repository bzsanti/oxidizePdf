use crate::text::Font;
use lazy_static::lazy_static;
use std::collections::HashMap;
use std::sync::RwLock;

/// Character width information for standard PDF fonts
/// All widths are in 1/1000 of a unit (font size 1.0)
#[derive(Clone)]
pub struct FontMetrics {
    widths: HashMap<char, u16>,
    default_width: u16,
}

impl FontMetrics {
    fn new(default_width: u16) -> Self {
        Self {
            widths: HashMap::new(),
            default_width,
        }
    }

    fn with_widths(mut self, widths: &[(char, u16)]) -> Self {
        for &(ch, width) in widths {
            self.widths.insert(ch, width);
        }
        self
    }

    pub fn char_width(&self, ch: char) -> u16 {
        self.widths.get(&ch).copied().unwrap_or(self.default_width)
    }
}

// Dynamic registry for custom font metrics
lazy_static! {
    static ref CUSTOM_FONT_METRICS: RwLock<HashMap<String, FontMetrics>> =
        RwLock::new(HashMap::new());
}

lazy_static::lazy_static! {
    static ref FONT_METRICS: HashMap<Font, FontMetrics> = {
        let mut metrics = HashMap::new();

        // Helvetica
        metrics.insert(Font::Helvetica, FontMetrics::new(556).with_widths(&[
            (' ', 278), ('!', 278), ('"', 355), ('#', 556), ('$', 556), ('%', 889),
            ('&', 667), ('\'', 191), ('(', 333), (')', 333), ('*', 389), ('+', 584),
            (',', 278), ('-', 333), ('.', 278), ('/', 278), ('0', 556), ('1', 556),
            ('2', 556), ('3', 556), ('4', 556), ('5', 556), ('6', 556), ('7', 556),
            ('8', 556), ('9', 556), (':', 278), (';', 278), ('<', 584), ('=', 584),
            ('>', 584), ('?', 556), ('@', 1015), ('A', 667), ('B', 667), ('C', 722),
            ('D', 722), ('E', 667), ('F', 611), ('G', 778), ('H', 722), ('I', 278),
            ('J', 500), ('K', 667), ('L', 556), ('M', 833), ('N', 722), ('O', 778),
            ('P', 667), ('Q', 778), ('R', 722), ('S', 667), ('T', 611), ('U', 722),
            ('V', 667), ('W', 944), ('X', 667), ('Y', 667), ('Z', 611), ('[', 278),
            ('\\', 278), (']', 278), ('^', 469), ('_', 556), ('`', 333), ('a', 556),
            ('b', 556), ('c', 500), ('d', 556), ('e', 556), ('f', 278), ('g', 556),
            ('h', 556), ('i', 222), ('j', 222), ('k', 500), ('l', 222), ('m', 833),
            ('n', 556), ('o', 556), ('p', 556), ('q', 556), ('r', 333), ('s', 500),
            ('t', 278), ('u', 556), ('v', 500), ('w', 722), ('x', 500), ('y', 500),
            ('z', 500), ('{', 334), ('|', 260), ('}', 334), ('~', 584),
        ]));

        // Helvetica Bold
        metrics.insert(Font::HelveticaBold, FontMetrics::new(611).with_widths(&[
            (' ', 278), ('!', 333), ('"', 474), ('#', 556), ('$', 556), ('%', 889),
            ('&', 722), ('\'', 238), ('(', 333), (')', 333), ('*', 389), ('+', 584),
            (',', 278), ('-', 333), ('.', 278), ('/', 278), ('0', 556), ('1', 556),
            ('2', 556), ('3', 556), ('4', 556), ('5', 556), ('6', 556), ('7', 556),
            ('8', 556), ('9', 556), (':', 333), (';', 333), ('<', 584), ('=', 584),
            ('>', 584), ('?', 611), ('@', 975), ('A', 722), ('B', 722), ('C', 722),
            ('D', 722), ('E', 667), ('F', 611), ('G', 778), ('H', 722), ('I', 278),
            ('J', 556), ('K', 722), ('L', 611), ('M', 833), ('N', 722), ('O', 778),
            ('P', 667), ('Q', 778), ('R', 722), ('S', 667), ('T', 611), ('U', 722),
            ('V', 667), ('W', 944), ('X', 667), ('Y', 667), ('Z', 611), ('[', 333),
            ('\\', 278), (']', 333), ('^', 584), ('_', 556), ('`', 333), ('a', 556),
            ('b', 611), ('c', 556), ('d', 611), ('e', 556), ('f', 333), ('g', 611),
            ('h', 611), ('i', 278), ('j', 278), ('k', 556), ('l', 278), ('m', 889),
            ('n', 611), ('o', 611), ('p', 611), ('q', 611), ('r', 389), ('s', 556),
            ('t', 333), ('u', 611), ('v', 556), ('w', 778), ('x', 556), ('y', 556),
            ('z', 500), ('{', 389), ('|', 280), ('}', 389), ('~', 584),
        ]));

        // Times Roman
        metrics.insert(Font::TimesRoman, FontMetrics::new(500).with_widths(&[
            (' ', 250), ('!', 333), ('"', 408), ('#', 500), ('$', 500), ('%', 833),
            ('&', 778), ('\'', 180), ('(', 333), (')', 333), ('*', 500), ('+', 564),
            (',', 250), ('-', 333), ('.', 250), ('/', 278), ('0', 500), ('1', 500),
            ('2', 500), ('3', 500), ('4', 500), ('5', 500), ('6', 500), ('7', 500),
            ('8', 500), ('9', 500), (':', 278), (';', 278), ('<', 564), ('=', 564),
            ('>', 564), ('?', 444), ('@', 921), ('A', 722), ('B', 667), ('C', 667),
            ('D', 722), ('E', 611), ('F', 556), ('G', 722), ('H', 722), ('I', 333),
            ('J', 389), ('K', 722), ('L', 611), ('M', 889), ('N', 722), ('O', 722),
            ('P', 556), ('Q', 722), ('R', 667), ('S', 556), ('T', 611), ('U', 722),
            ('V', 722), ('W', 944), ('X', 722), ('Y', 722), ('Z', 611), ('[', 333),
            ('\\', 278), (']', 333), ('^', 469), ('_', 500), ('`', 333), ('a', 444),
            ('b', 500), ('c', 444), ('d', 500), ('e', 444), ('f', 333), ('g', 500),
            ('h', 500), ('i', 278), ('j', 278), ('k', 500), ('l', 278), ('m', 778),
            ('n', 500), ('o', 500), ('p', 500), ('q', 500), ('r', 333), ('s', 389),
            ('t', 278), ('u', 500), ('v', 500), ('w', 722), ('x', 500), ('y', 500),
            ('z', 444), ('{', 480), ('|', 200), ('}', 480), ('~', 541),
        ]));

        // Courier (all characters have the same width)
        metrics.insert(Font::Courier, FontMetrics::new(600).with_widths(&[
            (' ', 600), ('!', 600), ('"', 600), ('#', 600), ('$', 600), ('%', 600),
            ('&', 600), ('\'', 600), ('(', 600), (')', 600), ('*', 600), ('+', 600),
            (',', 600), ('-', 600), ('.', 600), ('/', 600), ('0', 600), ('1', 600),
            ('2', 600), ('3', 600), ('4', 600), ('5', 600), ('6', 600), ('7', 600),
            ('8', 600), ('9', 600), (':', 600), (';', 600), ('<', 600), ('=', 600),
            ('>', 600), ('?', 600), ('@', 600), ('A', 600), ('B', 600), ('C', 600),
            ('D', 600), ('E', 600), ('F', 600), ('G', 600), ('H', 600), ('I', 600),
            ('J', 600), ('K', 600), ('L', 600), ('M', 600), ('N', 600), ('O', 600),
            ('P', 600), ('Q', 600), ('R', 600), ('S', 600), ('T', 600), ('U', 600),
            ('V', 600), ('W', 600), ('X', 600), ('Y', 600), ('Z', 600), ('[', 600),
            ('\\', 600), (']', 600), ('^', 600), ('_', 600), ('`', 600), ('a', 600),
            ('b', 600), ('c', 600), ('d', 600), ('e', 600), ('f', 600), ('g', 600),
            ('h', 600), ('i', 600), ('j', 600), ('k', 600), ('l', 600), ('m', 600),
            ('n', 600), ('o', 600), ('p', 600), ('q', 600), ('r', 600), ('s', 600),
            ('t', 600), ('u', 600), ('v', 600), ('w', 600), ('x', 600), ('y', 600),
            ('z', 600), ('{', 600), ('|', 600), ('}', 600), ('~', 600),
        ]));

        // For now, use the same metrics for variations
        metrics.insert(Font::HelveticaOblique, metrics[&Font::Helvetica].clone());
        metrics.insert(Font::HelveticaBoldOblique, metrics[&Font::HelveticaBold].clone());
        metrics.insert(Font::TimesBold, metrics[&Font::TimesRoman].clone());
        metrics.insert(Font::TimesItalic, metrics[&Font::TimesRoman].clone());
        metrics.insert(Font::TimesBoldItalic, metrics[&Font::TimesRoman].clone());
        metrics.insert(Font::CourierBold, metrics[&Font::Courier].clone());
        metrics.insert(Font::CourierOblique, metrics[&Font::Courier].clone());
        metrics.insert(Font::CourierBoldOblique, metrics[&Font::Courier].clone());

        metrics
    };
}

impl FontMetrics {
    fn clone(&self) -> Self {
        Self {
            widths: self.widths.clone(),
            default_width: self.default_width,
        }
    }
}

/// Measure the width of a text string in a given font and size
pub fn measure_text(text: &str, font: Font, font_size: f64) -> f64 {
    if font.is_symbolic() {
        // Symbol and ZapfDingbats need special handling
        return text.len() as f64 * font_size * 0.6;
    }

    let metrics = get_font_metrics(&font);

    let width_units: u32 = text.chars().map(|ch| metrics.char_width(ch) as u32).sum();

    (width_units as f64 / 1000.0) * font_size
}

/// Measure the width of a single character
pub fn measure_char(ch: char, font: Font, font_size: f64) -> f64 {
    if font.is_symbolic() {
        return font_size * 0.6;
    }

    let metrics = get_font_metrics(&font);

    (metrics.char_width(ch) as f64 / 1000.0) * font_size
}

/// Split text into words, preserving spaces
pub fn split_into_words(text: &str) -> Vec<&str> {
    let mut words = Vec::new();
    let mut start = 0;
    let mut in_space = false;

    for (i, ch) in text.char_indices() {
        if ch.is_whitespace() {
            if !in_space {
                if i > start {
                    words.push(&text[start..i]);
                }
                start = i;
                in_space = true;
            }
        } else if in_space {
            if i > start {
                words.push(&text[start..i]);
            }
            start = i;
            in_space = false;
        }
    }

    if start < text.len() {
        words.push(&text[start..]);
    }

    words
}

/// Register metrics for a custom font
pub fn register_custom_font_metrics(font_name: String, metrics: FontMetrics) {
    if let Ok(mut custom_metrics) = CUSTOM_FONT_METRICS.write() {
        custom_metrics.insert(font_name, metrics);
    }
}

/// Get metrics for a custom font
pub fn get_custom_font_metrics(font_name: &str) -> Option<FontMetrics> {
    if let Ok(custom_metrics) = CUSTOM_FONT_METRICS.read() {
        custom_metrics.get(font_name).cloned()
    } else {
        None
    }
}

/// Get font metrics for any font (standard or custom)
fn get_font_metrics(font: &Font) -> FontMetrics {
    match font {
        Font::Custom(font_name) => {
            // Try to get custom metrics first
            if let Some(custom_metrics) = get_custom_font_metrics(font_name) {
                custom_metrics
            } else {
                // Register default metrics for unknown custom fonts
                let default_metrics = create_default_custom_metrics();
                register_custom_font_metrics(font_name.clone(), default_metrics.clone());
                tracing::debug!(
                    "Warning: Using default metrics for unknown custom font: {}",
                    font_name
                );
                default_metrics
            }
        }
        _ => {
            // Standard fonts
            FONT_METRICS.get(font).cloned().unwrap_or_else(|| {
                tracing::debug!(
                    "Warning: Standard font metrics not found for {:?}, using default",
                    font
                );
                create_default_custom_metrics()
            })
        }
    }
}

/// Create default metrics for a custom font (fallback when no specific metrics available)
pub fn create_default_custom_metrics() -> FontMetrics {
    // Use Helvetica-like metrics as a reasonable default for unknown custom fonts
    // This prevents panics while providing functional text measurement
    FontMetrics::new(556).with_widths(&[
        (' ', 278),
        ('!', 278),
        ('"', 355),
        ('#', 556),
        ('$', 556),
        ('%', 889),
        ('&', 667),
        ('\'', 191),
        ('(', 333),
        (')', 333),
        ('*', 389),
        ('+', 584),
        (',', 278),
        ('-', 333),
        ('.', 278),
        ('/', 278),
        ('0', 556),
        ('1', 556),
        ('2', 556),
        ('3', 556),
        ('4', 556),
        ('5', 556),
        ('6', 556),
        ('7', 556),
        ('8', 556),
        ('9', 556),
        (':', 278),
        (';', 278),
        ('<', 584),
        ('=', 584),
        ('>', 584),
        ('?', 556),
        ('@', 1015),
        ('A', 667),
        ('B', 667),
        ('C', 722),
        ('D', 722),
        ('E', 667),
        ('F', 611),
        ('G', 778),
        ('H', 722),
        ('I', 278),
        ('J', 500),
        ('K', 667),
        ('L', 556),
        ('M', 833),
        ('N', 722),
        ('O', 778),
        ('P', 667),
        ('Q', 778),
        ('R', 722),
        ('S', 667),
        ('T', 611),
        ('U', 722),
        ('V', 667),
        ('W', 944),
        ('X', 667),
        ('Y', 667),
        ('Z', 611),
        ('[', 278),
        ('\\', 278),
        (']', 278),
        ('^', 469),
        ('_', 556),
        ('`', 333),
        ('a', 556),
        ('b', 556),
        ('c', 500),
        ('d', 556),
        ('e', 556),
        ('f', 278),
        ('g', 556),
        ('h', 556),
        ('i', 222),
        ('j', 222),
        ('k', 500),
        ('l', 222),
        ('m', 833),
        ('n', 556),
        ('o', 556),
        ('p', 556),
        ('q', 556),
        ('r', 333),
        ('s', 500),
        ('t', 278),
        ('u', 556),
        ('v', 500),
        ('w', 722),
        ('x', 500),
        ('y', 500),
        ('z', 500),
        ('{', 334),
        ('|', 260),
        ('}', 334),
        ('~', 584),
        // Add basic CJK character support with reasonable estimates
        ('你', 1000),
        ('好', 1000),
        ('世', 1000),
        ('界', 1000),
        ('中', 1000),
        ('文', 1000),
        ('测', 1000),
        ('试', 1000),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_font_metrics_creation() {
        let metrics = FontMetrics::new(500);
        assert_eq!(metrics.default_width, 500);
        assert!(metrics.widths.is_empty());
    }

    #[test]
    fn test_font_metrics_with_widths() {
        let widths = [('A', 600), ('B', 700), ('C', 650)];
        let metrics = FontMetrics::new(500).with_widths(&widths);

        assert_eq!(metrics.char_width('A'), 600);
        assert_eq!(metrics.char_width('B'), 700);
        assert_eq!(metrics.char_width('C'), 650);
        assert_eq!(metrics.char_width('Z'), 500); // Default for unmapped
    }

    #[test]
    fn test_font_metrics_clone() {
        let widths = [('A', 600), ('B', 700)];
        let metrics1 = FontMetrics::new(500).with_widths(&widths);
        let metrics2 = metrics1.clone();

        assert_eq!(metrics1.char_width('A'), metrics2.char_width('A'));
        assert_eq!(metrics1.default_width, metrics2.default_width);
    }

    #[test]
    fn test_measure_text_helvetica() {
        let text = "Hello";
        let width = measure_text(text, Font::Helvetica, 12.0);

        // Helvetica "H" = 722, "e" = 556, "l" = 222, "l" = 222, "o" = 556
        // Total = 2278 units = 2.278 at size 1.0, * 12.0 = 27.336
        assert!((width - 27.336).abs() < 0.01);
    }

    #[test]
    fn test_measure_text_courier() {
        let text = "ABC";
        let width = measure_text(text, Font::Courier, 10.0);

        // Courier is monospace: all chars = 600 units
        // 3 chars * 600 = 1800 units = 1.8 at size 1.0, * 10.0 = 18.0
        assert_eq!(width, 18.0);
    }

    #[test]
    fn test_measure_text_symbolic_fonts() {
        let text = "ABC";
        let symbol_width = measure_text(text, Font::Symbol, 12.0);
        let zapf_width = measure_text(text, Font::ZapfDingbats, 12.0);

        // Symbolic fonts use approximation: len * font_size * 0.6
        let expected = 3.0 * 12.0 * 0.6; // = 21.6
        assert_eq!(symbol_width, expected);
        assert_eq!(zapf_width, expected);
    }

    #[test]
    fn test_measure_char_helvetica() {
        let width = measure_char('A', Font::Helvetica, 12.0);

        // Helvetica "A" = 667 units = 0.667 at size 1.0, * 12.0 = 8.004
        assert!((width - 8.004).abs() < 0.01);
    }

    #[test]
    fn test_measure_char_courier() {
        let width = measure_char('X', Font::Courier, 10.0);

        // Courier "X" = 600 units = 0.6 at size 1.0, * 10.0 = 6.0
        assert_eq!(width, 6.0);
    }

    #[test]
    fn test_measure_char_symbolic() {
        let symbol_width = measure_char('A', Font::Symbol, 15.0);
        let zapf_width = measure_char('B', Font::ZapfDingbats, 15.0);

        // Symbolic fonts: font_size * 0.6
        let expected = 15.0 * 0.6; // = 9.0
        assert_eq!(symbol_width, expected);
        assert_eq!(zapf_width, expected);
    }

    #[test]
    fn test_split_into_words_simple() {
        let text = "Hello World";
        let words = split_into_words(text);

        assert_eq!(words, vec!["Hello", " ", "World"]);
    }

    #[test]
    fn test_split_into_words_multiple_spaces() {
        let text = "Hello   World";
        let words = split_into_words(text);

        assert_eq!(words, vec!["Hello", "   ", "World"]);
    }

    #[test]
    fn test_split_into_words_leading_trailing_spaces() {
        let text = " Hello World ";
        let words = split_into_words(text);

        assert_eq!(words, vec![" ", "Hello", " ", "World", " "]);
    }

    #[test]
    fn test_split_into_words_tabs_newlines() {
        let text = "Hello\tWorld\nTest";
        let words = split_into_words(text);

        assert_eq!(words, vec!["Hello", "\t", "World", "\n", "Test"]);
    }

    #[test]
    fn test_split_into_words_empty() {
        let text = "";
        let words = split_into_words(text);

        assert!(words.is_empty());
    }

    #[test]
    fn test_split_into_words_only_spaces() {
        let text = "   ";
        let words = split_into_words(text);

        assert_eq!(words, vec!["   "]);
    }

    #[test]
    fn test_split_into_words_single_word() {
        let text = "Hello";
        let words = split_into_words(text);

        assert_eq!(words, vec!["Hello"]);
    }

    #[test]
    fn test_all_font_metrics_exist() {
        let fonts = [
            Font::Helvetica,
            Font::HelveticaBold,
            Font::HelveticaOblique,
            Font::HelveticaBoldOblique,
            Font::TimesRoman,
            Font::TimesBold,
            Font::TimesItalic,
            Font::TimesBoldItalic,
            Font::Courier,
            Font::CourierBold,
            Font::CourierOblique,
            Font::CourierBoldOblique,
        ];

        for font in &fonts {
            // Should not panic - all fonts should have metrics
            let _width = measure_text("A", font.clone(), 12.0);
        }
    }

    #[test]
    fn test_helvetica_specific_characters() {
        let chars = [
            (' ', 278),
            ('A', 667),
            ('B', 667),
            ('C', 722),
            ('a', 556),
            ('b', 556),
            ('0', 556),
            ('1', 556),
            ('@', 1015),
            ('M', 833),
            ('W', 944),
            ('i', 222),
        ];

        for (ch, expected_width) in &chars {
            let width = measure_char(*ch, Font::Helvetica, 1000.0);
            let expected = *expected_width as f64;
            assert!(
                (width - expected).abs() < 0.1,
                "Character '{ch}' width mismatch: {width} vs {expected}"
            );
        }
    }

    #[test]
    fn test_times_specific_characters() {
        let chars = [
            (' ', 250),
            ('A', 722),
            ('B', 667),
            ('C', 667),
            ('a', 444),
            ('b', 500),
            ('0', 500),
            ('1', 500),
            ('@', 921),
            ('M', 889),
            ('W', 944),
            ('i', 278),
        ];

        for (ch, expected_width) in &chars {
            let width = measure_char(*ch, Font::TimesRoman, 1000.0);
            let expected = *expected_width as f64;
            assert_eq!(width, expected, "Character '{ch}' width mismatch");
        }
    }

    #[test]
    fn test_courier_monospace_property() {
        let chars = [
            ' ', 'A', 'B', 'C', 'a', 'b', '0', '1', '@', 'M', 'W', 'i', '~',
        ];

        for ch in &chars {
            let width = measure_char(*ch, Font::Courier, 1000.0);
            assert_eq!(width, 600.0, "Courier character '{ch}' should be 600 units");
        }
    }

    #[test]
    fn test_font_size_scaling() {
        let sizes = [6.0, 12.0, 18.0, 24.0, 36.0];

        for size in &sizes {
            let width = measure_char('A', Font::Helvetica, *size);
            let expected = 667.0 * size / 1000.0; // Helvetica 'A' = 667 units
            assert!(
                (width - expected).abs() < 0.01,
                "Size {size} scaling incorrect"
            );
        }
    }

    #[test]
    fn test_measure_text_empty_string() {
        let width = measure_text("", Font::Helvetica, 12.0);
        assert_eq!(width, 0.0);
    }

    #[test]
    fn test_measure_text_consistency() {
        let text = "Hello";

        // Measuring whole text should equal sum of individual characters
        let total_width = measure_text(text, Font::Helvetica, 12.0);
        let individual_sum: f64 = text
            .chars()
            .map(|ch| measure_char(ch, Font::Helvetica, 12.0))
            .sum();

        assert!((total_width - individual_sum).abs() < 0.01);
    }

    #[test]
    fn test_font_variants_use_base_metrics() {
        // Test that font variations use the base font metrics
        let base_width = measure_char('A', Font::Helvetica, 12.0);
        let oblique_width = measure_char('A', Font::HelveticaOblique, 12.0);
        let bold_oblique_width = measure_char('A', Font::HelveticaBoldOblique, 12.0);

        // Should use same metrics (though in reality, they'd be different)
        assert_eq!(base_width, oblique_width);

        let bold_width = measure_char('A', Font::HelveticaBold, 12.0);
        assert_eq!(bold_width, bold_oblique_width);
    }

    #[test]
    fn test_unicode_characters_default_width() {
        // Test characters not in the metrics tables
        let unicode_chars = ['β', 'π', '€', '™'];

        for ch in &unicode_chars {
            let helvetica_width = measure_char(*ch, Font::Helvetica, 12.0);
            let times_width = measure_char(*ch, Font::TimesRoman, 12.0);
            let courier_width = measure_char(*ch, Font::Courier, 12.0);

            // Should use default widths
            let helvetica_expected = 556.0 * 12.0 / 1000.0;
            let times_expected = 500.0 * 12.0 / 1000.0;
            let courier_expected = 600.0 * 12.0 / 1000.0;

            assert!(
                (helvetica_width - helvetica_expected).abs() < 0.01,
                "Helvetica width mismatch"
            );
            assert!(
                (times_width - times_expected).abs() < 0.01,
                "Times width mismatch"
            );
            assert!(
                (courier_width - courier_expected).abs() < 0.01,
                "Courier width mismatch"
            );
        }
    }

    #[test]
    fn test_register_custom_font_metrics() {
        let metrics = FontMetrics::new(750).with_widths(&[('A', 800), ('B', 850)]);
        register_custom_font_metrics("TestFont".to_string(), metrics);

        let retrieved = get_custom_font_metrics("TestFont");
        assert!(retrieved.is_some());

        let retrieved = retrieved.unwrap();
        assert_eq!(retrieved.char_width('A'), 800);
        assert_eq!(retrieved.char_width('B'), 850);
        assert_eq!(retrieved.char_width('Z'), 750); // default
    }

    #[test]
    fn test_get_custom_font_metrics_not_found() {
        let result = get_custom_font_metrics("NonExistentFont12345");
        // May or may not be found depending on previous tests
        // Just verify no panic
        let _ = result;
    }

    #[test]
    fn test_measure_text_custom_font() {
        // Register a custom font with known metrics
        let metrics = FontMetrics::new(500).with_widths(&[('A', 600), ('B', 600), ('C', 600)]);
        register_custom_font_metrics("MyCustomFont".to_string(), metrics);

        let width = measure_text("ABC", Font::Custom("MyCustomFont".to_string()), 10.0);

        // 3 chars * 600 units = 1800 units = 1.8 at size 1.0 * 10.0 = 18.0
        assert!((width - 18.0).abs() < 0.01);
    }

    #[test]
    fn test_measure_char_custom_font() {
        let metrics = FontMetrics::new(500).with_widths(&[('X', 700)]);
        register_custom_font_metrics("CustomCharTest".to_string(), metrics);

        let width = measure_char('X', Font::Custom("CustomCharTest".to_string()), 10.0);

        // 700 units / 1000 * 10.0 = 7.0
        assert!((width - 7.0).abs() < 0.01);
    }

    #[test]
    fn test_custom_font_auto_register_default() {
        // When using an unregistered custom font, default metrics should be registered
        let width = measure_char('A', Font::Custom("AutoRegisterTest".to_string()), 10.0);

        // Should use default metrics (Helvetica-like), A = 667
        let expected = 667.0 * 10.0 / 1000.0;
        assert!((width - expected).abs() < 0.01);

        // Now it should be registered
        let metrics = get_custom_font_metrics("AutoRegisterTest");
        assert!(metrics.is_some());
    }

    #[test]
    fn test_create_default_custom_metrics() {
        let metrics = create_default_custom_metrics();

        // Test some expected values
        assert_eq!(metrics.char_width('A'), 667);
        assert_eq!(metrics.char_width(' '), 278);
        assert_eq!(metrics.char_width('0'), 556);
        assert_eq!(metrics.char_width('你'), 1000); // CJK
        assert_eq!(metrics.default_width, 556);
    }

    #[test]
    fn test_times_roman_metrics() {
        let width = measure_char('A', Font::TimesRoman, 10.0);
        // Times Roman 'A' = 722 units
        let expected = 722.0 * 10.0 / 1000.0;
        assert!((width - expected).abs() < 0.01);
    }

    #[test]
    fn test_helvetica_bold_metrics() {
        let width = measure_char('A', Font::HelveticaBold, 10.0);
        // Helvetica Bold 'A' = 722 units
        let expected = 722.0 * 10.0 / 1000.0;
        assert!((width - expected).abs() < 0.01);
    }

    #[test]
    fn test_times_bold_uses_base_metrics() {
        let base_width = measure_char('A', Font::TimesRoman, 12.0);
        let bold_width = measure_char('A', Font::TimesBold, 12.0);
        let italic_width = measure_char('A', Font::TimesItalic, 12.0);
        let bold_italic_width = measure_char('A', Font::TimesBoldItalic, 12.0);

        // All Times variants use base Times Roman metrics
        assert_eq!(base_width, bold_width);
        assert_eq!(base_width, italic_width);
        assert_eq!(base_width, bold_italic_width);
    }

    #[test]
    fn test_courier_variants_use_base_metrics() {
        let base_width = measure_char('X', Font::Courier, 12.0);
        let bold_width = measure_char('X', Font::CourierBold, 12.0);
        let oblique_width = measure_char('X', Font::CourierOblique, 12.0);
        let bold_oblique_width = measure_char('X', Font::CourierBoldOblique, 12.0);

        // All Courier variants use base Courier metrics
        assert_eq!(base_width, bold_width);
        assert_eq!(base_width, oblique_width);
        assert_eq!(base_width, bold_oblique_width);
    }

    #[test]
    fn test_split_into_words_mixed_whitespace() {
        let words = split_into_words("A B  C   D");
        assert_eq!(words, vec!["A", " ", "B", "  ", "C", "   ", "D"]);
    }
}
