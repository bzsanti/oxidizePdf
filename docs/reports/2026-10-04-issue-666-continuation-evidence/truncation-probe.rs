//! #666 C07: source-code boundaries and notdef selection are separate from Unicode.
//! Adobe TN5014 §§5.2/5.4/7: notdef ranges select a constant CID; ordinary mappings win.
#[path = "../../../oxidize-pdf-core/tests/common/text_contracts.rs"]
mod contract;
use contract::assembler::stream_obj;
use oxidize_pdf::parser::{ParseOptions, PdfReader};
use oxidize_pdf::text::{ExtractionOptions, TextExtractor};
use std::io::Cursor;

fn cmap_stream(kind: u8, name: &str, body: &str, parent: &str) -> Vec<u8> {
    let ros = "/Registry (Contract) /Ordering (Synthetic) /Supplement 0";
    let dictionary = if kind == 1 {
        format!("/Type /CMap /CMapName /{name} /CIDSystemInfo << {ros} >> /WMode 0 {parent}")
    } else {
        String::new()
    };
    let program = format!("/CIDInit /ProcSet findresource begin 12 dict begin begincmap /CMapName /{name} def /CMapType {kind} def /CIDSystemInfo << {ros} >> def /WMode 0 def {body} endcmap CMapName currentdict /CMap defineresource pop end end");
    stream_obj(&dictionary, program.as_bytes())
}

fn document(child: &str, parent: Option<&str>, unicode: &str, content: &[u8]) -> Vec<u8> {
    let mut objects = vec![
        b"<< /Type /Font /Subtype /CIDFontType0 /BaseFont /ContractCID /CIDSystemInfo << /Registry (Contract) /Ordering (Synthetic) /Supplement 0 >> /FontDescriptor 10 0 R /DW 900 /W [0 [500] 17 [400] 29 [700]] >>".to_vec(),
        cmap_stream(1, "BoundaryChild", child, if parent.is_some() { "/UseCMap 9 0 R" } else { "" }),
        cmap_stream(2, "BoundaryUnicode", unicode, ""),
        cmap_stream(1, "BoundaryParent", parent.unwrap_or(""), ""),
        b"<< /Type /FontDescriptor /FontName /ContractCID /Flags 4 /FontBBox [0 0 700 600] /ItalicAngle 0 /Ascent 600 /Descent 0 /CapHeight 600 /StemV 80 /FontFile3 11 0 R >>".to_vec(),
    ];
    objects.push(stream_obj(
        "/Subtype /CIDFontType0C",
        include_bytes!("../../../oxidize-pdf-core/tests/fixtures/text_contracts/fonts/ContractCID.cff"),
    ));
    contract::pdf("<< /Type /Font /Subtype /Type0 /BaseFont /ContractCID /Encoding 7 0 R /ToUnicode 8 0 R /DescendantFonts [6 0 R] >>",content,objects)
}

fn main() {
 let space="4 begincodespacerange <00> <7F> <8100> <81FF> <820000> <82FFFF> <83000000> <83FFFFFF> endcodespacerange";
 let enc=format!("{space} 4 begincidchar <41> 17 <8101> 29 <820002> 17 <83000003> 29 endcidchar");
 let uni=format!("{space} 4 beginbfchar <41> <0041> <8101> <0042> <820002> <0043> <83000003> <0044> endbfchar");
 for strict in [false,true] {
  for (label,operand) in [("valid","<41810182000283000003> Tj"),("tail2","<4181> Tj"),("tail3","<418200> Tj"),("tail4","<41830000> Tj"),("partial_only","<81> Tj"),("unmapped","<418102> Tj"),("split","[<4181> <01>] TJ")] {
   let content=format!("BT /F1 10 Tf 100 700 Td {operand} ET");
   let doc=PdfReader::new_with_options(Cursor::new(document(&enc,None,&uni,content.as_bytes())),if strict {ParseOptions::strict()} else {ParseOptions::lenient()}).unwrap().into_document();
   let result=TextExtractor::with_options(ExtractionOptions {sort_by_position:false,..Default::default()}).extract_from_page(&doc,0);
   println!("strict={strict} case={label} result={:?}",result.map(|x|x.text));
  }
 }
}
