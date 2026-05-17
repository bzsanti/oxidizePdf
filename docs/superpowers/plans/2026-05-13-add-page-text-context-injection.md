# Plan: Fix `Document::add_page` text_context font metrics injection (v2.8.1)

## Context

`Document::add_page` injects the per-Document `FontMetricsStore` into
`page.font_metrics_store` but does NOT inject it into
`page.text_context.font_metrics_store`. The doc comment at
`document.rs:143–149` promises that "their text_flow / text contexts can
resolve custom fonts via Document scope" — the promise is partially broken.

Pages constructed via `Page::a4()`, `Page::letter()`, or `Page::new(...)` have
`text_context: TextContext::new()` (page.rs:167) which carries
`font_metrics_store: None`. After `doc.add_page(page)`, the page-level
`font_metrics_store` is correctly set, but `text_context.font_metrics_store`
remains `None`. Any current or future code that reads
`text_context.font_metrics_store` directly (e.g., measuring text via the
`TextContext` rather than via `TextFlowContext`) falls through to the legacy
global registry — the cross-Document leak that issue #230 closed
architecturally.

The factory path (`Document::new_page_a4/letter/new_page`) is correct: it
calls `p.text_context = TextContext::with_metrics_store(Some(store.clone()))`,
so both fields are set. The bug is confined to the `Page::*() + doc.add_page`
path.

**Stack detected**: Rust / no external framework  
**Conventions**: integration tests in `oxidize-pdf-core/tests/`, no inline
`#[cfg(test)] mod tests`, filenames keyed to issue or feature,
`pub(crate)` for intra-crate visibility, `#[cfg(test)] pub(crate)` for test
helpers accessible only within unit tests.  
**Affects hot path**: No — `add_page` is a document-construction step, not a
measurement or rendering hot path. No throughput benchmarks are required.

## Decisions already made (no alternatives to explore)

- Fix mutates only `text_context.font_metrics_store` in-place via a new
  `TextContext::set_metrics_store(&mut self, store: Option<FontMetricsStore>)`
  method. This preserves any ops accumulated in `text_context` before
  `add_page` is called.
- `set_metrics_store` is `pub(crate)` — same visibility as
  `with_metrics_store`.
- The guard in `add_page` mirrors the existing `page.font_metrics_store`
  guard: only inject when `text_context.font_metrics_store.is_none()`. This
  preserves the invariant that factory-produced pages (already carrying a
  store) are not overwritten.
- `TextFlowContext` has the same field and the same bug. It also gets a
  `set_metrics_store` and is injected from `add_page` via the same guard.
- No public API changes. Target: `v2.8.1` patch.

## Sibling structure investigation result

`TextFlowContext` (`src/text/flow.rs:16`) has `pub(crate) font_metrics_store:
Option<FontMetricsStore>` (line 53). It is NOT stored on `Page` as a field —
pages create a new `TextFlowContext` on each call to `page.text_flow()` (line
929), and that method already reads `self.font_metrics_store` (line 945), NOT
`self.text_context.font_metrics_store`. Therefore, `TextFlowContext` is not
affected by this bug via `page.text_flow()`. However, `TextFlowContext` itself
has the same gap: if a caller constructs a `TextFlowContext::new(...)` and
later tries to set a store, no setter exists. Adding `set_metrics_store` to
`TextFlowContext` is symmetric hygiene, but it is NOT required by the current
bug.

**Decision**: Add `TextContext::set_metrics_store` (required by the fix).
Do NOT add `TextFlowContext::set_metrics_store` — no current caller and
adding "symmetric hygiene" violates the project's no-speculative-API rule.

---

## Plan of Execution

### Phase 1 — RED: write the failing integration test

**Cycle 1.1 — Integration test that reproduces the bug**

