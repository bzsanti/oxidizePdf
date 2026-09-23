//! Issue #417 — `detect_and_sort_columns` performs its own line-grouping pass
//! (run when `reorder_columns=true`) and still used the pre-#408 logic:
//! `(last_y - fragment.y).abs() > newline_threshold`, i.e. distance to the
//! *previous* fragment against a fixed 10pt band. On dense prose with tight
//! (sub-`newline_threshold`) leading, that accumulates drift and merges nearly
//! the whole page into one pseudo-line; an incidental gap wider than
//! `column_threshold` then flags it "columnar" and the merged line is reshuffled
//! by X, shredding tokens.
//!
//! The fix anchors line grouping to the line head with a font-relative tolerance
//! (`min(head, frag).height * 0.2`), matching the #408 fix in
//! `sort_and_merge_fragments`. No smoke tests: we assert real identifiers stay
//! contiguous in the reordered output.

use oxidize_pdf::parser::{ParseOptions, PdfReader};
use oxidize_pdf::text::{ExtractionOptions, TextExtractor};
use std::io::Write;

fn escape(ch: char) -> String {
    match ch {
        '(' => "\\(".to_string(),
        ')' => "\\)".to_string(),
        '\\' => "\\\\".to_string(),
        _ => ch.to_string(),
    }
}

/// Emit one glyph per `Tj` (CID-font granularity). A `|` inserts a 65pt jump
/// (above the default `column_threshold` of 50) instead of a glyph — an
/// incidental wide gap (quoted aside, wide parenthetical, kerning artifact),
/// not a real table column.
fn emit_line(content: &mut Vec<u8>, text: &str, start_x: f64, y: f64, advance: f64) {
    let mut x = start_x;
    for ch in text.chars() {
        if ch == '|' {
            x += 65.0;
            continue;
        }
        let escaped = escape(ch);
        content.extend_from_slice(
            format!("BT\n/F1 10 Tf\n{x:.2} {y:.2} Td\n({escaped}) Tj\nET\n").as_bytes(),
        );
        x += advance;
    }
}

