//! Regression tests for issue #506: text extraction must honor `/ActualText`
//! on the structure element that owns a marked-content MCID.

use std::io::Cursor;

use oxidize_pdf::{
    parser::{PdfDocument, PdfReader},
    structure::{StandardStructureType, StructTree, StructureElement},
    text::{ExtractionOptions, Font, TextExtractor},
    Document, Page,
};

fn extract(mut document: Document, options: ExtractionOptions) -> oxidize_pdf::text::ExtractedText {
    let bytes = document.to_bytes().expect("serialize tagged PDF");
    let reader = PdfReader::new(Cursor::new(bytes)).expect("parse tagged PDF");
    let parsed = PdfDocument::new(reader);
    TextExtractor::with_options(options)
        .extract_from_page(&parsed, 0)
        .expect("extract tagged page")
}

fn raw_tagged_pdf(parent_tree: &str, extra_objects: &[String], mcid: u32) -> Vec<u8> {
    let content =
        format!("BT /F1 12 Tf 1 0 0 1 72 720 Tm /Span << /MCID {mcid} >> BDC (VISUAL) Tj EMC ET");
    let mut objects = vec![
        "<< /Type /Catalog /Pages 3 0 R /StructTreeRoot 6 0 R >>".to_string(),
        "<< /Type /Page /Parent 3 0 R /MediaBox [0 0 612 792] /StructParents 0 /Resources << /Font << /F1 4 0 R >> >> /Contents 5 0 R >>".to_string(),
        "<< /Type /Pages /Kids [2 0 R] /Count 1 >>".to_string(),
        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_string(),
        format!("<< /Length {} >>\nstream\n{content}\nendstream", content.len()),
        "<< /Type /StructTreeRoot /ParentTree 7 0 R >>".to_string(),
        parent_tree.to_string(),
    ];
    objects.extend(extra_objects.iter().cloned());

    let mut pdf = b"%PDF-1.7\n".to_vec();
    let mut offsets = Vec::with_capacity(objects.len());
    for (index, body) in objects.iter().enumerate() {
        offsets.push(pdf.len());
        pdf.extend_from_slice(format!("{} 0 obj\n{body}\nendobj\n", index + 1).as_bytes());
    }
    let xref = pdf.len();
    pdf.extend_from_slice(
        format!("xref\n0 {}\n0000000000 65535 f \n", objects.len() + 1).as_bytes(),
    );
    for offset in offsets {
        pdf.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    pdf.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n",
            objects.len() + 1
        )
        .as_bytes(),
    );
    pdf
}

fn extract_raw(pdf: Vec<u8>) -> String {
    let reader = PdfReader::new(Cursor::new(pdf)).expect("parse raw tagged PDF");
    let parsed = PdfDocument::new(reader);
    TextExtractor::with_options(ExtractionOptions {
        preserve_layout: true,
        ..Default::default()
    })
    .extract_from_page(&parsed, 0)
    .expect("malformed structure metadata must not fail extraction")
    .text
}

fn tagged_form_pdf() -> Vec<u8> {
    let page_content = "/Fm Do";
    let form_content =
        "BT /F1 12 Tf 1 0 0 1 0 0 Tm /Span << /MCID 0 >> BDC (VISUAL_FORM) Tj EMC ET";
    let objects = vec![
        "<< /Type /Catalog /Pages 3 0 R /StructTreeRoot 7 0 R >>".to_string(),
        "<< /Type /Page /Parent 3 0 R /MediaBox [0 0 612 792] /StructParents 0 /Resources << /Font << /F1 4 0 R >> /XObject << /Fm 6 0 R >> >> /Contents 5 0 R >>".to_string(),
        "<< /Type /Pages /Kids [2 0 R] /Count 1 >>".to_string(),
        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_string(),
        format!("<< /Length {} >>\nstream\n{page_content}\nendstream", page_content.len()),
        format!("<< /Type /XObject /Subtype /Form /BBox [0 0 100 100] /StructParents 1 /Resources << /Font << /F1 4 0 R >> >> /Length {} >>\nstream\n{form_content}\nendstream", form_content.len()),
        "<< /Type /StructTreeRoot /ParentTree 8 0 R >>".to_string(),
        "<< /Nums [0 [9 0 R] 1 [10 0 R]] >>".to_string(),
        "<< /Type /StructElem /ActualText (PAGE_WRONG) >>".to_string(),
        "<< /Type /StructElem /ActualText (FORM_RIGHT) >>".to_string(),
    ];
    let mut pdf = b"%PDF-1.7\n".to_vec();
    let mut offsets = Vec::new();
    for (index, body) in objects.iter().enumerate() {
        offsets.push(pdf.len());
        pdf.extend_from_slice(format!("{} 0 obj\n{body}\nendobj\n", index + 1).as_bytes());
    }
    let xref = pdf.len();
    pdf.extend_from_slice(
        format!("xref\n0 {}\n0000000000 65535 f \n", objects.len() + 1).as_bytes(),
    );
    for offset in offsets {
        pdf.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    pdf.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n",
            objects.len() + 1
        )
        .as_bytes(),
    );
    pdf
}

