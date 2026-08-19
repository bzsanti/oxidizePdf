//! Issue #513 — coherent parser-side font resources for renderers.

mod common;

use common::pdf_assembler::{assemble_pdf, stream_obj};
use flate2::{write::ZlibEncoder, Compression};
use oxidize_pdf::fonts::{EmbeddedFontFormat, FontSubtype, ResolvedFontResource, WritingMode};
use oxidize_pdf::parser::{PdfDocument, PdfReader};
use std::io::Cursor;
use std::io::Write;

fn identity_type2_pdf() -> Vec<u8> {
    let to_unicode = b"/CIDInit /ProcSet findresource begin\n\
12 dict begin begincmap\n\
1 begincodespacerange <0000> <FFFF> endcodespacerange\n\
1 beginbfchar <0001> <00E1> endbfchar\n\
endcmap end end";
    let objects = vec![
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 /Resources << /Font << /F1 5 0 R >> >> >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /Contents 4 0 R /MediaBox [0 0 100 100] >>".to_vec(),
        stream_obj("", b"BT /F1 12 Tf <00010002> Tj ET"),
        b"<< /Type /Font /Subtype /Type0 /BaseFont /Demo /Encoding /Identity-H /DescendantFonts [6 0 R] /ToUnicode 9 0 R >>".to_vec(),
        b"<< /Type /Font /Subtype /CIDFontType2 /BaseFont /Demo /CIDSystemInfo << /Registry (Adobe) /Ordering (Identity) /Supplement 0 >> /FontDescriptor 7 0 R /W [1 [600 700]] /DW 1000 /CIDToGIDMap 8 0 R >>".to_vec(),
        b"<< /Type /FontDescriptor /FontName /Demo /Flags 4 /FontBBox [0 0 1000 1000] /ItalicAngle 0 /Ascent 800 /Descent -200 /CapHeight 700 /StemV 80 /FontFile2 10 0 R >>".to_vec(),
        stream_obj("", &[0, 0, 0, 5, 0, 9]),
        stream_obj("", to_unicode),
        stream_obj("", b"\0\x01\0\0fake-ttf"),
    ];
    assemble_pdf(&objects)
}

#[test]
fn resolves_inherited_identity_type2_into_renderer_glyphs() {
    let document = PdfDocument::new(PdfReader::new(Cursor::new(identity_type2_pdf())).unwrap());
    let font = ResolvedFontResource::from_page(&document, 0, "F1").unwrap();

    assert_eq!(font.base_font.as_deref(), Some("Demo"));
    assert_eq!(font.subtype, FontSubtype::CidFontType2);
    assert_eq!(font.writing_mode, WritingMode::Horizontal);
    let embedded = font.embedded_font.as_ref().expect("FontFile2 resolved");
    assert_eq!(embedded.format, EmbeddedFontFormat::TrueType);
    assert_eq!(embedded.data, b"\0\x01\0\0fake-ttf");

    let glyphs = font.decode_glyphs(&[0, 1, 0, 2]).unwrap();
    assert_eq!(glyphs.len(), 2);
    assert_eq!(glyphs[0].source_code, vec![0, 1]);
    assert_eq!(glyphs[0].cid, Some(1));
    assert_eq!(glyphs[0].gid, Some(5));
    assert_eq!(glyphs[0].unicode.as_deref(), Some("á"));
    assert_eq!(glyphs[0].advance, 600.0);
    assert_eq!(glyphs[1].cid, Some(2));
    assert_eq!(glyphs[1].gid, Some(9));
    assert_eq!(glyphs[1].unicode, None);
    assert_eq!(glyphs[1].advance, 700.0);
}

fn one_font_pdf(font: Vec<u8>, extra: Vec<Vec<u8>>) -> Vec<u8> {
    let mut objects = vec![
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /Resources << /Font << /F1 5 0 R >> >> /Contents 4 0 R /MediaBox [0 0 100 100] >>".to_vec(),
        stream_obj("", b""),
        font,
    ];
    objects.extend(extra);
    assemble_pdf(&objects)
}

