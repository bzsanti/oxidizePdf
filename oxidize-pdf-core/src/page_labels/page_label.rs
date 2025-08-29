//! Page label definitions according to ISO 32000-1

use crate::objects::{Dictionary, Object};

/// Page label numbering style
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PageLabelStyle {
    /// Decimal arabic numerals (1, 2, 3, ...)
    DecimalArabic,
    /// Uppercase roman numerals (I, II, III, IV, ...)
    UppercaseRoman,
    /// Lowercase roman numerals (i, ii, iii, iv, ...)
    LowercaseRoman,
    /// Uppercase letters (A, B, C, ... AA, BB, ...)
    UppercaseLetters,
    /// Lowercase letters (a, b, c, ... aa, bb, ...)
    LowercaseLetters,
    /// No page numbers (labels consist only of prefix)
    None,
}

impl PageLabelStyle {
    /// Convert to PDF name
    pub fn to_pdf_name(&self) -> Option<&'static str> {
        match self {
            PageLabelStyle::DecimalArabic => Some("D"),
            PageLabelStyle::UppercaseRoman => Some("r"),
            PageLabelStyle::LowercaseRoman => Some("R"),
            PageLabelStyle::UppercaseLetters => Some("A"),
            PageLabelStyle::LowercaseLetters => Some("a"),
            PageLabelStyle::None => None,
        }
    }

    /// Format a page number in this style
    pub fn format(&self, number: u32) -> String {
        match self {
            PageLabelStyle::DecimalArabic => number.to_string(),
            PageLabelStyle::UppercaseRoman => to_roman(number).to_uppercase(),
            PageLabelStyle::LowercaseRoman => to_roman(number),
            PageLabelStyle::UppercaseLetters => to_letters(number, true),
            PageLabelStyle::LowercaseLetters => to_letters(number, false),
            PageLabelStyle::None => String::new(),
        }
    }
}

/// Page label for a range of pages
#[derive(Debug, Clone)]
pub struct PageLabel {
    /// Numbering style
    pub style: PageLabelStyle,
    /// Label prefix (e.g., "Chapter " for "Chapter 1")
    pub prefix: Option<String>,
    /// First value of the numeric portion (default 1)
    pub start: u32,
}

impl PageLabel {
    /// Create a new page label
    pub fn new(style: PageLabelStyle) -> Self {
        Self {
            style,
            prefix: None,
            start: 1,
        }
    }

    /// Create decimal arabic label (1, 2, 3, ...)
    pub fn decimal() -> Self {
        Self::new(PageLabelStyle::DecimalArabic)
    }

    /// Create uppercase roman label (I, II, III, ...)
    pub fn roman_uppercase() -> Self {
        Self::new(PageLabelStyle::UppercaseRoman)
    }

    /// Create lowercase roman label (i, ii, iii, ...)
    pub fn roman_lowercase() -> Self {
        Self::new(PageLabelStyle::LowercaseRoman)
    }

    /// Create uppercase letter label (A, B, C, ...)
    pub fn letters_uppercase() -> Self {
        Self::new(PageLabelStyle::UppercaseLetters)
    }

    /// Create lowercase letter label (a, b, c, ...)
    pub fn letters_lowercase() -> Self {
        Self::new(PageLabelStyle::LowercaseLetters)
    }

    /// Create label with no numbers (prefix only)
    pub fn prefix_only(prefix: impl Into<String>) -> Self {
        Self {
            style: PageLabelStyle::None,
            prefix: Some(prefix.into()),
            start: 1,
        }
    }

