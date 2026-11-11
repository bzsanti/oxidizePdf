//! Regression coverage for issue #861: opt-in extraction of text drawn by
//! annotation appearance streams (`/AP /N`), such as signature widgets and
//! filled form fields.

mod common;

use common::pdf_assembler::{assemble_pdf, stream_obj};
use oxidize_pdf::parser::PdfReader;
use oxidize_pdf::text::{ExtractionOptions, RecoveryLocation, TextExtractor};
use std::io::Cursor;

const HELVETICA: &[u8] =
    b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>";

/// One-page PDF with body text "Body" and the given `/Annots` array content.
/// Object 5 is Helvetica; callers append extra objects starting at 6.
fn fixture(annots: &str, extra: &[Vec<u8>]) -> Vec<u8> {
    let mut objects = vec![
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        format!(
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 300 300] \
             /Resources << /Font << /F1 5 0 R >> >> /Contents 4 0 R /Annots [{annots}] >>"
        )
        .into_bytes(),
        stream_obj("", b"BT /F1 12 Tf 20 250 Td (Body) Tj ET"),
        HELVETICA.to_vec(),
    ];
    objects.extend_from_slice(extra);
    assemble_pdf(&objects)
}

fn appearance(content: &[u8]) -> Vec<u8> {
    stream_obj(
        "/Type /XObject /Subtype /Form /BBox [0 0 100 20] /Resources << /Font << /F1 5 0 R >> >>",
        content,
    )
}

fn extract(pdf: Vec<u8>, extractor: TextExtractor) -> oxidize_pdf::text::ExtractedText {
    let document = PdfReader::new(Cursor::new(pdf))
        .expect("parse fixture")
        .into_document();
    let mut extractor = extractor;
    extractor
        .extract_from_page(&document, 0)
        .expect("extract page")
}

fn enabled() -> TextExtractor {
    TextExtractor::new().with_annotation_appearance_extraction(true)
}

#[test]
fn appearance_text_is_extracted_only_when_enabled() {
    let pdf = fixture(
        "<< /Type /Annot /Subtype /Widget /FT /Tx /F 4 /Rect [20 100 120 120] /AP << /N 6 0 R >> >>",
        &[appearance(b"BT /F1 10 Tf 2 5 Td (Field value) Tj ET")],
    );

    let default = extract(pdf.clone(), TextExtractor::new());
    assert_eq!(default.text, "Body");

    let with_appearances = extract(pdf, enabled());
    assert_eq!(with_appearances.text, "Body\nField value");
    assert!(!with_appearances.truncated);
}

#[test]
fn signature_widget_with_nested_form_xobjects_is_extracted() {
    // Mirrors common signing tools: /AP /N paints /FRM, which paints the
    // text layer /n2 — two levels of `Do` below the appearance stream.
    let pdf = fixture(
        "6 0 R",
        &[
            b"<< /Type /Annot /Subtype /Widget /FT /Sig /F 132 /T (Signature1) \
              /Rect [150 20 290 60] /AP << /N 7 0 R >> >>"
                .to_vec(),
            stream_obj(
                "/Type /XObject /Subtype /Form /BBox [0 0 140 40] \
                 /Resources << /XObject << /FRM 8 0 R >> >>",
                b"q 1 0 0 1 0 0 cm /FRM Do Q",
            ),
            stream_obj(
                "/Type /XObject /Subtype /Form /BBox [0 0 140 40] \
                 /Resources << /XObject << /n2 9 0 R >> >>",
                b"q 1 0 0 1 0 0 cm /n2 Do Q",
            ),
            stream_obj(
                "/Type /XObject /Subtype /Form /BBox [0 0 140 40] \
                 /Resources << /Font << /F1 5 0 R >> >>",
                b"BT /F1 8 Tf 2 25 Td (Digitally signed by Jane Doe) Tj 0 -10 Td (ID 000.000.000-00) Tj ET",
            ),
        ],
    );

    let text = extract(pdf, enabled()).text;
    assert!(text.starts_with("Body\n"), "{text:?}");
    assert!(text.contains("Digitally signed by Jane Doe"), "{text:?}");
    assert!(text.contains("ID 000.000.000-00"), "{text:?}");
}

