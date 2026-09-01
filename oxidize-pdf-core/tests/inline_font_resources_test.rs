//! A `/Font` entry may legally be a direct dictionary, not only an indirect
//! reference (ISO 32000-1 §7.3.7: any dictionary value may be direct, and §9.5
//! puts no reference requirement on the font subdictionary's entries). Every
//! text extractor cached only the entries that were indirect references, so a
//! page whose fonts are inline decoded with an empty font cache: no `/ToUnicode`
//! CMap, no `/Encoding`, byte-wise fallback — the glyph codes came out as
//! whatever the raw bytes happen to mean in WinAnsi.
//!
//! This is not a synthetic concern: our own writer emits page resources with
//! inline font dictionaries, so any document round-tripped through this library
//! (`merge`, and every page operation) came back unreadable to our own
//! extractors while other readers read it fine.
//!
//! The oracle is falsifiable by construction: `/ToUnicode` maps `A` to `Ω` and
//! `B` to `Δ`. A decode that consults the font yields `ΩΔ`; the byte-wise
//! fallback yields `AB`. The two answers cannot be confused.

mod common;

use common::pdf_assembler::{assemble_pdf, stream_obj};
use oxidize_pdf::operations::reconstruct::{merge_pdfs, MergeInput, MetadataMode};
use oxidize_pdf::parser::{ParseOptions, PdfDocument, PdfReader};
use oxidize_pdf::text::{ExtractionOptions, PlainTextExtractor, TextExtractor};
use std::io::Cursor;

/// `/ToUnicode` CMap mapping the single-byte codes 0x41/0x42 to Ω/Δ.
fn tounicode_cmap() -> Vec<u8> {
    b"/CIDInit /ProcSet findresource begin\n\
      12 dict begin\n\
      begincmap\n\
      /CMapName /Test-UCS2 def\n\
      /CMapType 2 def\n\
      1 begincodespacerange\n\
      <00> <FF>\n\
      endcodespacerange\n\
      2 beginbfchar\n\
      <41> <03A9>\n\
      <42> <0394>\n\
      endbfchar\n\
      endcmap\n\
      CMapName currentdict /CMap defineresource pop\n\
      end\n\
      end"
    .to_vec()
}

/// Page draws `(AB)` with `/F1`, whose font dictionary carries the `/ToUnicode`
/// above. `font_dict_indirect` selects whether the `/Font` **subdictionary**
/// itself is written inline or behind a reference; in both shapes the `/F1`
/// entry inside it is a direct dictionary, which is what this pins.
fn pdf_with_inline_font(font_dict_indirect: bool) -> Vec<u8> {
    let font_entry = "/F1 << /Type /Font /Subtype /Type1 /BaseFont /Helvetica /ToUnicode 5 0 R >>";
    let content = b"BT /F1 12 Tf 20 100 Td (AB) Tj ET".to_vec();

    let (resources, extra) = if font_dict_indirect {
        ("/Font 6 0 R".to_string(), true)
    } else {
        (format!("/Font << {font_entry} >>"), false)
    };

    let mut objects = vec![
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        format!(
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] \
             /Resources << {resources} >> /Contents 4 0 R >>"
        )
        .into_bytes(),
        stream_obj("", &content),
        stream_obj("", &tounicode_cmap()),
    ];
    if extra {
        objects.push(format!("<< {font_entry} >>").into_bytes());
    }
    assemble_pdf(&objects)
}

fn doc_of(bytes: Vec<u8>) -> PdfDocument<Cursor<Vec<u8>>> {
    let reader =
        PdfReader::new_with_options(Cursor::new(bytes), ParseOptions::default()).expect("parses");
    PdfDocument::new(reader)
}

#[test]
fn text_extractor_reads_tounicode_from_an_inline_font_dictionary() {
    let doc = doc_of(pdf_with_inline_font(false));
    let mut extractor = TextExtractor::with_options(ExtractionOptions::default());
    let text = extractor.extract_from_page(&doc, 0).expect("extracts").text;

    assert!(
        text.contains('\u{03A9}') && text.contains('\u{0394}'),
        "inline font dictionary ignored: /ToUnicode never consulted, got {text:?}"
    );
    assert!(
        !text.contains("AB"),
        "raw glyph codes leaked through the byte-wise fallback: {text:?}"
    );
}

