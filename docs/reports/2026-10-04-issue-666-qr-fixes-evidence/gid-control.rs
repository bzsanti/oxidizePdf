//! #666 F09/F10: full/subset CID fonts, multiple FDArray entries, Identity/stream GIDs.
use oxidize_pdf::fonts::ResolvedFontResource;
use oxidize_pdf::parser::{ParseOptions, PdfReader};
use oxidize_pdf::text::{ExtractionOptions, TextExtractor};
use sha2::{Digest, Sha256};
use std::io::Cursor;
use std::path::PathBuf;
fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/text_contracts/composite")
}
fn manifest() -> serde_json::Value {
    serde_json::from_str(include_str!(
        "/home/santi/repos/BelowZero/oxidizePdf/oxidize-pdf/oxidize-pdf-core/tests/fixtures/text_contracts/composite/readers.json"
    ))
    .unwrap()
}
fn verify(select: impl Fn(&str) -> bool, expected_count: usize) {
    let mut checked = 0;
    for case in manifest()["cases"].as_array().unwrap() {
        let path = case["path"].as_str().unwrap();
        if !select(path) {
            continue;
        }
        checked += 1;
        for options in [ParseOptions::strict(), ParseOptions::lenient()] {
            let doc = PdfReader::new_with_options(
                Cursor::new(std::fs::read(root().join(path)).unwrap()),
                options,
            )
            .unwrap()
            .into_document();
            if path.starts_with("ttf-") {
                let remapped = path.ends_with("-remapped.pdf");
                let identity = path.ends_with("-identity-gids.pdf");
                let codes: &[u8] = if remapped {
                    b"AB"
                } else if identity {
                    &[0, 2, 0, 3]
                } else {
                    &[0, 17, 0, 29]
                };
                let font = ResolvedFontResource::from_page(&doc, 0, "F1").unwrap();
                let glyphs = font.decode_glyphs(codes).unwrap();
                let expected_cids = if remapped {
                    [29, 17]
                } else if identity {
                    [2, 3]
                } else {
                    [17, 29]
                };
                let expected_gids = if remapped { [3, 2] } else { [2, 3] };
                assert_eq!(glyphs.len(), 2, "{path}");
                for (i, glyph) in glyphs.iter().enumerate() {
                    assert_eq!(glyph.cid, Some(expected_cids[i]), "{path}: CID {i}");
                    assert_eq!(glyph.gid, Some(expected_gids[i]), "{path}: GID {i}");
                }
            }
            let result = TextExtractor::with_options(ExtractionOptions {
                preserve_layout: true,
                sort_by_position: false,
                ..ExtractionOptions::default()
            })
            .extract_from_page(&doc, 0)
            .unwrap();
            assert_eq!(result.text, "AB", "{path}");
            assert_eq!(result.fragments.len(), 2, "{path}");
            for (fragment, expected) in result
                .fragments
                .iter()
                .zip(case["expected_trace_origins"].as_array().unwrap())
            {
                assert_eq!(fragment.text, expected["text"].as_str().unwrap(), "{path}");
                let x = expected["origin"][0].as_f64().unwrap();
                assert!(
                    (fragment.x - x).abs() < 0.0001 && (fragment.y - 700.0).abs() < 0.0001,
                    "{path}: expected({x},700), got({},{})",
                    fragment.x,
                    fragment.y
                );
            }
        }
    }
    assert_eq!(checked, expected_count);
}
#[test]
fn cff_fixtures_preserve_identity_cid_text_and_pdf_widths() {
    verify(|p| p.starts_with("cff-") && p.ends_with("-identity.pdf"), 4);
}
#[test]
fn cff_fixtures_preserve_remapped_cid_text_and_pdf_widths() {
    verify(|p| p.starts_with("cff-") && p.ends_with("-remapped.pdf"), 4);
}
#[test]
fn full_and_subset_truetype_support_identity_cid_to_gid_map() {
    verify(|p| p.ends_with("-identity-gids.pdf"), 2);
}
#[test]
fn full_and_subset_truetype_support_stream_cid_to_gid_map() {
    verify(|p| p.ends_with("-stream-gids.pdf"), 2);
}
#[test]
fn full_and_subset_truetype_keep_source_codes_cids_and_gids_distinct() {
    verify(|p| p.starts_with("ttf-") && p.ends_with("-remapped.pdf"), 2);
}
#[test]
fn six_real_programs_and_fourteen_pdfs_match_their_frozen_hashes() {
    let p: serde_json::Value = serde_json::from_str(include_str!(
        "/home/santi/repos/BelowZero/oxidizePdf/oxidize-pdf/oxidize-pdf-core/tests/fixtures/text_contracts/composite/provenance.json"
    ))
    .unwrap();
    let programs = p["programs"].as_array().unwrap();
    assert_eq!(programs.len(), 6);
    for program in programs.iter().filter(|p| p["kind"] == "cff") {
        let fds = program["fds"].as_array().unwrap();
        assert!(fds.contains(&serde_json::json!(0)) && fds.contains(&serde_json::json!(1)));
        assert_eq!(program["charset"][1], "cid00017");
        assert_eq!(program["charset"][2], "cid00029");
    }
    let hashes = p["sha256"].as_object().unwrap();
    assert_eq!(hashes.len(), 20);
    for (path, digest) in hashes {
        assert_eq!(
            format!(
                "{:x}",
                Sha256::digest(std::fs::read(root().join(path)).unwrap())
            ),
            digest.as_str().unwrap(),
            "{path}"
        );
    }
}