#[test]
fn resolves_cidfont_type0_open_type_program_and_vertical_mode() {
    let pdf = one_font_pdf(
        b"<< /Type /Font /Subtype /Type0 /BaseFont /CffDemo /Encoding /Identity-V /DescendantFonts [6 0 R] >>".to_vec(),
        vec![
            b"<< /Type /Font /Subtype /CIDFontType0 /BaseFont /CffDemo /CIDSystemInfo << /Registry (Adobe) /Ordering (Identity) /Supplement 0 >> /FontDescriptor 7 0 R /DW 880 >>".to_vec(),
            b"<< /Type /FontDescriptor /FontName /CffDemo /Flags 4 /FontBBox [0 0 1000 1000] /ItalicAngle 0 /Ascent 800 /Descent -200 /CapHeight 700 /StemV 80 /FontFile3 8 0 R >>".to_vec(),
            stream_obj("/Subtype /OpenType", b"OTTOcff-data"),
        ],
    );
    let document = PdfDocument::new(PdfReader::new(Cursor::new(pdf)).unwrap());
    let font = ResolvedFontResource::from_page(&document, 0, "F1").unwrap();

    assert_eq!(font.subtype, FontSubtype::CidFontType0);
    assert_eq!(font.writing_mode, WritingMode::Vertical);
    assert_eq!(
        font.embedded_font.as_ref().unwrap().format,
        EmbeddedFontFormat::OpenType
    );
    let glyph = font.decode_glyphs(&[0, 7]).unwrap().remove(0);
    assert_eq!(glyph.cid, Some(7));
    assert_eq!(glyph.gid, None);
    assert_eq!(glyph.advance, 880.0);
}

#[test]
fn resolves_symbol_difference_as_a_bullet_not_latin_x() {
    let pdf = one_font_pdf(
        b"<< /Type /Font /Subtype /Type1 /BaseFont /Symbol /FirstChar 120 /LastChar 120 /Widths [500] /Encoding << /Differences [120 /bullet] >> /FontDescriptor 6 0 R >>".to_vec(),
        vec![b"<< /Type /FontDescriptor /FontName /Symbol /Flags 4 /FontBBox [0 0 1000 1000] /ItalicAngle 0 /Ascent 800 /Descent -200 /CapHeight 700 /StemV 80 >>".to_vec()],
    );
    let document = PdfDocument::new(PdfReader::new(Cursor::new(pdf)).unwrap());
    let font = ResolvedFontResource::from_page(&document, 0, "F1").unwrap();
    let glyph = font.decode_glyphs(b"x").unwrap().remove(0);

    assert_eq!(
        font.differences.get(&120).map(String::as_str),
        Some("bullet")
    );
    assert_eq!(glyph.source_code, b"x");
    assert_eq!(glyph.unicode.as_deref(), Some("•"));
    assert_eq!(glyph.cid, None);
    assert_eq!(glyph.advance, 500.0);
}

#[test]
fn resolves_non_identity_encoding_cmap_and_default_identity_gid() {
    let encoding = b"begincmap /WMode 0 def 1 begincodespacerange <00> <FF> endcodespacerange 1 begincidchar <41> 5 endcidchar endcmap";
    let to_unicode = b"begincmap 1 begincodespacerange <00> <FF> endcodespacerange 1 beginbfchar <41> <0041> endbfchar endcmap";
    let pdf = one_font_pdf(
        b"<< /Type /Font /Subtype /Type0 /BaseFont /Mapped /Encoding 7 0 R /DescendantFonts [6 0 R] /ToUnicode 8 0 R >>".to_vec(),
        vec![
            b"<< /Type /Font /Subtype /CIDFontType2 /BaseFont /Mapped /CIDSystemInfo << /Registry (Adobe) /Ordering (Identity) /Supplement 0 >> /W [5 [444]] >>".to_vec(),
            stream_obj("", encoding),
            stream_obj("", to_unicode),
        ],
    );
    let document = PdfDocument::new(PdfReader::new(Cursor::new(pdf)).unwrap());
    let font = ResolvedFontResource::from_page(&document, 0, "F1").unwrap();
    let glyph = font.decode_glyphs(b"A").unwrap().remove(0);

    assert_eq!(glyph.source_code, b"A");
    assert_eq!(glyph.cid, Some(5));
    assert_eq!(glyph.gid, Some(5));
    assert_eq!(glyph.unicode.as_deref(), Some("A"));
    assert_eq!(glyph.advance, 444.0);
}

#[test]
fn utf16_encoding_preserves_unicode_without_tounicode() {
    let pdf = one_font_pdf(
        b"<< /Type /Font /Subtype /Type0 /BaseFont /UnicodeCid /Encoding /UniJIS-UTF16-H /DescendantFonts [6 0 R] >>".to_vec(),
        vec![b"<< /Type /Font /Subtype /CIDFontType2 /BaseFont /UnicodeCid /CIDSystemInfo << /Registry (Adobe) /Ordering (Identity) /Supplement 0 >> /W [225 [555]] >>".to_vec()],
    );
    let document = PdfDocument::new(PdfReader::new(Cursor::new(pdf)).unwrap());
    let font = ResolvedFontResource::from_page(&document, 0, "F1").unwrap();
    let glyph = font.decode_glyphs(&[0, 0xE1]).unwrap().remove(0);

    assert_eq!(glyph.cid, Some(0xE1));
    assert_eq!(glyph.gid, Some(0xE1));
    assert_eq!(glyph.unicode.as_deref(), Some("á"));
    assert_eq!(glyph.advance, 555.0);
}

