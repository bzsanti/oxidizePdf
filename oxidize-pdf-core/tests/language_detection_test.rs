//! Language detection on the DocumentChunker path (#293).
//! Content-verifying: asserts exact ISO 639-3 codes on real native-script text.
#![cfg(feature = "language-detection")]

use oxidize_pdf::ai::{DetectedLanguage, DocumentChunker};

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
    // A 2-char string is detectable by whatlang but must never be marked
    // reliable (it carries an effectively-random code).
    let chunks = chunker.chunk_text("ab").unwrap();
    if let Some(lang) = &chunks[0].metadata.language {
        assert!(!lang.reliable, "2-char detection must not be reliable");
    }
}
