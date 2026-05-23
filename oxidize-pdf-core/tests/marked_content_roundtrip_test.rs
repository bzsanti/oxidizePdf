//! Issue #269 Phase 1 — writer-to-extractor roundtrip.
//!
//! Produces a tagged 1-page PDF using the real writer API (`Page::begin_marked_content`,
//! `Page::text()`, `Page::end_marked_content`, `Document::to_bytes`): two paragraphs at
//! identical baseline (Y=700 pt) with distinct MCIDs assigned by the page counter.
//! The extractor must keep them as distinct fragments and tag each with the
//! corresponding `mcid` and `struct_tag`.
//!
//! **Option A** was used: `Page::begin_marked_content(tag)` → auto-assigns mcid,
//! writes `/Tag <</MCID N>> BDC` into the content stream; `Page::end_marked_content()`
//! appends `EMC`. `Document::to_bytes()` serializes a real, parseable PDF.

use oxidize_pdf::parser::{PdfDocument, PdfReader};
use oxidize_pdf::text::{ExtractionOptions, TextExtractor};
use oxidize_pdf::{Document, Font, Page};
use std::io::Cursor;

#[test]
fn writer_to_extractor_keeps_overlaid_mcid_blocks_distinct() {
    // === BUILD ===
    // Two paragraphs at Y=700:
    //   /P <</MCID 0>> BDC  BT /F1 12 Tf 100 700 Td (Hello) Tj ET  EMC
    //   /P <</MCID 1>> BDC  BT /F1 12 Tf 300 700 Td (World) Tj ET  EMC
    //
    // `begin_marked_content` appends the BDC inline with the text context buffer;
    // each `text().at(x, y).write(s)` call emits BT ... ET at the given position.

    let mut page = Page::a4();

    // First marked-content block (gets MCID 0 from the page's counter)
    let mcid_hello = page
        .begin_marked_content("P")
        .expect("begin_marked_content P/Hello");
    page.text()
        .set_font(Font::Helvetica, 12.0)
        .at(100.0, 700.0)
        .write("Hello")
        .expect("write Hello");
    page.end_marked_content()
        .expect("end_marked_content P/Hello");

    // Second marked-content block (gets MCID 1)
    let mcid_world = page
        .begin_marked_content("P")
        .expect("begin_marked_content P/World");
    page.text()
        .set_font(Font::Helvetica, 12.0)
        .at(300.0, 700.0)
        .write("World")
        .expect("write World");
    page.end_marked_content()
        .expect("end_marked_content P/World");

    assert_eq!(mcid_hello, 0, "first BDC must receive MCID 0");
    assert_eq!(mcid_world, 1, "second BDC must receive MCID 1");

    let mut doc = Document::new();
    doc.add_page(page);
    let pdf_bytes = doc.to_bytes().expect("Document::to_bytes");

    // === READ ===
    let reader = PdfReader::new(Cursor::new(pdf_bytes)).expect("PdfReader::new");
    let document = PdfDocument::new(reader);
    let opts = ExtractionOptions {
        preserve_layout: true,
        ..Default::default()
    };
    let mut extractor = TextExtractor::with_options(opts);
    let extracted = extractor
        .extract_from_page(&document, 0)
        .expect("extract page 0");

    // === ASSERT: text survives as distinct fragments ===
    let texts: Vec<&str> = extracted
        .fragments
        .iter()
        .map(|f| f.text.as_str())
        .collect();
    assert!(
        texts.contains(&"Hello"),
        "'Hello' must survive as its own fragment; got {:?}",
        texts
    );
    assert!(
        texts.contains(&"World"),
        "'World' must survive as its own fragment; got {:?}",
        texts
    );
    assert!(
        !texts
            .iter()
            .any(|t| t.contains("HelloWorld") || t.contains("WorldHello")),
        "fragments must not be merged across MCID boundaries; got {:?}",
        texts
    );

    // === ASSERT: each fragment carries the writer's MCID and tag ===
    let hello = extracted
        .fragments
        .iter()
        .find(|f| f.text == "Hello")
        .expect("fragment for 'Hello'");
    let world = extracted
        .fragments
        .iter()
        .find(|f| f.text == "World")
        .expect("fragment for 'World'");

    assert_eq!(hello.mcid, Some(0), "Hello must carry MCID 0");
    assert_eq!(world.mcid, Some(1), "World must carry MCID 1");
    assert_eq!(
        hello.struct_tag.as_deref(),
        Some("P"),
        "Hello must carry struct_tag 'P'"
    );
    assert_eq!(
        world.struct_tag.as_deref(),
        Some("P"),
        "World must carry struct_tag 'P'"
    );
}
