//! Choice field widgets (ComboBox and ListBox) for PDF forms
//!
//! This module implements ISO 32000-1 Section 12.7.4.4 (Choice Fields)
//! including combo boxes (dropdowns) and list boxes with single or multi-select.

use crate::annotations::{Annotation, AnnotationType};
use crate::error::Result;
use crate::forms::{ComboBox, ListBox};
use crate::geometry::Rectangle;
use crate::graphics::Color;
use crate::objects::{Dictionary, Object, Stream};
use crate::text::Font;
use std::fmt::Write;

/// Widget annotation for choice fields (ComboBox and ListBox)
#[derive(Debug, Clone)]
pub struct ChoiceWidget {
    /// Widget rectangle
    pub rect: Rectangle,
    /// Border color
    pub border_color: Color,
    /// Border width
    pub border_width: f64,
    /// Background color
    pub background_color: Option<Color>,
    /// Text color
    pub text_color: Color,
    /// Font
    pub font: Font,
    /// Font size
    pub font_size: f64,
    /// Highlight color for selected items
    pub highlight_color: Option<Color>,
}

impl Default for ChoiceWidget {
    fn default() -> Self {
        Self {
            rect: Rectangle::from_position_and_size(0.0, 0.0, 100.0, 20.0),
            border_color: Color::rgb(0.0, 0.0, 0.0),
            border_width: 1.0,
            background_color: Some(Color::rgb(1.0, 1.0, 1.0)),
            text_color: Color::rgb(0.0, 0.0, 0.0),
            font: Font::Helvetica,
            font_size: 10.0,
            highlight_color: Some(Color::rgb(0.8, 0.8, 1.0)),
        }
    }
}

impl ChoiceWidget {
    /// Create a new choice widget
    pub fn new(rect: Rectangle) -> Self {
        Self {
            rect,
            ..Default::default()
        }
    }

    /// Set border color
    pub fn with_border_color(mut self, color: Color) -> Self {
        self.border_color = color;
        self
    }

    /// Set border width
    pub fn with_border_width(mut self, width: f64) -> Self {
        self.border_width = width;
        self
    }

    /// Set background color
    pub fn with_background_color(mut self, color: Option<Color>) -> Self {
        self.background_color = color;
        self
    }

    /// Set text color
    pub fn with_text_color(mut self, color: Color) -> Self {
        self.text_color = color;
        self
    }

    /// Set font
    pub fn with_font(mut self, font: Font) -> Self {
        self.font = font;
        self
    }

    /// Set font size
    pub fn with_font_size(mut self, size: f64) -> Self {
        self.font_size = size;
        self
    }

    /// Set highlight color for selected items
    pub fn with_highlight_color(mut self, color: Option<Color>) -> Self {
        self.highlight_color = color;
        self
    }

