use oxidize_pdf::pipeline::{Element, ElementData, ElementGraph, ElementMetadata};

fn make_title(text: &str) -> Element {
    Element::Title(ElementData {
        text: text.to_string(),
        metadata: ElementMetadata {
            parent_heading: Some(text.to_string()),
            ..Default::default()
        },
    })
}

fn make_paragraph(text: &str) -> Element {
    Element::Paragraph(ElementData {
        text: text.to_string(),
        metadata: ElementMetadata::default(),
    })
}

fn make_para_with_heading(text: &str, heading: &str) -> Element {
    Element::Paragraph(ElementData {
        text: text.to_string(),
        metadata: ElementMetadata {
            parent_heading: if heading.is_empty() {
                None
            } else {
                Some(heading.to_string())
            },
            ..Default::default()
        },
    })
}

fn make_title_with_heading(text: &str, heading: &str) -> Element {
    Element::Title(ElementData {
        text: text.to_string(),
        metadata: ElementMetadata {
            parent_heading: Some(heading.to_string()),
            ..Default::default()
        },
    })
}

// ─── Cycle 1.1 ───────────────────────────────────────────────────────────────

#[test]
fn test_graph_build_empty() {
    let graph = ElementGraph::build(&[]);
    assert_eq!(graph.len(), 0);
}

#[test]
fn test_graph_build_single_element() {
    let elements = vec![make_title("Only Title")];
    let graph = ElementGraph::build(&elements);
    assert_eq!(graph.len(), 1);
    assert!(graph.parent_of(0).is_none());
    assert!(graph.children_of(0).is_empty());
    assert!(graph.next_of(0).is_none());
    assert!(graph.prev_of(0).is_none());
}

// ─── Cycle 1.2 ───────────────────────────────────────────────────────────────

#[test]
fn test_graph_next_prev_chain() {
    let elements = vec![
        make_paragraph("First"),
        make_paragraph("Second"),
        make_paragraph("Third"),
    ];
    let graph = ElementGraph::build(&elements);
    assert_eq!(graph.next_of(0), Some(1));
    assert_eq!(graph.next_of(1), Some(2));
    assert!(graph.next_of(2).is_none());
    assert!(graph.prev_of(0).is_none());
    assert_eq!(graph.prev_of(1), Some(0));
    assert_eq!(graph.prev_of(2), Some(1));
}

// ─── Cycle 1.3 ───────────────────────────────────────────────────────────────

#[test]
fn test_graph_parent_child_simple() {
    let elements = vec![
        make_title_with_heading("Introduction", "Introduction"),
        make_para_with_heading("Para 1", "Introduction"),
        make_para_with_heading("Para 2", "Introduction"),
    ];
    let graph = ElementGraph::build(&elements);

    assert_eq!(graph.children_of(0), &[1, 2]);
    assert_eq!(graph.parent_of(1), Some(0));
    assert_eq!(graph.parent_of(2), Some(0));
    assert!(graph.parent_of(0).is_none());
}

#[test]
fn test_graph_parent_switches_on_new_title() {
    let elements = vec![
        make_title_with_heading("Chapter 1", "Chapter 1"),
        make_para_with_heading("Para A", "Chapter 1"),
        make_title_with_heading("Chapter 2", "Chapter 2"),
        make_para_with_heading("Para B", "Chapter 2"),
    ];
    let graph = ElementGraph::build(&elements);

    assert_eq!(graph.children_of(0), &[1]);
    assert_eq!(graph.children_of(2), &[3]);
    assert_eq!(graph.parent_of(1), Some(0));
    assert_eq!(graph.parent_of(3), Some(2));
}

#[test]
fn test_graph_no_parent_heading_means_no_parent() {
    let elements = vec![
        make_para_with_heading("Preamble", ""),
        make_title_with_heading("Intro", "Intro"),
        make_para_with_heading("Body", "Intro"),
    ];
    let graph = ElementGraph::build(&elements);

    assert!(graph.parent_of(0).is_none());
    assert_eq!(graph.parent_of(2), Some(1));
}

// ─── Cycle 1.4 ───────────────────────────────────────────────────────────────

#[test]
fn test_graph_section_elements() {
    let elements = vec![
        make_title_with_heading("Ch1", "Ch1"),
        make_para_with_heading("P1", "Ch1"),
        make_para_with_heading("P2", "Ch1"),
        make_title_with_heading("Ch2", "Ch2"),
        make_para_with_heading("P3", "Ch2"),
    ];
    let graph = ElementGraph::build(&elements);

    let ch1_section = graph.elements_in_section(0);
    assert_eq!(ch1_section, vec![1, 2]);

    let ch2_section = graph.elements_in_section(3);
    assert_eq!(ch2_section, vec![4]);
}

#[test]
fn test_graph_top_level_sections() {
    let elements = vec![
        make_para_with_heading("Preamble", ""),
        make_title_with_heading("Ch1", "Ch1"),
        make_para_with_heading("P1", "Ch1"),
        make_title_with_heading("Ch2", "Ch2"),
    ];
    let graph = ElementGraph::build(&elements);

    let sections = graph.top_level_sections();
    assert_eq!(sections, vec![1, 3]);
}

// ─── Cycle 1.5 ───────────────────────────────────────────────────────────────

#[test]
#[should_panic]
fn test_graph_out_of_bounds_panics() {
    let elements = vec![make_title("T")];
    let graph = ElementGraph::build(&elements);
    let _ = graph.parent_of(99);
}
