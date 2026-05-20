# Real-World RAG Examples — Rust Implementation Plan (PR #1 of 3)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ship the Rust example (`oxidize-pdf-core/examples/rag_realworld.rs`) that ingests five real-world government and academic PDFs into RAG-ready JSONL, plus an offline JSONL-writer unit test and a weekly URL watchdog GitHub Action.

**Architecture:** Single-file example. Downloads each PDF to `./corpus_cache/` (sha-keyed) if not present, opens it with `PdfReader::open`, produces `Vec<RagChunk>` via the default `rag_chunks()`, then writes one JSONL line per chunk to `./out/<slug>.jsonl`. Per-document stats and final summary go to stderr; stdout reserved for future piping. Exit code = number of failed documents (0 ok, N if N failed, 2 if fatal). Errors are skip-and-continue. No retries. No async. Offline test exercises the JSONL serialization in isolation against synthetic `RagChunk` values — no PDF fixture required.

**Tech Stack:** Rust stable. `oxidize-pdf-core` (workspace), `ureq` 2.x (sync HTTP client, dev-dep), `serde_json` (already workspace dev-dep), `sha1` (new dev-dep, tiny). Standard library: `std::fs`, `std::io`, `std::path`, `std::process::ExitCode`.

**Related spec:** `docs/superpowers/specs/2026-05-20-rag-realworld-examples-design.md`

**Branch:** `feature/rag-realworld-rust` (create from `develop`, merge back to `develop` when complete).

---

## File structure

| Path | Action | Purpose |
|---|---|---|
| `oxidize-pdf-core/examples/rag_realworld.rs` | Create | Main example (~250 LOC). Contains `CORPUS` constant, `download`, `run_one`, `write_jsonl_line`, `main`. |
| `oxidize-pdf-core/Cargo.toml` | Modify | Add `ureq` and `sha1` to `[dev-dependencies]`; register the example with `[[example]]` block. |
| `oxidize-pdf-core/tests/rag_realworld_jsonl_test.rs` | Create | Offline test of the JSONL writer using synthetic `RagChunk` values. |
| `oxidize-pdf-core/.gitignore` | Modify (or create at repo root) | Ignore `corpus_cache/` and `out/`. |
| `.github/workflows/verify-corpus.yml` | Create | Weekly cron, HEAD-checks each of the 5 corpus URLs, opens issue on failure. |
| `docs/superpowers/plans/2026-05-20-rag-realworld-rust.md` | Already created | This file. |

**Decomposition note:** The example deliberately keeps everything in one file. It is showcase code that a new reader scans top-to-bottom. The `tests/rag_realworld_jsonl_test.rs` extracts the JSONL line-formatting function via a thin shared helper so the test can call it. The helper lives in the example file (Rust examples can be linked into integration tests with `#[path = "..."]` or by duplicating the small helper — see Task 5 for the chosen approach).

---

## Pre-implementation: URL verification

Before writing any code, verify all five corpus URLs return HTTP 200 with a PDF content-type. If any has moved, find the canonical replacement and update the spec.

- [ ] **Step 0.1: Verify corpus URLs**

```bash
for url in \
  "https://www.boe.es/boe/dias/2022/05/04/pdfs/BOE-A-2022-7191.pdf" \
  "https://arxiv.org/pdf/1207.7214" \
  "https://www.bsi.bund.de/SharedDocs/Downloads/DE/BSI/Publications/TechGuidelines/TG02102/BSI-TR-02102.pdf?__blob=publicationFile" \
  "https://www.ncsc.gov.uk/files/NCSC_CAF_v3.2.pdf" ; do
    echo "=== $url ==="
    curl -sIL -o /dev/null -w "HTTP %{http_code}  type=%{content_type}  size=%{size_download}\n" "$url"
done
```

Expected: each line shows `HTTP 200` with `content_type` containing `pdf` (or `application/octet-stream` acceptable). If any is non-200 or HTML, halt and search for the canonical replacement before continuing.

- [ ] **Step 0.2: Pick a fixed BOE sumario diario URL**

Browse `https://www.boe.es/boe/dias/2025/` and pick a sumario from a date with a reasonable amount of content (5–10 MB). Confirm with curl as above. Record the exact URL — it goes into the `CORPUS` constant in Task 2.

Suggested candidate to test first:
```
https://www.boe.es/boe/dias/2025/01/15/pdfs/BOE-S-2025-13.pdf
```
(The `-13` suffix is the day-of-year sequence number, which the BOE assigns. Adjust if the actual file uses a different sequence.)

- [ ] **Step 0.3: Update the spec if URLs changed**

If any URL in the spec needs correction, edit `docs/superpowers/specs/2026-05-20-rag-realworld-examples-design.md` with the corrected URLs and commit on the same branch (single commit "docs: lock corpus URLs after verification").

- [ ] **Step 0.4: Create the feature branch**

