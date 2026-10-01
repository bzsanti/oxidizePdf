//! #666: document strings are distinct from encoded glyph strings.
#[path = "common/text_contracts.rs"]
mod contract;

use contract::assembler::{assemble_pdf, stream_obj};
use contract::{extract, font, pdf};
use oxidize_pdf::parser::{ParseOptions, PdfReader};
use sha2::{Digest, Sha256};
use std::io::Cursor;

const TABLE: &str = include_str!("fixtures/text_contracts/PDFDocEncoding.tsv");

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02X}")).collect()
}

fn metadata_pdf(bytes: &[u8]) -> Vec<u8> {
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

#[test]
fn document_oracle_preserves_defined_and_undefined_positions() {
    let rows: Vec<_> = TABLE
        .lines()
        .filter(|line| !line.starts_with('#'))
        .collect();
    assert_eq!(rows.len(), 256);
    for (index, row) in rows.iter().enumerate() {
        let columns: Vec<_> = row.split('\t').collect();
        assert_eq!(columns.len(), 3, "row {index}");
        assert_eq!(usize::from_str_radix(columns[0], 16).unwrap(), index);
        assert!(matches!(columns[1], "defined" | "undefined"));
        assert_eq!(columns[1] == "undefined", columns[2] == "-");
    }
    assert_eq!(defined_rows().count(), 232);
    let provenance: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/text_contracts/provenance.json")).unwrap();
    assert_eq!(
        provenance["table_sha256"]["PDFDocEncoding.tsv"].as_str(),
        Some(format!("{:x}", Sha256::digest(TABLE.as_bytes())).as_str())
    );
}

fn check_document_table(decode: fn(&[u8]) -> String, route: &str) {
    let mut failures = Vec::new();
    for (code, expected) in defined_rows() {
        let actual = decode(&[code]);
        if actual != expected {
            failures.push(format!(
                "{route} 0x{code:02X}: expected {expected:?}, actual {actual:?}"
            ));
        }
    }
    eprintln!(
        "{route}: 232 defined positions, {} mismatches",
        failures.len()
    );
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn metadata_matches_pdfdocencoding_normative_table() {
    check_document_table(metadata, "metadata");
}
#[test]
fn actualtext_matches_pdfdocencoding_normative_table() {
    check_document_table(actual_text, "ActualText");
}

const UTF16_CASES: &[(&[u8], &str)] = &[
    (&[0xFE, 0xFF], ""),
    (&[0xFE, 0xFF, 0x00, 0x41], "A"),
    (&[0xFE, 0xFF, 0xD8, 0x3D, 0xDE, 0x00], "😀"),
    (&[0xFE, 0xFF, 0x00, 0x65, 0x03, 0x01], "e\u{0301}"),
    (&[0xFE, 0xFF, 0xFE, 0xFF, 0x00, 0x41], "\u{FEFF}A"),
    (&[0xFE, 0xFF, 0x00, 0x66, 0x00, 0x69], "fi"),
];

#[test]
fn metadata_utf16be_bom_sequences_are_exact() {
    for (bytes, expected) in UTF16_CASES {
        assert_eq!(metadata(bytes), *expected, "{bytes:02X?}");
    }
}
#[test]
fn actualtext_utf16be_bom_sequences_are_exact() {
    for (bytes, expected) in UTF16_CASES {
        assert_eq!(actual_text(bytes), *expected, "{bytes:02X?}");
    }
}
#[test]
fn empty_document_strings_remain_empty() {
    assert_eq!(metadata(b""), "");
    assert_eq!(actual_text(b""), "");
}

fn navigation_form_pdf(bytes: &[u8]) -> Vec<u8> {
    assemble_pdf(&[
        b"<< /Type /Catalog /Pages 2 0 R /Outlines 4 0 R /AcroForm 6 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Resources << >> >>".to_vec(),
        b"<< /Type /Outlines /First 5 0 R /Last 5 0 R /Count 1 >>".to_vec(),
        format!("<< /Title <{}> /Parent 4 0 R /Dest [3 0 R /Fit] >>",hex(bytes)).into_bytes(),
        b"<< /Fields [7 0 R] /DA (/Helv 12 Tf 0 g) /DR << /Font << /Helv << /Type /Font /Subtype /Type1 /BaseFont /Helvetica >> >> >> >>".to_vec(),
        format!("<< /FT /Tx /T (contract) /V <{}> >>",hex(bytes)).into_bytes(),
    ])
}
fn bookmark(bytes: &[u8]) -> String {
    let doc = PdfReader::new_with_options(
        Cursor::new(navigation_form_pdf(bytes)),
        ParseOptions::strict(),
    )
    .expect("outline PDF")
    .into_document();
    let outline = doc.outline().expect("read outlines").expect("outline tree");
    assert_eq!(outline.items.len(), 1);
    outline.items[0].title.clone()
}
fn form_value(bytes: &[u8]) -> String {
    // Follow the actual catalog/AcroForm/Fields route using public object APIs.
    // This tests stored field values, not filling or rendering an appearance.
    let mut reader = PdfReader::new_with_options(
        Cursor::new(navigation_form_pdf(bytes)),
        ParseOptions::strict(),
    )
    .expect("form PDF");
    let (id, gen) = reader
        .catalog()
        .unwrap()
        .get("AcroForm")
        .unwrap()
        .as_reference()
        .unwrap();
    let form = reader.get_object(id, gen).unwrap();
    let (id, gen) = form
        .as_dict()
        .unwrap()
        .get("Fields")
        .unwrap()
        .as_array()
        .unwrap()
        .0[0]
        .as_reference()
        .unwrap();
    reader
        .get_object(id, gen)
        .unwrap()
        .as_dict()
        .unwrap()
        .get("V")
        .unwrap()
        .as_string()
        .unwrap()
        .to_text()
}
#[test]
fn bookmark_titles_match_pdfdocencoding_normative_table() {
    check_document_table(bookmark, "bookmark");
}
#[test]
fn stored_form_values_match_pdfdocencoding_normative_table() {
    check_document_table(form_value, "form V");
}
#[test]
fn bookmarks_and_form_values_preserve_utf16be_sequences() {
    for (bytes, expected) in UTF16_CASES {
        assert_eq!(bookmark(bytes), *expected, "bookmark {bytes:02X?}");
        assert_eq!(form_value(bytes), *expected, "form {bytes:02X?}");
    }
}
#[test]
fn empty_bookmarks_and_form_values_remain_empty() {
    assert_eq!(bookmark(b""), "");
    assert_eq!(form_value(b""), "");
}

// #667 recovery policy for the infallible text-string API: preserve valid text
// and emit U+FFFD for undefined bytes or incomplete UTF-16 units. ParseOptions
// controls PDF syntax; it does not make PdfString::to_text a validating API.
#[test]
fn undefined_pdfdoc_bytes_are_visible_replacements_without_losing_suffixes() {
    use oxidize_pdf::parser::objects::PdfString;
    let mut failures = Vec::new();
    for line in TABLE.lines().filter(|l| !l.starts_with('#')) {
        let row: Vec<_> = line.split('\t').collect();
        if row[1] != "undefined" {
            continue;
        }
        let code = u8::from_str_radix(row[0], 16).unwrap();
        let actual = PdfString::new(vec![b'A', code, b'B']).to_text();
        if actual != "A\u{fffd}B" {
            failures.push(format!("{code:02X}: {actual:?}"));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
#[test]
fn malformed_utf16_preserves_valid_units_and_exposes_incomplete_tail() {
    use oxidize_pdf::parser::objects::PdfString;
    let cases: &[(&[u8], &str)] = &[
        (&[0xfe, 0xff, 0x00], "\u{fffd}"),
        (&[0xfe, 0xff, 0x00, 0x41, 0x00], "A\u{fffd}"),
        (&[0xfe, 0xff, 0xd8, 0x00, 0x00, 0x42], "\u{fffd}B"),
        (&[0xfe, 0xff, 0xdc, 0x00, 0x00, 0x42], "\u{fffd}B"),
        (&[0xfe, 0xff, 0xd8, 0x00, 0x00], "\u{fffd}\u{fffd}"),
    ];
    let mut failures = Vec::new();
    for &(bytes, expected) in cases {
        let actual = PdfString::new(bytes.to_vec()).to_text();
        if actual != expected {
            failures.push(format!(
                "{bytes:02X?}: expected {expected:?}, actual {actual:?}"
            ));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