    /// Create appearance stream for a combo box
    fn create_combobox_appearance(&self, combo: &ComboBox) -> String {
        let mut stream = String::new();

        // Save graphics state
        writeln!(&mut stream, "q").unwrap();

        // Draw background if specified
        if let Some(bg_color) = &self.background_color {
            writeln!(
                &mut stream,
                "{:.3} {:.3} {:.3} rg",
                bg_color.r(),
                bg_color.g(),
                bg_color.b()
            )
            .unwrap();
            writeln!(
                &mut stream,
                "0 0 {} {} re",
                self.rect.width(),
                self.rect.height()
            )
            .unwrap();
            writeln!(&mut stream, "f").unwrap();
        }

        // Draw border
        writeln!(
            &mut stream,
            "{:.3} {:.3} {:.3} RG",
            self.border_color.r(),
            self.border_color.g(),
            self.border_color.b()
        )
        .unwrap();
        writeln!(&mut stream, "{} w", self.border_width).unwrap();
        writeln!(
            &mut stream,
            "0 0 {} {} re",
            self.rect.width(),
            self.rect.height()
        )
        .unwrap();
        writeln!(&mut stream, "S").unwrap();

        // Draw dropdown arrow on the right
        let arrow_x = self.rect.width() - 15.0;
        let arrow_y = self.rect.height() / 2.0;
        writeln!(&mut stream, "{:.3} {:.3} {:.3} rg", 0.3, 0.3, 0.3).unwrap();
        writeln!(&mut stream, "{} {} m", arrow_x, arrow_y + 3.0).unwrap();
        writeln!(&mut stream, "{} {} l", arrow_x + 8.0, arrow_y + 3.0).unwrap();
        writeln!(&mut stream, "{} {} l", arrow_x + 4.0, arrow_y - 3.0).unwrap();
        writeln!(&mut stream, "f").unwrap();

        // Draw selected text if any
        if let Some(selected_idx) = combo.selected {
            if let Some((_, display_text)) = combo.options.get(selected_idx) {
                writeln!(&mut stream, "BT").unwrap();
                writeln!(
                    &mut stream,
                    "/{} {} Tf",
                    self.font.pdf_name(),
                    self.font_size
                )
                .unwrap();
                writeln!(
                    &mut stream,
                    "{:.3} {:.3} {:.3} rg",
                    self.text_color.r(),
                    self.text_color.g(),
                    self.text_color.b()
                )
                .unwrap();
                writeln!(
                    &mut stream,
                    "2 {} Td",
                    (self.rect.height() - self.font_size) / 2.0
                )
                .unwrap();
                writeln!(&mut stream, "({}) Tj", escape_pdf_string(display_text)).unwrap();
                writeln!(&mut stream, "ET").unwrap();
            }
        }

        // Restore graphics state
        writeln!(&mut stream, "Q").unwrap();

        stream
    }

    /// Create appearance stream for a list box
    fn create_listbox_appearance(&self, listbox: &ListBox) -> String {
        let mut stream = String::new();

        // Save graphics state
        writeln!(&mut stream, "q").unwrap();

        // Draw background
        if let Some(bg_color) = &self.background_color {
            writeln!(
                &mut stream,
                "{:.3} {:.3} {:.3} rg",
                bg_color.r(),
                bg_color.g(),
                bg_color.b()
            )
            .unwrap();
            writeln!(
                &mut stream,
                "0 0 {} {} re",
                self.rect.width(),
                self.rect.height()
            )
            .unwrap();
            writeln!(&mut stream, "f").unwrap();
        }

        // Draw border
        writeln!(
            &mut stream,
            "{:.3} {:.3} {:.3} RG",
            self.border_color.r(),
            self.border_color.g(),
            self.border_color.b()
        )
        .unwrap();
        writeln!(&mut stream, "{} w", self.border_width).unwrap();
        writeln!(
            &mut stream,
            "0 0 {} {} re",
            self.rect.width(),
            self.rect.height()
        )
        .unwrap();
        writeln!(&mut stream, "S").unwrap();

        // Calculate visible items
        let item_height = self.font_size + 4.0;
        let visible_items = (self.rect.height() / item_height) as usize;

        // Draw items

        for (idx, (_, display_text)) in listbox.options.iter().enumerate().take(visible_items) {
            let y_pos = self.rect.height() - ((idx + 1) as f64 * item_height);

            // Draw highlight for selected items
            if listbox.selected.contains(&idx) {
                if let Some(highlight) = &self.highlight_color {
                    writeln!(
                        &mut stream,
                        "{:.3} {:.3} {:.3} rg",
                        highlight.r(),
                        highlight.g(),
                        highlight.b()
                    )
                    .unwrap();
                    writeln!(
                        &mut stream,
                        "0 {} {} {} re",
                        y_pos,
                        self.rect.width(),
                        item_height
                    )
                    .unwrap();
                    writeln!(&mut stream, "f").unwrap();
                }
            }

            // Draw text
            writeln!(&mut stream, "BT").unwrap();
            writeln!(
                &mut stream,
                "/{} {} Tf",
                self.font.pdf_name(),
                self.font_size
            )
            .unwrap();
            writeln!(
                &mut stream,
                "{:.3} {:.3} {:.3} rg",
                self.text_color.r(),
                self.text_color.g(),
                self.text_color.b()
            )
            .unwrap();
            writeln!(&mut stream, "2 {} Td", y_pos + 2.0).unwrap();
            writeln!(&mut stream, "({}) Tj", escape_pdf_string(display_text)).unwrap();
            writeln!(&mut stream, "ET").unwrap();
        }

        // Draw scrollbar if needed
        if listbox.options.len() > visible_items {
            writeln!(&mut stream, "0.7 0.7 0.7 rg").unwrap();
            let scrollbar_x = self.rect.width() - 10.0;
            writeln!(&mut stream, "{} 0 8 {} re", scrollbar_x, self.rect.height()).unwrap();
            writeln!(&mut stream, "f").unwrap();

            // Draw scroll thumb
            writeln!(&mut stream, "0.4 0.4 0.4 rg").unwrap();
            let thumb_height =
                (visible_items as f64 / listbox.options.len() as f64) * self.rect.height();
            writeln!(
                &mut stream,
                "{} {} 8 {} re",
                scrollbar_x,
                self.rect.height() - thumb_height,
                thumb_height
            )
            .unwrap();
            writeln!(&mut stream, "f").unwrap();
        }

        // Restore graphics state
        writeln!(&mut stream, "Q").unwrap();

        stream
    }
}

