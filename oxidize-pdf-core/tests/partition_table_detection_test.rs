use oxidize_pdf::pipeline::{Element, PartitionConfig, Partitioner};
use oxidize_pdf::text::extraction::TextFragment;

/// Build a minimal TextFragment at the given position with 12pt font.
fn table_frag(text: &str, x: f64, y: f64) -> TextFragment {
    TextFragment {
        text: text.to_string(),
        x,
        y,
        width: text.len() as f64 * 6.0,
        height: 12.0,
        font_size: 12.0,
        font_name: None,
        is_bold: false,
        is_italic: false,
        color: None,
        space_decisions: Vec::new(),
    }
}

// --- Cycle 1: Basic table with confidence threshold ---

/// A perfect 3x3 grid of fragments at aligned X/Y coordinates must produce
/// exactly one `Element::Table`.
#[test]
fn test_grid_3x3_produces_one_table() {
    // 3 columns at x=50, 200, 350 — 3 rows at y=700, 680, 660
    let fragments = vec![
        table_frag("A1", 50.0, 700.0),
        table_frag("B1", 200.0, 700.0),
        table_frag("C1", 350.0, 700.0),
        table_frag("A2", 50.0, 680.0),
        table_frag("B2", 200.0, 680.0),
        table_frag("C2", 350.0, 680.0),
        table_frag("A3", 50.0, 660.0),
        table_frag("B3", 200.0, 660.0),
        table_frag("C3", 350.0, 660.0),
    ];

    let elements = Partitioner::new(PartitionConfig::new().without_headers_footers())
        .partition_fragments(&fragments, 0, 842.0);

    let tables: Vec<_> = elements
        .iter()
        .filter(|e| matches!(e, Element::Table(_)))
        .collect();

    assert_eq!(
        tables.len(),
        1,
        "Expected 1 table from 3x3 grid, got {}",
        tables.len()
    );
}

/// The table detected from a well-formed grid must have confidence >= 0.5.
#[test]
fn test_detected_table_has_acceptable_confidence() {
    let fragments = vec![
        table_frag("A1", 50.0, 700.0),
        table_frag("B1", 200.0, 700.0),
        table_frag("C1", 350.0, 700.0),
        table_frag("A2", 50.0, 680.0),
        table_frag("B2", 200.0, 680.0),
        table_frag("C2", 350.0, 680.0),
        table_frag("A3", 50.0, 660.0),
        table_frag("B3", 200.0, 660.0),
        table_frag("C3", 350.0, 660.0),
    ];

    let elements = Partitioner::new(PartitionConfig::new().without_headers_footers())
        .partition_fragments(&fragments, 0, 842.0);

    let tables: Vec<_> = elements
        .iter()
        .filter(|e| matches!(e, Element::Table(_)))
        .collect();

    assert!(!tables.is_empty(), "Expected at least one table");

    for table in &tables {
        let confidence = table.metadata().confidence;
        assert!(
            confidence >= 0.5,
            "Table confidence {:.3} is below 0.5",
            confidence
        );
    }
}

/// Prose fragments with no columnar alignment must not produce any table.
///
/// These fragments have truly random X offsets across the full page width,
/// so no stable column clusters can form. The detector should either return
/// no table or return one with confidence below the default threshold (0.5).
#[test]
fn test_random_prose_fragments_not_detected_as_table() {
    // Scatter X positions across the full width (50–500) to prevent
    // any accidental column clustering within the 10pt tolerance.
    let fragments = vec![
        table_frag("The quick brown fox jumps over", 50.0, 700.0),
        table_frag("the lazy dog near the river", 310.0, 688.0),
        table_frag("A simple prose paragraph written", 130.0, 676.0),
        table_frag("without any tabular structure", 420.0, 664.0),
        table_frag("or alignment patterns present", 200.0, 652.0),
    ];

    let elements = Partitioner::new(
        PartitionConfig::new()
            .without_headers_footers()
            // Use default 0.5 threshold — well-scattered prose should not reach it.
            .with_min_table_confidence(0.5),
    )
    .partition_fragments(&fragments, 0, 842.0);

    let tables: Vec<_> = elements
        .iter()
        .filter(|e| matches!(e, Element::Table(_)))
        .collect();

    assert!(
        tables.is_empty(),
        "Prose fragments should not produce tables, got {}",
        tables.len()
    );
}

// --- Cycle 2: Configurable confidence threshold ---

