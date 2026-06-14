# Analysis SPI (v1) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a `ChunkingStrategy` SPI + an open `extra` metadata bag so a closed crate can plug in custom chunking and surface domain fields, without forking the MIT core.

**Architecture:** A new `ChunkingStrategy` trait returns `ChunkGroup`s (element groupings); the pipeline owns everything downstream (RagChunk, ids, links, metadata). `HybridChunker` becomes the default impl. A new `AnalysisPipeline` builder + `PdfDocument::rag_chunks_with_pipeline` entry point run a strategy and reuse the existing `build_rag_chunks` machinery. `ChunkMetadata` gains an open `extra: BTreeMap<String, serde_json::Value>` bag. All SPI items are behind `unstable-spi`; `extra` is behind `semantic`. Every existing entry point is unchanged.

**Tech Stack:** Rust (MSRV 1.88), `serde_json` (already a dep under `semantic`). No new dependencies.

**Spec:** `docs/superpowers/specs/2026-06-13-analysis-spi-design.md`

## Prerequisites / sequencing

- This plan targets `develop`. It modifies `chunk_metadata.rs`, `rag.rs`,
  `hybrid_chunking.rs`, `parser/document.rs`, `pipeline/mod.rs`, `Cargo.toml`.
- PR #326 (`feature/chunk-metadata-enrichment`) also modifies `chunk_metadata.rs`
  and `rag.rs`. **Merge #326 to develop first, then rebase this branch onto it**
  so the SPI, the enrichment, and the bridge work ship together (per spec §9) and
  to avoid conflicts in the metadata struct.
- Branch for this work: `feature/analysis-spi` (already created).

## File Structure

- **Create** `oxidize-pdf-core/src/pipeline/spi.rs` — `ChunkGroup`, `ChunkingStrategy`, `AnalysisPipeline`. One responsibility: the SPI surface. All `#[cfg(feature = "unstable-spi")]`.
- **Create** `oxidize-pdf-core/tests/analysis_spi_test.rs` — integration tests (custom strategy, decorator, parity, source stamping). Gated `#![cfg(feature = "unstable-spi")]`.
- **Modify** `oxidize-pdf-core/Cargo.toml` — add `unstable-spi = []`.
- **Modify** `oxidize-pdf-core/src/pipeline/mod.rs` — `mod spi;` + cfg-gated re-exports.
- **Modify** `oxidize-pdf-core/src/pipeline/hybrid_chunking.rs` — `HybridChunk::into_group`, `HybridChunk::from_group`, `impl ChunkingStrategy for HybridChunker`.
- **Modify** `oxidize-pdf-core/src/pipeline/chunk_metadata.rs` — `extra` field + construction + unit tests.
- **Modify** `oxidize-pdf-core/src/parser/document.rs` — `autofill_source` helper + `rag_chunks_with_pipeline`.

---

### Task 1: `ChunkGroup` + `ChunkingStrategy` trait (feature + module)

**Files:**
- Modify: `oxidize-pdf-core/Cargo.toml`
- Create: `oxidize-pdf-core/src/pipeline/spi.rs`
- Modify: `oxidize-pdf-core/src/pipeline/mod.rs`
- Test: `oxidize-pdf-core/tests/analysis_spi_test.rs`

- [ ] **Step 1: Add the feature**

In `oxidize-pdf-core/Cargo.toml`, under `[features]`, add:

```toml
# Unstable analysis SPI (ChunkingStrategy + AnalysisPipeline). Exempt from
# semver while experimental; may change until promoted to a stable feature.
unstable-spi = []
```

- [ ] **Step 2: Create the SPI module**

Create `oxidize-pdf-core/src/pipeline/spi.rs`:

