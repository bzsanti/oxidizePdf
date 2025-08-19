//! Form field appearance streams per ISO 32000-1 §12.7.3.3
//!
//! This module handles the visual representation of form fields,
//! including text fields, checkboxes, radio buttons, and buttons.

use crate::error::Result;
use crate::graphics::Color;
use crate::objects::{Dictionary, Object, Stream};

/// Appearance characteristics for form fields
#[derive(Debug, Clone)]
pub struct AppearanceCharacteristics {
    /// Rotation of annotation (0, 90, 180, 270)
    pub rotation: i32,
    /// Border color (RGB)
    pub border_color: Option<Color>,
    /// Background color (RGB)
    pub background_color: Option<Color>,
    /// Normal caption
    pub normal_caption: Option<String>,
    /// Rollover caption
    pub rollover_caption: Option<String>,
    /// Down (pressed) caption
    pub down_caption: Option<String>,
    /// Normal icon
    pub normal_icon: Option<Stream>,
    /// Rollover icon
    pub rollover_icon: Option<Stream>,
    /// Down icon
    pub down_icon: Option<Stream>,
    /// Icon fit parameters
    pub icon_fit: Option<IconFit>,
    /// Text position relative to icon
    pub text_position: TextPosition,
}

impl Default for AppearanceCharacteristics {
    fn default() -> Self {
        Self {
            rotation: 0,
            border_color: None,
            background_color: None,
            normal_caption: None,
            rollover_caption: None,
            down_caption: None,
            normal_icon: None,
            rollover_icon: None,
            down_icon: None,
            icon_fit: None,
            text_position: TextPosition::CaptionOnly,
        }
    }
}

/// Icon fit parameters
#[derive(Debug, Clone)]
pub struct IconFit {
    /// Scale type
    pub scale_type: IconScaleType,
    /// Scale when type
    pub scale_when: IconScaleWhen,
    /// Horizontal alignment (0.0 = left, 1.0 = right)
    pub align_x: f64,
    /// Vertical alignment (0.0 = bottom, 1.0 = top)
    pub align_y: f64,
    /// Fit to bounds ignoring aspect ratio
    pub fit_bounds: bool,
}

/// How to scale icon
#[derive(Debug, Clone, PartialEq)]
pub enum IconScaleType {
    /// Always scale
    Always,
    /// Scale only when icon is bigger
    Bigger,
    /// Scale only when icon is smaller
    Smaller,
    /// Never scale
    Never,
}

/// When to scale icon
#[derive(Debug, Clone, PartialEq)]
pub enum IconScaleWhen {
    /// Always scale
    Always,
    /// Scale only when icon is bigger than bounds
    IconBigger,
    /// Scale only when icon is smaller than bounds
    IconSmaller,
    /// Never scale
    Never,
}

/// Text position relative to icon
#[derive(Debug, Clone, PartialEq)]
pub enum TextPosition {
    /// No icon, caption only
    CaptionOnly,
    /// No caption, icon only
    IconOnly,
    /// Caption below icon
    CaptionBelowIcon,
    /// Caption above icon
    CaptionAboveIcon,
    /// Caption to the right of icon
    CaptionRightIcon,
    /// Caption to the left of icon
    CaptionLeftIcon,
    /// Caption overlaid on icon
    CaptionOverlayIcon,
}

impl AppearanceCharacteristics {
    /// Convert to PDF dictionary
    pub fn to_dict(&self) -> Dictionary {
        let mut dict = Dictionary::new();

        if self.rotation != 0 {
            dict.set("R", Object::Integer(self.rotation as i64));
        }

        if let Some(color) = &self.border_color {
            dict.set("BC", color.to_array());
        }

        if let Some(color) = &self.background_color {
            dict.set("BG", color.to_array());
        }

        if let Some(caption) = &self.normal_caption {
            dict.set("CA", Object::String(caption.clone()));
        }

        if let Some(caption) = &self.rollover_caption {
            dict.set("RC", Object::String(caption.clone()));
        }

        if let Some(caption) = &self.down_caption {
            dict.set("AC", Object::String(caption.clone()));
        }

        if let Some(fit) = &self.icon_fit {
            dict.set("IF", fit.to_dict());
        }

        dict.set("TP", Object::Integer(self.text_position.to_int()));

        dict
    }
}

