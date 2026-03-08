use oxidize_pdf::pipeline::{Element, PartitionConfig};
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
    }
}

// --- Step 3.1: PartitionConfig ---

#[test]
fn test_partition_config_defaults() {
    let cfg = PartitionConfig::default();
    assert!(cfg.detect_tables);
    assert!(cfg.detect_headers_footers);
    assert!((cfg.title_min_font_ratio - 1.3).abs() < f64::EPSILON);
}

#[test]
fn test_partition_config_builder() {
    let cfg = PartitionConfig::new()
        .with_title_min_font_ratio(1.5)
        .without_tables();
    assert!((cfg.title_min_font_ratio - 1.5).abs() < f64::EPSILON);
    assert!(!cfg.detect_tables);
}

// --- Step 3.2: Title detection ---

#[test]
fn test_partition_detects_title_by_font_size() {
    let fragments = vec![
        frag("Introduction", 50.0, 700.0, 24.0, true),
        frag(
            "This is the body text of the document.",
            50.0,
            660.0,
            12.0,
            false,
        ),
        frag("More body text continues here.", 50.0, 640.0, 12.0, false),
    ];

    let elements = oxidize_pdf::pipeline::Partitioner::new(PartitionConfig::default())
        .partition_fragments(&fragments, 0, 842.0);

    let titles: Vec<_> = elements
        .iter()
        .filter(|e| matches!(e, Element::Title(_)))
        .collect();
    assert_eq!(titles.len(), 1);
    assert_eq!(titles[0].text(), "Introduction");
}

#[test]
fn test_partition_no_title_when_uniform_size() {
    let fragments = vec![
        frag("First paragraph of text.", 50.0, 700.0, 12.0, false),
        frag("Second paragraph of text.", 50.0, 680.0, 12.0, false),
        frag("Third paragraph of text.", 50.0, 660.0, 12.0, false),
    ];

    let elements = oxidize_pdf::pipeline::Partitioner::new(PartitionConfig::default())
        .partition_fragments(&fragments, 0, 842.0);

    let titles: Vec<_> = elements
        .iter()
        .filter(|e| matches!(e, Element::Title(_)))
        .collect();
    assert!(titles.is_empty());
}

#[test]
fn test_partition_multiple_titles() {
    let fragments = vec![
        frag("Chapter 1", 50.0, 750.0, 24.0, true),
        frag("Body text under chapter 1.", 50.0, 720.0, 12.0, false),
        frag("Chapter 2", 50.0, 600.0, 24.0, true),
        frag("Body text under chapter 2.", 50.0, 570.0, 12.0, false),
    ];

    let elements = oxidize_pdf::pipeline::Partitioner::new(PartitionConfig::default())
        .partition_fragments(&fragments, 0, 842.0);

    let titles: Vec<_> = elements
        .iter()
        .filter(|e| matches!(e, Element::Title(_)))
        .collect();
    assert_eq!(titles.len(), 2);
}

// --- Step 3.3: Header/Footer detection ---

#[test]
fn test_partition_detects_header() {
    let page_height = 842.0;
    // Header: near top of page (y > 95% of height)
    let fragments = vec![
        frag("Company Report 2025", 50.0, page_height - 20.0, 8.0, false),
        frag("Main body content here.", 50.0, 600.0, 12.0, false),
    ];

    let elements = oxidize_pdf::pipeline::Partitioner::new(PartitionConfig::default())
        .partition_fragments(&fragments, 0, page_height);

    let headers: Vec<_> = elements
        .iter()
        .filter(|e| matches!(e, Element::Header(_)))
        .collect();
    assert_eq!(headers.len(), 1);
    assert_eq!(headers[0].text(), "Company Report 2025");
}

#[test]
fn test_partition_detects_footer() {
    let page_height = 842.0;
    // Footer: near bottom of page (y < 5% of height)
    let fragments = vec![
        frag("Main body content here.", 50.0, 600.0, 12.0, false),
        frag("Page 1", 300.0, 20.0, 8.0, false),
    ];

    let elements = oxidize_pdf::pipeline::Partitioner::new(PartitionConfig::default())
        .partition_fragments(&fragments, 0, page_height);

    let footers: Vec<_> = elements
        .iter()
        .filter(|e| matches!(e, Element::Footer(_)))
        .collect();
    assert_eq!(footers.len(), 1);
    assert!(footers[0].text().contains("Page 1"));
}

