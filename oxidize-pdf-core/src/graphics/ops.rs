//! Typed intermediate representation for PDF content-stream operators.
//!
//! Both `GraphicsContext` and `TextContext` accumulate `Op` values instead
//! of pre-formatted byte strings. `Page` orders ops in call order across
//! contexts so the painter model is preserved (issue #227). All non-finite
//! `f64` inputs are sanitised at serialisation time via `finite_or_zero`,
//! mirroring the colour-emission fix in 2.6.0 (issues #220, #221).
//!
//! Variants are introduced as the contexts that emit them are migrated.
//! The module-wide `dead_code` allowance covers variants whose emitting
//! site arrives in a later phase — it is removed once every context has
//! been migrated through the v2.7.0 refactor.

#![allow(dead_code)]

use super::color::{finite_or_zero, write_fill_color_bytes, write_stroke_color_bytes, Color};
use std::io::Write;

/// One element of a `TJ` text array (`Op::ShowTextArray`).
///
/// A `TJ` array interleaves shown glyph strings with numeric position
/// adjustments (ISO 32000-1 §9.4.3). `Glyphs` holds uppercase hex digits
/// (2-byte codes for a Type0/Identity font); `Adjust` is a position
/// adjustment in thousandths of a unit of text space (negative moves the
/// next glyph forward — the convention used for kerning).
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum TextArrayElement {
    /// `<HEX>` — uppercase hex digits, emitted inside angle brackets.
    Glyphs(Vec<u8>),
    /// A numeric position adjustment (thousandths of text-space units).
    Adjust(f32),
}

/// PDF content-stream operators as typed values.
///
/// `Op::Raw` is an escape hatch for operators that have not been modelled
/// yet, or for content sourced from external/preserved streams. Everything
/// else is sanitised at the emission boundary by `serialize_ops`.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Op {
    // ── path construction ──
    /// `x y m`
    MoveTo { x: f64, y: f64 },
    /// `x y l`
    LineTo { x: f64, y: f64 },
    /// `x1 y1 x2 y2 x3 y3 c`
    CurveTo {
        x1: f64,
        y1: f64,
        x2: f64,
        y2: f64,
        x3: f64,
        y3: f64,
    },
    /// `x y w h re`
    Rect { x: f64, y: f64, w: f64, h: f64 },
    /// `h`
    ClosePath,

    // ── path painting ──
    /// `S`
    Stroke,
    /// `f` — fill using non-zero winding rule
    FillNonZero,
    /// `B` — fill and stroke
    FillStroke,

    // ── colour state ──
    /// non-stroking colour selector (`rg` / `g` / `k`); routed through
    /// `write_fill_color_bytes` so the existing NaN/inf sanitisation and
    /// device-space selection are reused verbatim.
    SetFillColor(Color),
    /// stroking colour selector (`RG` / `G` / `K`)
    SetStrokeColor(Color),
    /// `/name cs` — selects a named non-stroking colour space
    SetFillColorSpace(String),
    /// `/name CS`
    SetStrokeColorSpace(String),
    /// `c1 c2 … sc` — non-stroking colour components in the active space
    SetFillColorComponents(Vec<f64>),
    /// `c1 c2 … SC`
    SetStrokeColorComponents(Vec<f64>),

    // ── line / dash ──
    /// `width w`
    SetLineWidth(f64),
    /// `style J`
    SetLineCap(u8),
    /// `style j`
    SetLineJoin(u8),
    /// `limit M`
    SetMiterLimit(f64),
    /// `[…] phase d` — pattern is already formatted (NaN/inf sanitisation
    /// lives in `LineDashPattern::to_pdf_string` if/when added).
    SetDashPatternRaw(String),
    /// `flatness i`
    SetFlatness(f64),

    // ── ExtGState ──
    /// `/name gs`
    SetExtGState(String),
    /// `/name ri`
    SetRenderingIntent(String),

    // ── state stack ──
    /// `q`
    SaveState,
    /// `Q`
    RestoreState,

    // ── transforms ──
    /// `a b c d e f cm`
    Cm {
        a: f64,
        b: f64,
        c: f64,
        d: f64,
        e: f64,
        f: f64,
    },

    // ── images / forms ──
    /// `/name Do`
    InvokeXObject(String),

    // ── text ──
    /// `BT`
    BeginText,
    /// `ET`
    EndText,
    /// `/name size Tf`
    SetFont { name: String, size: f64 },
    /// `tx ty Td`
    SetTextPosition { x: f64, y: f64 },
    /// `(escaped) Tj` — the bytes inside the parens are pre-escaped per
    /// ISO 32000-1 §7.3.4.2 (literal strings).
    ShowText(Vec<u8>),
    /// `<HEX> Tj` — the bytes are uppercase hex digits.
    ShowTextHex(Vec<u8>),
    /// `[ <HEX> adj <HEX> ... ] TJ` — show glyph strings with per-element
    /// position adjustments (ISO 32000-1 §9.4.3). Used to draw a positioned
    /// glyph run (kerning / shaped output) over a Type0/Identity font.
    ShowTextArray(Vec<TextArrayElement>),
    /// `value Tw`
    SetWordSpacing(f64),
    /// `value Tc`
    SetCharSpacing(f64),
    /// `value Tz` — horizontal scaling, expressed as a percentage
    /// (`100.0` is "no scaling"). Caller passes the percentage value.
    SetHorizontalScaling(f64),
    /// `value TL` — text leading
    SetLeading(f64),
    /// `value Ts` — text rise
    SetTextRise(f64),
    /// `mode Tr` — text rendering mode (`0`..=`7` per ISO 32000-1 §9.3.6)
    SetRenderingMode(u8),

    // ── path painting (no-op) ──
    /// `n` — end the path without filling or stroking. Required to
    /// terminate a clipping path (`W n`) per ISO 32000-1 §8.5.4.
    EndPath,

    // ── clipping ──
    /// `W` — modify current clipping path using the non-zero winding rule.
    ClipNonZero,
    /// `W*` — modify current clipping path using the even-odd rule.
    ClipEvenOdd,
    /// `W S` — clip then stroke. Used by the `clip_stroke` builder for
    /// the common pattern of stroking the boundary of the clip region.
    ClipStroke,

    // ── shading ──
    /// `/name sh` — paint the named shading into the current clip region
    /// (ISO 32000-1 §8.7.4.2). The shading must be registered in
    /// `/Resources/Shading` under `name`.
    PaintShading(String),

    // ── special ──
    /// `% comment` — a PDF comment line. Used for transparency-group
    /// markers; ignored by viewers but useful for diff/debug.
    Comment(String),
    /// Bytes emitted verbatim. Use for operators not yet modelled or for
    /// content sourced from external/preserved streams.
    Raw(Vec<u8>),
}

