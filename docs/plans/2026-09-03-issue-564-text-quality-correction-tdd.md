# Issue #564 — text-quality correction (TDD trace)

## Red

- [x] Add a focused regression proving that layout-aware plain text follows
  column reading order when content streams interleave the two columns.
- [x] Confirm the regression fails with the reviewed `detect_columns = false`
  configuration.

## Green

- [x] Reject direct column reconstruction after it reached 61.10% text similarity
  but regressed native reading-order edit distance to 0.27748.
- [x] Use the existing scale-relative XY-Cut orderer without restoring the
  rejected error-swallowing fallback.
- [x] Pass the focused #564 tests and neighboring #477, #495, and #521 tests
  (4 + 5 + 3 + 1 passed).
- [x] Pass the library suite (6,777 passed, 3 ignored), Clippy, formatting,
  T0 (21 passed, 3 ignored), and T1 (22 passed).

## Benchmark acceptance

- [x] Generate predictions from the candidate with OCR disabled and without
  flattening line breaks (981 written, 1 known invalid-filter failure).
- [x] Run pinned OmniDocBench `end2end_eval` / `quick_match`.
- [x] Confirm official global text similarity is at least 55% (60.01%).
- [ ] Re-run the benchmark from the final clean commit and seal its manifest.
- [ ] Attach global, native-text, category, extraction-error, and reading-order
  before/after metrics.
- [ ] Keep the issue open if the exact committed candidate misses any criterion.

## Quality review and delivery

- [ ] Run the formal QR, including Kripteia test-quality and security analysis.
- [ ] Correct every finding and repeat affected checks.
- [ ] Open a PR only after the QR has no remaining findings.
