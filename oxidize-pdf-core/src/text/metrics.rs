use crate::text::Font;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// Character width information for standard PDF fonts
/// All widths are in 1/1000 of a unit (font size 1.0)
#[derive(Clone, Debug)]
pub struct FontMetrics {
    widths: HashMap<char, u16>,
    default_width: u16,
}

impl FontMetrics {
    pub fn new(default_width: u16) -> Self {
        Self {
            widths: HashMap::new(),
            default_width,
        }
    }

    pub fn with_widths(mut self, widths: &[(char, u16)]) -> Self {
        for &(ch, width) in widths {
            self.widths.insert(ch, width);
        }
        self
    }

    /// Create metrics from a pre-built character width map
    pub fn from_char_map(widths: HashMap<char, u16>, default_width: u16) -> Self {
        Self {
            widths,
            default_width,
        }
    }

    pub fn char_width(&self, ch: char) -> u16 {
        self.widths.get(&ch).copied().unwrap_or(self.default_width)
    }
}

/// Per-Document store of custom font metrics.
///
/// Cheap to clone (Arc-backed). The lifetime of registered metrics is bound
/// to the lifetime of the owning Document — when the Document is dropped,
/// the metrics are freed (assuming no other Arc clones survive).
///
/// This type was introduced in v2.8.0 to replace the process-wide
/// `CUSTOM_FONT_METRICS` lazy_static registry, which leaked across
/// Document lifetimes (issue #230).
#[derive(Clone, Debug)]
pub struct FontMetricsStore {
    inner: Arc<RwLock<HashMap<String, Arc<FontMetrics>>>>,
}

impl FontMetricsStore {
    /// Create a new empty store.
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register or replace metrics for `font_name`. Last-writer-wins on the
    /// same name. Concurrent calls into the same store are serialised by the
    /// internal RwLock; concurrent calls into the same Document are
    /// prevented by `Document::add_font_from_bytes` taking `&mut self`.
    pub fn register(&self, font_name: impl Into<String>, metrics: FontMetrics) {
        let name = font_name.into();
        match self.inner.write() {
            Ok(mut map) => {
                map.insert(name, Arc::new(metrics));
            }
            Err(e) => {
                tracing::warn!(
                    "FontMetricsStore lock is poisoned; could not register '{}': {}",
                    name,
                    e
                );
            }
        }
    }

    /// Look up metrics by name. Returns `None` on miss; no side effects.
    pub fn get(&self, font_name: &str) -> Option<Arc<FontMetrics>> {
        let map = self.inner.read().ok()?;
        map.get(font_name).cloned()
    }

    /// Number of registered fonts. Diagnostic / test introspection.
    pub fn len(&self) -> usize {
        self.inner.read().map(|m| m.len()).unwrap_or(0)
    }

    /// Whether the store contains no fonts.
    pub fn is_empty(&self) -> bool {
        self.inner.read().map(|m| m.is_empty()).unwrap_or(true)
    }
}

impl Default for FontMetricsStore {
    fn default() -> Self {
        Self::new()
    }
}

// Dynamic registry for custom font metrics
lazy_static::lazy_static! {
    static ref CUSTOM_FONT_METRICS: RwLock<HashMap<String, FontMetrics>> =
        RwLock::new(HashMap::new());
}

lazy_static::lazy_static! {
    static ref FONT_METRICS: HashMap<Font, FontMetrics> = {
        // Single source of truth: the byte-indexed Adobe AFM tables in
        // `text/fonts/standard.rs`. Each char-keyed entry is derived by decoding
        // every WinAnsi byte code to its Unicode character, so this char-keyed
        // API and the byte-keyed `StandardFontMetrics` API stay in lock-step
        // (#313). Symbolic fonts (Symbol, ZapfDingbats) are excluded: their
        // encoding is not WinAnsi and `measure_*` short-circuits them anyway.
        use crate::text::encoding::winansi_decode_char;
        use crate::text::fonts::get_standard_font_metrics;

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

        let mut metrics = HashMap::new();
        for font in fonts {
            if let Some(sm) = get_standard_font_metrics(&font) {
                let mut widths = HashMap::new();
                for code in 0u16..=255 {
                    let ch = winansi_decode_char(code as u8);
                    widths.insert(ch, sm.widths[code as usize] as u16);
                }
                metrics.insert(
                    font,
                    FontMetrics::from_char_map(widths, sm.default_width as u16),
                );
            }
        }
        metrics
    };
}

