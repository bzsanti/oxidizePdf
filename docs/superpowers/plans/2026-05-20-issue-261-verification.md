# Issue #261 — Live Verification Against Real Corpus

**Date:** 2026-05-20
**Fix branch HEAD:** `2516f1c` (Task 5 commit, post-wiring)
**Verification method:** temporarily merged fix branch into `feature/rag-realworld-rust`, ran `cargo run --example rag_realworld` against five live URLs, then hard-reset the feature branch back to its pre-merge state (`6004c04`). The fix branch ships pure.

## Corpus

The four documents that parse successfully (Higgs is blocked by parser bug #260):

- **ens** — Real Decreto 311/2022 (Esquema Nacional de Seguridad), Spanish gov, ~1 MB
- **boe-sumario** — BOE sumario diario 2025-01-15, Spanish gov, ~330 KB
- **bsi-tr-02102** — BSI TR-02102 (Cryptographic Mechanisms, German master), ~1 MB
- **ncsc-caf** — NCSC Cyber Assessment Framework v4.0, UK gov, ~615 KB

## Before / After

| Slug | Before chunks / avg tok | After chunks / avg tok | Reduction | Notes |
|---|---|---|---|---|
| ens | 8279 / 4.6 (48 % 1-tok) | 803 / 45 (6 oversized) | 10.3× fewer, 9.8× larger | URL footer correctly joined into one chunk |
| boe-sumario | 1066 / 24.0 | 102 / 250 (12 oversized) | 10.5× fewer, 10.4× larger | Largest gain per-chunk |
| higgs | parse fail #260 | parse fail #260 | n/a | TeX /Length tolerance bug (separate) |
| bsi-tr-02102 | 1674 / 27.5 (88 % 1-tok) | 284 / 47 (88 headings) | 5.9× fewer, 1.7× larger | Heading detection preserved |
| ncsc-caf | 12180 / 6.3 (95 % 1-tok) | 519 / 142 | 23.5× fewer, 22.5× larger | Largest reduction in pathological 1-tok chunks |

## Sample chunks — before vs after

**ENS chunk 0** (BOE footer URL):

Before:
```
ens-0000 (tok=1): "V"
ens-0001 (tok=3): "erificable en https://www"
ens-0002 (tok=1): ".boe.es"
ens-0003 (tok=2): "cve: BOE-A-2022-7191"
```

After:
```
ens-0000 (tok=5): "Verificable en https://www.boe.es\ncve: BOE-A-2022-7191"
```

The four pre-fix chunks become one paragraph-shaped chunk.

**ENS chunk 1** (document title):

After:
```
ens-0001 (tok=14): "7191Real Decreto 311/2022, de 3 de mayo, por el que se regula el Esquema"
```

The title spans multiple PDF fragments. Now a single readable line. (The leading "7191" is the preceding page-id artifact from the footer — separate cosmetic issue, not blocking.)

## Residual issues observed

These remain after the fix and are out of scope for #261:

1. **BSI title strings concatenated without spaces** — e.g. `"TechnischeRichtlinie–KryptographischeAlgorithmenundSchlüssellängen"`. Root cause: the kerning-fix path (`merge_close_fragments`) merges fragments whose X gap is below `0.5 × font_size`. On display headings with letter-spacing > 1 but < that threshold, individual words still merge without spaces. Task 4's incidental fix to `merge_close_fragments` (insert a space when gap exceeds `space_threshold × font_size`) addresses this for body text but not for tightly letter-spaced display type. Tunable via the `space_threshold` field in a follow-up issue.

2. **Higgs paper parse failure** — tracked separately in issue #260, unrelated to the chunking pipeline.

## Conclusion

The fix delivers paragraph-granularity chunks across all four real-world PDFs that parse, with 6–24× reduction in chunk count and proportionally larger average chunks. Output is now suitable for RAG ingestion against the four targets.
