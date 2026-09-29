use oxidize_pdf::operations::existing_document::{
    plan_split_pdf, split_pdf, ExistingDocumentPolicy,
};
use oxidize_pdf::operations::PageRange;
use oxidize_pdf::parser::{
    objects::{PdfDictionary, PdfObject},
    PdfReader,
};
use oxidize_pdf::text::TextExtractor;
use oxidize_pdf::verification::tagged_pdf::validate_tagged_pdf;
use std::{collections::BTreeMap, fs};

fn fixture(missing_indexes: bool, bad_metadata: bool, corrupt_index: bool) -> Vec<u8> {
    fixture_custom(missing_indexes, bad_metadata, corrupt_index, |_| {})
}
fn fixture_custom(
    missing_indexes: bool,
    bad_metadata: bool,
    corrupt_index: bool,
    customize: impl FnOnce(&mut BTreeMap<u32, Vec<u8>>),
) -> Vec<u8> {
    let mut objects: BTreeMap<u32, Vec<u8>> = BTreeMap::new();
    objects.insert(1, format!("<< /Type /Catalog /Pages 2 0 R /Lang (en-US) /MarkInfo << /Marked true >> /StructTreeRoot 3 0 R {} >>", if bad_metadata {"/Metadata 9 0 R"} else {""}).into_bytes());
    objects.insert(
        2,
        format!(
            "<< /Type /Pages /Count 12 /Kids [{}] >>",
            (10..22)
                .map(|n| format!("{n} 0 R"))
                .collect::<Vec<_>>()
                .join(" ")
        )
        .into_bytes(),
    );
    objects.insert(3, b"<< /Type /StructTreeRoot /K 4 0 R /ParentTree 6 0 R /IDTree 7 0 R /ParentTreeNextKey 12 >>".to_vec());
    objects.insert(
        4,
        b"<< /Type /StructElem /S /Document /P 3 0 R /K 5 0 R >>".to_vec(),
    );
    objects.insert(
        5,
        format!(
            "<< /Type /StructElem /S /Sect /P 4 0 R /Pg 10 0 R /K [{}] >>",
            (50..62)
                .map(|n| format!("{n} 0 R"))
                .collect::<Vec<_>>()
                .join(" ")
        )
        .into_bytes(),
    );
    if !missing_indexes {
        objects.insert(
            6,
            if corrupt_index {
                b"not-a-pdf-object".to_vec()
            } else {
                format!(
                    "<< /Nums [{}] >>",
                    (0..12)
                        .map(|n| format!("{n} [{} 0 R]", 50 + n))
                        .collect::<Vec<_>>()
                        .join(" ")
                )
                .into_bytes()
            },
        );
        objects.insert(
            7,
            format!(
                "<< /Names [{}] >>",
                (0..12)
                    .map(|n| format!("(p{n:02}) {} 0 R", 50 + n))
                    .collect::<Vec<_>>()
                    .join(" ")
            )
            .into_bytes(),
        );
    }
    objects.insert(
        8,
        b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_vec(),
    );
    for n in 0..12 {
        objects.insert(10+n,format!("<< /Type /Page /Parent 2 0 R /MediaBox [0 0 300 300] /Resources << /Font << /F1 8 0 R >> >> /Contents {} 0 R /StructParents {n} >>",30+n).into_bytes());
        let content = format!(
            "/P << /MCID 0 >> BDC BT /F1 12 Tf 20 200 Td (page {}) Tj ET EMC\n",
            n + 1
        );
        objects.insert(
            30 + n,
            format!(
                "<< /Length {} >>\nstream\n{content}endstream",
                content.len()
            )
            .into_bytes(),
        );
        objects.insert(
            50 + n,
            format!(
                "<< /Type /StructElem /S /P /P 5 0 R /Pg {} 0 R /K 0 /ID (p{n:02}) >>",
                10 + n
            )
            .into_bytes(),
        );
    }
    customize(&mut objects);
    let size = objects.keys().max().copied().unwrap() + 1;
    let mut bytes = b"%PDF-1.4\n".to_vec();
    let mut offsets = BTreeMap::new();
    for (id, object) in objects {
        offsets.insert(id, bytes.len());
        bytes.extend_from_slice(format!("{id} 0 obj\n").as_bytes());
        bytes.extend(object);
        bytes.extend_from_slice(b"\nendobj\n");
    }
    let xref = bytes.len();
    bytes.extend_from_slice(format!("xref\n0 {size}\n0000000000 65535 f \n").as_bytes());
    for id in 1..size {
        let in_use = offsets.contains_key(&id)
            || (missing_indexes && (id == 6 || id == 7))
            || (bad_metadata && id == 9);
        bytes.extend_from_slice(
            format!(
                "{:010} 00000 {} \n",
                offsets.get(&id).copied().unwrap_or(0),
                if in_use { 'n' } else { 'f' }
            )
            .as_bytes(),
        );
    }
    bytes.extend_from_slice(
        format!("trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n").as_bytes(),
    );
    bytes
}
fn dictionary(reader: &mut PdfReader<std::fs::File>, value: PdfObject) -> PdfDictionary {
    match value {
        PdfObject::Dictionary(d) => d,
        PdfObject::Reference(n, g) => match reader.get_object(n, g).unwrap() {
            PdfObject::Dictionary(d) => d.clone(),
            other => panic!("{other:?}"),
        },
        other => panic!("{other:?}"),
    }
}
fn verify_split(missing: bool) {
    verify_split_custom(missing, |_| {});
}
fn verify_split_custom(missing: bool, customize: impl FnOnce(&mut BTreeMap<u32, Vec<u8>>)) {
    let tmp = tempfile::tempdir().unwrap();
    let source = tmp.path().join("source.pdf");
    let base = fixture_custom(missing, false, false, customize);
    fs::write(&source, &base).unwrap();
    if !missing {
        let report = validate_tagged_pdf(&base, &Default::default()).unwrap();
        assert!(report.valid, "{:?}", report.findings);
    }
    let ranges = [
        PageRange::List(vec![0, 1, 2]),
        PageRange::List(vec![3, 4, 5]),
        PageRange::List((6..12).collect()),
    ];
    let outputs: Vec<_> = (0..3)
        .map(|n| tmp.path().join(format!("part-{n}.pdf")))
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
    for (part, (start, end)) in [(0, 3), (3, 6), (6, 12)].into_iter().enumerate() {
        let bytes = fs::read(&outputs[part]).unwrap();
        if let Some(directory) = std::env::var_os("OXIDIZE_621_FIXTURES") {
            fs::write(
                std::path::Path::new(&directory).join(format!("part-{missing}-{part}.pdf")),
                &bytes,
            )
            .unwrap();
        }
        assert!(bytes.starts_with(&base));
        let tags = validate_tagged_pdf(&bytes, &Default::default()).unwrap();
        assert!(tags.valid, "{:?}", tags.findings);
        assert_eq!(tags.parent_tree_entries, end - start);
        assert_eq!(tags.elements.len(), end - start + 2);
        let mut reader = PdfReader::open(&outputs[part]).unwrap();
        assert_eq!(reader.page_count().unwrap() as usize, end - start);
        let root = reader
            .catalog()
            .unwrap()
            .get("StructTreeRoot")
            .unwrap()
            .clone();
        let root = dictionary(&mut reader, root);
        let ids = dictionary(&mut reader, root.get("IDTree").unwrap().clone());
        let names = match ids.get("Names").unwrap() {
            PdfObject::Array(a) => a,
            _ => panic!("IDTree must be rebuilt"),
        };
        let actual: Vec<_> = names
            .0
            .chunks_exact(2)
            .map(|pair| pair[0].as_string().unwrap().as_bytes().to_vec())
            .collect();
        assert_eq!(
            actual,
            (start..end)
                .map(|n| format!("p{n:02}").into_bytes())
                .collect::<Vec<_>>()
        );
        let doc = PdfReader::open_document(&outputs[part]).unwrap();
        let mut extractor = TextExtractor::new();
        for (page, n) in (start..end).enumerate() {
            assert_eq!(
                extractor
                    .extract_from_page(&doc, page as u32)
                    .unwrap()
                    .text
                    .trim(),
                format!("page {}", n + 1)
            );
        }
    }
    assert_eq!(fs::read(source).unwrap(), base);
}
#[test]
fn valid_tagged_twelve_page_split_preserves_each_part() {
    verify_split(false);
}
#[test]
fn missing_zero_offset_parent_and_id_indexes_are_reconstructed() {
    verify_split(true);
}
#[test]
fn unrelated_missing_reference_identifies_object_and_publishes_nothing() {
    let tmp = tempfile::tempdir().unwrap();
    let input = tmp.path().join("in.pdf");
    fs::write(&input, fixture(true, true, false)).unwrap();
    let output = tmp.path().join("out.pdf");
    let error = split_pdf(
        &input,
        &[PageRange::Single(0)],
        std::slice::from_ref(&output),
        ExistingDocumentPolicy::preserve_base(),
    )
    .unwrap_err()
    .to_string();
    assert!(error.contains("9 0") && error.contains("zero"), "{error}");
    assert!(!output.exists());
}
#[test]
fn corrupt_nonzero_index_is_not_silently_rebuilt() {
    let tmp = tempfile::tempdir().unwrap();
    let input = tmp.path().join("in.pdf");
    fs::write(&input, fixture(false, false, true)).unwrap();
    let output = tmp.path().join("out.pdf");
    let error = split_pdf(
        &input,
        &[PageRange::Single(0)],
        std::slice::from_ref(&output),
        ExistingDocumentPolicy::preserve_base(),
    )
    .unwrap_err()
    .to_string();
    assert!(error.contains("6 0"), "{error}");
    assert!(!output.exists());
}
#[test]
fn invalid_later_part_does_not_publish_earlier_parts() {
    let tmp = tempfile::tempdir().unwrap();
    let input = tmp.path().join("in.pdf");
    let base = fixture(false, false, false);
    fs::write(&input, &base).unwrap();
    let outputs = [tmp.path().join("a.pdf"), tmp.path().join("b.pdf")];
    assert!(split_pdf(
        &input,
        &[PageRange::Single(0), PageRange::Single(12)],
        &outputs,
        ExistingDocumentPolicy::preserve_base()
    )
    .is_err());
    assert!(outputs.iter().all(|p| !p.exists()));
    assert_eq!(fs::read(input).unwrap(), base);
}

