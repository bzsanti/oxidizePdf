//! Issue #212 — verify the writer emits a custom font even when the only
//! reference to it lives in a form-field /DA (no page content stream uses
//! it). Without this, `write_fonts` skipped the font, leaving the AP
//! placeholder dict's `/Resources/Font/<name>` entry unrewritten and the
//! produced PDF malformed.

use oxidize_pdf::forms::{FormManager, TextField, Widget};
use oxidize_pdf::geometry::{Point, Rectangle};
use oxidize_pdf::graphics::Color;
use oxidize_pdf::parser::objects::PdfObject;
use oxidize_pdf::parser::PdfReader;
use oxidize_pdf::text::Font;
use oxidize_pdf::{Document, Page};
use std::io::Cursor;

const LATIN_FONT_PATH: &str = "../test-pdfs/Roboto-Regular.ttf";

fn load_latin_font() -> Option<Vec<u8>> {
    if std::path::Path::new(LATIN_FONT_PATH).exists() {
        Some(std::fs::read(LATIN_FONT_PATH).expect("read Roboto"))
    } else {
        eprintln!("SKIPPED: {LATIN_FONT_PATH} not found");
        None
    }
}

#[test]
fn form_only_custom_font_is_emitted_after_fill_field() {
    let font_data = match load_latin_font() {
        Some(d) => d,
        None => return,
    };

    let mut doc = Document::new();
    doc.add_font_from_bytes("Roboto", font_data)
        .expect("register Roboto");

    let mut page = Page::a4();
    let mut fm = FormManager::new();

    let rect = Rectangle::new(Point::new(50.0, 700.0), Point::new(250.0, 720.0));
    let widget = Widget::new(rect);

    // The custom font is referenced ONLY in this form field's typed DA.
    // No page content stream uses Roboto.
    let field = TextField::new("f1").with_default_appearance(
        Font::Custom("Roboto".to_string()),
        12.0,
        Color::black(),
    );
    let field_ref = fm
        .add_text_field(field, widget.clone(), None)
        .expect("add_text_field");
    page.add_form_widget_with_ref(widget, field_ref)
        .expect("add_form_widget_with_ref");
    doc.add_page(page);
    doc.set_form_manager(fm);

    doc.fill_field("f1", "Hello").expect("fill_field");

    let pdf = doc.to_bytes().expect("serialize");

    // Walk the PDF and find the /AP/N stream's /Resources/Font/Roboto.
    // It must be either an inline Type0 dict (post-rewrite to indirect ref
    // is not always applied for Latin TTFs that subset to Type0 — both
    // shapes are valid here as long as the resolved subtype is Type0).
    let mut reader = PdfReader::new(Cursor::new(&pdf)).expect("PdfReader::new");
    let pages = reader.pages().expect("pages tree").clone();
    let kids = pages
        .get("Kids")
        .and_then(|o| o.as_array())
        .expect("/Pages/Kids");
    let (pn, pg) = kids.0[0].as_reference().expect("page ref");
    let page_obj = reader.get_object(pn, pg).expect("page obj").clone();
    let page_dict = page_obj.as_dict().expect("page dict").clone();
    let annots = page_dict
        .get("Annots")
        .and_then(|o| o.as_array())
        .expect("/Annots");
    let (an, ag) = annots.0[0].as_reference().expect("annot ref");
    let annot_obj = reader.get_object(an, ag).expect("annot obj").clone();
    let annot_dict = annot_obj.as_dict().expect("annot dict").clone();
    let ap = annot_dict
        .get("AP")
        .and_then(|o| o.as_dict())
        .expect("/AP")
        .clone();
    let n_entry = ap.get("N").expect("/AP/N").clone();

    let ap_stream_dict = match n_entry {
        PdfObject::Reference(n2, g2) => {
            let s = reader.get_object(n2, g2).expect("AP/N stream").clone();
            s.as_stream().expect("AP/N is stream").dict.clone()
        }
        PdfObject::Stream(ref s) => s.dict.clone(),
        other => panic!("unexpected /AP/N: {other:?}"),
    };

    let resources = ap_stream_dict
        .get("Resources")
        .and_then(|o| o.as_dict())
        .expect("/AP/N /Resources");
    let fonts = resources
        .get("Font")
        .and_then(|o| o.as_dict())
        .expect("/AP/N /Resources/Font");
    let roboto_entry = fonts.get("Roboto").expect("/AP/N /Resources/Font/Roboto");

    let resolved_subtype = match roboto_entry {
        PdfObject::Reference(n3, g3) => {
            // Indirect ref — the writer rewrote the placeholder. The font
            // object itself must exist and have /Subtype /Type0. If
            // write_fonts had skipped this font, the indirect ref would
            // either be unresolvable or still point to the placeholder.
            let mut reader2 = PdfReader::new(Cursor::new(&pdf)).expect("reader2");
            let font_obj = reader2.get_object(*n3, *g3).expect("resolve").clone();
            let fd = font_obj.as_dict().expect("resolved dict").clone();
            fd.get("Subtype")
                .and_then(|o| o.as_name())
                .map(|nm| nm.as_str().to_string())
                .unwrap_or_default()
        }
        PdfObject::Dictionary(d) => d
            .get("Subtype")
            .and_then(|o| o.as_name())
            .map(|nm| nm.as_str().to_string())
            .unwrap_or_default(),
        other => panic!("unexpected font entry shape: {other:?}"),
    };

    assert_eq!(
        resolved_subtype, "Type0",
        "/AP/N /Resources/Font/Roboto must resolve to /Subtype /Type0 even \
         when the font is referenced only in a form field's /DA. Got {resolved_subtype:?} \
         — this would mean write_fonts skipped the form-only font (issue #212 guard regression)."
    );
}
