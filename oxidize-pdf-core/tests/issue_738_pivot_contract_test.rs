//! #738: unsupported pivots must never render demonstration data as user results.
use oxidize_pdf::{
    dashboard::{
        AggregateFunction, ComponentPosition, DashboardBuilder, DashboardComponent, DashboardTheme,
        KpiCard, PivotConfig, PivotTable,
    },
    parser::PdfReader,
    Document, Font, Page, PdfError,
};
use std::{collections::HashMap, io::Cursor, process::Command};

fn records(label: &str, value: &str) -> Vec<HashMap<String, String>> {
    vec![([
        ("g".to_string(), label.to_string()),
        ("v".to_string(), value.to_string()),
    ])
    .into_iter()
    .collect()]
}
fn position() -> ComponentPosition {
    ComponentPosition {
        x: 40.,
        y: 200.,
        width: 500.,
        height: 300.,
    }
}
fn pdf(page: Page) -> Vec<u8> {
    let mut doc = Document::new();
    doc.add_page(page);
    doc.to_bytes().unwrap()
}
fn content(page: Page) -> Vec<Vec<u8>> {
    let doc = PdfReader::new(Cursor::new(pdf(page)))
        .unwrap()
        .into_document();
    doc.get_page_content_streams(&doc.get_page(0).unwrap())
        .unwrap()
}
fn assert_unsupported(result: Result<(), PdfError>) {
    match result {
        Err(PdfError::InvalidOperation(message)) => {
            assert!(
                message.contains("PivotTable") && message.contains("not implemented"),
                "{message}"
            );
        }
        other => panic!("expected explicit unsupported pivot error, got {other:?}"),
    }
}
#[test]
fn distinct_data_and_all_aggregation_configs_reject_demo_output() {
    for data in [
        vec![],
        records("UNIQUE_INPUT_X", "1"),
        records("SECOND_INPUT_Y", "913"),
    ] {
        let pivot = PivotTable::new(data.clone());
        assert_unsupported(pivot.render(&mut Page::a4(), position(), &DashboardTheme::default()));
        for aggregation in [
            AggregateFunction::Count,
            AggregateFunction::Sum,
            AggregateFunction::Average,
            AggregateFunction::Min,
            AggregateFunction::Max,
        ] {
            let config = PivotConfig {
                title: Some("ACTUAL_REQUEST".into()),
                row_groups: vec!["g".into()],
                column_groups: vec!["v".into()],
                value_columns: vec!["v".into()],
                aggregations: vec![aggregation],
                show_totals: false,
                show_subtotals: true,
            };
            assert_unsupported(PivotTable::new(data.clone()).with_config(config).render(
                &mut Page::a4(),
                position(),
                &DashboardTheme::default(),
            ));
        }
    }
}
#[test]
fn unsupported_pivot_does_not_modify_existing_page() {
    let mut page = Page::a4();
    page.text()
        .set_font(Font::Helvetica, 12.)
        .at(50., 50.)
        .write("KEEP_EXISTING")
        .unwrap();
    let before = content(page.clone());
    assert_unsupported(PivotTable::new(records("UNIQUE_INPUT_X", "1")).render(
        &mut page,
        position(),
        &DashboardTheme::default(),
    ));
    assert_eq!(content(page), before);
}
#[test]
fn dashboard_propagates_unsupported_pivot_instead_of_reporting_success() {
    let dashboard = DashboardBuilder::new()
        .title("Audit")
        .add_component(Box::new(PivotTable::new(records("UNIQUE_INPUT_X", "1"))))
        .build()
        .unwrap();
    assert_unsupported(dashboard.render_to_page(&mut Page::a4()));
}
#[test]
fn supported_dashboard_still_renders_supplied_labels_and_values() {
    let dashboard = DashboardBuilder::new()
        .title("REAL_REPORT")
        .add_component(Box::new(KpiCard::new("USER_TOTAL", "37")))
        .build()
        .unwrap();
    let mut page = Page::a4();
    dashboard.render_to_page(&mut page).unwrap();
    let doc = PdfReader::new(Cursor::new(pdf(page)))
        .unwrap()
        .into_document();
    let text = doc.extract_text_from_page(0).unwrap().text;
    assert!(
        text.contains("REAL_REPORT") && text.contains("USER_TOTAL") && text.contains("37"),
        "{text}"
    );
    assert!(!text.contains("Group A") && !text.contains("Group B"));
}
fn independent_text(page: Page) -> String {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("pivot-contract.pdf");
    std::fs::write(&file, pdf(page)).unwrap();
    let check = Command::new("qpdf")
        .arg("--check")
        .arg(&file)
        .output()
        .expect("qpdf required");
    assert!(
        check.status.success(),
        "{}",
        String::from_utf8_lossy(&check.stderr)
    );
    let text = Command::new("pdftotext")
        .arg(&file)
        .arg("-")
        .output()
        .expect("Poppler required");
    assert!(text.status.success());
    String::from_utf8(text.stdout).unwrap()
}
#[test]
#[ignore = "requires qpdf and Poppler; run explicitly in interoperability CI"]
fn independent_output_oracle() {
    for (label, total) in [("FIRST_REAL_TOTAL", "37"), ("SECOND_REAL_TOTAL", "913")] {
        let mut page = Page::a4();
        let dashboard = DashboardBuilder::new()
            .title("REAL_REPORT")
            .add_component(Box::new(KpiCard::new(label, total)))
            .build()
            .unwrap();
        dashboard.render_to_page(&mut page).unwrap();
        let before = independent_text(page.clone());
        assert!(before.lines().any(|line| line == label), "{before}");
        assert!(before.lines().any(|line| line == total), "{before}");
        let result = PivotTable::new(records(label, total)).render(
            &mut page,
            position(),
            &DashboardTheme::default(),
        );
        let after = independent_text(page);
        assert!(
            result.is_err(),
            "expected pivot error; independent extraction: {after}"
        );
        assert_unsupported(result);
        assert_eq!(after, before);
        assert!(!before.contains("Group A") && !before.contains("Group B"));
    }
}
