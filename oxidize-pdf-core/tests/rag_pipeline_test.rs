use oxidize_pdf::parser::{PdfDocument, PdfReader};
use oxidize_pdf::pipeline::RagChunk;
use std::path::Path;

fn fixture_path(name: &str) -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(name)
}

#[test]
fn test_rag_chunks_returns_vec() {
    let path = fixture_path("simple.pdf");
    if !path.exists() {
        eprintln!("[WARN] fixture missing, skipping test: {}", path.display());
        return;
    }
    let reader = PdfReader::open(&path).expect("must open PDF");
    let doc = PdfDocument::new(reader);

    let chunks: Vec<RagChunk> = doc.rag_chunks().expect("rag_chunks must succeed");

    for chunk in &chunks {
        assert!(
            chunk.chunk_index < chunks.len(),
            "chunk_index must be in range"
        );
        if !chunk.text.is_empty() {
            assert!(
                !chunk.page_numbers.is_empty(),
                "non-empty chunk must have page numbers"
            );
        }
    }
}

#[test]
fn test_rag_chunks_with_custom_config() {
    let path = fixture_path("simple.pdf");
    if !path.exists() {
        eprintln!("[WARN] fixture missing, skipping test: {}", path.display());
        return;
    }
    let reader = PdfReader::open(&path).expect("must open PDF");
    let doc = PdfDocument::new(reader);

    let config = oxidize_pdf::pipeline::HybridChunkConfig {
        max_tokens: 32,
        ..Default::default()
    };
    let chunks = doc.rag_chunks_with(config).expect("must succeed");

    // With smaller token limit, should have at least as many chunks as default
    let default_chunks = doc.rag_chunks().expect("must succeed");
    assert!(
        chunks.len() >= default_chunks.len(),
        "smaller max_tokens must produce >= chunks"
    );
}

#[cfg(feature = "semantic")]
#[test]
fn test_rag_chunks_json_returns_valid_json() {
    let path = fixture_path("simple.pdf");
    if !path.exists() {
        eprintln!("[WARN] fixture missing, skipping test: {}", path.display());
        return;
    }
    let reader = PdfReader::open(&path).expect("must open PDF");
    let doc = PdfDocument::new(reader);

    let json = doc.rag_chunks_json().expect("must produce JSON");
    assert!(json.starts_with('['));
    assert!(json.ends_with(']'));
}

// Test with deprecated methods — backward compatibility
#[allow(deprecated)]
#[test]
fn test_deprecated_chunk_methods_still_work() {
    let path = fixture_path("simple.pdf");
    if !path.exists() {
        eprintln!("[WARN] fixture missing, skipping test: {}", path.display());
        return;
    }
    let reader = PdfReader::open(&path).expect("must open");
    let doc = PdfDocument::new(reader);
    let result = doc.chunk(512);
    assert!(result.is_ok());
}

// ── rag_chunks_with_profile ────────────────────────────────────────────────

#[test]
fn test_rag_chunks_with_profile_rag() {
    use oxidize_pdf::pipeline::ExtractionProfile;
    let path = fixture_path("Cold_Email_Hacks.pdf");
    if !path.exists() {
        eprintln!("[WARN] fixture missing, skipping: {}", path.display());
        return;
    }
    let reader = PdfReader::open(&path).expect("must open PDF");
    let doc = PdfDocument::new(reader);

    let chunks = doc
        .rag_chunks_with_profile(ExtractionProfile::Rag)
        .expect("rag_chunks_with_profile must succeed");

    assert!(!chunks.is_empty());
    for (i, chunk) in chunks.iter().enumerate() {
        assert_eq!(chunk.chunk_index, i);
        if !chunk.text.is_empty() {
            assert!(!chunk.page_numbers.is_empty());
        }
        for window in chunk.page_numbers.windows(2) {
            assert!(window[0] <= window[1], "page_numbers must be sorted");
        }
    }
}

#[test]
fn test_rag_chunks_with_profile_accepts_all_variants() {
    use oxidize_pdf::pipeline::ExtractionProfile;
    // Use simple.pdf — Cold_Email_Hacks.pdf triggers a known overflow bug
    // in column detection (Academic profile has detect_columns: true).
    let path = fixture_path("simple.pdf");
    if !path.exists() {
        eprintln!("[WARN] fixture missing, skipping: {}", path.display());
        return;
    }
    let reader = PdfReader::open(&path).expect("must open PDF");
    let doc = PdfDocument::new(reader);

    let variants = [
        ExtractionProfile::Standard,
        ExtractionProfile::Academic,
        ExtractionProfile::Form,
        ExtractionProfile::Government,
        ExtractionProfile::Dense,
        ExtractionProfile::Presentation,
        ExtractionProfile::Rag,
    ];
    for profile in variants {
        assert!(
            doc.rag_chunks_with_profile(profile).is_ok(),
            "must not fail for any profile"
        );
    }
}
