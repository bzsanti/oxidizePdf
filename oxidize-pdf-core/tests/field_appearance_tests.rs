//! Tests for form field appearance streams

use oxidize_pdf::forms::{
    AppearanceCharacteristics, ButtonAppearanceGenerator, ButtonBorderStyle, ButtonStyle,
    FieldAppearanceGenerator, IconFit, IconScaleType, IconScaleWhen, PushButtonAppearanceGenerator,
    TextAlignment, TextPosition,
};
use oxidize_pdf::graphics::Color;
use oxidize_pdf::objects::Object;

#[test]
fn test_appearance_characteristics_default() {
    let chars = AppearanceCharacteristics::default();

    assert_eq!(chars.rotation, 0);
    assert!(chars.border_color.is_none());
    assert!(chars.background_color.is_none());
    assert!(chars.normal_caption.is_none());
    assert_eq!(chars.text_position, TextPosition::CaptionOnly);
}

#[test]
fn test_appearance_characteristics_to_dict() {
    let mut chars = AppearanceCharacteristics::default();
    chars.rotation = 90;
    chars.border_color = Some(Color::rgb(0.0, 0.0, 1.0)); // Blue
    chars.background_color = Some(Color::rgb(1.0, 1.0, 1.0)); // White
    chars.normal_caption = Some("Click Me".to_string());
    chars.text_position = TextPosition::CaptionBelowIcon;

    let dict = chars.to_dict();

    assert_eq!(dict.get("R"), Some(&Object::Integer(90)));
    assert!(dict.get("BC").is_some());
    assert!(dict.get("BG").is_some());
    assert_eq!(
        dict.get("CA"),
        Some(&Object::String("Click Me".to_string()))
    );
    assert_eq!(dict.get("TP"), Some(&Object::Integer(2))); // CaptionBelowIcon
}

#[test]
fn test_icon_fit() {
    let fit = IconFit {
        scale_type: IconScaleType::Always,
        scale_when: IconScaleWhen::IconBigger,
        align_x: 0.5,
        align_y: 0.5,
        fit_bounds: true,
    };

    let dict_obj = fit.to_dict();

    if let Object::Dictionary(dict) = dict_obj {
        assert_eq!(dict.get("SW"), Some(&Object::Name("B".to_string()))); // IconBigger
        assert_eq!(dict.get("S"), Some(&Object::Name("A".to_string()))); // Always

        if let Some(Object::Array(align)) = dict.get("A") {
            assert_eq!(align.len(), 2);
            assert_eq!(align[0], Object::Real(0.5));
            assert_eq!(align[1], Object::Real(0.5));
        } else {
            panic!("Alignment array not found");
        }

        assert_eq!(dict.get("FB"), Some(&Object::Boolean(true)));
    } else {
        panic!("Expected Dictionary object");
    }
}

#[test]
fn test_text_position_values() {
    assert_eq!(TextPosition::CaptionOnly.to_int(), 0);
    assert_eq!(TextPosition::IconOnly.to_int(), 1);
    assert_eq!(TextPosition::CaptionBelowIcon.to_int(), 2);
    assert_eq!(TextPosition::CaptionAboveIcon.to_int(), 3);
    assert_eq!(TextPosition::CaptionRightIcon.to_int(), 4);
    assert_eq!(TextPosition::CaptionLeftIcon.to_int(), 5);
    assert_eq!(TextPosition::CaptionOverlayIcon.to_int(), 6);
}

#[test]
fn test_field_appearance_text_field() {
    let generator = FieldAppearanceGenerator {
        value: "Hello World".to_string(),
        font: "Helvetica".to_string(),
        font_size: 12.0,
        text_color: Color::rgb(0.0, 0.0, 0.0),
        background_color: Some(Color::rgb(1.0, 1.0, 1.0)),
        border_color: Some(Color::rgb(0.5, 0.5, 0.5)),
        border_width: 1.0,
        rect: [0.0, 0.0, 200.0, 20.0],
        alignment: TextAlignment::Left,
        multiline: false,
        max_length: None,
        comb: false,
    };

    let stream = generator.generate_text_field().unwrap();

    // Check stream has proper structure
    assert_eq!(
        stream.dictionary().get("Type"),
        Some(&Object::Name("XObject".to_string()))
    );
    assert_eq!(
        stream.dictionary().get("Subtype"),
        Some(&Object::Name("Form".to_string()))
    );

    // Check BBox
    if let Some(Object::Array(bbox)) = stream.dictionary().get("BBox") {
        assert_eq!(bbox.len(), 4);
        assert_eq!(bbox[2], Object::Real(200.0));
        assert_eq!(bbox[3], Object::Real(20.0));
    }
}

