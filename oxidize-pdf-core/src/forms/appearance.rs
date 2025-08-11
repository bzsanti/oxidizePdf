//! Appearance streams for form fields according to ISO 32000-1 Section 12.7.3.3
//!
//! This module provides appearance stream generation for interactive form fields,
//! ensuring visual representation of field content and states.

use crate::error::Result;
use crate::forms::{BorderStyle, FieldType, Widget};
use crate::graphics::Color;
use crate::objects::{Dictionary, Object, Stream};
use crate::text::Font;
use std::collections::HashMap;

/// Appearance states for form fields
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AppearanceState {
    /// Normal appearance (default state)
    Normal,
    /// Rollover appearance (mouse hover)
    Rollover,
    /// Down appearance (mouse pressed)
    Down,
}

impl AppearanceState {
    /// Get the PDF name for this state
    pub fn pdf_name(&self) -> &'static str {
        match self {
            AppearanceState::Normal => "N",
            AppearanceState::Rollover => "R",
            AppearanceState::Down => "D",
        }
    }
}

/// Appearance stream for a form field
#[derive(Debug, Clone)]
pub struct AppearanceStream {
    /// The content stream data
    pub content: Vec<u8>,
    /// Resources dictionary (fonts, colors, etc.)
    pub resources: Dictionary,
    /// Bounding box for the appearance
    pub bbox: [f64; 4],
}

impl AppearanceStream {
    /// Create a new appearance stream
    pub fn new(content: Vec<u8>, bbox: [f64; 4]) -> Self {
        Self {
            content,
            resources: Dictionary::new(),
            bbox,
        }
    }

    /// Set resources dictionary
    pub fn with_resources(mut self, resources: Dictionary) -> Self {
        self.resources = resources;
        self
    }

    /// Convert to a Stream object
    pub fn to_stream(&self) -> Stream {
        let mut dict = Dictionary::new();
        dict.set("Type", Object::Name("XObject".to_string()));
        dict.set("Subtype", Object::Name("Form".to_string()));

        // Set bounding box
        let bbox_array = vec![
            Object::Real(self.bbox[0]),
            Object::Real(self.bbox[1]),
            Object::Real(self.bbox[2]),
            Object::Real(self.bbox[3]),
        ];
        dict.set("BBox", Object::Array(bbox_array));

        // Set resources
        if !self.resources.is_empty() {
            dict.set("Resources", Object::Dictionary(self.resources.clone()));
        }

        // Create stream with dictionary
        Stream::with_dictionary(dict, self.content.clone())
    }
}

/// Appearance dictionary for a form field
#[derive(Debug, Clone)]
pub struct AppearanceDictionary {
    /// Appearance streams by state
    appearances: HashMap<AppearanceState, AppearanceStream>,
    /// Down appearances for different values (checkboxes, radio buttons)
    down_appearances: HashMap<String, AppearanceStream>,
}

impl AppearanceDictionary {
    /// Create a new appearance dictionary
    pub fn new() -> Self {
        Self {
            appearances: HashMap::new(),
            down_appearances: HashMap::new(),
        }
    }

    /// Set appearance for a specific state
    pub fn set_appearance(&mut self, state: AppearanceState, stream: AppearanceStream) {
        self.appearances.insert(state, stream);
    }

    /// Set down appearance for a specific value
    pub fn set_down_appearance(&mut self, value: String, stream: AppearanceStream) {
        self.down_appearances.insert(value, stream);
    }

    /// Get appearance for a state
    pub fn get_appearance(&self, state: AppearanceState) -> Option<&AppearanceStream> {
        self.appearances.get(&state)
    }

    /// Convert to PDF dictionary
    pub fn to_dict(&self) -> Dictionary {
        let mut dict = Dictionary::new();

        // Add appearances by state
        for (state, stream) in &self.appearances {
            let stream_obj = stream.to_stream();
            dict.set(
                state.pdf_name(),
                Object::Stream(stream_obj.dictionary().clone(), stream_obj.data().to_vec()),
            );
        }

        // Add down appearances if any
        if !self.down_appearances.is_empty() {
            let mut down_dict = Dictionary::new();
            for (value, stream) in &self.down_appearances {
                let stream_obj = stream.to_stream();
                down_dict.set(
                    value,
                    Object::Stream(stream_obj.dictionary().clone(), stream_obj.data().to_vec()),
                );
            }
            dict.set("D", Object::Dictionary(down_dict));
        }

        dict
    }
}

