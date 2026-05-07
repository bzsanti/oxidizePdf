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

    // ── clipping ──
    /// `W` — modify current clipping path using the non-zero winding rule.
    ClipNonZero,
    /// `W*` — modify current clipping path using the even-odd rule.
    ClipEvenOdd,
    /// `W S` — clip then stroke. Used by the `clip_stroke` builder for
    /// the common pattern of stroking the boundary of the clip region.
    ClipStroke,

    // ── special ──
    /// `% comment` — a PDF comment line. Used for transparency-group
    /// markers; ignored by viewers but useful for diff/debug.
    Comment(String),
    /// Bytes emitted verbatim. Use for operators not yet modelled or for
    /// content sourced from external/preserved streams.
    Raw(Vec<u8>),
}

/// Serialises a slice of `Op` values to a byte buffer in PDF
/// content-stream syntax. Non-finite floats are clamped to `0.0` via
/// `finite_or_zero` at the emission boundary.
pub(crate) fn serialize_ops(out: &mut Vec<u8>, ops: &[Op]) {
    for op in ops {
        match op {
            // ── path construction ──
            Op::MoveTo { x, y } => {
                let x = finite_or_zero(*x);
                let y = finite_or_zero(*y);
                writeln!(out, "{x:.2} {y:.2} m").expect("writing to Vec<u8> never fails");
            }
            Op::LineTo { x, y } => {
                let x = finite_or_zero(*x);
                let y = finite_or_zero(*y);
                writeln!(out, "{x:.2} {y:.2} l").expect("writing to Vec<u8> never fails");
            }
            Op::CurveTo {
                x1,
                y1,
                x2,
                y2,
                x3,
                y3,
            } => {
                let x1 = finite_or_zero(*x1);
                let y1 = finite_or_zero(*y1);
                let x2 = finite_or_zero(*x2);
                let y2 = finite_or_zero(*y2);
                let x3 = finite_or_zero(*x3);
                let y3 = finite_or_zero(*y3);
                writeln!(out, "{x1:.2} {y1:.2} {x2:.2} {y2:.2} {x3:.2} {y3:.2} c")
                    .expect("writing to Vec<u8> never fails");
            }
            Op::Rect { x, y, w, h } => {
                let x = finite_or_zero(*x);
                let y = finite_or_zero(*y);
                let w = finite_or_zero(*w);
                let h = finite_or_zero(*h);
                writeln!(out, "{x:.2} {y:.2} {w:.2} {h:.2} re")
                    .expect("writing to Vec<u8> never fails");
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
                for v in values {
                    let v = finite_or_zero(*v);
                    write!(out, "{v:.4} ").expect("writing to Vec<u8> never fails");
                }
                out.extend_from_slice(b"sc\n");
            }
            Op::SetStrokeColorComponents(values) => {
                for v in values {
                    let v = finite_or_zero(*v);
                    write!(out, "{v:.4} ").expect("writing to Vec<u8> never fails");
                }
                out.extend_from_slice(b"SC\n");
            }

            // ── line / dash ──
            Op::SetLineWidth(width) => {
                let w = finite_or_zero(*width);
                writeln!(out, "{w:.2} w").expect("writing to Vec<u8> never fails");
            }
            Op::SetLineCap(cap) => {
                writeln!(out, "{cap} J").expect("writing to Vec<u8> never fails");
            }
            Op::SetLineJoin(join) => {
                writeln!(out, "{join} j").expect("writing to Vec<u8> never fails");
            }
            Op::SetMiterLimit(limit) => {
                let l = finite_or_zero(*limit);
                writeln!(out, "{l:.2} M").expect("writing to Vec<u8> never fails");
            }
            Op::SetDashPatternRaw(s) => {
                writeln!(out, "{s} d").expect("writing to Vec<u8> never fails");
            }
            Op::SetFlatness(value) => {
                let v = finite_or_zero(*value);
                writeln!(out, "{v:.2} i").expect("writing to Vec<u8> never fails");
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
                let a = finite_or_zero(*a);
                let b = finite_or_zero(*b);
                let c = finite_or_zero(*c);
                let d = finite_or_zero(*d);
                let e = finite_or_zero(*e);
                let f = finite_or_zero(*f);
                writeln!(out, "{a:.2} {b:.2} {c:.2} {d:.2} {e:.2} {f:.2} cm")
                    .expect("writing to Vec<u8> never fails");
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
                writeln!(out, "/{name} {size} Tf").expect("writing to Vec<u8> never fails");
            }
            Op::SetTextPosition { x, y } => {
                let x = finite_or_zero(*x);
                let y = finite_or_zero(*y);
                writeln!(out, "{x:.2} {y:.2} Td").expect("writing to Vec<u8> never fails");
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
            Op::SetWordSpacing(value) => {
                let v = finite_or_zero(*value);
                writeln!(out, "{v:.2} Tw").expect("writing to Vec<u8> never fails");
            }
            Op::SetCharSpacing(value) => {
                let v = finite_or_zero(*value);
                writeln!(out, "{v:.2} Tc").expect("writing to Vec<u8> never fails");
            }
            Op::SetHorizontalScaling(value) => {
                let v = finite_or_zero(*value);
                writeln!(out, "{v:.2} Tz").expect("writing to Vec<u8> never fails");
            }
            Op::SetLeading(value) => {
                let v = finite_or_zero(*value);
                writeln!(out, "{v:.2} TL").expect("writing to Vec<u8> never fails");
            }
            Op::SetTextRise(value) => {
                let v = finite_or_zero(*value);
                writeln!(out, "{v:.2} Ts").expect("writing to Vec<u8> never fails");
            }
            Op::SetRenderingMode(mode) => {
                writeln!(out, "{mode} Tr").expect("writing to Vec<u8> never fails");
            }

            // ── clipping ──
            Op::ClipNonZero => out.extend_from_slice(b"W\n"),
            Op::ClipEvenOdd => out.extend_from_slice(b"W*\n"),
            Op::ClipStroke => out.extend_from_slice(b"W S\n"),

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
    fn fill_color_components_pad_with_trailing_space_before_sc() {
        let ops = vec![Op::SetFillColorComponents(vec![0.1, 0.2, 0.3])];
        assert_eq!(ops_to_string(&ops), "0.1000 0.2000 0.3000 sc\n");
    }

    #[test]
    fn comment_emits_percent_prefix() {
        let ops = vec![Op::Comment("Begin Transparency Group".to_string())];
        assert_eq!(ops_to_string(&ops), "% Begin Transparency Group\n");
    }
}