#[test]
fn test_field_appearance_multiline() {
    let generator = FieldAppearanceGenerator {
        value: "Line 1\nLine 2\nLine 3".to_string(),
        font: "Helvetica".to_string(),
        font_size: 10.0,
        text_color: Color::rgb(0.0, 0.0, 0.0),
        background_color: None,
        border_color: None,
        border_width: 0.0,
        rect: [0.0, 0.0, 200.0, 60.0],
        alignment: TextAlignment::Center,
        multiline: true,
        max_length: None,
        comb: false,
    };

    let stream = generator.generate_text_field().unwrap();

    // Content should contain multiple text positioning commands for multiline
    let content = String::from_utf8_lossy(stream.data());
    assert!(content.contains("Line 1"));
    assert!(content.contains("Line 2"));
    assert!(content.contains("Line 3"));
}

#[test]
fn test_field_appearance_comb() {
    let generator = FieldAppearanceGenerator {
        value: "12345".to_string(),
        font: "Courier".to_string(),
        font_size: 12.0,
        text_color: Color::rgb(0.0, 0.0, 0.0),
        background_color: None,
        border_color: Some(Color::rgb(0.0, 0.0, 0.0)),
        border_width: 1.0,
        rect: [0.0, 0.0, 100.0, 20.0],
        alignment: TextAlignment::Left,
        multiline: false,
        max_length: Some(10),
        comb: true,
    };

    let stream = generator.generate_text_field().unwrap();

    // Comb field should space characters evenly
    let content = String::from_utf8_lossy(stream.data());
    assert!(content.contains("(1)"));
    assert!(content.contains("(2)"));
    assert!(content.contains("(3)"));
    assert!(content.contains("(4)"));
    assert!(content.contains("(5)"));
}

#[test]
fn test_button_appearance_checkbox() {
    let generator = ButtonAppearanceGenerator {
        style: ButtonStyle::Check,
        size: 12.0,
        border_color: Color::rgb(0.0, 0.0, 0.0),
        background_color: Color::rgb(1.0, 1.0, 1.0),
        check_color: Color::rgb(0.0, 0.0, 0.0),
        border_width: 1.0,
    };

    // Test checked state
    let checked = generator.generate_checked().unwrap();
    assert_eq!(
        checked.dictionary().get("Type"),
        Some(&Object::Name("XObject".to_string()))
    );

    // Test unchecked state
    let unchecked = generator.generate_unchecked().unwrap();
    assert_eq!(
        unchecked.dictionary().get("Type"),
        Some(&Object::Name("XObject".to_string()))
    );
}

#[test]
fn test_button_appearance_radio() {
    let generator = ButtonAppearanceGenerator {
        style: ButtonStyle::Radio,
        size: 12.0,
        border_color: Color::rgb(0.0, 0.0, 0.0),
        background_color: Color::rgb(1.0, 1.0, 1.0),
        check_color: Color::rgb(0.0, 0.0, 0.0),
        border_width: 1.0,
    };

    let checked = generator.generate_checked().unwrap();
    let unchecked = generator.generate_unchecked().unwrap();

    // Both should have proper bounding box
    if let Some(Object::Array(bbox)) = checked.dictionary().get("BBox") {
        assert_eq!(bbox[2], Object::Real(12.0));
        assert_eq!(bbox[3], Object::Real(12.0));
    }
}

#[test]
fn test_button_styles() {
    let styles = vec![
        ButtonStyle::Check,
        ButtonStyle::Cross,
        ButtonStyle::Diamond,
        ButtonStyle::Circle,
        ButtonStyle::Star,
        ButtonStyle::Square,
        ButtonStyle::Radio,
    ];

    for style in styles {
        let generator = ButtonAppearanceGenerator {
            style,
            size: 12.0,
            border_color: Color::rgb(0.0, 0.0, 0.0),
            background_color: Color::rgb(1.0, 1.0, 1.0),
            check_color: Color::rgb(0.0, 0.0, 0.0),
            border_width: 1.0,
        };

        let checked = generator.generate_checked().unwrap();
        assert!(checked.data().len() > 0);
    }
}

#[test]
fn test_push_button_appearance() {
    let generator = PushButtonAppearanceGenerator {
        caption: "Submit".to_string(),
        font: "Helvetica-Bold".to_string(),
        font_size: 14.0,
        text_color: Color::rgb(1.0, 1.0, 1.0),
        background_color: Color::rgb(0.0, 0.5, 1.0),
        border_color: Color::rgb(0.0, 0.3, 0.8),
        border_width: 2.0,
        size: [100.0, 30.0],
        border_style: ButtonBorderStyle::Solid,
    };

    let normal = generator.generate_normal().unwrap();
    let rollover = generator.generate_rollover().unwrap();
    let down = generator.generate_down().unwrap();

    // All states should have proper structure
    assert_eq!(
        normal.dictionary().get("Type"),
        Some(&Object::Name("XObject".to_string()))
    );
    assert_eq!(
        rollover.dictionary().get("Type"),
        Some(&Object::Name("XObject".to_string()))
    );
    assert_eq!(
        down.dictionary().get("Type"),
        Some(&Object::Name("XObject".to_string()))
    );

    // Check size in BBox
    if let Some(Object::Array(bbox)) = normal.dictionary().get("BBox") {
        assert_eq!(bbox[2], Object::Real(100.0));
        assert_eq!(bbox[3], Object::Real(30.0));
    }
}

