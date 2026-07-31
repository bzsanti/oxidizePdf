//! Issue #459: the consumers that read a PDF string as text must decode it.
//!
//! Preserving the bytes of a literal string (see
//! `issue_459_literal_binary_string_test.rs`) moves the decoding responsibility
//! to whoever reads the string as text. Every consumer that used to receive
//! bytes the lexer had already turned into UTF-8 now receives the file's bytes,
//! and `String::from_utf8_lossy` on PDFDocEncoding text yields `U+FFFD` for
//! every accented character. These are the two places where that is not
//! cosmetic: an AcroForm field is addressed *by name*, so a mangled name cannot
//! be filled at all, and signature metadata is shown to a person.

use oxidize_pdf::signatures::detect_signature_fields;
use oxidize_pdf::writer::IncrementalFormFiller;
use std::io::{Cursor, Write};

/// Escapes a byte string into PDF literal-string syntax.
fn literal(bytes: &[u8]) -> Vec<u8> {
    let mut out = vec![b'('];
    for &b in bytes {
        match b {
            b'(' | b')' | b'\\' => {
                out.push(b'\\');
                out.push(b);
            }
            0x20..=0x7E => out.push(b),
            _ => out.extend_from_slice(format!("\\{:03o}", b).as_bytes()),
        }
    }
    out.push(b')');
    out
}

/// Encodes text as the PDFDocEncoding bytes a producer writes without a BOM.
///
/// Every character used here lives in the Latin-1 range, where PDFDocEncoding
/// and Latin-1 agree, so this is a byte-per-character mapping.
fn pdfdoc(text: &str) -> Vec<u8> {
    text.chars()
        .map(|c| {
            let code = c as u32;
            assert!(code <= 0xFF, "{c:?} is outside the Latin-1 range");
            code as u8
        })
        .collect()
}

/// Assembles a PDF from object bodies, with a correct xref table.
fn build_pdf(objects: &[Vec<u8>], trailer_extra: &str) -> Vec<u8> {
    let mut pdf = Vec::from(&b"%PDF-1.7\n"[..]);
    let mut offsets = Vec::with_capacity(objects.len());

    for (i, body) in objects.iter().enumerate() {
        offsets.push(pdf.len());
        writeln!(pdf, "{} 0 obj", i + 1).unwrap();
        pdf.extend_from_slice(body);
        pdf.extend_from_slice(b"\nendobj\n");
    }

    let xref_offset = pdf.len();
    writeln!(pdf, "xref\n0 {}\n0000000000 65535 f ", objects.len() + 1).unwrap();
    for offset in &offsets {
        writeln!(pdf, "{:010} 00000 n ", offset).unwrap();
    }
    write!(
        pdf,
        "trailer\n<< /Size {} /Root 1 0 R {} >>\nstartxref\n{}\n%%EOF\n",
        objects.len() + 1,
        trailer_extra,
        xref_offset
    )
    .unwrap();
    pdf
}

#[test]
fn an_acroform_field_named_with_accents_is_still_addressable_by_that_name() {
    let field_name = "Año de ejercicio";

    let mut field = Vec::from(
        &b"<< /Type /Annot /Subtype /Widget /FT /Tx /Ff 0 /Rect [100 100 300 130] /P 3 0 R /T "[..],
    );
    field.extend_from_slice(&literal(&pdfdoc(field_name)));
    field.extend_from_slice(b" >>");

    // An incremental fill requires /AcroForm to be an indirect reference, so it
    // can be rewritten without rewriting the catalog.
    let objects = vec![
        Vec::from(&b"<< /Type /Catalog /Pages 2 0 R /AcroForm 5 0 R >>"[..]),
        Vec::from(&b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>"[..]),
        Vec::from(&b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Annots [4 0 R] >>"[..]),
        field,
        Vec::from(&b"<< /Fields [4 0 R] >>"[..]),
    ];
    let pdf = build_pdf(&objects, "");

    let filled = IncrementalFormFiller::new(&pdf)
        .fill(field_name, "2026")
        .expect("a field is addressed by the name the file gives it");

    assert!(
        filled.len() > pdf.len(),
        "filling appends an incremental update"
    );
    let appended = String::from_utf8_lossy(&filled[pdf.len()..]).into_owned();
    assert!(
        appended.contains("2026"),
        "the appended update must carry the new value, got: {appended}"
    );

    // The mangled form is what a lossy conversion produces: it must not resolve.
    assert!(
        IncrementalFormFiller::new(&pdf)
            .fill("A\u{FFFD}o de ejercicio", "2026")
            .is_err(),
        "a name that is not in the document must not match a field"
    );
}

#[test]
fn signature_metadata_written_in_pdfdocencoding_reaches_the_caller_as_text() {
    let name = "Firma del Interventor";
    let reason = "Revisión anual de cuentas";
    let location = "Bilbao, Bizkaia";
    let contact = "interventoría@example.org";

    let mut sig_dict = Vec::from(
        &b"<< /Type /Sig /Filter /Adobe.PPKLite /SubFilter /adbe.pkcs7.detached /ByteRange [0 100 200 300] /Contents <deadbeef> /Reason "[..],
    );
    sig_dict.extend_from_slice(&literal(&pdfdoc(reason)));
    sig_dict.extend_from_slice(b" /Location ");
    sig_dict.extend_from_slice(&literal(&pdfdoc(location)));
    sig_dict.extend_from_slice(b" /ContactInfo ");
    sig_dict.extend_from_slice(&literal(&pdfdoc(contact)));
    sig_dict.extend_from_slice(b" >>");

    let mut field = Vec::from(&b"<< /FT /Sig /V 5 0 R /T "[..]);
    field.extend_from_slice(&literal(&pdfdoc(name)));
    field.extend_from_slice(b" >>");

    let objects = vec![
        Vec::from(&b"<< /Type /Catalog /Pages 2 0 R /AcroForm << /Fields [4 0 R] >> >>"[..]),
        Vec::from(&b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>"[..]),
        Vec::from(&b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] >>"[..]),
        field,
        sig_dict,
    ];
    let pdf = build_pdf(&objects, "");

    let mut reader =
        oxidize_pdf::parser::PdfReader::new(Cursor::new(pdf.as_slice())).expect("open");
    let signatures = detect_signature_fields(&mut reader).expect("detect signatures");

    assert_eq!(signatures.len(), 1, "the document holds one signed field");
    let signature = &signatures[0];
    assert_eq!(signature.name.as_deref(), Some(name));
    assert_eq!(signature.reason.as_deref(), Some(reason));
    assert_eq!(signature.location.as_deref(), Some(location));
    assert_eq!(signature.contact_info.as_deref(), Some(contact));
}
