//! Public regressions for issue #690: no inherited live xref entries at offset zero.
use oxidize_pdf::{
    parser::{xref::XRefTable, ParseOptions, PdfReader},
    writer::IncrementalTaggedPdfEditor,
};
use std::io::{BufReader, Cursor};

const SOURCE: &[u8] = include_bytes!("fixtures/issue690-unused-xref.pdf");

fn assert_retired(bytes: &[u8], ids: &[u32]) {
    let xref = XRefTable::parse_with_options(
        &mut BufReader::new(Cursor::new(bytes)),
        &ParseOptions::strict(),
    )
    .unwrap();
    for id in ids {
        let entry = xref.get_entry(*id).expect("retired entry remains explicit");
        assert!(
            !entry.in_use,
            "object {id} must no longer be in use at offset zero"
        );
        assert_eq!(
            entry.generation, 65535,
            "retired identities must never be reused"
        );
        assert_eq!(
            entry.offset, 0,
            "permanently free entries link to object zero"
        );
    }
}

#[test]
fn public_reproducer_retires_only_the_unreachable_zero_offset_entry() {
    let editor = IncrementalTaggedPdfEditor::new(SOURCE)
        .with_recovered_indexes()
        .unwrap();
    assert!(editor.preparation_objects().contains(&(10, 0).into()));
    let result = editor.apply(&[]).unwrap();
    assert!(result.validation_after.valid);
    assert!(result.pdf_bytes.starts_with(SOURCE));
    assert_retired(&result.pdf_bytes, &[10]);
    let mut before = PdfReader::new(Cursor::new(SOURCE)).unwrap();
    let mut after = PdfReader::new(Cursor::new(&result.pdf_bytes)).unwrap();
    for id in [1, 2, 5, 8, 9] {
        assert_eq!(
            before.get_object(id, 0).unwrap(),
            after.get_object(id, 0).unwrap()
        );
    }
    assert!(after.get_object(6, 0).unwrap().as_dict().is_some());
    assert!(after.get_object(7, 0).unwrap().as_dict().is_some());
}

#[test]
fn trailer_reference_to_zero_offset_object_is_not_discarded() {
    let source = String::from_utf8(SOURCE.to_vec())
        .unwrap()
        .replace("/Size 11 /Root 1 0 R", "/Size 11 /Root 1 0 R /Info 10 0 R");
    let error = IncrementalTaggedPdfEditor::new(source.as_bytes())
        .with_recovered_indexes()
        .err()
        .expect("reachable trailer corruption must fail");
    assert!(error.to_string().contains("10"), "{error}");
}

#[test]
fn catalog_reference_to_zero_offset_object_is_not_discarded() {
    // Append a valid incremental catalog definition, preserving all old offsets.
    let mut source = SOURCE.to_vec();
    let offset = source.len();
    source.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R /Lang (en) /MarkInfo << /Marked true >> /StructTreeRoot 3 0 R /Private 10 0 R >>\nendobj\n");
    let xref = source.len();
    source.extend_from_slice(format!("xref\n1 1\n{offset:010} 00000 n \ntrailer\n<< /Size 11 /Root 1 0 R /Prev 653 >>\nstartxref\n{xref}\n%%EOF\n").as_bytes());
    let error = IncrementalTaggedPdfEditor::new(&source)
        .with_recovered_indexes()
        .err()
        .expect("reachable catalog corruption must fail");
    assert!(error.to_string().contains("10"), "{error}");
}

#[allow(dead_code)]
#[path = "common/issue690_fixture.rs"]
mod fixture;

fn record(name: &str, bytes: &[u8]) {
    if let Some(directory) = std::env::var_os("OXIDIZE_690_EVIDENCE_DIR") {
        let directory = std::path::PathBuf::from(directory);
        std::fs::create_dir_all(&directory).unwrap();
        std::fs::write(directory.join(name), bytes).unwrap();
    }
}

fn append_entry(
    source: &[u8],
    number: u32,
    generation: u16,
    object: Option<&[u8]>,
    trailer: &str,
) -> Vec<u8> {
    let mut bytes = source.to_vec();
    let offset = bytes.len();
    if let Some(object) = object {
        bytes.extend_from_slice(format!("{number} {generation} obj\n").as_bytes());
        bytes.extend_from_slice(object);
        bytes.extend_from_slice(b"\nendobj\n");
    }
    let xref = bytes.len();
    let entry_offset = if object.is_some() { offset } else { 0 };
    let size = 11.max(number + 1);
    bytes.extend_from_slice(format!("xref\n{number} 1\n{entry_offset:010} {generation:05} n \ntrailer\n<< /Size {size} /Root 1 0 R /Prev 653 {trailer} >>\nstartxref\n{xref}\n%%EOF\n").as_bytes());
    bytes
}

fn stream_source(hybrid: bool) -> Vec<u8> {
    stream_source_with_missing_container(hybrid, false)
}

