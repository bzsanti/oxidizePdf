//! Integration tests for issue #318: filling AcroForm fields on an existing
//! (parsed) PDF via an ISO 32000-1 §7.5.6 incremental update.
//!
//! Every assertion verifies real wire content after a full
//! parse -> fill -> serialize -> parse round-trip — no smoke tests.

use oxidize_pdf::forms::{FormManager, TextField, Widget, WidgetAppearance};
use oxidize_pdf::geometry::{Point, Rectangle};
use oxidize_pdf::parser::objects::PdfObject;
use oxidize_pdf::parser::PdfReader;
use oxidize_pdf::writer::IncrementalFormFiller;
use oxidize_pdf::{Document, Page};
use std::io::Cursor;

// ---------------------------------------------------------------------------
// Fixture helpers
// ---------------------------------------------------------------------------

/// Build a single-page PDF carrying AcroForm text fields with the given
/// names (no values set), serialized to bytes through the public writer.
fn build_base_pdf_with_fields(names: &[&str]) -> Vec<u8> {
    let mut doc = Document::new();
    let mut page = Page::a4();
    let mut fm = FormManager::new();

    let mut y = 700.0;
    for name in names {
        let rect = Rectangle::new(Point::new(100.0, y), Point::new(300.0, y + 20.0));
        let widget = Widget::new(rect).with_appearance(WidgetAppearance::default());
        let field = TextField::new(*name);
        let field_ref = fm
            .add_text_field(field, widget.clone(), None)
            .expect("add_text_field");
        page.add_form_widget_with_ref(widget, field_ref)
            .expect("add_form_widget_with_ref");
        y -= 40.0;
    }

    doc.add_page(page);
    doc.set_form_manager(fm);
    doc.to_bytes().expect("serialize base document")
}

/// Resolve `/AcroForm` -> (object id, dict clone) from PDF bytes.
fn acroform_object(bytes: &[u8]) -> ((u32, u16), oxidize_pdf::parser::objects::PdfDictionary) {
    let mut reader = PdfReader::new(Cursor::new(bytes)).expect("parse");
    let catalog = reader.catalog().expect("catalog").clone();
    let acro_ref = catalog
        .get("AcroForm")
        .expect("/AcroForm present")
        .as_reference()
        .expect("/AcroForm indirect");
    let dict = reader
        .get_object(acro_ref.0, acro_ref.1)
        .expect("resolve AcroForm")
        .as_dict()
        .expect("AcroForm dict")
        .clone();
    (acro_ref, dict)
}

/// Recover a terminal field's object id and `/V` string by fully-qualified
/// name from PDF bytes (walks /Fields and /Kids).
fn field_value(bytes: &[u8], name: &str) -> Option<((u32, u16), Option<String>)> {
    let mut reader = PdfReader::new(Cursor::new(bytes)).expect("parse");
    let (_, acro) = acroform_object(bytes);
    let field_refs: Vec<(u32, u16)> = match acro.get("Fields") {
        Some(PdfObject::Array(arr)) => arr.0.iter().filter_map(|o| o.as_reference()).collect(),
        _ => Vec::new(),
    };
    for r in field_refs {
        if let Some(found) = walk_field(&mut reader, r, "", name) {
            return Some(found);
        }
    }
    None
}

fn walk_field(
    reader: &mut PdfReader<Cursor<&[u8]>>,
    node_ref: (u32, u16),
    prefix: &str,
    target: &str,
) -> Option<((u32, u16), Option<String>)> {
    let node = reader
        .get_object(node_ref.0, node_ref.1)
        .ok()?
        .as_dict()?
        .clone();
    let partial = node
        .get("T")
        .and_then(|o| o.as_string())
        .map(|s| String::from_utf8_lossy(s.as_bytes()).into_owned());
    let full = match (&partial, prefix.is_empty()) {
        (Some(t), true) => t.clone(),
        (Some(t), false) => format!("{prefix}.{t}"),
        (None, _) => prefix.to_string(),
    };
    let kids: Vec<(u32, u16)> = match node.get("Kids") {
        Some(PdfObject::Array(arr)) => arr.0.iter().filter_map(|o| o.as_reference()).collect(),
        _ => Vec::new(),
    };
    if kids.is_empty() {
        if partial.is_some() && full == target {
            let v = node
                .get("V")
                .and_then(|o| o.as_string())
                .map(|s| String::from_utf8_lossy(s.as_bytes()).into_owned());
            return Some((node_ref, v));
        }
        None
    } else {
        for kid in kids {
            if let Some(found) = walk_field(reader, kid, &full, target) {
                return Some(found);
            }
        }
        None
    }
}