impl IconFit {
    /// Convert to PDF dictionary
    pub fn to_dict(&self) -> Object {
        let mut dict = Dictionary::new();

        dict.set(
            "SW",
            Object::Name(
                match self.scale_when {
                    IconScaleWhen::Always => "A",
                    IconScaleWhen::IconBigger => "B",
                    IconScaleWhen::IconSmaller => "S",
                    IconScaleWhen::Never => "N",
                }
                .to_string(),
            ),
        );

        dict.set(
            "S",
            Object::Name(
                match self.scale_type {
                    IconScaleType::Always => "A",
                    IconScaleType::Bigger => "B",
                    IconScaleType::Smaller => "S",
                    IconScaleType::Never => "N",
                }
                .to_string(),
            ),
        );

        dict.set(
            "A",
            Object::Array(vec![Object::Real(self.align_x), Object::Real(self.align_y)]),
        );

        if self.fit_bounds {
            dict.set("FB", Object::Boolean(true));
        }

        Object::Dictionary(dict)
    }
}

impl TextPosition {
    pub fn to_int(&self) -> i64 {
        match self {
            TextPosition::CaptionOnly => 0,
            TextPosition::IconOnly => 1,
            TextPosition::CaptionBelowIcon => 2,
            TextPosition::CaptionAboveIcon => 3,
            TextPosition::CaptionRightIcon => 4,
            TextPosition::CaptionLeftIcon => 5,
            TextPosition::CaptionOverlayIcon => 6,
        }
    }
}

/// Text field appearance generator
pub struct FieldAppearanceGenerator {
    /// Field value
    pub value: String,
    /// Font to use
    pub font: String,
    /// Font size
    pub font_size: f64,
    /// Text color
    pub text_color: Color,
    /// Background color
    pub background_color: Option<Color>,
    /// Border color
    pub border_color: Option<Color>,
    /// Border width
    pub border_width: f64,
    /// Field rectangle [x1, y1, x2, y2]
    pub rect: [f64; 4],
    /// Text alignment
    pub alignment: TextAlignment,
    /// Multi-line field
    pub multiline: bool,
    /// Max length (for comb fields)
    pub max_length: Option<usize>,
    /// Comb field (evenly spaced characters)
    pub comb: bool,
}

/// Text alignment in fields
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TextAlignment {
    Left,
    Center,
    Right,
}

