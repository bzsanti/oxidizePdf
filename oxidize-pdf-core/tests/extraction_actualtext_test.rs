//! Issue #269 Phase 1 — `/ActualText` overrides decoded glyphs at the
//! BDC scope level, with UTF-16BE support and multi-`Tj` collapsing.

#[path = "common/mod.rs"]
mod common;
use common::synthetic_pdf::build_pdf_with_content_stream;

use oxidize_pdf::parser::{PdfDocument, PdfReader};
use oxidize_pdf::text::{ExtractionOptions, TextExtractor};
use std::io::Cursor;

fn extract(content: &[u8]) -> Vec<oxidize_pdf::text::TextFragment> {
    let pdf = build_pdf_with_content_stream(content);
    let reader = PdfReader::new(Cursor::new(pdf)).expect("reader");
    let document = PdfDocument::new(reader);
    let opts = ExtractionOptions {
        preserve_layout: true,
        ..ExtractionOptions::default()
    };
    let mut extractor = TextExtractor::with_options(opts);
    extractor
        .extract_from_page(&document, 0)
        .expect("extract")
        .fragments
}

#[test]
fn literal_actualtext_overrides_decoded_glyphs() {
    // /Span <</ActualText (fi)>> BDC (xy) Tj EMC -> single fragment "fi"
    let content = b"BT\n/F1 12 Tf\n100 700 Td\n\
                    /Span << /ActualText (fi) >> BDC\n\
                    (xy) Tj\n\
                    EMC\n\
                    ET\n";
    let frags = extract(content);
    let texts: Vec<&str> = frags.iter().map(|f| f.text.as_str()).collect();
    assert!(
        texts.iter().any(|t| *t == "fi"),
        "fragment must be ActualText 'fi', not glyph 'xy'; got {:?}",
        texts
    );
    assert!(
        !texts.iter().any(|t| *t == "xy"),
        "raw glyph 'xy' must not be emitted under ActualText scope"
    );
}

#[test]
fn utf16be_actualtext_overrides_decoded_glyphs() {
    // ActualText <FEFF00660069> = UTF-16BE for "fi"
    let content = b"BT\n/F1 12 Tf\n100 700 Td\n\
                    /Span << /ActualText <FEFF00660069> >> BDC\n\
                    (junk) Tj\n\
                    EMC\n\
                    ET\n";
    let frags = extract(content);
    let texts: Vec<&str> = frags.iter().map(|f| f.text.as_str()).collect();
    assert!(
        texts.iter().any(|t| *t == "fi"),
        "UTF-16BE ActualText must decode to 'fi'; got {:?}",
        texts
    );
    assert!(!texts.iter().any(|t| *t == "junk"));
}

#[test]
fn actualtext_collapses_multi_tj_run_to_single_fragment() {
    // Two separate Tj inside one ActualText scope -> one fragment with "ff"
    let content = b"BT\n/F1 12 Tf\n100 700 Td\n\
                    /Span << /ActualText (ff) >> BDC\n\
                    (f) Tj\n\
                    (i) Tj\n\
                    EMC\n\
                    ET\n";
    let frags = extract(content);
    let texts: Vec<&str> = frags.iter().map(|f| f.text.as_str()).collect();
    let ff_count = texts.iter().filter(|t| **t == "ff").count();
    assert_eq!(
        ff_count, 1,
        "expected exactly one 'ff' fragment, got {:?}",
        texts
    );
    assert!(!texts.iter().any(|t| *t == "f"));
    assert!(!texts.iter().any(|t| *t == "i"));
}
