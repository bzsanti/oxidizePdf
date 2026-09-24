# OmniDocBench text serialization contract

Issue: [#620](https://github.com/bzsanti/oxidizePdf/issues/620).

The default benchmark serialization is `rust-split-whitespace-v1`. It matches
oxidize-stats release 5.1.3. The alternative `preserve-text-v1` must be selected
explicitly and has a different benchmark identity. The PDF extraction API is
unchanged. This is a benchmark output policy, not a text-extraction correction.

## Exact bytes

Both contracts write UTF-8 without adding a BOM or terminal newline.
`preserve-text-v1` writes the extracted string unchanged, including any existing
BOM, spaces and line breaks. `rust-split-whitespace-v1` splits on the following
fixed Unicode code points, drops empty tokens and joins tokens with U+0020:

- U+0009–U+000D, U+0020, U+0085, U+00A0, U+1680;
- U+2000–U+200A, U+2028, U+2029, U+202F, U+205F, U+3000.

No other characters are changed. U+200B, U+FEFF and U+001C–U+001F are preserved.
Unicode composition, punctuation and word order are unchanged. Empty or
whitespace-only normalized text produces zero bytes. Extraction errors also
produce an empty prediction, but remain separately identified in the report.
Never remove such pages from a benchmark population.

The fixed set reproduces the historical Rust `split_whitespace` semantics. Its
explicit definition prevents a future Unicode/toolchain update from silently
changing the contract. Python `str.split()` is not an equivalent substitute.

The Rust implementation and literal conformance vectors live in
`oxidize-pdf-core/examples/support/`. The stats copies live in
`benchmark/src/bin/support/`. Keep each pair byte-identical; tests cover both
contracts, the historical Rust operation, all separators and negative controls.
A semantic change requires a new contract ID and new benchmark identity.

## Commands

```bash
cargo run --locked -p oxidize-pdf --example omnidocbench_export --   jobs.json predictions report.json
cargo run --locked -p oxidize-pdf --example omnidocbench_export --   jobs.json predictions-preserved report-preserved.json   --serialization preserve-text-v1
```

`omnidocbench_gate.py export` accepts the same `--serialization` option. It
writes the actual serialization from the exporter report into the manifest,
records postprocessing source and vector hashes, and writes a sibling
`.evaluation.yaml` for the fixed official text/reading-order configuration.
Use `evaluate` to execute the pinned evaluator in a fresh output directory. It
copies the dataset JSON and predictions, verifies their hashes, generates the
configuration from the pinned template and checks that inputs remain unchanged
during execution. Only a successful run with valid per-page scores produces
`evaluation-run.json`. `summarize` requires that record and verifies its export
manifest, identity, configuration and scores. External scores without a matching
execution record cannot be summarized or compared.

```bash
python3 tools/benchmarks/omnidocbench_gate.py evaluate \
  --dataset /path/to/OmniDocBench.json --predictions predictions \
  --manifest export.json --evaluator-root /path/to/evaluator \
  --python /path/to/evaluator/.venv310/bin/python --output evaluation-new
python3 tools/benchmarks/omnidocbench_gate.py summarize \
  --dataset /path/to/OmniDocBench.json --predictions predictions \
  --manifest export.json --evaluator-root /path/to/evaluator \
  --scores evaluation-new/result/predictions_quick_match_text_block_per_page_edit.json \
  --evaluation-run evaluation-new/evaluation-run.json --output summary.json
```

Set `TMPDIR` to a directory inside the working checkout for temporary harnesses.
The evaluator checkout must pass the gate's existing revision/cleanliness checks.

The stats `quality_oracle` accepts `--serialization <id>` after its optional
prediction directory. Its default is normalized. Its approximate text score
continues to use its previous normalization independently of Markdown export.
`release_oracle.py` requires the normalized report, records its effective
configuration and hashes, and propagates the contract to the new summary.
Unknown contract IDs and incorrectly declared export reports are rejected.

Compare the real exports before comparing scores:

```bash
python3 tools/benchmarks/omnidocbench_gate.py verify-exports   --left predictions --left-report report.json   --right stats-predictions --right-report stats-predictions.report.json   --output equivalence.json
```

This command checks bytes, exact prediction names, extraction configuration,
serialization and failure records. It rejects omitted empty files or any added
terminal newline, even when normalized text might appear equivalent.

## Manifest compatibility

The gate schema is `oxidize-pdf-omnidocbench-gate/v2`. Compatibility requires the
same dataset, evaluator, metric protocol, OCR setting, population, effective
extraction configuration, serialization, conformance vectors and evaluation
configuration template hash. Text and reading order are distinct metrics:

- `OmniDocBench/end2end_eval/quick_match/text`
- `OmniDocBench/end2end_eval/quick_match/reading_order`

The gate compares text summaries. Stats reports identify both metric protocols;
its aggregate wrapper name `omnidocbench-v1-official` alone is insufficient to
establish comparability. Consumers of stats summaries must compare the complete
`benchmark_contract` metadata, not only the wrapper name or library version.
Stats imports require the typed schema `omnidocbench-contract/v1` for official
results. Worker publication also requires an identical contract in the manifest.
Database migration 14 adds the canonical contract JSON to summary and breakdown
uniqueness; the API exposes both `benchmark_contract` and `contract_key`. The
dashboard includes that key when grouping comparisons and selecting breakdowns.

Migration 14 preserves all historical values and marks their identity
`legacy-unverified`. Historical rows are displayed separately by library/version;
they are never implicitly assigned the normalized contract. New official imports
without a contract fail. Importing historical external summaries requires an
explicit distinct legacy protocol, or a new verified evaluation with the complete
contract. Do not relabel old evidence as a verified new run.

Exporter source hashes, Cargo.lock and toolchain versions remain mandatory
provenance, but need not be equal between implementations of the same semantic
contract. Prediction and score hashes also need not be equal when measuring a
new extractor. They remain integrity evidence in each sealed summary. The gate
continues to reject dirty provenance, changed datasets and changed populations.
Hashes are integrity records, not signatures or proof against an untrusted
producer fabricating a report.

## Historical validation and rollout

Legacy manifests without a serialization identity are not automatically upgraded
or assumed compatible. Preserve them. Reproduce their exact prediction files
with the historical extractor and record a separate equivalence sidecar that
references the original manifest hash. A non-equivalent migration uses a new
identity and new artifacts; it must not overwrite the old baseline.

For #620 the historical check uses the exact 5.1.3 crate, separately from the
integrated develop extractor. Exporter equivalence on develop must use the same
extractor revision on both sides. Differences between develop and 5.1.3 are not
serialization failures if both exporters agree for each revision.

Full OmniDocBench evaluation runs locally with dataset
`f5f559bddf50e36f7f9899d842d0006f13ce8afc` and evaluator
`337cc26965893db3ef53ddc119a6d6bb5bde096f`, OCR off. Preserve 981 predictions,
921 scored text pages and 780 native pages, extraction errors and evaluator
fallbacks. Preserve per-page text and reading-order outputs. Lightweight
conformance tests may run in CI; the full corpus is not a CI requirement.
