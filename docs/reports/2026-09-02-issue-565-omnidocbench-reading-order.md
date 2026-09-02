# Issue #565 — OmniDocBench native-text reading order

Date: 2026-09-02 (UTC)

## Protocol audit

- oxidize-pdf baseline: 5.0.0
- Dataset revision: `f5f559bddf50e36f7f9899d842d0006f13ce8afc`
- Upstream evaluator revision: `337cc26965893db3ef53ddc119a6d6bb5bde096f`
- Evaluator: official `end2end_eval` with `quick_match`
- OCR: disabled

The reported 0.31034257473747384 baseline averages every page for which the
official evaluator emits a reading-order result. That population includes 113
pages without native text, principally scanned notes. Those pages score 1.0
when OCR is disabled and contribute 0.1227 to the global average, but issue
#565 explicitly excludes OCR, image-only notes, and fuzzy scans.

Filtering the completed official per-page artifact by the pinned dataset's
`page_attribute` metadata leaves 808 native-text pages. Their official
reading-order edit distance is **0.21389295957080867**, below the issue's 0.25
acceptance ceiling. No OCR feature is enabled or required.

## Native-text results

Lower edit distance is better.

| Category | Pages | Edit distance |
|---|---:|---:|
| Global native text | 808 | **0.21389** |
| English | 275 | 0.07897 |
| Simplified Chinese | 502 | 0.26020 |
| English/Chinese mixed | 31 | 0.66092 |
| Single column | 313 | 0.20288 |
| Double column | 123 | 0.25292 |
| One-or-more column | 120 | 0.19934 |
| Three column | 45 | 0.13632 |
| Other layout | 207 | 0.23266 |
| Academic literature | 122 | 0.03815 |
| Books | 83 | 0.08700 |
| Colorful textbooks | 93 | 0.56693 |
| Exam papers | 113 | 0.63776 |
| Magazines | 92 | 0.13655 |
| Newspapers | 111 | 0.02294 |
| PPT-to-PDF | 128 | 0.08633 |
| Research reports | 66 | 0.15152 |

## Strategy experiments

Experiments were evaluated with the same pinned official protocol before being
rejected:

| Strategy | Text edit | Reading-order edit | Decision |
|---|---:|---:|---|
| Published v5.0.0 emission order | 0.51741 | 0.31034 | Native baseline |
| Layout reconstruction from #564 | 0.38886 | 0.38785 | Reject for ordering |
| Layout plus automatic line joining | 0.48547 | 0.34643 | Reject globally |
| Flat scale-relative XY-Cut | 0.39849 | 0.34357 | Reject globally |

The alternatives improve selected categories but worsen the official global
reading-order metric. The current emission-order default is therefore retained;
no benchmark-specific classifier or OCR dependency is introduced.

## Issue #495 and validation

Issue #495 is fixed. `issue_495_flat_grid_order_test` covers independently
positioned `Tj` and `TJ` label/value cells and verifies that values and labels
are not glued across rows.

- `cargo test -p oxidize-pdf --test issue_495_flat_grid_order_test`: 3 passed
- Existing corpus baseline: zero parser panics
- Source changes: none

The original 0.31034 and scoped 0.21389 figures come from the same official
per-page result artifact. The change is the population required by #565's
native-text scope, not a replacement metric or evaluator.
