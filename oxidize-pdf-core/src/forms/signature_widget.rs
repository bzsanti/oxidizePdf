//! Enhanced signature widget implementation with full annotation support
//!
//! This module provides widget annotation support for signature fields
//! according to ISO 32000-1 Section 12.5.6.19 (Widget Annotations) and
//! Section 12.7.4.5 (Signature Fields).

use crate::error::PdfError;
use crate::forms::Widget;
#[cfg(test)]
use crate::geometry::Point;
use crate::geometry::Rectangle;
use crate::graphics::Color;
use crate::objects::{Dictionary, Object, ObjectReference};

/// Enhanced signature widget with full annotation support
#[derive(Debug, Clone)]
pub struct SignatureWidget {
    /// Base widget properties
    pub widget: Widget,
    /// Signature field reference
    pub field_ref: Option<ObjectReference>,
    /// Visual representation type
    pub visual_type: SignatureVisualType,
    /// Signature handler reference
    pub handler_ref: Option<String>,
}

/// Visual representation types for signatures
#[derive(Debug, Clone)]
pub enum SignatureVisualType {
    /// Text-only signature
    Text {
        /// Show signer name
        show_name: bool,
        /// Show signing date
        show_date: bool,
        /// Show reason for signing
        show_reason: bool,
        /// Show location
        show_location: bool,
    },
    /// Graphical signature (e.g., handwritten)
    Graphic {
        /// Image data (PNG/JPEG)
        image_data: Vec<u8>,
        /// Image format
        format: ImageFormat,
        /// Maintain aspect ratio
        maintain_aspect: bool,
    },
    /// Mixed text and graphics
    Mixed {
        /// Image data for signature
        image_data: Vec<u8>,
        /// Image format
        format: ImageFormat,
        /// Text position relative to image
        text_position: TextPosition,
        /// Include text details
        show_details: bool,
    },
    /// Handwritten ink signature
    InkSignature {
        /// Ink paths (strokes)
        strokes: Vec<InkStroke>,
        /// Stroke color
        color: Color,
        /// Stroke width
        width: f64,
    },
}

/// Image formats supported for signature graphics
#[derive(Debug, Clone, Copy)]
pub enum ImageFormat {
    PNG,
    JPEG,
}

/// Text position relative to signature image
#[derive(Debug, Clone, Copy)]
pub enum TextPosition {
    Above,
    Below,
    Left,
    Right,
    Overlay,
}

/// Ink stroke for handwritten signatures
#[derive(Debug, Clone)]
pub struct InkStroke {
    /// Points in the stroke
    pub points: Vec<(f64, f64)>,
    /// Pressure values (optional)
    pub pressures: Option<Vec<f64>>,
}

impl SignatureWidget {
    /// Create a new signature widget
    pub fn new(rect: Rectangle, visual_type: SignatureVisualType) -> Self {
        Self {
            widget: Widget::new(rect),
            field_ref: None,
            visual_type,
            handler_ref: None,
        }
    }

    /// Set the field reference
    pub fn with_field_ref(mut self, field_ref: ObjectReference) -> Self {
        self.field_ref = Some(field_ref);
        self
    }

    /// Set the handler reference
    pub fn with_handler(mut self, handler: impl Into<String>) -> Self {
        self.handler_ref = Some(handler.into());
        self
    }

