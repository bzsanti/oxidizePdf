//! Integration tests for issue #216:
//!
//! `Page::text_flow()` returns a `TextFlowContext` that should inherit the
//! page's current text state (font, font size, fill color) so that
//! `text_flow_at`-style helpers (Python wrapper, .NET wrapper) emit text
//! using the font and color most recently set via `Page::text().set_font(...)`
//! and `Page::text().set_fill_color(...)`.
//!
//! Before the fix the flow context was created with hardcoded defaults
//! (Helvetica / 12pt / no fill colour) and silently ignored anything set on
//! the page-level `TextContext`.

use oxidize_pdf::graphics::Color;
use oxidize_pdf::{Document, Font, Page};

/// `Page::text_flow()` must return a context whose `current_font` and
/// `font_size` match what was last set via `Page::text().set_font(...)`.
#[test]
fn text_flow_inherits_font_and_size_from_page_text_state() {
    let mut page = Page::new(612.0, 792.0);

    page.text().set_font(Font::TimesBold, 32.0);

    let ctx = page.text_flow();

    assert_eq!(
        ctx.current_font(),
        &Font::TimesBold,
        "Page::text_flow() ignored the page-level font set via set_font()"
    );
    assert_eq!(
        ctx.font_size(),
        32.0,
        "Page::text_flow() ignored the page-level font size set via set_font()"
    );
}

/// `Page::text_flow()` must inherit the fill colour set on the page-level
/// `TextContext` so that wrapped text honours `set_text_color`.
#[test]
fn text_flow_inherits_fill_color_from_page_text_state() {
    let mut page = Page::new(612.0, 792.0);

    page.text().set_fill_color(Color::rgb(0.8, 0.1, 0.1));

    let ctx = page.text_flow();

    assert_eq!(
        ctx.fill_color(),
        Some(Color::rgb(0.8, 0.1, 0.1)),
        "Page::text_flow() ignored the page-level fill color set via set_fill_color()"
    );
}

/// End-to-end check: setting Times-Bold/32 then writing wrapped text must
/// produce a content stream whose `Tf` operator references Times-Bold at 32pt.
/// This is the exact bug reported in #216.
#[test]
fn write_wrapped_emits_inherited_font_in_content_stream() {
    let mut doc = Document::new();
    let mut page = Page::new(612.0, 792.0);
    page.set_margins(50.0, 50.0, 50.0, 50.0);

    page.text().set_font(Font::TimesBold, 32.0);

    let mut ctx = page.text_flow();
    ctx.at(100.0, 700.0)
        .write_wrapped("Heading in Times-Bold 32")
        .expect("write_wrapped should not fail");

    let ops = ctx.operations().to_owned();

    assert!(
        ops.contains("/Times-Bold 32 Tf"),
        "Expected `/Times-Bold 32 Tf` operator in operations.\n\
         Found instead:\n{ops}"
    );
    assert!(
        !ops.contains("/Helvetica 12 Tf"),
        "Found default `/Helvetica 12 Tf` — page-level font was NOT inherited.\n\
         Operations:\n{ops}"
    );

    // Round-trip the document with compression disabled so we can assert the
    // operator survives the writer pipeline, not just the in-memory ops buffer.
    page.add_text_flow(&ctx);
    doc.set_compress(false);
    doc.add_page(page);
    let bytes = doc.to_bytes().expect("to_bytes must succeed");
    let needle = b"/Times-Bold 32 Tf";
    assert!(
        bytes.windows(needle.len()).any(|w| w == needle),
        "Serialized PDF does not contain `/Times-Bold 32 Tf` — propagation \
         was lost somewhere in the writer pipeline."
    );
}

/// End-to-end check: setting a red fill colour then writing wrapped text
/// must produce a content stream that emits `0.800 0.100 0.100 rg` inside
/// the `BT … ET` block before the `Tj`. Without this propagation, the wrapped
/// text inherits whatever fill colour the surrounding graphics state happens
/// to carry (typically black) and silently drops the user's intent.
#[test]
fn write_wrapped_emits_inherited_fill_color_before_tj() {
    let mut page = Page::new(612.0, 792.0);
    page.set_margins(50.0, 50.0, 50.0, 50.0);

    page.text().set_fill_color(Color::rgb(0.8, 0.1, 0.1));
    page.text().set_font(Font::Helvetica, 14.0);

    let mut ctx = page.text_flow();
    ctx.at(100.0, 700.0)
        .write_wrapped("Red wrapped text")
        .expect("write_wrapped should not fail");

    let ops = ctx.operations().to_owned();

    let rg_pos = ops.find("0.800 0.100 0.100 rg").unwrap_or_else(|| {
        panic!(
            "Expected `0.800 0.100 0.100 rg` (non-stroking color) in operations.\n\
             Found instead:\n{ops}"
        )
    });
    let tj_pos = ops.find(" Tj").expect("Expected at least one Tj operator");

    assert!(
        rg_pos < tj_pos,
        "Color operator `rg` must precede the first `Tj` so the show-text uses it.\n\
         rg_pos={rg_pos} tj_pos={tj_pos}\nOperations:\n{ops}"
    );
}

