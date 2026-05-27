//! Offline unit test for the JSONL writer used by examples/rag_realworld.rs.
//!
//! Verifies field presence, types, and content against synthetic RagChunk values.
//! No PDF parsing and no network — this test isolates the JSONL contract.

#[path = "../examples/rag_realworld.rs"]
#[allow(dead_code)]
mod example;

use oxidize_pdf::pipeline::{ElementBBox, RagChunk};
use serde_json::Value;

fn sample_chunk_with_heading() -> RagChunk {
    RagChunk {
        chunk_index: 3,
        text: "Artículo 1. Objeto y ámbito de aplicación.".to_string(),
        full_text: "CAPÍTULO I > Artículo 1\n\nArtículo 1. Objeto y ámbito de aplicación."
            .to_string(),
        page_numbers: vec![3, 4],
        bounding_boxes: vec![ElementBBox::new(50.0, 700.0, 400.0, 12.0)],
        element_types: vec!["heading".to_string(), "paragraph".to_string()],
        heading_context: Some("CAPÍTULO I > Artículo 1".to_string()),
        token_estimate: 487,
        is_oversized: false,
    }
}

fn sample_chunk_without_heading() -> RagChunk {
    RagChunk {
        chunk_index: 0,
        text: "Plain body text with no heading.".to_string(),
        full_text: "Plain body text with no heading.".to_string(),
        page_numbers: vec![1],
        bounding_boxes: vec![ElementBBox::new(0.0, 0.0, 10.0, 10.0)],
        element_types: vec!["paragraph".to_string()],
        heading_context: None,
        token_estimate: 6,
        is_oversized: false,
    }
}

fn sample_oversized_chunk() -> RagChunk {
    RagChunk {
        chunk_index: 99,
        text: "An oversized chunk that exceeds max_tokens.".to_string(),
        full_text: "Big Section\n\nAn oversized chunk that exceeds max_tokens.".to_string(),
        page_numbers: vec![10, 11, 12],
        bounding_boxes: vec![ElementBBox::new(0.0, 0.0, 10.0, 10.0)],
        element_types: vec!["paragraph".to_string()],
        heading_context: Some("Big Section".to_string()),
        token_estimate: 1024,
        is_oversized: true,
    }
}

#[test]
fn jsonl_line_with_heading_has_all_required_fields() {
    let chunk = sample_chunk_with_heading();
    let line = example::jsonl_line(
        "ens",
        "BOE Real Decreto 311/2022",
        "ES",
        "es",
        "https://www.boe.es/example.pdf",
        &chunk,
    );

    // Parses as a single JSON object
    let v: Value = serde_json::from_str(&line).expect("must parse as JSON");
    assert!(v.is_object(), "top-level must be an object");

    // Top-level fields
    assert_eq!(v["id"], "ens-0003");
    assert_eq!(v["text"], "Artículo 1. Objeto y ámbito de aplicación.");

    let m = &v["metadata"];
    assert!(m.is_object(), "metadata must be an object");

    // Field-by-field
    assert_eq!(m["source_url"], "https://www.boe.es/example.pdf");
    assert_eq!(m["document_name"], "BOE Real Decreto 311/2022");
    assert_eq!(m["country"], "ES");
    assert_eq!(m["language"], "es");
    assert_eq!(m["page_numbers"], serde_json::json!([3, 4]));
    assert_eq!(m["heading_context"], "CAPÍTULO I > Artículo 1");
    assert_eq!(
        m["element_types"],
        serde_json::json!(["heading", "paragraph"])
    );
    assert_eq!(m["token_estimate"], 487);
    assert_eq!(m["is_oversized"], false);
}

#[test]
fn jsonl_line_without_heading_serializes_null() {
    let chunk = sample_chunk_without_heading();
    let line = example::jsonl_line(
        "higgs",
        "ATLAS Higgs paper",
        "CERN",
        "en",
        "https://arxiv.org/pdf/1207.7214",
        &chunk,
    );
    let v: Value = serde_json::from_str(&line).unwrap();
    assert_eq!(v["id"], "higgs-0000");
    assert!(
        v["metadata"]["heading_context"].is_null(),
        "heading_context must be JSON null when no parent heading, got {:?}",
        v["metadata"]["heading_context"]
    );
}

#[test]
fn jsonl_line_oversized_preserves_flag_and_pages() {
    let chunk = sample_oversized_chunk();
    let line = example::jsonl_line(
        "bsi-tr-02102",
        "BSI TR-02102-1",
        "DE",
        "de",
        "https://www.bsi.bund.de/example.pdf",
        &chunk,
    );
    let v: Value = serde_json::from_str(&line).unwrap();
    assert_eq!(
        v["id"], "bsi-tr-02102-0099",
        "chunk_index 99 must zero-pad to 0099"
    );
    assert_eq!(v["metadata"]["is_oversized"], true);
    assert_eq!(
        v["metadata"]["page_numbers"],
        serde_json::json!([10, 11, 12])
    );
    assert_eq!(v["metadata"]["token_estimate"], 1024);
}

#[test]
fn jsonl_line_is_single_line_with_no_internal_newlines() {
    let chunk = sample_chunk_with_heading();
    let line = example::jsonl_line("ens", "doc", "ES", "es", "https://example.com", &chunk);
    assert!(
        !line.contains('\n'),
        "JSONL line must not contain a literal newline (serde_json default escapes them)"
    );
    // Field with multiline content should be escaped, not raw-broken
    let chunk_multiline = RagChunk {
        text: "Line one.\nLine two.".to_string(),
        ..chunk
    };
    let line2 = example::jsonl_line("x", "y", "X", "x", "u", &chunk_multiline);
    assert!(
        !line2.contains('\n'),
        "embedded newlines must be escaped, not literal"
    );
    // Confirm it round-trips
    let v: Value = serde_json::from_str(&line2).unwrap();
    assert_eq!(v["text"], "Line one.\nLine two.");
}
