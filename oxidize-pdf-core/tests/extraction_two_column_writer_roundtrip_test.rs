//! Issue #265 — writer→extractor roundtrip verifying that two parallel
//! columns written with the Page text API extract as separated paragraphs
//! (no character interleaving). Complements the NCSC corpus test by
//! exercising a deterministic synthetic input.

use oxidize_pdf::parser::{PdfDocument, PdfReader};
use oxidize_pdf::text::{ExtractionOptions, TextExtractor};
use oxidize_pdf::{Document, Font, Page};
use std::io::Cursor;

#[test]
fn two_column_layout_extracts_without_interleaving() {
    // Build a PDF with two parallel paragraphs at distinct X but
    // overlapping Y baselines. Emission order: column 1 fully, then
    // column 2 fully — mimics how the NCSC PDF lays out tables.
    let mut doc = Document::new();
    let mut page = Page::a4();

    // Column 1: X=50, Y descending 700..650 in 12-unit steps.
    for (i, text) in ["Col1-line1", "Col1-line2", "Col1-line3"]
        .iter()
        .enumerate()
    {
        page.text()
            .set_font(Font::Helvetica, 10.0)
            .at(50.0, 700.0 - (i as f64) * 12.0)
            .write(text)
            .expect("col1 write");
    }
    // Column 2: X=300, Y baselines near col1's (overlap inside Y-tolerance).
    for (i, text) in ["Col2-line1", "Col2-line2", "Col2-line3"]
        .iter()
        .enumerate()
    {
        page.text()
            .set_font(Font::Helvetica, 10.0)
            .at(300.0, 700.5 - (i as f64) * 12.0)
            .write(text)
            .expect("col2 write");
    }

    doc.add_page(page);

    let pdf_bytes = doc.to_bytes().expect("write PDF");

    let reader = PdfReader::new(Cursor::new(pdf_bytes)).expect("read PDF");
    let document = PdfDocument::new(reader);

    let opts = ExtractionOptions {
        preserve_layout: true,
        reconstruct_paragraphs: true,
        ..ExtractionOptions::default()
    };
    let mut extractor = TextExtractor::with_options(opts);
    let extracted = extractor.extract_from_page(&document, 0).expect("extract");
    let text = extracted.text.as_str();

    // Negative: no character interleaving between columns. A literal
    // interleaved sequence would contain substrings like "CoCl1ol2".
    assert!(
        !text.contains("CoCl"),
        "expected no character interleaving between columns; got:\n{}",
        text
    );

    // Positive: each column's lines survive as recognizable runs.
    for needle in &[
        "Col1-line1",
        "Col1-line2",
        "Col1-line3",
        "Col2-line1",
        "Col2-line2",
        "Col2-line3",
    ] {
        assert!(
            text.contains(needle),
            "missing column run {:?} in extracted text:\n{}",
            needle,
            text
        );
    }
}
