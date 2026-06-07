use oxidize_pdf::pipeline::PartitionConfig;

#[test]
fn prefer_ruling_tables_defaults_on() {
    assert!(PartitionConfig::default().prefer_ruling_tables);
}

use oxidize_pdf::pipeline::Partitioner;
use oxidize_pdf::text::TextFragment;

#[test]
fn with_graphics_none_matches_legacy_partition() {
    let frags: Vec<TextFragment> = vec![];
    let p = Partitioner::new(PartitionConfig::default());
    let legacy = p.partition_fragments(&frags, 1, 800.0);
    let with_graphics = p.partition_fragments_with_graphics(&frags, None, 1, 800.0);
    assert_eq!(legacy.len(), with_graphics.len());
}

use oxidize_pdf::graphics::extraction::{ExtractedGraphics, GraphicsExtractor};
use oxidize_pdf::parser::PdfReader;
use oxidize_pdf::pipeline::Element;
use oxidize_pdf::text::{ExtractionOptions, Table, TextExtractor};
use oxidize_pdf::{Document, Page};

/// Build a 3-column bordered table PDF with the given rows; return (graphics, fragments).
fn bordered_table_inputs(
    rows: &[[&str; 3]],
) -> (ExtractedGraphics, Vec<oxidize_pdf::text::TextFragment>) {
    let mut doc = Document::new();
    let mut page = Page::a4();
    let mut table = Table::with_equal_columns(3, 400.0);
    table.set_position(50.0, 700.0);
    for r in rows {
        table
            .add_row(vec![r[0].to_string(), r[1].to_string(), r[2].to_string()])
            .unwrap();
    }
    page.add_table(&table).unwrap();
    doc.add_page(page);
    let path = std::env::temp_dir().join(format!("rt_{}.pdf", rows.len()));
    doc.save(&path).unwrap();

    let pdoc = PdfReader::open_document(&path).unwrap();
    let mut gx = GraphicsExtractor::default();
    let graphics = gx.extract_from_page(&pdoc, 0).unwrap();
    let opts = ExtractionOptions {
        preserve_layout: true,
        ..Default::default()
    };
    let mut tx = TextExtractor::with_options(opts);
    let frags = tx.extract_from_page(&pdoc, 0).unwrap().fragments;
    (graphics, frags)
}

fn table_rows(elements: &[Element]) -> Vec<Vec<String>> {
    elements
        .iter()
        .find_map(|e| match e {
            Element::Table(t) => Some(t.rows.clone()),
            _ => None,
        })
        .expect("a Table element")
}

#[test]
fn ruling_detects_exact_bordered_grid() {
    let (graphics, frags) =
        bordered_table_inputs(&[["H1", "H2", "H3"], ["a1", "a2", "a3"], ["b1", "b2", "b3"]]);
    assert!(
        graphics.has_table_structure(),
        "writer borders must yield grid lines"
    );

    let p = Partitioner::new(PartitionConfig::default());
    let elements = p.partition_fragments_with_graphics(&frags, Some(&graphics), 0, 842.0);

    assert_eq!(
        table_rows(&elements),
        vec![
            vec!["H1".to_string(), "H2".to_string(), "H3".to_string()],
            vec!["a1".to_string(), "a2".to_string(), "a3".to_string()],
            vec!["b1".to_string(), "b2".to_string(), "b3".to_string()],
        ]
    );
}

use oxidize_pdf::graphics::extraction::VectorLine;

fn h_line(x1: f64, x2: f64, y: f64) -> VectorLine {
    VectorLine::new(x1, y, x2, y, 1.0, true, None)
}
fn v_line(x: f64, y1: f64, y2: f64) -> VectorLine {
    VectorLine::new(x, y1, x, y2, 1.0, true, None)
}
fn frag(text: &str, x: f64, y: f64) -> TextFragment {
    TextFragment {
        text: text.to_string(),
        x,
        y,
        width: 10.0,
        height: 8.0,
        font_size: 8.0,
        font_name: None,
        is_bold: false,
        is_italic: false,
        color: None,
        space_decisions: vec![],
        mcid: None,
        struct_tag: None,
    }
}

