use oxidize_pdf::operations::split as legacy_split;
use oxidize_pdf::operations::{
    extract_page, extract_page_range, extract_page_range_to_file, extract_page_to_file,
    extract_pages, extract_pages_to_file, extract_pdf_pages_lossless, merge_pdf_files, merge_pdfs,
    merge_pdfs_lossless, plan_extract_pdf_pages_lossless, plan_merge_pdfs_lossless,
    plan_split_pdf_lossless, split_into_pages, split_pdf, split_pdf_lossless, LosslessMergeInput,
    MergeInput, MergeOptions, PageExtractionOptions, PageExtractor, PageRange, PdfMerger,
    PdfSplitter, SplitMode, SplitOptions,
};
use oxidize_pdf::operations::{merge as legacy_merge, page_extraction as legacy_extraction};
use oxidize_pdf::parser::PdfReader;
use oxidize_pdf::{Document, Page};

fn write_pdf(path: &std::path::Path, pages: usize) {
    let mut document = Document::new();
    for _ in 0..pages {
        document.add_page(Page::a4());
    }
    document
        .save(path)
        .expect("compatibility fixture must be writable");
}

#[test]
fn every_v4_operation_family_remains_reachable_during_the_preview() {
    let directory = tempfile::tempdir().expect("compatibility tempdir must be creatable");
    let source = directory.path().join("source.pdf");
    let reconstructed_page = directory.path().join("reconstructed-page.pdf");
    let reconstructed_merge = directory.path().join("reconstructed-merge.pdf");
    write_pdf(&source, 2);

    extract_page_to_file(&source, 0, &reconstructed_page)
        .expect("v4 extract_page_to_file must remain callable");
    assert_eq!(
        extract_page(&source, 0)
            .expect("v4 extract_page must remain callable")
            .pages()
            .len(),
        1
    );
    assert_eq!(
        extract_pages(&source, &[1, 0])
            .expect("v4 extract_pages must remain callable")
            .pages()
            .len(),
        2
    );
    assert_eq!(
        extract_page_range(&source, &PageRange::Range(0, 1))
            .expect("v4 extract_page_range must remain callable")
            .pages()
            .len(),
        2
    );
    extract_pages_to_file(
        &source,
        &[1, 0],
        directory.path().join("reconstructed-pages.pdf"),
    )
    .expect("v4 extract_pages_to_file must remain callable");
    extract_page_range_to_file(
        &source,
        &PageRange::Range(0, 1),
        directory.path().join("reconstructed-range.pdf"),
    )
    .expect("v4 extract_page_range_to_file must remain callable");
    let mut extractor = PageExtractor::new(
        PdfReader::open_document(&source).expect("v4 extractor input must open"),
    );
    assert_eq!(
        extractor
            .extract_pages(&[0, 1])
            .expect("v4 PageExtractor must remain callable")
            .pages()
            .len(),
        2
    );
    let mut configured_extractor = legacy_extraction::PageExtractor::with_options(
        PdfReader::open_document(&source).expect("v4 extractor input must open"),
        PageExtractionOptions::default(),
    );
    assert_eq!(
        configured_extractor
            .extract_page(0)
            .expect("v4 module-path extractor must remain callable")
            .pages()
            .len(),
        1
    );
    merge_pdfs(
        vec![MergeInput::new(&source)],
        &reconstructed_merge,
        MergeOptions::default(),
    )
    .expect("v4 merge_pdfs must remain callable");
    merge_pdf_files(
        &[&source],
        directory.path().join("reconstructed-simple-merge.pdf"),
    )
    .expect("v4 merge_pdf_files must remain callable");
    let mut merger = PdfMerger::new(MergeOptions::default());
    merger.add_input(MergeInput::new(&source));
    assert_eq!(
        merger
            .merge()
            .expect("v4 PdfMerger must remain callable")
            .pages()
            .len(),
        2
    );
    let mut module_merger = legacy_merge::PdfMerger::new(legacy_merge::MergeOptions::default());
    module_merger.add_inputs([legacy_merge::MergeInput::new(&source)]);
    module_merger
        .merge_to_file(directory.path().join("module-path-merge.pdf"))
        .expect("v4 module-path merger must remain callable");
    let split_outputs = split_pdf(
        &source,
        SplitOptions {
            mode: SplitMode::ChunkSize(1),
            output_pattern: directory.path().join("legacy_{}.pdf").display().to_string(),
            ..SplitOptions::default()
        },
    )
    .expect("v4 split_pdf must remain callable");
    assert_eq!(split_outputs.len(), 2);
    let single_pattern = directory.path().join("single_{}.pdf").display().to_string();
    assert_eq!(
        split_into_pages(&source, &single_pattern)
            .expect("v4 split_into_pages must remain callable")
            .len(),
        2
    );
    let module_pattern = directory.path().join("module_{}.pdf").display().to_string();
    assert_eq!(
        legacy_split::split_into_pages(&source, &module_pattern)
            .expect("v4 module-path split must remain callable")
            .len(),
        2
    );
    let mut splitter = PdfSplitter::new(
        PdfReader::open_document(&source).expect("v4 splitter input must open"),
        SplitOptions {
            output_pattern: directory.path().join("object_{}.pdf").display().to_string(),
            ..SplitOptions::default()
        },
    );
    assert_eq!(
        splitter
            .split()
            .expect("v4 PdfSplitter must remain callable")
            .len(),
        2
    );

    let preserved_extract = directory.path().join("preserved-extract.pdf");
    let preserved_merge = directory.path().join("preserved-merge.pdf");
    let preserved_split = directory.path().join("preserved-split.pdf");
    let inputs = [LosslessMergeInput::new(&source)];
    assert_eq!(
        plan_extract_pdf_pages_lossless(&source, &[0])
            .expect("v4 lossless extract plan must remain callable"),
        extract_pdf_pages_lossless(&source, &preserved_extract, &[0])
            .expect("v4 lossless extract must remain callable")
    );
    assert_eq!(
        plan_merge_pdfs_lossless(&inputs).expect("v4 lossless merge plan must remain callable"),
        merge_pdfs_lossless(&inputs, &preserved_merge)
            .expect("v4 lossless merge must remain callable")
    );
    assert_eq!(
        plan_split_pdf_lossless(&source, &[PageRange::Single(0)])
            .expect("v4 lossless split plan must remain callable"),
        split_pdf_lossless(&source, &[PageRange::Single(0)], &[preserved_split])
            .expect("v4 lossless split must remain callable")
    );
}