/// Emit exactly representable integral magnitudes without float formatting.
/// Keep the sign bit separately so negative zero remains observable. Values
/// outside this bounded fast path use Rust's original formatter unchanged.
fn write_integral(out: &mut Vec<u8>, value: f64) -> bool {
    let magnitude = value.abs();
    if magnitude > 9_007_199_254_740_992.0 || !value.is_finite() {
        return false;
    }
    let mut integer = magnitude as u64;
    if integer as f64 != magnitude {
        return false;
    }
    let mut digits = [0u8; 16]; // 2^53 has sixteen decimal digits.
    let mut start = digits.len();
    loop {
        start -= 1;
        digits[start] = b'0' + (integer % 10) as u8;
        integer /= 10;
        if integer == 0 {
            break;
        }
    }
    if value.is_sign_negative() {
        out.push(b'-');
    }
    out.extend_from_slice(&digits[start..]);
    true
}

fn write_fixed<const PRECISION: usize>(out: &mut Vec<u8>, value: f64) {
    let value = finite_or_zero(value);
    if write_integral(out, value) {
        out.push(b'.');
        out.extend_from_slice(&[b'0'; PRECISION]);
    } else {
        write!(out, "{value:.PRECISION$}").expect("writing to Vec<u8> never fails");
    }
}

