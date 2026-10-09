# Candidate evidence (#700–#703)

Base: develop `512d7c54309cce7b744a9d747bf849ba4de4da52` (tree identical to 5.4.1). Product files are identified by `accepted-source-hashes.json`. Executable hashes identify the controls and candidates retained locally under `target/performance-700-703`; binaries and large rasters are not committed.

## Reproduction

Create an independent Cargo consumer with this Cargo.toml/Cargo.lock and adapt only the dependency path to the checkout being tested. Put invoice.rs in src/main.rs and workloads.rs in src/bin/workloads.rs. Adapt the bundled CJK include path to `oxidize-pdf-core/tests/fixtures/writer_resources/WriterCjkTest-Regular.otf`. Build with `cargo build --release --locked --offline --bin writer-perf-probe --bin workloads`; preserve the resulting binaries for each source candidate. Metrics requires `--features alloc-count --bin metrics` with metrics.rs in src/bin/metrics.rs. Do not run unrelated builds/profiles while timing.

Use `compare.py LABEL CONTROL_BINARY CANDIDATE_BINARY KIND` with binaries next to the script. Kinds: layout, encoding, graphics, invoice. Run `validate_pairs.py LABEL` afterwards. It validates all output PDFs with qpdf, compares Poppler bbox/text, and hashes every page raster at 72 dpi. Synthetic `.out` PDFs/layouts and validation JSON are retained; generated rasters are local only.

The main report explains phase controls. The workload harness grew CJK and fractional-graphics modes between phases; already-measured layout, flow and invoice inputs were unchanged. Never compare samples across different phases/days as an estimate of improvement. Each reported ratio uses its own interleaved control/candidate blocks, and fallback regressions are included.

## Evidence selection

- #700: `700-red.log` (expected allocation assertion failure), `700-green.log`, `700-unit.log`, `700-contracts.log`, `700-summary.json`, `700-external.json`.
- #702 final: `702-unit-candidate.log`, `702-final-bytes.log`, `702-final-summary.json`, `702-final-external.json`. Initial #702 timings remain for transparency; they are superseded by exact UTF-16 reservation.
- #701: `701-unit.log`, `701-contracts.log`, `701-mutation-red.log`, `701-encoding-summary.json`, `701-invoice-summary.json`, `701-encoding-external.json`. The mutant deliberately drops non-ASCII characters; it is not product code.
- #703 final: `703-fallback-unit.log`, `703-mutation-green.log` / `703-mutation-red.log`, `accepted-graphics-summary.json`, `accepted-graphics-external.json`. The numeric mutation probe copies the actual operators/helpers/test and stubs only unexercised color emitters; it verifies that losing the sign of negative zero fails. The probe predates grouped fallback formatting; the mutated sign branch and its expected-byte assertion are unchanged in the final source. Final native product tests provide integration evidence. Earlier #703 variants are superseded, not silently removed.
- Complete candidate: `accepted-invoice-summary.json`, `accepted-invoice-external.json`, `accepted-heap.log`, `accepted-workspace.log`, `final-clippy2.log`, `final-fmt2.log`, `final-kripteia-*.log`.
- Allocations are whole-process cumulative requests measured separately from timing (104 documents including warmups/output). They are neither peak live memory nor RSS. `control-heap-final.log` is the valid control; `control-heap.log` failed before executing the control binary.

`702-unit.log` is an accidental root-checkout invocation that was interrupted; it is not candidate evidence. `703-initial-workspace-interrupted.log` was stopped when performance evidence invalidated that candidate, before counting any full-suite pass. Early #701 logs were replaced by the successful rerun after fixing type inference; no setup/compile failure is claimed as a discriminating RED.

CI/integration status belongs to TASKS.md and the PR. A local PASS does not imply the issues are closed or the change is released.
