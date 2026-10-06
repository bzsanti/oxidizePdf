TextExtractor silently drops incomplete multibyte tails after valid ToUnicode text. A standalone malformed tail takes a different fallback. Reproduction from #666 uses complete 1–4 byte codespaces and public PdfReader/TextExtractor: complete sequence yields ABCD; A + partial two/three/four-byte code yields A in both parser modes. A complete unmapped code after A is also discarded.

Fix acceptance:
- Preserve valid text and expose undecodable codes with U+FFFD, once per incomplete tail or complete unmapped code.
- Respect source-code boundaries; do not reinterpret suffix bytes of an unmapped code or combine separate Tj/TJ strings.
- Cover ToUnicode, Identity CID collection and Encoding CMap paths, preserving intentional CID0 behavior and existing malformed-simple-font compatibility.
- Keep best-effort extraction policy in both parser modes; renderer truncation rejection remains unchanged.
- Public regressions, library/contracts and differential corpus validation without baseline recalibration.

Reproduction: docs/reports/2026-10-04-issue-666-continuation-evidence/truncation-probe.rs and .log in current #666 worktree.