impl FieldAppearanceGenerator {
    /// Generate appearance stream for text field
    pub fn generate_text_field(&self) -> Result<Stream> {
        let mut ops = Vec::new();
        let width = self.rect[2] - self.rect[0];
        let height = self.rect[3] - self.rect[1];

        // Save graphics state
        ops.push("q".to_string());

        // Draw background if specified
        if let Some(bg_color) = &self.background_color {
            ops.push(format!(
                "{} {} {} rg",
                bg_color.r(),
                bg_color.g(),
                bg_color.b()
            ));
            ops.push(format!("0 0 {} {} re", width, height));
            ops.push("f".to_string());
        }

        // Draw border if specified
        if let Some(border_color) = &self.border_color {
            if self.border_width > 0.0 {
                ops.push(format!("{} w", self.border_width));
                ops.push(format!(
                    "{} {} {} RG",
                    border_color.r(),
                    border_color.g(),
                    border_color.b()
                ));
                ops.push(format!(
                    "{} {} {} {} re",
                    self.border_width / 2.0,
                    self.border_width / 2.0,
                    width - self.border_width,
                    height - self.border_width
                ));
                ops.push("S".to_string());
            }
        }

        // Begin text
        ops.push("BT".to_string());

        // Set font and size
        ops.push(format!("/{} {} Tf", self.font, self.font_size));

        // Set text color
        ops.push(format!(
            "{} {} {} rg",
            self.text_color.r(),
            self.text_color.g(),
            self.text_color.b()
        ));

        // Calculate text position
        let padding = 2.0;
        let text_y = height / 2.0 - self.font_size / 2.0;

        if self.comb && self.max_length.is_some() {
            // Comb field - evenly space characters
            let max_len = self.max_length.unwrap();
            let char_width = (width - 2.0 * padding) / max_len as f64;

            for (i, ch) in self.value.chars().take(max_len).enumerate() {
                let x = padding + (i as f64 + 0.5) * char_width;
                ops.push(format!("{} {} Td", x, text_y));
                ops.push(format!("({}) Tj", escape_string(&ch.to_string())));
                if i < self.value.len() - 1 {
                    ops.push(format!("{} 0 Td", -x));
                }
            }
        } else if self.multiline {
            // Multi-line text field
            let lines = self.value.lines();
            let line_height = self.font_size * 1.2;
            let mut y = height - padding - self.font_size;

            for line in lines {
                let x = match self.alignment {
                    TextAlignment::Left => padding,
                    TextAlignment::Center => width / 2.0,
                    TextAlignment::Right => width - padding,
                };

                ops.push(format!("{} {} Td", x, y));
                ops.push(format!("({}) Tj", escape_string(line)));

                y -= line_height;
                if y < padding {
                    break;
                }
            }
        } else {
            // Single line text field
            let x = match self.alignment {
                TextAlignment::Left => padding,
                TextAlignment::Center => width / 2.0,
                TextAlignment::Right => width - padding,
            };

            ops.push(format!("{} {} Td", x, text_y));
            ops.push(format!("({}) Tj", escape_string(&self.value)));
        }

        // End text
        ops.push("ET".to_string());

        // Restore graphics state
        ops.push("Q".to_string());

        let content = ops.join("\n");

        let mut stream = Stream::new(content.into_bytes());
        stream
            .dictionary_mut()
            .set("Type", Object::Name("XObject".to_string()));
        stream
            .dictionary_mut()
            .set("Subtype", Object::Name("Form".to_string()));
        stream.dictionary_mut().set(
            "BBox",
            Object::Array(vec![
                Object::Real(0.0),
                Object::Real(0.0),
                Object::Real(width),
                Object::Real(height),
            ]),
        );

        Ok(stream)
    }
}

/// Checkbox/Radio button appearance generator
pub struct ButtonAppearanceGenerator {
    /// Button style
    pub style: ButtonStyle,
    /// Size of the button
    pub size: f64,
    /// Border color
    pub border_color: Color,
    /// Background color
    pub background_color: Color,
    /// Check/dot color
    pub check_color: Color,
    /// Border width
    pub border_width: f64,
}

/// Button visual style
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ButtonStyle {
    /// Checkbox with checkmark
    Check,
    /// Checkbox with cross
    Cross,
    /// Checkbox with diamond
    Diamond,
    /// Checkbox with circle
    Circle,
    /// Checkbox with star
    Star,
    /// Checkbox with square
    Square,
    /// Radio button (circle with dot)
    Radio,
}

