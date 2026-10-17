use oxidize_pdf::parser::objects::{PdfArray, PdfDictionary, PdfObject, PdfString};
use oxidize_pdf::parser::PdfReader;
use oxidize_pdf::verification::tagged_pdf::TaggedPdfObjectRef;
use oxidize_pdf::writer::{IncrementalTaggedPdfEditor, TaggedPdfMutation};
use std::io::Cursor;

fn id(n: u32) -> TaggedPdfObjectRef {
    (n, if n == 6 { 7 } else { 0 }).into()
}
fn reference(n: u32) -> PdfObject {
    PdfObject::Reference(n, id(n).generation)
}
fn text(s: &str) -> PdfObject {
    PdfObject::String(PdfString::new(s.as_bytes().to_vec()))
}
fn attr(n: u32, key: &str, value: Option<PdfObject>) -> TaggedPdfMutation {
    TaggedPdfMutation::SetElementAttribute {
        element: id(n),
        key: key.into(),
        value,
    }
}
fn move_to(n: u32, parent: u32, index: Option<usize>) -> TaggedPdfMutation {
    TaggedPdfMutation::ReparentElement {
        element: id(n),
        new_parent: id(parent),
        index,
    }
}
fn entry(value: Option<PdfObject>) -> TaggedPdfMutation {
    TaggedPdfMutation::SetParentTreeEntry { key: 0, value }
}
fn owners() -> PdfObject {
    PdfObject::Array(PdfArray(vec![reference(6)]))
}

// Independently serialized, with a nonzero generation, unknown binary entries,
// two siblings and a second possible parent. No production writer builds input.
fn fixture() -> Vec<u8> {
    fixture_with_permission(None)
}
fn fixture_with_permission(permission: Option<u8>) -> Vec<u8> {
    fixture_with_content(permission, "/CustomP <</MCID 0>> BDC EMC")
}
fn fixture_with_content(permission: Option<u8>, content: &str) -> Vec<u8> {
    let stream = format!(
        "<< /Length {} >>\nstream\n{content}\nendstream",
        content.len()
    );
    let catalog = format!(
        "<< /Type /Catalog /Pages 2 0 R /StructTreeRoot 5 0 R /Lang (en-US) {} >>",
        if permission.is_some() {
            "/Perms << /DocMDP 11 0 R >>"
        } else {
            ""
        }
    );
    let signature = format!("<< /Type /Sig /ByteRange [0 0000000000 0000000000 0000000000] /Contents <00> /Reference [<< /TransformMethod /DocMDP /TransformParams << /Type /TransformParams /P {} /V /1.2 >> >>] >>", permission.unwrap_or(1));
    let mut objects = vec![
        &catalog,
        "<< /Type /Pages /Count 1 /Kids [3 0 R] >>",
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /StructParents 0 /Contents 4 0 R >>",
        &stream,
        "<< /Type /StructTreeRoot /K [8 0 R 9 0 R] /ParentTree 7 0 R /RoleMap << /CustomP /P >> >>",
        "<< /Type /StructElem /S /CustomP /P 8 0 R /Pg 3 0 R /Lang (es) /T (title) /Vendor << /Raw <00ff> /Flag true >> /K << /Type /MCR /Pg 3 0 R /MCID 0 >> >>",
        "<< /Nums [0 [6 7 R]] /Vendor (index) >>",
        "<< /Type /StructElem /S /Sect /P 5 0 R /K [6 7 R 10 0 R] >>",
        "<< /Type /StructElem /S /Sect /P 5 0 R >>",
        "<< /Type /StructElem /S /P /P 8 0 R /T (other) >>",
    ];
    if permission.is_some() {
        objects.push(&signature);
    }
    let count = objects.len() + 1;
    let mut bytes = b"%PDF-1.7\n".to_vec();
    let mut offsets = Vec::new();
    for (i, object) in objects.iter().enumerate() {
        let n = (i + 1) as u32;
        offsets.push(bytes.len());
        bytes.extend_from_slice(
            format!("{n} {} obj\n{object}\nendobj\n", id(n).generation).as_bytes(),
        );
    }
    let start = bytes.len();
    bytes.extend_from_slice(format!("xref\n0 {count}\n0000000000 65535 f \n").as_bytes());
    for (i, offset) in offsets.iter().enumerate() {
        bytes.extend_from_slice(
            format!("{offset:010} {:05} n \n", id((i + 1) as u32).generation).as_bytes(),
        );
    }
    bytes.extend_from_slice(
        format!("trailer\n<< /Size {count} /Root 1 0 R >>\nstartxref\n{start}\n%%EOF\n").as_bytes(),
    );
    if permission.is_some() {
        // Syntactically covered certification definition, not a crypto-valid signature.
        let marker = b"0000000000 0000000000 0000000000";
        let pos = bytes
            .windows(marker.len())
            .position(|w| w == marker)
            .unwrap();
        let contents = bytes
            .windows(b"/Contents <00>".len())
            .position(|w| w == b"/Contents <00>")
            .unwrap()
            + b"/Contents ".len();
        let end = contents + 4;
        let ranges = format!("{contents:010} {end:010} {:010}", bytes.len() - end);
        bytes[pos..pos + marker.len()].copy_from_slice(ranges.as_bytes());
    }
    bytes
}
fn dictionary(bytes: &[u8], n: u32) -> PdfDictionary {
    PdfReader::new(Cursor::new(bytes))
        .unwrap()
        .get_object(n, id(n).generation)
        .unwrap()
        .as_dict()
        .unwrap()
        .clone()
}
fn apply(base: &[u8], mutations: &[TaggedPdfMutation], changed: &[u32]) -> Vec<u8> {
    let editor = IncrementalTaggedPdfEditor::new(base);
    let plan = editor.plan(mutations).unwrap();
    let result = editor.apply(mutations).unwrap();
    assert_eq!(plan, result.plan);
    assert_eq!(
        plan.changed_objects
            .iter()
            .map(|c| c.object)
            .collect::<Vec<_>>(),
        changed.iter().copied().map(id).collect::<Vec<_>>()
    );
    assert!(result.pdf_bytes.starts_with(base));
    assert!(
        result.validation_before.valid,
        "{:?}",
        result.validation_before.findings
    );
    assert!(
        result.validation_after.valid,
        "{:?}",
        result.validation_after.findings
    );
    for n in [1, 2, 3, 4, 5, 6, 7, 8, 9, 10] {
        if !changed.contains(&n) {
            let mut before = PdfReader::new(Cursor::new(base)).unwrap();
            let mut after = PdfReader::new(Cursor::new(&result.pdf_bytes)).unwrap();
            assert_eq!(
                before.get_object(n, id(n).generation).unwrap(),
                after.get_object(n, id(n).generation).unwrap()
            );
        }
    }
    assert_eq!(
        dictionary(base, 6).get("Vendor"),
        dictionary(&result.pdf_bytes, 6).get("Vendor")
    );
    result.pdf_bytes
}

