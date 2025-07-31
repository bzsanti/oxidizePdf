//! Form field and widget definitions

use crate::geometry::Rectangle;
use crate::graphics::Color;
use crate::objects::{Dictionary, Object};

/// Field flags according to ISO 32000-1 Table 221
#[derive(Debug, Clone, Copy, Default)]
pub struct FieldFlags {
    /// Field is read-only
    pub read_only: bool,
    /// Field is required
    pub required: bool,
    /// Field should not be exported
    pub no_export: bool,
}

impl FieldFlags {
    /// Convert to PDF flags integer
    pub fn to_flags(&self) -> u32 {
        let mut flags = 0u32;
        if self.read_only {
            flags |= 1 << 0;
        }
        if self.required {
            flags |= 1 << 1;
        }
        if self.no_export {
            flags |= 1 << 2;
        }
        flags
    }
}

/// Field options
#[derive(Debug, Clone, Default)]
pub struct FieldOptions {
    /// Field flags
    pub flags: FieldFlags,
    /// Default appearance string
    pub default_appearance: Option<String>,
    /// Quadding (justification): 0=left, 1=center, 2=right
    pub quadding: Option<i32>,
}

/// Widget appearance settings
#[derive(Debug, Clone)]
pub struct WidgetAppearance {
    /// Border color
    pub border_color: Option<Color>,
    /// Background color
    pub background_color: Option<Color>,
    /// Border width
    pub border_width: f64,
    /// Border style: S (solid), D (dashed), B (beveled), I (inset), U (underline)
    pub border_style: BorderStyle,
}

/// Border style for widgets
#[derive(Debug, Clone, Copy)]
pub enum BorderStyle {
    /// Solid border
    Solid,
    /// Dashed border
    Dashed,
    /// Beveled border
    Beveled,
    /// Inset border
    Inset,
    /// Underline only
    Underline,
}

impl BorderStyle {
    /// Get PDF name
    pub fn pdf_name(&self) -> &'static str {
        match self {
            BorderStyle::Solid => "S",
            BorderStyle::Dashed => "D",
            BorderStyle::Beveled => "B",
            BorderStyle::Inset => "I",
            BorderStyle::Underline => "U",
        }
    }
}

impl Default for WidgetAppearance {
    fn default() -> Self {
        Self {
            border_color: Some(Color::black()),
            background_color: None,
            border_width: 1.0,
            border_style: BorderStyle::Solid,
        }
    }
}

/// Widget annotation for form field
#[derive(Debug, Clone)]
pub struct Widget {
    /// Rectangle for widget position
    pub rect: Rectangle,
    /// Appearance settings
    pub appearance: WidgetAppearance,
    /// Parent field reference (will be set by FormManager)
    pub parent: Option<String>,
}

impl Widget {
    /// Create a new widget
    pub fn new(rect: Rectangle) -> Self {
        Self {
            rect,
            appearance: WidgetAppearance::default(),
            parent: None,
        }
    }

    /// Set appearance
    pub fn with_appearance(mut self, appearance: WidgetAppearance) -> Self {
        self.appearance = appearance;
        self
    }

    /// Convert to annotation dictionary
    pub fn to_annotation_dict(&self) -> Dictionary {
        let mut dict = Dictionary::new();

        // Annotation type
        dict.set("Type", Object::Name("Annot".to_string()));
        dict.set("Subtype", Object::Name("Widget".to_string()));

        // Rectangle
        let rect_array = vec![
            Object::Real(self.rect.lower_left.x),
            Object::Real(self.rect.lower_left.y),
            Object::Real(self.rect.upper_right.x),
            Object::Real(self.rect.upper_right.y),
        ];
        dict.set("Rect", Object::Array(rect_array));

        // Border style
        let mut bs_dict = Dictionary::new();
        bs_dict.set("W", Object::Real(self.appearance.border_width));
        bs_dict.set(
            "S",
            Object::Name(self.appearance.border_style.pdf_name().to_string()),
        );
        dict.set("BS", Object::Dictionary(bs_dict));

        // Appearance characteristics
        let mut mk_dict = Dictionary::new();

        if let Some(border_color) = &self.appearance.border_color {
            let bc = match border_color {
                Color::Rgb(r, g, b) => vec![Object::Real(*r), Object::Real(*g), Object::Real(*b)],
                Color::Gray(g) => vec![Object::Real(*g)],
                Color::Cmyk(c, m, y, k) => vec![
                    Object::Real(*c),
                    Object::Real(*m),
                    Object::Real(*y),
                    Object::Real(*k),
                ],
            };
            mk_dict.set("BC", Object::Array(bc));
        }

        if let Some(bg_color) = &self.appearance.background_color {
            let bg = match bg_color {
                Color::Rgb(r, g, b) => vec![Object::Real(*r), Object::Real(*g), Object::Real(*b)],
                Color::Gray(g) => vec![Object::Real(*g)],
                Color::Cmyk(c, m, y, k) => vec![
                    Object::Real(*c),
                    Object::Real(*m),
                    Object::Real(*y),
                    Object::Real(*k),
                ],
            };
            mk_dict.set("BG", Object::Array(bg));
        }

        dict.set("MK", Object::Dictionary(mk_dict));

        // Flags - print flag
        dict.set("F", Object::Integer(4));

        dict
    }
}

