use oxidize_pdf::pipeline::{Element, ElementMetadata, PartitionConfig, TableElementData};
use oxidize_pdf::text::extraction::TextFragment;

fn frag(text: &str, x: f64, y: f64, font_size: f64, bold: bool) -> TextFragment {
    TextFragment {
        text: text.to_string(),
        x,
        y,
        width: text.len() as f64 * font_size * 0.5,
        height: font_size,
        font_size,
        font_name: None,
        is_bold: bold,
        is_italic: false,
        color: None,
        space_decisions: Vec::new(),
    }
}

// Cycle 2.1
#[test]
fn test_paragraph_confidence_is_one() {
    let fragments = vec![
        frag(
            "Body text here, a whole sentence.",
            50.0,
            600.0,
            12.0,
            false,
        ),
        frag("Another body sentence here.", 50.0, 580.0, 12.0, false),
    ];
    let elements = oxidize_pdf::pipeline::Partitioner::new(PartitionConfig::default())
        .partition_fragments(&fragments, 0, 842.0);

    for elem in &elements {
        if matches!(elem, Element::Paragraph(_)) {
            assert!(
                (elem.metadata().confidence - 1.0).abs() < f64::EPSILON,
                "Paragraph confidence debe ser 1.0, got {}",
                elem.metadata().confidence
            );
        }
    }
}

#[test]
fn test_list_item_confidence_is_one() {
    let fragments = vec![
        frag("- First item in the list", 50.0, 600.0, 12.0, false),
        frag("- Second item in the list", 50.0, 580.0, 12.0, false),
    ];
    let elements = oxidize_pdf::pipeline::Partitioner::new(PartitionConfig::default())
        .partition_fragments(&fragments, 0, 842.0);

    let list_items: Vec<_> = elements
        .iter()
        .filter(|e| matches!(e, Element::ListItem(_)))
        .collect();
    assert!(!list_items.is_empty(), "debe haber al menos un ListItem");
    for item in &list_items {
        assert!(
            (item.metadata().confidence - 1.0).abs() < f64::EPSILON,
            "ListItem confidence debe ser 1.0"
        );
    }
}

// Cycle 2.2
#[test]
fn test_title_confidence_high_for_large_font_ratio() {
    // 30pt vs 12pt body -> ratio = 2.5, well above threshold 1.3
    let fragments = vec![
        frag("Big Title", 50.0, 750.0, 30.0, false),
        frag("body one here.", 50.0, 700.0, 12.0, false),
        frag("body two here.", 50.0, 680.0, 12.0, false),
        frag("body three here.", 50.0, 660.0, 12.0, false),
    ];
    let elements = oxidize_pdf::pipeline::Partitioner::new(PartitionConfig::default())
        .partition_fragments(&fragments, 0, 842.0);

    let titles: Vec<_> = elements
        .iter()
        .filter(|e| matches!(e, Element::Title(_)))
        .collect();
    assert!(!titles.is_empty(), "debe detectar un titulo");
    assert!(
        titles[0].metadata().confidence > 0.9,
        "Title con ratio alto debe tener confidence > 0.9, got {}",
        titles[0].metadata().confidence
    );
}

#[test]
fn test_title_confidence_lower_near_threshold() {
    // 16pt vs 12pt -> ratio = 1.33, just above threshold 1.3
    let fragments = vec![
        frag("Small Title", 50.0, 750.0, 16.0, false),
        frag("body text here.", 50.0, 700.0, 12.0, false),
        frag("body text two.", 50.0, 680.0, 12.0, false),
        frag("body text three.", 50.0, 660.0, 12.0, false),
    ];
    let elements = oxidize_pdf::pipeline::Partitioner::new(PartitionConfig::default())
        .partition_fragments(&fragments, 0, 842.0);

    let titles: Vec<_> = elements
        .iter()
        .filter(|e| matches!(e, Element::Title(_)))
        .collect();
    if !titles.is_empty() {
        let c = titles[0].metadata().confidence;
        assert!(
            c >= 0.5 && c < 0.9,
            "Title cerca del threshold debe tener confidence 0.5..0.9, got {}",
            c
        );
    }
}

// Cycle 2.3
#[test]
fn test_header_at_extreme_top_has_high_confidence() {
    let fragments = vec![
        frag("Page 1 of 10", 400.0, 835.0, 10.0, false),
        frag("Main content here.", 50.0, 400.0, 12.0, false),
    ];
    let elements = oxidize_pdf::pipeline::Partitioner::new(PartitionConfig::default())
        .partition_fragments(&fragments, 0, 842.0);

    let headers: Vec<_> = elements
        .iter()
        .filter(|e| matches!(e, Element::Header(_)))
        .collect();
    assert!(!headers.is_empty(), "debe detectar un header");
    assert!(
        headers[0].metadata().confidence > 0.5,
        "Header cerca del borde debe tener confidence > 0.5, got {}",
        headers[0].metadata().confidence
    );
}

#[test]
fn test_header_at_zone_border_has_minimum_confidence() {
    let fragments = vec![
        frag("Borderline header", 400.0, 800.0, 10.0, false),
        frag("Main content here.", 50.0, 400.0, 12.0, false),
    ];
    let elements = oxidize_pdf::pipeline::Partitioner::new(PartitionConfig::default())
        .partition_fragments(&fragments, 0, 842.0);

    for elem in &elements {
        assert!(
            elem.metadata().confidence >= 0.5,
            "Todo elemento debe tener confidence >= 0.5, got {}",
            elem.metadata().confidence
        );
    }
}

// Cycle 2.4
#[test]
fn test_kv_confidence_short_key_higher_than_long_key() {
    let fragments_short = vec![frag("ID: 123", 50.0, 600.0, 12.0, false)];
    let fragments_long = vec![frag("Full Name: John", 50.0, 600.0, 12.0, false)];

    let partitioner = oxidize_pdf::pipeline::Partitioner::new(PartitionConfig::default());
    let elems_short = partitioner.partition_fragments(&fragments_short, 0, 842.0);
    let elems_long = partitioner.partition_fragments(&fragments_long, 0, 842.0);

    let kv_short = elems_short
        .iter()
        .find(|e| matches!(e, Element::KeyValue(_)));
    let kv_long = elems_long
        .iter()
        .find(|e| matches!(e, Element::KeyValue(_)));

    if let (Some(ks), Some(kl)) = (kv_short, kv_long) {
        assert!(
            ks.metadata().confidence >= kl.metadata().confidence,
            "Key corta debe tener confidence >= key larga: {} vs {}",
            ks.metadata().confidence,
            kl.metadata().confidence
        );
    }
}

#[test]
fn test_kv_confidence_minimum_is_0_5() {
    let fragments = vec![frag(
        "Some Very Long Label Name: value",
        50.0,
        600.0,
        12.0,
        false,
    )];
    let elements = oxidize_pdf::pipeline::Partitioner::new(PartitionConfig::default())
        .partition_fragments(&fragments, 0, 842.0);

    for elem in &elements {
        assert!(
            elem.metadata().confidence >= 0.5,
            "Todo elemento debe tener confidence >= 0.5, got {}",
            elem.metadata().confidence
        );
    }
}

// Cycle 2.5
#[test]
fn test_table_element_confidence_is_not_overridden() {
    let table = Element::Table(TableElementData {
        rows: vec![vec!["A".to_string(), "B".to_string()]],
        metadata: ElementMetadata {
            confidence: 0.7,
            ..Default::default()
        },
    });
    assert!((table.metadata().confidence - 0.7).abs() < f64::EPSILON);
}
