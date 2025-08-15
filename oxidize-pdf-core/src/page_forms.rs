//! Page-level forms API
//!
//! This module provides a simpler API for adding working form fields to pages

use crate::annotations::{Annotation, AnnotationType};
use crate::error::Result;
use crate::forms::{
    create_checkbox_dict, create_combo_box_dict, create_list_box_dict, create_push_button_dict,
    create_radio_button_dict, create_text_field_dict,
};
use crate::geometry::Rectangle;
use crate::page::Page;

/// Extension trait for Page to add form fields easily
pub trait PageForms {
    /// Add a text field to the page
    fn add_text_field(
        &mut self,
        name: &str,
        rect: Rectangle,
        default_value: Option<&str>,
    ) -> Result<()>;

    /// Add a checkbox to the page
    fn add_checkbox(&mut self, name: &str, rect: Rectangle, checked: bool) -> Result<()>;

    /// Add a radio button to the page
    fn add_radio_button(
        &mut self,
        name: &str,
        rect: Rectangle,
        export_value: &str,
        checked: bool,
    ) -> Result<()>;

    /// Add a combo box (dropdown) to the page
    fn add_combo_box(
        &mut self,
        name: &str,
        rect: Rectangle,
        options: Vec<(&str, &str)>,
        default_value: Option<&str>,
    ) -> Result<()>;

    /// Add a list box to the page
    fn add_list_box(
        &mut self,
        name: &str,
        rect: Rectangle,
        options: Vec<(&str, &str)>,
        selected: Vec<usize>,
        multi_select: bool,
    ) -> Result<()>;

    /// Add a push button to the page
    fn add_push_button(&mut self, name: &str, rect: Rectangle, caption: &str) -> Result<()>;
}

impl PageForms for Page {
    fn add_text_field(
        &mut self,
        name: &str,
        rect: Rectangle,
        default_value: Option<&str>,
    ) -> Result<()> {
        // Create the field dictionary
        let field_dict = create_text_field_dict(name, rect, default_value);

        // Create annotation with the field properties
        let mut annotation = Annotation::new(AnnotationType::Widget, rect);
        for (key, value) in field_dict.entries() {
            annotation.properties.set(key, value.clone());
        }

        // Add to page
        self.annotations_mut().push(annotation);

        Ok(())
    }

    fn add_checkbox(&mut self, name: &str, rect: Rectangle, checked: bool) -> Result<()> {
        // Create the field dictionary
        let field_dict = create_checkbox_dict(name, rect, checked);

        // Create annotation with the field properties
        let mut annotation = Annotation::new(AnnotationType::Widget, rect);
        for (key, value) in field_dict.entries() {
            annotation.properties.set(key, value.clone());
        }

        // Add to page
        self.annotations_mut().push(annotation);

        Ok(())
    }

    fn add_radio_button(
        &mut self,
        name: &str,
        rect: Rectangle,
        export_value: &str,
        checked: bool,
    ) -> Result<()> {
        // Create the field dictionary
        let field_dict = create_radio_button_dict(name, rect, export_value, checked);

        // Create annotation with the field properties
        let mut annotation = Annotation::new(AnnotationType::Widget, rect);
        for (key, value) in field_dict.entries() {
            annotation.properties.set(key, value.clone());
        }

        // Add to page
        self.annotations_mut().push(annotation);

        Ok(())
    }

    fn add_combo_box(
        &mut self,
        name: &str,
        rect: Rectangle,
        options: Vec<(&str, &str)>,
        default_value: Option<&str>,
    ) -> Result<()> {
        // Create the field dictionary
        let field_dict = create_combo_box_dict(name, rect, options, default_value);

        // Create annotation with the field properties
        let mut annotation = Annotation::new(AnnotationType::Widget, rect);
        for (key, value) in field_dict.entries() {
            annotation.properties.set(key, value.clone());
        }

        // Add to page
        self.annotations_mut().push(annotation);

        Ok(())
    }

    fn add_list_box(
        &mut self,
        name: &str,
        rect: Rectangle,
        options: Vec<(&str, &str)>,
        selected: Vec<usize>,
        multi_select: bool,
    ) -> Result<()> {
        // Create the field dictionary
        let field_dict = create_list_box_dict(name, rect, options, selected, multi_select);

        // Create annotation with the field properties
        let mut annotation = Annotation::new(AnnotationType::Widget, rect);
        for (key, value) in field_dict.entries() {
            annotation.properties.set(key, value.clone());
        }

        // Add to page
        self.annotations_mut().push(annotation);

        Ok(())
    }