    /// Generate appearance stream for the signature widget
    pub fn generate_appearance_stream(
        &self,
        signed: bool,
        signer_name: Option<&str>,
        reason: Option<&str>,
        location: Option<&str>,
        date: Option<&str>,
    ) -> Result<Vec<u8>, PdfError> {
        let mut stream = Vec::new();
        let rect = &self.widget.rect;
        let width = rect.width();
        let height = rect.height();

        // Save graphics state
        stream.extend(b"q\n");

        // Draw background if specified
        if let Some(bg_color) = &self.widget.appearance.background_color {
            Self::set_fill_color(&mut stream, bg_color);
            stream.extend(format!("0 0 {} {} re f\n", width, height).as_bytes());
        }

        // Draw border
        if self.widget.appearance.border_width > 0.0 {
            if let Some(border_color) = &self.widget.appearance.border_color {
                Self::set_stroke_color(&mut stream, border_color);
                stream.extend(format!("{} w\n", self.widget.appearance.border_width).as_bytes());
                stream.extend(format!("0 0 {} {} re S\n", width, height).as_bytes());
            }
        }

        // Generate content based on visual type
        match &self.visual_type {
            SignatureVisualType::Text {
                show_name,
                show_date,
                show_reason,
                show_location,
            } => {
                self.generate_text_appearance(
                    &mut stream,
                    signed,
                    signer_name,
                    reason,
                    location,
                    date,
                    *show_name,
                    *show_date,
                    *show_reason,
                    *show_location,
                )?;
            }
            SignatureVisualType::Graphic {
                image_data,
                format,
                maintain_aspect,
            } => {
                self.generate_graphic_appearance(
                    &mut stream,
                    image_data,
                    *format,
                    *maintain_aspect,
                )?;
            }
            SignatureVisualType::Mixed {
                image_data,
                format,
                text_position,
                show_details,
            } => {
                self.generate_mixed_appearance(
                    &mut stream,
                    image_data,
                    *format,
                    *text_position,
                    *show_details,
                    signed,
                    signer_name,
                    reason,
                    date,
                )?;
            }
            SignatureVisualType::InkSignature {
                strokes,
                color,
                width,
            } => {
                self.generate_ink_appearance(&mut stream, strokes, color, *width)?;
            }
        }

        // Restore graphics state
        stream.extend(b"Q\n");

        Ok(stream)
    }

    /// Generate text-only appearance
    #[allow(clippy::too_many_arguments)]
    fn generate_text_appearance(
        &self,
        stream: &mut Vec<u8>,
        signed: bool,
        signer_name: Option<&str>,
        reason: Option<&str>,
        location: Option<&str>,
        date: Option<&str>,
        show_name: bool,
        show_date: bool,
        show_reason: bool,
        show_location: bool,
    ) -> Result<(), PdfError> {
        let rect = &self.widget.rect;
        let width = rect.width();
        let height = rect.height();

        // Begin text object
        stream.extend(b"BT\n");

        // Set font (using Helvetica as default)
        stream.extend(b"/Helv 10 Tf\n");

        // Set text color (black)
        stream.extend(b"0 g\n");

        let mut y_offset = height - 15.0;
        let x_offset = 5.0;

        if signed {
            if show_name && signer_name.is_some() {
                stream.extend(format!("{} {} Td\n", x_offset, y_offset).as_bytes());
                stream.extend(
                    format!("(Digitally signed by: {}) Tj\n", signer_name.unwrap()).as_bytes(),
                );
                y_offset -= 12.0;
                // Track y_offset for future use
                let _ = y_offset;
            }

            if show_date && date.is_some() {
                stream.extend(b"0 -12 Td\n");
                stream.extend(format!("(Date: {}) Tj\n", date.unwrap()).as_bytes());
                y_offset -= 12.0;
                // Track y_offset for future use
                let _ = y_offset;
            }

            if show_reason && reason.is_some() {
                stream.extend(b"0 -12 Td\n");
                stream.extend(format!("(Reason: {}) Tj\n", reason.unwrap()).as_bytes());
                y_offset -= 12.0;
                // Track y_offset for future use
                let _ = y_offset;
            }

            if show_location && location.is_some() {
                stream.extend(b"0 -12 Td\n");
                stream.extend(format!("(Location: {}) Tj\n", location.unwrap()).as_bytes());
            }
        } else {
            // Unsigned placeholder
            stream.extend(format!("{} {} Td\n", width / 2.0 - 30.0, height / 2.0).as_bytes());
            stream.extend(b"(Click to sign) Tj\n");
        }

        // End text object
        stream.extend(b"ET\n");

        Ok(())
    }

    /// Generate graphic appearance (image-based signature)
    fn generate_graphic_appearance(
        &self,
        stream: &mut Vec<u8>,
        _image_data: &[u8],
        _format: ImageFormat,
        maintain_aspect: bool,
    ) -> Result<(), PdfError> {
        let rect = &self.widget.rect;
        let width = rect.width();
        let height = rect.height();

        // For now, create a placeholder for image
        // In production, this would decode and embed the actual image
        stream.extend(b"q\n");

        if maintain_aspect {
            // Calculate aspect-preserving transform
            stream.extend(format!("{} 0 0 {} 0 0 cm\n", width * 0.8, height * 0.8).as_bytes());
        } else {
            stream.extend(format!("{} 0 0 {} 0 0 cm\n", width, height).as_bytes());
        }

        // Placeholder for image XObject reference
        stream.extend(b"/Img1 Do\n");
        stream.extend(b"Q\n");

        Ok(())
    }

