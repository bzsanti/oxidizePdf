#![cfg(feature = "ocr-tesseract")]
use oxidize_pdf::text::{OcrOptions, OcrProvider, RustyTesseractConfig, RustyTesseractProvider};
use oxidize_pdf::{Document, Font, Page};
use std::process::Command;

struct LegacyProvider;
impl OcrProvider for LegacyProvider {
    fn process_image(
        &self,
        _: &[u8],
        _: &OcrOptions,
    ) -> oxidize_pdf::text::OcrResult<oxidize_pdf::text::OcrProcessingResult> {
        Ok(oxidize_pdf::text::OcrProcessingResult::new(
            "legacy".into(),
            1.,
            vec![oxidize_pdf::text::OcrTextFragment::new(
                "legacy".into(),
                50.,
                70.,
                20.,
                10.,
                1.,
                10.,
                oxidize_pdf::text::FragmentType::Word,
            )],
            0,
            "legacy".into(),
            "en".into(),
            (1, 1),
        ))
    }
    fn supported_formats(&self) -> Vec<oxidize_pdf::ImageFormat> {
        vec![oxidize_pdf::ImageFormat::Png]
    }
    fn engine_name(&self) -> &str {
        "legacy"
    }
    fn engine_type(&self) -> oxidize_pdf::text::OcrEngine {
        oxidize_pdf::text::OcrEngine::Mock
    }
}

#[test]
fn default_configuration_does_not_restrict_language_alphabet() {
    assert!(!RustyTesseractConfig::default()
        .config_variables
        .contains_key("tessedit_char_whitelist"));
}

