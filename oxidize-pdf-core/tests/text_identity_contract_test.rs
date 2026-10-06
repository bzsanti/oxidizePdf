//! #666 C01: Identity code units are CIDs, including surrogate-looking pairs.
#[path = "common/text_contracts.rs"]
mod contract;
use oxidize_pdf::fonts::ResolvedFontResource;
use oxidize_pdf::parser::{ParseOptions, PdfReader};
use std::io::Cursor;

fn fixture(cff: bool, vertical: bool, unicode: bool, hex: &str) -> Vec<u8> {
    let font = if cff {
        include_bytes!("fixtures/text_contracts/fonts/ContractCID.cff").as_slice()
    } else {
        include_bytes!("fixtures/text_contracts/cid_truetype/full.ttf").as_slice()
    };
    let kind = if cff { 0 } else { 2 };
    let file = if cff { 3 } else { 2 };
    let encoding = if vertical { "Identity-V" } else { "Identity-H" };
    let mapping = if unicode { "/ToUnicode 9 0 R" } else { "" };
    contract::pdf(&format!("<< /Type /Font /Subtype /Type0 /BaseFont /Contract /Encoding /{encoding} /DescendantFonts [6 0 R] {mapping} >>"),format!("BT /F1 10 Tf <{hex}> Tj ET").as_bytes(),vec![
        format!("<< /Type /Font /Subtype /CIDFontType{kind} /BaseFont /Contract /CIDSystemInfo << /Registry (Contract) /Ordering (Synthetic) /Supplement 0 >> /FontDescriptor 7 0 R /DW 500 {} >>",if cff {""}else{"/CIDToGIDMap /Identity"}).into_bytes(),
        format!("<< /Type /FontDescriptor /FontName /Contract /Flags 4 /FontBBox [0 0 700 600] /ItalicAngle 0 /Ascent 600 /Descent 0 /CapHeight 600 /StemV 80 /FontFile{file} 8 0 R >>").into_bytes(),
        contract::assembler::stream_obj(if cff {"/Subtype /CIDFontType0C"}else{""},font),
        contract::cmap("<0000> <005A> <FFFF> <0058> <D83D> <00660069> <DE00> <0059>",4,"<0000> <FFFF>")])
}

#[test]
fn identity_never_treats_cids_as_unicode_or_combines_surrogate_looking_codes() {
    for cff in [false, true] {
        for vertical in [false, true] {
            for unicode in [false, true] {
                for options in [ParseOptions::strict(), ParseOptions::lenient()] {
                    let bytes = fixture(cff, vertical, unicode, "0000FFFFD83DDE00");
                    assert_eq!(
                        contract::extract(bytes.clone(), options.clone()).text,
                        if unicode { "ZXfiY" } else { "����" }
                    );
                    let doc = PdfReader::new_with_options(Cursor::new(bytes), options)
                        .unwrap()
                        .into_document();
                    let font = ResolvedFontResource::from_page(&doc, 0, "F1").unwrap();
                    let glyphs = font
                        .decode_glyphs(&[0, 0, 255, 255, 0xd8, 0x3d, 0xde, 0])
                        .unwrap();
                    assert_eq!(
                        glyphs.iter().map(|g| g.cid).collect::<Vec<_>>(),
                        [Some(0), Some(65535), Some(55357), Some(56832)]
                    );
                    assert_eq!(
                        glyphs
                            .iter()
                            .map(|g| g.unicode.as_deref())
                            .collect::<Vec<_>>(),
                        if unicode {
                            vec![Some("Z"), Some("X"), Some("fi"), Some("Y")]
                        } else {
                            vec![None; 4]
                        }
                    );
                    assert!(glyphs
                        .iter()
                        .all(|g| g.advance == if vertical { -1000. } else { 500. }));
                }
            }
        }
    }
}

#[test]
fn identity_empty_and_odd_tail_have_explicit_consumer_policies() {
    for cff in [false, true] {
        for vertical in [false, true] {
            for options in [ParseOptions::strict(), ParseOptions::lenient()] {
                for (hex, expected) in [("", ""), ("FF", "�"), ("FFFF00", "��")] {
                    let bytes = fixture(cff, vertical, false, hex);
                    assert_eq!(
                        contract::extract(bytes.clone(), options.clone()).text,
                        expected
                    );
                    let doc = PdfReader::new_with_options(Cursor::new(bytes), options.clone())
                        .unwrap()
                        .into_document();
                    let font = ResolvedFontResource::from_page(&doc, 0, "F1").unwrap();
                    if hex.is_empty() {
                        assert!(font.decode_glyphs(&[]).unwrap().is_empty());
                    } else {
                        assert!(font
                            .decode_glyphs(if hex.len() == 2 {
                                &[255]
                            } else {
                                &[255, 255, 0]
                            })
                            .unwrap_err()
                            .to_string()
                            .contains("truncated"));
                    }
                }
            }
        }
    }
}
