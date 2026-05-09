# Per-Document Font Metrics — Design

**Date**: 2026-05-07
**Author**: BelowZero (santiago.fernandez@belowzero.tech)
**Issue**: [#230](https://github.com/bzsanti/oxidizePdf/issues/230) — Add `unregister_custom_font_metrics(name)` to bound the global custom-font metrics registry
**Target release**: v2.8.0
**Status**: design approved, awaiting implementation plan

## Background

`oxidize-pdf` ≤ 2.7.x stores custom-font measurement metrics in a process-wide
`lazy_static` registry (`text::metrics::CUSTOM_FONT_METRICS`). The registry exposes
`register_custom_font_metrics(name, metrics)` and `get_custom_font_metrics(name)`
but no unregister, remove, clear, or scope-bounded equivalent. Every call to
`Document::add_font_from_bytes` plants a permanent entry. Drop of the `Document`
does not remove it.

This produces three independent defects:

1. **Memory growth without bound.** A long-running service that registers
   custom fonts per request leaks 5 KiB (Latin TTF) – 500 KiB (CJK TTF) per
   registration. Realistic loads OOM 1–2 GiB containers within hours.
2. **Last-writer-wins under concurrent races on the same name.** Two
   `Document`s registering the same name end up sharing whichever metrics
   wrote last; the loser's `Document` renders with the winner's metrics.
3. **Cross-`Document` name leak after drop.** Registering name X in
   `Document` A and dropping A still resolves X from a fresh `Document` B
   with no registration of its own.

Issue #230 proposed three fix options; this spec implements **Option 2**
(per-`Document` scoped metrics with the global registry retained as a
deprecated fallback). Option 1 (`unregister_*` band-aid) was rejected because
it leaves defects 2 and 3 active and accumulates technical debt: the moment
Option 2 lands, the symmetric `register/unregister` API becomes redundant.

A fourth, undocumented defect surfaced during exploration:
`text::metrics::get_font_metrics` (`metrics.rs:228-256`) auto-registers default
metrics into the global on every read miss for an unknown `Font::Custom(name)`.
This converts measurement (a read path) into a write to the global registry.
Any `measure_text(text, &Font::custom("Typo"), 12.0)` plants an entry. This
spec also fixes that vector.

## Goals

- Bound the lifetime of custom-font metrics to the `Document` they belong to.
- Make per-`Document` namespaces independent: same name in different documents
  must not collide.
- Keep the v2.x public API non-breaking. Existing callers using only
  documented APIs (`Document::add_font_from_bytes`, `Page::a4()`,
  `page.text_flow()`, etc.) get the fix automatically with no source changes.
- Eliminate the read-path auto-registration in `get_font_metrics`.
- Provide a clean migration path for callers who currently call the
  deprecated global `register_custom_font_metrics` directly.

## Non-goals

The following are explicitly **not** part of v2.8.0:

- Removing the global registry. The deprecated API stays public and
  functional. Removal is queued for v3.0 as a separate issue.
- Cross-`Document` sharing of a single `FontMetricsStore` (advanced caller
  path, not requested, trivially addable later via the existing Arc-clonable
  shape).
- Changing the lookup of standard fonts (`FONT_METRICS` lazy_static for the
  base-14): out of scope; the issue is custom fonts.
- Refactoring `text/metrics.rs` file structure beyond the surfaces touched
  by this spec.
- Backporting to 2.7.x.
- Resolving issues #218 (Table overflow) or #212 (Type0/CID widget AP).

## Architecture

```
+---------------------+
|     Document        |
|---------------------|
|  custom_fonts:      |       (unchanged: parsed font data, glyph mapping)
|    FontCache        |
|---------------------|
|  font_metrics:      |  ← NEW (this spec)
|    FontMetricsStore |
+---------------------+
       |  Document::add_page(page)  →  inject self.font_metrics into page if not bound
       |  Document::new_page_a4()   →  return Page already bound to self.font_metrics
       v
+---------------------+
|        Page         |
|---------------------|
|  font_metrics_      |  ← NEW (Option<FontMetricsStore>)
|    store:           |
|    Option<...>      |
+---------------------+
       |  page.text_flow()  →  TextFlowContext { ..., font_metrics_store: page.font_metrics_store.clone() }
       |  page.text()       →  TextContext     { ..., font_metrics_store: page.font_metrics_store.clone() }
       v
+----------------------+
| TextFlowContext /    |
| TextContext          |
|----------------------|
|  font_metrics_store: |  ← NEW (Option<FontMetricsStore>)
|    Option<...>       |
+----------------------+
       |  measure_text_with(text, &font, size, store_opt)
       v
+---------------------------------------------------------------+
|  metrics::lookup(font, store_opt) -> Arc<FontMetrics>         |
|  ──────────────────────────────────────────────────────────── |
|  match font {                                                 |
|    Standard → FONT_METRICS lazy_static (existing global)      |
|    Custom(name) →                                             |
|      1. store_opt.and_then(|s| s.get(name))   ← Document scope (precedence)
|      2. legacy_global_get(name)               ← deprecated fallback
|      3. default + warn-once-per-name          ← read miss, NO register
|  }                                                            |
+---------------------------------------------------------------+
```

### Threading model

`Page` is constructed independently and added to `Document` via
`Document::add_page(page)`. The `Page` does not hold a reference to its
`Document`; the relationship is one-way (Document → Pages). This forces a
choice for how a `Page` acquires the metrics store.

**Adopted strategy: factory + on-add fallback.**

- **Canonical path**: `Document::new_page_a4()` (and siblings) return a
  `Page` already bound to the Document's `FontMetricsStore`. This is the
  recommended API for any code using custom fonts. Documented in the
  migration guide.
- **Back-compat path**: `Document::add_page(page)` injects the Document's
  store into the page if (and only if) the page does not already carry one.
  This covers callers using the legacy `Page::a4()` + `add_page` flow when
  measurements happen *after* `add_page`.
- **Known gap (documented in migration guide)**: callers who measure
  *before* adding the page to the Document (the typical `Page::a4()` then
  `text_flow()` then `add_page` flow) get the legacy global path or the
  default+warn fallback for those mid-construction measurements. The fix
  is to migrate to `Document::new_page_a4()`. The diagnostic warning emitted
  on read-miss makes this discoverable.

The factory + on-add combination was chosen over alternatives:

- *Bind explicitly* (`page.bind_metrics_from(&doc)`) requires a separate
  call that callers will forget; falls back to global silently.
- *On-add only* breaks the typical mid-construction measurement flow and
  cannot be repaired without rewriting content streams already emitted.

## Components

### `FontMetricsStore` (new public type, `text::metrics`)

```rust
/// Per-Document store of custom font metrics. Cheap to clone (Arc-backed).
/// Lifetime is bound to the owning Document — drop the Document, the metrics go.
#[derive(Clone, Debug)]
pub struct FontMetricsStore {
    inner: Arc<RwLock<HashMap<String, Arc<FontMetrics>>>>,
}

impl FontMetricsStore {
    pub fn new() -> Self;

    /// Register or replace metrics for `font_name`. Last-writer-wins
    /// within a single Document; concurrent calls require external
    /// synchronization (Document::add_font_from_bytes is &mut self,
    /// so safe code cannot race within one Document).
    pub fn register(&self, font_name: impl Into<String>, metrics: FontMetrics);

    /// Lookup. None on miss. No side effects.
    pub fn get(&self, font_name: &str) -> Option<Arc<FontMetrics>>;

    /// Number of registered fonts. Diagnostic / test introspection.
    pub fn len(&self) -> usize;

    pub fn is_empty(&self) -> bool;
}

impl Default for FontMetricsStore { /* delegates to new() */ }
```

### `Document` (modified, `oxidize-pdf-core/src/document.rs`)

```rust
pub struct Document {
    // ...existing fields...
    pub(crate) custom_fonts: FontCache,
    pub(crate) font_metrics: FontMetricsStore,  // ← NEW
    // ...
}

impl Document {
    /// Public signature unchanged. Internal change: registers into
    /// self.font_metrics (per-Document) instead of CUSTOM_FONT_METRICS (global).
    pub fn add_font_from_bytes(&mut self, name: impl Into<String>, data: Vec<u8>) -> Result<()>;

    // NEW factory methods (canonical path)
    pub fn new_page_a4(&self) -> Page;
    pub fn new_page_letter(&self) -> Page;
    pub fn new_page(&self, width: f64, height: f64) -> Page;

    // Existing — adds store injection (back-compat fallback)
    pub fn add_page(&mut self, mut page: Page) {
        if page.font_metrics_store.is_none() {
            page.font_metrics_store = Some(self.font_metrics.clone());
        }
        // ...rest of existing add_page logic...
    }
}
```

### `Page` (modified, `oxidize-pdf-core/src/page.rs`)

```rust
pub struct Page {
    // ...existing fields...
    pub(crate) font_metrics_store: Option<FontMetricsStore>,  // ← NEW
    // ...
}

impl Page {
    // Existing constructors: store = None
    pub fn a4() -> Self;
    pub fn letter() -> Self;
    pub fn new(width: f64, height: f64) -> Self;

    // pub(crate) constructors used by Document factory methods
    pub(crate) fn a4_with_metrics(store: FontMetricsStore) -> Self;
    pub(crate) fn letter_with_metrics(store: FontMetricsStore) -> Self;
    pub(crate) fn new_with_metrics(width: f64, height: f64, store: FontMetricsStore) -> Self;

    // Existing — propagates the handle to the context
    pub fn text_flow(&self) -> TextFlowContext;  // body now uses with_metrics_store(...)
    pub fn text(&mut self) -> &mut TextContext;  // updated to construct with the store
}
```

### `TextFlowContext` and `TextContext`

```rust
impl TextFlowContext {
    // Existing
    pub fn new(page_width: f64, page_height: f64, margins: Margins) -> Self;

    // NEW
    pub(crate) fn with_metrics_store(
        page_width: f64,
        page_height: f64,
        margins: Margins,
        store: Option<FontMetricsStore>,
    ) -> Self;
}
```

Identical pattern in `TextContext`.

### `metrics::lookup` (private, replaces `get_font_metrics`)

```rust
fn lookup(font: &Font, store: Option<&FontMetricsStore>) -> Arc<FontMetrics> {
    match font {
        Font::Custom(name) => {
            // 1. Document scope (precedence)
            if let Some(s) = store {
                if let Some(m) = s.get(name) { return m; }
            }
            // 2. Global legacy (hierarchical fallback for deprecated callers)
            if let Some(m) = legacy_global_get(name) { return m; }
            // 3. Default + warn-once (replaces auto-register-on-miss bug)
            warn_unknown_custom_font_once(name);
            default_custom_metrics()  // Arc'd lazy_static, no register
        }
        _ => standard_metrics(font),
    }
}
```

The previous `get_font_metrics` function is removed. Its sole effect-free
behaviour (return metrics for a known font) is preserved by this lookup.
Its previous side effect (auto-register a default into the global on miss)
is removed and replaced by step 3.

### Free measurement functions

```rust
// Existing — back-compat shim, calls measure_text_with(text, font, size, None)
pub fn measure_text(text: &str, font: &Font, font_size: f64) -> f64;

// NEW — scope-aware variant, used internally by TextFlowContext / TextContext / etc.
pub fn measure_text_with(
    text: &str,
    font: &Font,
    font_size: f64,
    store: Option<&FontMetricsStore>,
) -> f64;
```

Same pattern for `measure_char` / `measure_char_with` and
`text::text_block::measure_text_block` /
`text::text_block::measure_text_block_with`.

### Deprecated global API

```rust
#[deprecated(since = "2.8.0", note = "use Document::add_font_from_bytes; the global registry is process-wide and not bounded — see issue #230")]
pub fn register_custom_font_metrics(font_name: String, metrics: FontMetrics);

#[deprecated(since = "2.8.0", note = "use FontMetricsStore::get via a Document — the global registry is process-wide and not bounded — see issue #230")]
pub fn get_custom_font_metrics(font_name: &str) -> Option<FontMetrics>;
```

Bodies remain functional (write to / read from the legacy `CUSTOM_FONT_METRICS`
RwLock). The deprecation is at compile time only.

## Data flow

### Scenario A — canonical (factory)

```
1. let mut doc = Document::new();
2. doc.add_font_from_bytes("MyCJK", bytes)?;
   ├─ self.custom_fonts.add_font("MyCJK", parsed_font)?      [existing]
   └─ self.font_metrics.register("MyCJK", text_metrics)      [NEW]

3. let mut page = doc.new_page_a4();
   └─ Page::a4_with_metrics(self.font_metrics.clone())       [Arc clone]

4. page.set_font(Font::custom("MyCJK"), 12.0);

5. let mut text = page.text_flow();
   └─ TextFlowContext::with_metrics_store(..., Some(store))

6. text.write("高効能");
   └─ measure_text_with(..., Some(&store))
       └─ lookup(font, Some(&store))
           └─ store.get("MyCJK") → Some(Arc<FontMetrics>)    [HIT: Document scope]

7. page.add_text_flow(&text);
8. doc.add_page(page);
   └─ page.font_metrics_store already Some(...) → injection guard skips (preserves existing binding)

9. drop(doc);
   ├─ doc.custom_fonts dropped
   └─ doc.font_metrics dropped → last Arc → memory freed
```

### Scenario B — back-compat (`Page::a4()` + `add_page`)

```
1. let mut doc = Document::new();
2. doc.add_font_from_bytes("MyCJK", bytes)?;
3. let mut page = Page::a4();                                [store = None]
4. page.set_font(Font::custom("MyCJK"), 12.0);
5. let mut text = page.text_flow();                          [None propagates]
6. text.write("高効能");
   └─ lookup(font, None)
       ├─ store_opt = None → skip Document scope
       ├─ legacy_global_get("MyCJK") → None (doc did not write to global)
       └─ warn-once + default                                [⚠ degraded measurement]
7. page.add_text_flow(&text);                                [content stream emitted with default widths]
8. doc.add_page(page);                                       [too late: store injected, content already generated]
```

This flow degrades gracefully (default widths, valid PDF) and emits a single
diagnostic warning per font name that names the migration path. Callers
hitting this path should migrate to `Document::new_page_a4()`.

### Scenario C — hierarchical fallback (legacy global active)

```
1. register_custom_font_metrics("LegacyFont", legacy_metrics);   [⚠ deprecation warning at call site]
2. let mut doc = Document::new();
3. let mut page = doc.new_page_a4();                         [Some(empty_store)]
4. page.set_font(Font::custom("LegacyFont"), 12.0);
5. text.write("Hello");
   └─ lookup(font, Some(&empty_store))
       ├─ empty_store.get("LegacyFont") → None
       ├─ legacy_global_get("LegacyFont") → Some(legacy_metrics)  [HIT global fallback]
       └─ returns legacy_metrics
```

### Scenario D — Document precedence over global

```
1. register_custom_font_metrics("X", metrics_global);
2. let mut doc = Document::new();
3. doc.add_font_from_bytes("X", bytes_doc)?;
4. lookup(Font::custom("X"), Some(&doc.font_metrics))
   ├─ doc.font_metrics.get("X") → Some(metrics_doc)          [HIT: precedence]
   └─ returns metrics_doc                                    [global never consulted]
```

### Scenario E — multi-Document, same name, no leak

```
1. let mut doc_a = Document::new();
2. doc_a.add_font_from_bytes("X", bytes_a)?;
3. let mut doc_b = Document::new();
4. doc_b.add_font_from_bytes("X", bytes_b)?;

5. doc_a.font_metrics["X"] === metrics_a (unaffected by doc_b)
6. doc_b.font_metrics["X"] === metrics_b
7. drop(doc_a) → metrics_a freed, doc_b intact
8. doc_b renders correctly with metrics_b

[pre-fix bug: step 4 overwrote the global entry used by doc_a → resolved by construction]
```

### Scenario F — detached page, font in global only

```
1. register_custom_font_metrics("Z", metrics_z);
2. let page = Page::a4();                                    [store = None]
3. page.text_flow().write_with_font(Font::custom("Z"), ...);
   └─ lookup(font, None)
       ├─ store_opt = None → skip
       ├─ legacy_global_get("Z") → Some(metrics_z)           [HIT global]
       └─ returns metrics_z
```

## Error handling

### `RwLock` poisoning

`FontMetricsStore` uses `Arc<RwLock<HashMap<...>>>`. If a thread panics while
holding the lock, it becomes poisoned. Policy (mirrors the existing
`FontCache` pattern):

| Operation | Lock poisoned → |
|---|---|
| `register(name, metrics)` | `tracing::warn!`, silently no-op |
| `get(name)` | `tracing::warn!`, return `None` |
| `len()` / `is_empty()` | `tracing::warn!`, return 0 / true |

A `None` from `get` falls through to the warn-once + default path; the PDF
is generated with default widths instead of correct ones. This is degradation,
not corruption — a poisoned-lock failure does not produce a malformed PDF.
The alternative (propagating lock poisoning as `Result` through the
measurement signatures) would require a breaking-change cascade through
the entire layout/text/charts/tables surface; rejected.

### Lookup miss

`Font::Custom("typo_in_name")` not present in the Document store, the global,
or anywhere else:

1. `default_custom_metrics()` returns an Arc'd lazy_static fallback.
2. First miss for that name emits
   `tracing::warn!("custom font 'typo_in_name' measured but not registered; widths will use defaults — register via Document::add_font_from_bytes")`.
3. Subsequent misses for the same name are silent (rate-limit via
   `Arc<RwLock<HashSet<String>>>`).
4. **The global registry is never modified** by this path.

This replaces the previous `get_font_metrics:237` behaviour that wrote a
default into the global on every read miss.

### Concurrency within one `Document`

`Document::add_font_from_bytes(&mut self, ...)` takes `&mut self`. The Rust
borrow checker disallows concurrent calls from safe code on the same
`Document` instance. Last-writer-wins on the same name is only possible
through sequential calls, which is the intentional re-registration
semantic.

The original cross-`Document` race (issue #230) is resolved by
construction: each Document owns its store.

### Errors at the `add_font_from_bytes` boundary

`Document::add_font_from_bytes` already returns `Result<()>`. The new code
adds one error path: `font_metrics.register` failing on lock poisoning.
Policy: log a warning, continue — do not escalate to the `Result<()>`.
Reason: failing here after `custom_fonts.add_font` succeeded would leave
`FontCache` populated and `FontMetricsStore` empty, an inconsistent state
to expose via `Result::Err`. Consistent state under degradation: the next
measurement falls back to legacy global / default+warn.

### No new error types

No new variants added to `PdfError`. No public signature gains a `Result`.
The hierarchical lookup is total: every input produces some `Arc<FontMetrics>`
return value (worst case: the default).

## Testing

All tests verify observable output (numerical widths, store contents, log
contents) per the project's no-smoke-tests policy.

### Suite 1 — Bug regressions (3 issue defects)

Lives in `oxidize-pdf-core/tests/font_metrics_per_document_test.rs`. Uses
real TTFs from `tests/fixtures/multilingual/` (DejaVu Sans, Noto Sans CJK).

#### Test 1.1 — `metrics_die_with_document` (memory growth bound)

```text
1. let global_size_before = legacy_global_size();
2. {
       let mut doc = Document::new();
       doc.add_font_from_bytes("Sentinel-A", noto_cjk_bytes.clone())?;
       assert_eq!(doc.font_metrics.len(), 1);
       assert_eq!(legacy_global_size(), global_size_before); // global untouched
   }
3. // doc dropped
4. assert_eq!(legacy_global_size(), global_size_before);     // zero leak
```

Verifies: `add_font_from_bytes` does not grow the global, and the per-Document
store is freed on Document drop (cross-checked via Arc strong count assertions
in a sibling test).

#### Test 1.2 — `multi_document_isolation` (last-writer-wins fix)

```text
1. doc_a registers "X" with DejaVu metrics
2. doc_b registers "X" with Noto CJK metrics
3. let width_a = measure_text_with("A", ..., Some(&doc_a.font_metrics));
4. let width_b = measure_text_with("A", ..., Some(&doc_b.font_metrics));
5. assert!((width_a - expected_dejavu_a).abs() < 0.01);
6. assert!((width_b - expected_noto_a).abs() < 0.01);
7. assert!((width_a - width_b).abs() > 0.5);  // doc_a not contaminated by doc_b
```

Pre-fix: both calls return the last-writer's metrics. Post-fix: each
Document's measurements reflect its own font.

#### Test 1.3 — `cross_document_no_leak_after_drop`

```text
1. {
       let mut doc_a = Document::new();
       doc_a.add_font_from_bytes("Ghost", noto_cjk_bytes)?;
   } // doc_a dropped
2. let doc_b = Document::new();
3. let warns = capture_tracing_warnings();
4. let width = measure_text_with("A", &Font::custom("Ghost"), 12.0, Some(&doc_b.font_metrics));
5. assert width ≈ default_width("A", 12.0); // no ghost metrics
6. assert warns contain "Ghost" + "not registered"
```

### Suite 2 — Hierarchical lookup

Same file. Verifies decision-4A behaviour (Document scope > global > default).

- **2.1** `document_scope_takes_precedence_over_global` — same name in both,
  Document wins.
- **2.2** `legacy_global_visible_when_document_misses` — empty Document store
  falls through to global.
- **2.3** `unknown_font_warns_once_no_register` — 100 calls, exactly one
  warning, neither global nor Document store grows.

### Suite 3 — Threading / API surface

- **3.1** `factory_method_attaches_store` — `doc.new_page_a4()` produces a
  page with `font_metrics_store == Some(...)`.
- **3.2** `add_page_fallback_attaches_store` — `Page::a4()` then `doc.add_page`
  yields a page with the store injected.
- **3.3** `add_page_does_not_overwrite_existing_store` — page already bound
  to doc_a, then `doc_b.add_page(page)` does not reassign. Documents the
  invariant.

### Suite 4 — Concrete content via real PDF render

`oxidize-pdf-core/tests/font_metrics_per_document_render_test.rs`:

- **4.1** `cjk_render_per_document_widths` — full pipeline render with NotoCJK,
  parses the emitted content stream, cross-checks Tj advance widths against
  values computed directly from the TTF cmap. Catches regressions where the
  refactor breaks the lookup → emission chain.
- **4.2** `cjk_render_two_documents_no_cross_contamination` — two docs share
  a font name with different bytes; each rendered PDF carries widths from
  its own font.

### Suite 5 — Deprecation gate (compile-time)

`tests/deprecation_warning_test.rs`:

```rust
#[allow(deprecated)]
fn _verify_deprecated_global_api_still_compiles() {
    let _ = oxidize_pdf::text::metrics::register_custom_font_metrics(
        "X".into(),
        FontMetrics::new(500),
    );
    let _ = oxidize_pdf::text::metrics::get_custom_font_metrics("X");
}
```

If the `#[deprecated]` attribute is removed, the `#[allow(deprecated)]`
becomes `unused_attributes` → warning-as-error → CI fails. Documents the
deprecation contract.

### Suite 6 — Performance regression (criterion bench)

`oxidize-pdf-core/benches/font_metrics_lookup.rs`:

- `lookup_standard_font` (Helvetica) — baseline
- `lookup_custom_font_in_document_store_hit` — most common path
- `lookup_custom_font_global_fallback` — hierarchical fallback
- `lookup_custom_font_unknown_with_warn` — default+warn (rate-limited)

Acceptance threshold: path 1 within ±5% of the existing global-only path
(pre-fix 2.7.x baseline).

## Migration

### CHANGELOG (v2.8.0)

```
### Added
- `FontMetricsStore` in `text::metrics` — per-Document custom font
  metrics store. Cheap-to-clone (Arc-backed), bounded by Document
  lifetime, resolves cross-Document leak and last-writer-wins races on
  the process-wide registry. See issue #230.
- `Document::new_page_a4()` / `new_page_letter()` / `new_page(width, height)`
  — factory methods that produce a `Page` already bound to the
  Document's metrics store. Recommended path for any code using
  custom fonts.
- `measure_text_with(text, &Font, size, Option<&FontMetricsStore>)`,
  `measure_char_with(...)`, `measure_text_block_with(...)` —
  scope-aware variants of the existing measurement helpers.

### Changed
- `Document::add_font_from_bytes` now stores measurement metrics in
  the per-Document `FontMetricsStore` instead of the process-wide
  global registry. Public signature unchanged. Existing callers
  benefit automatically: metrics now die with the Document.
- `Document::add_page(page)` injects the Document's metrics store
  into the page if the page does not already carry one.
- Custom font lookup in measurement helpers no longer auto-registers
  default metrics on read miss. Read paths are now pure reads;
  misses log a single rate-limited warning per name and return
  default widths without persisting anything.

### Deprecated
- `text::metrics::register_custom_font_metrics(name, metrics)` —
  use `Document::add_font_from_bytes`. Function continues to work
  (writes to the legacy global registry) but emits a deprecation
  warning at call sites. Long-running services should migrate to
  the per-Document path.
- `text::metrics::get_custom_font_metrics(name)` — same rationale.

### Fixed
- Resolves issue #230: process-wide `CUSTOM_FONT_METRICS` registry
  leaked metrics across `Document` lifetimes, enabling memory
  growth and cross-document name collisions in long-running
  services.
- Side fix: `text::metrics::get_font_metrics` no longer plants
  default metrics in the global registry from the read path on
  unknown `Font::Custom(name)` lookups.
```

### Migration guide (`docs/migration/v2.8.md`)

Pattern 1 (standard usage): no source change required.

Pattern 2 (recommended for new code): replace `Page::a4()` with
`doc.new_page_a4()` to get the metrics store at page construction.

Pattern 3 (server-side, the case from #230): no source change required.
The existing per-request `Document` flow stops leaking automatically.

Pattern 4 (callers using `register_custom_font_metrics` directly):
deprecation warning fires; migrate to the Document path or
`#[allow(deprecated)]` until v3.0.

### v3.0 plan

- `register_custom_font_metrics` / `get_custom_font_metrics`: removed
  from public API or made `pub(crate)`.
- `CUSTOM_FONT_METRICS` global lazy_static: removed.
- Hierarchical lookup collapses to Document scope only (step 2 of
  `lookup` removed).
- Documented as a separate v3.0 issue at the time #230 is closed.

### Version & release pipeline

- `Cargo.toml` workspace version: `2.8.0`.
- `Cargo.lock` regenerated.
- No feature flag required (changes are 100% backward-compatible).
- `oxidize-pdf-dotnet`: alignment is out of scope for this feature
  branch; tracked separately.
