//! Issue #375 Task 9: the ruling-detector partition path must emit the RICH
//! `TableStructure` (merged cells + header rows), with the flat `rows` view
//! derived from it via `TableElementData::from_structure`. The spatial path
//! keeps `structure: None` (unchanged, not exercised here).
//!
//! Fixture: same merged-header grid as the Task 7 detect test (a 2-col table
//! whose top row omits the middle vertical divider, producing one spanning
//! header cell with `col_span == 2`), run through the full partition path.

use oxidize_pdf::graphics::extraction::{ExtractedGraphics, VectorLine};
use oxidize_pdf::pipeline::{Element, PartitionConfig, Partitioner};
use oxidize_pdf::text::extraction::TextFragment;

fn hline(x1: f64, x2: f64, y: f64) -> VectorLine {
    VectorLine::new(x1, y, x2, y, 1.0, true, None)
}

fn vline(x: f64, y1: f64, y2: f64) -> VectorLine {
    VectorLine::new(x, y1, x, y2, 1.0, true, None)
}

fn frag(t: &str, x: f64, y: f64) -> TextFragment {
    TextFragment::new(t, x, y, 30.0, 10.0, 10.0)
}

fn build_graphics(h: Vec<VectorLine>, v: Vec<VectorLine>) -> ExtractedGraphics {
    let mut g = ExtractedGraphics::new();
    for line in h.into_iter().chain(v) {
        g.add_line(line);
    }
    g
}

#[test]
fn ruling_partition_emits_rich_structure_with_merged_header() {
    // Grid X at 100,200,300 ; Y at 100 (bottom),150 (mid),200 (top).
    // Row 0 (top band, y 150..200) has NO middle vertical => one spanning cell.
    // Row 1 (y 100..150) HAS the middle vertical => two cells.
    let h = vec![
        hline(100.0, 300.0, 100.0),
        hline(100.0, 300.0, 150.0),
        hline(100.0, 300.0, 200.0),
    ];
    let v = vec![
        vline(100.0, 100.0, 200.0), // left border (full height)
        vline(300.0, 100.0, 200.0), // right border (full height)
        vline(200.0, 100.0, 150.0), // MIDDLE divider only in row 1
    ];
    let graphics = build_graphics(h, v);

    let mut header_frag = frag("Header", 150.0, 170.0);
    header_frag.struct_tag = Some("TH".to_string());
    let frags = vec![
        header_frag,
        frag("A", 130.0, 120.0),
        frag("B", 230.0, 120.0),
    ];

    let elements = Partitioner::new(PartitionConfig::default()).partition_fragments_with_graphics(
        &frags,
        Some(&graphics),
        0,
        842.0,
    );

    let table = elements
        .iter()
        .find_map(|e| match e {
            Element::Table(t) => Some(t),
            _ => None,
        })
        .expect("a table element");

    let st = table.structure.as_ref().expect("rich structure present");
    assert!(
        st.cells
            .iter()
            .any(|c| c.row == 0 && c.col == 0 && c.col_span == 2 && c.is_header),
        "expected a spanning header cell at row 0, col 0 with col_span == 2"
    );
    assert_eq!(st.header_rows, 1, "single header row expected");

    // Flat view still complete (spanned value repeated across both columns).
    assert_eq!(table.rows[0].len(), 2);
}
