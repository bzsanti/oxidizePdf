//! Guards for the flat-path reading-order primitive (issue #448, design §5.5).
//! Every test constructs synthetic line-group boxes so the expected permutation
//! is unambiguous and the orderer is exercised as the pure function it is.

use super::flat_reading_order::{reading_order, CutConfig, OrderBox};

/// A single-line box `font_size` tall at the given left/bottom corner.
fn line(min_x: f64, bottom_y: f64, width: f64, font_size: f64) -> OrderBox {
    OrderBox {
        min_x,
        max_x: min_x + width,
        min_y: bottom_y,
        max_y: bottom_y + font_size,
        font_size,
    }
}

fn cfg() -> CutConfig {
    CutConfig {
        horizontal_k: 1.0,
        vertical_k: 1.5,
    }
}

/// §5.5 guard 4: two columns whose lines are drawn right-column-first must come
/// out left-column-first. Ablation teeth: with the stub identity permutation
/// this fails, proving the ordering is what fixes it.
#[test]
fn two_columns_drawn_right_first_come_out_left_first() {
    // font 10, gutter = 350 - 150 = 200 pt = 20x the glyph size.
    let boxes = vec![
        line(350.0, 700.0, 100.0, 10.0), // right col, top    (stream 0)
        line(350.0, 680.0, 100.0, 10.0), // right col, bottom (stream 1)
        line(50.0, 700.0, 100.0, 10.0),  // left col,  top    (stream 2)
        line(50.0, 680.0, 100.0, 10.0),  // left col,  bottom (stream 3)
    ];
    assert_eq!(reading_order(&boxes, &cfg()), vec![2, 3, 0, 1]);
}

/// A section break (gap along Y) drawn bottom-block-first must come out
/// top-block-first. Drives the horizontal cut; without it this stays [0,1,2,3].
#[test]
fn stacked_sections_drawn_bottom_first_come_out_top_first() {
    // Two single-column blocks. Bottom block near y=100, top block near y=700;
    // the 570 pt gap is 57x the glyph size. Stream lists the bottom one first.
    let boxes = vec![
        line(50.0, 100.0, 200.0, 10.0), // bottom block, upper line (stream 0)
        line(50.0, 80.0, 200.0, 10.0),  // bottom block, lower line (stream 1)
        line(50.0, 700.0, 200.0, 10.0), // top block, upper line    (stream 2)
        line(50.0, 680.0, 200.0, 10.0), // top block, lower line    (stream 3)
    ];
    assert_eq!(reading_order(&boxes, &cfg()), vec![2, 3, 0, 1]);
}

/// §5.5 guard 1: the output is always a permutation of the input indices — no
/// box created, dropped or duplicated — whatever the layout.
#[test]
fn output_is_always_a_permutation() {
    // A two-column page with a section break inside each column: exercises both
    // axes and the recursion.
    let boxes = vec![
        line(350.0, 700.0, 100.0, 10.0),
        line(350.0, 300.0, 100.0, 10.0),
        line(50.0, 700.0, 100.0, 10.0),
        line(50.0, 300.0, 100.0, 10.0),
        line(50.0, 690.0, 100.0, 10.0),
    ];
    let order = reading_order(&boxes, &cfg());
    let mut sorted = order.clone();
    sorted.sort_unstable();
    assert_eq!(sorted, (0..boxes.len()).collect::<Vec<_>>());
}

/// §5.2 precondition: a single column already in top-to-bottom stream order is
/// returned unchanged (identity), which is what lets the caller emit verbatim.
#[test]
fn single_column_in_reading_order_is_the_identity_permutation() {
    let boxes = vec![
        line(50.0, 700.0, 200.0, 10.0),
        line(50.0, 686.0, 200.0, 10.0),
        line(50.0, 672.0, 200.0, 10.0),
        line(50.0, 658.0, 200.0, 10.0),
    ];
    assert_eq!(reading_order(&boxes, &cfg()), vec![0, 1, 2, 3]);
}