```bash
git checkout develop
git pull origin develop
git checkout -b feature/rag-realworld-rust
```

Expected: branch created, working tree clean.

---

## Task 1: Add dependencies and example registration

**Files:**
- Modify: `oxidize-pdf-core/Cargo.toml`

- [ ] **Step 1.1: Inspect current dev-dependencies and example block locations**

```bash
grep -n "^\[dev-dependencies\]\|^\[\[example\]\]" oxidize-pdf-core/Cargo.toml | head -5
```

Expected output shows `[dev-dependencies]` line number and the first `[[example]]` line number (so we know where to insert).

- [ ] **Step 1.2: Add `ureq` and `sha1` to `[dev-dependencies]`**

Edit `oxidize-pdf-core/Cargo.toml`. Inside the `[dev-dependencies]` block (locate via `grep -n "^\[dev-dependencies\]" oxidize-pdf-core/Cargo.toml`), append two lines after the existing entries:

```toml
ureq = { version = "2.10", default-features = false, features = ["tls"] }
sha1 = "0.10"
```

`default-features = false` + `features = ["tls"]` keeps the dependency lean (no native-tls + no JSON helpers — we only need HTTPS GET).

- [ ] **Step 1.3: Register the example**

In the same `Cargo.toml`, at the end of the existing `[[example]]` blocks (search for the last `[[example]]` block), append:

```toml
[[example]]
name = "rag_realworld"
path = "examples/rag_realworld.rs"
```

- [ ] **Step 1.4: Verify the manifest still parses**

```bash
cargo metadata --manifest-path oxidize-pdf-core/Cargo.toml --format-version 1 --no-deps > /dev/null
```

Expected: exit 0, no output.

- [ ] **Step 1.5: Verify ureq + sha1 resolve**

```bash
cargo check --manifest-path oxidize-pdf-core/Cargo.toml --examples 2>&1 | tail -20
```

Expected: `error[E0432]: unresolved import` or similar errors about the example not yet existing — that is fine. What matters is no error mentioning `ureq` or `sha1`. If you see "failed to select a version for ureq", adjust the version constraint.

- [ ] **Step 1.6: Commit dependency bump**

```bash
git add oxidize-pdf-core/Cargo.toml Cargo.lock
git commit -m "chore: add ureq + sha1 dev-deps and register rag_realworld example"
```

Expected: clean commit; pre-commit hooks pass (the example file does not exist yet, but cargo check on the library should still succeed).

---

## Task 2: Define the CORPUS constant

**Files:**
- Create: `oxidize-pdf-core/examples/rag_realworld.rs` (initial skeleton)

- [ ] **Step 2.1: Create skeleton with corpus constant**

Create `oxidize-pdf-core/examples/rag_realworld.rs` with:

```rust
//! Real-world RAG ingestion example
//!
//! Downloads five real government and academic PDFs, runs the default
//! `rag_chunks()` pipeline on each, and writes RAG-ready JSONL to `./out/`.
//!
//! Run with:
//!   cargo run --example rag_realworld
//!
//! Output:
//!   - ./corpus_cache/<sha1>.pdf  (downloaded PDFs, kept across runs)
//!   - ./out/<slug>.jsonl         (one chunk per line, RAG-ready)
//!
//! Exit code: 0 if every document succeeded; N if N documents failed;
//! 2 if a fatal error occurred (filesystem, etc.).

use std::path::PathBuf;

#[derive(Debug, Clone, Copy)]
struct CorpusEntry {
    slug: &'static str,
    name: &'static str,
    url: &'static str,
    country: &'static str,
    language: &'static str,
}

const CORPUS: &[CorpusEntry] = &[
    CorpusEntry {
        slug: "ens",
        name: "BOE Real Decreto 311/2022 (Esquema Nacional de Seguridad)",
        url: "https://www.boe.es/boe/dias/2022/05/04/pdfs/BOE-A-2022-7191.pdf",
        country: "ES",
        language: "es",
    },
    CorpusEntry {
        slug: "boe-sumario",
        name: "BOE sumario diario (2025-01-15)",
        url: "https://www.boe.es/boe/dias/2025/01/15/pdfs/BOE-S-2025-13.pdf",
        country: "ES",
        language: "es",
    },
    CorpusEntry {
        slug: "higgs",
        name: "ATLAS Collaboration — Higgs boson observation (Phys. Lett. B 716, 2012)",
        url: "https://arxiv.org/pdf/1207.7214",
        country: "CERN",
        language: "en",
    },
    CorpusEntry {
        slug: "bsi-tr-02102",
        name: "BSI TR-02102 — Kryptographische Verfahren (German master)",
        url: "https://www.bsi.bund.de/SharedDocs/Downloads/DE/BSI/Publikationen/TechnischeRichtlinien/TR02102/BSI-TR-02102.pdf?__blob=publicationFile&v=15",
        country: "DE",
        language: "de",
    },
    CorpusEntry {
        slug: "ncsc-caf",
        name: "NCSC Cyber Assessment Framework v4.0",
        url: "https://www.ncsc.gov.uk/sites/default/files/documents/NCSC-Cyber-Assessment-Framework-4.0.pdf",
        country: "UK",
        language: "en",
    },
];

const CACHE_DIR: &str = "corpus_cache";
const OUT_DIR: &str = "out";
const DOWNLOAD_TIMEOUT_SECS: u64 = 30;

fn cache_path(url: &str) -> PathBuf {
    use sha1::{Digest, Sha1};
    let mut h = Sha1::new();
    h.update(url.as_bytes());
    let digest = h.finalize();
    let hex: String = digest.iter().take(8).map(|b| format!("{:02x}", b)).collect();
    PathBuf::from(CACHE_DIR).join(format!("{}.pdf", hex))
}

fn main() {
    eprintln!("rag_realworld: corpus has {} documents", CORPUS.len());
    for entry in CORPUS {
        eprintln!("  - {} ({}) → {}", entry.slug, entry.country, entry.url);
    }
}
```