```rust
//! Unstable analysis SPI — extension points for the chunking pipeline.
//!
//! Behind the `unstable-spi` feature. The trait surface is exempt from semver
//! while experimental and may change until promoted.

use crate::pipeline::element::Element;

/// A grouping of elements destined to become one chunk. The chunking strategy
/// decides the boundaries; the pipeline owns everything downstream (RagChunk,
/// chunk_id, links, metadata).
#[non_exhaustive]
pub struct ChunkGroup {
    /// The elements that form this chunk, in order.
    pub elements: Vec<Element>,
    /// Optional heading context to prepend for embedding.
    pub heading_context: Option<String>,
}

impl ChunkGroup {
    /// Construct a group from elements and an optional heading context.
    pub fn new(elements: Vec<Element>, heading_context: Option<String>) -> Self {
        Self {
            elements,
            heading_context,
        }
    }
}

/// Decides which elements group into a chunk. Implement this in a (possibly
/// closed) crate to override how the pipeline forms chunks. The pipeline
/// computes `oversized`, `chunk_id`, prev/next links, and `ChunkMetadata`.
pub trait ChunkingStrategy: Send + Sync {
    /// Group `elements` into chunks. Called once per document.
    fn chunk(&self, elements: &[Element]) -> Vec<ChunkGroup>;
}
```

- [ ] **Step 3: Wire the module + re-exports**

In `oxidize-pdf-core/src/pipeline/mod.rs`, after the existing `pub mod ...` lines, add:

```rust
#[cfg(feature = "unstable-spi")]
pub mod spi;
```

And after the existing `pub use ...` block, add:

```rust
#[cfg(feature = "unstable-spi")]
pub use spi::{ChunkGroup, ChunkingStrategy};
```

- [ ] **Step 4: Write the failing test**

Create `oxidize-pdf-core/tests/analysis_spi_test.rs`:

```rust
//! Integration tests for the unstable analysis SPI.
#![cfg(feature = "unstable-spi")]

use oxidize_pdf::pipeline::{ChunkGroup, ChunkingStrategy};
use oxidize_pdf::pipeline::{Element, ElementData, ElementMetadata};

/// A strategy that emits exactly one chunk per element.
struct OnePerElement;

impl ChunkingStrategy for OnePerElement {
    fn chunk(&self, elements: &[Element]) -> Vec<ChunkGroup> {
        elements
            .iter()
            .map(|e| ChunkGroup::new(vec![e.clone()], None))
            .collect()
    }
}

fn para(text: &str) -> Element {
    Element::Paragraph(ElementData {
        text: text.to_string(),
        metadata: ElementMetadata::default(),
    })
}

#[test]
fn custom_strategy_is_object_safe_and_groups_per_element() {
    let strategy: Box<dyn ChunkingStrategy> = Box::new(OnePerElement);
    let elements = vec![para("alpha"), para("bravo"), para("charlie")];
    let groups = strategy.chunk(&elements);
    assert_eq!(groups.len(), 3, "one chunk per element");
    assert_eq!(groups[0].elements.len(), 1);
    assert_eq!(groups[0].elements[0].text(), "alpha");
    assert_eq!(groups[2].elements[0].text(), "charlie");
}
```

> Note: `Element::text()` is the accessor used elsewhere in tests; confirm it exists in `pipeline/element.rs` (it does — used by `chunk_metadata` tests). If the variant constructor differs, mirror the `para` helper in `tests/rag_chunk_test.rs`.

- [ ] **Step 5: Run the test**

Run: `cargo test -p oxidize-pdf --features unstable-spi --test analysis_spi_test custom_strategy_is_object_safe_and_groups_per_element`
Expected: PASS (the trait + type are defined; the test exercises object-safety via `Box<dyn>`).

- [ ] **Step 6: Verify the default build is unaffected**

Run: `cargo build -p oxidize-pdf`
Expected: success; `spi` module not compiled (no feature).

- [ ] **Step 7: Commit**

```bash
git add oxidize-pdf-core/Cargo.toml oxidize-pdf-core/src/pipeline/spi.rs oxidize-pdf-core/src/pipeline/mod.rs oxidize-pdf-core/tests/analysis_spi_test.rs
git commit -m "feat(spi): ChunkingStrategy trait + ChunkGroup behind unstable-spi"
```

---

### Task 2: `HybridChunker` as the default `ChunkingStrategy`

**Files:**
- Modify: `oxidize-pdf-core/src/pipeline/hybrid_chunking.rs`
- Test: `oxidize-pdf-core/tests/analysis_spi_test.rs`

