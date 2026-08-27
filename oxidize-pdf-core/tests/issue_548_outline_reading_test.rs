use oxidize_pdf::graphics::Color;
use oxidize_pdf::parser::{OutlineReadOptions, PdfDocument, PdfReader};
use oxidize_pdf::{DestinationType, PageDestination};
use std::io::Cursor;

fn assert_error_contains<T>(result: Result<T, impl std::fmt::Display>, expected: &str) {
    let message = match result {
        Ok(_) => panic!("expected error containing {expected:?}"),
        Err(error) => error.to_string(),
    };
    assert!(
        message.contains(expected),
        "expected {expected:?} in error, got {message:?}"
    );
}

fn pdf(objects: &[&str]) -> Vec<u8> {
    let mut bytes = b"%PDF-1.7\n".to_vec();
    let mut offsets = vec![0usize; objects.len() + 1];
    for (index, object) in objects.iter().enumerate() {
        offsets[index + 1] = bytes.len();
        bytes.extend_from_slice(format!("{} 0 obj\n{object}\nendobj\n", index + 1).as_bytes());
    }
    let xref = bytes.len();
    bytes.extend_from_slice(
        format!("xref\n0 {}\n0000000000 65535 f \n", objects.len() + 1).as_bytes(),
    );
    for offset in offsets.iter().skip(1) {
        bytes.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    bytes.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n",
            objects.len() + 1
        )
        .as_bytes(),
    );
    bytes
}

fn document(bytes: Vec<u8>) -> PdfDocument<Cursor<Vec<u8>>> {
    PdfDocument::new(PdfReader::new(Cursor::new(bytes)).expect("parse fixture"))
}

#[test]
fn reads_complete_hierarchy_and_resolves_direct_legacy_and_name_tree_destinations() {
    let bytes = pdf(&[
        "<< /Type /Catalog /Pages 2 0 R /Outlines 8 0 R /Dests << /Legacy [5 0 R /FitH 700] >> /Names << /Dests 14 0 R >> >>",
        "<< /Type /Pages /Count 2 /Kids [3 0 R 4 0 R] >>",
        "<< /Type /Pages /Count 1 /Kids [5 0 R] /Parent 2 0 R >>",
        "<< /Type /Pages /Count 1 /Kids [6 0 R] /Parent 2 0 R >>",
        "<< /Type /Page /Parent 3 0 R /MediaBox [0 0 612 792] >>",
        "<< /Type /Page /Parent 4 0 R /MediaBox [0 0 612 792] >>",
        "null",
        "<< /Type /Outlines /First 9 0 R /Last 11 0 R /Count 3 >>",
        "<< /Title (Direct) /Parent 8 0 R /Next 10 0 R /Dest [5 0 R /XYZ 10 20 1] /C [1 0.5 0] /F 3 >>",
        "<< /Title (Legacy) /Parent 8 0 R /Prev 9 0 R /Next 11 0 R /Dest /Legacy /First 12 0 R /Last 12 0 R /Count -1 >>",
        "<< /Title (Action) /Parent 8 0 R /Prev 10 0 R /A << /S /GoTo /D (Modern) >> >>",
        "<< /Title (Nested) /Parent 10 0 R /Dest [6 0 R /FitR 0 0 100 200] >>",
        "null",
        "<< /Names [(Modern) [6 0 R /FitBV null]] >>",
    ]);
    let outline = document(bytes)
        .outline()
        .expect("read outline")
        .expect("outline");
    assert_eq!(outline.items.len(), 3);
    assert_eq!(outline.items[0].title, "Direct");
    assert_eq!(outline.items[0].color, Some(Color::Rgb(1.0, 0.5, 0.0)));
    assert!(outline.items[0].flags.bold && outline.items[0].flags.italic);
    assert_eq!(
        outline.items[0].destination.as_ref().unwrap().page,
        PageDestination::PageNumber(0)
    );
    assert!(matches!(
        outline.items[0].destination.as_ref().unwrap().dest_type,
        DestinationType::XYZ {
            left: Some(10.0),
            top: Some(20.0),
            zoom: Some(1.0)
        }
    ));
    assert!(!outline.items[1].open);
    assert_eq!(outline.items[1].children[0].title, "Nested");
    assert!(matches!(
        outline.items[1].destination.as_ref().unwrap().dest_type,
        DestinationType::FitH { top: Some(700.0) }
    ));
    assert_eq!(
        outline.items[1].children[0]
            .destination
            .as_ref()
            .unwrap()
            .page,
        PageDestination::PageNumber(1)
    );
    assert!(matches!(
        outline.items[2].destination.as_ref().unwrap().dest_type,
        DestinationType::FitBV { left: None }
    ));
}

#[test]
fn absence_is_not_an_error_and_parsing_is_repeatable() {
    let bytes = pdf(&[
        "<< /Type /Catalog /Pages 2 0 R >>",
        "<< /Type /Pages /Count 1 /Kids [3 0 R] >>",
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] >>",
    ]);
    let document = document(bytes);
    assert_eq!(document.outline().expect("first read"), None);
    assert_eq!(document.outline().expect("second read"), None);
}