#[test]
fn decodes_win_ansi_without_confusing_code_and_unicode() {
    let pdf = one_font_pdf(
        b"<< /Type /Font /Subtype /TrueType /BaseFont /Arial /FirstChar 149 /LastChar 149 /Widths [350] /Encoding /WinAnsiEncoding >>".to_vec(),
        vec![],
    );
    let document = PdfDocument::new(PdfReader::new(Cursor::new(pdf)).unwrap());
    let font = ResolvedFontResource::from_page(&document, 0, "F1").unwrap();
    let glyph = font.decode_glyphs(&[0x95]).unwrap().remove(0);

    assert_eq!(glyph.source_code, vec![0x95]);
    assert_eq!(glyph.cid, None);
    assert_eq!(glyph.unicode.as_deref(), Some("•"));
    assert_eq!(glyph.advance, 350.0);
}

#[test]
fn exposes_type3_through_the_coherent_resource_model() {
    let pdf = one_font_pdf(
        b"<< /Type /Font /Subtype /Type3 /Name /Painted /FontBBox [0 0 1 1] /FontMatrix [0.001 0 0 0.001 0 0] /FirstChar 65 /LastChar 65 /Widths [500] /Encoding << /Differences [65 /A] >> /CharProcs << /A 6 0 R >> >>".to_vec(),
        vec![stream_obj("", b"500 0 d0")],
    );
    let document = PdfDocument::new(PdfReader::new(Cursor::new(pdf)).unwrap());
    let font = ResolvedFontResource::from_page(&document, 0, "F1").unwrap();

    assert_eq!(font.subtype, FontSubtype::Type3);
    assert_eq!(font.type3.as_ref().unwrap().glyph(65).unwrap().width, 500.0);
    assert_eq!(font.decode_glyphs(b"A").unwrap()[0].advance, 500.0);
}

#[test]
fn type3_uses_tounicode_when_decoding_glyphs() {
    let to_unicode = b"begincmap 1 begincodespacerange <00> <FF> endcodespacerange 1 beginbfchar <41> <03A9> endbfchar endcmap";
    let pdf = one_font_pdf(
        b"<< /Type /Font /Subtype /Type3 /Name /Painted /FontBBox [0 0 1 1] /FontMatrix [0.001 0 0 0.001 0 0] /FirstChar 65 /LastChar 65 /Widths [500] /Encoding << /Differences [65 /A] >> /CharProcs << /A 6 0 R >> /ToUnicode 7 0 R >>".to_vec(),
        vec![stream_obj("", b"500 0 d0"), stream_obj("", to_unicode)],
    );
    let document = PdfDocument::new(PdfReader::new(Cursor::new(pdf)).unwrap());
    let font = ResolvedFontResource::from_page(&document, 0, "F1").unwrap();

    assert_eq!(
        font.decode_glyphs(b"A").unwrap()[0].unicode.as_deref(),
        Some("Ω")
    );
}

#[test]
fn unmapped_composite_code_uses_default_cid_width() {
    let encoding = b"begincmap 1 begincodespacerange <00> <FF> endcodespacerange endcmap";
    let pdf = one_font_pdf(
        b"<< /Type /Font /Subtype /Type0 /BaseFont /Mapped /Encoding 7 0 R /DescendantFonts [6 0 R] >>".to_vec(),
        vec![
            b"<< /Type /Font /Subtype /CIDFontType2 /BaseFont /Mapped /CIDSystemInfo << /Registry (Adobe) /Ordering (Identity) /Supplement 0 >> /DW 777 >>".to_vec(),
            stream_obj("", encoding),
        ],
    );
    let document = PdfDocument::new(PdfReader::new(Cursor::new(pdf)).unwrap());
    let font = ResolvedFontResource::from_page(&document, 0, "F1").unwrap();
    let glyph = font.decode_glyphs(b"A").unwrap().remove(0);

    assert_eq!(glyph.cid, None);
    assert_eq!(glyph.advance, 777.0);
}

