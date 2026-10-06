//! #666: vertical text pen advances are distinct from glyph placement offsets.
#[path = "../../tests/common/text_contracts.rs"]
mod contract;
use contract::assembler::stream_obj;
use contract::{cmap, pdf};
use oxidize_pdf::parser::{ParseOptions, PdfReader};
use oxidize_pdf::text::{ExtractionOptions, TextExtractor};
use std::io::Cursor;
const FONT: &[u8] = include_bytes!("../../tests/fixtures/text_contracts/fonts/SourceSans3-Regular.ttf");

pub fn vertical_pdf(metrics: &str, embedded: bool, content: &[u8]) -> Vec<u8> {
    let definition=format!("<< /Type /Font /Subtype /Type0 /BaseFont /SourceSans3-Regular /Encoding {} /DescendantFonts [6 0 R] /ToUnicode 9 0 R >>",if embedded {"11 0 R"} else {"/Identity-V"});
    let descendant=format!("<< /Type /Font /Subtype /CIDFontType2 /BaseFont /SourceSans3-Regular /CIDSystemInfo << /Registry (Contract) /Ordering (Vertical) /Supplement 0 >> /FontDescriptor 7 0 R /CIDToGIDMap 10 0 R /W [17 [544] 19 [588]] {metrics} >>");
    let descriptor=b"<< /Type /FontDescriptor /FontName /SourceSans3-Regular /Flags 4 /FontBBox [-614 -295 2159 958] /ItalicAngle 0 /Ascent 984 /Descent -273 /CapHeight 660 /StemV 80 /FontFile2 8 0 R >>".to_vec();
    let unicode = cmap("<0011> <0041>\n<0013> <0042>", 2, "<0000> <FFFF>");
    let mut gids = vec![0u8; 40];
    gids[34..36].copy_from_slice(&2u16.to_be_bytes());
    gids[38..40].copy_from_slice(&3u16.to_be_bytes());
    let encoding=stream_obj("/Type /CMap /CMapName /ContractVertical /CIDSystemInfo << /Registry (Contract) /Ordering (Vertical) /Supplement 0 >> /WMode 1",
        b"/CIDInit /ProcSet findresource begin 12 dict begin begincmap /CIDSystemInfo << /Registry (Contract) /Ordering (Vertical) /Supplement 0 >> def /CMapName /ContractVertical def /CMapType 1 def /WMode 1 def 1 begincodespacerange <0000> <FFFF> endcodespacerange 2 begincidchar <0011> 17 <0013> 19 endcidchar endcmap CMapName currentdict /CMap defineresource pop end end");
    pdf(
        &definition,
        content,
        vec![
            descendant.into_bytes(),
            descriptor,
            stream_obj(&format!("/Length1 {}", FONT.len()), FONT),
            unicode,
            stream_obj("", &gids),
            encoding,
        ],
    )
}