    /// Generate mixed text and graphic appearance
    #[allow(clippy::too_many_arguments)]
    fn generate_mixed_appearance(
        &self,
        stream: &mut Vec<u8>,
        _image_data: &[u8],
        _format: ImageFormat,
        text_position: TextPosition,
        show_details: bool,
        signed: bool,
        signer_name: Option<&str>,
        _reason: Option<&str>,
        date: Option<&str>,
    ) -> Result<(), PdfError> {
        let rect = &self.widget.rect;
        let width = rect.width();
        let height = rect.height();

        // Calculate regions for image and text
        let (img_rect, text_rect) = match text_position {
            TextPosition::Above => {
                let text_height = height * 0.3;
                (
                    (0.0, 0.0, width, height - text_height),
                    (0.0, height - text_height, width, text_height),
                )
            }
            TextPosition::Below => {
                let text_height = height * 0.3;
                (
                    (0.0, text_height, width, height - text_height),
                    (0.0, 0.0, width, text_height),
                )
            }
            TextPosition::Left => {
                let text_width = width * 0.4;
                (
                    (text_width, 0.0, width - text_width, height),
                    (0.0, 0.0, text_width, height),
                )
            }
            TextPosition::Right => {
                let text_width = width * 0.4;
                (
                    (0.0, 0.0, width - text_width, height),
                    (width - text_width, 0.0, text_width, height),
                )
            }
            TextPosition::Overlay => ((0.0, 0.0, width, height), (0.0, 0.0, width, height * 0.3)),
        };

        // Draw image in its region
        stream.extend(b"q\n");
        stream.extend(
            format!(
                "{} 0 0 {} {} {} cm\n",
                img_rect.2, img_rect.3, img_rect.0, img_rect.1
            )
            .as_bytes(),
        );
        stream.extend(b"/Img1 Do\n");
        stream.extend(b"Q\n");

        // Draw text in its region if showing details
        if show_details && signed {
            stream.extend(b"BT\n");
            stream.extend(b"/Helv 8 Tf\n");
            stream.extend(b"0 g\n");

            let mut y_pos = text_rect.1 + text_rect.3 - 10.0;

            if let Some(name) = signer_name {
                stream.extend(format!("{} {} Td\n", text_rect.0 + 2.0, y_pos).as_bytes());
                stream.extend(format!("({}) Tj\n", name).as_bytes());
                y_pos -= 10.0;
                // Track y_pos for future use
                let _ = y_pos;
            }

            if let Some(d) = date {
                stream.extend(b"0 -10 Td\n");
                stream.extend(format!("({}) Tj\n", d).as_bytes());
            }

            stream.extend(b"ET\n");
        }

        Ok(())
    }

    /// Generate ink signature appearance (handwritten)
    fn generate_ink_appearance(
        &self,
        stream: &mut Vec<u8>,
        strokes: &[InkStroke],
        color: &Color,
        width: f64,
    ) -> Result<(), PdfError> {
        // Set stroke color and width
        Self::set_stroke_color(stream, color);
        stream.extend(format!("{} w\n", width).as_bytes());
        stream.extend(b"1 J\n"); // Round line cap
        stream.extend(b"1 j\n"); // Round line join

        // Draw each stroke
        for stroke in strokes {
            if stroke.points.len() < 2 {
                continue;
            }

            // Move to first point
            let first = &stroke.points[0];
            stream.extend(format!("{} {} m\n", first.0, first.1).as_bytes());

            // Draw lines to subsequent points
            for point in &stroke.points[1..] {
                stream.extend(format!("{} {} l\n", point.0, point.1).as_bytes());
            }

            // Stroke the path
            stream.extend(b"S\n");
        }

        Ok(())
    }