#[test]
fn appearance_subdictionary_uses_the_appearance_state() {
    let pdf = fixture(
        "<< /Type /Annot /Subtype /Widget /FT /Btn /Rect [20 100 120 120] /AS /On \
           /AP << /N << /On 6 0 R /Off 7 0 R >> >> >>",
        &[
            appearance(b"BT /F1 10 Tf 2 5 Td (Chosen state) Tj ET"),
            appearance(b"BT /F1 10 Tf 2 5 Td (Other state) Tj ET"),
        ],
    );

    let text = extract(pdf, enabled()).text;
    assert_eq!(text, "Body\nChosen state");
}

#[test]
fn appearance_subdictionary_without_a_matching_state_is_skipped() {
    let pdf = fixture(
        "<< /Type /Annot /Subtype /Widget /Rect [20 100 120 120] /AP << /N << /On 6 0 R >> >> >> \
         << /Type /Annot /Subtype /Widget /Rect [20 100 120 120] /AS /Missing /AP << /N << /On 6 0 R >> >> >>",
        &[appearance(b"BT /F1 10 Tf 2 5 Td (Never shown) Tj ET")],
    );

    assert_eq!(extract(pdf, enabled()).text, "Body");
}

#[test]
fn link_popup_and_hidden_annotations_are_skipped() {
    let pdf = fixture(
        "<< /Type /Annot /Subtype /Link /Rect [0 0 100 20] /AP << /N 6 0 R >> >> \
         << /Type /Annot /Subtype /Popup /Rect [0 0 100 20] /AP << /N 6 0 R >> >> \
         << /Type /Annot /Subtype /FreeText /F 2 /Rect [0 0 100 20] /AP << /N 6 0 R >> >> \
         << /Type /Annot /Subtype /FreeText /F 32 /Rect [0 0 100 20] /AP << /N 7 0 R >> >>",
        &[
            appearance(b"BT /F1 10 Tf 2 5 Td (Skipped) Tj ET"),
            appearance(b"BT /F1 10 Tf 2 5 Td (NoView included) Tj ET"),
        ],
    );

    assert_eq!(extract(pdf, enabled()).text, "Body\nNoView included");
}

#[test]
fn annotations_keep_array_order() {
    let pdf = fixture(
        "<< /Subtype /FreeText /Rect [20 100 120 120] /AP << /N 6 0 R >> >> \
         << /Subtype /FreeText /Rect [20 200 120 220] /AP << /N 7 0 R >> >>",
        &[
            appearance(b"BT /F1 10 Tf 2 5 Td (First) Tj ET"),
            appearance(b"BT /F1 10 Tf 2 5 Td (Second) Tj ET"),
        ],
    );

    assert_eq!(extract(pdf, enabled()).text, "Body\nFirst\nSecond");
}

#[test]
fn malformed_appearances_do_not_fail_the_page() {
    let pdf = fixture(
        "<< /Subtype /Widget /Rect [0 0 10 10] /AP 99 0 R >> \
         << /Subtype /Widget /Rect [0 0 10 10] /AP << /N 99 0 R >> >> \
         << /Subtype /Widget /Rect [0 0 10 10] /AP << /N 6 0 R >> >> \
         << /Subtype /Widget /Rect [0 0 10 10] /AP (not a dictionary) >> \
         << /Subtype /Widget >> \
         << /Subtype /Widget /Rect [20 100 120 120] /AP << /N 7 0 R >> >>",
        &[
            // A dictionary where a stream is required.
            b"<< /Type /XObject /Subtype /Form >>".to_vec(),
            appearance(b"BT /F1 10 Tf 2 5 Td (Still here) Tj ET"),
        ],
    );

    assert_eq!(extract(pdf, enabled()).text, "Body\nStill here");
}

