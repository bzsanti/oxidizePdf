use oxidize_pdf::operations::existing_document::{
    extract_pdf_pages, merge_pdfs, plan_extract_pdf_pages, plan_merge_pdfs, split_pdf,
    DocumentStructure, ExistingDocumentEngine, ExistingDocumentMergeInput, ExistingDocumentPolicy,
    SecondaryStructurePolicy, StructureDisposition,
};
use oxidize_pdf::operations::PageRange;
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

    let policy = ExistingDocumentPolicy::preserve_base();
    let planned = plan_extract_pdf_pages(&source, &[2, 0], policy).unwrap();
    let written = extract_pdf_pages(&source, &output, &[2, 0], policy).unwrap();

    assert_eq!(planned, written);
    assert_eq!(written.plan.page_count(), 2);
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
        ExistingDocumentMergeInput::new(&first),
        ExistingDocumentMergeInput::new(&second),
    ];

    let policy = ExistingDocumentPolicy::preserve_base();
    let planned = plan_merge_pdfs(&inputs, policy).unwrap();
    let written = merge_pdfs(&inputs, &output, policy).unwrap();

    assert_eq!(planned, written);
    assert_eq!(written.plan.page_count(), 4);
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

    let reports = split_pdf(
        &source,
        &[PageRange::List(vec![0, 2]), PageRange::Single(1)],
        &[first.clone(), second.clone()],
        ExistingDocumentPolicy::preserve_base(),
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

    let report = extract_pdf_pages(
        &source,
        &output,
        &[2, 0, 2],
        ExistingDocumentPolicy::preserve_base(),
    )
    .unwrap();

    assert_eq!(report.plan.page_count(), 3);
    let oxidize_pdf::operations::existing_document::ExistingDocumentExecutionPlan::Incremental(
        mutation,
    ) = &report.plan
    else {
        panic!("preserving extraction must use an incremental plan");
    };
    assert!(!mutation.added_objects.is_empty());
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

    let duplicate = split_pdf(
        &source,
        &ranges,
        &[output.clone(), output.clone()],
        ExistingDocumentPolicy::preserve_base(),
    );
    assert!(duplicate.unwrap_err().to_string().contains("duplicate"));
    assert!(!output.exists());

    let alias = split_pdf(
        &source,
        &[PageRange::Single(0)],
        std::slice::from_ref(&source),
        ExistingDocumentPolicy::preserve_base(),
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

    for policy in [
        ExistingDocumentPolicy::preserve_base(),
        ExistingDocumentPolicy::reconstruct(),
    ] {
        fs::write(&first, b"keep me").unwrap();
        let result = split_pdf(
            &source,
            &[PageRange::Single(0), PageRange::Single(1)],
            &[first.clone(), invalid_second.clone()],
            policy,
        );

        assert!(result.is_err());
        assert_eq!(fs::read(&first).unwrap(), b"keep me");
    }
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
        ExistingDocumentMergeInput::new(&first),
        ExistingDocumentMergeInput::new(&second),
    ];

    let error = plan_merge_pdfs(&inputs, ExistingDocumentPolicy::preserve_base()).unwrap_err();

    assert!(error.to_string().contains("PageLabels"));
    assert_eq!(fs::read(output).unwrap(), b"do not replace");
}

#[test]
fn merge_policy_explicitly_controls_each_secondary_structure() {
    let directory = tempfile::tempdir().unwrap();
    let first = directory.path().join("first.pdf");
    let second = directory.path().join("labeled.pdf");
    write_pdf(&first, 1);
    let mut labeled = Document::new();
    labeled.add_page(Page::a4());
    let mut labels = PageLabelTree::new();
    labels.add_range(0, PageLabel::roman_lowercase());
    labeled.set_page_labels(labels);
    labeled.save(&second).unwrap();
    let inputs = [
        ExistingDocumentMergeInput::new(first),
        ExistingDocumentMergeInput::new(second),
    ];
    let policy = ExistingDocumentPolicy::preserve_base()
        .with_page_labels(SecondaryStructurePolicy::FirstInputWins);

    let report = plan_merge_pdfs(&inputs, policy).unwrap();

    assert!(report.inputs[1].structures.iter().any(|entry| {
        entry.structure == DocumentStructure::PageLabels
            && entry.disposition == StructureDisposition::FirstInputWins
    }));
}

#[test]
fn reconstructive_policy_is_explicit_and_reports_discarded_semantics() {
    let directory = tempfile::tempdir().unwrap();
    let first = directory.path().join("first.pdf");
    let second = directory.path().join("labeled.pdf");
    let output = directory.path().join("merged.pdf");
    write_pdf(&first, 1);
    let mut labeled = Document::new();
    labeled.add_page(Page::a4());
    let mut labels = PageLabelTree::new();
    labels.add_range(0, PageLabel::roman_lowercase());
    labeled.set_page_labels(labels);
    labeled.save(&second).unwrap();
    let inputs = [
        ExistingDocumentMergeInput::new(&first),
        ExistingDocumentMergeInput::new(&second),
    ];

    let policy = ExistingDocumentPolicy::reconstruct();
    let planned = plan_merge_pdfs(&inputs, policy).unwrap();
    let written = merge_pdfs(&inputs, &output, policy).unwrap();

    assert_eq!(planned, written);
    assert_eq!(written.engine, ExistingDocumentEngine::Reconstruct);
    assert!(written.inputs[1].structures.iter().any(|entry| {
        entry.structure == DocumentStructure::PageLabels
            && entry.disposition == StructureDisposition::Discarded
    }));
    assert!(!fs::read(&output)
        .unwrap()
        .starts_with(&fs::read(first).unwrap()));
    assert_eq!(PdfReader::open(output).unwrap().page_count().unwrap(), 2);
}

