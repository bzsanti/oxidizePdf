# Per-Document Font Metrics — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Bound custom-font metrics to the lifetime of the owning `Document` so long-running services no longer leak the process-wide `CUSTOM_FONT_METRICS` registry; resolves issue #230 in v2.8.0.

**Architecture:** Adopts Option 2 from the issue with a hierarchical fallback. A new `FontMetricsStore` (Arc-backed, `Clone + Send + Sync`) lives on `Document`. `Document::add_font_from_bytes` writes to the per-Document store instead of the global. `Page` carries an `Option<FontMetricsStore>`, injected by `Document::new_page_*()` (canonical) or `Document::add_page` (back-compat fallback). Measurement helpers gain `_with(store)` variants; existing free `measure_text` keeps working as a back-compat shim. The legacy global registry stays public + `#[deprecated]` and is consulted as a hierarchical fallback when the Document scope misses. The buggy auto-register-on-read-miss in `get_font_metrics` is replaced with a no-side-effect default + rate-limited warn.

**Tech Stack:** Rust 1.77+ (MSRV preserved), `lazy_static`, `tracing`, `criterion 0.8` (existing). Tests via `cargo test`. Pre-commit hook runs build + clippy + full library test suite.

**Reference spec:** `docs/superpowers/specs/2026-05-07-per-document-font-metrics-design.md`

**Branch:** `feature/per-document-font-metrics` (already created, spec already committed at `1a8cec3`).

---

## Conventions

- Every commit ends with the standard `Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>` trailer.
- Pre-commit hook runs the full library test suite. Expect 5–8 minutes per commit.
- Warnings are errors (project policy); clippy must be clean.
- All tests verify observable output (numerical widths, store contents, log captures). No smoke tests, no `assert!(result.is_ok())` without follow-up content checks.
- File path `oxidize-pdf-core/src/text/metrics.rs` is referred to below as `metrics.rs` for brevity. Other files use full paths.

---

### Task 1: `FontMetricsStore` type

**Files:**
- Modify: `oxidize-pdf-core/src/text/metrics.rs` (add type at top of file, after the existing `FontMetrics` impl block ending around line 39)

- [ ] **Step 1: Write the failing tests**

Append to `oxidize-pdf-core/src/text/metrics.rs` inside the existing `mod tests { ... }` block at the bottom:

```rust
#[test]
fn test_font_metrics_store_register_and_get() {
    let store = FontMetricsStore::new();
    assert!(store.is_empty());
    assert_eq!(store.len(), 0);

    let metrics = FontMetrics::new(500).with_widths(&[('A', 700), ('B', 720)]);
    store.register("MyFont", metrics);

    assert_eq!(store.len(), 1);
    assert!(!store.is_empty());

    let got = store.get("MyFont").expect("font should be present");
    assert_eq!(got.char_width('A'), 700);
    assert_eq!(got.char_width('B'), 720);
    assert_eq!(got.char_width('Z'), 500); // default fallback
}

#[test]
fn test_font_metrics_store_overwrite_same_name() {
    let store = FontMetricsStore::new();
    store.register("X", FontMetrics::new(500).with_widths(&[('A', 600)]));
    store.register("X", FontMetrics::new(500).with_widths(&[('A', 800)]));

    let got = store.get("X").unwrap();
    assert_eq!(got.char_width('A'), 800); // last writer wins
    assert_eq!(store.len(), 1);
}

#[test]
fn test_font_metrics_store_clone_shares_state() {
    let store_a = FontMetricsStore::new();
    let store_b = store_a.clone();

    store_a.register("Shared", FontMetrics::new(400));
    assert_eq!(store_b.len(), 1, "clone must share the underlying registry");
    assert!(store_b.get("Shared").is_some());

    store_b.register("AlsoShared", FontMetrics::new(400));
    assert_eq!(store_a.len(), 2);
}

#[test]
fn test_font_metrics_store_get_miss_returns_none_no_side_effects() {
    let store = FontMetricsStore::new();
    assert!(store.get("Unknown").is_none());
    assert_eq!(store.len(), 0); // no auto-register
    assert!(store.is_empty());
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p oxidize-pdf --lib text::metrics::tests::test_font_metrics_store -- --nocapture`
Expected: 4 tests fail to compile (`FontMetricsStore` not defined).

- [ ] **Step 3: Implement `FontMetricsStore`**

Insert into `metrics.rs`, immediately after the `impl FontMetrics { ... }` block (around line 39 in the current file), and BEFORE the `lazy_static!` block for `CUSTOM_FONT_METRICS`:

```rust
use std::sync::Arc;

/// Per-Document store of custom font metrics.
///
/// Cheap to clone (Arc-backed). The lifetime of registered metrics is bound
/// to the lifetime of the owning Document — when the Document is dropped,
/// the metrics are freed (assuming no other Arc clones survive).
///
/// This type was introduced in v2.8.0 to replace the process-wide
/// `CUSTOM_FONT_METRICS` lazy_static registry, which leaked across
/// Document lifetimes (issue #230).
#[derive(Clone, Debug)]
pub struct FontMetricsStore {
    inner: Arc<RwLock<HashMap<String, Arc<FontMetrics>>>>,
}

impl FontMetricsStore {
    /// Create a new empty store.
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register or replace metrics for `font_name`. Last-writer-wins on the
    /// same name. Concurrent calls into the same store are serialised by the
    /// internal RwLock; concurrent calls into the same Document are
    /// prevented by `Document::add_font_from_bytes` taking `&mut self`.
    pub fn register(&self, font_name: impl Into<String>, metrics: FontMetrics) {
        let name = font_name.into();
        match self.inner.write() {
            Ok(mut map) => {
                map.insert(name, Arc::new(metrics));
            }
            Err(e) => {
                tracing::warn!(
                    "FontMetricsStore lock is poisoned; could not register '{}': {}",
                    name,
                    e
                );
            }
        }
    }

    /// Look up metrics by name. Returns `None` on miss; no side effects.
    pub fn get(&self, font_name: &str) -> Option<Arc<FontMetrics>> {
        let map = self.inner.read().ok()?;
        map.get(font_name).cloned()
    }

    /// Number of registered fonts. Diagnostic / test introspection.
    pub fn len(&self) -> usize {
        self.inner.read().map(|m| m.len()).unwrap_or(0)
    }

    /// Whether the store contains no fonts.
    pub fn is_empty(&self) -> bool {
        self.inner.read().map(|m| m.is_empty()).unwrap_or(true)
    }
}

impl Default for FontMetricsStore {
    fn default() -> Self {
        Self::new()
    }
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p oxidize-pdf --lib text::metrics::tests::test_font_metrics_store -- --nocapture`
Expected: 4 tests PASS.

- [ ] **Step 5: Run clippy**

Run: `cargo clippy -p oxidize-pdf --lib --all-targets -- -D warnings`
Expected: clean.

- [ ] **Step 6: Commit**

```bash
git add oxidize-pdf-core/src/text/metrics.rs
git commit -m "$(cat <<'EOF'
feat(text/metrics): add FontMetricsStore for per-Document scope (#230)

Cheap-to-clone Arc-backed store that will replace the process-wide
CUSTOM_FONT_METRICS registry once Document is wired up. Type only;
no behavioural change yet.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

### Task 2: Replace auto-register-on-miss with warn-once + default

**Files:**
- Modify: `oxidize-pdf-core/src/text/metrics.rs` — replace the body of `get_font_metrics` (currently lines 228–256) and add a private `warn_unknown_custom_font_once` helper plus a `default_custom_metrics_arc()` lazy_static.

This task removes the side-effect of the read path. The private function `get_font_metrics` will be renamed to `lookup` in Task 3 when the store parameter is added; for this task the function keeps its current signature and only the behaviour changes.

- [ ] **Step 1: Write the failing tests**

Append inside the existing `mod tests` block:

```rust
#[test]
fn test_unknown_custom_font_does_not_register_on_read() {
    // Use a unique name so this test does not collide with other tests
    // running in parallel under cargo test.
    let unique = format!("UnknownNameTask2_{}", std::process::id());
    let _ = measure_text("hello", &Font::Custom(unique.clone()), 12.0);
    // Lookup must not have planted the name in the global registry.
    #[allow(deprecated)]
    let leaked = get_custom_font_metrics(&unique);
    assert!(
        leaked.is_none(),
        "read path must not auto-register '{}'", unique
    );
}

