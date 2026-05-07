//! Regression tests for issue #227 — `Page::generate_content_with_page_info`
//! must preserve PDF painter-model call order across `Page::graphics()` and
//! `Page::text()` switches. Pre-2.7.0 the implementation concatenated the
//! graphics buffer in full before the text buffer, regardless of the order
//! in which the caller mixed gfx and text calls.
//!
//! These tests are content-verifying (no smoke tests): they assert the
//! relative byte position of operators in the emitted content stream.

use oxidize_pdf::graphics::Color;
use oxidize_pdf::Page;

#[test]
fn interleaved_gfx_text_gfx_preserves_call_order() {
    let mut page = Page::a4();

    // 1. Draw a blue rectangle (graphics).
    page.graphics()
        .set_fill_color(Color::Rgb(0.0, 0.0, 1.0))
        .rect(50.0, 700.0, 100.0, 50.0)
        .fill();

    // 2. Write a label (text).
    page.text()
        .set_font(oxidize_pdf::text::Font::Helvetica, 12.0)
        .at(50.0, 680.0)
        .write("Label")
        .unwrap();

    // 3. Draw a red rectangle AFTER the text (graphics again).
    page.graphics()
        .set_fill_color(Color::Rgb(1.0, 0.0, 0.0))
        .rect(50.0, 640.0, 100.0, 30.0)
        .fill();

    // The page generates its content stream when added to a Document and
    // serialized. We exercise the same code path via `Document::to_bytes`
    // and inspect the resulting PDF's content stream.
    let mut doc = oxidize_pdf::Document::new();
    doc.set_compress(false); // inspect raw operators
    doc.add_page(page);
    let bytes = doc.to_bytes().expect("to_bytes must succeed");

    // The blue rg, the (Label) Tj, and the red rg must appear in *call*
    // order — not in "all-graphics-then-all-text" order which is the
    // pre-2.7.0 bug (#227).
    let s = String::from_utf8_lossy(&bytes);
    let pos_blue = s
        .find("0.000 0.000 1.000 rg")
        .expect("blue rg must be emitted");
    let pos_label = s.find("(Label)").expect("Label literal must be emitted");
    let pos_red = s
        .find("1.000 0.000 0.000 rg")
        .expect("red rg must be emitted");

    assert!(
        pos_blue < pos_label,
        "blue rg ({pos_blue}) must precede the text label ({pos_label}) — \
         call order should win, content stream:\n{s}"
    );
    assert!(
        pos_label < pos_red,
        "text label ({pos_label}) must precede the red rg ({pos_red}) — \
         call order should win, content stream:\n{s}"
    );
}

#[test]
fn text_then_graphics_preserves_call_order() {
    let mut page = Page::a4();

    // Text first, then graphics. With the pre-2.7.0 fixed-order flush
    // (graphics first, then text), the rectangle would render BEFORE the
    // text — wrong z-order for any caller that wants the rectangle to
    // overlay the text.
    page.text()
        .set_font(oxidize_pdf::text::Font::Helvetica, 12.0)
        .at(50.0, 700.0)
        .write("Underneath")
        .unwrap();

    page.graphics()
        .set_fill_color(Color::Rgb(0.5, 0.5, 0.5))
        .rect(40.0, 695.0, 200.0, 20.0)
        .fill();

    let mut doc = oxidize_pdf::Document::new();
    doc.set_compress(false); // inspect raw operators
    doc.add_page(page);
    let bytes = doc.to_bytes().expect("to_bytes must succeed");

    let s = String::from_utf8_lossy(&bytes);
    let pos_label = s
        .find("(Underneath)")
        .expect("text literal must be emitted");
    let pos_rect_fill = s
        .find("0.500 0.500 0.500 rg")
        .expect("grey rg must be emitted");

    assert!(
        pos_label < pos_rect_fill,
        "text-first call order must put `(Underneath) Tj` ({pos_label}) before \
         the grey rectangle's `rg` ({pos_rect_fill}); content stream:\n{s}"
    );
}

#[test]
fn pure_graphics_only_does_not_regress() {
    // Sanity: a page with only graphics calls must still produce the
    // expected sequence of operators (no text buffer flushing logic
    // should alter the all-graphics path).
    let mut page = Page::a4();
    page.graphics()
        .move_to(0.0, 0.0)
        .line_to(100.0, 100.0)
        .stroke();

    let mut doc = oxidize_pdf::Document::new();
    doc.set_compress(false); // inspect raw operators
    doc.add_page(page);
    let bytes = doc.to_bytes().expect("to_bytes must succeed");
    let s = String::from_utf8_lossy(&bytes);

    let pos_m = s.find("0.00 0.00 m").expect("m must be emitted");
    let pos_l = s.find("100.00 100.00 l").expect("l must be emitted");
    // `S` is emitted as a standalone token followed by `\n` — match on
    // `\nS\n` to avoid colliding with the literal `S` inside an `S/m/l/cm`
    // sequence inside a longer command.
    let pos_s = s.find("\nS\n").expect("S must be emitted");

    assert!(pos_m < pos_l && pos_l < pos_s);
}

