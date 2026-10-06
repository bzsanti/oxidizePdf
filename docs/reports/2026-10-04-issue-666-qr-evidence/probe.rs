#[path = "/home/santi/repos/BelowZero/oxidizePdf/oxidize-pdf/oxidize-pdf-core/tests/common/text_contracts.rs"] mod contract;
use oxidize_pdf::parser::{PdfReader,ParseOptions};
use oxidize_pdf::fonts::ResolvedFontResource;
use std::io::Cursor;
fn root() -> std::path::PathBuf { std::path::PathBuf::from("/home/santi/repos/BelowZero/oxidizePdf/oxidize-pdf/oxidize-pdf-core/tests/fixtures/text_contracts") }
fn resolved(path:&str, codes:&[u8]) -> Vec<oxidize_pdf::fonts::DecodedGlyph> {
 let doc=PdfReader::open(root().join(path)).unwrap().into_document();
 ResolvedFontResource::from_page(&doc,0,"F1").unwrap().decode_glyphs(codes).unwrap()
}
#[test] fn resolved_flat_control() {
 let g=resolved("usecmap/flat.pdf", &[1,2]);
 assert_eq!(g.iter().map(|g|g.cid).collect::<Vec<_>>(),vec![Some(17),Some(29)]);
 assert_eq!(g.iter().map(|g|g.unicode.as_deref()).collect::<Vec<_>>(),vec![Some("A"),Some("B")]);
 assert_eq!(g[0].advance,400.0);
}
#[test] fn resolved_encoding_parent() {
 let g=resolved("usecmap/encoding-parent.pdf", &[1,2]);
 assert_eq!(g.iter().map(|g|g.cid).collect::<Vec<_>>(),vec![Some(17),Some(29)]);
 assert_eq!(g[0].advance,400.0);
}
#[test] fn resolved_unicode_parent() {
 let g=resolved("usecmap/unicode-parent.pdf", &[1,2]);
 assert_eq!(g.iter().map(|g|g.unicode.as_deref()).collect::<Vec<_>>(),vec![Some("A"),Some("B")]);
}
#[test] fn resolved_named_parent() {
 let g=resolved("usecmap/named-dictionary.pdf", &[0,17,0,29]);
 assert_eq!(g.len(),2);
 assert_eq!(g[0].cid,Some(17));
}
#[test] fn resolved_intrinsic_type1() {
 for kind in ["pfb","cff"] {
 let g=resolved(&format!("type1/{kind}-full-intrinsic.pdf"),b"AB");
 assert_eq!(g.iter().map(|g|g.unicode.as_deref()).collect::<Vec<_>>(),vec![Some("B"),Some("A")],"{kind}");
 }
}
#[test] fn resolved_vertical_advance() {
 let bytes=contract::pdf("<< /Type /Font /Subtype /Type0 /Encoding /Identity-V /DescendantFonts [6 0 R] /ToUnicode 7 0 R >>",b"", vec![b"<< /Type /Font /Subtype /CIDFontType2 /BaseFont /Demo /W [17 [544]] /DW2 [880 -1300] /W2 [17 [-1200 272 880]] >>".to_vec(),contract::cmap("<0011> <0041>",1,"<0000> <FFFF>")]);
 let doc=PdfReader::new(Cursor::new(bytes)).unwrap().into_document();
 let font=ResolvedFontResource::from_page(&doc,0,"F1").unwrap();
 assert_eq!(font.writing_mode,oxidize_pdf::fonts::WritingMode::Vertical);
 assert_eq!(font.decode_glyphs(&[0,17]).unwrap()[0].advance,-1200.0);
}
#[test] fn strict_public_extraction_rejects_cyclic_usecmap() {
 let bytes=contract::pdf("<< /Type /Font /Subtype /Type0 /Encoding /Identity-H /DescendantFonts [6 0 R] /ToUnicode 7 0 R >>",b"BT /F1 10 Tf (A) Tj ET",vec![b"<< /Type /Font /Subtype /CIDFontType2 /BaseFont /Demo /DW 500 >>".to_vec(),contract::assembler::stream_obj("/UseCMap 7 0 R",b"begincmap 1 begincodespacerange <00> <FF> endcodespacerange 1 beginbfchar <41> <0058> endbfchar endcmap")]);
 let doc=PdfReader::new_with_options(Cursor::new(bytes),ParseOptions::strict()).unwrap().into_document();
 let result=doc.extract_text();
 assert!(result.is_err(),"strict cyclic UseCMap: {result:?}");
}
#[test] fn type1_procedure_is_not_executed() {
 let path=std::path::Path::new("/home/santi/repos/BelowZero/oxidizePdf/oxidize-pdf/docs/reports/2026-10-04-issue-666-qr-evidence/type1-procedure.pdf");
 let bytes=std::fs::read(path).unwrap();
 assert_eq!(contract::extract(bytes,ParseOptions::strict()).text,"BA");
}

#[test] fn inherited_adobe_kr_is_not_korea1() {
 // Independent pinned sample: Adobe-KR CID14238 -> U+4E00.
 for parent in ["", "/UseCMap /Adobe-KR-UCS2"] {
 let operator=if parent.is_empty() { "/Adobe-KR-UCS2 usecmap" } else { "" };
 let unicode=format!("begincmap {operator} 1 begincodespacerange <0000> <FFFF> endcodespacerange endcmap");
 let bytes=contract::pdf("<< /Type /Font /Subtype /Type0 /Encoding /Identity-H /DescendantFonts [6 0 R] /ToUnicode 7 0 R >>", b"BT /F1 10 Tf <379E> Tj ET",vec![b"<< /Type /Font /Subtype /CIDFontType2 /BaseFont /Demo /CIDSystemInfo << /Registry (Adobe) /Ordering (KR) /Supplement 9 >> /DW 500 >>".to_vec(),contract::assembler::stream_obj(parent,unicode.as_bytes())]);
 assert_eq!(contract::extract(bytes,ParseOptions::strict()).text,"一","{parent}");
 }
}
#[test] fn indirect_cid_collection_matches_direct_extraction() {
 let bytes=contract::pdf("<< /Type /Font /Subtype /Type0 /Encoding /UniJIS-UTF16-H /DescendantFonts [6 0 R] >>", b"BT /F1 10 Tf <00E1> Tj ET",vec![b"<< /Type /Font /Subtype /CIDFontType2 /BaseFont /Demo /CIDSystemInfo 7 0 R /W [194 [555]] >>".to_vec(),b"<< /Registry (Adobe) /Ordering (Japan1) /Supplement 7 >>".to_vec()]);
 assert_eq!(contract::extract(bytes,ParseOptions::strict()).text,"á");
}
