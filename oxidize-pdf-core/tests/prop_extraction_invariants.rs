//! Core invariants of text extraction, stated from the *contract* of "extract
//! the text that was drawn" — NOT reverse-engineered from any single bug report.
//! Each property is checked against hundreds of randomized layouts, so the whole
//! class it describes is guarded before the first report, and every future edit
//! to the extraction pipeline is re-checked against it.
//!
//! Invariants:
//!   1. CONSERVATION — every glyph drawn on the page appears in the extracted
//!      text. Extraction may add separators (spaces/newlines) but must never
//!      silently drop a character. Guards the "returns empty / drops content /
//!      renders '?'" class (#330, #392, #415).
//!   2. REORDER IS A PERMUTATION — `reorder_columns` rearranges glyphs; it must
//!      never add, drop, or mutate one. The non-whitespace character multiset is
//!      identical with reordering off and on.
//!   3. TOKEN CONTIGUITY UNDER REORDER — a token contiguous in reading order
//!      stays contiguous after reordering. Guards the column-shredding family
//!      (#389, #403, #408, #417, #422, #425). (Distinct from #2: scattering a
//!      token preserves the character multiset but breaks contiguity.)
//!   4. DETERMINISM — extracting the same bytes twice yields identical text.
//!   5. LINE STRUCTURE — the newline structure of the flat extraction matches
//!      the lines actually drawn: glyphs drawn on one baseline stay on one
//!      output line (no spurious newline, #441), and glyphs drawn on distinct
//!      baselines end up on distinct output lines whenever the transition is
//!      geometrically detectable (no missing newline, #390). Unlike #1-#4,
//!      this oracle does NOT filter whitespace away — it is the only invariant
//!      that can see separator bugs, the class where #438 and #441 escaped.
//!
//!      SCOPE — read before trusting this as a class guard. This is NOT a
//!      "separator class is closed" invariant; it is a regression guard for the
//!      line shapes the generator KNOWS HOW TO DRAW. Its oracle is co-designed
//!      with the generator (`emit_line_structure` both draws the page and
//!      declares the expected lines), so it is structurally blind to any shape
//!      the generator does not emit. Concretely: every wrap it emits drops the
//!      pen to a LOWER baseline (dy > 0), and every dy == 0 backward jump it
//!      emits is a mid-line reposition — so a real wrap whose two lines share
//!      the SAME content-stream Y (dy == 0) is outside its input space entirely.
//!      That exact shape is issue #447, and it escaped PAST this invariant to a
//!      user. Recovering producer intent (which glyphs form a logical line)
//!      from local pen deltas is an underdetermined inverse problem; no
//!      synthetic oracle built from our own model can certify it. The durable
//!      guard for that class is measurement against real documents with an
//!      independent oracle (differential extraction / provisioned ground truth /
//!      the document's own tagged structure), not another generated property.
//!      Treat #447's fix as pinned by `issue_447_same_y_wrap_test`, not here.
//!   6. LINE STRUCTURE UNDER ROTATION — the same oracle under a rotated CTM:
//!      a rigid page rotation must not change the extracted line structure.
//!      PINNED `#[ignore]` to open issue #443 (separator heuristics measure
//!      post-CTM user space); flips to a permanent guard when it is fixed.

use oxidize_pdf::parser::{ParseOptions, PdfReader};
use oxidize_pdf::text::{ExtractionOptions, TextExtractor};
use proptest::prelude::*;
use std::collections::BTreeMap;
use std::io::{Cursor, Write};

const FONT_SIZE: f64 = 9.0;
// Courier is monospaced at 600 units, so this matches the font's real advance.
const GLYPH_ADVANCE: f64 = FONT_SIZE * 0.6;
const LEADING: f64 = 13.0;

const WORDS: [&str; 10] = [
    "lorem", "ipsum", "dolor", "amet", "sed", "tempor", "labore", "aliqua", "quis", "nostrud",
];

