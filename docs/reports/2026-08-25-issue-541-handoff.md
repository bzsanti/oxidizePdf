# Issue #541 secure redaction handoff

Date: 2026-08-25

## Current state

- Branch: `feat/issue-541-secure-redaction`
- Published commit: `3436a09 fix(redaction): distinguish visual masking from secure removal`
- Remote branch: `origin/feat/issue-541-secure-redaction`
- Pull request: not opened; issue #541 is not complete.
- No additional worktree was created.

The first safety milestone is complete. The existing redactor is now explicitly described and reported as visual masking, with residual recoverability risks. A separate `Irreversible` mode and `redact_irreversible` entry point were added, but that entry point deliberately fails closed with `SecureRedactionUnsupported`; it never returns visually masked bytes as if they were securely sanitized.

## Changes in the published commit

- Added `RedactionMode::{VisualMask, Irreversible}`.
- Added mode and residual-risk information to `RedactionReport`, with `mode()`, `residual_risks()`, and `is_irreversible()` accessors.
- Corrected the legacy `SemanticRedactor::redact` documentation and changelog claim.
- Added the fail-closed irreversible API and regression tests proving that visual masking cannot claim irreversible removal.

Changed files:

- `CHANGELOG.md`
- `oxidize-pdf-core/src/operations/mod.rs`
- `oxidize-pdf-core/src/operations/semantic_redactor.rs`
- `oxidize-pdf-core/tests/semantic_redactor_test.rs`

## Validation completed

- `cargo test -p oxidize-pdf --test semantic_redactor_test`: 16 passed, 0 failed.
- `cargo clippy -p oxidize-pdf --lib --test semantic_redactor_test -- -D warnings`: passed.
- Commit hook: formatting, Clippy, library build, and library tests passed; library tests reported 6707 passed, 0 failed, 3 ignored.

## Remaining work for #541

The actual irreversible engine is not implemented. It must remove targeted content rather than cover it, and it must not claim success while recoverable copies remain. Required areas include:

- Rewrite page content streams for matched text, image, and vector content.
- Handle or reject shared/nested XObjects, transformations, clipping, transparency, optional content, forms, and appearance streams.
- Sanitize annotations, form values, document metadata/XMP, attachments, and embedded files according to an explicit policy.
- Rebuild output rather than append an incremental revision so earlier revisions do not retain sensitive data.
- Produce a machine-readable record of removed objects and residual risks.
- Add forensic fixtures verifying that search, text extraction, copy operations, and object inspection cannot recover the target.
- Fail closed on every unsupported construct and leave the source untouched; use atomic publication if a path-writing API is introduced.

## Recommended next implementation slice

Implement a narrow, real secure path for simple page-direct text first:

1. Accept exact, non-empty `SemanticEntity.content` matches constrained by page and bounding box.
2. Parse and rewrite supported page content operators, then replace the page content through the crate-internal page API.
3. Rebuild the PDF output and reject documents containing relevant unsupported storage or rendering paths.
4. Add adversarial and object-level tests before expanding support to XObjects, images, vectors, forms, metadata, and attachments.

Useful code observations:

- `ContentParser` parses operators, but no complete content-stream serializer was identified yet.
- `Page::from_parsed_with_content` concatenates page streams into private content.
- `Page::set_content` is crate-visible and can be used by the semantic redactor module to install rewritten content.

## Session cleanup

- `cargo sweep --dry-run --time 5 /home/santi/repos/BelowZero/oxidizePdf/oxidize-pdf` identified only the repository's `target/` directory.
- `cargo sweep --time 5 /home/santi/repos/BelowZero/oxidizePdf/oxidize-pdf` removed 7.55 GiB of build artifacts older than five days.
- No other clearly project-owned temporary directories were identified for deletion.