impl ButtonAppearanceGenerator {
    /// Generate appearance stream for checked state
    pub fn generate_checked(&self) -> Result<Stream> {
        let mut ops = Vec::new();

        // Save graphics state
        ops.push("q".to_string());

        // Draw background
        ops.push(format!(
            "{} {} {} rg",
            self.background_color.r(),
            self.background_color.g(),
            self.background_color.b()
        ));

        match self.style {
            ButtonStyle::Radio => {
                // Circle background
                self.draw_circle(&mut ops, self.size / 2.0, self.size / 2.0, self.size / 2.0);
                ops.push("f".to_string());

                // Draw border
                if self.border_width > 0.0 {
                    ops.push(format!("{} w", self.border_width));
                    ops.push(format!(
                        "{} {} {} RG",
                        self.border_color.r(),
                        self.border_color.g(),
                        self.border_color.b()
                    ));
                    ops.push("s".to_string());
                }

                // Draw dot
                let dot_size = self.size * 0.3;
                ops.push(format!(
                    "{} {} {} rg",
                    self.check_color.r(),
                    self.check_color.g(),
                    self.check_color.b()
                ));
                self.draw_circle(&mut ops, self.size / 2.0, self.size / 2.0, dot_size);
                ops.push("f".to_string());
            }
            _ => {
                // Rectangle background
                ops.push(format!("0 0 {} {} re", self.size, self.size));
                ops.push("f".to_string());

                // Draw border
                if self.border_width > 0.0 {
                    ops.push(format!("{} w", self.border_width));
                    ops.push(format!(
                        "{} {} {} RG",
                        self.border_color.r(),
                        self.border_color.g(),
                        self.border_color.b()
                    ));
                    ops.push(format!(
                        "{} {} {} {} re",
                        self.border_width / 2.0,
                        self.border_width / 2.0,
                        self.size - self.border_width,
                        self.size - self.border_width
                    ));
                    ops.push("S".to_string());
                }

                // Draw check mark based on style
                ops.push(format!(
                    "{} {} {} rg",
                    self.check_color.r(),
                    self.check_color.g(),
                    self.check_color.b()
                ));

                self.draw_check_style(&mut ops);
            }
        }

        // Restore graphics state
        ops.push("Q".to_string());

        let content = ops.join("\n");

        let mut stream = Stream::new(content.into_bytes());
        stream
            .dictionary_mut()
            .set("Type", Object::Name("XObject".to_string()));
        stream
            .dictionary_mut()
            .set("Subtype", Object::Name("Form".to_string()));
        stream.dictionary_mut().set(
            "BBox",
            Object::Array(vec![
                Object::Real(0.0),
                Object::Real(0.0),
                Object::Real(self.size),
                Object::Real(self.size),
            ]),
        );

        Ok(stream)
    }

    /// Generate appearance stream for unchecked state
    pub fn generate_unchecked(&self) -> Result<Stream> {
        let mut ops = Vec::new();

        // Save graphics state
        ops.push("q".to_string());

        // Draw background
        ops.push(format!(
            "{} {} {} rg",
            self.background_color.r(),
            self.background_color.g(),
            self.background_color.b()
        ));

        if self.style == ButtonStyle::Radio {
            // Circle background
            self.draw_circle(&mut ops, self.size / 2.0, self.size / 2.0, self.size / 2.0);
            ops.push("f".to_string());

            // Draw border
            if self.border_width > 0.0 {
                ops.push(format!("{} w", self.border_width));
                ops.push(format!(
                    "{} {} {} RG",
                    self.border_color.r(),
                    self.border_color.g(),
                    self.border_color.b()
                ));
                ops.push("s".to_string());
            }
        } else {
            // Rectangle background
            ops.push(format!("0 0 {} {} re", self.size, self.size));
            ops.push("f".to_string());

            // Draw border
            if self.border_width > 0.0 {
                ops.push(format!("{} w", self.border_width));
                ops.push(format!(
                    "{} {} {} RG",
                    self.border_color.r(),
                    self.border_color.g(),
                    self.border_color.b()
                ));
                ops.push(format!(
                    "{} {} {} {} re",
                    self.border_width / 2.0,
                    self.border_width / 2.0,
                    self.size - self.border_width,
                    self.size - self.border_width
                ));
                ops.push("S".to_string());
            }
        }

        // Restore graphics state
        ops.push("Q".to_string());

        let content = ops.join("\n");

        let mut stream = Stream::new(content.into_bytes());
        stream
            .dictionary_mut()
            .set("Type", Object::Name("XObject".to_string()));
        stream
            .dictionary_mut()
            .set("Subtype", Object::Name("Form".to_string()));
        stream.dictionary_mut().set(
            "BBox",
            Object::Array(vec![
                Object::Real(0.0),
                Object::Real(0.0),
                Object::Real(self.size),
                Object::Real(self.size),
            ]),
        );

        Ok(stream)
    }

