//! Page extension for list rendering
//!
//! This module provides traits and implementations to easily add lists to PDF pages.

use crate::error::PdfError;
use crate::graphics::Color;
use crate::page::Page;
use crate::text::{BulletStyle, Font, ListOptions, OrderedList, OrderedListStyle, UnorderedList};

/// Extension trait for adding lists to pages
pub trait PageLists {
    /// Add an ordered list to the page
    fn add_ordered_list(
        &mut self,
        list: &OrderedList,
        x: f64,
        y: f64,
    ) -> Result<&mut Self, PdfError>;

    /// Add an unordered list to the page
    fn add_unordered_list(
        &mut self,
        list: &UnorderedList,
        x: f64,
        y: f64,
    ) -> Result<&mut Self, PdfError>;

    /// Create and add a quick ordered list with default styling
    fn add_quick_ordered_list(
        &mut self,
        items: Vec<String>,
        x: f64,
        y: f64,
        style: OrderedListStyle,
    ) -> Result<&mut Self, PdfError>;

    /// Create and add a quick unordered list with default styling
    fn add_quick_unordered_list(
        &mut self,
        items: Vec<String>,
        x: f64,
        y: f64,
        bullet: BulletStyle,
    ) -> Result<&mut Self, PdfError>;

    /// Create and add a styled ordered list
    fn add_styled_ordered_list(
        &mut self,
        items: Vec<String>,
        x: f64,
        y: f64,
        style: ListStyle,
    ) -> Result<&mut Self, PdfError>;

    /// Create and add a styled unordered list
    fn add_styled_unordered_list(
        &mut self,
        items: Vec<String>,
        x: f64,
        y: f64,
        style: ListStyle,
    ) -> Result<&mut Self, PdfError>;
}

/// Predefined list styles
#[derive(Debug, Clone)]
pub struct ListStyle {
    /// List type
    pub list_type: ListType,
    /// Font for text
    pub font: Font,
    /// Font size
    pub font_size: f64,
    /// Text color
    pub text_color: Color,
    /// Marker color (None = same as text)
    pub marker_color: Option<Color>,
    /// Maximum width for text wrapping
    pub max_width: Option<f64>,
    /// Line spacing multiplier
    pub line_spacing: f64,
    /// Indentation per level
    pub indent: f64,
    /// Paragraph spacing after items
    pub paragraph_spacing: f64,
    /// Whether to draw separators
    pub draw_separator: bool,
}

/// List type for styling
#[derive(Debug, Clone, Copy)]
pub enum ListType {
    /// Ordered list with specific style
    Ordered(OrderedListStyle),
    /// Unordered list with specific bullet
    Unordered(BulletStyle),
}

impl ListStyle {
    /// Create a minimal list style
    pub fn minimal(list_type: ListType) -> Self {
        Self {
            list_type,
            font: Font::Helvetica,
            font_size: 10.0,
            text_color: Color::black(),
            marker_color: None,
            max_width: None,
            line_spacing: 1.2,
            indent: 20.0,
            paragraph_spacing: 0.0,
            draw_separator: false,
        }
    }

    /// Create a professional list style
    pub fn professional(list_type: ListType) -> Self {
        Self {
            list_type,
            font: Font::Helvetica,
            font_size: 11.0,
            text_color: Color::gray(0.1),
            marker_color: Some(Color::rgb(0.2, 0.4, 0.7)),
            max_width: Some(500.0),
            line_spacing: 1.3,
            indent: 25.0,
            paragraph_spacing: 3.0,
            draw_separator: false,
        }
    }

    /// Create a document list style (for formal documents)
    pub fn document(list_type: ListType) -> Self {
        Self {
            list_type,
            font: Font::TimesRoman,
            font_size: 12.0,
            text_color: Color::black(),
            marker_color: None,
            max_width: Some(450.0),
            line_spacing: 1.5,
            indent: 30.0,
            paragraph_spacing: 5.0,
            draw_separator: false,
        }
    }

    /// Create a presentation list style
    pub fn presentation(list_type: ListType) -> Self {
        Self {
            list_type,
            font: Font::HelveticaBold,
            font_size: 14.0,
            text_color: Color::gray(0.2),
            marker_color: Some(Color::rgb(0.8, 0.2, 0.2)),
            max_width: Some(600.0),
            line_spacing: 1.6,
            indent: 35.0,
            paragraph_spacing: 8.0,
            draw_separator: false,
        }
    }

