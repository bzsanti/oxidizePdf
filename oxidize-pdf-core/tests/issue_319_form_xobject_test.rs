//! Issue #319 (real root cause for the good-energy invoice): text drawn
//! inside a Form XObject (invoked with `Do`) was never extracted. The
//! producer (RML2PDF/pluscode) embeds the page body as a Form XObject
//! ("inclPDF"); only the page's direct content (a footer) was extracted,
//! so the "charges in detail" body went missing.
//!
//! These tests build PDFs whose text lives inside a Form XObject (flat,
//! nested, and translated via /Matrix) and assert it is extracted. Content
//! is asserted exactly — no smoke tests.

use oxidize_pdf::parser::{PdfDocument, PdfReader};
use oxidize_pdf::text::{ExtractionOptions, TextExtractor};
use std::io::Cursor;

/// Build a single-page PDF from raw object bodies (object 1 = catalog).
/// `streams` maps an object index (1-based) to raw stream bytes that get a
/// correct `/Length` and `stream`/`endstream` wrapper appended to the body.
fn build_pdf(bodies: &[(&str, Option<&[u8]>)]) -> Vec<u8> {
    let mut pdf = Vec::new();
    pdf.extend_from_slice(b"%PDF-1.7\n");
    let mut offsets = Vec::with_capacity(bodies.len());
    for (i, (dict_body, stream)) in bodies.iter().enumerate() {
        offsets.push(pdf.len() as u64);
        pdf.extend_from_slice(format!("{} 0 obj\n", i + 1).as_bytes());
        match stream {
            Some(data) => {
                pdf.extend_from_slice(
                    format!("<< {} /Length {} >>\nstream\n", dict_body, data.len()).as_bytes(),
                );
                pdf.extend_from_slice(data);
                pdf.extend_from_slice(b"\nendstream");
            }
            None => pdf.extend_from_slice(format!("<< {dict_body} >>").as_bytes()),
        }
        pdf.extend_from_slice(b"\nendobj\n");
    }
    let xref_pos = pdf.len() as u64;
    let n = bodies.len() + 1;
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

fn extract(pdf: &[u8]) -> String {
    let reader = PdfReader::new(Cursor::new(pdf)).expect("parse");
    let document = PdfDocument::new(reader);
    let mut extractor = TextExtractor::with_options(ExtractionOptions::default());
    extractor
        .extract_from_page(&document, 0)
        .expect("extract page 0")
        .text
}

#[test]
fn text_inside_form_xobject_is_extracted() {
    // Page content: paint a Form XObject, then draw a direct footer.
    let page_content = b"q /Fx Do Q BT /F1 12 Tf 72 60 Td (Direct footer) Tj ET";
    // XObject content: the "body" text.
    let xobj_content = b"BT /F1 12 Tf 72 500 Td (Inside the form xobject) Tj ET";

    let pdf = build_pdf(&[
        // 1 catalog
        ("/Type /Catalog /Pages 2 0 R", None),
        // 2 pages
        ("/Type /Pages /Kids [3 0 R] /Count 1", None),
        // 3 page
        (
            "/Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] \
             /Resources << /Font << /F1 5 0 R >> /XObject << /Fx 6 0 R >> >> /Contents 4 0 R",
            None,
        ),
        // 4 page content
        ("", Some(page_content)),
        // 5 font
        (
            "/Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding",
            None,
        ),
        // 6 Form XObject
        (
            "/Type /XObject /Subtype /Form /BBox [0 0 612 792] \
             /Resources << /Font << /F1 5 0 R >> >>",
            Some(xobj_content),
        ),
    ]);

    let text = extract(&pdf);
    assert!(
        text.contains("Direct footer"),
        "direct page content must extract: {text:?}"
    );
    assert!(
        text.contains("Inside the form xobject"),
        "text inside the Form XObject must be extracted: {text:?}"
    );
}

#[test]
fn nested_form_xobjects_are_extracted() {
    // Page -> Fx (outer) -> Fy (inner). Inner text must surface.
    let page_content = b"/Fx Do";
    let outer = b"BT /F1 12 Tf 72 600 Td (Outer body) Tj ET /Fy Do";
    let inner = b"BT /F1 12 Tf 72 400 Td (Inner nested body) Tj ET";

    let pdf = build_pdf(&[
        ("/Type /Catalog /Pages 2 0 R", None),
        ("/Type /Pages /Kids [3 0 R] /Count 1", None),
        (
            "/Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] \
             /Resources << /Font << /F1 5 0 R >> /XObject << /Fx 6 0 R >> >> /Contents 4 0 R",
            None,
        ),
        ("", Some(page_content)),
        (
            "/Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding",
            None,
        ),
        // 6 outer XObject references inner /Fy
        (
            "/Type /XObject /Subtype /Form /BBox [0 0 612 792] \
             /Resources << /Font << /F1 5 0 R >> /XObject << /Fy 7 0 R >> >>",
            Some(outer),
        ),
        // 7 inner XObject
        (
            "/Type /XObject /Subtype /Form /BBox [0 0 612 792] \
             /Resources << /Font << /F1 5 0 R >> >>",
            Some(inner),
        ),
    ]);

    let text = extract(&pdf);
    assert!(text.contains("Outer body"), "outer XObject text: {text:?}");
    assert!(
        text.contains("Inner nested body"),
        "nested XObject text must be extracted: {text:?}"
    );
}

#[test]
fn form_xobject_matrix_offsets_position_without_losing_text() {
    // The XObject is painted with a /Matrix translation. The text must still
    // be extracted (position composes through the matrix; we assert content).
    let page_content = b"q 1 0 0 1 100 200 cm /Fx Do Q";
    let xobj_content = b"BT /F1 12 Tf 0 0 Td (Matrixed body) Tj ET";

    let pdf = build_pdf(&[
        ("/Type /Catalog /Pages 2 0 R", None),
        ("/Type /Pages /Kids [3 0 R] /Count 1", None),
        (
            "/Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] \
             /Resources << /Font << /F1 5 0 R >> /XObject << /Fx 6 0 R >> >> /Contents 4 0 R",
            None,
        ),
        ("", Some(page_content)),
        (
            "/Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding",
            None,
        ),
        (
            "/Type /XObject /Subtype /Form /BBox [0 0 612 792] /Matrix [1 0 0 1 0 0] \
             /Resources << /Font << /F1 5 0 R >> >>",
            Some(xobj_content),
        ),
    ]);

    let text = extract(&pdf);
    assert!(
        text.contains("Matrixed body"),
        "text in a matrixed XObject must be extracted: {text:?}"
    );
}
