//! #666 F09: actual CID charset/FDSelect selection and hostile structural limits.
use oxidize_pdf::text::fonts::cff::cid::CidFont;
use oxidize_pdf::text::fonts::cff::index::parse_cff_index;
use std::path::PathBuf;

#[test]
fn real_full_and_subset_cff_select_distinct_font_dicts_and_charstrings() {
    let root =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/text_contracts/composite");
    for format in [0, 3] {
        let mut programs = Vec::new();
        for variant in ["full", "subset"] {
            let raw = std::fs::read(root.join(format!("cff-fd{format}-{variant}.cff"))).unwrap();
            let font = CidFont::parse(&raw).unwrap();
            assert_eq!(font.glyph_id(0), Some(0));
            assert_eq!(font.glyph_id(17), Some(1));
            assert_eq!(font.glyph_id(29), Some(2));
            assert_eq!(
                font.glyph_id(42),
                if variant == "full" { Some(3) } else { None }
            );
            assert_eq!(font.glyph_id(65535), None);
            assert_eq!(font.font_dict_index(1), Some(0));
            assert_eq!(font.font_dict_index(2), Some(1));
            assert_eq!(font.font_dict_index(65535), None);
            assert_ne!(font.font_dict(0), font.font_dict(1));
            assert!(font.font_dict(2).is_none());
            let first = font.charstring(1).unwrap();
            let second = font.charstring(2).unwrap();
            assert_ne!(first, second);
            assert_eq!(first.last(), Some(&14));
            assert_eq!(second.last(), Some(&14));
            assert!(font.charstring(65535).is_none());
            programs.push((first.to_vec(), second.to_vec()));
        }
        assert_eq!(
            programs[0], programs[1],
            "subset preserves selected Type2 programs"
        );
    }
}
// An independent minimal CFF assembler. All DICT offsets use the fixed i32
// operand form so replacing an offset cannot move any subsequent structure.
fn index(items: &[Vec<u8>]) -> Vec<u8> {
    let mut out = (items.len() as u16).to_be_bytes().to_vec();
    if items.is_empty() {
        return out;
    }
    out.push(1);
    let mut end = 1u8;
    out.push(end);
    for item in items {
        end += item.len() as u8;
        out.push(end);
    }
    for item in items {
        out.extend(item)
    }
    out
}
fn number(n: usize) -> Vec<u8> {
    let mut b = vec![29];
    b.extend((n as i32).to_be_bytes());
    b
}
fn fixture(charset: &[u8], fdselect: &[u8]) -> Vec<u8> {
    fn top(offsets: [usize; 4]) -> Vec<u8> {
        let mut d = vec![139, 139, 139, 12, 30];
        for (offset, op) in
            offsets
                .into_iter()
                .zip([vec![15], vec![17], vec![12, 36], vec![12, 37]])
        {
            d.extend(number(offset));
            d.extend(op);
        }
        d
    }
    let mut prefix = vec![1, 0, 4, 4];
    prefix.extend(index(&[b"F09".to_vec()]));
    let base = prefix.len() + index(&[top([0; 4])]).len() + 4;
    let charstrings = index(&[vec![14], vec![139, 14], vec![140, 14]]);
    let private = vec![139, 20];
    let mut fd = number(private.len());
    fd.extend(number(
        base + charset.len() + charstrings.len() + fdselect.len(),
    ));
    fd.push(18);
    let fds = index(&[fd.clone(), fd]);
    prefix.extend(index(&[top([
        base,
        base + charset.len(),
        base + charset.len() + charstrings.len() + fdselect.len() + private.len(),
        base + charset.len() + charstrings.len(),
    ])]));
    prefix.extend([0, 0, 0, 0]);
    prefix.extend(charset);
    prefix.extend(charstrings);
    prefix.extend(fdselect);
    prefix.extend(private);
    prefix.extend(fds);
    prefix
}
#[test]
fn charset_formats_zero_one_two_preserve_nonidentity_cids() {
    for charset in [
        vec![0, 0, 17, 0, 18],
        vec![1, 0, 17, 1],
        vec![2, 0, 17, 0, 1],
    ] {
        let raw = fixture(&charset, &[0, 0, 0, 1]);
        let font = CidFont::parse(&raw).unwrap();
        assert_eq!(
            font.mappings().collect::<Vec<_>>(),
            vec![(0, 0), (17, 1), (18, 2)]
        );
        assert_eq!(font.font_dict_index(2), Some(1));
    }
    for charset in [
        vec![0, 255, 254, 255, 255],
        vec![1, 255, 254, 1],
        vec![2, 255, 254, 0, 1],
    ] {
        let raw = fixture(&charset, &[0, 0, 0, 1]);
        let font = CidFont::parse(&raw).unwrap();
        assert_eq!(font.glyph_id(65534), Some(1));
        assert_eq!(font.glyph_id(65535), Some(2));
    }
}
#[test]
fn charset_rejects_duplicates_reserved_zero_ranges_and_truncation() {
    for charset in [
        vec![0, 0, 17, 0, 17],
        vec![0, 0, 0, 0, 17],
        vec![1, 0, 17, 2],
        vec![1, 255, 255, 1],
        vec![2, 0, 17, 255, 255],
        vec![3, 0, 17, 0, 18],
    ] {
        assert!(
            CidFont::parse(&fixture(&charset, &[0, 0, 0, 1])).is_err(),
            "{charset:?}"
        );
    }
    let raw = fixture(&[0, 0, 17, 0, 18], &[0, 0, 0, 1]);
    // No truncated prefix can contain all referenced font structures.
    for end in 0..raw.len() {
        assert!(CidFont::parse(&raw[..end]).is_err(), "prefix {end}");
    }
}
#[test]
fn fdselect_requires_full_ordered_coverage_and_existing_font_dicts() {
    let valid = [3, 0, 2, 0, 0, 0, 0, 2, 1, 0, 3];
    let raw = fixture(&[0, 0, 17, 0, 18], &valid);
    let font = CidFont::parse(&raw).unwrap();
    assert_eq!(
        (
            font.font_dict_index(0),
            font.font_dict_index(1),
            font.font_dict_index(2)
        ),
        (Some(0), Some(0), Some(1))
    );
    for (index, value) in [(4, 1), (7, 0), (10, 2), (10, 4), (8, 2)] {
        let mut bad = valid;
        bad[index] = value;
        assert!(
            CidFont::parse(&fixture(&[0, 0, 17, 0, 18], &bad)).is_err(),
            "FDSelect mutation {index}={value}"
        );
    }
    assert!(CidFont::parse(&fixture(&[0, 0, 17, 0, 18], &[0, 0, 0, 2])).is_err());
    assert!(CidFont::parse(&fixture(&[0, 0, 17, 0, 18], &[3, 0, 0, 0, 3])).is_err());
}
#[test]
fn cff_index_rejects_zero_descending_out_of_file_and_overflowing_offsets() {
    for raw in [
        vec![0, 1, 1, 0, 1],
        vec![0, 2, 1, 1, 3, 2, 1, 2],
        vec![0, 1, 4, 0, 0, 0, 1, 255, 255, 255, 255],
        vec![0, 1, 1, 2, 3, 0, 0],
    ] {
        assert!(parse_cff_index(&raw, 0).is_err(), "{raw:?}");
    }
    assert!(parse_cff_index(&[0, 0], usize::MAX).is_err());
    let raw = [0, 2, 1, 1, 1, 2, 42];
    let parsed = parse_cff_index(&raw, 0).unwrap();
    assert_eq!(parsed.get_item(0, &raw), Some([].as_slice()));
    assert_eq!(parsed.get_item(1, &raw), Some([42].as_slice()));
    assert!(parsed.get_item(usize::MAX, &raw).is_none());
}

