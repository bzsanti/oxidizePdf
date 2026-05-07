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
