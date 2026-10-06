#[path = "common/text_contracts.rs"]
mod contract;
use oxidize_pdf::fonts::ResolvedFontResource;
use oxidize_pdf::parser::{ParseOptions, PdfReader};
use std::io::Cursor;
fn root() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/text_contracts")
}
fn resolved(path: &str, codes: &[u8]) -> Vec<oxidize_pdf::fonts::DecodedGlyph> {
    let doc = PdfReader::open(root().join(path)).unwrap().into_document();
    ResolvedFontResource::from_page(&doc, 0, "F1")
        .unwrap()
        .decode_glyphs(codes)
        .unwrap()
}
#[test]
fn resolved_flat_control() {
    let g = resolved("usecmap/flat.pdf", &[1, 2]);
    assert_eq!(
        g.iter().map(|g| g.cid).collect::<Vec<_>>(),
        vec![Some(17), Some(29)]
    );
    assert_eq!(
        g.iter().map(|g| g.unicode.as_deref()).collect::<Vec<_>>(),
        vec![Some("A"), Some("B")]
    );
    assert_eq!(g[0].advance, 400.0);
}
#[test]
fn resolved_encoding_parent() {
    let g = resolved("usecmap/encoding-parent.pdf", &[1, 2]);
    assert_eq!(
        g.iter().map(|g| g.cid).collect::<Vec<_>>(),
        vec![Some(17), Some(29)]
    );
    assert_eq!(g[0].advance, 400.0);
}
#[test]
fn resolved_unicode_parent() {
    let g = resolved("usecmap/unicode-parent.pdf", &[1, 2]);
    assert_eq!(
        g.iter().map(|g| g.unicode.as_deref()).collect::<Vec<_>>(),
        vec![Some("A"), Some("B")]
    );
}
#[test]
fn resolved_named_parent() {
    let g = resolved("usecmap/named-dictionary.pdf", &[0, 17, 0, 29]);
    assert_eq!(g.len(), 2);
    assert_eq!(g[0].cid, Some(17));
}
#[test]
fn resolved_intrinsic_type1() {
    intrinsic("pfb");
}
#[test]
fn resolved_intrinsic_cff() {
    intrinsic("cff");
}
fn intrinsic(kind: &str) {
    let g = resolved(&format!("type1/{kind}-full-intrinsic.pdf"), b"AB");
    assert_eq!(
        g.iter().map(|g| g.unicode.as_deref()).collect::<Vec<_>>(),
        vec![Some("B"), Some("A")],
        "{kind}"
    );
}
#[test]
fn resolved_vertical_advance() {
    let bytes=contract::pdf("<< /Type /Font /Subtype /Type0 /Encoding /Identity-V /DescendantFonts [6 0 R] /ToUnicode 7 0 R >>",b"", vec![b"<< /Type /Font /Subtype /CIDFontType2 /BaseFont /Demo /W [17 [544]] /DW2 [880 -1300] /W2 [17 [-1200 272 880]] >>".to_vec(),contract::cmap("<0011> <0041>",1,"<0000> <FFFF>")]);
    let doc = PdfReader::new(Cursor::new(bytes)).unwrap().into_document();
    let font = ResolvedFontResource::from_page(&doc, 0, "F1").unwrap();
    assert_eq!(font.writing_mode, oxidize_pdf::fonts::WritingMode::Vertical);
    assert_eq!(font.decode_glyphs(&[0, 17]).unwrap()[0].advance, -1200.0);
}
#[test]
fn strict_public_extraction_rejects_cyclic_usecmap() {
    let bytes=contract::pdf("<< /Type /Font /Subtype /Type0 /Encoding /Identity-H /DescendantFonts [6 0 R] /ToUnicode 7 0 R >>",b"BT /F1 10 Tf (A) Tj ET",vec![b"<< /Type /Font /Subtype /CIDFontType2 /BaseFont /Demo /DW 500 >>".to_vec(),contract::assembler::stream_obj("/UseCMap 7 0 R",b"begincmap 1 begincodespacerange <00> <FF> endcodespacerange 1 beginbfchar <41> <0058> endbfchar endcmap")]);
    let doc = PdfReader::new_with_options(Cursor::new(bytes), ParseOptions::strict())
        .unwrap()
        .into_document();
    let result = doc.extract_text();
    assert!(result.is_err(), "strict cyclic UseCMap: {result:?}");
}
#[test]
fn type1_procedure_is_not_executed() {
    let path = root().join("qr/type1-procedure.pdf");
    let bytes = std::fs::read(path).unwrap();
    assert_eq!(contract::extract(bytes, ParseOptions::strict()).text, "BA");
}