- [ ] **Step 1: Write the failing test**

Append to `oxidize-pdf-core/tests/analysis_spi_test.rs`:

```rust
use oxidize_pdf::pipeline::{HybridChunkConfig, HybridChunker, MergePolicy};

#[test]
fn hybrid_chunker_is_the_default_strategy() {
    let elements = vec![para("alpha one two three"), para("bravo four five six")];
    let chunker = HybridChunker::new(HybridChunkConfig {
        max_tokens: 4,
        overlap_tokens: 0,
        merge_adjacent: true,
        propagate_headings: true,
        merge_policy: MergePolicy::AnyInlineContent,
    });

    // Inherent API: Vec<HybridChunk>.
    let hybrid = HybridChunker::chunk(&chunker, &elements);
    // Trait API: Vec<ChunkGroup>, same grouping.
    let groups = ChunkingStrategy::chunk(&chunker, &elements);

    assert_eq!(groups.len(), hybrid.len(), "same number of chunks");
    for (g, h) in groups.iter().zip(hybrid.iter()) {
        let g_text: Vec<&str> = g.elements.iter().map(|e| e.text()).collect();
        let h_text: Vec<&str> = h.elements().iter().map(|e| e.text()).collect();
        assert_eq!(g_text, h_text, "same element grouping");
        assert_eq!(g.heading_context, h.heading_context);
    }
}
```

- [ ] **Step 2: Run it to verify it fails**

Run: `cargo test -p oxidize-pdf --features unstable-spi --test analysis_spi_test hybrid_chunker_is_the_default_strategy`
Expected: FAIL — `HybridChunker` does not implement `ChunkingStrategy` (trait method `chunk` not found via `ChunkingStrategy::chunk`).

- [ ] **Step 3: Implement `into_group` + the trait impl**

In `oxidize-pdf-core/src/pipeline/hybrid_chunking.rs`, inside `impl HybridChunk { ... }` (the block starting near the `pub fn elements` accessor), add:

```rust
    /// Convert into a [`ChunkGroup`](crate::pipeline::spi::ChunkGroup),
    /// dropping the derived `oversized` flag (the pipeline recomputes it).
    #[cfg(feature = "unstable-spi")]
    pub(crate) fn into_group(self) -> crate::pipeline::spi::ChunkGroup {
        crate::pipeline::spi::ChunkGroup {
            elements: self.elements,
            heading_context: self.heading_context,
        }
    }
```

At the end of the file (module scope), add the trait impl:

```rust
#[cfg(feature = "unstable-spi")]
impl crate::pipeline::spi::ChunkingStrategy for HybridChunker {
    fn chunk(&self, elements: &[Element]) -> Vec<crate::pipeline::spi::ChunkGroup> {
        // Call the inherent method explicitly to avoid recursing into this impl.
        HybridChunker::chunk(self, elements)
            .into_iter()
            .map(HybridChunk::into_group)
            .collect()
    }
}
```

- [ ] **Step 4: Run the test to verify it passes**

Run: `cargo test -p oxidize-pdf --features unstable-spi --test analysis_spi_test hybrid_chunker_is_the_default_strategy`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add oxidize-pdf-core/src/pipeline/hybrid_chunking.rs oxidize-pdf-core/tests/analysis_spi_test.rs
git commit -m "feat(spi): HybridChunker implements ChunkingStrategy (default impl)"
```

---

### Task 3: `HybridChunk::from_group` (ChunkGroup → HybridChunk)

**Files:**
- Modify: `oxidize-pdf-core/src/pipeline/hybrid_chunking.rs`

- [ ] **Step 1: Write the failing unit test**

In `oxidize-pdf-core/src/pipeline/hybrid_chunking.rs`, find the `#[cfg(test)] mod tests { ... }` block and add (inside it):