fn escape(ch: char) -> String {
    match ch {
        '(' => "\\(".to_string(),
        ')' => "\\)".to_string(),
        '\\' => "\\\\".to_string(),
        _ => ch.to_string(),
    }
}

fn emit_glyphs(content: &mut Vec<u8>, text: &str, start_x: f64, y: f64) -> f64 {
    let mut x = start_x;
    for ch in text.chars() {
        let e = escape(ch);
        content.extend_from_slice(
            format!("BT\n/F1 {FONT_SIZE} Tf\n{x:.2} {y:.2} Td\n({e}) Tj\nET\n").as_bytes(),
        );
        x += GLYPH_ADVANCE;
    }
    x
}

fn wrap_pdf(content: &[u8]) -> Vec<u8> {
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
    obj4.extend_from_slice(content);
    obj4.extend_from_slice(b"\nendstream\nendobj\n");
    pdf.extend_from_slice(&obj4);
    offsets.push(pdf.len());
    pdf.extend_from_slice(
        b"5 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Courier >>\nendobj\n",
    );
    let xref_pos = pdf.len();
    writeln!(pdf, "xref\n0 6\n0000000000 65535 f ").unwrap();
    for off in &offsets {
        writeln!(pdf, "{off:010} 00000 n ").unwrap();
    }
    write!(
        pdf,
        "trailer\n<< /Size 6 /Root 1 0 R >>\nstartxref\n{xref_pos}\n%%EOF\n"
    )
    .unwrap();
    pdf
}

fn extract(pdf: &[u8], reorder: bool) -> String {
    let reader = PdfReader::new_with_options(Cursor::new(pdf.to_vec()), ParseOptions::lenient())
        .expect("parse");
    let document = reader.into_document();
    let mut extractor = TextExtractor::with_options(ExtractionOptions {
        reorder_columns: reorder,
        detect_columns: reorder,
        ..Default::default()
    });
    extractor
        .extract_from_page(&document, 0)
        .expect("extract")
        .text
}

/// Multiset of non-whitespace characters.
fn char_counts(s: &str) -> BTreeMap<char, usize> {
    let mut m = BTreeMap::new();
    for c in s.chars().filter(|c| !c.is_whitespace()) {
        *m.entry(c).or_insert(0) += 1;
    }
    m
}

/// Build a random page and return (pdf_bytes, drawn_text). Some lines carry a
/// wide column gap so the column-detection path is exercised; words never
/// exactly overlap, so every drawn glyph is unambiguously recoverable.
fn build_page(spec: &[(Vec<usize>, Option<usize>)]) -> (Vec<u8>, String) {
    let mut content = Vec::new();
    let mut drawn = String::new();
    let mut y = 760.0;
    for (word_idxs, gap_before) in spec {
        let mut x = 50.0;
        for (pos, &wi) in word_idxs.iter().enumerate() {
            // A gap corridor before word `gap_before` exercises columns.
            if Some(pos) == *gap_before {
                x += 70.0; // > column_threshold (50)
            }
            let w = WORDS[wi % WORDS.len()];
            x = emit_glyphs(&mut content, w, x, y);
            drawn.push_str(w);
            x += 8.0;
        }
        y -= LEADING;
    }
    (wrap_pdf(&content), drawn)
}

// ---- Line-structure builder for the #390/#441 separator property. ----------

/// One generated line: word indices, an optional same-line backward
/// "correction" (word index + backward jump in pt), and the leading below it.
type LineSpec = (Vec<usize>, Option<(usize, f64)>, f64);