impl Default for AppearanceDictionary {
    fn default() -> Self {
        Self::new()
    }
}

/// Trait for generating appearance streams for different field types
pub trait AppearanceGenerator {
    /// Generate appearance stream for the field
    fn generate_appearance(
        &self,
        widget: &Widget,
        value: Option<&str>,
        state: AppearanceState,
    ) -> Result<AppearanceStream>;
}

/// Text field appearance generator
pub struct TextFieldAppearance {
    /// Font to use
    pub font: Font,
    /// Font size
    pub font_size: f64,
    /// Text color
    pub text_color: Color,
    /// Justification (0=left, 1=center, 2=right)
    pub justification: i32,
    /// Multiline text
    pub multiline: bool,
}

impl Default for TextFieldAppearance {
    fn default() -> Self {
        Self {
            font: Font::Helvetica,
            font_size: 12.0,
            text_color: Color::black(),
            justification: 0,
            multiline: false,
        }
    }
}

impl AppearanceGenerator for TextFieldAppearance {
    fn generate_appearance(
        &self,
        widget: &Widget,
        value: Option<&str>,
        _state: AppearanceState,
    ) -> Result<AppearanceStream> {
        let width = widget.rect.upper_right.x - widget.rect.lower_left.x;
        let height = widget.rect.upper_right.y - widget.rect.lower_left.y;

        let mut content = String::new();

        // Save graphics state
        content.push_str("q\n");

        // Draw background if specified
        if let Some(bg_color) = &widget.appearance.background_color {
            match bg_color {
                Color::Gray(g) => content.push_str(&format!("{g} g\n")),
                Color::Rgb(r, g, b) => content.push_str(&format!("{r} {g} {b} rg\n")),
                Color::Cmyk(c, m, y, k) => content.push_str(&format!("{c} {m} {y} {k} k\n")),
            }
            content.push_str(&format!("0 0 {width} {height} re f\n"));
        }

        // Draw border
        if let Some(border_color) = &widget.appearance.border_color {
            match border_color {
                Color::Gray(g) => content.push_str(&format!("{g} G\n")),
                Color::Rgb(r, g, b) => content.push_str(&format!("{r} {g} {b} RG\n")),
                Color::Cmyk(c, m, y, k) => content.push_str(&format!("{c} {m} {y} {k} K\n")),
            }
            content.push_str(&format!("{} w\n", widget.appearance.border_width));

            match widget.appearance.border_style {
                BorderStyle::Solid => {
                    content.push_str(&format!("0 0 {width} {height} re S\n"));
                }
                BorderStyle::Dashed => {
                    content.push_str("[3 2] 0 d\n");
                    content.push_str(&format!("0 0 {width} {height} re S\n"));
                }
                BorderStyle::Beveled | BorderStyle::Inset => {
                    // Simplified beveled/inset border
                    content.push_str(&format!("0 0 {width} {height} re S\n"));
                }
                BorderStyle::Underline => {
                    content.push_str(&format!("0 0 m {width} 0 l S\n"));
                }
            }
        }

        // Draw text if value is provided
        if let Some(text) = value {
            // Set text color
            match self.text_color {
                Color::Gray(g) => content.push_str(&format!("{g} g\n")),
                Color::Rgb(r, g, b) => content.push_str(&format!("{r} {g} {b} rg\n")),
                Color::Cmyk(c, m, y, k) => content.push_str(&format!("{c} {m} {y} {k} k\n")),
            }

            // Begin text
            content.push_str("BT\n");
            content.push_str(&format!(
                "/{} {} Tf\n",
                self.font.pdf_name(),
                self.font_size
            ));

            // Calculate text position
            let padding = 2.0;
            let text_y = (height - self.font_size) / 2.0 + self.font_size * 0.3;

            let text_x = match self.justification {
                1 => width / 2.0,     // Center (would need text width calculation)
                2 => width - padding, // Right
                _ => padding,         // Left
            };

            content.push_str(&format!("{text_x} {text_y} Td\n"));

            // Show text (escape special characters)
            let escaped_text = text
                .replace('\\', "\\\\")
                .replace('(', "\\(")
                .replace(')', "\\)");
            content.push_str(&format!("({escaped_text}) Tj\n"));

            // End text
            content.push_str("ET\n");
        }

        // Restore graphics state
        content.push_str("Q\n");

        // Create resources dictionary
        let mut resources = Dictionary::new();

        // Add font resource
        let mut font_dict = Dictionary::new();
        let mut font_res = Dictionary::new();
        font_res.set("Type", Object::Name("Font".to_string()));
        font_res.set("Subtype", Object::Name("Type1".to_string()));
        font_res.set("BaseFont", Object::Name(self.font.pdf_name()));
        font_dict.set(self.font.pdf_name(), Object::Dictionary(font_res));
        resources.set("Font", Object::Dictionary(font_dict));

        let stream = AppearanceStream::new(content.into_bytes(), [0.0, 0.0, width, height])
            .with_resources(resources);

        Ok(stream)
    }
}

