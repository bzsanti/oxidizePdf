# Language Detection (per-chunk/document) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add opt-in, feature-gated per-chunk and document-level language detection to the `DocumentChunker` path, exposed as `Option<DetectedLanguage>` on `ChunkMetadata`.

**Architecture:** A `language-detection` Cargo feature pulls pure-Rust `whatlang` 0.18. `whatlang` stays an internal detail behind an owned `DetectedLanguage` type (ISO 639-3 code + confidence + reliable). Detection is enabled per-chunker via `with_language_detection(true)`; it runs during chunk construction and is a no-op when the flag is off or the feature is absent. A `document_language` associated function aggregates per-chunk results weighted by content length.

**Tech Stack:** Rust (workspace MSRV 1.88), `whatlang = "0.18"` (edition 2024), `cargo test`. Tests are integration-level in `oxidize-pdf-core/tests/`, content-verifying per `CLAUDE.md` (no smoke tests). Spike-verified 2026-06-06: CJK fixtures extract to clean Unicode (cmn/jpn/kor at confidence 1.0); Arabic/Hebrew fixtures do NOT extract to native script, so RTL is covered by synthetic UDHR Article-1 strings instead.

---

## File Structure

- `oxidize-pdf-core/Cargo.toml` — add optional `whatlang` dep + `language-detection` feature. (Already applied in the working tree during planning; Task 1 commits it.)
- `oxidize-pdf-core/src/ai/chunking.rs` — `DetectedLanguage` type; `ChunkMetadata.language` field; `DocumentChunker.detect_language` field + `with_language_detection` builder; `detect_chunk_language` cfg-gated helper; wiring in `chunk_text_internal`; `document_language` associated fn.
- `oxidize-pdf-core/src/ai/mod.rs` — re-export `DetectedLanguage`.
- `oxidize-pdf-core/tests/language_detection_test.rs` — new integration test file (synthetic + edge + aggregate + flag semantics).
- `oxidize-pdf-core/tests/language_detection_corpus_test.rs` — new integration test file (CJK corpus, gated on `language-detection` + `multilingual-fixtures`).

---

## Task 1: Add `whatlang` dependency and `language-detection` feature

**Files:**
- Modify: `oxidize-pdf-core/Cargo.toml`

- [ ] **Step 1: Verify the optional dependency is present**

In `oxidize-pdf-core/Cargo.toml`, in `[dependencies]`, immediately after the
`time = { version = "0.3", optional = true }` line and before `# Unicode processing`:

```toml
# Language detection (opt-in via `language-detection` feature; pure Rust, trigram + script based)
whatlang = { version = "0.18", optional = true }
```

- [ ] **Step 2: Verify the feature is present**

In `[features]`, immediately after `semantic = ["dep:serde_json"]`:

```toml
# Language detection for RAG chunks (opt-in: pulls pure-Rust `whatlang`)
language-detection = ["dep:whatlang"]
```

- [ ] **Step 3: Verify default build does not pull whatlang**

Run: `cargo tree --no-default-features --features compression | grep -c whatlang`
Expected: `0`

- [ ] **Step 4: Verify feature build resolves whatlang**

Run: `cargo tree --features language-detection | grep -m1 whatlang`
Expected: a line `whatlang v0.18.0`

- [ ] **Step 5: Commit**

```bash
git add oxidize-pdf-core/Cargo.toml Cargo.lock
git commit -m "feat(rag): add optional whatlang dep + language-detection feature (#293)"
```

---

## Task 2: `DetectedLanguage` type, `ChunkMetadata.language` field, re-export

**Files:**
- Modify: `oxidize-pdf-core/src/ai/chunking.rs` (the `ChunkMetadata` struct ~line 62)
- Modify: `oxidize-pdf-core/src/ai/mod.rs:33`

- [ ] **Step 1: Add the `DetectedLanguage` type above `ChunkMetadata`**

