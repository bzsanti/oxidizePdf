## Problem

`oxidize-pdf-core/src/graphics/ops.rs:197-226,317-345` still uses general write!/writeln! formatting for numeric operators. #661 optimized object-writer numbers, not this route. `serialize_ops` accounts for 18.73% inclusive instructions with compression and 27.56% raw in the current invoice profile, including other serialization work. These percentages are not promised savings.

## Scope

Evaluate bounded numeric fast paths with the historical formatter as fallback, or another demonstrably equivalent formatter. Preserve precision per operator, rounding, signs/negative zero, very large/small values, non-finite clamping, operator order and painter semantics. Do not replace fixed decimal formats with shortest formatting without proving the required equivalence.

## Specific acceptance

- [ ] Differential byte tests use the historical formatter as control and cover rounding boundaries, extremes, negatives/zero and non-finite inputs for each affected format.
- [ ] Text and graphics outputs preserve geometry and independently rendered pixels.
- [ ] Demonstrate end-to-end improvement on text plus graphics-heavy workloads; record fast-path hit rates and fallback costs.


## Evidence and boundaries

Read-only review of oxidize-pdf 5.4.1, checkout HEAD `8a92dce443e743b385aad27bebf85ed03762b09c`, tree identical to tag v5.4.1. This is follow-up work after the completed #661, not a reopening of its delivered scope.

Local evidence is retained in `docs/reports/2026-10-09-performance-review.md` and the adjacent `2026-10-09-performance-review-evidence/` directory (not yet published). The generation experiment has 72 samples / 8,880 timed documents, two separate Callgrind profiles, and six representative PDFs with clean qpdf checks, exact page counts and expected Poppler text. The consumer uses compression-only features, an independent lock (flate2 1.1.10/miniz_oxide 0.9.1), Rust 1.96.0 and its own release profile. Measurements are workload-specific. No speedup is yet demonstrated and no historical benchmark baseline was changed.

## Shared acceptance

- [ ] Compare candidate and unchanged control under the same features, dependency lock, compiler, workloads and interleaved measurement protocol; report uncertainty and regressions.
- [ ] Measure allocations separately from timing; distinguish cumulative requested bytes from peak live memory and RSS.
- [ ] Preserve the affected public contracts with discriminating regressions and independent output checks where applicable.
- [ ] Complete focused quality/security review, proportional regression/lint checks and applicable CI before integration.
- [ ] Keep the mandatory pure-Rust/no-C dependency requirement. Work PRs target develop. Do not close this issue for profiling or documentation alone.