/// Dense prose at 8pt leading (below the 10pt `newline_threshold`) plus a small
/// unrelated table, so a columnar block exists for the reorder pass to act on.
fn build_pdf() -> Vec<u8> {
    let mut content = Vec::new();
    let lines = [
        "Line one contains|identifier AA-11112222-33 in the middle of it.",
        "Line two continues|with unrelated filler text for demonstration.",
        "Line three has|another identifier BB-44445555-66 embedded here.",
        "Line four is|more filler content padding out the paragraph body.",
        "Line five contains|a final identifier CC-77778888-99 to check.",
        "Line six closes out|the paragraph with some trailing filler text.",
    ];
    let mut y = 700.0;
    for line in &lines {
        emit_line(&mut content, line, 72.0, y, 6.0);
        y -= 8.0;
    }
    emit_line(&mut content, "Col1", 72.0, 500.0, 6.0);
    emit_line(&mut content, "Col2", 250.0, 500.0, 6.0);
    emit_line(&mut content, "Col3", 420.0, 500.0, 6.0);

    let mut pdf = Vec::new();
    pdf.extend_from_slice(b"%PDF-1.4\n");
    let mut offsets = Vec::new();

    offsets.push(pdf.len());
    pdf.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");
    offsets.push(pdf.len());
    pdf.extend_from_slice(b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n");
    offsets.push(pdf.len());
    pdf.extend_from_slice(b"3 0 obj\n<< /Type /Page /Parent 2 0 R /Resources << /Font << /F1 5 0 R >> >> /MediaBox [0 0 612 792] /Contents 4 0 R >>\nendobj\n");
    offsets.push(pdf.len());
    let mut obj4 = Vec::new();
    write!(obj4, "4 0 obj\n<< /Length {} >>\nstream\n", content.len()).unwrap();
    obj4.extend_from_slice(&content);
    obj4.extend_from_slice(b"\nendstream\nendobj\n");
    pdf.extend_from_slice(&obj4);
    offsets.push(pdf.len());
    // Courier's fixed 600-unit width matches the synthetic 6pt glyph advance
    // at 10pt font size.
    pdf.extend_from_slice(
        b"5 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Courier >>\nendobj\n",
    );

    let xref_offset = pdf.len();
    pdf.extend_from_slice(b"xref\n");
    writeln!(pdf, "0 {}", offsets.len() + 1).unwrap();
    pdf.extend_from_slice(b"0000000000 65535 f \n");
    for off in &offsets {
        writeln!(pdf, "{:010} 00000 n ", off).unwrap();
    }
    write!(
        pdf,
        "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{}\n%%EOF",
        offsets.len() + 1,
        xref_offset
    )
    .unwrap();
    pdf
}

fn extract(opts: ExtractionOptions) -> String {
    let doc =
        PdfReader::new_with_options(std::io::Cursor::new(build_pdf()), ParseOptions::lenient())
            .expect("PDF should parse")
            .into_document();
    TextExtractor::with_options(opts)
        .extract_from_page(&doc, 0)
        .expect("extraction should succeed")
        .text
}

const IDS: [&str; 3] = ["AA-11112222-33", "BB-44445555-66", "CC-77778888-99"];

#[test]
fn reorder_columns_keeps_identifiers_intact_on_tight_leading() {
    let text = extract(ExtractionOptions {
        reorder_columns: true,
        ..Default::default()
    });
    for id in IDS {
        assert!(
            text.contains(id),
            "reorder_columns shredded `{id}` — tight-leading prose was merged into \
             one pseudo-line and reshuffled by X. Got: {text:?}"
        );
    }
}

#[test]
fn flat_extraction_keeps_identifiers_intact() {
    // Baseline / contrast: the default flat path was never affected.
    let text = extract(ExtractionOptions::default());
    for id in IDS {
        assert!(
            text.contains(id),
            "flat extraction must keep `{id}` intact. Got: {text:?}"
        );
    }
}

/// Builds a genuine two-column layout with the given row pitch. Each row holds a
/// left cell and a right cell separated by a wide (column-sized) gap.
fn build_two_column_pdf(row_pitch: f64) -> Vec<u8> {
    let mut content = Vec::new();
    let rows = [("Alpha", "Beta"), ("Gamma", "Delta"), ("Epsilon", "Zeta")];
    let mut y = 700.0;
    for (left, right) in rows {
        emit_line(&mut content, left, 72.0, y, 6.0);
        emit_line(&mut content, right, 300.0, y, 6.0);
        y -= row_pitch;
    }

    let mut pdf = Vec::new();
    pdf.extend_from_slice(b"%PDF-1.4\n");
    let mut offsets = Vec::new();
    offsets.push(pdf.len());
    pdf.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");
    offsets.push(pdf.len());
    pdf.extend_from_slice(b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n");
    offsets.push(pdf.len());
    pdf.extend_from_slice(b"3 0 obj\n<< /Type /Page /Parent 2 0 R /Resources << /Font << /F1 5 0 R >> >> /MediaBox [0 0 612 792] /Contents 4 0 R >>\nendobj\n");
    offsets.push(pdf.len());
    let mut obj4 = Vec::new();
    write!(obj4, "4 0 obj\n<< /Length {} >>\nstream\n", content.len()).unwrap();
    obj4.extend_from_slice(&content);
    obj4.extend_from_slice(b"\nendstream\nendobj\n");
    pdf.extend_from_slice(&obj4);
    offsets.push(pdf.len());
    // Courier's fixed 600-unit width matches the synthetic 6pt glyph advance
    // at 10pt font size.
    pdf.extend_from_slice(
        b"5 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Courier >>\nendobj\n",
    );
    let xref_offset = pdf.len();
    pdf.extend_from_slice(b"xref\n");
    writeln!(pdf, "0 {}", offsets.len() + 1).unwrap();
    pdf.extend_from_slice(b"0000000000 65535 f \n");
    for off in &offsets {
        writeln!(pdf, "{:010} 00000 n ", off).unwrap();
    }
    write!(
        pdf,
        "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{}\n%%EOF",
        offsets.len() + 1,
        xref_offset
    )
    .unwrap();
    pdf
}

fn extract_two_column(row_pitch: f64) -> String {
    let doc = PdfReader::new_with_options(
        std::io::Cursor::new(build_two_column_pdf(row_pitch)),
        ParseOptions::lenient(),
    )
    .expect("PDF should parse")
    .into_document();
    TextExtractor::with_options(ExtractionOptions {
        reorder_columns: true,
        ..Default::default()
    })
    .extract_from_page(&doc, 0)
    .expect("extraction should succeed")
    .text
}

/// Normal-pitch (rows ≥ one line height apart, 10pt font / 14pt pitch) real
/// columns still reflow **column-major**: the whole left column, then the whole
/// right column.
#[test]
fn normal_pitch_two_columns_reflow_column_major() {
    let text = extract_two_column(14.0);
    let left_last = text.find("Epsilon").expect("left column present");
    let right_first = text.find("Beta").expect("right column present");
    assert!(
        left_last < right_first,
        "normal-pitch columns must reflow column-major (all left cells before any \
         right cell). Got: {text:?}"
    );
}

/// Documented, accepted limitation (issue #417): a real two-column layout whose
/// rows are pitched **tighter than one line height** (here 8pt pitch under a
/// 10pt font) is geometrically indistinguishable from tight-leading prose with
/// an incidental gap. To avoid shredding prose, `reorder_columns` deliberately
/// declines to reflow such blocks and leaves them in reading (row-major) order.
/// This pins that trade-off so it stays intentional; text is never corrupted,
/// only left un-reordered.
#[test]
fn tight_pitch_two_columns_stay_in_reading_order_accepted_limitation() {
    let text = extract_two_column(8.0);
    let beta = text.find("Beta").expect("Beta present");
    let gamma = text.find("Gamma").expect("Gamma present");
    assert!(
        beta < gamma,
        "tight-pitch columns are intentionally left row-major (Beta, the row-1 \
         right cell, before Gamma, the row-2 left cell). Got: {text:?}"
    );
}