fn stream_source_with_missing_container(hybrid: bool, compressed: bool) -> Vec<u8> {
    let mut source = SOURCE.to_vec();
    source[7] = b'5';
    let xref = source.len();
    // Exact original offsets; this fixture is independent of the product writer.
    let mut offsets = vec![0u64, 9, 120, 177, 261, 387, 0, 0, 456, 583, 0, xref as u64];
    if compressed {
        offsets.push(0);
    }
    let size = offsets.len();
    let mut data = Vec::new();
    for (id, offset) in offsets.into_iter().enumerate() {
        data.push(if compressed && id == 10 {
            2
        } else {
            u8::from(id != 0)
        });
        let location = if compressed && id == 10 { 12 } else { offset };
        data.extend_from_slice(&location.to_be_bytes());
        data.extend_from_slice(&(if id == 0 { 65535u16 } else { 0 }).to_be_bytes());
    }
    source.extend_from_slice(format!("11 0 obj\n<< /Type /XRef /Size {size} /Root 1 0 R /W [1 8 2] /Length {} /Prev 653 >>\nstream\n", data.len()).as_bytes());
    source.extend_from_slice(&data);
    source.extend_from_slice(b"\nendstream\nendobj\n");
    if hybrid {
        let table = source.len();
        source.extend_from_slice(format!("xref\n11 1\n{xref:010} 00000 n \ntrailer\n<< /Size 12 /Root 1 0 R /Prev 653 /XRefStm {xref} >>\nstartxref\n{table}\n%%EOF\n").as_bytes());
    } else {
        source.extend_from_slice(format!("startxref\n{xref}\n%%EOF\n").as_bytes());
    }
    source
}

#[test]
fn table_stream_and_hybrid_preparation_remain_clean_across_revisions() {
    for (name, source) in [
        ("table", SOURCE.to_vec()),
        ("stream", stream_source(false)),
        ("hybrid", stream_source(true)),
    ] {
        record(&format!("{name}-source.pdf"), &source);
        let editor = IncrementalTaggedPdfEditor::new(&source)
            .with_recovered_indexes()
            .unwrap();
        let prepared = editor.apply(&[]).unwrap();
        assert!(prepared.pdf_bytes.starts_with(&source));
        assert_retired(&prepared.pdf_bytes, &[10]);
        record(&format!("{name}-prepared.pdf"), &prepared.pdf_bytes);
        let repeated_editor = IncrementalTaggedPdfEditor::new(&prepared.pdf_bytes)
            .with_recovered_indexes()
            .unwrap();
        assert!(
            !repeated_editor
                .preparation_objects()
                .contains(&(10, 0).into()),
            "retirement is not repeated"
        );
        let repeated = repeated_editor.apply(&[]).unwrap();
        assert!(repeated.pdf_bytes.starts_with(&prepared.pdf_bytes));
        assert_retired(&repeated.pdf_bytes, &[10]);
        record(&format!("{name}-repeated.pdf"), &repeated.pdf_bytes);
    }
}

#[test]
fn trailer_aliases_and_nested_references_cannot_hide_live_corruption() {
    for id in [6, 10] {
        for generation in [0, 1] {
            let source = append_entry(
                SOURCE,
                12,
                0,
                Some(format!("<< /Private [{id} {generation} R] >>").as_bytes()),
                "/Info 12 0 R",
            );
            let error = IncrementalTaggedPdfEditor::new(&source)
                .with_recovered_indexes()
                .err()
                .expect("trailer aliases are not derived-index root edges");
            assert!(
                error.to_string().contains(&format!("{id} {generation}")),
                "{error}"
            );
        }
    }
}

#[test]
fn latest_live_definition_and_nonzero_corruption_are_not_retired() {
    let source = append_entry(
        SOURCE,
        10,
        4,
        Some(b"<< /Value (keep orphan object) >>"),
        "",
    );
    let editor = IncrementalTaggedPdfEditor::new(&source)
        .with_recovered_indexes()
        .unwrap();
    assert!(!editor
        .preparation_objects()
        .iter()
        .any(|id| id.object_number == 10));
    let prepared = editor.apply(&[]).unwrap();
    let mut after = PdfReader::new(Cursor::new(&prepared.pdf_bytes)).unwrap();
    let mut before = PdfReader::new(Cursor::new(&source)).unwrap();
    assert_eq!(
        after.get_object(10, 4).unwrap(),
        before.get_object(10, 4).unwrap()
    );
    record("latest-definition-prepared.pdf", &prepared.pdf_bytes);

    // A nonzero invalid entry is outside this narrowly defined recovery.
    let source = append_entry(SOURCE, 10, 0, None, "");
    let source = String::from_utf8(source)
        .unwrap()
        .replace("10 1\n0000000000", "10 1\n0000000001");
    let error = IncrementalTaggedPdfEditor::new(source.as_bytes())
        .with_recovered_indexes()
        .err()
        .expect("nonzero corrupt offsets remain rejected");
    assert!(
        error.to_string().contains("inspect signature object"),
        "{error}"
    );
}