/// Create a widget annotation for a combo box
pub fn create_combobox_widget(combo: &ComboBox, widget: &ChoiceWidget) -> Result<Annotation> {
    let mut annotation = Annotation::new(AnnotationType::Widget, widget.rect);

    // Set field reference
    let mut field_dict = combo.to_dict();

    // Add widget-specific entries
    field_dict.set(
        "Rect",
        Object::Array(vec![
            Object::Real(widget.rect.lower_left.x),
            Object::Real(widget.rect.lower_left.y),
            Object::Real(widget.rect.upper_right.x),
            Object::Real(widget.rect.upper_right.y),
        ]),
    );

    // Create appearance stream
    let appearance_content = widget.create_combobox_appearance(combo);
    let appearance_stream = create_appearance_stream(
        appearance_content.as_bytes(),
        widget.rect.width(),
        widget.rect.height(),
    );

    // Create appearance dictionary
    let mut ap_dict = Dictionary::new();
    let mut n_dict = Dictionary::new();
    n_dict.set(
        "default",
        Object::Stream(
            appearance_stream.dictionary().clone(),
            appearance_stream.data().to_vec(),
        ),
    );
    ap_dict.set("N", Object::Dictionary(n_dict));
    field_dict.set("AP", Object::Dictionary(ap_dict));

    // Set default appearance string
    let da = format!(
        "/{} {} Tf {} {} {} rg",
        widget.font.pdf_name(),
        widget.font_size,
        widget.text_color.r(),
        widget.text_color.g(),
        widget.text_color.b(),
    );
    field_dict.set("DA", Object::String(da));

    // Set the field dictionary as the annotation's dictionary
    annotation.set_field_dict(field_dict);

    Ok(annotation)
}

