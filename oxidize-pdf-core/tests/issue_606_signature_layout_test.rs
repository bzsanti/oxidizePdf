use oxidize_pdf::signatures::{
    prepare_incremental_signature_with_appearance, SignatureAppearance,
    SignaturePreparationOptions, SignatureRect, SignatureTarget, SignatureWatermark,
};
use oxidize_pdf::text::{escape_pdf_string_literal, measure_text, Font, TextEncoding};
use oxidize_pdf::{Document, Page};

fn rect(width: f64, height: f64) -> SignatureRect {
    SignatureRect {
        left: 10.0,
        bottom: 20.0,
        right: 10.0 + width,
        top: 20.0 + height,
    }
}

fn watermark() -> SignatureWatermark {
    SignatureWatermark {
        width: 2,
        height: 1,
        rgb: vec![255, 0, 0, 0, 128, 255],
        opacity: 0.25,
    }
}

fn options(rect: SignatureRect) -> SignaturePreparationOptions {
    let mut options = SignaturePreparationOptions::invisible("Approval");
    options.target = SignatureTarget::New {
        field_name: "Approval".into(),
        page_index: 0,
        rect: Some(rect),
    };
    options
}

fn base() -> Vec<u8> {
    let mut document = Document::new();
    document.add_page(Page::a4());
    document.to_bytes().unwrap()
}

#[test]
fn logo_is_centered_behind_text_and_does_not_reserve_a_column() {
    let area = rect(300.0, 100.0);
    let mut appearance = SignatureAppearance {
        signer_name: Some("MARIA DEL CARMEN FERNANDEZ".into()),
        signing_date: Some("2026-09-20".into()),
        text: vec!["Approved".into()],
        watermark: None,
    };
    let plain = appearance.layout(area).unwrap();
    let plain_pdf =
        prepare_incremental_signature_with_appearance(&base(), &options(area), &appearance)
            .unwrap();
    appearance.watermark = Some(watermark());
    assert_eq!(appearance.layout(area).unwrap(), plain);
    let marked_pdf =
        prepare_incremental_signature_with_appearance(&base(), &options(area), &appearance)
            .unwrap();
    let marked = String::from_utf8_lossy(marked_pdf.prepared_pdf());
    let text_ops = |pdf: &[u8]| {
        String::from_utf8_lossy(pdf)
            .lines()
            .filter(|line| line.starts_with("BT /F1"))
            .map(str::to_owned)
            .collect::<Vec<_>>()
    };
    assert_eq!(
        text_ops(plain_pdf.prepared_pdf()),
        text_ops(marked_pdf.prepared_pdf())
    );
    // A 2:1 logo scales to 152 x 76 in a 300 x 100 widget, centered at (150,50).
    assert!(marked.contains("152 0 0 76 74 12 cm /Im0 Do Q"));
    assert!(marked.contains("/ca 0.25"));
    assert!(marked.find("/Im0 Do").unwrap() < marked.find("BT /F1").unwrap());
    assert!(!marked.contains(" re W n"), "fit must not rely on clipping");
}

#[test]
fn long_names_and_additional_lines_wrap_without_losing_content() {
    let area = rect(160.0, 100.0);
    let appearance = SignatureAppearance {
        signer_name: Some("MARIA DEL CARMEN FERNÁNDEZ DE LA CRUZ".into()),
        signing_date: Some("2026-09-20".into()),
        text: vec![
            "Reviewed and approved by the document owner".into(),
            "Reference: ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789".into(),
        ],
        watermark: Some(watermark()),
    };
    let layout = appearance.layout(area).unwrap();
    assert!(layout.lines.len() > 4);
    let original = appearance
        .signer_name
        .iter()
        .chain(appearance.signing_date.iter())
        .chain(appearance.text.iter())
        .cloned()
        .collect::<String>();
    assert_eq!(layout.lines.concat(), original);
    assert!((6.0..=12.0).contains(&layout.font_size));
    for (i, line) in layout.lines.iter().enumerate() {
        assert!(
            measure_text(line, &Font::Helvetica, layout.font_size)
                <= 160.0 - 2.0 * layout.margin + 1e-8
        );
        let baseline = layout.first_baseline - i as f64 * layout.line_height;
        assert!(baseline - 0.25 * layout.font_size >= layout.margin - 1e-8);
        assert!(baseline + layout.font_size <= 100.0 - layout.margin + 1e-8);
    }
    let prepared =
        prepare_incremental_signature_with_appearance(&base(), &options(area), &appearance)
            .unwrap();
    let digest = prepared.bytes_to_digest();
    let signed_content = String::from_utf8_lossy(&digest);
    for (i, line) in layout.lines.iter().enumerate() {
        let literal =
            escape_pdf_string_literal(&TextEncoding::WinAnsiEncoding.encode_strict(line).unwrap());
        let y = layout.first_baseline - i as f64 * layout.line_height;
        assert!(signed_content.contains(&format!(
            "BT /F1 {} Tf 0 g {} {y} Td ({literal}) Tj ET",
            layout.font_size, layout.margin
        )));
    }
    assert!(digest.windows(6).any(|bytes| bytes == watermark().rgb));
}