In `oxidize-pdf-core/src/ai/chunking.rs`, immediately before `/// Metadata for a document chunk`:

```rust
/// A language detected for a chunk or aggregated over a document.
///
/// `code` is the ISO 639-3 code (e.g. `"eng"`, `"spa"`, `"cmn"`). The
/// underlying detector (`whatlang`) is an internal implementation detail and is
/// not part of this public API.
#[derive(Debug, Clone, PartialEq)]
pub struct DetectedLanguage {
    /// ISO 639-3 language code.
    pub code: String,
    /// Detector confidence in `[0.0, 1.0]`.
    pub confidence: f32,
    /// Whether the detector considers this detection reliable.
    pub reliable: bool,
}
```

- [ ] **Step 2: Add the `language` field to `ChunkMetadata`**

In the `ChunkMetadata` struct, after the `sentence_boundary_respected: bool,` field:

```rust
    /// Detected language for this chunk, if language detection ran
    /// (`DocumentChunker::with_language_detection(true)` + the
    /// `language-detection` feature). `None` otherwise.
    pub language: Option<DetectedLanguage>,
```

(The struct keeps `#[derive(Debug, Clone, Default)]`; `Option` defaults to `None`.)

- [ ] **Step 3: Re-export the type**

In `oxidize-pdf-core/src/ai/mod.rs:33`, change:

```rust
pub use chunking::{ChunkMetadata, ChunkPosition, DocumentChunk, DocumentChunker};
```
to:
```rust
pub use chunking::{
    ChunkMetadata, ChunkPosition, DetectedLanguage, DocumentChunk, DocumentChunker,
};
```

- [ ] **Step 4: Fix the existing `ChunkMetadata` literal**

In `chunk_text_internal` (~line 340), the `ChunkMetadata { ... }` literal must
now set `language`. Add `language: None,` after `sentence_boundary_respected,`:

```rust
                metadata: ChunkMetadata {
                    position: ChunkPosition {
                        start_char,
                        end_char,
                        first_page: first_pg,
                        last_page: last_pg,
                    },
                    confidence: 1.0, // Default high confidence for text-based chunking
                    sentence_boundary_respected,
                    language: None,
                },
```

- [ ] **Step 5: Verify it compiles (default and feature)**

Run: `cargo build -p oxidize-pdf --lib`
Expected: Finished, no errors.
Run: `cargo build -p oxidize-pdf --lib --features language-detection`
Expected: Finished, no errors.

- [ ] **Step 6: Commit**

```bash
git add oxidize-pdf-core/src/ai/chunking.rs oxidize-pdf-core/src/ai/mod.rs
git commit -m "feat(rag): add DetectedLanguage type and ChunkMetadata.language field (#293)"
```

---

## Task 3: Per-chunk detection — builder, cfg-gated helper, wiring

**Files:**
- Modify: `oxidize-pdf-core/src/ai/chunking.rs`
- Test: `oxidize-pdf-core/tests/language_detection_test.rs` (create)

- [ ] **Step 1: Write the failing tests**

Create `oxidize-pdf-core/tests/language_detection_test.rs`:

```rust
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
fn empty_and_short_text_yield_none() {
    let chunker = DocumentChunker::new(512, 0).with_language_detection(true);
    assert!(chunker.chunk_text("").unwrap().is_empty());
    // "ab" is one chunk but too short to detect.
    let chunks = chunker.chunk_text("ab").unwrap();
    assert!(chunks[0].metadata.language.is_none());
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p oxidize-pdf --features language-detection --test language_detection_test 2>&1 | head -30`
Expected: compile error — `no method named with_language_detection found for struct DocumentChunker`.

- [ ] **Step 3: Add the `detect_language` field and builder**

In `oxidize-pdf-core/src/ai/chunking.rs`, change the `DocumentChunker` struct:

```rust
pub struct DocumentChunker {
    /// Target size for each chunk in tokens
    chunk_size: usize,

    /// Number of tokens to overlap between consecutive chunks
    overlap: usize,

    /// Whether to run per-chunk language detection
    detect_language: bool,
}
```

