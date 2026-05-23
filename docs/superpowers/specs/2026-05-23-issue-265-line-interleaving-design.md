# Design — Issue #265 residual line-interleaving (`row_id` Y-up-jump heuristic)

**Date**: 2026-05-23
**Issue**: [#265 — Partitioner: adjacent visual lines interleaved at character level on tightly-spaced PDF text](https://github.com/bzsanti/oxidizePdf/issues/265)
**Branch**: `fix/issue-269-marked-content-extraction` (continuation; PR #270 scope expands from "Phase 1 only" to "Phase 1 + residual #265 mitigation")
**Status**: design approved, pending implementation plan

## Problem

After PR #270 (Tagged-PDF Phase 1), NCSC CAF v4.0 page 12 still produces alphabet-soup substrings on text spans that share a marked-content scope but belong to two visually separate columns of a table. Examples:

```
sesyssteenmtias rell fuevnctainon(t tos)  are
A2.a Risk Management Process iprdeionrtiitfiiseed,d, an anald myseand, aged.
Yinfoorur rimsekd a bsys aesn smund enknoetrswns atare nd  ing of...
```

Root cause confirmed by inspecting the raw content stream of page 12:

- The entire page sits inside a single outer `BDC` (depth=1 everywhere). Phase 1's `group_by (Y_bucket, mcid)` therefore has a constant `mcid` and degenerates to `group_by (Y_bucket)` alone.
- The two-column body is emitted column-major: column 1 (X≈49.68) fully top-to-bottom from Y=480.36 down to Y=451.85, then column 2 (X≈65.08) starts at Y=486.36 (Y jumped UP by 34.51pt).
- Column Y-baselines differ by ~0.03–1.49pt — well inside any reasonable Y-tolerance.
- After `sort by (Y desc, X asc)` the fragments from both columns end up in the same Y-bucket and X-sort interleaves them.

The signal that the two streams are distinct is destroyed by the global sort. The only place where the signal still exists is **emission order**, where the transition from column 1's bottom to column 2's top is a sharp Y-up-jump.

## Why prior approaches failed

Issue #269 documented four investigation rounds on stream-tracking (v1: BT/Tm bumping; v2: overlap detection at emit; v3: best-match stream assignment; v4: per-glyph chunk). All either over-merged (alphabet soup persists) or over-split (chunk counts explode 25–35×). Those approaches tried to assign a stream identity from local positional heuristics applied **per Tj operator**. This proposal works at a **different layer**: assign a row identity once based on the macroscopic Y-up-jump pattern in emission order, then preserve it through grouping.

## Approach: `row_id` from Y-up-jump in emission order

Add a pre-pass to `merge_into_lines` (in `oxidize-pdf-core/src/text/extraction.rs`) that walks fragments in emission order and assigns an increasing `row_id: u32`. Increment when the Y delta from the previous fragment is positive and exceeds a threshold. Then change the grouping key from `(Y_bucket, mcid)` to `(row_id, Y_bucket, mcid)` with `row_id` as primary sort key.

### Algorithm

```text
fn assign_row_ids(fragments: &[TextFragment]) -> Vec<u32> {
    let mut result = Vec::with_capacity(fragments.len());
    let mut row_id: u32 = 0;
    let mut prev_y: Option<f64> = None;
    for frag in fragments {
        if let Some(py) = prev_y {
            let delta = frag.y - py;                     // positive = Y went UP
            let threshold = (frag.font_size * 0.5).max(2.0);
            if delta > threshold {
                row_id += 1;
            }
        }
        result.push(row_id);
        prev_y = Some(frag.y);
    }
    result
}
```

### Integration into `merge_into_lines`

Before:

```text
sort fragments by (y desc, x asc)
group by (Y_tolerance, mcid)
build_line_fragment per group
```

After:

```text
row_ids = assign_row_ids(fragments)            // emission order
sort (fragment, row_id) tuples by (row_id asc, y desc, x asc)
group by (row_id, Y_tolerance, mcid)
build_line_fragment per group
```

`row_id` is the primary sort key so that two fragments with the same Y_bucket but different `row_id` never become candidates for the same line group.

### Threshold rationale

`threshold = max(font_size × 0.5, 2.0)`

| Font size | Threshold | Subscript delta (typical) | Column reset delta (NCSC) |
|---|---|---|---|
| 6pt | 3.0pt (floor 2.0pt → 3.0) | 1.5–2pt | tens of pt |
| 9pt | 4.5pt | 2–3pt | 34.51pt (measured) |
| 12pt | 6.0pt | 3–4pt | tens of pt |
| 24pt | 12.0pt | 6–8pt | tens of pt |

Floor of 2.0pt guards against very small fonts where `font_size × 0.5` would be under typical font-metric noise.

## Scope

### In scope

- Single function change: `merge_into_lines` + new private helper `assign_row_ids`.
- 6 new unit tests + 2 new integration tests + extension of existing NCSC test.
- Regression validation against full `rag_realworld` corpus (5 docs, 795 chunks baseline).

### Out of scope

- New public API surface. No `TextFragment` field added. No `ExtractionOptions` flag added (heuristic always active).
- Parser changes. No new `ContentOperation` variant. No content-stream parsing modification.
- `merge_into_paragraphs` changes (operates on lines produced by `merge_into_lines`, transparent to row_id).
- Partitioner / reading-order changes. The fix preserves emission order across rows, which downstream `XYCut` / `Simple` strategies already handle.
- Interleaved column emission (col1.l1, col2.l1, col1.l2, col2.l2, ...). Tracked as known limitation for Phase 2/3 if it appears in future corpora. NCSC does not present this pattern.

## Acceptance criteria

Mapped from issue #265:

| Issue criterion | Test |
|---|---|
| Criterion 1 (synthetic tight-baseline-gap) | Already closed by #268 + new tests `merge_into_lines_splits_two_columns_emitted_sequentially` and `extraction_two_column_writer_roundtrip_test` |
| Criterion 2 (NCSC alphabet-soup absent, "system" coherent) | Extended `ncsc_page_12_extracts_coherent_text_no_alphabet_soup` test |
| Criterion 3 (no regression in ENS chunks) | `rag_realworld` regression run with ±5% per-document tolerance |

## Tests (TDD strict — content-verifying, no smoke tests)

### Unit tests (`text::extraction::tests`)

1. `assign_row_ids_monotone_y_descending_keeps_zero` — Y=400, 395, 390 → `[0, 0, 0]`.
2. `assign_row_ids_increments_on_y_up_jump_above_threshold` — Y=400, 395, 420 (font 9pt) → `[0, 0, 1]`.
3. `assign_row_ids_ignores_superscript_within_threshold` — Y=400, 402.5, 395 (font 9pt) → `[0, 0, 0]`.
4. `assign_row_ids_floor_2pt_for_small_fonts` — Y=100, 102.5 (font 3pt) → `[0, 1]`.
5. `merge_into_lines_splits_two_columns_emitted_sequentially` — 4 fragments in emission order (col1.l1 Y=400 X=50, col1.l2 Y=395 X=50, col2.l1 Y=405 X=200, col2.l2 Y=400 X=200), all width=80. Expected: 4 distinct lines with `(x=50, y=400)`, `(x=50, y=395)`, `(x=200, y=405)`, `(x=200, y=400)`.
6. `merge_into_lines_preserves_single_column_continuation` — 3 fragments (Y=400 X=50 w=30, Y=400 X=85 w=40, Y=395 X=50 w=70). Expected: 2 lines, first with concatenated `(X=50, X=85)`, second with `(X=50, Y=395)`.

### Integration tests (`oxidize-pdf-core/tests/`)

7. `extraction_two_column_writer_roundtrip_test.rs` (new file) — writes PDF with writer API: two paragraphs in parallel columns (col1 X=50, col2 X=300, parallel Y baselines 400..380), no BDC. Asserts: col1 text and col2 text emit as separated paragraphs in extraction output, no character interleaving.
8. Extension to `ncsc_no_alphabet_soup_test.rs::ncsc_page_12_extracts_coherent_text_no_alphabet_soup` — scan full page 12 output (not just first 600 chars). Assert ABSENT: `"sesyssteenmtias"`, `"iprdeionrtiitfiiseed"`, `"Yinfoorur"`, `"rimsekd"`, `"smund"`. Assert PRESENT: `"identified, analysed"`, `"prioritised, and managed"`, `"Your organisation has effective internal processes"`.

### Regression (no new tests, mandatory execution)

9. `cargo test --tests -p oxidize-pdf --no-fail-fast` — 6406 lib tests + 30 integration crates remain green.
10. `cargo run --example rag_realworld` — 5/5 documents OK, per-doc chunk count within ±5% of post-Phase 1 baseline (ENS 84, BOE 26, Higgs 142, BSI 302, NCSC 241; total 795).
11. `cargo bench --bench text_extraction` vs baseline `v2.0.0-profiling` — synthetic_10p cumulative regression (Phase 1 + #265 fix) ≤ +10%; Cold_Email_Hacks full no worse than −3% improvement over baseline (i.e., retains most of the −4.7% gain from Phase 1).

## Risks and mitigations

| Risk | Likelihood | Mitigation |
|---|---|---|
| False positive Y-up-jump from glyph re-render | Low | Conservative threshold; raise floor if observed in corpus. No instance in `corpus_cache/` or `tests/fixtures/`. |
| False negative on interleaved column emission | Medium for future corpora; zero for current | Documented out-of-scope. Tracked for Phase 2/3. |
| Multi-cell row emitted column-major changes from "row concatenation" to "N paragraphs" | Confirmed in NCSC table header (rows [0051..0059]) | Semantically more correct (each cell is a logical unit). Table-detection downstream can regroup. ENS regression test validates no significant chunk drop. |
| Performance | Low | O(n) pre-pass, n typically <10k per page. Estimated <100µs additional cost on Cold_Email_Hacks (vs 85ms baseline). Bench validates. |
| Output text order changes globally | Confirmed | row_id primary sort makes col1-then-col2 the default order. Matches emission order, which in real gov/academic PDFs aligns with reading order. Partitioner reading-order strategies (XYCut, Simple) already operate downstream and are unaffected. |

## Public API impact

None. `TextFragment` unchanged. `ExtractionOptions` unchanged. `merge_into_lines` signature unchanged.

## Files touched

- `oxidize-pdf-core/src/text/extraction.rs` — add `assign_row_ids` helper, modify `merge_into_lines` body, add 6 unit tests in `tests` module.
- `oxidize-pdf-core/tests/extraction_two_column_writer_roundtrip_test.rs` — new file, integration test 7.
- `oxidize-pdf-core/tests/ncsc_no_alphabet_soup_test.rs` — extend existing test (8).

## Success criterion (non-negotiable)

1. Integration test 8 green — garbage substrings 100% absent from full page-12 output.
2. `rag_realworld` 5/5 with per-document chunk delta ≤ ±5%.
3. Lib 6406/0 + integration green.
4. Cumulative bench regression (Phase 1 + this fix) on synthetic_10p ≤ +10%.

## Known limitations documented

- Heuristic depends on emission order being column-major in multi-column layouts. Most gov/academic/compliance PDFs satisfy this; interleaved emission is rare but possible. If a future corpus surfaces it, the fix degrades gracefully (no regression vs current state — alphabet soup persists for that document but no new bugs introduced).
- `row_id` is local to `merge_into_lines` scope. Cannot be queried by callers. If future features need per-row tagging in the public API, that's a separate spec.
