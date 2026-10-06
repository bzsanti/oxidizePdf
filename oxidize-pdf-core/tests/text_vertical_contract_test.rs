//! #666: vertical text pen advances are distinct from glyph placement offsets.
#[path = "common/text_contracts.rs"]
mod contract;
use contract::assembler::stream_obj;
use contract::{cmap, pdf};
use oxidize_pdf::parser::{ParseOptions, PdfReader};
use oxidize_pdf::text::{ExtractionOptions, TextExtractor};
use std::io::Cursor;
const FONT: &[u8] = include_bytes!("fixtures/text_contracts/fonts/SourceSans3-Regular.ttf");

fn vertical_pdf(metrics: &str, embedded: bool, content: &[u8]) -> Vec<u8> {
    vertical_pdf_with_unicode(metrics, embedded, content, "<0011> <0041>\n<0013> <0042>")
}

fn vertical_pdf_with_unicode(
    metrics: &str,
    embedded: bool,
    content: &[u8],
    mapping: &str,
) -> Vec<u8> {
    let definition=format!("<< /Type /Font /Subtype /Type0 /BaseFont /SourceSans3-Regular /Encoding {} /DescendantFonts [6 0 R] /ToUnicode 9 0 R >>",if embedded {"11 0 R"} else {"/Identity-V"});
    let descendant=format!("<< /Type /Font /Subtype /CIDFontType2 /BaseFont /SourceSans3-Regular /CIDSystemInfo << /Registry (Contract) /Ordering (Vertical) /Supplement 0 >> /FontDescriptor 7 0 R /CIDToGIDMap 10 0 R /W [17 [544] 19 [588]] {metrics} >>");
    let descriptor=b"<< /Type /FontDescriptor /FontName /SourceSans3-Regular /Flags 4 /FontBBox [-614 -295 2159 958] /ItalicAngle 0 /Ascent 984 /Descent -273 /CapHeight 660 /StemV 80 /FontFile2 8 0 R >>".to_vec();
    let unicode = cmap(mapping, 2, "<0000> <FFFF>");
    let mut gids = vec![0u8; 40];
    gids[34..36].copy_from_slice(&2u16.to_be_bytes());
    gids[38..40].copy_from_slice(&3u16.to_be_bytes());
    let encoding=stream_obj("/Type /CMap /CMapName /ContractVertical /CIDSystemInfo << /Registry (Contract) /Ordering (Vertical) /Supplement 0 >> /WMode 1",
        b"/CIDInit /ProcSet findresource begin 12 dict begin begincmap /CIDSystemInfo << /Registry (Contract) /Ordering (Vertical) /Supplement 0 >> def /CMapName /ContractVertical def /CMapType 1 def /WMode 1 def 1 begincodespacerange <0000> <FFFF> endcodespacerange 2 begincidchar <0011> 17 <0013> 19 endcidchar endcmap CMapName currentdict /CMap defineresource pop end end");
    pdf(
        &definition,
        content,
        vec![
            descendant.into_bytes(),
            descriptor,
            stream_obj(&format!("/Length1 {}", FONT.len()), FONT),
            unicode,
            stream_obj("", &gids),
            encoding,
        ],
    )
}

