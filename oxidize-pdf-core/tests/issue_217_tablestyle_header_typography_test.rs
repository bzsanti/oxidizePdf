//! Integration tests for `TableStyle` header-typography overrides — issue #217.
//!
//! Before this fix, `add_styled_table` hardcoded `Font::Helvetica` + `bold: true`
//! at the call-site that builds `HeaderStyle`. None of the four presets
//! (`minimal` / `simple` / `professional` / `colorful`) could override this.
//!
//! The tests verify content-stream output, never just absence of crash.

use oxidize_pdf::page_tables::{PageTables, TableStyle};
use oxidize_pdf::text::Font;
use oxidize_pdf::Page;

// ---------- helpers ----------

fn render_styled_table_and_get_ops(style: TableStyle) -> String {
    let mut page = Page::a4();
    page.add_styled_table(
        vec!["Item".into(), "Qty".into()],
        vec![vec!["Widget".into(), "2".into()]],
        50.0,
        700.0,
        400.0,
        style,
    )
    .expect("add_styled_table must succeed");
    page.graphics().get_operations().to_string()
}

// ---------- spec-compliance defaults ----------
//
// Pre-fix, `add_styled_table` called `add_row(headers)` (not `add_header_row`),
// so `HeaderStyle` was *never* applied — the header rendered in plain
// `/Helvetica`. The two tests below lock in the fixed default (Helvetica-Bold
// for the bundled presets that supply colour fields) so a future regression
// to `add_row` is caught.

#[test]
fn default_professional_preset_emits_helvetica_bold_header() {
    let ops = render_styled_table_and_get_ops(TableStyle::professional());
    assert!(
        ops.contains("/Helvetica-Bold"),
        "professional preset must default to /Helvetica-Bold for the header; ops:\n{ops}"
    );
}

#[test]
fn default_colorful_preset_emits_helvetica_bold_header() {
    let ops = render_styled_table_and_get_ops(TableStyle::colorful());
    assert!(
        ops.contains("/Helvetica-Bold"),
        "colorful preset must default to /Helvetica-Bold for the header; ops:\n{ops}"
    );
}

// ---------- new fields directly settable ----------

#[test]
fn header_font_override_to_times_roman_with_bold_emits_times_bold() {
    // Times-Roman + bold=true → Times-Bold (per existing HeaderStyle bold→variant
    // mapping in table.rs:625-628).
    let style = TableStyle {
        header_font: Some(Font::TimesRoman),
        header_bold: Some(true),
        ..TableStyle::professional()
    };
    let ops = render_styled_table_and_get_ops(style);
    assert!(
        ops.contains("/Times-Bold"),
        "header_font=TimesRoman + header_bold=true must emit /Times-Bold; ops:\n{ops}"
    );
    assert!(
        !ops.contains("/Helvetica-Bold"),
        "header_font override must replace default /Helvetica-Bold; ops:\n{ops}"
    );
}

#[test]
fn header_bold_false_with_helvetica_emits_helvetica_not_helvetica_bold() {
    // Helvetica + bold=false → Helvetica (no bold variant). The data row also
    // renders Helvetica, so the assertion is "emits /Helvetica AND never
    // /Helvetica-Bold". This is the regression for the hardcoded `bold: true`.
    let style = TableStyle {
        header_font: Some(Font::Helvetica),
        header_bold: Some(false),
        ..TableStyle::professional()
    };
    let ops = render_styled_table_and_get_ops(style);
    assert!(
        ops.contains("/Helvetica"),
        "must emit /Helvetica; ops:\n{ops}"
    );
    assert!(
        !ops.contains("/Helvetica-Bold"),
        "header_bold=false must NOT emit /Helvetica-Bold; ops:\n{ops}"
    );
}

#[test]
fn header_font_courier_with_bold_emits_courier_bold() {
    let style = TableStyle {
        header_font: Some(Font::Courier),
        header_bold: Some(true),
        ..TableStyle::professional()
    };
    let ops = render_styled_table_and_get_ops(style);
    assert!(
        ops.contains("/Courier-Bold"),
        "Courier + bold=true must emit /Courier-Bold; ops:\n{ops}"
    );
}

#[test]
fn header_font_oblique_with_bold_preserves_oblique_variant() {
    // HelveticaOblique is not in the bold→variant mapping match arms (only
    // Helvetica/TimesRoman/Courier are). The fallthrough `_ => style.font`
    // means an oblique font stays oblique even with bold=true. This locks
    // that behaviour: setting header_font=HelveticaOblique must NOT collapse
    // back to Helvetica-Bold.
    let style = TableStyle {
        header_font: Some(Font::HelveticaOblique),
        header_bold: Some(true),
        ..TableStyle::professional()
    };
    let ops = render_styled_table_and_get_ops(style);
    assert!(
        ops.contains("/Helvetica-Oblique"),
        "Oblique font must be preserved; ops:\n{ops}"
    );
}

// ---------- presets default to None for typography overrides ----------

#[test]
fn all_presets_default_header_typography_overrides_to_none() {
    for (name, style) in [
        ("minimal", TableStyle::minimal()),
        ("simple", TableStyle::simple()),
        ("professional", TableStyle::professional()),
        ("colorful", TableStyle::colorful()),
    ] {
        assert!(
            style.header_font.is_none(),
            "{name}: header_font must default to None so callers can override"
        );
        assert!(
            style.header_bold.is_none(),
            "{name}: header_bold must default to None so callers can override"
        );
    }
}

// ---------- builder methods (ergonomic chaining on presets) ----------

#[test]
fn with_header_font_sets_field_and_chains() {
    let style = TableStyle::professional().with_header_font(Font::TimesRoman);
    assert_eq!(style.header_font, Some(Font::TimesRoman));
    // Chain preserves other preset fields:
    assert!(style.header_background.is_some());
    assert!(style.header_text_color.is_some());
}

#[test]
fn with_header_bold_sets_field_and_chains() {
    let style = TableStyle::simple().with_header_bold(false);
    assert_eq!(style.header_bold, Some(false));
}

#[test]
fn builder_chain_full_typography_override() {
    let style = TableStyle::professional()
        .with_header_font(Font::TimesRoman)
        .with_header_bold(false);
    assert_eq!(style.header_font, Some(Font::TimesRoman));
    assert_eq!(style.header_bold, Some(false));
}

// ---------- preset chained override actually renders correctly ----------

#[test]
fn professional_preset_with_times_bold_override_renders_times_bold() {
    // End-to-end: pick a preset, override only the typography, render, verify.
    let style = TableStyle::professional()
        .with_header_font(Font::TimesRoman)
        .with_header_bold(true);
    let ops = render_styled_table_and_get_ops(style);
    assert!(
        ops.contains("/Times-Bold"),
        "professional + Times-Bold override must emit /Times-Bold; ops:\n{ops}"
    );
}
