//! List rendering support for PDF documents
//!
//! This module provides ordered and unordered list functionality
//! with advanced formatting options including:
//! - Multiple numbering styles (decimal, alphabetic, roman)
//! - Custom bullet styles and symbols
//! - Nested lists with automatic indentation
//! - Text wrapping for long items
//! - Custom spacing and alignment
//! - Rich formatting options

use crate::error::PdfError;
use crate::graphics::{Color, GraphicsContext};
use crate::text::{Font, TextAlign};

/// List style for ordered lists
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum OrderedListStyle {
    /// Arabic numerals (1, 2, 3, ...)
    Decimal,
    /// Lowercase letters (a, b, c, ...)
    LowerAlpha,
    /// Uppercase letters (A, B, C, ...)
    UpperAlpha,
    /// Lowercase Roman numerals (i, ii, iii, ...)
    LowerRoman,
    /// Uppercase Roman numerals (I, II, III, ...)
    UpperRoman,
    /// Decimal with leading zeros (01, 02, 03, ...)
    DecimalLeadingZero,
    /// Greek lowercase letters (α, β, γ, ...)
    GreekLower,
    /// Greek uppercase letters (Α, Β, Γ, ...)
    GreekUpper,
    /// Hebrew letters (א, ב, ג, ...)
    Hebrew,
    /// Japanese hiragana (あ, い, う, ...)
    Hiragana,
    /// Japanese katakana (ア, イ, ウ, ...)
    Katakana,
    /// Chinese numbers (一, 二, 三, ...)
    ChineseSimplified,
}

/// Bullet style for unordered lists
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BulletStyle {
    /// Filled circle (•)
    Disc,
    /// Empty circle (○)
    Circle,
    /// Filled square (■)
    Square,
    /// Dash (-)
    Dash,
    /// Custom character
    Custom(char),
}

/// Combined list style enum
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ListStyle {
    /// Ordered list with specific style
    Ordered(OrderedListStyle),
    /// Unordered list with specific bullet
    Unordered(BulletStyle),
}

/// Options for list rendering
#[derive(Debug, Clone)]
pub struct ListOptions {
    /// Font for list text
    pub font: Font,
    /// Font size in points
    pub font_size: f64,
    /// Text color
    pub text_color: Color,
    /// Indentation per level in points
    pub indent: f64,
    /// Line spacing multiplier
    pub line_spacing: f64,
    /// Space between bullet/number and text
    pub marker_spacing: f64,
    /// Maximum width for text wrapping (None = no wrapping)
    pub max_width: Option<f64>,
    /// Text alignment for wrapped lines
    pub text_align: TextAlign,
    /// Font for markers (bullets/numbers)
    pub marker_font: Font,
    /// Marker color (None = same as text)
    pub marker_color: Option<Color>,
    /// Paragraph spacing after each item
    pub paragraph_spacing: f64,
    /// Whether to draw a line after each item
    pub draw_separator: bool,
    /// Separator line color
    pub separator_color: Color,
    /// Separator line width
    pub separator_width: f64,
    /// Custom prefix before markers (e.g., "Chapter ", "Section ")
    pub marker_prefix: String,
    /// Custom suffix after markers (e.g., ")", "]", ":")
    pub marker_suffix: String,
}

impl Default for ListOptions {
    fn default() -> Self {
        Self {
            font: Font::Helvetica,
            font_size: 10.0,
            text_color: Color::black(),
            indent: 20.0,
            line_spacing: 1.2,
            marker_spacing: 10.0,
            max_width: None,
            text_align: TextAlign::Left,
            marker_font: Font::Helvetica,
            marker_color: None,
            paragraph_spacing: 0.0,
            draw_separator: false,
            separator_color: Color::gray(0.8),
            separator_width: 0.5,
            marker_prefix: String::new(),
            marker_suffix: ".".to_string(),
        }
    }
}

/// Represents an ordered list
#[derive(Debug, Clone)]
pub struct OrderedList {
    items: Vec<ListItem>,
    style: OrderedListStyle,
    start_number: u32,
    options: ListOptions,
    position: (f64, f64),
}

/// Represents an unordered list
#[derive(Debug, Clone)]
pub struct UnorderedList {
    items: Vec<ListItem>,
    bullet_style: BulletStyle,
    options: ListOptions,
    position: (f64, f64),
}

/// A single list item that can contain text and nested lists
#[derive(Debug, Clone)]
pub struct ListItem {
    text: String,
    children: Vec<ListElement>,
}

/// Element that can be in a list (for nested lists)
#[derive(Debug, Clone)]
pub enum ListElement {
    Ordered(OrderedList),
    Unordered(UnorderedList),
}

impl OrderedList {
    /// Create a new ordered list
    pub fn new(style: OrderedListStyle) -> Self {
        Self {
            items: Vec::new(),
            style,
            start_number: 1,
            options: ListOptions::default(),
            position: (0.0, 0.0),
        }
    }