/// Build a page whose line structure is known by construction, and return
/// (pdf_bytes, glyphs-per-line in draw order).
///
/// Every line starts at x=50 and ends with its pen well right of the margin
/// (≥ 2 words), so each line transition is geometrically detectable: either
/// dy > newline_threshold (10pt), or a tight leading (dy < 10pt, nonzero)
/// combined with a wrap back to x=50 of more than 2× the threshold — the #390
/// class. A correction re-draws a word backward ON THE SAME baseline
/// (dy = 0, dx < -(2 × threshold)) mid-line — the #441 class — and must NOT
/// break the line.
fn emit_line_structure(lines: &[LineSpec]) -> (Vec<u8>, Vec<String>) {
    let mut content = Vec::new();
    let mut drawn_lines = Vec::new();
    let mut y = 760.0;
    for (word_idxs, correction, leading) in lines {
        let mut x = 50.0;
        let mut line = String::new();
        for (pos, &wi) in word_idxs.iter().enumerate() {
            let w = WORDS[wi % WORDS.len()];
            x = emit_glyphs(&mut content, w, x, y);
            line.push_str(w);
            x += 8.0;
            // After the first word, optionally jump BACKWARD on the same
            // baseline and draw a correction word there (the #441 signature),
            // then resume forward past everything drawn so far.
            if pos == 0 {
                if let Some((ci, back)) = correction {
                    let w2 = WORDS[ci % WORDS.len()];
                    let corr_x = (x - back).max(5.0);
                    emit_glyphs(&mut content, w2, corr_x, y);
                    line.push_str(w2);
                    x += 30.0;
                }
            }
        }
        drawn_lines.push(line);
        y -= leading;
    }
    (content, drawn_lines)
}

fn build_line_structure_page(lines: &[LineSpec]) -> (Vec<u8>, Vec<String>) {
    let (content, drawn) = emit_line_structure(lines);
    (wrap_pdf(&content), drawn)
}

/// Same page under a CTM rotated by `theta_deg`. A rigid page rotation
/// changes nothing about the text's logical structure — glyphs that share a
/// text-space baseline still share it — so the drawn-lines oracle is
/// unchanged. What rotation DOES change is every post-CTM user-space delta
/// the flat-path separator heuristics currently measure (issue #443).
///
/// The rotation pivots on the content-stream origin, so glyphs may land
/// outside the MediaBox or at negative coordinates. Harmless by design: the
/// flat extraction path performs no MediaBox clipping (verified), and the
/// oracle only cares about line structure, not placement.
fn build_rotated_line_structure_page(lines: &[LineSpec], theta_deg: f64) -> (Vec<u8>, Vec<String>) {
    let (inner, drawn) = emit_line_structure(lines);
    let (s, c) = theta_deg.to_radians().sin_cos();
    let mut content = Vec::new();
    write!(content, "q\n{c:.6} {s:.6} {:.6} {c:.6} 0 0 cm\n", -s).unwrap();
    content.extend_from_slice(&inner);
    content.extend_from_slice(b"Q\n");
    (wrap_pdf(&content), drawn)
}

/// Non-whitespace glyph runs per output line. Empty entries are kept: a
/// blank output line (e.g. a doubled newline) is itself a structure defect
/// and must surface as a mismatch, not be silently absorbed.
fn output_line_structure(text: &str) -> Vec<String> {
    text.lines()
        .map(|l| l.chars().filter(|c| !c.is_whitespace()).collect::<String>())
        .collect()
}

/// Deterministic pin for #443: a single baseline with a same-line backward
/// correction (the #441 shape), under a 20° page rotation. One line was
/// drawn, so exactly one line must come out. Today the rotation defeats the
/// #441 gate (the rotated advance gives every glyph a nonzero user-space Δy)
/// and the output splits into several lines.
#[test]
fn issue_443_rotated_page_keeps_single_line() {
    let lines = vec![(vec![0, 1], Some((2, 35.0)), 13.0)];
    let (pdf, drawn) = build_rotated_line_structure_page(&lines, 20.0);
    let flat = extract(&pdf, false);
    assert_eq!(
        output_line_structure(&flat),
        drawn,
        "rotated single-baseline page must extract as one line (#443)\n--- flat ---\n{flat}"
    );
}

