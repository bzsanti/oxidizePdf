//! Regression tests for issue #239 — `Page::set_fill_color` on the
//! graphics context did not propagate to text rendering. Per ISO 32000-1
//! §8.6.8, `rg` is the **non-stroking colour** of the graphics state and
//! applies both to path fills and to glyph fills at text rendering mode
//! 0 (default). Pre-fix, `GraphicsContext.current_color` and
//! `TextContext.fill_color` were independent slots, so a caller that
//! set the fill colour via `graphics().set_fill_color(...)` and then
//! emitted text saw the text painted in whichever colour the last
//! emitted `rg` had left in the stream — typically the colour of the
//! previous path fill, never the colour the caller intended for the
//! text.
//!
//! The fix is a handoff in `Page::text()` (and `Page::text_flow()`):
//! when the text context has no explicit fill colour of its own, it
//! inherits the current graphics-state fill colour. An explicit
//! `text().set_fill_color(...)` still overrides the inherited value.
//!
//! These tests are content-verifying (no smoke): they parse the
//! uncompressed PDF content stream and assert that the expected `rg`
//! / `g` operator appears inside the `BT … ET` block that surrounds the
//! show-text literal.

use oxidize_pdf::graphics::Color;
use oxidize_pdf::parser::{PdfDocument, PdfReader};
use oxidize_pdf::text::TextExtractor;
use oxidize_pdf::{Document, Font, Page};
use std::io::Cursor;

/// Build an uncompressed PDF from `page` so the content stream is
/// directly observable in the byte output.
fn render(page: Page) -> Vec<u8> {
    let mut doc = Document::new();
    doc.set_compress(false);
    doc.add_page(page);
    doc.to_bytes().expect("to_bytes must succeed")
}

/// Find the `BT … ET` block that contains `needle` (a substring of the
/// content stream, typically a `(text)` literal we just emitted), and
/// return the bytes between the matching `BT\n` and `ET\n`. Used to
/// scope assertions about non-stroking colour to the text block — the
/// graphics-state `rg` that preceded the block is not relevant for what
/// the GLYPHS are painted in, only the `rg` inside the block is.
fn bt_et_window_containing<'a>(stream: &'a [u8], needle: &[u8]) -> &'a [u8] {
    let pos = find_subsequence(stream, needle).unwrap_or_else(|| {
        panic!(
            "needle {:?} not found in content stream:\n{}",
            std::str::from_utf8(needle).unwrap_or("<binary>"),
            String::from_utf8_lossy(stream),
        )
    });
    // Walk backwards for the nearest `BT\n` start.
    let bt = b"BT\n";
    let bt_start = (0..=pos.saturating_sub(bt.len()))
        .rev()
        .find(|&i| stream[i..].starts_with(bt))
        .unwrap_or_else(|| {
            panic!(
                "no `BT\\n` precedes needle in content stream:\n{}",
                String::from_utf8_lossy(stream),
            )
        });
    // Walk forwards for the next `ET\n`.
    let et = b"ET\n";
    let et_end_offset = find_subsequence(&stream[pos..], et).unwrap_or_else(|| {
        panic!(
            "no `ET\\n` after needle in content stream:\n{}",
            String::from_utf8_lossy(stream),
        )
    });
    &stream[bt_start..pos + et_end_offset + et.len()]
}

