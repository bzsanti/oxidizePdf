use oxidize_pdf::{
    compare_pdfs_semantically, Document, Font, Page, SemanticComparisonOptions,
    SemanticDifferenceClass,
};
use std::path::Path;
use std::process::Command;

fn tool_available(tool: &str) -> bool {
    Command::new(tool)
        .arg("-v")
        .output()
        .map(|output| output.status.success() || !output.stderr.is_empty())
        .unwrap_or(false)
}

fn document(text: &str, compress: bool) -> Vec<u8> {
    let mut document = Document::new();
    document.set_compress(compress);
    let mut page = Page::a4();
    page.text()
        .set_font(Font::Helvetica, 12.0)
        .at(30.0, 800.0)
        .write(text)
        .unwrap();
    document.add_page(page);
    document.to_bytes().unwrap()
}

fn run(command: &mut Command) {
    let output = command.output().unwrap();
    assert!(
        output.status.success(),
        "external differential command failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn qpdf_check(path: &Path) {
    run(Command::new("qpdf").arg("--check").arg(path));
}

#[test]
fn independent_tools_confirm_semantic_and_serialization_results() {
    if !(tool_available("qpdf") && tool_available("pdftotext") && tool_available("pdftoppm")) {
        assert_ne!(
            std::env::var("OXIDIZE_PDF_REQUIRE_DIFFERENTIAL_TOOLS").as_deref(),
            Ok("1"),
            "qpdf and poppler tools are required by this CI job"
        );
        eprintln!("skipping differential test: qpdf/poppler tools are unavailable");
        return;
    }

    let directory = tempfile::tempdir().unwrap();
    let plain = directory.path().join("plain.pdf");
    let compressed = directory.path().join("compressed.pdf");
    let changed = directory.path().join("changed.pdf");
    std::fs::write(&plain, document("same logical text", false)).unwrap();
    std::fs::write(&compressed, document("same logical text", true)).unwrap();
    std::fs::write(&changed, document("different text", true)).unwrap();

    qpdf_check(&plain);
    qpdf_check(&compressed);
    qpdf_check(&changed);

    let equal = compare_pdfs_semantically(
        &std::fs::read(&plain).unwrap(),
        &std::fs::read(&compressed).unwrap(),
        &SemanticComparisonOptions::default(),
    )
    .unwrap();
    assert!(equal.semantically_equal);
    assert!(equal
        .differences
        .iter()
        .all(|difference| difference.class == SemanticDifferenceClass::SerializationOnly));

    let changed_result = compare_pdfs_semantically(
        &std::fs::read(&plain).unwrap(),
        &std::fs::read(&changed).unwrap(),
        &SemanticComparisonOptions::default(),
    )
    .unwrap();
    assert!(!changed_result.semantically_equal);
    assert!(changed_result
        .differences
        .iter()
        .any(|difference| difference.class == SemanticDifferenceClass::Visual));
    assert!(changed_result
        .differences
        .iter()
        .any(|difference| difference.class == SemanticDifferenceClass::Textual));

    let plain_text = directory.path().join("plain.txt");
    let compressed_text = directory.path().join("compressed.txt");
    let changed_text = directory.path().join("changed.txt");
    run(Command::new("pdftotext").arg(&plain).arg(&plain_text));
    run(Command::new("pdftotext")
        .arg(&compressed)
        .arg(&compressed_text));
    run(Command::new("pdftotext").arg(&changed).arg(&changed_text));
    assert_eq!(
        std::fs::read(&plain_text).unwrap(),
        std::fs::read(&compressed_text).unwrap()
    );
    assert_ne!(
        std::fs::read(&plain_text).unwrap(),
        std::fs::read(&changed_text).unwrap()
    );

    let plain_render = directory.path().join("plain-render");
    let compressed_render = directory.path().join("compressed-render");
    let changed_render = directory.path().join("changed-render");
    for (input, output) in [
        (&plain, &plain_render),
        (&compressed, &compressed_render),
        (&changed, &changed_render),
    ] {
        run(Command::new("pdftoppm")
            .args(["-png", "-singlefile", "-r", "72"])
            .arg(input)
            .arg(output));
    }
    assert_eq!(
        std::fs::read(plain_render.with_extension("png")).unwrap(),
        std::fs::read(compressed_render.with_extension("png")).unwrap()
    );
    assert_ne!(
        std::fs::read(plain_render.with_extension("png")).unwrap(),
        std::fs::read(changed_render.with_extension("png")).unwrap()
    );
}