#[test]
fn appearance_text_respects_the_byte_budget() {
    let pdf = fixture(
        "<< /Subtype /FreeText /Rect [20 100 120 120] /AP << /N 6 0 R >> >>",
        &[appearance(b"BT /F1 10 Tf 2 5 Td (Overflowing note) Tj ET")],
    );
    let budgeted = |limit| {
        TextExtractor::with_options(ExtractionOptions {
            max_extracted_bytes: Some(limit),
            ..Default::default()
        })
        .with_annotation_appearance_extraction(true)
    };

    let exact = extract(pdf.clone(), budgeted("Body\nOverflowing note".len()));
    assert_eq!(exact.text, "Body\nOverflowing note");
    assert!(!exact.truncated);

    // Text runs are budgeted whole, as on the page itself: a run that does
    // not fit is dropped and the page is marked truncated.
    let short = extract(pdf, budgeted("Body\nOverflowing note".len() - 1));
    assert_eq!(short.text, "Body");
    assert!(short.truncated);
}

#[test]
fn appearance_is_placed_at_the_annotation_rect_in_layout_mode() {
    // /BBox [0 0 100 20] is mapped onto /Rect [50 150 250 190]: scale 2x, so a
    // glyph at form (2, 5) lands at page (54, 160).
    let pdf = fixture(
        "<< /Subtype /FreeText /Rect [50 150 250 190] /AP << /N 6 0 R >> >>",
        &[appearance(b"BT /F1 10 Tf 2 5 Td (Placed) Tj ET")],
    );
    let extracted = extract(
        pdf,
        TextExtractor::with_options(ExtractionOptions {
            preserve_layout: true,
            ..Default::default()
        })
        .with_annotation_appearance_extraction(true),
    );

    let placed = extracted
        .fragments
        .iter()
        .find(|f| f.text.contains("Placed"))
        .expect("appearance fragment");
    assert!((placed.x - 54.0).abs() < 0.01, "x = {}", placed.x);
    assert!((placed.y - 160.0).abs() < 0.01, "y = {}", placed.y);
    assert!(
        (placed.font_size - 20.0).abs() < 0.01,
        "size = {}",
        placed.font_size
    );
}

#[test]
fn stops_at_the_first_annotation_once_the_budget_is_spent() {
    // The first appearance fills the budget exactly; the second is never run.
    let pdf = fixture(
        "<< /Subtype /FreeText /Rect [20 100 120 120] /AP << /N 6 0 R >> >> \
         << /Subtype /FreeText /Rect [20 200 120 220] /AP << /N 7 0 R >> >>",
        &[
            appearance(b"BT /F1 10 Tf 2 5 Td (First) Tj 0 -12 Td (overflow) Tj ET"),
            appearance(b"BT /F1 10 Tf 2 5 Td (Second) Tj ET"),
        ],
    );
    let extracted = extract(
        pdf,
        TextExtractor::with_options(ExtractionOptions {
            max_extracted_bytes: Some("Body\nFirst".len()),
            ..Default::default()
        })
        .with_annotation_appearance_extraction(true),
    );

    assert_eq!(extracted.text, "Body\nFirst");
    assert!(extracted.truncated);
}

#[test]
fn undecodable_appearance_stream_is_skipped() {
    // Declared FlateDecode, but the bytes are not a zlib stream.
    let pdf = fixture(
        "<< /Subtype /FreeText /Rect [20 100 120 120] /AP << /N 6 0 R >> >> \
         << /Subtype /FreeText /Rect [20 200 120 220] /AP << /N 7 0 R >> >>",
        &[
            stream_obj(
                "/Type /XObject /Subtype /Form /BBox [0 0 100 20] /Filter /FlateDecode",
                b"definitely not zlib data",
            ),
            appearance(b"BT /F1 10 Tf 2 5 Td (Readable) Tj ET"),
        ],
    );

    let strict_document = PdfReader::new(Cursor::new(pdf.clone()))
        .unwrap()
        .into_document();
    assert!(enabled().extract_from_page(&strict_document, 0).is_err());

    // The recovery API reports the omitted appearance instead of failing.
    let document = PdfReader::new(Cursor::new(pdf))
        .expect("parse fixture")
        .into_document();
    let recovered = enabled()
        .extract_from_page_with_recovery(&document, 0, 1 << 20)
        .expect("extract with recovery");
    assert_eq!(recovered.text.text, "Body\nReadable");
    assert!(recovered.diagnostics.iter().any(|d| matches!(
        d.location,
        RecoveryLocation::AnnotationAppearance { object: (6, 0) }
    )));
}

