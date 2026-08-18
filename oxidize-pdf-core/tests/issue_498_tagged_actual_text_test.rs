//! Regression tests for issue #498: tagged `/ActualText` must be connected
//! to the page and structure tree, not emitted as an orphan MCID.

use std::io::Cursor;

use oxidize_pdf::{
    parser::{objects::PdfObject, PdfReader},
    structure::{StandardStructureType, StructTree, StructureElement},
    text::Font,
    Document, Page, PdfError,
};

fn object_ref(object: &PdfObject) -> (u32, u16) {
    match object {
        PdfObject::Reference(id, generation) => (*id, *generation),
        other => panic!("expected indirect reference, got {other:?}"),
    }
}

#[test]
fn actual_text_mcid_is_connected_through_the_parent_tree() {
    let mut page = Page::a4();
    let mcid = page
        .begin_marked_content_with_actual_text("Span", "2⁴⁰")
        .expect("begin ActualText span");
    page.text()
        .set_font(Font::Helvetica, 12.0)
        .at(72.0, 720.0)
        .write("240")
        .expect("write visual fallback");
    page.end_marked_content().expect("end ActualText span");

    let mut tree = StructTree::new();
    let root = tree.set_root(StructureElement::new(StandardStructureType::Document));
    let mut span = StructureElement::new(StandardStructureType::Span).with_actual_text("2⁴⁰");
    span.add_mcid(0, mcid);
    tree.add_child(root, span).expect("attach span");

    let mut document = Document::new();
    document.add_page(page);
    document.set_struct_tree(tree);
    let pdf = document.to_bytes().expect("serialize tagged PDF");

    let mut reader = PdfReader::new(Cursor::new(pdf)).expect("parse generated PDF");
    let catalog = reader.catalog().expect("catalog").clone();

    let pages_ref = object_ref(catalog.get("Pages").expect("catalog /Pages"));
    let pages = reader
        .get_object(pages_ref.0, pages_ref.1)
        .expect("pages object");
    let page_ref = match pages {
        PdfObject::Dictionary(dict) => match dict.get("Kids").expect("pages /Kids") {
            PdfObject::Array(kids) => object_ref(&kids.0[0]),
            other => panic!("expected /Kids array, got {other:?}"),
        },
        other => panic!("expected pages dictionary, got {other:?}"),
    };
    let page_object = reader
        .get_object(page_ref.0, page_ref.1)
        .expect("page object");
    let struct_parents = match page_object {
        PdfObject::Dictionary(dict) => match dict.get("StructParents") {
            Some(PdfObject::Integer(key)) => *key,
            other => panic!("expected page /StructParents integer, got {other:?}"),
        },
        other => panic!("expected page dictionary, got {other:?}"),
    };

    let tree_ref = object_ref(
        catalog
            .get("StructTreeRoot")
            .expect("catalog /StructTreeRoot"),
    );
    let tree_object = reader
        .get_object(tree_ref.0, tree_ref.1)
        .expect("structure tree root");
    let parent_tree_ref = match tree_object {
        PdfObject::Dictionary(dict) => {
            object_ref(dict.get("ParentTree").expect("StructTreeRoot /ParentTree"))
        }
        other => panic!("expected structure tree root dictionary, got {other:?}"),
    };
    let parent_tree = reader
        .get_object(parent_tree_ref.0, parent_tree_ref.1)
        .expect("parent tree");
    let owner_ref = match parent_tree {
        PdfObject::Dictionary(dict) => match dict.get("Nums").expect("ParentTree /Nums") {
            PdfObject::Array(nums) => {
                assert_eq!(nums.0[0], PdfObject::Integer(struct_parents));
                match &nums.0[1] {
                    PdfObject::Array(owners) => object_ref(&owners.0[mcid as usize]),
                    other => panic!("expected parent array, got {other:?}"),
                }
            }
            other => panic!("expected /Nums array, got {other:?}"),
        },
        other => panic!("expected parent tree dictionary, got {other:?}"),
    };

    let owner = reader
        .get_object(owner_ref.0, owner_ref.1)
        .expect("owning structure element");
    match owner {
        PdfObject::Dictionary(dict) => match dict.get("K").expect("StructElem /K") {
            PdfObject::Array(kids) => match &kids.0[0] {
                PdfObject::Dictionary(mcr) => {
                    assert_eq!(object_ref(mcr.get("Pg").expect("MCR /Pg")), page_ref);
                    assert_eq!(mcr.get("MCID"), Some(&PdfObject::Integer(mcid as i64)));
                }
                other => panic!("expected MCR dictionary, got {other:?}"),
            },
            other => panic!("expected StructElem /K array, got {other:?}"),
        },
        other => panic!("expected StructElem dictionary, got {other:?}"),
    }

    let owner = reader
        .get_object(owner_ref.0, owner_ref.1)
        .expect("owning structure element");
    match owner {
        PdfObject::Dictionary(dict) => match dict.get("ActualText") {
            Some(PdfObject::String(value)) => {
                let mut expected = vec![0xFE, 0xFF];
                for unit in "2⁴⁰".encode_utf16() {
                    expected.extend_from_slice(&unit.to_be_bytes());
                }
                assert_eq!(value.as_bytes(), expected.as_slice());
            }
            other => panic!("expected UTF-16BE StructElem /ActualText, got {other:?}"),
        },
        other => panic!("expected StructElem dictionary, got {other:?}"),
    }
}

