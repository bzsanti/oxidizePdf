//! Regression test: PushButtonAppearance with Font::Custom must emit
//! /Resources/Font/<name> as a Type0 placeholder (not Type1) so the
//! writer can rewrite it to an indirect Reference to the document-level
//! Type0 font object (issue #212).

use oxidize_pdf::forms::{
    AppearanceGenerator, AppearanceState, PushButtonAppearance, Widget, WidgetAppearance,
};
use oxidize_pdf::geometry::{Point, Rectangle};
use oxidize_pdf::objects::Object;
use oxidize_pdf::text::Font;

fn make_widget() -> Widget {
    let rect = Rectangle::new(Point::new(0.0, 0.0), Point::new(100.0, 30.0));
    Widget::new(rect).with_appearance(WidgetAppearance::default())
}

#[test]
fn pushbutton_custom_font_resources_emit_type0_not_type1() {
    let mut appearance_gen = PushButtonAppearance::default();
    appearance_gen.font = Font::Custom("CJK".to_string());
    appearance_gen.label = "Submit".to_string();

    let widget = make_widget();
    let stream = appearance_gen
        .generate_appearance(&widget, None, AppearanceState::Normal)
        .expect("generate_appearance must not fail");

    let font_entry = stream
        .resources
        .get("Font")
        .expect("/Resources/Font must be present");

    let font_dict = match font_entry {
        Object::Dictionary(d) => d,
        other => panic!("/Resources/Font is not a dict: {:?}", other),
    };

    let cjk_entry = font_dict
        .get("CJK")
        .expect("/Resources/Font/CJK must be present when font is Font::Custom(\"CJK\")");

    let placeholder_dict = match cjk_entry {
        Object::Dictionary(d) => d,
        other => panic!(
            "/Resources/Font/CJK must be an inline placeholder dict; got {:?}",
            other
        ),
    };

    let subtype = placeholder_dict
        .get("Subtype")
        .and_then(|o| match o {
            Object::Name(n) => Some(n.as_str()),
            _ => None,
        })
        .expect("/Subtype must be present in the font placeholder dict");

    assert_eq!(
        subtype, "Type0",
        "PushButtonAppearance with Font::Custom must emit /Subtype /Type0 in the \
         placeholder, not /Type1 (the writer can only rewrite Type0 placeholders \
         into indirect references — see rewrite_ap_stream_font_resources). Got: {:?}",
        subtype
    );

    let encoding = placeholder_dict
        .get("Encoding")
        .and_then(|o| match o {
            Object::Name(n) => Some(n.as_str()),
            _ => None,
        })
        .expect("/Encoding must be present");

    assert_eq!(
        encoding, "Identity-H",
        "Type0 placeholder must declare /Encoding /Identity-H"
    );
}

#[test]
fn pushbutton_builtin_font_resources_still_emit_type1() {
    // Regression: Helvetica (built-in) must continue to produce a Type1
    // inline dict — the Type1 path is correct for built-ins and the writer
    // does NOT attempt to rewrite it.
    let mut appearance_gen = PushButtonAppearance::default();
    appearance_gen.font = Font::Helvetica;
    appearance_gen.label = "Click".to_string();

    let widget = make_widget();
    let stream = appearance_gen
        .generate_appearance(&widget, None, AppearanceState::Normal)
        .expect("generate_appearance for Helvetica must succeed");

    let font_entry = stream
        .resources
        .get("Font")
        .expect("/Resources/Font must be present");

    let font_dict = match font_entry {
        Object::Dictionary(d) => d,
        other => panic!("/Resources/Font is not a dict: {:?}", other),
    };

    let helv_entry = font_dict
        .get("Helvetica")
        .expect("/Resources/Font/Helvetica must be present");

    let placeholder_dict = match helv_entry {
        Object::Dictionary(d) => d,
        other => panic!(
            "/Resources/Font/Helvetica must be an inline dict; got {:?}",
            other
        ),
    };

    let subtype = placeholder_dict
        .get("Subtype")
        .and_then(|o| match o {
            Object::Name(n) => Some(n.as_str()),
            _ => None,
        })
        .expect("/Subtype present");

    assert_eq!(
        subtype, "Type1",
        "Built-in Helvetica must still emit /Subtype /Type1 (regression guard)"
    );
}
