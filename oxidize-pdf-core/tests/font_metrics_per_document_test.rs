//! Integration tests for issue #230 — per-Document font metrics.
//!
//! Each test verifies observable output (numerical widths, store contents)
//! per the project's no-smoke-tests policy. Real TTFs are used for content
//! coverage; synthetic FontMetrics are reserved for behavioural unit tests
//! inside metrics.rs.

use oxidize_pdf::text::metrics::FontMetrics;
use oxidize_pdf::text::{measure_text_with, Font};
use oxidize_pdf::{Document, Page};

// NOTE: `FontMetrics` (the character-width metrics) lives at
// `oxidize_pdf::text::metrics::FontMetrics`. The `oxidize_pdf::text::FontMetrics`
// re-export resolves to a different type (`text::font_manager::FontMetrics`,
// the font-embedding descriptor). These tests do not need either type by
// name — measurements go through `measure_text_with` against the
// `Document::font_metrics` store.

const LATIN_FONT_PATH: &str = "../test-pdfs/Roboto-Regular.ttf";
const CJK_FONT_PATH: &str = "../test-pdfs/SourceHanSansSC-Regular.otf";

fn load_latin_font() -> Option<Vec<u8>> {
    if std::path::Path::new(LATIN_FONT_PATH).exists() {
        Some(std::fs::read(LATIN_FONT_PATH).expect("read Latin TTF fixture"))
    } else {
        eprintln!("SKIPPED: Latin font fixture not found at {LATIN_FONT_PATH}");
        None
    }
}

fn load_cjk_font() -> Option<Vec<u8>> {
    if std::path::Path::new(CJK_FONT_PATH).exists() {
        Some(std::fs::read(CJK_FONT_PATH).expect("read CJK OTF fixture"))
    } else {
        eprintln!("SKIPPED: CJK font fixture not found at {CJK_FONT_PATH}");
        None
    }
}