#[test]
fn rejects_an_mcid_owned_by_multiple_structure_elements() {
    let mut page = Page::a4();
    let mcid = page.begin_marked_content("Span").expect("begin span");
    page.end_marked_content().expect("end span");

    let mut tree = StructTree::new();
    let root = tree.set_root(StructureElement::new(StandardStructureType::Document));
    for _ in 0..2 {
        let mut span = StructureElement::new(StandardStructureType::Span);
        span.add_mcid(0, mcid);
        tree.add_child(root, span).expect("attach span");
    }

    let mut document = Document::new();
    document.add_page(page);
    document.set_struct_tree(tree);

    let error = document
        .to_bytes()
        .expect_err("duplicate MCID ownership must be rejected");
    match error {
        PdfError::InvalidStructure(message) => {
            assert!(message.contains("owned by more than one"), "{message}");
        }
        other => panic!("expected InvalidStructure, got {other:?}"),
    }
}

#[test]
fn rejects_a_structure_reference_to_an_unknown_mcid() {
    let page = Page::a4();
    let mut tree = StructTree::new();
    let root = tree.set_root(StructureElement::new(StandardStructureType::Document));
    let mut span = StructureElement::new(StandardStructureType::Span);
    span.add_mcid(0, 0);
    tree.add_child(root, span).expect("attach span");

    let mut document = Document::new();
    document.add_page(page);
    document.set_struct_tree(tree);

    let error = document
        .to_bytes()
        .expect_err("unknown MCID must be rejected");
    match error {
        PdfError::InvalidStructure(message) => {
            assert!(
                message.contains("only 0 marked-content sequences"),
                "{message}"
            );
        }
        other => panic!("expected InvalidStructure, got {other:?}"),
    }
}

#[test]
fn rejects_a_structure_reference_to_an_unknown_page() {
    let mut page = Page::a4();
    let mcid = page.begin_marked_content("Span").expect("begin span");
    page.end_marked_content().expect("end span");

    let mut tree = StructTree::new();
    let root = tree.set_root(StructureElement::new(StandardStructureType::Document));
    let mut span = StructureElement::new(StandardStructureType::Span);
    span.add_mcid(1, mcid);
    tree.add_child(root, span).expect("attach span");

    let mut document = Document::new();
    document.add_page(page);
    document.set_struct_tree(tree);

    let error = document
        .to_bytes()
        .expect_err("unknown page reference must be rejected");
    match error {
        PdfError::InvalidStructure(message) => {
            assert!(
                message.contains("references page 1 but document has 1 pages"),
                "{message}"
            );
        }
        other => panic!("expected InvalidStructure, got {other:?}"),
    }
}

