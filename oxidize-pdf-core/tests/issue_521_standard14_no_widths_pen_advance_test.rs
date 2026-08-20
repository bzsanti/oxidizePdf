//! Repro for issue #521: a standard-14 simple font (e.g. bare `/Helvetica`)
//! that ships no `/Widths` array -- valid per spec, ISO 32000-1 §9.6.2.2 --
//! made `calculate_text_width`/`calculate_text_width_from_codes` fall back to
//! a flat `len * font_size * 0.5` pen-advance guess, completely disconnected
//! from the font's real, highly variable per-glyph AFM widths (e.g. Helvetica
//! "i" = 222 vs "m" = 833 per 1000 em). When a token is split across
//! multiple `Tj` runs (common generator output: per-character coloring,
//! kerning-pair runs, redaction overlays), the residual gap between the
//! wrong pen position and the next run's declared `Tm`/`Td` position is
//! essentially arbitrary and can spuriously cross `flat_space_gap_threshold`,
//! inserting a space in the middle of a token.
//!
//! Distinct from issue #510/#511 (a threshold-scaling bug): here the pen
//! advance itself (`dx`) is wrong, so no amount of threshold scaling can fix
//! the comparison -- `font_space_advance` already had a `standard_14_space_width`
//! fallback for computing the *threshold*, but the *pen-advance* functions had
//! no equivalent standard-14 AFM fallback at all.
mod common;

use common::pdf_assembler::{assemble_pdf, stream_obj};
use oxidize_pdf::parser::PdfReader;
use oxidize_pdf::text::TextExtractor;
use std::io::Cursor;

/// A bare standard-14 Helvetica font: no `/Widths`, no `/FirstChar`/`/LastChar`
/// -- exactly the form seen in the real-world PDF this issue was found on
/// (a DocuSign certificate-of-completion page).
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

fn extract(content: &[u8]) -> String {
    let pdf = build_pdf_with_content(content);
    let doc = PdfReader::new(Cursor::new(pdf))
        .expect("PDF should parse")
        .into_document();
    TextExtractor::new()
        .extract_from_page(&doc, 0)
        .expect("extraction should succeed")
        .text
}

#[test]
fn no_widths_standard14_font_does_not_split_a_token_across_tj_runs() {
    // Reproduces the exact real-world content-stream pattern (byte-for-byte
    // structurally): `Tf 1` with the real 7.2pt rendering scale baked into
    // `Tm`, an email address split across five `Tj` runs by per-run `Tc`
    // (character-spacing) changes -- a common per-character-color/kerning
    // generator artifact.
    let text = extract(
        b"BT\n/F1 1 Tf\n7.2 0 0 7.2 100 700 Tm\n\
0.0269 Tc 1.41 0 Td\n(suelen.mat)Tj\n\
0 Tc 4.547 0 Td\n(s)Tj\n\
0.0398 Tc 0.361 0 Td\n(udo)Tj\n\
0 Tc 1.596 0 Td\n(@)Tj\n\
0.0285 Tc 0.958 0 Td\n(xpi.com.br )Tj\nET\n",
    );
    assert_eq!(
        text.trim(),
        "suelen.matsudo@xpi.com.br",
        "a standard-14 font with no /Widths array must use its real per-glyph \
         AFM metrics for the pen advance, not a flat len*font_size*0.5 guess, \
         got: {text:?}"
    );
}

#[test]
fn no_widths_standard14_font_still_inserts_a_real_word_space() {
    // Guard against overcorrecting: a genuine word gap between two standard-14
    // words (no /Widths) must still become a space, not get welded together.
    let text = extract(
        b"BT\n/F1 1 Tf\n7.2 0 0 7.2 100 700 Tm\n\
(Hello)Tj\n\
7.2 0 0 7.2 130 700 Tm\n(World)Tj\nET\n",
    );
    assert_eq!(text.trim(), "Hello World");
}

#[test]
fn bold_variant_uses_its_own_metrics_not_the_regular_weight() {
    // Helvetica-Bold "m" = 889/1000 em, wider than regular Helvetica's
    // 722/1000; using the wrong table would reintroduce the same class of
    // pen-advance drift this issue is about. At Tf 1 / Tm x-scale 7.2, ten
    // "m"s advance 10 * 889/1000 * 7.2 = 64.008 user-space units, so the
    // second run's Tm x (100 + 64.008 = 164.008) only welds onto the first
    // with no gap if the bold table (not regular Helvetica's) was used.
    let objects: Vec<Vec<u8>> = vec![
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /Resources << /Font << /F1 5 0 R >> >> \
          /Contents 4 0 R /MediaBox [0 0 595 842] >>"
            .to_vec(),
        stream_obj(
            "",
            b"BT\n/F1 1 Tf\n7.2 0 0 7.2 100 700 Tm\n\
(mmmmmmmmmm)Tj\n\
7.2 0 0 7.2 164.008 700 Tm\n(mmmmmmmmmm)Tj\nET\n",
        ),
        b"<< /BaseFont /Helvetica-Bold /Encoding /WinAnsiEncoding /Subtype /Type1 /Type /Font >>"
            .to_vec(),
    ];
    let pdf = assemble_pdf(&objects);
    let doc = PdfReader::new(Cursor::new(pdf))
        .expect("PDF should parse")
        .into_document();
    let text = TextExtractor::new()
        .extract_from_page(&doc, 0)
        .expect("extraction should succeed")
        .text;
    assert_eq!(text.trim(), "mmmmmmmmmmmmmmmmmmmm");
}
