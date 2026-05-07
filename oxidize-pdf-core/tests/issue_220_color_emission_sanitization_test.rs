//! Integration tests for non-finite-float sanitisation in colour emission —
//! issue #220, coordinated with the shared `write_fill_color`/`write_stroke_color`
//! helper from issue #221.
//!
//! Direct enum construction (`Color::Rgb(f64::NAN, 0.0, 0.0)`) bypasses the
//! `Color::rgb`/`gray`/`cmyk` clamps. Without sanitisation at emission, the
//! `{:.3}` formatter emits `NaN`, `inf`, or `-inf` tokens — ISO 32000-1 §7.3.3
//! rejects those as numeric values and conformant viewers reject the entire
//! content stream.
//!
//! These tests verify the full pipeline: every emission site now refuses
//! to put a non-finite float on the wire and substitutes `0.000` instead.

use oxidize_pdf::graphics::{Color, GraphicsContext, PatternManager};
use oxidize_pdf::text::{Font, TextContext};
use oxidize_pdf::Margins;

// We need the `text::flow::TextFlowContext` for the TextFlowContext tests.
// It's accessible through `oxidize_pdf::text::TextFlowContext`.
use oxidize_pdf::text::TextFlowContext;

// ---------- helper ----------

fn assert_no_non_finite_tokens(ops: &str, context: &str) {
    for forbidden in ["NaN", "inf", "Inf", "INF", "-inf", "-Inf"] {
        assert!(
            !ops.contains(forbidden),
            "{context}: forbidden token '{forbidden}' appears in content stream:\n{ops}"
        );
    }
}

// ---------- TextContext: fill colour ----------

#[test]
fn text_context_fill_color_nan_red_emits_zero_and_keeps_other_components() {
    let mut ctx = TextContext::new();
    ctx.set_font(Font::Helvetica, 12.0);
    ctx.set_fill_color(Color::Rgb(f64::NAN, 0.5, 0.75));
    ctx.write("hello").expect("write must succeed");

    let ops = ctx.operations();
    assert_no_non_finite_tokens(&ops, "TextContext fill NaN red");
    assert!(
        ops.contains("0.000 0.500 0.750 rg"),
        "NaN red must be substituted with 0.000 while green/blue pass through; ops:\n{ops}"
    );
}

#[test]
fn text_context_fill_color_pos_inf_emits_zero() {
    let mut ctx = TextContext::new();
    ctx.set_font(Font::Helvetica, 12.0);
    ctx.set_fill_color(Color::Rgb(f64::INFINITY, 0.0, 0.0));
    ctx.write("x").unwrap();

    let ops = ctx.operations();
    assert_no_non_finite_tokens(&ops, "TextContext fill +inf red");
    assert!(
        ops.contains("0.000 0.000 0.000 rg"),
        "+inf red must be sanitised to 0.000; ops:\n{ops}"
    );
}

#[test]
fn text_context_fill_color_neg_inf_gray_emits_zero() {
    let mut ctx = TextContext::new();
    ctx.set_font(Font::Helvetica, 12.0);
    ctx.set_fill_color(Color::Gray(f64::NEG_INFINITY));
    ctx.write("x").unwrap();

    let ops = ctx.operations();
    assert_no_non_finite_tokens(&ops, "TextContext fill -inf gray");
    assert!(
        ops.contains("0.000 g"),
        "-inf gray must emit 0.000 g; ops:\n{ops}"
    );
}

#[test]
fn text_context_fill_color_cmyk_with_nan_yellow() {
    let mut ctx = TextContext::new();
    ctx.set_font(Font::Helvetica, 12.0);
    ctx.set_fill_color(Color::Cmyk(0.1, 0.2, f64::NAN, 0.4));
    ctx.write("x").unwrap();

    let ops = ctx.operations();
    assert_no_non_finite_tokens(&ops, "TextContext fill cmyk NaN yellow");
    assert!(
        ops.contains("0.100 0.200 0.000 0.400 k"),
        "NaN yellow must be 0.000, others pass through; ops:\n{ops}"
    );
}

