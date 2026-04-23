//! Regression tests for the v2.5.6 post-release code-quality audit:
//!
//!   * **SEC-F1** — PDF string literals (`Object::String`) must escape
//!     `\`, `(`, `)` before writing (ISO 32000-1 §7.3.4.2). Prior to
//!     this fix the writer emitted raw bytes inside `(...)` so a
//!     caller-supplied value containing `)` could close the string
//!     early and inject arbitrary dict keys into the enclosing object
//!     (reachable from `Document::fill_field` and from every
//!     `/Info`-metadata setter).
//!
//!   * **SEC-F5** — `Page::add_color_space` / `add_pattern` /
//!     `add_shading` / `add_form_xobject` must validate the supplied
//!     resource name against ISO 32000-1 §7.3.5 (no whitespace, no
//!     delimiter characters `( ) < > [ ] { } / %`, no `#` escape
//!     introducer). A caller-controlled name containing delimiters
//!     produces a /Name token that prematurely closes the resource
//!     dict, opening a dict-level injection parallel to F1.
//!
//!   * **QUAL-9** — Resource-dictionary entries (`/Font`, `/XObject`,
//!     `/ColorSpace`, `/Pattern`, `/Shading`, `/ExtGState`) must be
//!     emitted in deterministic order so two logically-identical
//!     documents serialise to byte-identical PDFs. Previously the
//!     writer iterated `HashMap`s whose iteration order is randomised
//!     per-instance, making reproducible builds and PDF diffs
//!     impossible.
//!
//! All assertions are wire-format: parse the emitted PDF and inspect
//! resolved objects. No smoke tests.

use oxidize_pdf::forms::{FormManager, TextField, Widget, WidgetAppearance};
use oxidize_pdf::geometry::{Point, Rectangle};
use oxidize_pdf::graphics::{
    DeviceColorSpace, FormXObject, PageColorSpace, PaintType, TilingPattern, TilingType,
};
use oxidize_pdf::parser::PdfReader;
use oxidize_pdf::{Document, Page};
use std::io::Cursor;

fn build_document_with_filled_field(value: &str) -> Vec<u8> {
    let mut doc = Document::new();
    let mut page = Page::a4();
    let mut fm = FormManager::new();

    let rect = Rectangle::new(Point::new(100.0, 700.0), Point::new(300.0, 720.0));
    let widget = Widget::new(rect).with_appearance(WidgetAppearance::default());
    let field = TextField::new("email");
    let field_ref = fm
        .add_text_field(field, widget.clone(), None)
        .expect("add_text_field");

    page.add_form_widget_with_ref(widget, field_ref)
        .expect("add_form_widget_with_ref");
    doc.add_page(page);
    doc.set_form_manager(fm);

    doc.fill_field("email", value).expect("fill_field");
    doc.to_bytes().expect("serialize")
}

/// SEC-F1 primary assertion. A value containing the delimiters that
/// close a PDF string literal (`)`), plus a backslash (`\`), MUST
/// round-trip through write→parse as the exact same bytes — no dict
/// injection, no byte mangling.
///
/// The attack payload mimics a real injection: `foo) /Evil (true` in
/// the raw byte stream would close the `(foo)` string, inject an
/// `/Evil true` dict key, and leave `/true` dangling. After the fix,
/// the payload is written as `(foo\) /Evil \(true)` and the parser
/// returns the original bytes verbatim.
#[test]
fn object_string_escapes_delimiters_in_dict_injection_payload() {
    let payload = "foo) /Evil (true";
    let bytes = build_document_with_filled_field(payload);

    // Parse the PDF and resolve /AcroForm/Fields[0]/V.
    let mut reader = PdfReader::new(Cursor::new(&bytes)).expect("parse");
    let catalog = reader.catalog().expect("catalog").clone();
    let (acro_n, acro_g) = catalog
        .get("AcroForm")
        .and_then(|o| o.as_reference())
        .expect("/AcroForm indirect");
    let acro = reader
        .get_object(acro_n, acro_g)
        .expect("resolve /AcroForm")
        .clone();
    let (field_n, field_g) = acro
        .as_dict()
        .and_then(|d| d.get("Fields"))
        .and_then(|o| o.as_array())
        .and_then(|a| a.get(0))
        .and_then(|o| o.as_reference())
        .expect("/AcroForm/Fields[0]");
    let field_obj = reader
        .get_object(field_n, field_g)
        .expect("resolve field")
        .clone();
    let field_dict = field_obj.as_dict().expect("field dict").clone();

    // Assertion 1: /V must resolve to the EXACT original bytes.
    // If the writer didn't escape, the parser would see either
    //   - a string "foo" followed by a rogue /Evil key (breaking the
    //     dict), or
    //   - a truncated string "foo" with trailing garbage.
    // Either way, /V would NOT equal the original payload.
    let v = field_dict
        .get("V")
        .and_then(|o| o.as_string())
        .and_then(|s| s.as_str().ok())
        .expect("/V must be a parseable PDF string");
    assert_eq!(
        v, payload,
        "fill_field value must round-trip byte-identically; got {:?}",
        v
    );

    // Assertion 2: /Evil must NOT appear as a key in the field dict.
    // (Belt-and-braces: if escaping failed silently in a way that
    // still parses as a string, this wouldn't fire, but it's cheap
    // and catches the obvious injection.)
    assert!(
        field_dict.get("Evil").is_none(),
        "injected /Evil key must not appear in the field dict"
    );
}

