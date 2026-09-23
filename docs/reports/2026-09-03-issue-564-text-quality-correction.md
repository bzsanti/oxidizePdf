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

The clean implementation commit is
`36a9d11c7da38e3b959171cfad06840cc5ff724e`. Its prediction-tree SHA-256 is
`6a9240ec5b3a8a800a5f8813942edd2ee51eabed85d33f98e9c3f0d6ce3abedf` and
the sealed summary SHA-256 is
`431f967ef1f64ac66cc74ddc62d304164452f10cf29be97eadd080a0081c59a0`.

## Reproducibility correction

The first rerun incorrectly used the historical comparison runner, which
normalizes whitespace before writing Markdown predictions and therefore
reported only 49.14% similarity. The versioned gate now prefers exact
page-specific PDFs (`source.pdf_N.pdf`, page index zero) when present and falls
back to a combined source PDF (`source.pdf`, page index N-1). Its exporter writes
the extracted text unchanged, preserving the line structure used by the
official evaluator.

The final gate also verifies materialized Git LFS objects by their committed
SHA-256 and size when `git-lfs` is unavailable, rejects altered objects, accepts
the evaluator's prediction-dependent optional text pages only when they belong
to the pinned dataset, and seals the exact scored-page population in the
summary.

## Validation and quality review

- Library suite: 6,777 passed, 3 ignored.
- Focused #564: 4 passed; neighboring #477/#495/#521: 9 passed.
- T0: 21 passed, 3 maintenance generators ignored; T1: 22 passed.
- Exporter tests: 2 passed; benchmark gate: 21 passed.
- Formatting, Clippy, build and `git diff --check`: passed.
- Kripteia Rust: 94/100 globally; focused #564 tests: 90/100.
- Kripteia Python: no tests discovered; the 21 `unittest` cases above are the
  direct test evidence.
- Kripteia security: no issues in either Rust or Python scope.

The review found and corrected misleading cross-revision extraction metadata,
missing split-PDF resolution, Git LFS provenance handling and incomplete
official score-population handling. A proposed `page_no` validation finding was
disproved against the pinned dataset and was not retained. The repeated final
review has no remaining findings. No PR was opened during validation.

## Session handoff

The session closed on branch `fix/issue-564-omnidocbench-text-quality` at
`472f4fbc3b6ad26d3919170d62c9b444cfc7dc0b`, after refreshing
`origin/develop` to `d1a1ab0e99098e01cc51a04aa5ee315f5772ab4d`. The diff
against that base contains only the six #564 files listed in this report and
plan. The quality review is complete; the only unchecked acceptance item is
opening the PR.

The worktree also contains pre-existing or user-owned changes in `README.md`,
`docs/reports/2026-09-02-issue-565-omnidocbench-reading-order.md`,
`oxidize-pdf-core/Cargo.toml`, and eight untracked handoff reports dated
2026-08-25 through 2026-09-01. They are unrelated to #564 and must not be
staged with this branch.

To continue, first verify `git diff --check` and
`git diff --name-status origin/develop...HEAD`; then push only
`fix/issue-564-omnidocbench-text-quality` and open a PR against `develop`.
After the PR exists, mark the final checklist item and report its CI status.

For cleanup, `cargo-sweep 0.8.0` was run with both `--dry-run --time 5` and
`--time 5` against this workspace; both reported nothing eligible, so zero
bytes were reclaimed. Six project-scoped benchmark paths under `/tmp` were
inspected (about 1.83 GB total). All were created on 2026-09-03, are newer than
the five-day cutoff, and include evidence referenced above, so all six were
preserved. No uncertain cleanup candidates were deleted.

## Delivery

PR #570 was opened against `develop` after the quality review completed with no
remaining findings: https://github.com/bzsanti/oxidizePdf/pull/570
