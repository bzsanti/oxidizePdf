//! Regression test for PR #362 (contributor: Hatell).
//!
//! Bug: when a page's `/Resources` has the `/Font` entry as an *indirect
//! reference* (`/Font 1 0 R`) instead of an inline dictionary,
//! `Page::from_parsed_with_content` skipped its font-resolution block
//! entirely (the block matched only `Object::Dictionary`, never
//! `Object::Reference`). The `/Font` entry was therefore carried over as a
//! dangling reference into the rebuilt page, so the embedded fonts were lost
//! when the page was written into a new document (text rendered with the
//! wrong font or invisible).
//!
//! This test builds a minimal PDF whose page resources use `/Font 1 0 R`,
//! converts the page via `from_parsed_with_content`, and asserts that the
//! preserved resources expose `/Font` as a *resolved dictionary* whose entry
//! `/F1` is the actual font dictionary — not a dangling reference.
//!
//! RED (without the fix): `/Font` stays an `Object::Reference` and the
//! assertion fails. GREEN (with the fix): `/Font` is resolved to a dictionary.

use oxidize_pdf::parser::{PdfDocument, PdfReader};
use oxidize_pdf::pdf_objects::Object;
use oxidize_pdf::Page;
use std::io::Cursor;

/// Build a minimal, valid PDF where the page's `/Font` resource is an indirect
/// reference to object 1 (a font sub-dictionary), which in turn references the
/// font object 5. Exercises the contributor's exact case: `/Font 1 0 R`.
fn build_pdf_with_indirect_font_resource() -> Vec<u8> {
    let content = "BT /F1 12 Tf 72 720 Td (Hello) Tj ET";
    let obj6 = format!(
        "<< /Length {} >>\nstream\n{}\nendstream",
        content.len(),
        content
    );

    // Object bodies, 1-indexed by position.
    let objects: Vec<String> = vec![
        // 1: Font sub-dictionary, referenced indirectly by the page's /Font.
        "<< /F1 5 0 R >>".to_string(),
        // 2: Catalog.
        "<< /Type /Catalog /Pages 3 0 R >>".to_string(),
        // 3: Page tree root.
        "<< /Type /Pages /Kids [4 0 R] /Count 1 >>".to_string(),
        // 4: Page — note `/Font 1 0 R` (the whole Font entry is a reference).
        "<< /Type /Page /Parent 3 0 R /MediaBox [0 0 612 792] \
         /Resources << /Font 1 0 R >> /Contents 6 0 R >>"
            .to_string(),
        // 5: The actual font dictionary.
        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_string(),
        // 6: Content stream.
        obj6,
    ];

    let mut pdf = Vec::new();
    pdf.extend_from_slice(b"%PDF-1.7\n");

    let mut offsets = Vec::with_capacity(objects.len());
    for (i, body) in objects.iter().enumerate() {
        offsets.push(pdf.len());
        pdf.extend_from_slice(format!("{} 0 obj\n{}\nendobj\n", i + 1, body).as_bytes());
    }

    let xref_offset = pdf.len();
    let size = objects.len() + 1;
    pdf.extend_from_slice(format!("xref\n0 {}\n", size).as_bytes());
    pdf.extend_from_slice(b"0000000000 65535 f \n");
    for off in &offsets {
        pdf.extend_from_slice(format!("{:010} 00000 n \n", off).as_bytes());
    }
    pdf.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 2 0 R >>\nstartxref\n{}\n%%EOF",
            size, xref_offset
        )
        .as_bytes(),
    );

    pdf
}

#[test]
fn indirect_font_resource_is_resolved_to_dictionary() {
    let pdf_bytes = build_pdf_with_indirect_font_resource();

    let reader =
        PdfReader::new(Cursor::new(&pdf_bytes)).expect("hand-built PDF must be re-parseable");
    let document = PdfDocument::new(reader);
    let parsed_page = document.get_page(0).expect("page 0 must parse");

    let page = Page::from_parsed_with_content(&parsed_page, &document)
        .expect("from_parsed_with_content must succeed");

    let resources = page
        .get_preserved_resources()
        .expect("page must preserve its resources");

    let font_entry = resources
        .get("Font")
        .expect("preserved resources must keep a /Font entry");

    // The bug: /Font stayed an indirect reference, so font embedding was
    // skipped. The fix resolves it to the underlying font sub-dictionary.
    let fonts = match font_entry {
        Object::Dictionary(d) => d,
        Object::Reference(id) => panic!(
            "/Font was left as a dangling reference {:?} — indirect /Font resource not resolved",
            id
        ),
        other => panic!("/Font has unexpected type: {:?}", other),
    };

    let f1 = fonts
        .get("F1")
        .expect("resolved /Font dictionary must contain /F1");

    let font_dict = match f1 {
        Object::Dictionary(d) => d,
        other => panic!("/F1 was not resolved to a font dictionary: {:?}", other),
    };

    let base_font = font_dict
        .get("BaseFont")
        .and_then(|o| match o {
            Object::Name(n) => Some(n.as_str()),
            _ => None,
        })
        .expect("/F1 font dictionary must carry a /BaseFont name");

    assert_eq!(
        base_font, "Helvetica",
        "resolved font dictionary must be the real Helvetica font"
    );
}