/// Parse the object numbers listed in an appended xref section (raw text
/// `xref` format), returning every object number across all subsections.
fn appended_xref_object_numbers(appended: &[u8]) -> Vec<u32> {
    let s = String::from_utf8_lossy(appended);
    let xref_pos = s.find("xref").expect("appended bytes must contain xref");
    let after = &s[xref_pos + 4..];
    let mut nums = Vec::new();
    let mut lines = after.lines().filter(|l| !l.trim().is_empty());
    while let Some(header) = lines.next() {
        let header = header.trim();
        if header.starts_with("trailer") {
            break;
        }
        let parts: Vec<&str> = header.split_whitespace().collect();
        if parts.len() != 2 {
            break;
        }
        let (start, count): (u32, u32) = match (parts[0].parse(), parts[1].parse()) {
            (Ok(a), Ok(b)) => (a, b),
            _ => break,
        };
        for i in 0..count {
            // consume one entry line per object
            if lines.next().is_some() {
                nums.push(start + i);
            }
        }
    }
    nums
}

/// Read the base PDF's most-recent startxref offset.
fn base_startxref(bytes: &[u8]) -> u64 {
    let reader = PdfReader::new(Cursor::new(bytes)).expect("parse");
    reader.trailer().xref_offset
}

/// Base trailer `/Size` (= the first object id a fresh appearance stream takes).
fn base_size(bytes: &[u8]) -> u32 {
    let reader = PdfReader::new(Cursor::new(bytes)).expect("parse");
    reader.trailer().size().expect("base /Size") as u32
}

/// Object ids of the first page's widget annotations (the bearers of /AP after
/// a fill in the separate-widget layouts).
fn page_widget_ids(bytes: &[u8]) -> Vec<u32> {
    let mut reader = PdfReader::new(Cursor::new(bytes)).expect("parse");
    let pages = reader.pages().expect("pages").clone();
    let (pn, pg) = match pages.get("Kids").and_then(|o| o.as_array()) {
        Some(arr) => arr.0[0].as_reference().expect("page ref"),
        None => return Vec::new(),
    };
    let page = reader
        .get_object(pn, pg)
        .expect("page")
        .as_dict()
        .expect("page dict")
        .clone();
    match page.get("Annots") {
        Some(PdfObject::Array(arr)) => arr
            .0
            .iter()
            .filter_map(|o| o.as_reference().map(|(n, _)| n))
            .collect(),
        _ => Vec::new(),
    }
}

/// Read `/Prev` from an appended incremental trailer (text form).
fn appended_prev(appended: &[u8]) -> u64 {
    let s = String::from_utf8_lossy(appended);
    let pos = s.find("/Prev").expect("appended trailer must carry /Prev");
    let after = &s[pos + 5..];
    after
        .split_whitespace()
        .next()
        .expect("number after /Prev")
        .parse()
        .expect("/Prev integer")
}

// ---------------------------------------------------------------------------
// Cycle 4: single-field round-trip
// ---------------------------------------------------------------------------

#[test]
fn fill_single_field_incremental_roundtrip() {
    let base = build_base_pdf_with_fields(&["email"]);

    // The fixture must parse and the field must start without a value.
    let (field_id_before, v_before) = field_value(&base, "email").expect("field present in base");
    assert!(
        v_before.is_none() || v_before.as_deref() == Some(""),
        "base field must start empty, got {v_before:?}"
    );

    let prev_offset = base_startxref(&base);
    let (acro_id, _) = acroform_object(&base);

    let output = IncrementalFormFiller::new(&base)
        .fill("email", "hello@test.com")
        .expect("fill must succeed");

    // 1) Base bytes are a verbatim prefix (true incremental update).
    assert_eq!(
        &output[..base.len()],
        &base[..],
        "incremental update must preserve base bytes verbatim"
    );
    assert!(output.len() > base.len(), "must append an update section");

    // 2) Re-parse and recover the exact /V.
    let (field_id_after, v_after) = field_value(&output, "email").expect("field present in output");
    assert_eq!(
        v_after.as_deref(),
        Some("hello@test.com"),
        "recovered /V must equal the filled value"
    );
    // Same object id reused (incremental, not a new field object).
    assert_eq!(
        field_id_before, field_id_after,
        "field object id must be reused"
    );

    // 3) NeedAppearances flipped true.
    let (_, acro_after) = acroform_object(&output);
    assert_eq!(
        acro_after.get("NeedAppearances").and_then(|o| o.as_bool()),
        Some(true),
        "/AcroForm/NeedAppearances must be true"
    );

    // 4) Appended xref lists the changed objects: field + its widget + AcroForm
    //    + one synthesized /AP appearance stream (the new follow-up behaviour).
    let appended = &output[base.len()..];
    let mut ids = appended_xref_object_numbers(appended);
    ids.sort_unstable();
    let widget_ids = page_widget_ids(&base);
    let ap_stream_id = base_size(&base); // single text field -> single new stream
    let mut expected = vec![field_id_after.0, acro_id.0, ap_stream_id];
    expected.extend(widget_ids);
    expected.sort_unstable();
    expected.dedup();
    assert_eq!(
        ids, expected,
        "appended xref must list the field, its widget, the AcroForm and the new /AP stream"
    );

    // 5) /Prev chains to the base startxref.
    assert_eq!(
        appended_prev(appended),
        prev_offset,
        "/Prev must equal the base startxref offset"
    );
}