/// SEC-F1 extended: a `\`-containing payload must survive round-trip.
/// The PDF escape grammar treats `\` as the escape introducer, so a
/// naive double-escape (only escaping `(` and `)`) would flip `\`
/// to garbage.
#[test]
fn object_string_escapes_backslash() {
    let payload = r"C:\Users\admin\secrets.txt";
    let bytes = build_document_with_filled_field(payload);
    let mut reader = PdfReader::new(Cursor::new(&bytes)).expect("parse");
    let catalog = reader.catalog().expect("catalog").clone();
    let (acro_n, acro_g) = catalog
        .get("AcroForm")
        .and_then(|o| o.as_reference())
        .expect("/AcroForm");
    let acro = reader.get_object(acro_n, acro_g).expect("acro").clone();
    let (field_n, field_g) = acro
        .as_dict()
        .and_then(|d| d.get("Fields"))
        .and_then(|o| o.as_array())
        .and_then(|a| a.get(0))
        .and_then(|o| o.as_reference())
        .expect("fields[0]");
    let field = reader.get_object(field_n, field_g).expect("field").clone();
    let v = field
        .as_dict()
        .and_then(|d| d.get("V"))
        .and_then(|o| o.as_string())
        .and_then(|s| s.as_str().ok())
        .expect("/V");
    assert_eq!(v, payload, "backslash payload must round-trip");
}

/// SEC-F5: delimiter-containing resource name must be rejected at the
/// public `add_*` boundary (returns `Err`), not silently accepted and
/// written as a malformed /Name token that closes the resource dict.
#[test]
fn add_color_space_rejects_name_with_delimiters() {
    let mut page = Page::a4();
    // `>>` inside a Name would close the resource dict.
    let result = page.add_color_space(
        "Foo>>/Evil",
        PageColorSpace::DeviceAlias(DeviceColorSpace::Rgb),
    );
    assert!(
        result.is_err(),
        "add_color_space must reject names containing delimiters, got Ok"
    );
}

#[test]
fn add_pattern_rejects_name_with_whitespace() {
    let mut page = Page::a4();
    let pattern = TilingPattern::new(
        "P1".to_string(),
        PaintType::Colored,
        TilingType::ConstantSpacing,
        [0.0, 0.0, 10.0, 10.0],
        10.0,
        10.0,
    );
    let result = page.add_pattern("Pat tern", pattern);
    assert!(
        result.is_err(),
        "add_pattern must reject names containing whitespace"
    );
}

#[test]
fn add_shading_rejects_empty_name() {
    use oxidize_pdf::graphics::{
        AxialShading, Color, ColorStop, Point as ShadingPoint, ShadingDefinition,
    };
    let mut page = Page::a4();
    let axial = AxialShading::new(
        "Sh1".to_string(),
        ShadingPoint::new(0.0, 0.0),
        ShadingPoint::new(10.0, 0.0),
        vec![ColorStop::new(0.0, Color::Rgb(1.0, 0.0, 0.0))],
    );
    let result = page.add_shading("", ShadingDefinition::Axial(axial));
    assert!(
        result.is_err(),
        "add_shading must reject empty names (§7.3.5 requires at least one regular character)"
    );
}

#[test]
fn add_form_xobject_rejects_name_with_hash_escape_introducer() {
    let mut page = Page::a4();
    let bbox = Rectangle::new(Point::new(0.0, 0.0), Point::new(10.0, 10.0));
    // `#` is the /Name hex-escape introducer (§7.3.5). A raw `#` in a
    // Name without two following hex digits is illegal.
    let result = page.add_form_xobject("F#XX", FormXObject::new(bbox));
    assert!(
        result.is_err(),
        "add_form_xobject must reject names containing the `#` escape introducer"
    );
}

