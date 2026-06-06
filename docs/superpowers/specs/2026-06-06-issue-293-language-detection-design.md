# Design — per-chunk/document language detection (#293)

**Date:** 2026-06-06
**Issue:** #293 (RAG: per-chunk/document language detection)
**Branch:** `feature/issue-293-language-detection` (from `chore/bump-msrv-1.88`)

## Motivation

There is no language detection in the codebase. For RAG ingestion, per-chunk
language enables routing to the correct embedding model, multilingual filtering,
and a language hint for OCR. This adds language detection to the
`ai/chunking.rs` chunk path, exposed on `ChunkMetadata`.

Scope is the `DocumentChunker` / `ChunkMetadata` path only. The pipeline
`RagChunk` type (`parser/document.rs::rag_chunks`) is explicitly out of scope
(possible follow-up).

## Dependency

`whatlang` 0.18.0 — pure-Rust trigram + script language detector. Edition 2024
(requires Rust 1.85), compatible with the project MSRV 1.88 (bumped in #296,
the base branch). Verified API:

- `whatlang::detect(text: &str) -> Option<Info>`
- `Info::lang() -> Lang`, `Info::confidence() -> f64`, `Info::is_reliable() -> bool`
- `Lang::code() -> &'static str` (ISO 639-3, e.g. `"eng"`, `"spa"`, `"cmn"`)

### Feature gate

```toml
[features]
language-detection = ["dep:whatlang"]
```

```toml
whatlang = { version = "0.18", optional = true }
```

Not in `default`. Consistent with the project pattern (`external-images`,
`ocr-tesseract`, `signatures`, `semantic`). Default build does not pull
`whatlang`.

## Public API

New type in `ai/chunking.rs`, re-exported from `ai/mod.rs`:

```rust
/// A language detected for a chunk or document.
#[derive(Debug, Clone, PartialEq)]
pub struct DetectedLanguage {
    /// ISO 639-3 code (whatlang `Lang::code()`), e.g. "eng", "spa", "cmn".
    pub code: String,
    /// Detector confidence in [0.0, 1.0].
    pub confidence: f32,
    /// Whether whatlang considers the detection reliable.
    pub reliable: bool,
}
```

`DetectedLanguage` is an owned type: `whatlang` stays an internal implementation
detail and is NOT part of the public API surface, so a future `whatlang` major
bump does not force a major bump here.

`ChunkMetadata` gains:

```rust
pub language: Option<DetectedLanguage>,
```

The field exists in every build (with or without the `language-detection`
feature). It is `None` by `#[derive(Default)]` and stays `None` unless detection
runs. Adding a field to this struct is a minor (additive) change; the struct is
already `#[derive(Debug, Clone, Default)]` and is constructed internally.

## Detection trigger (runtime, zero-cost when off)

`DocumentChunker` gains a field and a builder method:

```rust
pub struct DocumentChunker {
    chunk_size: usize,
    overlap: usize,
    detect_language: bool, // default false
}

impl DocumentChunker {
    /// Enable per-chunk language detection (requires the `language-detection`
    /// feature; a no-op otherwise). Default: disabled.
    pub fn with_language_detection(mut self, enabled: bool) -> Self {
        self.detect_language = enabled;
        self
    }
}
```

`new()` and `default()` set `detect_language: false`, preserving current
behavior (non-breaking).

During chunk construction (in the shared `chunk_text_with_pages` path so all
public entry points benefit), when `self.detect_language` is true:

```rust
#[cfg(feature = "language-detection")]
fn detect_chunk_language(text: &str) -> Option<DetectedLanguage> {
    whatlang::detect(text).map(|info| DetectedLanguage {
        code: info.lang().code().to_string(),
        confidence: info.confidence() as f32,
        reliable: info.is_reliable(),
    })
}

#[cfg(not(feature = "language-detection"))]
fn detect_chunk_language(_text: &str) -> Option<DetectedLanguage> {
    None
}
```

`metadata.language` is set from this helper. Without the feature, the flag is a
no-op and the field stays `None` (documented on `with_language_detection`).

## Document-level aggregate

Associated function on `DocumentChunker`:

```rust
/// Dominant language across already-detected chunks, weighted by chunk content
/// length. Returns `None` if no chunk carries a detected language.
pub fn document_language(chunks: &[DocumentChunk]) -> Option<DetectedLanguage>
```

Algorithm:

1. For each chunk with `metadata.language = Some(lang)`, accumulate weight
   `chunk.content.chars().count()` into a per-`code` total.
2. Winner = `code` with the highest total weight.
3. `confidence` = weight-weighted mean of that code's chunk confidences.
4. `reliable` = true if at least one contributing chunk for the winning code was
   `reliable`.
5. `None` if no chunk had a language.

This reuses per-chunk detections (no second `whatlang` pass), gives the dominant
language for mixed-language documents by majority-of-length, and is composable
(operates on chunks, needs no `Document`).

## Data flow

```
Document/text
  -> DocumentChunker::with_language_detection(true)
  -> chunk_text_with_pages
       per chunk: detect_chunk_language(content) -> metadata.language
  -> Vec<DocumentChunk>           (each chunk carries Option<DetectedLanguage>)
  -> DocumentChunker::document_language(&chunks) -> Option<DetectedLanguage>
```

## Error handling

Detection never errors: `whatlang::detect` returns `Option`, mapped to
`Option<DetectedLanguage>`. Empty / too-short / undetectable text yields `None`.
No new `Result` variants; chunking signatures are unchanged.

## Testing (content-verifying, no smoke tests)

Tests gated on `#[cfg(all(feature = "language-detection", feature = "multilingual-fixtures"))]`
for the corpus tests; the synthetic-text and aggregate tests need only
`language-detection`.

1. **Multilingual corpus** — for each
   `oxidize-pdf-core/tests/fixtures/multilingual/udhr_{chinese,japanese,korean,arabic,hebrew}.pdf`:
   extract text, chunk with detection enabled, assert each chunk's
   `metadata.language.code` equals the expected ISO 639-3 code (exact codes
   asserted against the real `whatlang` output, pinned during implementation)
   and `confidence > 0.0`.
2. **Known Latin-script text** — fixed multi-sentence English and Spanish
   strings (long enough for reliable detection) via `chunk_text` assert
   `code == "eng"` / `code == "spa"` and `reliable == true`. Reliability is
   only asserted on this sufficiently-long text, never on short inputs.
3. **Mixed-language document** — synthetic text with distinct-language sections
   sized to fall in different chunks: assert per-chunk `code` differs and
   `document_language` returns the dominant-by-length code.
4. **Edge cases** — empty string and a 2–3 char string assert
   `metadata.language == None`; `document_language(&[])` returns `None`.
5. **Feature/flag semantics** — with detection flag NOT set, `language` is
   `None` even when the feature is compiled in.

## Out of scope (YAGNI)

- `RagChunk` (pipeline) language field.
- Exposing `whatlang::Script`.
- ISO 639-1 (2-letter) mapping.
- Per-call language allowlist / `whatlang::Options` denylist tuning.
