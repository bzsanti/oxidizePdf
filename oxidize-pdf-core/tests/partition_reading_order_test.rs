use oxidize_pdf::pipeline::{PartitionConfig, ReadingOrderStrategy};
use oxidize_pdf::text::extraction::TextFragment;

fn frag(text: &str, x: f64, y: f64, font_size: f64) -> TextFragment {
    TextFragment {
        text: text.to_string(),
        x,
        y,
        width: text.len() as f64 * font_size * 0.5,
        height: font_size,
        font_size,
        font_name: None,
        is_bold: false,
        is_italic: false,
        color: None,
        space_decisions: Vec::new(),
    }
}

// Cycle 1.1
#[test]
fn test_partition_config_default_reading_order_is_simple() {
    let cfg = PartitionConfig::default();
    assert!(matches!(cfg.reading_order, ReadingOrderStrategy::Simple));
}

// Cycle 1.2
#[test]
fn test_partition_config_builder_with_reading_order() {
    let cfg =
        PartitionConfig::new().with_reading_order(ReadingOrderStrategy::XYCut { min_gap: 30.0 });
    assert!(matches!(
        cfg.reading_order,
        ReadingOrderStrategy::XYCut { min_gap } if (min_gap - 30.0).abs() < f64::EPSILON
    ));
}

// Cycle 1.3
#[test]
fn test_partition_none_strategy_preserves_input_order() {
    // Give fragments in bottom-to-top order (reversed from natural reading)
    let fragments = vec![
        frag("bottom", 50.0, 100.0, 12.0),
        frag("middle", 50.0, 400.0, 12.0),
        frag("top", 50.0, 700.0, 12.0),
    ];

    let cfg = PartitionConfig::new()
        .with_reading_order(ReadingOrderStrategy::None)
        .without_headers_footers();

    let elements =
        oxidize_pdf::pipeline::Partitioner::new(cfg).partition_fragments(&fragments, 0, 842.0);

    // With None strategy, no reordering: bottom comes first
    assert_eq!(elements[0].text(), "bottom");
    assert_eq!(elements[1].text(), "middle");
    assert_eq!(elements[2].text(), "top");
}

// Cycle 1.4
#[test]
fn test_partition_xycut_left_column_before_right() {
    // Two clear columns: left (x=50) and right (x=350), gap > 100pt
    let fragments = vec![
        frag("R1", 350.0, 700.0, 12.0),
        frag("L1", 50.0, 700.0, 12.0),
        frag("R2", 350.0, 680.0, 12.0),
        frag("L2", 50.0, 680.0, 12.0),
    ];

    let cfg = PartitionConfig::new()
        .with_reading_order(ReadingOrderStrategy::XYCut { min_gap: 100.0 })
        .without_headers_footers()
        .without_tables();

    let elements =
        oxidize_pdf::pipeline::Partitioner::new(cfg).partition_fragments(&fragments, 0, 842.0);

    let texts: Vec<&str> = elements.iter().map(|e| e.text()).collect();
    let l1_pos = texts.iter().position(|&t| t == "L1").unwrap();
    let l2_pos = texts.iter().position(|&t| t == "L2").unwrap();
    let r1_pos = texts.iter().position(|&t| t == "R1").unwrap();
    let r2_pos = texts.iter().position(|&t| t == "R2").unwrap();
    assert!(l1_pos < r1_pos, "L1 debe estar antes que R1");
    assert!(l2_pos < r2_pos, "L2 debe estar antes que R2");
    assert!(l1_pos < l2_pos, "L1 debe estar antes que L2");
}

// Cycle 1.5
#[test]
fn test_partition_default_backwards_compatible() {
    let fragments = vec![
        frag("Introduction", 50.0, 700.0, 24.0),
        frag("Body text content here.", 50.0, 650.0, 12.0),
        frag("More content.", 50.0, 630.0, 12.0),
    ];

    let default_elements = oxidize_pdf::pipeline::Partitioner::new(PartitionConfig::default())
        .partition_fragments(&fragments, 0, 842.0);

    let simple_elements = oxidize_pdf::pipeline::Partitioner::new(
        PartitionConfig::new().with_reading_order(ReadingOrderStrategy::Simple),
    )
    .partition_fragments(&fragments, 0, 842.0);

    assert_eq!(default_elements.len(), simple_elements.len());
    for (a, b) in default_elements.iter().zip(simple_elements.iter()) {
        assert_eq!(a.text(), b.text());
    }
}

// Cycle 1.6
#[test]
fn test_partition_xycut_three_columns_correct_order() {
    // Three columns: left (x=20), center (x=220), right (x=420), gap ~100pt
    let fragments = vec![
        frag("C1", 220.0, 700.0, 12.0),
        frag("R1", 420.0, 700.0, 12.0),
        frag("L1", 20.0, 700.0, 12.0),
        frag("C2", 220.0, 680.0, 12.0),
        frag("L2", 20.0, 680.0, 12.0),
        frag("R2", 420.0, 680.0, 12.0),
    ];

    let cfg = PartitionConfig::new()
        .with_reading_order(ReadingOrderStrategy::XYCut { min_gap: 50.0 })
        .without_headers_footers()
        .without_tables();

    let elements =
        oxidize_pdf::pipeline::Partitioner::new(cfg).partition_fragments(&fragments, 0, 842.0);

    let texts: Vec<&str> = elements.iter().map(|e| e.text()).collect();
    let l1 = texts.iter().position(|&t| t == "L1").unwrap();
    let l2 = texts.iter().position(|&t| t == "L2").unwrap();
    let c1 = texts.iter().position(|&t| t == "C1").unwrap();
    let r1 = texts.iter().position(|&t| t == "R1").unwrap();

    assert!(l1 < c1, "L1 antes que C1");
    assert!(l2 < c1, "L2 antes que C1");
    assert!(c1 < r1, "C1 antes que R1");
}