/// §5.5 guard 5: the gutter threshold has teeth on both sides. The same
/// reversed two-column input splits when `horizontal_k` is small and does NOT
/// split (stays in stream order) when it is large enough that the gutter no
/// longer clears the bar.
#[test]
fn horizontal_threshold_degrades_on_both_sides() {
    // Gutter (right edge of left box to left edge of right box) = 190 - 170
    // = 20 pt = 2x the glyph size.
    let boxes = vec![
        line(190.0, 700.0, 20.0, 10.0), // right col, x in [190,210] (stream 0)
        line(150.0, 700.0, 20.0, 10.0), // left col,  x in [150,170] (stream 1)
    ];
    let split = CutConfig {
        horizontal_k: 1.0,
        vertical_k: 1.5,
    };
    let no_split = CutConfig {
        horizontal_k: 3.0,
        vertical_k: 1.5,
    };
    assert_eq!(reading_order(&boxes, &split), vec![1, 0]); // gutter 2x >= 1x
    assert_eq!(reading_order(&boxes, &no_split), vec![0, 1]); // gutter 2x < 3x
}

/// §5.5 guard 5, vertical axis: the section threshold has teeth on both sides
/// too. The same input splits into two sections when `vertical_k` is small and
/// stays in stream order when it is large.
#[test]
fn vertical_threshold_degrades_on_both_sides() {
    // Vertical gap between the two blocks = 700 - 660 = 40 pt = 4x glyph size.
    // Stream order lists the lower block first.
    let boxes = vec![
        line(50.0, 640.0, 200.0, 10.0), // lower block, upper line (stream 0)
        line(50.0, 620.0, 200.0, 10.0), // lower block, lower line (stream 1)
        line(50.0, 700.0, 200.0, 10.0), // upper block            (stream 2)
    ];
    let split = CutConfig {
        horizontal_k: 1.0,
        vertical_k: 1.5,
    };
    let no_split = CutConfig {
        horizontal_k: 1.0,
        vertical_k: 6.0,
    };
    assert_eq!(reading_order(&boxes, &split), vec![2, 0, 1]); // 4x >= 1.5x
    assert_eq!(reading_order(&boxes, &no_split), vec![0, 1, 2]); // 4x < 6x
}

/// §5.5 guard 6 (adversarial): degenerate inputs must not panic and must still
/// return a valid permutation.
#[test]
fn adversarial_inputs_do_not_panic() {
    // Empty.
    assert_eq!(reading_order(&[], &cfg()), Vec::<usize>::new());
    // Single box.
    assert_eq!(
        reading_order(&[line(0.0, 0.0, 10.0, 10.0)], &cfg()),
        vec![0]
    );
    // All boxes at the same position (no gap on either axis) -> stream order.
    let coincident = vec![
        line(10.0, 10.0, 5.0, 10.0),
        line(10.0, 10.0, 5.0, 10.0),
        line(10.0, 10.0, 5.0, 10.0),
    ];
    assert_eq!(reading_order(&coincident, &cfg()), vec![0, 1, 2]);
    // Non-positive font size (degenerate scale) -> leaf, no division by it.
    let zero_font = vec![
        OrderBox {
            min_x: 300.0,
            max_x: 400.0,
            min_y: 0.0,
            max_y: 0.0,
            font_size: 0.0,
        },
        OrderBox {
            min_x: 0.0,
            max_x: 100.0,
            min_y: 0.0,
            max_y: 0.0,
            font_size: 0.0,
        },
    ];
    assert_eq!(reading_order(&zero_font, &cfg()), vec![0, 1]);
    // Overlapping boxes.
    let overlap = vec![line(0.0, 100.0, 300.0, 10.0), line(50.0, 98.0, 300.0, 10.0)];
    let order = reading_order(&overlap, &cfg());
    let mut sorted = order.clone();
    sorted.sort_unstable();
    assert_eq!(sorted, vec![0, 1]);
    // NaN coordinates (a degenerate CTM upstream) must not panic; total_cmp
    // orders NaN deterministically, so the result is still a permutation.
    let nan = vec![
        OrderBox {
            min_x: f64::NAN,
            max_x: f64::NAN,
            min_y: 0.0,
            max_y: 10.0,
            font_size: 10.0,
        },
        line(0.0, 0.0, 10.0, 10.0),
    ];
    let order = reading_order(&nan, &cfg());
    let mut sorted = order.clone();
    sorted.sort_unstable();
    assert_eq!(sorted, vec![0, 1]);
}