- [ ] **Step 2.2: Verify the skeleton compiles**

```bash
cargo build --manifest-path oxidize-pdf-core/Cargo.toml --example rag_realworld 2>&1 | tail -10
```

Expected: build succeeds (warnings about unused functions are OK at this stage).

- [ ] **Step 2.3: Run the skeleton**

```bash
cargo run --manifest-path oxidize-pdf-core/Cargo.toml --example rag_realworld
```

Expected output (stderr):
```
rag_realworld: corpus has 5 documents
  - ens (ES) → https://www.boe.es/...
  - boe-sumario (ES) → https://www.boe.es/...
  - higgs (CERN) → https://arxiv.org/pdf/1207.7214
  - bsi-tr-02102 (DE) → https://www.bsi.bund.de/...
  - ncsc-caf (UK) → https://www.ncsc.gov.uk/...
```

- [ ] **Step 2.4: Commit the skeleton**

```bash
git add oxidize-pdf-core/examples/rag_realworld.rs
git commit -m "feat(example): skeleton for rag_realworld with corpus constant"
```

---

## Task 3: Implement download with cache

**Files:**
- Modify: `oxidize-pdf-core/examples/rag_realworld.rs`

- [ ] **Step 3.1: Add `ensure_local_copy` function above `main`**

In `oxidize-pdf-core/examples/rag_realworld.rs`, add the following functions before `fn main`:

```rust
use std::fs;
use std::io::Write;
use std::time::Duration;

#[derive(Debug)]
enum FetchError {
    Http(String),
    Io(std::io::Error),
}

impl std::fmt::Display for FetchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FetchError::Http(msg) => write!(f, "http error: {}", msg),
            FetchError::Io(e) => write!(f, "io error: {}", e),
        }
    }
}

impl From<std::io::Error> for FetchError {
    fn from(e: std::io::Error) -> Self { FetchError::Io(e) }
}

/// Returns Ok(path) on success. If the file is already cached, no HTTP request
/// is made. Network errors and non-2xx responses become `FetchError::Http`.
fn ensure_local_copy(entry: &CorpusEntry) -> Result<PathBuf, FetchError> {
    fs::create_dir_all(CACHE_DIR)?;
    let path = cache_path(entry.url);
    if path.exists() {
        return Ok(path);
    }
    let agent = ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(10))
        .timeout_read(Duration::from_secs(DOWNLOAD_TIMEOUT_SECS))
        .build();
    let resp = agent
        .get(entry.url)
        .call()
        .map_err(|e| FetchError::Http(e.to_string()))?;
    let status = resp.status();
    if !(200..300).contains(&status) {
        return Err(FetchError::Http(format!("status {}", status)));
    }
    let mut reader = resp.into_reader();
    let tmp = path.with_extension("pdf.partial");
    {
        let mut out = fs::File::create(&tmp)?;
        std::io::copy(&mut reader, &mut out)?;
        out.flush()?;
    }
    fs::rename(&tmp, &path)?;
    Ok(path)
}
```

- [ ] **Step 3.2: Wire the download into `main` for one document**

Replace the contents of `fn main` (still without parsing yet) with:

```rust
fn main() {
    let entry = &CORPUS[0]; // ENS, smallest known-good URL
    match ensure_local_copy(entry) {
        Ok(path) => {
            let size = fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
            eprintln!("[ok]   {} cached at {} ({} bytes)", entry.slug, path.display(), size);
        }
        Err(e) => {
            eprintln!("[fail] {} → {}", entry.slug, e);
            std::process::exit(1);
        }
    }
}
```

- [ ] **Step 3.3: Run end-to-end against one URL**

```bash
rm -rf oxidize-pdf-core/corpus_cache
cargo run --manifest-path oxidize-pdf-core/Cargo.toml --example rag_realworld 2>&1
```

