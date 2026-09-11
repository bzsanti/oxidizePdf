# Issue #564 — OmniDocBench native-text quality

Date: 2026-09-02 (UTC)

## Protocol

- oxidize-pdf baseline: 5.0.0
- Dataset revision: `f5f559bddf50e36f7f9899d842d0006f13ce8afc`
- Upstream evaluator revision: `337cc26965893db3ef53ddc119a6d6bb5bde096f`
- Evaluator: official `end2end_eval` with `quick_match`
- Pages: 981
- OCR: disabled
- Extraction errors: 0 before and after

One malformed corpus file fails during document opening with `Invalid Filter
type` in both runs. It is represented by an empty prediction, as in the
baseline, and is not an extraction-stage regression.

## Change

`PlainTextExtractor::preserve_layout()` now delegates to the complete
position-bearing text engine. This removes the independent partial extraction
path for layout-aware calls and makes the plain-text facade inherit its font
metrics, form/content traversal, graphics-state handling, `/ActualText`
substitution, `/Artifact` filtering, and geometric reconstruction.

The lightweight non-layout path remains unchanged.

## Official results

Lower edit distance and higher similarity are better.

| Metric/category | Baseline edit | New edit | Baseline similarity | New similarity |
|---|---:|---:|---:|---:|
| Global text | 0.51741 | **0.38886** | 48.26% | **61.11%** |
| English | 0.26885 | **0.19880** | 73.12% | **80.12%** |
| Simplified Chinese | 0.58763 | **0.41622** | 41.24% | **58.38%** |
| English/Chinese mixed | 0.88236 | **0.87599** | 11.76% | **12.40%** |
| Academic literature | 0.31317 | **0.20835** | 68.68% | **79.16%** |
| PPT-to-PDF | 0.32537 | **0.23464** | 67.46% | **76.54%** |
| Books | 0.36134 | **0.31019** | 63.87% | **68.98%** |
| Magazines | 0.35673 | **0.21462** | 64.33% | **78.54%** |
| Newspapers | 0.40844 | **0.11743** | 59.16% | **88.26%** |

The official global acceptance threshold is 55% similarity. The measured
result is 61.11%, 6.11 percentage points above the threshold and 12.85 points
above the v5.0.0 baseline.

The evaluator wrote the complete metric JSON before its optional display step
failed against NumPy 2 (`np.NaN` was removed). The figures above come directly
from that completed official metric artifact.

The official reading-order edit distance changed from 0.31034 to 0.38785.
An official A/B run with geometric sorting disabled measured 0.38819, showing
that sorting and column detection do not account for this change. Reading-order
strategy selection and correction are deliberately tracked by #565; this issue
changes text recovery and reconstruction, and does not claim a reading-order
improvement.

## Validation

- `cargo clippy -p oxidize-pdf --lib --test issue_564_plaintext_quality_test -- -D warnings`
- `cargo test -p oxidize-pdf --lib`: 6,777 passed, 3 ignored
- Focused #564 tests: 3 passed
- Neighbor regressions #477, #495, and #521: 9 passed
- T0 regression corpus: 21 passed, 3 maintenance generators ignored
- T1 spec corpus: 22 passed
- `cargo fmt --all -- --check`
- `git diff --check`

The focused regressions prove that layout-aware plain text filters marked
artifacts, applies `/ActualText`, uses Standard-14 AFM advances for contiguous
positioned runs, and propagates extraction errors without silently downgrading
to the legacy parser.
