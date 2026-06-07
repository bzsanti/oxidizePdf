//! Language detection on the DocumentChunker path (#293).
//! Content-verifying: asserts exact ISO 639-3 codes on real native-script text.
#![cfg(feature = "language-detection")]

use oxidize_pdf::ai::{
    ChunkMetadata, ChunkPosition, DetectedLanguage, DocumentChunk, DocumentChunker,
};

// UDHR Article 1, used as guaranteed-valid native-script input.
const EN: &str = "All human beings are born free and equal in dignity and rights. They are endowed with reason and conscience and should act towards one another in a spirit of brotherhood.";
const ES: &str = "Todos los seres humanos nacen libres e iguales en dignidad y derechos. Dotados de razón y conciencia, deben comportarse fraternalmente los unos con los otros.";
const AR: &str = "جميع الناس يولدون أحرارا متساوين في الكرامة والحقوق. وقد وهبوا عقلا وضميرا وعليهم أن يعامل بعضهم بعضا بروح الإخاء.";
const HE: &str = "כל בני האדם נולדו בני חורין ושווים בערכם ובזכויותיהם. כולם חוננו בתבונה ובמצפון, לפיכך חובה עליהם לנהוג איש ברעהו ברוח של אחווה.";

fn only_chunk_language(text: &str) -> Option<DetectedLanguage> {
    let chunker = DocumentChunker::new(512, 0).with_language_detection(true);
    let chunks = chunker.chunk_text(text).unwrap();
    assert!(!chunks.is_empty(), "expected at least one chunk");
    chunks[0].metadata.language.clone()
}

#[test]
fn detects_english() {
    let lang = only_chunk_language(EN).expect("english should be detected");
    assert_eq!(lang.code, "eng");
    assert!(lang.reliable, "english detection should be reliable");
    assert!(lang.confidence > 0.0);
}

#[test]
fn detects_spanish() {
    let lang = only_chunk_language(ES).expect("spanish should be detected");
    assert_eq!(lang.code, "spa");
    assert!(lang.reliable);
}

#[test]
fn detects_arabic_synthetic() {
    let lang = only_chunk_language(AR).expect("arabic should be detected");
    assert_eq!(lang.code, "ara");
    assert!(lang.reliable);
}

#[test]
fn detects_hebrew_synthetic() {
    let lang = only_chunk_language(HE).expect("hebrew should be detected");
    assert_eq!(lang.code, "heb");
    assert!(lang.reliable);
}

#[test]
fn no_detection_when_flag_off() {
    // Feature compiled in, but detection not requested -> language stays None.
    let chunker = DocumentChunker::new(512, 0);
    let chunks = chunker.chunk_text(EN).unwrap();
    assert!(chunks[0].metadata.language.is_none());
}

#[test]
fn empty_text_yields_no_chunks_and_short_text_is_unreliable() {
    let chunker = DocumentChunker::new(512, 0).with_language_detection(true);
    // Empty input produces no chunks at all.
    assert!(chunker.chunk_text("").unwrap().is_empty());
    // A 2-char string yields a (current whatlang) detection that must never be
    // marked reliable — it carries an effectively-random code. Asserting `Some`
    // makes this a real regression gate (not vacuously true) for the current
    // whatlang behavior.
    let chunks = chunker.chunk_text("ab").unwrap();
    let lang = chunks[0]
        .metadata
        .language
        .as_ref()
        .expect("whatlang produces a (low-confidence) detection for 2 chars");
    assert!(!lang.reliable, "2-char detection must not be reliable");
}

fn chunk_with(content: &str, code: &str, confidence: f32, reliable: bool) -> DocumentChunk {
    DocumentChunk {
        id: "c".to_string(),
        content: content.to_string(),
        tokens: 0,
        page_numbers: vec![],
        chunk_index: 0,
        metadata: ChunkMetadata {
            position: ChunkPosition::default(),
            confidence: 1.0,
            sentence_boundary_respected: false,
            language: Some(DetectedLanguage {
                code: code.to_string(),
                confidence,
                reliable,
            }),
        },
    }
}

#[test]
fn document_language_picks_dominant_by_length() {
    // "spa" carries far more characters than "eng" -> spa wins.
    let chunks = vec![
        chunk_with("short english bit", "eng", 0.9, true),
        chunk_with(&"texto en español ".repeat(20), "spa", 0.95, true),
    ];
    let doc = DocumentChunker::document_language(&chunks).expect("a dominant language");
    assert_eq!(doc.code, "spa");
    assert!(doc.reliable);
}

#[test]
fn document_language_none_when_no_chunk_has_language() {
    let chunker = DocumentChunker::new(512, 0); // detection off -> all None
    let chunks = chunker.chunk_text(EN).unwrap();
    assert!(DocumentChunker::document_language(&chunks).is_none());
}

#[test]
fn document_language_empty_slice_is_none() {
    assert!(DocumentChunker::document_language(&[]).is_none());
}

#[test]
fn document_language_reliable_if_any_winner_chunk_reliable() {
    let chunks = vec![
        chunk_with(&"a".repeat(10), "eng", 0.4, false),
        chunk_with(&"b".repeat(10), "eng", 0.8, true),
    ];
    let doc = DocumentChunker::document_language(&chunks).unwrap();
    assert_eq!(doc.code, "eng");
    assert!(
        doc.reliable,
        "reliable if any contributing chunk was reliable"
    );
}

#[test]
fn document_language_skips_chunks_without_language() {
    // A chunk with no detected language must not contribute or dilute the result.
    let no_lang = DocumentChunk {
        id: "n".to_string(),
        content: "x".repeat(1000), // long, but carries no language
        tokens: 0,
        page_numbers: vec![],
        chunk_index: 0,
        metadata: ChunkMetadata {
            position: ChunkPosition::default(),
            confidence: 1.0,
            sentence_boundary_respected: false,
            language: None,
        },
    };
    let chunks = vec![chunk_with("short english", "eng", 0.9, true), no_lang];
    let doc =
        DocumentChunker::document_language(&chunks).expect("eng from the only detected chunk");
    assert_eq!(doc.code, "eng");
    assert!(doc.reliable);
}
