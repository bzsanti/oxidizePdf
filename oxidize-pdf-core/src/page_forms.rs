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
        let field_dict = create_text_field_dict(name, rect.clone(), default_value);

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
        let field_dict = create_checkbox_dict(name, rect.clone(), checked);

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
        let field_dict = create_radio_button_dict(name, rect.clone(), export_value, checked);

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
        let field_dict = create_combo_box_dict(name, rect.clone(), options, default_value);

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
        let field_dict = create_list_box_dict(name, rect.clone(), options, selected, multi_select);

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
        let field_dict = create_push_button_dict(name, rect.clone(), caption);

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