// ---------------------------------------------------------------------------
// Cycle 5a: multi-field fill
// ---------------------------------------------------------------------------

#[test]
fn fill_many_fields_incremental_roundtrip() {
    let base = build_base_pdf_with_fields(&["first_name", "last_name"]);

    let output = IncrementalFormFiller::new(&base)
        .fill_many(&[("first_name", "Jane"), ("last_name", "Doe")])
        .expect("fill_many must succeed");

    assert_eq!(&output[..base.len()], &base[..], "verbatim prefix");

    let (_, first) = field_value(&output, "first_name").expect("first_name present");
    let (_, last) = field_value(&output, "last_name").expect("last_name present");
    assert_eq!(first.as_deref(), Some("Jane"));
    assert_eq!(last.as_deref(), Some("Doe"));

    let (_, acro_after) = acroform_object(&output);
    assert_eq!(
        acro_after.get("NeedAppearances").and_then(|o| o.as_bool()),
        Some(true)
    );

    // Appended xref: 2 fields + 2 widgets + AcroForm + 2 /AP streams = 7 ids.
    let ids = appended_xref_object_numbers(&output[base.len()..]);
    assert_eq!(
        ids.len(),
        7,
        "two fields + two widgets + AcroForm + two /AP streams, got {ids:?}"
    );
}

#[test]
fn fill_unknown_field_errors() {
    let base = build_base_pdf_with_fields(&["email"]);
    let err = IncrementalFormFiller::new(&base).fill("does_not_exist", "x");
    assert!(err.is_err(), "filling a missing field must error");
}

// ---------------------------------------------------------------------------
// Cycle 5b: genuine hierarchical field tree (/Kids + /Parent)
// ---------------------------------------------------------------------------

/// Hand-craft a minimal valid PDF whose AcroForm has a parent field
/// (`T = "address"`) with two terminal kids (`street`, `city`), exercising
/// the recursive field-tree walk and fully-qualified naming.
fn build_hierarchical_form_pdf() -> Vec<u8> {
    // Object layout:
    //  1: Catalog  2: Pages  3: Page  4: AcroForm
    //  5: parent field "address"  6: kid "street"  7: kid "city"
    let objects: Vec<String> = vec![
        // 1 Catalog
        "<< /Type /Catalog /Pages 2 0 R /AcroForm 4 0 R >>".to_string(),
        // 2 Pages
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_string(),
        // 3 Page
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] >>".to_string(),
        // 4 AcroForm
        "<< /Fields [5 0 R] >>".to_string(),
        // 5 parent field
        "<< /T (address) /Kids [6 0 R 7 0 R] >>".to_string(),
        // 6 kid street
        "<< /FT /Tx /T (street) /Parent 5 0 R >>".to_string(),
        // 7 kid city
        "<< /FT /Tx /T (city) /Parent 5 0 R >>".to_string(),
    ];

    let mut pdf = Vec::new();
    pdf.extend_from_slice(b"%PDF-1.7\n");
    let mut offsets = Vec::with_capacity(objects.len());
    for (i, body) in objects.iter().enumerate() {
        offsets.push(pdf.len() as u64);
        pdf.extend_from_slice(format!("{} 0 obj\n{}\nendobj\n", i + 1, body).as_bytes());
    }
    let xref_pos = pdf.len() as u64;
    let n = objects.len() + 1; // +1 for free object 0
    pdf.extend_from_slice(format!("xref\n0 {n}\n").as_bytes());
    pdf.extend_from_slice(b"0000000000 65535 f \n");
    for off in &offsets {
        pdf.extend_from_slice(format!("{off:010} 00000 n \n").as_bytes());
    }
    pdf.extend_from_slice(
        format!("trailer\n<< /Size {n} /Root 1 0 R >>\nstartxref\n{xref_pos}\n%%EOF\n").as_bytes(),
    );
    pdf
}

