//! Fill AcroForm fields on an EXISTING (already-serialized) PDF via an
//! ISO 32000-1 §7.5.6 incremental update — issue #318.
//!
//! This is the standard "fill a form template" use case: take a PDF that
//! already contains AcroForm fields and set their values so a form reader
//! recovers them, without rewriting the original document.
//!
//! Run with: `cargo run --example fill_existing_form`

use oxidize_pdf::forms::{FormManager, TextField, Widget, WidgetAppearance};
use oxidize_pdf::geometry::{Point, Rectangle};
use oxidize_pdf::parser::objects::PdfObject;
use oxidize_pdf::parser::PdfReader;
use oxidize_pdf::writer::IncrementalFormFiller;
use oxidize_pdf::{Document, Page};
use std::io::Cursor;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // --- Produce a base form PDF (stands in for a template from Acrobat) ---
    let mut doc = Document::new();
    let mut page = Page::a4();
    let mut fm = FormManager::new();
    let rect = Rectangle::new(Point::new(100.0, 700.0), Point::new(320.0, 720.0));
    let widget = Widget::new(rect).with_appearance(WidgetAppearance::default());
    fm.add_text_field(TextField::new("full_name"), widget.clone(), None)
        .and_then(|field_ref| page.add_form_widget_with_ref(widget, field_ref))?;
    doc.add_page(page);
    doc.set_form_manager(fm);
    let base_pdf: Vec<u8> = doc.to_bytes()?;

    // --- Fill the existing field on the parsed bytes (incremental update) ---
    let filled: Vec<u8> =
        IncrementalFormFiller::new(&base_pdf).fill("full_name", "Ada Lovelace")?;

    // The base bytes are preserved verbatim; the value lives in the appended
    // section. A form reader recovers /V after re-parsing.
    assert_eq!(&filled[..base_pdf.len()], &base_pdf[..]);

    let recovered = read_field_value(&filled, "full_name")?;
    println!("Base PDF: {} bytes", base_pdf.len());
    println!(
        "Filled PDF: {} bytes (+{} appended)",
        filled.len(),
        filled.len() - base_pdf.len()
    );
    println!("Recovered /V for 'full_name': {recovered:?}");

    std::fs::write("filled_form.pdf", &filled)?;
    println!("Wrote filled_form.pdf");
    Ok(())
}

/// Walk /AcroForm/Fields and return the `/V` string of a terminal field.
fn read_field_value(
    bytes: &[u8],
    name: &str,
) -> Result<Option<String>, Box<dyn std::error::Error>> {
    let mut reader = PdfReader::new(Cursor::new(bytes))?;
    let catalog = reader.catalog()?.clone();
    let acro_ref = catalog
        .get("AcroForm")
        .and_then(|o| o.as_reference())
        .ok_or("no /AcroForm")?;
    let acro = reader
        .get_object(acro_ref.0, acro_ref.1)?
        .as_dict()
        .cloned()
        .ok_or("AcroForm not dict")?;
    let fields = match acro.get("Fields") {
        Some(PdfObject::Array(a)) => a.0.clone(),
        _ => Vec::new(),
    };
    for f in fields {
        if let Some((n, g)) = f.as_reference() {
            let dict = reader
                .get_object(n, g)?
                .as_dict()
                .cloned()
                .ok_or("field not dict")?;
            let t = dict
                .get("T")
                .and_then(|o| o.as_string())
                .map(|s| String::from_utf8_lossy(s.as_bytes()).into_owned());
            if t.as_deref() == Some(name) {
                return Ok(dict
                    .get("V")
                    .and_then(|o| o.as_string())
                    .map(|s| String::from_utf8_lossy(s.as_bytes()).into_owned()));
            }
        }
    }
    Ok(None)
}
