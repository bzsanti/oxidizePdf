//! Issue #572 — complete Adobe Glyph List mapping in `glyph_name_to_unicode`
//! for `/Differences` font encodings and proper resolution of indirect `/Encoding`
//! and `/Differences` references.

mod common;

use common::pdf_assembler::{assemble_pdf, stream_obj};
use oxidize_pdf::parser::{PdfDocument, PdfReader};
use oxidize_pdf::text::{
    glyph_name_to_unicode, glyph_name_to_unicode_sequence, ExtractionOptions, TextExtractor,
};
use std::io::Cursor;

#[test]
fn glyph_name_to_unicode_comprehensive() {
    // 1. Single character glyph names
    assert_eq!(glyph_name_to_unicode("A"), Some('A'));
    assert_eq!(glyph_name_to_unicode("z"), Some('z'));
    assert_eq!(glyph_name_to_unicode("5"), Some('5'));
    assert_eq!(glyph_name_to_unicode("+"), Some('+'));
    assert_eq!(glyph_name_to_unicode("/"), Some('/'));
    assert_eq!(glyph_name_to_unicode("."), Some('.'));

    // 2. uniXXXX Unicode escapes (4 hex digits)
    assert_eq!(glyph_name_to_unicode("uni0041"), Some('A'));
    assert_eq!(glyph_name_to_unicode("uni00E9"), Some('é'));
    assert_eq!(glyph_name_to_unicode("uni20AC"), Some('€'));
    assert_eq!(glyph_name_to_unicode("uni00DF"), Some('ß'));
    assert_eq!(glyph_name_to_unicode("uni00B7"), Some('·'));

    // 3. uXXXX / uXXXXXX Unicode escapes (4..6 hex digits)
    assert_eq!(glyph_name_to_unicode("u0041"), Some('A'));
    assert_eq!(glyph_name_to_unicode("u00E9"), Some('é'));
    assert_eq!(glyph_name_to_unicode("u1F600"), Some('😀'));
    assert_eq!(glyph_name_to_unicode("u000041"), Some('A'));
    assert_eq!(glyph_name_to_unicode("u2022"), Some('•'));

    // AGL entries outside the previous hand-written subset and multi-scalar
    // `uni` names must not be silently lost.
    assert_eq!(glyph_name_to_unicode("afii10017"), Some('А'));
    assert_eq!(
        glyph_name_to_unicode_sequence("uni00410042").as_deref(),
        Some("AB")
    );

    // 4. AGL standard Latin glyph names
    assert_eq!(glyph_name_to_unicode("space"), Some(' '));
    assert_eq!(glyph_name_to_unicode("zero"), Some('0'));
    assert_eq!(glyph_name_to_unicode("nine"), Some('9'));
    assert_eq!(glyph_name_to_unicode("exclam"), Some('!'));
    assert_eq!(glyph_name_to_unicode("question"), Some('?'));

    // 5. Accented letters (lowercase & uppercase)
    assert_eq!(glyph_name_to_unicode("aacute"), Some('á'));
    assert_eq!(glyph_name_to_unicode("eacute"), Some('é'));
    assert_eq!(glyph_name_to_unicode("atilde"), Some('ã'));
    assert_eq!(glyph_name_to_unicode("ccedilla"), Some('ç'));
    assert_eq!(glyph_name_to_unicode("agrave"), Some('à'));
    assert_eq!(glyph_name_to_unicode("acircumflex"), Some('â'));
    assert_eq!(glyph_name_to_unicode("adieresis"), Some('ä'));
    assert_eq!(glyph_name_to_unicode("aring"), Some('å'));
    assert_eq!(glyph_name_to_unicode("egrave"), Some('è'));
    assert_eq!(glyph_name_to_unicode("ecircumflex"), Some('ê'));
    assert_eq!(glyph_name_to_unicode("edieresis"), Some('ë'));
    assert_eq!(glyph_name_to_unicode("iacute"), Some('í'));
    assert_eq!(glyph_name_to_unicode("igrave"), Some('ì'));
    assert_eq!(glyph_name_to_unicode("icircumflex"), Some('î'));
    assert_eq!(glyph_name_to_unicode("idieresis"), Some('ï'));
    assert_eq!(glyph_name_to_unicode("ntilde"), Some('ñ'));
    assert_eq!(glyph_name_to_unicode("oacute"), Some('ó'));
    assert_eq!(glyph_name_to_unicode("ograve"), Some('ò'));
    assert_eq!(glyph_name_to_unicode("ocircumflex"), Some('ô'));
    assert_eq!(glyph_name_to_unicode("otilde"), Some('õ'));
    assert_eq!(glyph_name_to_unicode("odieresis"), Some('ö'));
    assert_eq!(glyph_name_to_unicode("uacute"), Some('ú'));
    assert_eq!(glyph_name_to_unicode("ugrave"), Some('ù'));
    assert_eq!(glyph_name_to_unicode("ucircumflex"), Some('û'));
    assert_eq!(glyph_name_to_unicode("udieresis"), Some('ü'));
    assert_eq!(glyph_name_to_unicode("yacute"), Some('ý'));
    assert_eq!(glyph_name_to_unicode("ydieresis"), Some('ÿ'));
    assert_eq!(glyph_name_to_unicode("oslash"), Some('ø'));
    assert_eq!(glyph_name_to_unicode("thorn"), Some('þ'));
    assert_eq!(glyph_name_to_unicode("eth"), Some('ð'));

    assert_eq!(glyph_name_to_unicode("Aacute"), Some('Á'));
    assert_eq!(glyph_name_to_unicode("Eacute"), Some('É'));
    assert_eq!(glyph_name_to_unicode("Atilde"), Some('Ã'));
    assert_eq!(glyph_name_to_unicode("Ccedilla"), Some('Ç'));
    assert_eq!(glyph_name_to_unicode("Adieresis"), Some('Ä'));
    assert_eq!(glyph_name_to_unicode("Aring"), Some('Å'));
    assert_eq!(glyph_name_to_unicode("Oslash"), Some('Ø'));
    assert_eq!(glyph_name_to_unicode("Ntilde"), Some('Ñ'));
    assert_eq!(glyph_name_to_unicode("Thorn"), Some('Þ'));
    assert_eq!(glyph_name_to_unicode("Eth"), Some('Ð'));

    // 6. German eszett
    assert_eq!(glyph_name_to_unicode("germandbls"), Some('ß'));

    // 7. Typographical & Punctuation symbols
    assert_eq!(glyph_name_to_unicode("periodcentered"), Some('·'));
    assert_eq!(glyph_name_to_unicode("bullet"), Some('•'));
    assert_eq!(glyph_name_to_unicode("hyphen"), Some('-'));
    assert_eq!(glyph_name_to_unicode("endash"), Some('–'));
    assert_eq!(glyph_name_to_unicode("emdash"), Some('—'));
    assert_eq!(glyph_name_to_unicode("quoteleft"), Some('‘'));
    assert_eq!(glyph_name_to_unicode("quoteright"), Some('’'));
    assert_eq!(glyph_name_to_unicode("quotedblleft"), Some('“'));
    assert_eq!(glyph_name_to_unicode("quotedblright"), Some('”'));
    assert_eq!(glyph_name_to_unicode("ellipsis"), Some('…'));
    assert_eq!(glyph_name_to_unicode("dagger"), Some('†'));
    assert_eq!(glyph_name_to_unicode("daggerdbl"), Some('‡'));
    assert_eq!(glyph_name_to_unicode("perthousand"), Some('‰'));

    // 8. Ligatures
    assert_eq!(glyph_name_to_unicode("fi"), Some('ﬁ'));
    assert_eq!(glyph_name_to_unicode("fl"), Some('ﬂ'));
    assert_eq!(glyph_name_to_unicode("ffi"), Some('ﬃ'));
    assert_eq!(glyph_name_to_unicode("ffl"), Some('ﬄ'));
    assert_eq!(glyph_name_to_unicode("ff"), Some('ﬀ'));
    assert_eq!(glyph_name_to_unicode("ft"), Some('ﬅ'));
    assert_eq!(glyph_name_to_unicode("st"), Some('ﬆ'));
    assert_eq!(glyph_name_to_unicode("oe"), Some('œ'));
    assert_eq!(glyph_name_to_unicode("OE"), Some('Œ'));
    assert_eq!(glyph_name_to_unicode("ae"), Some('æ'));
    assert_eq!(glyph_name_to_unicode("AE"), Some('Æ'));

    // 9. Symbols
    assert_eq!(glyph_name_to_unicode("plus"), Some('+'));
    assert_eq!(glyph_name_to_unicode("minus"), Some('−'));
    assert_eq!(glyph_name_to_unicode("slash"), Some('/'));
    assert_eq!(glyph_name_to_unicode("backslash"), Some('\\'));
    assert_eq!(glyph_name_to_unicode("Euro"), Some('€'));
    assert_eq!(glyph_name_to_unicode("trademark"), Some('™'));
    assert_eq!(glyph_name_to_unicode("copyright"), Some('©'));
    assert_eq!(glyph_name_to_unicode("registered"), Some('®'));
    assert_eq!(glyph_name_to_unicode("checkmark"), Some('✓'));
    assert_eq!(glyph_name_to_unicode("multiply"), Some('×'));
    assert_eq!(glyph_name_to_unicode("divide"), Some('÷'));

    // 10. Variant suffix handling
    assert_eq!(glyph_name_to_unicode("A.swash"), Some('A'));
    assert_eq!(glyph_name_to_unicode("aacute.alt"), Some('á'));
    assert_eq!(glyph_name_to_unicode("bullet.custom"), Some('•'));
}

