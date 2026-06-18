//! Regression test for issue #341: xref stream double-decode.
//!
//! `XRefStream::parse` was designed to receive RAW (filtered) stream data and
//! decode it once. Its only production caller (`xref.rs`) decodes the stream
//! first via `stream.decode()` and passes the ALREADY-DECODED buffer. Because
//! the stream dict still carries `/Filter`, `parse` re-applied the filter to the
//! already-inflated bytes, which yields 0 bytes, so `to_xref_entries` reported
//! "Xref stream data truncated".
//!
//! This breaks ANY PDF whose cross-reference table is a `/Type /XRef` stream on
//! the strict reader path — including documents oxidize-pdf writes itself. The
//! lenient `PdfReader::open` path masks it via object-scan recovery.

use oxidize_pdf::parser::PdfReader;
use oxidize_pdf::writer::{PdfWriter, WriterConfig};
use oxidize_pdf::{Document, Font, Page};
use std::io::Cursor;

/// Build a minimal, well-formed PDF 1.5 whose cross-reference table is a
/// FlateDecode `/Type /XRef` stream, using oxidize-pdf's own writer.
fn write_xref_stream_pdf(num_pages: usize) -> Vec<u8> {
    let mut doc = Document::new();
    doc.set_title("Issue 341 XRef Stream");

    for i in 0..num_pages {
        let mut page = Page::a4();
        page.text()
            .set_font(Font::Helvetica, 12.0)
            .at(100.0, 700.0)
            .write(&format!("Page {}", i + 1))
            .unwrap();
        doc.add_page(page);
    }

    let mut buffer = Vec::new();
    {
        let config = WriterConfig {
            use_xref_streams: true,
            use_object_streams: false,
            pdf_version: "1.5".to_string(),
            compress_streams: true,
            incremental_update: false,
        };
        let mut writer = PdfWriter::with_config(&mut buffer, config);
        writer.write_document(&mut doc).unwrap();
    }
    buffer
}

#[test]
fn xref_stream_pdf_parses_via_strict_reader() {
    let buffer = write_xref_stream_pdf(1);

    // The strict path must register the xref-stream entries and resolve the
    // page tree. Before the fix this errored with
    // `SyntaxError { message: "Xref stream data truncated at obj 0" }`.
    let mut reader =
        PdfReader::new(Cursor::new(buffer)).expect("strict reader must parse xref-stream PDF");
    let page_count = reader
        .page_count()
        .expect("page_count must succeed on xref-stream PDF");

    assert_eq!(
        page_count, 1,
        "single-page xref-stream PDF must report 1 page"
    );
}

#[test]
fn multi_page_xref_stream_pdf_parses_via_strict_reader() {
    let buffer = write_xref_stream_pdf(5);

    let mut reader =
        PdfReader::new(Cursor::new(buffer)).expect("strict reader must parse xref-stream PDF");
    let page_count = reader
        .page_count()
        .expect("page_count must succeed on multi-page xref-stream PDF");

    assert_eq!(
        page_count, 5,
        "five-page xref-stream PDF must report 5 pages"
    );
}
