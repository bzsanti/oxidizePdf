//! Generates the cross-viewer copy/paste fixture for issue #498.
//!
//! Run from the workspace root:
//!
//! ```text
//! cargo run -p oxidize-pdf --example generate_issue_498_interop_fixture
//! ```

use oxidize_pdf::{
    structure::{StandardStructureType, StructTree, StructureElement},
    text::Font,
    Document, Page, PdfError,
};

const OUTPUT: &str = "oxidize-pdf-core/tests/fixtures/issue_498_actual_text_interop.pdf";
const LOGICAL_TEXT: &str = "2⁴⁰ E = mc² Aⁿ⁺¹B";

fn main() -> Result<(), PdfError> {
    let mut page = Page::a4();
    let mcid = page.begin_marked_content_with_actual_text("Span", LOGICAL_TEXT)?;
    page.text()
        .set_font(Font::Helvetica, 24.0)
        .at(72.0, 720.0)
        .write("2")?;
    page.text()
        .set_font(Font::Helvetica, 14.0)
        .at(86.0, 731.0)
        .write("40")?;
    page.text()
        .set_font(Font::Helvetica, 24.0)
        .at(105.0, 720.0)
        .write(" E = mc")?;
    page.text()
        .set_font(Font::Helvetica, 14.0)
        .at(199.0, 731.0)
        .write("2")?;
    page.text()
        .set_font(Font::Helvetica, 24.0)
        .at(210.0, 720.0)
        .write(" A")?;
    page.text()
        .set_font(Font::Helvetica, 14.0)
        .at(238.0, 731.0)
        .write("n+1")?;
    page.text()
        .set_font(Font::Helvetica, 24.0)
        .at(263.0, 720.0)
        .write("B")?;
    page.end_marked_content()?;

    let mut tree = StructTree::new();
    let root = tree.set_root(StructureElement::new(StandardStructureType::Document));
    let mut span =
        StructureElement::new(StandardStructureType::Span).with_actual_text(LOGICAL_TEXT);
    span.add_mcid(0, mcid);
    tree.add_child(root, span)
        .map_err(PdfError::InvalidStructure)?;

    let mut document = Document::new();
    document.set_title("ActualText cross-viewer interoperability fixture");
    document.add_page(page);
    document.set_struct_tree(tree);
    document.save(OUTPUT)?;

    println!("wrote {OUTPUT}");
    println!("expected copied text: {LOGICAL_TEXT}");
    Ok(())
}
