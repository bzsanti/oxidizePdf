# QR — release validator process fixtures (#800)

## Resumen ejecutivo

Issue: [#800](https://github.com/bzsanti/oxidizePdf/issues/800). Scope: only the cfg(test) process-contract helper and new regression in oxidize-pdf-core/src/verification/validators.rs, on top of88b5ce3a. PR806 coverage job119282706297 failed with qpdf execution error Text file busy (ETXTBSY), despite its overall workflow success via continue-on-error. Integration was withheld.

## Contratos y evidencia

Every simulated validator case must independently exercise the production process invocation with its own script/status. The previous helper truncated the same executable path between cases. A deterministic regression keeps a writable handle to the previous case, then invokes the next case through production run_qpdf: old helper RED reproduces ETXTBSY; new helper GREEN returns the expected success, preserves the first case content and then its expected failure status after closing the writer.

Use a distinct NamedTempFile per case, set executable permission and convert into TempPath to close the writer before exec while retaining path cleanup. Existing qpdf/veraPDF argument, exit-code and report assertions remain intact. No retry, suppression or product-code change. All23validator tests pass. Exact scheduling/descriptor provenance in the remote failure is not retrospectively proven; the shared executable lifetime hazard is reproduced and removed. Remote coverage must still validate the final candidate.

## Hallazgos

1. **Mutable executable shared between cases**: oxidize-pdf-core/src/verification/validators.rs:646 — repeated truncation of the same executable couples successive process fixtures and permits ETXTBSY when an earlier handle remains writable. Resolved by independently owned files with writers closed before invocation. RED/GREEN and23validator contracts retained.

## Calidad de Tests (Kripteia)

Mandatory Rust scan:93overall,23tests,1file. Existing helper/tool-availability tests limit external-validator claims; this new regression invokes the actual process boundary with shell fixtures and preserves distinct exit semantics. It does not claim to validate qpdf/PDF-A conformance; the separate interoperability job remains mandatory.

## Análisis de Seguridad (Kripteia Security)

Mandatory scan: No security issues found. Manual review confirms changes stay under cfg(test), use unique temporary executable paths with0700permissions, close writable descriptors before exec, and clean up through TempPath ownership. Product error handling remains unchanged.

## Métricas

- Source files reviewed:1; production behavior changes:0.
- Findings:1 corrected; regression RED then23tests GREEN.
- Clippy lib+tests PASS; format and diff whitespace PASS.
- Pending: final remote CI/coverage, release integration/publication.
