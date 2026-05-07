//! Regression tests for issue #222 — `Page::text_flow()` must propagate
//! the full set of text-state parameters configured on the page-level
//! `TextContext` into the derived `TextFlowContext`. Pre-2.7.0 only
//! font, font_size, and fill_color propagated (PR #219); the other
//! seven fields (character_spacing, word_spacing, horizontal_scaling,
//! leading, text_rise, rendering_mode, stroke_color) silently dropped.
//!
//! These tests are content-verifying (no smoke tests): they assert
//! that the corresponding PDF operators appear in the bytes emitted
//! by `TextFlowContext::generate_operations` after a write.

use oxidize_pdf::graphics::Color;
use oxidize_pdf::text::TextRenderingMode;
use oxidize_pdf::Page;

fn flow_ops_after_write(page: &mut Page, text: &str) -> String {
    let mut flow = page.text_flow();
    flow.write_wrapped(text)
        .expect("write_wrapped must succeed");
    String::from_utf8(flow.generate_operations()).expect("operations must be UTF-8")
}

#[test]
fn fill_color_propagates_from_text_context() {
    // Established by PR #219; included here as a regression guard so
    // a future refactor cannot quietly drop the propagation.
    let mut page = Page::a4();
    page.text().set_fill_color(Color::Rgb(0.25, 0.5, 0.75));
    let ops = flow_ops_after_write(&mut page, "hello");
    assert!(
        ops.contains("0.250 0.500 0.750 rg"),
        "fill_color must propagate to TextFlowContext, got: {ops}"
    );
}

#[test]
fn character_spacing_propagates_from_text_context() {
    let mut page = Page::a4();
    page.text().set_character_spacing(2.5);
    let ops = flow_ops_after_write(&mut page, "hello");
    assert!(
        ops.contains("2.50 Tc"),
        "character_spacing must propagate to TextFlowContext, got: {ops}"
    );
}

#[test]
fn word_spacing_propagates_from_text_context() {
    let mut page = Page::a4();
    page.text().set_word_spacing(1.75);
    // Use Left alignment so the justified path doesn't override Tw.
    let ops = flow_ops_after_write(&mut page, "hello");
    assert!(
        ops.contains("1.75 Tw"),
        "word_spacing must propagate to TextFlowContext, got: {ops}"
    );
}

#[test]
fn horizontal_scaling_propagates_from_text_context() {
    let mut page = Page::a4();
    page.text().set_horizontal_scaling(0.85); // 85 % expressed as ratio
    let ops = flow_ops_after_write(&mut page, "hello");
    assert!(
        ops.contains("85.00 Tz"),
        "horizontal_scaling must propagate to TextFlowContext as Tz percentage, got: {ops}"
    );
}

#[test]
fn leading_propagates_from_text_context() {
    let mut page = Page::a4();
    page.text().set_leading(14.5);
    let ops = flow_ops_after_write(&mut page, "hello");
    assert!(
        ops.contains("14.50 TL"),
        "leading must propagate to TextFlowContext, got: {ops}"
    );
}

#[test]
fn text_rise_propagates_from_text_context() {
    let mut page = Page::a4();
    page.text().set_text_rise(3.0);
    let ops = flow_ops_after_write(&mut page, "hello");
    assert!(
        ops.contains("3.00 Ts"),
        "text_rise must propagate to TextFlowContext, got: {ops}"
    );
}

#[test]
fn rendering_mode_propagates_from_text_context() {
    let mut page = Page::a4();
    page.text().set_rendering_mode(TextRenderingMode::Stroke);
    let ops = flow_ops_after_write(&mut page, "hello");
    assert!(
        ops.contains("1 Tr"),
        "rendering_mode must propagate to TextFlowContext, got: {ops}"
    );
}

#[test]
fn stroke_color_propagates_from_text_context() {
    let mut page = Page::a4();
    page.text().set_stroke_color(Color::Rgb(0.1, 0.2, 0.3));
    let ops = flow_ops_after_write(&mut page, "hello");
    assert!(
        ops.contains("0.100 0.200 0.300 RG"),
        "stroke_color must propagate to TextFlowContext, got: {ops}"
    );
}

/// Regression guard for the documented semantics: when both a caller-
/// configured `word_spacing` and a `Justified` alignment are present,
/// the per-line justified Tw still wins (current behaviour). The
/// caller-set Tw still appears, but the justified pass overrides it
/// per line as designed.
#[test]
fn justified_alignment_overrides_caller_word_spacing_per_line() {
    use oxidize_pdf::text::TextAlign;

    let mut page = Page::a4();
    page.text().set_word_spacing(1.0); // user value
    let mut flow = page.text_flow();
    flow.set_alignment(TextAlign::Justified);
    flow.write_wrapped(
        "this paragraph has more than one line so the justified pass kicks in to spread the words",
    )
    .expect("write_wrapped must succeed");
    let ops = String::from_utf8(flow.generate_operations()).expect("operations must be UTF-8");

    // The user's `1.00 Tw` MUST appear (propagation worked) AND the
    // justified per-line `Tw` adjustment MUST also appear (justified
    // pass still functions). The justified reset emits `0.00 Tw`.
    assert!(
        ops.contains("1.00 Tw"),
        "user-configured Tw must still propagate, got: {ops}"
    );
    assert!(
        ops.contains("0.00 Tw"),
        "justified pass must still emit its 0.00 Tw reset, got: {ops}"
    );
}
