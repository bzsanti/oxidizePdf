/// Integration tests for Tagged PDF functionality (ISO 32000-1 Section 14.8)
///
/// These tests verify the complete workflow of creating Tagged PDFs with:
/// - Structure tree
/// - Marked content operators (BMC/BDC/EMC)
/// - MCID assignment
/// - PDF/UA compliance basics
use oxidize_pdf::{
    structure::{StandardStructureType, StructTree, StructureElement},
    text::Font,
    Page,
};

#[test]
fn test_simple_tagged_pdf_structure() {
    // Create a basic tagged PDF structure with one paragraph
    let mut page = Page::a4();

    // Build structure tree
    let mut tree = StructTree::new();
    let doc_idx = tree.set_root(StructureElement::new(StandardStructureType::Document));

    // Create paragraph structure element
    let mut para = StructureElement::new(StandardStructureType::P);

    // Add marked content to page
    let mcid = page
        .begin_marked_content("P")
        .expect("Failed to begin marked content");

    page.text()
        .set_font(Font::Helvetica, 12.0)
        .at(100.0, 700.0)
        .write("Hello, Tagged PDF!")
        .expect("Failed to write text");

    page.end_marked_content()
        .expect("Failed to end marked content");

    // Connect MCID to structure element
    para.add_mcid(0, mcid); // page_index=0, mcid from above

    tree.add_child(doc_idx, para)
        .expect("Failed to add paragraph to tree");

    // Verify structure
    assert_eq!(tree.len(), 2); // Document + P
    assert!(tree.root().is_some());

    // Verify marked content was tracked
    assert_eq!(page.next_mcid(), 1);
    assert_eq!(page.marked_content_depth(), 0);
}

#[test]
fn test_nested_structure_with_marked_content() {
    // Create structure with nested elements (Div > P)
    let mut page = Page::a4();

    // Build structure tree
    let mut tree = StructTree::new();
    let doc_idx = tree.set_root(StructureElement::new(StandardStructureType::Document));

    // Create Div container
    let mut div = StructureElement::new(StandardStructureType::Div);

    // Create paragraph
    let mut para = StructureElement::new(StandardStructureType::P);

    // Add nested marked content to page
    let mcid1 = page
        .begin_marked_content("Div")
        .expect("Failed to begin Div marked content");

    let mcid2 = page
        .begin_marked_content("P")
        .expect("Failed to begin P marked content");

    page.text()
        .set_font(Font::Helvetica, 12.0)
        .at(100.0, 700.0)
        .write("Text inside paragraph")
        .expect("Failed to write text");

    page.end_marked_content()
        .expect("Failed to end P marked content");
    page.end_marked_content()
        .expect("Failed to end Div marked content");

    // Connect MCIDs to structure elements
    div.add_mcid(0, mcid1);
    para.add_mcid(0, mcid2);

    // Build tree structure
    let div_idx = tree.add_child(doc_idx, div).expect("Failed to add Div");
    tree.add_child(div_idx, para)
        .expect("Failed to add paragraph to Div");

    // Verify structure
    assert_eq!(tree.len(), 3); // Document + Div + P
    assert!(tree.root().is_some());

    // Verify marked content depth tracking
    assert_eq!(page.marked_content_depth(), 0); // All closed
}

#[test]
fn test_multiple_paragraphs_with_unique_mcids() {
    // Verify that each marked content section gets a unique MCID
    let mut page = Page::a4();

    // Build structure tree
    let mut tree = StructTree::new();
    let doc_idx = tree.set_root(StructureElement::new(StandardStructureType::Document));

    // Create three paragraphs
    let paragraphs = vec![
        ("First paragraph", 700.0),
        ("Second paragraph", 680.0),
        ("Third paragraph", 660.0),
    ];

    let mut mcids = Vec::new();

    for (text, y_pos) in &paragraphs {
        let mcid = page
            .begin_marked_content("P")
            .expect("Failed to begin marked content");
        mcids.push(mcid);

        page.text()
            .set_font(Font::Helvetica, 12.0)
            .at(100.0, *y_pos)
            .write(text)
            .expect("Failed to write text");

        page.end_marked_content()
            .expect("Failed to end marked content");
    }

    // Verify all MCIDs are unique
    assert_eq!(mcids.len(), 3);
    assert_eq!(mcids[0], 0);
    assert_eq!(mcids[1], 1);
    assert_eq!(mcids[2], 2);

    // Add structure elements
    for mcid in mcids {
        let mut para = StructureElement::new(StandardStructureType::P);
        para.add_mcid(0, mcid);
        tree.add_child(doc_idx, para)
            .expect("Failed to add paragraph");
    }

    assert_eq!(tree.len(), 4); // Document + 3 paragraphs
}

