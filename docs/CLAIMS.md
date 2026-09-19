# Public-claim inventory

This is the release-review record for the public capability claims in scope for
[#603](https://github.com/bzsanti/oxidizePdf/issues/603). It is deliberately
not a product roadmap, benchmark scorecard or a substitute for API
documentation.

| Claim | Public source | Checked at | Version / source revision | Verification evidence | Limit |
|---|---|---|---|---|---|
| PDF/A conformance validation is available for eight levels. | [`README.md`](../README.md) (start-here and PDF processing); [`IDEAL_USE_CASES.md`](IDEAL_USE_CASES.md) | 2026-09-19 | `5.1.1` / `4a5d850ab135` | Public `PdfAValidator` export in `oxidize-pdf-core/src/lib.rs`; validator implementation and unit tests in `oxidize-pdf-core/src/pdfa/validator.rs`. | No claim of PDF/A authoring, remediation or certification. |
| Signature detection, PKCS#7 verification and certificate validation are available; an incremental-signature preparation API is public. | [`README.md`](../README.md) (PDF processing); [`IDEAL_USE_CASES.md`](IDEAL_USE_CASES.md) | 2026-09-19 | `5.1.1` / `4a5d850ab135` | Public signature exports in `oxidize-pdf-core/src/signatures/mod.rs`, including verification, certificate validation and `prepare_incremental_signature`. | No managed keys, certificate issuance, hosted signing service or legal/compliance assurance. The caller provides the external signer. |
| oxidize-pdf is a library, not a managed enterprise document service. | [`IDEAL_USE_CASES.md`](IDEAL_USE_CASES.md) | 2026-09-19 | `5.1.1` / `4a5d850ab135` | The library surface above provides APIs only; no service, support or managed-workflow component is present in this repository. | No phone support, PDF/UA, archival certification or managed workflows. |
| Comparative performance is not a supported public claim. | [`PERFORMANCE_HONEST_REPORT.md`](PERFORMANCE_HONEST_REPORT.md) | 2026-09-19 | `5.1.1` / `4a5d850ab135` | The report records that no comparison with other Rust libraries was executed. | Historical figures are not comparative evidence unless a versioned protocol and artefacts are linked. |

Release review must update the affected row before publishing a capability
change: re-check the public source and code evidence, set the release version
and commit, and state the relevant limitation. A claim without all six fields
must be removed or marked unsupported.