    /// Set the starting number
    pub fn set_start_number(&mut self, start: u32) -> &mut Self {
        self.start_number = start;
        self
    }

    /// Set list options
    pub fn set_options(&mut self, options: ListOptions) -> &mut Self {
        self.options = options;
        self
    }

    /// Set list position
    pub fn set_position(&mut self, x: f64, y: f64) -> &mut Self {
        self.position = (x, y);
        self
    }

    /// Add a simple text item
    pub fn add_item(&mut self, text: String) -> &mut Self {
        self.items.push(ListItem {
            text,
            children: Vec::new(),
        });
        self
    }

    /// Add an item with nested lists
    pub fn add_item_with_children(
        &mut self,
        text: String,
        children: Vec<ListElement>,
    ) -> &mut Self {
        self.items.push(ListItem { text, children });
        self
    }

    /// Generate the marker for a given index
    fn generate_marker(&self, index: usize) -> String {
        let number = self.start_number + index as u32;
        let marker_core = match self.style {
            OrderedListStyle::Decimal => format!("{number}"),
            OrderedListStyle::DecimalLeadingZero => format!("{number:02}"),
            OrderedListStyle::LowerAlpha => {
                let letter = char::from_u32('a' as u32 + (number - 1) % 26).unwrap_or('?');
                format!("{letter}")
            }
            OrderedListStyle::UpperAlpha => {
                let letter = char::from_u32('A' as u32 + (number - 1) % 26).unwrap_or('?');
                format!("{letter}")
            }
            OrderedListStyle::LowerRoman => to_roman(number).to_lowercase(),
            OrderedListStyle::UpperRoman => to_roman(number),
            OrderedListStyle::GreekLower => get_greek_letter(number, false),
            OrderedListStyle::GreekUpper => get_greek_letter(number, true),
            OrderedListStyle::Hebrew => get_hebrew_letter(number),
            OrderedListStyle::Hiragana => get_hiragana_letter(number),
            OrderedListStyle::Katakana => get_katakana_letter(number),
            OrderedListStyle::ChineseSimplified => get_chinese_number(number),
        };

        format!(
            "{}{}{}",
            self.options.marker_prefix, marker_core, self.options.marker_suffix
        )
    }

    /// Calculate the total height of the list
    pub fn get_height(&self) -> f64 {
        self.calculate_height_recursive(0)
    }

    fn calculate_height_recursive(&self, _level: usize) -> f64 {
        let mut height = 0.0;
        for item in &self.items {
            height += self.options.font_size * self.options.line_spacing;
            for child in &item.children {
                height += match child {
                    ListElement::Ordered(list) => list.calculate_height_recursive(_level + 1),
                    ListElement::Unordered(list) => list.calculate_height_recursive(_level + 1),
                };
            }
        }
        height
    }

    /// Render the list to a graphics context
    pub fn render(&self, graphics: &mut GraphicsContext) -> Result<(), PdfError> {
        let (x, y) = self.position;
        self.render_recursive(graphics, x, y, 0)?;
        Ok(())
    }

    fn render_recursive(
        &self,
        graphics: &mut GraphicsContext,
        x: f64,
        mut y: f64,
        level: usize,
    ) -> Result<f64, PdfError> {
        let indent = x + (level as f64 * self.options.indent);

        for (index, item) in self.items.iter().enumerate() {
            // Draw marker
            let marker = self.generate_marker(index);
            graphics.save_state();
            graphics.set_font(self.options.marker_font, self.options.font_size);
            let marker_color = self.options.marker_color.unwrap_or(self.options.text_color);
            graphics.set_fill_color(marker_color);
            graphics.begin_text();
            graphics.set_text_position(indent, y);
            graphics.show_text(&marker)?;
            graphics.end_text();
            graphics.restore_state();

            // Draw text (with wrapping support)
            let text_x =
                indent + self.calculate_marker_width(&marker) + self.options.marker_spacing;

            let text_lines = if let Some(max_width) = self.options.max_width {
                let available_width = max_width - text_x;
                self.wrap_text(&item.text, available_width)
            } else {
                vec![item.text.clone()]
            };

            // Draw each line of text
            let mut line_y = y;
            for (line_index, line) in text_lines.iter().enumerate() {
                graphics.save_state();
                graphics.set_font(self.options.font, self.options.font_size);
                graphics.set_fill_color(self.options.text_color);
                graphics.begin_text();

                // For wrapped lines (not the first), add extra indent
                let line_x = if line_index == 0 {
                    text_x
                } else {
                    text_x + self.options.font_size // Additional indent for wrapped lines
                };

                graphics.set_text_position(line_x, line_y);
                graphics.show_text(line)?;
                graphics.end_text();
                graphics.restore_state();

                if line_index < text_lines.len() - 1 {
                    line_y += self.options.font_size * self.options.line_spacing;
                }
            }

            y = line_y;

            y +=
                self.options.font_size * self.options.line_spacing + self.options.paragraph_spacing;

            // Draw separator if enabled
            if self.options.draw_separator && index < self.items.len() - 1 {
                graphics.save_state();
                graphics.set_stroke_color(self.options.separator_color);
                graphics.set_line_width(self.options.separator_width);
                graphics.move_to(indent, y + 2.0);
                graphics.line_to(
                    indent + (self.options.max_width.unwrap_or(500.0) - indent),
                    y + 2.0,
                );
                graphics.stroke();
                graphics.restore_state();
                y += 5.0;
            }

            // Render children
            for child in &item.children {
                y = match child {
                    ListElement::Ordered(list) => {
                        let mut child_list = list.clone();
                        child_list.options = self.options.clone();
                        child_list.render_recursive(graphics, x, y, level + 1)?
                    }
                    ListElement::Unordered(list) => {
                        let mut child_list = list.clone();
                        child_list.options = self.options.clone();
                        child_list.render_recursive(graphics, x, y, level + 1)?
                    }
                };
            }
        }

        Ok(y)
    }

