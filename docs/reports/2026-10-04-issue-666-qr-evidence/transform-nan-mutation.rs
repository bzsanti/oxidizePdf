//! #666: literal affine-coordinate oracles, ISO 32000-1 8.3.3 and 9.4.4.
#[path = "/home/santi/repos/BelowZero/oxidizePdf/oxidize-pdf/oxidize-pdf-core/tests/common/text_contracts.rs"]
mod contract;
use contract::assembler::{assemble_pdf, stream_obj};
use contract::{font, pdf};
use oxidize_pdf::parser::{ParseOptions, PdfReader};
use oxidize_pdf::text::{ExtractionOptions, TextExtractor};
use std::io::Cursor;

fn assert_origins(content: &str, expected: &[(f64, f64)]) {
    let bytes = pdf(
        &font("Helvetica", "/WinAnsiEncoding", Some(500.0), ""),
        content.as_bytes(),
        vec![],
    );
    assert_pdf_origins(bytes, expected);
}
fn assert_pdf_origins(bytes: Vec<u8>, expected: &[(f64, f64)]) {
    let doc = PdfReader::new_with_options(Cursor::new(bytes), ParseOptions::strict())
        .unwrap()
        .into_document();
    let mut result = TextExtractor::with_options(ExtractionOptions {
        preserve_layout: true,
        sort_by_position: false,
        ..ExtractionOptions::default()
    })
    .extract_from_page(&doc, 0)
    .unwrap();
    for fragment in &mut result.fragments { fragment.x = f64::NAN; fragment.y = f64::NAN; }
    assert_eq!(
        result.fragments.len(),
        expected.len(),
        "{:?}",
        result.fragments
    );
    let mut failures = Vec::new();
    for (i, (fragment, &(x, y))) in result.fragments.iter().zip(expected).enumerate() {
        let text = char::from(b'A' + i as u8).to_string();
        if fragment.text.trim_end_matches(' ') != text
            || (fragment.x - x).abs() > 0.0001
            || (fragment.y - y).abs() > 0.0001
        {
            failures.push(format!(
                "{text} expected ({x},{y}), actual {:?} at ({},{})",
                fragment.text, fragment.x, fragment.y
            ));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
// Rendering-mode changes only separate extraction fragments; advances remain 5pt.
#[test]
fn tm_rotation_rotates_glyph_advance() {
    assert_origins(
        "BT /F1 10 Tf 0 1 -1 0 300 100 Tm (A) Tj 1 Tr (B) Tj ET",
        &[(300.0, 100.0), (300.0, 105.0)],
    );
}
#[test]
fn tm_shear_preserves_both_advance_components() {
    assert_origins(
        "BT /F1 10 Tf 1 0.5 0.25 1 100 200 Tm (A) Tj 1 Tr (B) Tj ET",
        &[(100.0, 200.0), (105.0, 202.5)],
    );
}
#[test]
fn ctm_nonuniform_scale_and_translation_apply_to_text_origins() {
    assert_origins(
        "q 2 0 0 3 10 20 cm BT /F1 10 Tf 1 0 0 1 100 200 Tm (A) Tj 1 Tr (B) Tj ET Q",
        &[(210.0, 620.0), (220.0, 620.0)],
    );
}
#[test]
fn ctm_shear_and_translation_apply_to_text_origins() {
    assert_origins(
        "q 1 0.5 0.25 1 10 20 cm BT /F1 10 Tf 1 0 0 1 100 200 Tm (A) Tj 1 Tr (B) Tj ET Q",
        &[(160.0, 270.0), (165.0, 272.5)],
    );
}
#[test]
fn ctm_scale_composes_with_rotated_tm_in_pdf_order() {
    assert_origins(
        "q 2 0 0 3 10 20 cm BT /F1 10 Tf 0 1 -1 0 100 100 Tm (A) Tj 1 Tr (B) Tj ET Q",
        &[(210.0, 320.0), (210.0, 335.0)],
    );
}
#[test]
fn q_and_q_restore_ctm_before_the_next_text_object() {
    assert_origins("q 2 0 0 2 10 20 cm BT /F1 10 Tf 100 100 Td (A) Tj ET Q BT /F1 10 Tf 1 Tr 100 100 Td (B) Tj ET", &[(210.0,220.0),(100.0,100.0)]);
}
#[test]
fn bt_resets_text_matrix_and_retains_text_state() {
    assert_origins(
        "BT /F1 10 Tf 100 200 Td (A) Tj ET BT 1 Tr 10 20 Td (B) Tj ET",
        &[(100.0, 200.0), (10.0, 20.0)],
    );
}

#[test]
fn font_switch_changes_advance_without_resetting_text_matrix() {
    let objects = vec![
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Resources << /Font << /F1 4 0 R /F2 6 0 R >> >> /Contents 5 0 R >>".to_vec(),
        font("Helvetica", "/WinAnsiEncoding", Some(500.0), "").into_bytes(),
        stream_obj("", b"BT /F1 10 Tf 100 200 Td (A) Tj /F2 10 Tf 1 Tr (B) Tj /F1 10 Tf 0 Tr (C) Tj ET"),
        font("Courier", "/WinAnsiEncoding", Some(800.0), "").into_bytes(),
    ];
    assert_pdf_origins(
        assemble_pdf(&objects),
        &[(100.0, 200.0), (105.0, 200.0), (113.0, 200.0)],
    );
}
#[test]
fn nested_forms_compose_matrices_shadow_fonts_and_restore_parent_state() {
    let objects = vec![
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Resources << /Font << /F1 4 0 R >> /XObject << /Outer 6 0 R >> >> /Contents 5 0 R >>".to_vec(),
        font("Helvetica", "/WinAnsiEncoding", Some(500.0), "").into_bytes(),
        stream_obj("", b"BT /F1 10 Tf 100 200 Td (A) Tj ET /Outer Do BT /F1 10 Tf 100 300 Td (D) Tj 1 Tr (E) Tj ET"),
        stream_obj("/Type /XObject /Subtype /Form /BBox [0 0 200 200] /Matrix [1 0 0 1 50 60] /Resources << /XObject << /Inner 7 0 R >> >>", b"/Inner Do"),
        stream_obj("/Type /XObject /Subtype /Form /BBox [0 0 100 100] /Matrix [2 0 0 2 5 6] /Resources << /Font << /F1 8 0 R >> >>", b"BT /F1 10 Tf 10 20 Td (B) Tj 1 Tr (C) Tj ET"),
        font("Courier", "/WinAnsiEncoding", Some(800.0), "").into_bytes(),
    ];
    assert_pdf_origins(
        assemble_pdf(&objects),
        &[
            (100.0, 200.0),
            (75.0, 106.0),
            (91.0, 106.0),
            (100.0, 300.0),
            (105.0, 300.0),
        ],
    );
}