/// Checkbox appearance generator
pub struct CheckBoxAppearance {
    /// Check mark style
    pub check_style: CheckStyle,
    /// Check color
    pub check_color: Color,
}

/// Style of check mark
#[derive(Debug, Clone, Copy)]
pub enum CheckStyle {
    /// Check mark (✓)
    Check,
    /// Cross (✗)
    Cross,
    /// Square (■)
    Square,
    /// Circle (●)
    Circle,
    /// Star (★)
    Star,
}

impl Default for CheckBoxAppearance {
    fn default() -> Self {
        Self {
            check_style: CheckStyle::Check,
            check_color: Color::black(),
        }
    }
}

impl AppearanceGenerator for CheckBoxAppearance {
    fn generate_appearance(
        &self,
        widget: &Widget,
        value: Option<&str>,
        _state: AppearanceState,
    ) -> Result<AppearanceStream> {
        let width = widget.rect.upper_right.x - widget.rect.lower_left.x;
        let height = widget.rect.upper_right.y - widget.rect.lower_left.y;
        let is_checked = value.is_some_and(|v| v == "Yes" || v == "On" || v == "true");

        let mut content = String::new();

        // Save graphics state
        content.push_str("q\n");

        // Draw background
        if let Some(bg_color) = &widget.appearance.background_color {
            match bg_color {
                Color::Gray(g) => content.push_str(&format!("{g} g\n")),
                Color::Rgb(r, g, b) => content.push_str(&format!("{r} {g} {b} rg\n")),
                Color::Cmyk(c, m, y, k) => content.push_str(&format!("{c} {m} {y} {k} k\n")),
            }
            content.push_str(&format!("0 0 {width} {height} re f\n"));
        }

        // Draw border
        if let Some(border_color) = &widget.appearance.border_color {
            match border_color {
                Color::Gray(g) => content.push_str(&format!("{g} G\n")),
                Color::Rgb(r, g, b) => content.push_str(&format!("{r} {g} {b} RG\n")),
                Color::Cmyk(c, m, y, k) => content.push_str(&format!("{c} {m} {y} {k} K\n")),
            }
            content.push_str(&format!("{} w\n", widget.appearance.border_width));
            content.push_str(&format!("0 0 {width} {height} re S\n"));
        }

        // Draw check mark if checked
        if is_checked {
            // Set check color
            match self.check_color {
                Color::Gray(g) => content.push_str(&format!("{g} g\n")),
                Color::Rgb(r, g, b) => content.push_str(&format!("{r} {g} {b} rg\n")),
                Color::Cmyk(c, m, y, k) => content.push_str(&format!("{c} {m} {y} {k} k\n")),
            }

            let inset = width * 0.2;

            match self.check_style {
                CheckStyle::Check => {
                    // Draw check mark path
                    content.push_str(&format!("{} {} m\n", inset, height * 0.5));
                    content.push_str(&format!("{} {} l\n", width * 0.4, inset));
                    content.push_str(&format!("{} {} l\n", width - inset, height - inset));
                    content.push_str("3 w S\n");
                }
                CheckStyle::Cross => {
                    // Draw X
                    content.push_str(&format!("{inset} {inset} m\n"));
                    content.push_str(&format!("{} {} l\n", width - inset, height - inset));
                    content.push_str(&format!("{} {inset} m\n", width - inset));
                    content.push_str(&format!("{inset} {} l\n", height - inset));
                    content.push_str("2 w S\n");
                }
                CheckStyle::Square => {
                    // Draw filled square
                    content.push_str(&format!(
                        "{inset} {inset} {} {} re f\n",
                        width - 2.0 * inset,
                        height - 2.0 * inset
                    ));
                }
                CheckStyle::Circle => {
                    // Draw filled circle (simplified)
                    let cx = width / 2.0;
                    let cy = height / 2.0;
                    let r = (width.min(height) - 2.0 * inset) / 2.0;

                    // Use Bézier curves to approximate circle
                    let k = 0.552284749831;
                    content.push_str(&format!("{} {} m\n", cx + r, cy));
                    content.push_str(&format!(
                        "{} {} {} {} {} {} c\n",
                        cx + r,
                        cy + k * r,
                        cx + k * r,
                        cy + r,
                        cx,
                        cy + r
                    ));
                    content.push_str(&format!(
                        "{} {} {} {} {} {} c\n",
                        cx - k * r,
                        cy + r,
                        cx - r,
                        cy + k * r,
                        cx - r,
                        cy
                    ));
                    content.push_str(&format!(
                        "{} {} {} {} {} {} c\n",
                        cx - r,
                        cy - k * r,
                        cx - k * r,
                        cy - r,
                        cx,
                        cy - r
                    ));
                    content.push_str(&format!(
                        "{} {} {} {} {} {} c\n",
                        cx + k * r,
                        cy - r,
                        cx + r,
                        cy - k * r,
                        cx + r,
                        cy
                    ));
                    content.push_str("f\n");
                }
                CheckStyle::Star => {
                    // Draw 5-pointed star (simplified)
                    let cx = width / 2.0;
                    let cy = height / 2.0;
                    let r = (width.min(height) - 2.0 * inset) / 2.0;

                    // Star points (simplified)
                    for i in 0..5 {
                        let angle = std::f64::consts::PI * 2.0 * i as f64 / 5.0
                            - std::f64::consts::PI / 2.0;
                        let x = cx + r * angle.cos();
                        let y = cy + r * angle.sin();

                        if i == 0 {
                            content.push_str(&format!("{x} {y} m\n"));
                        } else {
                            content.push_str(&format!("{x} {y} l\n"));
                        }
                    }
                    content.push_str("f\n");
                }
            }
        }

        // Restore graphics state
        content.push_str("Q\n");

        let stream = AppearanceStream::new(content.into_bytes(), [0.0, 0.0, width, height]);

        Ok(stream)
    }
}