    fn add_push_button(&mut self, name: &str, rect: Rectangle, caption: &str) -> Result<()> {
        // Create the field dictionary
        let field_dict = create_push_button_dict(name, rect, caption);

        // Create annotation with the field properties
        let mut annotation = Annotation::new(AnnotationType::Widget, rect);
        for (key, value) in field_dict.entries() {
            annotation.properties.set(key, value.clone());
        }

        // Add to page
        self.annotations_mut().push(annotation);

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::{Point, Rectangle};
    use crate::page::Page;

    #[test]
    fn test_add_text_field() {
        let mut page = Page::a4();
        let rect = Rectangle::new(Point::new(100.0, 700.0), Point::new(300.0, 720.0));

        let result = page.add_text_field("name", rect, Some("John Doe"));
        assert!(result.is_ok());

        // Verify annotation was added
        assert_eq!(page.annotations().len(), 1);
    }

    #[test]
    fn test_add_text_field_without_default() {
        let mut page = Page::a4();
        let rect = Rectangle::new(Point::new(100.0, 650.0), Point::new(300.0, 670.0));

        let result = page.add_text_field("email", rect, None);
        assert!(result.is_ok());
        assert_eq!(page.annotations().len(), 1);
    }

    #[test]
    fn test_add_checkbox() {
        let mut page = Page::a4();
        let rect = Rectangle::new(Point::new(100.0, 600.0), Point::new(120.0, 620.0));

        let result = page.add_checkbox("agree", rect, true);
        assert!(result.is_ok());
        assert_eq!(page.annotations().len(), 1);
    }

    #[test]
    fn test_add_checkbox_unchecked() {
        let mut page = Page::a4();
        let rect = Rectangle::new(Point::new(100.0, 550.0), Point::new(120.0, 570.0));

        let result = page.add_checkbox("subscribe", rect, false);
        assert!(result.is_ok());
        assert_eq!(page.annotations().len(), 1);
    }

    #[test]
    fn test_add_radio_button() {
        let mut page = Page::a4();
        let rect = Rectangle::new(Point::new(100.0, 500.0), Point::new(120.0, 520.0));

        let result = page.add_radio_button("gender", rect, "male", true);
        assert!(result.is_ok());
        assert_eq!(page.annotations().len(), 1);
    }

    #[test]
    fn test_add_multiple_radio_buttons() {
        let mut page = Page::a4();

        let rect1 = Rectangle::new(Point::new(100.0, 450.0), Point::new(120.0, 470.0));
        let rect2 = Rectangle::new(Point::new(150.0, 450.0), Point::new(170.0, 470.0));
        let rect3 = Rectangle::new(Point::new(200.0, 450.0), Point::new(220.0, 470.0));

        assert!(page.add_radio_button("size", rect1, "small", false).is_ok());
        assert!(page.add_radio_button("size", rect2, "medium", true).is_ok());
        assert!(page.add_radio_button("size", rect3, "large", false).is_ok());

        assert_eq!(page.annotations().len(), 3);
    }

    #[test]
    fn test_add_combo_box() {
        let mut page = Page::a4();
        let rect = Rectangle::new(Point::new(100.0, 400.0), Point::new(250.0, 420.0));

        let options = vec![("US", "United States"), ("CA", "Canada"), ("MX", "Mexico")];

        let result = page.add_combo_box("country", rect, options, Some("US"));
        assert!(result.is_ok());
        assert_eq!(page.annotations().len(), 1);
    }

    #[test]
    fn test_add_combo_box_no_default() {
        let mut page = Page::a4();
        let rect = Rectangle::new(Point::new(100.0, 350.0), Point::new(250.0, 370.0));

        let options = vec![("red", "Red"), ("green", "Green"), ("blue", "Blue")];

        let result = page.add_combo_box("color", rect, options, None);
        assert!(result.is_ok());
        assert_eq!(page.annotations().len(), 1);
    }

    #[test]
    fn test_add_list_box() {
        let mut page = Page::a4();
        let rect = Rectangle::new(Point::new(100.0, 250.0), Point::new(250.0, 330.0));

        let options = vec![
            ("item1", "First Item"),
            ("item2", "Second Item"),
            ("item3", "Third Item"),
            ("item4", "Fourth Item"),
        ];

        let result = page.add_list_box("items", rect, options, vec![1], false);
        assert!(result.is_ok());
        assert_eq!(page.annotations().len(), 1);
    }

    #[test]
    fn test_add_list_box_multiple_selection() {
        let mut page = Page::a4();
        let rect = Rectangle::new(Point::new(100.0, 150.0), Point::new(250.0, 230.0));

        let options = vec![
            ("opt1", "Option 1"),
            ("opt2", "Option 2"),
            ("opt3", "Option 3"),
        ];

        let result = page.add_list_box("multi_select", rect, options, vec![0, 2], true);
        assert!(result.is_ok());
        assert_eq!(page.annotations().len(), 1);
    }

    #[test]
    fn test_add_push_button() {
        let mut page = Page::a4();
        let rect = Rectangle::new(Point::new(100.0, 100.0), Point::new(200.0, 130.0));

        let result = page.add_push_button("submit", rect, "Submit");
        assert!(result.is_ok());
        assert_eq!(page.annotations().len(), 1);
    }

    #[test]
    fn test_add_multiple_form_fields() {
        let mut page = Page::a4();

        // Add various form fields
        let text_rect = Rectangle::new(Point::new(100.0, 700.0), Point::new(300.0, 720.0));
        assert!(page
            .add_text_field("name", text_rect, Some("Enter name"))
            .is_ok());

        let check_rect = Rectangle::new(Point::new(100.0, 650.0), Point::new(120.0, 670.0));
        assert!(page.add_checkbox("agree", check_rect, false).is_ok());

        let combo_rect = Rectangle::new(Point::new(100.0, 600.0), Point::new(250.0, 620.0));
        let options = vec![("opt1", "Option 1"), ("opt2", "Option 2")];
        assert!(page
            .add_combo_box("choice", combo_rect, options, None)
            .is_ok());

        let button_rect = Rectangle::new(Point::new(100.0, 550.0), Point::new(200.0, 580.0));
        assert!(page
            .add_push_button("submit", button_rect, "Submit")
            .is_ok());

        // Verify all annotations were added
        assert_eq!(page.annotations().len(), 4);
    }

    #[test]
    fn test_field_positioning() {
        let mut page = Page::a4();

        // Test different positions
        let top_rect = Rectangle::new(Point::new(50.0, 750.0), Point::new(150.0, 770.0));
        let middle_rect = Rectangle::new(Point::new(200.0, 400.0), Point::new(300.0, 420.0));
        let bottom_rect = Rectangle::new(Point::new(350.0, 50.0), Point::new(450.0, 70.0));

        assert!(page.add_text_field("top", top_rect, None).is_ok());
        assert!(page.add_text_field("middle", middle_rect, None).is_ok());
        assert!(page.add_text_field("bottom", bottom_rect, None).is_ok());

        assert_eq!(page.annotations().len(), 3);
    }

    #[test]
    fn test_empty_options_list() {
        let mut page = Page::a4();
        let rect = Rectangle::new(Point::new(100.0, 400.0), Point::new(250.0, 420.0));

        let options: Vec<(&str, &str)> = vec![];

        // Empty options should still create the field
        let result = page.add_combo_box("empty_combo", rect, options, None);
        assert!(result.is_ok());
    }

    #[test]
    fn test_special_characters_in_names() {
        let mut page = Page::a4();
        let rect = Rectangle::new(Point::new(100.0, 400.0), Point::new(250.0, 420.0));

        // Test with special characters in field names
        assert!(page.add_text_field("field.with.dots", rect, None).is_ok());

        let rect2 = Rectangle::new(Point::new(100.0, 350.0), Point::new(250.0, 370.0));
        assert!(page
            .add_text_field("field-with-dashes", rect2, None)
            .is_ok());

        let rect3 = Rectangle::new(Point::new(100.0, 300.0), Point::new(250.0, 320.0));
        assert!(page
            .add_text_field("field_with_underscores", rect3, None)
            .is_ok());

        assert_eq!(page.annotations().len(), 3);
    }

    #[test]
    fn test_overlapping_fields() {
        let mut page = Page::a4();

        // Create overlapping rectangles
        let rect1 = Rectangle::new(Point::new(100.0, 400.0), Point::new(200.0, 420.0));
        let rect2 = Rectangle::new(Point::new(150.0, 410.0), Point::new(250.0, 430.0));

        // Both should be added successfully despite overlap
        assert!(page.add_text_field("field1", rect1, None).is_ok());
        assert!(page.add_text_field("field2", rect2, None).is_ok());

        assert_eq!(page.annotations().len(), 2);
    }

    #[test]
    fn test_large_text_in_fields() {
        let mut page = Page::a4();
        let rect = Rectangle::new(Point::new(100.0, 400.0), Point::new(300.0, 420.0));

        let long_text = "This is a very long text that might not fit completely in the field but should still be accepted as default value";

        let result = page.add_text_field("long_text", rect, Some(long_text));
        assert!(result.is_ok());
    }

    #[test]
    fn test_unicode_in_field_values() {
        let mut page = Page::a4();
        let rect = Rectangle::new(Point::new(100.0, 400.0), Point::new(300.0, 420.0));

        // Test with unicode characters
        let unicode_text = "日本語 العربية Ñoño";
        let result = page.add_text_field("unicode_field", rect, Some(unicode_text));
        assert!(result.is_ok());

        // Test with emoji
        let emoji_text = "Hello 👋 World 🌍";
        let rect2 = Rectangle::new(Point::new(100.0, 350.0), Point::new(300.0, 370.0));
        let result2 = page.add_text_field("emoji_field", rect2, Some(emoji_text));
        assert!(result2.is_ok());

        assert_eq!(page.annotations().len(), 2);
    }
}