#[test]
fn inherited_adobe_kr_operator() {
    inherited_kr("");
}
#[test]
fn inherited_adobe_kr_dictionary() {
    inherited_kr("/UseCMap /Adobe-KR-UCS2");
}
fn inherited_kr(parent: &str) {
    // Independent pinned sample: Adobe-KR CID14238 -> U+4E00.

    let operator = if parent.is_empty() {
        "/Adobe-KR-UCS2 usecmap"
    } else {
        ""
    };
    let unicode = format!(
        "begincmap {operator} 1 begincodespacerange <0000> <FFFF> endcodespacerange endcmap"
    );
    let bytes=contract::pdf("<< /Type /Font /Subtype /Type0 /Encoding /Identity-H /DescendantFonts [6 0 R] /ToUnicode 7 0 R >>", b"BT /F1 10 Tf <379E> Tj ET",vec![b"<< /Type /Font /Subtype /CIDFontType2 /BaseFont /Demo /CIDSystemInfo << /Registry (Adobe) /Ordering (KR) /Supplement 9 >> /DW 500 >>".to_vec(),contract::assembler::stream_obj(parent,unicode.as_bytes())]);
    assert_eq!(
        contract::extract(bytes, ParseOptions::strict()).text,
        "一",
        "{parent}"
    );
}
#[test]
fn indirect_cid_collection_matches_direct_extraction() {
    let bytes=contract::pdf("<< /Type /Font /Subtype /Type0 /Encoding /UniJIS-UTF16-H /DescendantFonts [6 0 R] >>", b"BT /F1 10 Tf <00E1> Tj ET",vec![b"<< /Type /Font /Subtype /CIDFontType2 /BaseFont /Demo /CIDSystemInfo 7 0 R /W [194 [555]] >>".to_vec(),b"<< /Registry (Adobe) /Ordering (Japan1) /Supplement 7 >>".to_vec()]);
    assert_eq!(contract::extract(bytes, ParseOptions::strict()).text, "á");
}

#[test]
fn legacy_collection_matches_stay_exhaustive() {
    use oxidize_pdf::text::cid_to_unicode::{AdobeCidCollection, CidCollection};
    fn ordering(collection: CidCollection) -> &'static str {
        match collection {
            CidCollection::Cns1 => "CNS1",
            CidCollection::Gb1 => "GB1",
            CidCollection::Japan1 => "Japan1",
            CidCollection::Korea1 => "Korea1",
        }
    }
    for name in ["CNS1", "GB1", "Japan1", "Korea1"] {
        assert_eq!(ordering(CidCollection::from_ordering(name).unwrap()), name);
    }
    assert_eq!(CidCollection::from_ordering("KR"), None);
    assert_eq!(
        AdobeCidCollection::from_ordering("KR")
            .unwrap()
            .cid_to_unicode(14238),
        Some('一')
    );
}

fn cycle_pdf(form: bool, cyclic: bool, inline: bool) -> Vec<u8> {
    use contract::assembler::{assemble_pdf, stream_obj};
    let font = "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /StandardEncoding /ToUnicode 7 0 R >>";
    let resource = if inline { font } else { "4 0 R" };
    let resources = format!("<< /Font << /F1 {resource} >> >>");
    let page_resources = if form {
        "<< /XObject << /Fm 6 0 R >> >>"
    } else {
        &resources
    };
    assemble_pdf(&[
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        format!("<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Resources {page_resources} /Contents 5 0 R >>").into_bytes(),
        font.as_bytes().to_vec(),
        stream_obj("", if form { b"/Fm Do" } else { b"BT /F1 10 Tf (A) Tj ET" }),
        stream_obj(&format!("/Type /XObject /Subtype /Form /BBox [0 0 612 792] /Resources {resources}"), b"BT /F1 10 Tf (A) Tj ET"),
        stream_obj(if cyclic { "/UseCMap 7 0 R" } else { "" }, b"begincmap 1 begincodespacerange <00> <FF> endcodespacerange 1 beginbfchar <41> <0058> endbfchar endcmap"),
    ])
}

#[test]
fn page_and_form_strict_cycles_error_while_lenient_keeps_child() {
    for form in [false, true] {
        for inline in [false, true] {
            for cyclic in [false, true] {
                for strict in [false, true] {
                    let options = if strict {
                        ParseOptions::strict()
                    } else {
                        ParseOptions::lenient()
                    };
                    let doc = PdfReader::new_with_options(
                        Cursor::new(cycle_pdf(form, cyclic, inline)),
                        options,
                    )
                    .unwrap()
                    .into_document();
                    let result = doc.extract_text();
                    if strict && cyclic {
                        assert!(
                            result.unwrap_err().to_string().contains("UseCMap"),
                            "form={form}, inline={inline}"
                        );
                    } else {
                        assert_eq!(
                            result.unwrap()[0].text,
                            "X",
                            "form={form}, inline={inline}, cyclic={cyclic}, strict={strict}"
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn resolved_intrinsic_differences_and_tounicode_precedence() {
    for kind in ["pfb", "cff"] {
        // Existing original fixtures independently fix their expected strings.
        let manifest: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(root().join("type1/readers.json")).unwrap(),
        )
        .unwrap();
        for case in manifest["cases"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|c| c["path"].as_str().unwrap().starts_with(kind))
        {
            let path = case["path"].as_str().unwrap();
            let glyphs = resolved(&format!("type1/{path}"), b"AB");
            let actual: String = glyphs.iter().filter_map(|g| g.unicode.as_deref()).collect();
            assert_eq!(actual, case["expected_text"].as_str().unwrap(), "{path}");
        }
    }
}