// Inline so fixed-size coordinate tuples specialize away slice-length dispatch.
#[inline]
fn write_numbers<const PRECISION: usize>(out: &mut Vec<u8>, values: &[f64], suffix: &[u8]) {
    // Keep one formatter invocation for fractional coordinate tuples. Formatting
    // each operand separately adds overhead to the fallback-only workload.
    let integral = values.iter().all(|&v| {
        let v = finite_or_zero(v).abs();
        v <= 9_007_199_254_740_992.0 && (v as u64) as f64 == v
    });
    if !integral {
        match values {
            [a, b] => {
                write!(
                    out,
                    "{:.PRECISION$} {:.PRECISION$} ",
                    finite_or_zero(*a),
                    finite_or_zero(*b)
                )
                .expect("writing to Vec<u8> never fails");
            }
            [a, b, c, d] => {
                write!(
                    out,
                    "{:.PRECISION$} {:.PRECISION$} {:.PRECISION$} {:.PRECISION$} ",
                    finite_or_zero(*a),
                    finite_or_zero(*b),
                    finite_or_zero(*c),
                    finite_or_zero(*d)
                )
                .expect("writing to Vec<u8> never fails");
            }
            [a, b, c, d, e, f] => {
                write!(out, "{:.PRECISION$} {:.PRECISION$} {:.PRECISION$} {:.PRECISION$} {:.PRECISION$} {:.PRECISION$} ",
                    finite_or_zero(*a), finite_or_zero(*b), finite_or_zero(*c), finite_or_zero(*d), finite_or_zero(*e), finite_or_zero(*f))
                    .expect("writing to Vec<u8> never fails");
            }
            _ => {
                for &value in values {
                    write_fixed::<PRECISION>(out, value);
                    out.push(b' ');
                }
            }
        }
    } else {
        for &value in values {
            write_fixed::<PRECISION>(out, value);
            out.push(b' ');
        }
    }
    out.extend_from_slice(suffix);
}

