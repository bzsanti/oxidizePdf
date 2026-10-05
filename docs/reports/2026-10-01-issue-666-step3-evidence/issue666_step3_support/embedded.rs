//! Full, licensed OpenType/CFF font embedded as a simple PDF Type1 font.
#[path = "../../tests/common/text_contracts.rs"]
mod contract;
use contract::assembler::stream_obj;
use contract::{cmap, extract, pdf};
use oxidize_pdf::parser::{ParseOptions, PdfReader};
use oxidize_pdf::text::{ExtractionOptions, TextExtractor};
use sha2::{Digest, Sha256};
use std::io::Cursor;

const FONT: &[u8] = include_bytes!("../../tests/fixtures/text_contracts/fonts/SourceSans3-Regular.otf");

pub fn embedded_pdf(unicode: bool) -> Vec<u8> {
    let definition = format!(
        "<< /Type /Font /Subtype /Type1 /BaseFont /SourceSans3-Regular \
        /Encoding /WinAnsiEncoding /FirstChar 65 /LastChar 66 /Widths [544 588] \
        /FontDescriptor 6 0 R {} >>",
        if unicode { "/ToUnicode 8 0 R" } else { "" }
    );
    let descriptor = b"<< /Type /FontDescriptor /FontName /SourceSans3-Regular /Flags 32 \
        /FontBBox [-614 -295 2159 958] /ItalicAngle 0 /Ascent 984 /Descent -273 \
        /CapHeight 660 /StemV 80 /FontFile3 7 0 R >>"
        .to_vec();
    let mut objects = vec![descriptor, stream_obj("/Subtype /OpenType", FONT)];
    if unicode {
        objects.push(cmap("<41> <00660069>\n<42> <D83DDE00>", 2, "<00> <FF>"));
    }
    let mut bytes = pdf(
        &definition,
        b"BT /F1 10 Tf 100 700 Td (A) Tj 1 Tr (B) Tj ET",
        objects,
    );
    // OpenType embedding is available from PDF 1.6; same-length header preserves xref.
    assert_eq!(&bytes[..8], b"%PDF-1.4");
    bytes[7] = b'7';
    bytes
}