    fn draw_circle(&self, ops: &mut Vec<String>, cx: f64, cy: f64, r: f64) {
        // Draw circle using Bezier curves
        let k = 0.552284749831; // Magic constant for circle approximation

        ops.push(format!("{} {} m", cx + r, cy));
        ops.push(format!(
            "{} {} {} {} {} {} c",
            cx + r,
            cy + r * k,
            cx + r * k,
            cy + r,
            cx,
            cy + r
        ));
        ops.push(format!(
            "{} {} {} {} {} {} c",
            cx - r * k,
            cy + r,
            cx - r,
            cy + r * k,
            cx - r,
            cy
        ));
        ops.push(format!(
            "{} {} {} {} {} {} c",
            cx - r,
            cy - r * k,
            cx - r * k,
            cy - r,
            cx,
            cy - r
        ));
        ops.push(format!(
            "{} {} {} {} {} {} c",
            cx + r * k,
            cy - r,
            cx + r,
            cy - r * k,
            cx + r,
            cy
        ));
    }

    fn draw_check_style(&self, ops: &mut Vec<String>) {
        match self.style {
            ButtonStyle::Check => {
                // Draw checkmark
                ops.push(format!("{} w", self.size * 0.1));
                ops.push(format!("{} {} m", self.size * 0.2, self.size * 0.5));
                ops.push(format!("{} {} l", self.size * 0.4, self.size * 0.3));
                ops.push(format!("{} {} l", self.size * 0.8, self.size * 0.7));
                ops.push("S".to_string());
            }
            ButtonStyle::Cross => {
                // Draw X
                ops.push(format!("{} w", self.size * 0.1));
                ops.push(format!("{} {} m", self.size * 0.2, self.size * 0.2));
                ops.push(format!("{} {} l", self.size * 0.8, self.size * 0.8));
                ops.push(format!("{} {} m", self.size * 0.2, self.size * 0.8));
                ops.push(format!("{} {} l", self.size * 0.8, self.size * 0.2));
                ops.push("S".to_string());
            }
            ButtonStyle::Diamond => {
                // Draw diamond
                ops.push(format!("{} {} m", self.size * 0.5, self.size * 0.8));
                ops.push(format!("{} {} l", self.size * 0.8, self.size * 0.5));
                ops.push(format!("{} {} l", self.size * 0.5, self.size * 0.2));
                ops.push(format!("{} {} l", self.size * 0.2, self.size * 0.5));
                ops.push("f".to_string());
            }
            ButtonStyle::Circle => {
                // Draw filled circle
                self.draw_circle(ops, self.size / 2.0, self.size / 2.0, self.size * 0.3);
                ops.push("f".to_string());
            }
            ButtonStyle::Star => {
                // Draw star
                let cx = self.size / 2.0;
                let cy = self.size / 2.0;
                let r = self.size * 0.4;

                ops.push(format!("{} {} m", cx, cy + r));
                for i in 1..10 {
                    let angle = i as f64 * 36.0 * std::f64::consts::PI / 180.0;
                    let radius = if i % 2 == 0 { r } else { r * 0.5 };
                    let x = cx + radius * angle.sin();
                    let y = cy + radius * angle.cos();
                    ops.push(format!("{} {} l", x, y));
                }
                ops.push("f".to_string());
            }
            ButtonStyle::Square => {
                // Draw filled square
                let inset = self.size * 0.25;
                ops.push(format!(
                    "{} {} {} {} re",
                    inset,
                    inset,
                    self.size - 2.0 * inset,
                    self.size - 2.0 * inset
                ));
                ops.push("f".to_string());
            }
            _ => {}
        }
    }
}

