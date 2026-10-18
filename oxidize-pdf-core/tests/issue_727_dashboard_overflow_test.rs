//! Public contracts from #727: successful default rendering must not omit components.
use oxidize_pdf::{
    dashboard::{layout::GridSystem, ComponentSpan, Dashboard, DashboardBuilder, KpiCard},
    Page,
};

fn dashboard(count: usize) -> Dashboard {
    let mut builder = DashboardBuilder::new().title("audit");
    for i in 0..count {
        builder = builder.add_component(Box::new(KpiCard::new(format!("SENTINEL{i}X"), "1")));
    }
    builder.build().unwrap()
}

#[test]
fn default_overflow_is_an_error_with_remaining_identities() {
    let error = dashboard(10)
        .render_to_page(&mut Page::a4())
        .expect_err("must not silently omit seven cards");
    assert!(error.to_string().contains("3..10"), "{error}");
}

#[test]
fn exact_fit_and_first_overflow() {
    let d = dashboard(1);
    let grid = GridSystem::new(12, 0., 20.);
    assert_eq!(
        grid.layout_components(&d.components, 0., 120., 600., 120., 120.)
            .unwrap()
            .len(),
        1
    );
    assert!(grid
        .layout_components(&d.components, 0., 120., 600., 119.99, 120.)
        .is_err());
}

#[test]
fn multiple_rows_require_space_for_every_component() {
    let mut d = dashboard(4);
    for c in &mut d.components {
        c.set_span(ComponentSpan::new(6));
    }
    let grid = GridSystem::new(12, 0., 20.);
    assert_eq!(
        grid.layout_components(&d.components, 0., 260., 600., 260., 120.)
            .unwrap()
            .len(),
        4
    );
    let error = grid
        .layout_components(&d.components, 0., 260., 600., 259.99, 120.)
        .unwrap_err();
    assert!(error.to_string().contains("2..4"), "{error}");
}

#[test]
fn non_finite_bounds_cannot_bypass_overflow() {
    let d = dashboard(10);
    assert!(GridSystem::new(12, 0., 20.)
        .layout_components(&d.components, 0., 120., 600., f64::NAN, 120.)
        .is_err());
}

use oxidize_pdf::{dashboard::DashboardOverflowPolicy, parser::PdfReader, Document, Font};
use std::{io::Cursor, process::Command};

fn pdf(page: Page) -> Vec<u8> {
    let mut doc = Document::new();
    doc.add_page(page);
    doc.to_bytes().unwrap()
}

fn content(page: Page) -> Vec<Vec<u8>> {
    let parsed = PdfReader::new(Cursor::new(pdf(page)))
        .unwrap()
        .into_document();
    parsed
        .get_page_content_streams(&parsed.get_page(0).unwrap())
        .unwrap()
}

#[test]
fn overflow_leaves_existing_page_unchanged() {
    let mut page = Page::a4();
    page.text()
        .set_font(Font::Helvetica, 12.)
        .at(50., 50.)
        .write("EXISTING")
        .unwrap();
    let before = content(page.clone());
    assert!(dashboard(10).render_to_page(&mut page).is_err());
    assert_eq!(
        content(page),
        before,
        "even the dashboard header must not be added"
    );
}

#[test]
fn omission_is_explicit_and_identifies_the_entire_remainder() {
    let report = dashboard(10)
        .render_to_page_with_overflow(&mut Page::a4(), DashboardOverflowPolicy::Omit)
        .unwrap();
    assert_eq!(report.rendered_indices, vec![0, 1, 2]);
    assert_eq!(report.omitted_indices, vec![3, 4, 5, 6, 7, 8, 9]);
}

#[test]
fn first_row_overflow_and_empty_dashboard() {
    let mut page = Page::new(595., 300.);
    let error = dashboard(2).render_to_page(&mut page).unwrap_err();
    assert!(error.to_string().contains("0..2"), "{error}");
    let report = dashboard(2)
        .render_to_page_with_overflow(&mut page, DashboardOverflowPolicy::Omit)
        .unwrap();
    assert_eq!(report.rendered_indices, Vec::<usize>::new());
    assert_eq!(report.omitted_indices, vec![0, 1]);
    let report = dashboard(0)
        .render_to_page_with_overflow(&mut Page::a4(), DashboardOverflowPolicy::Error)
        .unwrap();
    assert_eq!(report.rendered_indices, Vec::<usize>::new());
    assert_eq!(report.omitted_indices, Vec::<usize>::new());
}