Expected: stderr shows `[ok] ens cached at corpus_cache/<hex>.pdf (NNNNNN bytes)` with NNNNNN > 100000.

- [ ] **Step 3.4: Re-run and confirm cache hit**

```bash
cargo run --manifest-path oxidize-pdf-core/Cargo.toml --example rag_realworld 2>&1
```

Expected: same output, but takes <100ms (no download). Confirm by checking elapsed time is much shorter than first run.

- [ ] **Step 3.5: Test failure path**

Temporarily change the URL in `CORPUS[0]` to `https://www.boe.es/this-does-not-exist.pdf`, rerun:

```bash
rm -rf oxidize-pdf-core/corpus_cache
cargo run --manifest-path oxidize-pdf-core/Cargo.toml --example rag_realworld 2>&1
echo "exit=$?"
```

Expected: stderr shows `[fail] ens → http error: status 404` (or similar), exit code = 1.

Restore the correct URL before committing.

- [ ] **Step 3.6: Commit**

```bash
git add oxidize-pdf-core/examples/rag_realworld.rs
git commit -m "feat(example): cache-aware download for rag_realworld corpus"
```

---

## Task 4: Implement chunking + stats per document

**Files:**
- Modify: `oxidize-pdf-core/examples/rag_realworld.rs`

- [ ] **Step 4.1: Add the run_one function and DocStats type**

Add to `oxidize-pdf-core/examples/rag_realworld.rs` above `fn main`:

```rust
use oxidize_pdf::parser::{PdfDocument, PdfReader};
use oxidize_pdf::pipeline::RagChunk;

#[derive(Debug)]
enum RunError {
    Fetch(FetchError),
    Parse(String),
    Empty,
}

impl std::fmt::Display for RunError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RunError::Fetch(e) => write!(f, "{}", e),
            RunError::Parse(s) => write!(f, "parse error: {}", s),
            RunError::Empty => write!(f, "produced 0 valid chunks"),
        }
    }
}

impl From<FetchError> for RunError {
    fn from(e: FetchError) -> Self { RunError::Fetch(e) }
}

struct DocStats {
    chunks: usize,
    avg_tokens: usize,
    oversized: usize,
    headings: usize,
}

fn run_one(entry: &CorpusEntry) -> Result<(Vec<RagChunk>, DocStats), RunError> {
    let path = ensure_local_copy(entry)?;
    let reader = PdfReader::open(&path).map_err(|e| RunError::Parse(e.to_string()))?;
    let doc = PdfDocument::new(reader);
    let chunks = doc.rag_chunks().map_err(|e| RunError::Parse(e.to_string()))?;
    let non_empty: Vec<RagChunk> = chunks
        .into_iter()
        .filter(|c| !c.text.trim().is_empty())
        .collect();
    if non_empty.is_empty() {
        return Err(RunError::Empty);
    }
    let total_tokens: usize = non_empty.iter().map(|c| c.token_estimate).sum();
    let stats = DocStats {
        chunks: non_empty.len(),
        avg_tokens: total_tokens / non_empty.len(),
        oversized: non_empty.iter().filter(|c| c.is_oversized).count(),
        headings: non_empty.iter().filter(|c| c.heading_context.is_some()).count(),
    };
    Ok((non_empty, stats))
}
```

- [ ] **Step 4.2: Replace `main` with full corpus loop (stats only, no JSONL yet)**

Replace `fn main`:

```rust
fn main() -> std::process::ExitCode {
    let mut failed = 0usize;
    let mut total_chunks = 0usize;

    for entry in CORPUS {
        match run_one(entry) {
            Ok((chunks, stats)) => {
                total_chunks += stats.chunks;
                eprintln!(
                    "[ok]   {:<13} → {} chunks   ~{} tok/avg   {} oversized   {} headings",
                    entry.slug, stats.chunks, stats.avg_tokens, stats.oversized, stats.headings
                );
                let _ = chunks; // JSONL output added in Task 5
            }
            Err(e) => {
                failed += 1;
                eprintln!("[fail] {:<13} → {}", entry.slug, e);
            }
        }
    }

    let ok = CORPUS.len() - failed;
    if failed == 0 {
        eprintln!(
            "\n{}/{} documents processed successfully · {} total chunks",
            ok, CORPUS.len(), total_chunks
        );
        std::process::ExitCode::SUCCESS
    } else {
        eprintln!(
            "\n{}/{} documents processed ({} failed) · exit {}",
            ok, CORPUS.len(), failed, failed
        );
        std::process::ExitCode::from(failed.min(255) as u8)
    }
}
```

- [ ] **Step 4.3: Run the full corpus**

```bash
cargo run --manifest-path oxidize-pdf-core/Cargo.toml --example rag_realworld 2>&1
echo "exit=$?"
```

