# Issue #620 — implementation and local validation

Issue: [#620](https://github.com/bzsanti/oxidizePdf/issues/620).

Implementation is complete locally in `fix/issue-620-serialization`, based on
`e1792e83dbf0fdb42f65742fbc705dc8de906e25`. The user created this worktree next
to the original repository and explicitly authorized edits to the affected
oxidize-stats files. No commits, pushes, PRs or issue-state changes were made.

## Result

Both exporters default to `rust-split-whitespace-v1`; `preserve-text-v1` is
explicit and incompatible for score comparisons. They share identical source
and literal Unicode conformance vectors. The extraction API is unchanged.

The v2 gate requires effective serialization, conformance-vector and evaluator
configuration identities. It retains implementation provenance and integrity
checks while allowing different exporter implementations under the same
contract. `verify-exports` verifies actual bytes, names and error records.

Stats captures the contract and configuration in its export report, release
manifest and new official summary. Its approximate text benchmark retains its
previous behavior. See [the contract](../omnidocbench-serialization.md).

## TDD and checks

The initial regression on integrated develop wrote `Alpha\nBeta` instead of
`Alpha Beta`. Two comparison regressions reported `ValueError not raised` for
incompatible or missing serialization. The stats release-manifest regression
found no serialization field. A further RED proved evaluator configuration was
not part of compatibility. Each was fixed before the final checks.

- Rust exporter: 6 tests pass, including actual PDF exports under both modes,
  Unicode vectors, empty pages and separately recorded extraction errors.
- Python gate: 28 tests pass, including actual compiled exporter integration,
  incompatible/missing/unknown identities, provenance and exact byte checks.
- Stats Rust: 8 tests pass using an isolated runner with the same develop crate.
- Stats release manifest: 4 tests pass, including rejection before evaluation
  when the exporter declares a different contract.
- Six temporary mutations are detected: bypass normalization, ASCII-only split,
  insert LF separators, omit empty prediction, ignore serialization identity,
  and append terminal LF. All mutations were removed; exporter tests pass again.
- `cargo fmt --check`, Clippy for the changed example with `-D warnings`, and
  `git diff --check` pass.

An intermediate Python suite failed because its already-imported module still
pointed to the conformance fixture while that fixture was being relocated.
The final run against the settled implementation passes all 28 tests. This
intermediate result is not used as acceptance evidence.

## Historical and cross-exporter equivalence

With the exact published 5.1.3 crate, all **981** predictions match the published
5.1.3 prediction files byte for byte. All **994** files recorded in the historical
manifest were checked against their hashes and remain unchanged. The original
manifest hash is `98a1a50b1e497f368749213cd1d23f54b7547349c9dc1d154ba3e4026c4b3ba2`.

On integrated develop, the local exporter and the real stats CLI produce the
same **981** predictions and the same extraction failure. Their common tree
hash is `ca23487e62e2347ce0e1a7edd6d82dddf910435d8490db4debe6dcb83eab433f`.
The error remains `Invalid Filter type` for `jiaocaineedrop_chap10.pdf_8.md`;
its empty prediction is retained. A final writer-only harness checks the final
stats refactor without rerunning its independent approximate benchmark.

## Official local evaluation

Dataset `f5f559bddf50e36f7f9899d842d0006f13ce8afc`, evaluator
`337cc26965893db3ef53ddc119a6d6bb5bde096f`, `end2end_eval / quick_match`, OCR off.
There are 981 predictions, 921 scored text pages and 780 native pages.

| Metric | Integrated develop |
| --- | ---: |
| Official text edit distance | 0.47311686306064565 |
| Official reading-order edit distance | 0.29228846821555693 |

The upstream matcher used its timeout fallback on
`newspaper_0b1bb8d03b4287eb95f67b68c2cf9f92_1.jpg`. This fallback and the extraction
error were retained. Scores describe integrated develop, not an improvement
attributed to serialization. No baseline, corpus or evaluator was modified.

Provenance was checked by content: the 141 modified dataset entries are
materialized LFS objects matching the revision's size and SHA-256. Its untracked
files are `DATASET_INFO` and `SHA256SUMS`. The evaluator's tracked sources match
its revision; its virtual environments are untracked. These are not described
as globally clean Git trees. The local code is also explicitly uncommitted.

## Artifacts and next step

[Machine-readable evidence](2026-09-24-issue-620-validation.json) records hashes,
configuration, populations, test counts and mutations. Full logs, prediction
trees, per-page text/order results, lockfiles and the historical equivalence
sidecar are under this worktree's `target/issue-620/`. The own-session diff of
pre-existing stats files is `target/issue-620/stats-session.diff`; their original
contents are retained in `target/issue-620/stats-before/`. The stats `Cargo.toml`
and `Cargo.lock` were not edited; dependency resolution used a separate
runner. All working artifacts stayed inside the worktree, not `/tmp`.

Next: review these changes, prepare separate commits for oxidizePdf and stats,
and integrate both sides. Keep #620 open until that coordinated integration;
legacy manifests remain immutable and require explicit equivalence evidence.
