use oxidize_pdf::operations::existing_document::{
    plan_split_pdf, split_pdf, ExistingDocumentPolicy,
};
use oxidize_pdf::operations::PageRange;
use oxidize_pdf::parser::{objects::PdfObject, PdfReader};
use oxidize_pdf::text::TextExtractor;
use oxidize_pdf::verification::tagged_pdf::validate_tagged_pdf;
use std::{collections::BTreeMap, fs};

#[path = "common/tagged_split_fixture.rs"]
mod tagged_split_fixture;
use tagged_split_fixture::fixture_custom;

#[path = "common/tagged_missing_keys_fixture.rs"]
mod tagged_missing_keys_fixture;
use tagged_missing_keys_fixture::{missing_pages, replace};

fn verify(base: Vec<u8>, groups: &[Vec<usize>]) {
    let tmp = tempfile::tempdir().unwrap();
    let source = tmp.path().join("source.pdf");
    fs::write(&source, &base).unwrap();
    let ranges: Vec<_> = groups.iter().cloned().map(PageRange::List).collect();
    let outputs: Vec<_> = (0..groups.len())
        .map(|i| tmp.path().join(format!("part-{i}.pdf")))
        .collect();
    let plans = plan_split_pdf(&source, &ranges, ExistingDocumentPolicy::preserve_base()).unwrap();
    let reports = split_pdf(
        &source,
        &ranges,
        &outputs,
        ExistingDocumentPolicy::preserve_base(),
    )
    .unwrap();
    assert_eq!(plans, reports);
    let source_doc = PdfReader::open_document(&source).unwrap();
    let mut source_reader = PdfReader::open(&source).unwrap();
    for (out, pages) in outputs.iter().zip(groups) {
        let bytes = fs::read(out).unwrap();
        assert!(bytes.starts_with(&base));
        let tags = validate_tagged_pdf(&bytes, &Default::default()).unwrap();
        assert!(tags.valid, "{:?}", tags.findings);
        let doc = PdfReader::open_document(out).unwrap();
        assert_eq!(doc.page_count().unwrap() as usize, pages.len());
        let mut reader = PdfReader::open(out).unwrap();
        for (i, &source_index) in pages.iter().enumerate() {
            let mut extractor = TextExtractor::new();
            let text = extractor.extract_from_page(&doc, i as u32).unwrap().text;
            assert_eq!(text.trim(), format!("accessible page {}", source_index + 1));
            assert_eq!(
                text,
                extractor
                    .extract_from_page(&source_doc, source_index as u32)
                    .unwrap()
                    .text
            );
            // Object identities and encoded stream bytes must remain unchanged.
            let id = 30 + source_index as u32;
            let original = source_reader.get_object(id, 0).unwrap().clone();
            assert_eq!(reader.get_object(id, 0).unwrap(), &original);
            let page = reader
                .get_object(10 + source_index as u32, 0)
                .unwrap()
                .as_dict()
                .unwrap();
            let key = page
                .get("StructParents")
                .and_then(PdfObject::as_integer)
                .unwrap();
            assert!(key >= 0);
            if let Some(existing) = source_reader
                .get_object(10 + source_index as u32, 0)
                .unwrap()
                .as_dict()
                .unwrap()
                .get("StructParents")
            {
                assert_eq!(Some(key), existing.as_integer());
            }
            let index = reader
                .get_object(6, 0)
                .unwrap()
                .as_dict()
                .unwrap()
                .get("Nums")
                .unwrap()
                .as_array()
                .unwrap();
            let owners = index
                .0
                .chunks_exact(2)
                .find(|pair| pair[0].as_integer() == Some(key))
                .unwrap()[1]
                .as_array()
                .unwrap();
            assert_eq!(
                owners.0[0].as_reference(),
                Some((50 + source_index as u32, 0))
            );
        }
    }
    assert_eq!(fs::read(source).unwrap(), base);
}

#[test]
fn missing_keys_and_indexes_preserve_content_actualtext_and_three_parts() {
    verify(
        fixture_custom(true, false, false, missing_pages),
        &[vec![0, 1, 2], vec![3, 4, 5], (6..12).collect()],
    );
}

#[test]
fn missing_keys_with_existing_indexes_preserve_reordered_pages() {
    verify(
        fixture_custom(false, false, false, missing_pages),
        &[vec![2, 0, 1], vec![11, 6, 8]],
    );
}

#[test]
fn page_reparenting_preserves_reconstructed_keys() {
    let base = fixture_custom(true, false, false, |o| {
        missing_pages(o);
        replace(o, 2, "/Kids [10 0 R", "/Kids [80 0 R");
        replace(o, 10, "/Parent 2 0 R", "/Parent 80 0 R");
        o.insert(
            80,
            b"<< /Type /Pages /Parent 2 0 R /Count 1 /Kids [10 0 R] >>".to_vec(),
        );
    });
    verify(base, &[vec![0, 1, 2], vec![3, 4, 5], (6..12).collect()]);
}

