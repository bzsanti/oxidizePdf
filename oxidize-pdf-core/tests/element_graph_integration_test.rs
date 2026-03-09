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

// ─── Cycle 2.1 ───────────────────────────────────────────────────────────────

#[test]
fn test_partition_graph_from_fragments() {
    let fragments = vec![
        frag("Chapter One", 50.0, 750.0, 24.0),
        frag("First paragraph text.", 50.0, 700.0, 12.0),
        frag("Second paragraph text.", 50.0, 680.0, 12.0),
    ];
    let elements =
        Partitioner::new(PartitionConfig::default()).partition_fragments(&fragments, 0, 842.0);

    let graph = ElementGraph::build(&elements);

    let title_idx = elements
        .iter()
        .position(|e| matches!(e, Element::Title(_)))
        .unwrap();
    assert_eq!(graph.children_of(title_idx).len(), 2);
}

// ─── Cycle 2.2 ───────────────────────────────────────────────────────────────

#[test]
fn test_pdfdocument_partition_graph_with_fixture() {
    let fixture = format!(
        "{}/tests/fixtures/Cold_Email_Hacks.pdf",
        env!("CARGO_MANIFEST_DIR")
    );
    if !std::path::Path::new(&fixture).exists() {
        eprintln!("Skipping: fixture not available");
        return;
    }
    let doc = oxidize_pdf::parser::PdfDocument::open(&fixture).unwrap();
    let (elements, graph) = doc.partition_graph(PartitionConfig::default()).unwrap();

    assert_eq!(graph.len(), elements.len());

    let has_titles = elements.iter().any(|e| matches!(e, Element::Title(_)));
    if has_titles {
        let titles_with_children: Vec<_> = elements
            .iter()
            .enumerate()
            .filter(|(i, e)| matches!(e, Element::Title(_)) && !graph.children_of(*i).is_empty())
            .collect();
        assert!(
            !titles_with_children.is_empty(),
            "At least one title must have children in the graph"
        );
    }
}