    fn calculate_marker_width(&self, marker: &str) -> f64 {
        // Simple approximation: average character width * marker length
        marker.len() as f64 * self.options.font_size * 0.5
    }

    /// Wrap text to fit within the given width
    fn wrap_text(&self, text: &str, max_width: f64) -> Vec<String> {
        // Simple character-based wrapping
        // In a real implementation, this would use proper font metrics
        let avg_char_width = self.options.font_size * 0.5;
        let chars_per_line = (max_width / avg_char_width) as usize;

        if chars_per_line == 0 || text.len() <= chars_per_line {
            return vec![text.to_string()];
        }

        let mut lines = Vec::new();
        let words: Vec<&str> = text.split_whitespace().collect();
        let mut current_line = String::new();

        for word in words {
            let test_line = if current_line.is_empty() {
                word.to_string()
            } else {
                format!("{} {}", current_line, word)
            };

            if test_line.len() <= chars_per_line {
                current_line = test_line;
            } else {
                if !current_line.is_empty() {
                    lines.push(current_line);
                }
                current_line = word.to_string();
            }
        }

        if !current_line.is_empty() {
            lines.push(current_line);
        }

        lines
    }
}

impl UnorderedList {
    /// Create a new unordered list
    pub fn new(bullet_style: BulletStyle) -> Self {
        Self {
            items: Vec::new(),
            bullet_style,
            options: ListOptions::default(),
            position: (0.0, 0.0),
        }
    }

    /// Set list options
    pub fn set_options(&mut self, options: ListOptions) -> &mut Self {
        self.options = options;
        self
    }

    /// Set list position
    pub fn set_position(&mut self, x: f64, y: f64) -> &mut Self {
        self.position = (x, y);
        self
    }

    /// Add a simple text item
    pub fn add_item(&mut self, text: String) -> &mut Self {
        self.items.push(ListItem {
            text,
            children: Vec::new(),
        });
        self
    }

    /// Add an item with nested lists
    pub fn add_item_with_children(
        &mut self,
        text: String,
        children: Vec<ListElement>,
    ) -> &mut Self {
        self.items.push(ListItem { text, children });
        self
    }

    /// Get the bullet character
    fn get_bullet_char(&self) -> &str {
        match self.bullet_style {
            BulletStyle::Disc => "•",
            BulletStyle::Circle => "○",
            BulletStyle::Square => "■",
            BulletStyle::Dash => "-",
            BulletStyle::Custom(ch) => {
                // This is a bit hacky but works for single characters
                match ch {
                    '→' => "→",
                    '▸' => "▸",
                    '▹' => "▹",
                    '★' => "★",
                    '☆' => "☆",
                    _ => "•", // Fallback
                }
            }
        }
    }

    /// Calculate the total height of the list
    pub fn get_height(&self) -> f64 {
        self.calculate_height_recursive(0)
    }

    fn calculate_height_recursive(&self, _level: usize) -> f64 {
        let mut height = 0.0;
        for item in &self.items {
            height += self.options.font_size * self.options.line_spacing;
            for child in &item.children {
                height += match child {
                    ListElement::Ordered(list) => list.calculate_height_recursive(_level + 1),
                    ListElement::Unordered(list) => list.calculate_height_recursive(_level + 1),
                };
            }
        }
        height
    }

    /// Render the list to a graphics context
    pub fn render(&self, graphics: &mut GraphicsContext) -> Result<(), PdfError> {
        let (x, y) = self.position;
        self.render_recursive(graphics, x, y, 0)?;
        Ok(())
    }

