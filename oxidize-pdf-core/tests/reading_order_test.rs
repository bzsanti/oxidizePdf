use oxidize_pdf::pipeline::reading_order::{ReadingOrder, SimpleReadingOrder, XYCutReadingOrder};
use oxidize_pdf::text::extraction::TextFragment;

fn frag(text: &str, x: f64, y: f64) -> TextFragment {
    TextFragment {
        text: text.to_string(),
        x,
        y,
        width: 50.0,
        height: 12.0,
        font_size: 12.0,
        font_name: None,
        is_bold: false,
        is_italic: false,
        color: None,
    }
}

// --- Step 4.1: SimpleReadingOrder ---

#[test]
fn test_simple_reading_order_top_to_bottom() {
    let mut fragments = vec![
        frag("bottom", 50.0, 100.0),
        frag("top", 50.0, 700.0),
        frag("middle", 50.0, 400.0),
    ];

    SimpleReadingOrder::new(5.0).order(&mut fragments);

    // Higher Y = top of page → should come first
    assert_eq!(fragments[0].text, "top");
    assert_eq!(fragments[1].text, "middle");
    assert_eq!(fragments[2].text, "bottom");
}

#[test]
fn test_simple_reading_order_left_to_right_same_line() {
    let mut fragments = vec![
        frag("right", 300.0, 500.0),
        frag("left", 50.0, 502.0), // within threshold of 5.0
        frag("center", 150.0, 499.0),
    ];

    SimpleReadingOrder::new(5.0).order(&mut fragments);

    assert_eq!(fragments[0].text, "left");
    assert_eq!(fragments[1].text, "center");
    assert_eq!(fragments[2].text, "right");
}

// --- Step 4.2: XY-Cut ---

#[test]
fn test_xycut_single_column() {
    let mut fragments_xy = vec![
        frag("line3", 50.0, 100.0),
        frag("line1", 50.0, 700.0),
        frag("line2", 50.0, 400.0),
    ];
    let mut fragments_simple = fragments_xy.clone();

    XYCutReadingOrder::default().order(&mut fragments_xy);
    SimpleReadingOrder::new(5.0).order(&mut fragments_simple);

    // Same result for single column
    for (a, b) in fragments_xy.iter().zip(fragments_simple.iter()) {
        assert_eq!(a.text, b.text);
    }
}

#[test]
fn test_xycut_two_columns() {
    // Left column: three lines
    // Right column: three lines
    // With a clear gap between x~100 and x~300
    let mut fragments = vec![
        frag("R1", 300.0, 700.0),
        frag("L1", 50.0, 700.0),
        frag("R2", 300.0, 680.0),
        frag("L2", 50.0, 680.0),
        frag("R3", 300.0, 660.0),
        frag("L3", 50.0, 660.0),
    ];

    XYCutReadingOrder::default().order(&mut fragments);

    // XY-Cut should read left column first, then right column
    let texts: Vec<&str> = fragments.iter().map(|f| f.text.as_str()).collect();
    assert_eq!(texts, vec!["L1", "L2", "L3", "R1", "R2", "R3"]);
}

#[test]
fn test_xycut_mixed_single_and_double() {
    // Title spanning full width at top
    // Then two columns below
    let mut fragments = vec![
        frag("Title", 50.0, 750.0),   // full-width title
        frag("ColA-1", 50.0, 600.0),  // left column
        frag("ColB-1", 300.0, 600.0), // right column
        frag("ColA-2", 50.0, 580.0),
        frag("ColB-2", 300.0, 580.0),
    ];
    // Make title wider
    fragments[0].width = 400.0;

    XYCutReadingOrder::default().order(&mut fragments);

    let texts: Vec<&str> = fragments.iter().map(|f| f.text.as_str()).collect();
    // Title first, then left column, then right column
    assert_eq!(texts[0], "Title");
    assert!(texts[1..3].contains(&"ColA-1"));
    assert!(texts[1..3].contains(&"ColA-2"));
}

#[test]
fn test_xycut_empty() {
    let mut fragments: Vec<TextFragment> = vec![];
    XYCutReadingOrder::default().order(&mut fragments);
    assert!(fragments.is_empty());
}

#[test]
fn test_xycut_single_fragment() {
    let mut fragments = vec![frag("only", 50.0, 500.0)];
    XYCutReadingOrder::default().order(&mut fragments);
    assert_eq!(fragments.len(), 1);
    assert_eq!(fragments[0].text, "only");
}

#[test]
fn test_xycut_newspaper_three_columns() {
    let mut fragments = vec![
        frag("C1", 50.0, 700.0),
        frag("C2", 200.0, 700.0),
        frag("C3", 350.0, 700.0),
        frag("C1b", 50.0, 680.0),
        frag("C2b", 200.0, 680.0),
        frag("C3b", 350.0, 680.0),
    ];

    XYCutReadingOrder::default().order(&mut fragments);

    let texts: Vec<&str> = fragments.iter().map(|f| f.text.as_str()).collect();
    // Left-to-right columns, top-to-bottom within each
    assert_eq!(texts, vec!["C1", "C1b", "C2", "C2b", "C3", "C3b"]);
}