/// Serialises a slice of `Op` values to a byte buffer in PDF
/// content-stream syntax. Non-finite floats are clamped to `0.0` via
/// `finite_or_zero` at the emission boundary.
pub(crate) fn serialize_ops(out: &mut Vec<u8>, ops: &[Op]) {
    for op in ops {
        match op {
            // ── path construction ──
            Op::MoveTo { x, y } => {
                write_numbers::<2>(out, &[*x, *y], b"m\n");
            }
            Op::LineTo { x, y } => {
                write_numbers::<2>(out, &[*x, *y], b"l\n");
            }
            Op::CurveTo {
                x1,
                y1,
                x2,
                y2,
                x3,
                y3,
            } => {
                write_numbers::<2>(out, &[*x1, *y1, *x2, *y2, *x3, *y3], b"c\n");
            }
            Op::Rect { x, y, w, h } => {
                write_numbers::<2>(out, &[*x, *y, *w, *h], b"re\n");
            }
            Op::ClosePath => out.extend_from_slice(b"h\n"),

            // ── path painting ──
            Op::Stroke => out.extend_from_slice(b"S\n"),
            Op::FillNonZero => out.extend_from_slice(b"f\n"),
            Op::FillStroke => out.extend_from_slice(b"B\n"),

            // ── colour state ──
            Op::SetFillColor(color) => write_fill_color_bytes(out, *color),
            Op::SetStrokeColor(color) => write_stroke_color_bytes(out, *color),
            Op::SetFillColorSpace(name) => {
                writeln!(out, "/{name} cs").expect("writing to Vec<u8> never fails");
            }
            Op::SetStrokeColorSpace(name) => {
                writeln!(out, "/{name} CS").expect("writing to Vec<u8> never fails");
            }
            Op::SetFillColorComponents(values) => {
                write_numbers::<4>(out, values, b"sc\n");
            }
            Op::SetStrokeColorComponents(values) => {
                write_numbers::<4>(out, values, b"SC\n");
            }

            // ── line / dash ──
            Op::SetLineWidth(width) => {
                write_numbers::<2>(out, &[*width], b"w\n");
            }
            Op::SetLineCap(cap) => {
                writeln!(out, "{cap} J").expect("writing to Vec<u8> never fails");
            }
            Op::SetLineJoin(join) => {
                writeln!(out, "{join} j").expect("writing to Vec<u8> never fails");
            }
            Op::SetMiterLimit(limit) => {
                write_numbers::<2>(out, &[*limit], b"M\n");
            }
            Op::SetDashPatternRaw(s) => {
                writeln!(out, "{s} d").expect("writing to Vec<u8> never fails");
            }
            Op::SetFlatness(value) => {
                write_numbers::<2>(out, &[*value], b"i\n");
            }

            // ── ExtGState ──
            Op::SetExtGState(name) => {
                writeln!(out, "/{name} gs").expect("writing to Vec<u8> never fails");
            }
            Op::SetRenderingIntent(name) => {
                writeln!(out, "/{name} ri").expect("writing to Vec<u8> never fails");
            }

            // ── state stack ──
            Op::SaveState => out.extend_from_slice(b"q\n"),
            Op::RestoreState => out.extend_from_slice(b"Q\n"),

            // ── transforms ──
            Op::Cm { a, b, c, d, e, f } => {
                write_numbers::<2>(out, &[*a, *b, *c, *d, *e, *f], b"cm\n");
            }

            // ── images / forms ──
            Op::InvokeXObject(name) => {
                writeln!(out, "/{name} Do").expect("writing to Vec<u8> never fails");
            }

            // ── text ──
            Op::BeginText => out.extend_from_slice(b"BT\n"),
            Op::EndText => out.extend_from_slice(b"ET\n"),
            Op::SetFont { name, size } => {
                let size = finite_or_zero(*size);
                out.push(b'/');
                out.extend_from_slice(name.as_bytes());
                out.push(b' ');
                if !write_integral(out, size) {
                    write!(out, "{size}").expect("writing to Vec<u8> never fails");
                }
                out.extend_from_slice(b" Tf\n");
            }
            Op::SetTextPosition { x, y } => {
                write_numbers::<2>(out, &[*x, *y], b"Td\n");
            }
            Op::ShowText(bytes) => {
                out.push(b'(');
                out.extend_from_slice(bytes);
                out.extend_from_slice(b") Tj\n");
            }
            Op::ShowTextHex(bytes) => {
                out.push(b'<');
                out.extend_from_slice(bytes);
                out.extend_from_slice(b"> Tj\n");
            }
            Op::ShowTextArray(elements) => {
                out.extend_from_slice(b"[");
                for element in elements {
                    match element {
                        TextArrayElement::Glyphs(bytes) => {
                            out.extend_from_slice(b" <");
                            out.extend_from_slice(bytes);
                            out.push(b'>');
                        }
                        TextArrayElement::Adjust(value) => {
                            let v = finite_or_zero(*value as f64);
                            write!(out, " {v:.2}").expect("writing to Vec<u8> never fails");
                        }
                    }
                }
                out.extend_from_slice(b" ] TJ\n");
            }
            Op::SetWordSpacing(value) => {
                write_numbers::<2>(out, &[*value], b"Tw\n");
            }
            Op::SetCharSpacing(value) => {
                write_numbers::<2>(out, &[*value], b"Tc\n");
            }
            Op::SetHorizontalScaling(value) => {
                write_numbers::<2>(out, &[*value], b"Tz\n");
            }
            Op::SetLeading(value) => {
                write_numbers::<2>(out, &[*value], b"TL\n");
            }
            Op::SetTextRise(value) => {
                write_numbers::<2>(out, &[*value], b"Ts\n");
            }
            Op::SetRenderingMode(mode) => {
                writeln!(out, "{mode} Tr").expect("writing to Vec<u8> never fails");
            }

            // ── path painting (no-op) ──
            Op::EndPath => out.extend_from_slice(b"n\n"),

            // ── clipping ──
            Op::ClipNonZero => out.extend_from_slice(b"W\n"),
            Op::ClipEvenOdd => out.extend_from_slice(b"W*\n"),
            Op::ClipStroke => out.extend_from_slice(b"W S\n"),

            // ── shading ──
            Op::PaintShading(name) => {
                writeln!(out, "/{name} sh").expect("writing to Vec<u8> never fails");
            }

            // ── special ──
            Op::Comment(text) => {
                writeln!(out, "% {text}").expect("writing to Vec<u8> never fails");
            }
            Op::Raw(bytes) => out.extend_from_slice(bytes),
        }
    }
}