Expected (verified on 2026-05-20):
- ENS, BOE sumario, BSI, NCSC all parse OK.
- The Higgs paper (arXiv:1207.7214) fails with `parse error: Syntax error at position 169585: Unknown keyword: Sendstream`. This is a known oxidize-pdf parser limitation tracked in **issue #260** (parser does not tolerate /Length mismatch in TeX-generated stream objects).
- Result: 4/5 `[ok]` lines, 1 `[fail]` line for higgs, summary `4/5 documents processed (1 failed)`, exit=1.
- This is the intended demonstration of the example's skip-and-continue + exit-code-on-failure behavior. Until #260 is fixed, ship 4/5.
- Chunk counts vary widely (e.g. ENS may produce thousands of small chunks at ~4 tok/avg, BSI gives ~1700 chunks at ~27 tok/avg with hundreds of headings). This reflects real-world variation in PDF structure and is not a defect for this example.

- [ ] **Step 4.4: Commit**

```bash
git add oxidize-pdf-core/examples/rag_realworld.rs
git commit -m "feat(example): run rag_chunks() over corpus and print stats"
```

---

## Task 5: Implement JSONL output

**Files:**
- Modify: `oxidize-pdf-core/examples/rag_realworld.rs`

- [ ] **Step 5.1: Add the JSONL writer function**

Add to `oxidize-pdf-core/examples/rag_realworld.rs` above `fn main`:

```rust
use serde_json::json;
use std::io::BufWriter;

/// Serialize a single chunk to the canonical JSONL line shape.
/// Pub so the integration test can call it; lives in the example for showcase clarity.
pub fn jsonl_line(entry_slug: &str, entry_name: &str, entry_country: &str, entry_language: &str,
                 entry_url: &str, chunk: &RagChunk) -> String {
    let value = json!({
        "id": format!("{}-{:04}", entry_slug, chunk.chunk_index),
        "text": chunk.text,
        "metadata": {
            "source_url": entry_url,
            "document_name": entry_name,
            "country": entry_country,
            "language": entry_language,
            "page_numbers": chunk.page_numbers,
            "heading_context": chunk.heading_context,
            "element_types": chunk.element_types,
            "token_estimate": chunk.token_estimate,
            "is_oversized": chunk.is_oversized,
        }
    });
    value.to_string()
}

fn write_jsonl(entry: &CorpusEntry, chunks: &[RagChunk]) -> std::io::Result<PathBuf> {
    fs::create_dir_all(OUT_DIR)?;
    let path = PathBuf::from(OUT_DIR).join(format!("{}.jsonl", entry.slug));
    let file = fs::File::create(&path)?;
    let mut w = BufWriter::new(file);
    for chunk in chunks {
        let line = jsonl_line(entry.slug, entry.name, entry.country, entry.language, entry.url, chunk);
        writeln!(w, "{}", line)?;
    }
    w.flush()?;
    Ok(path)
}
```

- [ ] **Step 5.2: Wire `write_jsonl` into the main loop**

In `fn main`, replace the line `let _ = chunks; // JSONL output added in Task 5` with:

```rust
match write_jsonl(entry, &chunks) {
    Ok(out_path) => {
        eprintln!(
            "[ok]   {:<13} → {} chunks   ~{} tok/avg   {} oversized   {} headings   {}",
            entry.slug, stats.chunks, stats.avg_tokens, stats.oversized, stats.headings,
            out_path.display()
        );
    }
    Err(e) => {
        failed += 1;
        eprintln!("[fail] {:<13} → io error writing jsonl: {}", entry.slug, e);
        continue;
    }
}
```

Remove the now-duplicate `eprintln!("[ok]   {:<13} ...")` that printed earlier in the `Ok` arm — there should only be one `[ok]` log per document.

- [ ] **Step 5.3: Run the full pipeline**

```bash
rm -rf oxidize-pdf-core/out
cargo run --manifest-path oxidize-pdf-core/Cargo.toml --example rag_realworld 2>&1
echo "exit=$?"
ls -la oxidize-pdf-core/out/
```

Expected: five `.jsonl` files in `oxidize-pdf-core/out/`, each non-empty.

- [ ] **Step 5.4: Verify JSONL shape on one document**

```bash
head -1 oxidize-pdf-core/out/ens.jsonl | python3 -m json.tool
wc -l oxidize-pdf-core/out/ens.jsonl
```

Expected:
- `json.tool` prints a well-formed JSON object with keys `id`, `text`, `metadata`.
- `metadata` has the eight expected fields.
- `wc -l` matches the chunk count from the stats line.

- [ ] **Step 5.5: Verify all five JSONL files**

```bash
for f in oxidize-pdf-core/out/*.jsonl; do
  echo "=== $f ==="
  echo "lines: $(wc -l < "$f")"
  head -1 "$f" | python3 -c 'import json,sys; d=json.loads(sys.stdin.read()); print("id:", d["id"]); print("text first 80:", d["text"][:80]); print("metadata keys:", sorted(d["metadata"].keys()))'
done
```

