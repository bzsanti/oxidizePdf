use oxidize_pdf::parser::PdfDocument;
use oxidize_pdf::pipeline::{SemanticChunkConfig, SemanticChunker};

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
    assert!(!elements.is_empty(), "partition must produce elements");

    // Strip zero-width spaces (U+200B) before tokenizing. Cold_Email_Hacks
    // interleaves ZWSP between visible spaces in extract_text output, but
    // not in the partition's element texts, so a naive HashSet comparison
    // sees `"cold\u{200B}"` vs `"cold"` as different words.
    let strip_zwsp = |s: &str| s.replace('\u{200B}', "");

    let text_original = doc.extract_text().unwrap();
    let original_words: std::collections::HashSet<String> = text_original
        .iter()
        .flat_map(|p| {
            strip_zwsp(&p.text)
                .split_whitespace()
                .map(|w| w.to_lowercase())
                .collect::<Vec<_>>()
        })
        .filter(|w| w.len() > 2)
        .collect();

    // Partition emits one Element per TextFragment (no paragraph reconstruction
    // at this layer — that lives in #261's HybridChunker), so words that
    // extract_text concatenates from adjacent fragments may appear split
    // across element boundaries (e.g. fragments "us" + "ed" → two elements
    // that never form the token "used"). Compare via substring containment
    // in the joined element corpus rather than exact-token set membership.
    let element_corpus: String = elements
        .iter()
        .map(|e| strip_zwsp(e.text()).to_lowercase())
        .collect::<Vec<_>>()
        .join(" ");
    // Strip remaining whitespace so cross-fragment word concatenations match.
    let element_corpus_compact: String = element_corpus
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect();

    let missing: Vec<&String> = original_words
        .iter()
        .filter(|w| {
            let w_compact: String = w.chars().filter(|c| !c.is_whitespace()).collect();
            !element_corpus_compact.contains(&w_compact)
        })
        .collect();

    let coverage = 1.0 - (missing.len() as f64 / original_words.len().max(1) as f64);
    // Threshold is set at 70% (not the ideal 100%) because the partitioner
    // produces one Element per PDF text fragment, and text fragments do not
    // align with word boundaries on PDFs whose typesetter (LaTeX, certain
    // DTP tools) emits per-syllable or per-letter Tj operators. The remaining
    // ~30% gap closes once paragraph reconstruction (#261) is applied, which
    // is its own separate pipeline pass.
    assert!(
        coverage > 0.70,
        "Text coverage should be >70%, got {:.1}% ({} missing of {})",
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
    #[allow(deprecated)]
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