/// Convenience: serialise to `String` (used by the legacy `operations()`
/// public getter on contexts during the migration). Content streams are
/// always ASCII when produced by the IR — `from_utf8_unchecked` would be
/// safe but `from_utf8` keeps the contract auditable.
pub(crate) fn ops_to_string(ops: &[Op]) -> String {
    let mut buf = Vec::new();
    serialize_ops(&mut buf, ops);
    String::from_utf8(buf).expect("serialize_ops emits ASCII content-stream tokens")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rect_fill_roundtrip_emits_re_f() {
        let ops = vec![
            Op::Rect {
                x: 10.0,
                y: 20.0,
                w: 100.0,
                h: 50.0,
            },
            Op::FillNonZero,
        ];
        let mut out = Vec::new();
        serialize_ops(&mut out, &ops);
        assert_eq!(out, b"10.00 20.00 100.00 50.00 re\nf\n");
    }

    #[test]
    fn move_to_with_nan_components_sanitises_to_zero() {
        let ops = vec![Op::MoveTo {
            x: f64::NAN,
            y: f64::INFINITY,
        }];
        let mut out = Vec::new();
        serialize_ops(&mut out, &ops);
        assert_eq!(out, b"0.00 0.00 m\n");
    }

    #[test]
    fn raw_op_passes_bytes_through_unchanged() {
        let ops = vec![Op::Raw(b"/Gs1 gs\n".to_vec())];
        let mut out = Vec::new();
        serialize_ops(&mut out, &ops);
        assert_eq!(out, b"/Gs1 gs\n");
    }

    #[test]
    fn line_width_with_nan_clamps_to_zero() {
        let ops = vec![Op::SetLineWidth(f64::NAN)];
        assert_eq!(ops_to_string(&ops), "0.00 w\n");
    }

    #[test]
    fn cm_translate_with_neg_inf_clamps_to_zero() {
        let ops = vec![Op::Cm {
            a: 1.0,
            b: 0.0,
            c: 0.0,
            d: 1.0,
            e: f64::NEG_INFINITY,
            f: 50.0,
        }];
        assert_eq!(ops_to_string(&ops), "1.00 0.00 0.00 1.00 0.00 50.00 cm\n");
    }

    #[test]
    fn td_with_nan_clamps_to_zero() {
        let ops = vec![Op::SetTextPosition {
            x: f64::NAN,
            y: -1.0,
        }];
        assert_eq!(ops_to_string(&ops), "0.00 -1.00 Td\n");
    }

    #[test]
    fn show_text_wraps_with_parens_and_tj() {
        let ops = vec![Op::ShowText(b"Hello world".to_vec())];
        assert_eq!(ops_to_string(&ops), "(Hello world) Tj\n");
    }

    #[test]
    fn show_text_hex_wraps_with_angle_brackets_and_tj() {
        let ops = vec![Op::ShowTextHex(b"4E2D6587".to_vec())];
        assert_eq!(ops_to_string(&ops), "<4E2D6587> Tj\n");
    }

    #[test]
    fn show_text_array_interleaves_glyphs_and_adjustments_as_tj() {
        let ops = vec![Op::ShowTextArray(vec![
            TextArrayElement::Glyphs(b"00410042".to_vec()),
            TextArrayElement::Adjust(-50.0),
            TextArrayElement::Glyphs(b"0043".to_vec()),
        ])];
        assert_eq!(ops_to_string(&ops), "[ <00410042> -50.00 <0043> ] TJ\n");
    }

    #[test]
    fn show_text_array_single_glyph_run_has_no_adjustment() {
        let ops = vec![Op::ShowTextArray(vec![TextArrayElement::Glyphs(
            b"0041".to_vec(),
        )])];
        assert_eq!(ops_to_string(&ops), "[ <0041> ] TJ\n");
    }

    #[test]
    fn show_text_array_sanitises_non_finite_adjustment_to_zero() {
        let ops = vec![Op::ShowTextArray(vec![
            TextArrayElement::Glyphs(b"0041".to_vec()),
            TextArrayElement::Adjust(f32::NAN),
            TextArrayElement::Glyphs(b"0042".to_vec()),
        ])];
        assert_eq!(ops_to_string(&ops), "[ <0041> 0.00 <0042> ] TJ\n");
    }

    #[test]
    fn fill_color_components_pad_with_trailing_space_before_sc() {
        let ops = vec![Op::SetFillColorComponents(vec![0.1, 0.2, 0.3])];
        assert_eq!(ops_to_string(&ops), "0.1000 0.2000 0.3000 sc\n");
    }

    #[test]
    fn comment_emits_percent_prefix() {
        let ops = vec![Op::Comment("Begin Transparency Group".to_string())];
        assert_eq!(ops_to_string(&ops), "% Begin Transparency Group\n");
    }

    #[test]
    fn paint_shading_emits_name_sh() {
        // Issue #297 D: `/name sh` paints a registered shading (ISO 32000-1 §8.7.4.2).
        let ops = vec![Op::PaintShading("Grad1".to_string())];
        assert_eq!(ops_to_string(&ops), "/Grad1 sh\n");
    }

    #[test]
    fn clip_end_path_emits_w_then_n() {
        // ISO 32000-1 §8.5.4: a clip path is terminated by `W n`.
        let ops = vec![Op::ClipNonZero, Op::EndPath];
        assert_eq!(ops_to_string(&ops), "W\nn\n");
    }
}

