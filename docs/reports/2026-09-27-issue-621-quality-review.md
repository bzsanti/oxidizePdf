# Issue #621 — pre-PR quality review

## Resumen ejecutivo

Reviewed the new tagged projection module, its integration into incremental page
mutations, zero-offset reader diagnostics and 14 synthetic regressions against
develop f9132e1661b76bae319a19b7824bcf5b5c550332. No user document content was
used. The change preserves the source byte prefix, page/content object identities
and existing permission/publication boundaries. No public API or version change.
Review completed locally before opening the implementation PR.

## Contratos y evidencia

| Contract | Boundary and counterexample | Evidence |
|---|---|---|
| Split tagged pages | Existing-document split -> incremental mutations -> /K projection -> rebuilt ParentTree/IDTree. A cross-group section must not retain /Pg pointing at a removed page. | RED: valid 12-page source fails deletion. GREEN: groups 1–3, 4–6, 7–12 have 3/3/6 pages, exact content/order, correct IDs, tag counts and valid parent links. Dry-run and materialized reports match. |
| Derived-index recovery | Only exact-generation, in-use offset-zero ParentTree/IDTree root edges are recoverable; /K and page keys are authoritative. | Missing roots pass after rebuilding; a Metadata alias to the same bad object still fails. Wrong generation and nonzero corrupt index are rejected with object identity. |
| Accessibility preservation | Integer MCIDs, explicit MCR page refs, inherited page context and OBJR annotations retain their ownership. | Tagged validator accepts outputs; annotation ParentTree keys exist only in the part containing the annotation. Role/class/attribute dictionaries remain copied with retained nodes. |
| Malformed and adversarial input | Cycles, incorrect /P, missing marked content and sparse-index memory exhaustion must fail. | Four focused negative cases pass. Traversal depth/entry and total dense ParentTree allocation are bounded. Unsupported external-stream ownership fails explicitly. |
| Permission/source/atomicity | Existing DocMDP/encryption checks precede projection; all outputs validate and stage before destination replacement. | Certified malformed source is refused before repair; source stays byte-identical. A later invalid part publishes no earlier part. Existing mutation and split suites retain permission and staging checks. |
| Structural interoperability | Latest xref, streams and references must remain readable externally. | qpdf --check exits 0 on six generated parts. This is structural validation, not PDF/UA certification. |

Local validation: 14 new regressions; 37 existing operation/tagged tests (one
external CLI test remains ignored); 6791 library tests pass, 3 ignored. Formatting
and diff checks pass. All-target Clippy was run with warnings denied; final log
is recorded in the validation evidence. No benchmark baseline was updated.

## Hallazgos

No unresolved findings in the final scope. During implementation, bounded dense
ParentTree allocation and root-edge-specific recovery were added to avoid large
sparse allocations and accidentally exempting aliases outside the derived index.
Final projection writes use sorted object IDs for reproducible object order.
The source PDF from the issue is unavailable: behavior is demonstrated with
independently assembled synthetic fixtures, not asserted for that private file.

## Calidad de Tests (Kripteia)

Overall Score: 94 | Tests: 117 | Files: 3.
New integration file: 14 tests, score 89. Reader: 93/95; reorder: 10/97.
The two end-to-end wrappers score 65 because the analyzer does not follow their
shared verify_split helper. Manual tracing confirms real public split/planning,
content extraction, IDTree assertions, page order, prefix identity and tagged
validation. Initial RED contains four intended failures and one passing atomicity
control. Tests do not reuse the projection implementation to derive expectations.

## Análisis de Seguridad (Kripteia Security)

Required scan completed: "No security issues found." Manual review covers input
bounds, ownership, parse failures, permission ordering and filesystem staging.
No new unsafe/FFI, subprocess invocation, dependencies, telemetry or persistence
of user content. Existing publication uses source snapshots and stages all parts;
the patch does not change its rollback protocol. Recovered xref objects remain
visible in mutation reports, and recovery cannot excuse unrelated graph edges.

## Métricas

- Changed code/test files reviewed: 5; unchanged semantic publication and tagged
  validator consumers were also traced.
- Unresolved findings: 0. Remote CI and integration remain pending.
- Review tool snapshot preceded only the final sort of replacement IDs; that
  ordering change was inspected and the 14 cases were rerun afterward.
- Explicit limits: external-stream MCRs, direct structure roots and mixed
  deletion+clone/import are rejected; no unsupported structure is rasterized or
  silently discarded to make the operation succeed.

Companion JSON records source hashes, actual RED/GREEN logs, qpdf output and the
full Kripteia output. This review does not claim the fix is released or merged.
