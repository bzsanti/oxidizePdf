use oxidize_pdf::{
    operations::{
        existing_document::{plan_split_pdf, split_pdf},
        ExistingDocumentPolicy, PageRange,
    },
    parser::PdfReader,
    verification::tagged_pdf::validate_tagged_pdf,
};
use std::fs;
#[path = "common/issue690_fixture.rs"]
mod fixture;

fn ranges() -> [PageRange; 3] {
    [
        PageRange::List(vec![0, 1, 2]),
        PageRange::List(vec![3, 4, 5]),
        PageRange::List(vec![6, 7, 8, 9, 10, 11]),
    ]
}

fn verify_split(source: &[u8]) {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("source.pdf");
    fs::write(&input, source).unwrap();
    let before = PdfReader::open_document(&input)
        .unwrap()
        .extract_text()
        .unwrap();
    let outputs = (0..3)
        .map(|i| dir.path().join(format!("part{i}.pdf")))
        .collect::<Vec<_>>();
    let planned =
        plan_split_pdf(&input, &ranges(), ExistingDocumentPolicy::preserve_base()).unwrap();
    let published = split_pdf(
        &input,
        &ranges(),
        &outputs,
        ExistingDocumentPolicy::preserve_base(),
    )
    .unwrap();
    assert_eq!(planned, published, "plan and materialized reports agree");
    for (output, (start, end)) in outputs.iter().zip([(0, 3), (3, 6), (6, 12)]) {
        let bytes = fs::read(output).unwrap();
        assert!(bytes.starts_with(source), "preserve source prefix");
        let tags = validate_tagged_pdf(&bytes, &Default::default()).unwrap();
        assert!(tags.valid, "{:?}", tags.findings);
        assert_eq!(tags.parent_tree_entries, end - start);
        let after = PdfReader::open_document(output)
            .unwrap()
            .extract_text()
            .unwrap();
        assert_eq!(after.len(), end - start);
        for (actual, expected) in after.iter().zip(&before[start..end]) {
            assert_eq!(
                actual.text, expected.text,
                "preserve page content and order"
            );
        }
    }
    assert_eq!(
        fs::read(&input).unwrap(),
        source,
        "never rewrite the source"
    );
}

#[test]
fn nested_tagged_tree_with_authoritative_metadata_splits_3_3_6() {
    verify_split(&fixture::fixture(false, true, true));
}

use oxidize_pdf::parser::objects::{PdfObject, PdfString};
use oxidize_pdf::writer::{IncrementalTaggedPdfEditor, TaggedPdfMutation, TaggedPdfMutationReport};
use oxidize_pdf::{error::Result, verification::tagged_pdf::TaggedPdfValidationReport};

// Fixture input data, not a second product API. All paths below exercise the
// existing editor and its mutation/report contracts.
#[derive(Default)]
struct TaggedPdfMetadata {
    language: Option<String>,
    alternate_text: BTreeMap<TaggedPdfObjectRef, String>,
}
fn preflight_tagged_pdf(bytes: &[u8]) -> Result<TaggedPdfValidationReport> {
    IncrementalTaggedPdfEditor::new(bytes).preflight()
}
fn prepare_tagged_pdf(bytes: &[u8], values: &TaggedPdfMetadata) -> Result<TaggedPdfMutationReport> {
    let mut editor = IncrementalTaggedPdfEditor::new(bytes).with_recovered_indexes()?;
    if let Some(language) = &values.language {
        editor = editor.with_document_language(language)?;
    }
    let mutations: Vec<_> = values
        .alternate_text
        .iter()
        .map(|(element, value)| {
            let mut encoded = vec![0xfe, 0xff];
            for unit in value.encode_utf16() {
                encoded.extend_from_slice(&unit.to_be_bytes());
            }
            TaggedPdfMutation::SetElementAttribute {
                element: *element,
                key: "Alt".into(),
                value: Some(PdfObject::String(PdfString::new(encoded))),
            }
        })
        .collect();
    let plan = editor.plan(&mutations)?;
    let result = editor.apply(&mutations)?;
    assert_eq!(plan, result.plan);
    Ok(result)
}
use oxidize_pdf::verification::tagged_pdf::{TaggedPdfFindingCode as Code, TaggedPdfObjectRef};
use std::collections::BTreeMap;

