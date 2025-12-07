//! Basic Tagged PDF Example with Full Marked Content
//!
//! This example demonstrates how to create a fully accessible PDF using Tagged PDF
//! with marked content operators. The PDF includes:
//!
//! - ✅ Structure tree hierarchy
//! - ✅ Parent references (ISO 32000-1 §14.7.2)
//! - ✅ Marked content operators (BDC/EMC)
//! - ✅ MCIDs connecting structure to content
//! - ✅ Screen reader compatible
//!
//! # Features Demonstrated
//!
//! - Creating a structure tree with Document root
//! - Adding headings (H1, H2) with language attributes and MCIDs
//! - Adding paragraphs with ActualText and MCIDs
//! - Adding lists with structure and MCIDs
//! - Using `Page::begin_marked_content()` and `Page::end_marked_content()`
//! - Connecting MCIDs to structure elements

use oxidize_pdf::{
    structure::{StandardStructureType, StructTree, StructureElement},
    text::Font,
    Document, Page,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Creating Tagged PDF with accessibility structure...");

    // Create a new document
    let mut doc = Document::new();
    doc.set_title("Accessible Document Example");
    doc.set_author("oxidize-pdf");
    doc.set_subject("Tagged PDF Demonstration");

    // Create structure tree FIRST (before adding content)
    let mut tree = StructTree::new();

    // Create document root element
    let doc_elem = StructureElement::new(StandardStructureType::Document)
        .with_id("doc1")
        .with_language("en-US");
    let doc_idx = tree.set_root(doc_elem);

    // Add a page
    let mut page = Page::a4();

    // H1 with marked content
    let mcid_h1 = page.begin_marked_content("H1")?;
    page.text()
        .set_font(Font::HelveticaBold, 24.0)
        .at(50.0, 750.0)
        .write("Welcome to Tagged PDF")?;
    page.end_marked_content()?;

    let mut h1 = StructureElement::new(StandardStructureType::H1)
        .with_id("h1")
        .with_language("en-US")
        .with_actual_text("Welcome to Tagged PDF");
    h1.add_mcid(0, mcid_h1);
    tree.add_child(doc_idx, h1)?;

    // Intro paragraph with marked content
    let mcid_intro = page.begin_marked_content("P")?;
    page.text()
        .set_font(Font::Helvetica, 12.0)
        .at(50.0, 720.0)
        .write("This document has semantic structure for accessibility.")?;
    page.end_marked_content()?;

    let mut intro_para = StructureElement::new(StandardStructureType::P)
        .with_id("intro")
        .with_actual_text("This document has semantic structure for accessibility.");
    intro_para.add_mcid(0, mcid_intro);
    tree.add_child(doc_idx, intro_para)?;

    // H2 Introduction with marked content
    let mcid_h2_intro = page.begin_marked_content("H2")?;
    page.text()
        .set_font(Font::HelveticaBold, 16.0)
        .at(50.0, 680.0)
        .write("Introduction")?;
    page.end_marked_content()?;

    let mut h2_intro = StructureElement::new(StandardStructureType::H2)
        .with_id("h2-intro")
        .with_actual_text("Introduction");
    h2_intro.add_mcid(0, mcid_h2_intro);
    tree.add_child(doc_idx, h2_intro)?;

    // Section paragraph with marked content
    let mcid_intro_content = page.begin_marked_content("P")?;
    page.text()
        .set_font(Font::Helvetica, 12.0)
        .at(50.0, 650.0)
        .write("Tagged PDFs make documents accessible to screen readers.")?;
    page.end_marked_content()?;

    let mut intro_content = StructureElement::new(StandardStructureType::P)
        .with_id("intro-content")
        .with_actual_text("Tagged PDFs make documents accessible to screen readers.");
    intro_content.add_mcid(0, mcid_intro_content);
    tree.add_child(doc_idx, intro_content)?;

    // H2 Benefits with marked content
    let mcid_h2_benefits = page.begin_marked_content("H2")?;
    page.text()
        .set_font(Font::HelveticaBold, 16.0)
        .at(50.0, 610.0)
        .write("Benefits")?;
    page.end_marked_content()?;

    let mut h2_benefits = StructureElement::new(StandardStructureType::H2)
        .with_id("h2-benefits")
        .with_actual_text("Benefits");
    h2_benefits.add_mcid(0, mcid_h2_benefits);
    tree.add_child(doc_idx, h2_benefits)?;

    // Benefits list
    let benefits_list = StructureElement::new(StandardStructureType::L).with_id("benefits-list");
    let list_idx = tree.add_child(doc_idx, benefits_list)?;

    // List item 1 with marked content
    let mcid_li1 = page.begin_marked_content("LI")?;
    page.text()
        .set_font(Font::Helvetica, 12.0)
        .at(70.0, 580.0)
        .write("- Better accessibility for users with disabilities")?;
    page.end_marked_content()?;

    let mut li1 = StructureElement::new(StandardStructureType::LI)
        .with_id("li1")
        .with_actual_text("Better accessibility for users with disabilities");
    li1.add_mcid(0, mcid_li1);
    tree.add_child(list_idx, li1)?;

    // List item 2 with marked content
    let mcid_li2 = page.begin_marked_content("LI")?;
    page.text()
        .set_font(Font::Helvetica, 12.0)
        .at(70.0, 560.0)
        .write("- Improved content extraction")?;
    page.end_marked_content()?;

    let mut li2 = StructureElement::new(StandardStructureType::LI)
        .with_id("li2")
        .with_actual_text("Improved content extraction");
    li2.add_mcid(0, mcid_li2);
    tree.add_child(list_idx, li2)?;

    // List item 3 with marked content
    let mcid_li3 = page.begin_marked_content("LI")?;
    page.text()
        .set_font(Font::Helvetica, 12.0)
        .at(70.0, 540.0)
        .write("- Enhanced document reflow on mobile devices")?;
    page.end_marked_content()?;

    let mut li3 = StructureElement::new(StandardStructureType::LI)
        .with_id("li3")
        .with_actual_text("Enhanced document reflow on mobile devices");
    li3.add_mcid(0, mcid_li3);
    tree.add_child(list_idx, li3)?;

    // Add page to document
    doc.add_page(page);

    // Attach structure tree to document
    doc.set_struct_tree(tree);

    // Save the PDF
    let output_path = "examples/results/tagged_pdf_basic.pdf";
    doc.save(output_path)?;

    println!("✅ Tagged PDF created successfully!");
    println!("📁 Saved to: {}", output_path);
    println!("\n📊 Structure Information:");
    println!("   - Document root with language: en-US");
    println!("   - 1 H1 heading");
    println!("   - 2 H2 headings");
    println!("   - 2 paragraphs");
    println!("   - 1 list with 3 items");
    println!("   - Total: 10 structure elements");
    println!("\n✅ Accessibility Features:");
    println!("   - ✅ Structure tree with parent references");
    println!("   - ✅ Marked content operators (BMC/BDC/EMC)");
    println!("   - ✅ MCIDs connecting structure to content");
    println!("   - ✅ Screen reader compatible structure");
    println!("   - ✅ All {} elements marked with MCIDs", 8);
    println!("\n♿ This PDF should now work with:");
    println!("   - Adobe Acrobat Reader's Read Out Loud");
    println!("   - NVDA and JAWS screen readers");
    println!("   - PDF accessibility checkers (PAC)");
    println!("\n🔮 Future improvements (v1.5.0+):");
    println!("   - Automatic MCID assignment");
    println!("   - ParentTree for multi-page documents");
    println!("   - PDF/UA full compliance validation");

    // Print structure tree summary
    if let Some(struct_tree) = doc.struct_tree() {
        println!("\n📋 Structure Tree Summary:");
        println!("   - Total elements: {}", struct_tree.len());
        println!("   - Root index: {:?}", struct_tree.root_index());
        if let Some(root) = struct_tree.root() {
            println!("   - Root type: {:?}", root.structure_type);
            println!("   - Root children: {}", root.children.len());
        }
    }

    Ok(())
}
