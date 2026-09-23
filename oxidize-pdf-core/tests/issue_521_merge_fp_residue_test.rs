//! Regression test for a follow-up found in review of the fix for issue
//! #521 (encoding-aware Standard-14 AFM widths, #523/#524).
//!
//! With real AFM-based pen advances, a run's end position and the next
//! run's declared start position (its own `Tm`-derived origin) are computed
//! through independent floating-point paths that are mathematically
//! identical for a truly contiguous pair of runs, but not bit-identical --
//! the difference can land a few ULPs on either side of zero.
//! `merge_close_fragments` treated two same-line fragments as adjacent only
//! when `x_gap >= 0.0`, so a `-1e-13`-class negative residue on an otherwise
//! touching pair rejected the merge and left the pair as two separate
//! `TextFragment`s instead of one.

mod common;

use common::pdf_assembler::{assemble_pdf, stream_obj};
use oxidize_pdf::parser::PdfReader;
use oxidize_pdf::text::{ExtractionOptions, TextExtractor};
use std::io::Cursor;

/// A bare standard-14 Helvetica font: no `/Widths`, no `/FirstChar`/`/LastChar`
/// -- valid per ISO 32000-1 §9.6.2.2 -- so the pen advance comes from the
/// AFM-by-encoding resolver (#523), which is where the floating-point
/// residue this test guards against originates.
fn build_pdf_with_content(content: &[u8]) -> Vec<u8> {
    let objects: Vec<Vec<u8>> = vec![
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /Resources << /Font << /F1 5 0 R >> >> \
          /Contents 4 0 R /MediaBox [0 0 595 842] >>"
            .to_vec(),
        stream_obj("", content),
        b"<< /BaseFont /Helvetica /Encoding /WinAnsiEncoding /Subtype /Type1 /Type /Font >>"
            .to_vec(),
    ];
    assemble_pdf(&objects)
}

#[test]
fn contiguous_afm_positioned_runs_merge_despite_floating_point_residue() {
    // Two bare `Tj` runs back-to-back with no repositioning `Tm`/`Td`
    // between them: the second run starts exactly where the extractor's own
    // pen advance placed the first run's end. Any gap between them is pure
    // floating-point noise, not a real one, and must not produce either a
    // spurious space or an extra fragment. Sixteen narrow "i" glyphs
    // (Helvetica AFM width 222/1000 em) at a 1:1 text-space scale is a
    // minimal reproduction of the residue: the extractor's pen-advance
    // accumulation (`advance_pen`, one multiply per `Tj`) and the
    // AFM-width-sum-based merge-gap check (`calculate_text_width_from_codes`,
    // one multiply-and-sum) compute the same nominal position through
    // different floating-point operation sequences.
    let pdf = build_pdf_with_content(
        b"BT\n/F1 12 Tf\n14 TL\n1 0 0 1 100 700 Tm\n\
(iiiiiiiiiiiiiiii)Tj\n\
(X)Tj\nET\n",
    );
    let doc = PdfReader::new(Cursor::new(pdf))
        .expect("PDF should parse")
        .into_document();
    let extracted = TextExtractor::with_options(ExtractionOptions {
        preserve_layout: true,
        ..Default::default()
    })
    .extract_from_page(&doc, 0)
    .expect("extraction should succeed");

    assert_eq!(
        extracted.fragments.len(),
        1,
        "the two contiguous AFM-positioned runs must merge into a single \
         fragment, not be split apart by floating-point residue in the \
         merge gap check; got {} fragments: {:?}",
        extracted.fragments.len(),
        extracted
            .fragments
            .iter()
            .map(|f| &f.text)
            .collect::<Vec<_>>()
    );
    assert_eq!(
        extracted.fragments[0].text, "iiiiiiiiiiiiiiiiX",
        "a genuinely touching pair (no positioning gap) must not gain a \
         spurious space either; got {:?}",
        extracted.fragments[0].text
    );
}