#[test]
fn fill_hierarchical_field_incremental() {
    let base = build_hierarchical_form_pdf();

    // Sanity: the hand-crafted base parses and the qualified name resolves.
    let (street_id, v_before) =
        field_value(&base, "address.street").expect("address.street must resolve in base");
    assert!(v_before.is_none(), "kid starts empty");

    let output = IncrementalFormFiller::new(&base)
        .fill("address.street", "221B Baker St")
        .expect("fill hierarchical field");

    assert_eq!(&output[..base.len()], &base[..], "verbatim prefix");

    let (street_id_after, v_after) =
        field_value(&output, "address.street").expect("resolve after fill");
    assert_eq!(v_after.as_deref(), Some("221B Baker St"));
    assert_eq!(street_id, street_id_after, "kid object id reused");

    // The other kid is untouched.
    let (_, city_v) = field_value(&output, "address.city").expect("city still resolvable");
    assert!(city_v.is_none(), "untouched kid keeps no value");

    // Only the street kid (obj 6) + AcroForm (obj 4) change.
    let mut ids = appended_xref_object_numbers(&output[base.len()..]);
    ids.sort_unstable();
    assert_eq!(ids, vec![4, 6], "only street kid + AcroForm rewritten");
}

// ---------------------------------------------------------------------------
// Terminal field whose /Kids are WIDGET annotations (not sub-fields)
// ---------------------------------------------------------------------------

/// Read `/V` of a specific object id directly (bypasses the field-tree walk,
/// so it works regardless of widget-vs-subfield kid classification).
fn object_v(bytes: &[u8], num: u32, gen: u16) -> Option<String> {
    let mut reader = PdfReader::new(Cursor::new(bytes)).expect("parse");
    let dict = reader.get_object(num, gen).ok()?.as_dict()?.clone();
    dict.get("V")
        .and_then(|o| o.as_string())
        .map(|s| String::from_utf8_lossy(s.as_bytes()).into_owned())
}

/// The most common real-world layout (Acrobat): a single terminal field
/// dict that carries `/T`/`/FT` AND a `/Kids` array whose elements are
/// widget annotation dicts (`/Subtype /Widget`, NO `/T`). The field
/// resolver must treat this node as terminal — recursing into the widgets
/// (which have no `/T`) would lose the field entirely.
fn build_widget_kids_form_pdf() -> Vec<u8> {
    // 1 Catalog  2 Pages  3 Page  4 AcroForm
    // 5 terminal field "email" with widget kid 6
    // 6 widget annotation (no /T)
    let objects: Vec<String> = vec![
        "<< /Type /Catalog /Pages 2 0 R /AcroForm 4 0 R >>".to_string(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_string(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Annots [6 0 R] >>".to_string(),
        "<< /Fields [5 0 R] >>".to_string(),
        "<< /FT /Tx /T (email) /Kids [6 0 R] >>".to_string(),
        "<< /Type /Annot /Subtype /Widget /Parent 5 0 R /Rect [100 700 300 720] >>".to_string(),
    ];

    let mut pdf = Vec::new();
    pdf.extend_from_slice(b"%PDF-1.7\n");
    let mut offsets = Vec::with_capacity(objects.len());
    for (i, body) in objects.iter().enumerate() {
        offsets.push(pdf.len() as u64);
        pdf.extend_from_slice(format!("{} 0 obj\n{}\nendobj\n", i + 1, body).as_bytes());
    }
    let xref_pos = pdf.len() as u64;
    let n = objects.len() + 1;
    pdf.extend_from_slice(format!("xref\n0 {n}\n").as_bytes());
    pdf.extend_from_slice(b"0000000000 65535 f \n");
    for off in &offsets {
        pdf.extend_from_slice(format!("{off:010} 00000 n \n").as_bytes());
    }
    pdf.extend_from_slice(
        format!("trailer\n<< /Size {n} /Root 1 0 R >>\nstartxref\n{xref_pos}\n%%EOF\n").as_bytes(),
    );
    pdf
}

#[test]
fn fill_field_with_widget_kids() {
    let base = build_widget_kids_form_pdf();
    // The terminal field is object 5; before fill it has no value.
    assert!(object_v(&base, 5, 0).is_none(), "field starts empty");

    let output = IncrementalFormFiller::new(&base)
        .fill("email", "user@example.com")
        .expect("must resolve a terminal field whose kids are widgets");

    assert_eq!(&output[..base.len()], &base[..], "verbatim prefix");
    assert_eq!(
        object_v(&output, 5, 0).as_deref(),
        Some("user@example.com"),
        "/V must be set on the terminal field object (5), not lost in widget recursion"
    );
    // Field (5) gets /V, AcroForm (4) gets NeedAppearances, the widget (6) is
    // rewritten with /AP, and one new appearance stream (7) is appended.
    let mut ids = appended_xref_object_numbers(&output[base.len()..]);
    ids.sort_unstable();
    assert_eq!(ids, vec![4, 5, 6, 7]);
    // The widget (6) now carries a synthesized /AP/N showing the value.
    let mut reader = PdfReader::new(Cursor::new(&output)).expect("parse");
    let widget = reader
        .get_object(6, 0)
        .expect("widget 6")
        .as_dict()
        .expect("dict")
        .clone();
    assert!(widget.contains_key("AP"), "widget gains /AP");
}