fn find_subsequence(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

fn window_contains(window: &[u8], needle: &[u8]) -> bool {
    window.windows(needle.len()).any(|w| w == needle)
}

/// Reproductor literal del issue body: una banda magenta dibujada con
/// `graphics().set_fill_color(magenta).fill()`, luego
/// `graphics().set_fill_color(white)` y `text().write("Hello white")`.
/// El texto debe pintarse en BLANCO porque el caller dejó el último
/// non-stroking colour del graphics state en blanco antes de emitir el
/// texto, y el texto NO tiene colour explícito propio.
///
/// Pre-fix, el `BT … ET` de "Hello white" no contenía ningún `rg`, y el
/// último `rg` del stream era el magenta de la banda; el viewer pintaba
/// el texto en magenta.
#[test]
fn graphics_fill_color_inherited_by_text_when_text_has_no_explicit_color() {
    let mut page = Page::a4();

    page.graphics()
        .set_fill_color(Color::Rgb(0.851, 0.275, 0.937))
        .rect(0.0, 600.0, 595.0, 200.0)
        .fill();

    page.graphics().set_fill_color(Color::Rgb(1.0, 1.0, 1.0));

    page.text()
        .set_font(Font::HelveticaBold, 32.0)
        .at(50.0, 700.0)
        .write("Hello white")
        .expect("text().write must succeed");

    let pdf = render(page);
    let window = bt_et_window_containing(&pdf, b"(Hello white)");

    assert!(
        window_contains(window, b"1.000 1.000 1.000 rg"),
        "white `rg` must appear inside the BT…ET that emits (Hello white) — \
         text inherits the current graphics-state non-stroking colour per \
         ISO 32000-1 §8.6.8.\nBT…ET window:\n{}",
        String::from_utf8_lossy(window),
    );
}

/// Override semantics: an explicit `text().set_fill_color(blue)` after
/// `graphics().set_fill_color(red)` must win. The text block emits the
/// blue `rg`, not the red one inherited from the graphics state.
#[test]
fn explicit_text_fill_color_overrides_graphics_color() {
    let mut page = Page::a4();

    page.graphics()
        .set_fill_color(Color::Rgb(1.0, 0.0, 0.0))
        .rect(0.0, 600.0, 595.0, 200.0)
        .fill();

    page.text()
        .set_font(Font::Helvetica, 12.0)
        .set_fill_color(Color::Rgb(0.0, 0.0, 1.0))
        .at(50.0, 700.0)
        .write("Blue wins")
        .expect("text().write must succeed");

    let pdf = render(page);
    let window = bt_et_window_containing(&pdf, b"(Blue wins)");

    assert!(
        window_contains(window, b"0.000 0.000 1.000 rg"),
        "explicit blue `rg` must appear inside (Blue wins) BT…ET; got:\n{}",
        String::from_utf8_lossy(window),
    );
    assert!(
        !window_contains(window, b"1.000 0.000 0.000 rg"),
        "inherited red `rg` must NOT appear inside (Blue wins) BT…ET; got:\n{}",
        String::from_utf8_lossy(window),
    );
}

/// Guard contra `is_none()` mal aplicado: si el caller fija un colour
/// explícito en `text()` ANTES de cualquier `graphics().set_fill_color`,
/// el handoff posterior NO debe sobreescribirlo. Secuencia:
///   1. `text().set_fill_color(green)` → text_context.fill_color = Some(green).
///   2. `graphics().set_fill_color(red).fill()` → graphics state = red.
///   3. `text().write("X")` → segundo handoff; el guard `is_none()` es
///      `false`, por tanto el verde sobrevive.
#[test]
fn text_fill_color_set_before_graphics_is_not_overwritten_by_handoff() {
    let mut page = Page::a4();

    page.text()
        .set_font(Font::Helvetica, 12.0)
        .set_fill_color(Color::Rgb(0.0, 1.0, 0.0));

    page.graphics()
        .set_fill_color(Color::Rgb(1.0, 0.0, 0.0))
        .rect(0.0, 600.0, 595.0, 200.0)
        .fill();

    page.text()
        .at(50.0, 700.0)
        .write("Green stays")
        .expect("text().write must succeed");

    let pdf = render(page);
    let window = bt_et_window_containing(&pdf, b"(Green stays)");

    assert!(
        window_contains(window, b"0.000 1.000 0.000 rg"),
        "explicit green `rg` set before graphics must persist; got:\n{}",
        String::from_utf8_lossy(window),
    );
    assert!(
        !window_contains(window, b"1.000 0.000 0.000 rg"),
        "red from graphics state must NOT overwrite the prior explicit \
         text colour; got:\n{}",
        String::from_utf8_lossy(window),
    );
}

/// Edge case behavioural change: una página recién creada (sin que
/// nadie haya llamado `set_fill_color`) ahora emite el operador del
/// colour por defecto del graphics state (`Color::Gray(0.0)`) dentro
/// del `BT … ET` del texto, en lugar de no emitir ningún colour. El
/// resultado visual es idéntico (negro = default) pero el contenido del
/// stream cambia. Documentado en CHANGELOG.
#[test]
fn graphics_default_black_propagates_to_text_on_fresh_page() {
    let mut page = Page::a4();

    page.text()
        .set_font(Font::Helvetica, 12.0)
        .at(50.0, 700.0)
        .write("Default")
        .expect("text().write must succeed");

    let pdf = render(page);
    let window = bt_et_window_containing(&pdf, b"(Default)");

    // `Color::black()` is `Color::Gray(0.0)`, which serialises as
    // `0.000 g` (gray non-stroking colour) per ISO 32000-1 §8.6.8.
    assert!(
        window_contains(window, b"0.000 g"),
        "default black `0.000 g` must appear inside the BT…ET on a fresh \
         page — the graphics-state default colour is now propagated. \
         Window:\n{}",
        String::from_utf8_lossy(window),
    );
}

/// `Page::text_flow()` también debe heredar el non-stroking colour del
/// graphics state cuando el text_context no tiene colour propio. Pre-fix,
/// `Page::text_flow()` SOLO copiaba `text_context.fill_color()`; el del
/// graphics state se perdía.
#[test]
fn text_flow_inherits_graphics_color_when_text_context_has_no_explicit_color() {
    let mut page = Page::a4();
    page.set_margins(50.0, 50.0, 50.0, 50.0);

    page.graphics()
        .set_fill_color(Color::Rgb(0.5, 0.0, 0.0))
        .rect(0.0, 600.0, 595.0, 200.0)
        .fill();

    let mut flow = page.text_flow();
    flow.set_font(Font::Helvetica, 14.0);
    flow.at(100.0, 700.0);
    flow.write_wrapped("Maroon flow")
        .expect("write_wrapped must succeed");
    page.add_text_flow(&flow);

    let pdf = render(page);
    let window = bt_et_window_containing(&pdf, b"(Maroon flow)");

    assert!(
        window_contains(window, b"0.500 0.000 0.000 rg"),
        "graphics-state maroon `rg` must propagate into the BT…ET of the \
         flow when the text_context has no explicit colour. Window:\n{}",
        String::from_utf8_lossy(window),
    );
}

/// Integración end-to-end del reproductor exacto del issue #239. Además
/// de inspeccionar el stream emitido (cobertura del Ciclo 5), parseamos
/// el PDF resultante con `PdfReader` + `TextExtractor` y verificamos
/// que el texto "Hello white" sobrevive el round-trip. Esto asegura
/// que el fix de colour no rompió la extracción de texto (regresión
/// trivial pero documentada).
#[test]
fn issue_239_reproducer_white_text_on_colored_band_round_trip() {
    let mut page = Page::a4();
    page.graphics()
        .set_fill_color(Color::Rgb(0.851, 0.275, 0.937))
        .rect(0.0, 600.0, 595.0, 200.0)
        .fill();
    page.graphics().set_fill_color(Color::Rgb(1.0, 1.0, 1.0));
    page.text()
        .set_font(Font::HelveticaBold, 32.0)
        .at(50.0, 700.0)
        .write("Hello white")
        .expect("text().write must succeed");

    let pdf = render(page);

    // Stream-level check: white `rg` precedes the show-text inside BT…ET.
    let window = bt_et_window_containing(&pdf, b"(Hello white)");
    assert!(
        window_contains(window, b"1.000 1.000 1.000 rg"),
        "stream-level: white rg must live inside (Hello white) BT…ET",
    );

    // Round-trip via the parser: the extracted text must contain the
    // exact phrase we wrote — the fix must not alter the text payload.
    let reader = PdfReader::new(Cursor::new(pdf)).expect("PdfReader must open in-memory PDF");
    let doc = PdfDocument::new(reader);
    let mut extractor = TextExtractor::new();
    let extracted = extractor
        .extract_from_document(&doc)
        .expect("extract_from_document must succeed");
    let all_text: String = extracted
        .iter()
        .map(|page_text| page_text.text.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        all_text.contains("Hello white"),
        "round-trip extracted text must contain `Hello white`; got: {all_text:?}",
    );
}