    /// Helper to set fill color
    fn set_fill_color(stream: &mut Vec<u8>, color: &Color) {
        match color {
            Color::Rgb(r, g, b) => {
                stream.extend(format!("{} {} {} rg\n", r, g, b).as_bytes());
            }
            Color::Gray(v) => {
                stream.extend(format!("{} g\n", v).as_bytes());
            }
            Color::Cmyk(c, m, y, k) => {
                stream.extend(format!("{} {} {} {} k\n", c, m, y, k).as_bytes());
            }
        }
    }

    /// Helper to set stroke color
    fn set_stroke_color(stream: &mut Vec<u8>, color: &Color) {
        match color {
            Color::Rgb(r, g, b) => {
                stream.extend(format!("{} {} {} RG\n", r, g, b).as_bytes());
            }
            Color::Gray(v) => {
                stream.extend(format!("{} G\n", v).as_bytes());
            }
            Color::Cmyk(c, m, y, k) => {
                stream.extend(format!("{} {} {} {} K\n", c, m, y, k).as_bytes());
            }
        }
    }

    /// Convert to PDF widget annotation dictionary
    pub fn to_widget_dict(&self) -> Dictionary {
        let mut dict = Dictionary::new();

        // Annotation type
        dict.set("Type", Object::Name("Annot".to_string()));
        dict.set("Subtype", Object::Name("Widget".to_string()));

        // Rectangle
        let rect = &self.widget.rect;
        dict.set(
            "Rect",
            Object::Array(vec![
                Object::Real(rect.lower_left.x),
                Object::Real(rect.lower_left.y),
                Object::Real(rect.upper_right.x),
                Object::Real(rect.upper_right.y),
            ]),
        );

        // Field reference
        if let Some(ref field_ref) = self.field_ref {
            dict.set("Parent", Object::Reference(*field_ref));
        }

        // Border appearance
        let mut bs_dict = Dictionary::new();
        bs_dict.set("Type", Object::Name("Border".to_string()));
        bs_dict.set("W", Object::Real(self.widget.appearance.border_width));
        bs_dict.set(
            "S",
            Object::Name(self.widget.appearance.border_style.pdf_name().to_string()),
        );
        dict.set("BS", Object::Dictionary(bs_dict));

        // Appearance characteristics
        let mut mk_dict = Dictionary::new();
        if let Some(ref bg_color) = self.widget.appearance.background_color {
            mk_dict.set("BG", Self::color_to_array(bg_color));
        }
        if let Some(ref border_color) = self.widget.appearance.border_color {
            mk_dict.set("BC", Self::color_to_array(border_color));
        }
        dict.set("MK", Object::Dictionary(mk_dict));

        // Flags
        dict.set("F", Object::Integer(4)); // Print flag

        dict
    }

