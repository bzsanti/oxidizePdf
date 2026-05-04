/// Example: `fill_field` with a CJK (Type0/CID) custom font — issue #212.
///
/// Before this fix, `Document::fill_field` always produced the widget
/// appearance stream with Helvetica + WinAnsi, which silently corrupted any
/// non-WinAnsi value (CJK, Arabic, Hebrew, and even Latin-extended like
/// `é ñ ü`). The value stored in `/V` was fine, but the `/AP` stream that
/// viewers render was wrong.
///
/// The fix requires three things the user controls and one the library
/// handles automatically:
///
/// 1. Register the custom font on the `Document`
///    (`add_font_from_bytes`).
/// 2. Attach a typed `/DA` to the `TextField` naming that font via
///    `with_default_appearance(Font::Custom("...".into()), size, color)`.
/// 3. Call `fill_field` normally. The library routes through the Type0/CID
///    path, emits hex-encoded glyph indices in the content stream, and
///    wires `/AP/N /Resources/Font/<name>` to an indirect reference to the
///    document-level Type0 font.
///
/// The fixture font path matches the one used by the test suite. If it's
/// not present the example logs a skip message instead of failing.
use oxidize_pdf::forms::{FormManager, TextField, Widget, WidgetAppearance};
use oxidize_pdf::geometry::{Point, Rectangle};
use oxidize_pdf::graphics::Color;
use oxidize_pdf::text::Font;
use oxidize_pdf::{Document, Page};

const CJK_FONT_PATH: &str = "../test-pdfs/SourceHanSansSC-Regular.otf";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== fill_field with CJK custom font (issue #212) ===\n");

    let cjk_data = match std::fs::read(CJK_FONT_PATH) {
        Ok(d) => d,
        Err(e) => {
            eprintln!(
                "SKIPPED: CJK fixture not available ({}): {}",
                CJK_FONT_PATH, e
            );
            return Ok(());
        }
    };

    // 1) Register the custom Type0 font on the document.
    let mut doc = Document::new();
    doc.set_title("Issue #212 — fill_field CJK demo");
    doc.add_font_from_bytes("SourceHanSansSC", cjk_data)?;

    // 2) Build a page with a single text field. The widget carries a
    //    typed /DA that names the custom font — this is the switch that
    //    unlocks the Type0/CID path in fill_field.
    let mut page = Page::a4();
    let mut fm = FormManager::new();

    // Place the widget near the top so the example PDF is visually clear.
    let rect = Rectangle::new(Point::new(72.0, 740.0), Point::new(522.0, 770.0));
    let widget = Widget::new(rect).with_appearance(WidgetAppearance::default());

    let field = TextField::new("message").with_default_appearance(
        Font::Custom("SourceHanSansSC".to_string()),
        16.0,
        Color::black(),
    );
    let field_ref = fm.add_text_field(field, widget.clone(), None)?;
    page.add_form_widget_with_ref(widget, field_ref)?;

    // Visible label above the widget to anchor the demo. The label itself
    // uses the built-in Helvetica path (works because it's ASCII).
    page.text()
        .set_font(Font::Helvetica, 10.0)
        .at(72.0, 780.0)
        .write("Multilingual fill via fill_field (issue #212):")?;

    doc.add_page(page);
    doc.set_form_manager(fm);

    // 3) Fill with a value that contains CJK, Latin-extended, and ASCII all
    //    at once. Any one of those would have silently corrupted the /AP
    //    stream before the fix.
    let value = "高效能 PDF café — résumé";
    doc.fill_field("message", value)?;
    println!("Field 'message' filled with: {:?}", value);

    let out_path = "examples/results/fill_field_cjk.pdf";
    doc.save(out_path)?;
    println!("Saved: {}", out_path);
    println!();
    println!("Open the PDF in any viewer — the field should render the full");
    println!("value (CJK + Latin-extended) via the document-level Type0 font.");
    println!("Pre-fix output would have shown .notdef boxes or mojibake.");

    Ok(())
}