fn author_metadata() -> TaggedPdfMetadata {
    TaggedPdfMetadata {
        language: Some("es-ES".to_string()),
        alternate_text: BTreeMap::from([
            (
                TaggedPdfObjectRef::from((1452, 0)),
                "Descripción explícita uno".to_string(),
            ),
            (
                TaggedPdfObjectRef::from((1453, 0)),
                "Descripción explícita dos".to_string(),
            ),
        ]),
    }
}

#[test]
fn preflight_reports_language_and_both_figures_together() {
    let bytes = fixture::fixture(false, false, false);
    let report = oxidize_pdf::writer::IncrementalTaggedPdfEditor::new(&bytes)
        .preflight()
        .expect("recover only unambiguous derived indexes");
    assert!(!report.valid);
    assert_eq!(
        report
            .findings
            .iter()
            .filter(|f| f.code == Code::MissingDocumentLanguage)
            .count(),
        1
    );
    let missing = report
        .findings
        .iter()
        .filter(|f| f.code == Code::MissingAlternateText)
        .map(|f| f.object.unwrap())
        .collect::<Vec<_>>();
    assert_eq!(
        missing,
        vec![
            TaggedPdfObjectRef::from((1452, 0)),
            TaggedPdfObjectRef::from((1453, 0))
        ]
    );
    assert_eq!(
        report.findings.len(),
        3,
        "derived keys recovered, author values still required"
    );
    assert_eq!(report.parent_tree_entries, 12);
}

#[test]
fn supplied_metadata_prepares_nested_source_without_rewriting_it() {
    let original = fixture::fixture(false, false, false);
    let prepared = prepare_tagged_pdf(&original, &author_metadata())
        .expect("explicit values complete the original nested source");
    assert!(prepared.pdf_bytes.starts_with(&original));
    assert!(prepared.validation_after.valid);
    let mut reader = PdfReader::new(std::io::Cursor::new(&prepared.pdf_bytes)).unwrap();
    for (id, parent) in [(10, 22), (17, 22), (18, 23), (21, 23), (22, 2), (23, 2)] {
        assert_eq!(
            reader
                .get_object(id, 0)
                .unwrap()
                .as_dict()
                .unwrap()
                .get("Parent")
                .unwrap()
                .as_reference(),
            Some((parent, 0)),
            "preparation must not flatten page tree"
        );
    }
    assert_eq!(
        reader
            .catalog()
            .unwrap()
            .get("Lang")
            .unwrap()
            .as_string()
            .unwrap()
            .to_text(),
        "es-ES"
    );
    for (element, expected) in author_metadata().alternate_text {
        assert_eq!(
            reader
                .get_object(element.object_number, element.generation)
                .unwrap()
                .as_dict()
                .unwrap()
                .get("Alt")
                .unwrap()
                .as_string()
                .unwrap()
                .to_text(),
            expected
        );
    }
    let mut before = PdfReader::new(std::io::Cursor::new(&original)).unwrap();
    for id in 30..42 {
        assert_eq!(
            reader.get_object(id, 0).unwrap(),
            before.get_object(id, 0).unwrap(),
            "content streams unchanged"
        );
    }
    assert_eq!(
        reader.get_object(1001, 0).unwrap(),
        before.get_object(1001, 0).unwrap(),
        "existing description unchanged"
    );
    verify_split(&prepared.pdf_bytes);
}

#[test]
fn incomplete_metadata_never_produces_prepared_bytes() {
    let original = fixture::fixture(false, false, false);
    assert!(prepare_tagged_pdf(&original, &TaggedPdfMetadata::default()).is_err());
    let mut metadata = author_metadata();
    metadata
        .alternate_text
        .remove(&TaggedPdfObjectRef::from((1453, 0)));
    assert!(prepare_tagged_pdf(&original, &metadata).is_err());
}

#[test]
fn split_rejects_all_source_metadata_before_publishing_any_part() {
    let source = fixture::fixture(false, false, false);
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("source.pdf");
    fs::write(&input, &source).unwrap();
    let outputs = (0..3)
        .map(|i| dir.path().join(format!("part{i}.pdf")))
        .collect::<Vec<_>>();
    for out in &outputs {
        fs::write(out, b"sentinel").unwrap();
    }
    let err = split_pdf(
        &input,
        &ranges(),
        &outputs,
        ExistingDocumentPolicy::preserve_base(),
    )
    .unwrap_err()
    .to_string();
    assert!(err.contains("MissingDocumentLanguage"), "{err}");
    assert_eq!(err.matches("MissingAlternateText").count(), 2, "{err}");
    for out in outputs {
        assert_eq!(fs::read(out).unwrap(), b"sentinel");
    }
    assert_eq!(fs::read(input).unwrap(), source);
}