/// Escape special characters in PDF strings
fn escape_string(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            '(' => "\\(".to_string(),
            ')' => "\\)".to_string(),
            '\\' => "\\\\".to_string(),
            '\n' => "\\n".to_string(),
            '\r' => "\\r".to_string(),
            '\t' => "\\t".to_string(),
            c => c.to_string(),
        })
        .collect()
}

/// Push button appearance generator
pub struct PushButtonAppearanceGenerator {
    /// Button caption
    pub caption: String,
    /// Font to use
    pub font: String,
    /// Font size
    pub font_size: f64,
    /// Text color
    pub text_color: Color,
    /// Background color
    pub background_color: Color,
    /// Border color
    pub border_color: Color,
    /// Border width
    pub border_width: f64,
    /// Button rectangle [width, height]
    pub size: [f64; 2],
    /// Border style
    pub border_style: ButtonBorderStyle,
}

/// Border style for buttons
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ButtonBorderStyle {
    /// Solid border
    Solid,
    /// Dashed border
    Dashed,
    /// Beveled (3D raised)
    Beveled,
    /// Inset (3D pressed)
    Inset,
    /// Underline only
    Underline,
}

impl PushButtonAppearanceGenerator {
    /// Generate normal appearance
    pub fn generate_normal(&self) -> Result<Stream> {
        self.generate_appearance(false)
    }

    /// Generate rollover appearance
    pub fn generate_rollover(&self) -> Result<Stream> {
        // Make slightly lighter for rollover
        let mut appearance = self.clone();
        appearance.background_color = appearance.background_color.lighten(0.1);
        appearance.generate_appearance(false)
    }

    /// Generate down (pressed) appearance
    pub fn generate_down(&self) -> Result<Stream> {
        self.generate_appearance(true)
    }

    fn generate_appearance(&self, pressed: bool) -> Result<Stream> {
        let mut ops = Vec::new();
        let [width, height] = self.size;

        // Save graphics state
        ops.push("q".to_string());

        // Draw background
        let bg_color = if pressed {
            self.background_color.darken(0.1)
        } else {
            self.background_color
        };

        ops.push(format!(
            "{} {} {} rg",
            bg_color.r(),
            bg_color.g(),
            bg_color.b()
        ));
        ops.push(format!("0 0 {} {} re", width, height));
        ops.push("f".to_string());

        // Draw border based on style
        self.draw_border(&mut ops, width, height, pressed);

        // Draw caption text
        if !self.caption.is_empty() {
            ops.push("BT".to_string());
            ops.push(format!("/{} {} Tf", self.font, self.font_size));
            ops.push(format!(
                "{} {} {} rg",
                self.text_color.r(),
                self.text_color.g(),
                self.text_color.b()
            ));

            // Center text
            let text_x = width / 2.0;
            let text_y = height / 2.0 - self.font_size / 2.0;

            ops.push(format!("{} {} Td", text_x, text_y));
            ops.push(format!("({}) Tj", escape_string(&self.caption)));
            ops.push("ET".to_string());
        }

        // Restore graphics state
        ops.push("Q".to_string());

        let content = ops.join("\n");

        let mut stream = Stream::new(content.into_bytes());
        stream
            .dictionary_mut()
            .set("Type", Object::Name("XObject".to_string()));
        stream
            .dictionary_mut()
            .set("Subtype", Object::Name("Form".to_string()));
        stream.dictionary_mut().set(
            "BBox",
            Object::Array(vec![
                Object::Real(0.0),
                Object::Real(0.0),
                Object::Real(width),
                Object::Real(height),
            ]),
        );

        Ok(stream)
    }