#[test]
fn indirect_appearance_resources_are_resolved() {
    let pdf = fixture(
        "<< /Subtype /FreeText /Rect [20 100 120 120] /AP << /N 6 0 R >> >>",
        &[
            stream_obj(
                "/Type /XObject /Subtype /Form /BBox [0 0 100 20] /Resources 7 0 R",
                b"BT /F1 10 Tf 2 5 Td (Indirect resources) Tj ET",
            ),
            b"<< /Font << /F1 5 0 R >> >>".to_vec(),
        ],
    );

    assert_eq!(extract(pdf, enabled()).text, "Body\nIndirect resources");
}

#[test]
fn reading_order_keeps_appearance_groups_after_the_page() {
    // Two page lines and a two-line appearance: the appearance's line groups
    // must be shifted to its position in the page text.
    let pdf = assemble_pdf(&[
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 300 300] \
          /Resources << /Font << /F1 5 0 R >> >> /Contents 4 0 R \
          /Annots [<< /Subtype /FreeText /Rect [20 40 120 80] /AP << /N 6 0 R >> >>] >>"
            .to_vec(),
        stream_obj(
            "",
            b"BT /F1 12 Tf 20 250 Td (Page one) Tj 0 -20 Td (Page two) Tj ET",
        ),
        HELVETICA.to_vec(),
        stream_obj(
            "/Type /XObject /Subtype /Form /BBox [0 0 100 40] /Resources << /Font << /F1 5 0 R >> >>",
            b"BT /F1 10 Tf 2 25 Td (Note one) Tj 0 -15 Td (Note two) Tj ET",
        ),
    ]);

    let text = extract(pdf, enabled().with_reading_order(true)).text;
    assert_eq!(text, "Page one\nPage two\nNote one\nNote two");
}

#[test]
fn malformed_geometry_and_appearance_entries_are_skipped() {
    let pdf = fixture(
        // /N is neither a stream reference nor a state dictionary.
        "<< /Subtype /FreeText /Rect [0 0 10 10] /AP << /N /NotAStream >> >> \
         << /Subtype /FreeText /Rect [0 0 10] /AP << /N 7 0 R >> >> \
         << /Subtype /FreeText /Rect [0 0 10 (x)] /AP << /N 7 0 R >> >> \
         << /Subtype /FreeText /Rect [20 100 120 120] /AP << /N 6 0 R >> >>",
        &[
            // /Matrix with the wrong arity is ignored (identity), and a
            // zero-width /BBox keeps unit scale instead of dividing by zero.
            stream_obj(
                "/Type /XObject /Subtype /Form /BBox [0 0 0 20] /Matrix [1 0 0 1] \
                 /Resources << /Font << /F1 5 0 R >> >>",
                b"BT /F1 10 Tf 2 5 Td (Degenerate box) Tj ET",
            ),
            appearance(b"BT /F1 10 Tf 2 5 Td (Never shown) Tj ET"),
        ],
    );

    let extracted = extract(
        pdf,
        TextExtractor::with_options(ExtractionOptions {
            preserve_layout: true,
            ..Default::default()
        })
        .with_annotation_appearance_extraction(true),
    );
    assert!(
        !extracted.text.contains("Never shown"),
        "{:?}",
        extracted.text
    );
    let placed = extracted
        .fragments
        .iter()
        .find(|f| f.text.contains("Degenerate box"))
        .expect("degenerate appearance still extracted");
    // Unit x-scale: form x=2 lands at rect x0 + 2.
    assert!((placed.x - 22.0).abs() < 0.01, "x = {}", placed.x);
}