    fn render_recursive(
        &self,
        graphics: &mut GraphicsContext,
        x: f64,
        mut y: f64,
        level: usize,
    ) -> Result<f64, PdfError> {
        let indent = x + (level as f64 * self.options.indent);
        let bullet = self.get_bullet_char();

        for (index, item) in self.items.iter().enumerate() {
            // Draw bullet
            graphics.save_state();
            graphics.set_font(self.options.marker_font, self.options.font_size);
            let marker_color = self.options.marker_color.unwrap_or(self.options.text_color);
            graphics.set_fill_color(marker_color);
            graphics.begin_text();
            graphics.set_text_position(indent, y);
            graphics.show_text(bullet)?;
            graphics.end_text();
            graphics.restore_state();

            // Draw text (with wrapping support)
            let text_x = indent + self.options.font_size + self.options.marker_spacing;

            let text_lines = if let Some(max_width) = self.options.max_width {
                let available_width = max_width - text_x;
                self.wrap_text(&item.text, available_width)
            } else {
                vec![item.text.clone()]
            };

            // Draw each line of text
            let mut line_y = y;
            for (line_index, line) in text_lines.iter().enumerate() {
                graphics.save_state();
                graphics.set_font(self.options.font, self.options.font_size);
                graphics.set_fill_color(self.options.text_color);
                graphics.begin_text();

                // For wrapped lines (not the first), add extra indent
                let line_x = if line_index == 0 {
                    text_x
                } else {
                    text_x + self.options.font_size // Additional indent for wrapped lines
                };

                graphics.set_text_position(line_x, line_y);
                graphics.show_text(line)?;
                graphics.end_text();
                graphics.restore_state();

                if line_index < text_lines.len() - 1 {
                    line_y += self.options.font_size * self.options.line_spacing;
                }
            }

            y = line_y;

            y +=
                self.options.font_size * self.options.line_spacing + self.options.paragraph_spacing;

            // Draw separator if enabled
            if self.options.draw_separator && (index < self.items.len() - 1) {
                graphics.save_state();
                graphics.set_stroke_color(self.options.separator_color);
                graphics.set_line_width(self.options.separator_width);
                graphics.move_to(indent, y + 2.0);
                graphics.line_to(
                    indent + (self.options.max_width.unwrap_or(500.0) - indent),
                    y + 2.0,
                );
                graphics.stroke();
                graphics.restore_state();
                y += 5.0;
            }

            // Render children
            for child in &item.children {
                y = match child {
                    ListElement::Ordered(list) => {
                        let mut child_list = list.clone();
                        child_list.options = self.options.clone();
                        child_list.render_recursive(graphics, x, y, level + 1)?
                    }
                    ListElement::Unordered(list) => {
                        let mut child_list = list.clone();
                        child_list.options = self.options.clone();
                        child_list.render_recursive(graphics, x, y, level + 1)?
                    }
                };
            }
        }

        Ok(y)
    }

    /// Wrap text to fit within the given width
    fn wrap_text(&self, text: &str, max_width: f64) -> Vec<String> {
        // Simple character-based wrapping
        // In a real implementation, this would use proper font metrics
        let avg_char_width = self.options.font_size * 0.5;
        let chars_per_line = (max_width / avg_char_width) as usize;

        if chars_per_line == 0 || text.len() <= chars_per_line {
            return vec![text.to_string()];
        }

        let mut lines = Vec::new();
        let words: Vec<&str> = text.split_whitespace().collect();
        let mut current_line = String::new();

        for word in words {
            let test_line = if current_line.is_empty() {
                word.to_string()
            } else {
                format!("{} {}", current_line, word)
            };

            if test_line.len() <= chars_per_line {
                current_line = test_line;
            } else {
                if !current_line.is_empty() {
                    lines.push(current_line);
                }
                current_line = word.to_string();
            }
        }

        if !current_line.is_empty() {
            lines.push(current_line);
        }

        lines
    }
}

/// Convert a number to Roman numerals
fn to_roman(num: u32) -> String {
    let values = [
        (1000, "M"),
        (900, "CM"),
        (500, "D"),
        (400, "CD"),
        (100, "C"),
        (90, "XC"),
        (50, "L"),
        (40, "XL"),
        (10, "X"),
        (9, "IX"),
        (5, "V"),
        (4, "IV"),
        (1, "I"),
    ];

    let mut result = String::new();
    let mut n = num;

    for (value, numeral) in &values {
        while n >= *value {
            result.push_str(numeral);
            n -= value;
        }
    }

    result
}

/// Get Greek letter for a given number
fn get_greek_letter(num: u32, uppercase: bool) -> String {
    let lower = [
        "α", "β", "γ", "δ", "ε", "ζ", "η", "θ", "ι", "κ", "λ", "μ", "ν", "ξ", "ο", "π", "ρ", "σ",
        "τ", "υ", "φ", "χ", "ψ", "ω",
    ];
    let upper = [
        "Α", "Β", "Γ", "Δ", "Ε", "Ζ", "Η", "Θ", "Ι", "Κ", "Λ", "Μ", "Ν", "Ξ", "Ο", "Π", "Ρ", "Σ",
        "Τ", "Υ", "Φ", "Χ", "Ψ", "Ω",
    ];

    let index = ((num - 1) % 24) as usize;
    if uppercase {
        upper[index].to_string()
    } else {
        lower[index].to_string()
    }
}