/// Regression guard: an explicit `set_font` on the flow context must override
/// whatever was inherited from the page. This keeps existing call sites that
/// configure the flow context directly working unchanged.
#[test]
fn flow_set_font_overrides_inherited_page_font() {
    let mut page = Page::new(612.0, 792.0);
    page.text().set_font(Font::TimesBold, 32.0);

    let mut ctx = page.text_flow();
    ctx.set_font(Font::Courier, 10.0);

    assert_eq!(ctx.current_font(), &Font::Courier);
    assert_eq!(ctx.font_size(), 10.0);
}

/// Symmetric regression guard for fill colour: an explicit `set_fill_color`
/// on the flow context must override the colour inherited from the page-level
/// `TextContext`, both at the field level and in the emitted content stream.
#[test]
fn flow_set_fill_color_overrides_inherited_page_color() {
    let mut page = Page::new(612.0, 792.0);
    page.text().set_fill_color(Color::rgb(0.8, 0.1, 0.1)); // red on the page

    let mut ctx = page.text_flow();
    ctx.set_fill_color(Color::rgb(0.1, 0.2, 0.9)); // blue overrides

    assert_eq!(ctx.fill_color(), Some(Color::rgb(0.1, 0.2, 0.9)));

    ctx.at(100.0, 700.0)
        .write_wrapped("Should be blue, not red")
        .expect("write_wrapped should not fail");

    let ops = ctx.operations().to_owned();
    assert!(
        ops.contains("0.100 0.200 0.900 rg"),
        "Override colour `0.100 0.200 0.900 rg` not found in operations:\n{ops}"
    );
    assert!(
        !ops.contains("0.800 0.100 0.100 rg"),
        "Inherited red `0.800 0.100 0.100 rg` should have been overridden:\n{ops}"
    );
}

/// Coverage for the `Gray` arm of the colour emission match block. Without
/// this test a regression in the operator/precision for `g` would slip past
/// the existing `Rgb`-only test.
#[test]
fn write_wrapped_emits_gray_fill_color_operator() {
    let mut page = Page::new(612.0, 792.0);
    page.text().set_fill_color(Color::Gray(0.5));

    let mut ctx = page.text_flow();
    ctx.at(100.0, 700.0)
        .write_wrapped("Mid-gray text")
        .expect("write_wrapped should not fail");

    let ops = ctx.operations().to_owned();

    let g_pos = ops
        .find("0.500 g")
        .unwrap_or_else(|| panic!("Expected `0.500 g` in operations.\nOperations:\n{ops}"));
    let tj_pos = ops.find(" Tj").expect("Expected at least one Tj operator");
    assert!(
        g_pos < tj_pos,
        "Gray operator must precede the first Tj. g_pos={g_pos} tj_pos={tj_pos}\n{ops}"
    );
    // Negative-space check: no `rg` or `k` from the other arms.
    assert!(
        !ops.contains(" rg") && !ops.contains(" k"),
        "Gray fill must emit only `g`, not `rg`/`k`:\n{ops}"
    );
}

/// Coverage for the `Cmyk` arm of the colour emission match block.
#[test]
fn write_wrapped_emits_cmyk_fill_color_operator() {
    let mut page = Page::new(612.0, 792.0);
    page.text().set_fill_color(Color::Cmyk(0.0, 0.5, 0.5, 0.0));

    let mut ctx = page.text_flow();
    ctx.at(100.0, 700.0)
        .write_wrapped("CMYK orange-ish")
        .expect("write_wrapped should not fail");

    let ops = ctx.operations().to_owned();

    let k_pos = ops.find("0.000 0.500 0.500 0.000 k").unwrap_or_else(|| {
        panic!("Expected `0.000 0.500 0.500 0.000 k` in operations.\nOperations:\n{ops}")
    });
    let tj_pos = ops.find(" Tj").expect("Expected at least one Tj operator");
    assert!(
        k_pos < tj_pos,
        "CMYK operator must precede the first Tj. k_pos={k_pos} tj_pos={tj_pos}\n{ops}"
    );
    assert!(
        !ops.contains(" rg") && !ops.contains(" g\n"),
        "CMYK fill must emit only `k`, not `rg`/`g`:\n{ops}"
    );
}
