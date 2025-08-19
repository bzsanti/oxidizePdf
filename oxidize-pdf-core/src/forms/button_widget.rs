//! Button field widget integration for ISO 32000-1 compliance
//!
//! This module provides complete widget annotation support for button fields
//! including checkboxes, radio buttons, and push buttons with proper appearance streams.

use crate::annotations::{Annotation, AnnotationType};
use crate::error::Result;
use crate::forms::{CheckBox, PushButton, RadioButton};
use crate::geometry::Rectangle;
use crate::graphics::Color;
use crate::objects::{Dictionary, Object, Stream};
use std::io::Write;

/// Button widget configuration
#[derive(Debug, Clone)]
pub struct ButtonWidget {
    /// Widget rectangle on page
    pub rect: Rectangle,
    /// Border width
    pub border_width: f64,
    /// Border color
    pub border_color: Color,
    /// Background color
    pub background_color: Option<Color>,
    /// Text color for captions
    pub text_color: Color,
    /// Font size for captions
    pub font_size: f64,
}

impl Default for ButtonWidget {
    fn default() -> Self {
        Self {
            rect: Rectangle::new((0.0, 0.0).into(), (100.0, 20.0).into()),
            border_width: 1.0,
            border_color: Color::rgb(0.0, 0.0, 0.0),
            background_color: Some(Color::rgb(1.0, 1.0, 1.0)),
            text_color: Color::rgb(0.0, 0.0, 0.0),
            font_size: 10.0,
        }
    }
}

impl ButtonWidget {
    /// Create a new button widget
    pub fn new(rect: Rectangle) -> Self {
        Self {
            rect,
            ..Default::default()
        }
    }

    /// Set border width
    pub fn with_border_width(mut self, width: f64) -> Self {
        self.border_width = width;
        self
    }