```rust
    #[cfg(feature = "unstable-spi")]
    #[test]
    fn from_group_recomputes_oversized_and_preserves_content() {
        use crate::pipeline::spi::ChunkGroup;

        let big = Element::Paragraph(ElementData {
            text: "one two three four five six seven eight".to_string(),
            metadata: ElementMetadata::default(),
        });
        // Budget far below the token count → oversized.
        let group = ChunkGroup::new(vec![big.clone()], Some("H".to_string()));
        let hc = HybridChunk::from_group(group, 2);
        assert!(hc.is_oversized(), "8-word chunk over a 2-token budget is oversized");
        assert_eq!(hc.heading_context.as_deref(), Some("H"));
        assert_eq!(hc.elements().len(), 1);

        // Generous budget → not oversized.
        let group2 = ChunkGroup::new(vec![big], None);
        let hc2 = HybridChunk::from_group(group2, 100);
        assert!(!hc2.is_oversized());
    }
```

> Note: confirm the test module already imports `Element`, `ElementData`, `ElementMetadata`. If not, add `use crate::pipeline::element::{Element, ElementData, ElementMetadata};` at the top of the `tests` module.

- [ ] **Step 2: Run it to verify it fails**

Run: `cargo test -p oxidize-pdf --features unstable-spi --lib hybrid_chunking::tests::from_group_recomputes_oversized_and_preserves_content`
Expected: FAIL — `HybridChunk::from_group` not found.

- [ ] **Step 3: Implement `from_group`**

In `oxidize-pdf-core/src/pipeline/hybrid_chunking.rs`, inside `impl HybridChunk { ... }`, add:

```rust
    /// Build a chunk from a [`ChunkGroup`](crate::pipeline::spi::ChunkGroup),
    /// recomputing `oversized` against `max_tokens`. Used by the pipeline when a
    /// custom strategy produced the grouping.
    #[cfg(feature = "unstable-spi")]
    pub(crate) fn from_group(
        group: crate::pipeline::spi::ChunkGroup,
        max_tokens: usize,
    ) -> Self {
        let text = group
            .elements
            .iter()
            .map(|e| e.display_text())
            .collect::<Vec<_>>()
            .join("\n");
        let oversized = estimate_tokens(&text) > max_tokens;
        HybridChunk {
            elements: group.elements,
            heading_context: group.heading_context,
            oversized,
        }
    }
```

- [ ] **Step 4: Run the test to verify it passes**

Run: `cargo test -p oxidize-pdf --features unstable-spi --lib hybrid_chunking::tests::from_group_recomputes_oversized_and_preserves_content`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add oxidize-pdf-core/src/pipeline/hybrid_chunking.rs
git commit -m "feat(spi): HybridChunk::from_group recomputes oversized vs budget"
```

---

### Task 4: `ChunkMetadata::extra` open bag

**Files:**
- Modify: `oxidize-pdf-core/src/pipeline/chunk_metadata.rs`

- [ ] **Step 1: Write the failing unit test**

In `oxidize-pdf-core/src/pipeline/chunk_metadata.rs`, inside `#[cfg(test)] mod tests { ... }`, add:

```rust
    #[cfg(feature = "semantic")]
    #[test]
    fn extra_bag_defaults_empty_and_roundtrips() {
        let mut m = ChunkMetadata::default();
        assert!(m.extra.is_empty(), "extra defaults to empty");

        // Empty extra is omitted from the serialized output.
        let json_empty = serde_json::to_string(&m).unwrap();
        assert!(
            !json_empty.contains("\"extra\""),
            "empty extra must be skipped in JSON"
        );

        // Populated extra survives a deterministic round-trip.
        m.extra
            .insert("legal.clause_number".to_string(), serde_json::json!("3.2"));
        m.extra.insert(
            "legal.defined_terms".to_string(),
            serde_json::json!(["Party", "Agreement"]),
        );
        let json = serde_json::to_string(&m).unwrap();
        assert!(json.contains("\"extra\""));
        let back: ChunkMetadata = serde_json::from_str(&json).unwrap();
        assert_eq!(back.extra, m.extra, "extra survives round-trip");
        assert_eq!(
            back.extra.get("legal.clause_number").unwrap(),
            &serde_json::json!("3.2")
        );
    }
```

- [ ] **Step 2: Run it to verify it fails**