- [ ] Create file `oxidize-pdf-core/tests/issue_230_text_context_injection_test.rs`
  - Test name: `add_page_injects_store_into_text_context`
  - Purpose: prove `text_context.font_metrics_store` is `None` after
    `add_page` on v2.8.0 and `Some` after the fix.
  - Setup:
    ```
    // Plant synthetic metrics for font "TC_Wide" in the legacy global
    // (deprecated API, #[allow(deprecated)]). Width of 'A' = 100 units
    // → measure_text_with("A", font, 12.0, None) = 1.2 pts.
    register_custom_font_metrics(
        "TC_Wide_5_1".to_string(),
        FontMetrics::new(500).with_widths(&[('A', 100)]),
    );

    // Register the same name in the Document store with 'A' = 800 units
    // → 9.6 pts. A numerical gap of >5 pts ensures the test would fail
    // if the wrong store is used.
    let mut doc = Document::new();
    doc.font_metrics()
        .register("TC_Wide_5_1", FontMetrics::new(500).with_widths(&[('A', 800)]));

    let page = Page::a4(); // text_context.font_metrics_store = None here
    doc.add_page(page);

    let stored_page = doc.pages().last().expect("page");
    ```
  - Assertion (using a new `pub(crate)` accessor — see Phase 2, task 2.1):
    ```
    let tc_store = stored_page
        .text_context_metrics_store_for_test()
        .expect("text_context must carry the Document store after add_page");
    let doc_width = measure_text_with(
        "A",
        &Font::Custom("TC_Wide_5_1".into()),
        12.0,
        Some(tc_store),
    );
    let global_width = measure_text_with(
        "A",
        &Font::Custom("TC_Wide_5_1".into()),
        12.0,
        None,
    );
    // Document store width (800 units) ≠ global width (100 units).
    // If the fix is absent, tc_store would be None and the test panics
    // at the .expect() above.
    assert!(
        (global_width - 1.2_f64).abs() < 0.05,
        "global store width must be 1.2 pts for 'A' at 12pt; got {global_width}"
    );
    assert!(
        (doc_width - 9.6_f64).abs() < 0.05,
        "text_context store width must be 9.6 pts for 'A' at 12pt; got {doc_width}"
    );
    assert!(
        (doc_width - global_width).abs() > 5.0,
        "document scope and global scope must differ by >5 pts; \
         doc={doc_width}, global={global_width}"
    );
    ```
  - **Expected state on v2.8.0**: RED — `.expect("text_context must carry ...")` panics.
  - Imports needed:
    ```rust
    use oxidize_pdf::text::metrics::{FontMetrics, FontMetricsStore};
    use oxidize_pdf::text::{measure_text_with, Font};
    use oxidize_pdf::{Document, Page};
    ```
  - Note: `text_context_metrics_store_for_test()` does not yet exist on `Page`.
    The test file will not compile until Phase 2 task 2.1 is complete.

**Cycle 1.2 — Ops-preservation test**

- [ ] Add test `text_context_ops_preserved_across_add_page` to the same file.
  - Setup: call `page.text().set_font(Font::Helvetica, 14.0).at(50.0, 700.0).write("hello")` BEFORE `doc.add_page(page)`.
  - After `doc.add_page(page)`, obtain the stored page and call
    `stored_page.text_context_ops_count_for_test()` (second accessor, Phase 2 task 2.1).
  - Assertion: ops count > 0 (the `write("hello")` accumulated at least one op;
    the exact count is not asserted — only that it is non-zero, since the
    operation sequence is an implementation detail).
  - **Expected state on v2.8.0**: RED (accessor does not exist).
  - After fix: GREEN with ops count unchanged.

**Cycle 1.3 — Factory path regression test**

- [ ] Add test `factory_path_text_context_store_unchanged` to the same file.
  - Create a page via `doc.new_page_a4()`, confirm
    `text_context_metrics_store_for_test()` is `Some` before `add_page`
    (factory already sets it).
  - Call `doc.add_page(page)` and confirm the store is still `Some` pointing
    to the same document (verify via a width measurement, not pointer equality).
  - This test verifies the `is_none()` guard in `add_page` does NOT
    overwrite the factory-supplied store.
  - **Expected state on v2.8.0**: RED (accessor does not exist).
  - After fix: GREEN.

