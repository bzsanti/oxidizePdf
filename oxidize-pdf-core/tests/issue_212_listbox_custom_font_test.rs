//! ListBoxAppearance with Font::Custom: the new `generate_appearance_with_font`
//! variant must emit a Type0 placeholder resource for custom fonts and keep the
//! Type1 inline dict for built-ins.

use oxidize_pdf::forms::{AppearanceState, ListBoxAppearance, Widget, WidgetAppearance};
use oxidize_pdf::geometry::{Point, Rectangle};
use oxidize_pdf::objects::Object;
use oxidize_pdf::text::Font;

fn make_widget() -> Widget {
    let rect = Rectangle::new(Point::new(0.0, 0.0), Point::new(150.0, 100.0));
    Widget::new(rect).with_appearance(WidgetAppearance::default())
}

#[test]
fn listbox_custom_font_generate_appearance_with_font_emits_type0_resource() {
    // Pre-fix: ListBoxAppearance has no generate_appearance_with_font and
    // its AppearanceGenerator::generate_appearance calls emit_tj_for_builtin
    // unconditionally → custom fonts fail with EncodingError before any
    // resource dict is constructed.
    //
    // Post-fix: generate_appearance_with_font dispatches on is_custom() and
    // emits a Type0 placeholder resource entry that the writer's
    // rewrite_ap_stream_font_resources can rewrite into an indirect
    // reference to the document-level CIDFontType0 object.
    //
    // With an empty options list there are no Tj operators to encode, so
    // passing custom_font: None is acceptable for this test — what we are
    // verifying here is the resource dict shape.

    let appearance_gen = ListBoxAppearance {
        font: Font::Custom("CJK".to_string()),
        options: vec![],
        ..ListBoxAppearance::default()
    };

    let widget = make_widget();
    let stream = appearance_gen
        .generate_appearance_with_font(&widget, None, AppearanceState::Normal, None)
        .expect("generate_appearance_with_font with empty options must succeed")
        .stream;

    let font_entry = stream
        .resources
        .get("Font")
        .expect("/Resources/Font must be present");
    let font_dict = match font_entry {
        Object::Dictionary(d) => d,
        other => panic!("/Resources/Font is not a dict: {:?}", other),
    };
    let cjk = font_dict
        .get("CJK")
        .expect("/Resources/Font/CJK must be present");
    let placeholder_dict = match cjk {
        Object::Dictionary(d) => d,
        other => panic!("/Resources/Font/CJK is not a dict: {:?}", other),
    };
    let subtype = placeholder_dict
        .get("Subtype")
        .and_then(|o| match o {
            Object::Name(n) => Some(n.as_str()),
            _ => None,
        })
        .expect("/Subtype present");
    assert_eq!(
        subtype, "Type0",
        "Custom font resource must be a Type0 placeholder; got {:?}",
        subtype
    );

    let encoding = placeholder_dict
        .get("Encoding")
        .and_then(|o| match o {
            Object::Name(n) => Some(n.as_str()),
            _ => None,
        })
        .expect("/Encoding present");
    assert_eq!(
        encoding, "Identity-H",
        "Type0 placeholder must declare /Encoding /Identity-H"
    );
}

#[test]
fn listbox_builtin_font_generate_appearance_with_font_emits_type1() {
    let appearance_gen = ListBoxAppearance {
        font: Font::Helvetica,
        options: vec!["Option A".to_string(), "Option B".to_string()],
        ..ListBoxAppearance::default()
    };

    let widget = make_widget();
    let stream = appearance_gen
        .generate_appearance_with_font(&widget, None, AppearanceState::Normal, None)
        .expect("built-in font must succeed")
        .stream;

    let font_dict = stream
        .resources
        .get("Font")
        .and_then(|o| match o {
            Object::Dictionary(d) => Some(d),
            _ => None,
        })
        .expect("/Resources/Font present");
    let helv = font_dict
        .get("Helvetica")
        .expect("/Resources/Font/Helvetica entry");
    let placeholder_dict = match helv {
        Object::Dictionary(d) => d,
        other => panic!("Helvetica entry is not a dict: {:?}", other),
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
        "Built-in Helvetica regression: must still emit /Subtype /Type1"
    );
}

#[test]
fn listbox_custom_font_with_options_but_no_font_param_returns_error() {
    // When options are non-empty AND the configured font is Custom AND the
    // caller passes None for the font parameter, the generator must reject
    // explicitly (matching ComboBox / TextField behaviour). Silent fallback
    // is forbidden — the result would be malformed.
    let appearance_gen = ListBoxAppearance {
        font: Font::Custom("CJK".to_string()),
        options: vec!["项目一".to_string()],
        ..ListBoxAppearance::default()
    };

    let widget = make_widget();
    let err = appearance_gen
        .generate_appearance_with_font(&widget, None, AppearanceState::Normal, None)
        .expect_err("Custom font + non-empty options + custom_font=None must fail explicitly");
    // Verify the error variant + message specifically — silent fallback is
    // forbidden, but so is conflating this with an unrelated failure mode.
    let msg = format!("{err}");
    assert!(
        msg.contains("Custom") && msg.contains("not found"),
        "expected EncodingError mentioning the missing custom font; got: {msg}"
    );
}