Expected: each file's first line parses, `id` starts with the slug, `text` is non-empty, metadata has all required keys.

- [ ] **Step 5.6: Commit**

```bash
git add oxidize-pdf-core/examples/rag_realworld.rs
git commit -m "feat(example): write RAG-ready JSONL per document"
```

---

## Task 6: Offline JSONL writer test

**Files:**
- Create: `oxidize-pdf-core/tests/rag_realworld_jsonl_test.rs`

This test exercises `jsonl_line` against synthetic `RagChunk` values, isolating the JSONL serialization from the network and the PDF parser. No PDF fixture used.

- [ ] **Step 6.1: Create the test file**

Create `oxidize-pdf-core/tests/rag_realworld_jsonl_test.rs`:

```rust
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
        full_text: "CAPÍTULO I > Artículo 1\n\nArtículo 1. Objeto y ámbito de aplicación.".to_string(),
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
    assert_eq!(m["element_types"], serde_json::json!(["heading", "paragraph"]));
    assert_eq!(m["token_estimate"], 487);
    assert_eq!(m["is_oversized"], false);
}

#[test]
fn jsonl_line_without_heading_serializes_null() {
    let chunk = sample_chunk_without_heading();
    let line = example::jsonl_line(
        "higgs", "ATLAS Higgs paper", "CERN", "en",
        "https://arxiv.org/pdf/1207.7214", &chunk
    );
    let v: Value = serde_json::from_str(&line).unwrap();
    assert_eq!(v["id"], "higgs-0000");
    assert!(v["metadata"]["heading_context"].is_null(),
        "heading_context must be JSON null when no parent heading, got {:?}",
        v["metadata"]["heading_context"]);
}

#[test]
fn jsonl_line_oversized_preserves_flag_and_pages() {
    let chunk = sample_oversized_chunk();
    let line = example::jsonl_line(
        "bsi-tr-02102", "BSI TR-02102-1", "DE", "de",
        "https://www.bsi.bund.de/example.pdf", &chunk
    );
    let v: Value = serde_json::from_str(&line).unwrap();
    assert_eq!(v["id"], "bsi-tr-02102-0099", "chunk_index 99 must zero-pad to 0099");
    assert_eq!(v["metadata"]["is_oversized"], true);
    assert_eq!(v["metadata"]["page_numbers"], serde_json::json!([10, 11, 12]));
    assert_eq!(v["metadata"]["token_estimate"], 1024);
}

#[test]
fn jsonl_line_is_single_line_with_no_internal_newlines() {
    let chunk = sample_chunk_with_heading();
    let line = example::jsonl_line(
        "ens", "doc", "ES", "es", "https://example.com", &chunk
    );
    assert!(!line.contains('\n'),
        "JSONL line must not contain a literal newline (serde_json default escapes them)");
    // Field with multiline content should be escaped, not raw-broken
    let chunk_multiline = RagChunk {
        text: "Line one.\nLine two.".to_string(),
        ..chunk
    };
    let line2 = example::jsonl_line("x","y","X","x","u",&chunk_multiline);
    assert!(!line2.contains('\n'), "embedded newlines must be escaped, not literal");
    // Confirm it round-trips
    let v: Value = serde_json::from_str(&line2).unwrap();
    assert_eq!(v["text"], "Line one.\nLine two.");
}
```

- [ ] **Step 6.2: Run the new test**

```bash
cargo test --manifest-path oxidize-pdf-core/Cargo.toml --test rag_realworld_jsonl_test 2>&1 | tail -20
```

Expected: 4 tests pass, 0 failed.

If the test file fails to compile because `example::jsonl_line` is not visible, the `#[path = "..."]` module include is wrong — confirm the path is relative to `oxidize-pdf-core/tests/` and that `jsonl_line` is `pub` in the example file.

- [ ] **Step 6.3: Commit**

```bash
git add oxidize-pdf-core/tests/rag_realworld_jsonl_test.rs
git commit -m "test: offline JSONL writer test for rag_realworld example"
```

---

## Task 7: Gitignore corpus_cache/ and out/

**Files:**
- Modify: `.gitignore` (root)

- [ ] **Step 7.1: Inspect existing .gitignore**

```bash
test -f .gitignore && cat .gitignore || echo "no root .gitignore"
```

If a root `.gitignore` exists, append. If not, create one.

- [ ] **Step 7.2: Append the new ignores**

Append the following lines to the root `.gitignore` (create the file if absent):

```
# rag_realworld example artifacts
corpus_cache/
out/
oxidize-pdf-core/corpus_cache/
oxidize-pdf-core/out/
```

The duplication covers the case where the example is run from the workspace root or from `oxidize-pdf-core/`.

- [ ] **Step 7.3: Verify git ignores them**

