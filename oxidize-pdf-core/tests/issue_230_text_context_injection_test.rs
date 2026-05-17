#![cfg(feature = "internal-testing")]
//! Integration tests for issue #230 follow-up M1 — `Document::add_page`
//! must also inject the per-Document `FontMetricsStore` into the page's
//! `text_context.font_metrics_store`, not only into `page.font_metrics_store`.
//!
//! Bug: pages constructed via `Page::a4() / Page::letter() / Page::new()`
//! carry `text_context: TextContext::new()` with `font_metrics_store: None`.
//! Before the fix, `Document::add_page` only set `page.font_metrics_store`;
//! the text context was left wired to the legacy global registry — defeating
//! the per-Document scope that issue #230 closed architecturally.
//!
//! These tests are content-verifying (no smoke asserts):
//! - Test 1 plants the same font name in the legacy global AND in the
//!   per-Document store with NUMERICALLY DIFFERENT widths. If the fix is
//!   present, measuring via `page.text_context.font_metrics_store` returns
//!   the Document width; if absent (or partial), the call falls through.
//! - Test 2 pushes operations into `page.text_context` BEFORE `add_page` and
//!   verifies the operation count is preserved across the injection — i.e.
//!   the fix mutates only the `font_metrics_store` field, never replacing
//!   the whole `TextContext`.
//! - Test 3 exercises the factory path (`doc_a.new_page_a4()`) to confirm
//!   the `is_none()` guard in `add_page` does NOT overwrite a factory-
//!   supplied store when the page is later handed to a second `Document`.

use oxidize_pdf::text::metrics::FontMetrics;
use oxidize_pdf::text::{measure_text_with, Font};
use oxidize_pdf::{Document, Page};

/// Test 1.1 — `Document::add_page` injects the per-Document store into
/// `page.text_context.font_metrics_store` (which is `None` on `Page::a4()`).
///
/// Width plan:
/// - Legacy global: `'A' = 100` units → `12pt × 100 / 1000 = 1.2 pt`.
/// - Per-Document store: `'A' = 800` units → `12pt × 800 / 1000 = 9.6 pt`.
///
/// Gap of 8.4 pt is far above measurement noise; either width is unique to
/// its source. The test panics with `.expect("text_context must carry...")`
/// on v2.8.0 because the accessor returns `None`.
#[test]
fn add_page_injects_store_into_text_context() {
    let name = format!("TC_Wide_1_1_{}", std::process::id());

    // `register_custom_font_metrics` is `#[deprecated since "2.8.0"]` (issue #230, Task 12).
    // The legacy global registry is exactly what this test contrasts against the
    // per-Document store; the deprecation is intentional and this call is the
    // canonical way to plant a value into the global path. When the API is
    // eventually removed, the entire `global_width` arm of this test must be
    // dropped — see oxidize-pdf-core/src/text/metrics.rs:299.
    #[allow(deprecated)]
    oxidize_pdf::text::metrics::register_custom_font_metrics(
        name.clone(),
        FontMetrics::new(500).with_widths(&[('A', 100)]),
    );

    let mut doc = Document::new();
    doc.font_metrics()
        .register(&name, FontMetrics::new(500).with_widths(&[('A', 800)]));

    let page = Page::a4();
    doc.add_page(page);

    let stored_page = doc
        .pages()
        .last()
        .expect("doc must have a page after add_page");

    let tc_store = stored_page
        .text_context_metrics_store_for_test()
        .expect("text_context must carry the Document store after add_page");

    let doc_width = measure_text_with("A", &Font::Custom(name.clone()), 12.0, Some(tc_store));
    let global_width = measure_text_with("A", &Font::Custom(name.clone()), 12.0, None);

    assert!(
        (global_width - 1.2_f64).abs() < 0.05,
        "global store width must be 1.2 pt for 'A' at 12pt; got {global_width}"
    );
    assert!(
        (doc_width - 9.6_f64).abs() < 0.05,
        "text_context store width must be 9.6 pt for 'A' at 12pt; got {doc_width}"
    );
    assert!(
        (doc_width - global_width).abs() > 5.0,
        "Document scope and legacy global must differ by >5 pt; \
         doc={doc_width}, global={global_width}"
    );
}

/// Test 1.2 — Operations accumulated in `page.text_context` BEFORE
/// `add_page` are preserved across the injection. The fix must mutate only
/// the `font_metrics_store` field, never reconstruct the `TextContext`.
#[test]
fn text_context_ops_preserved_across_add_page() {
    let mut page = Page::a4();
    page.text()
        .set_font(Font::Helvetica, 14.0)
        .at(50.0, 700.0)
        .write("hello")
        .expect("write should succeed");

    let ops_before = page.text_context_ops_count_for_test();
    assert!(
        ops_before > 0,
        "pre-condition: page.text() must have pushed at least one op; \
         got {ops_before}"
    );

    let mut doc = Document::new();
    doc.add_page(page);
    let stored_page = doc.pages().last().expect("doc must have a page");

    let ops_after = stored_page.text_context_ops_count_for_test();
    assert_eq!(
        ops_after, ops_before,
        "add_page must preserve text_context ops; before={ops_before}, after={ops_after}"
    );
}

/// Test 1.3 — Factory-supplied stores are NOT overwritten when the page is
/// added to a second `Document`. Verifies the `is_none()` guard in
/// `add_page` correctly skips re-injection.
///
/// Setup: `doc_a` registers `'A' = 800` (→ 9.6 pt). Factory page from
/// `doc_a.new_page_a4()` carries `doc_a`'s store. Hand the page to `doc_b`
/// (empty store). After `doc_b.add_page(page)`, measuring through the page's
/// `text_context.font_metrics_store` must still see `doc_a`'s width — if
/// the guard accidentally re-injected, the text context would now point at
/// `doc_b`'s empty store and the measurement would fall through to the
/// (unset) legacy global → default fallback width, well below 9.6 pt.
#[test]
fn factory_path_text_context_store_unchanged() {
    let name = format!("FactoryGuard_1_3_{}", std::process::id());

    let doc_a = Document::new();
    doc_a
        .font_metrics()
        .register(&name, FontMetrics::new(500).with_widths(&[('A', 800)]));

    let page = doc_a.new_page_a4();

    // Sanity: factory already wired the store on the page before transfer.
    assert!(
        page.text_context_metrics_store_for_test().is_some(),
        "factory new_page_a4 must wire text_context store"
    );
    let factory_width = measure_text_with(
        "A",
        &Font::Custom(name.clone()),
        12.0,
        page.text_context_metrics_store_for_test(),
    );
    assert!(
        (factory_width - 9.6).abs() < 0.05,
        "factory path must resolve to doc_a's width 9.6 pt; got {factory_width}"
    );

    let mut doc_b = Document::new();
    doc_b.add_page(page);
    let stored_page = doc_b.pages().last().expect("doc_b must have a page");

    let post_add_width = measure_text_with(
        "A",
        &Font::Custom(name.clone()),
        12.0,
        stored_page.text_context_metrics_store_for_test(),
    );
    assert!(
        (post_add_width - 9.6).abs() < 0.05,
        "doc_b.add_page must NOT overwrite doc_a's text_context store; \
         expected 9.6 pt, got {post_add_width}"
    );
}