    fn draw_border(&self, ops: &mut Vec<String>, width: f64, height: f64, pressed: bool) {
        match self.border_style {
            ButtonBorderStyle::Solid => {
                if self.border_width > 0.0 {
                    ops.push(format!("{} w", self.border_width));
                    ops.push(format!(
                        "{} {} {} RG",
                        self.border_color.r(),
                        self.border_color.g(),
                        self.border_color.b()
                    ));
                    ops.push(format!(
                        "{} {} {} {} re",
                        self.border_width / 2.0,
                        self.border_width / 2.0,
                        width - self.border_width,
                        height - self.border_width
                    ));
                    ops.push("S".to_string());
                }
            }
            ButtonBorderStyle::Dashed => {
                if self.border_width > 0.0 {
                    ops.push(format!("{} w", self.border_width));
                    ops.push("[3 3] 0 d".to_string()); // Dash pattern
                    ops.push(format!(
                        "{} {} {} RG",
                        self.border_color.r(),
                        self.border_color.g(),
                        self.border_color.b()
                    ));
                    ops.push(format!(
                        "{} {} {} {} re",
                        self.border_width / 2.0,
                        self.border_width / 2.0,
                        width - self.border_width,
                        height - self.border_width
                    ));
                    ops.push("S".to_string());
                }
            }
            ButtonBorderStyle::Beveled | ButtonBorderStyle::Inset => {
                let is_inset = self.border_style == ButtonBorderStyle::Inset || pressed;
                let light_color = if is_inset {
                    self.border_color.darken(0.3)
                } else {
                    self.border_color.lighten(0.3)
                };
                let dark_color = if is_inset {
                    self.border_color.lighten(0.3)
                } else {
                    self.border_color.darken(0.3)
                };

                // Top and left edges (light)
                ops.push(format!("{} w", self.border_width));
                ops.push(format!(
                    "{} {} {} RG",
                    light_color.r(),
                    light_color.g(),
                    light_color.b()
                ));
                ops.push(format!("{} {} m", 0.0, 0.0));
                ops.push(format!("{} {} l", 0.0, height));
                ops.push(format!("{} {} l", width, height));
                ops.push("S".to_string());

                // Bottom and right edges (dark)
                ops.push(format!(
                    "{} {} {} RG",
                    dark_color.r(),
                    dark_color.g(),
                    dark_color.b()
                ));
                ops.push(format!("{} {} m", width, height));
                ops.push(format!("{} {} l", width, 0.0));
                ops.push(format!("{} {} l", 0.0, 0.0));
                ops.push("S".to_string());
            }
            ButtonBorderStyle::Underline => {
                if self.border_width > 0.0 {
                    ops.push(format!("{} w", self.border_width));
                    ops.push(format!(
                        "{} {} {} RG",
                        self.border_color.r(),
                        self.border_color.g(),
                        self.border_color.b()
                    ));
                    ops.push(format!("{} {} m", 0.0, self.border_width / 2.0));
                    ops.push(format!("{} {} l", width, self.border_width / 2.0));
                    ops.push("S".to_string());
                }
            }
        }
    }
}

impl Clone for PushButtonAppearanceGenerator {
    fn clone(&self) -> Self {
        Self {
            caption: self.caption.clone(),
            font: self.font.clone(),
            font_size: self.font_size,
            text_color: self.text_color,
            background_color: self.background_color,
            border_color: self.border_color,
            border_width: self.border_width,
            size: self.size,
            border_style: self.border_style,
        }
    }
}

impl Color {
    pub fn lighten(&self, amount: f64) -> Color {
        Color::rgb(
            (self.r() + amount).min(1.0),
            (self.g() + amount).min(1.0),
            (self.b() + amount).min(1.0),
        )
    }

    pub fn darken(&self, amount: f64) -> Color {
        Color::rgb(
            (self.r() - amount).max(0.0),
            (self.g() - amount).max(0.0),
            (self.b() - amount).max(0.0),
        )
    }

    pub fn to_array(&self) -> Object {
        Object::Array(vec![
            Object::Real(self.r()),
            Object::Real(self.g()),
            Object::Real(self.b()),
        ])
    }
}