#[test]
fn complete_matrix_preserves_nested_shape_actualtext_and_sparse_mcids() {
    for depth in 0..3 {
        for indexes in [false, true] {
            for keys in [false, true] {
                for state in 0..4 {
                    let language = state & 1 != 0;
                    let descriptions = state & 2 != 0;
                    let original = fixture::matrix(depth, indexes, keys, language, descriptions);
                    let report = preflight_tagged_pdf(&original).unwrap();
                    assert_eq!(
                        report.findings.len(),
                        usize::from(!language) + 2 * usize::from(!descriptions),
                        "depth={depth} indexes={indexes} keys={keys} state={state}"
                    );
                    let mut metadata = author_metadata();
                    if language {
                        metadata.language = None;
                    }
                    if descriptions {
                        metadata.alternate_text.clear();
                    }
                    let prepared = prepare_tagged_pdf(&original, &metadata).unwrap();
                    assert!(prepared.pdf_bytes.starts_with(&original));
                    let mut before = PdfReader::new(std::io::Cursor::new(&original)).unwrap();
                    let mut after =
                        PdfReader::new(std::io::Cursor::new(&prepared.pdf_bytes)).unwrap();
                    for id in 10..22 {
                        assert_eq!(
                            before
                                .get_object(id, 0)
                                .unwrap()
                                .as_dict()
                                .unwrap()
                                .get("Parent"),
                            after
                                .get_object(id, 0)
                                .unwrap()
                                .as_dict()
                                .unwrap()
                                .get("Parent")
                        );
                    }
                    verify_split(&prepared.pdf_bytes);
                }
            }
        }
    }
}

#[test]
fn unknown_blank_or_overwriting_metadata_is_rejected() {
    let base = fixture::fixture(false, false, false);
    let mut blank = author_metadata();
    blank.language = Some(" \t".to_string());
    assert!(prepare_tagged_pdf(&base, &blank).is_err());
    let mut unknown = author_metadata();
    unknown
        .alternate_text
        .insert((1554, 0).into(), "unknown".to_string());
    assert!(prepare_tagged_pdf(&base, &unknown).is_err());
    let mut overwrite = author_metadata();
    overwrite
        .alternate_text
        .insert((1001, 0).into(), "overwrite existing".to_string());
    assert!(prepare_tagged_pdf(&base, &overwrite).is_err());
    assert!(prepare_tagged_pdf(&fixture::fixture(false, true, false), &author_metadata()).is_err());
}

#[test]
fn genuine_deleted_page_references_are_not_hidden_by_pages_type() {
    for via_catalog in [false, true] {
        let bytes = fixture::fixture_with(false, true, true, |objects| {
            let parent = if via_catalog { 1 } else { 10 };
            let value = String::from_utf8(objects[&parent].clone())
                .unwrap()
                .replacen("<<", "<< /Private 900 0 R", 1);
            objects.insert(parent, value.into_bytes());
            objects.insert(900, b"<< /Type /Pages /Target 18 0 R >>".to_vec());
        });
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("source.pdf");
        fs::write(&input, &bytes).unwrap();
        let output = dir.path().join("output.pdf");
        let error = split_pdf(
            &input,
            &[PageRange::List(vec![0, 1, 2])],
            std::slice::from_ref(&output),
            ExistingDocumentPolicy::preserve_base(),
        )
        .unwrap_err()
        .to_string();
        assert!(error.contains("references it"), "{error}");
        assert!(!output.exists());
        assert_eq!(fs::read(input).unwrap(), bytes);
    }
}

#[test]
fn corrupt_parent_count_and_generation_remain_rejected() {
    for (old, new) in [
        ("/Parent 2 0 R", "/Parent 2 1 R"),
        ("/Count 8", "/Count 9"),
        ("/Kids [10 0 R", "/Kids [22 0 R"),
    ] {
        let bytes = fixture::fixture_with(false, true, true, |objects| {
            objects.insert(
                22,
                String::from_utf8(objects[&22].clone())
                    .unwrap()
                    .replace(old, new)
                    .into_bytes(),
            );
        });
        assert!(preflight_tagged_pdf(&bytes).is_err(), "{new}");
        assert!(prepare_tagged_pdf(&bytes, &TaggedPdfMetadata::default()).is_err());
    }
}

