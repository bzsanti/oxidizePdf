//! Generates a copy/paste diagnostic matrix for issue #498.

use oxidize_pdf::{
    structure::{StandardStructureType, StructTree, StructureElement},
    text::Font,
    Document, Page, PdfError,
};

const OUTPUT: &str = "oxidize-pdf-core/tests/fixtures/issue_498_actual_text_diagnostic.pdf";

fn write_label(page: &mut Page, y: f64, label: &str) -> Result<(), PdfError> {
    page.text()
        .set_font(Font::Helvetica, 11.0)
        .at(72.0, y)
        .write(label)?;
    Ok(())
}

fn write_visual(page: &mut Page, y: f64, visual: &str) -> Result<(), PdfError> {
    page.text()
        .set_font(Font::Helvetica, 18.0)
        .at(300.0, y)
        .write(visual)?;
    Ok(())
}

fn main() -> Result<(), PdfError> {
    let mut page = Page::a4();
    let mut tree = StructTree::new();
    let root = tree.set_root(StructureElement::new(StandardStructureType::Document));

    write_label(&mut page, 740.0, "Inline ASCII (expect INLINE_ASCII):")?;
    let inline_mcid = page.begin_marked_content_with_actual_text("Span", "INLINE_ASCII")?;
    write_visual(&mut page, 740.0, "VISUAL_A")?;
    page.end_marked_content()?;
    let mut inline_span = StructureElement::new(StandardStructureType::Span);
    inline_span.add_mcid(0, inline_mcid);
    tree.add_child(root, inline_span)
        .map_err(PdfError::InvalidStructure)?;

    write_label(&mut page, 700.0, "Structural ASCII (expect STRUCT_ASCII):")?;
    let structural_mcid = page.begin_marked_content("Span")?;
    write_visual(&mut page, 700.0, "VISUAL_B")?;
    page.end_marked_content()?;
    let mut structural_span =
        StructureElement::new(StandardStructureType::Span).with_actual_text("STRUCT_ASCII");
    structural_span.add_mcid(0, structural_mcid);
    tree.add_child(root, structural_span)
        .map_err(PdfError::InvalidStructure)?;

    write_label(
        &mut page,
        660.0,
        "Inline + structural Unicode (expect superscripts):",
    )?;
    let logical = "2⁴⁰ E = mc² Aⁿ⁺¹B";
    let combined_mcid = page.begin_marked_content_with_actual_text("Span", logical)?;
    write_visual(&mut page, 660.0, "VISUAL_C")?;
    page.end_marked_content()?;
    let mut combined_span =
        StructureElement::new(StandardStructureType::Span).with_actual_text(logical);
    combined_span.add_mcid(0, combined_mcid);
    tree.add_child(root, combined_span)
        .map_err(PdfError::InvalidStructure)?;

    let mut document = Document::new();
    document.set_title("ActualText diagnostic matrix");
    document.add_page(page);
    document.set_struct_tree(tree);
    document.save(OUTPUT)?;
    println!("wrote {OUTPUT}");
    Ok(())
}