/// Create a widget annotation for a list box
pub fn create_listbox_widget(listbox: &ListBox, widget: &ChoiceWidget) -> Result<Annotation> {
    let mut annotation = Annotation::new(AnnotationType::Widget, widget.rect);

    // Set field reference
    let mut field_dict = listbox.to_dict();

    // Add widget-specific entries
    field_dict.set(
        "Rect",
        Object::Array(vec![
            Object::Real(widget.rect.lower_left.x),
            Object::Real(widget.rect.lower_left.y),
            Object::Real(widget.rect.upper_right.x),
            Object::Real(widget.rect.upper_right.y),
        ]),
    );

    // Create appearance stream
    let appearance_content = widget.create_listbox_appearance(listbox);
    let appearance_stream = create_appearance_stream(
        appearance_content.as_bytes(),
        widget.rect.width(),
        widget.rect.height(),
    );

    // Create appearance dictionary
    let mut ap_dict = Dictionary::new();
    let mut n_dict = Dictionary::new();
    n_dict.set(
        "default",
        Object::Stream(
            appearance_stream.dictionary().clone(),
            appearance_stream.data().to_vec(),
        ),
    );
    ap_dict.set("N", Object::Dictionary(n_dict));
    field_dict.set("AP", Object::Dictionary(ap_dict));

    // Set default appearance string
    let da = format!(
        "/{} {} Tf {} {} {} rg",
        widget.font.pdf_name(),
        widget.font_size,
        widget.text_color.r(),
        widget.text_color.g(),
        widget.text_color.b(),
    );
    field_dict.set("DA", Object::String(da));

    // Set the field dictionary as the annotation's dictionary
    annotation.set_field_dict(field_dict);

    Ok(annotation)
}

/// Helper function to escape PDF strings
fn escape_pdf_string(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            '(' => "\\(".to_string(),
            ')' => "\\)".to_string(),
            '\\' => "\\\\".to_string(),
            _ => c.to_string(),
        })
        .collect()
}

/// Create an appearance stream
fn create_appearance_stream(content: &[u8], width: f64, height: f64) -> Stream {
    let mut dict = Dictionary::new();
    dict.set("Type", Object::Name("XObject".to_string()));
    dict.set("Subtype", Object::Name("Form".to_string()));
    dict.set(
        "BBox",
        Object::Array(vec![
            Object::Integer(0),
            Object::Integer(0),
            Object::Real(width),
            Object::Real(height),
        ]),
    );

    Stream::with_dictionary(dict, content.to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::Point;

    #[test]
    fn test_choice_widget_creation() {
        let rect = Rectangle::new(Point::new(100.0, 100.0), Point::new(200.0, 120.0));
        let widget = ChoiceWidget::new(rect.clone());

        assert_eq!(widget.rect, rect);
        assert_eq!(widget.font_size, 10.0);
    }

    #[test]
    fn test_choice_widget_builder() {
        let rect = Rectangle::new(Point::new(0.0, 0.0), Point::new(100.0, 30.0));
        let widget = ChoiceWidget::new(rect)
            .with_border_color(Color::rgb(1.0, 0.0, 0.0))
            .with_font_size(12.0)
            .with_font(Font::HelveticaBold);

        assert_eq!(widget.border_color, Color::rgb(1.0, 0.0, 0.0));
        assert_eq!(widget.font_size, 12.0);
        assert_eq!(widget.font, Font::HelveticaBold);
    }

    #[test]
    fn test_combobox_widget_creation() {
        let combo = ComboBox::new("country")
            .add_option("US", "United States")
            .add_option("CA", "Canada")
            .with_selected(0);

        let rect = Rectangle::new(Point::new(100.0, 100.0), Point::new(250.0, 125.0));
        let widget = ChoiceWidget::new(rect);

        let annotation = create_combobox_widget(&combo, &widget);
        assert!(annotation.is_ok());
    }

    #[test]
    fn test_listbox_widget_creation() {
        let listbox = ListBox::new("languages")
            .add_option("en", "English")
            .add_option("es", "Spanish")
            .add_option("fr", "French")
            .with_selected(vec![0, 2]);

        let rect = Rectangle::new(Point::new(100.0, 100.0), Point::new(200.0, 200.0));
        let widget = ChoiceWidget::new(rect);

        let annotation = create_listbox_widget(&listbox, &widget);
        assert!(annotation.is_ok());
    }

    #[test]
    fn test_escape_pdf_string() {
        assert_eq!(escape_pdf_string("Hello"), "Hello");
        assert_eq!(escape_pdf_string("Hello (World)"), "Hello \\(World\\)");
        assert_eq!(escape_pdf_string("Path\\to\\file"), "Path\\\\to\\\\file");
    }
}