#[test]
fn width_measurement_distinguishes_wide_and_narrow_glyphs() {
    let wide = SignatureAppearance {
        text: vec!["WWWWWW".into()],
        ..Default::default()
    };
    let narrow = SignatureAppearance {
        text: vec!["iiiiii".into()],
        ..Default::default()
    };
    let area = rect(40.0, 20.0);
    let wide = wide.layout(area).unwrap();
    let narrow = narrow.layout(area).unwrap();
    assert_eq!(wide.lines, ["WWWWWW"]);
    assert_eq!(wide.font_size, 6.0); // 6 * 944/1000 * 6 = 33.984, available width 34.
    assert!(narrow.font_size > wide.font_size);
}

#[test]
fn minimum_size_fits_with_descenders_and_rejects_just_too_short() {
    let appearance = SignatureAppearance {
        text: vec!["g".into(), "g".into()],
        ..Default::default()
    };
    let fits = appearance.layout(rect(60.0, 21.0)).unwrap();
    assert_eq!(fits.font_size, 6.0);
    assert_eq!(fits.lines, ["g", "g"]);
    let error = appearance.layout(rect(60.0, 20.9)).unwrap_err();
    assert!(error.to_string().contains("minimum font size of 6 points"));
}

#[test]
fn impossible_layout_fails_preflight_and_preparation_before_pdf_access() {
    for area in [rect(10.0, 100.0), rect(300.0, 10.0)] {
        let appearance = SignatureAppearance {
            signer_name: Some("WWWWWW".into()),
            ..Default::default()
        };
        let expected = appearance.layout(area).unwrap_err();
        // Invalid source bytes prove layout validation happens before PDF parsing or signing.
        let actual = prepare_incremental_signature_with_appearance(
            b"not a PDF",
            &options(area),
            &appearance,
        )
        .unwrap_err();
        assert_eq!(actual, expected);
        assert!(actual.to_string().contains("signature appearance layout"));
    }
    let appearance = SignatureAppearance {
        text: vec!["An additional line".into(); 20],
        watermark: Some(watermark()),
        ..Default::default()
    };
    let area = rect(300.0, 100.0);
    let expected = appearance.layout(area).unwrap_err();
    let actual =
        prepare_incremental_signature_with_appearance(&base(), &options(area), &appearance)
            .unwrap_err();
    assert_eq!(actual, expected, "excess lines must fail, never disappear");
}

#[test]
fn hard_breaks_blank_lines_and_long_tokens_are_preserved() {
    let appearance = SignatureAppearance {
        text: vec!["First\r\n\r\nABCDEFGHIJKLMN\rLast\tline".into()],
        ..Default::default()
    };
    let layout = appearance.layout(rect(55.0, 120.0)).unwrap();
    assert_eq!(layout.lines[0], "First");
    assert_eq!(layout.lines[1], "");
    assert_eq!(layout.lines.concat(), "FirstABCDEFGHIJKLMNLast line");
    assert!(layout.lines.len() > 4);
}

#[test]
fn preflight_validates_watermark_encoding_controls_and_finite_dimensions() {
    let mut appearance = SignatureAppearance {
        text: vec!["Valid".into()],
        watermark: Some(watermark()),
        ..Default::default()
    };
    appearance.watermark.as_mut().unwrap().opacity = f32::NAN;
    assert!(appearance.layout(rect(300.0, 100.0)).is_err());
    appearance.watermark = None;
    for text in ["東", "bad\0text", ""] {
        appearance.text = vec![text.into()];
        assert!(appearance.layout(rect(300.0, 100.0)).is_err());
    }
    appearance.text = vec!["Valid".into()];
    assert!(appearance
        .layout(SignatureRect {
            left: -f64::MAX,
            right: f64::MAX,
            bottom: 0.0,
            top: 100.0
        })
        .is_err());
}