/// Radio button appearance generator
pub struct RadioButtonAppearance {
    /// Button color when selected
    pub selected_color: Color,
}

impl Default for RadioButtonAppearance {
    fn default() -> Self {
        Self {
            selected_color: Color::black(),
        }
    }
}

impl AppearanceGenerator for RadioButtonAppearance {
    fn generate_appearance(
        &self,
        widget: &Widget,
        value: Option<&str>,
        _state: AppearanceState,
    ) -> Result<AppearanceStream> {
        let width = widget.rect.upper_right.x - widget.rect.lower_left.x;
        let height = widget.rect.upper_right.y - widget.rect.lower_left.y;
        let is_selected = value.is_some_and(|v| v == "Yes" || v == "On" || v == "true");

        let mut content = String::new();

        // Save graphics state
        content.push_str("q\n");

        // Draw background circle
        if let Some(bg_color) = &widget.appearance.background_color {
            match bg_color {
                Color::Gray(g) => content.push_str(&format!("{g} g\n")),
                Color::Rgb(r, g, b) => content.push_str(&format!("{r} {g} {b} rg\n")),
                Color::Cmyk(c, m, y, k) => content.push_str(&format!("{c} {m} {y} {k} k\n")),
            }
        } else {
            content.push_str("1 g\n"); // White background
        }

        let cx = width / 2.0;
        let cy = height / 2.0;
        let r = width.min(height) / 2.0 - widget.appearance.border_width;

        // Draw outer circle
        let k = 0.552284749831;
        content.push_str(&format!("{} {} m\n", cx + r, cy));
        content.push_str(&format!(
            "{} {} {} {} {} {} c\n",
            cx + r,
            cy + k * r,
            cx + k * r,
            cy + r,
            cx,
            cy + r
        ));
        content.push_str(&format!(
            "{} {} {} {} {} {} c\n",
            cx - k * r,
            cy + r,
            cx - r,
            cy + k * r,
            cx - r,
            cy
        ));
        content.push_str(&format!(
            "{} {} {} {} {} {} c\n",
            cx - r,
            cy - k * r,
            cx - k * r,
            cy - r,
            cx,
            cy - r
        ));
        content.push_str(&format!(
            "{} {} {} {} {} {} c\n",
            cx + k * r,
            cy - r,
            cx + r,
            cy - k * r,
            cx + r,
            cy
        ));
        content.push_str("f\n");

        // Draw border
        if let Some(border_color) = &widget.appearance.border_color {
            match border_color {
                Color::Gray(g) => content.push_str(&format!("{g} G\n")),
                Color::Rgb(r, g, b) => content.push_str(&format!("{r} {g} {b} RG\n")),
                Color::Cmyk(c, m, y, k) => content.push_str(&format!("{c} {m} {y} {k} K\n")),
            }
            content.push_str(&format!("{} w\n", widget.appearance.border_width));

            content.push_str(&format!("{} {} m\n", cx + r, cy));
            content.push_str(&format!(
                "{} {} {} {} {} {} c\n",
                cx + r,
                cy + k * r,
                cx + k * r,
                cy + r,
                cx,
                cy + r
            ));
            content.push_str(&format!(
                "{} {} {} {} {} {} c\n",
                cx - k * r,
                cy + r,
                cx - r,
                cy + k * r,
                cx - r,
                cy
            ));
            content.push_str(&format!(
                "{} {} {} {} {} {} c\n",
                cx - r,
                cy - k * r,
                cx - k * r,
                cy - r,
                cx,
                cy - r
            ));
            content.push_str(&format!(
                "{} {} {} {} {} {} c\n",
                cx + k * r,
                cy - r,
                cx + r,
                cy - k * r,
                cx + r,
                cy
            ));
            content.push_str("S\n");
        }

        // Draw inner dot if selected
        if is_selected {
            match self.selected_color {
                Color::Gray(g) => content.push_str(&format!("{g} g\n")),
                Color::Rgb(r, g, b) => content.push_str(&format!("{r} {g} {b} rg\n")),
                Color::Cmyk(c, m, y, k) => content.push_str(&format!("{c} {m} {y} {k} k\n")),
            }

            let inner_r = r * 0.4;
            content.push_str(&format!("{} {} m\n", cx + inner_r, cy));
            content.push_str(&format!(
                "{} {} {} {} {} {} c\n",
                cx + inner_r,
                cy + k * inner_r,
                cx + k * inner_r,
                cy + inner_r,
                cx,
                cy + inner_r
            ));
            content.push_str(&format!(
                "{} {} {} {} {} {} c\n",
                cx - k * inner_r,
                cy + inner_r,
                cx - inner_r,
                cy + k * inner_r,
                cx - inner_r,
                cy
            ));
            content.push_str(&format!(
                "{} {} {} {} {} {} c\n",
                cx - inner_r,
                cy - k * inner_r,
                cx - k * inner_r,
                cy - inner_r,
                cx,
                cy - inner_r
            ));
            content.push_str(&format!(
                "{} {} {} {} {} {} c\n",
                cx + k * inner_r,
                cy - inner_r,
                cx + inner_r,
                cy - k * inner_r,
                cx + inner_r,
                cy
            ));
            content.push_str("f\n");
        }

        // Restore graphics state
        content.push_str("Q\n");

        let stream = AppearanceStream::new(content.into_bytes(), [0.0, 0.0, width, height]);

        Ok(stream)
    }
}