#[test]
fn test_push_button_border_styles() {
    let styles = vec![
        ButtonBorderStyle::Solid,
        ButtonBorderStyle::Dashed,
        ButtonBorderStyle::Beveled,
        ButtonBorderStyle::Inset,
        ButtonBorderStyle::Underline,
    ];

    for style in styles {
        let generator = PushButtonAppearanceGenerator {
            caption: "Test".to_string(),
            font: "Helvetica".to_string(),
            font_size: 12.0,
            text_color: Color::rgb(0.0, 0.0, 0.0),
            background_color: Color::rgb(0.9, 0.9, 0.9),
            border_color: Color::rgb(0.5, 0.5, 0.5),
            border_width: 1.0,
            size: [80.0, 25.0],
            border_style: style,
        };

        let normal = generator.generate_normal().unwrap();
        assert!(normal.data().len() > 0);

        // Dashed should contain dash pattern
        if style == ButtonBorderStyle::Dashed {
            let content = String::from_utf8_lossy(normal.data());
            assert!(content.contains("[3 3] 0 d"));
        }
    }
}

#[test]
fn test_color_lighten_darken() {
    let color = Color::rgb(0.5, 0.5, 0.5);

    let lighter = color.lighten(0.2);
    assert_eq!(lighter.r(), 0.7);
    assert_eq!(lighter.g(), 0.7);
    assert_eq!(lighter.b(), 0.7);

    let darker = color.darken(0.2);
    assert_eq!(darker.r(), 0.3);
    assert_eq!(darker.g(), 0.3);
    assert_eq!(darker.b(), 0.3);

    // Test clamping
    let white = Color::rgb(1.0, 1.0, 1.0);
    let lighter_white = white.lighten(0.5);
    assert_eq!(lighter_white.r(), 1.0);

    let black = Color::rgb(0.0, 0.0, 0.0);
    let darker_black = black.darken(0.5);
    assert_eq!(darker_black.r(), 0.0);
}

#[test]
fn test_color_to_array() {
    let color = Color::rgb(0.25, 0.5, 0.75);
    let array = color.to_array();

    if let Object::Array(values) = array {
        assert_eq!(values.len(), 3);
        assert_eq!(values[0], Object::Real(0.25));
        assert_eq!(values[1], Object::Real(0.5));
        assert_eq!(values[2], Object::Real(0.75));
    } else {
        panic!("Expected Array object");
    }
}

#[test]
fn test_text_alignment() {
    let alignments = vec![
        TextAlignment::Left,
        TextAlignment::Center,
        TextAlignment::Right,
    ];

    for alignment in alignments {
        let generator = FieldAppearanceGenerator {
            value: "Aligned Text".to_string(),
            font: "Helvetica".to_string(),
            font_size: 12.0,
            text_color: Color::rgb(0.0, 0.0, 0.0),
            background_color: None,
            border_color: None,
            border_width: 0.0,
            rect: [0.0, 0.0, 200.0, 20.0],
            alignment,
            multiline: false,
            max_length: None,
            comb: false,
        };

        let stream = generator.generate_text_field().unwrap();
        assert!(stream.data().len() > 0);
    }
}

#[test]
fn test_escape_string() {
    // Test internal function behavior through generated content
    let generator = FieldAppearanceGenerator {
        value: "Text with (parens) and \\backslash".to_string(),
        font: "Helvetica".to_string(),
        font_size: 12.0,
        text_color: Color::rgb(0.0, 0.0, 0.0),
        background_color: None,
        border_color: None,
        border_width: 0.0,
        rect: [0.0, 0.0, 200.0, 20.0],
        alignment: TextAlignment::Left,
        multiline: false,
        max_length: None,
        comb: false,
    };

    let stream = generator.generate_text_field().unwrap();
    let content = String::from_utf8_lossy(stream.data());

    // Should contain escaped characters
    assert!(content.contains("\\("));
    assert!(content.contains("\\)"));
    assert!(content.contains("\\\\"));
}