#[test]
fn nonzero_generation_absent_entry_is_permanently_retired_and_reported_by_source_id() {
    for generation in [1, 65534, 65535] {
        let source = append_entry(SOURCE, 10, generation, None, "");
        let editor = IncrementalTaggedPdfEditor::new(&source)
            .with_recovered_indexes()
            .unwrap();
        assert!(editor
            .preparation_objects()
            .contains(&(10, generation).into()));
        let prepared = editor.apply(&[]).unwrap();
        assert_retired(&prepared.pdf_bytes, &[10]);
    }
}

#[test]
fn nested_source_and_each_split_retire_all_four_absent_entries() {
    use oxidize_pdf::operations::{
        existing_document::{plan_split_pdf, split_pdf},
        ExistingDocumentPolicy, PageRange,
    };
    let source = fixture::fixture(false, true, true);
    let prepared = IncrementalTaggedPdfEditor::new(&source)
        .with_recovered_indexes()
        .unwrap()
        .apply(&[])
        .unwrap();
    assert_retired(&prepared.pdf_bytes, &[9, 26, 27, 28]);
    record("nested-source.pdf", &source);
    record("nested-prepared.pdf", &prepared.pdf_bytes);
    let mut before = PdfReader::new(Cursor::new(&source)).unwrap();
    let mut after = PdfReader::new(Cursor::new(&prepared.pdf_bytes)).unwrap();
    for id in [2, 22, 23, 30, 41, 1001, 1553] {
        assert_eq!(
            before.get_object(id, 0).unwrap(),
            after.get_object(id, 0).unwrap(),
            "source object {id}"
        );
    }
    for (name, input) in [("direct", &source), ("prepared", &prepared.pdf_bytes)] {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("source.pdf");
        std::fs::write(&path, input).unwrap();
        let outputs: Vec<_> = (0..3)
            .map(|i| dir.path().join(format!("part{i}.pdf")))
            .collect();
        let ranges = [
            PageRange::List(vec![0, 1, 2]),
            PageRange::List(vec![3, 4, 5]),
            PageRange::List(vec![6, 7, 8, 9, 10, 11]),
        ];
        let plan = plan_split_pdf(&path, &ranges, ExistingDocumentPolicy::preserve_base()).unwrap();
        let report = split_pdf(
            &path,
            &ranges,
            &outputs,
            ExistingDocumentPolicy::preserve_base(),
        )
        .unwrap();
        assert_eq!(plan, report);
        let source_text = PdfReader::open_document(&path)
            .unwrap()
            .extract_text()
            .unwrap();
        for (index, (start, end)) in [(0, 3), (3, 6), (6, 12)].into_iter().enumerate() {
            let bytes = std::fs::read(&outputs[index]).unwrap();
            assert!(bytes.starts_with(input));
            assert_retired(&bytes, &[9, 26, 27, 28]);
            let text = PdfReader::open_document(&outputs[index])
                .unwrap()
                .extract_text()
                .unwrap();
            assert_eq!(text.len(), end - start);
            for (a, b) in text.iter().zip(&source_text[start..end]) {
                assert_eq!(a.text, b.text);
            }
            record(&format!("nested-{name}-part{index}.pdf"), &bytes);
        }
        assert_eq!(std::fs::read(path).unwrap(), *input);
    }
}

#[test]
fn missing_object_stream_container_is_not_treated_as_an_unused_object() {
    let source = stream_source_with_missing_container(false, true);
    let error = IncrementalTaggedPdfEditor::new(&source)
        .with_recovered_indexes()
        .err()
        .expect("an absent object stream has unresolved members");
    assert!(
        error.to_string().contains("missing object stream"),
        "{error}"
    );
}

#[test]
fn retirement_preserves_an_existing_free_list() {
    let mut source = SOURCE.to_vec();
    let xref = source.len();
    source.extend_from_slice(format!("xref\n0 1\n0000000011 65535 f \n11 1\n0000000000 00007 f \ntrailer\n<< /Size 12 /Root 1 0 R /Prev 653 >>\nstartxref\n{xref}\n%%EOF\n").as_bytes());
    let prepared = IncrementalTaggedPdfEditor::new(&source)
        .with_recovered_indexes()
        .unwrap()
        .apply(&[])
        .unwrap();
    assert_retired(&prepared.pdf_bytes, &[10]);
    let table = XRefTable::parse_with_options(
        &mut BufReader::new(Cursor::new(&prepared.pdf_bytes)),
        &ParseOptions::strict(),
    )
    .unwrap();
    assert_eq!(table.get_entry(0).unwrap().offset, 11);
    assert_eq!(table.get_entry(11).unwrap().offset, 0);
    assert_eq!(table.get_entry(11).unwrap().generation, 7);
    assert!(!table.get_entry(11).unwrap().in_use);
    record("free-list-prepared.pdf", &prepared.pdf_bytes);
}
