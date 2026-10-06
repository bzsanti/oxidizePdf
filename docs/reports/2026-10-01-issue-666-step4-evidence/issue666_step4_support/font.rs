//! Full/subset TrueType fixtures: independent font program and metrics.
#[path = "../../tests/common/text_contracts.rs"]
mod contract;
use contract::assembler::stream_obj;
use contract::{cmap, extract, pdf};
use oxidize_pdf::parser::{ParseOptions, PdfReader};
use oxidize_pdf::text::{ExtractionOptions, TextExtractor};
use sha2::{Digest, Sha256};
use std::io::Cursor;
const FULL: &[u8] = include_bytes!("../../tests/fixtures/text_contracts/fonts/SourceSans3-Regular.ttf");
const SUBSET: &[u8] =
    include_bytes!("../../tests/fixtures/text_contracts/fonts/ContractSansSubset-Regular.ttf");
const PROVENANCE: &str = include_str!("../../tests/fixtures/text_contracts/fonts/truetype-provenance.json");

pub fn truetype_pdf(subset: bool, unicode: bool, content: &[u8]) -> Vec<u8> {
    let (name, bytes) = if subset {
        ("ABCDEF+ContractSansSubset-Regular", SUBSET)
    } else {
        ("SourceSans3-Regular", FULL)
    };
    let reference: serde_json::Value = serde_json::from_str(PROVENANCE).unwrap();
    let widths = reference["winansi_widths_32_233"]
        .as_array()
        .unwrap()
        .iter()
        .map(|n| n.to_string())
        .collect::<Vec<_>>()
        .join(" ");
    let definition = format!(
        "<< /Type /Font /Subtype /TrueType /BaseFont /{name} /Encoding /WinAnsiEncoding \
        /FirstChar 32 /LastChar 233 /Widths [{widths}] /FontDescriptor 6 0 R {} >>",
        if unicode { "/ToUnicode 8 0 R" } else { "" }
    );
    let descriptor = format!(
        "<< /Type /FontDescriptor /FontName /{name} /Flags 32 /FontBBox [-614 -295 2159 958] \
        /ItalicAngle 0 /Ascent 984 /Descent -273 /CapHeight 660 /StemV 80 /FontFile2 7 0 R >>"
    );
    let mut objects = vec![
        descriptor.into_bytes(),
        stream_obj(&format!("/Length1 {}", bytes.len()), bytes),
    ];
    if unicode {
        objects.push(cmap(
            "<41> <00660069>\n<E9> <00E9>\n<42> <D83DDE00>",
            3,
            "<00> <FF>",
        ));
    }
    pdf(&definition, content, objects)
}

