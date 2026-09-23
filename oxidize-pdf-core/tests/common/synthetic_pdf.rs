//! Handcrafted PDF builder for content-stream-level tests. Produces a minimal
//! valid 1-page PDF with a Type1 Helvetica font as `/F1` and the supplied
//! bytes as the `/Contents` stream. Identical layout to the helper introduced
//! by issue #235; extracted here so Phase 1 tests can reuse it without copy.
//!
//! `dead_code` is suppressed at the module level: cargo compiles this file
//! once per test crate that imports `common::synthetic_pdf`, but other
//! test crates flag the helpers as unused. This is the conventional
//! pattern for `tests/common/` modules in Rust.
#![allow(dead_code)]

/// Write a PDF object body to `bytes`, recording its absolute offset in
/// `offset` for the xref table. Used by `build_pdf_with_content_stream`
/// to lay out objects 1..5 sequentially.
fn write_obj(bytes: &mut Vec<u8>, offset: &mut usize, body: &str) {
    *offset = bytes.len();
    bytes.extend_from_slice(body.as_bytes());
}

/// Build a minimal valid 1-page PDF whose Contents stream is the supplied
/// raw byte sequence (typically a hand-crafted sequence of text operators).
/// Resources expose two independently named Type1 Helvetica fonts, `/F1` and
/// `/F2`, so tests can exercise a font switch without depending on a fixture.
pub fn build_pdf_with_content_stream(content: &[u8]) -> Vec<u8> {
    let mut bytes: Vec<u8> = Vec::with_capacity(1024 + content.len());
    let mut offsets: Vec<usize> = vec![0; 7]; // index by object id (1..=6)

    bytes.extend_from_slice(b"%PDF-1.4\n%\xE2\xE3\xCF\xD3\n");

    write_obj(
        &mut bytes,
        &mut offsets[1],
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n",
    );
    write_obj(
        &mut bytes,
        &mut offsets[2],
        "2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n",
    );
    write_obj(
        &mut bytes,
        &mut offsets[3],
        "3 0 obj\n<< /Type /Page /Parent 2 0 R /Resources << /Font << /F1 4 0 R /F2 5 0 R >> >> /Contents 6 0 R /MediaBox [0 0 612 792] >>\nendobj\n",
    );
    write_obj(
        &mut bytes,
        &mut offsets[4],
        "4 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>\nendobj\n",
    );
    write_obj(
        &mut bytes,
        &mut offsets[5],
        "5 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>\nendobj\n",
    );

    offsets[6] = bytes.len();
    bytes.extend_from_slice(
        format!("6 0 obj\n<< /Length {} >>\nstream\n", content.len()).as_bytes(),
    );
    bytes.extend_from_slice(content);
    bytes.extend_from_slice(b"\nendstream\nendobj\n");

    let xref_off = bytes.len();
    bytes.extend_from_slice(b"xref\n0 7\n0000000000 65535 f \n");
    for off in offsets.iter().skip(1) {
        bytes.extend_from_slice(format!("{:010} 00000 n \n", off).as_bytes());
    }
    bytes.extend_from_slice(
        format!(
            "trailer\n<< /Size 7 /Root 1 0 R >>\nstartxref\n{}\n%%EOF\n",
            xref_off
        )
        .as_bytes(),
    );

    bytes
}

/// Build a minimal page with one Form XObject (`/Fm`) and two Helvetica font
/// resource names. The form and the page share the fonts so text-extraction
/// tests can verify state propagated across a `Do` boundary.
pub fn build_pdf_with_form_xobject(page_content: &[u8], form_content: &[u8]) -> Vec<u8> {
    let mut bytes: Vec<u8> = Vec::with_capacity(1536 + page_content.len() + form_content.len());
    let mut offsets: Vec<usize> = vec![0; 8]; // index by object id (1..=7)

    bytes.extend_from_slice(b"%PDF-1.4\n%\xE2\xE3\xCF\xD3\n");
    write_obj(
        &mut bytes,
        &mut offsets[1],
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n",
    );
    write_obj(
        &mut bytes,
        &mut offsets[2],
        "2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n",
    );
    write_obj(
        &mut bytes,
        &mut offsets[3],
        "3 0 obj\n<< /Type /Page /Parent 2 0 R /Resources << /Font << /F1 4 0 R /F2 5 0 R >> /XObject << /Fm 7 0 R >> >> /Contents 6 0 R /MediaBox [0 0 612 792] >>\nendobj\n",
    );
    write_obj(
        &mut bytes,
        &mut offsets[4],
        "4 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>\nendobj\n",
    );
    write_obj(
        &mut bytes,
        &mut offsets[5],
        "5 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>\nendobj\n",
    );

    offsets[6] = bytes.len();
    bytes.extend_from_slice(
        format!("6 0 obj\n<< /Length {} >>\nstream\n", page_content.len()).as_bytes(),
    );
    bytes.extend_from_slice(page_content);
    bytes.extend_from_slice(b"\nendstream\nendobj\n");

    offsets[7] = bytes.len();
    bytes.extend_from_slice(
        format!(
            "7 0 obj\n<< /Type /XObject /Subtype /Form /BBox [0 0 612 792] /Resources << /Font << /F1 4 0 R /F2 5 0 R >> >> /Length {} >>\nstream\n",
            form_content.len()
        )
        .as_bytes(),
    );
    bytes.extend_from_slice(form_content);
    bytes.extend_from_slice(b"\nendstream\nendobj\n");

    let xref_off = bytes.len();
    bytes.extend_from_slice(b"xref\n0 8\n0000000000 65535 f \n");
    for off in offsets.iter().skip(1) {
        bytes.extend_from_slice(format!("{:010} 00000 n \n", off).as_bytes());
    }
    bytes.extend_from_slice(
        format!(
            "trailer\n<< /Size 8 /Root 1 0 R >>\nstartxref\n{}\n%%EOF\n",
            xref_off
        )
        .as_bytes(),
    );
    bytes
}
