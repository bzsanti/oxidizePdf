//! Repro for a flat-path spurious mid-token space regression introduced by
//! the `flat_space_gap_threshold` font-derived tightening (commit ce79992,
//! part of PR #499 / issue #496's follow-up hardening; released in 4.5.0).
//!
//! `flat_space_gap_threshold` replaced the old flat `0.3 * font_size` cutoff
//! with `0.5 * real_space_advance` (the font's actual `/Widths` entry for
//! code 32, in text-space units) whenever available. `real_space_advance` is
//! computed from the nominal `Tf` font size alone, but the inter-run gap
//! (`dx`) it is compared against is measured in *user space*, i.e. already
//! scaled by `Tm`/CTM. When a PDF generator bakes its real rendering scale
//! into `Tm` rather than `Tf` (drawing at `Tf 1` with e.g. `9 0 0 9 x y Tm`
//! -- a common technique, seen here on a real Brazilian financial-prospectus
//! template), the two sides of the comparison are off by that scale factor:
//! the threshold ends up far smaller, relative to `dx`, than the old flat
//! threshold was. A hyphenated phone number like "3030-7160" split across
//! separate `Tj` runs (or even separate `BT`/`ET` text objects) with a
//! perfectly ordinary few-hundredths-of-a-unit forward positioning residue
//! between runs -- well under the old threshold -- now crosses the new one
//! and gets a spurious space inserted, corrupting "3030-7160" into
//! "3030- 7160".
mod common;

use common::pdf_assembler::{assemble_pdf, stream_obj};
use oxidize_pdf::parser::PdfReader;
use oxidize_pdf::text::TextExtractor;
use std::io::Cursor;

/// A simple WinAnsi TrueType font, `FirstChar 32`, real Verdana-ish widths:
/// space (32) = 278, `-` (45) = 333, digits 0-9 (48-57) = 556 each --
/// approximate real Verdana metrics.
fn widths_array() -> String {
    // widths[i] corresponds to code (32+i), for codes 32..=64 inclusive.
    let mut widths = vec![500u32; 64 - 32 + 1];
    widths[0] = 278; // space (32)
    widths[45 - 32] = 333; // '-'
    for digit_code in 48..=57 {
        widths[digit_code - 32] = 556; // '0'..'9'
    }
    format!(
        "[{}]",
        widths
            .iter()
            .map(|w| w.to_string())
            .collect::<Vec<_>>()
            .join(" ")
    )
}

fn build_pdf() -> Vec<u8> {
    // font_size = 1, with the real 9pt rendering scale baked into `Tm`
    // (a=d=9). An absolute `Tm` before each run precisely controls each
    // run's user-space start x, avoiding any ambiguity from `Td`'s
    // line-matrix-relative (not pen-relative) semantics.
    //
    // Natural advances at font_size 1, Tm x-scale 9 (from `widths_array`):
    // "3030" = 4*556/1000*9 = 20.016 user-space units; "-" = 333/1000*9 =
    // 2.997. The "-" run starts 0.05 units *short* of the natural pen
    // position after "3030" (an ordinary small residue, either sign is
    // harmless), and "7160" starts 0.15 units *past* the natural pen
    // position after "-".
    //
    // The threshold is computed at the nominal font_size (1), unscaled by
    // Tm: `0.5 * 278/1000 * 1 = 0.139` (new, font-derived) vs.
    // `0.3 * font_size(1) = 0.3` (old, flat). `dx` is measured in
    // Tm-scaled user-space units, so 0.15 clears the new threshold but not
    // the old one -- the mismatch between an unscaled threshold and a
    // Tm-scaled `dx` is exactly the bug.
    let content = b"BT\n/F1 1 Tf\n9 0 0 9 100 700 Tm\n(3030)Tj\n\
9 0 0 9 119.966 700 Tm\n(-)Tj\n\
9 0 0 9 123.113 700 Tm\n(7160)Tj\nET\n";
    let widths = widths_array();
    let objects: Vec<Vec<u8>> = vec![
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /Resources << /Font << /F1 5 0 R >> >> \
          /Contents 4 0 R /MediaBox [0 0 595 842] >>"
            .to_vec(),
        stream_obj("", content),
        format!(
            "<< /Type /Font /Subtype /TrueType /BaseFont /Verdana \
             /Encoding /WinAnsiEncoding /FirstChar 32 /LastChar 64 \
             /Widths {widths} /FontDescriptor 6 0 R >>"
        )
        .into_bytes(),
        b"<< /Type /FontDescriptor /FontName /Verdana /Flags 32 \
          /FontBBox [0 -200 1000 900] /ItalicAngle 0 /Ascent 900 /Descent -200 \
          /CapHeight 700 /StemV 80 >>"
            .to_vec(),
    ];
    assemble_pdf(&objects)
}

#[test]
fn small_forward_residue_between_simple_font_runs_at_font_size_1_is_not_a_space() {
    let pdf = build_pdf();
    let doc = PdfReader::new(Cursor::new(pdf))
        .expect("PDF should parse")
        .into_document();
    let text = TextExtractor::new()
        .extract_from_page(&doc, 0)
        .expect("extraction should succeed")
        .text;
    assert_eq!(
        text, "3030-7160",
        "small positive Td residue between simple-font runs at font_size=1 \
         (well below the old flat 0.3*font_size threshold) must not be \
         promoted to a word space by the font-derived threshold, got: {text:?}"
    );
}