#[test]
fn rejects_non_cid_descendant_in_type0_font() {
    let pdf = one_font_pdf(
        b"<< /Type /Font /Subtype /Type0 /BaseFont /Bad /Encoding /Identity-H /DescendantFonts [6 0 R] >>".to_vec(),
        vec![b"<< /Type /Font /Subtype /TrueType /BaseFont /Bad >>".to_vec()],
    );
    let document = PdfDocument::new(PdfReader::new(Cursor::new(pdf)).unwrap());
    let error = ResolvedFontResource::from_page(&document, 0, "F1").unwrap_err();

    assert!(error.to_string().contains("Type0 descendant"));
}

#[test]
fn rejects_odd_cid_to_gid_map_without_panicking() {
    let pdf = one_font_pdf(
        b"<< /Type /Font /Subtype /Type0 /BaseFont /Bad /Encoding /Identity-H /DescendantFonts [6 0 R] >>".to_vec(),
        vec![
            b"<< /Type /Font /Subtype /CIDFontType2 /BaseFont /Bad /CIDSystemInfo << /Registry (Adobe) /Ordering (Identity) /Supplement 0 >> /CIDToGIDMap 7 0 R >>".to_vec(),
            stream_obj("", &[0]),
        ],
    );
    let document = PdfDocument::new(PdfReader::new(Cursor::new(pdf)).unwrap());
    let error = ResolvedFontResource::from_page(&document, 0, "F1").unwrap_err();
    assert!(error.to_string().contains("odd length"));
}

#[test]
fn rejects_type0_font_without_required_encoding() {
    let pdf = one_font_pdf(
        b"<< /Type /Font /Subtype /Type0 /BaseFont /Bad /DescendantFonts [6 0 R] >>".to_vec(),
        vec![b"<< /Type /Font /Subtype /CIDFontType2 /BaseFont /Bad /CIDSystemInfo << /Registry (Adobe) /Ordering (Identity) /Supplement 0 >> >>".to_vec()],
    );
    let document = PdfDocument::new(PdfReader::new(Cursor::new(pdf)).unwrap());
    let error = ResolvedFontResource::from_page(&document, 0, "F1").unwrap_err();
    assert!(error.to_string().contains("missing its Encoding"));
}

#[test]
fn rejects_oversized_cid_to_gid_map_after_bounded_decompression() {
    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
    encoder
        .write_all(&vec![0; (u16::MAX as usize + 1) * 2 + 1])
        .unwrap();
    let compressed = encoder.finish().unwrap();
    let pdf = one_font_pdf(
        b"<< /Type /Font /Subtype /Type0 /BaseFont /Huge /Encoding /Identity-H /DescendantFonts [6 0 R] >>".to_vec(),
        vec![
            b"<< /Type /Font /Subtype /CIDFontType2 /BaseFont /Huge /CIDSystemInfo << /Registry (Adobe) /Ordering (Identity) /Supplement 0 >> /CIDToGIDMap 7 0 R >>".to_vec(),
            stream_obj("/Filter /FlateDecode", &compressed),
        ],
    );
    let document = PdfDocument::new(PdfReader::new(Cursor::new(pdf)).unwrap());
    let error = ResolvedFontResource::from_page(&document, 0, "F1").unwrap_err();
    assert!(error.to_string().contains("limit"));
}

#[test]
fn rejects_circular_font_resource_without_panicking() {
    let pdf = one_font_pdf(b"6 0 R".to_vec(), vec![b"5 0 R".to_vec()]);
    let document = PdfDocument::new(PdfReader::new(Cursor::new(pdf)).unwrap());
    assert!(ResolvedFontResource::from_page(&document, 0, "F1").is_err());
}

#[test]
fn rejects_oversized_embedded_font_after_bounded_decompression() {
    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(&vec![0; 10 * 1024 * 1024 + 1]).unwrap();
    let compressed = encoder.finish().unwrap();
    let pdf = one_font_pdf(
        b"<< /Type /Font /Subtype /TrueType /BaseFont /Huge /FirstChar 0 /LastChar 0 /Widths [500] /FontDescriptor 6 0 R >>".to_vec(),
        vec![
            b"<< /Type /FontDescriptor /FontName /Huge /Flags 32 /FontBBox [0 0 1000 1000] /ItalicAngle 0 /Ascent 800 /Descent -200 /CapHeight 700 /StemV 80 /FontFile2 7 0 R >>".to_vec(),
            stream_obj("/Filter /FlateDecode", &compressed),
        ],
    );
    let document = PdfDocument::new(PdfReader::new(Cursor::new(pdf)).unwrap());
    let error = ResolvedFontResource::from_page(&document, 0, "F1").unwrap_err();
    assert!(error.to_string().contains("limit"));
}
