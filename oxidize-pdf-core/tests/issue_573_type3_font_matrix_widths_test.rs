//! Issue #573 — Type 3 /FontMatrix scaling when computing character widths from /Widths.
//!
//! When Type 3 fonts specify `/FontMatrix [1.0 0.0 0.0 1.0 0.0 0.0]` (or arbitrary FontMatrix `[a, b, c, d, e, f]`),
//! the `/Widths` array contains values in glyph coordinate space, which is transformed into text space
//! by `FontMatrix` (specifically scaled by `FontMatrix[0]`).
//! Standard simple fonts have an implicit `FontMatrix [0.001 0 0 0.001 0 0]`, so their widths are in milli-ems.
//! Normalizing Type 3 widths to milli-ems in `FontMetrics` ensures accurate character widths during text extraction
//! and renderer font resolution.

mod common;

use common::pdf_assembler::{assemble_pdf, stream_obj};
use oxidize_pdf::fonts::{FontSubtype, ResolvedFontResource, Type3Font};
use oxidize_pdf::parser::{PdfDocument, PdfReader};
use oxidize_pdf::text::{ExtractionOptions, TextExtractor};
use std::io::Cursor;

fn type3_pdf(font_matrix_str: &str, widths_str: &str) -> Vec<u8> {
    let font_obj = format!(
        "<< /Type /Font /Subtype /Type3 /Name /T3Font /FontBBox [0 0 1 1] \
           /FontMatrix {font_matrix_str} /FirstChar 65 /LastChar 66 /Widths {widths_str} \
           /Encoding << /Differences [65 /A /B] >> /CharProcs << /A 6 0 R /B 7 0 R >> >>"
    );
    let objects = vec![
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 /Resources << /Font << /F1 5 0 R >> >> >>"
            .to_vec(),
        b"<< /Type /Page /Parent 2 0 R /Contents 4 0 R /MediaBox [0 0 612 792] >>".to_vec(),
        stream_obj("", b"BT /F1 10 Tf 100 700 Td (AB) Tj ET"),
        font_obj.into_bytes(),
        stream_obj("", b"0.5 0 d0"),
        stream_obj("", b"0.6 0 d0"),
    ];
    assemble_pdf(&objects)
}

fn type3_indirect_pdf() -> Vec<u8> {
    let objects = vec![
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 /Resources << /Font << /F1 5 0 R >> >> >>"
            .to_vec(),
        b"<< /Type /Page /Parent 2 0 R /Contents 4 0 R /MediaBox [0 0 612 792] >>".to_vec(),
        stream_obj("", b"BT /F1 10 Tf 100 700 Td (AB) Tj ET"),
        b"<< /Type /Font /Subtype /Type3 /Name /T3Indirect /FontBBox 8 0 R \
           /FontMatrix 9 0 R /FirstChar 10 0 R /LastChar 11 0 R /Widths 12 0 R \
           /Encoding << /Differences [65 /A /B] >> /CharProcs << /A 6 0 R /B 7 0 R >> >>"
            .to_vec(),
        stream_obj("", b"0.5 0 d0"),
        stream_obj("", b"0.6 0 d0"),
        b"[0 0 1 1]".to_vec(),
        b"[1.0 0.0 0.0 1.0 0.0 0.0]".to_vec(),
        b"65".to_vec(),
        b"66".to_vec(),
        b"[0.5 0.6]".to_vec(),
    ];
    assemble_pdf(&objects)
}

