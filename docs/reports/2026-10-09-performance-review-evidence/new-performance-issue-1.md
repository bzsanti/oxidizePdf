## Problem

`oxidize-pdf-core/src/text/metrics.rs:286-308` returns owned `FontMetrics`, cloning the standard table or the contents of an existing custom `Arc`. `TextFlowContext::write_wrapped` calls measurement for each word and line; `text_block` also measures each word. Work scales with the complete font map even for a short string.

A public-API allocation probe after warmup measured 1,000 allocation calls and 4,624,000 cumulative requested bytes for 1,000 Helvetica width queries. A synthetic custom table of 10,000 characters produced 1,000 calls and 147,472,000 bytes for the same query count. These are not RSS or peak-memory figures.

## Scope

Borrow immutable standard metrics and retain shared ownership of custom metrics instead of cloning maps. Consider resolving once per layout operation only where replacement semantics permit it. Preserve document-store precedence, legacy fallback, missing-font behavior, same-name replacement and cross-document isolation. Avoid holding registry locks throughout layout or introducing an unbounded global cache.

## Specific acceptance

- [ ] Repeated lookups after warmup avoid full-map allocations for standard and document-scoped custom fonts.
- [ ] Exact widths and line wrapping remain equivalent for standard/custom/CJK fonts, replacement, fallback and two documents sharing a font name.
- [ ] Demonstrate improvement in real wrapped paragraphs/tables, with small and large metric tables; do not rely only on the lookup microbenchmark.


## Evidence and boundaries

Read-only review of oxidize-pdf 5.4.1, checkout HEAD `8a92dce443e743b385aad27bebf85ed03762b09c`, tree identical to tag v5.4.1. This is follow-up work after the completed #661, not a reopening of its delivered scope.

Local evidence is retained in `docs/reports/2026-10-09-performance-review.md` and the adjacent `2026-10-09-performance-review-evidence/` directory (not yet published). The generation experiment has 72 samples / 8,880 timed documents, two separate Callgrind profiles, and six representative PDFs with clean qpdf checks, exact page counts and expected Poppler text. The consumer uses compression-only features, an independent lock (flate2 1.1.10/miniz_oxide 0.9.1), Rust 1.96.0 and its own release profile. Measurements are workload-specific. No speedup is yet demonstrated and no historical benchmark baseline was changed.

## Shared acceptance

- [ ] Compare candidate and unchanged control under the same features, dependency lock, compiler, workloads and interleaved measurement protocol; report uncertainty and regressions.
- [ ] Measure allocations separately from timing; distinguish cumulative requested bytes from peak live memory and RSS.
- [ ] Preserve the affected public contracts with discriminating regressions and independent output checks where applicable.
- [ ] Complete focused quality/security review, proportional regression/lint checks and applicable CI before integration.
- [ ] Keep the mandatory pure-Rust/no-C dependency requirement. Work PRs target develop. Do not close this issue for profiling or documentation alone.
