# Analysis SPI — Design Spec

**Date:** 2026-06-13
**Status:** Approved design, pending implementation plan
**Target crate:** `oxidize-pdf-core` (published as `oxidize-pdf`), MIT
**Feature gate:** `unstable-spi` (+ `semantic` for the `extra` bag)
**Version impact:** additive MINOR; no new dependencies; MSRV 1.88 preserved

## 1. Purpose

Expose a **Service Provider Interface (SPI)** in the analysis pipeline so that a
closed, proprietary crate (e.g. `oxidize-legal`) can plug in its own chunking —
and, in later phases, classification and metadata enrichment — **without
forking the core and without releasing its IP**.

The MIT core is permissive (not copyleft), so a crate that depends on
`oxidize-pdf` may already be fully proprietary. The SPI is therefore **not a
licensing mechanism** — it is the *technical* seam that lets a closed crate hook
into a pipeline whose classifier and chunker are concrete/hardcoded today
(`Partitioner`, `HybridChunker`). It also becomes a product feature in its own
right: the audience for the Tessera funnel is organizations with proprietary
domain logic; an extension surface that keeps their IP private removes a real
adoption barrier.

### Guiding rule — "the socket, not the device"

Only **anemic, generic** interfaces live in the MIT core. Domain semantics
(what a clause is, how a defined term is linked) live in the closed crate. The
core's contract types carry opaque containers (`ClassLabel(String)`,
`extra: Map<String, Value>`), never legal-domain types.

## 2. Scope

### v1 — built

- `trait ChunkingStrategy` + `struct ChunkGroup`.
- `struct AnalysisPipeline` (builder) + `PdfDocument::rag_chunks_with_pipeline`.
- `HybridChunker` becomes the default `ChunkingStrategy` impl.
- `ChunkMetadata::extra: BTreeMap<String, serde_json::Value>` (open bag, under `semantic`).
- All behind `unstable-spi`; `extra` additionally under `semantic`.
- Full backward compatibility: every existing `rag_chunks*` entry point is unchanged.

### v1 — documented, NOT built

- `trait ElementClassifier` + `ClassLabel` + `ElementMetadata.class_label`.
- `trait MetadataEnricher` + `EnrichContext`.
- The complete `AnalysisPipeline` (classifier + enrichers fields).
- Bridge story: surfacing `ChunkMetadata` (incl. `extra`) in the Python and
  .NET bridges, and the closed "pro build" that selects a strategy by name.

## 3. Architecture and boundary

Three extension points, in pipeline order:

```
fragments ──▶ [ElementClassifier] ──▶ elements
elements  ──▶ [ChunkingStrategy]   ──▶ ChunkGroup[] ──▶ RagChunk + ChunkMetadata
RagChunk  ──▶ [MetadataEnricher]   ──▶ ChunkMetadata.extra enriched
```

License boundary:

| Layer | Lives in | License | Content |
|---|---|---|---|
| The socket (traits + contract types) | `oxidize-pdf-core` | MIT | Anemic interfaces, no domain semantics |
| The default impl (`HybridChunker`, current classifier) | `oxidize-pdf-core` | MIT | The "basic plugin" — current tuned pipeline |
| The device (legal chunking, defined-terms, citations) | `oxidize-legal` (private repo) | proprietary | All valuable IP |

`oxidize-legal` depends on `oxidize-pdf` as a normal dependency (crates.io/git),
**outside this workspace** (all workspace members are MIT). It enables
`unstable-spi` + `semantic`.

## 4. ChunkingStrategy contract (v1)

```rust
/// Decides which elements group into a chunk. The pipeline owns everything
/// downstream (HybridChunk wrapping, RagChunk, chunk_id, link_chunks,
/// ChunkMetadata, extra).
pub trait ChunkingStrategy: Send + Sync {
    fn chunk(&self, elements: &[Element]) -> Vec<ChunkGroup>;
}

/// A grouping of elements destined to become one chunk.
#[non_exhaustive]
pub struct ChunkGroup {
    pub elements: Vec<Element>,
    pub heading_context: Option<String>,
}
```

- **Object-safe + `Send + Sync`** (like `OcrProvider`): no generics, no
  `Self`-returning methods → `Box<dyn ChunkingStrategy>` works; usable in
  batch/concurrent contexts. **Synchronous** (chunking is CPU-bound; no async).