/// Deterministic pin for the mirrored-baseline behavior change of the #443
/// fix: under a negative-x-scale CTM, `dx` is measured along the text's own
/// advance direction, so a plain forward advance no longer misfires the
/// backward-jump wrap gate (pre-#443 it saw raw `dx < 0` and inserted a
/// newline once the advance exceeded 2× the threshold). One drawn baseline
/// must come out as exactly one line, glyphs in draw order.
#[test]
fn mirrored_baseline_stays_a_single_line() {
    let (inner, drawn) = emit_line_structure(&[(vec![0, 1, 2], None, 13.0)]);
    let mut content = Vec::new();
    content.extend_from_slice(b"q\n-1 0 0 1 612 0 cm\n");
    content.extend_from_slice(&inner);
    content.extend_from_slice(b"Q\n");
    let flat = extract(&wrap_pdf(&content), false);
    assert_eq!(
        output_line_structure(&flat),
        drawn,
        "mirrored single-baseline page must extract as one line\n--- flat ---\n{flat}"
    );
    // The observable delta vs the pre-#443 code: word gaps advance the pen
    // LEFT in raw user space, so the old dx-based space gate never fired and
    // words came out glued ("loremipsum"). Projected onto the baseline the
    // gap is a positive forward dx and the space is inserted.
    let (w0, w1) = (WORDS[0], WORDS[1]);
    assert!(
        flat.contains(&format!("{w0} {w1}")),
        "mirrored word gap must still produce a space\n--- flat ---\n{flat}"
    );
}

// ---- Drift-chain builder for the #425 token-contiguity property. ----------

fn build_drift_page(n_lines: usize, token_line: usize, token: &str, drift_step: f64) -> Vec<u8> {
    const GAP_BASE: f64 = 150.0;
    let leaders = ["ab", "cd"];
    let mut content = Vec::new();
    let mut y = 760.0;
    for i in 0..n_lines {
        let mut x = 50.0;
        for w in leaders {
            x = emit_glyphs(&mut content, w, x, y);
            x += 8.0;
        }
        let gap_x = GAP_BASE + (i as f64) * drift_step;
        if i == token_line {
            emit_glyphs(&mut content, token, gap_x, y);
        } else {
            emit_glyphs(&mut content, "filler", gap_x, y);
        }
        y -= LEADING;
    }
    wrap_pdf(&content)
}