#[test]
#[ignore = "requires Tesseract with spa and Poppler; run explicitly in OCR CI"]
fn spanish_recognition_has_real_word_regions_and_confidence() {
    let dir = tempfile::tempdir().unwrap();
    let mut doc = Document::new();
    let mut page = Page::new(1200., 220.);
    page.text()
        .set_font(Font::Helvetica, 52.)
        .at(40., 100.)
        .write("El niño tomó café con azúcar.")
        .unwrap();
    doc.add_page(page);
    doc.save(dir.path().join("text.pdf")).unwrap();
    let status = Command::new("pdftoppm")
        .current_dir(dir.path())
        .args(["-r", "72", "-png", "-singlefile", "text.pdf", "scan"])
        .status()
        .unwrap();
    assert!(status.success());
    let data = std::fs::read(dir.path().join("scan.png")).unwrap();
    let provider = RustyTesseractProvider::with_config(RustyTesseractConfig {
        language: "spa".into(),
        psm: Some(6),
        ..Default::default()
    });
    let result = provider
        .process_image(&data, &OcrOptions::default())
        .unwrap();
    // Independent CLI invocation establishes the raw coordinates and confidence.
    let tsv = Command::new("tesseract")
        .current_dir(dir.path())
        .args([
            "scan.png", "stdout", "-l", "spa", "--dpi", "300", "--psm", "6", "--oem", "3", "tsv",
        ])
        .output()
        .unwrap();
    assert!(tsv.status.success());
    let tsv = String::from_utf8(tsv.stdout).unwrap();
    let rows: Vec<_> = tsv
        .lines()
        .skip(1)
        .map(|line| line.split('\t').collect::<Vec<_>>())
        .filter(|row| row[0] == "5" && !row[11].is_empty())
        .collect();
    assert_eq!(rows.len(), result.fragments.len());
    for (row, word) in rows.iter().zip(&result.fragments) {
        let n = |i: usize| row[i].parse::<f64>().unwrap();
        assert_eq!(word.text, row[11]);
        assert_eq!(
            [word.x, word.y, word.width, word.height],
            [n(6), 220. - n(7) - n(9), n(8), n(9)]
        );
        assert!((word.confidence - n(10) / 100.).abs() < 1e-6);
    }
    let crop = oxidize_pdf::text::OcrRegion::new(33, 44, 1200, 220);
    let regional = provider
        .process_image_regions(&[(&data, &crop)], &OcrOptions::default())
        .unwrap();
    assert_eq!(regional[0].image_dimensions, result.image_dimensions);
    assert_eq!(regional[0].fragments[0].y, result.fragments[0].y);
    assert_eq!(regional[0].processed_region.as_ref().unwrap().y, 44);
    assert_eq!(result.text, "El niño tomó café con azúcar.");
    assert_eq!(result.image_dimensions, (1200, 220));
    assert_eq!(result.fragments.len(), 6);
    assert!(result.confidence > 0.7 && result.confidence <= 1.0);
    let mut right = 0.;
    for fragment in &result.fragments {
        assert!(fragment.x >= right && fragment.width > 0. && fragment.height > 0.);
        assert!(fragment.y >= 0. && fragment.y + fragment.height <= 220.);
        right = fragment.x + fragment.width;
        assert!(fragment.confidence > 0.7 && fragment.confidence <= 1.0);
    }
    assert!(right < 1200.);
    for rotation in [0, 90, 180, 270] {
        let raster = image::load_from_memory(&data).unwrap();
        let raster = match rotation {
            90 => raster.rotate270(),
            180 => raster.rotate180(),
            270 => raster.rotate90(),
            _ => raster,
        };
        let (w, h) = (raster.width(), raster.height());
        let raw = raster.into_rgb8().into_raw();
        let content = format!(
            "q {} 0 0 {} 37 53 cm /Scan Do Q\n",
            f64::from(w) / 2.,
            f64::from(h) / 2.
        );
        let mut objects = vec![
            b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
            format!("<< /Type /Pages /Count 1 /Kids [3 0 R] /MediaBox [0 0 {} {}] /CropBox [37 53 {} {}] >>",f64::from(w)/2.+74.,f64::from(h)/2.+106.,f64::from(w)/2.+37.,f64::from(h)/2.+53.).into_bytes(),
            format!("<< /Type /Page /Parent 2 0 R /Rotate {rotation} /Resources << /XObject << /Scan 5 0 R >> >> /Contents 4 0 R >>").into_bytes(),
            format!("<< /Length {} >>\nstream\n{content}endstream",content.len()).into_bytes(),
        ];
        let mut stream = format!("<< /Type /XObject /Subtype /Image /Width {w} /Height {h} /ColorSpace /DeviceRGB /BitsPerComponent 8 /Length {} >>\nstream\n",raw.len()).into_bytes();
        stream.extend(raw);
        stream.extend(b"\nendstream");
        objects.push(stream);
        let mut base = b"%PDF-1.7\n".to_vec();
        let mut offsets = Vec::new();
        for (i, object) in objects.iter().enumerate() {
            offsets.push(base.len());
            base.extend(format!("{} 0 obj\n", i + 1).as_bytes());
            base.extend(object);
            base.extend(b"\nendobj\n");
        }
        let start = base.len();
        base.extend(b"xref\n0 6\n0000000000 65535 f \n");
        for offset in offsets {
            base.extend(format!("{offset:010} 00000 n \n").as_bytes());
        }
        base.extend(
            format!("trailer\n<< /Size 6 /Root 1 0 R >>\nstartxref\n{start}\n%%EOF\n").as_bytes(),
        );
        let source = dir.path().join("source.pdf");
        let output = dir.path().join("output.pdf");
        std::fs::write(&source, &base).unwrap();
        let conversion = oxidize_pdf::operations::pdf_ocr_converter::PdfOcrConverter::new()
            .unwrap()
            .convert_to_searchable_pdf(&source, &output, &provider, &Default::default())
            .unwrap();
        assert_eq!(conversion.pages_ocr_processed, 1);
        assert!(std::fs::read(&output).unwrap().starts_with(&base));
        let status = Command::new("pdftotext")
            .current_dir(dir.path())
            .args(["-bbox", "-cropbox", "output.pdf", "output.html"])
            .status()
            .unwrap();
        assert!(status.success());
        let html = std::fs::read_to_string(dir.path().join("output.html")).unwrap();
        let words: Vec<_> = html.split("<word ").skip(1).collect();
        assert_eq!(words.len(), 6, "{html}");
        for (word, fragment) in words.iter().zip(&result.fragments) {
            let extracted = word
                .split('>')
                .nth(1)
                .unwrap()
                .split("</word")
                .next()
                .unwrap();
            assert_eq!(extracted.trim_end(), fragment.text, "{html}");
            let expected = [
                fragment.x / 2.,
                (220. - fragment.y - fragment.height) / 2.,
                (fragment.x + fragment.width) / 2.,
                (220. - fragment.y) / 2.,
            ];
            for (key, expected) in ["xMin", "yMin", "xMax", "yMax"].iter().zip(expected) {
                let actual: f64 = word
                    .split(&format!("{key}=\""))
                    .nth(1)
                    .unwrap()
                    .split('"')
                    .next()
                    .unwrap()
                    .parse()
                    .unwrap();
                assert!(
                    (actual - expected).abs() < 0.2,
                    "rotation={rotation} {} {key} {actual} expected {expected}",
                    fragment.text
                );
            }
        }
        for name in ["source", "output"] {
            assert!(Command::new("pdftoppm")
                .current_dir(dir.path())
                .args([
                    "-r",
                    "36",
                    "-singlefile",
                    "-cropbox",
                    &format!("{name}.pdf"),
                    name
                ])
                .status()
                .unwrap()
                .success());
        }
        assert_eq!(
            std::fs::read(dir.path().join("source.ppm")).unwrap(),
            std::fs::read(dir.path().join("output.ppm")).unwrap()
        );
        if rotation == 0 {
            assert_eq!(
                LegacyProvider.coordinate_space(),
                oxidize_pdf::text::OcrCoordinateSpace::PagePoints
            );
            oxidize_pdf::operations::pdf_ocr_converter::PdfOcrConverter::new()
                .unwrap()
                .convert_to_searchable_pdf(&source, &output, &LegacyProvider, &Default::default())
                .unwrap();
            assert!(Command::new("pdftotext")
                .current_dir(dir.path())
                .args(["-bbox", "-cropbox", "output.pdf", "legacy.html"])
                .status()
                .unwrap()
                .success());
            let html = std::fs::read_to_string(dir.path().join("legacy.html")).unwrap();
            assert!(html.contains("xMin=\"13.000000\" yMin=\"83.000000\" xMax=\"33.000000\" yMax=\"93.000000\">legacy"),"{html}");
        }
    }
}