fn rejects_fixture(base: Vec<u8>, message: &str) {
    let tmp = tempfile::tempdir().unwrap();
    let source = tmp.path().join("source.pdf");
    let output = tmp.path().join("out.pdf");
    fs::write(&source, &base).unwrap();
    let error = split_pdf(
        &source,
        &[PageRange::Single(0)],
        std::slice::from_ref(&output),
        ExistingDocumentPolicy::preserve_base(),
    )
    .unwrap_err()
    .to_string();
    assert!(error.contains(message), "{error}");
    assert!(!output.exists());
    assert_eq!(fs::read(source).unwrap(), base);
}
#[test]
fn zero_offset_index_alias_in_metadata_is_not_exempted() {
    rejects_fixture(
        fixture_custom(true, false, false, |objects| {
            objects.insert(
                1,
                b"<< /Type /Catalog /Pages 2 0 R /StructTreeRoot 3 0 R /Metadata 6 0 R >>".to_vec(),
            );
        }),
        "6 0 R",
    );
}
#[test]
fn cyclic_structure_fails_without_output() {
    rejects_fixture(
        fixture_custom(false, false, false, |objects| {
            objects.insert(
                50,
                b"<< /Type /StructElem /S /P /P 5 0 R /Pg 10 0 R /K 5 0 R >>".to_vec(),
            );
        }),
        "cyclic",
    );
}
#[test]
fn missing_marked_content_is_not_declared_accessible() {
    rejects_fixture(
        fixture_custom(false, false, false, |objects| {
            objects.insert(
                50,
                b"<< /Type /StructElem /S /P /P 5 0 R /Pg 10 0 R /K 1 >>".to_vec(),
            );
        }),
        "projected tagged structure is invalid",
    );
}
#[test]
fn broken_structure_parent_is_rejected() {
    rejects_fixture(
        fixture_custom(false, false, false, |objects| {
            objects.insert(
                50,
                b"<< /Type /StructElem /S /P /P 4 0 R /Pg 10 0 R /K 0 >>".to_vec(),
            );
        }),
        "inconsistent /P",
    );
}

