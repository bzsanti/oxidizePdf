//! Integration tests for issue #230 — per-Document font metrics.
//!
//! Each test verifies observable output (numerical widths, store contents)
//! per the project's no-smoke-tests policy. Real TTFs are used for content
//! coverage; synthetic FontMetrics are reserved for behavioural unit tests
//! inside metrics.rs.

use oxidize_pdf::text::{measure_text_with, Font};
use oxidize_pdf::Document;

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