#[test]
fn pure_text_only_does_not_regress() {
    let mut page = Page::a4();
    page.text()
        .set_font(oxidize_pdf::text::Font::Helvetica, 12.0)
        .at(100.0, 700.0)
        .write("Hello")
        .unwrap();

    let mut doc = oxidize_pdf::Document::new();
    doc.set_compress(false); // inspect raw operators
    doc.add_page(page);
    let bytes = doc.to_bytes().expect("to_bytes must succeed");
    let s = String::from_utf8_lossy(&bytes);

    let pos_bt = s.find("BT\n").expect("BT must be emitted");
    let pos_label = s.find("(Hello)").expect("Hello literal must be emitted");
    let pos_et = s.find("ET\n").expect("ET must be emitted");

    assert!(pos_bt < pos_label && pos_label < pos_et);
}

/// RED for the residual #227 gap surfaced by the v2.7.0 review:
/// `Page::add_text_flow` writes to `self.content`, which is appended
/// AFTER `page_ops` and the context tails in `generate_content_with_page_info`.
/// A caller that interleaves `graphics()` → `add_text_flow()` → `graphics()`
/// has the flow rendered LAST regardless of its position in call order.
/// The fix routes `add_text_flow` through `page_ops` instead.
#[test]
fn add_text_flow_between_gfx_preserves_call_order() {
    use oxidize_pdf::page::Margins;
    use oxidize_pdf::text::TextFlowContext;

    let mut page = Page::a4();
    page.graphics()
        .set_fill_color(Color::Rgb(0.0, 0.0, 1.0))
        .rect(50.0, 700.0, 100.0, 50.0)
        .fill();

    let mut flow = TextFlowContext::new(595.0, 842.0, Margins::default());
    flow.set_font(oxidize_pdf::text::Font::Helvetica, 12.0);
    flow.at(50.0, 680.0);
    flow.write_wrapped("FlowLabel").unwrap();
    page.add_text_flow(&flow);

    page.graphics()
        .set_fill_color(Color::Rgb(1.0, 0.0, 0.0))
        .rect(50.0, 640.0, 100.0, 30.0)
        .fill();

    let mut doc = oxidize_pdf::Document::new();
    doc.set_compress(false);
    doc.add_page(page);
    let bytes = doc.to_bytes().expect("to_bytes must succeed");
    let s = String::from_utf8_lossy(&bytes);

    let pos_blue = s.find("0.000 0.000 1.000 rg").expect("blue rg must exist");
    let pos_flow = s.find("(FlowLabel)").expect("FlowLabel literal must exist");
    let pos_red = s.find("1.000 0.000 0.000 rg").expect("red rg must exist");

    assert!(
        pos_blue < pos_flow,
        "blue rg ({pos_blue}) must precede the flow text ({pos_flow}) — content stream:\n{s}"
    );
    assert!(
        pos_flow < pos_red,
        "flow text ({pos_flow}) must precede the red rg ({pos_red}) — content stream:\n{s}"
    );
}

/// RED for the residual #227 gap surfaced by the v2.7.0 review:
/// `add_text_flow` interleaved with `text()` calls must respect call
/// order. Pre-fix, `add_text_flow` writes to `self.content` which is
/// emitted last in `generate_content_with_page_info`, so the flow text
/// ends up after the page text regardless of the order of calls.
#[test]
fn add_text_flow_then_page_text_preserves_call_order() {
    use oxidize_pdf::page::Margins;
    use oxidize_pdf::text::TextFlowContext;

    let mut page = Page::a4();

    let mut flow = TextFlowContext::new(595.0, 842.0, Margins::default());
    flow.set_font(oxidize_pdf::text::Font::Helvetica, 12.0);
    flow.at(50.0, 700.0);
    flow.write_wrapped("Flow").unwrap();
    page.add_text_flow(&flow);

    page.text()
        .set_font(oxidize_pdf::text::Font::Helvetica, 12.0)
        .at(50.0, 680.0)
        .write("PageText")
        .unwrap();

    let mut doc = oxidize_pdf::Document::new();
    doc.set_compress(false);
    doc.add_page(page);
    let bytes = doc.to_bytes().expect("to_bytes must succeed");
    let s = String::from_utf8_lossy(&bytes);

    let pos_flow = s.find("(Flow)").expect("(Flow) literal must exist");
    let pos_text = s.find("(PageText)").expect("(PageText) literal must exist");

    assert!(
        pos_flow < pos_text,
        "flow text added first ({pos_flow}) must precede page text ({pos_text}); content stream:\n{s}"
    );
}

/// RED for the IR-internal sanitisation gap on `Op::SetFont { size }`
/// surfaced by the security review: `serialize_ops` formats `size`
/// directly without `finite_or_zero`. A caller passing `f64::NAN` as
/// font size emits `/Helvetica NaN Tf`, an invalid PDF token per
/// ISO 32000-1 §7.3.3.
#[test]
fn nan_font_size_sanitised_at_emission_via_text_context() {
    let mut page = Page::a4();
    page.text()
        .set_font(oxidize_pdf::text::Font::Helvetica, f64::NAN)
        .at(100.0, 700.0)
        .write("hello")
        .unwrap();

    let mut doc = oxidize_pdf::Document::new();
    doc.set_compress(false);
    doc.add_page(page);
    let bytes = doc.to_bytes().expect("to_bytes must succeed");
    let s = String::from_utf8_lossy(&bytes);

    assert!(
        !s.contains("NaN") && !s.contains("/Helvetica inf"),
        "NaN/inf must not appear in `Tf` emission, content stream:\n{s}"
    );
    assert!(
        s.contains("/Helvetica 0 Tf") || s.contains("/Helvetica 0.0 Tf"),
        "NaN font size must clamp to 0 in `Tf`, content stream:\n{s}"
    );
}