#[test]
fn explicit_mcr_and_inherited_page_associations_survive_split() {
    verify_split_custom(false, |objects| {
        for n in 0..12 {
            objects.insert(50+n,format!("<< /Type /StructElem /S /P /P 5 0 R /K << /Type /MCR /Pg {} 0 R /MCID 0 >> /ID (p{n:02}) >>",10+n).into_bytes());
        }
    });
}
#[test]
fn certified_source_is_rejected_before_attempting_tag_repair() {
    rejects_fixture(
        fixture_custom(true, false, false, |objects| {
            objects.insert(1,b"<< /Type /Catalog /Pages 2 0 R /StructTreeRoot 3 0 R /Perms << /DocMDP 70 0 R >> >>".to_vec());
            objects.insert(70,b"<< /Type /Sig /ByteRange [0 1 2 3] /Contents <00> /Reference [<< /TransformMethod /DocMDP /TransformParams << /Type /TransformParams /P 1 /V /1.2 >> >>] >>".to_vec());
        }),
        "DocMDP",
    );
}
#[test]
fn missing_index_with_wrong_generation_is_not_repaired() {
    rejects_fixture(
        fixture_custom(true, false, false, |objects| {
            objects.insert(
                3,
                b"<< /Type /StructTreeRoot /K 4 0 R /ParentTree 6 1 R /IDTree 7 0 R >>".to_vec(),
            );
        }),
        "6 1 R",
    );
}
#[test]
fn sparse_parent_tree_allocation_is_bounded() {
    let tmp = tempfile::tempdir().unwrap();
    let input = tmp.path().join("in.pdf");
    let base = fixture_custom(false, false, false, |objects| {
        for n in 0..12 {
            objects.insert(
                50 + n,
                format!(
                    "<< /Type /StructElem /S /P /P 5 0 R /Pg {} 0 R /K 99999 >>",
                    10 + n
                )
                .into_bytes(),
            );
        }
    });
    fs::write(&input, base).unwrap();
    let err = plan_split_pdf(
        &input,
        &[PageRange::List((0..11).collect())],
        ExistingDocumentPolicy::preserve_base(),
    )
    .unwrap_err()
    .to_string();
    assert!(err.contains("allocation exceeds limit"), "{err}");
}

