//! Independent Poppler checks of the actual selectable text, not /OxidizeRegion.
use oxidize_pdf::writer::{IncrementalOcrLayerEditor, OcrLayerFragment, OcrLayerPage};
use std::process::Command;

fn fixture(rotation: i32) -> Vec<u8> {
    // Legal trailing state must not leak into the appended OCR layer.
    let stream = "0.2 0.4 0.8 rg 69 133 80 18 re f\n2 0 0 2 10 20 cm\nBT 5 Tc 6 Tw 7 Ts ET\n";
    let objects = [
        "<< /Type /Catalog /Pages 2 0 R >>".to_string(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 /MediaBox [0 0 600 500] /CropBox [37 53 537 453] >>".to_string(),
        format!("<< /Type /Page /Parent 2 0 R /Rotate {rotation} /Resources << >> /Contents 4 0 R >>"),
        format!("<< /Length {} >>\nstream\n{stream}endstream", stream.len()),
    ];
    let mut bytes = b"%PDF-1.7\n".to_vec();
    let mut offsets = vec![0];
    for (index, object) in objects.iter().enumerate() {
        offsets.push(bytes.len());
        bytes.extend(format!("{} 0 obj\n{object}\nendobj\n", index + 1).as_bytes());
    }
    let start = bytes.len();
    bytes.extend(b"xref\n0 5\n0000000000 65535 f \n");
    for offset in &offsets[1..] {
        bytes.extend(format!("{offset:010} 00000 n \n").as_bytes());
    }
    bytes.extend(
        format!("trailer\n<< /Size 5 /Root 1 0 R >>\nstartxref\n{start}\n%%EOF\n").as_bytes(),
    );
    bytes
}

fn run(tool: &str, args: &[&str], directory: &std::path::Path) {
    let output = Command::new(tool)
        .args(args)
        .current_dir(directory)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{tool}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn planning_and_applying_reject_the_same_unrepresentable_transform() {
    let base = fixture(0);
    let layer = OcrLayerPage {
        page_index: 0,
        language: "en".into(),
        fragments: vec![OcrLayerFragment {
            text: "word".into(),
            region: [10., 20., f64::MAX, 10.],
            confidence: 1.,
            reading_order: 0,
        }],
    };
    let editor = IncrementalOcrLayerEditor::new(&base);
    assert_eq!(
        editor
            .plan(std::slice::from_ref(&layer))
            .unwrap_err()
            .to_string(),
        editor.apply(&[layer]).unwrap_err().to_string()
    );
}

#[test]
fn tiny_positive_regions_do_not_serialize_as_zero_scale() {
    let base = fixture(0);
    let layer = OcrLayerPage {
        page_index: 0,
        language: "en".into(),
        fragments: vec![OcrLayerFragment {
            text: "word".into(),
            region: [10., 20., 1e-12, 10.],
            confidence: 1.,
            reading_order: 0,
        }],
    };
    let result = IncrementalOcrLayerEditor::new(&base)
        .apply(&[layer])
        .unwrap();
    let added = String::from_utf8_lossy(&result.pdf_bytes[base.len()..]);
    let scale: f64 = added
        .lines()
        .find_map(|line| line.strip_suffix(" Tz"))
        .unwrap()
        .parse()
        .unwrap();
    assert!(scale > 0. && scale < 1e-9, "{scale}");
}

#[test]
#[ignore = "requires Poppler; run explicitly in interoperability CI"]
fn touching_words_retain_a_logical_separator() {
    let base = fixture(0);
    let layer = OcrLayerPage {
        page_index: 0,
        language: "en".into(),
        fragments: vec![
            OcrLayerFragment {
                text: "one".into(),
                region: [70., 150., 30., 10.],
                confidence: 1.,
                reading_order: 0,
            },
            OcrLayerFragment {
                text: "two".into(),
                region: [100., 150., 30., 10.],
                confidence: 1.,
                reading_order: 1,
            },
        ],
    };
    let output = IncrementalOcrLayerEditor::new(&base)
        .apply(&[layer])
        .unwrap();
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("result.pdf"), &output.pdf_bytes).unwrap();
    run("pdftotext", &["-raw", "result.pdf", "text.txt"], dir.path());
    assert_eq!(
        std::fs::read_to_string(dir.path().join("text.txt"))
            .unwrap()
            .split_whitespace()
            .collect::<Vec<_>>(),
        ["one", "two"]
    );
    let document = oxidize_pdf::parser::PdfDocument::new(
        oxidize_pdf::parser::PdfReader::new(std::io::Cursor::new(output.pdf_bytes)).unwrap(),
    );
    let text = oxidize_pdf::text::TextExtractor::new()
        .extract_from_page(&document, 0)
        .unwrap();
    assert!(text.text.contains("one two"), "{}", text.text);
}

#[test]
#[ignore = "requires Poppler and qpdf; run explicitly in interoperability CI"]
fn selectable_unicode_bounds_follow_regions_at_all_page_rotations() {
    for rotation in [0, 90, 180, 270] {
        let base = fixture(rotation);
        // Inverse map of a displayed box [32, 80, 80, 18] in top-left coordinates.
        let region = match rotation {
            0 => [69., 355., 80., 18.],
            90 => [117., 85., 18., 80.],
            180 => [425., 133., 80., 18.],
            270 => [439., 341., 18., 80.],
            _ => unreachable!(),
        };
        for word in ["El", "niño", "café", "世界", "iiii", "WWWW"] {
            let layer = OcrLayerPage {
                page_index: 0,
                language: "es".into(),
                fragments: vec![OcrLayerFragment {
                    text: word.into(),
                    region,
                    confidence: 0.95,
                    reading_order: 0,
                }],
            };
            let result = IncrementalOcrLayerEditor::new(&base)
                .apply(&[layer])
                .unwrap();
            assert!(result.pdf_bytes.starts_with(&base));
            let dir = tempfile::tempdir().unwrap();
            std::fs::write(dir.path().join("base.pdf"), &base).unwrap();
            std::fs::write(dir.path().join("result.pdf"), result.pdf_bytes).unwrap();
            run("qpdf", &["--check", "result.pdf"], dir.path());
            run(
                "pdftotext",
                &["-bbox", "-cropbox", "result.pdf", "bounds.html"],
                dir.path(),
            );
            let html = std::fs::read_to_string(dir.path().join("bounds.html")).unwrap();
            let tag = html.split("<word ").nth(1).expect("searchable word");
            assert!(tag.contains(&format!(">{word}</word>")), "{html}");
            for (key, expected) in [("xMin", 32.), ("yMin", 80.), ("xMax", 112.), ("yMax", 98.)] {
                let actual: f64 = tag
                    .split(&format!("{key}=\""))
                    .nth(1)
                    .unwrap()
                    .split('"')
                    .next()
                    .unwrap()
                    .parse()
                    .unwrap();
                assert!(
                    (actual - expected).abs() < 0.15,
                    "rotation={rotation} word={word} {key}: {actual} expected {expected}"
                );
            }
            for name in ["base", "result"] {
                run(
                    "pdftoppm",
                    &[
                        "-r",
                        "36",
                        "-singlefile",
                        "-cropbox",
                        &format!("{name}.pdf"),
                        name,
                    ],
                    dir.path(),
                );
            }
            assert_eq!(
                std::fs::read(dir.path().join("base.ppm")).unwrap(),
                std::fs::read(dir.path().join("result.ppm")).unwrap()
            );
        }
    }
}
