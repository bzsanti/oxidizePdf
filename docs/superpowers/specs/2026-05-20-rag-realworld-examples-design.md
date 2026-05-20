# Real-World RAG Examples (Rust, Python, .NET) — Design Spec

**Date:** 2026-05-20
**Status:** Approved (pending user review of this file)
**Author:** Santi + Claude (Opus 4.7)
**Related:** Causa 2 postmortem — RAG-first positioning ([project_session_2026_05_20.md](../../../../.claude-belowzero/projects/-home-santi-repos-BelowZero-oxidizePdf-oxidize-pdf/memory/project_session_2026_05_20.md))

## Goal

Ship three runnable, public examples — one in each language binding — that demonstrate **oxidize-pdf ingesting real-world government and academic PDFs into RAG-ready chunks**. Same architecture across Rust, Python, and .NET; same corpus; same JSONL output shape.

Primary use: **showcase** in the repos (Rust: `oxidize-pdf-core/examples/`, Python: `oxidize-python/examples/`, .NET: `oxidize-dotnet/examples/RealWorldRag/`).
Secondary use: code material for blog posts, landing page, and Discussions threads.

Out of scope: vector store integration, embedding generation, retrieval. The examples stop at "JSONL ready to feed any vector store."

## Corpus (5 documents, hardcoded in each example)

| # | Slug | Document | URL | Country | Language | Approx size |
|---|------|----------|-----|---------|----------|-------------|
| 1 | `ens` | Real Decreto 311/2022 (Esquema Nacional de Seguridad) | `https://www.boe.es/boe/dias/2022/05/04/pdfs/BOE-A-2022-7191.pdf` | ES | es | ~1 MB |
| 2 | `boe-sumario` | BOE sumario diario (fecha fija, a verificar en implementación) | `https://www.boe.es/boe/dias/2025/XX/XX/pdfs/BOE-S-2025-XXX.pdf` | ES | es | ~5 MB |
| 3 | `higgs` | ATLAS Collaboration — Higgs boson observation (Phys. Lett. B 716, 2012) | `https://arxiv.org/pdf/1207.7214` | CERN | en | ~1.5 MB |
| 4 | `bsi-tr-02102` | BSI Technische Richtlinie TR-02102-1 — Cryptographic Mechanisms (German version) | `https://www.bsi.bund.de/SharedDocs/Downloads/DE/BSI/Publications/TechGuidelines/TG02102/BSI-TR-02102.pdf?__blob=publicationFile` (canonical URL to confirm during implementation) | DE | de | ~1 MB |
| 5 | `ncsc-caf` | NCSC Cyber Assessment Framework v3.2 | `https://www.ncsc.gov.uk/files/NCSC_CAF_v3.2.pdf` (URL to confirm during implementation) | UK | en | ~1 MB |

**URL verification gate**: before implementing each example, fetch each URL with `HEAD` and confirm 200 + content-type PDF. If a URL has moved, find the canonical replacement before writing code.

**Why this corpus**:
- Three national cybersecurity baselines (ENS / BSI / NCSC) form a coherent compliance/regulatory thread — common RAG use case.
- Higgs paper diversifies away from CS literature (proves oxidize-pdf works on PDFs outside its own industry).
- BOE sumario diario adds heterogeneous noise (multiple unrelated articles in one PDF) — stresses the partitioner.
- Three of five PDFs are non-English (es, es, de) — reinforces the multilingual story; aligns with CFF subsetting / cmap Format 12 work landed in v2.4.

## Architecture (identical across the three languages)

```
example entry-point
   │
   ▼
CORPUS constant (array of {slug, name, url, country, language})
   │
   ▼
for each document in CORPUS:
   ├─ cache_path = ./corpus_cache/<sha1(url)[:16]>.pdf
   ├─ if not exists(cache_path): download(url, timeout=30s) → cache_path
   ├─ open PDF via the language's binding
   ├─ chunks = doc.rag_chunks()      # default HybridChunkConfig
   ├─ assert len(chunks) >= 1 && any chunk has non-empty text
   ├─ for each chunk: write JSONL line to ./out/<slug>.jsonl
   └─ print stats line to stderr
   │
   ▼
summary line + exit code (0 if all ok; = N if N documents failed; 2 if fatal)
```