#[test]
fn test_field_with_background_and_border() {
    let generator = FieldAppearanceGenerator {
        value: "Styled Field".to_string(),
        font: "Helvetica".to_string(),
        font_size: 12.0,
        text_color: Color::rgb(0.0, 0.0, 0.0),
        background_color: Some(Color::rgb(0.95, 0.95, 1.0)),
        border_color: Some(Color::rgb(0.0, 0.0, 0.5)),
        border_width: 2.0,
        rect: [0.0, 0.0, 150.0, 25.0],
        alignment: TextAlignment::Center,
        multiline: false,
        max_length: None,
        comb: false,
    };

    let stream = generator.generate_text_field().unwrap();
    let content = String::from_utf8_lossy(stream.data());

    // Should contain background fill
    assert!(content.contains("0.95 0.95 1 rg"));
    assert!(content.contains("re"));
    assert!(content.contains("f"));

    // Should contain border stroke
    assert!(content.contains("2 w"));
    assert!(content.contains("0 0 0.5 RG"));
    assert!(content.contains("S"));
}

#[test]
fn test_push_button_pressed_state() {
    let generator = PushButtonAppearanceGenerator {
        caption: "Press Me".to_string(),
        font: "Helvetica".to_string(),
        font_size: 12.0,
        text_color: Color::rgb(0.0, 0.0, 0.0),
        background_color: Color::rgb(0.8, 0.8, 0.8),
        border_color: Color::rgb(0.4, 0.4, 0.4),
        border_width: 1.0,
        size: [80.0, 25.0],
        border_style: ButtonBorderStyle::Beveled,
    };

    let normal = generator.generate_normal().unwrap();
    let down = generator.generate_down().unwrap();

    // Both should be valid streams
    assert!(normal.data().len() > 0);
    assert!(down.data().len() > 0);

    // Down state should have darker background
    let down_content = String::from_utf8_lossy(down.data());
    assert!(down_content.contains("0.7")); // Darkened background
}

#[test]
fn test_multiline_overflow() {
    let generator = FieldAppearanceGenerator {
        value: "Line 1\nLine 2\nLine 3\nLine 4\nLine 5\nLine 6\nLine 7\nLine 8".to_string(),
        font: "Helvetica".to_string(),
        font_size: 12.0,
        text_color: Color::rgb(0.0, 0.0, 0.0),
        background_color: None,
        border_color: None,
        border_width: 0.0,
        rect: [0.0, 0.0, 200.0, 50.0], // Small height for 8 lines
        alignment: TextAlignment::Left,
        multiline: true,
        max_length: None,
        comb: false,
    };

    let stream = generator.generate_text_field().unwrap();

    // Should handle overflow gracefully
    assert!(stream.data().len() > 0);
}

#[test]
fn test_comb_field_truncation() {
    let generator = FieldAppearanceGenerator {
        value: "1234567890ABCDEF".to_string(), // 16 characters
        font: "Courier".to_string(),
        font_size: 12.0,
        text_color: Color::rgb(0.0, 0.0, 0.0),
        background_color: None,
        border_color: None,
        border_width: 0.0,
        rect: [0.0, 0.0, 100.0, 20.0],
        alignment: TextAlignment::Left,
        multiline: false,
        max_length: Some(10), // Only 10 slots
        comb: true,
    };

    let stream = generator.generate_text_field().unwrap();
    let content = String::from_utf8_lossy(stream.data());

    // Should only contain first 10 characters
    assert!(content.contains("(9)"));
    assert!(content.contains("(0)"));
    assert!(!content.contains("(A)")); // Should be truncated
}

#[test]
fn test_icon_scale_types() {
    let scale_types = vec![
        (IconScaleType::Always, "A"),
        (IconScaleType::Bigger, "B"),
        (IconScaleType::Smaller, "S"),
        (IconScaleType::Never, "N"),
    ];

    for (scale_type, expected) in scale_types {
        let fit = IconFit {
            scale_type,
            scale_when: IconScaleWhen::Always,
            align_x: 0.5,
            align_y: 0.5,
            fit_bounds: false,
        };

        if let Object::Dictionary(dict) = fit.to_dict() {
            assert_eq!(dict.get("S"), Some(&Object::Name(expected.to_string())));
        }
    }
}

#[test]
fn test_icon_scale_when() {
    let scale_whens = vec![
        (IconScaleWhen::Always, "A"),
        (IconScaleWhen::IconBigger, "B"),
        (IconScaleWhen::IconSmaller, "S"),
        (IconScaleWhen::Never, "N"),
    ];

    for (scale_when, expected) in scale_whens {
        let fit = IconFit {
            scale_type: IconScaleType::Always,
            scale_when,
            align_x: 0.5,
            align_y: 0.5,
            fit_bounds: false,
        };

        if let Object::Dictionary(dict) = fit.to_dict() {
            assert_eq!(dict.get("SW"), Some(&Object::Name(expected.to_string())));
        }
    }
}
