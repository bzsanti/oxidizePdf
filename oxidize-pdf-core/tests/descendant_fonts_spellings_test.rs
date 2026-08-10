//! `/DescendantFonts` may legally be written in four combinations: the value
//! is a direct array or an indirect reference to one, and the CIDFont element
//! inside is a direct dictionary or an indirect reference. ISO 32000-1
//! Table 121 types the entry as `array` with no reference requirement, §7.3.6
//! lets array elements be "dictionaries, or any other objects", and §7.3.7
//! lets a dictionary value "be any kind of object" — where the spec wants an
//! indirect reference it says so (Table 117 marks the CIDFont's
//! FontDescriptor "shall be an indirect reference"), and the DescendantFonts
//! row carries no such words. The same argument as #463 for inline `/Font`
//! resource entries. Producers use the inline form: ReportLab's
//! `UnicodeCIDFont` writes the CIDFont as a direct dictionary.
//!
//! The extractor read only the reference-inside-direct-array combination. For
//! the other three, `descendant_font` stayed empty, which silently skipped the
//! `cid_encoding` branch in `decode_text_with_font` and fell back to byte-wise
//! decoding.
//!
//! The oracle is falsifiable by construction: the page shows 第1章 概要
//! GRIMWALD through `/Encoding /UniJIS-UCS2-H` (the character codes ARE the
//! UTF-16BE values) with no `/ToUnicode`, so a decode that reaches the
//! descendant yields the Japanese text, while the byte-wise fallback renders
//! the UTF-16BE code units one byte at a time (`第` = U+7B2C comes out as
//! `{,`). The two answers cannot be confused.

mod common;

use common::pdf_assembler::assemble_pdf_with_version;
use oxidize_pdf::parser::{PdfDocument, PdfReader};
use oxidize_pdf::text::{ExtractionOptions, TextExtractor};
use std::io::Cursor;

const EXPECTED: &str = "第1章 概要 GRIMWALD";

/// ISO 32000-1 Table 117: the CIDFont's FontDescriptor is
/// "(Required; shall be an indirect reference)" — object 7 here.
const FONT_DESCRIPTOR: &str = "<< /Type /FontDescriptor /FontName /HeiseiKakuGo-W5 /Flags 4 \
     /FontBBox [ -92 -250 1010 922 ] /ItalicAngle 0 /Ascent 752 /Descent -221 \
     /CapHeight 737 /StemV 114 >>";

const CID_FONT: &str = "<< /Type /Font /Subtype /CIDFontType0 /BaseFont /HeiseiKakuGo-W5 \
     /CIDSystemInfo << /Registry (Adobe) /Ordering (Japan1) /Supplement 2 >> \
     /FontDescriptor 7 0 R /DW 1000 >>";

#[derive(Clone, Copy)]
enum Spelling {
    /// `/DescendantFonts [ 6 0 R ]` — the only combination read before the fix
    IndirectFont,
    /// `/DescendantFonts [ << ...CIDFont... >> ]` — what ReportLab emits
    DirectDict,
    /// `/DescendantFonts 8 0 R` where `8 0 obj` is `[ 6 0 R ]`
    IndirectArray,
    /// `/DescendantFonts 8 0 R` where `8 0 obj` is `[ << ...CIDFont... >> ]`
    IndirectArrayDirectDict,
}

/// One page drawing `EXPECTED` in a Type0 font. Objects 1..=7 are identical
/// across spellings (object 6 is written whether or not anything references
/// it); only object 5's `/DescendantFonts` value differs, so the three
/// documents differ in exactly that respect.
fn build(spelling: Spelling) -> Vec<u8> {
    let hex: String = EXPECTED
        .encode_utf16()
        .flat_map(|unit| unit.to_be_bytes())
        .map(|byte| format!("{byte:02X}"))
        .collect();
    let content = format!("BT /F2 18 Tf 60 700 Td <{hex}> Tj ET");

    let descendant_fonts = match spelling {
        Spelling::DirectDict => format!("[ {CID_FONT} ]"),
        Spelling::IndirectFont => "[ 6 0 R ]".to_string(),
        Spelling::IndirectArray | Spelling::IndirectArrayDirectDict => "8 0 R".to_string(),
    };

    let mut objects: Vec<Vec<u8>> = vec![
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [ 3 0 R ] /Count 1 >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /MediaBox [ 0 0 595 842 ] \
           /Resources << /Font << /F2 5 0 R >> >> /Contents 4 0 R >>"
            .to_vec(),
        format!(
            "<< /Length {} >>\nstream\n{content}\nendstream",
            content.len()
        )
        .into_bytes(),
        format!(
            "<< /Type /Font /Subtype /Type0 /BaseFont /HeiseiKakuGo-W5 \
             /Encoding /UniJIS-UCS2-H /DescendantFonts {descendant_fonts} >>"
        )
        .into_bytes(),
        CID_FONT.as_bytes().to_vec(),
        FONT_DESCRIPTOR.as_bytes().to_vec(),
    ];
    match spelling {
        Spelling::IndirectArray => objects.push(b"[ 6 0 R ]".to_vec()),
        Spelling::IndirectArrayDirectDict => objects.push(format!("[ {CID_FONT} ]").into_bytes()),
        _ => {}
    }

    assemble_pdf_with_version("1.7", &objects)
}

fn extract_page_text(bytes: Vec<u8>) -> String {
    let document = PdfDocument::new(PdfReader::new(Cursor::new(bytes)).expect("open PDF"));
    let mut extractor = TextExtractor::with_options(ExtractionOptions::default());
    extractor
        .extract_from_page(&document, 0)
        .expect("extract page 0")
        .text
}

/// The spelling that already worked — pins the existing behavior so the fix
/// cannot regress it.
#[test]
fn descendant_written_as_an_indirect_reference_decodes() {
    assert_eq!(extract_page_text(build(Spelling::IndirectFont)), EXPECTED);
}

#[test]
fn descendant_written_as_a_direct_dictionary_decodes() {
    assert_eq!(extract_page_text(build(Spelling::DirectDict)), EXPECTED);
}

#[test]
fn descendant_array_written_as_an_indirect_value_decodes() {
    assert_eq!(extract_page_text(build(Spelling::IndirectArray)), EXPECTED);
}

#[test]
fn indirect_array_holding_a_direct_dictionary_decodes() {
    assert_eq!(
        extract_page_text(build(Spelling::IndirectArrayDirectDict)),
        EXPECTED
    );
}
