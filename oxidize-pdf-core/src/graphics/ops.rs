//! Typed intermediate representation for PDF content-stream operators.
//!
//! Both `GraphicsContext` and `TextContext` accumulate `Op` values instead
//! of pre-formatted byte strings. `Page` orders ops in call order across
//! contexts so the painter model is preserved (issue #227). All non-finite
//! `f64` inputs are sanitised at serialisation time via `finite_or_zero`,
//! mirroring the colour-emission fix in 2.6.0 (issues #220, #221).
//!
//! The `dead_code` allowance below is temporary: Phase 0 of the v2.7.0
//! refactor introduces this module as scaffolding. Phases 1–5 wire it
//! into the existing contexts and the allowance can then be removed.

#![cfg_attr(not(test), allow(dead_code))]

use super::color::finite_or_zero;
use std::io::Write;

/// PDF content-stream operators as typed values.
///
/// Variants are added incrementally as each context is migrated through
/// the v2.7.0 refactor. Unmodelled operators may be emitted via `Op::Raw`,
/// which writes its bytes verbatim to the output stream — this lets
/// emitters that have not yet been migrated (forms, annotations, writer)
/// participate in the unified content stream without a full rewrite.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Op {
    /// `x y m` — begin a new subpath at the given point.
    MoveTo { x: f64, y: f64 },
    /// `x y w h re` — append a rectangle to the current path.
    Rect { x: f64, y: f64, w: f64, h: f64 },
    /// `f` — fill the current path using the non-zero winding rule.
    FillNonZero,
    /// Bytes emitted verbatim. Use only for operators not yet modelled
    /// or for content sourced from external/preserved streams.
    Raw(Vec<u8>),
}

/// Serialises a slice of `Op` values to a byte buffer in PDF
/// content-stream syntax. Non-finite floats are clamped to `0.0` via
/// `finite_or_zero` at the emission boundary.
pub(crate) fn serialize_ops(out: &mut Vec<u8>, ops: &[Op]) {
    for op in ops {
        match op {
            Op::MoveTo { x, y } => {
                let x = finite_or_zero(*x);
                let y = finite_or_zero(*y);
                writeln!(out, "{x:.2} {y:.2} m").expect("writing to Vec<u8> never fails");
            }
            Op::Rect { x, y, w, h } => {
                let x = finite_or_zero(*x);
                let y = finite_or_zero(*y);
                let w = finite_or_zero(*w);
                let h = finite_or_zero(*h);
                writeln!(out, "{x:.2} {y:.2} {w:.2} {h:.2} re")
                    .expect("writing to Vec<u8> never fails");
            }
            Op::FillNonZero => out.extend_from_slice(b"f\n"),
            Op::Raw(bytes) => out.extend_from_slice(bytes),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{serialize_ops, Op};

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
}