fn pen_failures(
    metrics: &str,
    embedded: bool,
    content: &[u8],
    expected: &[(f64, f64)],
) -> Vec<String> {
    let document = PdfReader::new_with_options(
        Cursor::new(vertical_pdf(metrics, embedded, content)),
        ParseOptions::strict(),
    )
    .expect("vertical fixture")
    .into_document();
    let result = TextExtractor::with_options(ExtractionOptions {
        preserve_layout: true,
        sort_by_position: false,
        ..ExtractionOptions::default()
    })
    .extract_from_page(&document, 0)
    .expect("extract vertical text");
    assert_eq!(
        result.fragments.len(),
        expected.len(),
        "fragments: {:?}",
        result.fragments
    );
    let mut failures = Vec::new();
    for (index, (fragment, (x, y))) in result.fragments.iter().zip(expected).enumerate() {
        let text = if index == 1 { "B" } else { "A" };
        if fragment.text != text
            || (fragment.x - x).abs() > 0.0001
            || (fragment.y - y).abs() > 0.0001
        {
            failures.push(format!("glyph {index}, embedded={embedded}, metrics={metrics}: expected {text} pen=({x},{y}), actual {:?}=({},{})",fragment.text,fragment.x,fragment.y));
        }
    }
    failures
}
fn assert_pen_positions(metrics: &str, embedded: bool, content: &[u8], expected: &[(f64, f64)]) {
    let failures = pen_failures(metrics, embedded, content, expected);
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
const TWO: &[u8] = b"BT /F1 10 Tf 100 700 Td <0011> Tj 1 Tr <0013> Tj ET";
#[test]
fn identity_v_uses_default_vertical_advance() {
    assert_pen_positions("", false, TWO, &[(100.0, 700.0), (100.0, 690.0)]);
}
#[test]
fn embedded_wmode_one_selects_vertical_advance() {
    assert_pen_positions("", true, TWO, &[(100.0, 700.0), (100.0, 690.0)]);
}
#[test]
fn dw2_overrides_default_vertical_advance() {
    assert_pen_positions(
        "/DW2 [900 -1300]",
        false,
        TWO,
        &[(100.0, 700.0), (100.0, 687.0)],
    );
}
#[test]
fn w2_array_overrides_dw2_for_the_selected_cid() {
    assert_pen_positions(
        "/DW2 [900 -1300] /W2 [17 [-1200 272 880]]",
        false,
        TWO,
        &[(100.0, 700.0), (100.0, 688.0)],
    );
}
#[test]
fn w2_range_overrides_dw2_for_the_selected_cid() {
    assert_pen_positions(
        "/DW2 [900 -1300] /W2 [17 19 -1200 272 880]",
        false,
        TWO,
        &[(100.0, 700.0), (100.0, 688.0)],
    );
}
#[test]
fn missing_w2_entry_falls_back_to_dw2() {
    assert_pen_positions(
        "/DW2 [900 -1300] /W2 [17 [-1200 272 880]]",
        false,
        b"BT /F1 10 Tf 100 700 Td <0011> Tj 1 Tr <0013> Tj 0 Tr <0011> Tj ET",
        &[(100.0, 700.0), (100.0, 688.0), (100.0, 675.0)],
    );
}
#[test]
fn vertical_tj_adjustment_changes_y_with_the_pdf_sign() {
    let mut failures = Vec::new();
    for (adjustment, y) in [(300, 687.0), (-300, 693.0)] {
        let content = format!("BT /F1 10 Tf 100 700 Td [<0011> {adjustment}] TJ 1 Tr <0013> Tj ET");
        failures.extend(pen_failures(
            "",
            false,
            content.as_bytes(),
            &[(100.0, 700.0), (100.0, y)],
        ));
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
#[test]
fn horizontal_scale_does_not_scale_vertical_pen_advance() {
    let mut failures = Vec::new();
    for scale in [50, 200] {
        let content = format!("BT /F1 10 Tf {scale} Tz 100 700 Td <0011> Tj 1 Tr <0013> Tj ET");
        failures.extend(pen_failures(
            "",
            false,
            content.as_bytes(),
            &[(100.0, 700.0), (100.0, 690.0)],
        ));
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
#[test]
fn character_spacing_applies_on_the_vertical_axis() {
    assert_pen_positions(
        "",
        false,
        b"BT /F1 10 Tf 2 Tc 100 700 Td <0011> Tj 1 Tr <0013> Tj ET",
        &[(100.0, 700.0), (100.0, 692.0)],
    );
}

#[test]
fn word_spacing_does_not_apply_to_two_byte_source_codes() {
    assert_pen_positions(
        "",
        false,
        b"BT /F1 10 Tf 100 Tw 100 700 Td <0011> Tj 1 Tr <0013> Tj ET",
        &[(100.0, 700.0), (100.0, 690.0)],
    );
}
#[test]
fn vertical_tj_ignores_horizontal_scale_for_both_signs() {
    for (adjustment, y) in [(300, 687.0), (-300, 693.0)] {
        for scale in [50, 200] {
            let content = format!(
                "BT /F1 10 Tf {scale} Tz 100 700 Td [<0011> {adjustment}] TJ 1 Tr <0013> Tj ET"
            );
            assert_pen_positions("", false, content.as_bytes(), &[(100.0, 700.0), (100.0, y)]);
        }
    }
}
#[test]
fn vertical_pen_follows_rotated_ctm() {
    assert_pen_positions(
        "",
        false,
        b"0 1 -1 0 0 0 cm BT /F1 10 Tf 100 700 Td <0011> Tj 1 Tr <0013> Tj ET",
        &[(-700.0, 100.0), (-690.0, 100.0)],
    );
}
#[test]
fn font_size_changes_apply_to_subsequent_vertical_advances() {
    assert_pen_positions(
        "",
        false,
        b"BT /F1 10 Tf 100 700 Td <0011> Tj /F1 20 Tf 1 Tr <0013> Tj /F1 10 Tf 0 Tr <0011> Tj ET",
        &[(100.0, 700.0), (100.0, 690.0), (100.0, 670.0)],
    );
}
#[test]
fn w2_array_uses_selected_cid_at_each_position() {
    assert_pen_positions(
        "/DW2 [880 -1500] /W2 [17 [-1200 272 880 -900 272 880 -700 294 880]]",
        false,
        b"BT /F1 10 Tf 100 700 Td <0011> Tj 1 Tr <0013> Tj 0 Tr <0011> Tj ET",
        &[(100.0, 700.0), (100.0, 688.0), (100.0, 681.0)],
    );
}

#[test]
fn vertical_multiscalar_unicode_advances_once_per_source_code() {
    for options in [ParseOptions::strict(), ParseOptions::lenient()] {
        let bytes = vertical_pdf_with_unicode(
            "",
            false,
            b"BT /F1 10 Tf 100 700 Td <00110013> Tj 1 Tr <0011> Tj ET",
            "<0011> <00660069> <0013> <D83DDE00>",
        );
        let doc = PdfReader::new_with_options(Cursor::new(bytes), options)
            .unwrap()
            .into_document();
        let result = TextExtractor::with_options(ExtractionOptions {
            preserve_layout: true,
            sort_by_position: false,
            ..ExtractionOptions::default()
        })
        .extract_from_page(&doc, 0)
        .unwrap();
        assert_eq!(result.fragments.len(), 2);
        assert_eq!(result.fragments[0].text, "fi😀");
        assert_eq!(result.fragments[1].text, "fi");
        assert_eq!(
            (result.fragments[0].x, result.fragments[0].y),
            (100.0, 700.0)
        );
        assert_eq!(
            (result.fragments[1].x, result.fragments[1].y),
            (100.0, 680.0),
            "two source glyphs, not three Unicode scalars"
        );
    }
}