#[test]
fn ruling_keeps_wrapped_cell_as_single_cell() {
    let mut graphics = ExtractedGraphics::new();
    for y in [100.0, 150.0, 200.0] {
        graphics.add_line(h_line(100.0, 300.0, y));
    }
    for x in [100.0, 200.0, 300.0] {
        graphics.add_line(v_line(x, 100.0, 200.0));
    }
    assert!(graphics.has_table_structure());

    let frags = vec![
        frag("Wrapped", 120.0, 180.0),
        frag("Line", 120.0, 160.0),
        frag("B", 220.0, 170.0),
        frag("c", 120.0, 120.0),
        frag("d", 220.0, 120.0),
    ];

    let p = Partitioner::new(PartitionConfig::default());
    let elements = p.partition_fragments_with_graphics(&frags, Some(&graphics), 0, 842.0);
    let rows = table_rows(&elements);
    assert_eq!(rows.len(), 2, "grid has exactly two rows, got {:?}", rows);
    assert!(
        rows[0][0].contains("Wrapped") && rows[0][0].contains("Line"),
        "wrapped lines stay in one cell, got {:?}",
        rows[0][0]
    );
}

#[test]
fn full_pipeline_partition_emits_bordered_table() {
    // Build the same bordered table, save, reopen, run the public partition entry.
    let mut doc = Document::new();
    let mut page = Page::a4();
    let mut table = Table::with_equal_columns(3, 400.0);
    table.set_position(50.0, 700.0);
    for r in [
        ["N", "Qty", "Price"],
        ["Apple", "3", "1.20"],
        ["Pear", "5", "0.90"],
    ] {
        table
            .add_row(vec![r[0].into(), r[1].into(), r[2].into()])
            .unwrap();
    }
    page.add_table(&table).unwrap();
    doc.add_page(page);
    let path = std::env::temp_dir().join("rt_fullpipe.pdf");
    doc.save(&path).unwrap();

    let pdoc = PdfReader::open_document(&path).unwrap();
    let elements = pdoc.partition().unwrap();
    let rows = table_rows(&elements);
    assert_eq!(
        rows[0],
        vec!["N".to_string(), "Qty".to_string(), "Price".to_string()]
    );
    assert_eq!(
        rows[2],
        vec!["Pear".to_string(), "5".to_string(), "0.90".to_string()]
    );
}

#[test]
fn flag_off_uses_spatial_only() {
    // With the same bordered table inputs but prefer_ruling_tables = false,
    // the ruling path is skipped; passing graphics must not change the result
    // versus passing None (graphics are ignored when the flag is off).
    let (graphics, frags) =
        bordered_table_inputs(&[["H1", "H2", "H3"], ["a1", "a2", "a3"], ["b1", "b2", "b3"]]);
    let cfg = PartitionConfig {
        prefer_ruling_tables: false,
        ..Default::default()
    };
    let p = Partitioner::new(cfg);
    let with_g = p.partition_fragments_with_graphics(&frags, Some(&graphics), 0, 842.0);
    let without_g = p.partition_fragments_with_graphics(&frags, None, 0, 842.0);
    assert_eq!(
        with_g.len(),
        without_g.len(),
        "graphics ignored when flag off"
    );
}

#[test]
fn no_grid_falls_back_to_spatial() {
    // Graphics with no table structure -> ruling path skipped, spatial still runs.
    let empty = ExtractedGraphics::new();
    assert!(!empty.has_table_structure());
    let (_g, frags) = bordered_table_inputs(&[["x1", "x2", "x3"], ["y1", "y2", "y3"]]);
    let p = Partitioner::new(PartitionConfig::default());
    // Passing empty graphics must not panic and must still classify via spatial,
    // producing the same element count as the None path.
    let elements = p.partition_fragments_with_graphics(&frags, Some(&empty), 0, 842.0);
    let none_path = p.partition_fragments_with_graphics(&frags, None, 0, 842.0);
    assert_eq!(elements.len(), none_path.len());
}