Run: `cargo test -p oxidize-pdf --features semantic --lib chunk_metadata::tests::extra_bag_defaults_empty_and_roundtrips`
Expected: FAIL — no field `extra` on `ChunkMetadata`.

- [ ] **Step 3: Add the `use` for `BTreeMap`**

At the top of `oxidize-pdf-core/src/pipeline/chunk_metadata.rs`, after the existing `use` lines, add:

```rust
#[cfg(feature = "semantic")]
use std::collections::BTreeMap;
```

- [ ] **Step 4: Add the field to `ChunkMetadata`**

In the `pub struct ChunkMetadata { ... }` definition, add as the last field (after `source`):

```rust
    /// Open extension bag for provider-supplied fields (e.g. a closed analyzer
    /// stamping `legal.clause_number`). Namespacing keys by provider avoids
    /// collisions. Serializes nested under `"extra"`; omitted when empty.
    #[cfg(feature = "semantic")]
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub extra: BTreeMap<String, serde_json::Value>,
```

- [ ] **Step 5: Fill the field in `from_elements`**

In `ChunkMetadata::from_elements`, in the returned `ChunkMetadata { ... }` literal, add as the last field (after `source: None,`):

```rust
            #[cfg(feature = "semantic")]
            extra: BTreeMap::new(),
```

- [ ] **Step 6: Run the test to verify it passes**

Run: `cargo test -p oxidize-pdf --features semantic --lib chunk_metadata::tests::extra_bag_defaults_empty_and_roundtrips`
Expected: PASS.

- [ ] **Step 7: Verify default (no-feature) build still compiles**

Run: `cargo build -p oxidize-pdf`
Expected: success (the `extra` field is cfg-gated out without `semantic`).

- [ ] **Step 8: Commit**

```bash
git add oxidize-pdf-core/src/pipeline/chunk_metadata.rs
git commit -m "feat(spi): open ChunkMetadata.extra bag under semantic"
```

---

### Task 5: `AnalysisPipeline` + `rag_chunks_with_pipeline`

**Files:**
- Modify: `oxidize-pdf-core/src/pipeline/spi.rs`
- Modify: `oxidize-pdf-core/src/pipeline/mod.rs`
- Modify: `oxidize-pdf-core/src/parser/document.rs`
- Test: `oxidize-pdf-core/tests/analysis_spi_test.rs`

- [ ] **Step 1: Add `AnalysisPipeline` to the SPI module**

Append to `oxidize-pdf-core/src/pipeline/spi.rs`:

```rust
use crate::pipeline::hybrid_chunking::{HybridChunkConfig, HybridChunker};
use crate::pipeline::DocumentSource;

/// Configures the analysis pipeline: which chunking strategy to run, the token
/// budget used to flag oversized chunks, and optional source-document metadata.
pub struct AnalysisPipeline {
    pub(crate) chunking: Box<dyn ChunkingStrategy>,
    pub(crate) max_tokens: usize,
    pub(crate) source: Option<DocumentSource>,
}

impl Default for AnalysisPipeline {
    fn default() -> Self {
        Self::new()
    }
}

impl AnalysisPipeline {
    /// Default pipeline: the built-in `HybridChunker`, default token budget, no
    /// source. Reproduces `PdfDocument::rag_chunks()` exactly.
    pub fn new() -> Self {
        let config = HybridChunkConfig::default();
        Self {
            max_tokens: config.max_tokens,
            chunking: Box::new(HybridChunker::new(config)),
            source: None,
        }
    }

    /// Replace the chunking strategy.
    pub fn with_chunking(mut self, strategy: Box<dyn ChunkingStrategy>) -> Self {
        self.chunking = strategy;
        self
    }

    /// Set the token budget used to flag oversized chunks.
    pub fn with_max_tokens(mut self, max_tokens: usize) -> Self {
        self.max_tokens = max_tokens;
        self
    }

    /// Stamp source-document metadata onto every chunk.
    pub fn with_source(mut self, source: DocumentSource) -> Self {
        self.source = Some(source);
        self
    }
}
```

- [ ] **Step 2: Re-export `AnalysisPipeline`**

In `oxidize-pdf-core/src/pipeline/mod.rs`, update the cfg-gated re-export to:

```rust
#[cfg(feature = "unstable-spi")]
pub use spi::{AnalysisPipeline, ChunkGroup, ChunkingStrategy};
```

- [ ] **Step 3: Write the failing integration test**

Append to `oxidize-pdf-core/tests/analysis_spi_test.rs`:

```rust
use oxidize_pdf::parser::{PdfDocument, PdfReader};
use oxidize_pdf::pipeline::AnalysisPipeline;
use oxidize_pdf::text::Font;
use oxidize_pdf::{Document, Page};
use std::io::Cursor;

fn build_two_section_doc() -> Vec<u8> {
    let mut doc = Document::new();
    let mut page = Page::a4();
    page.text()
        .set_font(Font::HelveticaBold, 16.0)
        .at(50.0, 760.0)
        .write("Section One")
        .unwrap();
    page.text()
        .set_font(Font::Helvetica, 11.0)
        .at(50.0, 730.0)
        .write("First body paragraph with enough words to chunk on its own line.")
        .unwrap();
    page.text()
        .set_font(Font::Helvetica, 11.0)
        .at(50.0, 700.0)
        .write("Second body paragraph also with several words to fill a bucket.")
        .unwrap();
    doc.add_page(page);
    doc.to_bytes().expect("pdf generation")
}

#[test]
fn default_pipeline_matches_rag_chunks() {
    let bytes = build_two_section_doc();

    let parsed_a = PdfDocument::new(PdfReader::new(Cursor::new(&bytes)).unwrap());
    let baseline = parsed_a.rag_chunks().expect("rag_chunks");

    let parsed_b = PdfDocument::new(PdfReader::new(Cursor::new(&bytes)).unwrap());
    let via_pipeline = parsed_b
        .rag_chunks_with_pipeline(&AnalysisPipeline::new())
        .expect("rag_chunks_with_pipeline");

    assert_eq!(
        via_pipeline.len(),
        baseline.len(),
        "default pipeline produces the same chunk count"
    );
    for (p, b) in via_pipeline.iter().zip(baseline.iter()) {
        assert_eq!(p.text, b.text, "same chunk text");
        assert_eq!(p.metadata.chunk_id, b.metadata.chunk_id, "same chunk_id");
        assert_eq!(p.is_oversized, b.is_oversized, "same oversized flag");
        assert_eq!(
            p.metadata.prev_chunk_id, b.metadata.prev_chunk_id,
            "same prev link"
        );
    }
}

#[test]
fn custom_strategy_drives_chunk_count_and_pipeline_owns_ids() {
    let bytes = build_two_section_doc();
    let parsed = PdfDocument::new(PdfReader::new(Cursor::new(&bytes)).unwrap());

    let pipeline = AnalysisPipeline::new().with_chunking(Box::new(OnePerElement));
    let chunks = parsed
        .rag_chunks_with_pipeline(&pipeline)
        .expect("rag_chunks_with_pipeline");

    // One element per chunk → at least as many chunks as the default merge.
    assert!(chunks.len() >= 3, "one-per-element yields >= 3 chunks");
    // The pipeline (not the strategy) derived ids and links.
    assert!(chunks[0].metadata.prev_chunk_id.is_none());
    assert_eq!(
        chunks[0].metadata.next_chunk_id.as_deref(),
        Some(chunks[1].metadata.chunk_id.as_str()),
        "pipeline wired prev/next"
    );
    for c in &chunks {
        assert!(!c.metadata.chunk_id.is_empty());
    }
}
```

- [ ] **Step 4: Run it to verify it fails**

Run: `cargo test -p oxidize-pdf --features unstable-spi --test analysis_spi_test default_pipeline_matches_rag_chunks`
Expected: FAIL — no method `rag_chunks_with_pipeline`.

- [ ] **Step 5: Extract the source auto-fill helper (DRY)**

In `oxidize-pdf-core/src/parser/document.rs`, locate `rag_chunks_with_source_and_config`. It contains an `if let Ok(meta) = self.metadata() { ... }` block plus a `total_pages` fallback. Extract that into a private method on the same `impl` block:

```rust
    /// Fill `title`/`author`/`creation_date`/`total_pages` from the info
    /// dictionary where the caller left them `None`.
    fn autofill_source(&self, source: &mut crate::pipeline::DocumentSource) {
        if let Ok(meta) = self.metadata() {
            source.title = source.title.take().or(meta.title);
            source.author = source.author.take().or(meta.author);
            source.creation_date = source.creation_date.take().or(meta.creation_date);
            source.total_pages = source.total_pages.or(meta.page_count);
        }
        if source.total_pages.is_none() {
            source.total_pages = self.page_count().ok();
        }
    }
```

Then replace the inlined block in `rag_chunks_with_source_and_config` with a call:

```rust
        self.autofill_source(&mut source);
```

(keep the rest of `rag_chunks_with_source_and_config` unchanged).

- [ ] **Step 6: Add `rag_chunks_with_pipeline`**

In `oxidize-pdf-core/src/parser/document.rs`, in the same `impl<R: Read + Seek> PdfDocument<R>` block (near the other `rag_chunks*` methods), add:

```rust
    /// Run a custom [`AnalysisPipeline`](crate::pipeline::AnalysisPipeline):
    /// partition, apply the pipeline's chunking strategy, then build linked
    /// `RagChunk`s (ids, prev/next, metadata, optional source) exactly as the
    /// other `rag_chunks*` entry points do.
    ///
    /// `AnalysisPipeline::new()` reproduces [`rag_chunks`](Self::rag_chunks).
    #[cfg(feature = "unstable-spi")]
    pub fn rag_chunks_with_pipeline(
        &self,
        pipeline: &crate::pipeline::AnalysisPipeline,
    ) -> ParseResult<Vec<crate::pipeline::RagChunk>> {
        let mut source = pipeline.source.clone();
        if let Some(src) = source.as_mut() {
            self.autofill_source(src);
        }
        let elements = self.partition()?;
        let groups = pipeline.chunking.chunk(&elements);
        let hybrid: Vec<crate::pipeline::HybridChunk> = groups
            .into_iter()
            .map(|g| {
                crate::pipeline::HybridChunk::from_group(g, pipeline.max_tokens)
            })
            .collect();
        Ok(self.build_rag_chunks(&hybrid, source))
    }
```

> Note: `from_group` is `pub(crate)`, so it is callable from `parser/document.rs` (same crate). `build_rag_chunks` already maps `&[HybridChunk]` + `Option<DocumentSource>` → linked `Vec<RagChunk>` (it handles `from_hybrid_chunk` / `from_hybrid_chunk_with_source` + `link_chunks`). Confirm `HybridChunk` is re-exported from `crate::pipeline` (it is, via `hybrid_chunking::HybridChunk`).

- [ ] **Step 7: Run the tests to verify they pass**

Run: `cargo test -p oxidize-pdf --features unstable-spi --test analysis_spi_test`
Expected: PASS (all four tests: object-safety, default-strategy, parity, custom-strategy).