#[test]
fn assignments_and_removals_restore_original_or_absent_values() {
    let base = fixture();
    for values in [
        vec![Some(text("changed")), Some(text("title"))],
        vec![None, Some(text("title"))],
        vec![
            Some(text("changed")),
            Some(text("changed")),
            Some(text("last")),
        ],
    ] {
        let expected = values.last().unwrap().clone();
        let mutations = values
            .into_iter()
            .map(|v| attr(6, "T", v))
            .collect::<Vec<_>>();
        let bytes = apply(&base, &mutations, &[6]);
        assert_eq!(dictionary(&bytes, 6).get("T"), expected.as_ref());
    }
    let bytes = apply(
        &base,
        &[
            attr(6, "Absent", Some(text("new"))),
            attr(6, "Absent", None),
        ],
        &[6],
    );
    assert_eq!(dictionary(&bytes, 6).get("Absent"), None);
}
#[test]
fn independent_attributes_and_elements_do_not_share_state() {
    let base = fixture();
    let bytes = apply(
        &base,
        &[
            attr(6, "T", Some(text("changed"))),
            attr(10, "T", Some(text("new other"))),
            attr(6, "Lang", Some(text("fr"))),
            attr(6, "T", Some(text("title"))),
        ],
        &[6, 10],
    );
    assert_eq!(dictionary(&bytes, 6).get("T"), Some(&text("title")));
    assert_eq!(dictionary(&bytes, 6).get("Lang"), Some(&text("fr")));
    assert_eq!(dictionary(&bytes, 10).get("T"), Some(&text("new other")));
}
#[test]
fn unchanged_batch_is_byte_identical() {
    let base = fixture();
    let bytes = apply(
        &base,
        &[
            attr(6, "T", Some(text("title"))),
            attr(6, "Missing", None),
            entry(Some(owners())),
            move_to(6, 8, None),
        ],
        &[],
    );
    assert_eq!(bytes, base);
}
#[test]
fn parent_tree_remove_restore_and_repeated_assignments() {
    let base = fixture();
    for first in [None, Some(PdfObject::Array(PdfArray(vec![reference(10)])))] {
        let bytes = apply(&base, &[entry(first), entry(Some(owners()))], &[7]);
        assert_eq!(
            dictionary(&bytes, 7).get("Nums"),
            Some(&PdfObject::Array(PdfArray(vec![
                PdfObject::Integer(0),
                owners()
            ])))
        );
        assert_eq!(dictionary(&bytes, 7).get("Vendor"), Some(&text("index")));
    }
    let bytes = apply(
        &base,
        &[
            TaggedPdfMutation::SetParentTreeEntry {
                key: 42,
                value: Some(reference(6)),
            },
            TaggedPdfMutation::SetParentTreeEntry {
                key: 42,
                value: None,
            },
        ],
        &[7],
    );
    assert_eq!(
        dictionary(&bytes, 7).get("Nums"),
        dictionary(&base, 7).get("Nums")
    );
}
#[test]
fn repeated_reparent_uses_current_parent_and_restores_original() {
    let base = fixture();
    let bytes = apply(
        &base,
        &[move_to(6, 9, None), move_to(6, 8, None)],
        &[6, 8, 9],
    );
    assert_eq!(dictionary(&bytes, 6).get("P"), Some(&reference(8)));
    assert_eq!(
        dictionary(&bytes, 8).get("K"),
        Some(&PdfObject::Array(PdfArray(vec![
            reference(10),
            reference(6)
        ])))
    );
    assert_eq!(dictionary(&bytes, 9).get("K"), None);
}
#[test]
fn indices_use_current_children_after_moves_and_creations() {
    let base = fixture();
    let bytes = apply(
        &base,
        &[move_to(6, 9, Some(0)), move_to(10, 9, Some(1))],
        &[6, 8, 9, 10],
    );
    assert_eq!(
        dictionary(&bytes, 9).get("K"),
        Some(&PdfObject::Array(PdfArray(vec![
            reference(6),
            reference(10)
        ])))
    );
    let create = |index| TaggedPdfMutation::CreateElement {
        parent: id(9),
        structure_type: "Span".into(),
        attributes: PdfDictionary::new(),
        index: Some(index),
    };
    let bytes = apply(&base, &[create(0), create(1)], &[9, 11, 12]);
    assert_eq!(
        dictionary(&bytes, 9).get("K"),
        Some(&PdfObject::Array(PdfArray(vec![
            reference(11),
            reference(12)
        ])))
    );
}
#[test]
fn invalid_later_operations_fail_atomically_in_plan_and_apply() {
    let base = fixture();
    let saved = base.clone();
    for mutations in [
        vec![move_to(8, 9, None), move_to(9, 8, None)],
        vec![move_to(6, 9, None), move_to(10, 8, Some(1))],
        vec![
            attr(6, "T", Some(text("changed"))),
            attr(6, "P", Some(reference(9))),
        ],
    ] {
        let editor = IncrementalTaggedPdfEditor::new(&base);
        assert!(editor.plan(&mutations).is_err());
        assert!(editor.apply(&mutations).is_err());
        assert_eq!(base, saved);
        assert_eq!(
            dictionary(&editor.apply(&[]).unwrap().pdf_bytes, 6).get("T"),
            Some(&text("title"))
        );
    }
}
#[test]
#[ignore = "requires independent qpdf executable"]
fn independent_qpdf_observes_restored_value_and_generation() {
    let base = fixture();
    let bytes = apply(
        &base,
        &[
            attr(6, "T", Some(text("changed"))),
            attr(6, "T", Some(text("title"))),
            entry(None),
            entry(Some(owners())),
            move_to(6, 9, None),
            move_to(6, 8, Some(0)),
        ],
        &[6, 7, 8, 9],
    );
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("edited.pdf");
    std::fs::write(&path, bytes).unwrap();
    let checked = std::process::Command::new("qpdf")
        .arg("--check")
        .arg(&path)
        .output()
        .unwrap();
    assert!(checked.status.success(), "{:?}", checked);
    let object = std::process::Command::new("qpdf")
        .arg("--show-object=6,7")
        .arg(&path)
        .output()
        .unwrap();
    assert!(object.status.success());
    let value = String::from_utf8(object.stdout).unwrap();
    assert!(value.contains("/T (title)"), "{value}");
    assert!(value.contains("/Raw <00ff>"), "{value}");
    assert!(!value.contains("changed"), "{value}");
    assert!(value.contains("/P 8 0 R"), "{value}");
    for (number, expected) in [(7, "/Nums [ 0 [ 6 7 R ] ]"), (8, "/K [ 6 7 R 10 0 R ]")] {
        let output = std::process::Command::new("qpdf")
            .arg(format!("--show-object={number}"))
            .arg(&path)
            .output()
            .unwrap();
        assert!(output.status.success());
        let value = String::from_utf8(output.stdout).unwrap();
        assert!(value.contains(expected), "{value}");
    }
}

