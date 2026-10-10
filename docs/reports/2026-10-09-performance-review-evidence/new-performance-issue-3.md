## Problem

`oxidize-pdf-core/src/text/mod.rs:150-174` first creates a WinAnsi vector and then a separate escaped vector. `text/encoding.rs:48` grows the first from Vec::new; `:523` reserves four times the encoded length for the second. The custom-font path materializes Vec<u16> and formats hexadecimal per unit. `build_show_text_op` accounts for 9.37% inclusive instructions with compression and 13.76% raw in the current invoice profile; these are not CPU-time or achievable-speedup percentages.

## Scope

Evaluate direct encoding/escaping into one output buffer with measured capacity policy. Iterate UTF-16 directly and evaluate a hexadecimal lookup table for custom fonts. Preserve existing encoding substitution, named/octal escapes and uppercase UTF-16 hex semantics across TextContext and TextFlowContext.

## Specific acceptance

- [ ] Byte-exact tests cover ASCII, parentheses/backslash, all control/high bytes, unsupported characters, empty input, CJK and surrogate pairs.
- [ ] Large input capacity arithmetic remains safe; no silent truncation or altered error behavior.
- [ ] Allocation and timing comparisons cover ASCII invoices, accent-heavy text and custom/CJK text, with end-to-end PDF text/geometry checks.


## Evidence and boundaries

Read-only review of oxidize-pdf 5.4.1, checkout HEAD `8a92dce443e743b385aad27bebf85ed03762b09c`, tree identical to tag v5.4.1. This is follow-up work after the completed #661, not a reopening of its delivered scope.

Local evidence is retained in `docs/reports/2026-10-09-performance-review.md` and the adjacent `2026-10-09-performance-review-evidence/` directory (not yet published). The generation experiment has 72 samples / 8,880 timed documents, two separate Callgrind profiles, and six representative PDFs with clean qpdf checks, exact page counts and expected Poppler text. The consumer uses compression-only features, an independent lock (flate2 1.1.10/miniz_oxide 0.9.1), Rust 1.96.0 and its own release profile. Measurements are workload-specific. No speedup is yet demonstrated and no historical benchmark baseline was changed.

## Shared acceptance

- [ ] Compare candidate and unchanged control under the same features, dependency lock, compiler, workloads and interleaved measurement protocol; report uncertainty and regressions.
- [ ] Measure allocations separately from timing; distinguish cumulative requested bytes from peak live memory and RSS.
- [ ] Preserve the affected public contracts with discriminating regressions and independent output checks where applicable.
- [ ] Complete focused quality/security review, proportional regression/lint checks and applicable CI before integration.
- [ ] Keep the mandatory pure-Rust/no-C dependency requirement. Work PRs target develop. Do not close this issue for profiling or documentation alone.