#[test]
fn larger_page_renders_all_indices_and_duplicate_labels_remain_distinct() {
    let report = dashboard(10)
        .render_to_page_with_overflow(&mut Page::new(595., 2200.), DashboardOverflowPolicy::Error)
        .unwrap();
    assert_eq!(report.rendered_indices, (0..10).collect::<Vec<_>>());
    assert!(report.omitted_indices.is_empty());
    let mut d = dashboard(1);
    d.components = vec![d.components[0].clone(); 5];
    let report = d
        .render_to_page_with_overflow(&mut Page::a4(), DashboardOverflowPolicy::Omit)
        .unwrap();
    assert_eq!(report.rendered_indices, vec![0, 1, 2]);
    assert_eq!(report.omitted_indices, vec![3, 4]);
}

#[test]
fn invalid_grid_geometry_and_spans_are_errors() {
    let mut d = dashboard(1);
    for grid in [
        GridSystem::new(0, 0., 0.),
        GridSystem::new(12, -1., 0.),
        GridSystem::new(12, 0., f64::INFINITY),
        GridSystem::new(12, 100., 0.),
    ] {
        assert!(grid
            .layout_components(&d.components, 0., 120., 600., 120., 120.)
            .is_err());
    }
    d.components[0].set_span(ComponentSpan {
        columns: 0,
        rows: None,
    });
    assert!(GridSystem::new(12, 0., 0.)
        .layout_components(&d.components, 0., 120., 600., 120., 120.)
        .is_err());
    d.components[0].set_span(ComponentSpan {
        columns: 255,
        rows: None,
    });
    assert!(GridSystem::new(12, 0., 0.)
        .layout_components(&d.components, 0., 120., 600., 120., 120.)
        .is_err());
}

fn independent_text(page: Page) -> String {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("dashboard.pdf");
    std::fs::write(&file, pdf(page)).unwrap();
    let check = Command::new("qpdf")
        .arg("--check")
        .arg(&file)
        .output()
        .expect("qpdf is required");
    assert!(
        check.status.success(),
        "qpdf: {} {}",
        String::from_utf8_lossy(&check.stdout),
        String::from_utf8_lossy(&check.stderr)
    );
    let text = Command::new("pdftotext")
        .arg(&file)
        .arg("-")
        .output()
        .expect("Poppler is required");
    assert!(
        text.status.success(),
        "{}",
        String::from_utf8_lossy(&text.stderr)
    );
    String::from_utf8(text.stdout).unwrap()
}

#[test]
#[ignore = "requires qpdf and Poppler; explicitly run in interoperability CI"]
fn independent_output_oracle() {
    let d = dashboard(10);
    let mut full = Page::new(595., 2200.);
    d.render_to_page(&mut full).unwrap();
    let text = independent_text(full);
    for i in 0..10 {
        assert_eq!(
            text.matches(&format!("SENTINEL{i}X")).count(),
            1,
            "missing or duplicated marker {i}: {text}"
        );
    }
    let mut partial = Page::a4();
    let report = d
        .render_to_page_with_overflow(&mut partial, DashboardOverflowPolicy::Omit)
        .unwrap();
    let text = independent_text(partial);
    for i in 0..10 {
        let expected = usize::from(i < 3);
        assert_eq!(
            text.matches(&format!("SENTINEL{i}X")).count(),
            expected,
            "unexpected marker identity {i}: {text}"
        );
        assert_eq!(report.omitted_indices.contains(&i), i >= 3);
    }
    let mut original = Page::a4();
    original
        .text()
        .set_font(Font::Helvetica, 12.)
        .at(50., 50.)
        .write("EXISTING")
        .unwrap();
    assert!(d.render_to_page(&mut original).is_err());
    let text = independent_text(original);
    assert_eq!(text.trim(), "EXISTING");
}
