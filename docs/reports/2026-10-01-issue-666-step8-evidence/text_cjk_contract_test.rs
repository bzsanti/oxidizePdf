//! #666: named Adobe CMaps and CID-to-Unicode fallback from pinned upstream data.
//! The embedded schematic font tests decoding, not CJK typography or shaping.
#[path = "common/text_contracts.rs"]
mod contract;
use contract::assembler::stream_obj;
use contract::{cmap, extract, pdf};
use oxidize_pdf::parser::ParseOptions;
use sha2::{Digest, Sha256};
const SAMPLES: &str = include_str!("fixtures/text_contracts/cjk/samples.tsv");
const FONT: &[u8] = include_bytes!("fixtures/text_contracts/fonts/ContractSansSubset-Regular.ttf");

fn fixture(row: &[&str], explicit: bool) -> Vec<u8> {
    let cid: usize = row[6].parse().unwrap();
    // Associate the selected CID with a real schematic glyph. Its outline carries
    // no Unicode claim; the character identity comes from the official collection.
    let mut gid_map = vec![0; (cid + 1) * 2];
    gid_map[cid * 2 + 1] = 2;
    let unicode_entry = if explicit { "/ToUnicode 10 0 R" } else { "" };
    let font = format!("<< /Type /Font /Subtype /Type0 /BaseFont /ContractSansSubset-Regular /Encoding /{} /DescendantFonts [6 0 R] {unicode_entry} >>", row[2]);
    let descendant = format!("<< /Type /Font /Subtype /CIDFontType2 /BaseFont /ContractSansSubset-Regular /CIDSystemInfo << /Registry (Adobe) /Ordering ({}) /Supplement {} >> /FontDescriptor 7 0 R /CIDToGIDMap 9 0 R /DW 600 >>", row[0], row[1]);
    let descriptor = b"<< /Type /FontDescriptor /FontName /ContractSansSubset-Regular /Flags 4 /FontBBox [0 -200 1000 1000] /ItalicAngle 0 /Ascent 800 /Descent -200 /CapHeight 700 /StemV 80 /FontFile2 8 0 R >>".to_vec();
    let code = row[5];
    let width = code.len();
    let space = format!("<{}> <{}>", "0".repeat(width), "F".repeat(width));
    let content = format!("BT /F1 12 Tf 100 700 Td <{code}> Tj ET");
    pdf(
        &font,
        content.as_bytes(),
        vec![
            descendant.into_bytes(),
            descriptor,
            stream_obj(&format!("/Length1 {}", FONT.len()), FONT),
            stream_obj("", &gid_map),
            cmap(&format!("<{code}> <005A>"), 1, &space),
        ],
    )
}
fn check(ordering: &str, explicit: bool) {
    let mut failures = Vec::new();
    let mut count = 0;
    for line in SAMPLES.lines().skip(1) {
        let row: Vec<_> = line.split('\t').collect();
        if row[0] != ordering {
            continue;
        }
        count += 1;
        let expected: String = if explicit {
            "Z".into()
        } else {
            row[7]
                .split(' ')
                .map(|scalar| char::from_u32(u32::from_str_radix(scalar, 16).unwrap()).unwrap())
                .collect()
        };
        let actual = extract(fixture(&row, explicit), ParseOptions::strict()).text;
        if actual != expected {
            failures.push(format!(
                "{} {} code={} CID={}: expected {:?}, got {:?}",
                row[2], row[4], row[5], row[6], expected, actual
            ));
        }
    }
    assert!(count > 0, "missing collection {ordering}");
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
#[test]
fn cjk_samples_have_pinned_identity_and_distinct_collections() {
    let manifest: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/text_contracts/cjk/provenance.json")).unwrap();
    assert_eq!(
        manifest["samples_sha256"].as_str().unwrap(),
        format!("{:x}", Sha256::digest(SAMPLES.as_bytes()))
    );
    let rows: Vec<Vec<_>> = SAMPLES
        .lines()
        .skip(1)
        .map(|l| l.split('\t').collect())
        .collect();
    assert_eq!(rows.len(), manifest["samples"].as_u64().unwrap() as usize);
    let maps: std::collections::BTreeSet<_> = rows.iter().map(|r| r[2]).collect();
    assert_eq!(maps.len(), 35);
    assert!(rows
        .iter()
        .any(|r| r[0] == "KR" && r[2] == "UniAKR-UTF16-H"));
    assert!(rows
        .iter()
        .any(|r| r[0] == "Korea1" && r[2] == "UniKS-UTF16-H"));
    assert!(rows.iter().any(|r| r[4] == "non-bmp"));
    assert!(rows.iter().any(|r| r[4] == "vertical-override"));
}
#[test]
fn adobe_gb1_named_cmaps_decode_collection_cids() {
    check("GB1", false);
}
#[test]
fn adobe_cns1_named_cmaps_decode_collection_cids() {
    check("CNS1", false);
}
#[test]
fn adobe_japan1_named_cmaps_decode_collection_cids() {
    check("Japan1", false);
}
#[test]
fn adobe_korea1_named_cmaps_decode_collection_cids() {
    check("Korea1", false);
}
#[test]
fn adobe_kr_is_a_separate_collection() {
    check("KR", false);
}
#[test]
fn explicit_unicode_overrides_all_five_collection_fallbacks() {
    for ordering in ["GB1", "CNS1", "Japan1", "Korea1", "KR"] {
        check(ordering, true);
    }
}
