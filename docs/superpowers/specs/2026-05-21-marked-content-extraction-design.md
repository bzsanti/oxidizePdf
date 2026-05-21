# Marked-Content Extraction for Tagged PDFs (Phase 1)

**Date:** 2026-05-21
**Branch:** `fix/issue-269-marked-content-extraction`
**Closes:** [#269](https://github.com/bzsanti/oxidizePdf/issues/269)
**Scope:** Phase 1 of the Tagged-PDF initiative. Subsequent phases (struct-tree reading, role-tag classification, writer ParentTree, PDF/UA) are out of scope here.

## Context

`text/extraction.rs` currently ignores marked-content operators (`BDC`/`BMC`/`EMC`) even though the parser already emits them as `ContentOperation::BeginMarkedContent{,WithProps}` and `ContentOperation::EndMarkedContent`. As a result, two visually-overlaid text runs inside separate `BDC..EMC` blocks at the same baseline get merged by `merge_into_lines` and produce character-interleaved chunks ("alphabet soup" on NCSC Cyber Assessment Framework v4.0 page 12, e.g. `"Tahre mere iansag neod s efysftecemtaitivecl yp.roc ess"`).

The fix is to consume marked-content semantics in the extractor:

1. Track the marked-content stack per page.
2. Tag each `TextFragment` with the innermost ancestor's `MCID` and structural tag.
3. Honor `/ActualText` overrides.
4. Filter `/Artifact` content by default (page headers/footers, watermarks, decorative content).
5. Use `MCID` as part of the line-grouping key so two overlaid blocks at the same baseline stay on distinct logical lines.

The writer side of Tagged PDFs (v1.4.0, October 2025) already produces these operators correctly; this work is purely on the read/extract side.

## Goals

- Close #269.
- Eliminate the NCSC alphabet-soup regression and equivalent failures on government/compliance PDFs (a corpus of strategic importance to the Tessera funnel).
- Improve RAG corpus quality by honoring `/ActualText` (UTF-16BE-decoded substitutions for ligatures, math, decorative glyphs).
- Filter page-furniture noise (`/Artifact`) from RAG output by default.
- Establish the data structures (`mcid`, `struct_tag`) on `TextFragment` that Phase 3 (role-tag classification) will consume.

## Non-Goals (Phase 1)

- Parsing `StructTreeRoot`, `ParentTree`, or the document-level structure tree from the catalog. (Phase 2.)
- Using `struct_tag` for partitioner classification (Title vs Header vs Paragraph vs Figure vs Table). (Phase 3.)
- Handling `/Alt` text for images. (Phase 3.)
- Per-element `/Lang` propagation. (Phase 3 or 5.)
- PDF/UA-1 validation. (Phase 5.)
- Writer-side ParentTree multi-page completeness or automatic MCID assignment. (Phase 4.)

## Success Criteria

All of the following must hold and be verified by content-checking tests (no smoke tests, per `CLAUDE.md`):

1. **Synthetic overlay.** A PDF with two `BDC..EMC` text blocks at identical `Y` produces two distinct lines in `merge_into_lines`, not one interleaved line.
2. **ActualText override.** `<</ActualText (fi)>> BDC (xy) Tj EMC` extracts as `"fi"`, not `"xy"`.
3. **UTF-16BE ActualText.** `<</ActualText <FEFF00660069>> BDC (junk) Tj EMC` extracts as `"fi"`.
4. **Artifact filter default.** `/Artifact BDC (page 12) Tj EMC` produces zero fragments under default `ExtractionOptions`.
5. **Artifact opt-in.** Same input with `include_artifacts = true` produces one fragment containing `"page 12"`.
6. **NCSC alphabet-soup.** Extracting NCSC CAF v4.0 page 12 produces no chunk containing the substrings `"Tahre"`, `"iansag"`, or `"efysftecemtaitivecl"`. At least one chunk contains coherent English (`"There"`, `"systems"`, `"Security"`).
7. **Real-world regression.** `examples/rag_realworld.rs` continues to process 5/5 PDFs (BSI, Higgs, ENS, BOE, NCSC) with chunk counts within established bands. No documents regress to per-glyph fragmentation or alphabet-soup output.
8. **Non-tagged PDFs unchanged.** Existing tests on non-tagged PDFs (the bulk of the corpus) produce identical output. `mcid = None` for every fragment means legacy behavior is preserved.
9. **Test suite.** `cargo test --workspace` passes. No existing test deleted, weakened, or marked `#[ignore]` to make this pass.

## Architecture

```
content stream bytes
        |
        v
parser/content.rs
    ContentOperation::{BeginMarkedContent, BeginMarkedContentWithProps, EndMarkedContent, ...}
    Change: properties carrier becomes enum-typed (see §3) so UTF-16BE strings, hex strings, and resource refs survive parsing.
        |
        v
text/extraction.rs
    TextState gains mc_stack: Vec<MarkedContentEntry>
        push on BDC/BMC, pop on EMC, defensive on unbalanced input.

    Per-fragment emission:
        if any ancestor in stack is /Artifact and !options.include_artifacts -> skip emission
        mcid       = innermost ancestor.mcid that is Some
        struct_tag = tag of that owning BDC (or None if none in stack)
        text       = innermost ancestor.actual_text if Some, else decoded glyphs

    TextFragment gains: mcid: Option<u32>, struct_tag: Option<String>

    merge_into_lines:        group key = (Y_bucket, mcid)
    merge_close_fragments:   predicate adds a.mcid == b.mcid
    merge_into_paragraphs:   predicate adds a.mcid == b.mcid
```

For PDFs that are not tagged, the `mc_stack` stays empty, `mcid` and `struct_tag` are `None` for every fragment, and the grouping key `(Y_bucket, None)` reduces to grouping by `Y_bucket` alone (legacy behavior).

## Detailed Changes

### 3. Parser API (breaking, internal-only)

Current shape:

```rust
BeginMarkedContentWithProps(String, HashMap<String, String>)
DefineMarkedContentPointWithProps(String, HashMap<String, String>)
```

`HashMap<String, String>` is lossy for the property values we care about:

- `/ActualText` is almost always a UTF-16BE PDF string with a `\xFE\xFF` BOM; lossy UTF-8 conversion of the raw bytes destroys it.
- Hex strings `<FEFF00660069>` aren't even reached by the current code path for string values.
- The dict-or-name case stores a resource reference as `__resource_ref` magic key, which is fragile.

Replace with:

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum MarkedContentValue {
    String(Vec<u8>),                                // raw PDF string bytes; decode lazily
    Integer(i64),
    Real(f64),
    Name(String),
    Array(Vec<MarkedContentValue>),
    Dict(HashMap<String, MarkedContentValue>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum MarkedContentProps {
    Inline(HashMap<String, MarkedContentValue>),
    ResourceRef(String),                            // resolve against page /Resources /Properties
}

ContentOperation::BeginMarkedContentWithProps(String, MarkedContentProps)
ContentOperation::DefineMarkedContentPointWithProps(String, MarkedContentProps)
```

`pop_dict_or_name` is rewritten to preserve token types (String bytes, Integer, Real, Name, nested Arrays/Dicts). Resource-ref resolution happens in the extractor by looking up `page.resources/Properties/<name>`; out of scope for this phase if the dict cannot be resolved, fall back to treating it as `Inline(HashMap::new())` and log a warning.

This is a breaking change to the `ContentOperation` enum, but `ContentOperation` is an internal API. A grep before implementation enumerates all call sites; expected to be confined to `oxidize-pdf-core/src/` and possibly `oxidize-pdf-core/examples/`. Update each site.

### 3a. PDF string decoding helper

In `text/extraction.rs` (or a small helper module), add:

```rust
fn decode_pdf_string(bytes: &[u8]) -> String {
    if bytes.len() >= 2 && bytes[0] == 0xFE && bytes[1] == 0xFF {
        // UTF-16BE with BOM
        let mut chars = Vec::new();
        let mut i = 2;
        while i + 1 < bytes.len() {
            let cu = u16::from_be_bytes([bytes[i], bytes[i + 1]]);
            chars.push(cu);
            i += 2;
        }
        String::from_utf16_lossy(&chars)
    } else {
        // PDFDocEncoding: close enough to Latin-1 for the ASCII subset used in ActualText
        // Reuse existing PDFDocEncoding -> String helper from parser/encoding.rs if present.
        bytes.iter().map(|&b| b as char).collect()
    }
}
```

If `parser/encoding.rs` already provides a richer PDFDocEncoding mapper, reuse it. Verify before duplicating.

### 4. Extractor state and emission

```rust
struct MarkedContentEntry {
    tag: String,                  // e.g. "P", "H1", "Artifact", "Span"
    mcid: Option<u32>,            // from /MCID property if present
    actual_text: Option<String>,  // decoded from /ActualText property if present
    is_artifact: bool,            // tag == "Artifact" || any ancestor.is_artifact
}

struct TextState {
    // ... existing fields ...
    mc_stack: Vec<MarkedContentEntry>,
}
```

`mc_stack` is initialized empty per page.

**BDC/BMC handling:**

```rust
ContentOperation::BeginMarkedContent(tag) => {
    let parent_artifact = state.mc_stack.last().map_or(false, |e| e.is_artifact);
    state.mc_stack.push(MarkedContentEntry {
        is_artifact: tag == "Artifact" || parent_artifact,
        tag, mcid: None, actual_text: None,
    });
}

ContentOperation::BeginMarkedContentWithProps(tag, props) => {
    let parent_artifact = state.mc_stack.last().map_or(false, |e| e.is_artifact);
    let (mcid, actual_text) = resolve_props(&props, page_resources);
    state.mc_stack.push(MarkedContentEntry {
        is_artifact: tag == "Artifact" || parent_artifact,
        tag, mcid, actual_text,
    });
}

ContentOperation::EndMarkedContent => {
    if state.mc_stack.pop().is_none() {
        // log warning, do not panic
    }
}
```

`resolve_props` walks the `MarkedContentProps`, extracts `MCID` as `Integer` (cast to `u32`, `None` on out-of-range), extracts `ActualText` via `decode_pdf_string`. For `ResourceRef(name)`, look up `page.resources.get_dict("Properties")?.get(name)`; if found, treat as inline; if not, return `(None, None)` and log warning.

**Fragment emission:**

```rust
fn emit_text_fragment(...) {
    // Skip if inside Artifact and not opted in
    if state.mc_stack.iter().any(|e| e.is_artifact) && !options.include_artifacts {
        return;
    }

    // Resolve mcid + struct_tag from innermost ancestor with mcid
    let (mcid, struct_tag) = state.mc_stack.iter().rev()
        .find(|e| e.mcid.is_some())
        .map_or((None, None), |e| (e.mcid, Some(e.tag.clone())));

    // ActualText override: innermost ancestor with actual_text wins
    let text = state.mc_stack.iter().rev()
        .find_map(|e| e.actual_text.clone())
        .unwrap_or_else(|| decoded_glyphs);

    fragments.push(TextFragment {
        text, x, y, width, height, font_size, font_name, is_bold, is_italic,
        color, space_decisions,
        mcid, struct_tag,
    });
}
```

**ActualText collapses a run.** When an ActualText span wraps multiple `Tj` calls, the substitution applies to the entire run, not each `Tj`. Implementation: when an ActualText ancestor is in scope, suppress per-`Tj` fragment emission; on EMC of that scope (or when the ActualText scope is exited), emit one synthetic fragment with `text = actual_text`, `x = first_tj.x`, `y = first_tj.y`, `width = sum_of_tj_widths`, `font_size = first_tj.font_size`.

This means tracking "pending ActualText run" state. Simplest implementation: a `Option<PendingActualTextRun>` field on `TextState`, populated when entering a BDC with `actual_text`, drained and emitted on the matching EMC.

Edge case: nested ActualText (inner overrides outer). The innermost ActualText scope wins; the outer ActualText is not emitted because its contents (the inner scope) have already been replaced.

### 5. Merge invariants

```rust
fn merge_into_lines(&self, fragments: &[TextFragment]) -> Vec<TextFragment> {
    // sort by (Y descending, X ascending)
    // group by (Y_bucket, mcid)   <-- key change
}

fn merge_close_fragments(...) {
    // existing close-fragment predicate AND a.mcid == b.mcid
}

fn merge_into_paragraphs(...) {
    // existing paragraph predicate AND a.mcid == b.mcid
}
```

`None == None` is treated as equal, preserving legacy behavior for non-tagged PDFs.

### 6. Public API additions

```rust
pub struct TextFragment {
    // ... 11 existing fields ...
    pub mcid: Option<u32>,
    pub struct_tag: Option<String>,
}

pub struct ExtractionOptions {
    // ... 9 existing fields ...
    pub include_artifacts: bool,  // default false
}
```

`ExtractionOptions::default()` adds `include_artifacts: false`.

Backward compatibility: existing constructors and serializations of `TextFragment` need to populate `mcid: None`, `struct_tag: None`. All non-test call sites that build `TextFragment` literally must be updated.

## Testing Strategy

All tests verify content. No `is_ok()`, no byte-count checks, no "doesn't panic". Per `CLAUDE.md` (oxidize-pdf): tests for bug fixes use TDD-strict — reproducer first, fix second.

| # | Level | File | Verifies |
|---|---|---|---|
| 1 | Parser unit | `oxidize-pdf-core/tests/marked_content_props_test.rs::utf16be_actualtext_preserved` | `BDC <</ActualText <FEFF00660069>>` produces `MarkedContentValue::String([0xFE, 0xFF, 0, 'f', 0, 'i'])` |
| 2 | Parser unit | same | `BDC /PropsName` produces `MarkedContentProps::ResourceRef("PropsName")` |
| 3 | Parser unit | same | `BDC <</MCID 0>>` produces `MarkedContentValue::Integer(0)` for MCID key |
| 4 | Extract unit | `oxidize-pdf-core/tests/extraction_mcid_test.rs::overlaid_baselines_distinct_lines` | Two `BDC..EMC` synthetic content streams at same Y → `merge_into_lines` returns 2 lines |
| 5 | Extract unit | same | Nested BDCs: `/P <</MCID 0>> BDC /Span BDC (x) Tj EMC EMC` → fragment.mcid = 0, struct_tag = "P" |
| 6 | Extract unit | `oxidize-pdf-core/tests/extraction_actualtext_test.rs::literal_string_overrides_glyphs` | `/Span <</ActualText (fi)>> BDC (xy) Tj EMC` → fragment.text = "fi" |
| 7 | Extract unit | same | `/Span <</ActualText <FEFF00660069>>` → fragment.text = "fi" (UTF-16BE decoded) |
| 8 | Extract unit | same | ActualText spanning two `Tj` calls collapses to one fragment with substituted text |
| 9 | Extract unit | `oxidize-pdf-core/tests/extraction_artifact_test.rs::filtered_by_default` | `/Artifact BDC (page 12) Tj EMC` → fragments is empty |
| 10 | Extract unit | same | Same with `include_artifacts = true` → one fragment "page 12" |
| 11 | Extract unit | same | Nested: `/Artifact BDC /P BDC (x) Tj EMC EMC` → still filtered (inherited from ancestor) |
| 12 | Defensive | `oxidize-pdf-core/tests/extraction_unbalanced_bdc_test.rs::extra_emc_no_panic` | Content with `EMC EMC EMC` and no matching BDC does not panic; extractor recovers |
| 13 | Defensive | same | Content with `BDC BDC` and no matching EMC does not panic; on end-of-stream stack is silently flushed |
| 14 | Integration | `oxidize-pdf-core/tests/marked_content_roundtrip_test.rs::writer_to_extractor` | Use existing v1.4.0 writer to produce a PDF with two `BDC..EMC` paragraphs at same baseline → extract → 2 distinct lines, each with correct mcid |
| 15 | Real corpus | `oxidize-pdf-core/tests/ncsc_no_alphabet_soup_test.rs` (uses `corpus_cache/e0e3ff11371c09c2.pdf` already present) | Extract NCSC CAF v4.0 page 12, partition with default options. Assert: no chunk contains `"Tahre"`, `"iansag"`, or `"efysftecemtaitivecl"`. Assert: at least one chunk contains `"There"` or `"systems"`. |
| 16 | Regression | `examples/rag_realworld.rs` (manual run, captured in PR body) | 5/5 PDFs processed; chunk counts logged before/after, no doc balloons or collapses |

The NCSC test requires the corpus file to be present. If absent on CI, gate behind `#[ignore]` with a clear message that the file is provided by the `rag_realworld` example. Local runs must include it; CI runs that include it pin via a checksum.

## Risks and Mitigations

**Breaking change to `ContentOperation::BeginMarkedContentWithProps`.** Mitigation: grep all call sites before changing the type; expected to be `<10` internal sites. Update each in the same PR.

**Unbalanced BDC/EMC in real-world PDFs.** Some PDFs in the wild have unbalanced operators. Mitigation: defensive push/pop with warning logs (no panic), tests #12 and #13.

**ActualText runs that span complex content** (multiple `Tj`, `TJ`, `'`, `"`, with positioning ops in between). Mitigation: ActualText collapses positioning to the run's bounding box. The first text-show op's position is used; widths sum. Tests #6, #7, #8 cover the common cases. If a real PDF emerges that hits a pathological pattern, file a follow-up.

**Performance regression on non-tagged PDFs.** Adding `mc_stack` push/pop logic and `is_artifact` checks in the hot path. Mitigation: the BDC/BMC operator branch is already cheap; new fields default to `None` and are zero-cost to skip. Benchmark before/after using the existing `criterion` baseline (`v2.0.0-profiling`).

**`struct_tag` memory cost.** `Option<String>` per fragment adds ~24 bytes (Option discriminant + String inline) per fragment, even when empty. On a 50k-fragment document that's ~1.2 MB. Mitigation accepted: tagged PDFs are a strategic priority; the cost is bounded and one-shot per extraction. Revisit interning (e.g., `Option<Arc<str>>`) only if a real benchmark shows it matters.

## Rollback Plan

If a regression appears post-merge, the change cannot be cleanly feature-flagged at runtime because the `TextFragment` struct shape is part of the public API. Rollback is by `git revert` of the merge commit. Before merging:

- All 16 tests above must pass locally and in CI.
- `rag_realworld` example output captured in the PR body, with chunk counts before/after.
- `cargo bench` results captured for the existing text-extraction baseline.

## Estimated Effort

~16-20 hours with strict TDD, in one PR to `develop`. Breakdown:

- Parser refactor (`MarkedContentValue`, `MarkedContentProps`, `pop_dict_or_name` rewrite) and call-site updates: ~4h.
- `TextState.mc_stack`, BDC/EMC handling, defensive paths: ~3h.
- `TextFragment` field additions, all literal call sites updated: ~2h.
- Merge predicates and grouping key changes: ~2h.
- ActualText run collapsing logic: ~2h.
- Tests #1-#13 (TDD, reproducer-then-fix loop): ~3-4h.
- Real-corpus test #15 + regression #16: ~1-2h.
- PR writeup with before/after evidence: ~1h.

## Branch and PR

- Branch: `fix/issue-269-marked-content-extraction` (created from current `develop`).
- PR target: `develop`. Title prefix: `fix(text-extract):`. Body: links #269, lists success criteria with green checks, includes NCSC before/after sample, includes `rag_realworld` chunk-count table, includes bench delta if non-trivial.
- No version bump in this PR (Phase 1 of a multi-phase initiative; bump at the end of the initiative or when the user explicitly authorizes release).
- No release, no tag, no main merge in this PR's scope.

## Subsequent Phases (Roadmap, Not Bound)

- **Phase 2** — Parse `StructTreeRoot` and `ParentTree` from the catalog into a logical structure tree. Resolves MCID → logical StructElem.
- **Phase 3** — Use `struct_tag` (and Phase 2's structure tree) in the partitioner for element classification (Title, Header, Paragraph, List, Figure, Table).
- **Phase 4** — Writer: ParentTree for multi-page tagged PDFs; automatic MCID assignment when writing text inside a marked-content scope.
- **Phase 5** — PDF/UA-1 (ISO 14289-1) basic conformance level "A": ActualText required on Figures, Lang required at root, validation.

Each phase gets its own spec, plan, and PR.