### What is shared conceptually (NOT physically)

The three examples share:
- Same five URLs hardcoded
- Same JSONL field shape and ordering
- Same stats output format
- Same exit code semantics

They do NOT share:
- A common file, submodule, or generator. Each repo's example is self-contained source code that someone can read and understand without leaving the repo.

## File layout

### Rust — `oxidize-pdf` repo
- New file: `oxidize-pdf-core/examples/rag_realworld.rs`
- Dependencies: stdlib + one HTTP client. Prefer `ureq` (sync, minimal) added as `dev-dependency` of `oxidize-pdf-core` only if not already present.
- Run: `cargo run --example rag_realworld` from `oxidize-pdf-core/`.
- One Rust test in `oxidize-pdf-core/tests/rag_realworld_jsonl_test.rs` that validates the JSONL writer against an existing small fixture PDF (no network, no real corpus).

### Python — `oxidize-python` repo
- New directory: `examples/` (does not exist today).
- New file: `examples/rag_realworld.py`
- Dependencies: `oxidize-pdf` (installed via pip), stdlib only otherwise (`urllib.request`, `hashlib`, `json`, `pathlib`, `sys`, `time`). No `requests`.
- Run: `python examples/rag_realworld.py`
- One Python test in `tests/test_rag_realworld_jsonl.py` validating JSONL writer against existing fixture.

### .NET — `oxidize-dotnet` repo
- New directory: `examples/RealWorldRag/`
- Files: `RealWorldRag.csproj` (modeled on `examples/BasicUsage/` and `examples/KernelMemory/`), `Program.cs`
- Dependencies: `OxidizePdf.NET` (PackageReference or ProjectReference per existing pattern — confirm during implementation), `System.Net.Http` (built-in).
- Run: `dotnet run --project examples/RealWorldRag`
- One xUnit test (or whatever the existing test pattern is — confirm during implementation) validating JSONL writer.

## JSONL output schema

One line per chunk, written to `./out/<slug>.jsonl`:

```json
{
  "id": "ens-0001",
  "text": "Artículo 1. Objeto y ámbito de aplicación...",
  "metadata": {
    "source_url": "https://www.boe.es/boe/dias/2022/05/04/pdfs/BOE-A-2022-7191.pdf",
    "document_name": "BOE Real Decreto 311/2022 (ENS)",
    "country": "ES",
    "language": "es",
    "page_numbers": [3, 4],
    "heading_context": "CAPÍTULO I > Artículo 1",
    "element_types": ["paragraph", "heading"],
    "token_estimate": 487,
    "is_oversized": false
  }
}
```

Field rules:
- `id`: `"<slug>-<4-digit chunk_index>"`, stable across runs against the same PDF.
- `text`: verbatim from `RagChunk.text`.
- `metadata.source_url` and `metadata.document_name` and `country` and `language`: pulled from the corpus constant for that document.
- `metadata.page_numbers`, `heading_context`, `element_types`, `token_estimate`, `is_oversized`: passed through from `RagChunk` fields.
- `heading_context` may be `null` if the chunk has no parent heading.

This shape is compatible with LangChain `JSONLoader` (jq-style `text` + `metadata` separation), LlamaIndex `JSONReader`, and Pinecone batch upsert (`id` + `text` + `metadata`).

## Stats line format

Printed to stderr (so stdout-redirected JSONL is not contaminated). Both per-document lines and the final summary go to stderr; stdout is reserved for future use if a caller wants to pipe data out (currently unused by the example):

```
[ok]   ens          → 42 chunks   ~480 tok/avg   0 oversized   12 headings   out/ens.jsonl
[ok]   boe-sumario  → 187 chunks  ~412 tok/avg   3 oversized   45 headings   out/boe-sumario.jsonl
[ok]   higgs        → 31 chunks   ~510 tok/avg   1 oversized   8 headings    out/higgs.jsonl
[ok]   bsi-tr-02102 → 95 chunks   ~488 tok/avg   2 oversized   28 headings   out/bsi-tr-02102.jsonl
[ok]   ncsc-caf     → 56 chunks   ~470 tok/avg   0 oversized   18 headings   out/ncsc-caf.jsonl

5/5 documents processed successfully · 411 total chunks · ./out/
```