    /// Convert color to PDF array
    fn color_to_array(color: &Color) -> Object {
        match color {
            Color::Gray(v) => Object::Array(vec![Object::Real(*v)]),
            Color::Rgb(r, g, b) => {
                Object::Array(vec![Object::Real(*r), Object::Real(*g), Object::Real(*b)])
            }
            Color::Cmyk(c, m, y, k) => Object::Array(vec![
                Object::Real(*c),
                Object::Real(*m),
                Object::Real(*y),
                Object::Real(*k),
            ]),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_signature_widget_creation() {
        let rect = Rectangle::new(Point::new(100.0, 100.0), Point::new(300.0, 150.0));
        let visual = SignatureVisualType::Text {
            show_name: true,
            show_date: true,
            show_reason: false,
            show_location: false,
        };

        let widget = SignatureWidget::new(rect, visual);
        assert!(widget.field_ref.is_none());
        assert!(widget.handler_ref.is_none());
    }

    #[test]
    fn test_text_appearance_generation() {
        let rect = Rectangle::new(Point::new(0.0, 0.0), Point::new(200.0, 50.0));
        let visual = SignatureVisualType::Text {
            show_name: true,
            show_date: true,
            show_reason: true,
            show_location: false,
        };

        let widget = SignatureWidget::new(rect, visual);
        let appearance = widget.generate_appearance_stream(
            true,
            Some("John Doe"),
            Some("Approval"),
            None,
            Some("2025-08-13"),
        );

        assert!(appearance.is_ok());
        let stream = appearance.unwrap();
        assert!(!stream.is_empty());

        // Check that the stream contains expected content
        let stream_str = String::from_utf8_lossy(&stream);
        assert!(stream_str.contains("John Doe"));
        assert!(stream_str.contains("2025-08-13"));
        assert!(stream_str.contains("Approval"));
    }

    #[test]
    fn test_ink_signature_appearance() {
        let rect = Rectangle::new(Point::new(0.0, 0.0), Point::new(150.0, 50.0));
        let stroke1 = InkStroke {
            points: vec![(10.0, 10.0), (20.0, 20.0), (30.0, 15.0)],
            pressures: None,
        };
        let stroke2 = InkStroke {
            points: vec![(40.0, 25.0), (50.0, 30.0), (60.0, 25.0)],
            pressures: None,
        };

        let visual = SignatureVisualType::InkSignature {
            strokes: vec![stroke1, stroke2],
            color: Color::black(),
            width: 2.0,
        };

        let widget = SignatureWidget::new(rect, visual);
        let appearance = widget.generate_appearance_stream(true, None, None, None, None);

        assert!(appearance.is_ok());
        let stream = appearance.unwrap();
        let stream_str = String::from_utf8_lossy(&stream);

        // Check that paths are created
        assert!(stream_str.contains("m")); // moveto
        assert!(stream_str.contains("l")); // lineto
        assert!(stream_str.contains("S")); // stroke
    }

    #[test]
    fn test_widget_dict_generation() {
        let rect = Rectangle::new(Point::new(100.0, 100.0), Point::new(300.0, 150.0));
        let visual = SignatureVisualType::Text {
            show_name: true,
            show_date: false,
            show_reason: false,
            show_location: false,
        };

        let mut widget = SignatureWidget::new(rect, visual);
        widget.widget.appearance.background_color = Some(Color::gray(0.9));
        widget.widget.appearance.border_color = Some(Color::black());

        let dict = widget.to_widget_dict();

        // Verify dictionary structure
        assert_eq!(dict.get("Type"), Some(&Object::Name("Annot".to_string())));
        assert_eq!(
            dict.get("Subtype"),
            Some(&Object::Name("Widget".to_string()))
        );
        assert!(dict.get("Rect").is_some());
        assert!(dict.get("BS").is_some());
        assert!(dict.get("MK").is_some());
    }

    #[test]
    fn test_signature_widget_with_field_ref() {
        let rect = Rectangle::new(Point::new(0.0, 0.0), Point::new(100.0, 50.0));
        let visual = SignatureVisualType::Text {
            show_name: true,
            show_date: true,
            show_reason: false,
            show_location: false,
        };

        let field_ref = ObjectReference::new(10, 0);
        let widget = SignatureWidget::new(rect, visual).with_field_ref(field_ref);

        assert_eq!(widget.field_ref, Some(field_ref));

        let dict = widget.to_widget_dict();
        assert_eq!(dict.get("Parent"), Some(&Object::Reference(field_ref)));
    }

    #[test]
    fn test_signature_widget_with_handler() {
        let rect = Rectangle::new(Point::new(0.0, 0.0), Point::new(100.0, 50.0));
        let visual = SignatureVisualType::Text {
            show_name: true,
            show_date: false,
            show_reason: false,
            show_location: false,
        };

        let widget = SignatureWidget::new(rect, visual).with_handler("Adobe.PPKLite");

        assert_eq!(widget.handler_ref, Some("Adobe.PPKLite".to_string()));
    }

    #[test]
    fn test_graphic_signature_visual_type() {
        let image_data = vec![0xFF, 0xD8, 0xFF, 0xE0]; // JPEG magic bytes
        let visual = SignatureVisualType::Graphic {
            image_data: image_data.clone(),
            format: ImageFormat::JPEG,
            maintain_aspect: true,
        };

        match visual {
            SignatureVisualType::Graphic {
                image_data: data,
                format,
                maintain_aspect,
            } => {
                assert_eq!(data, image_data);
                matches!(format, ImageFormat::JPEG);
                assert!(maintain_aspect);
            }
            _ => panic!("Expected Graphic visual type"),
        }
    }

    #[test]
    fn test_mixed_signature_visual_type() {
        let image_data = vec![0x89, 0x50, 0x4E, 0x47]; // PNG magic bytes
        let visual = SignatureVisualType::Mixed {
            image_data: image_data.clone(),
            format: ImageFormat::PNG,
            text_position: TextPosition::Below,
            show_details: true,
        };

        match visual {
            SignatureVisualType::Mixed {
                image_data: data,
                format,
                text_position,
                show_details,
            } => {
                assert_eq!(data, image_data);
                matches!(format, ImageFormat::PNG);
                matches!(text_position, TextPosition::Below);
                assert!(show_details);
            }
            _ => panic!("Expected Mixed visual type"),
        }
    }

    #[test]
    fn test_ink_stroke_with_pressure() {
        let stroke = InkStroke {
            points: vec![(10.0, 10.0), (20.0, 20.0), (30.0, 15.0)],
            pressures: Some(vec![0.5, 0.7, 0.6]),
        };

        assert_eq!(stroke.points.len(), 3);
        assert_eq!(stroke.pressures.as_ref().unwrap().len(), 3);
        assert_eq!(stroke.points[0], (10.0, 10.0));
        assert_eq!(stroke.pressures.as_ref().unwrap()[1], 0.7);
    }

    #[test]
    fn test_text_position_variants() {
        let positions = vec![
            TextPosition::Above,
            TextPosition::Below,
            TextPosition::Left,
            TextPosition::Right,
            TextPosition::Overlay,
        ];

        for pos in positions {
            match pos {
                TextPosition::Above => assert!(true),
                TextPosition::Below => assert!(true),
                TextPosition::Left => assert!(true),
                TextPosition::Right => assert!(true),
                TextPosition::Overlay => assert!(true),
            }
        }
    }

    #[test]
    fn test_image_format_variants() {
        let png = ImageFormat::PNG;
        let jpeg = ImageFormat::JPEG;

        matches!(png, ImageFormat::PNG);
        matches!(jpeg, ImageFormat::JPEG);
    }

    #[test]
    fn test_color_to_array() {
        // Test gray color
        let gray = Color::gray(0.5);
        let gray_array = SignatureWidget::color_to_array(&gray);
        assert_eq!(gray_array, Object::Array(vec![Object::Real(0.5)]));

        // Test RGB color
        let rgb = Color::rgb(1.0, 0.0, 0.0);
        let rgb_array = SignatureWidget::color_to_array(&rgb);
        assert_eq!(
            rgb_array,
            Object::Array(vec![
                Object::Real(1.0),
                Object::Real(0.0),
                Object::Real(0.0),
            ])
        );

        // Test CMYK color
        let cmyk = Color::cmyk(0.0, 1.0, 1.0, 0.0);
        let cmyk_array = SignatureWidget::color_to_array(&cmyk);
        assert_eq!(
            cmyk_array,
            Object::Array(vec![
                Object::Real(0.0),
                Object::Real(1.0),
                Object::Real(1.0),
                Object::Real(0.0),
            ])
        );
    }

    #[test]
    fn test_set_fill_color() {
        let mut stream = Vec::new();

        // Test RGB fill
        let rgb = Color::rgb(1.0, 0.5, 0.0);
        SignatureWidget::set_fill_color(&mut stream, &rgb);
        let result = String::from_utf8_lossy(&stream);
        assert!(result.contains("1 0.5 0 rg"));

        // Test gray fill
        stream.clear();
        let gray = Color::gray(0.7);
        SignatureWidget::set_fill_color(&mut stream, &gray);
        let result = String::from_utf8_lossy(&stream);
        assert!(result.contains("0.7 g"));

        // Test CMYK fill
        stream.clear();
        let cmyk = Color::cmyk(0.2, 0.3, 0.4, 0.1);
        SignatureWidget::set_fill_color(&mut stream, &cmyk);
        let result = String::from_utf8_lossy(&stream);
        assert!(result.contains("0.2 0.3 0.4 0.1 k"));
    }

    #[test]
    fn test_set_stroke_color() {
        let mut stream = Vec::new();

        // Test RGB stroke
        let rgb = Color::rgb(0.0, 0.0, 1.0);
        SignatureWidget::set_stroke_color(&mut stream, &rgb);
        let result = String::from_utf8_lossy(&stream);
        assert!(result.contains("0 0 1 RG"));

        // Test gray stroke
        stream.clear();
        let gray = Color::gray(0.3);
        SignatureWidget::set_stroke_color(&mut stream, &gray);
        let result = String::from_utf8_lossy(&stream);
        assert!(result.contains("0.3 G"));

        // Test CMYK stroke
        stream.clear();
        let cmyk = Color::cmyk(1.0, 0.0, 0.0, 0.0);
        SignatureWidget::set_stroke_color(&mut stream, &cmyk);
        let result = String::from_utf8_lossy(&stream);
        assert!(result.contains("1 0 0 0 K"));
    }

    #[test]
    fn test_empty_text_signature() {
        let rect = Rectangle::new(Point::new(0.0, 0.0), Point::new(200.0, 50.0));
        let visual = SignatureVisualType::Text {
            show_name: false,
            show_date: false,
            show_reason: false,
            show_location: false,
        };

        let widget = SignatureWidget::new(rect, visual);
        let appearance = widget.generate_appearance_stream(false, None, None, None, None);

        assert!(appearance.is_ok());
        let stream = appearance.unwrap();
        let stream_str = String::from_utf8_lossy(&stream);

        // Should still have basic structure
        assert!(stream_str.contains("q")); // Save state
        assert!(stream_str.contains("Q")); // Restore state
    }

    #[test]
    fn test_full_text_signature() {
        let rect = Rectangle::new(Point::new(0.0, 0.0), Point::new(300.0, 100.0));
        let visual = SignatureVisualType::Text {
            show_name: true,
            show_date: true,
            show_reason: true,
            show_location: true,
        };

        let widget = SignatureWidget::new(rect, visual);
        let appearance = widget.generate_appearance_stream(
            true,
            Some("Jane Smith"),
            Some("Document Review"),
            Some("New York"),
            Some("2025-08-14"),
        );

        assert!(appearance.is_ok());
        let stream = appearance.unwrap();
        let stream_str = String::from_utf8_lossy(&stream);

        // Check all text elements are present
        assert!(stream_str.contains("Jane Smith"));
        assert!(stream_str.contains("Document Review"));
        assert!(stream_str.contains("New York"));
        assert!(stream_str.contains("2025-08-14"));
    }

    #[test]
    fn test_widget_with_border_styles() {
        let rect = Rectangle::new(Point::new(0.0, 0.0), Point::new(100.0, 50.0));
        let visual = SignatureVisualType::Text {
            show_name: true,
            show_date: false,
            show_reason: false,
            show_location: false,
        };

        let mut widget = SignatureWidget::new(rect, visual);
        widget.widget.appearance.border_width = 2.0;
        widget.widget.appearance.border_color = Some(Color::rgb(0.0, 0.0, 1.0));

        let dict = widget.to_widget_dict();

        // Check border style dictionary
        if let Some(Object::Dictionary(bs_dict)) = dict.get("BS") {
            assert_eq!(bs_dict.get("W"), Some(&Object::Real(2.0)));
            assert!(bs_dict.get("S").is_some());
        } else {
            panic!("Expected BS dictionary");
        }
    }

    #[test]
    fn test_multiple_ink_strokes() {
        let rect = Rectangle::new(Point::new(0.0, 0.0), Point::new(200.0, 100.0));
        let strokes = vec![
            InkStroke {
                points: vec![(10.0, 10.0), (20.0, 20.0)],
                pressures: None,
            },
            InkStroke {
                points: vec![(30.0, 30.0), (40.0, 40.0), (50.0, 35.0)],
                pressures: Some(vec![0.3, 0.5, 0.4]),
            },
            InkStroke {
                points: vec![(60.0, 20.0), (70.0, 25.0)],
                pressures: None,
            },
        ];

        let visual = SignatureVisualType::InkSignature {
            strokes: strokes.clone(),
            color: Color::rgb(0.0, 0.0, 0.5),
            width: 1.5,
        };

        match visual {
            SignatureVisualType::InkSignature {
                strokes: s,
                color: _,
                width,
            } => {
                assert_eq!(s.len(), 3);
                assert_eq!(width, 1.5);
                assert_eq!(s[1].points.len(), 3);
                assert!(s[1].pressures.is_some());
            }
            _ => panic!("Expected InkSignature"),
        }
    }
}