/// Push button appearance generator
pub struct PushButtonAppearance {
    /// Button label
    pub label: String,
    /// Label font
    pub font: Font,
    /// Font size
    pub font_size: f64,
    /// Text color
    pub text_color: Color,
}

impl Default for PushButtonAppearance {
    fn default() -> Self {
        Self {
            label: String::new(),
            font: Font::Helvetica,
            font_size: 12.0,
            text_color: Color::black(),
        }
    }
}

impl AppearanceGenerator for PushButtonAppearance {
    fn generate_appearance(
        &self,
        widget: &Widget,
        _value: Option<&str>,
        state: AppearanceState,
    ) -> Result<AppearanceStream> {
        let width = widget.rect.upper_right.x - widget.rect.lower_left.x;
        let height = widget.rect.upper_right.y - widget.rect.lower_left.y;

        let mut content = String::new();

        // Save graphics state
        content.push_str("q\n");

        // Draw background with different colors for different states
        let bg_color = match state {
            AppearanceState::Down => Color::gray(0.8),
            AppearanceState::Rollover => Color::gray(0.95),
            AppearanceState::Normal => widget
                .appearance
                .background_color
                .unwrap_or(Color::gray(0.9)),
        };

        match bg_color {
            Color::Gray(g) => content.push_str(&format!("{g} g\n")),
            Color::Rgb(r, g, b) => content.push_str(&format!("{r} {g} {b} rg\n")),
            Color::Cmyk(c, m, y, k) => content.push_str(&format!("{c} {m} {y} {k} k\n")),
        }
        content.push_str(&format!("0 0 {width} {height} re f\n"));

        // Draw beveled border for button appearance
        if matches!(widget.appearance.border_style, BorderStyle::Beveled) {
            // Light edge (top and left)
            content.push_str("0.9 G\n");
            content.push_str("2 w\n");
            content.push_str(&format!("0 {height} m {width} {height} l\n"));
            content.push_str(&format!("{width} {height} l {width} 0 l S\n"));

            // Dark edge (bottom and right)
            content.push_str("0.3 G\n");
            content.push_str(&format!("0 0 m {width} 0 l\n"));
            content.push_str(&format!("0 0 l 0 {height} l S\n"));
        } else {
            // Regular border
            if let Some(border_color) = &widget.appearance.border_color {
                match border_color {
                    Color::Gray(g) => content.push_str(&format!("{g} G\n")),
                    Color::Rgb(r, g, b) => content.push_str(&format!("{r} {g} {b} RG\n")),
                    Color::Cmyk(c, m, y, k) => content.push_str(&format!("{c} {m} {y} {k} K\n")),
                }
                content.push_str(&format!("{} w\n", widget.appearance.border_width));
                content.push_str(&format!("0 0 {width} {height} re S\n"));
            }
        }

        // Draw label text
        if !self.label.is_empty() {
            match self.text_color {
                Color::Gray(g) => content.push_str(&format!("{g} g\n")),
                Color::Rgb(r, g, b) => content.push_str(&format!("{r} {g} {b} rg\n")),
                Color::Cmyk(c, m, y, k) => content.push_str(&format!("{c} {m} {y} {k} k\n")),
            }

            content.push_str("BT\n");
            content.push_str(&format!(
                "/{} {} Tf\n",
                self.font.pdf_name(),
                self.font_size
            ));

            // Center text (simplified - would need actual text width calculation)
            let text_x = width / 4.0; // Approximate centering
            let text_y = (height - self.font_size) / 2.0 + self.font_size * 0.3;

            content.push_str(&format!("{text_x} {text_y} Td\n"));

            let escaped_label = self
                .label
                .replace('\\', "\\\\")
                .replace('(', "\\(")
                .replace(')', "\\)");
            content.push_str(&format!("({escaped_label}) Tj\n"));

            content.push_str("ET\n");
        }

        // Restore graphics state
        content.push_str("Q\n");

        // Create resources dictionary
        let mut resources = Dictionary::new();

        // Add font resource
        let mut font_dict = Dictionary::new();
        let mut font_res = Dictionary::new();
        font_res.set("Type", Object::Name("Font".to_string()));
        font_res.set("Subtype", Object::Name("Type1".to_string()));
        font_res.set("BaseFont", Object::Name(self.font.pdf_name()));
        font_dict.set(self.font.pdf_name(), Object::Dictionary(font_res));
        resources.set("Font", Object::Dictionary(font_dict));

        let stream = AppearanceStream::new(content.into_bytes(), [0.0, 0.0, width, height])
            .with_resources(resources);

        Ok(stream)
    }
}