// ---------- TextContext: stroke colour ----------

#[test]
fn text_context_stroke_color_nan_emits_zero() {
    let mut ctx = TextContext::new();
    ctx.set_font(Font::Helvetica, 12.0);
    ctx.set_stroke_color(Color::Rgb(f64::NAN, 0.5, 0.5));
    ctx.write("x").unwrap();

    let ops = ctx.operations();
    assert_no_non_finite_tokens(&ops, "TextContext stroke NaN");
    assert!(
        ops.contains("0.000 0.500 0.500 RG"),
        "stroke RG with NaN red must sanitise; ops:\n{ops}"
    );
}

#[test]
fn text_context_stroke_color_inf_cmyk_emits_zero() {
    let mut ctx = TextContext::new();
    ctx.set_font(Font::Helvetica, 12.0);
    ctx.set_stroke_color(Color::Cmyk(f64::INFINITY, 0.2, 0.3, 0.4));
    ctx.write("x").unwrap();

    let ops = ctx.operations();
    assert_no_non_finite_tokens(&ops, "TextContext stroke +inf cyan");
    assert!(
        ops.contains("0.000 0.200 0.300 0.400 K"),
        "stroke K with inf cyan must sanitise; ops:\n{ops}"
    );
}

// ---------- TextFlowContext: fill colour ----------

#[test]
fn text_flow_context_fill_color_nan_emits_zero() {
    let mut ctx = TextFlowContext::new(595.0, 842.0, Margins::default());
    ctx.set_fill_color(Color::Rgb(0.5, f64::NAN, 0.5));
    ctx.write_wrapped("hello world").unwrap();

    let ops = ctx.operations();
    assert_no_non_finite_tokens(&ops, "TextFlowContext fill NaN green");
    assert!(
        ops.contains("0.500 0.000 0.500 rg"),
        "NaN green must be 0.000; ops:\n{ops}"
    );
}

#[test]
fn text_flow_context_fill_color_inf_gray_emits_zero() {
    let mut ctx = TextFlowContext::new(595.0, 842.0, Margins::default());
    ctx.set_fill_color(Color::Gray(f64::INFINITY));
    ctx.write_wrapped("x").unwrap();

    let ops = ctx.operations();
    assert_no_non_finite_tokens(&ops, "TextFlowContext fill +inf gray");
    assert!(
        ops.contains("0.000 g"),
        "+inf gray must emit 0.000 g; ops:\n{ops}"
    );
}

// ---------- GraphicsContext: fill + stroke ----------

#[test]
fn graphics_context_fill_color_nan_emits_zero() {
    let mut g = GraphicsContext::new();
    g.set_fill_color(Color::Rgb(f64::NAN, 0.4, 0.6));
    g.fill();

    let ops = g.get_operations();
    assert_no_non_finite_tokens(&ops, "GraphicsContext fill NaN");
    assert!(
        ops.contains("0.000 0.400 0.600 rg"),
        "GraphicsContext fill must sanitise NaN; ops:\n{ops}"
    );
}

#[test]
fn graphics_context_stroke_color_neg_inf_emits_zero() {
    let mut g = GraphicsContext::new();
    g.set_stroke_color(Color::Cmyk(f64::NEG_INFINITY, 0.0, 0.0, 0.0));
    g.stroke();

    let ops = g.get_operations();
    assert_no_non_finite_tokens(&ops, "GraphicsContext stroke -inf");
    assert!(
        ops.contains("0.000 0.000 0.000 0.000 K"),
        "GraphicsContext stroke must sanitise -inf; ops:\n{ops}"
    );
}

#[test]
fn graphics_context_finite_value_passes_through_unchanged() {
    // Regression guard: sanitisation must NOT alter finite values.
    let mut g = GraphicsContext::new();
    g.set_fill_color(Color::Rgb(0.123, 0.456, 0.789));
    g.fill();

    let ops = g.get_operations();
    assert!(
        ops.contains("0.123 0.456 0.789 rg"),
        "finite RGB must pass through with .3 precision unchanged; ops:\n{ops}"
    );
}