#[test]
fn text_extractor_reads_inline_font_behind_an_indirect_font_subdictionary() {
    let doc = doc_of(pdf_with_inline_font(true));
    let mut extractor = TextExtractor::with_options(ExtractionOptions::default());
    let text = extractor.extract_from_page(&doc, 0).expect("extracts").text;

    assert!(
        text.contains('\u{03A9}') && text.contains('\u{0394}'),
        "inline font behind an indirect /Font dictionary ignored, got {text:?}"
    );
}

#[test]
fn plain_text_extractor_reads_tounicode_from_an_inline_font_dictionary() {
    let doc = doc_of(pdf_with_inline_font(false));
    let mut extractor = PlainTextExtractor::new();
    let text = extractor.extract(&doc, 0).expect("extracts").text;

    assert!(
        text.contains('\u{03A9}') && text.contains('\u{0394}'),
        "PlainTextExtractor ignored the inline font dictionary, got {text:?}"
    );
    assert!(
        !text.contains("AB"),
        "raw glyph codes leaked through the byte-wise fallback: {text:?}"
    );
}

#[test]
fn plain_text_extractor_resolves_an_indirect_font_subdictionary() {
    let doc = doc_of(pdf_with_inline_font(true));
    let mut extractor = PlainTextExtractor::new();
    let text = extractor.extract(&doc, 0).expect("extracts").text;

    assert!(
        text.contains('\u{03A9}') && text.contains('\u{0394}'),
        "PlainTextExtractor did not resolve /Font as an indirect reference, got {text:?}"
    );
}

/// Extract every page of a document as one string.
fn whole_document_text(path: &str) -> String {
    let reader = PdfReader::open_with_options(path, ParseOptions::default()).expect("opens");
    let doc = PdfDocument::new(reader);
    let mut extractor = TextExtractor::with_options(ExtractionOptions::default());
    let mut out = String::new();
    for page in 0..doc.page_count().expect("page count") {
        if let Ok(extracted) = extractor.extract_from_page(&doc, page) {
            out.push_str(&extracted.text);
            out.push('\n');
        }
    }
    out
}

/// The end-to-end consequence, on a real 44-page document with subsetted
/// `Identity-H` CID fonts: a document written by this library must stay
/// readable to this library. Before the fix `merge` wrote a file other readers
/// (poppler) read perfectly while our own extractors returned the raw glyph
/// codes — `Cold` came back as `& R O G`.
///
/// The assertion is on whole words rather than on character counts on purpose:
/// the mojibake preserved length, so any size- or coverage-based check passed
/// straight through it.
#[test]
fn a_document_written_by_this_library_stays_readable_to_this_library() {
    let input = "tests/fixtures/Cold_Email_Hacks.pdf";
    let output = std::env::temp_dir().join("inline_font_round_trip.pdf");
    merge_pdfs(
        vec![MergeInput::new(input)],
        &output,
        MetadataMode::FromFirst,
    )
    .expect("merge writes the document");

    let before = whole_document_text(input);
    let after = whole_document_text(output.to_str().expect("utf-8 path"));

    let after_words: std::collections::HashSet<&str> = after.split_whitespace().collect();
    let before_words: Vec<&str> = before
        .split_whitespace()
        .filter(|w| w.chars().filter(|c| c.is_alphabetic()).count() >= 4)
        .collect();

    assert!(
        before_words.len() > 1000,
        "fixture no longer carries enough text to make this test meaningful: {} words",
        before_words.len()
    );

    let kept = before_words
        .iter()
        .filter(|w| after_words.contains(*w))
        .count();
    let retention = kept as f64 / before_words.len() as f64;

    assert!(
        retention > 0.99,
        "round trip lost the text: {kept}/{} words survived ({retention:.4}); \
         output begins {:?}",
        before_words.len(),
        after.chars().take(80).collect::<String>()
    );
}
