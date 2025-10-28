/// Demonstration of Tagged PDF creation with marked content operators
///
/// This example shows how to create a Tagged PDF with:
/// - Structure tree (Document > H1, P elements)
/// - Marked content operators (BDC/EMC)
/// - Automatic MCID assignment
/// - PDF/UA compliance basics
///
/// Tagged PDFs are essential for accessibility (screen readers) and proper
/// document structure for assistive technologies.
use oxidize_pdf::{
    structure::{StandardStructureType, StructTree, StructureElement},
    text::Font,
    Page,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Creating Tagged PDF with marked content...\n");

    // Create a new page
    let mut page = Page::a4();

    // Build structure tree
    println!("Building structure tree:");
    let mut tree = StructTree::new();
    let doc_idx = tree.set_root(StructureElement::new(StandardStructureType::Document));
    println!("  ✓ Document root created (index {})", doc_idx);

    // ===========================================
    // Section 1: Heading with marked content
    // ===========================================
    println!("\nAdding H1 heading with marked content:");
    let mcid_h1 = page
        .begin_marked_content("H1")
        .expect("Failed to begin H1 marked content");
    println!("  ✓ Marked content started (MCID {})", mcid_h1);

    page.text()
        .set_font(Font::HelveticaBold, 24.0)
        .at(100.0, 750.0)
        .write("Tagged PDF Demonstration")?;

    page.end_marked_content()?;
    println!("  ✓ Marked content ended");

    // Create H1 structure element and link to marked content
    let mut h1 = StructureElement::new(StandardStructureType::H1);
    h1.add_mcid(0, mcid_h1); // page_index=0, mcid from above
    tree.add_child(doc_idx, h1)?;
    println!("  ✓ H1 structure element added to tree");

    // ===========================================
    // Section 2: First paragraph
    // ===========================================
    println!("\nAdding first paragraph:");
    let mcid_p1 = page
        .begin_marked_content("P")
        .expect("Failed to begin P marked content");
    println!("  ✓ Marked content started (MCID {})", mcid_p1);

    page.text()
        .set_font(Font::Helvetica, 12.0)
        .at(100.0, 700.0)
        .write("This is a Tagged PDF created with oxidize-pdf. Tagged PDFs")?;

    page.text()
        .set_font(Font::Helvetica, 12.0)
        .at(100.0, 685.0)
        .write("include semantic structure information that makes them accessible")?;

    page.text()
        .set_font(Font::Helvetica, 12.0)
        .at(100.0, 670.0)
        .write("to screen readers and other assistive technologies.")?;

    page.end_marked_content()?;
    println!("  ✓ Marked content ended");

    let mut para1 = StructureElement::new(StandardStructureType::P);
    para1.add_mcid(0, mcid_p1);
    tree.add_child(doc_idx, para1)?;
    println!("  ✓ Paragraph structure element added to tree");

    // ===========================================
    // Section 3: Second paragraph with emphasis
    // ===========================================
    println!("\nAdding second paragraph:");
    let mcid_p2 = page
        .begin_marked_content("P")
        .expect("Failed to begin P marked content");
    println!("  ✓ Marked content started (MCID {})", mcid_p2);

    page.text()
        .set_font(Font::Helvetica, 12.0)
        .at(100.0, 640.0)
        .write("Key features of Tagged PDFs include:")?;

    page.end_marked_content()?;
    println!("  ✓ Marked content ended");

    let mut para2 = StructureElement::new(StandardStructureType::P);
    para2.add_mcid(0, mcid_p2);
    tree.add_child(doc_idx, para2)?;
    println!("  ✓ Paragraph structure element added to tree");

    // ===========================================
    // Section 4: List items (simplified as paragraphs)
    // ===========================================
    let list_items = vec![
        ("• Structure tree defining document hierarchy", 610.0),
        ("• Marked content operators (BDC/EMC) in content streams", 590.0),
        ("• MCID (Marked Content ID) linking content to structure", 570.0),
        ("• Support for semantic elements (H1-H6, P, Div, etc.)", 550.0),
    ];

    println!("\nAdding list items:");
    for (item_text, y_pos) in list_items {
        let mcid = page
            .begin_marked_content("P")
            .expect("Failed to begin marked content");
        println!("  ✓ Item MCID {}: {}", mcid, item_text);

        page.text()
            .set_font(Font::Helvetica, 11.0)
            .at(120.0, y_pos)
            .write(item_text)?;

        page.end_marked_content()?;

        let mut item_para = StructureElement::new(StandardStructureType::P);
        item_para.add_mcid(0, mcid);
        tree.add_child(doc_idx, item_para)?;
    }

    // ===========================================
    // Section 5: Closing paragraph
    // ===========================================
    println!("\nAdding closing paragraph:");
    let mcid_p3 = page
        .begin_marked_content("P")
        .expect("Failed to begin P marked content");
    println!("  ✓ Marked content started (MCID {})", mcid_p3);

    page.text()
        .set_font(Font::Helvetica, 12.0)
        .at(100.0, 510.0)
        .write("Tagged PDFs are essential for PDF/UA (ISO 14289) compliance,")?;

    page.text()
        .set_font(Font::Helvetica, 12.0)
        .at(100.0, 495.0)
        .write("which ensures documents are accessible to people with disabilities.")?;

    page.end_marked_content()?;
    println!("  ✓ Marked content ended");

    let mut para3 = StructureElement::new(StandardStructureType::P);
    para3.add_mcid(0, mcid_p3);
    tree.add_child(doc_idx, para3)?;
    println!("  ✓ Paragraph structure element added to tree");

    // ===========================================
    // Section 6: Footer with metadata
    // ===========================================
    println!("\nAdding footer:");
    let mcid_footer = page
        .begin_marked_content("P")
        .expect("Failed to begin marked content");
    println!("  ✓ Marked content started (MCID {})", mcid_footer);

    page.text()
        .set_font(Font::Courier, 9.0)
        .at(100.0, 50.0)
        .write("Generated with oxidize-pdf - Rust PDF library")?;

    page.end_marked_content()?;
    println!("  ✓ Marked content ended");

    let mut footer = StructureElement::new(StandardStructureType::P);
    footer.add_mcid(0, mcid_footer);
    tree.add_child(doc_idx, footer)?;
    println!("  ✓ Footer structure element added to tree");

    // ===========================================
    // Summary and statistics
    // ===========================================
    println!("\n{}", "=".repeat(60));
    println!("Tagged PDF Statistics:");
    println!("{}", "=".repeat(60));
    println!("Structure elements: {}", tree.len());
    println!("Total MCIDs used: {}", page.next_mcid());
    println!("Marked content depth: {}", page.marked_content_depth());
    println!("Root element: {:?}", tree.root().map(|e| &e.structure_type));
    println!("{}", "=".repeat(60));

    // Create document and write to file
    println!("\nWriting PDF to file...");
    let output_path = "examples/results/tagged_pdf_demo.pdf";

    // Note: This example demonstrates the API, but full PDF writing
    // with structure tree integration requires additional writer support
    // For now, we demonstrate the marked content and structure tree creation

    println!("\n✓ Tagged PDF structure created successfully!");
    println!("  Output would be saved to: {}", output_path);
    println!("\nThis example demonstrates:");
    println!("  ✓ Automatic MCID assignment");
    println!("  ✓ Marked content operators (BDC/EMC)");
    println!("  ✓ Structure tree building");
    println!("  ✓ Semantic structure elements (H1, P)");
    println!("  ✓ PDF/UA compliance basics");

    Ok(())
}