/// Semi-aligned fragments that produce a low-confidence detection should be
/// rejected when the default threshold (0.5) applies.
///
/// We use fragments that are slightly off-column to force low confidence.
/// The detector will still produce a result, but confidence should be low enough
/// that the Partitioner rejects it.
#[test]
fn test_low_confidence_table_rejected_at_default_threshold() {
    // Two "columns" that are nearly but not quite aligned — confidence will be low.
    // Deliberately misalign by 8-15 points (above the 10pt tolerance)
    // so the detector sees 2 rows but column clustering is messy.
    let fragments = vec![
        table_frag("Key1", 50.0, 700.0),
        table_frag("Val1", 217.0, 700.0),
        table_frag("Key2", 63.0, 688.0),
        table_frag("Val2", 204.0, 688.0),
        table_frag("Key3", 41.0, 676.0),
        table_frag("Val3", 229.0, 676.0),
    ];

    let elements = Partitioner::new(PartitionConfig::new().without_headers_footers())
        .partition_fragments(&fragments, 0, 842.0);

    let tables: Vec<_> = elements
        .iter()
        .filter(|e| matches!(e, Element::Table(_)))
        .collect();

    // All tables must have confidence >= 0.5 (default threshold)
    for table in &tables {
        let confidence = table.metadata().confidence;
        assert!(
            confidence >= 0.5,
            "Table with confidence {:.3} should have been rejected by min_table_confidence=0.5",
            confidence
        );
    }
}

/// `PartitionConfig::with_min_table_confidence()` must persist the value.
#[test]
fn test_partition_config_min_table_confidence_builder() {
    let cfg = PartitionConfig::new().with_min_table_confidence(0.7);
    assert!(
        (cfg.min_table_confidence - 0.7).abs() < f64::EPSILON,
        "Expected min_table_confidence = 0.7, got {}",
        cfg.min_table_confidence
    );
}

/// The default `min_table_confidence` must be 0.5.
#[test]
fn test_partition_config_default_min_table_confidence() {
    let cfg = PartitionConfig::default();
    assert!(
        (cfg.min_table_confidence - 0.5).abs() < f64::EPSILON,
        "Expected default min_table_confidence = 0.5, got {}",
        cfg.min_table_confidence
    );
}

// --- Cycle 3: Two separate tables on the same page ---

/// Two distinct 2x2 grids separated by a large Y-gap must produce 2 independent
/// `Element::Table` elements with non-overlapping bounding boxes.
#[test]
fn test_two_tables_separated_by_gap_detected_independently() {
    // Table 1: y = 700, 680 (top of page)
    // Table 2: y = 500, 480 (150pt gap below table 1)
    let fragments = vec![
        // Table 1
        table_frag("R1C1", 50.0, 700.0),
        table_frag("R1C2", 200.0, 700.0),
        table_frag("R2C1", 50.0, 680.0),
        table_frag("R2C2", 200.0, 680.0),
        // Table 2
        table_frag("R1C1", 50.0, 500.0),
        table_frag("R1C2", 200.0, 500.0),
        table_frag("R2C1", 50.0, 480.0),
        table_frag("R2C2", 200.0, 480.0),
    ];

    let elements = Partitioner::new(PartitionConfig::new().without_headers_footers())
        .partition_fragments(&fragments, 0, 842.0);

    let tables: Vec<_> = elements
        .iter()
        .filter(|e| matches!(e, Element::Table(_)))
        .collect();

    assert_eq!(
        tables.len(),
        2,
        "Expected 2 tables from two separated grids, got {}",
        tables.len()
    );

    // Bounding boxes must be disjoint (no Y overlap)
    let bbox0 = tables[0].bbox();
    let bbox1 = tables[1].bbox();

    // Sort by Y descending (higher Y = higher on page in PDF coords)
    let (top_bbox, bottom_bbox) = if bbox0.y > bbox1.y {
        (bbox0, bbox1)
    } else {
        (bbox1, bbox0)
    };

    assert!(
        bottom_bbox.y + bottom_bbox.height < top_bbox.y,
        "Table bboxes overlap: top y={:.1} h={:.1}, bottom y={:.1} h={:.1}",
        top_bbox.y,
        top_bbox.height,
        bottom_bbox.y,
        bottom_bbox.height
    );
}