    /// Create a checklist style (with checkboxes)
    pub fn checklist() -> Self {
        Self {
            list_type: ListType::Unordered(BulletStyle::Square),
            font: Font::Helvetica,
            font_size: 11.0,
            text_color: Color::gray(0.1),
            marker_color: Some(Color::gray(0.4)),
            max_width: Some(500.0),
            line_spacing: 1.4,
            indent: 25.0,
            paragraph_spacing: 5.0,
            draw_separator: true,
        }
    }
}

impl PageLists for Page {
    fn add_ordered_list(
        &mut self,
        list: &OrderedList,
        x: f64,
        y: f64,
    ) -> Result<&mut Self, PdfError> {
        let mut list_clone = list.clone();
        list_clone.set_position(x, y);
        list_clone.render(self.graphics())?;
        Ok(self)
    }

    fn add_unordered_list(
        &mut self,
        list: &UnorderedList,
        x: f64,
        y: f64,
    ) -> Result<&mut Self, PdfError> {
        let mut list_clone = list.clone();
        list_clone.set_position(x, y);
        list_clone.render(self.graphics())?;
        Ok(self)
    }

    fn add_quick_ordered_list(
        &mut self,
        items: Vec<String>,
        x: f64,
        y: f64,
        style: OrderedListStyle,
    ) -> Result<&mut Self, PdfError> {
        let mut list = OrderedList::new(style);
        for item in items {
            list.add_item(item);
        }
        self.add_ordered_list(&list, x, y)
    }

    fn add_quick_unordered_list(
        &mut self,
        items: Vec<String>,
        x: f64,
        y: f64,
        bullet: BulletStyle,
    ) -> Result<&mut Self, PdfError> {
        let mut list = UnorderedList::new(bullet);
        for item in items {
            list.add_item(item);
        }
        self.add_unordered_list(&list, x, y)
    }

    fn add_styled_ordered_list(
        &mut self,
        items: Vec<String>,
        x: f64,
        y: f64,
        style: ListStyle,
    ) -> Result<&mut Self, PdfError> {
        if let ListType::Ordered(ordered_style) = style.list_type {
            let mut list = OrderedList::new(ordered_style);

            // Apply style options
            let options = ListOptions {
                font: style.font,
                font_size: style.font_size,
                text_color: style.text_color,
                marker_color: style.marker_color,
                max_width: style.max_width,
                line_spacing: style.line_spacing,
                indent: style.indent,
                paragraph_spacing: style.paragraph_spacing,
                draw_separator: style.draw_separator,
                ..Default::default()
            };

            list.set_options(options);

            for item in items {
                list.add_item(item);
            }

            self.add_ordered_list(&list, x, y)
        } else {
            Err(PdfError::InvalidFormat(
                "Expected ordered list style".to_string(),
            ))
        }
    }