#[test]
fn extract_text_with_direct_differences_encoding() {
    let content =
        b"BT /F1 12 Tf 50 700 Td (\x01\x02\x03\x04\x05\x06\x07\x08\x09\x0A\x0B\x0C) Tj ET";
    let objects = vec![
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /Resources << /Font << /F1 5 0 R >> >> /Contents 4 0 R /MediaBox [0 0 600 800] >>".to_vec(),
        stream_obj("", content),
        b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding << /Type /Encoding /Differences [1 /aacute /germandbls /periodcentered /bullet /hyphen /endash /emdash /fi /fl /plus /minus /slash] >> >>".to_vec(),
    ];
    let pdf = assemble_pdf(&objects);
    let document = PdfDocument::new(PdfReader::new(Cursor::new(pdf)).unwrap());

    let mut extractor = TextExtractor::new();
    let text_result = extractor.extract_from_page(&document, 0).unwrap();
    assert_eq!(text_result.text.trim(), "áß·•-–—ﬁﬂ+−/");
}

#[test]
fn extract_text_with_indirect_encoding_reference() {
    // Font (obj 5) references indirect /Encoding (obj 6)
    let content = b"BT /F1 12 Tf 50 700 Td (\x01\x02\x03\x04\x05\x06) Tj ET";
    let objects = vec![
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /Resources << /Font << /F1 5 0 R >> >> /Contents 4 0 R /MediaBox [0 0 600 800] >>".to_vec(),
        stream_obj("", content),
        b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding 6 0 R >>".to_vec(),
        b"<< /Type /Encoding /BaseEncoding /WinAnsiEncoding /Differences [1 /eacute /atilde /ccedilla /ffi /ffl /backslash] >>".to_vec(),
    ];
    let pdf = assemble_pdf(&objects);
    let document = PdfDocument::new(PdfReader::new(Cursor::new(pdf)).unwrap());

    let mut extractor = TextExtractor::new();
    let text_result = extractor.extract_from_page(&document, 0).unwrap();
    assert_eq!(text_result.text.trim(), "éãçﬃﬄ\\");
}