/// Base field trait
pub trait Field {
    /// Get field name
    fn name(&self) -> &str;

    /// Get field type
    fn field_type(&self) -> &'static str;

    /// Convert to dictionary
    fn to_dict(&self) -> Dictionary;
}

/// Form field with widget
#[derive(Debug, Clone)]
pub struct FormField {
    /// Field dictionary
    pub field_dict: Dictionary,
    /// Associated widgets
    pub widgets: Vec<Widget>,
}

impl FormField {
    /// Create from field dictionary
    pub fn new(field_dict: Dictionary) -> Self {
        Self {
            field_dict,
            widgets: Vec::new(),
        }
    }

    /// Add a widget
    pub fn add_widget(&mut self, widget: Widget) {
        self.widgets.push(widget);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::Point;

    #[test]
    fn test_field_flags() {
        let flags = FieldFlags {
            read_only: true,
            required: true,
            no_export: false,
        };

        assert_eq!(flags.to_flags(), 3); // bits 0 and 1 set
    }

    #[test]
    fn test_border_style() {
        assert_eq!(BorderStyle::Solid.pdf_name(), "S");
        assert_eq!(BorderStyle::Dashed.pdf_name(), "D");
        assert_eq!(BorderStyle::Beveled.pdf_name(), "B");
        assert_eq!(BorderStyle::Inset.pdf_name(), "I");
        assert_eq!(BorderStyle::Underline.pdf_name(), "U");
    }

    #[test]
    fn test_widget_creation() {
        let rect = Rectangle::new(Point::new(100.0, 100.0), Point::new(200.0, 120.0));

        let widget = Widget::new(rect);
        assert_eq!(widget.rect.lower_left.x, 100.0);
        assert_eq!(widget.appearance.border_width, 1.0);
    }

    #[test]
    fn test_widget_annotation_dict() {
        let rect = Rectangle::new(Point::new(50.0, 50.0), Point::new(150.0, 70.0));

        let appearance = WidgetAppearance {
            border_color: Some(Color::Rgb(0.0, 0.0, 1.0)),
            background_color: Some(Color::Gray(0.9)),
            border_width: 2.0,
            border_style: BorderStyle::Solid,
        };

        let widget = Widget::new(rect).with_appearance(appearance);
        let dict = widget.to_annotation_dict();

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
    fn test_field_options_default() {
        let options = FieldOptions::default();
        assert!(!options.flags.read_only);
        assert!(!options.flags.required);
        assert!(!options.flags.no_export);
        assert!(options.default_appearance.is_none());
        assert!(options.quadding.is_none());
    }

    #[test]
    fn test_field_flags_all_combinations() {
        // Test all flag combinations
        let flag_combos = [
            (false, false, false, 0),
            (true, false, false, 1),
            (false, true, false, 2),
            (false, false, true, 4),
            (true, true, false, 3),
            (true, false, true, 5),
            (false, true, true, 6),
            (true, true, true, 7),
        ];

        for (read_only, required, no_export, expected) in flag_combos {
            let flags = FieldFlags {
                read_only,
                required,
                no_export,
            };
            assert_eq!(
                flags.to_flags(),
                expected,
                "Failed for read_only={}, required={}, no_export={}",
                read_only,
                required,
                no_export
            );
        }
    }

    #[test]
    fn test_field_flags_debug_clone_default() {
        let flags = FieldFlags::default();
        let debug_str = format!("{:?}", flags);
        assert!(debug_str.contains("FieldFlags"));

        let cloned = flags.clone();
        assert_eq!(flags.read_only, cloned.read_only);
        assert_eq!(flags.required, cloned.required);
        assert_eq!(flags.no_export, cloned.no_export);

        // Test that Copy trait works
        let copied = flags;
        assert_eq!(flags.read_only, copied.read_only);
    }

    #[test]
    fn test_border_style_debug_clone_copy() {
        let style = BorderStyle::Solid;
        let debug_str = format!("{:?}", style);
        assert!(debug_str.contains("Solid"));

        let cloned = style.clone();
        assert_eq!(style.pdf_name(), cloned.pdf_name());

        // Test Copy trait
        let copied = style;
        assert_eq!(style.pdf_name(), copied.pdf_name());
    }

    #[test]
    fn test_all_border_styles() {
        let styles = [
            (BorderStyle::Solid, "S"),
            (BorderStyle::Dashed, "D"),
            (BorderStyle::Beveled, "B"),
            (BorderStyle::Inset, "I"),
            (BorderStyle::Underline, "U"),
        ];

        for (style, expected) in styles {
            assert_eq!(style.pdf_name(), expected);
        }
    }

    #[test]
    fn test_widget_appearance_default() {
        let appearance = WidgetAppearance::default();
        assert_eq!(appearance.border_color, Some(Color::black()));
        assert_eq!(appearance.background_color, None);
        assert_eq!(appearance.border_width, 1.0);
        match appearance.border_style {
            BorderStyle::Solid => {}
            _ => panic!("Expected solid border style"),
        }
    }

    #[test]
    fn test_widget_appearance_debug_clone() {
        let appearance = WidgetAppearance {
            border_color: Some(Color::red()),
            background_color: Some(Color::gray(0.5)),
            border_width: 2.5,
            border_style: BorderStyle::Dashed,
        };

        let debug_str = format!("{:?}", appearance);
        assert!(debug_str.contains("WidgetAppearance"));

        let cloned = appearance.clone();
        assert_eq!(appearance.border_color, cloned.border_color);
        assert_eq!(appearance.background_color, cloned.background_color);
        assert_eq!(appearance.border_width, cloned.border_width);
    }

    #[test]
    fn test_widget_debug_clone() {
        let rect = Rectangle::new(Point::new(0.0, 0.0), Point::new(100.0, 50.0));
        let widget = Widget::new(rect);

        let debug_str = format!("{:?}", widget);
        assert!(debug_str.contains("Widget"));

        let cloned = widget.clone();
        assert_eq!(widget.rect.lower_left.x, cloned.rect.lower_left.x);
        assert_eq!(widget.parent, cloned.parent);
    }

    #[test]
    fn test_widget_with_parent() {
        let rect = Rectangle::new(Point::new(0.0, 0.0), Point::new(100.0, 50.0));
        let mut widget = Widget::new(rect);
        widget.parent = Some("TextField1".to_string());

        assert_eq!(widget.parent, Some("TextField1".to_string()));
    }

    #[test]
    fn test_widget_annotation_dict_rgb_colors() {
        let rect = Rectangle::new(Point::new(10.0, 20.0), Point::new(110.0, 40.0));
        let appearance = WidgetAppearance {
            border_color: Some(Color::rgb(1.0, 0.0, 0.0)),
            background_color: Some(Color::rgb(0.0, 1.0, 0.0)),
            border_width: 1.5,
            border_style: BorderStyle::Dashed,
        };

        let widget = Widget::new(rect).with_appearance(appearance);
        let dict = widget.to_annotation_dict();

        // Check rectangle array
        if let Some(Object::Array(rect_array)) = dict.get("Rect") {
            assert_eq!(rect_array.len(), 4);
            assert_eq!(rect_array[0], Object::Real(10.0));
            assert_eq!(rect_array[1], Object::Real(20.0));
            assert_eq!(rect_array[2], Object::Real(110.0));
            assert_eq!(rect_array[3], Object::Real(40.0));
        } else {
            panic!("Expected Rect array");
        }

        // Check border style
        if let Some(Object::Dictionary(bs_dict)) = dict.get("BS") {
            assert_eq!(bs_dict.get("W"), Some(&Object::Real(1.5)));
            assert_eq!(bs_dict.get("S"), Some(&Object::Name("D".to_string())));
        } else {
            panic!("Expected BS dictionary");
        }

        // Check appearance characteristics
        if let Some(Object::Dictionary(mk_dict)) = dict.get("MK") {
            // Check border color
            if let Some(Object::Array(bc_array)) = mk_dict.get("BC") {
                assert_eq!(bc_array.len(), 3);
                assert_eq!(bc_array[0], Object::Real(1.0));
                assert_eq!(bc_array[1], Object::Real(0.0));
                assert_eq!(bc_array[2], Object::Real(0.0));
            } else {
                panic!("Expected BC array");
            }

            // Check background color
            if let Some(Object::Array(bg_array)) = mk_dict.get("BG") {
                assert_eq!(bg_array.len(), 3);
                assert_eq!(bg_array[0], Object::Real(0.0));
                assert_eq!(bg_array[1], Object::Real(1.0));
                assert_eq!(bg_array[2], Object::Real(0.0));
            } else {
                panic!("Expected BG array");
            }
        } else {
            panic!("Expected MK dictionary");
        }

        // Check flags
        assert_eq!(dict.get("F"), Some(&Object::Integer(4)));
    }

    #[test]
    fn test_widget_annotation_dict_gray_colors() {
        let rect = Rectangle::new(Point::new(0.0, 0.0), Point::new(50.0, 25.0));
        let appearance = WidgetAppearance {
            border_color: Some(Color::gray(0.3)),
            background_color: Some(Color::gray(0.9)),
            border_width: 0.5,
            border_style: BorderStyle::Beveled,
        };

        let widget = Widget::new(rect).with_appearance(appearance);
        let dict = widget.to_annotation_dict();

        if let Some(Object::Dictionary(mk_dict)) = dict.get("MK") {
            // Check border color (gray)
            if let Some(Object::Array(bc_array)) = mk_dict.get("BC") {
                assert_eq!(bc_array.len(), 1);
                assert_eq!(bc_array[0], Object::Real(0.3));
            } else {
                panic!("Expected BC array");
            }

            // Check background color (gray)
            if let Some(Object::Array(bg_array)) = mk_dict.get("BG") {
                assert_eq!(bg_array.len(), 1);
                assert_eq!(bg_array[0], Object::Real(0.9));
            } else {
                panic!("Expected BG array");
            }
        } else {
            panic!("Expected MK dictionary");
        }
    }

    #[test]
    fn test_widget_annotation_dict_cmyk_colors() {
        let rect = Rectangle::new(Point::new(0.0, 0.0), Point::new(50.0, 25.0));
        let appearance = WidgetAppearance {
            border_color: Some(Color::cmyk(0.1, 0.2, 0.3, 0.4)),
            background_color: Some(Color::cmyk(0.5, 0.6, 0.7, 0.8)),
            border_width: 3.0,
            border_style: BorderStyle::Inset,
        };

        let widget = Widget::new(rect).with_appearance(appearance);
        let dict = widget.to_annotation_dict();

        if let Some(Object::Dictionary(mk_dict)) = dict.get("MK") {
            // Check border color (CMYK)
            if let Some(Object::Array(bc_array)) = mk_dict.get("BC") {
                assert_eq!(bc_array.len(), 4);
                assert_eq!(bc_array[0], Object::Real(0.1));
                assert_eq!(bc_array[1], Object::Real(0.2));
                assert_eq!(bc_array[2], Object::Real(0.3));
                assert_eq!(bc_array[3], Object::Real(0.4));
            } else {
                panic!("Expected BC array");
            }

            // Check background color (CMYK)
            if let Some(Object::Array(bg_array)) = mk_dict.get("BG") {
                assert_eq!(bg_array.len(), 4);
                assert_eq!(bg_array[0], Object::Real(0.5));
                assert_eq!(bg_array[1], Object::Real(0.6));
                assert_eq!(bg_array[2], Object::Real(0.7));
                assert_eq!(bg_array[3], Object::Real(0.8));
            } else {
                panic!("Expected BG array");
            }
        } else {
            panic!("Expected MK dictionary");
        }
    }

    #[test]
    fn test_widget_annotation_dict_no_colors() {
        let rect = Rectangle::new(Point::new(0.0, 0.0), Point::new(50.0, 25.0));
        let appearance = WidgetAppearance {
            border_color: None,
            background_color: None,
            border_width: 2.0,
            border_style: BorderStyle::Underline,
        };

        let widget = Widget::new(rect).with_appearance(appearance);
        let dict = widget.to_annotation_dict();

        if let Some(Object::Dictionary(mk_dict)) = dict.get("MK") {
            assert!(mk_dict.get("BC").is_none());
            assert!(mk_dict.get("BG").is_none());
        } else {
            panic!("Expected MK dictionary");
        }
    }

    #[test]
    fn test_field_options_with_values() {
        let flags = FieldFlags {
            read_only: true,
            required: false,
            no_export: true,
        };

        let options = FieldOptions {
            flags,
            default_appearance: Some("/Helv 12 Tf 0 g".to_string()),
            quadding: Some(1), // Center alignment
        };

        assert!(options.flags.read_only);
        assert!(!options.flags.required);
        assert!(options.flags.no_export);
        assert_eq!(
            options.default_appearance,
            Some("/Helv 12 Tf 0 g".to_string())
        );
        assert_eq!(options.quadding, Some(1));
    }

    #[test]
    fn test_field_options_debug_clone_default() {
        let options = FieldOptions::default();
        let debug_str = format!("{:?}", options);
        assert!(debug_str.contains("FieldOptions"));

        let cloned = options.clone();
        assert_eq!(options.flags.read_only, cloned.flags.read_only);
        assert_eq!(options.default_appearance, cloned.default_appearance);
        assert_eq!(options.quadding, cloned.quadding);
    }

    #[test]
    fn test_form_field_creation() {
        let mut field_dict = Dictionary::new();
        field_dict.set("T", Object::String("TestField".to_string()));
        field_dict.set("FT", Object::Name("Tx".to_string()));

        let form_field = FormField::new(field_dict);
        assert_eq!(form_field.widgets.len(), 0);
        assert_eq!(
            form_field.field_dict.get("T"),
            Some(&Object::String("TestField".to_string()))
        );
    }

    #[test]
    fn test_form_field_add_widget() {
        let mut field_dict = Dictionary::new();
        field_dict.set("T", Object::String("TestField".to_string()));

        let mut form_field = FormField::new(field_dict);

        let rect1 = Rectangle::new(Point::new(10.0, 10.0), Point::new(110.0, 30.0));
        let widget1 = Widget::new(rect1);

        let rect2 = Rectangle::new(Point::new(10.0, 50.0), Point::new(110.0, 70.0));
        let widget2 = Widget::new(rect2);

        form_field.add_widget(widget1);
        form_field.add_widget(widget2);

        assert_eq!(form_field.widgets.len(), 2);
        assert_eq!(form_field.widgets[0].rect.lower_left.x, 10.0);
        assert_eq!(form_field.widgets[1].rect.lower_left.y, 50.0);
    }

    #[test]
    fn test_form_field_debug_clone() {
        let field_dict = Dictionary::new();
        let form_field = FormField::new(field_dict);

        let debug_str = format!("{:?}", form_field);
        assert!(debug_str.contains("FormField"));

        let cloned = form_field.clone();
        assert_eq!(form_field.widgets.len(), cloned.widgets.len());
    }

    #[test]
    fn test_widget_rect_boundary_values() {
        // Test with various rectangle configurations
        let test_rects = [
            // Normal rectangle
            Rectangle::new(Point::new(0.0, 0.0), Point::new(100.0, 50.0)),
            // Small rectangle
            Rectangle::new(Point::new(5.0, 5.0), Point::new(6.0, 6.0)),
            // Large rectangle
            Rectangle::new(Point::new(0.0, 0.0), Point::new(1000.0, 800.0)),
            // Negative coordinates
            Rectangle::new(Point::new(-50.0, -25.0), Point::new(50.0, 25.0)),
        ];

        for rect in test_rects {
            let widget = Widget::new(rect);
            let dict = widget.to_annotation_dict();

            if let Some(Object::Array(rect_array)) = dict.get("Rect") {
                assert_eq!(rect_array.len(), 4);
                assert_eq!(rect_array[0], Object::Real(rect.lower_left.x));
                assert_eq!(rect_array[1], Object::Real(rect.lower_left.y));
                assert_eq!(rect_array[2], Object::Real(rect.upper_right.x));
                assert_eq!(rect_array[3], Object::Real(rect.upper_right.y));
            } else {
                panic!("Expected Rect array");
            }
        }
    }

    #[test]
    fn test_border_width_variations() {
        let rect = Rectangle::new(Point::new(0.0, 0.0), Point::new(100.0, 50.0));
        let border_widths = [0.0, 0.5, 1.0, 2.5, 10.0];

        for width in border_widths {
            let appearance = WidgetAppearance {
                border_color: Some(Color::black()),
                background_color: None,
                border_width: width,
                border_style: BorderStyle::Solid,
            };

            let widget = Widget::new(rect).with_appearance(appearance);
            let dict = widget.to_annotation_dict();

            if let Some(Object::Dictionary(bs_dict)) = dict.get("BS") {
                assert_eq!(bs_dict.get("W"), Some(&Object::Real(width)));
            } else {
                panic!("Expected BS dictionary");
            }
        }
    }

    #[test]
    fn test_quadding_values() {
        let test_quadding = [
            (None, "no quadding"),
            (Some(0), "left alignment"),
            (Some(1), "center alignment"),
            (Some(2), "right alignment"),
        ];

        for (quadding, description) in test_quadding {
            let options = FieldOptions {
                flags: FieldFlags::default(),
                default_appearance: None,
                quadding,
            };

            assert_eq!(options.quadding, quadding, "Failed for {}", description);
        }
    }
}