```bash
git status --ignored | grep -E "corpus_cache|out/" || echo "not seeing ignored entries; check .gitignore syntax"
```

Expected: either git shows them as `Ignored files:` or the directories don't exist yet (also fine).

- [ ] **Step 7.4: Commit**

```bash
git add .gitignore
git commit -m "chore: gitignore corpus_cache/ and out/ from rag_realworld example"
```

---

## Task 8: GitHub Action — verify-corpus.yml (weekly URL watchdog)

**Files:**
- Create: `.github/workflows/verify-corpus.yml`

- [ ] **Step 8.1: Confirm GH issue label exists or will be created on first run**

```bash
gh label list --search corpus-staleness 2>&1 | head
```

If the label is missing, the action's `gh issue create --label corpus-staleness` will fail until the label is created. Create it once:

```bash
gh label create corpus-staleness --description "Corpus URL no longer reachable" --color FBCA04
```

(If `gh` is not authenticated, fix that first with `gh auth login`. Do NOT skip this step.)

- [ ] **Step 8.2: Create the workflow file**

Create `.github/workflows/verify-corpus.yml`:

```yaml
name: verify-corpus

on:
  schedule:
    # Weekly on Monday 06:00 UTC
    - cron: '0 6 * * 1'
  workflow_dispatch:

permissions:
  contents: read
  issues: write

jobs:
  check-urls:
    runs-on: ubuntu-latest
    steps:
      - name: HEAD-check each corpus URL
        id: check
        shell: bash
        run: |
          set +e
          urls=(
            "https://www.boe.es/boe/dias/2022/05/04/pdfs/BOE-A-2022-7191.pdf|ens"
            "https://www.boe.es/boe/dias/2025/01/15/pdfs/BOE-S-2025-13.pdf|boe-sumario"
            "https://arxiv.org/pdf/1207.7214|higgs"
            "https://www.bsi.bund.de/SharedDocs/Downloads/DE/BSI/Publikationen/TechnischeRichtlinien/TR02102/BSI-TR-02102.pdf?__blob=publicationFile&v=15|bsi-tr-02102"
            "https://www.ncsc.gov.uk/sites/default/files/documents/NCSC-Cyber-Assessment-Framework-4.0.pdf|ncsc-caf"
          )
          fails=""
          for entry in "${urls[@]}"; do
            url="${entry%|*}"
            slug="${entry#*|}"
            # Use GET with -o /dev/null because some servers reject HEAD.
            code=$(curl -sSL -o /dev/null -w "%{http_code}" --max-time 30 "$url" || echo "000")
            echo "$slug → $url → HTTP $code"
            if [ "$code" -lt 200 ] || [ "$code" -ge 300 ]; then
              fails="${fails}- ${slug}: HTTP ${code} ${url}%0A"
            fi
          done
          if [ -n "$fails" ]; then
            echo "fails=${fails}" >> "$GITHUB_OUTPUT"
            exit 1
          fi
      - name: Open issue on failure
        if: failure() && steps.check.outputs.fails != ''
        env:
          GH_TOKEN: ${{ secrets.GITHUB_TOKEN }}
          FAILS: ${{ steps.check.outputs.fails }}
        run: |
          # Avoid opening duplicate issues if one is already open with this label.
          existing=$(gh issue list --label corpus-staleness --state open --json number --jq '.[].number' | head -n1)
          body="Weekly verify-corpus workflow detected unreachable URLs:%0A%0A${FAILS}%0AUpdate \`oxidize-pdf-core/examples/rag_realworld.rs\` (the CORPUS constant) and the workflow file with corrected URLs."
          if [ -n "$existing" ]; then
            gh issue comment "$existing" --body "$(printf '%b' "$body")"
          else
            gh issue create --title "[corpus] Some RAG-example URLs are unreachable" \
                            --label corpus-staleness \
                            --body "$(printf '%b' "$body")"
          fi
```

- [ ] **Step 8.3: Lint the workflow locally**

```bash
yq '.' .github/workflows/verify-corpus.yml > /dev/null && echo "YAML ok" || echo "YAML broken"
```

If `yq` is not available, fall back to:
```bash
python3 -c "import yaml,sys; yaml.safe_load(open('.github/workflows/verify-corpus.yml')); print('ok')"
```

Expected: "ok" / "YAML ok".

- [ ] **Step 8.4: Smoke-run via workflow_dispatch (post-merge only)**

The workflow includes `workflow_dispatch`, but it can only be triggered from the default branch after merge. Note this for the PR description as a post-merge verification step. Do not block the PR on it.

- [ ] **Step 8.5: Commit the workflow**

```bash
git add .github/workflows/verify-corpus.yml
git commit -m "ci: add verify-corpus weekly URL watchdog for RAG-example corpus"
```

---

## Task 9: Final integration check and PR

- [ ] **Step 9.1: Clean state run**

