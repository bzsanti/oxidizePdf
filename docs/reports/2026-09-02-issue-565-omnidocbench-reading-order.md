# Issue #565 — OmniDocBench native-text reading order

Date: 2026-09-02 (UTC)

## Protocol audit

- oxidize-pdf baseline: 5.0.0
- Dataset revision: `f5f559bddf50e36f7f9899d842d0006f13ce8afc`
- Upstream evaluator revision: `337cc26965893db3ef53ddc119a6d6bb5bde096f`
- Evaluator: official `end2end_eval` with `quick_match`
- OCR: disabled

The unfiltered 0.31034257473747384 diagnostic averages every page for which the
official evaluator emits a reading-order result. That population includes 113
pages labelled `data_source: note` and another 28 pages labelled `fuzzy_scan`
under `special_issue` by the pinned dataset. Notes score 1.0 when OCR is
disabled and contribute 0.1227 to the unfiltered average, but issue #565
explicitly excludes image-only notes, fuzzy scans, and OCR.

The repository's native-scope aggregator filters the completed official
per-page artifact using those explicit metadata labels. It leaves 780 native-text
pages with a scoped official reading-order edit distance of
**0.18610108290582916**, below the issue's 0.25 acceptance ceiling. No OCR
feature is enabled or required.

Reproduce the scoped artifact with:

```bash
python3 tools/benchmarks/omnidocbench_native_reading_order.py \
  /path/to/pinned/OmniDocBench.json \
  /path/to/official_reading_order_per_page_edit.json \
  --output native-reading-order.json
```

The output records the protocol, exact exclusion predicate, official scored
population, excluded and retained counts, global value, and category values.

## Native-text results

Lower edit distance is better.

| Category | Pages | Edit distance |
|---|---:|---:|
| Global native text | 780 | **0.18610** |
| English | 271 | 0.06537 |
| Simplified Chinese | 480 | 0.22699 |
| English/Chinese mixed | 29 | 0.63754 |
| Single column | 302 | 0.17384 |
| Double column | 113 | 0.18680 |
| One-or-more column | 119 | 0.19261 |
| Three column | 45 | 0.13632 |
| Other layout | 201 | 0.21141 |
| Academic literature | 122 | 0.03815 |
| Books | 82 | 0.07586 |
| Colorful textbooks | 81 | 0.50277 |
| Exam papers | 101 | 0.59472 |
| Magazines | 91 | 0.12706 |
| Newspapers | 111 | 0.02294 |
| PPT-to-PDF | 126 | 0.07447 |
| Research reports | 66 | 0.15152 |

## Strategy experiments

Experiments were evaluated with the same pinned official protocol before being
rejected:

| Strategy | Text edit (all) | Reading edit (all) | Reading edit (native) | Decision |
|---|---:|---:|---:|---|
| Published v5.0.0 emission order | 0.51741 | 0.31034 | **0.18610** | Retain |
| Layout reconstruction from #564 | 0.38886 | 0.38785 | 0.27744 | Reject for ordering |
| Layout plus automatic line joining | 0.48547 | 0.34643 | 0.22872 | Reject globally |
| Flat scale-relative XY-Cut | 0.39849 | 0.34357 | 0.22534 | Reject globally |

The alternatives improve selected categories but worsen the official global
reading-order metric. The current emission-order default is therefore retained;
no benchmark-specific classifier or OCR dependency is introduced.

## Issue #495 and validation

Issue #495 is fixed. `issue_495_flat_grid_order_test` covers independently
positioned `Tj` and `TJ` label/value cells and verifies that values and labels
are not glued across rows.

- `cargo test -p oxidize-pdf --test issue_495_flat_grid_order_test`: 3 passed
- `python3 -m unittest tools/benchmarks/omnidocbench_native_reading_order_test.py`: 7 passed
- Aggregator failure coverage: unknown and duplicate pages, invalid, non-finite and out-of-range scores, and empty native population
- Production extraction source changes: none

The unfiltered 0.31034 diagnostic and scoped 0.18610 acceptance figure come
from the same official per-page result artifact. The versioned aggregator and
its focused tests make the native-text population explicit and reproducible;
they do not replace or approximate the upstream metric.