/// SEC-F5 positive: valid, spec-compliant names keep working.
#[test]
fn add_color_space_accepts_valid_names() {
    let mut page = Page::a4();
    page.add_color_space("CS1", PageColorSpace::DeviceAlias(DeviceColorSpace::Rgb))
        .expect("CS1 is a valid Name");
    page.add_color_space(
        "MyCustomSpace",
        PageColorSpace::DeviceAlias(DeviceColorSpace::Cmyk),
    )
    .expect("MyCustomSpace is a valid Name");
    page.add_color_space(
        "CS_with_underscores-and-dashes",
        PageColorSpace::DeviceAlias(DeviceColorSpace::Gray),
    )
    .expect("underscores and dashes are valid in Names per §7.3.5");
}

/// QUAL-9: resource-dictionary entries MUST be emitted in a
/// deterministic (sorted) order so the PDF is reproducible and
/// diffable. We can't rely on the parser preserving dict key order
/// (the internal `PdfDictionary` uses a `HashMap`), so we inspect the
/// raw serialised bytes for the order of `/CSX` tokens within the
/// `/ColorSpace << ... >>` block.
///
/// Strategy: register 5 colour spaces in a deliberately unsorted
/// insertion order (CSE, CSA, CSC, CSB, CSD). In the emitted bytes
/// the keys of `/ColorSpace` must appear in ASCII-lexicographic order
/// — that's the invariant sorted emission guarantees, and it does NOT
/// depend on `HashMap` iteration happening to match insertion order.
#[test]
fn color_space_resource_entries_are_emitted_in_sorted_order() {
    let mut doc = Document::new();
    let mut page = Page::a4();
    let insertion_order = ["CSE", "CSA", "CSC", "CSB", "CSD"];
    for name in &insertion_order {
        page.add_color_space(*name, PageColorSpace::DeviceAlias(DeviceColorSpace::Rgb))
            .expect("add_color_space");
    }
    doc.add_page(page);
    let bytes = doc.to_bytes().expect("serialize");

    // Work in raw bytes throughout — the PDF is ASCII for resource
    // names and delimiters, but can contain arbitrary bytes elsewhere
    // (object streams, content streams). Using String::from_utf8_lossy
    // rewrites non-UTF8 bytes as U+FFFD (3 bytes), desynchronising
    // char-offsets from byte-offsets downstream.
    fn find_bytes(haystack: &[u8], needle: &[u8]) -> Option<usize> {
        haystack.windows(needle.len()).position(|w| w == needle)
    }

    let cs_marker = b"/ColorSpace";
    let cs_start = find_bytes(&bytes, cs_marker).expect("/ColorSpace in bytes");
    // Skip the marker, then find the `<<` that opens the ColorSpace
    // sub-dict.
    let after_marker = cs_start + cs_marker.len();
    let open_rel = find_bytes(&bytes[after_marker..], b"<<").expect("<< after /ColorSpace");
    let absolute_open = after_marker + open_rel;

    // Find the matching `>>` by walking with a depth counter starting
    // at 1 (we're already inside the opened dict). Note: the outer
    // /CalRGB parameter dicts would add nesting, but in this test we
    // only register Name-form entries (`/DeviceRGB`) so there are no
    // nested dicts — the first `>>` is the close. We still use the
    // balanced walker for robustness.
    let mut depth: usize = 1;
    let mut i = absolute_open + 2;
    let mut close: Option<usize> = None;
    while i + 1 < bytes.len() {
        match &bytes[i..i + 2] {
            b"<<" => {
                depth += 1;
                i += 2;
            }
            b">>" => {
                depth -= 1;
                if depth == 0 {
                    close = Some(i);
                    break;
                }
                i += 2;
            }
            _ => i += 1,
        }
    }
    let close = close.expect("matching >> for /ColorSpace <<");
    let cs_block = &bytes[absolute_open..close];

    // Scan the block for each /CSX token; the positions must be in
    // lexicographic order of the names (what sorted emission guarantees).
    let mut observed: Vec<(usize, &str)> = Vec::new();
    for name in &insertion_order {
        let needle: Vec<u8> = [b"/", name.as_bytes()].concat();
        let pos = find_bytes(cs_block, &needle)
            .unwrap_or_else(|| panic!("/{} missing from /ColorSpace block", name));
        observed.push((pos, *name));
    }
    observed.sort_by_key(|(pos, _)| *pos);
    let observed_names: Vec<&str> = observed.iter().map(|(_, n)| *n).collect();
    let mut expected = observed_names.clone();
    expected.sort();
    assert_eq!(
        observed_names, expected,
        "/ColorSpace entries must be emitted in ASCII-sorted order \
         regardless of insertion order; got {:?}",
        observed_names
    );
}