    /// Set border color
    pub fn with_border_color(mut self, color: Color) -> Self {
        self.border_color = color;
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

    /// Set font size
    pub fn with_font_size(mut self, size: f64) -> Self {
        self.font_size = size;
        self
    }
}

/// Create widget annotation for checkbox
pub fn create_checkbox_widget(checkbox: &CheckBox, widget: &ButtonWidget) -> Result<Annotation> {
    let mut annotation = Annotation::new(AnnotationType::Widget, widget.rect);

    // Set field reference
    annotation
        .properties
        .set("FT", Object::Name("Btn".to_string()));
    annotation
        .properties
        .set("T", Object::String(checkbox.name.clone()));

    // Set current state
    let state = if checkbox.checked {
        &checkbox.export_value
    } else {
        "Off"
    };
    annotation
        .properties
        .set("AS", Object::Name(state.to_string()));
    annotation
        .properties
        .set("V", Object::Name(state.to_string()));

    // Create appearance dictionary
    let mut ap_dict = Dictionary::new();

    // Normal appearance states
    let mut n_dict = Dictionary::new();

    // Create checked appearance
    let checked_stream = create_checkbox_appearance(widget, true)?;
    n_dict.set(
        &checkbox.export_value,
        Object::Stream(
            checked_stream.dictionary().clone(),
            checked_stream.data().to_vec(),
        ),
    );

    // Create unchecked appearance
    let unchecked_stream = create_checkbox_appearance(widget, false)?;
    n_dict.set(
        "Off",
        Object::Stream(
            unchecked_stream.dictionary().clone(),
            unchecked_stream.data().to_vec(),
        ),
    );

    ap_dict.set("N", Object::Dictionary(n_dict));
    annotation.properties.set("AP", Object::Dictionary(ap_dict));

    // Set widget flags
    let flags = 4; // Print flag
    annotation.properties.set("F", Object::Integer(flags));

    // Border style
    let mut bs_dict = Dictionary::new();
    bs_dict.set("W", Object::Real(widget.border_width));
    bs_dict.set("S", Object::Name("S".to_string())); // Solid
    annotation.properties.set("BS", Object::Dictionary(bs_dict));

    // Appearance characteristics
    let mut mk_dict = Dictionary::new();
    if let Some(bg) = &widget.background_color {
        mk_dict.set("BG", bg.to_pdf_array());
    }
    mk_dict.set("BC", widget.border_color.to_pdf_array());
    mk_dict.set("CA", Object::String("✓".to_string())); // Check mark
    annotation.properties.set("MK", Object::Dictionary(mk_dict));

    Ok(annotation)
}

/// Create widget annotation for radio button
pub fn create_radio_widget(
    radio: &RadioButton,
    widget: &ButtonWidget,
    option_index: usize,
) -> Result<Annotation> {
    let mut annotation = Annotation::new(AnnotationType::Widget, widget.rect);

    // Set field reference
    annotation
        .properties
        .set("FT", Object::Name("Btn".to_string()));
    annotation
        .properties
        .set("T", Object::String(radio.name.clone()));

    // Radio button flags
    let flags = (1 << 15) | 4; // Radio + Print
    annotation
        .properties
        .set("Ff", Object::Integer(flags as i64));

    // Get option value
    let (export_value, _label) = radio.options.get(option_index).ok_or_else(|| {
        crate::error::PdfError::InvalidStructure("Invalid radio option index".to_string())
    })?;

    // Set current state
    let state = if radio.selected == Some(option_index) {
        export_value.as_str()
    } else {
        "Off"
    };
    annotation
        .properties
        .set("AS", Object::Name(state.to_string()));

    // Create appearance dictionary
    let mut ap_dict = Dictionary::new();
    let mut n_dict = Dictionary::new();

    // Create selected appearance
    let selected_stream = create_radio_appearance(widget, true)?;
    n_dict.set(
        export_value,
        Object::Stream(
            selected_stream.dictionary().clone(),
            selected_stream.data().to_vec(),
        ),
    );

    // Create unselected appearance
    let unselected_stream = create_radio_appearance(widget, false)?;
    n_dict.set(
        "Off",
        Object::Stream(
            unselected_stream.dictionary().clone(),
            unselected_stream.data().to_vec(),
        ),
    );

    ap_dict.set("N", Object::Dictionary(n_dict));
    annotation.properties.set("AP", Object::Dictionary(ap_dict));

    // Border and appearance characteristics
    let mut bs_dict = Dictionary::new();
    bs_dict.set("W", Object::Real(widget.border_width));
    bs_dict.set("S", Object::Name("S".to_string()));
    annotation.properties.set("BS", Object::Dictionary(bs_dict));

    let mut mk_dict = Dictionary::new();
    if let Some(bg) = &widget.background_color {
        mk_dict.set("BG", bg.to_pdf_array());
    }
    mk_dict.set("BC", widget.border_color.to_pdf_array());
    mk_dict.set("CA", Object::String("●".to_string())); // Radio dot
    annotation.properties.set("MK", Object::Dictionary(mk_dict));

    Ok(annotation)
}

/// Create widget annotation for push button
pub fn create_pushbutton_widget(button: &PushButton, widget: &ButtonWidget) -> Result<Annotation> {
    let mut annotation = Annotation::new(AnnotationType::Widget, widget.rect);

    // Set field reference
    annotation
        .properties
        .set("FT", Object::Name("Btn".to_string()));
    annotation
        .properties
        .set("T", Object::String(button.name.clone()));

    // Push button flags
    let flags = (1 << 16) | 4; // Pushbutton + Print
    annotation
        .properties
        .set("Ff", Object::Integer(flags as i64));

    // Create appearance
    let mut ap_dict = Dictionary::new();
    let appearance_stream = create_pushbutton_appearance(widget, button.caption.as_deref())?;
    ap_dict.set(
        "N",
        Object::Stream(
            appearance_stream.dictionary().clone(),
            appearance_stream.data().to_vec(),
        ),
    );
    annotation.properties.set("AP", Object::Dictionary(ap_dict));

    // Border style
    let mut bs_dict = Dictionary::new();
    bs_dict.set("W", Object::Real(widget.border_width));
    bs_dict.set("S", Object::Name("B".to_string())); // Beveled
    annotation.properties.set("BS", Object::Dictionary(bs_dict));

    // Appearance characteristics
    let mut mk_dict = Dictionary::new();
    if let Some(bg) = &widget.background_color {
        mk_dict.set("BG", bg.to_pdf_array());
    }
    mk_dict.set("BC", widget.border_color.to_pdf_array());
    if let Some(caption) = &button.caption {
        mk_dict.set("CA", Object::String(caption.clone()));
    }
    annotation.properties.set("MK", Object::Dictionary(mk_dict));

    // Highlight mode
    annotation
        .properties
        .set("H", Object::Name("P".to_string())); // Push

    Ok(annotation)
}

/// Create checkbox appearance stream
fn create_checkbox_appearance(widget: &ButtonWidget, checked: bool) -> Result<Stream> {
    let mut content = Vec::new();
    let width = widget.rect.width();
    let height = widget.rect.height();

    // Draw background
    if let Some(bg) = &widget.background_color {
        match bg {
            Color::Rgb(r, g, b) => writeln!(&mut content, "{} {} {} rg", r, g, b)?,
            Color::Gray(g) => writeln!(&mut content, "{} g", g)?,
            Color::Cmyk(c, m, y, k) => writeln!(&mut content, "{} {} {} {} k", c, m, y, k)?,
        }
        writeln!(&mut content, "0 0 {} {} re f", width, height)?;
    }

    // Draw border
    match &widget.border_color {
        Color::Rgb(r, g, b) => writeln!(&mut content, "{} {} {} RG", r, g, b)?,
        Color::Gray(gray) => writeln!(&mut content, "{} G", gray)?,
        Color::Cmyk(c, m, y, k) => writeln!(&mut content, "{} {} {} {} K", c, m, y, k)?,
    }
    writeln!(&mut content, "{} w", widget.border_width)?;
    writeln!(&mut content, "0 0 {} {} re S", width, height)?;

    // Draw check mark if checked
    if checked {
        match &widget.text_color {
            Color::Rgb(r, g, b) => writeln!(&mut content, "{} {} {} RG", r, g, b)?,
            Color::Gray(gray) => writeln!(&mut content, "{} G", gray)?,
            Color::Cmyk(c, m, y, k) => writeln!(&mut content, "{} {} {} {} K", c, m, y, k)?,
        }
        writeln!(&mut content, "2 w")?;
        writeln!(&mut content, "1 J")?; // Round line cap

        // Draw check mark path
        let margin = width * 0.2;
        let x1 = margin;
        let y1 = height * 0.5;
        let x2 = width * 0.4;
        let y2 = margin;
        let x3 = width - margin;
        let y3 = height - margin;

        writeln!(&mut content, "{} {} m", x1, y1)?;
        writeln!(&mut content, "{} {} l", x2, y2)?;
        writeln!(&mut content, "{} {} l S", x3, y3)?;
    }

    let mut resources = Dictionary::new();
    resources.set(
        "ProcSet",
        Object::Array(vec![Object::Name("PDF".to_string())]),
    );
    let mut dict = Dictionary::new();
    dict.set("Resources", Object::Dictionary(resources));

    Ok(Stream::with_dictionary(dict, content))
}

/// Create radio button appearance stream
fn create_radio_appearance(widget: &ButtonWidget, selected: bool) -> Result<Stream> {
    let mut content = Vec::new();
    let width = widget.rect.width();
    let height = widget.rect.height();
    let radius = width.min(height) / 2.0;
    let center_x = width / 2.0;
    let center_y = height / 2.0;

    // Draw background circle
    if let Some(bg) = &widget.background_color {
        match bg {
            Color::Rgb(r, g, b) => writeln!(&mut content, "{} {} {} rg", r, g, b)?,
            Color::Gray(g) => writeln!(&mut content, "{} g", g)?,
            Color::Cmyk(c, m, y, k) => writeln!(&mut content, "{} {} {} {} k", c, m, y, k)?,
        }
        draw_circle(
            &mut content,
            center_x,
            center_y,
            radius - widget.border_width,
        )?;
        writeln!(&mut content, "f")?;
    }

    // Draw border circle
    match &widget.border_color {
        Color::Rgb(r, g, b) => writeln!(&mut content, "{} {} {} RG", r, g, b)?,
        Color::Gray(gray) => writeln!(&mut content, "{} G", gray)?,
        Color::Cmyk(c, m, y, k) => writeln!(&mut content, "{} {} {} {} K", c, m, y, k)?,
    }
    writeln!(&mut content, "{} w", widget.border_width)?;
    draw_circle(
        &mut content,
        center_x,
        center_y,
        radius - widget.border_width / 2.0,
    )?;
    writeln!(&mut content, "S")?;

    // Draw inner dot if selected
    if selected {
        match &widget.text_color {
            Color::Rgb(r, g, b) => writeln!(&mut content, "{} {} {} rg", r, g, b)?,
            Color::Gray(gray) => writeln!(&mut content, "{} g", gray)?,
            Color::Cmyk(c, m, y, k) => writeln!(&mut content, "{} {} {} {} k", c, m, y, k)?,
        }
        let dot_radius = radius * 0.4;
        draw_circle(&mut content, center_x, center_y, dot_radius)?;
        writeln!(&mut content, "f")?;
    }

    let mut resources = Dictionary::new();
    resources.set(
        "ProcSet",
        Object::Array(vec![Object::Name("PDF".to_string())]),
    );
    let mut dict = Dictionary::new();
    dict.set("Resources", Object::Dictionary(resources));

    Ok(Stream::with_dictionary(dict, content))
}

/// Create push button appearance stream
fn create_pushbutton_appearance(widget: &ButtonWidget, caption: Option<&str>) -> Result<Stream> {
    let mut content = Vec::new();
    let width = widget.rect.width();
    let height = widget.rect.height();

    // Draw background with beveled effect
    if let Some(bg) = &widget.background_color {
        // Main background
        match bg {
            Color::Rgb(r, g, b) => writeln!(&mut content, "{} {} {} rg", r, g, b)?,
            Color::Gray(g) => writeln!(&mut content, "{} g", g)?,
            Color::Cmyk(c, m, y, k) => writeln!(&mut content, "{} {} {} {} k", c, m, y, k)?,
        }
        writeln!(&mut content, "0 0 {} {} re f", width, height)?;

        // Top/left highlight (lighter)
        writeln!(&mut content, "0.9 0.9 0.9 RG")?;
        writeln!(&mut content, "2 w")?;
        writeln!(&mut content, "1 {} m", height - 1.0)?;
        writeln!(&mut content, "1 1 l")?;
        writeln!(&mut content, "{} 1 l S", width - 1.0)?;

        // Bottom/right shadow (darker)
        writeln!(&mut content, "0.5 0.5 0.5 RG")?;
        writeln!(&mut content, "{} 1 m", width - 1.0)?;
        writeln!(&mut content, "{} {} l", width - 1.0, height - 1.0)?;
        writeln!(&mut content, "1 {} l S", height - 1.0)?;
    }

    // Draw border
    match &widget.border_color {
        Color::Rgb(r, g, b) => writeln!(&mut content, "{} {} {} RG", r, g, b)?,
        Color::Gray(gray) => writeln!(&mut content, "{} G", gray)?,
        Color::Cmyk(c, m, y, k) => writeln!(&mut content, "{} {} {} {} K", c, m, y, k)?,
    }
    writeln!(&mut content, "{} w", widget.border_width)?;
    writeln!(&mut content, "0 0 {} {} re S", width, height)?;

    // Draw caption text
    if let Some(text) = caption {
        writeln!(&mut content, "BT")?;
        writeln!(&mut content, "/Helvetica {} Tf", widget.font_size)?;
        match &widget.text_color {
            Color::Rgb(r, g, b) => writeln!(&mut content, "{} {} {} rg", r, g, b)?,
            Color::Gray(gray) => writeln!(&mut content, "{} g", gray)?,
            Color::Cmyk(c, m, y, k) => writeln!(&mut content, "{} {} {} {} k", c, m, y, k)?,
        }

        // Center text
        let text_width = text.len() as f64 * widget.font_size * 0.5;
        let x = (width - text_width) / 2.0;
        let y = (height - widget.font_size) / 2.0;

        writeln!(&mut content, "{} {} Td", x, y)?;
        writeln!(&mut content, "({}) Tj", escape_pdf_string(text))?;
        writeln!(&mut content, "ET")?;
    }

    let mut resources = Dictionary::new();

    // Add font resources
    let mut fonts = Dictionary::new();
    let mut font_dict = Dictionary::new();
    font_dict.set("Type", Object::Name("Font".to_string()));
    font_dict.set("Subtype", Object::Name("Type1".to_string()));
    font_dict.set("BaseFont", Object::Name("Helvetica".to_string()));
    fonts.set("Helvetica", Object::Dictionary(font_dict));
    resources.set("Font", Object::Dictionary(fonts));

    resources.set(
        "ProcSet",
        Object::Array(vec![
            Object::Name("PDF".to_string()),
            Object::Name("Text".to_string()),
        ]),
    );

    let mut dict = Dictionary::new();
    dict.set("Resources", Object::Dictionary(resources));

    Ok(Stream::with_dictionary(dict, content))
}

/// Helper to draw a circle using Bézier curves
fn draw_circle<W: Write>(writer: &mut W, cx: f64, cy: f64, r: f64) -> Result<()> {
    let k = 0.5522847498; // Magic constant for circle approximation
    let dx = r * k;
    let dy = r * k;

    writeln!(writer, "{} {} m", cx + r, cy)?;
    writeln!(
        writer,
        "{} {} {} {} {} {} c",
        cx + r,
        cy + dy,
        cx + dx,
        cy + r,
        cx,
        cy + r
    )?;
    writeln!(
        writer,
        "{} {} {} {} {} {} c",
        cx - dx,
        cy + r,
        cx - r,
        cy + dy,
        cx - r,
        cy
    )?;
    writeln!(
        writer,
        "{} {} {} {} {} {} c",
        cx - r,
        cy - dy,
        cx - dx,
        cy - r,
        cx,
        cy - r
    )?;
    writeln!(
        writer,
        "{} {} {} {} {} {} c",
        cx + dx,
        cy - r,
        cx + r,
        cy - dy,
        cx + r,
        cy
    )?;

    Ok(())
}

/// Escape special characters in PDF strings
fn escape_pdf_string(s: &str) -> String {
    s.chars()
        .flat_map(|c| match c {
            '(' => vec!['\\', '('],
            ')' => vec!['\\', ')'],
            '\\' => vec!['\\', '\\'],
            _ => vec![c],
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_checkbox_widget() {
        let checkbox = CheckBox::new("agree").checked().with_export_value("Yes");

        let widget = ButtonWidget::new(Rectangle::new((0.0, 0.0).into(), (20.0, 20.0).into()));

        let annotation = create_checkbox_widget(&checkbox, &widget).unwrap();

        // Verify widget annotation properties
        assert_eq!(annotation.annotation_type, AnnotationType::Widget);
        assert!(annotation.properties.get("AP").is_some());
        assert!(annotation.properties.get("AS").is_some());
        assert_eq!(
            annotation.properties.get("AS"),
            Some(&Object::Name("Yes".to_string()))
        );
    }

    #[test]
    fn test_radio_widget() {
        let radio = RadioButton::new("size")
            .add_option("S", "Small")
            .add_option("M", "Medium")
            .add_option("L", "Large")
            .with_selected(1);

        let widget = ButtonWidget::new(Rectangle::new((0.0, 0.0).into(), (20.0, 20.0).into()));

        let annotation = create_radio_widget(&radio, &widget, 1).unwrap();

        // Verify radio button widget properties
        assert_eq!(annotation.annotation_type, AnnotationType::Widget);
        assert!(annotation.properties.get("AP").is_some());
        assert_eq!(
            annotation.properties.get("AS"),
            Some(&Object::Name("M".to_string()))
        );
    }

    #[test]
    fn test_pushbutton_widget() {
        let button = PushButton::new("submit").with_caption("Submit Form");

        let widget = ButtonWidget::new(Rectangle::new((0.0, 0.0).into(), (100.0, 30.0).into()));

        let annotation = create_pushbutton_widget(&button, &widget).unwrap();

        // Verify push button widget properties
        assert_eq!(annotation.annotation_type, AnnotationType::Widget);
        assert!(annotation.properties.get("AP").is_some());
        assert!(annotation.properties.get("MK").is_some());
    }

    #[test]
    fn test_widget_customization() {
        let widget = ButtonWidget::new(Rectangle::new((0.0, 0.0).into(), (50.0, 50.0).into()))
            .with_border_width(2.0)
            .with_border_color(Color::rgb(1.0, 0.0, 0.0))
            .with_background_color(Some(Color::rgb(0.9, 0.9, 1.0)))
            .with_text_color(Color::rgb(0.0, 0.0, 1.0))
            .with_font_size(12.0);

        assert_eq!(widget.border_width, 2.0);
        assert_eq!(widget.border_color, Color::rgb(1.0, 0.0, 0.0));
        assert_eq!(widget.font_size, 12.0);
    }
}
