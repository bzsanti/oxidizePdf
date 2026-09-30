//! Issue #662 — decode_macroman in extraction_cmap.rs is incomplete for bytes 0xA0..=0xFF.
//!
//! When a PDF font specifies `/Encoding /MacRomanEncoding` without a `/ToUnicode` map,
//! `decode_macroman` previously only mapped bytes 0x80..=0x9F, falling back to
//! `byte as char` for 0xA0..=0xFF. This erroneously interpreted MacRoman bytes as
//! ISO-8859-1 (Latin-1).
//!
//! For example:
//! - Byte 0xBC (masculine ordinal `º`, U+00BA) was extracted as `¼` (U+00BC).
//! - Byte 0xBB (feminine ordinal `ª`, U+00AA) was extracted as `»` (U+00BB).
//! - Byte 0xE7 (capital A with acute `Á`, U+00C1) was extracted as `ç` (U+00E7).
//! - Byte 0xA0 (dagger `†`, U+2020) was extracted as non-breaking space (U+00A0).
//! - Byte 0xDB (euro sign `€`, U+20AC) was extracted as `Û` (U+00DB).

use oxidize_pdf::parser::{ParseOptions, PdfReader};
use oxidize_pdf::text::TextExtractor;
use std::io::Cursor;

fn write_obj(bytes: &mut Vec<u8>, offset: &mut usize, body: &str) {
    *offset = bytes.len();
    bytes.extend_from_slice(body.as_bytes());
}

fn build_pdf_with_macroman_font(content: &[u8]) -> Vec<u8> {
    let mut bytes: Vec<u8> = Vec::with_capacity(1024 + content.len());
    let mut offsets: Vec<usize> = vec![0; 6];

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
        "3 0 obj\n<< /Type /Page /Parent 2 0 R /Resources << /Font << /F1 4 0 R >> >> /Contents 5 0 R /MediaBox [0 0 612 792] >>\nendobj\n",
    );
    write_obj(
        &mut bytes,
        &mut offsets[4],
        "4 0 obj\n<< /Type /Font /Subtype /TrueType /BaseFont /Helvetica /Encoding /MacRomanEncoding >>\nendobj\n",
    );

    offsets[5] = bytes.len();
    bytes.extend_from_slice(
        format!("5 0 obj\n<< /Length {} >>\nstream\n", content.len()).as_bytes(),
    );
    bytes.extend_from_slice(content);
    bytes.extend_from_slice(b"\nendstream\nendobj\n");

    let xref_off = bytes.len();
    bytes.extend_from_slice(b"xref\n0 6\n0000000000 65535 f \n");
    for off in offsets.iter().skip(1) {
        bytes.extend_from_slice(format!("{:010} 00000 n \n", off).as_bytes());
    }
    bytes.extend_from_slice(
        format!(
            "trailer\n<< /Size 6 /Root 1 0 R >>\nstartxref\n{}\n%%EOF\n",
            xref_off
        )
        .as_bytes(),
    );

    bytes
}

fn extract_text_from_content(content: &[u8]) -> String {
    let pdf_bytes = build_pdf_with_macroman_font(content);
    let reader =
        PdfReader::new_with_options(Cursor::new(&pdf_bytes), ParseOptions::lenient()).unwrap();
    let doc = reader.into_document();
    let mut extractor = TextExtractor::new();
    extractor.extract_from_page(&doc, 0).unwrap().text
}

#[test]
fn test_issue_662_ordinal_indicators_extracted_correctly() {
    // In MacRoman:
    // 0xBC = 'º' (U+00BA)
    // 0xBB = 'ª' (U+00AA)
    let mut content = Vec::new();
    content.extend_from_slice(b"BT\n/F1 12 Tf\n100 700 Td\n(N");
    content.push(0xBC);
    content.extend_from_slice(b" 45 e 2");
    content.push(0xBB);
    content.extend_from_slice(b" edicao) Tj\nET\n");

    let text = extract_text_from_content(&content);
    assert_eq!(text.trim(), "Nº 45 e 2ª edicao");
}

#[test]
fn test_issue_662_accented_characters_in_macroman_upper_range() {
    // In MacRoman:
    // 0xE7 = 'Á' (U+00C1)
    // 0xE5 = 'Â' (U+00C2)
    // 0xE6 = 'Ê' (U+00CA)
    // 0xEA = 'Í' (U+00CD)
    // 0xEE = 'Ó' (U+00D3)
    // 0xF2 = 'Ú' (U+00DA)
    let mut content = Vec::new();
    content.extend_from_slice(b"BT\n/F1 12 Tf\n100 700 Td\n(");
    content.push(0xE7);
    content.extend_from_slice(b"RIO ");
    content.push(0xE5);
    content.push(0xE6);
    content.push(0xEA);
    content.push(0xEE);
    content.push(0xF2);
    content.extend_from_slice(b") Tj\nET\n");

    let text = extract_text_from_content(&content);
    assert_eq!(text.trim(), "ÁRIO ÂÊÍÓÚ");
}

#[test]
fn test_issue_662_special_symbols_in_macroman_upper_range() {
    // In MacRoman:
    // 0xA0 = '†' (U+2020, dagger)
    // 0xDB = '€' (U+20AC, euro)
    // 0xD2 = '“' (U+201C, left double quote)
    // 0xD3 = '”' (U+201D, right double quote)
    // 0xD0 = '–' (U+2013, en dash)
    // 0xD1 = '—' (U+2014, em dash)
    let mut content = Vec::new();
    content.extend_from_slice(b"BT\n/F1 12 Tf\n100 700 Td\n(");
    content.push(0xDB);
    content.extend_from_slice(b"100 ");
    content.push(0xD2);
    content.extend_from_slice(b"quote");
    content.push(0xD3);
    content.extend_from_slice(b" A");
    content.push(0xD0);
    content.extend_from_slice(b"B");
    content.push(0xA0);
    content.extend_from_slice(b") Tj\nET\n");

    let text = extract_text_from_content(&content);
    assert_eq!(text.trim(), "€100 “quote” A–B†");
}
