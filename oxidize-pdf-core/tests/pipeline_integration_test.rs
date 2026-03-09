use oxidize_pdf::parser::PdfDocument;
use oxidize_pdf::pipeline::{Element, SemanticChunkConfig, SemanticChunker};

fn fixture(name: &str) -> String {
    format!("{}/tests/fixtures/{}", env!("CARGO_MANIFEST_DIR"), name)
}

#[test]
fn test_full_pipeline_hello_world() {
    let doc = PdfDocument::open(fixture("Cold_Email_Hacks.pdf")).unwrap();
    let elements = doc.partition().unwrap();
    assert!(!elements.is_empty(), "partition should produce elements");

    let markdown = doc.to_markdown().unwrap();
    assert!(!markdown.is_empty());
}

#[test]
fn test_full_pipeline_page_order() {
    let doc = PdfDocument::open(fixture("Cold_Email_Hacks.pdf")).unwrap();
    let elements = doc.partition().unwrap();
    assert!(!elements.is_empty());

    // Page numbers should be monotonically non-decreasing regardless of page count
    for window in elements.windows(2) {
        assert!(
            window[0].page() <= window[1].page(),
            "Page numbers should be non-decreasing: {} > {}",
            window[0].page(),
            window[1].page()
        );
    }

    // All page numbers should be valid (within document range)
    let page_count = doc.page_count().unwrap() as u32;
    for elem in &elements {
        assert!(
            elem.page() < page_count,
            "Page {} out of range (doc has {} pages)",
            elem.page(),
            page_count
        );
    }
}

#[test]
fn test_full_pipeline_with_semantic_chunking() {
    let doc = PdfDocument::open(fixture("Cold_Email_Hacks.pdf")).unwrap();
    let elements = doc.partition().unwrap();

    let chunks =
        SemanticChunker::new(SemanticChunkConfig::new(256).with_overlap(0)).chunk(&elements);

    assert!(!chunks.is_empty());
    for chunk in &chunks {
        assert!(!chunk.text().is_empty());
        assert!(!chunk.elements().is_empty());
        assert!(chunk.token_estimate() > 0);
    }
}

#[test]
fn test_pipeline_roundtrip_text_preservation() {
    let doc = PdfDocument::open(fixture("Cold_Email_Hacks.pdf")).unwrap();

    let elements = doc.partition().unwrap();

    // Skip if partition produced no body elements (all classified as headers/footers)
    let body_elements: Vec<_> = elements
        .iter()
        .filter(|e| !matches!(e, Element::Header(_) | Element::Footer(_)))
        .collect();
    if body_elements.is_empty() {
        eprintln!("Skipping: fixture has no body elements (all classified as headers/footers)");
        return;
    }

    let text_original = doc.extract_text().unwrap();
    let original_words: std::collections::HashSet<String> = text_original
        .iter()
        .flat_map(|p| p.text.split_whitespace().map(|w| w.to_lowercase()))
        .filter(|w| w.len() > 2)
        .collect();

    let element_words: std::collections::HashSet<String> = body_elements
        .iter()
        .flat_map(|e| e.text().split_whitespace().map(|w| w.to_lowercase()))
        .filter(|w| w.len() > 2)
        .collect();

    // Most words from original text should appear in elements
    let missing: Vec<_> = original_words
        .iter()
        .filter(|w| !element_words.contains(*w))
        .collect();

    let coverage = 1.0 - (missing.len() as f64 / original_words.len().max(1) as f64);
    assert!(
        coverage > 0.8,
        "Text coverage should be >80%, got {:.1}% ({} missing of {})",
        coverage * 100.0,
        missing.len(),
        original_words.len()
    );
}

#[test]
fn test_vibecoding_three_lines() {
    // The "golden path" — minimal code to process a PDF
    let doc = PdfDocument::open(fixture("Cold_Email_Hacks.pdf")).unwrap();
    let markdown = doc.to_markdown().unwrap();
    let chunks = doc.chunk(512).unwrap();

    assert!(!markdown.is_empty());
    assert!(!chunks.is_empty());
}

#[test]
fn test_vibecoding_partition_and_chunk() {
    // Zero-configuration partition + semantic chunk
    let doc = PdfDocument::open(fixture("Cold_Email_Hacks.pdf")).unwrap();
    let elements = doc.partition().unwrap();
    let chunks = SemanticChunker::default().chunk(&elements);

    assert!(!elements.is_empty());
    assert!(!chunks.is_empty());
}