#[test]
fn parent_tree_maps_multiple_pages_and_preserves_mcid_holes() {
    let mut first_page = Page::a4();
    let unused_mcid = first_page
        .begin_marked_content("Artifact")
        .expect("begin unowned marked content");
    first_page
        .end_marked_content()
        .expect("end unowned marked content");
    let first_owned_mcid = first_page
        .begin_marked_content_with_actual_text("Span", "first")
        .expect("begin first owned span");
    first_page
        .end_marked_content()
        .expect("end first owned span");
    assert_eq!(unused_mcid, 0);
    assert_eq!(first_owned_mcid, 1);

    let untagged_page = Page::a4();

    let mut last_page = Page::a4();
    let last_mcid = last_page
        .begin_marked_content_with_actual_text("Span", "last")
        .expect("begin last span");
    last_page.end_marked_content().expect("end last span");

    let mut tree = StructTree::new();
    let root = tree.set_root(StructureElement::new(StandardStructureType::Document));
    let mut first_span = StructureElement::new(StandardStructureType::Span);
    first_span.add_mcid(0, first_owned_mcid);
    tree.add_child(root, first_span).expect("attach first span");
    let mut last_span = StructureElement::new(StandardStructureType::Span);
    last_span.add_mcid(2, last_mcid);
    tree.add_child(root, last_span).expect("attach last span");

    let mut document = Document::new();
    document.add_page(first_page);
    document.add_page(untagged_page);
    document.add_page(last_page);
    document.set_struct_tree(tree);
    let pdf = document.to_bytes().expect("serialize multipage tagged PDF");

    let mut reader = PdfReader::new(Cursor::new(pdf)).expect("parse generated PDF");
    let catalog = reader.catalog().expect("catalog").clone();
    let pages_ref = object_ref(catalog.get("Pages").expect("catalog /Pages"));
    let page_refs = match reader
        .get_object(pages_ref.0, pages_ref.1)
        .expect("pages object")
    {
        PdfObject::Dictionary(dict) => match dict.get("Kids").expect("pages /Kids") {
            PdfObject::Array(kids) => kids.0.iter().map(object_ref).collect::<Vec<_>>(),
            other => panic!("expected /Kids array, got {other:?}"),
        },
        other => panic!("expected pages dictionary, got {other:?}"),
    };

    let mut keys = Vec::new();
    for (index, page_ref) in page_refs.iter().enumerate() {
        match reader
            .get_object(page_ref.0, page_ref.1)
            .expect("page object")
        {
            PdfObject::Dictionary(dict) => match (index, dict.get("StructParents")) {
                (0 | 2, Some(PdfObject::Integer(key))) => keys.push(*key),
                (1, None) => {}
                (_, other) => panic!("unexpected /StructParents on page {index}: {other:?}"),
            },
            other => panic!("expected page dictionary, got {other:?}"),
        }
    }
    assert_eq!(keys, vec![0, 1]);

    let tree_ref = object_ref(
        catalog
            .get("StructTreeRoot")
            .expect("catalog /StructTreeRoot"),
    );
    let parent_tree_ref = match reader
        .get_object(tree_ref.0, tree_ref.1)
        .expect("structure tree root")
    {
        PdfObject::Dictionary(dict) => {
            assert_eq!(dict.get("ParentTreeNextKey"), Some(&PdfObject::Integer(2)));
            object_ref(dict.get("ParentTree").expect("StructTreeRoot /ParentTree"))
        }
        other => panic!("expected structure tree root dictionary, got {other:?}"),
    };
    let nums = match reader
        .get_object(parent_tree_ref.0, parent_tree_ref.1)
        .expect("parent tree")
    {
        PdfObject::Dictionary(dict) => match dict.get("Nums").expect("ParentTree /Nums") {
            PdfObject::Array(nums) => nums.0.clone(),
            other => panic!("expected /Nums array, got {other:?}"),
        },
        other => panic!("expected parent tree dictionary, got {other:?}"),
    };
    assert_eq!(nums.len(), 4);
    assert_eq!(nums[0], PdfObject::Integer(0));
    match &nums[1] {
        PdfObject::Array(owners) => {
            assert_eq!(owners.0.len(), 2);
            assert_eq!(owners.0[0], PdfObject::Null);
            assert!(matches!(owners.0[1], PdfObject::Reference(_, _)));
        }
        other => panic!("expected first owner array, got {other:?}"),
    }
    assert_eq!(nums[2], PdfObject::Integer(1));
    match &nums[3] {
        PdfObject::Array(owners) => {
            assert_eq!(owners.0.len(), 1);
            assert!(matches!(owners.0[0], PdfObject::Reference(_, _)));
        }
        other => panic!("expected second owner array, got {other:?}"),
    }
}