#[test]
fn reconstructive_extract_and_split_preserve_selected_content_and_order() {
    let directory = tempfile::tempdir().unwrap();
    let source = directory.path().join("source.pdf");
    let extracted = directory.path().join("extracted.pdf");
    let first = directory.path().join("first.pdf");
    let second = directory.path().join("second.pdf");
    write_pdf(&source, 3);

    let extraction = extract_pdf_pages(
        &source,
        &extracted,
        &[2, 0, 2],
        ExistingDocumentPolicy::reconstruct(),
    )
    .unwrap();
    assert_eq!(extraction.plan.page_count(), 3);
    assert_eq!(page_texts(&extracted), ["page 3", "page 1", "page 3"]);

    let reports = split_pdf(
        &source,
        &[PageRange::List(vec![2, 0]), PageRange::Single(1)],
        &[first.clone(), second.clone()],
        ExistingDocumentPolicy::reconstruct(),
    )
    .unwrap();
    assert_eq!(reports.len(), 2);
    assert_eq!(page_texts(&first), ["page 3", "page 1"]);
    assert_eq!(page_texts(&second), ["page 2"]);
}

#[test]
fn merge_and_extract_reject_input_aliases_for_both_engines() {
    let directory = tempfile::tempdir().unwrap();
    let source = directory.path().join("source.pdf");
    write_pdf(&source, 2);
    let original = fs::read(&source).unwrap();
    let inputs = [ExistingDocumentMergeInput::new(&source)];

    for policy in [
        ExistingDocumentPolicy::preserve_base(),
        ExistingDocumentPolicy::reconstruct(),
    ] {
        assert!(merge_pdfs(&inputs, &source, policy)
            .unwrap_err()
            .to_string()
            .contains("aliases input"));
        assert!(extract_pdf_pages(&source, &source, &[0], policy)
            .unwrap_err()
            .to_string()
            .contains("aliases input"));
        assert_eq!(fs::read(&source).unwrap(), original);
    }
}

#[test]
fn reconstructive_failures_do_not_replace_existing_outputs() {
    let directory = tempfile::tempdir().unwrap();
    let source = directory.path().join("source.pdf");
    let output = directory.path().join("existing.pdf");
    write_pdf(&source, 1);
    fs::write(&output, b"keep me").unwrap();

    let result = extract_pdf_pages(
        &source,
        &output,
        &[99],
        ExistingDocumentPolicy::reconstruct(),
    );

    assert!(result.is_err());
    assert_eq!(fs::read(output).unwrap(), b"keep me");
}

#[test]
fn reconstructive_metadata_policy_matches_the_report_and_output() {
    let directory = tempfile::tempdir().unwrap();
    let source = directory.path().join("source.pdf");
    let discarded = directory.path().join("discarded.pdf");
    let retained = directory.path().join("retained.pdf");
    let mut document = Document::new();
    document.set_title("source title");
    document.add_page(Page::a4());
    document.save(&source).unwrap();
    let inputs = [ExistingDocumentMergeInput::new(&source)];

    let discarded_report =
        merge_pdfs(&inputs, &discarded, ExistingDocumentPolicy::reconstruct()).unwrap();
    let retained_report = merge_pdfs(
        &inputs,
        &retained,
        ExistingDocumentPolicy::reconstruct_with_metadata_from_first(),
    )
    .unwrap();

    assert!(discarded_report.inputs[0].structures.iter().any(|entry| {
        entry.structure == DocumentStructure::DocumentInfo
            && entry.disposition == StructureDisposition::Discarded
    }));
    assert!(retained_report.inputs[0].structures.iter().any(|entry| {
        entry.structure == DocumentStructure::DocumentInfo
            && entry.disposition == StructureDisposition::FirstInputWins
    }));
    assert_eq!(
        PdfReader::open_document(&discarded)
            .unwrap()
            .metadata()
            .unwrap()
            .title,
        None
    );
    assert_eq!(
        PdfReader::open_document(&retained)
            .unwrap()
            .metadata()
            .unwrap()
            .title
            .as_deref(),
        Some("source title")
    );
}

#[test]
fn empty_split_is_rejected_consistently_by_plan_and_execution() {
    let directory = tempfile::tempdir().unwrap();
    let source = directory.path().join("source.pdf");
    write_pdf(&source, 1);

    for policy in [
        ExistingDocumentPolicy::preserve_base(),
        ExistingDocumentPolicy::reconstruct(),
    ] {
        let planned =
            oxidize_pdf::operations::existing_document::plan_split_pdf(&source, &[], policy);
        let executed = split_pdf(&source, &[], &[], policy);

        assert!(matches!(
            planned,
            Err(oxidize_pdf::operations::OperationError::NoPagesToProcess)
        ));
        assert!(matches!(
            executed,
            Err(oxidize_pdf::operations::OperationError::NoPagesToProcess)
        ));
    }
}