#[test]
fn rejects_cycles_broken_links_and_resource_limit_exhaustion() {
    let cycle = pdf(&[
        "<< /Type /Catalog /Pages 2 0 R /Outlines 4 0 R >>",
        "<< /Type /Pages /Count 1 /Kids [3 0 R] >>",
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] >>",
        "<< /Type /Outlines /First 5 0 R /Last 5 0 R >>",
        "<< /Title (Cycle) /Parent 4 0 R /Next 5 0 R >>",
    ]);
    assert_error_contains(document(cycle).outline(), "cycle or duplicate");

    let broken = pdf(&[
        "<< /Type /Catalog /Pages 2 0 R /Outlines 4 0 R >>",
        "<< /Type /Pages /Count 1 /Kids [3 0 R] >>",
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] >>",
        "<< /Type /Outlines /First 5 0 R /Last 6 0 R >>",
        "<< /Title (One) /Parent 4 0 R /Next 6 0 R >>",
        "<< /Title (Two) /Parent 4 0 R >>",
    ]);
    assert_error_contains(document(broken).outline(), "Prev link");

    let limited = pdf(&[
        "<< /Type /Catalog /Pages 2 0 R /Outlines 4 0 R >>",
        "<< /Type /Pages /Count 1 /Kids [3 0 R] >>",
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] >>",
        "<< /Type /Outlines /First 5 0 R /Last 5 0 R >>",
        "<< /Title (One) /Parent 4 0 R >>",
    ]);
    let options = OutlineReadOptions {
        max_items: 0,
        ..Default::default()
    };
    assert_error_contains(
        document(limited).outline_with_options(&options),
        "item count exceeds",
    );
}

#[test]
fn rejects_destinations_to_objects_outside_the_nested_page_tree() {
    let bytes = pdf(&[
        "<< /Type /Catalog /Pages 2 0 R /Outlines 5 0 R >>",
        "<< /Type /Pages /Count 1 /Kids [3 0 R] >>",
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] >>",
        "<< /Type /Page /MediaBox [0 0 10 10] >>",
        "<< /Type /Outlines /First 6 0 R /Last 6 0 R >>",
        "<< /Title (Wrong page) /Parent 5 0 R /Dest [4 0 R /Fit] >>",
    ]);
    assert_error_contains(
        document(bytes).outline(),
        "page reference is not in the page tree",
    );
}

#[test]
fn rejects_cycles_between_named_destinations() {
    let bytes = pdf(&[
        "<< /Type /Catalog /Pages 2 0 R /Outlines 4 0 R /Dests << /A /B /B /A >> >>",
        "<< /Type /Pages /Count 1 /Kids [3 0 R] >>",
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] >>",
        "<< /Type /Outlines /First 5 0 R /Last 5 0 R >>",
        "<< /Title (Named cycle) /Parent 4 0 R /Dest /A >>",
    ]);
    assert_error_contains(
        document(bytes).outline(),
        "named destinations contain a cycle",
    );
}

#[test]
fn bounds_empty_name_tree_nodes_independently_from_destination_count() {
    let bytes = pdf(&[
        "<< /Type /Catalog /Pages 2 0 R /Outlines 4 0 R /Names << /Dests 6 0 R >> >>",
        "<< /Type /Pages /Count 1 /Kids [3 0 R] >>",
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] >>",
        "<< /Type /Outlines /First 5 0 R /Last 5 0 R >>",
        "<< /Title (No destination needed) /Parent 4 0 R >>",
        "<< /Kids [7 0 R] >>",
        "<< /Names [] >>",
    ]);
    let options = OutlineReadOptions {
        max_name_tree_nodes: 1,
        ..Default::default()
    };
    assert_error_contains(
        document(bytes).outline_with_options(&options),
        "name-tree nodes exceed",
    );
}

#[test]
fn absence_does_not_parse_a_broken_page_tree() {
    let bytes = pdf(&["<< /Type /Catalog /Pages 99 0 R >>"]);
    assert_eq!(document(bytes).outline().expect("outline absence"), None);
}

#[test]
fn resolves_indirect_title_flags_color_and_open_state() {
    let bytes = pdf(&[
        "<< /Type /Catalog /Pages 2 0 R /Outlines 4 0 R >>",
        "<< /Type /Pages /Count 1 /Kids [3 0 R] >>",
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] >>",
        "<< /Type /Outlines /First 5 0 R /Last 5 0 R >>",
        "<< /Title 6 0 R /Parent 4 0 R /F 7 0 R /C 8 0 R /Count 9 0 R /First 10 0 R /Last 10 0 R >>",
        "(Indirect title)",
        "3",
        "[0.25 0.5 0.75]",
        "-1",
        "<< /Title (Child) /Parent 5 0 R >>",
    ]);
    let outline = document(bytes)
        .outline()
        .expect("read outline")
        .expect("outline");
    let item = &outline.items[0];
    assert_eq!(item.title, "Indirect title");
    assert!(item.flags.bold && item.flags.italic);
    assert_eq!(item.color, Some(Color::Rgb(0.25, 0.5, 0.75)));
    assert!(!item.open);
    assert_eq!(item.children.len(), 1);
}

#[test]
fn rejects_overlapping_name_tree_child_ranges() {
    let bytes = pdf(&[
        "<< /Type /Catalog /Pages 2 0 R /Outlines 4 0 R /Names << /Dests 6 0 R >> >>",
        "<< /Type /Pages /Count 1 /Kids [3 0 R] >>",
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] >>",
        "<< /Type /Outlines /First 5 0 R /Last 5 0 R >>",
        "<< /Title (Item) /Parent 4 0 R >>",
        "<< /Kids [7 0 R 8 0 R] /Limits [(L) (M)] >>",
        "<< /Names [(M) [3 0 R /Fit]] /Limits [(M) (M)] >>",
        "<< /Names [(L) [3 0 R /Fit]] /Limits [(L) (L)] >>",
    ]);
    assert_error_contains(
        document(bytes).outline(),
        "overlap or are not strictly increasing",
    );
}