#[test]
fn tagged_annotation_parent_mapping_is_retained_only_in_its_part() {
    let base = fixture_custom(false, false, false, |objects| {
        let page = String::from_utf8(objects[&10].clone())
            .unwrap()
            .replace("/StructParents 0", "/StructParents 0 /Annots [70 0 R]");
        objects.insert(10, page.into_bytes());
        objects.insert(70,b"<< /Type /Annot /Subtype /Text /Rect [10 10 30 30] /P 10 0 R /StructParent 100 /Contents (public note) >>".to_vec());
        objects.insert(50,b"<< /Type /StructElem /S /P /P 5 0 R /Pg 10 0 R /K [0 << /Type /OBJR /Obj 70 0 R >>] /ID (p00) >>".to_vec());
        objects.insert(
            6,
            format!(
                "<< /Nums [{} 100 50 0 R] >>",
                (0..12)
                    .map(|n| format!("{n} [{} 0 R]", 50 + n))
                    .collect::<Vec<_>>()
                    .join(" ")
            )
            .into_bytes(),
        );
    });
    let input_tags = validate_tagged_pdf(&base, &Default::default()).unwrap();
    assert!(input_tags.valid, "{:?}", input_tags.findings);
    let tmp = tempfile::tempdir().unwrap();
    let input = tmp.path().join("in.pdf");
    fs::write(&input, &base).unwrap();
    let outputs = [tmp.path().join("first.pdf"), tmp.path().join("second.pdf")];
    split_pdf(
        &input,
        &[PageRange::Single(0), PageRange::Single(1)],
        &outputs,
        ExistingDocumentPolicy::preserve_base(),
    )
    .unwrap();
    for (i, out) in outputs.iter().enumerate() {
        let bytes = fs::read(out).unwrap();
        let tags = validate_tagged_pdf(&bytes, &Default::default()).unwrap();
        assert!(tags.valid, "{:?}", tags.findings);
        assert_eq!(tags.parent_tree_entries, if i == 0 { 2 } else { 1 });
        let mut reader = PdfReader::open(out).unwrap();
        let value = reader
            .catalog()
            .unwrap()
            .get("StructTreeRoot")
            .unwrap()
            .clone();
        let root = dictionary(&mut reader, value);
        let parent = dictionary(&mut reader, root.get("ParentTree").unwrap().clone());
        let entries = parent.get("Nums").unwrap().as_array().unwrap();
        let keys: Vec<_> = entries
            .0
            .chunks_exact(2)
            .map(|p| p[0].as_integer().unwrap())
            .collect();
        assert_eq!(keys, if i == 0 { vec![0, 100] } else { vec![1] });
    }
}
