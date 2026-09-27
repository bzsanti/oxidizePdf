use oxidize_pdf::parser::{PdfDocument, PdfReader};
use oxidize_pdf::{Destination, Document, OutlineItem, OutlineTree, Page, PageDestination};
use std::io::Cursor;

fn item(title: &str) -> OutlineItem {
    OutlineItem::new(title).with_destination(Destination::fit(PageDestination::PageNumber(0)))
}
fn check_round_trip(tree: OutlineTree) {
    let mut source = Document::new();
    source.add_page(Page::a4());
    source.set_outline(tree.clone());
    let bytes = source.to_bytes().unwrap();
    let parsed = PdfDocument::new(PdfReader::new(Cursor::new(bytes)).unwrap());
    let actual = parsed
        .outline()
        .expect("writer must produce valid outline links")
        .unwrap();
    assert_eq!(
        actual, tree,
        "titles, hierarchy, order, destinations and flags must round-trip"
    );
}
#[test]
fn root_siblings_after_descendants_round_trip() {
    let mut parent = item("Parent");
    parent.add_child(item("Child"));
    let mut tree = OutlineTree::new();
    tree.add_item(parent);
    tree.add_item(item("Sibling"));
    check_round_trip(tree);
}
#[test]
fn nested_sibling_branches_with_descendants_round_trip() {
    let mut root = item("Root");
    for name in ["First", "Middle", "Last"] {
        let mut branch = item(name);
        let mut child = item(&format!("{name} child"));
        child.add_child(item(&format!("{name} grandchild")));
        branch.add_child(child);
        branch.add_child(item(&format!("{name} sibling")));
        root.add_child(branch);
    }
    let mut tree = OutlineTree::new();
    tree.add_item(root);
    tree.add_item(item("Final root"));
    check_round_trip(tree);
}
#[test]
fn flat_outline_round_trip() {
    let mut tree = OutlineTree::new();
    for title in ["One", "Two", "Three"] {
        tree.add_item(item(title));
    }
    check_round_trip(tree);
}

#[test]
fn single_closed_branch_preserves_flags_and_child_links() {
    let mut parent = item("Closed").bold();
    parent.open = false;
    parent.add_child(item("Child"));
    let mut tree = OutlineTree::new();
    tree.add_item(parent);
    check_round_trip(tree);
}

#[test]
fn empty_outline_has_no_items() {
    let mut source = Document::new();
    source.add_page(Page::a4());
    source.set_outline(OutlineTree::new());
    let parsed = PdfDocument::new(PdfReader::new(Cursor::new(source.to_bytes().unwrap())).unwrap());
    assert!(parsed
        .outline()
        .unwrap()
        .is_none_or(|tree| tree.items.is_empty()));
}
