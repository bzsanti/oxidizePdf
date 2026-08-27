use oxidize_pdf::structure::{StandardStructureType, StructTree, StructureElement};
use oxidize_pdf::verification::tagged_pdf::{
    validate_tagged_pdf, TaggedPdfFindingCode, TaggedPdfValidationOptions,
};
use oxidize_pdf::{Document, Page};

#[test]
fn validates_a_tagged_pdf_created_by_the_public_writer() {
    let mut page = Page::a4();
    let mcid = page.begin_marked_content("P").unwrap();
    page.end_marked_content().unwrap();

    let mut tree = StructTree::new();
    let document = tree.set_root(StructureElement::new(StandardStructureType::Document));
    let mut paragraph = StructureElement::new(StandardStructureType::P);
    paragraph.add_mcid(0, mcid);
    tree.add_child(document, paragraph).unwrap();

    let mut document = Document::new();
    document.add_page(page);
    document.set_struct_tree(tree);
    let bytes = document.to_bytes().unwrap();

    let report = validate_tagged_pdf(&bytes, &TaggedPdfValidationOptions::default()).unwrap();
    assert!(report.tagged);
    assert!(report.valid, "{:?}", report.findings);
    assert_eq!(report.elements.len(), 2);
    assert_eq!(report.parent_tree_entries, 1);
    let root = report
        .elements
        .iter()
        .find(|element| element.structure_type.as_deref() == Some("Document"))
        .unwrap();
    let paragraph = report
        .elements
        .iter()
        .find(|element| element.structure_type.as_deref() == Some("P"))
        .unwrap();
    assert_eq!(root.marked_content_count, 0);
    assert_eq!(paragraph.marked_content_count, 1);
}

#[test]
fn exposes_stable_machine_readable_findings() {
    // This is deliberately a malformed tagged PDF assembled in the unit tests;
    // public callers should be able to branch on codes without parsing messages.
    let mut document = Document::new();
    document.add_page(Page::a4());
    let bytes = document.to_bytes().unwrap();
    let report = validate_tagged_pdf(&bytes, &Default::default()).unwrap();

    assert_eq!(report.findings.len(), 1);
    assert_eq!(
        report.findings[0].code,
        TaggedPdfFindingCode::MissingStructureTree
    );
}