    /// Set label prefix
    pub fn with_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.prefix = Some(prefix.into());
        self
    }

    /// Set starting number
    pub fn starting_at(mut self, start: u32) -> Self {
        self.start = start;
        self
    }

    /// Format a page label for a given offset
    pub fn format_label(&self, offset: u32) -> String {
        let mut label = String::new();

        if let Some(prefix) = &self.prefix {
            label.push_str(prefix);
        }

        if self.style != PageLabelStyle::None {
            let number = self.start + offset;
            label.push_str(&self.style.format(number));
        }

        label
    }

    /// Convert to PDF dictionary
    pub fn to_dict(&self) -> Dictionary {
        let mut dict = Dictionary::new();

        if let Some(type_name) = self.style.to_pdf_name() {
            dict.set("Type", Object::Name(type_name.to_string()));
        }

        if let Some(prefix) = &self.prefix {
            dict.set("P", Object::String(prefix.clone()));
        }

        if self.start != 1 {
            dict.set("St", Object::Integer(self.start as i64));
        }

        dict
    }
}

/// Page label range - associates a page label with a starting page
#[derive(Debug, Clone)]
pub struct PageLabelRange {
    /// Starting page index (0-based)
    pub start_page: u32,
    /// Page label for this range
    pub label: PageLabel,
}

impl PageLabelRange {
    /// Create a new page label range
    pub fn new(start_page: u32, label: PageLabel) -> Self {
        Self { start_page, label }
    }
}

/// Convert number to roman numerals
fn to_roman(mut num: u32) -> String {
    if num == 0 {
        return String::new();
    }

    let values = [
        (1000, "m"),
        (900, "cm"),
        (500, "d"),
        (400, "cd"),
        (100, "c"),
        (90, "xc"),
        (50, "l"),
        (40, "xl"),
        (10, "x"),
        (9, "ix"),
        (5, "v"),
        (4, "iv"),
        (1, "i"),
    ];

    let mut result = String::new();

    for (value, numeral) in values.iter() {
        while num >= *value {
            result.push_str(numeral);
            num -= value;
        }
    }

    result
}