/// Test 1.1 — `metrics_die_with_document` (memory growth bound)
#[test]
fn metrics_die_with_document() {
    let cjk = match load_cjk_font() {
        Some(b) => b,
        None => return,
    };

    let sentinel = format!("Sentinel_1_1_{}", std::process::id());

    {
        let mut doc = Document::new();
        doc.add_font_from_bytes(&sentinel, cjk)
            .expect("font registration");
        assert_eq!(
            doc.font_metrics().len(),
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
    let leaked_after_drop = oxidize_pdf::text::metrics::get_custom_font_metrics(&sentinel);
    assert!(
        leaked_after_drop.is_none(),
        "no leak via global after Document drop"
    );
}

/// Test 1.2 — `multi_document_isolation` (last-writer-wins fix)
#[test]
fn multi_document_isolation() {
    let latin = match load_latin_font() {
        Some(b) => b,
        None => return,
    };
    let cjk = match load_cjk_font() {
        Some(b) => b,
        None => return,
    };

    let shared_name = format!("X_1_2_{}", std::process::id());

    let mut doc_a = Document::new();
    doc_a
        .add_font_from_bytes(&shared_name, latin)
        .expect("doc_a font");

    let mut doc_b = Document::new();
    doc_b
        .add_font_from_bytes(&shared_name, cjk)
        .expect("doc_b font");

    let width_a = measure_text_with(
        "A",
        &Font::Custom(shared_name.clone()),
        12.0,
        Some(doc_a.font_metrics()),
    );
    let width_b = measure_text_with(
        "A",
        &Font::Custom(shared_name),
        12.0,
        Some(doc_b.font_metrics()),
    );

    assert!(
        width_a > 0.0 && width_b > 0.0,
        "both widths must be positive"
    );
    // The two TTFs have different 'A' advance widths. Without the fix,
    // both calls returned the last writer's metrics. With the fix, each
    // doc sees its own font.
    assert!(
        (width_a - width_b).abs() > 0.5,
        "doc_a (Roboto) and doc_b (SourceHanSansSC) must produce different widths; \
         got width_a={width_a}, width_b={width_b}"
    );
}

/// Test 1.3 — `cross_document_no_leak_after_drop`
#[test]
fn cross_document_no_leak_after_drop() {
    let cjk = match load_cjk_font() {
        Some(b) => b,
        None => return,
    };

    let ghost = format!("Ghost_1_3_{}", std::process::id());
    {
        let mut doc_a = Document::new();
        doc_a.add_font_from_bytes(&ghost, cjk).expect("doc_a font");
    }
    // doc_a dropped — Ghost should not be findable anywhere.

    let doc_b = Document::new();
    let width = measure_text_with(
        "A",
        &Font::Custom(ghost.clone()),
        12.0,
        Some(doc_b.font_metrics()),
    );

    // Default for unknown custom font → Helvetica-like metrics
    // ('A' = 667 units) → 667 / 1000 * 12 = 8.004
    let expected = 667.0 * 12.0 / 1000.0;
    assert!(
        (width - expected).abs() < 0.01,
        "expected default Helvetica-like width {expected} for unknown font 'Ghost'; got {width}"
    );
    // Confirm the global is also empty.
    #[allow(deprecated)]
    let global_lookup = oxidize_pdf::text::metrics::get_custom_font_metrics(&ghost);
    assert!(
        global_lookup.is_none(),
        "Ghost must not be findable in the legacy global after doc_a drop"
    );
}

// =================== Suite 2 — hierarchical lookup ===================

/// Test 2.1 — Document scope takes precedence over the legacy global.
#[test]
fn document_scope_takes_precedence_over_global() {
    let latin = match load_latin_font() {
        Some(b) => b,
        None => return,
    };

    let name = format!("PrecedenceCheck_2_1_{}", std::process::id());

    // Plant something in the legacy global with a known small width.
    #[allow(deprecated)]
    oxidize_pdf::text::metrics::register_custom_font_metrics(
        name.clone(),
        FontMetrics::new(500).with_widths(&[('A', 100)]),
    );

    // Per-Document store registers a different font under the same name.
    let mut doc = Document::new();
    doc.add_font_from_bytes(&name, latin).expect("doc font");

    let width_via_doc = measure_text_with(
        "A",
        &Font::Custom(name.clone()),
        12.0,
        Some(doc.font_metrics()),
    );
    // The legacy global value would be 100 / 1000 * 12 = 1.2.
    // Roboto's real 'A' is around 8 (well above 1.2). The exact value
    // depends on the TTF; we only need to assert "Document wins".
    assert!(
        width_via_doc > 2.0,
        "Document scope must win over the legacy global; got {width_via_doc}"
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
    let width = measure_text_with("A", &Font::Custom(name), 12.0, Some(doc.font_metrics()));
    // 700 / 1000 * 12 = 8.4
    assert!(
        (width - 8.4).abs() < 0.01,
        "expected legacy-global width 8.4; got {width}"
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
            Some(doc.font_metrics()),
        );
    }
    // Neither the global nor the Document store should have grown.
    #[allow(deprecated)]
    let leaked = oxidize_pdf::text::metrics::get_custom_font_metrics(&name);
    assert!(leaked.is_none(), "global must remain empty");
    assert!(
        doc.font_metrics().get(&name).is_none(),
        "doc store must remain empty"
    );
    assert_eq!(
        doc.font_metrics().len(),
        0,
        "doc store size must remain zero"
    );
}

// =================== Suite 3 — threading / API surface ===================

/// Test 3.1 — Document::new_page_a4 attaches the store.
#[test]
fn factory_method_attaches_store() {
    let latin = match load_latin_font() {
        Some(b) => b,
        None => return,
    };

    let mut doc = Document::new();
    doc.add_font_from_bytes(format!("Factory_3_1_{}", std::process::id()), latin)
        .expect("font");
    let page = doc.new_page_a4();
    assert!(page.font_metrics_store().is_some());
}

/// Test 3.2 — add_page injects the store into a legacy Page::a4.
#[test]
fn add_page_fallback_attaches_store() {
    let latin = match load_latin_font() {
        Some(b) => b,
        None => return,
    };

    let mut doc = Document::new();
    doc.add_font_from_bytes(format!("Fallback_3_2_{}", std::process::id()), latin)
        .expect("font");
    let page = Page::a4();
    assert!(page.font_metrics_store().is_none());
    doc.add_page(page);
    let stored = doc.pages().last().expect("page");
    assert!(stored.font_metrics_store().is_some());
}

/// Test 3.3 — add_page does not overwrite an existing store binding.
#[test]
fn add_page_does_not_overwrite_existing_store() {
    let latin = match load_latin_font() {
        Some(b) => b,
        None => return,
    };
    let cjk = match load_cjk_font() {
        Some(b) => b,
        None => return,
    };

    let mut doc_a = Document::new();
    doc_a
        .add_font_from_bytes("FromA_3_3", latin)
        .expect("doc_a");
    let page = doc_a.new_page_a4();

    let mut doc_b = Document::new();
    doc_b.add_font_from_bytes("FromB_3_3", cjk).expect("doc_b");
    doc_b.add_page(page);

    let stored = doc_b.pages().last().expect("page");
    let store = stored.font_metrics_store().expect("page kept its store");
    assert!(store.get("FromA_3_3").is_some(), "kept doc_a binding");
    assert!(store.get("FromB_3_3").is_none(), "doc_b did not override");
}