#[test]
fn form_matrix_is_applied_before_fitting_to_the_rect() {
    // /Matrix rotates the form 90°: the BBox [0 0 100 20] becomes a 20x100
    // box, which is then fitted onto a 20x100 /Rect without scaling.
    let pdf = fixture(
        "<< /Subtype /FreeText /Rect [200 100 220 200] /AP << /N 6 0 R >> >>",
        &[stream_obj(
            "/Type /XObject /Subtype /Form /BBox [0 0 100 20] /Matrix [0 1 -1 0 0 0] \
             /Resources << /Font << /F1 5 0 R >> >>",
            b"BT /F1 10 Tf 0 0 Td (Rotated) Tj ET",
        )],
    );
    let extracted = extract(
        pdf,
        TextExtractor::with_options(ExtractionOptions {
            preserve_layout: true,
            ..Default::default()
        })
        .with_annotation_appearance_extraction(true),
    );

    let placed = extracted
        .fragments
        .iter()
        .find(|f| f.text.contains("Rotated"))
        .expect("rotated appearance fragment");
    // Form origin (0,0) maps to (0,0) under the rotation, then the rotated box
    // x-range [-20, 0] shifts onto [200, 220]: origin lands at (220, 100).
    assert!((placed.x - 220.0).abs() < 0.01, "x = {}", placed.x);
    assert!((placed.y - 100.0).abs() < 0.01, "y = {}", placed.y);
}

#[test]
fn graphics_only_appearance_without_resources_contributes_no_text() {
    // An appearance containing only graphics does not contribute text.
    let pdf = fixture(
        "<< /Subtype /FreeText /Rect [20 100 120 120] /AP << /N 6 0 R >> >>",
        &[stream_obj(
            "/Type /XObject /Subtype /Form /BBox [0 0 100 20]",
            b"0 0 1 rg 0 0 100 20 re f",
        )],
    );

    let extracted = extract(pdf, enabled());
    assert_eq!(extracted.text, "Body");
    assert!(!extracted.truncated);
}