#[test]
fn resolves_inline_and_structure_actualtext_with_inline_precedence() {
    let mut page = Page::a4();

    let inline_only = page
        .begin_marked_content_with_actual_text("Span", "INLINE_ONLY")
        .expect("begin inline-only run");
    page.text()
        .set_font(Font::Helvetica, 12.0)
        .at(72.0, 720.0)
        .write("VISUAL_A")
        .expect("write first visual run");
    page.end_marked_content().expect("end inline-only run");

    let structure_only = page
        .begin_marked_content("Span")
        .expect("begin structure-only run");
    page.text()
        .set_font(Font::Helvetica, 12.0)
        .at(72.0, 700.0)
        .write("VISUAL_B")
        .expect("write second visual run");
    page.end_marked_content().expect("end structure-only run");

    let conflicting = page
        .begin_marked_content_with_actual_text("Span", "INLINE_WINS")
        .expect("begin conflicting run");
    page.text()
        .set_font(Font::Helvetica, 12.0)
        .at(72.0, 680.0)
        .write("VISUAL_C")
        .expect("write third visual run");
    page.end_marked_content().expect("end conflicting run");

    let unicode = page
        .begin_marked_content("Span")
        .expect("begin unicode structure run");
    page.text()
        .set_font(Font::Helvetica, 12.0)
        .at(72.0, 660.0)
        .write("VISUAL_D")
        .expect("write fourth visual run");
    page.end_marked_content()
        .expect("end unicode structure run");

    let mut tree = StructTree::new();
    let root = tree.set_root(StructureElement::new(StandardStructureType::Document));
    for (mcid, actual_text) in [
        (inline_only, None),
        (structure_only, Some("STRUCT_ONLY")),
        (conflicting, Some("STRUCT_LOSES")),
        (unicode, Some("2⁴⁰ E = mc² Aⁿ⁺¹B")),
    ] {
        let mut span = StructureElement::new(StandardStructureType::Span);
        if let Some(actual_text) = actual_text {
            span = span.with_actual_text(actual_text);
        }
        span.add_mcid(0, mcid);
        tree.add_child(root, span).expect("attach structure span");
    }

    let mut document = Document::new();
    document.add_page(page);
    document.set_struct_tree(tree);

    let extracted = extract(
        document,
        ExtractionOptions {
            preserve_layout: true,
            ..Default::default()
        },
    );
    let text_for = |mcid| {
        extracted
            .fragments
            .iter()
            .find(|fragment| fragment.mcid == Some(mcid))
            .map(|fragment| fragment.text.as_str())
    };

    assert_eq!(text_for(inline_only), Some("INLINE_ONLY"));
    assert_eq!(text_for(structure_only), Some("STRUCT_ONLY"));
    assert_eq!(text_for(conflicting), Some("INLINE_WINS"));
    assert_eq!(text_for(unicode), Some("2⁴⁰ E = mc² Aⁿ⁺¹B"));
    assert!(!extracted.text.contains("VISUAL_"), "{}", extracted.text);
    assert!(
        !extracted.text.contains("STRUCT_LOSES"),
        "{}",
        extracted.text
    );
}

#[test]
fn structure_actualtext_replaces_visual_text_in_flat_extraction() {
    let mut page = Page::a4();
    let mcid = page.begin_marked_content("Span").expect("begin span");
    page.text()
        .set_font(Font::Helvetica, 12.0)
        .at(72.0, 720.0)
        .write("VISUAL")
        .expect("write visual run");
    page.end_marked_content().expect("end span");
    page.text()
        .set_font(Font::Helvetica, 12.0)
        .at(72.0, 700.0)
        .write("TAIL")
        .expect("write following flat run");

    let mut tree = StructTree::new();
    let root = tree.set_root(StructureElement::new(StandardStructureType::Document));
    let mut span = StructureElement::new(StandardStructureType::Span).with_actual_text("STRUCT");
    span.add_mcid(0, mcid);
    tree.add_child(root, span).expect("attach span");
    let mut document = Document::new();
    document.add_page(page);
    document.set_struct_tree(tree);

    let bytes = document.to_bytes().expect("serialize flat fixture");
    let reader = PdfReader::new(Cursor::new(bytes)).expect("parse flat fixture");
    let parsed = PdfDocument::new(reader);
    let extracted = TextExtractor::with_options(ExtractionOptions::default())
        .with_reading_order(true)
        .extract_from_page(&parsed, 0)
        .expect("extract flat fixture");
    assert_eq!(extracted.text, "STRUCT\nTAIL");
    assert!(extracted.fragments.is_empty());
}

