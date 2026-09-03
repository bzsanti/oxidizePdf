# Issue #568 — reproducible OmniDocBench gate

## Contract

The repository owns both sides of the measurement boundary:

- `omnidocbench_export.rs` extracts every dataset page through
  `PlainTextExtractor::preserve_layout()` with OCR disabled. It writes an empty
  prediction for a failed page and records that failure instead of silently
  dropping the page.
- `omnidocbench_gate.py` resolves source PDFs, records provenance and hashes,
  validates the official per-page text artifact, reports global and native-text
  populations separately, and rejects incompatible comparisons.

The gate creates a temporary `git archive` of `--source-root`, adds the same
versioned exporter to that archive, and compiles it with the archived commit's
own workspace manifest and lockfile using `--locked --offline`. This measures a
clean `v5.0.0` worktree and a clean candidate without modifying either checkout
or introducing a separate path dependency.

## Reproduction identity

- Schema: `oxidize-pdf-omnidocbench-gate/v1`
- Dataset and evaluator revisions: verified against clean Git checkouts
- Protocol: official `end2end_eval` / `quick_match` / `text`
- OCR: disabled; it is neither configured nor linked by the exporter
- Native population: `native-text-v1`, excluding `data_source == note` and any
  page whose `special_issue` contains `fuzzy_scan`
- Extraction configuration: serialized by the Rust exporter from the effective
  `PlainTextConfig::preserve_layout()` values
- Build identity: archived `Cargo.lock` hash plus `rustc -Vv` and `cargo -V`

The official text population is derived from the evaluator's versioned text
categories (`text_block`, `title`, `code_txt`, and `reference`). With the pinned
dataset this identifies 921 text-scorable pages and 60 pages containing only
non-text/ignored categories.

## Commands

From this repository, generate predictions and their provenance manifest in one
command. `PDF_ROOT` may contain nested directories, but source PDF basenames
must be unique. The dataset image name and `page_no` resolve the source name and
zero-based page index (for example `paper.pdf_7.jpg` becomes page 6 of
`paper.pdf`).

```bash
python3 tools/benchmarks/omnidocbench_gate.py export \
  --source-root /path/to/clean/oxidize-pdf-checkout \
  --dataset-root /data/omnidocbench \
  --evaluator-root /path/to/clean/evaluator-checkout \
  --dataset /data/omnidocbench/OmniDocBench.json \
  --pdf-root /data/source-pdfs \
  --predictions /artifacts/predictions \
  --manifest /artifacts/run-manifest.json \
  --dataset-revision f5f559bddf50e36f7f9899d842d0006f13ce8afc \
  --evaluator-revision 337cc26965893db3ef53ddc119a6d6bb5bde096f
```

The command refuses a dirty source checkout by default. `--allow-dirty` is for
experiments only and records a dirty-state hash; such a run is not labelled as
the clean Git commit.

Run the pinned upstream evaluator over the generated Markdown directory. Then
validate its `*_quick_match_text_block_per_page_edit.json` artifact:

```bash
python3 tools/benchmarks/omnidocbench_gate.py summarize \
  --dataset /data/omnidocbench/OmniDocBench.json \
  --scores /artifacts/model_quick_match_text_block_per_page_edit.json \
  --predictions /artifacts/predictions \
  --evaluator-root /path/to/clean/evaluator-checkout \
  --manifest /artifacts/run-manifest.json \
  --output /artifacts/summary.json
```

The summary recomputes the prediction-tree hash and re-verifies the evaluator
revision. Compare clean, untampered baseline and candidate summaries only when
their machine-readable identities and population counts agree:

```bash
python3 tools/benchmarks/omnidocbench_gate.py compare \
  --baseline /artifacts/v5.0.0-summary.json \
  --candidate /artifacts/candidate-summary.json \
  --output /artifacts/comparison.json
```

## Reproduced final #564 result

Feeding the pinned official per-page artifact for `d5e2d0b` through the gate
produces:

| Population | Pages | Edit distance | Similarity |
|---|---:|---:|---:|
| Official global text | 921 | 0.5085693812417305 | 49.1430618758% |
| Native text v1 | 780 | 0.41990107039261854 | 58.0098929607% |

This reproduces the final commit rather than the earlier 61.11% experimental
artifact. The 60 non-scorable dataset pages are reported separately; all 981
pages must still have prediction files.

The implementation checklist and remaining external full-run validation are in
`docs/plans/2026-09-03-issue-568-omnidocbench-gate-tdd.md`.
