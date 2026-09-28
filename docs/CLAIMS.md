# Public-claim inventory

This is the release-review record for the public capability claims in scope for
[#603](https://github.com/bzsanti/oxidizePdf/issues/603). It is deliberately
not a product roadmap, benchmark scorecard or a substitute for API
documentation.

| Claim | Public source | Checked at | Version / source revision | Verification evidence | Limit |
|---|---|---|---|---|---|
| Certificate-based recipient encryption is unsupported; password encryption and signature certificate verification are separate APIs. | [`README.md`](../README.md); [`recipient-encryption.md`](recipient-encryption.md); encryption rustdoc | 2026-09-28 | Unreleased #640 change based on develop `089a927` | `issue_640_public_key_containment_test.rs`; all eight `SecurityHandler` entry points, recipient operations and permission checks inspected. | No supported certificate-based document writer/reader. Raw dictionary helpers do not protect or validate data. Actual implementation is tracked in #642; advanced PAdES remains outside the MIT-core scope. |
| PDF/A conformance validation is available for eight levels. | [`README.md`](../README.md) (start-here and PDF processing); [`IDEAL_USE_CASES.md`](IDEAL_USE_CASES.md) | 2026-09-19 | `5.1.3` / `v5.1.3` | Public `PdfAValidator` export in `oxidize-pdf-core/src/lib.rs`; validator implementation and unit tests in `oxidize-pdf-core/src/pdfa/validator.rs`. | No claim of PDF/A authoring, remediation or certification. |
| Signature detection, PKCS#7 verification and certificate validation are available; an incremental-signature preparation API is public. | [`README.md`](../README.md) (PDF processing); [`IDEAL_USE_CASES.md`](IDEAL_USE_CASES.md) | 2026-09-19 | `5.1.3` / `v5.1.3` | Public signature exports in `oxidize-pdf-core/src/signatures/mod.rs`, including verification, certificate validation and `prepare_incremental_signature`. | No managed keys, certificate issuance, hosted signing service or legal/compliance assurance. The caller provides the external signer. |
| oxidize-pdf is a library, not a managed enterprise document service. | [`IDEAL_USE_CASES.md`](IDEAL_USE_CASES.md) | 2026-09-19 | `5.1.3` / `v5.1.3` | The library surface above provides APIs only; no service, support or managed-workflow component is present in this repository. | No phone support, PDF/UA, archival certification or managed workflows. |
| Comparative performance is not a supported public claim. | [`PERFORMANCE_HONEST_REPORT.md`](PERFORMANCE_HONEST_REPORT.md) | 2026-09-19 | `5.1.3` / `v5.1.3` | The report records that no comparison with other Rust libraries was executed. | Historical figures are not comparative evidence unless a versioned protocol and artefacts are linked. |

Release review must update the affected row before publishing a capability
change: re-check the public source and code evidence, set the release version
and commit, and state the relevant limitation. A claim without all six fields
must be removed or marked unsupported.
