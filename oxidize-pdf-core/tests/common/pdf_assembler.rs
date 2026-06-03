//! Generic hand-rolled PDF assembler for tests that need precise control over
//! the object graph (catalog entries, resource dictionaries, stream filters)
//! rather than the shapes produced by `Document` + `PdfWriter`.
//!
//! Object `N` is `objects[N-1]`; the assembler prepends `"N 0 obj\n"` and
//! appends `"\nendobj\n"`, then emits a correct classic xref table, trailer and
//! `startxref`. The byte offsets are computed from the actual emitted bytes, so
//! the result parses through `PdfReader` without relying on xref recovery.
//!
//! `dead_code` is suppressed at the module level: cargo compiles this file once
//! per test crate that imports it, and crates using only a subset of helpers
//! flag the rest as unused. This is the conventional `tests/common/` pattern.
#![allow(dead_code)]

/// Assemble a PDF (header `%PDF-1.4`) from full object bodies.
///
/// `objects[i]` is the body of object `i + 1` — everything between
/// `"N 0 obj\n"` and `"\nendobj\n"`. `/Root` is fixed at `1 0 R`, so object 1
/// must be the catalog.
pub fn assemble_pdf(objects: &[Vec<u8>]) -> Vec<u8> {
    assemble_pdf_with_version("1.4", objects)
}

/// Same as [`assemble_pdf`] but with an explicit PDF header version string
/// (e.g. `"1.4"`, `"1.7"`). The version drives `PdfReader::version()`, which the
/// PDF/A version check reads.
pub fn assemble_pdf_with_version(version: &str, objects: &[Vec<u8>]) -> Vec<u8> {
    let n = objects.len();
    let mut bytes: Vec<u8> = Vec::with_capacity(1024);
    bytes.extend_from_slice(format!("%PDF-{}\n", version).as_bytes());
    bytes.extend_from_slice(b"%\xE2\xE3\xCF\xD3\n");

    let mut offsets = vec![0usize; n + 1];
    for (idx, body) in objects.iter().enumerate() {
        let id = idx + 1;
        offsets[id] = bytes.len();
        bytes.extend_from_slice(format!("{} 0 obj\n", id).as_bytes());
        bytes.extend_from_slice(body);
        bytes.extend_from_slice(b"\nendobj\n");
    }

    let xref_off = bytes.len();
    bytes.extend_from_slice(format!("xref\n0 {}\n0000000000 65535 f \n", n + 1).as_bytes());
    for off in offsets.iter().skip(1) {
        bytes.extend_from_slice(format!("{:010} 00000 n \n", off).as_bytes());
    }
    bytes.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{}\n%%EOF\n",
            n + 1,
            xref_off
        )
        .as_bytes(),
    );
    bytes
}

/// Build a stream object body: `<< {dict} /Length L >>\nstream\n{data}\nendstream`.
/// `dict` is the inner dictionary content without the enclosing `<< >>`.
pub fn stream_obj(dict: &str, data: &[u8]) -> Vec<u8> {
    let mut v = Vec::new();
    v.extend_from_slice(format!("<< {} /Length {} >>\nstream\n", dict, data.len()).as_bytes());
    v.extend_from_slice(data);
    v.extend_from_slice(b"\nendstream");
    v
}