/// Measure the width of a text string in a given font and size.
///
/// Variant of `measure_text` that consults a `FontMetricsStore` for
/// `Font::Custom` lookups before falling back to the legacy global
/// registry. Used internally by `TextFlowContext`, `TextContext`, and
/// `measure_text_block_with` to scope measurement to a single Document.
pub fn measure_text_with(
    text: &str,
    font: &Font,
    font_size: f64,
    store: Option<&FontMetricsStore>,
) -> f64 {
    if font.is_symbolic() {
        return text.len() as f64 * font_size * 0.6;
    }
    let metrics = lookup(font, store);
    let width_units: u32 = text.chars().map(|ch| metrics.char_width(ch) as u32).sum();
    (width_units as f64 / 1000.0) * font_size
}

/// Measure the width of a text string in a given font and size.
///
/// Back-compat shim. Delegates to `measure_text_with(text, font, font_size, None)`.
/// Custom fonts not registered globally fall back to default widths plus a
/// rate-limited diagnostic warning. For new code, prefer `measure_text_with`
/// or use `Document::new_page_a4()` so the measurement context carries a
/// `FontMetricsStore` automatically.
#[inline]
pub fn measure_text(text: &str, font: &Font, font_size: f64) -> f64 {
    measure_text_with(text, font, font_size, None)
}

/// Measure the width of a single character in a given font and size.
///
/// Variant of `measure_char` that consults a `FontMetricsStore` for
/// `Font::Custom` lookups before falling back to the legacy global
/// registry. Takes the font by value (matching the existing
/// `measure_char` signature, which predates this scope-aware variant).
pub fn measure_char_with(
    ch: char,
    font: Font,
    font_size: f64,
    store: Option<&FontMetricsStore>,
) -> f64 {
    if font.is_symbolic() {
        return font_size * 0.6;
    }
    let metrics = lookup(&font, store);
    (metrics.char_width(ch) as f64 / 1000.0) * font_size
}

/// Back-compat shim — see `measure_char_with`.
#[inline]
pub fn measure_char(ch: char, font: Font, font_size: f64) -> f64 {
    measure_char_with(ch, font, font_size, None)
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
#[deprecated(
    since = "2.8.0",
    note = "use Document::add_font_from_bytes; the global registry is process-wide and not bounded — see issue #230"
)]
pub fn register_custom_font_metrics(font_name: String, metrics: FontMetrics) {
    match CUSTOM_FONT_METRICS.write() {
        Ok(mut custom_metrics) => {
            custom_metrics.insert(font_name, metrics);
        }
        Err(e) => {
            tracing::warn!(
                "Font metrics registry lock is poisoned; \
                 could not register metrics for font '{}': {}",
                font_name,
                e
            );
        }
    }
}

/// Get metrics for a custom font
#[deprecated(
    since = "2.8.0",
    note = "use FontMetricsStore::get via a Document — the global registry is process-wide and not bounded — see issue #230"
)]
pub fn get_custom_font_metrics(font_name: &str) -> Option<FontMetrics> {
    if let Ok(custom_metrics) = CUSTOM_FONT_METRICS.read() {
        custom_metrics.get(font_name).cloned()
    } else {
        None
    }
}

/// Look up font metrics for any font (standard or custom).
///
/// Resolution order for `Font::Custom(name)`:
/// 1. Document scope (`store`) — takes precedence when present.
/// 2. Legacy global registry — hierarchical fallback (deprecated in Task 12 of #230).
/// 3. Default metrics + rate-limited warning via `warn_unknown_custom_font_once`.
///
/// Read path only — no side effects on either registry.
fn lookup(font: &Font, store: Option<&FontMetricsStore>) -> FontMetrics {
    match font {
        Font::Custom(font_name) => {
            // 1. Document scope (precedence)
            if let Some(s) = store {
                if let Some(arc_m) = s.get(font_name) {
                    return (*arc_m).clone();
                }
            }
            // 2. Legacy global (deprecated, hierarchical fallback)
            if let Some(custom_metrics) = get_custom_font_metrics_internal(font_name) {
                return custom_metrics;
            }
            // 3. Default + warn-once
            warn_unknown_custom_font_once(font_name);
            (*default_custom_metrics_arc()).clone()
        }
        _ => FONT_METRICS.get(font).cloned().unwrap_or_else(|| {
            tracing::debug!(
                "Warning: Standard font metrics not found for {:?}, using default",
                font
            );
            (*default_custom_metrics_arc()).clone()
        }),
    }
}