#[test]
fn docmdp_forbids_even_restoring_batches_without_publishing_bytes() {
    for permission in 1..=3 {
        let base = fixture_with_permission(Some(permission));
        let editor = IncrementalTaggedPdfEditor::new(&base);
        let mutations = [
            attr(6, "T", Some(text("changed"))),
            attr(6, "T", Some(text("title"))),
        ];
        let plan = editor.plan(&mutations);
        assert!(
            matches!(plan, Err(oxidize_pdf::PdfError::PermissionDenied(_))),
            "{plan:?}"
        );
        assert!(matches!(
            editor.apply(&mutations),
            Err(oxidize_pdf::PdfError::PermissionDenied(_))
        ));
        assert_eq!(dictionary(&base, 6).get("T"), Some(&text("title")));
    }
}

#[test]
fn effective_plan_omits_only_noops_at_their_position() {
    let base = fixture();
    let changed = attr(6, "T", Some(text("changed")));
    let restored = attr(6, "T", Some(text("title")));
    let mutations = [
        restored.clone(),
        changed.clone(),
        changed.clone(),
        restored.clone(),
    ];
    assert_eq!(
        IncrementalTaggedPdfEditor::new(&base)
            .plan(&mutations)
            .unwrap()
            .mutations,
        vec![changed, restored]
    );
    let first = move_to(6, 9, None);
    let restore = move_to(6, 8, Some(0));
    let mutations = [first.clone(), first.clone(), restore.clone()];
    assert_eq!(
        IncrementalTaggedPdfEditor::new(&base)
            .plan(&mutations)
            .unwrap()
            .mutations,
        vec![first, restore]
    );
    let bytes = apply(&base, &mutations, &[6, 8, 9]);
    assert_eq!(
        dictionary(&bytes, 8).get("K"),
        dictionary(&base, 8).get("K")
    );
}

