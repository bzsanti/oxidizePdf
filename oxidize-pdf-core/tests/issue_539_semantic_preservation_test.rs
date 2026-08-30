use oxidize_pdf::operations::{
    extract_pdf_pages_lossless, merge_pdfs_lossless, plan_extract_pdf_pages_lossless,
    plan_merge_pdfs_lossless, split_pdf_lossless, DocumentStructure, LosslessMergeInput, PageRange,
    StructureDisposition,
};
use oxidize_pdf::page_labels::{PageLabel, PageLabelTree};
use oxidize_pdf::parser::PdfReader;
use oxidize_pdf::text::TextExtractor;
use oxidize_pdf::{Document, Page};
use std::fs;

fn write_pdf(path: &std::path::Path, pages: usize) {
    let mut document = Document::new();
    for index in 0..pages {
        let mut page = Page::a4();
        page.text()
            .at(40.0, 750.0)
            .write(&format!("page {}", index + 1))
            .unwrap();
        document.add_page(page);
    }
    document.save(path).unwrap();
}

fn page_texts(path: &std::path::Path) -> Vec<String> {
    let document = PdfReader::open_document(path).unwrap();
    let count = document.page_count().unwrap();
    let mut extractor = TextExtractor::new();
    (0..count)
        .map(|page| {
            extractor
                .extract_from_page(&document, page)
                .unwrap()
                .text
                .trim()
                .to_string()
        })
        .collect()
}

#[test]
fn sparse_extraction_is_planned_and_keeps_the_source_prefix() {
    let directory = tempfile::tempdir().unwrap();
    let source = directory.path().join("source.pdf");
    let output = directory.path().join("output.pdf");
    write_pdf(&source, 3);

    let planned = plan_extract_pdf_pages_lossless(&source, &[2, 0]).unwrap();
    let written = extract_pdf_pages_lossless(&source, &output, &[2, 0]).unwrap();

    assert_eq!(planned, written);
    assert_eq!(written.mutation.page_count, 2);
    assert!(fs::read(&output)
        .unwrap()
        .starts_with(&fs::read(&source).unwrap()));
    assert_eq!(PdfReader::open(&output).unwrap().page_count().unwrap(), 2);
    assert_eq!(page_texts(&output), ["page 3", "page 1"]);
}

#[test]
fn merge_preserves_base_and_imports_self_contained_pages() {
    let directory = tempfile::tempdir().unwrap();
    let first = directory.path().join("first.pdf");
    let second = directory.path().join("second.pdf");
    let output = directory.path().join("merged.pdf");
    write_pdf(&first, 2);
    write_pdf(&second, 2);
    let inputs = vec![
        LosslessMergeInput::new(&first),
        LosslessMergeInput::new(&second),
    ];

    let planned = plan_merge_pdfs_lossless(&inputs).unwrap();
    let written = merge_pdfs_lossless(&inputs, &output).unwrap();

    assert_eq!(planned, written);
    assert_eq!(written.mutation.page_count, 4);
    assert!(written.inputs[1].structures.iter().any(|entry| {
        entry.structure == DocumentStructure::MetadataStream
            && entry.disposition == StructureDisposition::FirstInputWins
    }));
    assert!(fs::read(&output)
        .unwrap()
        .starts_with(&fs::read(&first).unwrap()));
    assert_eq!(PdfReader::open(&output).unwrap().page_count().unwrap(), 4);
    assert_eq!(
        page_texts(&output),
        ["page 1", "page 2", "page 1", "page 2"]
    );
}

#[test]
fn split_plans_every_output_before_atomic_publication() {
    let directory = tempfile::tempdir().unwrap();
    let source = directory.path().join("source.pdf");
    let first = directory.path().join("part-a.pdf");
    let second = directory.path().join("part-b.pdf");
    write_pdf(&source, 3);

    let reports = split_pdf_lossless(
        &source,
        &[PageRange::List(vec![0, 2]), PageRange::Single(1)],
        &[first.clone(), second.clone()],
    )
    .unwrap();

    assert_eq!(reports.len(), 2);
    assert_eq!(PdfReader::open(first).unwrap().page_count().unwrap(), 2);
    assert_eq!(PdfReader::open(second).unwrap().page_count().unwrap(), 1);
}

#[test]
fn extraction_honors_requested_order_and_duplicates() {
    let directory = tempfile::tempdir().unwrap();
    let source = directory.path().join("source.pdf");
    let output = directory.path().join("selected.pdf");
    write_pdf(&source, 3);

    let report = extract_pdf_pages_lossless(&source, &output, &[2, 0, 2]).unwrap();

    assert_eq!(report.mutation.page_count, 3);
    assert!(!report.mutation.added_objects.is_empty());
    assert_eq!(PdfReader::open(&output).unwrap().page_count().unwrap(), 3);
    assert_eq!(page_texts(&output), ["page 3", "page 1", "page 3"]);
}

#[test]
fn split_rejects_duplicate_and_input_alias_paths_before_writing() {
    let directory = tempfile::tempdir().unwrap();
    let source = directory.path().join("source.pdf");
    let output = directory.path().join("part.pdf");
    write_pdf(&source, 2);
    let ranges = [PageRange::Single(0), PageRange::Single(1)];

    let duplicate = split_pdf_lossless(&source, &ranges, &[output.clone(), output.clone()]);
    assert!(duplicate.unwrap_err().to_string().contains("duplicate"));
    assert!(!output.exists());

    let alias = split_pdf_lossless(
        &source,
        &[PageRange::Single(0)],
        std::slice::from_ref(&source),
    );
    assert!(alias.unwrap_err().to_string().contains("aliases the input"));
    assert_eq!(page_texts(&source), ["page 1", "page 2"]);
}

#[test]
fn split_stages_every_destination_before_replacing_any_output() {
    let directory = tempfile::tempdir().unwrap();
    let source = directory.path().join("source.pdf");
    let first = directory.path().join("existing.pdf");
    let invalid_second = directory.path().join("directory-output");
    write_pdf(&source, 2);
    fs::write(&first, b"keep me").unwrap();
    fs::create_dir(&invalid_second).unwrap();

    let result = split_pdf_lossless(
        &source,
        &[PageRange::Single(0), PageRange::Single(1)],
        &[first.clone(), invalid_second],
    );

    assert!(result.is_err());
    assert_eq!(fs::read(first).unwrap(), b"keep me");
}

#[test]
fn merge_rejects_secondary_catalog_semantics_before_writing() {
    let directory = tempfile::tempdir().unwrap();
    let first = directory.path().join("first.pdf");
    let second = directory.path().join("labeled.pdf");
    let output = directory.path().join("existing.pdf");
    write_pdf(&first, 1);
    let mut labeled = Document::new();
    labeled.add_page(Page::a4());
    let mut labels = PageLabelTree::new();
    labels.add_range(0, PageLabel::roman_lowercase());
    labeled.set_page_labels(labels);
    labeled.save(&second).unwrap();
    fs::write(&output, b"do not replace").unwrap();
    let inputs = vec![
        LosslessMergeInput::new(&first),
        LosslessMergeInput::new(&second),
    ];

    let error = plan_merge_pdfs_lossless(&inputs).unwrap_err();

    assert!(error.to_string().contains("PageLabels"));
    assert_eq!(fs::read(output).unwrap(), b"do not replace");
}
