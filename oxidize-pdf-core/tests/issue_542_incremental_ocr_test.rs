use oxidize_pdf::writer::{IncrementalOcrLayerEditor, OcrLayerFragment, OcrLayerPage};
use oxidize_pdf::{Document, Page};
use std::process::Command;

fn base_pdf(xref_stream: bool) -> Vec<u8> {
    let mut document = Document::new();
    document.enable_xref_streams(xref_stream);
    document.add_page(Page::a4());
    document.to_bytes().unwrap()
}

fn layer() -> OcrLayerPage {
    OcrLayerPage {
        page_index: 0,
        language: "es".to_string(),
        fragments: vec![OcrLayerFragment {
            text: "búsqueda".to_string(),
            region: [50.0, 700.0, 70.0, 12.0],
            confidence: 0.99,
            reading_order: 0,
        }],
    }
}

#[test]
#[ignore = "requires qpdf; exercised by the Ubuntu CI interoperability step"]
fn qpdf_accepts_classic_and_xref_stream_ocr_revisions() {
    for (name, base) in [
        ("classic", base_pdf(false)),
        ("xref-stream", base_pdf(true)),
    ] {
        let update = IncrementalOcrLayerEditor::new(&base)
            .apply(&[layer()])
            .unwrap();
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join(format!("{name}.pdf"));
        std::fs::write(&path, update.pdf_bytes).unwrap();
        let output = Command::new("qpdf")
            .arg("--check")
            .arg(path)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{name}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}