#[test]
fn test_heading_with_marked_content() {
    // Test heading structure types (H1-H6)
    let mut page = Page::a4();

    let mut tree = StructTree::new();
    let doc_idx = tree.set_root(StructureElement::new(StandardStructureType::Document));

    // Add H1 heading
    let mcid = page
        .begin_marked_content("H1")
        .expect("Failed to begin marked content");

    page.text()
        .set_font(Font::HelveticaBold, 18.0)
        .at(100.0, 750.0)
        .write("Main Heading")
        .expect("Failed to write heading");

    page.end_marked_content()
        .expect("Failed to end marked content");

    // Create H1 structure element
    let mut h1 = StructureElement::new(StandardStructureType::H1);
    h1.add_mcid(0, mcid);

    tree.add_child(doc_idx, h1).expect("Failed to add H1");

    assert_eq!(tree.len(), 2); // Document + H1
}

#[test]
fn test_error_unmatched_marked_content() {
    // Test that ending marked content without beginning fails
    let mut page = Page::a4();

    let result = page.end_marked_content();
    assert!(result.is_err());

    let error = result.unwrap_err();
    assert!(error
        .to_string()
        .contains("No marked content sequence to end"));
}

#[test]
fn test_marked_content_depth_tracking() {
    // Test that marked content depth is tracked correctly
    let mut page = Page::a4();

    assert_eq!(page.marked_content_depth(), 0);

    page.begin_marked_content("Div")
        .expect("Failed to begin Div");
    assert_eq!(page.marked_content_depth(), 1);

    page.begin_marked_content("P").expect("Failed to begin P");
    assert_eq!(page.marked_content_depth(), 2);

    page.end_marked_content().expect("Failed to end P");
    assert_eq!(page.marked_content_depth(), 1);

    page.end_marked_content().expect("Failed to end Div");
    assert_eq!(page.marked_content_depth(), 0);
}

#[test]
fn test_next_mcid_tracking() {
    // Test that next_mcid() correctly predicts the next MCID
    let mut page = Page::a4();

    assert_eq!(page.next_mcid(), 0);

    let mcid1 = page
        .begin_marked_content("P")
        .expect("Failed to begin marked content");
    assert_eq!(mcid1, 0);
    assert_eq!(page.next_mcid(), 1);

    page.end_marked_content()
        .expect("Failed to end marked content");

    let mcid2 = page
        .begin_marked_content("P")
        .expect("Failed to begin marked content");
    assert_eq!(mcid2, 1);
    assert_eq!(page.next_mcid(), 2);

    page.end_marked_content()
        .expect("Failed to end marked content");
}

#[test]
fn test_marked_content_operators_tracking() {
    // Test that marked content operators are tracked correctly
    let mut page = Page::a4();

    // Verify initial state
    assert_eq!(page.next_mcid(), 0);
    assert_eq!(page.marked_content_depth(), 0);

    // Begin first marked content
    let mcid1 = page
        .begin_marked_content("P")
        .expect("Failed to begin marked content");
    assert_eq!(mcid1, 0);
    assert_eq!(page.marked_content_depth(), 1);

    page.text()
        .set_font(Font::Helvetica, 12.0)
        .at(100.0, 700.0)
        .write("Tagged text")
        .expect("Failed to write text");

    page.end_marked_content()
        .expect("Failed to end marked content");

    // Verify state after ending
    assert_eq!(page.next_mcid(), 1);
    assert_eq!(page.marked_content_depth(), 0);
}

#[test]
fn test_structure_tree_relationships() {
    // Test parent-child relationships in structure tree
    let mut tree = StructTree::new();
    let doc_idx = tree.set_root(StructureElement::new(StandardStructureType::Document));

    let div = StructureElement::new(StandardStructureType::Div);
    let div_idx = tree.add_child(doc_idx, div).expect("Failed to add Div");

    let mut para1 = StructureElement::new(StandardStructureType::P);
    para1.add_mcid(0, 0);
    tree.add_child(div_idx, para1)
        .expect("Failed to add first paragraph");

    let mut para2 = StructureElement::new(StandardStructureType::P);
    para2.add_mcid(0, 1);
    tree.add_child(div_idx, para2)
        .expect("Failed to add second paragraph");

    // Verify tree structure
    assert_eq!(tree.len(), 4); // Document + Div + 2 paragraphs

    // Verify parent element exists and has children indices
    let div_elem = tree.get(div_idx).expect("Div element not found");
    assert_eq!(div_elem.children.len(), 2);
}

#[test]
fn test_marked_content_with_different_tags() {
    // Test different structure types
    let mut page = Page::a4();
    let _tree = StructTree::new();

    let tags = vec!["H1", "P", "Div", "Span", "Figure"];
    let mut mcid_counter = 0;

    for tag in tags {
        let mcid = page
            .begin_marked_content(tag)
            .expect("Failed to begin marked content");
        assert_eq!(mcid, mcid_counter);
        mcid_counter += 1;

        page.text()
            .set_font(Font::Helvetica, 12.0)
            .at(100.0, 700.0 - (mcid as f64 * 20.0))
            .write(&format!("Content for {tag}"))
            .expect("Failed to write text");

        page.end_marked_content()
            .expect("Failed to end marked content");
    }

    assert_eq!(page.next_mcid(), 5);
    assert_eq!(page.marked_content_depth(), 0);
}