#[test]
fn type3_with_unit_font_matrix_scales_widths_to_milli_ems() {
    let pdf_bytes = type3_pdf("[1.0 0.0 0.0 1.0 0.0 0.0]", "[0.5 0.6]");
    let document = PdfDocument::new(PdfReader::new(Cursor::new(&pdf_bytes)).unwrap());
    let font_obj = document.get_object(5, 0).unwrap();

    // 1. Check Type3Font::resolve preserves glyph-space width in Type3Glyph
    let type3 = Type3Font::resolve(&font_obj, &document).unwrap();
    assert_eq!(type3.font_matrix, [1.0, 0.0, 0.0, 1.0, 0.0, 0.0]);
    assert_eq!(type3.glyph(65).unwrap().width, 0.5);
    assert_eq!(type3.glyph(66).unwrap().width, 0.6);

    // 2. Check ResolvedFontResource decodes glyph advance in milli-ems
    let resolved = ResolvedFontResource::from_page(&document, 0, "F1").unwrap();
    assert_eq!(resolved.subtype, FontSubtype::Type3);
    let glyphs = resolved.decode_glyphs(b"AB").unwrap();
    assert_eq!(glyphs.len(), 2);
    // 0.5 in glyph space * 1.0 * 1000.0 = 500.0 milli-ems
    assert_eq!(glyphs[0].advance, 500.0);
    // 0.6 in glyph space * 1.0 * 1000.0 = 600.0 milli-ems
    assert_eq!(glyphs[1].advance, 600.0);

    // 3. Check full text extraction
    let mut text_extractor = TextExtractor::with_options(ExtractionOptions::default());
    let page = text_extractor.extract_from_page(&document, 0).unwrap();
    assert_eq!(page.text.trim(), "AB");
}

#[test]
fn type3_with_milli_em_font_matrix_preserves_widths() {
    let pdf_bytes = type3_pdf("[0.001 0.0 0.0 0.001 0.0 0.0]", "[500 600]");
    let document = PdfDocument::new(PdfReader::new(Cursor::new(&pdf_bytes)).unwrap());

    let resolved = ResolvedFontResource::from_page(&document, 0, "F1").unwrap();
    let glyphs = resolved.decode_glyphs(b"AB").unwrap();
    // 500 * 0.001 * 1000.0 = 500.0 milli-ems
    assert_eq!(glyphs[0].advance, 500.0);
    assert_eq!(glyphs[1].advance, 600.0);

    let mut text_extractor = TextExtractor::with_options(ExtractionOptions::default());
    let page = text_extractor.extract_from_page(&document, 0).unwrap();
    assert_eq!(page.text.trim(), "AB");
}

#[test]
fn type3_with_arbitrary_font_matrix() {
    // Arbitrary FontMatrix [0.002 0.0 0.0 0.002 0.0 0.0]
    let pdf_bytes = type3_pdf("[0.002 0.0 0.0 0.002 0.0 0.0]", "[250 300]");
    let document = PdfDocument::new(PdfReader::new(Cursor::new(&pdf_bytes)).unwrap());

    let resolved = ResolvedFontResource::from_page(&document, 0, "F1").unwrap();
    let glyphs = resolved.decode_glyphs(b"AB").unwrap();
    // 250 * 0.002 * 1000.0 = 500.0 milli-ems
    assert_eq!(glyphs[0].advance, 500.0);
    // 300 * 0.002 * 1000.0 = 600.0 milli-ems
    assert_eq!(glyphs[1].advance, 600.0);

    let mut text_extractor = TextExtractor::with_options(ExtractionOptions::default());
    let page = text_extractor.extract_from_page(&document, 0).unwrap();
    assert_eq!(page.text.trim(), "AB");
}

#[test]
fn type3_with_indirect_font_matrix_and_widths() {
    let pdf_bytes = type3_indirect_pdf();
    let document = PdfDocument::new(PdfReader::new(Cursor::new(&pdf_bytes)).unwrap());
    let font_obj = document.get_object(5, 0).unwrap();

    let type3 = Type3Font::resolve(&font_obj, &document).unwrap();
    assert_eq!(type3.font_matrix, [1.0, 0.0, 0.0, 1.0, 0.0, 0.0]);
    assert_eq!(type3.font_bbox, [0.0, 0.0, 1.0, 1.0]);
    assert_eq!(type3.glyph(65).unwrap().width, 0.5);
    assert_eq!(type3.glyph(66).unwrap().width, 0.6);

    let resolved = ResolvedFontResource::from_page(&document, 0, "F1").unwrap();
    let glyphs = resolved.decode_glyphs(b"AB").unwrap();
    assert_eq!(glyphs[0].advance, 500.0);
    assert_eq!(glyphs[1].advance, 600.0);

    let mut text_extractor = TextExtractor::with_options(ExtractionOptions::default());
    let page = text_extractor.extract_from_page(&document, 0).unwrap();
    assert_eq!(page.text.trim(), "AB");
}
