use oxidize_pdf::operations::existing_document::{
    self as existing, DocumentStructure, ExistingDocumentPolicy, StructureDisposition,
};
use oxidize_pdf::operations::PageRange;
use oxidize_pdf::parser::PdfReader;
use oxidize_pdf::{Document, Page};

fn check_metadata(split: bool, retain: bool) {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("input.pdf");
    let mut source = Document::new();
    source.set_title("source title");
    source.set_author("source author");
    source.set_subject("source subject");
    source.set_keywords("source keywords");
    source.add_page(Page::a4());
    source.add_page(Page::a4());
    source.save(&input).unwrap();
    let before = std::fs::read(&input).unwrap();
    let policy = if retain {
        ExistingDocumentPolicy::reconstruct_with_metadata_from_first()
    } else {
        ExistingDocumentPolicy::reconstruct()
    };
    let outputs = if split {
        vec![dir.path().join("one.pdf"), dir.path().join("two.pdf")]
    } else {
        vec![dir.path().join("extract.pdf")]
    };
    let reports = if split {
        existing::split_pdf(
            &input,
            &[PageRange::List(vec![0]), PageRange::List(vec![1])],
            &outputs,
            policy,
        )
        .unwrap()
    } else {
        vec![existing::extract_pdf_pages(&input, &outputs[0], &[1], policy).unwrap()]
    };
    assert_eq!(reports.len(), outputs.len());
    for (output, report) in outputs.iter().zip(reports) {
        let parsed = PdfReader::open_document(output).unwrap();
        assert_eq!(parsed.page_count().unwrap(), 1);
        let metadata = parsed.metadata().unwrap();
        for (actual, original) in [
            (metadata.title, "source title"),
            (metadata.author, "source author"),
            (metadata.subject, "source subject"),
            (metadata.keywords, "source keywords"),
        ] {
            assert_eq!(
                actual.as_deref(),
                retain.then_some(original),
                "metadata policy must control actual output"
            );
        }
        let expected = if retain {
            StructureDisposition::FirstInputWins
        } else {
            StructureDisposition::Discarded
        };
        assert!(report.inputs[0]
            .structures
            .iter()
            .any(|entry| entry.structure == DocumentStructure::DocumentInfo
                && entry.disposition == expected));
    }
    assert_eq!(std::fs::read(&input).unwrap(), before);
}
#[test]
fn extract_discards_metadata() {
    check_metadata(false, false);
}
#[test]
fn extract_retains_metadata_when_requested() {
    check_metadata(false, true);
}
#[test]
fn split_discards_metadata_in_every_part() {
    check_metadata(true, false);
}
#[test]
fn split_retains_metadata_when_requested() {
    check_metadata(true, true);
}