In `new`, set the new field:

```rust
    pub fn new(chunk_size: usize, overlap: usize) -> Self {
        Self {
            chunk_size,
            overlap,
            detect_language: false,
        }
    }
```

(`default()` calls `new`, so it inherits `detect_language: false`.)

Add the builder method inside `impl DocumentChunker`, right after `default()`:

```rust
    /// Enable per-chunk language detection.
    ///
    /// Requires the `language-detection` feature; without it this flag is a
    /// no-op and `ChunkMetadata::language` stays `None`. Disabled by default.
    pub fn with_language_detection(mut self, enabled: bool) -> Self {
        self.detect_language = enabled;
        self
    }
```

- [ ] **Step 4: Add the cfg-gated detection helper**

In `oxidize-pdf-core/src/ai/chunking.rs`, at module level (e.g. directly after
the `DetectedLanguage` definition), add both cfg variants:

```rust
/// Detect the language of `text` using `whatlang`. Returns `None` for
/// undetectable / too-short input.
#[cfg(feature = "language-detection")]
fn detect_chunk_language(text: &str) -> Option<DetectedLanguage> {
    whatlang::detect(text).map(|info| DetectedLanguage {
        code: info.lang().code().to_string(),
        confidence: info.confidence() as f32,
        reliable: info.is_reliable(),
    })
}

/// No-op when the `language-detection` feature is disabled.
#[cfg(not(feature = "language-detection"))]
fn detect_chunk_language(_text: &str) -> Option<DetectedLanguage> {
    None
}
```

- [ ] **Step 5: Wire detection into `chunk_text_internal`**

In `chunk_text_internal`, immediately after `let content = chunk_tokens.join(" ");`
and before `// Calculate character positions`, add:

```rust
            // Detect language for this chunk (no-op unless enabled + feature on)
            let language = if self.detect_language {
                detect_chunk_language(&content)
            } else {
                None
            };
```

Then in the `ChunkMetadata { ... }` literal, replace `language: None,` (added in
Task 2) with `language,`:

```rust
                metadata: ChunkMetadata {
                    position: ChunkPosition {
                        start_char,
                        end_char,
                        first_page: first_pg,
                        last_page: last_pg,
                    },
                    confidence: 1.0,
                    sentence_boundary_respected,
                    language,
                },
```

- [ ] **Step 6: Run the tests to verify they pass**

Run: `cargo test -p oxidize-pdf --features language-detection --test language_detection_test 2>&1 | tail -20`
Expected: `test result: ok. 6 passed; 0 failed`.

- [ ] **Step 7: Verify default build still has the field as None path**

Run: `cargo build -p oxidize-pdf --lib`
Expected: Finished (the `#[cfg(not(...))]` helper compiles; `detect_language` field unused-warning-free because it is read in `chunk_text_internal`).

- [ ] **Step 8: Commit**

```bash
git add oxidize-pdf-core/src/ai/chunking.rs oxidize-pdf-core/tests/language_detection_test.rs
git commit -m "feat(rag): per-chunk language detection via whatlang (#293)"
```

---

## Task 4: Document-level aggregate (`document_language`)

**Files:**
- Modify: `oxidize-pdf-core/src/ai/chunking.rs`
- Test: `oxidize-pdf-core/tests/language_detection_test.rs`

- [ ] **Step 1: Write the failing tests**

Append to `oxidize-pdf-core/tests/language_detection_test.rs`:

```rust
use oxidize_pdf::ai::{ChunkMetadata, ChunkPosition, DocumentChunk};

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
    assert!(doc.reliable, "reliable if any contributing chunk was reliable");
}
```

- [ ] **Step 2: Run to verify they fail**

Run: `cargo test -p oxidize-pdf --features language-detection --test language_detection_test 2>&1 | head -20`
Expected: compile error — `no function or associated item named document_language found`.