// ---------- Pattern: raw [f64; 3] arrays ----------

#[test]
fn pattern_checkerboard_with_nan_color_does_not_emit_nan() {
    // patterns.rs sites use raw [f64; 3], not Color enum. They are listed
    // in #220's "Affected sites (verified)" so they must also be sanitised.
    let mut mgr = PatternManager::new();
    let pattern_name = mgr
        .create_checkerboard_pattern(10.0, [f64::NAN, 0.0, 0.0], [0.5, 0.5, 0.5])
        .expect("checkerboard creation must succeed");
    let pattern = mgr
        .get_pattern(&pattern_name)
        .expect("registered pattern lookup must succeed");
    let stream = String::from_utf8(pattern.content_stream.clone())
        .expect("pattern content stream must be UTF-8");

    assert_no_non_finite_tokens(&stream, "pattern checkerboard NaN red");
}

#[test]
fn pattern_stripe_with_inf_color_does_not_emit_inf() {
    let mut mgr = PatternManager::new();
    let name = mgr
        .create_stripe_pattern(5.0, 0.0, [0.0, f64::INFINITY, 0.0], [0.2, 0.2, 0.2])
        .expect("stripe creation must succeed");
    let pattern = mgr.get_pattern(&name).unwrap();
    let stream = String::from_utf8(pattern.content_stream.clone()).unwrap();

    assert_no_non_finite_tokens(&stream, "pattern stripe +inf green");
}

// ---------- Parity (issue #221) ----------

#[test]
fn text_context_and_text_flow_context_emit_identical_rgb_fill() {
    // Both call sites collapse to the same helper; emitted byte-for-byte
    // identical operators is the contract.
    let mut tc = TextContext::new();
    tc.set_font(Font::Helvetica, 12.0);
    tc.set_fill_color(Color::Rgb(0.25, 0.5, 0.75));
    tc.write("x").unwrap();

    let mut tfc = TextFlowContext::new(595.0, 842.0, Margins::default());
    tfc.set_fill_color(Color::Rgb(0.25, 0.5, 0.75));
    tfc.write_wrapped("x").unwrap();

    let needle = "0.250 0.500 0.750 rg";
    assert!(
        tc.operations().contains(needle),
        "TextContext: missing '{needle}'; ops:\n{}",
        tc.operations()
    );
    assert!(
        tfc.operations().contains(needle),
        "TextFlowContext: missing '{needle}'; ops:\n{}",
        tfc.operations()
    );
}

#[test]
fn graphics_context_and_text_context_emit_identical_rgb_fill() {
    // GraphicsContext::apply_fill_color and TextContext::apply_text_state_parameters
    // must converge on the same helper output.
    let mut g = GraphicsContext::new();
    g.set_fill_color(Color::Rgb(0.25, 0.5, 0.75));
    g.fill();

    let mut tc = TextContext::new();
    tc.set_font(Font::Helvetica, 12.0);
    tc.set_fill_color(Color::Rgb(0.25, 0.5, 0.75));
    tc.write("x").unwrap();

    let needle = "0.250 0.500 0.750 rg";
    assert!(g.get_operations().contains(needle));
    assert!(tc.operations().contains(needle));
}

// ---------- Graphics: FormXObject ----------

