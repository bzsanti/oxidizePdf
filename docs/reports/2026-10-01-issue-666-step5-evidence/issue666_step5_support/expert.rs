//! Original name-keyed CFF, with all 165 MacExpert names and actual charstrings.
//! PUA is only required when explicitly declared in ToUnicode, not inferred policy.
#[path = "../../tests/common/text_contracts.rs"]
mod contract;
use contract::assembler::stream_obj;
use contract::{cmap, extract, pdf};
use oxidize_pdf::parser::ParseOptions;
use sha2::{Digest, Sha256};
const FONT: &[u8] = include_bytes!("../../tests/fixtures/text_contracts/fonts/ContractExpert.cff");
const TABLE: &str = include_str!("../../tests/fixtures/text_contracts/MacExpertEncoding.tsv");
pub fn expert_pdf(content: &[u8], unicode: Option<Vec<u8>>) -> Vec<u8> {
    let definition = format!(
        "<< /Type /Font /Subtype /Type1 /BaseFont /ContractExpert /Encoding /MacExpertEncoding \
        /FirstChar 0 /LastChar 255 /Widths [{}] /FontDescriptor 6 0 R {} >>",
        "500 ".repeat(256),
        if unicode.is_some() {
            "/ToUnicode 8 0 R"
        } else {
            ""
        }
    );
    let descriptor =
        b"<< /Type /FontDescriptor /FontName /ContractExpert /Flags 4 /FontBBox [0 0 500 600] \
        /ItalicAngle 0 /Ascent 600 /Descent 0 /CapHeight 600 /StemV 80 /FontFile3 7 0 R >>"
            .to_vec();
    let mut objects = vec![descriptor, stream_obj("/Subtype /Type1C", FONT)];
    if let Some(unicode) = unicode {
        objects.push(unicode);
    }
    pdf(&definition, content, objects)
}
