# Issue #265 — `row_id` Y-up-jump heuristic Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Eliminate residual line-interleaving in `merge_into_lines` on multi-column PDFs by assigning a `row_id` from Y-up-jumps in emission order and grouping by `(row_id, Y_bucket, mcid)`.

**Architecture:** Single function change in `oxidize-pdf-core/src/text/extraction.rs`. New private helper `assign_row_ids(&[TextFragment]) -> Vec<u32>` walks fragments in emission order and increments a counter on `Y delta > max(font_size * 0.5, 2.0)`. `merge_into_lines` zips fragments with row_ids, sorts by `(row_id asc, y desc, x asc)`, groups by the 3-tuple. No `TextFragment` field added, no public API change.

**Tech Stack:** Rust 2021 edition, MSRV 1.77, `cargo test`, `cargo bench` (criterion), pre-commit hook running fmt + clippy + build.

**Spec:** `docs/superpowers/specs/2026-05-23-issue-265-line-interleaving-design.md`
**Branch:** `fix/issue-269-marked-content-extraction` (continuation of PR #270 scope)

---

## File Structure

- **Modify** `oxidize-pdf-core/src/text/extraction.rs`:
  - Add free function `assign_row_ids` near other module-level helpers (around line 1706 where `build_line_fragment` lives).
  - Modify `merge_into_lines` body (lines 367–396) to use `assign_row_ids` and the new 3-tuple grouping.
  - Add 6 new tests in the existing `#[cfg(test)] mod tests` block.
- **Create** `oxidize-pdf-core/tests/extraction_two_column_writer_roundtrip_test.rs` — integration test.
- **Modify** `oxidize-pdf-core/tests/ncsc_no_alphabet_soup_test.rs` — extend assertions to full page output.
- **Modify** `.private/pr_269_body.md` — document scope expansion (NOT committed; per `feedback_never_commit_private`).

---

## Task 1: TDD cycle for `assign_row_ids` helper

**Files:**
- Modify: `oxidize-pdf-core/src/text/extraction.rs` (add helper near line 1706; add tests in `mod tests`)
- Test: same file, `#[cfg(test)] mod tests`

- [ ] **Step 1: Write the 4 failing unit tests**

In `oxidize-pdf-core/src/text/extraction.rs`, inside `#[cfg(test)] mod tests`, after the existing `merge_into_lines_*` tests (before the `tf` helper around line 2562), add:

```rust
#[test]
fn assign_row_ids_monotone_y_descending_keeps_zero() {
    let frags = vec![
        tf("A", 50.0, 400.0, 10.0, 9.0),
        tf("B", 50.0, 395.0, 10.0, 9.0),
        tf("C", 50.0, 390.0, 10.0, 9.0),
    ];
    let row_ids = super::assign_row_ids(&frags);
    assert_eq!(row_ids, vec![0u32, 0, 0]);
}

#[test]
fn assign_row_ids_increments_on_y_up_jump_above_threshold() {
    // font_size=9 → threshold = max(4.5, 2.0) = 4.5
    // deltas: 395-400=-5, 420-395=+25 (>4.5)
    let frags = vec![
        tf("A", 50.0, 400.0, 10.0, 9.0),
        tf("B", 50.0, 395.0, 10.0, 9.0),
        tf("C", 50.0, 420.0, 10.0, 9.0),
    ];
    let row_ids = super::assign_row_ids(&frags);
    assert_eq!(row_ids, vec![0u32, 0, 1]);
}

#[test]
fn assign_row_ids_ignores_superscript_within_threshold() {
    // font_size=9 → threshold 4.5. delta 2.5 must NOT trigger.
    let frags = vec![
        tf("A", 50.0, 400.0, 10.0, 9.0),
        tf("^2", 60.0, 402.5, 5.0, 9.0),
        tf("B", 65.0, 395.0, 10.0, 9.0),
    ];
    let row_ids = super::assign_row_ids(&frags);
    assert_eq!(row_ids, vec![0u32, 0, 0]);
}

#[test]
fn assign_row_ids_floor_2pt_for_small_fonts() {
    // font_size=3 → font_size*0.5 = 1.5; floor lifts threshold to 2.0
    // delta = +2.5 > 2.0 must trigger.
    let frags = vec![
        tf("A", 50.0, 100.0, 10.0, 3.0),
        tf("B", 50.0, 102.5, 10.0, 3.0),
    ];
    let row_ids = super::assign_row_ids(&frags);
    assert_eq!(row_ids, vec![0u32, 1]);
}
```

- [ ] **Step 2: Run tests, verify compilation failure**

Run: `cargo test -p oxidize-pdf --lib assign_row_ids 2>&1 | head -30`
Expected: compile error — `cannot find function 'assign_row_ids' in module 'super'`.

- [ ] **Step 3: Implement `assign_row_ids` at module level**

In `oxidize-pdf-core/src/text/extraction.rs`, immediately before `fn build_line_fragment` (around line 1706), add:

```rust
/// Assign a logical row identifier to each fragment based on Y-up-jumps in
/// emission order. Used by `merge_into_lines` to distinguish columns in
/// multi-column layouts where a single outer BDC scope makes mcid uniform.
///
/// Increments `row_id` whenever the next fragment's Y exceeds the previous
/// by more than `max(font_size * 0.5, 2.0)`. Superscripts (small positive
/// deltas) and normal line descents (negative deltas) leave `row_id`
/// unchanged. See `docs/superpowers/specs/2026-05-23-issue-265-line-interleaving-design.md`.
fn assign_row_ids(fragments: &[TextFragment]) -> Vec<u32> {
    let mut result = Vec::with_capacity(fragments.len());
    let mut row_id: u32 = 0;
    let mut prev_y: Option<f64> = None;
    for frag in fragments {
        if let Some(py) = prev_y {
            let delta = frag.y - py;
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

- [ ] **Step 4: Run tests, verify all 4 pass**

Run: `cargo test -p oxidize-pdf --lib assign_row_ids 2>&1 | tail -10`
Expected: `test result: ok. 4 passed; 0 failed`

- [ ] **Step 5: Commit**

```bash
git add oxidize-pdf-core/src/text/extraction.rs
git commit -m "feat(text-extract): assign_row_ids helper for column detection (addresses #265)

Walks fragments in emission order and assigns an increasing row_id when
the Y delta exceeds max(font_size * 0.5, 2.0). Used by next task to
disambiguate multi-column layouts where one outer BDC produces uniform
mcid.

4 unit tests cover monotone Y descending, Y-up-jump above threshold,
superscript within threshold, and small-font floor enforcement."
```

---

## Task 2: Wire `row_id` into `merge_into_lines`

**Files:**
- Modify: `oxidize-pdf-core/src/text/extraction.rs:367-396` (function body)
- Test: same file, `#[cfg(test)] mod tests`

- [ ] **Step 1: Write the 2 failing unit tests**

In `#[cfg(test)] mod tests`, after the existing `merge_into_lines_*` tests, add:

```rust
#[test]
fn merge_into_lines_splits_two_columns_emitted_sequentially() {
    let extractor = TextExtractor::with_options(ExtractionOptions {
        reconstruct_paragraphs: true,
        ..Default::default()
    });
    // Emission order: col1.l1, col1.l2 (Y monotone down), then col2.l1
    // (Y jumps UP by 10 > threshold 5 for font 10pt), col2.l2.
    let input = vec![
        tf("col1-top", 50.0, 400.0, 80.0, 10.0),
        tf("col1-bot", 50.0, 395.0, 80.0, 10.0),
        tf("col2-top", 200.0, 405.0, 80.0, 10.0),
        tf("col2-bot", 200.0, 400.0, 80.0, 10.0),
    ];
    let lines = extractor.merge_into_lines(&input);
    assert_eq!(lines.len(), 4, "two columns at near-identical Y must split into 4 lines");
    // row_id=0 batch first (col1), then row_id=1 (col2). Within each batch, Y desc.
    assert_eq!(lines[0].text, "col1-top");
    assert_eq!(lines[0].y, 400.0);
    assert_eq!(lines[1].text, "col1-bot");
    assert_eq!(lines[1].y, 395.0);
    assert_eq!(lines[2].text, "col2-top");
    assert_eq!(lines[2].y, 405.0);
    assert_eq!(lines[3].text, "col2-bot");
    assert_eq!(lines[3].y, 400.0);
}

#[test]
fn merge_into_lines_preserves_single_column_continuation() {
    let extractor = TextExtractor::with_options(ExtractionOptions {
        reconstruct_paragraphs: true,
        ..Default::default()
    });
    // Single column: same Y continuation (X grows), then next line down.
    let input = vec![
        tf("Hello", 50.0, 400.0, 30.0, 10.0),
        tf("world", 90.0, 400.0, 30.0, 10.0),
        tf("next-line", 50.0, 395.0, 70.0, 10.0),
    ];
    let lines = extractor.merge_into_lines(&input);
    assert_eq!(lines.len(), 2, "single column continuation must collapse to 2 lines");
    assert!(lines[0].text.contains("Hello"));
    assert!(lines[0].text.contains("world"));
    assert_eq!(lines[1].text, "next-line");
}
```

- [ ] **Step 2: Run, verify failures**

Run: `cargo test -p oxidize-pdf --lib merge_into_lines_splits_two_columns_emitted_sequentially merge_into_lines_preserves_single_column_continuation 2>&1 | tail -20`
Expected: `merge_into_lines_splits_two_columns_emitted_sequentially` fails — current code produces 2 lines (one per Y_bucket merging both columns), not 4. The continuation test should already pass (no Y-up-jump involved).

- [ ] **Step 3: Modify `merge_into_lines` body**

Replace the body of `fn merge_into_lines` (lines 367–396) with:

```rust
fn merge_into_lines(&self, fragments: &[TextFragment]) -> Vec<TextFragment> {
    if fragments.is_empty() {
        return Vec::new();
    }

    // Pre-pass: assign row_id from Y-up-jumps in emission order. This
    // disambiguates columns in multi-column layouts where a single outer
    // BDC makes mcid uniform across visually distinct columns. See
    // `docs/superpowers/specs/2026-05-23-issue-265-line-interleaving-design.md`.
    let row_ids = assign_row_ids(fragments);

    // Sort by (row_id asc, y desc, x asc). row_id primary ensures that
    // fragments from different visual rows never become candidates for
    // the same Y-bucket group.
    let mut indexed: Vec<(u32, &TextFragment)> = row_ids
        .iter()
        .copied()
        .zip(fragments.iter())
        .collect();
    indexed.sort_by(|a, b| {
        a.0.cmp(&b.0)
            .then(b.1.y.total_cmp(&a.1.y))
            .then(a.1.x.total_cmp(&b.1.x))
    });

    let mut lines: Vec<Vec<&TextFragment>> = Vec::new();
    let mut current_row_id: Option<u32> = None;
    for (rid, frag) in indexed {
        let same_row_id = current_row_id == Some(rid);
        let placed = same_row_id
            && lines.last_mut().is_some_and(|line| {
                let head = line[0];
                let tol = (head.height.min(frag.height)) * 0.2;
                (head.y - frag.y).abs() < tol && head.mcid == frag.mcid
            });
        if placed {
            lines.last_mut().unwrap().push(frag);
        } else {
            lines.push(vec![frag]);
            current_row_id = Some(rid);
        }
    }

    lines
        .into_iter()
        .map(|line| build_line_fragment(line, self.options.space_threshold))
        .collect()
}
```

- [ ] **Step 4: Run all extraction tests, verify pass**

Run: `cargo test -p oxidize-pdf --lib text::extraction::tests 2>&1 | tail -10`
Expected: `test result: ok. <N> passed; 0 failed` — all existing `merge_into_lines_*`, `merge_into_paragraphs_*`, and the new 2 tests pass.

- [ ] **Step 5: Run wider lib tests to catch regressions**

Run: `cargo test -p oxidize-pdf --lib 2>&1 | tail -10`
Expected: `test result: ok. 6406 passed` (or higher if other Phase-1 commits land in between).

If any test fails, the change in line ordering may have broken assertions that depend on global sort order. Investigate, repair, do NOT revert the change blindly — the new ordering is the desired behavior.

- [ ] **Step 6: Commit**

```bash
git add oxidize-pdf-core/src/text/extraction.rs
git commit -m "fix(text-extract): row_id-aware merge_into_lines for multi-column layouts (#265)

Replaces global sort+bucket with (row_id, Y_bucket, mcid) grouping.
row_id is the primary sort key so fragments from different visual rows
never become candidates for the same line. The Y-tolerance and mcid
predicate are unchanged from Phase 1.

Closes #265 criterion 2 for column-major emitted PDFs. Interleaved
column emission (col1.l1, col2.l1, col1.l2, ...) remains out of scope.

2 new unit tests cover two-column split and single-column continuation."
```

---

## Task 3: Integration test — synthetic 2-column writer roundtrip

**Files:**
- Create: `oxidize-pdf-core/tests/extraction_two_column_writer_roundtrip_test.rs`

- [ ] **Step 1: Create the integration test file**

```rust
//! Issue #265 — writer→extractor roundtrip verifying that two parallel
//! columns written with the Page text API extract as separated paragraphs
//! (no character interleaving). Complements the NCSC corpus test by
//! exercising a deterministic synthetic input.

use oxidize_pdf::parser::{PdfDocument, PdfReader};
use oxidize_pdf::text::{ExtractionOptions, TextExtractor};
use oxidize_pdf::{Document, Font, Page};
use std::io::Cursor;

#[test]
fn two_column_layout_extracts_without_interleaving() {
    // Build a PDF with two parallel paragraphs at distinct X but
    // overlapping Y baselines. Emission order: column 1 fully, then
    // column 2 fully — mimics how the NCSC PDF lays out tables.
    let mut doc = Document::new();
    let mut page = Page::a4();

    // Column 1: X=50, Y descending 700..650 in 10-unit steps.
    for (i, text) in ["Col1-line1", "Col1-line2", "Col1-line3"].iter().enumerate() {
        page.text()
            .set_font(Font::Helvetica, 10.0)
            .at(50.0, 700.0 - (i as f64) * 12.0)
            .write(text)
            .expect("col1 write");
    }
    // Column 2: X=300, Y baselines near col1's (overlap inside Y-tolerance).
    for (i, text) in ["Col2-line1", "Col2-line2", "Col2-line3"].iter().enumerate() {
        page.text()
            .set_font(Font::Helvetica, 10.0)
            .at(300.0, 700.5 - (i as f64) * 12.0)
            .write(text)
            .expect("col2 write");
    }

    doc.add_page(page);

    let mut buf: Vec<u8> = Vec::new();
    doc.write(&mut buf).expect("write PDF");

    let reader = PdfReader::new(Cursor::new(buf)).expect("read PDF");
    let document = PdfDocument::new(reader);

    let opts = ExtractionOptions {
        preserve_layout: true,
        reconstruct_paragraphs: true,
        ..ExtractionOptions::default()
    };
    let mut extractor = TextExtractor::with_options(opts);
    let extracted = extractor.extract_from_page(&document, 0).expect("extract");
    let text = extracted.text.as_str();

    // Negative: no character interleaving between columns. A literal
    // interleaved sequence would contain substrings like "CoCl1ol2".
    assert!(
        !text.contains("CoCl"),
        "expected no character interleaving between columns; got:\n{}",
        text
    );

    // Positive: each column's lines survive as recognizable runs.
    for needle in &["Col1-line1", "Col1-line2", "Col1-line3", "Col2-line1", "Col2-line2", "Col2-line3"] {
        assert!(
            text.contains(needle),
            "missing column run {:?} in extracted text:\n{}",
            needle,
            text
        );
    }
}
```

- [ ] **Step 2: Run the integration test**

Run: `cargo test -p oxidize-pdf --test extraction_two_column_writer_roundtrip_test 2>&1 | tail -10`
Expected: `test result: ok. 1 passed; 0 failed`.

If the test fails because the `Font` or `Page::a4` API differs in this version, adjust imports — search `oxidize-pdf-core/examples/` for current usage.

- [ ] **Step 3: Commit**

```bash
git add oxidize-pdf-core/tests/extraction_two_column_writer_roundtrip_test.rs
git commit -m "test(text-extract): two-column writer roundtrip (addresses #265)

Writes a 2-column page via the writer API with overlapping Y baselines
and column-major emission, then extracts and asserts no character
interleaving plus all 6 column runs survive intact."
```

---

## Task 4: Extend NCSC test to full-page assertions

**Files:**
- Modify: `oxidize-pdf-core/tests/ncsc_no_alphabet_soup_test.rs`

- [ ] **Step 1: Add new negative and positive assertions**

In `oxidize-pdf-core/tests/ncsc_no_alphabet_soup_test.rs`, replace the body of `ncsc_page_12_extracts_coherent_text_no_alphabet_soup` (lines 26–76) with:

```rust
#[test]
fn ncsc_page_12_extracts_coherent_text_no_alphabet_soup() {
    let path = match corpus_path() {
        Some(p) => p,
        None => {
            eprintln!("ncsc_no_alphabet_soup_test: corpus file missing, skipping");
            return;
        }
    };

    let reader = PdfReader::open(&path).expect("open NCSC corpus");
    let document = PdfDocument::new(reader);

    let opts = ExtractionOptions {
        preserve_layout: true,
        reconstruct_paragraphs: true,
        ..ExtractionOptions::default()
    };
    let mut extractor = TextExtractor::with_options(opts);

    let extracted = extractor
        .extract_from_page(&document, 11)
        .expect("extract page 12");

    let full_text = extracted.text.as_str();

    // Phase 1 garbage substrings (closed by #269 PR #270).
    for garbage in &["Tahre", "iansag", "efysftecemtaitivecl", "neod s ef"] {
        assert!(
            !full_text.contains(garbage),
            "Phase-1 garbage substring {:?} still present; extracted text:\n{}",
            garbage,
            full_text
        );
    }

    // Phase 2 (#265 row_id heuristic) — residual column-overlap garbage.
    // These substrings only appear in interleaved output; no legitimate
    // English token contains them.
    for garbage in &[
        "sesyssteenmtias",
        "iprdeionrtiitfiiseed",
        "Yinfoorur",
        "rimsekd",
        "smund",
    ] {
        assert!(
            !full_text.contains(garbage),
            "residual #265 column-interleave garbage substring {:?} still present; \
             extracted text:\n{}",
            garbage,
            full_text
        );
    }

    // Coherent runs from column 2 (right-hand cell of the A2.a table).
    // Their presence proves column 2 was extracted intact, not destroyed.
    for needle in &[
        "identified, analysed",
        "prioritised, and managed",
        "Your organisation has effective internal processes",
    ] {
        assert!(
            full_text.contains(needle),
            "expected coherent column-2 phrase {:?} missing; extracted text:\n{}",
            needle,
            full_text
        );
    }
}
```

- [ ] **Step 2: Run the extended NCSC test**

Run: `cargo test -p oxidize-pdf --test ncsc_no_alphabet_soup_test 2>&1 | tail -20`
Expected: `test result: ok. 1 passed`.

If the test fails:
- Negative assertion failure means the row_id heuristic did NOT eliminate that substring. Inspect the extracted text manually: `cargo test -p oxidize-pdf --test ncsc_no_alphabet_soup_test -- --nocapture 2>&1 | grep -A 5 "garbage substring"`. Then revisit the threshold or grouping logic.
- Positive assertion failure means the coherent text is missing. Likely a regression — the column 2 paragraphs may have been moved or absorbed elsewhere. Inspect emission order; the spec's threshold may be too aggressive.

Do NOT relax the assertions to make the test pass.

- [ ] **Step 3: Commit**

```bash
git add oxidize-pdf-core/tests/ncsc_no_alphabet_soup_test.rs
git commit -m "test(text-extract): NCSC full-page no column-interleave (addresses #265)

Extends the existing Phase-1 NCSC garbage test with 5 new negative
assertions for residual column-interleave substrings and 3 new positive
assertions for coherent column-2 phrases. Validates that the row_id
heuristic from Task 2 eliminates the two-column alphabet soup."
```

---

## Task 5: Regression validation — rag_realworld + bench

**Files:** none (verification only)

- [ ] **Step 1: Run full workspace tests**

Run: `cargo test --tests -p oxidize-pdf --no-fail-fast 2>&1 | tail -30`
Expected: all integration crates green; lib at 6406+ passed.

- [ ] **Step 2: Run `rag_realworld` example**

Run: `nice cargo run --example rag_realworld --release 2>&1 | tee .private/rag_realworld_after_265.log | tail -30`
Expected: 5/5 documents OK. Extract per-doc chunk counts.

Baseline (post-Phase 1): ENS 84, BOE 26, Higgs 142, BSI 302, NCSC 241, total 795.
Tolerance: ±5% per document.

- ENS: 80–88 (84 ± 4.2)
- BOE: 25–28 (26 ± 1.3)
- Higgs: 135–149 (142 ± 7.1)
- BSI: 287–317 (302 ± 15.1)
- NCSC: 229–253 (241 ± 12.05)

If any document is outside the band, capture which and analyze the diff in `out/*.jsonl`. NCSC is expected to potentially shift more because this fix targets it directly — verify the chunk count change reflects column separation (more discrete paragraphs) and that text quality is improved, not degraded.

- [ ] **Step 3: Run bench**

Run: `nice cargo bench --bench text_extraction 2>&1 | tee .private/bench_after_265.log | tail -40`
Expected: criterion compares to baseline `v2.0.0-profiling`. Acceptance:
- `text_extraction_full/synthetic_10p`: cumulative regression ≤ +10% (Phase 1 alone was +7.3%, budget +2.7% for this pass).
- `text_extraction_full/Cold_Email_Hacks`: no worse than −3% (Phase 1 was −4.7%, budget −1.7%).

If outside budget, analyze the criterion report HTML for hot spots. The pre-pass `assign_row_ids` is O(n) and should add <100µs per page; if bench shows larger cost, profile.

- [ ] **Step 4: Commit verification artifacts to `.private/`**

`.private/` is gitignored per `feedback_never_commit_private`. Do not stage or commit those files. Keep the logs locally for the PR body update.

- [ ] **Step 5: If any acceptance criterion fails, STOP and report**

Do not push or update the PR until all 4 success criteria from the spec are met:
1. Test 8 (NCSC extension) green.
2. `rag_realworld` 5/5 within ±5% per doc.
3. Lib + integration green.
4. Bench within +10% cumulative.

Report findings to the user before deciding on remediation.

---

## Task 6: Update PR #270 body (USER AUTHORIZATION REQUIRED)

**Files:**
- Modify: `.private/pr_269_body.md` (local only, not committed)

- [ ] **Step 1: Draft the scope-expansion addendum**

In `.private/pr_269_body.md`, after the existing "Scope discipline" section, insert:

```markdown
## Scope expansion — #265 residual column-interleave

After the Phase 1 commits landed, NCSC CAF v4.0 page 12 still produced
alphabet-soup substrings on a two-column table body where a single outer
BDC made `mcid` uniform across columns. This PR additionally includes
the `row_id` Y-up-jump heuristic from
`docs/superpowers/specs/2026-05-23-issue-265-line-interleaving-design.md`:

- New private helper `assign_row_ids` walks fragments in emission order
  and increments a row counter on `Y delta > max(font_size * 0.5, 2.0)`.
- `merge_into_lines` now groups by `(row_id, Y_bucket, mcid)` with
  `row_id` as primary sort key.
- 6 new unit tests + 1 new integration test (`extraction_two_column_writer_roundtrip_test.rs`)
  + extension of the existing NCSC test.

Addresses #265 criterion 2. Interleaved column emission (col1.l1, col2.l1, col1.l2, ...) remains out of scope.

### #265 acceptance verification

| Criterion | Status |
|---|---|
| 1 — synthetic tight-baseline-gap | Already closed by #268; reinforced by new tests |
| 2 — NCSC alphabet-soup absent | Verified by extended NCSC test (5 new negative assertions, 3 positive) |
| 3 — no regression in ENS chunks | <fill from regression run: ENS = N chunks (baseline 84 ± 5%)> |

### Final bench delta (Phase 1 + #265 fix combined)

<fill from .private/bench_after_265.log>
```

- [ ] **Step 2: Ask user for authorization to edit PR #270 body**

The PR body edit is visible to others. Per `feedback_no_auto_issue_comments`, ask the user explicitly before pushing the edit. Do not run `gh pr edit` autonomously.

Suggested message to user:

> Task 6 ready. Draft addendum prepared in `.private/pr_269_body.md` with `<fill>` placeholders to substitute from `.private/rag_realworld_after_265.log` and `.private/bench_after_265.log`. May I run `gh pr edit 270 --body-file .private/pr_269_body.md` to push the update?

- [ ] **Step 3: After user authorization, fill placeholders and push**

Substitute the `<fill>` markers with actual numbers from the logs. Then:

```bash
gh pr edit 270 --body-file .private/pr_269_body.md
gh pr view 270 --json title,body --jq '.body' | head -50
```

Verify the addendum appears in the live PR body.

- [ ] **Step 4: No commit — `.private/` stays untracked**

Confirm `git status` shows clean working tree post-edit (the PR body is on GitHub, not in the repo).

---

## Self-Review Result

**Spec coverage check:**
- Algorithm (`assign_row_ids`) — Task 1.
- `merge_into_lines` modification — Task 2.
- 6 unit tests — Tasks 1 (4 tests for helper) + 2 (2 tests for merge).
- Integration test 7 (two-column writer roundtrip) — Task 3.
- Integration test 8 (NCSC extension) — Task 4.
- Regression suite (steps 9, 10, 11) — Task 5.
- Risks/side effects validation — Task 5 (chunk count tolerance + bench budget).
- PR scope expansion documentation — Task 6.

All spec sections mapped. No gaps.

**Placeholder scan:**
- No "TBD"/"TODO"/"implement later" in step content. The `<fill>` markers in Task 6 are intentional — they require regression numbers that only exist after Task 5 completes.

**Type consistency:**
- `assign_row_ids(&[TextFragment]) -> Vec<u32>` used identically in Tasks 1 (definition) and 2 (call site).
- `tf(text, x, y, width, font_size)` matches existing helper signature at line 2562 (verified).
- `merge_into_lines` signature unchanged; verified consistent.
- Test names referenced across tasks are consistent (no rename mid-plan).

---

## Execution Notes

- **Branch already exists**: `fix/issue-269-marked-content-extraction` is checked out. No `git checkout` needed.
- **Pre-commit hook**: runs cargo fmt + clippy + library build. Will skip `cargo test --lib` for metadata-only changes. For source changes, it runs the lib tests. Expect ~30s per commit.
- **`nice` for heavy commands**: per CLAUDE.md, use `nice cargo run/bench` on long tasks. Lib `cargo test` is fast enough to run without.
- **Commit cadence**: one commit per task (5 total commits + PR edit). No squash needed; the merge to develop will keep them as separate atomic changes.
- **Do NOT push to origin until Task 5 succeeds**. Local commits only; push after regression validation green.