#[test]
fn regression_indirect_normal_state_dictionary() {
    let pdf = fixture(
        "<< /Subtype /Widget /Rect [20 100 120 120] /AS /On /AP << /N 6 0 R >> >>",
        &[
            b"<< /On 7 0 R >>".to_vec(),
            appearance(b"BT /F1 10 Tf 2 5 Td (Chosen state) Tj ET"),
        ],
    );
    assert_eq!(extract(pdf, enabled()).text, "Body\nChosen state");
}
#[test]
fn regression_recovery_resource_limit_must_propagate() {
    let mut content = b"BT /F1 10 Tf 2 5 Td (Long note) Tj ET ".to_vec();
    content.extend_from_slice(&vec![b' '; 256]);
    let pdf = fixture(
        "<< /Subtype /FreeText /Rect [20 100 120 120] /AP << /N 6 0 R >> >>",
        &[appearance(&content)],
    );
    let doc = PdfReader::new(Cursor::new(pdf)).unwrap().into_document();
    let result = enabled().extract_from_page_with_recovery(&doc, 0, 128);
    assert!(
        result.is_err(),
        "stream cap must return error, got {result:?}"
    );
}
#[test]
fn regression_layout_does_not_fuse_page_and_annotation() {
    let pdf=assemble_pdf(&[
 b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
 b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
 b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 300 300] /Resources << /Font << /F1 5 0 R >> >> /Contents 4 0 R /Annots [<< /Subtype /FreeText /Rect [20 225 120 245] /AP << /N 6 0 R >> >>] >>".to_vec(),
 stream_obj("",b"BT /F1 10 Tf 20 250 Td (auto-) Tj ET"), HELVETICA.to_vec(),
 appearance(b"BT /F1 10 Tf 0 5 Td (matic) Tj ET")]);
    assert_eq!(extract(pdf.clone(), enabled()).text, "auto-\nmatic");
    let out = extract(
        pdf,
        TextExtractor::with_options(ExtractionOptions {
            preserve_layout: true,
            ..Default::default()
        })
        .with_annotation_appearance_extraction(true),
    );
    assert_eq!(
        out.text, "auto-\nmatic",
        "must not invent one word across independent sources"
    );
}
#[test]
fn regression_missing_resources_with_actual_text() {
    let pdf = fixture(
        "<< /Subtype /FreeText /Rect [20 100 120 120] /AP << /N 6 0 R >> >>",
        &[stream_obj(
            "/Type /XObject /Subtype /Form /BBox [0 0 100 20]",
            b"BT /F1 10 Tf 2 5 Td (Resource-less text) Tj ET",
        )],
    );
    let out = extract(pdf, enabled());
    assert_eq!(
        out.text, "Body\nResource-less text",
        "missing resources preserve the existing unknown-font text fallback"
    );
}
#[test]
fn regression_probe_type0_width_cache_is_scoped_to_each_appearance() {
    // Object 6: an empty AP that loads a narrow Type0 font under local name F1.
    // Object 7: a different AP with simple F1, no declared space width.
    // Objects 8..12: Type0, CID descendant, custom simple font, descriptor, CMap.
    let widths = vec!["500"; 94].join(" ");
    let extra = vec![
        stream_obj(
            "/Type /XObject /Subtype /Form /BBox [0 0 100 20] /Resources << /Font << /F1 8 0 R >> >>",
            b"q Q",
        ),
        stream_obj(
            "/Type /XObject /Subtype /Form /BBox [0 0 100 20] /Resources << /Font << /F1 10 0 R >> >>",
            b"BT /F1 10 Tf 0 5 Td (ab) Tj 12 0 Td (cd) Tj ET",
        ),
        b"<< /Type /Font /Subtype /Type0 /BaseFont /Synthetic /Encoding /Identity-H /DescendantFonts [9 0 R] /ToUnicode 12 0 R >>".to_vec(),
        b"<< /Type /Font /Subtype /CIDFontType2 /BaseFont /Synthetic /CIDToGIDMap /Identity /CIDSystemInfo << /Registry (Adobe) /Ordering (Identity) /Supplement 0 >> /W [1 [500] 9 [200]] /DW 1000 /FontDescriptor 11 0 R >>".to_vec(),
        format!("<< /Type /Font /Subtype /Type1 /BaseFont /CustomSans /Encoding /WinAnsiEncoding /FirstChar 33 /LastChar 126 /Widths [{widths}] >>").into_bytes(),
        b"<< /Type /FontDescriptor /FontName /Synthetic /Flags 32 /FontBBox [0 -200 1000 900] /ItalicAngle 0 /Ascent 900 /Descent -200 /CapHeight 700 /StemV 80 >>".to_vec(),
        stream_obj("", b"begincmap\n1 begincodespacerange <0000> <FFFF> endcodespacerange\n1 beginbfchar <0001> <0041> endbfchar\nendcmap"),
    ];
    let target = "<< /Subtype /FreeText /Rect [20 100 120 120] /AP << /N 7 0 R >> >>";
    let alone = extract(fixture(target, &extra), enabled()).text;
    assert_eq!(
        alone, "Body\nabcd",
        "control: ordinary 0.3em fallback without a space width"
    );
    let with_preceding =
        format!("<< /Subtype /FreeText /Rect [20 200 120 220] /AP << /N 6 0 R >> >> {target}");
    let after_type0 = extract(fixture(&with_preceding, &extra), enabled()).text;
    assert_eq!(
        after_type0, alone,
        "an empty preceding appearance must not change another appearance's font-spacing policy"
    );
}

fn layout_extractors() -> Vec<TextExtractor> {
    vec![
        enabled(),
        enabled().with_reading_order(true),
        TextExtractor::with_options(ExtractionOptions {
            preserve_layout: true,
            ..Default::default()
        })
        .with_annotation_appearance_extraction(true),
        TextExtractor::with_options(ExtractionOptions {
            reorder_columns: true,
            ..Default::default()
        })
        .with_annotation_appearance_extraction(true),
        TextExtractor::with_options(ExtractionOptions {
            preserve_layout: true,
            reconstruct_paragraphs: true,
            ..Default::default()
        })
        .with_annotation_appearance_extraction(true),
    ]
}

#[test]
fn independent_appearances_keep_hyphens_and_array_order_in_every_layout_mode() {
    let pdf = fixture(
        "<< /Subtype /FreeText /Rect [20 150 120 170] /AP << /N 6 0 R >> >> \
         << /Subtype /FreeText /Rect [20 100 120 120] /AP << /N 7 0 R >> >>",
        &[
            appearance(b"BT /F1 10 Tf 0 5 Td (auto-) Tj ET"),
            appearance(b"BT /F1 10 Tf 0 5 Td (matic) Tj ET"),
        ],
    );
    for extractor in layout_extractors() {
        assert_eq!(extract(pdf.clone(), extractor).text, "Body\nauto-\nmatic");
    }
}