#[test]
fn form_xobject_fill_color_with_nan_emits_zero() {
    use oxidize_pdf::geometry::{Point, Rectangle};
    use oxidize_pdf::graphics::{FormTemplates, FormXObjectBuilder};

    let bbox = Rectangle::new(Point::new(0.0, 0.0), Point::new(100.0, 100.0));
    let form = FormXObjectBuilder::new(bbox)
        .fill_color(f64::NAN, 0.5, 0.75)
        .rectangle(0.0, 0.0, 100.0, 100.0)
        .fill()
        .build();

    let content = String::from_utf8(form.content).expect("content must be UTF-8");
    assert_no_non_finite_tokens(&content, "FormXObject fill NaN");
    assert!(
        content.contains("0.000 0.500 0.750 rg"),
        "FormXObject fill must sanitise; content:\n{content}"
    );

    // Regression: existing FormTemplates still produce sanitised output for
    // their hard-coded finite values.
    let star = FormTemplates::star(100.0, 5);
    let star_content = String::from_utf8(star.content).expect("content must be UTF-8");
    assert!(
        star_content.contains("1.000 0.800 0.000 rg"),
        "FormTemplates::star gold colour must round-trip through helper; content:\n{star_content}"
    );
}

#[test]
fn form_xobject_stroke_color_with_inf_emits_zero() {
    use oxidize_pdf::geometry::{Point, Rectangle};
    use oxidize_pdf::graphics::FormXObjectBuilder;

    let bbox = Rectangle::new(Point::new(0.0, 0.0), Point::new(50.0, 50.0));
    let form = FormXObjectBuilder::new(bbox)
        .stroke_color(0.5, f64::INFINITY, 0.5)
        .move_to(0.0, 0.0)
        .line_to(50.0, 50.0)
        .stroke()
        .build();

    let content = String::from_utf8(form.content).expect("content must be UTF-8");
    assert_no_non_finite_tokens(&content, "FormXObject stroke +inf");
    assert!(
        content.contains("0.500 0.000 0.500 RG"),
        "FormXObject stroke must sanitise; content:\n{content}"
    );
}

// ---------- Forms: field_appearance.rs ----------

#[test]
fn field_appearance_text_field_with_nan_background_emits_zero() {
    use oxidize_pdf::forms::field_appearance::{FieldAppearanceGenerator, TextAlignment};

    let generator = FieldAppearanceGenerator {
        font: "Helv".to_string(),
        font_size: 12.0,
        text_color: Color::black(),
        value: "x".to_string(),
        background_color: Some(Color::Rgb(f64::NAN, 0.5, 0.5)),
        border_color: Some(Color::Cmyk(f64::INFINITY, 0.2, 0.3, 0.4)),
        border_width: 1.0,
        rect: [0.0, 0.0, 100.0, 25.0],
        alignment: TextAlignment::Left,
        multiline: false,
        max_length: None,
        comb: false,
    };

    let stream = generator
        .generate_text_field()
        .expect("text field generation must succeed");
    let content = String::from_utf8_lossy(stream.data());
    assert_no_non_finite_tokens(&content, "field_appearance text-field");
    assert!(
        content.contains("0.000 0.500 0.500 rg"),
        "background NaN red must sanitise; content:\n{content}"
    );
    assert!(
        content.contains("0.000 0.200 0.300 0.400 K"),
        "border CMYK +inf cyan must sanitise; content:\n{content}"
    );
}

// ---------- Annotations: free-text /DA string ----------

#[test]
fn free_text_annotation_da_with_nan_emits_zero() {
    use oxidize_pdf::annotations::FreeTextAnnotation;
    use oxidize_pdf::geometry::{Point, Rectangle};

    let rect = Rectangle::new(Point::new(0.0, 0.0), Point::new(100.0, 50.0));
    let ann = FreeTextAnnotation::new(rect, "hello")
        .with_font(Font::Helvetica, 12.0, Color::Rgb(f64::NAN, 0.5, 0.5))
        .to_annotation();

    // The /DA entry on the annotation properties must not contain NaN.
    let da = ann
        .properties
        .get("DA")
        .expect("annotation must have /DA entry");
    let da_str = match da {
        oxidize_pdf::objects::Object::String(s) => s.clone(),
        other => panic!("/DA must be a String, got {:?}", other),
    };
    assert_no_non_finite_tokens(&da_str, "FreeText /DA");
    assert!(
        da_str.contains("0.000 0.500 0.500 rg"),
        "/DA must sanitise NaN red; /DA:\n{da_str}"
    );
}
