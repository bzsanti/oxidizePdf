//! #666: genuine CID-keyed CFF; charset CIDs 17/29 are not GIDs 1/2.
#[path = "../../tests/common/text_contracts.rs"]
mod contract;
use contract::assembler::stream_obj;
use contract::{cmap, extract, pdf};
use oxidize_pdf::parser::{ParseOptions, PdfReader};
use oxidize_pdf::text::{ExtractionOptions, TextExtractor};
use sha2::{Digest, Sha256};
use std::io::Cursor;
const FONT: &[u8] = include_bytes!("../../tests/fixtures/text_contracts/fonts/ContractCID.cff");
pub fn cid_cff_pdf(remap: bool, vertical: bool, content: &[u8]) -> Vec<u8> {
    let encoding = if remap {
        "10 0 R"
    } else if vertical {
        "/Identity-V"
    } else {
        "/Identity-H"
    };
    let font=format!("<< /Type /Font /Subtype /Type0 /BaseFont /ContractCID /Encoding {encoding} /DescendantFonts [6 0 R] /ToUnicode 9 0 R >>");
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