/// Deterministic pin for #425: a drift chain that scatters a token.
#[test]
fn issue_425_drift_chain_does_not_split_token() {
    const TOKEN: &str = "12.345.678/0001-99";
    let pdf = build_drift_page(30, 6, TOKEN, 8.0);
    let flat = extract(&pdf, false);
    let reordered = extract(&pdf, true);
    assert!(flat.contains(TOKEN), "token intact without reorder\n{flat}");
    assert!(
        reordered.contains(TOKEN),
        "reorder split the token (#425)\n--- flat ---\n{flat}\n--- reordered ---\n{reordered}"
    );
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(300))]

    /// INV-1 CONSERVATION: every drawn glyph appears in the extracted text.
    #[test]
    fn conservation_no_glyph_dropped(
        spec in prop::collection::vec(
            (prop::collection::vec(0usize..10, 2..7), prop::option::of(1usize..4)),
            3..25,
        ),
    ) {
        let (pdf, drawn) = build_page(&spec);
        let flat = extract(&pdf, false);
        let want = char_counts(&drawn);
        let got = char_counts(&flat);
        for (ch, n) in want {
            let g = got.get(&ch).copied().unwrap_or(0);
            prop_assert!(
                g >= n,
                "dropped glyph {:?}: drawn {} times, extracted {}\n{}",
                ch, n, g, flat
            );
        }
    }

    /// INV-2 REORDER IS A PERMUTATION: same non-whitespace multiset off vs on.
    #[test]
    fn reorder_is_a_permutation(
        spec in prop::collection::vec(
            (prop::collection::vec(0usize..10, 2..7), prop::option::of(1usize..4)),
            3..25,
        ),
    ) {
        let (pdf, _) = build_page(&spec);
        let flat = char_counts(&extract(&pdf, false));
        let reordered = char_counts(&extract(&pdf, true));
        prop_assert_eq!(flat, reordered, "reorder changed the character multiset");
    }

    /// INV-3 TOKEN CONTIGUITY UNDER REORDER (the #425 class).
    #[test]
    fn reorder_never_splits_a_token(
        n_lines in 15usize..40,
        token_line in 2usize..12,
        drift_step in 3.0f64..9.5,
        token in "[0-9]{2,4}[./-][0-9]{2,4}[./-][0-9]{2,4}",
    ) {
        let token_line = token_line.min(n_lines - 1);
        let pdf = build_drift_page(n_lines, token_line, &token, drift_step);
        let flat = extract(&pdf, false);
        if flat.contains(token.as_str()) {
            let reordered = extract(&pdf, true);
            prop_assert!(
                reordered.contains(token.as_str()),
                "reorder split token {:?}\n--- flat ---\n{}\n--- reordered ---\n{}",
                token, flat, reordered
            );
        }
    }

    /// INV-5 LINE STRUCTURE: flat extraction reproduces exactly the lines that
    /// were drawn — one output line per baseline, glyphs in draw order. Fails
    /// on a spurious newline (same-line backward jump misread as a wrap, #441)
    /// AND on a missing newline (tight-leading wrap glued, #390).
    #[test]
    fn flat_line_structure_matches_drawn_lines(
        lines in prop::collection::vec(
            (
                prop::collection::vec(0usize..10, 2..5),
                prop::option::of((0usize..10, 30.0f64..60.0)),
                prop_oneof![2.0f64..9.5, 12.0f64..30.0],
            ),
            3..12,
        ),
    ) {
        let (pdf, drawn) = build_line_structure_page(&lines);
        let flat = extract(&pdf, false);
        let got = output_line_structure(&flat);
        prop_assert_eq!(
            &got, &drawn,
            "extracted line structure differs from drawn lines\n--- flat ---\n{}",
            flat
        );
    }

    /// INV-6 LINE STRUCTURE UNDER ROTATION (#443, pinned): a rigid page
    /// rotation does not change the text's logical line structure, so the
    /// drawn-lines oracle of INV-5 must hold under any rotated CTM. Fails
    /// today: the flat-path separator heuristics measure post-CTM user-space
    /// deltas, so rotation both defeats the #441 same-baseline gate AND makes
    /// plain forward advance exceed `newline_threshold` vertically. Becomes a
    /// permanent guard when #443 is fixed (pen deltas measured in text space).
    #[test]
    fn flat_line_structure_survives_rotation(
        lines in prop::collection::vec(
            (
                prop::collection::vec(0usize..10, 2..5),
                prop::option::of((0usize..10, 30.0f64..60.0)),
                prop_oneof![2.0f64..9.5, 12.0f64..30.0],
            ),
            3..12,
        ),
        theta_deg in prop_oneof![
            0.5f64..90.0,
            -90.0f64..-0.5,
            Just(90.0f64),
            Just(-90.0f64)
        ],
    ) {
        let (pdf, drawn) = build_rotated_line_structure_page(&lines, theta_deg);
        let flat = extract(&pdf, false);
        let got = output_line_structure(&flat);
        prop_assert_eq!(
            &got, &drawn,
            "line structure changed under a {}° page rotation\n--- flat ---\n{}",
            theta_deg, flat
        );
    }

    /// INV-4 DETERMINISM: extracting the same bytes twice is identical.
    #[test]
    fn extraction_is_deterministic(
        spec in prop::collection::vec(
            (prop::collection::vec(0usize..10, 2..7), prop::option::of(1usize..4)),
            3..25,
        ),
        reorder in any::<bool>(),
    ) {
        let (pdf, _) = build_page(&spec);
        prop_assert_eq!(extract(&pdf, reorder), extract(&pdf, reorder));
    }
}