    fn add_styled_unordered_list(
        &mut self,
        items: Vec<String>,
        x: f64,
        y: f64,
        style: ListStyle,
    ) -> Result<&mut Self, PdfError> {
        if let ListType::Unordered(bullet_style) = style.list_type {
            let mut list = UnorderedList::new(bullet_style);

            // Apply style options
            let options = ListOptions {
                font: style.font,
                font_size: style.font_size,
                text_color: style.text_color,
                marker_color: style.marker_color,
                max_width: style.max_width,
                line_spacing: style.line_spacing,
                indent: style.indent,
                paragraph_spacing: style.paragraph_spacing,
                draw_separator: style.draw_separator,
                ..Default::default()
            };

            list.set_options(options);

            for item in items {
                list.add_item(item);
            }

            self.add_unordered_list(&list, x, y)
        } else {
            Err(PdfError::InvalidFormat(
                "Expected unordered list style".to_string(),
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ==================== ListType Tests ====================

    #[test]
    fn test_list_type_ordered_variants() {
        let decimal = ListType::Ordered(OrderedListStyle::Decimal);
        let upper_alpha = ListType::Ordered(OrderedListStyle::UpperAlpha);
        let lower_alpha = ListType::Ordered(OrderedListStyle::LowerAlpha);
        let upper_roman = ListType::Ordered(OrderedListStyle::UpperRoman);
        let lower_roman = ListType::Ordered(OrderedListStyle::LowerRoman);

        // Verify they are distinct
        if let ListType::Ordered(style) = decimal {
            assert_eq!(style, OrderedListStyle::Decimal);
        }
        if let ListType::Ordered(style) = upper_alpha {
            assert_eq!(style, OrderedListStyle::UpperAlpha);
        }
        if let ListType::Ordered(style) = lower_alpha {
            assert_eq!(style, OrderedListStyle::LowerAlpha);
        }
        if let ListType::Ordered(style) = upper_roman {
            assert_eq!(style, OrderedListStyle::UpperRoman);
        }
        if let ListType::Ordered(style) = lower_roman {
            assert_eq!(style, OrderedListStyle::LowerRoman);
        }
    }

    #[test]
    fn test_list_type_unordered_variants() {
        let disc = ListType::Unordered(BulletStyle::Disc);
        let circle = ListType::Unordered(BulletStyle::Circle);
        let square = ListType::Unordered(BulletStyle::Square);
        let dash = ListType::Unordered(BulletStyle::Dash);

        if let ListType::Unordered(style) = disc {
            assert_eq!(style, BulletStyle::Disc);
        }
        if let ListType::Unordered(style) = circle {
            assert_eq!(style, BulletStyle::Circle);
        }
        if let ListType::Unordered(style) = square {
            assert_eq!(style, BulletStyle::Square);
        }
        if let ListType::Unordered(style) = dash {
            assert_eq!(style, BulletStyle::Dash);
        }
    }

    #[test]
    fn test_list_type_clone() {
        let original = ListType::Ordered(OrderedListStyle::Decimal);
        let cloned = original;
        if let ListType::Ordered(style) = cloned {
            assert_eq!(style, OrderedListStyle::Decimal);
        }
    }

    #[test]
    fn test_list_type_debug() {
        let list_type = ListType::Ordered(OrderedListStyle::UpperRoman);
        let debug_str = format!("{:?}", list_type);
        assert!(debug_str.contains("Ordered"));
    }

    // ==================== ListStyle Tests ====================

    #[test]
    fn test_list_style_minimal_ordered() {
        let style = ListStyle::minimal(ListType::Ordered(OrderedListStyle::Decimal));

        assert_eq!(style.font, Font::Helvetica);
        assert_eq!(style.font_size, 10.0);
        assert_eq!(style.text_color, Color::black());
        assert!(style.marker_color.is_none());
        assert!(style.max_width.is_none());
        assert_eq!(style.line_spacing, 1.2);
        assert_eq!(style.indent, 20.0);
        assert_eq!(style.paragraph_spacing, 0.0);
        assert!(!style.draw_separator);

        if let ListType::Ordered(ordered_style) = style.list_type {
            assert_eq!(ordered_style, OrderedListStyle::Decimal);
        } else {
            panic!("Expected Ordered list type");
        }
    }

    #[test]
    fn test_list_style_minimal_unordered() {
        let style = ListStyle::minimal(ListType::Unordered(BulletStyle::Circle));

        assert_eq!(style.font, Font::Helvetica);
        assert_eq!(style.font_size, 10.0);

        if let ListType::Unordered(bullet_style) = style.list_type {
            assert_eq!(bullet_style, BulletStyle::Circle);
        } else {
            panic!("Expected Unordered list type");
        }
    }

    #[test]
    fn test_list_style_professional() {
        let style = ListStyle::professional(ListType::Ordered(OrderedListStyle::UpperAlpha));

        assert_eq!(style.font, Font::Helvetica);
        assert_eq!(style.font_size, 11.0);
        assert_eq!(style.text_color, Color::gray(0.1));
        assert!(style.marker_color.is_some());
        assert_eq!(style.max_width, Some(500.0));
        assert_eq!(style.line_spacing, 1.3);
        assert_eq!(style.indent, 25.0);
        assert_eq!(style.paragraph_spacing, 3.0);
        assert!(!style.draw_separator);
    }

    #[test]
    fn test_list_style_professional_marker_color() {
        let style = ListStyle::professional(ListType::Unordered(BulletStyle::Disc));

        if let Some(color) = style.marker_color {
            // Professional uses blue-ish color (0.2, 0.4, 0.7)
            assert!(color.r() < 0.3);
            assert!(color.g() > 0.3 && color.g() < 0.5);
            assert!(color.b() > 0.6);
        } else {
            panic!("Professional style should have marker color");
        }
    }

    #[test]
    fn test_list_style_document() {
        let style = ListStyle::document(ListType::Ordered(OrderedListStyle::UpperRoman));

        assert_eq!(style.font, Font::TimesRoman);
        assert_eq!(style.font_size, 12.0);
        assert_eq!(style.text_color, Color::black());
        assert!(style.marker_color.is_none());
        assert_eq!(style.max_width, Some(450.0));
        assert_eq!(style.line_spacing, 1.5);
        assert_eq!(style.indent, 30.0);
        assert_eq!(style.paragraph_spacing, 5.0);
        assert!(!style.draw_separator);
    }

    #[test]
    fn test_list_style_presentation() {
        let style = ListStyle::presentation(ListType::Unordered(BulletStyle::Dash));

        assert_eq!(style.font, Font::HelveticaBold);
        assert_eq!(style.font_size, 14.0);
        assert_eq!(style.text_color, Color::gray(0.2));
        assert!(style.marker_color.is_some());
        assert_eq!(style.max_width, Some(600.0));
        assert_eq!(style.line_spacing, 1.6);
        assert_eq!(style.indent, 35.0);
        assert_eq!(style.paragraph_spacing, 8.0);
        assert!(!style.draw_separator);
    }

    #[test]
    fn test_list_style_presentation_marker_color() {
        let style = ListStyle::presentation(ListType::Ordered(OrderedListStyle::Decimal));

        if let Some(color) = style.marker_color {
            // Presentation uses red-ish color (0.8, 0.2, 0.2)
            assert!(color.r() > 0.7);
            assert!(color.g() < 0.3);
            assert!(color.b() < 0.3);
        } else {
            panic!("Presentation style should have marker color");
        }
    }

    #[test]
    fn test_list_style_checklist() {
        let style = ListStyle::checklist();

        assert_eq!(style.font, Font::Helvetica);
        assert_eq!(style.font_size, 11.0);
        assert_eq!(style.text_color, Color::gray(0.1));
        assert!(style.marker_color.is_some());
        assert_eq!(style.max_width, Some(500.0));
        assert_eq!(style.line_spacing, 1.4);
        assert_eq!(style.indent, 25.0);
        assert_eq!(style.paragraph_spacing, 5.0);
        assert!(style.draw_separator);

        // Checklist uses Square bullets
        if let ListType::Unordered(bullet_style) = style.list_type {
            assert_eq!(bullet_style, BulletStyle::Square);
        } else {
            panic!("Checklist should be unordered with Square bullets");
        }
    }

    #[test]
    fn test_list_style_checklist_marker_color() {
        let style = ListStyle::checklist();

        if let Some(color) = style.marker_color {
            // Checklist uses gray marker (0.4)
            assert!(color.r() > 0.3 && color.r() < 0.5);
            assert!(color.g() > 0.3 && color.g() < 0.5);
            assert!(color.b() > 0.3 && color.b() < 0.5);
        } else {
            panic!("Checklist style should have marker color");
        }
    }

    #[test]
    fn test_list_style_clone() {
        let original = ListStyle::professional(ListType::Ordered(OrderedListStyle::Decimal));
        let cloned = original.clone();

        assert_eq!(cloned.font, Font::Helvetica);
        assert_eq!(cloned.font_size, 11.0);
        assert_eq!(cloned.indent, 25.0);
    }

    #[test]
    fn test_list_style_debug() {
        let style = ListStyle::minimal(ListType::Ordered(OrderedListStyle::Decimal));
        let debug_str = format!("{:?}", style);
        assert!(debug_str.contains("ListStyle"));
    }

    #[test]
    fn test_list_style_mutability() {
        let mut style = ListStyle::minimal(ListType::Ordered(OrderedListStyle::Decimal));

        style.font = Font::CourierBold;
        style.font_size = 16.0;
        style.text_color = Color::blue();
        style.marker_color = Some(Color::red());
        style.max_width = Some(400.0);
        style.line_spacing = 2.0;
        style.indent = 50.0;
        style.paragraph_spacing = 10.0;
        style.draw_separator = true;

        assert_eq!(style.font, Font::CourierBold);
        assert_eq!(style.font_size, 16.0);
        assert_eq!(style.text_color, Color::blue());
        assert_eq!(style.marker_color, Some(Color::red()));
        assert_eq!(style.max_width, Some(400.0));
        assert_eq!(style.line_spacing, 2.0);
        assert_eq!(style.indent, 50.0);
        assert_eq!(style.paragraph_spacing, 10.0);
        assert!(style.draw_separator);
    }

    // ==================== Page Integration Tests ====================

    #[test]
    fn test_page_lists_trait() {
        let mut page = Page::a4();

        // Test quick ordered list
        let items = vec![
            "First item".to_string(),
            "Second item".to_string(),
            "Third item".to_string(),
        ];

        let result = page.add_quick_ordered_list(items, 50.0, 700.0, OrderedListStyle::Decimal);
        assert!(result.is_ok());
    }

    #[test]
    fn test_quick_unordered_list() {
        let mut page = Page::a4();

        let items = vec![
            "Apple".to_string(),
            "Banana".to_string(),
            "Cherry".to_string(),
        ];

        let result = page.add_quick_unordered_list(items, 50.0, 700.0, BulletStyle::Disc);
        assert!(result.is_ok());
    }

    #[test]
    fn test_list_styles() {
        let minimal = ListStyle::minimal(ListType::Ordered(OrderedListStyle::Decimal));
        assert_eq!(minimal.font_size, 10.0);
        assert!(minimal.marker_color.is_none());

        let professional = ListStyle::professional(ListType::Unordered(BulletStyle::Circle));
        assert_eq!(professional.font_size, 11.0);
        assert!(professional.marker_color.is_some());

        let document = ListStyle::document(ListType::Ordered(OrderedListStyle::UpperRoman));
        assert_eq!(document.line_spacing, 1.5);

        let presentation = ListStyle::presentation(ListType::Unordered(BulletStyle::Dash));
        assert_eq!(presentation.font_size, 14.0);

        let checklist = ListStyle::checklist();
        assert!(checklist.draw_separator);
    }

    #[test]
    fn test_styled_lists() {
        let mut page = Page::a4();

        let items = vec![
            "Executive Summary".to_string(),
            "Market Analysis".to_string(),
            "Financial Projections".to_string(),
        ];

        let style = ListStyle::professional(ListType::Ordered(OrderedListStyle::UpperAlpha));
        let result = page.add_styled_ordered_list(items, 50.0, 700.0, style);
        assert!(result.is_ok());
    }

    #[test]
    fn test_empty_list() {
        let mut page = Page::a4();

        let items: Vec<String> = vec![];
        let result = page.add_quick_ordered_list(items, 50.0, 700.0, OrderedListStyle::Decimal);
        assert!(result.is_ok());
    }

    #[test]
    fn test_list_with_long_text() {
        let mut page = Page::a4();

        let items = vec![
            "This is a very long list item that should wrap to multiple lines when rendered with a maximum width constraint".to_string(),
            "Short item".to_string(),
        ];

        let mut style = ListStyle::professional(ListType::Ordered(OrderedListStyle::Decimal));
        style.max_width = Some(300.0);

        let result = page.add_styled_ordered_list(items, 50.0, 700.0, style);
        assert!(result.is_ok());
    }

    #[test]
    fn test_styled_unordered_list() {
        let mut page = Page::a4();

        let items = vec!["Red".to_string(), "Green".to_string(), "Blue".to_string()];

        let style = ListStyle::presentation(ListType::Unordered(BulletStyle::Circle));
        let result = page.add_styled_unordered_list(items, 50.0, 700.0, style);
        assert!(result.is_ok());
    }

    #[test]
    fn test_styled_ordered_with_wrong_type_fails() {
        let mut page = Page::a4();

        let items = vec!["Item".to_string()];

        // Try to use unordered style with add_styled_ordered_list
        let style = ListStyle::checklist(); // This is Unordered
        let result = page.add_styled_ordered_list(items, 50.0, 700.0, style);
        assert!(result.is_err());
    }

    #[test]
    fn test_styled_unordered_with_wrong_type_fails() {
        let mut page = Page::a4();

        let items = vec!["Item".to_string()];

        // Try to use ordered style with add_styled_unordered_list
        let style = ListStyle::document(ListType::Ordered(OrderedListStyle::Decimal));
        let result = page.add_styled_unordered_list(items, 50.0, 700.0, style);
        assert!(result.is_err());
    }

    #[test]
    fn test_all_ordered_list_styles() {
        let mut page = Page::a4();

        let styles = vec![
            OrderedListStyle::Decimal,
            OrderedListStyle::LowerAlpha,
            OrderedListStyle::UpperAlpha,
            OrderedListStyle::LowerRoman,
            OrderedListStyle::UpperRoman,
        ];

        for style in styles {
            let items = vec!["A".to_string(), "B".to_string()];
            let result = page.add_quick_ordered_list(items, 50.0, 700.0, style);
            assert!(result.is_ok(), "Failed for style {:?}", style);
        }
    }

    #[test]
    fn test_all_bullet_styles() {
        let mut page = Page::a4();

        let styles = vec![
            BulletStyle::Disc,
            BulletStyle::Circle,
            BulletStyle::Square,
            BulletStyle::Dash,
        ];

        for style in styles {
            let items = vec!["X".to_string(), "Y".to_string()];
            let result = page.add_quick_unordered_list(items, 50.0, 700.0, style);
            assert!(result.is_ok(), "Failed for bullet {:?}", style);
        }
    }
}