- **`ChunkGroup` is the only new contract type**, anemic and `#[non_exhaustive]`.
  The strategy supplies grouping + optional `heading_context`. The pipeline
  computes `oversized` (vs the token budget), `chunk_id`, prev/next links,
  `ChunkMetadata`, and `extra`. The strategy therefore **cannot corrupt**
  ids/metadata, and `HybridChunk`'s internals stay private (a third party never
  constructs `HybridChunk`, only `ChunkGroup`).
- **`HybridChunker` becomes the default impl**: `impl ChunkingStrategy for
  HybridChunker`. Its grouping step is extracted to produce `ChunkGroup`s; its
  current public method `chunk() -> Vec<HybridChunk>` is **kept** (backward
  compat). Internally the pipeline converts `ChunkGroup → HybridChunk` via a
  `pub(crate) fn` (no new public API) and reuses `from_hybrid_chunk`.
- **Decorator-ready**: because `HybridChunker` is public and implements the
  trait, a closed strategy holds one, calls `.chunk()` for base groups, and
  refines/re-groups ("delegate to default and refine").
- **`oversized` ownership**: the pipeline computes it from the budget, not the
  strategy. A legal strategy that ignores token limits still produces valid
  groups; the pipeline flags `oversized`.

## 5. The `extra` bag on `ChunkMetadata` (v1)

```rust
// additive field on ChunkMetadata (#[non_exhaustive] already permits it)
#[cfg(feature = "semantic")]
#[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
pub extra: BTreeMap<String, serde_json::Value>,
```

- **Gated on `semantic`** (forced by the type): `serde_json::Value` only exists
  under `semantic` (`semantic = ["dep:serde_json"]`). Coherent: the bag exists
  to flow into serialized RAG output, which requires `semantic` anyway. Same cfg
  pattern as the `language` field. `oxidize-legal` builds with `semantic`.
- **`BTreeMap`, not `HashMap`**: deterministic key ordering → stable, diffable
  JSONL and reproducible tests.
- **Serializes nested** inside `metadata` as `"extra": { ... }`, with
  `skip_serializing_if = is_empty` → when no enrichment happens, the output is
  byte-identical to today (no corpus regression).
- **Who writes it in v1**: the field is `pub` and mutable (mutating fields of a
  `#[non_exhaustive]` struct is permitted). The consumer calls
  `rag_chunks_with_pipeline(...)`, gets `Vec<RagChunk>`, and **mutates
  `chunk.metadata.extra` directly** before serialization. The `MetadataEnricher`
  trait (§7) only formalizes this hook later; it is NOT needed in v1.
- **Key discipline (convention, not code)**: providers namespace their keys
  (`legal.clause_number`, `legal.defined_terms`) to avoid collisions between
  enrichers. Documented; not enforced (YAGNI).

## 6. Pipeline wiring (v1)

```rust
pub struct AnalysisPipeline {
    chunking: Box<dyn ChunkingStrategy>,   // default: HybridChunker::default()
    max_tokens: usize,                     // budget for `oversized`; default = HybridChunkConfig::default().max_tokens (512)
    source: Option<DocumentSource>,        // reuses existing source-stamping
    // documented-for-later (NOT in v1):
    // classifier: Option<Box<dyn ElementClassifier>>,
    // enrichers: Vec<Box<dyn MetadataEnricher>>,
}

impl AnalysisPipeline {
    pub fn new() -> Self;                                       // = current behavior
    pub fn with_chunking(self, s: Box<dyn ChunkingStrategy>) -> Self;
    pub fn with_max_tokens(self, n: usize) -> Self;
    pub fn with_source(self, src: DocumentSource) -> Self;
}

impl PdfDocument {
    pub fn rag_chunks_with_pipeline(&self, p: &AnalysisPipeline)
        -> ParseResult<Vec<RagChunk>>;
}
```

Flow inside `rag_chunks_with_pipeline`:

1. `let elements = self.partition()?;` (v1: default partition — the classifier seam is documented-later).
2. `let groups = p.chunking.chunk(&elements);`
3. `groups → RagChunks`: per group, wrap into `HybridChunk` (via `pub(crate) fn`),
   compute `oversized` vs `p.max_tokens`, derive `ChunkMetadata`, stamp `source`.
   Reuses `build_rag_chunks` logic generalized to `ChunkGroup`.
