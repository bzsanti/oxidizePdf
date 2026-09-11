# Issue #568 — reproducible OmniDocBench gate (TDD plan)

This checklist is the implementation trace. A checked item has a corresponding
automated test or an explicitly recorded external validation requirement.

## Red: specify observable behavior

- [x] Reject duplicate JSON object keys instead of silently overwriting scores.
- [x] Reject duplicate dataset page names and unknown, missing, non-finite, or
  out-of-range page scores.
- [x] Keep official-global and native-text populations separate, with explicit
  included, excluded, missing, duplicate, and failed counts.
- [x] Version the `note` / `fuzzy_scan` native-text exclusion predicate.
- [x] Canonicalize JSON before hashing so input ordering cannot change hashes.
- [x] Reject baseline/candidate comparisons when their protocol identity differs.
- [x] Refuse clean-commit provenance for a dirty worktree unless dirty provenance
  is explicitly requested and includes a diff hash.
- [x] Resolve OmniDocBench image names to source PDFs and page indices
  deterministically.

## Green: minimal implementation

- [x] Add a repository-owned Rust prediction exporter using the public native-text
  extraction API and a fixed, serialized extraction configuration.
- [x] Add a Python gate that validates official per-page text scores, emits a
  versioned summary, and compares compatible baseline/candidate summaries.
- [x] Record Git SHA, worktree state, extraction configuration, dataset and
  evaluator revisions, protocol, OCR state, population definition, and artifact
  hashes.
- [x] Add one documented command for prediction export and summary generation
  from externally supplied PDFs, dataset metadata, and official score artifact.
- [x] Make failures and empty predictions explicit without introducing OCR.
- [x] Build from a temporary `git archive` with the evaluated commit's own
  locked dependencies; no auxiliary relative path dependency is used.
- [x] Verify dataset/evaluator Git revisions and record the Rust/Cargo toolchain.
- [x] Re-hash predictions during summary and reject tampered or dirty summaries.
- [x] Serialize effective extraction configuration from Rust, group pages by
  source PDF, and publish the prediction tree atomically.

## Refactor and verification

- [x] Run the focused Python suite and Rust exporter tests (17 gate tests and
  2 focused Rust exporter tests; native-reading-order regressions run separately).
- [x] Run formatting, clippy for the exporter, and `git diff --check`.
- [x] Verify two synthetic repeated runs produce identical prediction hashes.
- [x] Feed the existing pinned official `d5e2d0b` score artifact through the gate:
  921 global pages at `0.5085693812417305` edit distance and 780 native pages
  at `0.41990107039261854`.
- [ ] External: run the pinned OmniDocBench dataset/evaluator and confirm commit
  `d5e2d0ba5073f28ef56cf13a32871236cff6e11f` reports global edit distance
  approximately `0.50856938` (similarity `49.1431%`) within `1e-8`.
- [ ] External: create the v5.0.0 baseline with this same gate/schema and retain
  both immutable summary artifacts for future comparisons.

The two external items deliberately remain unchecked until the non-redistributable
dataset and pinned evaluator are supplied. Synthetic tests do not claim benchmark
acceptance.