/// Get Hebrew letter for a given number
fn get_hebrew_letter(num: u32) -> String {
    let letters = [
        "א", "ב", "ג", "ד", "ה", "ו", "ז", "ח", "ט", "י", "כ", "ל", "מ", "נ", "ס", "ע", "פ", "צ",
        "ק", "ר", "ש", "ת",
    ];
    let index = ((num - 1) % 22) as usize;
    letters[index].to_string()
}

/// Get Hiragana letter for a given number
fn get_hiragana_letter(num: u32) -> String {
    let letters = [
        "あ", "い", "う", "え", "お", "か", "き", "く", "け", "こ", "さ", "し", "す", "せ", "そ",
        "た", "ち", "つ", "て", "と", "な", "に", "ぬ", "ね", "の", "は", "ひ", "ふ", "へ", "ほ",
        "ま", "み", "む", "め", "も", "や", "ゆ", "よ", "ら", "り", "る", "れ", "ろ", "わ", "を",
        "ん",
    ];
    let index = ((num - 1) % 46) as usize;
    letters[index].to_string()
}

/// Get Katakana letter for a given number
fn get_katakana_letter(num: u32) -> String {
    let letters = [
        "ア", "イ", "ウ", "エ", "オ", "カ", "キ", "ク", "ケ", "コ", "サ", "シ", "ス", "セ", "ソ",
        "タ", "チ", "ツ", "テ", "ト", "ナ", "ニ", "ヌ", "ネ", "ノ", "ハ", "ヒ", "フ", "ヘ", "ホ",
        "マ", "ミ", "ム", "メ", "モ", "ヤ", "ユ", "ヨ", "ラ", "リ", "ル", "レ", "ロ", "ワ", "ヲ",
        "ン",
    ];
    let index = ((num - 1) % 46) as usize;
    letters[index].to_string()
}