```bash
rm -rf oxidize-pdf-core/corpus_cache oxidize-pdf-core/out
cargo build --manifest-path oxidize-pdf-core/Cargo.toml --example rag_realworld 2>&1 | tail -5
cargo run --manifest-path oxidize-pdf-core/Cargo.toml --example rag_realworld 2>&1
echo "exit=$?"
```

Expected:
- Build succeeds, no warnings about the example file (run `cargo clippy --example rag_realworld` if you want to be thorough).
- All five documents process with `[ok]` lines.
- Five JSONL files written to `oxidize-pdf-core/out/`.
- `exit=0`.

- [ ] **Step 9.2: Run all unit tests**

```bash
cargo test --manifest-path oxidize-pdf-core/Cargo.toml --test rag_realworld_jsonl_test 2>&1 | tail -10
cargo test --manifest-path oxidize-pdf-core/Cargo.toml --lib 2>&1 | tail -5
```

Expected: both green. The lib tests are unchanged (no regression introduced).

- [ ] **Step 9.3: Run clippy on the example**

```bash
cargo clippy --manifest-path oxidize-pdf-core/Cargo.toml --example rag_realworld -- -D warnings 2>&1 | tail -20
```

Expected: no warnings. Fix any clippy issues inline (most likely: unused `use`, unnecessary clone).

- [ ] **Step 9.4: Run the full pre-commit check the repo enforces**

```bash
git log --oneline develop..HEAD
```

Confirm the branch has exactly the commits planned and no junk.

- [ ] **Step 9.5: Verify all artifacts are gitignored**

```bash
git status --short
```

Expected: clean. If `corpus_cache/` or `out/` appears, the `.gitignore` is wrong.

- [ ] **Step 9.6: Push and open PR (only after explicit user authorization)**

Per `CLAUDE.local.md`: do not push or open PRs without user authorization. Once authorized:

```bash
git push -u origin feature/rag-realworld-rust
gh pr create --base develop \
  --title "feat(example): real-world RAG ingestion across 5 government and academic PDFs" \
  --body "$(cat <<'EOF'
## Summary
- New `rag_realworld` example downloads and chunks 5 real PDFs (ENS, BOE sumario, ATLAS Higgs paper, BSI TR-02102-1, NCSC CAF) into RAG-ready JSONL.
- Offline unit test for the JSONL writer (`tests/rag_realworld_jsonl_test.rs`).
- Weekly GitHub Action `verify-corpus.yml` monitors URL health and opens a tagged issue on failure.

## Why
First concrete showcase aligned with the RAG-first positioning (Causa 2 postmortem). Demonstrates oxidize-pdf ingesting real-world multilingual gov/academic content end-to-end, producing JSONL compatible with LangChain `JSONLoader`, LlamaIndex `JSONReader`, and Pinecone batch upsert.

## Spec
See `docs/superpowers/specs/2026-05-20-rag-realworld-examples-design.md`.

## Test plan
- [ ] `cargo run --example rag_realworld` produces 5 JSONL files in `out/` (network required first run; uses `corpus_cache/` thereafter).
- [ ] `cargo test --test rag_realworld_jsonl_test` passes (4 tests, offline).
- [ ] `cargo clippy --example rag_realworld -- -D warnings` clean.
- [ ] Post-merge: trigger `verify-corpus` workflow manually from the Actions tab and confirm all 5 URLs return 200.

## Follow-up
PRs in `oxidize-python` and `oxidize-dotnet` will mirror this example using the same corpus.
EOF
)"
```

---

## Self-review checklist (run after writing the plan, before handoff)

- [x] **Spec coverage:** Every spec section maps to at least one task.
  - Corpus list → Task 2.
  - Architecture flow → Tasks 3, 4, 5.
  - File layout (Rust portion) → Tasks 1, 2, 6.
  - JSONL schema → Task 5 + Task 6 test assertions.
  - Stats line → Tasks 4, 5.
  - Error handling table → Tasks 3, 4, 5.
  - CI integration → Tasks 8 (watchdog), 6 (offline test).
  - Testing standards (no smoke) → Task 6 test asserts every field, no return-code-only checks.
  - Delivery plan PR 1 → Task 9.
  - URL verification gate → Step 0.1.
- [x] **No placeholders.** Every step has either commands or full code.
- [x] **Type consistency.** `CorpusEntry`, `RagChunk`, `DocStats`, `RunError`, `FetchError` are defined exactly once and referenced consistently. `jsonl_line` is `pub` in the example and imported by the test via `#[path]` include.
- [x] **Frequent commits.** Nine commits planned (one per task, except dependency bump which is its own commit before code).

---

## Out of scope (for PR #1; covered by future plans)

- Python implementation — separate plan written after this PR merges.
- .NET implementation — separate plan written after Python PR merges.
- Blog post / landing-page integration of the JSONL output — follow-up work.