#[test]
fn test_partition_header_footer_excluded_from_body() {
    let page_height = 842.0;
    let fragments = vec![
        frag("Header Text", 50.0, page_height - 15.0, 8.0, false),
        frag("Body paragraph one.", 50.0, 600.0, 12.0, false),
        frag("Body paragraph two.", 50.0, 570.0, 12.0, false),
        frag("Page 3", 300.0, 15.0, 8.0, false),
    ];

    let elements = oxidize_pdf::pipeline::Partitioner::new(PartitionConfig::default())
        .partition_fragments(&fragments, 0, page_height);

    let paragraphs: Vec<_> = elements
        .iter()
        .filter(|e| matches!(e, Element::Paragraph(_)))
        .collect();

    for p in &paragraphs {
        assert!(!p.text().contains("Header Text"));
        assert!(!p.text().contains("Page 3"));
    }
}

// --- Step 3.5: KeyValue detection ---

#[test]
fn test_partition_detects_key_value() {
    let fragments = vec![
        frag("Name: John Doe", 50.0, 700.0, 12.0, false),
        frag("Date: 2024-01-01", 50.0, 680.0, 12.0, false),
        frag("Regular paragraph text follows.", 50.0, 640.0, 12.0, false),
    ];

    let elements = oxidize_pdf::pipeline::Partitioner::new(PartitionConfig::default())
        .partition_fragments(&fragments, 0, 842.0);

    let kvs: Vec<_> = elements
        .iter()
        .filter(|e| matches!(e, Element::KeyValue(_)))
        .collect();

    assert_eq!(kvs.len(), 2);
}

// --- Step 3.6: ListItem detection ---

#[test]
fn test_partition_detects_bullet_list() {
    let fragments = vec![
        frag("- Item 1", 70.0, 700.0, 12.0, false),
        frag("- Item 2", 70.0, 680.0, 12.0, false),
        frag("- Item 3", 70.0, 660.0, 12.0, false),
    ];

    let elements = oxidize_pdf::pipeline::Partitioner::new(PartitionConfig::default())
        .partition_fragments(&fragments, 0, 842.0);

    let lists: Vec<_> = elements
        .iter()
        .filter(|e| matches!(e, Element::ListItem(_)))
        .collect();

    assert_eq!(lists.len(), 3);
}

#[test]
fn test_partition_detects_numbered_list() {
    let fragments = vec![
        frag("1. First item", 70.0, 700.0, 12.0, false),
        frag("2. Second item", 70.0, 680.0, 12.0, false),
    ];

    let elements = oxidize_pdf::pipeline::Partitioner::new(PartitionConfig::default())
        .partition_fragments(&fragments, 0, 842.0);

    let lists: Vec<_> = elements
        .iter()
        .filter(|e| matches!(e, Element::ListItem(_)))
        .collect();

    assert_eq!(lists.len(), 2);
}

// --- Step 3.7: PdfDocument integration ---

#[test]
fn test_pdfdocument_partition() {
    let doc = oxidize_pdf::parser::PdfDocument::open(format!(
        "{}/examples/results/hello_world.pdf",
        env!("CARGO_MANIFEST_DIR")
    ))
    .unwrap();

    let elements = doc.partition().unwrap();
    assert!(!elements.is_empty());
    // At minimum, there should be at least one paragraph
    assert!(elements.iter().any(|e| matches!(e, Element::Paragraph(_))));
}

#[test]
fn test_pdfdocument_partition_with_config() {
    let doc = oxidize_pdf::parser::PdfDocument::open(format!(
        "{}/examples/results/hello_world.pdf",
        env!("CARGO_MANIFEST_DIR")
    ))
    .unwrap();

    let elements = doc
        .partition_with(PartitionConfig::new().without_tables())
        .unwrap();

    let tables: Vec<_> = elements
        .iter()
        .filter(|e| matches!(e, Element::Table(_)))
        .collect();
    assert!(tables.is_empty());
}

#[test]
fn test_partition_preserves_page_numbers() {
    let doc = oxidize_pdf::parser::PdfDocument::open(format!(
        "{}/examples/results/ai_ready_contract.pdf",
        env!("CARGO_MANIFEST_DIR")
    ))
    .unwrap();

    let elements = doc.partition().unwrap();
    // Page numbers should be monotonically non-decreasing
    for window in elements.windows(2) {
        assert!(
            window[0].page() <= window[1].page(),
            "Page numbers should be non-decreasing: {} > {}",
            window[0].page(),
            window[1].page()
        );
    }
}

#[test]
fn test_partition_reading_order() {
    let doc = oxidize_pdf::parser::PdfDocument::open(format!(
        "{}/examples/results/hello_world.pdf",
        env!("CARGO_MANIFEST_DIR")
    ))
    .unwrap();

    let elements = doc.partition().unwrap();

    // Within same page, Y should be descending (top-to-bottom in PDF coords)
    for window in elements.windows(2) {
        if window[0].page() == window[1].page() {
            assert!(
                window[0].bbox().y >= window[1].bbox().y,
                "Reading order violated: y {} < y {}",
                window[0].bbox().y,
                window[1].bbox().y
            );
        }
    }
}
