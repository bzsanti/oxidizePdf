use oxidize_pdf::text::text_block::measure_text_block;
use oxidize_pdf::{Font, Margins, TextAlign, TextFlowContext};

#[test]
fn test_single_line_no_wrap() {
    let metrics = measure_text_block("Hello", &Font::Helvetica, 12.0, 1.2, 500.0);
    assert_eq!(metrics.line_count, 1);
    assert!(
        (metrics.height - 14.4).abs() < 0.01,
        "expected height ~14.4 (12.0 * 1.2), got {}",
        metrics.height
    );
    assert!(metrics.width > 0.0);
    assert!(metrics.width <= 500.0);
}

#[test]
fn test_wraps_correctly() {
    let text = "word word word word";
    let metrics = measure_text_block(text, &Font::Helvetica, 12.0, 1.2, 60.0);
    assert!(
        metrics.line_count >= 2,
        "expected >= 2 lines for '{}' in 60pt, got {}",
        text,
        metrics.line_count
    );
    assert!(
        metrics.height > 14.4,
        "multiline block must be taller than one line (14.4), got {}",
        metrics.height
    );
    assert!(
        metrics.width <= 60.0 + 0.001,
        "width {} exceeds max_width 60.0",
        metrics.width
    );
}

#[test]
fn test_empty_string() {
    let metrics = measure_text_block("", &Font::Helvetica, 12.0, 1.2, 300.0);
    assert_eq!(metrics.line_count, 0);
    assert_eq!(metrics.width, 0.0);
    assert_eq!(metrics.height, 0.0);
}

#[test]
fn test_overlong_word() {
    let metrics = measure_text_block(
        "Pneumonoultramicroscopicsilicovolcanoconiosis",
        &Font::Helvetica,
        12.0,
        1.2,
        50.0,
    );
    assert_eq!(
        metrics.line_count, 1,
        "overlong single word stays on one line"
    );
    // width will exceed max_width — that's expected for unbreakable words
    assert!(metrics.width > 50.0);
}

#[test]
fn test_consistent_with_flow_context() {
    let max_width = 200.0;
    let font_size = 12.0;
    let line_height = 1.2;
    let text = "The quick brown fox jumps over the lazy dog and then runs away fast";

    let metrics = measure_text_block(text, &Font::Helvetica, font_size, line_height, max_width);

    // Render with TextFlowContext and count Td operators (= lines rendered)
    let margins = Margins {
        left: 50.0,
        right: 50.0,
        top: 50.0,
        bottom: 50.0,
    };
    // page_width such that content_width = max_width: 200 + 50 + 50 = 300
    let mut flow = TextFlowContext::new(300.0, 841.89, margins);
    flow.set_font(Font::Helvetica, font_size)
        .set_line_height(line_height)
        .set_alignment(TextAlign::Left);
    flow.write_wrapped(text).unwrap();
    let ops = flow.operations().to_owned();
    let td_count = ops.lines().filter(|l| l.ends_with(" Td")).count();

    assert_eq!(
        metrics.line_count, td_count,
        "measure_text_block line_count {} != TextFlowContext Td count {}.\nOperations:\n{}",
        metrics.line_count, td_count, ops
    );
}
