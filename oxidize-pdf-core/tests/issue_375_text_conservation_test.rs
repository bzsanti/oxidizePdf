// tests/issue_375_text_conservation_test.rs
//
// #375 data-loss guard: after partitioning, the union of all element text must
// cover the page's extracted text. A prose page misdetected as an (empty) table
// currently drops its text -> this fails RED until the claim fix lands.

use oxidize_pdf::parser::{PdfDocument, PdfReader};
use oxidize_pdf::pipeline::PartitionConfig;
use oxidize_pdf::text::TextExtractor;

const FIXTURE: &str = "tests/fixtures/issue_272_boe_sumario_2025_01_15.pdf";

/// Non-whitespace tokens of `s`, lowercased, for order-independent containment.
fn tokens(s: &str) -> Vec<String> {
    s.split_whitespace().map(|t| t.to_lowercase()).collect()
}

#[test]
fn partition_conserves_page_text_default_config() {
    let doc = PdfDocument::new(PdfReader::open(FIXTURE).expect("open fixture"));

    // Expected: raw extracted text of every page.
    let mut extractor = TextExtractor::new();
    let page_count = doc.page_count().expect("page_count");
    let mut expected = String::new();
    for p in 0..page_count {
        let page_text = extractor
            .extract_from_page(&doc, p)
            .expect("extract page")
            .text;
        expected.push('\n');
        expected.push_str(&page_text);
    }

    // Actual: union of all element display text.
    let elements = doc
        .partition_with(PartitionConfig::default())
        .expect("partition");
    let actual: String = elements
        .iter()
        .map(|e| e.display_text())
        .collect::<Vec<_>>()
        .join("\n");

    let actual_tokens: std::collections::HashSet<String> = tokens(&actual).into_iter().collect();

    // Every expected token must survive partitioning (allow a small slack for
    // tokenization artifacts at cell boundaries).
    let missing: Vec<String> = tokens(&expected)
        .into_iter()
        .filter(|t| !actual_tokens.contains(t))
        .collect();

    let miss_ratio = missing.len() as f64 / tokens(&expected).len().max(1) as f64;
    assert!(
        miss_ratio < 0.01,
        "partition dropped {:.1}% of page tokens ({} missing), e.g. {:?}",
        miss_ratio * 100.0,
        missing.len(),
        &missing.iter().take(15).collect::<Vec<_>>()
    );
}

use oxidize_pdf::pipeline::Partitioner;
use oxidize_pdf::text::extraction::TextFragment;

fn frag(text: &str, x: f64, y: f64) -> TextFragment {
    TextFragment::new(text, x, y, text.len() as f64 * 6.0, 12.0, 12.0)
}

#[test]
fn spatial_table_does_not_drop_interior_prose() {
    // A justified prose block: 3 X-columns x 4 Y-rows, but one stray token sits
    // in a gap between columns (inside the table bbox, outside every cell).
    //
    // NOTE: with the current spatial-cluster detector, an isolated in-gap point
    // like this becomes its own singleton column (any point far enough from its
    // neighbors starts a new cluster) and so is captured, not dropped. This test
    // currently passes; it is kept as a durable guard for that shape of
    // regression. The actual data-loss reproduction is
    // `spatial_table_drops_column_jitter_outliers` below.
    let mut frags = Vec::new();
    for (r, y) in [780.0, 760.0, 740.0, 720.0].iter().enumerate() {
        for (c, x) in [72.0, 200.0, 330.0].iter().enumerate() {
            frags.push(frag(&format!("w{r}{c}"), *x, *y));
        }
    }
    frags.push(frag("ORPHAN", 150.0, 730.0)); // in-bbox, between columns

    let elements =
        Partitioner::new(PartitionConfig::default()).partition_fragments(&frags, 0, 842.0);
    let all: String = elements
        .iter()
        .map(|e| e.display_text())
        .collect::<Vec<_>>()
        .join(" ");
    assert!(
        all.contains("ORPHAN"),
        "interior prose token was dropped: {all}"
    );
}

/// #375 root-cause reproduction: a column whose X positions drift within the
/// spatial detector's clustering tolerance (5.0pt) still forms a single column
/// cluster, but `find_cell_for_fragment` only accepts a fragment into a cell
/// when it is within `2 * column_alignment_tolerance` (10.0pt) of the
/// *cluster mean* — not the raw point. A cluster built from a chain of small
/// (<=5pt) steps can have a mean far enough from its own extreme members that
/// those members fail the cell-assignment check and are placed in no cell.
///
/// The bounding box used to *claim* fragments is computed from the column's
/// full raw extent (`min`/`max`, padded to a 50pt minimum width), which is
/// wide enough to still cover those extreme members. So they get claimed by
/// the table (removed from consideration for prose) but never appear in any
/// cell's text: the fragment's text is silently dropped, matching the #375
/// root cause ("claim without capture") described for the ruling-detector
/// case in the design doc, reproduced here via the spatial detector.
#[test]
fn spatial_table_drops_column_jitter_outliers() {
    let mut frags = Vec::new();
    // A tight, well-behaved column (x == 250.0 for every row).
    let ys = [700.0, 680.0, 660.0, 640.0, 620.0, 600.0, 580.0, 560.0];
    // A second column whose x drifts by 4.5pt per row (each step <= the 5.0pt
    // column_alignment_tolerance, so cluster_columns still merges all 8 points
    // into one cluster), spanning 31.5pt total -- enough for the two extreme
    // rows on each end to fall outside `2 * tolerance` (10.0pt) of the
    // resulting cluster mean.
    let drift_xs = [100.0, 104.5, 109.0, 113.5, 118.0, 122.5, 127.0, 131.5];
    for (i, (&y, &x)) in ys.iter().zip(drift_xs.iter()).enumerate() {
        frags.push(frag(&format!("A{i}"), 250.0, y));
        frags.push(frag(&format!("B{i}"), x, y));
    }

    let elements =
        Partitioner::new(PartitionConfig::default()).partition_fragments(&frags, 0, 842.0);
    let all: String = elements
        .iter()
        .map(|e| e.display_text())
        .collect::<Vec<_>>()
        .join(" ");

    let missing: Vec<String> = (0..8)
        .map(|i| format!("B{i}"))
        .filter(|tok| !all.contains(tok.as_str()))
        .collect();

    assert!(
        missing.is_empty(),
        "column-jitter outliers were claimed by the table but dropped from every cell: {missing:?}\nfull output: {all}"
    );
}