- [ ] **Step 3: Implement `document_language`**

In `oxidize-pdf-core/src/ai/chunking.rs`, inside `impl DocumentChunker`, add
(after `with_language_detection`). It needs `use std::collections::HashMap;` —
add it to the file's imports if not already present:

```rust
    /// Dominant language across chunks that already carry a detected language,
    /// weighted by chunk content length (chars). Returns `None` if no chunk has
    /// a language.
    ///
    /// `confidence` is the length-weighted mean of the winning code's chunk
    /// confidences; `reliable` is true if any contributing chunk for the winning
    /// code was reliable.
    pub fn document_language(chunks: &[DocumentChunk]) -> Option<DetectedLanguage> {
        // Per-code accumulators: (total_weight, weighted_confidence_sum, any_reliable)
        let mut acc: HashMap<String, (usize, f64, bool)> = HashMap::new();
        for chunk in chunks {
            if let Some(lang) = &chunk.metadata.language {
                let weight = chunk.content.chars().count().max(1);
                let entry = acc.entry(lang.code.clone()).or_insert((0, 0.0, false));
                entry.0 += weight;
                entry.1 += weight as f64 * lang.confidence as f64;
                entry.2 |= lang.reliable;
            }
        }

        // Winner = highest total weight; tie broken by code for determinism.
        let (code, (total_weight, conf_sum, reliable)) = acc
            .into_iter()
            .max_by(|a, b| a.1 .0.cmp(&b.1 .0).then_with(|| b.0.cmp(&a.0)))?;

        Some(DetectedLanguage {
            code,
            confidence: (conf_sum / total_weight as f64) as f32,
            reliable,
        })
    }
```

- [ ] **Step 4: Run to verify they pass**

Run: `cargo test -p oxidize-pdf --features language-detection --test language_detection_test 2>&1 | tail -20`
Expected: `test result: ok. 10 passed; 0 failed`.

- [ ] **Step 5: Commit**

```bash
git add oxidize-pdf-core/src/ai/chunking.rs oxidize-pdf-core/tests/language_detection_test.rs
git commit -m "feat(rag): document-level language aggregate (#293)"
```

---

## Task 5: CJK corpus tests (real extraction)

**Files:**
- Test: `oxidize-pdf-core/tests/language_detection_corpus_test.rs` (create)

- [ ] **Step 1: Write the corpus tests**

Create `oxidize-pdf-core/tests/language_detection_corpus_test.rs`:

```rust
//! End-to-end language detection over the CJK multilingual corpus (#293).
//! Verifies that text extracted from real PDFs is detected at the document
//! level. RTL fixtures (Arabic/Hebrew) are excluded: they do not currently
//! extract to native-script Unicode (separate extraction gap), so RTL detection
//! is covered by synthetic strings in language_detection_test.rs.
#![cfg(all(feature = "language-detection", feature = "multilingual-fixtures"))]

use oxidize_pdf::ai::DocumentChunker;
use oxidize_pdf::parser::PdfReader;
use oxidize_pdf::text::TextExtractor;
use std::path::Path;

fn extract_pages(filename: &str) -> Vec<(usize, String)> {
    let path = Path::new("tests/fixtures/multilingual").join(filename);
    let doc = PdfReader::open_document(&path)
        .unwrap_or_else(|e| panic!("open {filename}: {e:?}"));
    let n = doc.page_count().unwrap();
    let mut ex = TextExtractor::new();
    let mut pages = Vec::new();
    for i in 0..n {
        if let Ok(r) = ex.extract_from_page(&doc, i) {
            pages.push((i + 1, r.text));
        }
    }
    pages
}

fn detect_corpus_language(filename: &str) -> String {
    let pages = extract_pages(filename);
    let chunker = DocumentChunker::new(512, 50).with_language_detection(true);
    let chunks = chunker.chunk_text_with_pages(&pages).unwrap();
    let lang = DocumentChunker::document_language(&chunks)
        .unwrap_or_else(|| panic!("no language detected for {filename}"));
    assert!(lang.reliable, "{filename}: detection should be reliable, got {lang:?}");
    lang.code
}

#[test]
fn detects_chinese_corpus() {
    assert_eq!(detect_corpus_language("udhr_chinese.pdf"), "cmn");
}

#[test]
fn detects_japanese_corpus() {
    assert_eq!(detect_corpus_language("udhr_japanese.pdf"), "jpn");
}

#[test]
fn detects_korean_corpus() {
    assert_eq!(detect_corpus_language("udhr_korean.pdf"), "kor");
}
```