/// Convert number to letters (A, B, ... Z, AA, AB, ...)
fn to_letters(num: u32, uppercase: bool) -> String {
    if num == 0 {
        return String::new();
    }

    let mut result = String::new();
    let mut n = num;

    while n > 0 {
        let remainder = ((n - 1) % 26) as u8;
        let letter = if uppercase {
            (b'A' + remainder) as char
        } else {
            (b'a' + remainder) as char
        };
        result.insert(0, letter);
        n = (n - 1) / 26;
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_page_label_styles() {
        assert_eq!(PageLabelStyle::DecimalArabic.format(1), "1");
        assert_eq!(PageLabelStyle::DecimalArabic.format(42), "42");

        assert_eq!(PageLabelStyle::UppercaseRoman.format(1), "I");
        assert_eq!(PageLabelStyle::UppercaseRoman.format(4), "IV");
        assert_eq!(PageLabelStyle::UppercaseRoman.format(9), "IX");
        assert_eq!(PageLabelStyle::UppercaseRoman.format(58), "LVIII");

        assert_eq!(PageLabelStyle::LowercaseRoman.format(1), "i");
        assert_eq!(PageLabelStyle::LowercaseRoman.format(4), "iv");

        assert_eq!(PageLabelStyle::UppercaseLetters.format(1), "A");
        assert_eq!(PageLabelStyle::UppercaseLetters.format(26), "Z");
        assert_eq!(PageLabelStyle::UppercaseLetters.format(27), "AA");
        assert_eq!(PageLabelStyle::UppercaseLetters.format(52), "AZ");

        assert_eq!(PageLabelStyle::LowercaseLetters.format(1), "a");
        assert_eq!(PageLabelStyle::LowercaseLetters.format(26), "z");
        assert_eq!(PageLabelStyle::LowercaseLetters.format(27), "aa");

        assert_eq!(PageLabelStyle::None.format(1), "");
        assert_eq!(PageLabelStyle::None.format(100), "");
    }

    #[test]
    fn test_page_label_creation() {
        let label = PageLabel::decimal();
        assert_eq!(label.style, PageLabelStyle::DecimalArabic);
        assert_eq!(label.start, 1);
        assert!(label.prefix.is_none());

        let label = PageLabel::roman_lowercase()
            .with_prefix("Page ")
            .starting_at(5);
        assert_eq!(label.style, PageLabelStyle::LowercaseRoman);
        assert_eq!(label.start, 5);
        assert_eq!(label.prefix, Some("Page ".to_string()));
    }

    #[test]
    fn test_format_label() {
        let label = PageLabel::decimal().with_prefix("Chapter ");
        assert_eq!(label.format_label(0), "Chapter 1");
        assert_eq!(label.format_label(1), "Chapter 2");

        let label = PageLabel::roman_uppercase().starting_at(1);
        assert_eq!(label.format_label(0), "I");
        assert_eq!(label.format_label(3), "IV");

        let label = PageLabel::prefix_only("Appendix");
        assert_eq!(label.format_label(0), "Appendix");
        assert_eq!(label.format_label(10), "Appendix");
    }

    #[test]
    fn test_to_dict() {
        let label = PageLabel::decimal();
        let dict = label.to_dict();
        assert_eq!(dict.get("Type"), Some(&Object::Name("D".to_string())));
        assert!(dict.get("P").is_none());
        assert!(dict.get("St").is_none());

        let label = PageLabel::roman_lowercase()
            .with_prefix("p. ")
            .starting_at(5);
        let dict = label.to_dict();
        assert_eq!(dict.get("Type"), Some(&Object::Name("R".to_string())));
        assert_eq!(dict.get("P"), Some(&Object::String("p. ".to_string())));
        assert_eq!(dict.get("St"), Some(&Object::Integer(5)));
    }

    #[test]
    fn test_roman_conversion() {
        assert_eq!(to_roman(1), "i");
        assert_eq!(to_roman(3), "iii");
        assert_eq!(to_roman(4), "iv");
        assert_eq!(to_roman(5), "v");
        assert_eq!(to_roman(9), "ix");
        assert_eq!(to_roman(10), "x");
        assert_eq!(to_roman(40), "xl");
        assert_eq!(to_roman(50), "l");
        assert_eq!(to_roman(90), "xc");
        assert_eq!(to_roman(100), "c");
        assert_eq!(to_roman(400), "cd");
        assert_eq!(to_roman(500), "d");
        assert_eq!(to_roman(900), "cm");
        assert_eq!(to_roman(1000), "m");
        assert_eq!(to_roman(1984), "mcmlxxxiv");
        assert_eq!(to_roman(3999), "mmmcmxcix");
    }

    #[test]
    fn test_letter_conversion() {
        assert_eq!(to_letters(1, true), "A");
        assert_eq!(to_letters(26, true), "Z");
        assert_eq!(to_letters(27, true), "AA");
        assert_eq!(to_letters(52, true), "AZ");
        assert_eq!(to_letters(53, true), "BA");
        assert_eq!(to_letters(702, true), "ZZ");
        assert_eq!(to_letters(703, true), "AAA");

        assert_eq!(to_letters(1, false), "a");
        assert_eq!(to_letters(26, false), "z");
        assert_eq!(to_letters(27, false), "aa");
    }

    #[test]
    fn test_page_label_style_to_pdf_name() {
        assert_eq!(PageLabelStyle::DecimalArabic.to_pdf_name(), Some("D"));
        assert_eq!(PageLabelStyle::UppercaseRoman.to_pdf_name(), Some("r"));
        assert_eq!(PageLabelStyle::LowercaseRoman.to_pdf_name(), Some("R"));
        assert_eq!(PageLabelStyle::UppercaseLetters.to_pdf_name(), Some("A"));
        assert_eq!(PageLabelStyle::LowercaseLetters.to_pdf_name(), Some("a"));
        assert_eq!(PageLabelStyle::None.to_pdf_name(), None);
    }

    #[test]
    fn test_page_label_with_all_styles() {
        // Test all constructor methods
        let decimal = PageLabel::decimal();
        assert_eq!(decimal.style, PageLabelStyle::DecimalArabic);

        let roman_upper = PageLabel::roman_uppercase();
        assert_eq!(roman_upper.style, PageLabelStyle::UppercaseRoman);

        let roman_lower = PageLabel::roman_lowercase();
        assert_eq!(roman_lower.style, PageLabelStyle::LowercaseRoman);

        let letters_upper = PageLabel::letters_uppercase();
        assert_eq!(letters_upper.style, PageLabelStyle::UppercaseLetters);

        let letters_lower = PageLabel::letters_lowercase();
        assert_eq!(letters_lower.style, PageLabelStyle::LowercaseLetters);

        let prefix_only = PageLabel::prefix_only("Prefix");
        assert_eq!(prefix_only.style, PageLabelStyle::None);
        assert_eq!(prefix_only.prefix, Some("Prefix".to_string()));
    }

    #[test]
    fn test_page_label_chaining() {
        let label = PageLabel::decimal().with_prefix("Page ").starting_at(10);

        assert_eq!(label.style, PageLabelStyle::DecimalArabic);
        assert_eq!(label.prefix, Some("Page ".to_string()));
        assert_eq!(label.start, 10);

        // Test formatting with chained settings
        assert_eq!(label.format_label(0), "Page 10");
        assert_eq!(label.format_label(5), "Page 15");
    }

    #[test]
    fn test_format_label_edge_cases() {
        // Test with empty prefix
        let label = PageLabel::decimal().with_prefix("");
        assert_eq!(label.format_label(0), "1");

        // Test with long prefix
        let long_prefix = "This is a very long prefix that might appear in some documents: ";
        let label = PageLabel::roman_uppercase().with_prefix(long_prefix);
        assert_eq!(label.format_label(0), format!("{}I", long_prefix));

        // Test with high starting number
        let label = PageLabel::decimal().starting_at(9999);
        assert_eq!(label.format_label(0), "9999");
        assert_eq!(label.format_label(1), "10000");
    }

    #[test]
    fn test_roman_edge_cases() {
        // Test edge cases for roman numerals
        assert_eq!(to_roman(49), "xlix");
        assert_eq!(to_roman(99), "xcix");
        assert_eq!(to_roman(499), "cdxcix");
        assert_eq!(to_roman(999), "cmxcix");
        assert_eq!(to_roman(1444), "mcdxliv");
        assert_eq!(to_roman(1994), "mcmxciv");
        assert_eq!(to_roman(2023), "mmxxiii");

        // Test with style formatting
        assert_eq!(PageLabelStyle::UppercaseRoman.format(49), "XLIX");
        assert_eq!(PageLabelStyle::LowercaseRoman.format(49), "xlix");
    }

    #[test]
    fn test_letter_edge_cases() {
        // Test more letter conversion cases
        assert_eq!(to_letters(78, true), "BZ"); // 26*2 + 26
        assert_eq!(to_letters(104, true), "CZ"); // 26*3 + 26
        assert_eq!(to_letters(701, true), "ZY"); // Last before ZZ
        assert_eq!(to_letters(728, true), "AAZ"); // 26*26 + 26 + 26
        assert_eq!(to_letters(1378, true), "AZZ"); // Complex case

        // Lowercase versions
        assert_eq!(to_letters(78, false), "bz");
        assert_eq!(to_letters(104, false), "cz");
        assert_eq!(to_letters(701, false), "zy");
    }

    #[test]
    fn test_prefix_only_variations() {
        // Test prefix-only labels with different content
        let label1 = PageLabel::prefix_only("Cover");
        assert_eq!(label1.format_label(0), "Cover");
        assert_eq!(label1.format_label(100), "Cover"); // Should always be same

        let label2 = PageLabel::prefix_only("Appendix A");
        assert_eq!(label2.format_label(0), "Appendix A");

        // Unicode prefix
        let label3 = PageLabel::prefix_only("附录");
        assert_eq!(label3.format_label(0), "附录");

        // Special characters
        let label4 = PageLabel::prefix_only("§1");
        assert_eq!(label4.format_label(0), "§1");
    }

    #[test]
    fn test_to_dict_comprehensive() {
        // Test dictionary generation with all combinations
        let label1 = PageLabel::new(PageLabelStyle::DecimalArabic);
        let dict1 = label1.to_dict();
        assert_eq!(dict1.get("Type"), Some(&Object::Name("D".to_string())));
        assert!(dict1.get("P").is_none()); // No prefix
        assert!(dict1.get("St").is_none()); // Default start (1)

        let label2 = PageLabel::new(PageLabelStyle::UppercaseLetters)
            .with_prefix("Section ")
            .starting_at(10);
        let dict2 = label2.to_dict();
        assert_eq!(dict2.get("Type"), Some(&Object::Name("A".to_string())));
        assert_eq!(
            dict2.get("P"),
            Some(&Object::String("Section ".to_string()))
        );
        assert_eq!(dict2.get("St"), Some(&Object::Integer(10)));

        // Test prefix-only (no Type field)
        let label3 = PageLabel::prefix_only("Index");
        let dict3 = label3.to_dict();
        assert!(dict3.get("Type").is_none());
        assert_eq!(dict3.get("P"), Some(&Object::String("Index".to_string())));
    }

    #[test]
    fn test_sequential_page_labels() {
        // Simulate a document with different label ranges
        let front_matter = PageLabel::roman_lowercase().with_prefix("");
        let main_content = PageLabel::decimal().starting_at(1);
        let appendix = PageLabel::letters_uppercase().with_prefix("Appendix ");

        // Front matter pages (i, ii, iii, iv)
        assert_eq!(front_matter.format_label(0), "i");
        assert_eq!(front_matter.format_label(1), "ii");
        assert_eq!(front_matter.format_label(2), "iii");
        assert_eq!(front_matter.format_label(3), "iv");

        // Main content (1, 2, 3...)
        assert_eq!(main_content.format_label(0), "1");
        assert_eq!(main_content.format_label(99), "100");

        // Appendix (Appendix A, Appendix B...)
        assert_eq!(appendix.format_label(0), "Appendix A");
        assert_eq!(appendix.format_label(1), "Appendix B");
        assert_eq!(appendix.format_label(25), "Appendix Z");
    }

    #[test]
    fn test_large_number_formatting() {
        // Test with very large numbers
        let label = PageLabel::decimal().starting_at(999999);
        assert_eq!(label.format_label(0), "999999");
        assert_eq!(label.format_label(1), "1000000");

        // Roman numerals with large numbers (typically capped at 3999)
        assert_eq!(to_roman(4000), "mmmm"); // Graceful handling
        assert_eq!(to_roman(5000), "mmmmm");

        // Letters with large numbers
        assert_eq!(to_letters(18278, true).len() > 0, true); // Should produce something
    }

    #[test]
    fn test_special_prefix_combinations() {
        // Test various prefix and style combinations
        let combinations = vec![
            (PageLabel::decimal().with_prefix("№"), 0, "№1"),
            (
                PageLabel::roman_uppercase().with_prefix("Chapter "),
                0,
                "Chapter I",
            ),
            (
                PageLabel::letters_lowercase()
                    .with_prefix("(")
                    .with_prefix(")"),
                0,
                ")a",
            ),
            (
                PageLabel::decimal().with_prefix("Page ").starting_at(100),
                0,
                "Page 100",
            ),
        ];

        for (label, offset, expected) in combinations {
            assert_eq!(label.format_label(offset), expected);
        }
    }

    #[test]
    fn test_clone_and_equality() {
        let label1 = PageLabel::decimal().with_prefix("Page ");
        let label2 = label1.clone();

        assert_eq!(label1.style, label2.style);
        assert_eq!(label1.prefix, label2.prefix);
        assert_eq!(label1.start, label2.start);
    }
}
