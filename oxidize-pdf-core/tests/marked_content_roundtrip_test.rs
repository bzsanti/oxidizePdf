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

#[test]
fn writer_actual_text_roundtrips_unicode_and_replaces_visual_glyphs() {
    let mut page = Page::a4();
    let mcid = page
        .begin_marked_content_with_actual_text("Span", "^{40} € 😀")
        .expect("begin ActualText span");
    page.text()
        .set_font(Font::Helvetica, 9.0)
        .at(100.0, 705.0)
        .write("40")
        .expect("write visual superscript glyphs");
    page.end_marked_content().expect("end ActualText span");

    let mut doc = Document::new();
    doc.add_page(page);
    let bytes = doc.to_bytes().expect("serialize PDF");
    let reader = PdfReader::new(Cursor::new(bytes)).expect("read generated PDF");
    let document = PdfDocument::new(reader);
    let mut extractor = TextExtractor::with_options(ExtractionOptions {
        preserve_layout: true,
        ..Default::default()
    });
    let page = extractor
        .extract_from_page(&document, 0)
        .expect("extract generated page");

    assert_eq!(mcid, 0);
    assert!(
        page.fragments
            .iter()
            .any(|fragment| fragment.text == "^{40} € 😀" && fragment.mcid == Some(mcid)),
        "ActualText must replace the visual glyphs and retain its MCID: {:?}",
        page.fragments
    );
    assert!(
        page.fragments.iter().all(|fragment| fragment.text != "40"),
        "visual glyphs must not leak into logical extraction"
    );
}

#[test]
fn writer_actual_text_is_emitted_by_flat_extraction() {
    let mut page = Page::a4();
    page.begin_marked_content_with_actual_text("Span", "H_{2}O")
        .expect("begin ActualText span");
    page.text()
        .set_font(Font::Helvetica, 9.0)
        .at(100.0, 705.0)
        .write("H2O")
        .expect("write visual glyphs");
    page.end_marked_content().expect("end ActualText span");

    let mut doc = Document::new();
    doc.add_page(page);
    let bytes = doc.to_bytes().expect("serialize PDF");
    let reader = PdfReader::new(Cursor::new(bytes)).expect("read generated PDF");
    let document = PdfDocument::new(reader);
    let mut extractor = TextExtractor::new();
    let extracted = extractor
        .extract_from_page(&document, 0)
        .expect("flat extraction succeeds");

    assert_eq!(extracted.text, "H_{2}O");
    assert!(!extracted.text.contains("H2O"));
}

#[test]
fn writer_rejects_invalid_marked_content_tags_without_consuming_mcid() {
    let mut page = Page::a4();
    assert!(page
        .begin_marked_content_with_actual_text("Span /P", "unsafe")
        .is_err());
    assert!(page.begin_marked_content("also invalid").is_err());

    let mcid = page
        .begin_marked_content_with_actual_text("Span", "safe")
        .expect("valid tag");
    assert_eq!(mcid, 0, "invalid tags must not consume an MCID");
}