#[test]
fn extract_text_with_indirect_differences_array() {
    // Font (obj 5) references /Encoding (obj 6), which references /Differences (obj 7)
    let content = b"BT /F1 12 Tf 50 700 Td (\x10\x11\x12\x13\x14) Tj ET";
    let objects = vec![
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /Resources << /Font << /F1 5 0 R >> >> /Contents 4 0 R /MediaBox [0 0 600 800] >>".to_vec(),
        stream_obj("", content),
        b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding 6 0 R >>".to_vec(),
        b"<< /Type /Encoding /Differences 7 0 R >>".to_vec(),
        b"[16 /uni0041 /u00E9 /u1F600 /germandbls /bullet]".to_vec(),
    ];
    let pdf = assemble_pdf(&objects);
    let document = PdfDocument::new(PdfReader::new(Cursor::new(pdf)).unwrap());

    let mut extractor = TextExtractor::new();
    let text_result = extractor.extract_from_page(&document, 0).unwrap();
    assert_eq!(text_result.text.trim(), "Aé😀ß•");
}

#[test]
fn extract_text_with_single_char_and_unicode_escapes_in_differences() {
    let content = b"BT /F1 12 Tf 50 700 Td (\x41\x42\x43\x44\x45) Tj ET";
    let objects = vec![
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /Resources << /Font << /F1 5 0 R >> >> /Contents 4 0 R /MediaBox [0 0 600 800] >>".to_vec(),
        stream_obj("", content),
        b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding << /Differences [65 /X /uni00DF /u2014 /ff /A.swash] >> >>".to_vec(),
    ];
    let pdf = assemble_pdf(&objects);
    let document = PdfDocument::new(PdfReader::new(Cursor::new(pdf)).unwrap());

    let mut extractor = TextExtractor::with_options(ExtractionOptions::default());
    let text_result = extractor.extract_from_page(&document, 0).unwrap();
    assert_eq!(text_result.text.trim(), "Xß—ﬀA");
}