- [ ] **Step 2: Run to verify they pass**

Run: `cargo test -p oxidize-pdf --features language-detection,multilingual-fixtures --test language_detection_corpus_test 2>&1 | tail -20`
Expected: `test result: ok. 3 passed; 0 failed`.

(If this fails, do NOT weaken the assertion to a smoke test. The spike confirmed
cmn/jpn/kor extract at confidence 1.0; a failure means a real regression in
extraction or detection — debug that.)

- [ ] **Step 3: Commit**

```bash
git add oxidize-pdf-core/tests/language_detection_corpus_test.rs
git commit -m "test(rag): CJK corpus language detection (#293)"
```

---

## Task 6: Full verification (feature matrix, MSRV, clippy, lints)

**Files:** none (verification only)

- [ ] **Step 1: Default build + lib tests (no regression)**

Run: `cargo test -p oxidize-pdf --lib`
Expected: `6500 passed; 0 failed` (or current baseline), no new failures.

- [ ] **Step 2: Feature build clean on MSRV 1.88**

Run: `cargo +1.88 build --lib --all-features --locked`
Expected: Finished, 0 warnings.

- [ ] **Step 3: Clippy (CI-equivalent gate) with the feature**

Run: `cargo clippy --all --features language-detection -- -D warnings`
Expected: Finished, exit 0.

- [ ] **Step 4: All-features test run (corpus included)**

Run: `cargo test -p oxidize-pdf --features language-detection,multilingual-fixtures --test language_detection_test --test language_detection_corpus_test 2>&1 | tail -10`
Expected: all pass (10 + 3).

- [ ] **Step 5: fmt**

Run: `cargo fmt --all -- --check`
Expected: no output (clean).

- [ ] **Step 6: Update CHANGELOG**

In `CHANGELOG.md`, under `## [Unreleased]`, add an `### Added` section (above the
existing `### Changed`):

```markdown
### Added

- Per-chunk and document-level language detection for RAG chunks, behind the
  opt-in `language-detection` feature (pure-Rust `whatlang`). `ChunkMetadata`
  gains `language: Option<DetectedLanguage>` (ISO 639-3 code + confidence +
  reliability); enable via `DocumentChunker::with_language_detection(true)`.
  `DocumentChunker::document_language(&chunks)` returns the dominant language
  weighted by chunk length (#293).
```

Then commit:

```bash
git add CHANGELOG.md
git commit -m "docs(changelog): language detection feature (#293)"
```

---

## Self-Review Notes

- **Spec coverage:** feature gate (Task 1), `DetectedLanguage` + field + re-export (Task 2), builder + helper + wiring (Task 3), `document_language` aggregate (Task 4), corpus + synthetic tests covering all spec test cases incl. edge/flag-off (Tasks 3–5). RTL handling matches the spec's revised testing section.
- **Type consistency:** `DetectedLanguage { code: String, confidence: f32, reliable: bool }`, `with_language_detection(bool) -> Self`, `document_language(&[DocumentChunk]) -> Option<DetectedLanguage>`, `detect_chunk_language(&str) -> Option<DetectedLanguage>` — used identically across tasks.
- **No placeholders:** all codes (eng/spa/ara/heb/cmn/jpn/kor) and synthetic strings empirically verified 2026-06-06.
