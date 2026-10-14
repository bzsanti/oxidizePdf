//! Public-API contracts derived from #737 and the independent two-page fixture.
use oxidize_pdf::operations::{
    RedactionConfig, RedactionMode, RedactionStyle, SemanticRedactor, SemanticRedactorError,
};
use oxidize_pdf::parser::content::{ContentOperation, ContentParser};
use oxidize_pdf::parser::PdfReader;
use oxidize_pdf::semantic::{BoundingBox, EntityType, SemanticEntity};
use std::io::Cursor;

const PDF: &[u8] = include_bytes!("fixtures/issue_737_two_pages.pdf");

fn entity(id: &str, page: u32, kind: EntityType) -> SemanticEntity {
    SemanticEntity::new(
        id.into(),
        kind,
        BoundingBox::new(72.0, 695.0, 120.0, 20.0, page),
    )
}

fn config(style: RedactionStyle) -> RedactionConfig {
    RedactionConfig::new()
        .with_types(vec![EntityType::Text])
        .with_style(style)
}

fn assert_page_error(entities: &[SemanticEntity], page: u32, style: RedactionStyle) {
    let error = SemanticRedactor::redact(PDF, entities, config(style))
        .expect_err("every selected invalid page must reject the entire request");
    let SemanticRedactorError::PageReconstructionFailed(reason) = error else {
        panic!("expected a page error, got {error}");
    };
    assert!(reason.contains(&format!("page {page}")), "{reason}");
    assert!(
        reason.contains("2 pages"),
        "must explain the document's actual range: {reason}"
    );
}

#[test]
fn selected_page_zero_is_rejected() {
    assert_page_error(
        &[entity("zero", 0, EntityType::Text)],
        0,
        RedactionStyle::BlackBox,
    );
}

#[test]
fn selected_page_beyond_document_is_rejected() {
    for page in [3, u32::MAX] {
        assert_page_error(
            &[entity("outside", page, EntityType::Text)],
            page,
            RedactionStyle::BlackBox,
        );
    }
}

#[test]
fn mixed_valid_and_invalid_requests_never_return_partial_success() {
    for page in [0, 3] {
        for style in [
            RedactionStyle::BlackBox,
            RedactionStyle::Placeholder("MASKED".into()),
        ] {
            let mut entities = vec![
                entity("valid", 1, EntityType::Text),
                entity("invalid", page, EntityType::Text),
            ];
            assert_page_error(&entities, page, style.clone());
            entities.reverse();
            assert_page_error(&entities, page, style);
        }
    }
}

fn assert_masked_pages(output: &[u8], masked: &[u32]) {
    let document = PdfReader::new(Cursor::new(output)).unwrap().into_document();
    assert_eq!(document.page_count().unwrap(), 2);
    for (idx, original) in [b"FIRST SECRET".as_slice(), b"SECOND SECRET".as_slice()]
        .iter()
        .enumerate()
    {
        let page = document.get_page(idx as u32).unwrap();
        let streams = document.get_page_content_streams(&page).unwrap();
        let operations: Vec<_> = streams
            .iter()
            .flat_map(|s| ContentParser::parse_strict(s).unwrap())
            .collect();
        let rectangles: Vec<_> = operations
            .iter()
            .filter_map(|op| match op {
                ContentOperation::Rectangle(x, y, w, h) => Some((*x, *y, *w, *h)),
                _ => None,
            })
            .collect();
        let expected = if masked.contains(&(idx as u32 + 1)) {
            vec![(72.0, 695.0, 120.0, 20.0)]
        } else {
            vec![]
        };
        assert_eq!(rectangles, expected, "mask must be on the requested page");
        assert!(
            streams
                .iter()
                .any(|s| s.windows(original.len()).any(|w| w == *original)),
            "visual masks leave source content recoverable"
        );
    }
}

#[test]
fn first_and_last_pages_are_masked_and_reported_with_visual_risks() {
    // Reversed input order prevents input ordering from masquerading as page identity.
    let entities = [
        entity("last", 2, EntityType::Text),
        entity("first", 1, EntityType::Text),
    ];
    for style in [
        RedactionStyle::BlackBox,
        RedactionStyle::Placeholder("MASKED".into()),
    ] {
        let (output, report) = SemanticRedactor::redact(PDF, &entities, config(style)).unwrap();
        let entries: Vec<_> = report
            .entries()
            .iter()
            .map(|e| (e.entity_id.as_str(), e.page, e.entity_type.clone()))
            .collect();
        assert_eq!(
            entries,
            vec![
                ("first", 1, EntityType::Text),
                ("last", 2, EntityType::Text)
            ]
        );
        assert_eq!(report.mode(), RedactionMode::VisualMask);
        assert!(!report.is_irreversible());
        assert!(report
            .residual_risks()
            .iter()
            .any(|r| r.contains("underlying page content remains extractable")));
        assert_masked_pages(&output, &[1, 2]);
    }
}

#[test]
fn unselected_invalid_pages_do_not_reject_or_mask_selected_valid_pages() {
    let entities = [
        entity("ignored-zero", 0, EntityType::Image),
        entity("last", 2, EntityType::Text),
        entity("ignored-outside", 3, EntityType::Image),
    ];
    let (output, report) =
        SemanticRedactor::redact(PDF, &entities, config(RedactionStyle::BlackBox)).unwrap();
    assert_eq!(report.entries().len(), 1);
    assert_eq!(report.entries()[0].entity_id, "last");
    assert_eq!(report.entries()[0].page, 2);
    assert_masked_pages(&output, &[2]);
}

#[test]
fn no_selected_entities_preserves_exact_bytes_and_visual_warning() {
    let entities = [
        entity("outside", 3, EntityType::Image),
        entity("zero", 0, EntityType::Image),
    ];
    for cfg in [RedactionConfig::new(), config(RedactionStyle::BlackBox)] {
        let (output, report) = SemanticRedactor::redact(PDF, &entities, cfg).unwrap();
        assert_eq!(output, PDF);
        assert!(report.entries().is_empty());
        assert_eq!(report.mode(), RedactionMode::VisualMask);
        assert!(!report.is_irreversible());
        assert!(!report.residual_risks().is_empty());
    }
}
