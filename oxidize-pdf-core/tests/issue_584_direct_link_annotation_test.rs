//! Regression coverage for issue #584: page `/Annots` arrays may contain
//! direct annotation dictionaries as well as indirect references.

mod common;

use common::pdf_assembler::assemble_pdf;
use oxidize_pdf::parser::{PdfDocument, PdfReader};
use std::io::Cursor;

#[test]
fn extracts_a_direct_link_annotation_from_a_page_annots_array() {
    let pdf = assemble_pdf(&[
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Annots [<< /Type /Annot /Subtype /Link /Rect [10 20 30 40] /A << /S /URI /URI (https://example.com) >> >>] >>".to_vec(),
    ]);
    let document = PdfDocument::new(PdfReader::new(Cursor::new(pdf)).expect("parse fixture"));

    let annotations = document.get_page_annotations(0).expect("get annotations");

    assert_eq!(annotations.len(), 1);
    assert_eq!(
        annotations[0]
            .get("Subtype")
            .and_then(|object| object.as_name())
            .map(|name| name.0.as_str()),
        Some("Link")
    );
    assert!(annotations[0]
        .get("A")
        .and_then(|object| object.as_dict())
        .is_some());
}