/// Internal accessor for the legacy global registry. Wraps
/// `get_custom_font_metrics` (which Task 12 of #230 will mark `#[deprecated]`)
/// so the lookup path does not itself produce a deprecation warning at
/// every internal call site once Task 12 lands.
fn get_custom_font_metrics_internal(font_name: &str) -> Option<FontMetrics> {
    if let Ok(custom_metrics) = CUSTOM_FONT_METRICS.read() {
        custom_metrics.get(font_name).cloned()
    } else {
        None
    }
}

lazy_static::lazy_static! {
    /// Cached default metrics for unknown custom fonts. Building this map
    /// once (lazy_static) means subsequent fallbacks reuse the same data
    /// rather than rebuilding the CJK table on every miss.
    static ref DEFAULT_CUSTOM_METRICS_ARC: Arc<FontMetrics> =
        Arc::new(create_default_custom_metrics());
}

fn default_custom_metrics_arc() -> Arc<FontMetrics> {
    DEFAULT_CUSTOM_METRICS_ARC.clone()
}

lazy_static::lazy_static! {
    /// Names already warned about. Rate-limits the unknown-font warning to
    /// one emission per name per process.
    static ref WARNED_UNKNOWN_FONTS: RwLock<std::collections::HashSet<String>> =
        RwLock::new(std::collections::HashSet::new());
}

fn warn_unknown_custom_font_once(font_name: &str) {
    {
        if let Ok(set) = WARNED_UNKNOWN_FONTS.read() {
            if set.contains(font_name) {
                return;
            }
        }
    }
    if let Ok(mut set) = WARNED_UNKNOWN_FONTS.write() {
        if set.insert(font_name.to_string()) {
            tracing::warn!(
                "custom font '{}' measured but not registered; widths will use \
                 defaults — register via Document::add_font_from_bytes",
                font_name
            );
        }
    }
}

/// Create default metrics for a custom font (fallback when no specific metrics available).
/// Result is cached via `lazy_static` — the expensive CJK range insertion (~6,500 entries)
/// only happens once. Subsequent calls return a clone.
pub(crate) fn create_default_custom_metrics() -> FontMetrics {
    lazy_static::lazy_static! {
        static ref DEFAULT_CUSTOM_METRICS: FontMetrics = build_default_custom_metrics();
    }
    DEFAULT_CUSTOM_METRICS.clone()
}

#[cfg(test)]
pub(crate) static DEFAULT_CUSTOM_METRICS_BUILD_COUNT: std::sync::atomic::AtomicUsize =
    std::sync::atomic::AtomicUsize::new(0);

