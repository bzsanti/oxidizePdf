//! Deprecation gate for the legacy global custom-font metrics API.
//!
//! If the `#[deprecated]` attributes on `register_custom_font_metrics` and
//! `get_custom_font_metrics` are removed, the `#[allow(deprecated)]` here
//! becomes `unused_attributes` → warning-as-error → CI fails. This file
//! documents the v2.8 deprecation contract for issue #230.

#[allow(deprecated)]
use oxidize_pdf::text::metrics::{
    get_custom_font_metrics, register_custom_font_metrics, FontMetrics,
};

#[allow(deprecated)]
#[test]
fn _verify_deprecated_global_api_still_compiles() {
    register_custom_font_metrics("Z".into(), FontMetrics::new(500));
    let _ = get_custom_font_metrics("Z");
}
