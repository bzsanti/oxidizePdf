# Adoption-monitoring boundary and contract

Issue: [#603](https://github.com/bzsanti/oxidizePdf/issues/603)

Initial segment: Rust backend teams building AI/RAG ingestion pipelines.
Status: no adoption baseline or calibrated adoption probability exists yet.

`oxidize-pdf` does not collect, persist or transmit adoption telemetry. The
storage, dashboard, consent handling, retention enforcement, redaction and
decision engine belong to `oxidize-stats`. This document is the interface that
any such implementation must meet; it does not authorize this crate to retain
PDFs, feedback or identifiers.

## Auditable event envelope

Each accepted adoption observation needs an append-only audit record and all of
these fields:

```json
{
  "event_id": "uuid",
  "source": "public-repository|crates.io|consented-survey",
  "segment": "rust-backend-rag",
  "version": "5.1.1",
  "occurred_at": "2026-09-19T00:00:00Z",
  "pseudonymous_id": "rotating-hmac-or-equivalent",
  "stage": "quickstart|prototype|production|upgrade",
  "outcome": "defined-by-experiment",
  "denominator_id": "documented-cohort-or-null",
  "evidence_ref": "stable-public-url-or-consent-record",
  "event_hash": "sha256-of-canonical-event-without-event_hash",
  "previous_event_hash": "sha256-of-previous-record-or-null-for-genesis",
  "schema_version": 1
}
```

`source`, `segment`, `version`, `occurred_at`, `pseudonymous_id` and
`denominator_id` are required even when a denominator is not applicable: use
the explicit value `null` with a reason recorded in the experiment. The
pseudonymous identifier must be generated and rotated by `oxidize-stats`; this
repository must never receive its secret or a reversible identity.

`oxidize-stats` must persist this as an append-only log. An event cannot be
updated or deleted in place: a correction is a new record that references the
prior record. `event_hash` is computed over a documented canonical encoding;
`previous_event_hash` links each record in its partition, so an auditor can
detect substitution, reordering or deletion. Only the collection service may
append records, and its tests must reject an altered hash, an invalid previous
hash and an in-place update or deletion.

## Privacy and retention rules

- Public evidence is stored only as its stable URL and verification timestamp.
- Raw feedback or PDF content is rejected unless explicit, recorded consent
  identifies its purpose, retention duration and redaction procedure.
- Before any semantic evaluation, feedback is redacted, bounded and treated as
  untrusted data. Raw PDFs and raw feedback are not inputs to the evaluator.
- Retention expiry deletes or irreversibly redacts the retained content while
  keeping the minimal audit record: event id, consent record reference,
  deletion timestamp and policy version.

## Deterministic decision policy

Semantic outputs may classify evidence, estimate risk or suggest an action.
They are correlated assessments of the same evidence, never independent votes.
Only deterministic code may decide the following:

1. Missing source, segment, period, denominator explanation, calibration or
   confidence below `0.70` produces `request_more_data`; it cannot block or
   promote a release.
2. A calibrated trust or compatibility risk at or above `0.60` produces
   `human_triage`, with the reviewer and outcome recorded. It is not an
   automatic promotion block.
3. A documentation recommendation creates a review task only. It cannot edit,
   publish or close an issue automatically.
4. All other complete evidence remains `monitor` until a human records a
   deterministic decision and its rationale.

The audit log must retain the normalized input references, policy version,
model/prompt version when a semantic step is used, semantic output, final
deterministic result and human decision where applicable.

## Experiment record

Before collecting a new cohort, `oxidize-stats` must record an experiment with
an owner, due date, target segment, sampling rule and denominator, success
criterion, failure criterion, privacy/retention policy, next action and status.
No dashboard target or probability may be calibrated until this record has
observed outcomes sufficient for the approved temporal validation protocol.
