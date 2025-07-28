//! List rendering support for PDF documents
//!
//! This module provides ordered and unordered list functionality
//! with basic formatting options.

use crate::error::PdfError;
use crate::graphics::{Color, GraphicsContext};
use crate::text::Font;

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
        match self.style {
            OrderedListStyle::Decimal => format!("{number}."),
            OrderedListStyle::LowerAlpha => {
                let letter = char::from_u32('a' as u32 + (number - 1) % 26).unwrap_or('?');
                format!("{letter}.")
            }
            OrderedListStyle::UpperAlpha => {
                let letter = char::from_u32('A' as u32 + (number - 1) % 26).unwrap_or('?');
                format!("{letter}.")
            }
            OrderedListStyle::LowerRoman => format!("{}.", to_roman(number).to_lowercase()),
            OrderedListStyle::UpperRoman => format!("{}.", to_roman(number)),
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

        for (index, item) in self.items.iter().enumerate() {
            // Draw marker
            let marker = self.generate_marker(index);
            graphics.save_state();
            graphics.set_font(self.options.font, self.options.font_size);
            graphics.set_fill_color(self.options.text_color);
            graphics.begin_text();
            graphics.set_text_position(indent, y);
            graphics.show_text(&marker)?;
            graphics.end_text();
            graphics.restore_state();

            // Draw text
            let text_x =
                indent + self.calculate_marker_width(&marker) + self.options.marker_spacing;
            graphics.save_state();
            graphics.set_font(self.options.font, self.options.font_size);
            graphics.set_fill_color(self.options.text_color);
            graphics.begin_text();
            graphics.set_text_position(text_x, y);
            graphics.show_text(&item.text)?;
            graphics.end_text();
            graphics.restore_state();

            y += self.options.font_size * self.options.line_spacing;

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

        for item in &self.items {
            // Draw bullet
            graphics.save_state();
            graphics.set_font(self.options.font, self.options.font_size);
            graphics.set_fill_color(self.options.text_color);
            graphics.begin_text();
            graphics.set_text_position(indent, y);
            graphics.show_text(bullet)?;
            graphics.end_text();
            graphics.restore_state();

            // Draw text
            let text_x = indent + self.options.font_size + self.options.marker_spacing;
            graphics.save_state();
            graphics.set_font(self.options.font, self.options.font_size);
            graphics.set_fill_color(self.options.text_color);
            graphics.begin_text();
            graphics.set_text_position(text_x, y);
            graphics.show_text(&item.text)?;
            graphics.end_text();
            graphics.restore_state();

            y += self.options.font_size * self.options.line_spacing;

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
}
