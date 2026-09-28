# Adoption claims and monitoring acceptance — #603

Documentation correction checked against 5.1.5 / fb4042fd on 2026-09-28.
The inventory links consumer evidence and separately identifies implemented,
external, extension, unsupported and false-success operations. Unsupported
comparative tables are replaced by a separately measured, bounded five-engine
generation comparison; unsupported ROI assertions are removed. See the
[performance report](2026-09-28-generation-performance.md). Basic signature scope and
the external CMS contract are now consistent across public entry points.

## Monitoring evidence (2026-09-27; not a new run)

oxidize-stats records 61 passing Rust tests, 15 JavaScript checks and successful
Clippy/format validation after correcting WAL sidecar lifetime. Its isolated
live acceptance ran the same image later deployed to both production services:
`sha256:f0f76085dd4727101cf480aa7fa6cac6c79dc4f090e9b3bb1b559ec39960224e`.
The original synthetic [machine-readable acceptance](2026-09-27-monitoring-e2e.json)
is preserved here; its temporary local paths are historical artifact locations.

| Acceptance criterion | Evidence |
|---|---|
| Auditable adoption events | Collector workflow stores source, segment, version, timestamp, pseudonym and denominator; hash chain verified after seven acceptance records. Mutation, deletion and tail-truncation regressions are recorded in oxidize-stats/tests/monitoring.rs. |
| Incomplete evidence cannot block promotion | Uncalibrated high probability returns request_more_data; API automatic_promotion_block=false. |
| Privacy and retention | No raw PDF/text accepted; subject-bound consent and categorical feedback. Read-only API omits private identifiers; actual scheduler expiry deletes feedback and appends an erasure event. |
| Correlated semantic input, deterministic decision | One normalized semantic input; deterministic assessment with recorded human review. |
| Complete experiment protocol | Owner, due date, sample/denominator, success/failure criteria and next action visible in the acceptance API record. |
| Dashboard and append authority | Real browser renders workflow without JS errors; API rejects writes; web data mount read-only. |

Source implementation and full local evidence remain owned by oxidize-stats:
`docs/2026-09-27-issue-603-validation.md`, `src/monitoring.rs`,
`src/monitoring.sql`, `src/premortem.rs` and `tests/live-monitoring.cjs`.
No implementation code or private production data is copied into this repository.

The recorded production state contains no experiments/observations/calibrations.
Synthetic acceptance records were never inserted there. A real cohort and baseline
remain product operations; collection is required **before calibration**, not
invented as a condition of this implementation closure. No calibrated adoption
probability is claimed.

## Independent follow-ups

#637/#638 filter errors, #639 writer fingerprints, #640 recipient-encryption
containment and #642 actual recipient encryption retain their own acceptance
criteria. Documenting their limitations does not fix or close them. #641's basic
signature documentation is addressed alongside #603; PAdES is left as an
extension for anyone who wants to implement it. No PAdES implementation is added. Publishing this source documentation does
not rewrite the immutable 5.1.5 package or its historical docs.rs pages.

## Publication and deployment — 2026-09-28

PR #644 targets develop and preserves its recent fixes, including #640 via #643.
The release audit and performance measurements still identify published 5.1.5;
merging documentation into develop does not publish a new crate or update main.
The reproduction archive is included alongside the measurement report.

oxidize-stats was backed up before migration/import. Deployment exposed both
historical and five-engine runs through /api/performance (8 and 10 rows).
Both Docker services are healthy. The audit chain remains at 11 records with
head a3e3ff3161eb8d2126fa697036c5ede19e7306588cafcf0e03b84c1c61b7b51c.
An upgrade-path defect (v15 early return skipping v16) was corrected before
activation, with a data-preservation/idempotency regression test; all 71 Rust
service tests and Clippy pass. No synthetic adoption records were added.