#[cfg(test)]
mod numeric_performance_tests {
    use super::*;

    fn assert_legacy_bytes(value: f64) {
        let finite = if value.is_finite() { value } else { 0.0 };
        let mut actual = Vec::new();
        serialize_ops(
            &mut actual,
            &[
                Op::MoveTo {
                    x: value,
                    y: -value,
                },
                Op::LineTo { x: value, y: value },
                Op::CurveTo {
                    x1: value,
                    y1: value,
                    x2: value,
                    y2: value,
                    x3: value,
                    y3: value,
                },
                Op::Rect {
                    x: value,
                    y: value,
                    w: value,
                    h: value,
                },
                Op::Cm {
                    a: value,
                    b: value,
                    c: value,
                    d: value,
                    e: value,
                    f: value,
                },
                Op::SetTextPosition { x: value, y: value },
                Op::SetLineWidth(value),
                Op::SetMiterLimit(value),
                Op::SetFlatness(value),
                Op::SetWordSpacing(value),
                Op::SetCharSpacing(value),
                Op::SetHorizontalScaling(value),
                Op::SetLeading(value),
                Op::SetTextRise(value),
                Op::SetFont {
                    name: "F1".into(),
                    size: value,
                },
                Op::SetFillColorComponents(vec![value]),
                Op::SetStrokeColorComponents(vec![value]),
                Op::Stroke,
            ],
        );
        let opposite = if value.is_finite() { -value } else { 0.0 };
        let expected = format!(
            concat!(
                "{v:.2} {opposite:.2} m\n{v:.2} {v:.2} l\n",
                "{v:.2} {v:.2} {v:.2} {v:.2} {v:.2} {v:.2} c\n",
                "{v:.2} {v:.2} {v:.2} {v:.2} re\n",
                "{v:.2} {v:.2} {v:.2} {v:.2} {v:.2} {v:.2} cm\n",
                "{v:.2} {v:.2} Td\n{v:.2} w\n{v:.2} M\n{v:.2} i\n",
                "{v:.2} Tw\n{v:.2} Tc\n{v:.2} Tz\n{v:.2} TL\n{v:.2} Ts\n",
                "/F1 {v} Tf\n{v:.4} sc\n{v:.4} SC\nS\n"
            ),
            v = finite,
            opposite = opposite
        );
        assert_eq!(
            actual,
            expected.as_bytes(),
            "value bits: {:016x}",
            value.to_bits()
        );
    }

    #[test]
    fn numeric_operators_match_legacy_formatting_at_boundaries_and_random_bits() {
        for value in [
            0.,
            -0.,
            1.,
            -1.,
            9.,
            10.,
            99.,
            100.,
            1.005,
            -1.005,
            1.125,
            0.00005,
            -0.00005,
            f64::MIN_POSITIVE,
            f64::from_bits(1),
            f64::MAX,
            f64::MIN,
            f64::NAN,
            f64::INFINITY,
            f64::NEG_INFINITY,
            9_007_199_254_740_991.,
            9_007_199_254_740_992.,
            9_007_199_254_740_994.,
        ] {
            assert_legacy_bytes(value);
            assert_legacy_bytes(-value);
        }
        for value in -1000..=1000 {
            assert_legacy_bytes(value as f64);
        }
        let mut bits = 0x7037_0170_2700_u64;
        for _ in 0..4096 {
            bits ^= bits << 13;
            bits ^= bits >> 7;
            bits ^= bits << 17;
            assert_legacy_bytes(f64::from_bits(bits));
        }
    }
}