#[test]
fn genuine_hyphen_wrap_inside_one_appearance_is_still_repaired() {
    let pdf = fixture(
        "<< /Subtype /FreeText /Rect [20 100 120 120] /AP << /N 6 0 R >> >>",
        &[appearance(
            b"BT /F1 10 Tf 0 25 Td (auto-) Tj 0 -20 Td (matic) Tj ET",
        )],
    );
    for extractor in layout_extractors() {
        assert_eq!(extract(pdf.clone(), extractor).text, "Body\nautomatic");
    }
}

#[test]
fn missing_appearance_resources_do_not_inherit_page_font_names() {
    let pdf = assemble_pdf(&[
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 300 300] /Resources << /Font << /F1 5 0 R >> >> /Contents 4 0 R /Annots [<< /Subtype /Widget /Rect [20 100 120 120] /AP << /N 6 0 R >> >>] >>".to_vec(),
        stream_obj("",b"BT /F1 10 Tf 20 250 Td (A) Tj ET"),
        b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding << /BaseEncoding /WinAnsiEncoding /Differences [65 /X] >> >>".to_vec(),
        stream_obj("/Type /XObject /Subtype /Form /BBox [0 0 100 20]",b"BT /F1 10 Tf 0 5 Td (A) Tj ET"),
    ]);
    assert_eq!(extract(pdf, enabled()).text, "X\nA");
}

#[test]
fn finalized_appearance_text_and_separators_share_the_unicode_budget() {
    let pdf = fixture(
        "<< /Subtype /FreeText /Rect [20 150 120 170] /AP << /N 6 0 R >> >> \
         << /Subtype /FreeText /Rect [20 100 120 120] /AP << /N 7 0 R >> >>",
        &[
            appearance(b"BT /F1 10 Tf 0 5 Td (caf\xe9) Tj ET"),
            appearance(b"BT /F1 10 Tf 0 5 Td (value) Tj ET"),
        ],
    );
    let expected = "Body\ncafé\nvalue";
    for (preserve_layout, reorder_columns) in [(false, false), (true, false), (false, true)] {
        for limit in 0..=expected.len() + 1 {
            let extractor = TextExtractor::with_options(ExtractionOptions {
                preserve_layout,
                reorder_columns,
                max_extracted_bytes: Some(limit),
                ..Default::default()
            })
            .with_annotation_appearance_extraction(true);
            let out = extract(pdf.clone(), extractor);
            assert!(out.text.len() <= limit, "limit={limit}: {:?}", out.text);
            assert!(
                expected.starts_with(&out.text),
                "limit={limit}: {:?}",
                out.text
            );
            assert_eq!(out.truncated, limit < expected.len());
            if limit >= expected.len() {
                assert_eq!(out.text, expected);
            }
        }
    }
}

#[test]
fn stream_cap_errors_propagate_at_both_appearance_and_nested_form_boundaries() {
    let mut long = b"BT /F1 10 Tf 0 5 Td (Note) Tj ET ".to_vec();
    long.extend_from_slice(&[b' '; 256]);
    for nested in [false, true] {
        let root = if nested {
            stream_obj("/Type /XObject /Subtype /Form /BBox [0 0 100 20] /Resources << /XObject << /Nested 7 0 R >> >>",b"/Nested Do")
        } else {
            appearance(&long)
        };
        let pdf = fixture(
            "<< /Subtype /Widget /Rect [20 100 120 120] /AP << /N 6 0 R >> >>",
            &[root, appearance(&long)],
        );
        let doc = PdfReader::new(Cursor::new(pdf)).unwrap().into_document();
        let mut extractor = enabled();
        assert!(extractor
            .extract_from_page_with_recovery(&doc, 0, 128)
            .is_err());
        // Recovery state is cleared on error, and the next normal extraction works.
        assert_eq!(
            extractor.extract_from_page(&doc, 0).unwrap().text,
            "Body\nNote"
        );
    }
}
