use oxidize_pdf::parser::objects::{PdfObject, PdfString};
use oxidize_pdf::structure::{StandardStructureType, StructTree, StructureElement};
use oxidize_pdf::verification::tagged_pdf::{
    validate_tagged_pdf, TaggedPdfFindingCode, TaggedPdfValidationOptions,
};
use oxidize_pdf::viewer_preferences::ViewerPreferences;
use oxidize_pdf::writer::{IncrementalTaggedPdfEditor, TaggedPdfMutation};
use oxidize_pdf::{Document, Page};

#[test]
fn validates_a_tagged_pdf_created_by_the_public_writer() {
    let mut page = Page::a4();
    let mcid = page.begin_marked_content("P").unwrap();
    page.end_marked_content().unwrap();

    let mut tree = StructTree::new();
    let document = tree
        .set_root(StructureElement::new(StandardStructureType::Document).with_language("en-US"));
    let mut paragraph = StructureElement::new(StandardStructureType::P);
    paragraph.add_mcid(0, mcid);
    tree.add_child(document, paragraph).unwrap();

    let mut document = Document::new();
    document.add_page(page);
    document.set_struct_tree(tree);
    let bytes = document.to_bytes().unwrap();
    if let Some(path) = std::env::var_os("OXIDIZE_ISSUE_544_PDF") {
        std::fs::write(path, &bytes).unwrap();
    }

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

#[test]
fn dry_run_and_incremental_edit_preserve_unrelated_bytes_and_unknown_entries() {
    let mut page = Page::a4();
    let first = page.begin_marked_content("P").unwrap();
    page.end_marked_content().unwrap();
    let second = page.begin_marked_content("P").unwrap();
    page.end_marked_content().unwrap();

    let mut tree = StructTree::new();
    let root = tree
        .set_root(StructureElement::new(StandardStructureType::Document).with_language("en-US"));
    let mut paragraph = StructureElement::new(StandardStructureType::P);
    paragraph.add_mcid(0, first);
    tree.add_child(root, paragraph).unwrap();
    let mut document = Document::new();
    document.add_page(page);
    document.set_struct_tree(tree);
    let base = document.to_bytes().unwrap();

    let before = validate_tagged_pdf(&base, &Default::default()).unwrap();
    let paragraph = before
        .elements
        .iter()
        .find(|element| element.structure_type.as_deref() == Some("P"))
        .unwrap()
        .object;
    let page = *before.content_mcids.keys().next().unwrap();
    let mutations = vec![
        TaggedPdfMutation::SetElementAttribute {
            element: paragraph,
            key: "OxidizeUnknown".to_string(),
            value: Some(PdfObject::String(PdfString::new(b"preserved".to_vec()))),
        },
        TaggedPdfMutation::AssociateMcid {
            element: paragraph,
            page,
            mcid: second,
        },
    ];

    let editor = IncrementalTaggedPdfEditor::new(&base);
    let plan = editor.plan(&mutations).unwrap();
    assert_eq!(plan.changed_objects.len(), 2);
    let update = editor.apply(&mutations).unwrap();
    assert!(update.pdf_bytes.starts_with(&base));
    assert!(
        update.validation_after.valid,
        "{:?}",
        update.validation_after.findings
    );
    let paragraph = update
        .validation_after
        .elements
        .iter()
        .find(|element| element.object == paragraph)
        .unwrap();
    assert_eq!(paragraph.marked_content_count, 2);
    assert!(paragraph.dictionary.contains_key("OxidizeUnknown"));

    let no_op = TaggedPdfMutation::SetElementAttribute {
        element: paragraph.object,
        key: "OxidizeUnknown".to_string(),
        value: Some(PdfObject::String(PdfString::new(b"preserved".to_vec()))),
    };
    let no_op_update = IncrementalTaggedPdfEditor::new(&update.pdf_bytes)
        .apply(&[no_op])
        .unwrap();
    assert!(no_op_update.plan.changed_objects.is_empty());
    assert_eq!(no_op_update.pdf_bytes, update.pdf_bytes);
}

#[test]
fn cross_checks_missing_figure_alt_with_verapdf_when_available() {
    let Some(verapdf) = std::env::var_os("VERAPDF") else {
        return;
    };
    let mut tree = StructTree::new();
    let root = tree
        .set_root(StructureElement::new(StandardStructureType::Document).with_language("en-US"));
    tree.add_child(root, StructureElement::new(StandardStructureType::Figure))
        .unwrap();
    let mut document = Document::new();
    document.set_title("Issue 544 independent validation fixture");
    document.set_viewer_preferences(ViewerPreferences::new().display_doc_title(true));
    document.add_page(Page::a4());
    document.set_struct_tree(tree);
    let bytes = document.to_bytes().unwrap();
    let local = validate_tagged_pdf(&bytes, &Default::default()).unwrap();
    assert!(local
        .findings
        .iter()
        .any(|finding| finding.code == TaggedPdfFindingCode::MissingAlternateText));

    let path = std::env::temp_dir().join(format!("oxidize-issue-544-{}.pdf", std::process::id()));
    std::fs::write(&path, bytes).unwrap();
    let output = std::process::Command::new(verapdf)
        .args(["--format", "xml", "--flavour", "ua1"])
        .arg(&path)
        .output()
        .unwrap();
    let _ = std::fs::remove_file(path);
    let report = String::from_utf8(output.stdout).unwrap();
    assert!(!output.status.success());
    assert!(report.contains("isCompliant=\"false\""));
    assert!(
        report.to_ascii_lowercase().contains("alternate")
            || report.to_ascii_lowercase().contains("replacement text"),
        "veraPDF did not report the missing Figure alternative: {report}"
    );
}

#[test]
fn round_trips_nested_table_and_figure_structure() {
    let mut tree = StructTree::new();
    let document = tree
        .set_root(StructureElement::new(StandardStructureType::Document).with_language("en-US"));
    let section = tree
        .add_child(document, StructureElement::new(StandardStructureType::Sect))
        .unwrap();
    let table = tree
        .add_child(section, StructureElement::new(StandardStructureType::Table))
        .unwrap();
    let row = tree
        .add_child(table, StructureElement::new(StandardStructureType::TR))
        .unwrap();
    tree.add_child(row, StructureElement::new(StandardStructureType::TH))
        .unwrap();
    tree.add_child(row, StructureElement::new(StandardStructureType::TD))
        .unwrap();
    tree.add_child(
        section,
        StructureElement::new(StandardStructureType::Figure).with_alt_text("Accessible figure"),
    )
    .unwrap();

    let mut document = Document::new();
    document.add_page(Page::a4());
    document.set_struct_tree(tree);
    let bytes = document.to_bytes().unwrap();
    let report = validate_tagged_pdf(&bytes, &Default::default()).unwrap();
    assert!(report.valid, "{:?}", report.findings);
    assert_eq!(report.elements.len(), 7);
    for structure_type in ["Table", "TR", "TH", "TD", "Figure"] {
        assert!(report
            .elements
            .iter()
            .any(|element| element.structure_type.as_deref() == Some(structure_type)));
    }
}