#[test]
fn cid_cff_collection_unicode_direction_and_subset_matrix() {
    use oxidize_pdf::fonts::ResolvedFontResource;
    use oxidize_pdf::parser::{ParseOptions, PdfReader};
    use oxidize_pdf::text::{ExtractionOptions, TextExtractor};
    use std::io::Cursor;
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/text_contracts/cff_selection");
    let manifest: serde_json::Value = serde_json::from_str(include_str!(
        "fixtures/text_contracts/cff_selection/manifest.json"
    ))
    .unwrap();
    let cases = manifest["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 32);
    for case in cases {
        let name = case["path"].as_str().unwrap();
        for options in [ParseOptions::strict(), ParseOptions::lenient()] {
            let doc = PdfReader::new_with_options(
                Cursor::new(std::fs::read(root.join(name)).unwrap()),
                options,
            )
            .unwrap()
            .into_document();
            let font = ResolvedFontResource::from_page(&doc, 0, "F1").unwrap();
            let cids: Vec<u16> = case["cids"]
                .as_array()
                .unwrap()
                .iter()
                .map(|v| v.as_u64().unwrap() as u16)
                .collect();
            let codes: Vec<u8> = cids.iter().flat_map(|cid| cid.to_be_bytes()).collect();
            let glyphs = font.decode_glyphs(&codes).unwrap();
            let cff = CidFont::parse(&font.embedded_font.as_ref().unwrap().data).unwrap();
            let extracted = TextExtractor::with_options(ExtractionOptions {
                preserve_layout: true,
                sort_by_position: false,
                ..Default::default()
            })
            .extract_from_page(&doc, 0)
            .unwrap();
            assert_eq!(glyphs.len(), 2, "{name}");
            assert_eq!(extracted.fragments.len(), 2, "{name}");
            for i in 0..2 {
                assert_eq!(glyphs[i].cid, Some(u32::from(cids[i])), "{name}");
                assert_eq!(glyphs[i].gid, Some((i + 1) as u16), "{name}");
                assert_eq!(cff.font_dict_index((i + 1) as u16), Some(i as u8), "{name}");
                assert_eq!(
                    glyphs[i].unicode.as_deref(),
                    case["expected_unicode"][i].as_str(),
                    "{name}"
                );
                assert_eq!(
                    glyphs[i].advance,
                    case["advances"][i].as_f64().unwrap(),
                    "{name}"
                );
                assert_eq!(
                    extracted.fragments[i].text,
                    case["expected_unicode"][i].as_str().unwrap_or("\u{fffd}"),
                    "{name}"
                );
                let position = &case["text_origins"][i];
                assert!(
                    (extracted.fragments[i].x - position[0].as_f64().unwrap()).abs() < 0.0001
                        && (extracted.fragments[i].y - position[1].as_f64().unwrap()).abs()
                            < 0.0001,
                    "{name}"
                );
            }
        }
    }
}

#[test]
fn cid_cff_selection_fixtures_retain_frozen_identity() {
    use sha2::{Digest, Sha256};
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/text_contracts/cff_selection");
    let manifest: serde_json::Value = serde_json::from_str(include_str!(
        "fixtures/text_contracts/cff_selection/manifest.json"
    ))
    .unwrap();
    let hashes = manifest["sha256"].as_object().unwrap();
    assert_eq!(hashes.len(), 40);
    for (path, digest) in hashes {
        assert_eq!(
            format!(
                "{:x}",
                Sha256::digest(std::fs::read(root.join(path)).unwrap())
            ),
            digest.as_str().unwrap(),
            "{path}"
        );
    }
}

#[test]
fn name_keyed_opentype_cff_used_as_cidfont_keeps_identity_glyph_indices() {
    let raw = include_bytes!("fixtures/text_contracts/fonts/SourceSans3-Regular.otf");
    let table_count = u16::from_be_bytes([raw[4], raw[5]]) as usize;
    let entry = raw[12..12 + table_count * 16]
        .chunks_exact(16)
        .find(|entry| &entry[..4] == b"CFF ")
        .unwrap();
    let start = u32::from_be_bytes(entry[8..12].try_into().unwrap()) as usize;
    let length = u32::from_be_bytes(entry[12..16].try_into().unwrap()) as usize;
    let unwrapped = CidFont::parse(&raw[start..start + length]).unwrap();
    assert_eq!(unwrapped.glyph_id(65), Some(65));
    let font = CidFont::parse(raw).unwrap();
    for gid in [0, 17, 29, 65] {
        assert_eq!(font.glyph_id(gid), Some(gid));
        assert!(font.charstring(gid).is_some());
        assert_eq!(font.font_dict_index(gid), None);
    }
    assert_eq!(font.glyph_id(65535), None);
    assert!(font.font_dict(0).is_none());
}
