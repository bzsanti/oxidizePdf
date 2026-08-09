//! #453: `TextStreamer`, a public extraction path, tracked only `Td`. It
//! ignored `TD`, `T*`, `Tm`, and the `'`/`"` show-and-move operators, so every
//! line placed with one of them landed at the previous line's coordinates —
//! lines fused, positions wrong. This mirrors the `PlainTextExtractor` defect of
//! #451, on the third extractor.
//!
//! The oracle is the y-coordinate of each emitted chunk: a content stream that
//! places four lines with four different positioning operators must yield four
//! chunks at four descending y values.

use oxidize_pdf::streaming::{TextStreamOptions, TextStreamer};

/// A content stream that draws one word per line, each line reached by a
/// different positioning operator:
///   line 1: `Td` (absolute-ish move from the BT origin)
///   line 2: `T*` (uses the leading set by `TL`)
///   line 3: `TD` (moves and sets a new leading in one operator)
///   line 4: `Tm` (absolute text matrix)
const STREAM: &[u8] = b"BT\n\
/F1 12 Tf\n\
14 TL\n\
100 700 Td\n\
(Alpha) Tj\n\
T*\n\
(Bravo) Tj\n\
0 -20 TD\n\
(Charlie) Tj\n\
1 0 0 1 200 500 Tm\n\
(Delta) Tj\n\
ET\n";

fn chunk_ys(stream: &[u8]) -> Vec<(String, f64)> {
    let mut s = TextStreamer::new(TextStreamOptions::default());
    let chunks = s.process_chunk(stream).expect("parses");
    chunks.into_iter().map(|c| (c.text, c.y)).collect()
}

#[test]
fn text_streamer_honors_td_tstar_tm_positioning() {
    let got = chunk_ys(STREAM);
    let texts: Vec<&str> = got.iter().map(|(t, _)| t.as_str()).collect();
    assert_eq!(
        texts,
        vec!["Alpha", "Bravo", "Charlie", "Delta"],
        "every line must be emitted as its own chunk"
    );

    let y = |name: &str| got.iter().find(|(t, _)| t == name).unwrap().1;

    // Td placed line 1 at y = 700.
    assert!((y("Alpha") - 700.0).abs() < 0.01, "Td y = {}", y("Alpha"));
    // T* dropped one leading (14) → 686. Before the fix T* was ignored and this
    // stayed at 700, fused onto line 1.
    assert!((y("Bravo") - 686.0).abs() < 0.01, "T* y = {}", y("Bravo"));
    // TD set leading 20 and moved down 20 from 686 → 666.
    assert!(
        (y("Charlie") - 666.0).abs() < 0.01,
        "TD y = {}",
        y("Charlie")
    );
    // Tm placed line 4 absolutely at y = 500.
    assert!((y("Delta") - 500.0).abs() < 0.01, "Tm y = {}", y("Delta"));

    // Distinct y for every line: the defect collapsed them.
    let mut ys: Vec<f64> = got.iter().map(|(_, y)| *y).collect();
    ys.sort_by(|a, b| a.total_cmp(b));
    ys.dedup_by(|a, b| (*a - *b).abs() < 0.01);
    assert_eq!(
        ys.len(),
        4,
        "lines collapsed onto shared y coordinates: {got:?}"
    );
}

#[test]
fn text_streamer_apostrophe_operator_moves_to_next_line() {
    // `'` is "T* then show": it must both emit the text and advance one leading.
    let stream = b"BT\n/F1 12 Tf\n10 TL\n50 400 Td\n(First) Tj\n(Second) '\n(Third) '\nET\n";
    let got = chunk_ys(stream);
    let texts: Vec<&str> = got.iter().map(|(t, _)| t.as_str()).collect();
    assert_eq!(texts, vec!["First", "Second", "Third"]);
    let y = |name: &str| got.iter().find(|(t, _)| t == name).unwrap().1;
    assert!((y("First") - 400.0).abs() < 0.01);
    assert!((y("Second") - 390.0).abs() < 0.01, "' y = {}", y("Second"));
    assert!((y("Third") - 380.0).abs() < 0.01, "' y = {}", y("Third"));
}
