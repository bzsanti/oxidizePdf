## Summary
First concrete showcase for the RAG-first positioning: a Rust example that downloads five real-world government and academic PDFs and produces RAG-ready JSONL chunks.

- `oxidize-pdf-core/examples/rag_realworld.rs` ingests ENS (BOE), BOE sumario diario, ATLAS Higgs paper (arXiv), BSI TR-02102 (DE), and NCSC CAF v4.0 (UK).
- Output is one JSONL line per chunk, schema compatible with LangChain `JSONLoader`, LlamaIndex `JSONReader`, and Pinecone batch upsert.
- Per-document stats (chunk count, mean tokens, oversized count, headings detected) printed to stderr; failures handled as skip-and-continue with exit code = number of failures.
- Cached download in `./corpus_cache/` (sha1-keyed); JSONL output in `./out/`.

## Verification on the real corpus (post-#261 + #262)

| Slug | Chunks | Mean tok | Oversized | Headings detected |
|---|---|---|---|---|
| ens (BOE Real Decreto 311/2022) | 217 | 170 | 3 | 195 |
| boe-sumario (BOE 2025-01-15) | 84 | 304 | 3 | 80 |
| higgs (arXiv:1207.7214) | parse fail | — | — | — |
| bsi-tr-02102 (BSI cryptography, DE) | 281 | 48 | 0 | 88 |
| ncsc-caf (NCSC CAF v4.0, UK) | 100 | 168 | 0 | 5 |

Run:
```
[ok]   ens           → 217 chunks   ~170 tok/avg   3 oversized   195 headings   out/ens.jsonl
[ok]   boe-sumario   → 84 chunks   ~304 tok/avg   3 oversized   80 headings   out/boe-sumario.jsonl
[fail] higgs         → parse error: Syntax error at position 169585: Unknown keyword: Sendstream
[ok]   bsi-tr-02102  → 281 chunks   ~48 tok/avg   0 oversized   88 headings   out/bsi-tr-02102.jsonl
[ok]   ncsc-caf      → 100 chunks   ~168 tok/avg   0 oversized   5 headings   out/ncsc-caf.jsonl

4/5 documents processed (1 failed) · exit 1
```

Sample ENS chunk: heading `"BOLETÍN OFICIAL DEL ESTADO"`, text `"Núm. 106 Miércoles 4 de mayo de 2022 Sec. I. Pág. 61715"` — coherent body content with detected section context.

## Tests
- Offline unit test in `tests/rag_realworld_jsonl_test.rs` (4 cases) verifies the JSONL writer's contract against synthetic `RagChunk` values: id format, full metadata field set, `heading_context` JSON `null` serialization, oversized flag preservation, no embedded newlines in serialized output.
- Library suite: 6394 passed, 0 failed.

## CI
- `cargo build --examples` already validates the new example in the existing CI matrix.
- New workflow `.github/workflows/verify-corpus.yml` runs weekly (Mondays 06:00 UTC) and opens a tagged issue if any of the five corpus URLs returns non-200. Uses GET (not HEAD) because BSI's CDN rejects HEAD requests.
- `corpus_cache/` and `out/` added to root `.gitignore`.

## Known limitations
- **Higgs paper** fails on the arXiv/TeX `/Length`-mismatch parser bug tracked in **#260**. This is intentional skip-and-continue demonstration, not a defect in this PR.
- The first ENS chunk currently joins page footer + page header into one chunk because adjacent visual lines share an almost-identical baseline. Tracked in **#265** (line-grouping interleaving) — out of scope here.

## Test plan
- [x] `cargo run --example rag_realworld` produces 4/5 JSONL files; 1 fails with known #260.
- [x] `cargo test --test rag_realworld_jsonl_test` — 4/4 pass.
- [x] `cargo clippy --example rag_realworld -- -D warnings` — clean.
- [ ] Post-merge: trigger `verify-corpus` workflow manually from the Actions tab and confirm all 5 URLs return 200.

## Follow-up PRs
Mirrors of this example in `oxidize-python` and `oxidize-dotnet` come next (same corpus, same JSONL schema, same skip-and-continue semantics).