On failure:
```
[fail] boe-sumario  → http error: 404 (URL stale?)
[ok]   ens          → 42 chunks   ...
...
4/5 documents processed (1 failed) · exit 1
```

## Error handling

| Condition | Behavior |
|---|---|
| HTTP failure (timeout 30s, non-2xx, DNS) | `[fail]` line, increment counter, continue |
| PDF parse error | `[fail]` line, continue |
| `rag_chunks()` returns empty or all-empty-text chunks | `[fail]` line, continue |
| JSONL write IO error | `[fail]` line, continue |
| Cannot create `corpus_cache/` or `out/` | Fatal, exit 2 |
| Panic / unhandled exception inside `rag_chunks()` | Propagate (it's a bug, not a corpus issue) |

No automatic retries. The cache means re-running picks up where it left off without re-downloading what already succeeded.

## CI integration

- **Per-language** in each repo: existing build step (`cargo build --examples`, `dotnet build`, Python package install in CI) already validates compilation of the new example. No execution in CI (would need network + 5 PDFs).
- **Offline unit test**: each repo gets one test that exercises the JSONL writer against an existing small fixture PDF. Verifies field presence (`id`, `text`, `metadata.*`), correct field types, that `text` is non-empty for at least one chunk. Content-verifying, not smoke.
- **URL watchdog**: new GitHub Action `.github/workflows/verify-corpus.yml` in **oxidize-pdf only** (not duplicated to Python/.NET repos). Runs weekly (Mondays). For each of the 5 URLs: `HEAD` request, expect 200 and content-type containing `pdf`. If any fails, opens a GitHub issue tagged `corpus-staleness`.

## Testing standards

- **No smoke tests.** The offline JSONL writer test verifies actual JSONL content: parses each emitted line, checks every required field exists, checks `text` is non-empty for ≥1 chunk, checks `metadata.source_url` matches the input.
- The end-to-end runnable example is itself a content-verifying check at runtime: if any of the 5 PDFs produces 0 valid chunks, the example exits non-zero. This is not a CI test but is the real validation against real documents.

## Delivery plan

Three sequential PRs, one per repo:

1. **PR 1 — Rust (`oxidize-pdf`)**: example + offline test + `verify-corpus.yml` workflow. Includes URL verification of all 5 documents (`HEAD` requests done before coding).
2. **PR 2 — Python (`oxidize-python`)**: example + `examples/` directory + offline test. Same corpus, no workflow.
3. **PR 3 — .NET (`oxidize-dotnet`)**: example + offline test. Same corpus, no workflow.

Each PR is independent and self-contained. Failure of one corpus URL during PR 1 is the time to find a replacement — PRs 2 and 3 inherit the validated corpus.

## Open questions / pending verifications (to resolve during implementation, not in this spec)

- Exact URL for BOE sumario diario fixed date (pick a date in 2025, confirm URL works).
- Exact canonical URL for BSI TR-02102-1 German version (BSI uses `?__blob=publicationFile` query params; need the one that survives).
- Exact URL for NCSC CAF v3.2 (NCSC has moved files in the past).
- Whether `oxidize-dotnet/examples/` uses `ProjectReference` to the local `OxidizePdf.NET` project or `PackageReference` to the NuGet package (check what `BasicUsage` does).
- Whether `oxidize-pdf-core` already has an HTTP client dev-dependency we can reuse (check `Cargo.toml` `[dev-dependencies]`).

## What this spec does NOT cover

- Choice of embedding model.
- Choice of vector store.
- Retrieval / generation step.
- Performance benchmarking of `rag_chunks()` on these documents.
- Documentation of `RagChunk` API itself (lives in oxidize-pdf rustdoc, .NET XML docs, Python docstrings).
- Blog post / landing page copy.

These belong to follow-on work that can reference this spec.
