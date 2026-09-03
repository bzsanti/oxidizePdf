# Issue #564 — text-quality correction

Date: 2026-09-03 (UTC)

## Outcome

`PlainTextExtractor::preserve_layout()` now uses the complete text engine and
orders its line groups with the existing scale-relative XY-Cut implementation.
This retains `/ActualText`, artifact filtering, font metrics and error
propagation while satisfying both the text-quality gate from #564 and the
native reading-order gate from #565.

The first correction candidate enabled fragment-level column reconstruction.
It reached 61.10% text similarity, but was rejected because its native
reading-order edit distance was 0.27748, above #565's maximum of 0.25.

## Pinned protocol

- Dataset revision: `f5f559bddf50e36f7f9899d842d0006f13ce8afc`
- Evaluator revision: `337cc26965893db3ef53ddc119a6d6bb5bde096f`
- Evaluator: official `end2end_eval` / `quick_match`
- OCR: disabled
- Predictions written: 981
- Extraction failures: 1 (`Invalid Filter type`, also present in the baseline)

## Candidate measurements

Lower edit distance and higher similarity are better.

| Metric | v5.0.0 baseline | Candidate | Acceptance |
|---|---:|---:|---:|
| Global text edit distance | 0.51741 | **0.39994** | — |
| Global text similarity | 48.26% | **60.01%** | ≥55% |
| Global reading-order edit distance | 0.31034 | 0.34447 | diagnostic |
| Native reading-order edit distance | 0.18610 | **0.22639** | ≤0.25 |

The official text artifact contains 945 scored text entries; the official
reading-order artifact contains 921 pages, of which the versioned #565 filter
classifies 780 as native text.

## Reproducibility correction

The first rerun incorrectly used the historical comparison runner, which
normalizes whitespace before writing Markdown predictions and therefore
reported only 49.14% similarity. The versioned gate now prefers exact
page-specific PDFs (`source.pdf_N.pdf`, page index zero) when present and falls
back to a combined source PDF (`source.pdf`, page index N-1). Its exporter writes
the extracted text unchanged, preserving the line structure used by the
official evaluator.

Final clean-commit validation and formal quality review remain pending; no PR
will be opened before both complete.