fn build_default_custom_metrics() -> FontMetrics {
    #[cfg(test)]
    DEFAULT_CUSTOM_METRICS_BUILD_COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

    let mut metrics = FontMetrics::new(556).with_widths(&[
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
    ]);

    // CJK characters are full-width (1000 units). Insert defaults for common ranges
    // so that even without registered font metrics, CJK text measurement is reasonable.
    let cjk_ranges: &[(u32, u32)] = &[
        (0x3000, 0x303F), // CJK Symbols and Punctuation
        (0x3040, 0x309F), // Hiragana
        (0x30A0, 0x30FF), // Katakana
        (0x4E00, 0x9FFF), // CJK Unified Ideographs
        (0xF900, 0xFAFF), // CJK Compatibility Ideographs
        (0xFF00, 0xFFEF), // Halfwidth and Fullwidth Forms
    ];
    for &(start, end) in cjk_ranges {
        for code_point in start..=end {
            if let Some(ch) = char::from_u32(code_point) {
                metrics.widths.insert(ch, 1000);
            }
        }
    }

    metrics
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

    // ---- #309: non-ASCII WinAnsi glyph widths ----------------------------
    // At font_size 1000.0, `measure_char` returns the glyph advance directly in
    // font units, so these assert exact Adobe Core-14 AFM widths. Expected
    // values come from the Adobe AFM files (glyph name -> WX), mapped through
    // the Adobe Glyph List and WinAnsiEncoding (= Windows-1252). Pre-fix every
    // non-ASCII char resolved to the generic `default_width`.

    fn afm_width(ch: char, font: Font) -> u16 {
        measure_char(ch, font, 1000.0).round() as u16
    }

    #[test]
    fn test_non_ascii_winansi_width_matches_afm_helvetica() {
        // The exact case from issue #309: í must be 278, not the 556 default.
        assert_eq!(afm_width('í', Font::Helvetica), 278);
        assert_ne!(
            afm_width('í', Font::Helvetica),
            556,
            "must not fall back to default_width"
        );
        // Accented letters track their base-letter advance in Helvetica.
        assert_eq!(afm_width('á', Font::Helvetica), 556); // aacute
        assert_eq!(afm_width('ñ', Font::Helvetica), 556); // ntilde
        assert_eq!(afm_width('é', Font::Helvetica), 556); // eacute
                                                          // Typographic punctuation and symbols.
        assert_eq!(afm_width('—', Font::Helvetica), 1000); // emdash U+2014
        assert_eq!(afm_width('–', Font::Helvetica), 556); // endash U+2013
        assert_eq!(afm_width('•', Font::Helvetica), 350); // bullet U+2022
        assert_eq!(afm_width('©', Font::Helvetica), 737); // copyright
        assert_eq!(afm_width('€', Font::Helvetica), 556); // euro U+20AC
        assert_eq!(afm_width('’', Font::Helvetica), 222); // quoteright U+2019
    }

    #[test]
    fn test_non_ascii_winansi_width_matches_afm_helvetica_bold() {
        assert_eq!(afm_width('í', Font::HelveticaBold), 278);
        assert_eq!(afm_width('é', Font::HelveticaBold), 556);
        assert_eq!(afm_width('—', Font::HelveticaBold), 1000);
        assert_eq!(afm_width('’', Font::HelveticaBold), 278); // bold quoteright
    }

    #[test]
    fn test_non_ascii_winansi_width_matches_afm_times() {
        assert_eq!(afm_width('í', Font::TimesRoman), 278); // iacute
        assert_eq!(afm_width('é', Font::TimesRoman), 444); // eacute
        assert_eq!(afm_width('ñ', Font::TimesRoman), 500); // ntilde
        assert_eq!(afm_width('—', Font::TimesRoman), 1000);
        assert_eq!(afm_width('©', Font::TimesRoman), 760);
        assert_eq!(afm_width('™', Font::TimesRoman), 980); // trademark U+2122
    }

    #[test]
    fn test_non_ascii_winansi_width_courier_is_monospace() {
        // Every Courier glyph, ASCII or not, is 600 units.
        for ch in ['í', 'é', 'ñ', '—', '•', '©', '€', '’'] {
            assert_eq!(afm_width(ch, Font::Courier), 600, "char {ch:?}");
        }
    }

    #[test]
    fn test_non_ascii_measure_text_string() {
        // "café" — c=500, a=556, f=278, é=556 = 1890 units at size 1000.
        let w = measure_text("café", &Font::Helvetica, 1000.0);
        assert!((w - 1890.0).abs() < 0.5, "got {w}");
    }

    #[test]
    fn test_measure_text_helvetica() {
        let text = "Hello";
        let width = measure_text(text, &Font::Helvetica, 12.0);

        // Helvetica "H" = 722, "e" = 556, "l" = 222, "l" = 222, "o" = 556
        // Total = 2278 units = 2.278 at size 1.0, * 12.0 = 27.336
        assert!((width - 27.336).abs() < 0.01);
    }

    #[test]
    fn test_measure_text_courier() {
        let text = "ABC";
        let width = measure_text(text, &Font::Courier, 10.0);

        // Courier is monospace: all chars = 600 units
        // 3 chars * 600 = 1800 units = 1.8 at size 1.0, * 10.0 = 18.0
        assert_eq!(width, 18.0);
    }

    #[test]
    fn test_measure_text_symbolic_fonts() {
        let text = "ABC";
        let symbol_width = measure_text(text, &Font::Symbol, 12.0);
        let zapf_width = measure_text(text, &Font::ZapfDingbats, 12.0);

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
            let _width = measure_text("A", font, 12.0);
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
        let width = measure_text("", &Font::Helvetica, 12.0);
        assert_eq!(width, 0.0);
    }

    #[test]
    fn test_measure_text_consistency() {
        let text = "Hello";

        // Measuring whole text should equal sum of individual characters
        let total_width = measure_text(text, &Font::Helvetica, 12.0);
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
        // Characters with no glyph in the WinAnsi-encoded base-14 fonts (Greek,
        // CJK) must fall back to default_width. Note: € (U+20AC) and ™ (U+2122)
        // are NOT here — they ARE WinAnsi glyphs and now carry real AFM widths
        // (see test_non_ascii_winansi_width_matches_afm_*), #309.
        let unicode_chars = ['β', 'π', 'δ', '中', '雪'];

        for ch in &unicode_chars {
            let helvetica_width = measure_char(*ch, Font::Helvetica, 12.0);
            let times_width = measure_char(*ch, Font::TimesRoman, 12.0);
            let courier_width = measure_char(*ch, Font::Courier, 12.0);

            // Should use each font's default_width (the Adobe AFM/standard.rs
            // values, single source of truth per #313): Helvetica 278,
            // Times-Roman 250, Courier 600.
            let helvetica_expected = 278.0 * 12.0 / 1000.0;
            let times_expected = 250.0 * 12.0 / 1000.0;
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
    #[allow(deprecated)]
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
    #[allow(deprecated)]
    fn test_get_custom_font_metrics_not_found() {
        let result = get_custom_font_metrics("NonExistentFont12345");
        // May or may not be found depending on previous tests
        // Just verify no panic
        let _ = result;
    }

    #[test]
    #[allow(deprecated)]
    fn test_measure_text_custom_font() {
        // Register a custom font with known metrics
        let metrics = FontMetrics::new(500).with_widths(&[('A', 600), ('B', 600), ('C', 600)]);
        register_custom_font_metrics("MyCustomFont".to_string(), metrics);

        let width = measure_text("ABC", &Font::Custom("MyCustomFont".to_string()), 10.0);

        // 3 chars * 600 units = 1800 units = 1.8 at size 1.0 * 10.0 = 18.0
        assert!((width - 18.0).abs() < 0.01);
    }

    #[test]
    #[allow(deprecated)]
    fn test_measure_char_custom_font() {
        let metrics = FontMetrics::new(500).with_widths(&[('X', 700)]);
        register_custom_font_metrics("CustomCharTest".to_string(), metrics);

        let width = measure_char('X', Font::Custom("CustomCharTest".to_string()), 10.0);

        // 700 units / 1000 * 10.0 = 7.0
        assert!((width - 7.0).abs() < 0.01);
    }

    #[test]
    fn test_custom_font_no_auto_register_default() {
        // When using an unregistered custom font, default metrics are used but NOT
        // registered into the global. The read path must have no side effects.
        let unique = format!("NoAutoRegister_{}", std::process::id());
        let width = measure_char('A', Font::Custom(unique.clone()), 10.0);

        // Should use default metrics (Helvetica-like), A = 667
        let expected = 667.0 * 10.0 / 1000.0;
        assert!((width - expected).abs() < 0.01);

        // Must NOT be registered as a side effect
        // get_custom_font_metrics is deprecated by Task 12 of #230 (v2.8.0).
        // #[allow(deprecated)] is applied now to avoid churn when the attribute lands.
        #[allow(deprecated)]
        let metrics = get_custom_font_metrics(&unique);
        assert!(
            metrics.is_none(),
            "read path must not auto-register unknown custom fonts"
        );
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
    fn test_create_default_custom_metrics_is_cached() {
        // Verifies the lazy_static cache: build_default_custom_metrics must run
        // at most once regardless of how many times create_default_custom_metrics
        // is called. Instrumented via a #[cfg(test)] AtomicUsize counter so the
        // assertion is invariant to runner load (the prior timing-based threshold
        // was flaky under full-suite parallelism).
        use std::sync::atomic::Ordering;
        let before = DEFAULT_CUSTOM_METRICS_BUILD_COUNT.load(Ordering::Relaxed);
        for _ in 0..1000 {
            let _ = create_default_custom_metrics();
        }
        let after = DEFAULT_CUSTOM_METRICS_BUILD_COUNT.load(Ordering::Relaxed);
        let delta = after - before;
        assert!(
            delta <= 1,
            "build_default_custom_metrics ran {} times during 1000 calls; cache broken",
            delta
        );
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
    fn test_times_variants_use_their_own_metrics() {
        // Each Times variant carries its own Adobe AFM widths; they are NOT
        // aliased to Times-Roman (#313). For 'A': Roman 722, Bold 722,
        // Italic 611, BoldItalic 667.
        let w = |f| (measure_char('A', f, 1000.0)).round() as u16;
        assert_eq!(w(Font::TimesRoman), 722);
        assert_eq!(w(Font::TimesBold), 722);
        assert_eq!(w(Font::TimesItalic), 611);
        assert_eq!(w(Font::TimesBoldItalic), 667);
        // Italic is genuinely distinct from Roman here.
        assert_ne!(w(Font::TimesItalic), w(Font::TimesRoman));
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

    // ── Task 2 tests ────────────────────────────────────────────────────────

    /// Clear the warned-set between tests that assert warn-once behaviour.
    fn reset_warned_unknown_fonts() {
        if let Ok(mut set) = WARNED_UNKNOWN_FONTS.write() {
            set.clear();
        }
    }

    #[test]
    fn test_warn_unknown_font_rate_limited_once_per_name() {
        let unique = format!("RateLimitTask2_{}", std::process::id());
        // Isolate the warned-set from any state planted by earlier tests in this
        // process. Helper is intentionally test-only.
        reset_warned_unknown_fonts();

        warn_unknown_custom_font_once(&unique);
        warn_unknown_custom_font_once(&unique);
        warn_unknown_custom_font_once(&unique);

        let set = WARNED_UNKNOWN_FONTS.read().expect("lock");
        assert!(
            set.contains(&unique),
            "name should be in the warned set after first call"
        );
        let count = set.iter().filter(|n| *n == &unique).count();
        assert_eq!(
            count, 1,
            "warn_unknown_custom_font_once must rate-limit to one entry per name"
        );
    }

    #[test]
    fn test_unknown_custom_font_does_not_register_on_read() {
        // Use a unique name so this test does not collide with other tests
        // running in parallel under cargo test.
        let unique = format!("UnknownNameTask2_{}", std::process::id());
        let _ = measure_text("hello", &Font::Custom(unique.clone()), 12.0);
        // Lookup must not have planted the name in the global registry.
        // get_custom_font_metrics is deprecated by Task 12 of #230 (v2.8.0).
        // #[allow(deprecated)] is applied now to avoid churn when the attribute lands.
        #[allow(deprecated)]
        let leaked = get_custom_font_metrics(&unique);
        assert!(
            leaked.is_none(),
            "read path must not auto-register '{}'",
            unique
        );
    }

    #[test]
    fn test_unknown_custom_font_returns_default_widths() {
        let unique = format!("UnknownReturnTask2_{}", std::process::id());
        let width = measure_text("AAAA", &Font::Custom(unique), 12.0);
        // create_default_custom_metrics maps 'A' = 667; default_width = 556 for
        // unmapped chars. Test uses "AAAA": 4 × 667 / 1000 × 12 = 32.016.
        assert!(
            (width - 32.016).abs() < 0.01,
            "unknown custom fonts must use the default metrics (A=667), got {}",
            width
        );
    }

    #[test]
    fn test_split_into_words_mixed_whitespace() {
        let words = split_into_words("A B  C   D");
        assert_eq!(words, vec!["A", " ", "B", "  ", "C", "   ", "D"]);
    }

    #[test]
    fn test_font_metrics_store_register_and_get() {
        let store = FontMetricsStore::new();
        assert!(store.is_empty());
        assert_eq!(store.len(), 0);

        let metrics = FontMetrics::new(500).with_widths(&[('A', 700), ('B', 720)]);
        store.register("MyFont", metrics);

        assert_eq!(store.len(), 1);
        assert!(!store.is_empty());

        let got = store.get("MyFont").expect("font should be present");
        assert_eq!(got.char_width('A'), 700);
        assert_eq!(got.char_width('B'), 720);
        assert_eq!(got.char_width('Z'), 500); // default fallback
    }

    #[test]
    fn test_font_metrics_store_overwrite_same_name() {
        let store = FontMetricsStore::new();
        store.register("X", FontMetrics::new(500).with_widths(&[('A', 600)]));
        store.register("X", FontMetrics::new(500).with_widths(&[('A', 800)]));

        let got = store.get("X").unwrap();
        assert_eq!(got.char_width('A'), 800); // last writer wins
        assert_eq!(store.len(), 1);
    }

    #[test]
    fn test_font_metrics_store_clone_shares_state() {
        let store_a = FontMetricsStore::new();
        let store_b = store_a.clone();

        store_a.register("Shared", FontMetrics::new(400));
        assert_eq!(store_b.len(), 1, "clone must share the underlying registry");
        assert!(store_b.get("Shared").is_some());

        store_b.register("AlsoShared", FontMetrics::new(400));
        assert_eq!(store_a.len(), 2);
    }

    #[test]
    fn test_font_metrics_store_get_miss_returns_none_no_side_effects() {
        let store = FontMetricsStore::new();
        assert!(store.get("Unknown").is_none());
        assert_eq!(store.len(), 0); // no auto-register
        assert!(store.is_empty());
    }

    // ── Task 3 tests ────────────────────────────────────────────────────────

    #[test]
    fn test_lookup_document_scope_takes_precedence_over_global() {
        let unique = format!("PrecedenceTask3_{}", std::process::id());

        // Plant something in the legacy global.
        // get_custom_font_metrics is deprecated by Task 12 of #230 (v2.8.0).
        // #[allow(deprecated)] is applied now to avoid churn when the attribute lands.
        #[allow(deprecated)]
        register_custom_font_metrics(
            unique.clone(),
            FontMetrics::new(500).with_widths(&[('A', 100)]),
        );

        // Per-Document store has different metrics for the same name.
        let store = FontMetricsStore::new();
        store.register(
            unique.clone(),
            FontMetrics::new(500).with_widths(&[('A', 900)]),
        );

        let resolved = lookup(&Font::Custom(unique), Some(&store));
        assert_eq!(
            resolved.char_width('A'),
            900,
            "Document scope must win over global"
        );
    }

    #[test]
    fn test_lookup_falls_through_to_global_when_store_misses() {
        let unique = format!("FallthroughTask3_{}", std::process::id());

        // get_custom_font_metrics is deprecated by Task 12 of #230 (v2.8.0).
        // #[allow(deprecated)] is applied now to avoid churn when the attribute lands.
        #[allow(deprecated)]
        register_custom_font_metrics(
            unique.clone(),
            FontMetrics::new(500).with_widths(&[('A', 333)]),
        );

        let empty_store = FontMetricsStore::new();
        let resolved = lookup(&Font::Custom(unique), Some(&empty_store));
        assert_eq!(
            resolved.char_width('A'),
            333,
            "must fall through to legacy global when Document store misses"
        );
    }

    #[test]
    fn test_lookup_with_none_store_uses_global_then_default() {
        let unique = format!("NoneStoreTask3_{}", std::process::id());

        // No global, no store. Should default+warn.
        let resolved = lookup(&Font::Custom(unique), None);
        assert_eq!(resolved.char_width('A'), 667); // create_default_custom_metrics maps 'A' = 667
    }

    // ── Task 4 tests ────────────────────────────────────────────────────────

    #[test]
    fn test_measure_text_with_uses_document_scope() {
        let unique = format!("MeasureWithTask4_{}", std::process::id());
        let store = FontMetricsStore::new();
        store.register(
            unique.clone(),
            // Each char (A through F) at 1000 units; 'A' x 4 chars = 48.0 at 12pt.
            FontMetrics::new(500).with_widths(&[('A', 1000)]),
        );

        let width = measure_text_with("AAAA", &Font::Custom(unique), 12.0, Some(&store));
        // 4 * 1000 / 1000 * 12 = 48
        assert!((width - 48.0).abs() < 0.01, "got {}", width);
    }

    #[test]
    fn test_measure_text_back_compat_shim_passes_none() {
        let unique = format!("BackCompatTask4_{}", std::process::id());
        // Without store, with empty global → default 'A' from create_default_custom_metrics
        // ('A' = 667). 4 chars × 667 / 1000 × 12 = 32.016
        let width = measure_text("AAAA", &Font::Custom(unique), 12.0);
        assert!((width - 32.016).abs() < 0.01, "got {}", width);
    }

    #[test]
    fn test_measure_char_with_uses_document_scope() {
        let unique = format!("MeasureCharWithTask4_{}", std::process::id());
        let store = FontMetricsStore::new();
        store.register(
            unique.clone(),
            FontMetrics::new(500).with_widths(&[('Z', 800)]),
        );
        let width = measure_char_with('Z', Font::Custom(unique), 10.0, Some(&store));
        // 800 / 1000 * 10 = 8
        assert!((width - 8.0).abs() < 0.01, "got {}", width);
    }
}
