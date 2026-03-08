#![allow(deprecated)]
use oxidize_pdf::parser::{PdfDocument, PdfReader};

fn fixture(name: &str) -> String {
    format!("{}/examples/results/{}", env!("CARGO_MANIFEST_DIR"), name)
}

// --- Step 2.1: PdfDocument::open() ---

#[test]
fn test_pdfdocument_open_file() {
    let doc = PdfDocument::open(fixture("hello_world.pdf"));
    assert!(doc.is_ok(), "Failed to open PDF: {:?}", doc.err());
    let doc = doc.unwrap();
    assert!(doc.page_count().unwrap() > 0);
}

#[test]
fn test_pdfdocument_open_nonexistent() {
    let result = PdfDocument::open("nonexistent_file_that_does_not_exist.pdf");
    assert!(result.is_err());
}

#[test]
fn test_pdfdocument_open_matches_two_step() {
    let path = fixture("hello_world.pdf");

    // Old pattern
    let reader = PdfReader::open(&path).unwrap();
    let doc_old = PdfDocument::new(reader);

    // New pattern
    let doc_new = PdfDocument::open(&path).unwrap();

    assert_eq!(doc_old.page_count().unwrap(), doc_new.page_count().unwrap());

    let text_old = doc_old.extract_text().unwrap();
    let text_new = doc_new.extract_text().unwrap();
    assert_eq!(text_old.len(), text_new.len());
    for (old, new) in text_old.iter().zip(text_new.iter()) {
        assert_eq!(old.text, new.text);
    }
}

// --- Step 2.2: to_markdown(), to_contextual(), to_json() ---

#[test]
fn test_to_markdown_produces_output() {
    let doc = PdfDocument::open(fixture("hello_world.pdf")).unwrap();
    let result = doc.to_markdown();
    assert!(result.is_ok(), "to_markdown failed: {:?}", result.err());
    let md = result.unwrap();
    assert!(!md.is_empty());
    assert!(md.contains("---")); // YAML frontmatter
}

#[test]
fn test_to_markdown_matches_free_function() {
    let doc = PdfDocument::open(fixture("hello_world.pdf")).unwrap();
    let from_method = doc.to_markdown().unwrap();
    let from_free = oxidize_pdf::ai::export_to_markdown(&doc).unwrap();
    assert_eq!(from_method, from_free);
}

#[test]
fn test_to_contextual_produces_output() {
    let doc = PdfDocument::open(fixture("hello_world.pdf")).unwrap();
    let result = doc.to_contextual();
    assert!(result.is_ok(), "to_contextual failed: {:?}", result.err());
    let ctx = result.unwrap();
    assert!(!ctx.is_empty());
}

#[test]
fn test_to_contextual_matches_free_function() {
    let doc = PdfDocument::open(fixture("hello_world.pdf")).unwrap();
    let from_method = doc.to_contextual().unwrap();
    let from_free = oxidize_pdf::ai::export_to_contextual(&doc).unwrap();
    assert_eq!(from_method, from_free);
}

#[cfg(feature = "semantic")]
#[test]
fn test_to_json_produces_output() {
    let doc = PdfDocument::open(fixture("hello_world.pdf")).unwrap();
    let result = doc.to_json();
    assert!(result.is_ok(), "to_json failed: {:?}", result.err());
    let json = result.unwrap();
    assert!(!json.is_empty());
    assert!(json.contains("pages"));
}

#[cfg(feature = "semantic")]
#[test]
fn test_to_json_matches_free_function() {
    let doc = PdfDocument::open(fixture("hello_world.pdf")).unwrap();
    let from_method = doc.to_json().unwrap();
    let from_free = oxidize_pdf::ai::export_to_json(&doc).unwrap();
    assert_eq!(from_method, from_free);
}

// --- Step 2.3: chunk() ---

#[test]
fn test_chunk_basic() {
    let doc = PdfDocument::open(fixture("hello_world.pdf")).unwrap();
    let chunks = doc.chunk(512);
    assert!(chunks.is_ok(), "chunk failed: {:?}", chunks.err());
    let chunks = chunks.unwrap();
    assert!(!chunks.is_empty());
    for chunk in &chunks {
        assert!(!chunk.content.is_empty());
    }
}

#[test]
fn test_chunk_small_size_produces_more_chunks() {
    let doc = PdfDocument::open(fixture("ai_ready_contract.pdf")).unwrap();
    let small_chunks = doc.chunk(50).unwrap();
    let large_chunks = doc.chunk(512).unwrap();
    assert!(
        small_chunks.len() >= large_chunks.len(),
        "small chunks ({}) should be >= large chunks ({})",
        small_chunks.len(),
        large_chunks.len()
    );
}

#[test]
fn test_chunk_with_overlap() {
    let doc = PdfDocument::open(fixture("ai_ready_contract.pdf")).unwrap();
    let chunks = doc.chunk_with(100, 20).unwrap();
    assert!(!chunks.is_empty());
}