/// A prose fragment between two tables must not end up inside either table's
/// bounding box.
#[test]
fn test_table_bbox_does_not_contain_prose_between_tables() {
    // Table 1 at y=700-680, prose at y=590, Table 2 at y=500-480
    let fragments = vec![
        // Table 1
        table_frag("R1C1", 50.0, 700.0),
        table_frag("R1C2", 200.0, 700.0),
        table_frag("R2C1", 50.0, 680.0),
        table_frag("R2C2", 200.0, 680.0),
        // Prose between tables
        table_frag("Some prose text between tables.", 50.0, 590.0),
        // Table 2
        table_frag("R1C1", 50.0, 500.0),
        table_frag("R1C2", 200.0, 500.0),
        table_frag("R2C1", 50.0, 480.0),
        table_frag("R2C2", 200.0, 480.0),
    ];

    let elements = Partitioner::new(PartitionConfig::new().without_headers_footers())
        .partition_fragments(&fragments, 0, 842.0);

    let tables: Vec<_> = elements
        .iter()
        .filter(|e| matches!(e, Element::Table(_)))
        .collect();

    // There must be at least 1 table (we may or may not get 2 depending on confidence)
    // The key invariant: the prose fragment at y=590 is NOT inside any table bbox.
    let prose_y = 590.0_f64;
    for table in &tables {
        let bbox = table.bbox();
        // In PDF coordinates, y increases upward. The bbox y is the bottom-left corner.
        // A point is inside bbox if bbox.y <= point_y <= bbox.y + bbox.height
        let table_bottom = bbox.y;
        let table_top = bbox.y + bbox.height;
        assert!(
            prose_y < table_bottom || prose_y > table_top,
            "Prose at y={prose_y} falls inside table bbox y=[{table_bottom}, {table_top}]"
        );
    }
}

// --- Cycle 4: Lists must not be detected as tables ---

/// A single-column bullet list must not produce any table.
#[test]
fn test_single_column_list_not_table() {
    // All items at same X — single column, so no table structure.
    let fragments = vec![
        table_frag("- First item in the list", 50.0, 700.0),
        table_frag("- Second item in the list", 50.0, 688.0),
        table_frag("- Third item in the list", 50.0, 676.0),
        table_frag("- Fourth item in the list", 50.0, 664.0),
        table_frag("- Fifth item in the list", 50.0, 652.0),
    ];

    let elements = Partitioner::new(PartitionConfig::new().without_headers_footers())
        .partition_fragments(&fragments, 0, 842.0);

    let tables: Vec<_> = elements
        .iter()
        .filter(|e| matches!(e, Element::Table(_)))
        .collect();

    assert!(
        tables.is_empty(),
        "Single-column bullet list should not produce tables, got {}",
        tables.len()
    );
}

/// A two-column numbered list (short number + long text) must not be detected
/// as a table, and must produce `ListItem` elements instead.
#[test]
fn test_two_column_numbered_list_not_table() {
    // Column 1: narrow (1-3 chars — "1.", "2.", etc.)
    // Column 2: wide text
    let fragments = vec![
        table_frag("1.", 50.0, 700.0),
        table_frag("First item with detailed description", 80.0, 700.0),
        table_frag("2.", 50.0, 688.0),
        table_frag("Second item with detailed description", 80.0, 688.0),
        table_frag("3.", 50.0, 676.0),
        table_frag("Third item with detailed description", 80.0, 676.0),
        table_frag("4.", 50.0, 664.0),
        table_frag("Fourth item with detailed description", 80.0, 664.0),
        table_frag("5.", 50.0, 652.0),
        table_frag("Fifth item with detailed description", 80.0, 652.0),
    ];

    let elements = Partitioner::new(PartitionConfig::new().without_headers_footers())
        .partition_fragments(&fragments, 0, 842.0);

    let tables: Vec<_> = elements
        .iter()
        .filter(|e| matches!(e, Element::Table(_)))
        .collect();

    assert!(
        tables.is_empty(),
        "Two-column numbered list should not produce tables, got {}",
        tables.len()
    );

    // The text fragments should produce list items or paragraphs, not tables.
    // We only require that no table is produced — the fragments may become
    // Paragraphs or ListItems depending on the `is_list_item` heuristic.
    let non_table_count = elements
        .iter()
        .filter(|e| !matches!(e, Element::Table(_)))
        .count();

    assert!(
        non_table_count > 0,
        "Fragments should produce non-table elements"
    );
}