/// Get Chinese simplified number for a given number
fn get_chinese_number(num: u32) -> String {
    let numbers = [
        "一", "二", "三", "四", "五", "六", "七", "八", "九", "十", "十一", "十二", "十三", "十四",
        "十五", "十六", "十七", "十八", "十九", "二十",
    ];
    if num <= 20 {
        numbers[(num - 1) as usize].to_string()
    } else {
        format!("{}", num) // Fallback to Arabic for larger numbers
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ordered_list_creation() {
        let list = OrderedList::new(OrderedListStyle::Decimal);
        assert_eq!(list.style, OrderedListStyle::Decimal);
        assert_eq!(list.start_number, 1);
        assert!(list.items.is_empty());
    }

    #[test]
    fn test_unordered_list_creation() {
        let list = UnorderedList::new(BulletStyle::Disc);
        assert_eq!(list.bullet_style, BulletStyle::Disc);
        assert!(list.items.is_empty());
    }

    #[test]
    fn test_add_items() {
        let mut list = OrderedList::new(OrderedListStyle::Decimal);
        list.add_item("First item".to_string())
            .add_item("Second item".to_string());
        assert_eq!(list.items.len(), 2);
        assert_eq!(list.items[0].text, "First item");
        assert_eq!(list.items[1].text, "Second item");
    }

    #[test]
    fn test_marker_generation_decimal() {
        let list = OrderedList::new(OrderedListStyle::Decimal);
        assert_eq!(list.generate_marker(0), "1.");
        assert_eq!(list.generate_marker(1), "2.");
        assert_eq!(list.generate_marker(9), "10.");
    }

    #[test]
    fn test_marker_generation_lower_alpha() {
        let list = OrderedList::new(OrderedListStyle::LowerAlpha);
        assert_eq!(list.generate_marker(0), "a.");
        assert_eq!(list.generate_marker(1), "b.");
        assert_eq!(list.generate_marker(25), "z.");
    }

    #[test]
    fn test_marker_generation_upper_alpha() {
        let list = OrderedList::new(OrderedListStyle::UpperAlpha);
        assert_eq!(list.generate_marker(0), "A.");
        assert_eq!(list.generate_marker(1), "B.");
        assert_eq!(list.generate_marker(25), "Z.");
    }

    #[test]
    fn test_marker_generation_roman() {
        let list = OrderedList::new(OrderedListStyle::LowerRoman);
        assert_eq!(list.generate_marker(0), "i.");
        assert_eq!(list.generate_marker(3), "iv.");
        assert_eq!(list.generate_marker(8), "ix.");

        let list_upper = OrderedList::new(OrderedListStyle::UpperRoman);
        assert_eq!(list_upper.generate_marker(0), "I.");
        assert_eq!(list_upper.generate_marker(3), "IV.");
        assert_eq!(list_upper.generate_marker(8), "IX.");
    }

    #[test]
    fn test_start_number() {
        let mut list = OrderedList::new(OrderedListStyle::Decimal);
        list.set_start_number(5);
        assert_eq!(list.generate_marker(0), "5.");
        assert_eq!(list.generate_marker(1), "6.");
    }

    #[test]
    fn test_bullet_styles() {
        let disc = UnorderedList::new(BulletStyle::Disc);
        assert_eq!(disc.get_bullet_char(), "•");

        let circle = UnorderedList::new(BulletStyle::Circle);
        assert_eq!(circle.get_bullet_char(), "○");

        let square = UnorderedList::new(BulletStyle::Square);
        assert_eq!(square.get_bullet_char(), "■");

        let dash = UnorderedList::new(BulletStyle::Dash);
        assert_eq!(dash.get_bullet_char(), "-");
    }

    #[test]
    fn test_custom_bullet() {
        let arrow = UnorderedList::new(BulletStyle::Custom('→'));
        assert_eq!(arrow.get_bullet_char(), "→");

        let star = UnorderedList::new(BulletStyle::Custom('★'));
        assert_eq!(star.get_bullet_char(), "★");
    }

    #[test]
    fn test_list_options_default() {
        let options = ListOptions::default();
        assert_eq!(options.font_size, 10.0);
        assert_eq!(options.indent, 20.0);
        assert_eq!(options.line_spacing, 1.2);
        assert_eq!(options.marker_spacing, 10.0);
    }

    #[test]
    fn test_roman_numerals() {
        assert_eq!(to_roman(1), "I");
        assert_eq!(to_roman(4), "IV");
        assert_eq!(to_roman(5), "V");
        assert_eq!(to_roman(9), "IX");
        assert_eq!(to_roman(10), "X");
        assert_eq!(to_roman(40), "XL");
        assert_eq!(to_roman(50), "L");
        assert_eq!(to_roman(90), "XC");
        assert_eq!(to_roman(100), "C");
        assert_eq!(to_roman(400), "CD");
        assert_eq!(to_roman(500), "D");
        assert_eq!(to_roman(900), "CM");
        assert_eq!(to_roman(1000), "M");
        assert_eq!(to_roman(1994), "MCMXCIV");
    }

    #[test]
    fn test_nested_lists() {
        let mut parent = OrderedList::new(OrderedListStyle::Decimal);

        let mut child = UnorderedList::new(BulletStyle::Dash);
        child.add_item("Nested item 1".to_string());
        child.add_item("Nested item 2".to_string());

        parent.add_item_with_children(
            "Parent item".to_string(),
            vec![ListElement::Unordered(child)],
        );

        assert_eq!(parent.items.len(), 1);
        assert_eq!(parent.items[0].children.len(), 1);
    }

    #[test]
    fn test_list_position() {
        let mut list = OrderedList::new(OrderedListStyle::Decimal);
        list.set_position(100.0, 200.0);
        assert_eq!(list.position, (100.0, 200.0));
    }

    #[test]
    fn test_list_height_calculation() {
        let mut list = OrderedList::new(OrderedListStyle::Decimal);
        list.add_item("Item 1".to_string())
            .add_item("Item 2".to_string())
            .add_item("Item 3".to_string());

        let height = list.get_height();
        let expected = 3.0 * list.options.font_size * list.options.line_spacing;
        assert_eq!(height, expected);
    }

    #[test]
    fn test_list_item_structure() {
        let item = ListItem {
            text: "Test item".to_string(),
            children: vec![],
        };
        assert_eq!(item.text, "Test item");
        assert!(item.children.is_empty());
    }

    #[test]
    fn test_list_element_enum() {
        let ordered = OrderedList::new(OrderedListStyle::Decimal);
        let unordered = UnorderedList::new(BulletStyle::Disc);

        let elements = vec![
            ListElement::Ordered(ordered),
            ListElement::Unordered(unordered),
        ];

        assert_eq!(elements.len(), 2);
        match &elements[0] {
            ListElement::Ordered(_) => (),
            _ => panic!("Expected ordered list"),
        }
        match &elements[1] {
            ListElement::Unordered(_) => (),
            _ => panic!("Expected unordered list"),
        }
    }

    #[test]
    fn test_advanced_numbering_styles() {
        // Test decimal with leading zeros
        let list = OrderedList::new(OrderedListStyle::DecimalLeadingZero);
        assert_eq!(list.generate_marker(0), "01.");
        assert_eq!(list.generate_marker(8), "09.");
        assert_eq!(list.generate_marker(9), "10.");
        assert_eq!(list.generate_marker(99), "100.");

        // Test Greek lowercase
        let greek_lower = OrderedList::new(OrderedListStyle::GreekLower);
        assert_eq!(greek_lower.generate_marker(0), "α.");
        assert_eq!(greek_lower.generate_marker(1), "β.");
        assert_eq!(greek_lower.generate_marker(23), "ω.");
        assert_eq!(greek_lower.generate_marker(24), "α."); // Wraps around

        // Test Greek uppercase
        let greek_upper = OrderedList::new(OrderedListStyle::GreekUpper);
        assert_eq!(greek_upper.generate_marker(0), "Α.");
        assert_eq!(greek_upper.generate_marker(1), "Β.");
        assert_eq!(greek_upper.generate_marker(23), "Ω.");

        // Test Hebrew
        let hebrew = OrderedList::new(OrderedListStyle::Hebrew);
        assert_eq!(hebrew.generate_marker(0), "א.");
        assert_eq!(hebrew.generate_marker(1), "ב.");
        assert_eq!(hebrew.generate_marker(21), "ת.");

        // Test Hiragana
        let hiragana = OrderedList::new(OrderedListStyle::Hiragana);
        assert_eq!(hiragana.generate_marker(0), "あ.");
        assert_eq!(hiragana.generate_marker(1), "い.");
        assert_eq!(hiragana.generate_marker(4), "お.");

        // Test Katakana
        let katakana = OrderedList::new(OrderedListStyle::Katakana);
        assert_eq!(katakana.generate_marker(0), "ア.");
        assert_eq!(katakana.generate_marker(1), "イ.");
        assert_eq!(katakana.generate_marker(4), "オ.");

        // Test Chinese simplified
        let chinese = OrderedList::new(OrderedListStyle::ChineseSimplified);
        assert_eq!(chinese.generate_marker(0), "一.");
        assert_eq!(chinese.generate_marker(1), "二.");
        assert_eq!(chinese.generate_marker(9), "十.");
        assert_eq!(chinese.generate_marker(19), "二十.");
        assert_eq!(chinese.generate_marker(20), "21."); // Fallback to Arabic
    }

    #[test]
    fn test_custom_prefix_suffix() {
        let mut list = OrderedList::new(OrderedListStyle::Decimal);
        let mut options = ListOptions::default();
        options.marker_prefix = "Chapter ".to_string();
        options.marker_suffix = ":".to_string();
        list.set_options(options);

        assert_eq!(list.generate_marker(0), "Chapter 1:");
        assert_eq!(list.generate_marker(1), "Chapter 2:");

        // Test with Roman numerals
        let mut roman_list = OrderedList::new(OrderedListStyle::UpperRoman);
        let mut roman_options = ListOptions::default();
        roman_options.marker_prefix = "Part ".to_string();
        roman_options.marker_suffix = " -".to_string();
        roman_list.set_options(roman_options);

        assert_eq!(roman_list.generate_marker(0), "Part I -");
        assert_eq!(roman_list.generate_marker(3), "Part IV -");
    }

    #[test]
    fn test_text_wrapping() {
        let list = OrderedList::new(OrderedListStyle::Decimal);

        // Test short text (no wrapping)
        let wrapped = list.wrap_text("Short text", 100.0);
        assert_eq!(wrapped.len(), 1);
        assert_eq!(wrapped[0], "Short text");

        // Test long text with wrapping
        let long_text =
            "This is a very long line that should be wrapped because it exceeds the maximum width";
        let wrapped = list.wrap_text(long_text, 50.0); // ~10 chars per line at font size 10
        assert!(wrapped.len() > 1);

        // Test empty text
        let wrapped = list.wrap_text("", 100.0);
        assert_eq!(wrapped.len(), 1);
        assert_eq!(wrapped[0], "");

        // Test zero width (no wrapping possible)
        let wrapped = list.wrap_text("Test", 0.0);
        assert_eq!(wrapped.len(), 1);
        assert_eq!(wrapped[0], "Test");
    }

    #[test]
    fn test_list_options_advanced() {
        let mut options = ListOptions::default();

        // Test all new fields
        options.max_width = Some(300.0);
        options.text_align = TextAlign::Center;
        options.marker_font = Font::HelveticaBold;
        options.marker_color = Some(Color::red());
        options.paragraph_spacing = 5.0;
        options.draw_separator = true;
        options.separator_color = Color::gray(0.5);
        options.separator_width = 2.0;
        options.marker_prefix = "Item ".to_string();
        options.marker_suffix = ")".to_string();

        assert_eq!(options.max_width, Some(300.0));
        assert_eq!(options.text_align, TextAlign::Center);
        assert_eq!(options.marker_font, Font::HelveticaBold);
        assert!(options.marker_color.is_some());
        assert_eq!(options.paragraph_spacing, 5.0);
        assert!(options.draw_separator);
        assert_eq!(options.separator_width, 2.0);
        assert_eq!(options.marker_prefix, "Item ");
        assert_eq!(options.marker_suffix, ")");
    }

    #[test]
    fn test_unordered_list_custom_bullets() {
        // Test standard bullets
        let disc_list = UnorderedList::new(BulletStyle::Disc);
        assert_eq!(disc_list.get_bullet_char(), "•");

        let circle_list = UnorderedList::new(BulletStyle::Circle);
        assert_eq!(circle_list.get_bullet_char(), "○");

        let square_list = UnorderedList::new(BulletStyle::Square);
        assert_eq!(square_list.get_bullet_char(), "■");

        let dash_list = UnorderedList::new(BulletStyle::Dash);
        assert_eq!(dash_list.get_bullet_char(), "-");

        // Test custom bullets
        let arrow_list = UnorderedList::new(BulletStyle::Custom('→'));
        assert_eq!(arrow_list.get_bullet_char(), "→");

        let star_list = UnorderedList::new(BulletStyle::Custom('★'));
        assert_eq!(star_list.get_bullet_char(), "★");

        // Test fallback for unknown custom character
        let unknown_list = UnorderedList::new(BulletStyle::Custom('Z'));
        assert_eq!(unknown_list.get_bullet_char(), "•"); // Falls back to disc
    }

    #[test]
    fn test_deeply_nested_lists() {
        let mut level1 = OrderedList::new(OrderedListStyle::Decimal);

        // Create level 2
        let mut level2 = UnorderedList::new(BulletStyle::Circle);

        // Create level 3
        let mut level3 = OrderedList::new(OrderedListStyle::LowerAlpha);
        level3.add_item("Deep item a".to_string());
        level3.add_item("Deep item b".to_string());

        // Add level 3 to level 2
        level2.add_item_with_children(
            "Level 2 item with children".to_string(),
            vec![ListElement::Ordered(level3)],
        );
        level2.add_item("Level 2 item without children".to_string());

        // Add level 2 to level 1
        level1.add_item_with_children(
            "Level 1 item with nested list".to_string(),
            vec![ListElement::Unordered(level2)],
        );

        assert_eq!(level1.items.len(), 1);
        assert_eq!(level1.items[0].children.len(), 1);

        // Verify the structure
        if let ListElement::Unordered(ref list) = level1.items[0].children[0] {
            assert_eq!(list.items.len(), 2);
            assert_eq!(list.items[0].children.len(), 1);
        } else {
            panic!("Expected unordered list at level 2");
        }
    }

    #[test]
    fn test_height_calculation_with_nested() {
        let mut list = OrderedList::new(OrderedListStyle::Decimal);
        list.add_item("Item 1".to_string());
        list.add_item("Item 2".to_string());

        let height_simple = list.get_height();
        let expected_simple = 2.0 * 10.0 * 1.2; // 2 items * font_size * line_spacing
        assert_eq!(height_simple, expected_simple);

        // Add nested list
        let mut nested = UnorderedList::new(BulletStyle::Dash);
        nested.add_item("Nested 1".to_string());
        nested.add_item("Nested 2".to_string());

        list.add_item_with_children(
            "Item 3 with children".to_string(),
            vec![ListElement::Unordered(nested)],
        );

        let height_with_nested = list.get_height();
        let expected_with_nested = 5.0 * 10.0 * 1.2; // 5 total items * font_size * line_spacing
        assert_eq!(height_with_nested, expected_with_nested);
    }

    #[test]
    fn test_helper_functions() {
        // Test Greek letter generation
        assert_eq!(get_greek_letter(1, false), "α");
        assert_eq!(get_greek_letter(2, false), "β");
        assert_eq!(get_greek_letter(24, false), "ω");
        assert_eq!(get_greek_letter(25, false), "α"); // Wraps

        assert_eq!(get_greek_letter(1, true), "Α");
        assert_eq!(get_greek_letter(2, true), "Β");
        assert_eq!(get_greek_letter(24, true), "Ω");

        // Test Hebrew letter generation
        assert_eq!(get_hebrew_letter(1), "א");
        assert_eq!(get_hebrew_letter(22), "ת");
        assert_eq!(get_hebrew_letter(23), "א"); // Wraps

        // Test Hiragana generation
        assert_eq!(get_hiragana_letter(1), "あ");
        assert_eq!(get_hiragana_letter(5), "お");
        assert_eq!(get_hiragana_letter(46), "ん");
        assert_eq!(get_hiragana_letter(47), "あ"); // Wraps

        // Test Katakana generation
        assert_eq!(get_katakana_letter(1), "ア");
        assert_eq!(get_katakana_letter(5), "オ");
        assert_eq!(get_katakana_letter(46), "ン");

        // Test Chinese number generation
        assert_eq!(get_chinese_number(1), "一");
        assert_eq!(get_chinese_number(10), "十");
        assert_eq!(get_chinese_number(20), "二十");
        assert_eq!(get_chinese_number(21), "21"); // Fallback
    }

    #[test]
    fn test_list_cloning() {
        let mut original = OrderedList::new(OrderedListStyle::Decimal);
        original.add_item("Item 1".to_string());
        original.set_position(100.0, 200.0);
        original.set_start_number(5);

        let cloned = original.clone();
        assert_eq!(cloned.items.len(), 1);
        assert_eq!(cloned.position, (100.0, 200.0));
        assert_eq!(cloned.start_number, 5);
        assert_eq!(cloned.style, OrderedListStyle::Decimal);
    }
}