/// Generate default appearance stream for a field type
pub fn generate_default_appearance(
    field_type: FieldType,
    widget: &Widget,
    value: Option<&str>,
) -> Result<AppearanceStream> {
    match field_type {
        FieldType::Text => {
            let generator = TextFieldAppearance::default();
            generator.generate_appearance(widget, value, AppearanceState::Normal)
        }
        FieldType::Button => {
            // For now, default to checkbox appearance
            // In a real implementation, we'd need additional context to determine button type
            let generator = CheckBoxAppearance::default();
            generator.generate_appearance(widget, value, AppearanceState::Normal)
        }
        FieldType::Choice => {
            // Use text field appearance for choice fields (simplified)
            let generator = TextFieldAppearance::default();
            generator.generate_appearance(widget, value, AppearanceState::Normal)
        }
        FieldType::Signature => {
            // Use empty appearance for signature fields
            let width = widget.rect.upper_right.x - widget.rect.lower_left.x;
            let height = widget.rect.upper_right.y - widget.rect.lower_left.y;
            Ok(AppearanceStream::new(
                b"q\nQ\n".to_vec(),
                [0.0, 0.0, width, height],
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::{Point, Rectangle};

    #[test]
    fn test_appearance_state_names() {
        assert_eq!(AppearanceState::Normal.pdf_name(), "N");
        assert_eq!(AppearanceState::Rollover.pdf_name(), "R");
        assert_eq!(AppearanceState::Down.pdf_name(), "D");
    }

    #[test]
    fn test_appearance_stream_creation() {
        let content = b"q\n1 0 0 RG\n0 0 100 50 re S\nQ\n";
        let stream = AppearanceStream::new(content.to_vec(), [0.0, 0.0, 100.0, 50.0]);

        assert_eq!(stream.content, content);
        assert_eq!(stream.bbox, [0.0, 0.0, 100.0, 50.0]);
        assert!(stream.resources.is_empty());
    }

    #[test]
    fn test_appearance_stream_with_resources() {
        let mut resources = Dictionary::new();
        resources.set("Font", Object::Name("F1".to_string()));

        let content = b"BT\n/F1 12 Tf\n(Test) Tj\nET\n";
        let stream = AppearanceStream::new(content.to_vec(), [0.0, 0.0, 100.0, 50.0])
            .with_resources(resources.clone());

        assert_eq!(stream.resources, resources);
    }

    #[test]
    fn test_appearance_dictionary() {
        let mut app_dict = AppearanceDictionary::new();

        let normal_stream = AppearanceStream::new(b"normal".to_vec(), [0.0, 0.0, 10.0, 10.0]);
        let down_stream = AppearanceStream::new(b"down".to_vec(), [0.0, 0.0, 10.0, 10.0]);

        app_dict.set_appearance(AppearanceState::Normal, normal_stream.clone());
        app_dict.set_appearance(AppearanceState::Down, down_stream);

        assert!(app_dict.get_appearance(AppearanceState::Normal).is_some());
        assert!(app_dict.get_appearance(AppearanceState::Down).is_some());
        assert!(app_dict.get_appearance(AppearanceState::Rollover).is_none());
    }

    #[test]
    fn test_text_field_appearance() {
        let widget = Widget::new(Rectangle {
            lower_left: Point { x: 0.0, y: 0.0 },
            upper_right: Point { x: 200.0, y: 30.0 },
        });

        let generator = TextFieldAppearance::default();
        let result =
            generator.generate_appearance(&widget, Some("Test Text"), AppearanceState::Normal);

        assert!(result.is_ok());
        let stream = result.unwrap();
        assert_eq!(stream.bbox, [0.0, 0.0, 200.0, 30.0]);

        let content = String::from_utf8_lossy(&stream.content);
        assert!(content.contains("BT"));
        assert!(content.contains("(Test Text) Tj"));
        assert!(content.contains("ET"));
    }

    #[test]
    fn test_checkbox_appearance_checked() {
        let widget = Widget::new(Rectangle {
            lower_left: Point { x: 0.0, y: 0.0 },
            upper_right: Point { x: 20.0, y: 20.0 },
        });

        let generator = CheckBoxAppearance::default();
        let result = generator.generate_appearance(&widget, Some("Yes"), AppearanceState::Normal);

        assert!(result.is_ok());
        let stream = result.unwrap();
        let content = String::from_utf8_lossy(&stream.content);

        // Should contain check mark drawing commands
        assert!(content.contains(" m"));
        assert!(content.contains(" l"));
        assert!(content.contains(" S"));
    }

    #[test]
    fn test_checkbox_appearance_unchecked() {
        let widget = Widget::new(Rectangle {
            lower_left: Point { x: 0.0, y: 0.0 },
            upper_right: Point { x: 20.0, y: 20.0 },
        });

        let generator = CheckBoxAppearance::default();
        let result = generator.generate_appearance(&widget, Some("No"), AppearanceState::Normal);

        assert!(result.is_ok());
        let stream = result.unwrap();
        let content = String::from_utf8_lossy(&stream.content);

        // Should not contain complex drawing for check mark
        assert!(content.contains("q"));
        assert!(content.contains("Q"));
    }

    #[test]
    fn test_radio_button_appearance() {
        let widget = Widget::new(Rectangle {
            lower_left: Point { x: 0.0, y: 0.0 },
            upper_right: Point { x: 20.0, y: 20.0 },
        });

        let generator = RadioButtonAppearance::default();
        let result = generator.generate_appearance(&widget, Some("Yes"), AppearanceState::Normal);

        assert!(result.is_ok());
        let stream = result.unwrap();
        let content = String::from_utf8_lossy(&stream.content);

        // Should contain circle drawing commands (Bézier curves)
        assert!(
            content.contains(" c"),
            "Content should contain curve commands"
        );
        assert!(
            content.contains("f\n"),
            "Content should contain fill commands"
        );
    }

    #[test]
    fn test_push_button_appearance() {
        let mut generator = PushButtonAppearance::default();
        generator.label = "Click Me".to_string();

        let widget = Widget::new(Rectangle {
            lower_left: Point { x: 0.0, y: 0.0 },
            upper_right: Point { x: 100.0, y: 30.0 },
        });

        let result = generator.generate_appearance(&widget, None, AppearanceState::Normal);

        assert!(result.is_ok());
        let stream = result.unwrap();
        let content = String::from_utf8_lossy(&stream.content);

        assert!(content.contains("(Click Me) Tj"));
        assert!(!stream.resources.is_empty());
    }

    #[test]
    fn test_push_button_states() {
        let generator = PushButtonAppearance::default();
        let widget = Widget::new(Rectangle {
            lower_left: Point { x: 0.0, y: 0.0 },
            upper_right: Point { x: 100.0, y: 30.0 },
        });

        // Test different states produce different appearances
        let normal = generator
            .generate_appearance(&widget, None, AppearanceState::Normal)
            .unwrap();
        let down = generator
            .generate_appearance(&widget, None, AppearanceState::Down)
            .unwrap();
        let rollover = generator
            .generate_appearance(&widget, None, AppearanceState::Rollover)
            .unwrap();

        // Content should be different for different states (different background colors)
        assert_ne!(normal.content, down.content);
        assert_ne!(normal.content, rollover.content);
        assert_ne!(down.content, rollover.content);
    }

    #[test]
    fn test_check_styles() {
        let widget = Widget::new(Rectangle {
            lower_left: Point { x: 0.0, y: 0.0 },
            upper_right: Point { x: 20.0, y: 20.0 },
        });

        // Test different check styles
        for style in [
            CheckStyle::Check,
            CheckStyle::Cross,
            CheckStyle::Square,
            CheckStyle::Circle,
            CheckStyle::Star,
        ] {
            let mut generator = CheckBoxAppearance::default();
            generator.check_style = style;

            let result =
                generator.generate_appearance(&widget, Some("Yes"), AppearanceState::Normal);

            assert!(result.is_ok(), "Failed for style {:?}", style);
        }
    }
}
