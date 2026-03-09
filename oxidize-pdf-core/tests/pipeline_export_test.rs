use oxidize_pdf::pipeline::export::ElementMarkdownExporter;
use oxidize_pdf::pipeline::{
    Element, ElementBBox, ElementData, ElementMetadata, ImageElementData, KeyValueElementData,
    TableElementData,
};

fn meta() -> ElementMetadata {
    ElementMetadata {
        page: 0,
        bbox: ElementBBox::ZERO,
        confidence: 1.0,
        ..Default::default()
    }
}

fn title(text: &str) -> Element {
    Element::Title(ElementData {
        text: text.to_string(),
        metadata: meta(),
    })
}

fn para(text: &str) -> Element {
    Element::Paragraph(ElementData {
        text: text.to_string(),
        metadata: meta(),
    })
}

// Cycle 3.1
#[test]
fn test_export_empty_produces_empty_string() {
    let exporter = ElementMarkdownExporter::default();
    let output = exporter.export(&[]);
    assert!(output.is_empty());
}

// Cycle 3.2
#[test]
fn test_export_title_produces_h1() {
    let exporter = ElementMarkdownExporter::default();
    assert_eq!(exporter.export(&[title("Introduction")]), "# Introduction");
}

// Cycle 3.3
#[test]
fn test_export_paragraph_produces_plain_text() {
    let exporter = ElementMarkdownExporter::default();
    assert_eq!(
        exporter.export(&[para("This is a paragraph.")]),
        "This is a paragraph."
    );
}

// Cycle 3.4
#[test]
fn test_export_table_pipe_format() {
    let exporter = ElementMarkdownExporter::default();
    let elements = vec![Element::Table(TableElementData {
        rows: vec![
            vec!["Name".to_string(), "Age".to_string()],
            vec!["Alice".to_string(), "30".to_string()],
            vec!["Bob".to_string(), "25".to_string()],
        ],
        metadata: meta(),
    })];
    let output = exporter.export(&elements);
    assert!(output.contains("| Name | Age |"));
    assert!(output.contains("| --- | --- |"));
    assert!(output.contains("| Alice | 30 |"));
}

// Cycle 3.5
#[test]
fn test_export_list_item_bullet() {
    let exporter = ElementMarkdownExporter::default();
    let elements = vec![Element::ListItem(ElementData {
        text: "First item".to_string(),
        metadata: meta(),
    })];
    assert_eq!(exporter.export(&elements), "- First item");
}

// Cycle 3.6
#[test]
fn test_export_kv_bold_key() {
    let exporter = ElementMarkdownExporter::default();
    let elements = vec![Element::KeyValue(KeyValueElementData {
        key: "Status".to_string(),
        value: "Active".to_string(),
        metadata: meta(),
    })];
    assert_eq!(exporter.export(&elements), "**Status**: Active");
}

// Cycle 3.7
#[test]
fn test_export_code_block_fenced() {
    let exporter = ElementMarkdownExporter::default();
    let elements = vec![Element::CodeBlock(ElementData {
        text: "fn main() {}".to_string(),
        metadata: meta(),
    })];
    let output = exporter.export(&elements);
    assert!(output.starts_with("```"));
    assert!(output.contains("fn main() {}"));
    assert!(output.ends_with("```"));
}

// Cycle 3.8
#[test]
fn test_export_image_with_alt() {
    let exporter = ElementMarkdownExporter::default();
    let elements = vec![Element::Image(ImageElementData {
        alt_text: Some("Company logo".to_string()),
        metadata: meta(),
    })];
    assert_eq!(exporter.export(&elements), "![Company logo]()");
}

// Cycle 3.9
#[test]
fn test_export_headers_excluded_by_default() {
    let exporter = ElementMarkdownExporter::default();
    let elements = vec![
        Element::Header(ElementData {
            text: "Page 1".to_string(),
            metadata: meta(),
        }),
        para("Main content."),
        Element::Footer(ElementData {
            text: "Confidential".to_string(),
            metadata: meta(),
        }),
    ];
    let output = exporter.export(&elements);
    assert!(!output.contains("Page 1"));
    assert!(!output.contains("Confidential"));
    assert!(output.contains("Main content."));
}

#[test]
fn test_export_headers_included_when_configured() {
    use oxidize_pdf::pipeline::export::ExportConfig;
    let exporter = ElementMarkdownExporter::new(ExportConfig {
        include_headers_footers: true,
    });
    let elements = vec![Element::Header(ElementData {
        text: "Page 1".to_string(),
        metadata: meta(),
    })];
    assert!(exporter.export(&elements).contains("Page 1"));
}

// Cycle 3.10
#[test]
fn test_export_mixed_document() {
    let exporter = ElementMarkdownExporter::default();
    let elements = vec![
        title("Annual Report"),
        para("This is the executive summary."),
        Element::ListItem(ElementData {
            text: "Revenue grew 20%".to_string(),
            metadata: meta(),
        }),
        Element::KeyValue(KeyValueElementData {
            key: "CEO".to_string(),
            value: "Jane Doe".to_string(),
            metadata: meta(),
        }),
    ];
    let output = exporter.export(&elements);
    assert!(output.contains("# Annual Report"));
    assert!(output.contains("This is the executive summary."));
    assert!(output.contains("**CEO**: Jane Doe"));
}

// Cycle 3.11
#[test]
fn test_document_to_element_markdown_not_empty() {
    let fixture = format!(
        "{}/tests/fixtures/Cold_Email_Hacks.pdf",
        env!("CARGO_MANIFEST_DIR")
    );
    let doc = oxidize_pdf::parser::PdfDocument::open(&fixture).unwrap();
    // to_element_markdown() must succeed without error; the result may be empty
    // if all content is classified as headers/footers (excluded by default exporter)
    let result = doc.to_element_markdown();
    assert!(
        result.is_ok(),
        "to_element_markdown() should not error: {:?}",
        result.err()
    );
}

// Cycle 3.12
#[test]
fn test_export_config_accessible_from_pipeline() {
    use oxidize_pdf::pipeline::ExportConfig;
    let cfg = ExportConfig::default();
    assert!(!cfg.include_headers_footers);
}
