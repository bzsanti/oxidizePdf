# Issue #261 — Paragraph Reconstruction in Text Extraction Pipeline

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fix [issue #261](https://github.com/bzsanti/oxidizePdf/issues/261) so that `rag_chunks()` (and the entire `partition()` family) produces paragraph-level Elements instead of one Element per `Tj` text-show operator. Concretely: an input PDF whose page contains the rendered string `"Verificable en https://www.boe.es"` must produce **one** RagChunk for that string, not four.

**Architecture:** Insert two new reconstruction passes into the text-extraction pipeline between `merge_close_fragments` (kerning fix) and the partition classifier:

1. `merge_into_lines(fragments)` — groups fragments by baseline (Y-band tolerance ≈ font height) and concatenates with single spaces where `x_gap > space_threshold * font_size`. Output: one `TextFragment` per visual line, bounding box = union.
2. `merge_into_paragraphs(lines)` — groups consecutive lines whose vertical gap is within ~1.5× the modal line-leading into a single paragraph fragment, joined by `'\n'` (or by `' '` if hyphenation rule applies and previous line ended with `-`). Output: one `TextFragment` per visual paragraph.

The passes are gated by a new `ExtractionOptions::reconstruct_paragraphs: bool` field, default `false` to preserve backward compatibility for direct `extract_text()` callers. The `partition()`/`partition_with()`/`partition_with_profile()` entry points force the field to `true`.

**Tech Stack:** Rust stable. Pure changes inside `oxidize-pdf-core/src/text/extraction.rs` and `oxidize-pdf-core/src/parser/document.rs`. No new dependencies. TDD throughout.

**Related:**
- Issue: https://github.com/bzsanti/oxidizePdf/issues/261
- Evidence comes from a parallel branch `feature/rag-realworld-rust` (PR not yet open) which built the real-world corpus example that surfaced the bug.

**Branch:** `fix/issue-261-paragraph-reconstruction` (already created from `develop`).

---

## File structure

| Path | Action | Purpose |
|---|---|---|
| `oxidize-pdf-core/src/text/extraction.rs` | Modify | Add `reconstruct_paragraphs: bool` field to `ExtractionOptions`, add `merge_into_lines` and `merge_into_paragraphs` methods on `TextExtractor`, wire them into `extract_text` when the field is true. |
| `oxidize-pdf-core/src/parser/document.rs` | Modify | In `partition_with` (and `partition_with_profile`), set `reconstruct_paragraphs: true` on the `ExtractionOptions`. |
| `oxidize-pdf-core/tests/paragraph_reconstruction_test.rs` | Create | Synthetic-fragment TDD tests for the two new methods + an integration assertion using one of the existing small fixture PDFs. |

**Decomposition note:** All production code stays in `extraction.rs` and `document.rs`. The new helpers are private methods on `TextExtractor`. This keeps the change surgical — no new modules, no public API additions besides the one new `ExtractionOptions` field.

---

## Task 1: Add the failing reproducer test (TDD red)

Write a content-verifying test that synthesizes `TextFragment` values mimicking the bug — many small adjacent fragments on the same line — and asserts that, after running the (yet-to-be-implemented) reconstruction, a single fragment results.

**Files:**
- Create: `oxidize-pdf-core/tests/paragraph_reconstruction_test.rs`

- [ ] **Step 1.1: Inspect public exports of TextFragment + TextExtractor**

```bash
grep -nE "pub struct TextFragment|pub struct TextExtractor|pub fn new" oxidize-pdf-core/src/text/extraction.rs | head -10
grep -nE "pub use .*TextFragment|pub use .*TextExtractor" oxidize-pdf-core/src/text/mod.rs oxidize-pdf-core/src/lib.rs 2>/dev/null
```

This tells you the exact public path. The test must import via the public API (e.g. `use oxidize_pdf::text::TextFragment;`). If `TextExtractor` is not public, the reproducer must work through `PdfDocument::extract_text_with_options` against a synthetic PDF — but ideally we test the helpers directly.

If `TextExtractor` and the new merge helpers are not public, you have two options:
1. Make the helpers `pub(crate)` and add a public wrapper for testing
2. Test indirectly through `PdfDocument`

Pick option 1 by adding `pub fn merge_fragments_for_partition(&self, fragments: &[TextFragment]) -> Vec<TextFragment>` as a thin public wrapper that runs the merge chain (kerning → lines → paragraphs). This single entry point is what the test calls.

- [ ] **Step 1.2: Write the failing test file**

Create `oxidize-pdf-core/tests/paragraph_reconstruction_test.rs`:

```rust
//! Tests for paragraph reconstruction in the text extraction pipeline.
//!
//! Reproduces issue #261: prior to the fix, fragments arriving from PDF Tj/TJ
//! operators are passed through the partitioner one-per-fragment, producing
//! per-word "chunks" that are unusable for RAG ingestion.

use oxidize_pdf::text::{ExtractionOptions, TextExtractor, TextFragment};

fn frag(text: &str, x: f64, y: f64, width: f64, font_size: f64) -> TextFragment {
    TextFragment {
        text: text.to_string(),
        x,
        y,
        width,
        height: font_size,
        font_size,
        font_name: Some("Helvetica".to_string()),
        is_bold: false,
        is_italic: false,
        color: None,
        space_decisions: Vec::new(),
    }
}

#[test]
fn five_fragments_on_one_line_collapse_to_one_paragraph() {
    // Simulates "Verificable en https://www.boe.es" extracted from a PDF
    // as five fragments (the BOE footer pattern). Y = 50.0 (footer area),
    // font_size = 8.0pt, each fragment is consecutive in X.
    let opts = ExtractionOptions {
        reconstruct_paragraphs: true,
        ..ExtractionOptions::default()
    };
    let extractor = TextExtractor::with_options(opts);
    let input = vec![
        frag("V", 100.0, 50.0, 5.0, 8.0),
        frag("erificable en https://www", 105.0, 50.0, 140.0, 8.0),
        frag(".boe.es", 245.0, 50.0, 35.0, 8.0),
        frag("cve: BOE-A-2022-7191", 290.0, 50.0, 110.0, 8.0),
    ];

    let merged = extractor.merge_fragments_for_partition(&input);

    assert_eq!(merged.len(), 1, "all four fragments on one line must collapse to one");
    let f = &merged[0];
    assert!(
        f.text.contains("Verificable en https://www.boe.es"),
        "merged text must contain the joined URL, got {:?}",
        f.text
    );
    assert!(
        f.text.contains("cve: BOE-A-2022-7191"),
        "merged text must contain both spans, got {:?}",
        f.text
    );
    assert!(
        f.width >= 295.0,
        "merged width must span the entire line, got {}",
        f.width
    );
}

#[test]
fn fragments_on_three_consecutive_lines_collapse_to_one_paragraph() {
    // Three lines of body text with normal leading.
    // Y in PDF coords decreases as you go down. font_size 12pt, leading ≈ 14pt.
    let opts = ExtractionOptions {
        reconstruct_paragraphs: true,
        ..ExtractionOptions::default()
    };
    let extractor = TextExtractor::with_options(opts);
    let input = vec![
        // Line 1 at y=400
        frag("The", 50.0, 400.0, 20.0, 12.0),
        frag("first", 75.0, 400.0, 30.0, 12.0),
        frag("line.", 110.0, 400.0, 30.0, 12.0),
        // Line 2 at y=386 (14pt below)
        frag("Second", 50.0, 386.0, 38.0, 12.0),
        frag("line", 93.0, 386.0, 25.0, 12.0),
        frag("here.", 123.0, 386.0, 30.0, 12.0),
        // Line 3 at y=372
        frag("Third.", 50.0, 372.0, 35.0, 12.0),
    ];

    let merged = extractor.merge_fragments_for_partition(&input);

    assert_eq!(merged.len(), 1, "three lines with normal leading must form one paragraph");
    let f = &merged[0];
    assert!(f.text.starts_with("The first line."), "first line preserved: {:?}", f.text);
    assert!(f.text.contains("Second line here."), "second line preserved: {:?}", f.text);
    assert!(f.text.ends_with("Third."), "third line at end: {:?}", f.text);
    // Lines joined by '\n'
    assert_eq!(f.text.matches('\n').count(), 2, "two newlines between three lines");
}

#[test]
fn two_paragraphs_separated_by_large_gap_stay_separate() {
    // Two single-line paragraphs separated by 3× leading.
    let opts = ExtractionOptions {
        reconstruct_paragraphs: true,
        ..ExtractionOptions::default()
    };
    let extractor = TextExtractor::with_options(opts);
    let input = vec![
        frag("Paragraph", 50.0, 400.0, 60.0, 12.0),
        frag("one.", 115.0, 400.0, 25.0, 12.0),
        // 42pt below — three blank lines
        frag("Paragraph", 50.0, 358.0, 60.0, 12.0),
        frag("two.", 115.0, 358.0, 25.0, 12.0),
    ];

    let merged = extractor.merge_fragments_for_partition(&input);

    assert_eq!(merged.len(), 2, "gap > 1.5× leading must split paragraphs");
    assert!(merged[0].text.contains("Paragraph one."), "first paragraph: {:?}", merged[0].text);
    assert!(merged[1].text.contains("Paragraph two."), "second paragraph: {:?}", merged[1].text);
}

#[test]
fn hyphenated_line_break_joins_without_space() {
    // Word broken across two lines with hyphen.
    let opts = ExtractionOptions {
        reconstruct_paragraphs: true,
        merge_hyphenated: true,
        ..ExtractionOptions::default()
    };
    let extractor = TextExtractor::with_options(opts);
    let input = vec![
        // Line 1 ending with hyphen
        frag("crypto-", 50.0, 400.0, 50.0, 12.0),
        // Line 2 starting with "graphy"
        frag("graphy", 50.0, 386.0, 40.0, 12.0),
    ];

    let merged = extractor.merge_fragments_for_partition(&input);

    assert_eq!(merged.len(), 1, "hyphenated word must form one paragraph");
    assert!(
        merged[0].text.contains("cryptography") && !merged[0].text.contains("crypto-"),
        "hyphen must be elided and word joined: {:?}",
        merged[0].text
    );
}

#[test]
fn empty_input_returns_empty() {
    let opts = ExtractionOptions {
        reconstruct_paragraphs: true,
        ..ExtractionOptions::default()
    };
    let extractor = TextExtractor::with_options(opts);
    let merged = extractor.merge_fragments_for_partition(&[]);
    assert!(merged.is_empty());
}

#[test]
fn reconstruct_disabled_returns_input_unchanged_modulo_kerning_fix() {
    // With reconstruct_paragraphs=false, only the existing merge_close_fragments runs.
    // Same input as the BOE footer case: it must NOT collapse to one fragment.
    let opts = ExtractionOptions {
        reconstruct_paragraphs: false,
        preserve_layout: true,
        ..ExtractionOptions::default()
    };
    let extractor = TextExtractor::with_options(opts);
    let input = vec![
        frag("V", 100.0, 50.0, 5.0, 8.0),
        frag("erificable en https://www", 105.0, 50.0, 140.0, 8.0),
        frag(".boe.es", 245.0, 50.0, 35.0, 8.0),
        frag("cve: BOE-A-2022-7191", 290.0, 50.0, 110.0, 8.0),
    ];
    let merged = extractor.merge_fragments_for_partition(&input);
    assert!(
        merged.len() > 1,
        "with reconstruct_paragraphs=false, fragments must NOT be collapsed to paragraphs"
    );
}
```

- [ ] **Step 1.3: Run the test, confirm it fails to compile**

```bash
cargo test --manifest-path oxidize-pdf-core/Cargo.toml --test paragraph_reconstruction_test 2>&1 | tail -20
```

Expected: compilation errors mentioning `ExtractionOptions::reconstruct_paragraphs` (field does not exist), `TextExtractor::with_options` (might not be public), or `merge_fragments_for_partition` (method does not exist). This is the TDD "red".

- [ ] **Step 1.4: Commit the failing test**

```bash
git add oxidize-pdf-core/tests/paragraph_reconstruction_test.rs
git commit -m "test: TDD reproducer for issue #261 paragraph reconstruction"
```

The pre-commit hook will fail because the test does not compile. Override only this one commit by reordering: write the test, run `cargo build` (not test) to see what's missing, but DO NOT skip the hook. Instead: stage the test file but commit ONLY after the next task makes it compile. So in this step, save the file but DO NOT commit yet — proceed to Task 2 immediately and commit the test together with the type plumbing in Task 2.6.

Skip Step 1.4 if the hook fails. Move directly to Task 2.

---

## Task 2: Add `reconstruct_paragraphs` field + `merge_fragments_for_partition` skeleton

**Files:**
- Modify: `oxidize-pdf-core/src/text/extraction.rs`

- [ ] **Step 2.1: Locate the `ExtractionOptions` struct definition**

```bash
grep -n "pub struct ExtractionOptions\|impl Default for ExtractionOptions" oxidize-pdf-core/src/text/extraction.rs
```

You should find the struct (around line 18) and the Default impl (around line 42 per the diagnostic shown in the issue). The struct already has fields `preserve_layout`, `space_threshold`, `newline_threshold`, `sort_by_position`, `detect_columns`, `column_threshold`, `merge_hyphenated`, `track_space_decisions`. We add one more.

- [ ] **Step 2.2: Add the new field**

In the struct definition, after `track_space_decisions: bool,` add:

```rust
    /// Reconstruct visual lines and paragraphs from the raw text fragments
    /// produced by PDF text-show operators. When `true`, the extractor groups
    /// fragments by baseline into single-line fragments, then groups
    /// consecutive lines with normal leading into paragraph-level fragments.
    /// This is what the partition pipeline needs to produce Element values at
    /// paragraph granularity rather than at per-`Tj` granularity (see
    /// [issue #261](https://github.com/bzsanti/oxidizePdf/issues/261)).
    ///
    /// Default `false` for backward compatibility with direct `extract_text`
    /// callers. The `PdfDocument::partition*` entry points force this to
    /// `true`.
    pub reconstruct_paragraphs: bool,
```

In the `Default for ExtractionOptions` impl, add the default value:

```rust
            reconstruct_paragraphs: false,
```

Place it after `track_space_decisions: false,` to mirror the struct field order.

- [ ] **Step 2.3: Update every existing struct literal that uses `..ExtractionOptions::default()`**

Most callsites already use `..Default::default()` and will pick up the new field automatically. But any explicit struct literal must add the field. Find them:

```bash
grep -nE "ExtractionOptions \{" oxidize-pdf-core/src/ oxidize-pdf-core/tests/ -r | grep -v "::default()"
```

For each match that does NOT spread defaults, add `reconstruct_paragraphs: false,` (matching the rest of the file's style — boolean fields default to `false` unless context indicates otherwise).

- [ ] **Step 2.4: Add a public `with_options` constructor on `TextExtractor`**

If `TextExtractor::new` already exists but `with_options` doesn't, add it. Locate the existing constructor first:

```bash
grep -nE "impl TextExtractor|pub fn new\(" oxidize-pdf-core/src/text/extraction.rs | head
```

Then add (after the existing `new` method):

```rust
    /// Construct an extractor with explicit options.
    pub fn with_options(options: ExtractionOptions) -> Self {
        Self { options }
    }
```

If the struct internally holds more than just `options`, mirror what `new` does for the other fields.

- [ ] **Step 2.5: Add a public stub for `merge_fragments_for_partition`**

Inside `impl TextExtractor`, add:

```rust
    /// Run the full fragment-merge chain used by the partition pipeline:
    /// kerning fix → line reconstruction → paragraph reconstruction.
    ///
    /// Honors `ExtractionOptions::reconstruct_paragraphs`: when `false`, only
    /// `merge_close_fragments` (the kerning fix) runs and the input is
    /// returned at fragment granularity.
    ///
    /// This method is `pub` so the integration test in
    /// `tests/paragraph_reconstruction_test.rs` can exercise it without going
    /// through a PDF file. Production callers should prefer
    /// `PdfDocument::partition()` and friends, which use this internally.
    pub fn merge_fragments_for_partition(&self, fragments: &[TextFragment]) -> Vec<TextFragment> {
        // Step 1: kerning fix (existing behavior)
        let kerning_fixed = self.merge_close_fragments(fragments);
        if !self.options.reconstruct_paragraphs {
            return kerning_fixed;
        }
        // Step 2: line reconstruction (Task 3 fills this in)
        let lines = self.merge_into_lines(&kerning_fixed);
        // Step 3: paragraph reconstruction (Task 4 fills this in)
        self.merge_into_paragraphs(&lines)
    }

    fn merge_into_lines(&self, _fragments: &[TextFragment]) -> Vec<TextFragment> {
        // TODO: Task 3 — temporary identity stub so this task compiles.
        // The Task 1 tests will FAIL with this stub; Task 3 makes them pass.
        Vec::new() // intentional placeholder; Task 3 replaces
    }

    fn merge_into_paragraphs(&self, _lines: &[TextFragment]) -> Vec<TextFragment> {
        // TODO: Task 4 — temporary identity stub so this task compiles.
        Vec::new() // intentional placeholder; Task 4 replaces
    }
```

Note: the TODO comments here are explicitly allowed exceptions to the no-TODO rule — they tell the next subagent which task fills each in. Remove the TODO comment when the stub is replaced in Task 3 / Task 4.

If the `_fragments` underscore-prefix triggers `unused_variables` despite the leading underscore being the documented suppression, drop the underscore and add `#[allow(unused_variables)]` on the method.

- [ ] **Step 2.6: Compile the library and the new test together**

```bash
cargo build --manifest-path oxidize-pdf-core/Cargo.toml --tests 2>&1 | tail -10
```

Expected: builds cleanly. The new test file now compiles even though the assertions will fail (the stubs return empty vectors).

```bash
cargo test --manifest-path oxidize-pdf-core/Cargo.toml --test paragraph_reconstruction_test 2>&1 | tail -20
```

Expected: 6 tests run, most fail with `assertion failed` (this is the TDD red state, but it compiles).

- [ ] **Step 2.7: Commit type plumbing + the failing test**

```bash
git add oxidize-pdf-core/src/text/extraction.rs oxidize-pdf-core/tests/paragraph_reconstruction_test.rs
git commit -m "feat: add reconstruct_paragraphs field and merge_fragments_for_partition stub

Adds the type plumbing for issue #261 and the TDD reproducer tests.
The merge_into_lines and merge_into_paragraphs methods are stubs
that return empty vectors — the reproducer tests fail intentionally
until Tasks 3 and 4 fill them in."
```

The pre-commit hook will run `cargo test --lib` which exercises the LIBRARY tests only, not integration tests. The lib tests should still pass because we haven't changed any library behavior yet, only added a field with a `false` default. If the hook fails, investigate.

---

## Task 3: Implement `merge_into_lines`

**Files:**
- Modify: `oxidize-pdf-core/src/text/extraction.rs`

The line merger groups fragments whose baselines are within a small tolerance, sorts within each line by X, and concatenates with spaces inserted where the X gap exceeds the configured `space_threshold * font_size`.

- [ ] **Step 3.1: Write a unit test inside `extraction.rs` for `merge_into_lines`**

Locate the existing `#[cfg(test)] mod tests` block at the bottom of `extraction.rs`. Add inside it:

```rust
    // Helper for unit tests in this module. `TextFragment` does not implement
    // `Default`, so an explicit helper avoids 11-field struct literals
    // everywhere.
    fn tf(text: &str, x: f64, y: f64, width: f64, font_size: f64) -> TextFragment {
        TextFragment {
            text: text.to_string(),
            x,
            y,
            width,
            height: font_size,
            font_size,
            font_name: None,
            is_bold: false,
            is_italic: false,
            color: None,
            space_decisions: Vec::new(),
        }
    }

    #[test]
    fn merge_into_lines_groups_same_baseline_fragments() {
        let extractor = TextExtractor::with_options(ExtractionOptions {
            reconstruct_paragraphs: true,
            ..Default::default()
        });
        // Two lines, each with three fragments
        let input = vec![
            tf("Hello", 50.0, 400.0, 30.0, 12.0),
            tf("world", 90.0, 400.0, 30.0, 12.0),
            tf("now.", 130.0, 400.0, 25.0, 12.0),
            tf("Next", 50.0, 386.0, 30.0, 12.0),
            tf("line.", 90.0, 386.0, 25.0, 12.0),
        ];
        let lines = extractor.merge_into_lines(&input);
        assert_eq!(lines.len(), 2, "two distinct baselines must produce two line fragments");
        assert_eq!(lines[0].text, "Hello world now.", "first line concatenated with spaces");
        assert_eq!(lines[1].text, "Next line.", "second line concatenated");
    }

    #[test]
    fn merge_into_lines_inserts_space_only_when_gap_exceeds_threshold() {
        let extractor = TextExtractor::with_options(ExtractionOptions {
            reconstruct_paragraphs: true,
            space_threshold: 0.3,
            ..Default::default()
        });
        // Two fragments with a gap of 4pt (0.33× font_size = above threshold)
        let with_gap = vec![
            tf("AB", 50.0, 400.0, 10.0, 12.0),
            tf("CD", 64.0, 400.0, 10.0, 12.0), // gap = 4pt
        ];
        let lines = extractor.merge_into_lines(&with_gap);
        assert_eq!(lines[0].text, "AB CD", "gap above threshold must insert space");

        // Same shape but gap of 1pt (0.083× — below threshold)
        let tight = vec![
            tf("AB", 50.0, 400.0, 10.0, 12.0),
            tf("CD", 61.0, 400.0, 10.0, 12.0), // gap = 1pt
        ];
        let lines = extractor.merge_into_lines(&tight);
        assert_eq!(lines[0].text, "ABCD", "tight gap must NOT insert space");
    }

    #[test]
    fn merge_into_lines_unioned_bounding_box() {
        let extractor = TextExtractor::with_options(ExtractionOptions {
            reconstruct_paragraphs: true,
            ..Default::default()
        });
        let input = vec![
            tf("A", 50.0, 400.0, 10.0, 12.0),
            tf("B", 100.0, 400.0, 10.0, 12.0),
        ];
        let lines = extractor.merge_into_lines(&input);
        assert_eq!(lines.len(), 1);
        assert!((lines[0].x - 50.0).abs() < 0.01);
        assert!((lines[0].width - 60.0).abs() < 0.01, "width must span 50→110");
    }
```

- [ ] **Step 3.2: Run the new unit tests and confirm they fail**

```bash
cargo test --manifest-path oxidize-pdf-core/Cargo.toml --lib merge_into_lines 2>&1 | tail -20
```

Expected: all three tests fail (the method is still a stub returning `Vec::new()`).

- [ ] **Step 3.3: Implement `merge_into_lines`**

Replace the stub `fn merge_into_lines` with:

```rust
    /// Group fragments by baseline into single-line fragments.
    ///
    /// Two fragments are on the same line when their Y centers differ by less
    /// than half the smaller fragment's height. Within a line, fragments are
    /// sorted left-to-right; a space is inserted between adjacent fragments
    /// when the X gap exceeds `space_threshold * font_size`.
    ///
    /// The output bounding box for each line is the axis-aligned union of the
    /// input fragments' bounding boxes; `font_size` and `font_name` are
    /// inherited from the line's first fragment.
    fn merge_into_lines(&self, fragments: &[TextFragment]) -> Vec<TextFragment> {
        if fragments.is_empty() {
            return Vec::new();
        }

        // Sort by Y descending (PDF top-down), then X ascending
        let mut sorted: Vec<&TextFragment> = fragments.iter().collect();
        sorted.sort_by(|a, b| {
            b.y.total_cmp(&a.y).then(a.x.total_cmp(&b.x))
        });

        let mut lines: Vec<Vec<&TextFragment>> = Vec::new();
        for frag in sorted {
            let placed = lines.last_mut().is_some_and(|line| {
                let head = line[0];
                let tol = (head.height.min(frag.height)) * 0.5;
                (head.y - frag.y).abs() < tol
            });
            if placed {
                lines.last_mut().unwrap().push(frag);
            } else {
                lines.push(vec![frag]);
            }
        }

        lines
            .into_iter()
            .map(|line| build_line_fragment(line, self.options.space_threshold))
            .collect()
    }
```

Then add the helper at module scope (outside `impl TextExtractor`):

```rust
fn build_line_fragment(line: Vec<&TextFragment>, space_threshold: f64) -> TextFragment {
    // Already X-sorted by the caller's outer sort
    let head = line[0];
    let mut text = String::new();
    let mut x_min = head.x;
    let mut x_max = head.x + head.width;
    let mut y_min = head.y;
    let mut y_max = head.y + head.height;

    for (i, frag) in line.iter().enumerate() {
        if i > 0 {
            let prev = line[i - 1];
            let gap = frag.x - (prev.x + prev.width);
            if gap > space_threshold * frag.font_size {
                text.push(' ');
            }
        }
        text.push_str(&frag.text);
        x_min = x_min.min(frag.x);
        x_max = x_max.max(frag.x + frag.width);
        y_min = y_min.min(frag.y);
        y_max = y_max.max(frag.y + frag.height);
    }

    TextFragment {
        text,
        x: x_min,
        y: y_min,
        width: x_max - x_min,
        height: y_max - y_min,
        font_size: head.font_size,
        font_name: head.font_name.clone(),
        is_bold: head.is_bold,
        is_italic: head.is_italic,
        color: head.color,
        space_decisions: Vec::new(),
    }
}
```

- [ ] **Step 3.4: Run the unit tests, confirm green**

```bash
cargo test --manifest-path oxidize-pdf-core/Cargo.toml --lib merge_into_lines 2>&1 | tail -15
```

Expected: 3/3 pass.

- [ ] **Step 3.5: Commit**

```bash
git add oxidize-pdf-core/src/text/extraction.rs
git commit -m "feat: implement merge_into_lines for paragraph reconstruction"
```

---

## Task 4: Implement `merge_into_paragraphs`

**Files:**
- Modify: `oxidize-pdf-core/src/text/extraction.rs`

The paragraph merger groups consecutive lines whose vertical gap is close to the modal leading into a single paragraph fragment, joining lines with `'\n'` (or appending without separator + dropping the trailing hyphen when `merge_hyphenated` applies).

- [ ] **Step 4.1: Write unit tests inside the same `mod tests`**

```rust
    #[test]
    fn merge_into_paragraphs_groups_consecutive_lines() {
        let extractor = TextExtractor::with_options(ExtractionOptions {
            reconstruct_paragraphs: true,
            ..Default::default()
        });
        // Three lines, 14pt leading (line height 12pt, gap 2pt)
        let lines = vec![
            tf("Line one.", 50.0, 400.0, 60.0, 12.0),
            tf("Line two.", 50.0, 386.0, 60.0, 12.0),
            tf("Line three.", 50.0, 372.0, 70.0, 12.0),
        ];
        let paragraphs = extractor.merge_into_paragraphs(&lines);
        assert_eq!(paragraphs.len(), 1);
        assert_eq!(paragraphs[0].text, "Line one.\nLine two.\nLine three.");
    }

    #[test]
    fn merge_into_paragraphs_splits_on_large_vertical_gap() {
        let extractor = TextExtractor::with_options(ExtractionOptions {
            reconstruct_paragraphs: true,
            ..Default::default()
        });
        // First paragraph: y=400, then huge gap, then y=300
        let lines = vec![
            tf("P1L1.", 50.0, 400.0, 40.0, 12.0),
            tf("P1L2.", 50.0, 386.0, 40.0, 12.0),
            tf("P2L1.", 50.0, 300.0, 40.0, 12.0),
        ];
        let paragraphs = extractor.merge_into_paragraphs(&lines);
        assert_eq!(paragraphs.len(), 2);
        assert_eq!(paragraphs[0].text, "P1L1.\nP1L2.");
        assert_eq!(paragraphs[1].text, "P2L1.");
    }

    #[test]
    fn merge_into_paragraphs_drops_hyphen_when_merge_hyphenated() {
        let extractor = TextExtractor::with_options(ExtractionOptions {
            reconstruct_paragraphs: true,
            merge_hyphenated: true,
            ..Default::default()
        });
        let lines = vec![
            tf("Kryp-", 50.0, 400.0, 30.0, 12.0),
            tf("tographie", 50.0, 386.0, 60.0, 12.0),
        ];
        let paragraphs = extractor.merge_into_paragraphs(&lines);
        assert_eq!(paragraphs.len(), 1);
        assert_eq!(paragraphs[0].text, "Kryptographie", "hyphen elided, no newline inserted");
    }
```

- [ ] **Step 4.2: Run the unit tests, confirm they fail**

```bash
cargo test --manifest-path oxidize-pdf-core/Cargo.toml --lib merge_into_paragraphs 2>&1 | tail -15
```

Expected: 3 failures (stub returns empty).

- [ ] **Step 4.3: Implement `merge_into_paragraphs`**

Replace the stub:

```rust
    /// Group consecutive lines into paragraphs based on vertical gap.
    ///
    /// Two consecutive lines are part of the same paragraph when the vertical
    /// gap between them is less than 1.5× the median line height in the
    /// input. Hyphenated line breaks (previous line ends with `-` and
    /// `merge_hyphenated` is set) join without a separator and drop the
    /// hyphen; otherwise lines join with `'\n'`.
    fn merge_into_paragraphs(&self, lines: &[TextFragment]) -> Vec<TextFragment> {
        if lines.is_empty() {
            return Vec::new();
        }

        // Median line height — robust to outliers
        let mut heights: Vec<f64> = lines.iter().map(|l| l.height).collect();
        heights.sort_by(f64::total_cmp);
        let median_h = heights[heights.len() / 2];
        let max_paragraph_gap = median_h * 1.5;

        let mut paragraphs: Vec<TextFragment> = Vec::new();
        let mut current = lines[0].clone();

        for line in &lines[1..] {
            let prev_bottom = current.y;
            let line_top = line.y + line.height;
            let gap = prev_bottom - line_top;

            if gap < 0.0 || gap > max_paragraph_gap {
                paragraphs.push(current);
                current = line.clone();
                continue;
            }

            // Same paragraph — join
            let joined_text = if self.options.merge_hyphenated
                && current.text.ends_with('-')
            {
                let mut s = current.text.clone();
                s.pop(); // drop trailing hyphen
                s.push_str(&line.text);
                s
            } else {
                format!("{}\n{}", current.text, line.text)
            };

            let x_min = current.x.min(line.x);
            let x_max = (current.x + current.width).max(line.x + line.width);
            let y_min = current.y.min(line.y);
            let y_max = (current.y + current.height).max(line.y + line.height);

            current = TextFragment {
                text: joined_text,
                x: x_min,
                y: y_min,
                width: x_max - x_min,
                height: y_max - y_min,
                font_size: current.font_size,
                font_name: current.font_name.clone(),
                is_bold: current.is_bold,
                is_italic: current.is_italic,
                color: current.color,
                space_decisions: Vec::new(),
            };
        }
        paragraphs.push(current);

        paragraphs
    }
```

- [ ] **Step 4.4: Run all the unit tests for the merge functions**

```bash
cargo test --manifest-path oxidize-pdf-core/Cargo.toml --lib "merge_into_" 2>&1 | tail -15
```

Expected: 6/6 pass (3 from Task 3 + 3 from Task 4).

- [ ] **Step 4.5: Run the integration reproducer**

```bash
cargo test --manifest-path oxidize-pdf-core/Cargo.toml --test paragraph_reconstruction_test 2>&1 | tail -15
```

Expected: 6/6 pass (all the Task 1 reproducer tests now turn green).

- [ ] **Step 4.6: Commit**

```bash
git add oxidize-pdf-core/src/text/extraction.rs
git commit -m "feat: implement merge_into_paragraphs with hyphenation support"
```

---

## Task 5: Wire reconstruction into the extraction pipeline

**Files:**
- Modify: `oxidize-pdf-core/src/text/extraction.rs` (the `extract_text_with_options` body)
- Modify: `oxidize-pdf-core/src/parser/document.rs` (partition entry points)

The new public `merge_fragments_for_partition` is one entry point. The internal extraction flow that produces `ExtractedText` also needs to honor `reconstruct_paragraphs` so that `do_partition_pages` sees post-reconstruction fragments.

- [ ] **Step 5.1: Inspect the current extraction flow**

```bash
grep -nE "fn extract_text_with_options|fn extract_text\(" oxidize-pdf-core/src/text/extraction.rs | head -5
grep -nE "self.merge_close_fragments" oxidize-pdf-core/src/text/extraction.rs
```

Locate the call to `merge_close_fragments` inside the main extraction loop (around line 617 per the diagnostic). The fragments variable at that point holds the kerning-merged set.

- [ ] **Step 5.2: Add the reconstruction step after kerning merge**

In `extract_text_with_options` (or whichever method does the per-page extraction and produces the final `ExtractedText`), find the block:

```rust
            // Merge close fragments to eliminate spacing artifacts
            // This is crucial for table detection and structured data extraction
            if self.options.preserve_layout && !fragments.is_empty() {
                fragments = self.merge_close_fragments(&fragments);
            }
```

Replace with:

```rust
            // Merge close fragments to eliminate spacing artifacts (kerning fix)
            if self.options.preserve_layout && !fragments.is_empty() {
                fragments = self.merge_close_fragments(&fragments);
            }

            // Reconstruct visual lines and paragraphs from raw fragments.
            // Required for the partition pipeline to produce Element values at
            // paragraph granularity (issue #261).
            if self.options.reconstruct_paragraphs && !fragments.is_empty() {
                let lines = self.merge_into_lines(&fragments);
                fragments = self.merge_into_paragraphs(&lines);
            }
```

- [ ] **Step 5.3: Force `reconstruct_paragraphs: true` in partition entry points**

Open `oxidize-pdf-core/src/parser/document.rs`. Find both `partition_with` (around line 1524) and `partition_with_profile` (around line 1536). Each constructs `ExtractionOptions` with `preserve_layout: true`. Add the new flag in both places.

In `partition_with` (line ~1528-1531), change:

```rust
        let options = crate::text::ExtractionOptions {
            preserve_layout: true,
            ..Default::default()
        };
```

to:

```rust
        let options = crate::text::ExtractionOptions {
            preserve_layout: true,
            reconstruct_paragraphs: true,
            ..Default::default()
        };
```

In `partition_with_profile` (line ~1541-1546), change:

```rust
        let options = crate::text::ExtractionOptions {
            preserve_layout: true,
            space_threshold: profile_cfg.extraction.space_threshold,
            detect_columns: profile_cfg.extraction.detect_columns,
            ..crate::text::ExtractionOptions::default()
        };
```

to:

```rust
        let options = crate::text::ExtractionOptions {
            preserve_layout: true,
            reconstruct_paragraphs: true,
            space_threshold: profile_cfg.extraction.space_threshold,
            detect_columns: profile_cfg.extraction.detect_columns,
            ..crate::text::ExtractionOptions::default()
        };
```

- [ ] **Step 5.4: Build and check for compilation**

```bash
cargo build --manifest-path oxidize-pdf-core/Cargo.toml --all-targets 2>&1 | tail -10
```

Expected: clean build.

- [ ] **Step 5.5: Commit the wiring**

```bash
git add oxidize-pdf-core/src/text/extraction.rs oxidize-pdf-core/src/parser/document.rs
git commit -m "feat: wire paragraph reconstruction into partition pipeline (#261)"
```

---

## Task 6: Library regression — confirm existing tests still pass

The change in extraction default behavior is opt-in via `reconstruct_paragraphs: false` by default. But the partition path now forces it true, so any existing test that exercises `partition()`, `rag_chunks()`, `partition_with_profile`, etc. may see different output than before.

**Files:**
- Read and possibly modify: existing tests in `oxidize-pdf-core/tests/` that exercise the partition/RAG pipeline.

- [ ] **Step 6.1: Run the full library suite**

```bash
cargo test --manifest-path oxidize-pdf-core/Cargo.toml --lib 2>&1 | tail -15
```

Expected: 6388 passed (or whatever the current baseline is). If new failures appear:

- Inspect each failing test. If it asserts on fragment-level granularity that the partition path produced (e.g., element counts >> reality), the test was encoding the bug — adjust the assertion to match correct paragraph-level behavior.
- If the test exercises `extract_text` (not `partition`) and assumes per-fragment output, those continue to work because `reconstruct_paragraphs` defaults to `false`.

For each test you modify, explain the change in the commit message.

- [ ] **Step 6.2: Run the full integration suite**

```bash
cargo test --manifest-path oxidize-pdf-core/Cargo.toml --tests 2>&1 | tail -20
```

Look for failures specifically in:
- `tests/rag_chunk_test.rs`
- `tests/hybrid_chunking_test.rs`
- `tests/hybrid_chunking_graph_test.rs`
- `tests/semantic_chunking_test.rs`
- `tests/element_relationships_test.rs`
- `tests/element_graph_integration_test.rs`
- `tests/chunk_page_mapper_test.rs`

These are the most likely to depend on partition granularity. The tests build synthetic `Element` values directly and feed them to the chunker, so they should NOT be affected by extraction changes. Confirm by reading each.

If real test failures appear (not just count differences but actual breakage), STOP and report. Do not paper over.

- [ ] **Step 6.3: Commit any test adjustments**

If you needed to modify any existing test, commit them with descriptive messages:

```bash
git add <paths>
git commit -m "test: adjust <test name> for paragraph-granularity partition output"
```

If nothing needed adjusting, no commit — skip to Task 7.

---

## Task 7: Live verification on the real corpus

This task confirms the fix produces sensible output on real PDFs. We do not commit anything here — it is verification. The PDFs are downloaded to a scratch location, the partition pipeline is run, and the resulting chunk distribution is inspected.

- [ ] **Step 7.1: Switch temporarily to the rag_realworld example branch**

```bash
git stash 2>/dev/null  # nothing should be uncommitted; this is defensive
CURRENT_SHA=$(git rev-parse HEAD)
git checkout feature/rag-realworld-rust
git merge --no-ff fix/issue-261-paragraph-reconstruction -m "wip: bring in #261 fix for verification"
```

If the merge has conflicts, abort with `git merge --abort` and report BLOCKED — the branches were not meant to conflict, so there is something to investigate.

- [ ] **Step 7.2: Re-run the example against the real corpus**

```bash
cargo run --manifest-path oxidize-pdf-core/Cargo.toml --example rag_realworld 2>&1
```

Expected outcome (relative to the pre-fix baseline):
- ENS: was 8279 chunks @ 4.6 tok avg; expect drop to roughly 100–500 chunks at 50–500 tok avg.
- boe-sumario: was 1066 @ 24 tok; expect lower count, higher avg.
- BSI: was 1674 @ 27 tok with 1477 of them being exactly 1 token; expect dramatic improvement, very few 1-tok chunks.
- NCSC: was 12180 @ 6 tok; expect order-of-magnitude reduction.
- Higgs: still fails on parser bug #260 (out of scope here).

Inspect a few JSONL lines manually to confirm chunks are paragraph-shaped, not word-shaped:

```bash
head -3 out/ens.jsonl | python3 -c "
import json, sys
for line in sys.stdin:
    d = json.loads(line)
    print(f'{d[\"id\"]} tokens={d[\"metadata\"][\"token_estimate\"]} text={d[\"text\"][:120]!r}')
"
```

Expected: text starts to look like coherent paragraphs, not isolated words.

- [ ] **Step 7.3: Undo the merge — fix branch must ship pure**

```bash
git reset --hard $CURRENT_SHA  # this undoes the merge commit
git checkout fix/issue-261-paragraph-reconstruction
```

The verification merge was strictly for measuring the fix's effect on real PDFs. The fix PR ships on its own.

- [ ] **Step 7.4: Record the measurement in a file under the plan dir for the PR description**

Create `docs/superpowers/plans/2026-05-20-issue-261-verification.md` with the before/after table from your measurement. The PR body in Task 9 references it.

```bash
git add docs/superpowers/plans/2026-05-20-issue-261-verification.md
git commit -m "docs: before/after verification of #261 fix against real corpus"
```

---

## Task 8: Pre-PR clean run

- [ ] **Step 8.1: Branch state**

```bash
git log --oneline develop..HEAD
git status --short
```

Expected: clean working tree, commits in order:
1. test: TDD reproducer
2. feat: add reconstruct_paragraphs + stubs
3. feat: implement merge_into_lines
4. feat: implement merge_into_paragraphs
5. feat: wire reconstruction into partition pipeline
6. test: adjust existing tests (if any)
7. docs: before/after verification

- [ ] **Step 8.2: Run all tests, clippy, build**

```bash
cargo build --manifest-path oxidize-pdf-core/Cargo.toml --all-targets 2>&1 | tail -5
cargo test --manifest-path oxidize-pdf-core/Cargo.toml --lib 2>&1 | tail -5
cargo test --manifest-path oxidize-pdf-core/Cargo.toml --tests 2>&1 | tail -5
cargo clippy --manifest-path oxidize-pdf-core/Cargo.toml --all-targets -- -D warnings 2>&1 | tail -10
```

Expected: all clean.

- [ ] **Step 8.3: Diff size sanity check**

```bash
git diff --stat develop..HEAD
```

Expected: changes concentrated in `extraction.rs`, `document.rs`, and the new test file. Total diff under ~600 LOC including tests.

---

## Task 9: Draft PR body (no push)

**Files:**
- Create: `docs/superpowers/plans/2026-05-20-issue-261-pr-body.md`

- [ ] **Step 9.1: Write the PR body**

Create the file with:

```markdown
## Summary
Fixes #261. Adds line + paragraph reconstruction to the text extraction
pipeline so that `partition()`, `rag_chunks()`, and `rag_chunks_with_profile`
produce Elements at paragraph granularity instead of one Element per `Tj`
operator.

## Approach
Two new private methods on `TextExtractor`:
- `merge_into_lines(fragments)` — groups by baseline (Y tolerance ≈ ½ line
  height), sorts by X within each line, concatenates with spaces where
  `x_gap > space_threshold * font_size`.
- `merge_into_paragraphs(lines)` — groups consecutive lines whose vertical
  gap is within 1.5× the median line height, joining with `\n` or eliding
  the hyphen when `merge_hyphenated` applies.

Gated by a new `ExtractionOptions::reconstruct_paragraphs: bool`, default
`false` to preserve backward compatibility for direct `extract_text` callers.
`partition_with` and `partition_with_profile` force it to `true`.

## Verification (against real corpus)
See `docs/superpowers/plans/2026-05-20-issue-261-verification.md`. Summary:

| PDF | Before chunks / avg tok | After chunks / avg tok |
|---|---|---|
| ENS (BOE) | 8279 / 4.6 | <fill from measurement> |
| BOE sumario | 1066 / 24.0 | <fill> |
| BSI TR-02102 | 1674 / 27.5 (88 % 1-tok) | <fill> |
| NCSC CAF | 12180 / 6.3 (95 % 1-tok) | <fill> |

## Test plan
- [x] 6 unit tests on `merge_into_lines` and `merge_into_paragraphs` inside
  `extraction.rs` (synthetic fragments).
- [x] 6 integration tests in `tests/paragraph_reconstruction_test.rs`
  exercising `merge_fragments_for_partition` through the public API.
- [x] Full library test suite passes (no regressions).
- [x] Live verification against ENS, BOE sumario, BSI TR-02102, NCSC CAF.

## Out of scope
- arXiv/TeX `Sendstream` parser issue (#260) — fixed separately.
- The downstream `rag_realworld` example (PR #2) — ships once this lands.
```

Fill the `<fill>` markers using the actual numbers from your Task 7 measurement.

- [ ] **Step 9.2: Commit the PR body draft**

```bash
git add docs/superpowers/plans/2026-05-20-issue-261-pr-body.md
git commit -m "docs: draft PR body for #261 fix"
```

- [ ] **Step 9.3: Halt before push**

Per `CLAUDE.local.md`, do not push or open the PR without explicit user authorization. Report the final HEAD SHA and the PR body file path. The user runs:

```bash
git push -u origin fix/issue-261-paragraph-reconstruction
gh pr create --base develop --title "fix: paragraph reconstruction in text extraction (#261)" --body "$(cat docs/superpowers/plans/2026-05-20-issue-261-pr-body.md)"
```

---

## Self-review checklist

- [x] **Spec coverage:** Every part of the issue's "Suggested fix" maps to a task — line merger (Task 3), paragraph merger (Task 4), pipeline wiring (Task 5), tests (Task 1, 3, 4), verification (Task 7).
- [x] **No placeholders.** Every code block is complete. The two `TODO` comments in Task 2 are scoped pointers to Task 3/4 and explicitly noted as temporary.
- [x] **Type consistency.** `TextFragment`, `ExtractionOptions`, `TextExtractor::with_options`, `merge_fragments_for_partition`, `merge_into_lines`, `merge_into_paragraphs` defined exactly once and referenced consistently across tasks.
- [x] **Frequent commits.** Eight commits planned: 1 test + 4 feature + 1 (optional) test-adjust + 2 docs.
- [x] **TDD discipline.** Task 1 writes the failing reproducer first (committed together with the type plumbing in Task 2 since the test won't compile without the field). Tasks 3 and 4 each write internal unit tests before implementing the function.
- [x] **No smoke tests.** Every assertion verifies a content-bearing claim — exact strings, exact counts, exact widths. Nothing checks "is non-empty" or "didn't panic."

---

## Out of scope
- The arXiv/TeX `Sendstream` parser bug (#260) — separate PR.
- The `rag_realworld` example — already implemented on a parallel branch, sits until this PR lands then ships as its own PR.
- Multi-column reconstruction — when `detect_columns: true` is also set, the existing column detector runs before the new line merger. Composing the two correctly is a future enhancement.
