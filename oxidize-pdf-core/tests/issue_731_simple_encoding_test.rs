#[path = "common/text_contracts.rs"]
mod contract;
use oxidize_pdf::fonts::ResolvedFontResource;
use oxidize_pdf::parser::{ParseOptions, PdfReader};
use std::io::Cursor;

fn fixture(
    name: &str,
    encoding: &str,
    bytes: &[u8],
    extra: &str,
    objects: Vec<Vec<u8>>,
) -> Vec<u8> {
    let encoding = if encoding.is_empty() {
        String::new()
    } else {
        format!("/Encoding {encoding}")
    };
    let font = format!("<< /Type /Font /Subtype /Type1 /BaseFont /{name} {encoding} {extra} >>");
    let hex: String = bytes.iter().map(|b| format!("{b:02X}")).collect();
    contract::pdf(
        &font,
        format!("BT /F1 12 Tf 72 700 Td <{hex}> Tj ET").as_bytes(),
        objects,
    )
}
fn check(pdf: Vec<u8>, codes: &[u8], expected: &str) {
    for options in [ParseOptions::strict(), ParseOptions::lenient()] {
        let doc = PdfReader::new_with_options(Cursor::new(pdf.clone()), options.clone())
            .unwrap()
            .into_document();
        let text: String = ResolvedFontResource::from_page(&doc, 0, "F1")
            .unwrap()
            .decode_glyphs(codes)
            .unwrap()
            .into_iter()
            .map(|g| g.unicode.unwrap_or_else(|| "\u{fffd}".into()))
            .collect();
        assert_eq!(text, expected, "resolved font");
        assert_eq!(
            contract::extract(pdf.clone(), options).text,
            expected,
            "text extractor"
        );
    }
}
#[test]
fn implicit_standard14_encoding_in_both_consumers() {
    for name in contract::LATIN_STANDARD14 {
        check(
            fixture(name, "", &[0x27, 0x60, 0xe1], "", vec![]),
            &[0x27, 0x60, 0xe1],
            "’‘Æ",
        );
    }
    check(
        fixture("ABCDEF+Helvetica", "", &[0xe1], "", vec![]),
        &[0xe1],
        "Æ",
    );
}
#[test]
fn explicit_standard_encoding_in_both_consumers() {
    check(
        fixture(
            "Helvetica",
            "/StandardEncoding",
            &[0x27, 0x60, 0xe1],
            "",
            vec![],
        ),
        &[0x27, 0x60, 0xe1],
        "’‘Æ",
    );
}
#[test]
fn supported_named_encodings_agree() {
    for (encoding, code, expected) in [
        ("/MacExpertEncoding", 0x21, "\u{f721}"),
        ("/ZapfDingbatsEncoding", 0x6e, "■"),
        ("/SymbolEncoding", 0x61, "α"),
        ("/WinAnsiEncoding", 0x80, "€"),
        ("/MacRomanEncoding", 0xdb, "¤"),
    ] {
        check(
            fixture("Helvetica", encoding, &[code], "", vec![]),
            &[code],
            expected,
        );
    }
}
#[test]
fn differences_and_tounicode_keep_precedence() {
    let encoding = "<< /BaseEncoding /StandardEncoding /Differences [225 /Euro] >>";
    check(
        fixture("Helvetica", encoding, &[0xe1], "", vec![]),
        &[0xe1],
        "€",
    );
    let map = contract::cmap("<E1> <00660069>", 1, "<00> <FF>");
    check(
        fixture(
            "Helvetica",
            encoding,
            &[0xe1],
            "/ToUnicode 6 0 R",
            vec![map],
        ),
        &[0xe1],
        "fi",
    );
    check(
        fixture(
            "Helvetica",
            "<< /Differences [225 /Euro] >>",
            &[0x27, 0xe1],
            "",
            vec![],
        ),
        &[0x27, 0xe1],
        "’€",
    );
}
#[test]
fn symbolic_truetype_is_not_assigned_standard_encoding() {
    let bytes = contract::pdf("<< /Type /Font /Subtype /TrueType /BaseFont /Helvetica /FontDescriptor << /Type /FontDescriptor /Flags 4 >> >>", b"BT /F1 12 Tf <2760E1> Tj ET", vec![]);
    check(bytes, &[0x27, 0x60, 0xe1], "���");
}
#[test]
#[ignore = "requires Poppler; explicitly run by interoperability CI"]
fn independent_poppler_standard_encoding_control() {
    use std::io::Write;
    use std::process::{Command, Stdio};
    for encoding in ["", "/StandardEncoding"] {
        let pdf = fixture("Helvetica", encoding, &[0x27, 0x60, 0xe1], "", vec![]);
        let mut child = Command::new("pdftotext")
            .args(["-enc", "UTF-8", "-", "-"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();
        child.stdin.take().unwrap().write_all(&pdf).unwrap();
        let output = child.wait_with_output().unwrap();
        assert!(output.status.success());
        assert_eq!(String::from_utf8(output.stdout).unwrap().trim(), "’‘Æ");
        check(pdf, &[0x27, 0x60, 0xe1], "’‘Æ");
    }
}

#[test]
fn embedded_program_is_not_replaced_by_standard14_name() {
    let program = contract::assembler::stream_obj(
        "/Subtype /Type1C",
        include_bytes!("fixtures/text_contracts/type1/ContractIntrinsicCFF.cff"),
    );
    let descriptor = "/FontDescriptor << /Type /FontDescriptor /FontName /Helvetica /Flags 32 /FontBBox [0 0 1000 1000] /ItalicAngle 0 /Ascent 800 /Descent -200 /CapHeight 700 /StemV 80 /FontFile3 6 0 R >>";
    check(
        fixture("Helvetica", "", b"AB", descriptor, vec![program]),
        b"AB",
        "BA",
    );
}
#[test]
fn builtin_symbol_fonts_and_explicit_override() {
    check(fixture("Symbol", "", &[0x61], "", vec![]), &[0x61], "α");
    check(
        fixture("ZapfDingbats", "", &[0x6e], "", vec![]),
        &[0x6e],
        "■",
    );
    check(
        fixture("Symbol", "/WinAnsiEncoding", &[0x80], "", vec![]),
        &[0x80],
        "€",
    );
}
#[test]
fn symbolic_flag_and_invalid_subset_prefix_do_not_infer_latin_encoding() {
    for (name, descriptor) in [
        (
            "Helvetica",
            "/FontDescriptor << /Type /FontDescriptor /Flags 4 >>",
        ),
        ("A+Helvetica", ""),
    ] {
        let doc = PdfReader::new_with_options(
            Cursor::new(fixture(name, "", &[0xe1], descriptor, vec![])),
            ParseOptions::strict(),
        )
        .unwrap()
        .into_document();
        assert_eq!(
            ResolvedFontResource::from_page(&doc, 0, "F1")
                .unwrap()
                .encoding,
            None
        );
    }
}
