//! #666: genuine CID-keyed CFF; charset CIDs 17/29 are not GIDs 1/2.
#[path = "common/text_contracts.rs"]
mod contract;
use contract::assembler::stream_obj;
use contract::{cmap, extract, pdf};
use oxidize_pdf::parser::{ParseOptions, PdfReader};
use oxidize_pdf::text::{ExtractionOptions, TextExtractor};
use sha2::{Digest, Sha256};
use std::io::Cursor;
const FONT: &[u8] = include_bytes!("fixtures/text_contracts/fonts/ContractCID.cff");
fn cid_cff_pdf(remap: bool, vertical: bool, content: &[u8]) -> Vec<u8> {
    cid_cff_pdf_policy(remap, vertical, content, true)
}
fn cid_cff_pdf_policy(remap: bool, vertical: bool, content: &[u8], unicode: bool) -> Vec<u8> {
    let to_unicode = if unicode { "/ToUnicode 9 0 R" } else { "" };
    let encoding = if remap {
        "10 0 R"
    } else if vertical {
        "/Identity-V"
    } else {
        "/Identity-H"
    };
    let font=format!("<< /Type /Font /Subtype /Type0 /BaseFont /ContractCID /Encoding {encoding} /DescendantFonts [6 0 R] {to_unicode} >>");
    let descendant=b"<< /Type /Font /Subtype /CIDFontType0 /BaseFont /ContractCID /CIDSystemInfo << /Registry (Contract) /Ordering (Synthetic) /Supplement 0 >> /FontDescriptor 7 0 R /DW 500 /W [17 [400] 29 [700]] /DW2 [880 -1000] /W2 [17 [-1200 200 880] 29 [-900 350 880]] >>".to_vec();
    let descriptor=b"<< /Type /FontDescriptor /FontName /ContractCID /Flags 4 /FontBBox [0 0 700 600] /ItalicAngle 0 /Ascent 600 /Descent 0 /CapHeight 600 /StemV 80 /FontFile3 8 0 R >>".to_vec();
    let unicode = if remap {
        cmap("<41> <0042>\n<42> <0041>", 2, "<00> <FF>")
    } else {
        cmap("<0011> <0041>\n<001D> <0042>", 2, "<0000> <FFFF>")
    };
    let map=stream_obj("/Type /CMap /CMapName /ContractCIDMap /CIDSystemInfo << /Registry (Contract) /Ordering (Synthetic) /Supplement 0 >> /WMode 0",
        b"/CIDInit /ProcSet findresource begin 12 dict begin begincmap /CIDSystemInfo << /Registry (Contract) /Ordering (Synthetic) /Supplement 0 >> def /CMapName /ContractCIDMap def /CMapType 1 def /WMode 0 def 1 begincodespacerange <00> <FF> endcodespacerange 2 begincidchar <41> 29 <42> 17 endcidchar endcmap CMapName currentdict /CMap defineresource pop end end");
    pdf(
        &font,
        content,
        vec![
            descendant,
            descriptor,
            stream_obj("/Subtype /CIDFontType0C", FONT),
            unicode,
            map,
        ],
    )
}
#[test]
fn cid_keyed_cff_matches_its_independently_generated_program() {
    let reference: serde_json::Value = serde_json::from_str(include_str!(
        "fixtures/text_contracts/fonts/cid-cff-provenance.json"
    ))
    .unwrap();
    assert_eq!(
        reference["sha256"].as_str(),
        Some(format!("{:x}", Sha256::digest(FONT)).as_str())
    );
    assert_eq!(
        reference["charset_by_gid"],
        serde_json::json!([".notdef", "cid00017", "cid00029"])
    );
    assert_eq!(FONT[0], 1);
}
#[test]
fn identity_h_cid_cff_preserves_explicit_unicode() {
    assert_eq!(
        extract(
            cid_cff_pdf(false, false, b"BT /F1 12 Tf <0011001D> Tj ET"),
            ParseOptions::strict()
        )
        .text,
        "AB"
    );
}
#[test]
fn embedded_encoding_remaps_codes_before_selecting_cff_cids() {
    assert_eq!(
        extract(
            cid_cff_pdf(true, false, b"BT /F1 12 Tf <4142> Tj ET"),
            ParseOptions::strict()
        )
        .text,
        "BA"
    );
}
fn assert_positions(
    remap: bool,
    vertical: bool,
    content: &[u8],
    texts: &[&str],
    expected: &[(f64, f64)],
) {
    let doc = PdfReader::new_with_options(
        Cursor::new(cid_cff_pdf(remap, vertical, content)),
        ParseOptions::strict(),
    )
    .unwrap()
    .into_document();
    let result = TextExtractor::with_options(ExtractionOptions {
        preserve_layout: true,
        sort_by_position: false,
        ..ExtractionOptions::default()
    })
    .extract_from_page(&doc, 0)
    .unwrap();
    assert_eq!(result.fragments.len(), expected.len());
    let mut failures = Vec::new();
    for (i, (fragment, (x, y))) in result.fragments.iter().zip(expected).enumerate() {
        if fragment.text != texts[i]
            || (fragment.x - x).abs() > 0.0001
            || (fragment.y - y).abs() > 0.0001
        {
            failures.push(format!(
                "glyph {i}: expected {} at ({x},{y}), got {:?} at ({},{})",
                texts[i], fragment.text, fragment.x, fragment.y
            ));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
#[test]
fn cid_cff_widths_use_cids_not_charstring_indices() {
    assert_positions(
        false,
        false,
        b"BT /F1 10 Tf 100 700 Td <0011> Tj 1 Tr <001D> Tj ET",
        &["A", "B"],
        &[(100.0, 700.0), (104.0, 700.0)],
    );
}
#[test]
fn remapped_cid_cff_widths_follow_encoding_cids() {
    assert_positions(
        true,
        false,
        b"BT /F1 10 Tf 100 700 Td <41> Tj 1 Tr <42> Tj ET",
        &["B", "A"],
        &[(100.0, 700.0), (107.0, 700.0)],
    );
}
#[test]
fn cid_cff_vertical_advance_uses_w2() {
    assert_positions(
        false,
        true,
        b"BT /F1 10 Tf 100 700 Td <0011> Tj 1 Tr <001D> Tj ET",
        &["A", "B"],
        &[(100.0, 700.0), (100.0, 688.0)],
    );
}

#[test]
fn cff_renderer_keeps_cid_gid_unicode_and_vertical_advance_separate() {
    for options in [ParseOptions::strict(), ParseOptions::lenient()] {
        for vertical in [false, true] {
            let doc = PdfReader::new_with_options(
                Cursor::new(cid_cff_pdf(false, vertical, b"")),
                options.clone(),
            )
            .unwrap()
            .into_document();
            let font = oxidize_pdf::fonts::ResolvedFontResource::from_page(&doc, 0, "F1").unwrap();
            let glyphs = font.decode_glyphs(&[0, 17, 0, 29]).unwrap();
            assert_eq!(
                glyphs
                    .iter()
                    .map(|g| (g.cid, g.gid, g.unicode.as_deref(), g.advance))
                    .collect::<Vec<_>>(),
                vec![
                    (
                        Some(17),
                        Some(1),
                        Some("A"),
                        if vertical { -1200. } else { 400. }
                    ),
                    (
                        Some(29),
                        Some(2),
                        Some("B"),
                        if vertical { -900. } else { 700. }
                    )
                ]
            );
        }
    }
}
#[test]
fn private_cid_cff_without_tounicode_does_not_turn_cids_or_bytes_into_text() {
    for options in [ParseOptions::strict(), ParseOptions::lenient()] {
        for remap in [false, true] {
            let codes: &[u8] = if remap { b"AB" } else { &[0, 17, 0, 29] };
            let content: &[u8] = if remap {
                b"BT /F1 10 Tf 100 700 Td <41> Tj 1 Tr <42> Tj ET"
            } else {
                b"BT /F1 10 Tf 100 700 Td <0011> Tj 1 Tr <001D> Tj ET"
            };
            let doc = PdfReader::new_with_options(
                Cursor::new(cid_cff_pdf_policy(remap, false, content, false)),
                options.clone(),
            )
            .unwrap()
            .into_document();
            let font = oxidize_pdf::fonts::ResolvedFontResource::from_page(&doc, 0, "F1").unwrap();
            let glyphs = font.decode_glyphs(codes).unwrap();
            assert!(glyphs.iter().all(|g| g.unicode.is_none()));
            assert_eq!(
                glyphs.iter().map(|g| g.gid).collect::<Vec<_>>(),
                if remap {
                    vec![Some(2), Some(1)]
                } else {
                    vec![Some(1), Some(2)]
                }
            );
            let text = TextExtractor::with_options(ExtractionOptions {
                preserve_layout: true,
                sort_by_position: false,
                ..Default::default()
            })
            .extract_from_page(&doc, 0)
            .unwrap();
            assert_eq!(
                text.fragments
                    .iter()
                    .map(|f| f.text.as_str())
                    .collect::<Vec<_>>(),
                vec!["\u{fffd}", "\u{fffd}"]
            );
            assert!((text.fragments[1].x - if remap { 107. } else { 104. }).abs() < 0.0001);
        }
    }
}

#[test]
fn invalid_cff_program_is_rejected_without_discarding_explicit_text_metadata() {
    for options in [ParseOptions::strict(), ParseOptions::lenient()] {
        let mut raw = cid_cff_pdf(false, false, b"BT /F1 10 Tf <0011001D> Tj ET");
        let start = raw
            .windows(FONT.len())
            .position(|bytes| bytes == FONT)
            .unwrap();
        raw[start] = 2; // CFF2 is not the declared CIDFontType0C CFF1 format.
        let doc = PdfReader::new_with_options(Cursor::new(raw), options)
            .unwrap()
            .into_document();
        let result = oxidize_pdf::fonts::ResolvedFontResource::from_page(&doc, 0, "F1");
        assert!(result.is_err());
        assert_eq!(
            TextExtractor::new()
                .extract_from_page(&doc, 0)
                .unwrap()
                .text,
            "AB"
        );
    }
}