#[test]
fn fresh_keys_do_not_collide_with_existing_page_or_later_objr_keys() {
    let base = fixture_custom(true, false, false, |o| {
        missing_pages(o);
        replace(
            o,
            11,
            "/Contents 31 0 R",
            "/Contents 31 0 R /StructParents 0",
        );
        replace(
            o,
            10,
            "/Contents 30 0 R",
            "/Contents 30 0 R /Annots [70 0 R]",
        );
        replace(o, 50, "/K 0", "/K [0 << /Type /OBJR /Obj 70 0 R >>]");
        o.insert(70, b"<< /Type /Annot /Subtype /Text /Rect [10 10 30 30] /P 10 0 R /StructParent 1 /Contents (note) >>".to_vec());
    });
    verify(base, &[vec![0, 1, 2], vec![3, 4, 5], (6..12).collect()]);
}

fn rejects(customize: impl FnOnce(&mut BTreeMap<u32, Vec<u8>>), message: &str) {
    let base = fixture_custom(true, false, false, |o| {
        missing_pages(o);
        customize(o);
    });
    let tmp = tempfile::tempdir().unwrap();
    let source = tmp.path().join("source.pdf");
    fs::write(&source, &base).unwrap();
    let outputs = [tmp.path().join("first.pdf"), tmp.path().join("second.pdf")];
    let err = split_pdf(
        &source,
        &[PageRange::Single(0), PageRange::Single(1)],
        &outputs,
        ExistingDocumentPolicy::preserve_base(),
    )
    .unwrap_err()
    .to_string();
    assert!(err.contains(message), "{err}");
    assert!(outputs.iter().all(|p| !p.exists()));
    fs::write(&outputs[0], b"preexisting destination").unwrap();
    let retry = split_pdf(
        &source,
        &[PageRange::Single(0), PageRange::Single(1)],
        &outputs,
        ExistingDocumentPolicy::preserve_base(),
    )
    .unwrap_err()
    .to_string();
    assert!(retry.contains(message), "{retry}");
    assert_eq!(fs::read(&outputs[0]).unwrap(), b"preexisting destination");
    assert!(!outputs[1].exists());
    assert_eq!(fs::read(source).unwrap(), base);
}

#[test]
fn ambiguous_owners_on_later_page_publish_no_parts() {
    rejects(
        |o| replace(o, 51, "/K 0", "/K [0 0]"),
        "multiple structure owners",
    );
}

#[test]
fn missing_content_on_later_page_publishes_no_parts() {
    rejects(
        |o| replace(o, 51, "/K 0", "/K 1"),
        "projected tagged structure is invalid",
    );
}

#[test]
fn invalid_explicit_keys_are_not_treated_as_missing() {
    for value in ["-1", "/Wrong", "null"] {
        rejects(
            |o| {
                replace(
                    o,
                    10,
                    "/Contents 30 0 R",
                    &format!("/Contents 30 0 R /StructParents {value}"),
                )
            },
            "lacks valid /StructParents",
        );
    }
}

#[test]
fn ambiguous_existing_keys_are_not_renumbered() {
    rejects(
        |o| {
            for id in [10, 11] {
                replace(o, id, "/Type /Page ", "/Type /Page /StructParents 0 ");
            }
        },
        "share /StructParents key",
    );
}

#[test]
fn missing_effective_page_is_not_guessed() {
    rejects(
        |o| {
            replace(o, 5, "/Pg 10 0 R", "");
            replace(o, 50, "/Pg 10 0 R", "");
        },
        "no effective /Pg",
    );
}

#[test]
fn explicit_mcr_and_inherited_page_recover_missing_keys() {
    let base = fixture_custom(true, false, false, |o| {
        missing_pages(o);
        replace(o, 50, "/Pg 10 0 R /K 0", "/K << /Type /MCR /MCID 0 >>");
        for n in 1..12 {
            replace(
                o,
                50 + n,
                &format!("/Pg {} 0 R /K 0", 10 + n),
                &format!("/K << /Type /MCR /Pg {} 0 R /MCID 0 >>", 10 + n),
            );
        }
    });
    verify(base, &[vec![0, 1, 2], vec![3, 4, 5], (6..12).collect()]);
}

#[test]
fn distinct_structure_owners_for_one_mcid_are_rejected() {
    rejects(
        |o| {
            replace(o, 5, "/K [50 0 R", "/K [70 0 R 50 0 R");
            o.insert(
                70,
                b"<< /Type /StructElem /S /P /P 5 0 R /Pg 10 0 R /K 0 >>".to_vec(),
            );
        },
        "multiple structure owners",
    );
}

#[test]
fn duplicate_content_mcid_is_not_repaired() {
    rejects(
        |o| {
            let content =
                "/P << /MCID 0 >> BDC (first) Tj EMC /P << /MCID 0 >> BDC (second) Tj EMC";
            o.insert(
                31,
                format!(
                    "<< /Length {} >>\nstream\n{content}\nendstream",
                    content.len()
                )
                .into_bytes(),
            );
        },
        "projected tagged structure is invalid",
    );
}

#[test]
fn certificate_permissions_still_precede_missing_key_repair() {
    rejects(
        |o| {
            replace(
                o,
                1,
                "/Type /Catalog",
                "/Type /Catalog /Perms << /DocMDP 70 0 R >>",
            );
            o.insert(70, b"<< /Type /Sig /ByteRange [0 1 2 3] /Contents <00> /Reference [<< /TransformMethod /DocMDP /TransformParams << /Type /TransformParams /P 1 /V /1.2 >> >>] >>".to_vec());
        },
        "DocMDP",
    );
}