#[test]
fn mixed_parent_tree_and_association_operations_observe_batch_state() {
    let base = fixture_with_content(
        None,
        "/CustomP <</MCID 0>> BDC EMC /CustomP <</MCID 1>> BDC EMC",
    );
    let associate = TaggedPdfMutation::AssociateMcid {
        element: id(6),
        page: id(3),
        mcid: 1,
    };
    let extended = PdfObject::Array(PdfArray(vec![reference(6), reference(6)]));
    let editor = IncrementalTaggedPdfEditor::new(&base);
    // The explicit assignment is a no-op only after the association has run.
    let mutations = [associate.clone(), entry(Some(extended.clone()))];
    let result = editor.apply(&mutations).unwrap();
    assert_eq!(result.plan, editor.plan(&mutations).unwrap());
    assert_eq!(result.plan.mutations, vec![associate.clone()]);
    assert!(
        result.validation_after.valid,
        "{:?}",
        result.validation_after.findings
    );
    assert_eq!(result.validation_after.parent_tree.get(&0), Some(&extended));
    // Restoring the old index must be retained, even though that caller-requested
    // state is invalid. Ordinary apply reports it; prepared apply rejects it.
    let mutations = [associate.clone(), entry(Some(owners()))];
    let result = editor.apply(&mutations).unwrap();
    assert_eq!(result.plan.mutations, mutations);
    assert_eq!(result.plan, editor.plan(&mutations).unwrap());
    assert!(!result.validation_after.valid);
    assert_eq!(result.validation_after.parent_tree.get(&0), Some(&owners()));
    assert!(IncrementalTaggedPdfEditor::new(&base)
        .with_recovered_indexes()
        .unwrap()
        .apply(&mutations)
        .is_err());
    for mutations in [
        vec![associate.clone(), associate.clone()],
        vec![entry(Some(extended)), associate],
    ] {
        assert!(editor.plan(&mutations).is_err());
        assert!(editor.apply(&mutations).is_err());
    }
}