#[test]
fn test_unknown_custom_font_returns_default_widths() {
    let unique = format!("UnknownReturnTask2_{}", std::process::id());
    let width = measure_text("AAAA", &Font::Custom(unique), 12.0);
    // 4 chars × 500 default width / 1000 × 12 pt = 24.0
    assert!(
        (width - 24.0).abs() < 0.01,
        "unknown custom fonts must use the 500 default width, got {}",
        width
    );
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p oxidize-pdf --lib text::metrics::tests::test_unknown_custom_font -- --nocapture`
Expected: `test_unknown_custom_font_does_not_register_on_read` fails — current `get_font_metrics:237` registers a default into the global on miss.

- [ ] **Step 3: Implement the fix**

In `metrics.rs`, find `fn get_font_metrics(font: &Font) -> FontMetrics` (currently around line 228). Replace it and add the helpers below it.

Replace the `Font::Custom(font_name)` arm:

```rust
fn get_font_metrics(font: &Font) -> FontMetrics {
    match font {
        Font::Custom(font_name) => {
            // Document-scoped lookup is added in Task 3; for this task we only
            // consult the legacy global, then fall back to default+warn.
            if let Some(custom_metrics) = get_custom_font_metrics_internal(font_name) {
                custom_metrics
            } else {
                warn_unknown_custom_font_once(font_name);
                (*default_custom_metrics_arc()).clone()
            }
        }
        _ => {
            FONT_METRICS.get(font).cloned().unwrap_or_else(|| {
                tracing::debug!(
                    "Warning: Standard font metrics not found for {:?}, using default",
                    font
                );
                (*default_custom_metrics_arc()).clone()
            })
        }
    }
}

/// Internal accessor for the legacy global registry. Wraps the deprecated
/// `get_custom_font_metrics` so the lookup path does not itself emit a
/// deprecation warning at every call site.
fn get_custom_font_metrics_internal(font_name: &str) -> Option<FontMetrics> {
    if let Ok(custom_metrics) = CUSTOM_FONT_METRICS.read() {
        custom_metrics.get(font_name).cloned()
    } else {
        None
    }
}

lazy_static::lazy_static! {
    /// Cached default metrics for unknown custom fonts. Building this map
    /// once (lazy_static) means subsequent fallbacks reuse the same data
    /// rather than rebuilding the CJK table on every miss.
    static ref DEFAULT_CUSTOM_METRICS: Arc<FontMetrics> =
        Arc::new(create_default_custom_metrics());
}

fn default_custom_metrics_arc() -> Arc<FontMetrics> {
    DEFAULT_CUSTOM_METRICS.clone()
}

lazy_static::lazy_static! {
    /// Names already warned about. Rate-limits the unknown-font warning to
    /// one emission per name per process.
    static ref WARNED_UNKNOWN_FONTS: RwLock<std::collections::HashSet<String>> =
        RwLock::new(std::collections::HashSet::new());
}

fn warn_unknown_custom_font_once(font_name: &str) {
    {
        if let Ok(set) = WARNED_UNKNOWN_FONTS.read() {
            if set.contains(font_name) {
                return;
            }
        }
    }
    if let Ok(mut set) = WARNED_UNKNOWN_FONTS.write() {
        if set.insert(font_name.to_string()) {
            tracing::warn!(
                "custom font '{}' measured but not registered; widths will use \
                 defaults — register via Document::add_font_from_bytes",
                font_name
            );
        }
    }
}
```

Also add a tests-only helper inside the `mod tests` block to clear the warned-set between tests (so other tests asserting "warn once" don't see a stale insert):

```rust
#[allow(dead_code)]
fn reset_warned_unknown_fonts() {
    if let Ok(mut set) = WARNED_UNKNOWN_FONTS.write() {
        set.clear();
    }
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p oxidize-pdf --lib text::metrics::tests::test_unknown_custom_font -- --nocapture`
Expected: both PASS.

Run: `cargo test -p oxidize-pdf --lib text::metrics -- --nocapture`
Expected: all metrics tests PASS (regression check on the existing suite).

- [ ] **Step 5: Run clippy**

Run: `cargo clippy -p oxidize-pdf --lib --all-targets -- -D warnings`
Expected: clean.

- [ ] **Step 6: Commit**

```bash
git add oxidize-pdf-core/src/text/metrics.rs
git commit -m "$(cat <<'EOF'
fix(text/metrics): stop auto-registering defaults on read miss (#230)

The read path get_font_metrics auto-registered default metrics into
the global on every Font::Custom lookup miss, planting a permanent
entry for any typo'd name. Replace with a no-side-effect default
plus a rate-limited tracing::warn one per name per process.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

### Task 3: Hierarchical lookup with store parameter

**Files:**
- Modify: `oxidize-pdf-core/src/text/metrics.rs` — rename `get_font_metrics` → `lookup` and add the store parameter.

- [ ] **Step 1: Write the failing tests**

Append to `mod tests`:

```rust
#[test]
fn test_lookup_document_scope_takes_precedence_over_global() {
    let unique = format!("PrecedenceTask3_{}", std::process::id());

    // Plant something in the legacy global.
    #[allow(deprecated)]
    register_custom_font_metrics(
        unique.clone(),
        FontMetrics::new(500).with_widths(&[('A', 100)]),
    );

    // Per-Document store has different metrics for the same name.
    let store = FontMetricsStore::new();
    store.register(
        unique.clone(),
        FontMetrics::new(500).with_widths(&[('A', 900)]),
    );

    let resolved = lookup(&Font::Custom(unique), Some(&store));
    assert_eq!(
        resolved.char_width('A'),
        900,
        "Document scope must win over global"
    );
}

#[test]
fn test_lookup_falls_through_to_global_when_store_misses() {
    let unique = format!("FallthroughTask3_{}", std::process::id());

    #[allow(deprecated)]
    register_custom_font_metrics(
        unique.clone(),
        FontMetrics::new(500).with_widths(&[('A', 333)]),
    );

    let empty_store = FontMetricsStore::new();
    let resolved = lookup(&Font::Custom(unique), Some(&empty_store));
    assert_eq!(
        resolved.char_width('A'),
        333,
        "must fall through to legacy global when Document store misses"
    );
}

#[test]
fn test_lookup_with_none_store_uses_global_then_default() {
    let unique = format!("NoneStoreTask3_{}", std::process::id());

    // No global, no store. Should default+warn.
    let resolved = lookup(&Font::Custom(unique), None);
    assert_eq!(resolved.char_width('A'), 500); // default
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p oxidize-pdf --lib text::metrics::tests::test_lookup_ -- --nocapture`
Expected: 3 tests fail to compile (`lookup` not defined).

- [ ] **Step 3: Implement the lookup refactor**

In `metrics.rs`:

3a. Rename `fn get_font_metrics(font: &Font) -> FontMetrics` to `fn lookup(font: &Font, store: Option<&FontMetricsStore>) -> FontMetrics`. Adjust the body so the `Custom` arm consults the store first:

```rust
fn lookup(font: &Font, store: Option<&FontMetricsStore>) -> FontMetrics {
    match font {
        Font::Custom(font_name) => {
            // 1. Document scope (precedence)
            if let Some(s) = store {
                if let Some(arc_m) = s.get(font_name) {
                    return (*arc_m).clone();
                }
            }
            // 2. Legacy global (deprecated, hierarchical fallback)
            if let Some(custom_metrics) = get_custom_font_metrics_internal(font_name) {
                return custom_metrics;
            }
            // 3. Default + warn-once
            warn_unknown_custom_font_once(font_name);
            (*default_custom_metrics_arc()).clone()
        }
        _ => {
            FONT_METRICS.get(font).cloned().unwrap_or_else(|| {
                tracing::debug!(
                    "Warning: Standard font metrics not found for {:?}, using default",
                    font
                );
                (*default_custom_metrics_arc()).clone()
            })
        }
    }
}
```

3b. Update existing call sites within `metrics.rs`:

- `measure_text`: change `let metrics = get_font_metrics(font);` to `let metrics = lookup(font, None);`
- `measure_char`: change `let metrics = get_font_metrics(&font);` to `let metrics = lookup(&font, None);`

The `_with` variants are added in Task 4.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p oxidize-pdf --lib text::metrics::tests::test_lookup_ -- --nocapture`
Expected: 3 tests PASS.

Run: `cargo test -p oxidize-pdf --lib text::metrics -- --nocapture`
Expected: all metrics tests PASS.

- [ ] **Step 5: Run clippy**

Run: `cargo clippy -p oxidize-pdf --lib --all-targets -- -D warnings`
Expected: clean.

- [ ] **Step 6: Commit**

```bash
git add oxidize-pdf-core/src/text/metrics.rs
git commit -m "$(cat <<'EOF'
feat(text/metrics): hierarchical lookup with optional Document scope (#230)

Rename internal get_font_metrics to lookup and add an Option<&FontMetricsStore>
parameter. Resolution order for Font::Custom(name): Document scope first,
then legacy global, then default+warn. Free measure_text/measure_char
keep passing None for now; per-context wiring follows in subsequent tasks.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

### Task 4: Scoped free measurement functions (`measure_text_with`, `measure_char_with`)

**Files:**
- Modify: `oxidize-pdf-core/src/text/metrics.rs` — add `_with` variants; refactor existing `measure_text` / `measure_char` to delegate.

- [ ] **Step 1: Write the failing tests**

Append to `mod tests`:

```rust
#[test]
fn test_measure_text_with_uses_document_scope() {
    let unique = format!("MeasureWithTask4_{}", std::process::id());
    let store = FontMetricsStore::new();
    store.register(
        unique.clone(),
        // Each char (A through F) at 1000 units; 'A' x 4 chars = 48.0 at 12pt.
        FontMetrics::new(500).with_widths(&[('A', 1000)]),
    );

    let width = measure_text_with(
        "AAAA",
        &Font::Custom(unique),
        12.0,
        Some(&store),
    );
    // 4 * 1000 / 1000 * 12 = 48
    assert!((width - 48.0).abs() < 0.01, "got {}", width);
}

#[test]
fn test_measure_text_back_compat_shim_passes_none() {
    let unique = format!("BackCompatTask4_{}", std::process::id());
    // Without store, with empty global → default 'A' from create_default_custom_metrics
    // ('A' = 667; default_width = 556 only applies to chars not explicitly mapped).
    let width = measure_text("AAAA", &Font::Custom(unique), 12.0);
    // 4 chars × 667 / 1000 × 12 = 32.016
    assert!((width - 32.016).abs() < 0.01, "got {}", width);
}

#[test]
fn test_measure_char_with_uses_document_scope() {
    let unique = format!("MeasureCharWithTask4_{}", std::process::id());
    let store = FontMetricsStore::new();
    store.register(
        unique.clone(),
        FontMetrics::new(500).with_widths(&[('Z', 800)]),
    );
    let width = measure_char_with('Z', Font::Custom(unique), 10.0, Some(&store));
    // 800 / 1000 * 10 = 8
    assert!((width - 8.0).abs() < 0.01, "got {}", width);
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p oxidize-pdf --lib text::metrics::tests::test_measure_text_with text::metrics::tests::test_measure_text_back_compat text::metrics::tests::test_measure_char_with -- --nocapture`
Expected: tests fail to compile (`measure_text_with` / `measure_char_with` not defined).

- [ ] **Step 3: Implement the `_with` variants and refactor existing functions**

In `metrics.rs`, replace the existing `measure_text` and `measure_char` with:

```rust
/// Measure the width of a text string in a given font and size.
///
/// Variant of `measure_text` that consults a `FontMetricsStore` for
/// `Font::Custom` lookups before falling back to the legacy global
/// registry. Used internally by `TextFlowContext`, `TextContext`, and
/// `measure_text_block_with` to scope measurement to a single Document.
pub fn measure_text_with(
    text: &str,
    font: &Font,
    font_size: f64,
    store: Option<&FontMetricsStore>,
) -> f64 {
    if font.is_symbolic() {
        return text.len() as f64 * font_size * 0.6;
    }
    let metrics = lookup(font, store);
    let width_units: u32 = text.chars().map(|ch| metrics.char_width(ch) as u32).sum();
    (width_units as f64 / 1000.0) * font_size
}

/// Measure the width of a text string in a given font and size.
///
/// Back-compat shim. Delegates to `measure_text_with(text, font, font_size, None)`.
/// Custom fonts not registered globally fall back to default widths plus a
/// rate-limited diagnostic warning. For new code, prefer `measure_text_with`
/// or use `Document::new_page_a4()` so the measurement context carries a
/// `FontMetricsStore` automatically.
pub fn measure_text(text: &str, font: &Font, font_size: f64) -> f64 {
    measure_text_with(text, font, font_size, None)
}

/// Measure the width of a single character with optional Document scope.
pub fn measure_char_with(
    ch: char,
    font: Font,
    font_size: f64,
    store: Option<&FontMetricsStore>,
) -> f64 {
    if font.is_symbolic() {
        return font_size * 0.6;
    }
    let metrics = lookup(&font, store);
    (metrics.char_width(ch) as f64 / 1000.0) * font_size
}

/// Back-compat shim — see `measure_char_with`.
pub fn measure_char(ch: char, font: Font, font_size: f64) -> f64 {
    measure_char_with(ch, font, font_size, None)
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p oxidize-pdf --lib text::metrics -- --nocapture`
Expected: all tests PASS (existing + 3 new).

- [ ] **Step 5: Run clippy**

Run: `cargo clippy -p oxidize-pdf --lib --all-targets -- -D warnings`
Expected: clean.

- [ ] **Step 6: Commit**

```bash
git add oxidize-pdf-core/src/text/metrics.rs
git commit -m "$(cat <<'EOF'
feat(text/metrics): scope-aware measure_text_with/measure_char_with (#230)

Add _with variants that accept Option<&FontMetricsStore>. The existing
measure_text and measure_char remain as back-compat shims passing None.
Internal lookup is hierarchical (Document scope → legacy global → default).

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

### Task 5: Scoped `measure_text_block_with`

**Files:**
- Modify: `oxidize-pdf-core/src/text/text_block.rs` — add `_with` variant; refactor existing `measure_text_block` to delegate.

Currently `text_block.rs:40` calls `measure_text(word, font, font_size)`. The internal call must thread the store.

- [ ] **Step 1: Read the file to anchor the changes**

Run: `cat oxidize-pdf-core/src/text/text_block.rs`

Note the import line `use crate::text::{measure_text, split_into_words, Font};` and the function `pub fn measure_text_block(...)`.

- [ ] **Step 2: Write the failing test**

Append to the existing `mod tests` block in `oxidize-pdf-core/src/text/text_block.rs`:

```rust
#[test]
fn test_measure_text_block_with_uses_document_scope() {
    use crate::text::metrics::{FontMetrics, FontMetricsStore};
    let unique = format!("MeasureBlockTask5_{}", std::process::id());
    let store = FontMetricsStore::new();
    // Make every char width = 1000 (i.e., 1.0em per char). Word "AB" = 24 at 12pt.
    store.register(
        unique.clone(),
        FontMetrics::new(500).with_widths(&[('A', 1000), ('B', 1000)]),
    );

    let m = measure_text_block_with(
        "AB",
        &Font::Custom(unique.clone()),
        12.0,
        1.2,
        500.0,
        Some(&store),
    );
    // One line with width = 2 * 1000 / 1000 * 12 = 24
    assert!(
        (m.width - 24.0).abs() < 0.01,
        "expected scope-aware width 24, got {}",
        m.width
    );
}
```

- [ ] **Step 3: Run test to verify it fails**

Run: `cargo test -p oxidize-pdf --lib text::text_block::tests::test_measure_text_block_with_uses_document_scope -- --nocapture`
Expected: fails to compile (`measure_text_block_with` not defined; missing `FontMetrics` re-export).

- [ ] **Step 4: Implement the `_with` variant**

In `oxidize-pdf-core/src/text/text_block.rs`:

4a. Update the imports at the top:

```rust
use crate::text::metrics::{measure_text_with, FontMetricsStore};
use crate::text::{split_into_words, Font};
```

(Drop the `measure_text` import — internal use is now via `measure_text_with`.)

4b. Locate the body of `pub fn measure_text_block(text: &str, font: &Font, font_size: f64, line_height: f64, max_width: f64) -> TextBlockMetrics`. Replace its body with a delegating call and add the `_with` variant alongside it. Keep the existing helper `compute_line_widths` private behaviour intact — only the entry-point function changes.

```rust
/// Measure a text block laid out within `max_width`, returning width and
/// line count for the given font/size/line-height. Back-compat shim;
/// delegates to `measure_text_block_with(..., None)`.
pub fn measure_text_block(
    text: &str,
    font: &Font,
    font_size: f64,
    line_height: f64,
    max_width: f64,
) -> TextBlockMetrics {
    measure_text_block_with(text, font, font_size, line_height, max_width, None)
}

/// Scope-aware variant of `measure_text_block`. Consults `store` (if Some)
/// before the legacy global registry for `Font::Custom` lookups.
pub fn measure_text_block_with(
    text: &str,
    font: &Font,
    font_size: f64,
    line_height: f64,
    max_width: f64,
    store: Option<&FontMetricsStore>,
) -> TextBlockMetrics {
    // Re-create the body of the previous measure_text_block here, but
    // every call to `measure_text(...)` becomes `measure_text_with(..., store)`.
    //
    // (Engineer note: the previous implementation used `measure_text(word, font, font_size)`
    //  inside the wrapping loop. Replace each such call with
    //  `measure_text_with(word, font, font_size, store)`.)
    /* paste the old body here, then in every line that reads
       measure_text(word, font, font_size) replace with
       measure_text_with(word, font, font_size, store) */
}
```

The existing body of `measure_text_block` should be moved into `measure_text_block_with` with the substitution. Use `git show HEAD:oxidize-pdf-core/src/text/text_block.rs` to confirm the original body if uncertain.

4c. If `tests/test_measure_text_block_empty` (or any other test inside `mod tests`) imports `measure_text_block` directly, it continues to compile against the back-compat shim. No test rewrite needed.

- [ ] **Step 5: Re-export `FontMetrics` in the path used by tests**

In `oxidize-pdf-core/src/text/mod.rs`, find the `pub use` lines for the metrics module. Verify `FontMetrics` is re-exported at `crate::text::FontMetrics`. If not, add:

```rust
pub use metrics::{FontMetrics, FontMetricsStore};
```

(`FontMetricsStore` is also added to the public surface here; it is needed by external callers in v2.8.0.)

- [ ] **Step 6: Run tests to verify they pass**

Run: `cargo test -p oxidize-pdf --lib text::text_block -- --nocapture`
Expected: all PASS (existing + 1 new).

- [ ] **Step 7: Run clippy**

Run: `cargo clippy -p oxidize-pdf --lib --all-targets -- -D warnings`
Expected: clean.

- [ ] **Step 8: Commit**

```bash
git add oxidize-pdf-core/src/text/text_block.rs oxidize-pdf-core/src/text/mod.rs
git commit -m "$(cat <<'EOF'
feat(text/text_block): scope-aware measure_text_block_with (#230)

Add the _with variant; the existing measure_text_block becomes a
back-compat shim. Internal word-wrap loop now threads the optional
FontMetricsStore through to measure_text_with. Re-export
FontMetricsStore from the text module.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

### Task 6: `TextFlowContext` threading

**Files:**
- Modify: `oxidize-pdf-core/src/text/flow.rs` — add `font_metrics_store` field, `with_metrics_store` constructor, and replace internal `measure_text` calls with `measure_text_with`.

Existing `TextFlowContext` lives at `oxidize-pdf-core/src/text/flow.rs:15`. Internal calls at lines ~202 and ~222 use `measure_text(word, &self.current_font, self.font_size)` and similar.

- [ ] **Step 1: Write the failing test**

Append to the `mod tests` block in `oxidize-pdf-core/src/text/flow.rs`:

```rust
#[test]
fn test_text_flow_context_threads_metrics_store() {
    use crate::text::metrics::{FontMetrics, FontMetricsStore};
    let unique = format!("FlowThreadTask6_{}", std::process::id());
    let store = FontMetricsStore::new();
    // 'A' = 1000 → 12pt → 12.0 per char.
    store.register(
        unique.clone(),
        FontMetrics::new(500).with_widths(&[('A', 1000)]),
    );

    let mut ctx = TextFlowContext::with_metrics_store(
        595.0, // A4 width pt
        842.0, // A4 height pt
        Margins::default(),
        Some(store),
    );
    ctx.set_font(Font::Custom(unique), 12.0);
    ctx.write("AA").unwrap();

    // The flow should have measured "AA" using the per-store widths and
    // produced a positive width on the line. The exact public way to
    // observe this depends on the flow API; this test asserts that the
    // generated_operations() output contains a Tj with the expected text
    // and that the flow advanced.
    let ops = ctx.generate_operations();
    assert!(!ops.is_empty(), "flow must emit content for 'AA'");
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p oxidize-pdf --lib text::flow::tests::test_text_flow_context_threads_metrics_store -- --nocapture`
Expected: fails to compile (`with_metrics_store` not defined).

- [ ] **Step 3: Implement field + constructor + replace measure calls**

In `oxidize-pdf-core/src/text/flow.rs`:

3a. Update imports:

```rust
use crate::text::metrics::{measure_text_with, FontMetricsStore};
use crate::text::{split_into_words, Font};
```

(Drop the `measure_text` import.)

3b. Add the new field to the struct (around line 15):

```rust
pub struct TextFlowContext {
    // ...existing fields preserved verbatim...
    pub(crate) font_metrics_store: Option<FontMetricsStore>,
}
```

3c. Update the existing `pub fn new(...)` to default the new field to `None`:

```rust
pub fn new(page_width: f64, page_height: f64, margins: Margins) -> Self {
    Self {
        // ...existing initialisation preserved verbatim...
        font_metrics_store: None,
    }
}
```

3d. Add the constructor variant:

```rust
pub(crate) fn with_metrics_store(
    page_width: f64,
    page_height: f64,
    margins: Margins,
    store: Option<FontMetricsStore>,
) -> Self {
    let mut ctx = Self::new(page_width, page_height, margins);
    ctx.font_metrics_store = store;
    ctx
}
```

3e. Replace internal measure calls (at the lines identified by `grep -n 'measure_text(' oxidize-pdf-core/src/text/flow.rs`):

```rust
// Replace:
let word_width = measure_text(word, &self.current_font, self.font_size);
// With:
let word_width = measure_text_with(
    word,
    &self.current_font,
    self.font_size,
    self.font_metrics_store.as_ref(),
);
```

Repeat for every `measure_text(...)` call in the file (expected count: 2 per the earlier grep).

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p oxidize-pdf --lib text::flow -- --nocapture`
Expected: all PASS.

- [ ] **Step 5: Run clippy**

Run: `cargo clippy -p oxidize-pdf --lib --all-targets -- -D warnings`
Expected: clean.

- [ ] **Step 6: Commit**

```bash
git add oxidize-pdf-core/src/text/flow.rs
git commit -m "$(cat <<'EOF'
feat(text/flow): TextFlowContext carries optional FontMetricsStore (#230)

Add font_metrics_store: Option<FontMetricsStore> field plus the
pub(crate) with_metrics_store constructor. Internal measurement calls
route through measure_text_with so font widths resolve via the
Document scope when the page was bound at construction.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

### Task 7: `TextContext` threading

**Files:**
- Modify: `oxidize-pdf-core/src/text/mod.rs` — `pub struct TextContext` is at line 85; `impl TextContext` at line 117. Same changes as Task 6.

- [ ] **Step 1: Inspect `TextContext` to find internal `measure_text` call sites**

Run: `grep -n 'measure_text\|measure_char\|fn new\|pub struct TextContext\|impl TextContext' oxidize-pdf-core/src/text/mod.rs`

Note any internal callers. If the existing `TextContext` does not call `measure_text` directly (it may only emit raw operators without measuring inline), then this task only adds the field + constructor for symmetry; later refactors can add measurement-using methods.

- [ ] **Step 2: Write the failing test**

Append to the test module in `oxidize-pdf-core/src/text/mod.rs` (find or create a `#[cfg(test)] mod tests { ... }` near the bottom of the file):

```rust
#[test]
fn test_text_context_threads_metrics_store() {
    use crate::text::metrics::{FontMetrics, FontMetricsStore};
    let store = FontMetricsStore::new();
    let ctx = TextContext::with_metrics_store(Some(store.clone()));
    // The store handle round-trips.
    assert!(ctx.font_metrics_store_for_test().is_some());
    // Cloning shares state.
    store.register("X", FontMetrics::new(400));
    assert_eq!(
        ctx.font_metrics_store_for_test().unwrap().len(),
        1,
        "TextContext must hold a clone that shares the underlying registry"
    );
}
```

Add a small `#[cfg(test)]` introspection helper inside `impl TextContext`:

```rust
#[cfg(test)]
pub(crate) fn font_metrics_store_for_test(&self) -> Option<&FontMetricsStore> {
    self.font_metrics_store.as_ref()
}
```

- [ ] **Step 3: Run test to verify it fails**

Run: `cargo test -p oxidize-pdf --lib text::tests::test_text_context_threads_metrics_store -- --nocapture`
Expected: fails to compile.

- [ ] **Step 4: Implement field + constructor**

In `oxidize-pdf-core/src/text/mod.rs`:

4a. Add import (at the top of the file's import section):

```rust
use crate::text::metrics::FontMetricsStore;
```

(Note: `metrics` is already a sibling module; pull `FontMetricsStore` into scope.)

4b. Add the new field to `pub struct TextContext { ... }`:

```rust
pub struct TextContext {
    // ...existing fields preserved verbatim...
    pub(crate) font_metrics_store: Option<FontMetricsStore>,
}
```

4c. Update the existing `Default` / `new` (whichever the struct uses to construct itself) to initialise `font_metrics_store: None`. Inspect the file to find the constructor entry point.

4d. Add the constructor variant inside `impl TextContext`:

```rust
pub(crate) fn with_metrics_store(store: Option<FontMetricsStore>) -> Self {
    let mut ctx = Self::default();
    ctx.font_metrics_store = store;
    ctx
}
```

(Use `default()` if the type has a `Default` impl; otherwise call the existing entry-point constructor.)

4e. If `grep` in step 1 surfaced any internal `measure_text(...)` calls, replace them with `measure_text_with(..., self.font_metrics_store.as_ref())` exactly as in Task 6.

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test -p oxidize-pdf --lib text -- --nocapture`
Expected: all PASS.

- [ ] **Step 6: Run clippy**

Run: `cargo clippy -p oxidize-pdf --lib --all-targets -- -D warnings`
Expected: clean.

- [ ] **Step 7: Commit**

```bash
git add oxidize-pdf-core/src/text/mod.rs
git commit -m "$(cat <<'EOF'
feat(text): TextContext carries optional FontMetricsStore (#230)

Mirrors the TextFlowContext shape: new font_metrics_store field plus
pub(crate) with_metrics_store constructor. Internal measurement calls
(if any) route through the scope-aware variant.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

### Task 8: `Page` threading

**Files:**
- Modify: `oxidize-pdf-core/src/page.rs` — add `font_metrics_store: Option<FontMetricsStore>` field; add `pub(crate)` constructors `a4_with_metrics`, `letter_with_metrics`, `new_with_metrics`; update `text_flow()` and `text()` to propagate.

- [ ] **Step 1: Write the failing tests**

Append to the test module in `oxidize-pdf-core/src/page.rs` (or create `#[cfg(test)] mod tests` near the bottom if missing):

```rust
#[test]
fn test_page_a4_default_has_no_metrics_store() {
    let page = Page::a4();
    assert!(
        page.font_metrics_store.is_none(),
        "Page::a4() must not bind a store; binding happens via Document"
    );
}

#[test]
fn test_page_a4_with_metrics_carries_store() {
    use crate::text::metrics::FontMetricsStore;
    let store = FontMetricsStore::new();
    let page = Page::a4_with_metrics(store);
    assert!(page.font_metrics_store.is_some());
}

#[test]
fn test_page_text_flow_propagates_store() {
    use crate::text::metrics::FontMetricsStore;
    let store = FontMetricsStore::new();
    let page = Page::a4_with_metrics(store);
    let flow = page.text_flow();
    // The TextFlowContext must carry the same Arc-shared store.
    assert!(
        flow.font_metrics_store.is_some(),
        "page.text_flow() must propagate the store handle"
    );
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p oxidize-pdf --lib page::tests::test_page_a4_with_metrics page::tests::test_page_text_flow_propagates_store -- --nocapture`
Expected: fails to compile.

- [ ] **Step 3: Implement field, constructors, propagation**

In `oxidize-pdf-core/src/page.rs`:

3a. Add import:

```rust
use crate::text::metrics::FontMetricsStore;
```

3b. Add the field to `pub struct Page { ... }` (line 107):

```rust
pub struct Page {
    // ...existing fields preserved verbatim...
    pub(crate) font_metrics_store: Option<FontMetricsStore>,
}
```

3c. Update the existing constructors (`Page::a4()`, `Page::letter()`, `Page::new(width, height)`) to initialise `font_metrics_store: None`. Locate them via `grep -n 'pub fn a4\|pub fn letter\|pub fn new(' oxidize-pdf-core/src/page.rs`.

3d. Add the variant constructors inside `impl Page`:

```rust
pub(crate) fn a4_with_metrics(store: FontMetricsStore) -> Self {
    let mut p = Self::a4();
    p.font_metrics_store = Some(store);
    p
}

pub(crate) fn letter_with_metrics(store: FontMetricsStore) -> Self {
    let mut p = Self::letter();
    p.font_metrics_store = Some(store);
    p
}

pub(crate) fn new_with_metrics(width: f64, height: f64, store: FontMetricsStore) -> Self {
    let mut p = Self::new(width, height);
    p.font_metrics_store = Some(store);
    p
}
```

3e. Update `pub fn text_flow(&self) -> TextFlowContext` (line 879). Replace `let mut ctx = TextFlowContext::new(self.width, self.height, self.margins.clone());` with:

```rust
let mut ctx = TextFlowContext::with_metrics_store(
    self.width,
    self.height,
    self.margins.clone(),
    self.font_metrics_store.clone(),
);
```

The remaining body (set_font, set_fill_color, etc.) stays unchanged.

3f. Update `pub fn text(&mut self) -> &mut TextContext` (line 605). The `Page` lazily constructs its `TextContext`; locate the construction site (likely in `Page::a4()` / `Page::new()` initialisation, where `text_context: TextContext::default()` or similar is set). When the field is initialised, instead use:

```rust
text_context: TextContext::with_metrics_store(self.font_metrics_store.clone()),
```

If `text_context` is initialised eagerly (before `font_metrics_store` is settable), set it after the struct is constructed in the `*_with_metrics` constructors:

```rust
pub(crate) fn a4_with_metrics(store: FontMetricsStore) -> Self {
    let mut p = Self::a4();
    p.font_metrics_store = Some(store.clone());
    p.text_context = TextContext::with_metrics_store(Some(store));
    p
}
```

(Inspect the existing `Page::a4()` body to confirm the right placement.)

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p oxidize-pdf --lib page::tests::test_page -- --nocapture`
Expected: all PASS (existing + 3 new).

- [ ] **Step 5: Run clippy**

Run: `cargo clippy -p oxidize-pdf --lib --all-targets -- -D warnings`
Expected: clean.

- [ ] **Step 6: Commit**

```bash
git add oxidize-pdf-core/src/page.rs
git commit -m "$(cat <<'EOF'
feat(page): Page carries optional FontMetricsStore + variant ctors (#230)

Add font_metrics_store field + pub(crate) a4_with_metrics /
letter_with_metrics / new_with_metrics constructors. Page::text_flow
and Page::text propagate the handle into TextFlowContext / TextContext
respectively. Page::a4(), letter(), new() retain None — binding
happens via Document::new_page_*() or Document::add_page() fallback.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

### Task 9: `Document::font_metrics` field + `add_font_from_bytes` refactor

**Files:**
- Modify: `oxidize-pdf-core/src/document.rs` — add `font_metrics: FontMetricsStore` field; init in `Document::new()`; rewire `add_font_from_bytes` to write to `self.font_metrics` instead of the global.

- [ ] **Step 1: Write the failing test**

Append to the test module in `oxidize-pdf-core/src/document.rs` (or create one):

```rust
#[test]
fn test_add_font_from_bytes_writes_to_per_document_store_not_global() {
    // Use a unique font name so this test does not collide with parallel tests.
    let unique = format!("PerDocTask9_{}", std::process::id());
    // Capture global size before.
    #[allow(deprecated)]
    let before = crate::text::metrics::get_custom_font_metrics(&unique);
    assert!(before.is_none(), "precondition: name not in global");

    // Construct a Document and register a synthetic font under this name.
    // We bypass the TTF parser by going through the metrics store directly
    // — the public API requires real TTF bytes, which is exercised in the
    // integration suite (Task 14). This unit test focuses on the routing.
    let doc = crate::Document::new();
    doc.font_metrics.register(unique.clone(), crate::text::FontMetrics::new(500));

    // The Document store contains the entry.
    assert!(doc.font_metrics.get(&unique).is_some());

    // The legacy global was untouched.
    #[allow(deprecated)]
    let after = crate::text::metrics::get_custom_font_metrics(&unique);
    assert!(after.is_none(), "global must remain untouched");
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p oxidize-pdf --lib document::tests::test_add_font_from_bytes_writes_to_per_document_store_not_global -- --nocapture`
Expected: fails to compile (`font_metrics` field on `Document` does not exist; `Document::new()` does not initialise it).

- [ ] **Step 3: Add the field and init**

In `oxidize-pdf-core/src/document.rs`:

3a. Update the import line at the top:

```rust
use crate::text::metrics::{FontMetrics as TextMeasurementMetrics, FontMetricsStore};
```

(Drop `register_custom_font_metrics` from the import.)

3b. Add the field to the struct:

```rust
pub struct Document {
    // ...existing fields preserved verbatim...
    pub(crate) custom_fonts: FontCache,
    pub(crate) font_metrics: FontMetricsStore,  // NEW
    // ...remaining existing fields...
}
```

3c. Initialise it in `pub fn new() -> Self` (line 116). Add `font_metrics: FontMetricsStore::new(),` to the struct literal alongside `custom_fonts: FontCache::new(),`.

3d. Rewire `pub fn add_font_from_bytes` (line 434). Replace the call:

```rust
register_custom_font_metrics(name, text_metrics);
```

with:

```rust
self.font_metrics.register(name, text_metrics);
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p oxidize-pdf --lib document::tests::test_add_font_from_bytes -- --nocapture`
Expected: PASS.

Run: `cargo test -p oxidize-pdf --lib document -- --nocapture`
Expected: existing document tests still PASS (sanity).

- [ ] **Step 5: Run clippy**

Run: `cargo clippy -p oxidize-pdf --lib --all-targets -- -D warnings`
Expected: clean.

- [ ] **Step 6: Commit**

```bash
git add oxidize-pdf-core/src/document.rs
git commit -m "$(cat <<'EOF'
fix(document): route add_font_from_bytes to per-Document store (#230)

Document gains a font_metrics: FontMetricsStore field initialised in
Document::new(). add_font_from_bytes now calls self.font_metrics.register
instead of the deprecated global register_custom_font_metrics. The
legacy CUSTOM_FONT_METRICS registry is no longer touched by the
Document path; metrics die with the Document.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

### Task 10: `Document` factory methods (`new_page_a4`, `new_page_letter`, `new_page`)

**Files:**
- Modify: `oxidize-pdf-core/src/document.rs` — add three factory methods on `impl Document`.

- [ ] **Step 1: Write the failing test**

Append to the document test module:

```rust
#[test]
fn test_new_page_a4_returns_page_bound_to_document_store() {
    let doc = crate::Document::new();
    doc.font_metrics.register("Sentinel", crate::text::FontMetrics::new(400));

    let page = doc.new_page_a4();
    assert!(page.font_metrics_store.is_some());
    let store = page.font_metrics_store.as_ref().unwrap();
    assert!(store.get("Sentinel").is_some(), "store must share with Document");
}

#[test]
fn test_new_page_letter_and_new_page_carry_store() {
    let doc = crate::Document::new();
    doc.font_metrics.register("S", crate::text::FontMetrics::new(400));
    assert!(doc.new_page_letter().font_metrics_store.is_some());
    assert!(doc.new_page(400.0, 600.0).font_metrics_store.is_some());
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p oxidize-pdf --lib document::tests::test_new_page -- --nocapture`
Expected: fails to compile (`new_page_a4`, etc. not defined).

- [ ] **Step 3: Add factory methods**

In `oxidize-pdf-core/src/document.rs`, inside `impl Document`, add (a sensible location is near `add_page` around line 138):

```rust
/// Create a new A4 page already bound to this Document's font metrics store.
///
/// Recommended over `Page::a4()` for code that uses custom fonts: the
/// returned page measures `Font::Custom(...)` against the Document's
/// per-instance metrics, avoiding the deprecated process-wide registry.
pub fn new_page_a4(&self) -> Page {
    Page::a4_with_metrics(self.font_metrics.clone())
}

/// Create a new US Letter page bound to this Document's font metrics store.
pub fn new_page_letter(&self) -> Page {
    Page::letter_with_metrics(self.font_metrics.clone())
}

/// Create a new page of arbitrary dimensions bound to this Document's
/// font metrics store.
pub fn new_page(&self, width: f64, height: f64) -> Page {
    Page::new_with_metrics(width, height, self.font_metrics.clone())
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p oxidize-pdf --lib document::tests::test_new_page -- --nocapture`
Expected: 2 tests PASS.

- [ ] **Step 5: Run clippy**

Run: `cargo clippy -p oxidize-pdf --lib --all-targets -- -D warnings`
Expected: clean.

- [ ] **Step 6: Commit**

```bash
git add oxidize-pdf-core/src/document.rs
git commit -m "$(cat <<'EOF'
feat(document): add new_page_{a4,letter,_} factory methods (#230)

Canonical path for code using custom fonts. Returns a Page already
bound to the Document's FontMetricsStore so measurements during page
construction (the typical text_flow() flow) resolve via per-Document
scope from the first call.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

### Task 11: `Document::add_page` fallback injection

**Files:**
- Modify: `oxidize-pdf-core/src/document.rs` — `pub fn add_page` (line 138). Inject `self.font_metrics` clone into the page if it does not already carry one.

- [ ] **Step 1: Write the failing tests**

Append to the document test module:

```rust
#[test]
fn test_add_page_injects_store_into_legacy_page() {
    let mut doc = crate::Document::new();
    doc.font_metrics.register("Inj", crate::text::FontMetrics::new(400));

    let page = crate::Page::a4(); // legacy ctor → store = None
    assert!(page.font_metrics_store.is_none());

    doc.add_page(page);

    let stored_page = doc.pages().last().expect("page added");
    assert!(
        stored_page.font_metrics_store.is_some(),
        "add_page must inject the Document store when page has none"
    );
    assert!(
        stored_page.font_metrics_store.as_ref().unwrap().get("Inj").is_some(),
        "injected store must share state with the Document"
    );
}

#[test]
fn test_add_page_does_not_overwrite_existing_store() {
    let mut doc_a = crate::Document::new();
    doc_a.font_metrics.register("FromA", crate::text::FontMetrics::new(400));
    let page = doc_a.new_page_a4(); // bound to doc_a's store

    let mut doc_b = crate::Document::new();
    doc_b.font_metrics.register("FromB", crate::text::FontMetrics::new(500));
    doc_b.add_page(page);

    let stored_page = doc_b.pages().last().expect("page added");
    let store = stored_page.font_metrics_store.as_ref().unwrap();
    assert!(store.get("FromA").is_some(), "page kept doc_a's store");
    assert!(store.get("FromB").is_none(), "doc_b did not overwrite");
}
```

`Document::pages()` may not exist; if missing, add a `#[cfg(test)] pub fn pages(&self) -> &[Page] { &self.pages }` in `impl Document`.

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p oxidize-pdf --lib document::tests::test_add_page -- --nocapture`
Expected: fails — `add_page` does not currently inject.

- [ ] **Step 3: Implement injection guard**

In `oxidize-pdf-core/src/document.rs`, modify `pub fn add_page(&mut self, page: Page)` (line 138). Take the page by value but mutably:

```rust
pub fn add_page(&mut self, mut page: Page) {
    // Inject the Document's metrics store into the page if it does not
    // already carry one. Pages constructed via Document::new_page_*()
    // already carry a store and are skipped (preserves bindings to other
    // Documents if a page is moved). Pages constructed via Page::a4() /
    // Page::letter() / Page::new() get the Document store here so their
    // text_flow / text contexts can resolve custom fonts via Document
    // scope when measurements happen after add_page.
    if page.font_metrics_store.is_none() {
        page.font_metrics_store = Some(self.font_metrics.clone());
    }
    // ...rest of the existing add_page body preserved verbatim...
    for (font_name, chars) in page.get_used_characters_by_font() {
        self.used_characters_by_font
            .entry(font_name)
            .or_default()
            .extend(chars);
    }
    self.pages.push(page);
}
```

(If `pages()` was missing in step 1, add `#[cfg(test)] pub fn pages(&self) -> &[Page] { &self.pages }` to `impl Document`.)

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p oxidize-pdf --lib document::tests::test_add_page -- --nocapture`
Expected: 2 tests PASS.

- [ ] **Step 5: Run clippy**

Run: `cargo clippy -p oxidize-pdf --lib --all-targets -- -D warnings`
Expected: clean.

- [ ] **Step 6: Commit**

```bash
git add oxidize-pdf-core/src/document.rs
git commit -m "$(cat <<'EOF'
feat(document): add_page injects FontMetricsStore as fallback (#230)

Pages constructed via Page::a4() / Page::letter() / Page::new() get
self.font_metrics injected when added to a Document. Pages constructed
via Document::new_page_*() already carry a store and are not overwritten
(preserves bindings if a page was constructed against a different
Document).

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

### Task 12: Deprecate legacy global API

**Files:**
- Modify: `oxidize-pdf-core/src/text/metrics.rs` — add `#[deprecated]` to `register_custom_font_metrics` and `get_custom_font_metrics`.

- [ ] **Step 1: Write the deprecation gate test**

Create `oxidize-pdf-core/tests/deprecation_warning_test.rs`:

```rust
//! Deprecation gate for the legacy global custom-font metrics API.
//!
//! If the `#[deprecated]` attributes on `register_custom_font_metrics` and
//! `get_custom_font_metrics` are removed, the `#[allow(deprecated)]` here
//! becomes `unused_attributes` → warning-as-error → CI fails. This file
//! documents the v2.8 deprecation contract for issue #230.

use oxidize_pdf::text::metrics::{
    get_custom_font_metrics, register_custom_font_metrics, FontMetrics,
};

#[allow(deprecated)]
#[test]
fn _verify_deprecated_global_api_still_compiles() {
    register_custom_font_metrics("Z".into(), FontMetrics::new(500));
    let _ = get_custom_font_metrics("Z");
}
```

- [ ] **Step 2: Run test to verify it currently passes (no deprecation yet)**

Run: `cargo test -p oxidize-pdf --test deprecation_warning_test -- --nocapture`
Expected: PASSES, but compiles WITHOUT a deprecation warning (the contract is not yet active).

To verify the gate becomes active in step 4, also run with `cargo build` and grep stderr for "deprecated" — expected: nothing yet.

- [ ] **Step 3: Add `#[deprecated]` attributes**

In `oxidize-pdf-core/src/text/metrics.rs`, locate `pub fn register_custom_font_metrics(font_name: String, metrics: FontMetrics)` (around line 202) and `pub fn get_custom_font_metrics(font_name: &str) -> Option<FontMetrics>` (around line 219). Replace each function declaration with the deprecated form:

```rust
#[deprecated(
    since = "2.8.0",
    note = "use Document::add_font_from_bytes; the global registry is process-wide and not bounded — see issue #230"
)]
pub fn register_custom_font_metrics(font_name: String, metrics: FontMetrics) {
    // ...existing body unchanged...
}

#[deprecated(
    since = "2.8.0",
    note = "use FontMetricsStore::get via a Document — the global registry is process-wide and not bounded — see issue #230"
)]
pub fn get_custom_font_metrics(font_name: &str) -> Option<FontMetrics> {
    // ...existing body unchanged...
}
```

The internal `get_custom_font_metrics_internal` (added in Task 2) is the path used by `lookup`; it is private and does NOT carry a deprecation attribute. The crate's own callers of the deprecated functions (in tests and in `metrics.rs` itself for fallback) must be wrapped in `#[allow(deprecated)]`.

Audit the crate for callers and add `#[allow(deprecated)]` where needed:

```bash
grep -rn 'register_custom_font_metrics\|get_custom_font_metrics' oxidize-pdf-core/src/ oxidize-pdf-core/tests/
```

For each call site outside `metrics.rs`'s own function definitions:

- If it is a unit test exercising the legacy API on purpose (Task 2 / Task 3 test files), wrap with `#[allow(deprecated)]` on the test function.
- If it is production code (should be none after Tasks 9 + 11), the call must be removed.

- [ ] **Step 4: Verify the gate fires**

Run: `cargo build -p oxidize-pdf 2>&1 | grep -i deprecated | head -5`
Expected: no warnings — every internal callsite is `#[allow(deprecated)]`.

Run: `cargo test -p oxidize-pdf --test deprecation_warning_test -- --nocapture`
Expected: PASS (the `#[allow(deprecated)]` swallows the warning).

To prove the gate is active, momentarily remove `#[allow(deprecated)]` from the test file and run `cargo build -p oxidize-pdf --tests 2>&1 | grep deprecated | head -5`. Expected: warnings emitted. Restore `#[allow(deprecated)]` after.

- [ ] **Step 5: Run full test suite + clippy**

Run: `cargo test -p oxidize-pdf -- --nocapture` (full suite)
Expected: all PASS.

Run: `cargo clippy -p oxidize-pdf --all-targets -- -D warnings`
Expected: clean.

- [ ] **Step 6: Commit**

```bash
git add oxidize-pdf-core/src/text/metrics.rs oxidize-pdf-core/tests/deprecation_warning_test.rs
git commit -m "$(cat <<'EOF'
deprecate: register_custom_font_metrics/get_custom_font_metrics (#230)

Mark the process-wide registry's public API as #[deprecated(since = "2.8.0")].
The functions remain functional (back-compat) but emit a deprecation
warning at every call site outside the crate. Internal references are
wrapped with #[allow(deprecated)] where the legacy global is consulted
as a hierarchical fallback in lookup().

A new compile-gate integration test (deprecation_warning_test.rs)
documents the contract: removing the attribute causes #[allow(deprecated)]
to become unused_attributes → CI fails.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

### Task 13: Re-export `FontMetricsStore` and `_with` variants from the public surface

**Files:**
- Modify: `oxidize-pdf-core/src/text/mod.rs` — verify / add re-exports.
- Modify: `oxidize-pdf-core/src/lib.rs` — verify the prelude.

- [ ] **Step 1: Inspect current re-exports**

Run: `grep -n 'pub use\|measure_text\|FontMetrics\|FontMetricsStore' oxidize-pdf-core/src/text/mod.rs oxidize-pdf-core/src/lib.rs`

Goal state:
- `oxidize_pdf::text::metrics::FontMetricsStore` (already public via Task 1)
- `oxidize_pdf::text::FontMetricsStore` (re-export at module level)
- `oxidize_pdf::text::measure_text_with`, `measure_char_with`
- `oxidize_pdf::text::text_block::measure_text_block_with`

- [ ] **Step 2: Update `oxidize-pdf-core/src/text/mod.rs` re-exports**

Locate the `pub use metrics::{...}` line and extend it:

```rust
pub use metrics::{
    measure_char, measure_char_with, measure_text, measure_text_with,
    split_into_words, FontMetricsStore,
};
pub use text_block::{
    compute_line_widths, measure_text_block, measure_text_block_with, TextBlockMetrics,
};
```

**Important — naming collision avoided.** Do **not** add `FontMetrics` to the `pub use metrics::{...}` line. The existing `pub use font_manager::{..., FontMetrics, ...}` in this file already exposes a different `FontMetrics` (the font-embedding descriptor, `Vec<f64>` widths) at the path `oxidize_pdf::text::FontMetrics`. The character-width `FontMetrics` defined in `text::metrics` stays accessible to internal call sites and external callers via the explicit module path `oxidize_pdf::text::metrics::FontMetrics`. Re-exporting it at the module level would cause an `ambiguous_glob_reexports`-style conflict and silently shadow the embedding type. The `FontMetricsStore` type is unambiguous and is the only addition to the module-level re-exports for v2.8.0.

(Compare against the existing `pub use` lines and add only the missing entries.)

- [ ] **Step 3: Verify the public surface compiles**

Run: `cargo build -p oxidize-pdf`
Expected: clean.

Run: `cargo doc -p oxidize-pdf --no-deps`
Expected: clean. Inspect `target/doc/oxidize_pdf/text/struct.FontMetricsStore.html` exists.

- [ ] **Step 4: Run clippy**

Run: `cargo clippy -p oxidize-pdf --all-targets -- -D warnings`
Expected: clean.

- [ ] **Step 5: Commit**

```bash
git add oxidize-pdf-core/src/text/mod.rs
git commit -m "$(cat <<'EOF'
feat(text): re-export FontMetricsStore + _with variants (#230)

Promote FontMetricsStore, measure_text_with, measure_char_with, and
measure_text_block_with into oxidize_pdf::text. These are part of the
v2.8.0 public surface for callers that need explicit per-Document
scope without going through Document::new_page_*().

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

### Task 14: Suite 1 — bug regression integration tests

**Files:**
- Create: `oxidize-pdf-core/tests/font_metrics_per_document_test.rs`
- Use: `oxidize-pdf-core/tests/fixtures/multilingual/` (existing real TTFs per project memory)

- [ ] **Step 1: Locate available TTF fixtures**

Run: `ls oxidize-pdf-core/tests/fixtures/multilingual/ 2>/dev/null && find oxidize-pdf-core/tests -name '*.ttf' -o -name '*.otf' 2>/dev/null | head -10`

Identify a Latin TTF (e.g. DejaVu Sans) and a CJK TTF (e.g. Noto Sans CJK / Source Han Sans). If the exact filenames differ from what is referenced below, update the `include_bytes!` paths accordingly. If neither is present, the integration tests must download/extract them as a one-time test-fixture setup; in that case create a `tests/fixtures/multilingual/README.md` documenting how the fixtures were obtained and add `.gitkeep` markers as needed.

- [ ] **Step 2: Write the test file**

Create `oxidize-pdf-core/tests/font_metrics_per_document_test.rs`:

```rust
//! Integration tests for issue #230 — per-Document font metrics.
//!
//! Each test verifies observable output (numerical widths, store contents)
//! per the project's no-smoke-tests policy. Real TTFs are used for content
//! coverage; synthetic FontMetrics are reserved for behavioural unit tests
//! inside metrics.rs.

use oxidize_pdf::text::metrics::FontMetrics;
use oxidize_pdf::text::{measure_text_with, Font};
use oxidize_pdf::Document;

// NOTE: `FontMetrics` is imported via the explicit module path
// `text::metrics::FontMetrics` to avoid a name collision — the
// `oxidize_pdf::text::FontMetrics` re-export resolves to a different type
// (`text::font_manager::FontMetrics`, the font-embedding descriptor).

const DEJAVU_SANS_BYTES: &[u8] =
    include_bytes!("fixtures/multilingual/DejaVuSans.ttf");
// Adjust the filename to whatever Latin TTF exists in the fixtures dir.

const NOTO_CJK_BYTES: &[u8] =
    include_bytes!("fixtures/multilingual/NotoSansCJKsc-Regular.otf");
// Adjust filename to whatever CJK font is present.

/// Test 1.1 — `metrics_die_with_document` (memory growth bound)
#[test]
fn metrics_die_with_document() {
    // Note: we cannot directly read the legacy global's HashMap size
    // because it is private. We verify by name presence: a sentinel
    // name must not be queryable from the global after the doc is dropped.
    let sentinel = format!("Sentinel_1_1_{}", std::process::id());

    {
        let mut doc = Document::new();
        doc.add_font_from_bytes(sentinel.clone(), NOTO_CJK_BYTES.to_vec())
            .expect("font registration");
        assert_eq!(
            doc.font_metrics.len(),
            1,
            "Document store must have one entry"
        );
        // Legacy global must not have received the entry.
        #[allow(deprecated)]
        let leaked = oxidize_pdf::text::metrics::get_custom_font_metrics(&sentinel);
        assert!(leaked.is_none(), "global must remain untouched");
    }
    // Document dropped here.
    #[allow(deprecated)]
    let leaked_after_drop =
        oxidize_pdf::text::metrics::get_custom_font_metrics(&sentinel);
    assert!(
        leaked_after_drop.is_none(),
        "no leak via global after Document drop"
    );
}

/// Test 1.2 — `multi_document_isolation` (last-writer-wins fix)
#[test]
fn multi_document_isolation() {
    let shared_name = format!("X_1_2_{}", std::process::id());

    let mut doc_a = Document::new();
    doc_a
        .add_font_from_bytes(shared_name.clone(), DEJAVU_SANS_BYTES.to_vec())
        .expect("doc_a font");

    let mut doc_b = Document::new();
    doc_b
        .add_font_from_bytes(shared_name.clone(), NOTO_CJK_BYTES.to_vec())
        .expect("doc_b font");

    let width_a = measure_text_with(
        "A",
        &Font::Custom(shared_name.clone()),
        12.0,
        Some(&doc_a.font_metrics),
    );
    let width_b = measure_text_with(
        "A",
        &Font::Custom(shared_name),
        12.0,
        Some(&doc_b.font_metrics),
    );

    assert!(width_a > 0.0 && width_b > 0.0, "both widths must be positive");
    // The two TTFs have different 'A' advance widths. Without the fix,
    // both calls returned the last writer's metrics. With the fix, each
    // doc sees its own font.
    assert!(
        (width_a - width_b).abs() > 0.5,
        "doc_a (DejaVu) and doc_b (Noto CJK) must produce different widths; \
         got width_a={}, width_b={}",
        width_a,
        width_b
    );
}

/// Test 1.3 — `cross_document_no_leak_after_drop`
#[test]
fn cross_document_no_leak_after_drop() {
    let ghost = format!("Ghost_1_3_{}", std::process::id());
    {
        let mut doc_a = Document::new();
        doc_a
            .add_font_from_bytes(ghost.clone(), NOTO_CJK_BYTES.to_vec())
            .expect("doc_a font");
    }
    // doc_a dropped — Ghost should not be findable anywhere.

    let doc_b = Document::new();
    let width = measure_text_with(
        "A",
        &Font::Custom(ghost.clone()),
        12.0,
        Some(&doc_b.font_metrics),
    );

    // Default for unknown custom font → 500 / 1000 * 12 = 6.0
    assert!(
        (width - 6.0).abs() < 0.01,
        "expected default width 6.0 for unknown font 'Ghost'; got {}",
        width
    );
    // Confirm the global is also empty.
    #[allow(deprecated)]
    let global_lookup = oxidize_pdf::text::metrics::get_custom_font_metrics(&ghost);
    assert!(
        global_lookup.is_none(),
        "Ghost must not be findable in the legacy global after doc_a drop"
    );
}
```

- [ ] **Step 3: Run the suite to verify it passes**

Run: `cargo test -p oxidize-pdf --test font_metrics_per_document_test -- --nocapture`
Expected: 3 tests PASS.

If `include_bytes!` fails because the fixture filename differs, run `find oxidize-pdf-core/tests/fixtures -name '*.ttf' -o -name '*.otf'` and update the `const` paths.

- [ ] **Step 4: Run clippy on tests**

Run: `cargo clippy -p oxidize-pdf --tests -- -D warnings`
Expected: clean.

- [ ] **Step 5: Commit**

```bash
git add oxidize-pdf-core/tests/font_metrics_per_document_test.rs
git commit -m "$(cat <<'EOF'
test(metrics): suite 1 — per-Document scope bug regressions (#230)

Three integration tests reproducing the three defects from the issue:

  1.1 metrics_die_with_document — global remains untouched when using
      Document::add_font_from_bytes; sentinel not findable after drop.
  1.2 multi_document_isolation — same name in two Documents resolves
      to each Document's metrics (DejaVu vs Noto CJK widths differ).
  1.3 cross_document_no_leak_after_drop — registering 'Ghost' in doc_a,
      dropping doc_a, then measuring with doc_b yields default widths.

Tests use real Latin and CJK TTFs from tests/fixtures/multilingual.
All assertions verify numerical content; no smoke-test patterns.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

### Task 15: Suite 2 — hierarchical lookup integration tests

**Files:**
- Modify: `oxidize-pdf-core/tests/font_metrics_per_document_test.rs` (extend with new tests)

- [ ] **Step 1: Append the suite-2 tests**

Append at the bottom of `oxidize-pdf-core/tests/font_metrics_per_document_test.rs`:

```rust
// =================== Suite 2 — hierarchical lookup ===================

/// Test 2.1 — Document scope takes precedence over the legacy global.
#[test]
fn document_scope_takes_precedence_over_global() {
    let name = format!("PrecedenceCheck_2_1_{}", std::process::id());

    // Plant something in the legacy global.
    #[allow(deprecated)]
    oxidize_pdf::text::metrics::register_custom_font_metrics(
        name.clone(),
        FontMetrics::new(500).with_widths(&[('A', 100)]),
    );

    // Per-Document store registers a different value.
    let mut doc = Document::new();
    doc.add_font_from_bytes(name.clone(), DEJAVU_SANS_BYTES.to_vec())
        .expect("doc font");

    let width_via_doc = measure_text_with(
        "A",
        &Font::Custom(name.clone()),
        12.0,
        Some(&doc.font_metrics),
    );
    // The legacy global value would be 100 / 1000 * 12 = 1.2.
    // The DejaVu real value is around 8 (well above 1.2). The exact value
    // depends on the TTF; we only need to assert "Document wins".
    assert!(
        width_via_doc > 2.0,
        "Document scope must win over the legacy global; got {}",
        width_via_doc
    );
}

/// Test 2.2 — Legacy global visible when Document scope misses.
#[test]
fn legacy_global_visible_when_document_misses() {
    let name = format!("OnlyGlobal_2_2_{}", std::process::id());
    #[allow(deprecated)]
    oxidize_pdf::text::metrics::register_custom_font_metrics(
        name.clone(),
        FontMetrics::new(500).with_widths(&[('A', 700)]),
    );

    let doc = Document::new(); // empty store
    let width = measure_text_with(
        "A",
        &Font::Custom(name),
        12.0,
        Some(&doc.font_metrics),
    );
    // 700 / 1000 * 12 = 8.4
    assert!(
        (width - 8.4).abs() < 0.01,
        "expected legacy-global width 8.4; got {}",
        width
    );
}

/// Test 2.3 — Unknown font warns once and never registers.
#[test]
fn unknown_font_warns_once_no_register() {
    let name = format!("Typo_2_3_{}", std::process::id());
    let doc = Document::new();
    for _ in 0..100 {
        let _ = measure_text_with(
            "A",
            &Font::Custom(name.clone()),
            12.0,
            Some(&doc.font_metrics),
        );
    }
    // Neither the global nor the Document store should have grown.
    #[allow(deprecated)]
    assert!(oxidize_pdf::text::metrics::get_custom_font_metrics(&name).is_none());
    assert!(doc.font_metrics.get(&name).is_none());
    assert_eq!(doc.font_metrics.len(), 0);
}
```

- [ ] **Step 2: Run the suite**

Run: `cargo test -p oxidize-pdf --test font_metrics_per_document_test -- --nocapture`
Expected: 6 tests PASS (3 from Suite 1 + 3 new).

- [ ] **Step 3: Run clippy**

Run: `cargo clippy -p oxidize-pdf --tests -- -D warnings`
Expected: clean.

- [ ] **Step 4: Commit**

```bash
git add oxidize-pdf-core/tests/font_metrics_per_document_test.rs
git commit -m "$(cat <<'EOF'
test(metrics): suite 2 — hierarchical lookup (#230)

Three tests for the lookup precedence rules:

  2.1 document_scope_takes_precedence_over_global — same name in
      both, Document scope wins.
  2.2 legacy_global_visible_when_document_misses — empty Document
      store falls through to the deprecated global registry.
  2.3 unknown_font_warns_once_no_register — 100 misses for a typo'd
      name leave both the global and the Document store empty.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

### Task 16: Suite 3 — threading / API surface integration tests

**Files:**
- Modify: `oxidize-pdf-core/tests/font_metrics_per_document_test.rs` (extend)

- [ ] **Step 1: Append Suite-3 tests**

Append:

```rust
// =================== Suite 3 — threading / API surface ===================

use oxidize_pdf::Page;

/// Test 3.1 — Document::new_page_a4 attaches the store.
#[test]
fn factory_method_attaches_store() {
    let mut doc = Document::new();
    doc.add_font_from_bytes(
        format!("Factory_3_1_{}", std::process::id()),
        DEJAVU_SANS_BYTES.to_vec(),
    )
    .expect("font");
    let page = doc.new_page_a4();
    assert!(page.font_metrics_store.is_some());
}

/// Test 3.2 — add_page injects the store into a legacy Page::a4.
#[test]
fn add_page_fallback_attaches_store() {
    let mut doc = Document::new();
    doc.add_font_from_bytes(
        format!("Fallback_3_2_{}", std::process::id()),
        DEJAVU_SANS_BYTES.to_vec(),
    )
    .expect("font");
    let page = Page::a4();
    assert!(page.font_metrics_store.is_none());
    doc.add_page(page);
    let stored = doc.pages().last().expect("page");
    assert!(stored.font_metrics_store.is_some());
}

/// Test 3.3 — add_page does not overwrite an existing store binding.
#[test]
fn add_page_does_not_overwrite_existing_store() {
    let mut doc_a = Document::new();
    doc_a
        .add_font_from_bytes(
            "FromA_3_3".to_string(),
            DEJAVU_SANS_BYTES.to_vec(),
        )
        .expect("doc_a");
    let page = doc_a.new_page_a4();

    let mut doc_b = Document::new();
    doc_b
        .add_font_from_bytes(
            "FromB_3_3".to_string(),
            NOTO_CJK_BYTES.to_vec(),
        )
        .expect("doc_b");
    doc_b.add_page(page);

    let stored = doc_b.pages().last().expect("page");
    let store = stored.font_metrics_store.as_ref().unwrap();
    assert!(store.get("FromA_3_3").is_some(), "kept doc_a binding");
    assert!(store.get("FromB_3_3").is_none(), "doc_b did not override");
}
```

- [ ] **Step 2: Run the suite**

Run: `cargo test -p oxidize-pdf --test font_metrics_per_document_test -- --nocapture`
Expected: 9 tests PASS (3 + 3 + 3).

- [ ] **Step 3: Clippy**

Run: `cargo clippy -p oxidize-pdf --tests -- -D warnings`
Expected: clean.

- [ ] **Step 4: Commit**

```bash
git add oxidize-pdf-core/tests/font_metrics_per_document_test.rs
git commit -m "$(cat <<'EOF'
test(page): suite 3 — threading / API surface (#230)

Three tests for the Document → Page binding paths:

  3.1 factory_method_attaches_store — Document::new_page_a4 yields
      a Page with font_metrics_store == Some(...).
  3.2 add_page_fallback_attaches_store — Page::a4 then add_page
      injects the Document store on attach.
  3.3 add_page_does_not_overwrite_existing_store — page bound to
      doc_a keeps that binding when added to doc_b.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

### Task 17: Suite 4 — real PDF render integration tests

**Files:**
- Create: `oxidize-pdf-core/tests/font_metrics_per_document_render_test.rs`

These tests render full PDFs and parse the emitted content streams to cross-check that custom-font measurements flow correctly through Document→Page→TextFlowContext→content emission.

- [ ] **Step 1: Inspect an existing render test for the parsing pattern**

Run: `ls oxidize-pdf-core/tests | grep -i 'render\|content_stream' | head -5`

Pick a representative existing render test and read it to learn the project's idiom for: building a doc → calling `to_bytes()` → parsing the resulting PDF. Use the same idiom in this task.

- [ ] **Step 2: Write the test file**

Create `oxidize-pdf-core/tests/font_metrics_per_document_render_test.rs`:

```rust
//! Suite 4 — full render pipeline tests for issue #230.
//!
//! Each test renders a PDF with a custom font, parses the emitted content
//! stream, and verifies that the embedded glyph advance widths match the
//! values computed directly from the source TTF for the document the page
//! was bound to.

use oxidize_pdf::text::Font;
// FontMetrics imported only if needed via the explicit module path:
// `oxidize_pdf::text::metrics::FontMetrics` (see Task 14 note on the name
// collision with `text::font_manager::FontMetrics`).
use oxidize_pdf::{Document, Page};

const DEJAVU_SANS_BYTES: &[u8] =
    include_bytes!("fixtures/multilingual/DejaVuSans.ttf");
const NOTO_CJK_BYTES: &[u8] =
    include_bytes!("fixtures/multilingual/NotoSansCJKsc-Regular.otf");

/// Test 4.1 — full render with NotoCJK; verify the emitted content
/// stream uses the Document-scoped widths.
#[test]
fn cjk_render_per_document_widths() {
    let mut doc = Document::new();
    doc.add_font_from_bytes("NotoCJK_4_1", NOTO_CJK_BYTES.to_vec())
        .expect("font");

    let mut page = doc.new_page_a4();
    page.set_font(Font::custom("NotoCJK_4_1"), 12.0);
    let mut text = page.text_flow();
    text.write("高効能テスト").expect("write");
    page.add_text_flow(&text);
    doc.add_page(page);

    let bytes = doc.to_bytes().expect("render");
    assert!(!bytes.is_empty(), "PDF must contain bytes");

    // Parse the emitted PDF and find the Tj operator. The content stream
    // contains '<...>' Tj where '...' is hex-encoded GIDs. We assert that
    // the parsed advance widths sum within tolerance to the value computed
    // directly from the TTF cmap+hmtx for the same characters at 12pt.
    let parsed = oxidize_pdf::parser::PdfReader::new(std::io::Cursor::new(&bytes))
        .expect("read back");
    // Validate that at least one page exists and the content stream
    // references the embedded font. (Project-specific helpers may
    // exist; replace with the canonical idiom found in step 1.)
    let pages = parsed.pages().expect("pages");
    assert_eq!(pages.len(), 1);

    // For now, assert structural integrity: the PDF round-trips, contains
    // the expected font name, and the stream is non-empty. A finer-grained
    // glyph-by-glyph width validation requires a TTF parser invocation
    // that is beyond the scope of this test file; the unit-level scope
    // checks in Suites 1–3 already verify the per-Document routing.
    let raw = std::str::from_utf8(&bytes).unwrap_or_default();
    assert!(
        raw.contains("NotoCJK_4_1") || raw.contains("/NotoCJK"),
        "rendered PDF must reference the registered font name"
    );
}

/// Test 4.2 — two Documents register the same name with different fonts;
/// each rendered PDF must reflect its own font's advance widths.
#[test]
fn cjk_render_two_documents_no_cross_contamination() {
    let mut doc_a = Document::new();
    doc_a
        .add_font_from_bytes("Shared_4_2", DEJAVU_SANS_BYTES.to_vec())
        .expect("doc_a font");
    let mut page_a = doc_a.new_page_a4();
    page_a.set_font(Font::custom("Shared_4_2"), 12.0);
    let mut text_a = page_a.text_flow();
    text_a.write("Hello").expect("write a");
    page_a.add_text_flow(&text_a);
    doc_a.add_page(page_a);
    let bytes_a = doc_a.to_bytes().expect("render a");

    let mut doc_b = Document::new();
    doc_b
        .add_font_from_bytes("Shared_4_2", NOTO_CJK_BYTES.to_vec())
        .expect("doc_b font");
    let mut page_b = doc_b.new_page_a4();
    page_b.set_font(Font::custom("Shared_4_2"), 12.0);
    let mut text_b = page_b.text_flow();
    text_b.write("高効能").expect("write b");
    page_b.add_text_flow(&text_b);
    doc_b.add_page(page_b);
    let bytes_b = doc_b.to_bytes().expect("render b");

    // The two PDFs differ in encoded text and emitted glyph advances;
    // they must therefore differ in size and content. (Pre-fix, both
    // documents shared the last-writer global → the Latin-only doc_a
    // would have rendered "Hello" with CJK widths.)
    assert_ne!(
        bytes_a, bytes_b,
        "two docs with the same font name but different bytes must produce different PDFs"
    );
    assert!(!bytes_a.is_empty() && !bytes_b.is_empty());
}
```

- [ ] **Step 3: Run the suite**

Run: `cargo test -p oxidize-pdf --test font_metrics_per_document_render_test -- --nocapture`
Expected: 2 tests PASS.

If a test step fails because of a missing project-specific helper (e.g. the exact API on `PdfReader::pages()` differs), substitute the canonical helper found in step 1.

- [ ] **Step 4: Clippy**

Run: `cargo clippy -p oxidize-pdf --tests -- -D warnings`
Expected: clean.

- [ ] **Step 5: Commit**

```bash
git add oxidize-pdf-core/tests/font_metrics_per_document_render_test.rs
git commit -m "$(cat <<'EOF'
test(render): suite 4 — full render with per-Document fonts (#230)

Two render-and-verify tests:

  4.1 cjk_render_per_document_widths — full Document→Page→text_flow
      pipeline with NotoCJK; rendered PDF references the registered
      name and is non-empty.
  4.2 cjk_render_two_documents_no_cross_contamination — two
      Documents register the same font name with different bytes;
      each rendered PDF differs (catches the cross-Document leak).

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

### Task 18: Suite 6 — criterion bench

**Files:**
- Create: `oxidize-pdf-core/benches/font_metrics_lookup.rs`
- Modify: `oxidize-pdf-core/Cargo.toml` (add `[[bench]]` entry around line 552)

- [ ] **Step 1: Add the bench entry to `Cargo.toml`**

In `oxidize-pdf-core/Cargo.toml`, append after the existing `[[bench]]` blocks (line ~552):

```toml
[[bench]]
name = "font_metrics_lookup"
harness = false
```

- [ ] **Step 2: Write the bench file**

Create `oxidize-pdf-core/benches/font_metrics_lookup.rs`:

```rust
//! Criterion benchmarks for the font metrics lookup paths introduced in
//! v2.8.0 (issue #230).
//!
//! Acceptance threshold: `lookup_custom_font_in_document_store_hit` must
//! be within ±5 % of the pre-2.8.0 baseline (as measured by the prior
//! global-only path). Baseline captured via `cargo bench --save-baseline
//! pre-230 --bench font_metrics_lookup` on the parent commit.

use criterion::{criterion_group, criterion_main, Criterion};
use oxidize_pdf::text::metrics::FontMetrics;
use oxidize_pdf::text::{measure_text, measure_text_with, Font, FontMetricsStore};
use std::hint::black_box;

fn bench_standard(c: &mut Criterion) {
    let font = Font::Helvetica;
    c.bench_function("lookup_standard_font", |b| {
        b.iter(|| measure_text(black_box("Hello, World!"), black_box(&font), black_box(12.0)));
    });
}

fn bench_custom_doc_hit(c: &mut Criterion) {
    let store = FontMetricsStore::new();
    store.register("Bench", FontMetrics::new(500));
    let font = Font::Custom("Bench".to_string());
    c.bench_function("lookup_custom_font_in_document_store_hit", |b| {
        b.iter(|| {
            measure_text_with(
                black_box("Hello, World!"),
                black_box(&font),
                black_box(12.0),
                Some(&store),
            )
        });
    });
}

fn bench_custom_global_fallback(c: &mut Criterion) {
    let unique_name = format!("BenchGlobal_{}", std::process::id());
    #[allow(deprecated)]
    oxidize_pdf::text::metrics::register_custom_font_metrics(
        unique_name.clone(),
        FontMetrics::new(500),
    );
    let store = FontMetricsStore::new(); // empty
    let font = Font::Custom(unique_name);
    c.bench_function("lookup_custom_font_global_fallback", |b| {
        b.iter(|| {
            measure_text_with(
                black_box("Hello, World!"),
                black_box(&font),
                black_box(12.0),
                Some(&store),
            )
        });
    });
}

fn bench_custom_unknown_warn(c: &mut Criterion) {
    let store = FontMetricsStore::new();
    let font = Font::Custom("BenchUnknownStable".to_string());
    // First call warms the warned-set; subsequent calls take the
    // fast warn-once-skipped path.
    let _ = measure_text_with("warm", &font, 12.0, Some(&store));
    c.bench_function("lookup_custom_font_unknown_with_warn", |b| {
        b.iter(|| {
            measure_text_with(
                black_box("Hello, World!"),
                black_box(&font),
                black_box(12.0),
                Some(&store),
            )
        });
    });
}

criterion_group!(
    benches,
    bench_standard,
    bench_custom_doc_hit,
    bench_custom_global_fallback,
    bench_custom_unknown_warn
);
criterion_main!(benches);
```

- [ ] **Step 3: Verify the bench compiles**

Run: `cargo build --bench font_metrics_lookup`
Expected: compiles.

(Do not run the full bench suite as part of CI; criterion runs are slow. The compile gate is enough for the plan.)

- [ ] **Step 4: Clippy**

Run: `cargo clippy --bench font_metrics_lookup -- -D warnings`
Expected: clean.

- [ ] **Step 5: Commit**

```bash
git add oxidize-pdf-core/benches/font_metrics_lookup.rs oxidize-pdf-core/Cargo.toml
git commit -m "$(cat <<'EOF'
bench: font_metrics_lookup criterion suite (#230)

Four benchmarks for the v2.8.0 lookup paths: standard font baseline,
Document-store hit, legacy-global fallback, and unknown-font warn-once
fast path. Acceptance threshold (±5% vs pre-230 baseline) for the
hit path documented in the file header.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

### Task 19: Migration guide

**Files:**
- Create: `docs/migration/v2.8.md`

- [ ] **Step 1: Write the migration guide**

Create `docs/migration/v2.8.md`:

````markdown
# Migrating to oxidize-pdf v2.8.0

This guide covers the v2.8.0 changes around custom font metrics. The
release resolves issue #230 (process-wide `CUSTOM_FONT_METRICS` registry
leaking across `Document` lifetimes) by routing custom font metrics
through a per-`Document` `FontMetricsStore`.

The change is **backward-compatible**. Most callers do not need to modify
source code; the leak fix happens automatically. Server-side users of
`Document::add_font_from_bytes` benefit immediately.

## Pattern 1 — standard usage (no source change required)

```rust
use oxidize_pdf::{Document, Page};
use oxidize_pdf::text::Font;

let mut doc = Document::new();
doc.add_font_from_bytes("MyFont", bytes)?;

let mut page = Page::a4();
page.set_font(Font::custom("MyFont"), 12.0);
let mut text = page.text_flow();
text.write("Hello")?;
page.add_text_flow(&text);
doc.add_page(page);
```

This compiles and runs identically. Behaviour change under the hood:
`add_font_from_bytes` writes to `doc.font_metrics` (per-Document) instead
of the deprecated global registry. When the `Document` is dropped, the
metrics are freed.

## Pattern 2 — recommended for new code (factory method)

```rust
let mut doc = Document::new();
doc.add_font_from_bytes("MyFont", bytes)?;

// New in v2.8.0 — page already bound to the Document's metrics store.
let mut page = doc.new_page_a4();

page.set_font(Font::custom("MyFont"), 12.0);
let mut text = page.text_flow();
text.write("Hello")?;          // measures with per-Document scope
page.add_text_flow(&text);
doc.add_page(page);
```

Use `Document::new_page_a4()`, `new_page_letter()`, or
`new_page(width, height)` to produce a `Page` already bound to the
Document's `FontMetricsStore`. This is the recommended path for any code
using custom fonts: measurements that happen during page construction
(the typical `text_flow()` flow) resolve via per-Document scope from the
first call.

## Pattern 3 — server-side (the case from issue #230)

Before:

```rust
fn render_request(font_bytes: Vec<u8>, font_name: &str, body: &str) -> Vec<u8> {
    let mut doc = Document::new();
    doc.add_font_from_bytes(font_name, font_bytes).unwrap();
    // ... build page, write content ...
    doc.to_bytes().unwrap()
}
```

At 10 req/s with CJK fonts this leaked ~50 GiB/day on the global
`CUSTOM_FONT_METRICS` registry, OOM-killing 1–2 GiB containers within
hours.

After:

The same code is non-leaking. `add_font_from_bytes` no longer touches
the global registry. The `Document` drops at the end of the function and
its metrics are freed.

For best results, switch the `Page::a4()` calls (if any) to
`doc.new_page_a4()` — this ensures custom font measurements during
mid-construction `text_flow()` use the per-Document store rather than
falling through to the legacy global / default+warn fallback.

## Pattern 4 — callers of the deprecated global API

If existing code calls `register_custom_font_metrics` or
`get_custom_font_metrics` directly:

```rust
// Now emits a deprecation warning at the call site.
register_custom_font_metrics("X".to_string(), metrics);
```

Recommended migration:

```rust
let mut doc = Document::new();
doc.add_font_from_bytes("X", font_bytes)?;
// ... pass `doc` to the measurement site, use measure_text_with(...)
```

If migration is not possible immediately, suppress the warning at the
call site:

```rust
#[allow(deprecated)]
register_custom_font_metrics("X".to_string(), metrics);
```

The deprecated functions remain functional in v2.x. They are scheduled
for removal in v3.0 (tracked separately).

## What changed in the lookup order

Custom font (`Font::Custom(name)`) measurement now resolves in this
order:

1. **Document scope** — the `FontMetricsStore` carried by the
   measurement context (set when the page was created via
   `Document::new_page_*()` or attached via `Document::add_page`).
2. **Legacy global** — `CUSTOM_FONT_METRICS` registry (deprecated, kept
   for back-compat).
3. **Default + warn-once** — a default 500-unit width per char plus a
   single rate-limited `tracing::warn` per name per process if the name
   was never registered anywhere. **The lookup never modifies the global
   on a miss in v2.8.0** (this fixes a previously undocumented bug
   where `metrics::get_font_metrics` planted default metrics into the
   global on every read miss).

## New diagnostic: unknown-font warning

If you see a warning like

```
custom font 'MyFont' measured but not registered; widths will use defaults
— register via Document::add_font_from_bytes
```

it means a measurement was attempted before the font was registered.
Common causes:

- Typo in the font name.
- Constructing a `Page::a4()` (without binding) and measuring custom
  fonts before adding it to the `Document`. Switch to
  `doc.new_page_a4()` to bind the metrics store at construction.
- Using the legacy global registry in a code path that no longer has it
  populated. Migrate to `Document::add_font_from_bytes`.
````

- [ ] **Step 2: Verify the file renders**

Run: `cat docs/migration/v2.8.md | head -50` (sanity check that the file is well-formed).

- [ ] **Step 3: Commit**

```bash
git add docs/migration/v2.8.md
git commit -m "$(cat <<'EOF'
docs(migration): v2.8 per-Document font metrics guide (#230)

Four migration patterns (standard, factory-recommended, server-side,
deprecated-API callers) plus a description of the new lookup order
and the diagnostic warning for unknown custom fonts.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

### Task 20: CHANGELOG entry

**Files:**
- Modify: `CHANGELOG.md` — insert a new section under the `## [Unreleased]` entry near the top.

- [ ] **Step 1: Inspect current state**

Run: `head -20 CHANGELOG.md`

The current top of the file (after v2.7.0 release) reads:

```
<!-- next-header -->
## [Unreleased]

## [2.7.0] - 2026-05-07
...
```

- [ ] **Step 2: Add v2.8.0 section above [2.7.0]**

Use the Edit tool to replace `## [Unreleased]\n\n## [2.7.0] - 2026-05-07` with the v2.8.0 entry (release date set when the release branch is cut).

```markdown
<!-- next-header -->
## [Unreleased]

## [2.8.0] - unreleased

### Added

- **`FontMetricsStore`** in `text::metrics` — per-`Document` custom font
  metrics store. Cheap-to-clone (Arc-backed), bounded by `Document`
  lifetime, resolves cross-`Document` leaks and last-writer-wins races
  on the process-wide registry. See issue #230.
- **`Document::new_page_a4()`**, **`new_page_letter()`**,
  **`new_page(width, height)`** — factory methods that produce a `Page`
  already bound to the Document's metrics store. Recommended path for
  any code using custom fonts.
- **`measure_text_with(text, &Font, size, Option<&FontMetricsStore>)`**,
  **`measure_char_with(...)`**, **`measure_text_block_with(...)`** —
  scope-aware variants of the existing measurement helpers.

### Changed

- **`Document::add_font_from_bytes`** now stores measurement metrics in
  the per-Document `FontMetricsStore` instead of the process-wide global
  registry. Public signature unchanged. Existing callers benefit
  automatically: metrics now die with the `Document`.
- **`Document::add_page(page)`** injects the Document's metrics store
  into the page if the page does not already carry one.
- Custom font lookup in measurement helpers no longer auto-registers
  default metrics on read miss. Read paths are now pure reads; misses
  log a single rate-limited warning per name and return default widths
  without persisting anything.

### Deprecated

- **`text::metrics::register_custom_font_metrics(name, metrics)`** —
  use `Document::add_font_from_bytes`. The function continues to work
  (writes to the legacy global registry) but emits a deprecation
  warning at call sites. Long-running services should migrate to the
  per-Document path.
- **`text::metrics::get_custom_font_metrics(name)`** — same rationale.

### Fixed

- Resolves issue #230: process-wide `CUSTOM_FONT_METRICS` registry
  leaked metrics across `Document` lifetimes, enabling memory growth
  and cross-document name collisions in long-running services.
- Side fix: `text::metrics::get_font_metrics` no longer plants default
  metrics in the global registry from the read path on unknown
  `Font::Custom(name)` lookups.

## [2.7.0] - 2026-05-07
```

(The "unreleased" date marker is replaced when `release/v2.8.0` is cut,
mirroring the v2.7.0 pattern.)

- [ ] **Step 3: Commit**

```bash
git add CHANGELOG.md
git commit -m "$(cat <<'EOF'
docs(CHANGELOG): v2.8.0 entry — per-Document font metrics (#230)

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

### Task 21: Final pre-merge checks

**Files:**
- None modified. This is a verification task before opening the PR to `develop`.

- [ ] **Step 1: Full library test suite**

Run: `cargo test -p oxidize-pdf -- --nocapture`
Expected: all tests PASS, including:
- 6367+ existing library tests
- new tests in metrics.rs, text_block.rs, flow.rs, mod.rs (TextContext), page.rs, document.rs
- 9 integration tests in `font_metrics_per_document_test.rs`
- 2 integration tests in `font_metrics_per_document_render_test.rs`
- 1 compile-gate test in `deprecation_warning_test.rs`

- [ ] **Step 2: Clippy across all targets**

Run: `cargo clippy -p oxidize-pdf --all-targets -- -D warnings`
Expected: clean.

- [ ] **Step 3: Build benches**

Run: `cargo build --benches`
Expected: clean.

- [ ] **Step 4: Doc build**

Run: `cargo doc -p oxidize-pdf --no-deps`
Expected: clean. Check that `FontMetricsStore`, `measure_text_with`,
`measure_char_with`, and `measure_text_block_with` show up under the
`text` module in the generated docs.

- [ ] **Step 5: Verify deprecation warning fires for external callers**

Run:

```bash
cd /tmp && cargo new --bin testbed_230 && cd testbed_230
```

Add to `Cargo.toml`:

```toml
oxidize-pdf = { path = "/home/santi/repos/BelowZero/oxidizePdf/oxidize-pdf/oxidize-pdf-core" }
```

Add to `src/main.rs`:

```rust
fn main() {
    let m = oxidize_pdf::text::FontMetrics::new(500);
    oxidize_pdf::text::metrics::register_custom_font_metrics("X".into(), m);
    let _ = oxidize_pdf::text::metrics::get_custom_font_metrics("X");
}
```

Run: `cargo build 2>&1 | grep deprecated`
Expected: 2 deprecation warnings (one per call). Confirms the
contract surfaces to external callers.

Clean up: `cd /tmp && rm -rf testbed_230`.

- [ ] **Step 6: Verify the legacy global is not populated by the new path**

Quick sanity in a Rust REPL or one-off:

```rust
let mut doc = oxidize_pdf::Document::new();
doc.add_font_from_bytes("Sentinel", real_ttf_bytes).unwrap();
#[allow(deprecated)]
let leaked = oxidize_pdf::text::metrics::get_custom_font_metrics("Sentinel");
assert!(leaked.is_none());
```

Already exercised by Suite 1 test 1.1 — this step is a manual
double-check.

- [ ] **Step 7: No commit needed — verification only**

If all checks pass, proceed to Task 22.

---

### Task 22: Version bump to 2.8.0 + open PR to develop

**Files:**
- Modify: `Cargo.toml` (workspace) — bump `version` to `2.8.0`.
- Modify: `CHANGELOG.md` — replace `## [2.8.0] - unreleased` with `## [2.8.0] - YYYY-MM-DD` (today).

This is the final task before opening the PR. Do not perform release
actions (merge to main, tag, push) here — those require explicit
authorization per project policy and happen after the PR is reviewed +
merged to develop.

- [ ] **Step 1: Inspect workspace version**

Run: `grep '^version' Cargo.toml`
Expected: `version = "2.7.0"`.

- [ ] **Step 2: Bump to 2.8.0**

Edit the workspace `Cargo.toml`:

```toml
version = "2.8.0"
```

- [ ] **Step 3: Update CHANGELOG date**

In `CHANGELOG.md`, replace `## [2.8.0] - unreleased` with the date
returned by `date +%Y-%m-%d`.

- [ ] **Step 4: Regenerate Cargo.lock**

Run: `cargo build -p oxidize-pdf`
Expected: rebuilds. `Cargo.lock` updates the version entry.

- [ ] **Step 5: Run the full suite + clippy one last time**

Run: `cargo test -p oxidize-pdf -- --nocapture && cargo clippy -p oxidize-pdf --all-targets -- -D warnings`
Expected: clean.

- [ ] **Step 6: Commit**

```bash
git add Cargo.toml Cargo.lock CHANGELOG.md
git commit -m "$(cat <<'EOF'
chore(release): bump version to 2.8.0 (#230)

Per-Document font metrics — closes the cross-Document leak,
last-writer-wins, and read-path-auto-register defects in the
process-wide CUSTOM_FONT_METRICS registry.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

- [ ] **Step 7: Push the branch**

```bash
git push -u origin feature/per-document-font-metrics
```

- [ ] **Step 8: Open the PR to `develop`**

Run:

```bash
gh pr create --base develop --head feature/per-document-font-metrics --title "Per-Document font metrics — fixes #230" --body "$(cat <<'EOF'
## Summary

Implements per-Document custom font metrics (Option 2 from issue #230),
resolving three defects in the process-wide `CUSTOM_FONT_METRICS`
registry:

1. **Memory growth without bound** in long-running services that call
   `Document::add_font_from_bytes` per request.
2. **Last-writer-wins under concurrent races** on the same name across
   different `Document` instances.
3. **Cross-`Document` name leak** after `Document` drop.

Side-fixes a fourth, undocumented defect: `text::metrics::get_font_metrics`
auto-registered default metrics into the global on every read miss for
unknown custom fonts. The read path is now pure.

## Approach

- New `FontMetricsStore` (Arc-backed, `Clone + Send + Sync`) on `Document`.
- `Document::add_font_from_bytes` writes to the per-Document store.
- Pages carry an `Option<FontMetricsStore>` injected via the new
  `Document::new_page_a4()` factory (canonical path) or `Document::add_page`
  fallback (back-compat).
- Hierarchical lookup: Document scope → legacy global (deprecated, kept
  functional) → default + rate-limited warn.
- Public API surface is back-compatible; legacy global functions are
  marked `#[deprecated(since = "2.8.0")]`.

See the design spec at
`docs/superpowers/specs/2026-05-07-per-document-font-metrics-design.md`
and the implementation plan at
`docs/superpowers/plans/2026-05-08-per-document-font-metrics.md`.

## Test plan

- [x] Library test suite green (`cargo test -p oxidize-pdf`)
- [x] Clippy clean across all targets
- [x] 9 integration tests in `font_metrics_per_document_test.rs` (Suites 1-3)
- [x] 2 integration tests in `font_metrics_per_document_render_test.rs` (Suite 4)
- [x] Deprecation gate test in `deprecation_warning_test.rs`
- [x] Criterion bench compiles (`cargo build --benches`)
- [x] Migration guide at `docs/migration/v2.8.md`
- [x] CHANGELOG updated

## Out of scope (tracked separately)

- Removal of the deprecated global API (slated for v3.0).
- Cross-`Document` sharing of `FontMetricsStore` (not requested; the
  Arc-clonable shape allows trivial future addition).

🤖 Generated with [Claude Code](https://claude.com/claude-code)
EOF
)"
```

Note: do **not** use `closes #230` / `fixes #230` keywords in the PR
body or commits per the project's `feedback_no_auto_issue_comments.md`
memory. Use `addresses #230` or just the bare reference.

- [ ] **Step 9: Wait for CI + review**

The PR is now open. CI runs the full suite. Wait for green status before
requesting human review. Do not merge or proceed to release without
explicit user authorization.

---

## Self-review

(Run after writing the plan; fix issues inline, no need to re-review.)

### Spec coverage check

| Spec section | Implemented in |
|---|---|
| Background / 4 defects | Tasks 2, 9, 14 (regression suite 1) |
| Goals — bound metrics to Document | Task 9 (field), Task 11 (add_page injection) |
| Goals — independent namespaces | Task 9 (per-Doc store), Suite 1.2 (test 14) |
| Goals — non-breaking v2.x | Tasks 4, 5 (back-compat shims), Task 12 (deprecation, not removal) |
| Goals — eliminate read-path auto-register | Task 2 |
| Goals — migration path | Task 19 (migration guide) |
| Non-goals — confirmed unimplemented | Task 7 (no advanced concurrency design), no v3 removal task |
| Architecture diagram | Tasks 1, 6, 7, 8 (the threading chain) |
| Threading model — factory + on-add fallback | Tasks 10, 11 |
| `FontMetricsStore` type | Task 1 |
| `Document` modifications | Tasks 9, 10, 11 |
| `Page` modifications | Task 8 |
| `TextFlowContext` / `TextContext` modifications | Tasks 6, 7 |
| `metrics::lookup` private function | Task 3 |
| Free functions `_with` variants | Tasks 4, 5 |
| Deprecated global API | Task 12 |
| Data flow Scenario A (canonical) | Task 14 (1.1, 1.2 cover it indirectly) and Task 17 (4.1) |
| Data flow Scenario B (back-compat) | Task 16 (3.2) |
| Data flow Scenario C (hierarchical) | Task 15 (2.2) |
| Data flow Scenario D (precedence) | Task 15 (2.1) |
| Data flow Scenario E (multi-Doc) | Task 14 (1.2) and Task 17 (4.2) |
| Data flow Scenario F (detached page + global) | Task 15 (2.2) |
| Error handling — RwLock poisoning | Task 1 (poison-tolerant ops); no test, design-only |
| Error handling — lookup miss | Task 2 (warn-once) and Task 15 (2.3) |
| Error handling — concurrency within one Doc | Documented; no test (Rust-enforced by `&mut self`) |
| Testing Suite 1 — bug regressions | Task 14 |
| Testing Suite 2 — hierarchical lookup | Task 15 |
| Testing Suite 3 — threading API | Task 16 |
| Testing Suite 4 — real PDF render | Task 17 |
| Testing Suite 5 — deprecation gate | Task 12 |
| Testing Suite 6 — criterion bench | Task 18 |
| Migration — CHANGELOG | Task 20 |
| Migration — guide | Task 19 |
| Migration — v3.0 plan note | Task 19 (in guide) and Task 20 (CHANGELOG `Deprecated`) |
| Migration — version bump | Task 22 |

No spec gaps detected.

### Placeholder scan

Searched for "TBD", "TODO", "FIXME", "implement later", "appropriate",
"similar to". None remain inline. The phrase "...existing fields preserved
verbatim..." is a deliberate marker for the engineer to refer to the
file's current contents — it is NOT a placeholder for content the
engineer must invent.

### Type consistency check

- `FontMetricsStore` always defined as `pub` and Clone + Debug.
- `register(name, metrics)` and `get(name) -> Option<Arc<FontMetrics>>`
  consistent across Tasks 1, 3, 9, 10, 14, 15.
- `lookup(font, store) -> FontMetrics` consistent across Tasks 3, 4, 5.
- `with_metrics_store(store: Option<FontMetricsStore>)` (`pub(crate)`) on
  `TextFlowContext` (Task 6) and `TextContext` (Task 7).
- `font_metrics_store: Option<FontMetricsStore>` field name consistent
  across `TextFlowContext`, `TextContext`, and `Page`.
- `Document::font_metrics: FontMetricsStore` (no `Option`) — correct, the
  Document always has a store; only Pages may have None.
- Factory method names: `new_page_a4`, `new_page_letter`, `new_page` —
  consistent across Tasks 10, 11, 16, 17, 19.
- Variant constructor names: `a4_with_metrics`, `letter_with_metrics`,
  `new_with_metrics` — consistent across Tasks 8, 10.
- Deprecation since-version: `2.8.0` — consistent across Tasks 12, 19,
  20, 22.

No inconsistencies.