4. `link_chunks` (prev/next).
5. *(documented-later: run enrichers → `extra`)*.

Decisions:

- **Total backward compatibility**: `rag_chunks`, `rag_chunks_with`,
  `rag_chunks_with_source[_and_config]`, `rag_chunks_with_profile*` keep their
  signatures. They MAY delegate internally to
  `rag_chunks_with_pipeline(AnalysisPipeline::new()...)` for DRY, but this is not
  required in v1.
- **`AnalysisPipeline::new()` reproduces current behavior byte-for-byte** → the
  5-doc corpus yields exactly 1129 chunks (parity check, as validated for
  #325/#326).
- **`oversized` decoupled from the strategy** (see §4).
- **Partition/profile**: v1 uses the default partition. A profile-aware
  `AnalysisPipeline` (reading order, XYCut) is a documented follow-up, not v1.

## 7. Documented-but-not-built seams

Both `Send + Sync`, gated on `unstable-spi` (the enricher also on `semantic`).

### ElementClassifier — in `partition`, before chunking

```rust
pub trait ElementClassifier: Send + Sync {
    /// Return an open class label, or None to leave the core classification as-is.
    fn classify(&self, element: &Element, ctx: &ClassifyContext) -> Option<ClassLabel>;
}
pub struct ClassLabel(pub std::borrow::Cow<'static, str>);  // OPEN: "clause", "definition"... are strings
```

- **Open label (string), not a closed enum**: legal classes (`Clause`,
  `Definition`, `Party`) never live in MIT — the core only transports the
  string. Stored in a new `ElementMetadata.class_label: Option<String>`
  (orthogonal to the core-owned `Element` enum).
- The chunker/strategy then **reads** the label to make boundary decisions (a
  legal strategy splits on `clause`).
- **Decorator**: the legal classifier wraps the default — runs it, then refines
  (default says nothing → legal says `"clause-heading"`).

### MetadataEnricher — after metadata derivation, writes `extra`

```rust
pub trait MetadataEnricher: Send + Sync {
    fn enrich(&self, ctx: &EnrichContext, meta: &mut ChunkMetadata);
}
```

- `EnrichContext` gives read access to the chunk's text/elements/`heading_path`;
  the enricher writes `meta.extra` (e.g. `legal.clause_number`,
  `legal.defined_terms`). Formalizes the v1 "mutate `extra` directly" hook.
- Multiple enrichers run in order (`Vec<Box<dyn MetadataEnricher>>` in
  `AnalysisPipeline`). Wiring: after `link_chunks`,
  `for enricher { for chunk { enricher.enrich(ctx, &mut chunk.metadata) } }`.

### Complete AnalysisPipeline (target)

The two commented fields in §6 become active, with `with_classifier()` /
`with_enricher()` builder methods.

## 8. Feature-gating and stability posture

```toml
[features]
unstable-spi = []   # traits + AnalysisPipeline + entry point
# `extra` already lives under `semantic` (forced by serde_json::Value)
```

- Under `unstable-spi`: `ChunkingStrategy`, `ChunkGroup`, `AnalysisPipeline`,
  `rag_chunks_with_pipeline`, and `impl ChunkingStrategy for HybridChunker` are
  `pub`. Without the feature, none of it compiles → zero impact on the default
  build, MSRV (1.88, **no new deps**), or the bridges.
- The `extra` bag stays under `semantic`. `oxidize-legal` enables **both**.

**Posture: experimental-first (validate before freezing).**

- The `unstable-` prefix is a Rust convention: **unstable APIs are exempt from
  semver**, so the contract can iterate without breaking anyone on
  `develop`/releases while `oxidize-legal` validates the shape. This is what
  avoids designing the extension API in a vacuum.
- **Exit criterion** (explicit): once `oxidize-legal` has implemented its
  `ChunkingStrategy` and its output flows end-to-end through `extra` over a real
  legal corpus, the contract is considered validated → a later MINOR release
  **promotes** `unstable-spi` to a stable feature (or default).

**Semver discipline once stable:**

- `ChunkGroup` is `#[non_exhaustive]` (future fields without breaking).
- The **trait** stays minimal; future methods are added **with default impls**
  (adding a non-default method to a public trait is breaking). Documented as the
  SPI evolution rule.
- Sealed-trait pattern where applicable, if controlling what a third party may
  implement becomes necessary.

## 9. Bridges (Python + .NET)

Current state (verified): both bridges expose the RAG producer
(`rag_chunks()` in Python `parser.rs`; `PdfExtractor.RagChunksAsync` in .NET) but
their chunk models surface only the **legacy flat fields** (chunk_index, text,
full_text, page_numbers, element_types, heading_context, token_estimate,
is_oversized). **Neither exposes `RagChunk.metadata`** — the rich
`ChunkMetadata` (and therefore `extra`) is invisible to Python and .NET today.

Impact and plan (same release wave as the SPI + metadata enrichment):

- **Surface `ChunkMetadata` in both bridges**: Python adds a `metadata`
  accessor on `PyRagChunk` (a `PyChunkMetadata` class or a dict); .NET adds
  `Metadata` to `RagChunk.cs` + the native FFI DTO. The `extra` bag maps to a
  Python `dict` / .NET `Dictionary<string, object>` (or `JsonElement`). This is
  the prerequisite for any enrichment to reach bridge consumers.
- **The SPI is Rust-native, compile-time.** A Python/C# class cannot implement a
  Rust trait across FFI. Cross-FFI strategy *implementation* (callbacks +
  marshaling every `Element` across the boundary) is **out of scope** — costly,
  loses zero-copy, and unnecessary (legal logic is Rust).
- **Delivering legal chunking to Python/.NET**: a closed **"pro build"** of the
  bridge statically links `oxidize-legal` and selects the strategy **by name**
  (`doc.rag_chunks(strategy="legal")`). Cheap FFI (one string), zero IP leak (IP
  compiled into the binary), no element marshaling.

> The exact .NET binding mechanism (C-ABI `native/` cdylib + C# P/Invoke) is
> confirmed; the precise native RAG DTO mapping must be re-read before
> implementing the .NET metadata surface.

## 10. Testing and validation (v1)

All content-verifying (no smoke tests).

1. **Corpus parity (regression guard)**: `AnalysisPipeline::new()` must produce
   byte-identical chunks to current `rag_chunks()` (rag_realworld baseline vs SPI
   path → same 1129 chunks, same content). Proves the
   `HybridChunker → ChunkGroup → RagChunk` refactor doesn't regress.
2. **Custom strategy (unit)**: a toy `ChunkingStrategy` in the test (e.g. "one
   chunk per element" or "split on a marker") asserting the pipeline produces the
   expected grouping **with chunk_id/links/metadata derived by the pipeline** —
   proving the strategy only controls boundaries.
3. **Decorator**: a strategy wrapping `HybridChunker::default()`, calling it and
   post-processing (e.g. merging two adjacent groups); assert the result differs
   from default exactly there.
4. **`extra` round-trip (semantic)**: mutate `metadata.extra` with namespaced
   keys, serialize, assert `"extra"` appears nested, survives a deterministic
   round-trip (BTreeMap order), and that empty `extra` is omitted.
5. **`oversized` ownership**: a strategy emitting an over-budget group → the
   pipeline marks `is_oversized` regardless.
6. **Feature matrix in CI**: build/test with `unstable-spi`, `unstable-spi
   semantic`, and neither → no cfg breakage across combos. Default build
   unchanged.

Documented-later seams carry their test plans (classifier-decorator,
multi-enricher ordering) for when they land; 0 tests in v1.

Bridge tests live in the bridge repos (assert `chunk.metadata` surfaces the rich
fields incl. `extra` as a dict/Dictionary); referenced here, not built here.

## 11. Out of scope (explicit)

- Cross-FFI strategy implementation in Python/C#.
- Dynamic runtime plugin loading (`.so`/`.dll`): Rust has no stable ABI; the
  consumer profile (developers who compile their app) does not need it.
- Monetization/licensing of `oxidize-legal` itself (a separate product decision;
  the revenue is captured downstream in the sellable product/Tessera).
- Profile-aware `AnalysisPipeline` (reading order/XYCut) — documented follow-up.