#[test]
fn preparation_preserves_empty_structure_groups() {
    for kids in ["/K []", "/K null", ""] {
        let bytes = fixture::fixture_with(false, false, false, |objects| {
            let document = String::from_utf8(objects[&4].clone()).unwrap();
            objects.insert(4, document.replace("]", "1700 0 R ]").into_bytes());
            objects.insert(
                1700,
                format!("<< /Type /StructElem /S /Sect /P 4 0 R /ID (empty-group) {kids} >>")
                    .into_bytes(),
            );
        });
        let report =
            preflight_tagged_pdf(&bytes).expect("empty groups are authoritative structure");
        if kids == "/K null" {
            assert!(report
                .findings
                .iter()
                .any(|f| f.code == Code::InvalidStructureElement));
            assert!(prepare_tagged_pdf(&bytes, &author_metadata()).is_err());
            continue;
        }
        assert_eq!(report.findings.len(), 3, "{kids}: {:?}", report.findings);
        let prepared = prepare_tagged_pdf(&bytes, &author_metadata()).unwrap();
        let mut before = PdfReader::new(std::io::Cursor::new(&bytes)).unwrap();
        let mut after = PdfReader::new(std::io::Cursor::new(&prepared.pdf_bytes)).unwrap();
        for id in [4, 1700] {
            assert_eq!(
                before.get_object(id, 0).unwrap(),
                after.get_object(id, 0).unwrap()
            );
        }
        assert!(prepared.validation_after.valid);
    }
}

#[test]
fn existing_editor_prepares_then_plans_and_applies_attributes() {
    use oxidize_pdf::{
        parser::objects::{PdfObject, PdfString},
        writer::{IncrementalTaggedPdfEditor, TaggedPdfMutation},
    };
    let source = fixture::fixture(false, false, false);
    let editor = IncrementalTaggedPdfEditor::new(&source)
        .with_recovered_indexes()
        .unwrap()
        .with_document_language("es-ES")
        .unwrap();
    let changed = editor.preparation_objects();
    let expected: Vec<TaggedPdfObjectRef> = [1, 3, 6, 7, 9]
        .into_iter()
        .chain(10..22)
        .chain(26..29)
        .map(|id| (id, 0).into())
        .collect();
    assert_eq!(changed, expected);

    for id in [1, 3, 6, 7, 10, 21] {
        assert!(
            changed.contains(&(id, 0).into()),
            "missing preparation object {id}"
        );
    }
    assert!(
        editor.apply(&[]).is_err(),
        "missing descriptions must block bytes"
    );
    let mutations = [1452, 1453].map(|id| TaggedPdfMutation::SetElementAttribute {
        element: (id, 0).into(),
        key: "Alt".into(),
        value: Some(PdfObject::String(PdfString::new(
            b"Explicit description".to_vec(),
        ))),
    });
    let plan = editor.plan(&mutations).unwrap();
    assert_eq!(plan.changed_objects.len(), 2);
    let result = editor.apply(&mutations).unwrap();
    assert_eq!(plan, result.plan);
    assert_eq!(result.validation_before.findings.len(), 2);
    assert!(result.validation_after.valid);
    assert!(result.pdf_bytes.starts_with(&source));
    verify_split(&result.pdf_bytes);
}

#[test]
fn ordinary_editor_still_supports_explicit_existing_attribute_edits() {
    let source = fixture::matrix(1, true, true, true, true);
    let editor = IncrementalTaggedPdfEditor::new(&source);
    assert!(editor.preparation_objects().is_empty());
    let empty = editor.apply(&[]).unwrap();
    assert_eq!(empty.pdf_bytes, source);
    let mutation = TaggedPdfMutation::SetElementAttribute {
        element: (1001, 0).into(),
        key: "Alt".into(),
        value: Some(PdfObject::String(PdfString::new(
            b"Intentional replacement".to_vec(),
        ))),
    };
    let edited = editor.apply(&[mutation]).unwrap();
    assert!(edited.validation_after.valid);
    let mut reader = PdfReader::new(std::io::Cursor::new(edited.pdf_bytes)).unwrap();
    assert_eq!(
        reader
            .get_object(1001, 0)
            .unwrap()
            .as_dict()
            .unwrap()
            .get("Alt")
            .unwrap()
            .as_string()
            .unwrap()
            .to_text(),
        "Intentional replacement"
    );
}