#[test]
fn structure_actualtext_cannot_bypass_the_page_byte_budget() {
    let mut page = Page::a4();
    let mcid = page
        .begin_marked_content("Span")
        .expect("begin structure-only run");
    page.text()
        .set_font(Font::Helvetica, 12.0)
        .at(72.0, 720.0)
        .write("x")
        .expect("write visual run");
    page.end_marked_content().expect("end structure-only run");

    let mut tree = StructTree::new();
    let root = tree.set_root(StructureElement::new(StandardStructureType::Document));
    let mut span = StructureElement::new(StandardStructureType::Span)
        .with_actual_text("STRUCT_REPLACEMENT_IS_TOO_LONG");
    span.add_mcid(0, mcid);
    tree.add_child(root, span).expect("attach structure span");

    let mut document = Document::new();
    document.add_page(page);
    document.set_struct_tree(tree);
    let extracted = extract(
        document,
        ExtractionOptions {
            preserve_layout: true,
            max_extracted_bytes: Some(10),
            ..Default::default()
        },
    );

    assert!(extracted.truncated);
    assert!(extracted.text.len() <= 10, "{}", extracted.text.len());
    assert!(!extracted.text.contains("STRUCT_REPLACEMENT"));
}

#[test]
fn resolves_parent_tree_kids_and_indirect_owner() {
    let pdf = raw_tagged_pdf(
        "<< /Kids [9 0 R] >>",
        &[
            "<< /Type /StructElem /ActualText <FEFF005300540052005500430054> >>".to_string(),
            "<< /Nums [0 [8 0 R]] >>".to_string(),
        ],
        0,
    );
    assert_eq!(extract_raw(pdf), "STRUCT");
}

#[test]
fn malformed_parent_tree_falls_back_to_visual_text() {
    let pdf = raw_tagged_pdf("<< /Nums [0 /NotAnOwnerArray] >>", &[], 0);
    assert_eq!(extract_raw(pdf), "VISUAL");
}

#[test]
fn invalid_mcid_falls_back_to_visual_text() {
    let pdf = raw_tagged_pdf(
        "<< /Nums [0 [8 0 R]] >>",
        &["<< /Type /StructElem /ActualText (STRUCT) >>".to_string()],
        7,
    );
    assert_eq!(extract_raw(pdf), "VISUAL");
}

#[test]
fn cyclic_parent_tree_falls_back_to_visual_text() {
    let pdf = raw_tagged_pdf("<< /Kids [7 0 R] >>", &[], 0);
    assert_eq!(extract_raw(pdf), "VISUAL");
}

#[test]
fn overdeep_parent_tree_falls_back_to_visual_text() {
    let depth = 34usize;
    let owner_id = 8 + depth;
    let mut nodes = Vec::new();
    for index in 0..depth {
        let object_id = 8 + index;
        let body = if index + 1 == depth {
            format!("<< /Nums [0 [{owner_id} 0 R]] >>")
        } else {
            format!("<< /Kids [{} 0 R] >>", object_id + 1)
        };
        nodes.push(body);
    }
    nodes.push("<< /Type /StructElem /ActualText (STRUCT) >>".to_string());
    let pdf = raw_tagged_pdf("<< /Kids [8 0 R] >>", &nodes, 0);
    assert_eq!(extract_raw(pdf), "VISUAL");
}

#[test]
fn oversized_parent_tree_stops_at_the_node_limit() {
    const CHILDREN: usize = 4097;
    let owner_id = 8 + CHILDREN;
    let kids = (8..8 + CHILDREN)
        .map(|id| format!("{id} 0 R"))
        .collect::<Vec<_>>()
        .join(" ");
    let mut nodes = vec!["<< >>".to_string(); CHILDREN];
    nodes[CHILDREN - 1] = format!("<< /Nums [0 [{owner_id} 0 R]] >>");
    nodes.push("<< /Type /StructElem /ActualText (STRUCT) >>".to_string());

    let pdf = raw_tagged_pdf(&format!("<< /Kids [{kids}] >>"), &nodes, 0);
    assert_eq!(extract_raw(pdf), "VISUAL");
}

#[test]
fn form_xobject_uses_its_own_structparents_context() {
    assert_eq!(extract_raw(tagged_form_pdf()), "FORM_RIGHT");
}