---

### Phase 2 — GREEN: production code changes

**Cycle 2.1 — Add `Page::text_context_metrics_store_for_test` and
`Page::text_context_ops_count_for_test` test accessors**

- [ ] File: `oxidize-pdf-core/src/page.rs`
  - Action: add two `#[doc(hidden)] pub` methods to `impl Page`. They MUST
    be `pub` (not `pub(crate)`) so the integration test crate can see them,
    and MUST NOT be `#[cfg(test)]` because integration tests link the
    library compiled without `--test`. `#[doc(hidden)]` keeps them out of
    rustdoc — the standard "exposed for tests only" pattern in Rust.
    ```rust
    #[doc(hidden)]
    pub fn text_context_metrics_store_for_test(
        &self,
    ) -> Option<&crate::text::metrics::FontMetricsStore> {
        self.text_context.font_metrics_store.as_ref()
    }

    #[doc(hidden)]
    pub fn text_context_ops_count_for_test(&self) -> usize {
        self.text_context.ops_slice().len()
    }
    ```
  - Location: adjacent to the existing public `Page::font_metrics_store()`
    accessor near line 564.
  - Output: the two integration tests from Phase 1 compile (they still fail at
    runtime since the fix is not yet applied).

**Cycle 2.2 — Add `TextContext::set_metrics_store`**

- [ ] File: `oxidize-pdf-core/src/text/mod.rs`
  - Action: add method to `impl TextContext`, adjacent to `with_metrics_store`:
    ```rust
    /// Inject or replace the per-Document `FontMetricsStore` on an
    /// already-constructed context.
    ///
    /// Called by `Document::add_page` when the page was constructed via
    /// `Page::a4()` / `Page::letter()` / `Page::new()` (those start with
    /// `font_metrics_store: None`). Accumulated ops are preserved.
    pub(crate) fn set_metrics_store(&mut self, store: Option<FontMetricsStore>) {
        self.font_metrics_store = store;
    }
    ```
  - Output: method available to `document.rs`.

**Cycle 2.3 — Fix `Document::add_page`**

- [ ] File: `oxidize-pdf-core/src/document.rs`, lines 142–163
  - Action: inside the `if page.font_metrics_store.is_none()` block, also
    call `set_metrics_store` on `text_context`:
    ```rust
    if page.font_metrics_store.is_none() {
        page.font_metrics_store = Some(self.font_metrics.clone());
        page.text_context
            .set_metrics_store(Some(self.font_metrics.clone()));
    }
    ```
  - The guard is shared: if `page.font_metrics_store` is `Some` (factory
    path), neither assignment runs — preserving the factory-supplied stores.
  - Output: all three tests from Phase 1 turn GREEN.

**Cycle 2.4 — Update the `add_page` doc comment**

- [ ] File: `oxidize-pdf-core/src/document.rs`, lines 143–149
  - Action: replace the comment body to accurately describe both injections:
    ```rust
    // Inject the Document's metrics store into the page if it does not
    // already carry one. Pages constructed via Document::new_page_*()
    // already carry a store (on both page.font_metrics_store AND
    // page.text_context.font_metrics_store) and are skipped — preserving
    // bindings to other Documents if a page is moved.
    //
    // Pages constructed via Page::a4() / Page::letter() / Page::new()
    // start with both fields as None; both are set here so that
    // text_context.font_metrics_store resolves custom fonts via the
    // Document scope rather than the legacy global registry.
    ```

---

### Phase 3 — Regression sweep: existing test suite must stay GREEN

**Cycle 3.1 — Verify `font_metrics_per_document_test.rs`**

- [ ] Run: `cargo test -p oxidize-pdf --test font_metrics_per_document_test`
- Expected: all 9 tests pass, including `add_page_does_not_overwrite_existing_store` (test 3.3).
  - That test uses `doc.new_page_a4()` (factory path), calls `doc_b.add_page(page)`, and asserts the store from `doc_a` is kept and `doc_b` did not override it. The new code path only runs when `page.font_metrics_store.is_none()` — factory pages have `Some`, so the block is skipped. GREEN unchanged.

