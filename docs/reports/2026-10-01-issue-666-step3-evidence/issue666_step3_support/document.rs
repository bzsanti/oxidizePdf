//! #666: document strings are distinct from encoded glyph strings.
#[path = "../../tests/common/text_contracts.rs"]
mod contract;

use contract::assembler::{assemble_pdf, stream_obj};
use contract::{extract, font, pdf};
use oxidize_pdf::parser::{ParseOptions, PdfReader};
use sha2::{Digest, Sha256};
use std::io::Cursor;

const TABLE: &str = include_str!("../../tests/fixtures/text_contracts/PDFDocEncoding.tsv");

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02X}")).collect()
}

pub fn metadata_pdf(bytes: &[u8]) -> Vec<u8> {
    let info = format!("<< /Title <{}> /Author <{}> >>", hex(bytes), hex(bytes));
    let mut result = assemble_pdf(&[
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Resources << >> /Contents 5 0 R >>"
            .to_vec(),
        info.into_bytes(),
        stream_obj("", b""),
    ]);
    // Only extend the trailer after xref; every object offset and startxref stays valid.
    let marker = b"/Root 1 0 R >>";
    let pos = result
        .windows(marker.len())
        .position(|w| w == marker)
        .expect("trailer root");
    result.splice(
        pos..pos + marker.len(),
        b"/Root 1 0 R /Info 4 0 R >>".iter().copied(),
    );
    result
}

fn metadata(bytes: &[u8]) -> String {
    let mut reader =
        PdfReader::new_with_options(Cursor::new(metadata_pdf(bytes)), ParseOptions::strict())
            .expect("metadata fixture");
    let result = reader.metadata().expect("metadata extraction");
    assert_eq!(
        result.title, result.author,
        "same document string in independent fields"
    );
    result
        .title
        .expect("Title present, including empty strings")
}

fn actual_text(bytes: &[u8]) -> String {
    let content = format!(
        "BT /F1 12 Tf /Span << /ActualText <{}> >> BDC (x) Tj EMC ET",
        hex(bytes)
    );
    extract(
        pdf(
            &font("Helvetica", "/WinAnsiEncoding", None, ""),
            content.as_bytes(),
            vec![],
        ),
        ParseOptions::strict(),
    )
    .text
}

fn defined_rows() -> impl Iterator<Item = (u8, String)> {
    TABLE
        .lines()
        .filter(|line| !line.starts_with('#'))
        .filter_map(|line| {
            let columns: Vec<_> = line.split('\t').collect();
            if columns[1] == "undefined" {
                return None;
            }
            let code = u8::from_str_radix(columns[0], 16).expect("byte");
            let scalar = u32::from_str_radix(columns[2], 16).expect("scalar");
            Some((
                code,
                char::from_u32(scalar).expect("Unicode scalar").to_string(),
            ))
        })
}

