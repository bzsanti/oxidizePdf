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
    metadata_pdf_with_catalog_version(version, hex, None)
}
fn metadata_pdf_with_catalog_version(
    version: &str,
    hex: &str,
    catalog_version: Option<&str>,
) -> Vec<u8> {
    let mut bytes = assemble_pdf_with_version(version, &[
        format!("<< /Type /Catalog /Pages 2 0 R {} >>", catalog_version.map(|v| format!("/Version /{v}")).unwrap_or_default()).into_bytes(),
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

#[test]
fn catalog_version_upgrade_enables_utf8_and_downgrade_does_not_disable_it() {
    for (header, catalog) in [("1.7", "2.0"), ("2.0", "1.7")] {
        let bytes = metadata_pdf_with_catalog_version(header, "EFBBBFC3A9", Some(catalog));
        let mut reader =
            PdfReader::new_with_options(Cursor::new(bytes), ParseOptions::strict()).unwrap();
        assert_eq!(
            reader.metadata().unwrap().title.as_deref(),
            Some("é"),
            "header={header} catalog={catalog}"
        );
    }
}
#[test]
fn malformed_pdf20_utf8_replaces_invalid_subparts_and_preserves_suffix() {
    for (hex, expected) in [
        ("EFBBBFE241", "\u{fffd}A"),
        ("EFBBBF41F09F", "A\u{fffd}"),
        ("EFBBBFC0AF42", "\u{fffd}\u{fffd}B"),
    ] {
        assert_eq!(metadata("2.0", hex), expected);
        assert_eq!(actualtext(hex), expected);
    }
}

fn navigation_pdf() -> Vec<u8> {
    assemble_pdf_with_version("2.0", &[
        b"<< /Type /Catalog /Pages 2 0 R /Outlines 4 0 R /AcroForm 6 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Resources << >> >>".to_vec(),
        b"<< /Type /Outlines /First 5 0 R /Last 5 0 R /Count 1 >>".to_vec(),
        b"<< /Title <EFBBBFF09F9880> /Parent 4 0 R /Dest [3 0 R /Fit] >>".to_vec(),
        b"<< /Fields [7 0 R] /DA (/Helv 12 Tf 0 g) /DR << /Font << /Helv << /Type /Font /Subtype /Type1 /BaseFont /Helvetica >> >> >> >>".to_vec(),
        b"<< /FT /Tx /T (contract) /V <EFBBBFF09F9880> >>".to_vec(),
    ])
}
#[test]
fn pdf20_outline_title_uses_effective_document_version() {
    let doc = PdfReader::new_with_options(Cursor::new(navigation_pdf()), ParseOptions::strict())
        .unwrap()
        .into_document();
    assert_eq!(doc.outline().unwrap().unwrap().items[0].title, "😀");
}
#[test]
fn stored_pdf20_form_value_can_be_decoded_with_effective_version() {
    let mut reader =
        PdfReader::new_with_options(Cursor::new(navigation_pdf()), ParseOptions::strict()).unwrap();
    let version = reader.effective_version().unwrap();
    let (id, generation) = reader
        .catalog()
        .unwrap()
        .get("AcroForm")
        .unwrap()
        .as_reference()
        .unwrap();
    let form = reader.get_object(id, generation).unwrap();
    let (id, generation) = form
        .as_dict()
        .unwrap()
        .get("Fields")
        .unwrap()
        .as_array()
        .unwrap()
        .0[0]
        .as_reference()
        .unwrap();
    let field = reader.get_object(id, generation).unwrap();
    let value = field
        .as_dict()
        .unwrap()
        .get("V")
        .unwrap()
        .as_string()
        .unwrap();
    assert_eq!(
        value.as_bytes(),
        &[0xef, 0xbb, 0xbf, 0xf0, 0x9f, 0x98, 0x80]
    );
    assert_eq!(value.to_text_with_version(&version), "😀");
}

#[test]
fn upgraded_catalog_and_named_actualtext_properties_share_version_context() {
    for properties in [
        "/Span << /ActualText <EFBBBFF09F9880> >> BDC",
        "/Span /P1 BDC",
    ] {
        let bytes=assemble_pdf_with_version("1.7", &[
            b"<< /Type /Catalog /Pages 2 0 R /Version /2.0 >>".to_vec(),
            b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
            b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Resources << /Font << /F1 4 0 R >> /Properties << /P1 << /ActualText <EFBBBFF09F9880> >> >> >> /Contents 5 0 R >>".to_vec(),
            font("Helvetica", "/WinAnsiEncoding", None, "").into_bytes(),
            stream_obj("", format!("BT /F1 12 Tf {properties} (A) Tj EMC ET").as_bytes()),
        ]);
        assert_eq!(extract(bytes, ParseOptions::strict()).text, "😀");
    }
}

#[test]
fn malformed_catalog_version_names_do_not_enable_utf8() {
    for catalog in ["+2.0", "2.x", "2.0.0", "02.0"] {
        let bytes = metadata_pdf_with_catalog_version("1.7", "EFBBBF41", Some(catalog));
        let mut reader =
            PdfReader::new_with_options(Cursor::new(bytes), ParseOptions::strict()).unwrap();
        assert_eq!(
            reader.metadata().unwrap().title.as_deref(),
            Some("ï»¿A"),
            "{catalog}"
        );
    }
}

fn structured_actualtext_pdf(in_form: bool, inline_override: bool) -> Vec<u8> {
    let properties = if inline_override {
        "/MCID 0 /ActualText <EFBBBF5A>"
    } else {
        "/MCID 0"
    };
    let marked = format!("BT /F1 12 Tf 100 700 Td /Span << {properties} >> BDC (A) Tj EMC ET");
    assemble_pdf_with_version("1.7", &[
        b"<< /Type /Catalog /Version /2.0 /Pages 2 0 R /MarkInfo << /Marked true >> /StructTreeRoot 6 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /StructParents 0 /Resources << /Font << /F1 4 0 R >> /XObject << /Fm 10 0 R >> >> /Contents 5 0 R >>".to_vec(),
        font("Helvetica", "/WinAnsiEncoding", None, "").into_bytes(),
        stream_obj("", if in_form { b"/Fm Do" } else { marked.as_bytes() }),
        format!("<< /Type /StructTreeRoot /K [{} 0 R] /ParentTree 7 0 R /ParentTreeNextKey 2 >>", if in_form {9} else {8}).into_bytes(),
        format!("<< /Nums [{} [{} 0 R]] >>", if in_form {1} else {0}, if in_form {9} else {8}).into_bytes(),
        b"<< /Type /StructElem /S /Span /P 6 0 R /Pg 3 0 R /K 0 /ActualText <EFBBBFC3A9> >>".to_vec(),
        b"<< /Type /StructElem /S /Span /P 6 0 R /Pg 3 0 R /K << /Type /MCR /Pg 3 0 R /Stm 10 0 R /MCID 0 >> /ActualText <EFBBBFF09F9880> >>".to_vec(),
        stream_obj("/Type /XObject /Subtype /Form /BBox [0 0 612 792] /StructParents 1 /Resources << /Font << /F1 4 0 R >> >>", marked.as_bytes()),
    ])
}
#[test]
fn structure_actualtext_on_page_uses_catalog_version() {
    assert_eq!(
        extract(
            structured_actualtext_pdf(false, false),
            ParseOptions::strict()
        )
        .text,
        "é"
    );
}
#[test]
fn structure_actualtext_in_form_uses_its_own_parent_tree_key() {
    assert_eq!(
        extract(
            structured_actualtext_pdf(true, false),
            ParseOptions::strict()
        )
        .text,
        "😀"
    );
}
#[test]
fn inline_utf8_actualtext_overrides_structure_in_page_and_form() {
    for in_form in [false, true] {
        assert_eq!(
            extract(
                structured_actualtext_pdf(in_form, true),
                ParseOptions::strict()
            )
            .text,
            "Z"
        );
    }
}

#[test]
fn indirect_catalog_version_is_resolved() {
    let mut bytes=assemble_pdf_with_version("1.7", &[
        b"<< /Type /Catalog /Pages 2 0 R /Version 6 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Resources << >> /Contents 5 0 R >>".to_vec(),
        b"<< /Title <EFBBBFC3A9> >>".to_vec(),stream_obj("",b""),b"/2.0".to_vec(),
    ]);
    let marker = b"/Root 1 0 R >>";
    let i = bytes
        .windows(marker.len())
        .position(|w| w == marker)
        .unwrap();
    bytes.splice(
        i..i + marker.len(),
        b"/Root 1 0 R /Info 4 0 R >>".iter().copied(),
    );
    let mut reader =
        PdfReader::new_with_options(Cursor::new(bytes), ParseOptions::strict()).unwrap();
    assert_eq!(reader.metadata().unwrap().title.as_deref(), Some("é"));
}
#[test]
fn utf8_named_field_remains_addressable_to_form_filler() {
    let bytes=assemble_pdf_with_version("2.0", &[
        b"<< /Type /Catalog /Pages 2 0 R /AcroForm 5 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Annots [4 0 R] >>".to_vec(),
        b"<< /Type /Annot /Subtype /Widget /FT /Tx /Ff 0 /Rect [100 100 300 130] /P 3 0 R /T <EFBBBFC3A9> >>".to_vec(),
        b"<< /Fields [4 0 R] /DA (/Helv 12 Tf 0 g) /DR << /Font << /Helv << /Type /Font /Subtype /Type1 /BaseFont /Helvetica >> >> >> >>".to_vec(),
    ]);
    let filled = oxidize_pdf::writer::IncrementalFormFiller::new(&bytes).fill("é", "2026");
    let filled = filled.expect("UTF-8 field name resolves");
    assert_eq!(&filled[..bytes.len()], bytes.as_slice());
    let mut reader = PdfReader::new(Cursor::new(filled)).unwrap();
    let field = reader.get_object(4, 0).unwrap();
    assert_eq!(
        field
            .as_dict()
            .unwrap()
            .get("V")
            .unwrap()
            .as_string()
            .unwrap()
            .to_text(),
        "2026"
    );
    assert!(oxidize_pdf::writer::IncrementalFormFiller::new(&bytes)
        .fill("wrong", "2026")
        .is_err());
}
#[test]
fn signature_metadata_uses_pdf20_text_strings() {
    let bytes=assemble_pdf_with_version("2.0", &[
        b"<< /Type /Catalog /Pages 2 0 R /AcroForm << /Fields [4 0 R] >> >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] >>".to_vec(),
        b"<< /FT /Sig /V 5 0 R /T <EFBBBFC3A9> >>".to_vec(),
        b"<< /Type /Sig /Filter /Adobe.PPKLite /SubFilter /adbe.pkcs7.detached /ByteRange [0 100 200 300] /Contents <deadbeef> /Reason <EFBBBFC3A9> /Location <EFBBBFC3A9> /ContactInfo <EFBBBFC3A9> >>".to_vec(),
    ]);
    let mut reader = PdfReader::new(Cursor::new(bytes)).unwrap();
    let fields = oxidize_pdf::signatures::detect_signature_fields(&mut reader).unwrap();
    assert_eq!(fields.len(), 1);
    assert_eq!(fields[0].name.as_deref(), Some("é"));
    assert_eq!(fields[0].reason.as_deref(), Some("é"));
    assert_eq!(fields[0].location.as_deref(), Some("é"));
    assert_eq!(fields[0].contact_info.as_deref(), Some("é"));
}

fn version_chain_pdf(length: usize, cycle: bool) -> Vec<u8> {
    let mut objects = vec![
        b"<< /Type /Catalog /Pages 2 0 R /Version 4 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Resources << >> >>".to_vec(),
    ];
    for i in 0..length {
        objects.push(if i + 1 < length {
            format!("{} 0 R", i + 5).into_bytes()
        } else if cycle {
            b"4 0 R".to_vec()
        } else {
            b"/2.0".to_vec()
        });
    }
    assemble_pdf_with_version("1.7", &objects)
}
#[test]
fn indirect_version_resolution_is_bounded_and_rejects_cycles() {
    for length in [1, 2, 64] {
        let mut reader = PdfReader::new(Cursor::new(version_chain_pdf(length, false))).unwrap();
        assert_eq!(reader.effective_version().unwrap().to_string(), "2.0");
    }
    for (length, cycle) in [(1, true), (2, true), (65, false)] {
        let mut reader = PdfReader::new(Cursor::new(version_chain_pdf(length, cycle))).unwrap();
        assert!(
            reader.effective_version().is_err(),
            "length {length}, cycle {cycle}"
        );
    }
}
