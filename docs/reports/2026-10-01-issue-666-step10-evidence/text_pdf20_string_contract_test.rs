//! #666: ISO 32000-2 7.9.2.2; UTF-8 BOM applies to PDF 2.0 text strings.
//! Reference: https://pdfa.org/understanding-utf-8-in-pdf-2-0/
#[path = "common/text_contracts.rs"]
mod contract;
use contract::assembler::{assemble_pdf_with_version, stream_obj};
use contract::{cmap, extract, font, pdf};
use oxidize_pdf::parser::{ParseOptions, PdfReader};
use std::io::Cursor;
const CASES: &[(&str, &str)] = &[
    ("EFBBBF", ""),
    ("EFBBBF41", "A"),
    ("EFBBBFC3A9", "é"),
    ("EFBBBFF09F9880", "😀"),
    ("EFBBBF65CC81", "e\u{0301}"),
    ("EFBBBFEFBBBF41", "\u{feff}A"),
];
fn metadata_pdf(version: &str, hex: &str) -> Vec<u8> {
    let mut bytes = assemble_pdf_with_version(version, &[
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Resources << >> /Contents 5 0 R >>".to_vec(),
        format!("<< /Title <{hex}> >>").into_bytes(), stream_obj("", b""),
    ]);
    let marker = b"/Root 1 0 R >>";
    let index = bytes
        .windows(marker.len())
        .position(|w| w == marker)
        .unwrap();
    bytes.splice(
        index..index + marker.len(),
        b"/Root 1 0 R /Info 4 0 R >>".iter().copied(),
    );
    bytes
}
fn metadata(version: &str, hex: &str) -> String {
    PdfReader::new_with_options(
        Cursor::new(metadata_pdf(version, hex)),
        ParseOptions::strict(),
    )
    .unwrap()
    .metadata()
    .unwrap()
    .title
    .unwrap()
}
fn actualtext(hex: &str) -> String {
    let content = format!("BT /F1 12 Tf /Span << /ActualText <{hex}> >> BDC (A) Tj EMC ET");
    let mut bytes = pdf(
        &font("Helvetica", "/WinAnsiEncoding", None, ""),
        content.as_bytes(),
        vec![],
    );
    // Same-length header replacement preserves every xref offset.
    bytes[..8].copy_from_slice(b"%PDF-2.0");
    extract(bytes, ParseOptions::strict()).text
}
#[test]
fn pdf20_metadata_utf8_bom_decodes_exact_sequences() {
    let failures: Vec<_> = CASES
        .iter()
        .filter_map(|(hex, expected)| {
            let actual = metadata("2.0", hex);
            (actual != *expected)
                .then(|| format!("{hex}: expected {expected:?}, actual {actual:?}"))
        })
        .collect();
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
#[test]
fn pdf20_actualtext_utf8_bom_decodes_exact_sequences() {
    let failures: Vec<_> = CASES
        .iter()
        .filter_map(|(hex, expected)| {
            let actual = actualtext(hex);
            (actual != *expected)
                .then(|| format!("{hex}: expected {expected:?}, actual {actual:?}"))
        })
        .collect();
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
#[test]
fn pdf20_utf8_without_bom_remains_pdfdocencoding() {
    assert_eq!(metadata("2.0", "C3A9"), "Ã©");
    assert_eq!(actualtext("C3A9"), "Ã©");
}
#[test]
fn pdf17_does_not_silently_apply_pdf20_utf8_semantics() {
    assert_eq!(metadata("1.7", "EFBBBF41"), "ï»¿A");
}
#[test]
fn utf8_bom_in_glyph_string_is_decoded_by_font_not_document_encoding() {
    let mut bytes = pdf(
        &font(
            "Helvetica",
            "/WinAnsiEncoding",
            Some(500.0),
            "/ToUnicode 6 0 R",
        ),
        b"BT /F1 12 Tf <EFBBBFC3A9> Tj ET",
        vec![cmap(
            "<EF> <0058>\n<BB> <0059>\n<BF> <005A>\n<C3> <0051>\n<A9> <0052>",
            5,
            "<00> <FF>",
        )],
    );
    bytes[..8].copy_from_slice(b"%PDF-2.0");
    assert_eq!(extract(bytes, ParseOptions::strict()).text, "XYZQR");
}
