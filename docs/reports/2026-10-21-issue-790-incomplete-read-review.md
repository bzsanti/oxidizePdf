# QR — #790 truncated HTTP response fallback

## Resumen ejecutivo

Issue: [#790](https://github.com/bzsanti/oxidizePdf/issues/790). Scope: scripts/ci_evidence.py and scripts/test_ci_evidence.py on developcaee58c8. No product Rust or dependency change. Actual main run39607544386 failed before tests when GitHub returned an incomplete response while listing evidence jobs. Python HTTPException is not covered by the previous OSError handler.

## Contratos y evidencia

An unavailable evidence lookup must select actual validation, never claim reuse or halt the job before tests. The regression uses the real http.client.HTTPResponse reader with Content-Length40339 and a two-byte body; it reaches production GitHub.get/json.load and raises IncompleteRead with40337bytes missing. RED:20tests,1error. GREEN:20tests pass; output exactly hit=false/artifact=receipt and summary requests execution. The fixture substitutes transport only; it does not substitute exception parsing or the production decision.

Manual review traced probe through find_evidence/GitHub.get and the text-contracts workflow lines47–70: hit=false executes deterministic contracts and preserves results. Only the evidence probe catches HTTPException. Release coverage/registry verification still fail closed on errors; no failed run is trusted and no release gate is bypassed. Existing expiry, fork, matrix completeness and identity tests remain passing.

## Hallazgos

1. **Incomplete HTTP response bypassed fallback**: scripts/ci_evidence.py:130 — IncompleteRead derives HTTPException rather than OSError. Capture HTTPException alongside existing transport/parse errors so the probe records hit=false and returns normally. Resolved; real parser RED/GREEN evidence retained.

No unresolved implementation finding identified. Remote CI on the corrected commit and final publication remain pending.

## Calidad de Tests (Kripteia)

Mandatory invocation reports100/0tests/0files: it does not discover these Python tests, so no quality coverage is inferred from the score. Manual wiring review and the20executed Python contracts provide the behavioral evidence. Removing HTTPException from the handler restores the demonstrated failure.

## Análisis de Seguridad (Kripteia Security)

Mandatory invocation: No security issues found. Limited to these two files. Manual inspection confirms token is not logged, partial response bytes are not treated as successful evidence, and error fallback does not affect release authorization.

## Métricas

- Files reviewed:2 source files plus unchanged workflow consumer.
- Findings:1, corrected.
- Tests:20PASS, discriminating RED retained; diff whitespace PASS.
- Product files modified:0.
- Pending: remote CI and release integration/publication.
