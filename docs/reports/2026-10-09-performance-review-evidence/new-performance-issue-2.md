## Problem

`oxidize-pdf-core/src/text/mod.rs:262-269` inserts every character of every write into a per-font HashSet, including repeated ASCII characters. `Document::add_page` then aggregates usage by font name. The current invoice profile attributes 35.38% inclusive instructions to `TextContext::write` with compression and 51.87% without; this includes encoding/operator construction and is not the possible tracking speedup.

## Scope

Evaluate a compact ASCII membership representation plus a complete Unicode fallback, or another measured representation that reduces repeated hashing. Preserve every character and union by PDF font name across pages and text/graphics contexts. Do not simply skip builtin fonts: custom fonts may be registered later under colliding names. Keep Raw/resource and subsetting contracts intact.

## Specific acceptance

- [ ] Usage is preserved across repeated writes, pages, both contexts, late custom registration and builtin/custom name collisions.
- [ ] Unicode beyond ASCII, CJK and non-BMP characters remain distinct and correctly included in subsets.
- [ ] Compare repetitive invoices and diverse Unicode workloads, including representation conversion costs and memory tradeoffs.


## Evidence and boundaries

Read-only review of oxidize-pdf 5.4.1, checkout HEAD `8a92dce443e743b385aad27bebf85ed03762b09c`, tree identical to tag v5.4.1. This is follow-up work after the completed #661, not a reopening of its delivered scope.

Local evidence is retained in `docs/reports/2026-10-09-performance-review.md` and the adjacent `2026-10-09-performance-review-evidence/` directory (not yet published). The generation experiment has 72 samples / 8,880 timed documents, two separate Callgrind profiles, and six representative PDFs with clean qpdf checks, exact page counts and expected Poppler text. The consumer uses compression-only features, an independent lock (flate2 1.1.10/miniz_oxide 0.9.1), Rust 1.96.0 and its own release profile. Measurements are workload-specific. No speedup is yet demonstrated and no historical benchmark baseline was changed.

## Shared acceptance

- [ ] Compare candidate and unchanged control under the same features, dependency lock, compiler, workloads and interleaved measurement protocol; report uncertainty and regressions.
- [ ] Measure allocations separately from timing; distinguish cumulative requested bytes from peak live memory and RSS.
- [ ] Preserve the affected public contracts with discriminating regressions and independent output checks where applicable.
- [ ] Complete focused quality/security review, proportional regression/lint checks and applicable CI before integration.
- [ ] Keep the mandatory pure-Rust/no-C dependency requirement. Work PRs target develop. Do not close this issue for profiling or documentation alone.