Run the source-stamping regression too:
Run: `cargo test -p oxidize-pdf --features "unstable-spi semantic" --test rag_chunk_metadata_test`
Expected: PASS (the `autofill_source` extraction didn't change `rag_chunks_with_source*` behavior).

- [ ] **Step 8: Commit**

```bash
git add oxidize-pdf-core/src/pipeline/spi.rs oxidize-pdf-core/src/pipeline/mod.rs oxidize-pdf-core/src/parser/document.rs oxidize-pdf-core/tests/analysis_spi_test.rs
git commit -m "feat(spi): AnalysisPipeline + rag_chunks_with_pipeline (default = rag_chunks)"
```

---

### Task 6: Decorator test, feature matrix, and gate

**Files:**
- Test: `oxidize-pdf-core/tests/analysis_spi_test.rs`

- [ ] **Step 1: Write the decorator test**

Append to `oxidize-pdf-core/tests/analysis_spi_test.rs`:

```rust
/// A strategy that delegates to the default and then merges every pair of
/// adjacent groups — proving "delegate to the default and refine".
struct PairMerger {
    inner: HybridChunker,
}

impl ChunkingStrategy for PairMerger {
    fn chunk(&self, elements: &[Element]) -> Vec<ChunkGroup> {
        let base = ChunkingStrategy::chunk(&self.inner, elements);
        let mut out = Vec::new();
        let mut iter = base.into_iter();
        while let Some(mut a) = iter.next() {
            if let Some(b) = iter.next() {
                a.elements.extend(b.elements);
            }
            out.push(a);
        }
        out
    }
}

#[test]
fn decorator_wraps_default_and_refines() {
    let elements = vec![para("alpha"), para("bravo"), para("charlie"), para("delta")];

    let inner = HybridChunker::new(HybridChunkConfig {
        max_tokens: 1, // force one element per group from the default
        overlap_tokens: 0,
        merge_adjacent: false,
        propagate_headings: false,
        merge_policy: MergePolicy::AnyInlineContent,
    });
    let base_count = ChunkingStrategy::chunk(&inner, &elements).len();
    assert_eq!(base_count, 4, "default emits one group per element here");

    let decorated = PairMerger { inner };
    let groups = decorated.chunk(&elements);
    assert_eq!(groups.len(), 2, "pairs merged: 4 groups -> 2");
    assert_eq!(groups[0].elements.len(), 2);
    assert_eq!(groups[1].elements.len(), 2);
}
```

- [ ] **Step 2: Run the full SPI test suite**

Run: `cargo test -p oxidize-pdf --features unstable-spi --test analysis_spi_test`
Expected: PASS (all tests including the decorator).

- [ ] **Step 3: Run the feature matrix**

Run each and confirm success / no warnings:

```bash
cargo build -p oxidize-pdf
cargo build -p oxidize-pdf --features unstable-spi
cargo build -p oxidize-pdf --features "unstable-spi semantic"
cargo clippy -p oxidize-pdf --features "unstable-spi semantic" --all-targets -- -D warnings
cargo test -p oxidize-pdf --lib
cargo test -p oxidize-pdf --features "unstable-spi semantic" --test analysis_spi_test
```

Expected: all succeed; clippy clean on the touched files (`spi.rs`, `hybrid_chunking.rs`, `chunk_metadata.rs`, `parser/document.rs`). Pre-existing `--all-targets` clippy debt in unrelated test files is out of scope (see project notes).

- [ ] **Step 4: Format**

Run: `cargo fmt --all && cargo fmt --all -- --check`
Expected: clean.

- [ ] **Step 5: Commit**

```bash
git add oxidize-pdf-core/tests/analysis_spi_test.rs
git commit -m "test(spi): decorator wrapping the default strategy; feature matrix verified"
```

---

## Self-Review

- **Spec coverage:**
  - §4 ChunkingStrategy + ChunkGroup → Task 1; HybridChunker default impl → Task 2; ChunkGroup→HybridChunk → Task 3.
  - §5 `extra` bag → Task 4.
  - §6 AnalysisPipeline + `rag_chunks_with_pipeline` + backward-compat + parity → Task 5 (+ DRY `autofill_source`).
  - §8 feature-gating (`unstable-spi`/`semantic`) → all tasks gate accordingly; matrix in Task 6.
  - §10 testing: parity (Task 5), custom strategy (Task 5), decorator (Task 6), `extra` round-trip + skip-empty (Task 4), oversized ownership (Task 3 + Task 5 parity), feature matrix (Task 6).
  - §7 documented-not-built seams (ElementClassifier/MetadataEnricher) → intentionally no tasks (out of v1 scope).
  - §9 bridges → intentionally no tasks (separate repos / release-wave work).
- **Type consistency:** `ChunkGroup { elements, heading_context }`, `ChunkingStrategy::chunk(&self, &[Element]) -> Vec<ChunkGroup>`, `HybridChunk::into_group`/`from_group`, `AnalysisPipeline::{new,with_chunking,with_max_tokens,with_source}`, `rag_chunks_with_pipeline(&AnalysisPipeline)`, `autofill_source(&mut DocumentSource)` — used identically across tasks.
- **No placeholders:** every code step shows complete code; commands have expected output.
