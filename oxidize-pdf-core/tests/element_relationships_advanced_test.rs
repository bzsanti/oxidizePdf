use oxidize_pdf::pipeline::{Element, ElementGraph, PartitionConfig, Partitioner};
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

// ─── Cycle 4.1 ───────────────────────────────────────────────────────────────

#[test]
fn test_full_navigation_three_sections() {
    let fragments = vec![
        frag("Preamble text here.", 50.0, 800.0, 12.0),
        frag("Chapter One", 50.0, 750.0, 24.0),
        frag("Content of chapter one.", 50.0, 700.0, 12.0),
        frag("More content of chapter one.", 50.0, 680.0, 12.0),
        frag("Chapter Two", 50.0, 630.0, 24.0),
        frag("Content of chapter two.", 50.0, 580.0, 12.0),
        frag("Chapter Three", 50.0, 530.0, 24.0),
    ];
    let elements =
        Partitioner::new(PartitionConfig::default()).partition_fragments(&fragments, 0, 842.0);
    let graph = ElementGraph::build(&elements);

    let sections = graph.top_level_sections();
    assert_eq!(sections.len(), 3);

    let ch1_idx = sections[0];
    let ch1_children = graph.elements_in_section(ch1_idx);
    assert_eq!(ch1_children.len(), 2);

    let ch3_idx = sections[2];
    let ch3_children = graph.elements_in_section(ch3_idx);
    assert_eq!(ch3_children.len(), 0);

    // next_of the title should point to the element right after it
    assert_eq!(graph.next_of(ch1_idx), Some(ch1_idx + 1));
}

// ─── Cycle 4.2 ───────────────────────────────────────────────────────────────

#[test]
fn test_partition_api_unchanged() {
    let fragments = vec![
        frag("Title", 50.0, 750.0, 24.0),
        frag("Body.", 50.0, 700.0, 12.0),
    ];
    let elements: Vec<oxidize_pdf::pipeline::Element> =
        Partitioner::new(PartitionConfig::default()).partition_fragments(&fragments, 0, 842.0);

    assert!(!elements.is_empty());
    assert_eq!(
        elements[0].metadata().parent_heading.as_deref(),
        Some("Title")
    );
}

// Additional: verify graph is consistent with the element slice length.
#[test]
fn test_graph_len_matches_elements_len() {
    let fragments = vec![
        frag("Heading", 50.0, 750.0, 20.0),
        frag("Paragraph A.", 50.0, 700.0, 12.0),
        frag("Paragraph B.", 50.0, 680.0, 12.0),
    ];
    let elements =
        Partitioner::new(PartitionConfig::default()).partition_fragments(&fragments, 0, 842.0);
    let graph = ElementGraph::build(&elements);

    assert_eq!(graph.len(), elements.len());
    assert!(!graph.is_empty());
}

// Additional: parent/child relationships survive multiple pages when graph is
// built from the combined element list.
#[test]
fn test_graph_multi_page_elements() {
    // Simulate page 0 elements
    let frags_p0 = vec![
        frag("Section A", 50.0, 750.0, 24.0),
        frag("Para on page 0.", 50.0, 700.0, 12.0),
    ];
    let mut elements =
        Partitioner::new(PartitionConfig::default()).partition_fragments(&frags_p0, 0, 842.0);

    // Simulate page 1 elements
    let frags_p1 = vec![
        frag("Section B", 50.0, 750.0, 24.0),
        frag("Para on page 1.", 50.0, 700.0, 12.0),
    ];
    elements.extend(
        Partitioner::new(PartitionConfig::default()).partition_fragments(&frags_p1, 1, 842.0),
    );

    let graph = ElementGraph::build(&elements);

    assert_eq!(graph.len(), elements.len());

    let sections = graph.top_level_sections();
    assert_eq!(sections.len(), 2);

    // Each section should have exactly 1 child paragraph.
    for &sec_idx in &sections {
        assert_eq!(
            graph.elements_in_section(sec_idx).len(),
            1,
            "Section at {} should have 1 child",
            sec_idx
        );
    }
}

// Verify that the Element type returned by partition() is still the same Vec<Element>
// and can be iterated, mapped, and filtered without the graph.
#[test]
fn test_partition_returns_owned_vec() {
    let fragments = vec![
        frag("Header here", 50.0, 750.0, 18.0),
        frag("Some body text.", 50.0, 700.0, 12.0),
    ];
    let elements =
        Partitioner::new(PartitionConfig::default()).partition_fragments(&fragments, 0, 842.0);

    // Standard Vec operations must still work.
    let title_count = elements
        .iter()
        .filter(|e| matches!(e, Element::Title(_)))
        .count();
    assert!(title_count >= 1);

    let texts: Vec<&str> = elements.iter().map(|e| e.text()).collect();
    assert!(!texts.is_empty());
}