**Cycle 3.2 — Verify `font_metrics_per_document_render_test.rs`**

- [ ] Run: `cargo test -p oxidize-pdf --test font_metrics_per_document_render_test`
- Expected: tests 4.1 and 4.2 pass unchanged (they use `doc.new_page_a4()`, unaffected by the fix).

**Cycle 3.3 — Run full test suite**

- [ ] Run: `cargo test -p oxidize-pdf`
- Expected: no regressions. Any pre-existing failing test that was already broken before this change is out of scope for this plan.

---

### Phase 4 — Release preparation

**Cycle 4.1 — CHANGELOG entry**

- [ ] File: `CHANGELOG.md`
  - Action: under the `## [Unreleased]` header (line 9), add a `### Fixed` subsection (or append to it if one already exists):
    ```markdown
    ### Fixed

    - `Document::add_page` now also injects the per-Document font metrics
      store into `page.text_context.font_metrics_store` when the page was
      constructed via `Page::a4()` / `Page::letter()` / `Page::new()`. Before
      this fix, `text_context.font_metrics_store` remained `None` after
      `add_page`, causing any code that reads widths from the text context
      directly to fall through to the legacy global registry — re-introducing
      the cross-Document leak that issue #230 was intended to close
      architecturally. The factory methods (`Document::new_page_a4()`,
      `new_page_letter()`, `new_page()`) were unaffected. Issue #230
      follow-up.
    ```

**Cycle 4.2 — Version bump**

- [ ] File: `Cargo.toml` (workspace root), line 13
  - Action: change `version = "2.8.0"` to `version = "2.8.1"`.
  - This propagates to all crates via `version.workspace = true`.
  - Output: `cargo check --all-targets` passes with no version mismatches.

**Cycle 4.3 — Final compile and test verification**

- [ ] Run: `cargo check --all-targets 2>&1` — zero errors, zero warnings.
- [ ] Run: `cargo test -p oxidize-pdf --test issue_230_text_context_injection_test` — 3 tests GREEN.
- [ ] Run: `cargo test -p oxidize-pdf --test font_metrics_per_document_test` — 9 tests GREEN.
- [ ] Confirm no other test file references `add_page` in a way that would be broken by the new `text_context.set_metrics_store` call:
  ```bash
  grep -rn "add_page" oxidize-pdf-core/tests/ | grep -v "//\|#\[test\]" | head -30
  ```

---

## Estimation

| Phase | Work |
|---|---|
| Phase 1 — RED tests | 25 min |
| Phase 2 — GREEN production code | 20 min |
| Phase 3 — Regression sweep | 10 min |
| Phase 4 — Release prep | 10 min |
| **Total** | **~65 min** |

No throughput benchmarks required (this is not a hot path).

---

## Criteria of Success

- [ ] `issue_230_text_context_injection_test.rs` compiles and all 3 tests pass.
- [ ] `font_metrics_per_document_test.rs` 9 tests still pass, including `add_page_does_not_overwrite_existing_store`.
- [ ] `cargo check --all-targets` produces zero errors and zero warnings.
- [ ] CHANGELOG has a `### Fixed` entry under `[Unreleased]`.
- [ ] Workspace version is `2.8.1`.

---

## File Reference

| File | Action |
|---|---|
| `oxidize-pdf-core/tests/issue_230_text_context_injection_test.rs` | CREATE — 3 new integration tests |
| `oxidize-pdf-core/src/page.rs` | MODIFY — 2 new `#[cfg(test)] pub(crate)` accessors |
| `oxidize-pdf-core/src/text/mod.rs` | MODIFY — add `TextContext::set_metrics_store` |
| `oxidize-pdf-core/src/document.rs` | MODIFY — `add_page` body + doc comment |
| `CHANGELOG.md` | MODIFY — `[Unreleased]` Fixed entry |
| `Cargo.toml` (workspace root) | MODIFY — `2.8.0` → `2.8.1` |
