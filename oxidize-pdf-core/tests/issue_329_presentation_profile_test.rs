//! Acceptance tests for issue #329: `ExtractionProfile::Presentation` must
//! suppress spatial-cluster table false positives on slide-shape grids.
//!
//! Fixture: `tests/fixtures/synthetic_slide_deck.pdf` (4 pages, 16:9 deck).
//! Page layout (0-indexed, as returned by `ElementMetadata.page`):
//!   page 0 — cover slide (title + subtitle)
//!   page 1 — 4-column "Origin/Challenge/Solution/Impact" grid (failure reproducer)
//!   page 2 — prose summary slide (control: must not regress)
//!   page 3 — closing slide (control: must not silent-drop)
//!
//! NOTE: the body of issue #329 cites pages as 1/2/3 (1-indexed), but the
//! generator emits the 4-column layout as `pages[1]` and the closing slide as
//! `pages[3]`. This test uses the 0-indexed API values.

use oxidize_pdf::parser::PdfDocument;
use oxidize_pdf::pipeline::{Element, ExtractionProfile};

const FIXTURE: &str = "tests/fixtures/synthetic_slide_deck.pdf";

fn partition_presentation() -> Vec<Element> {
    let path = format!("{}/{}", env!("CARGO_MANIFEST_DIR"), FIXTURE);
    let doc = PdfDocument::open(&path).expect("open synthetic slide deck fixture");
    doc.partition_with_profile(ExtractionProfile::Presentation)
        .expect("partition_with_profile(Presentation) succeeds")
}

fn on_page(elements: &[Element], page: u32) -> Vec<&Element> {
    elements.iter().filter(|e| e.page() == page).collect()
}

/// Primary RED contract from #329: the 4-column slide on page 1 must NOT be
/// classified as a single `Table` element. The `Presentation` profile should
/// produce a structured set of `Title`/`Paragraph` elements instead.
#[test]
fn presentation_profile_does_not_collapse_4col_slide_to_table() {
    let elements = partition_presentation();
    let page1 = on_page(&elements, 1);

    let tables: Vec<&&Element> = page1
        .iter()
        .filter(|e| matches!(e, Element::Table(_)))
        .collect();
    assert!(
        tables.is_empty(),
        "page 1 (4-column slide) must not be classified as a Table — \
         found {} table element(s): {:?}",
        tables.len(),
        tables.iter().map(|e| e.display_text()).collect::<Vec<_>>()
    );

    let text_elements: Vec<&&Element> = page1
        .iter()
        .filter(|e| matches!(e, Element::Title(_) | Element::Paragraph(_)))
        .collect();
    assert!(
        text_elements.len() >= 10,
        "page 1 must produce >= 10 Title/Paragraph elements (4 headers + 12 cells \
         + slide title/subtitle) — got {} ({} total elements on page): {:?}",
        text_elements.len(),
        page1.len(),
        page1.iter().map(|e| e.type_name()).collect::<Vec<_>>()
    );
}

/// Acceptance #2 from #329: the closing slide (page 3) carries a large title
/// and a short subtitle. The Presentation profile must not silent-drop it.
#[test]
fn presentation_profile_does_not_drop_closing_slide() {
    let elements = partition_presentation();
    let page3 = on_page(&elements, 3);
    assert!(
        !page3.is_empty(),
        "page 3 (closing slide) must produce >= 1 element — got {} (silent-drop)",
        page3.len()
    );
    let has_closing_text = page3
        .iter()
        .any(|e| e.display_text().contains("Closing Slide"));
    assert!(
        has_closing_text,
        "page 3 must surface the 'Closing Slide' title text — elements: {:?}",
        page3
            .iter()
            .map(|e| (e.type_name(), e.display_text()))
            .collect::<Vec<_>>()
    );
}

/// Acceptance #3 from #329: control — the prose summary slide (page 2) must
/// not regress under the Presentation profile. The fix should only suppress
/// false-positive table classification; it must not strip prose pages of all
/// elements or reclassify continuous prose as a table.
///
/// Scope kept narrow on purpose: the title-vs-header classification on the
/// prose slide is an adjacent concern (header_zone = 0.10 captures the slide
/// title) and out of scope for #329 — see the layout note at the top of file.
#[test]
fn presentation_profile_preserves_prose_summary_slide() {
    let elements = partition_presentation();
    let page2 = on_page(&elements, 2);

    let tables: Vec<&&Element> = page2
        .iter()
        .filter(|e| matches!(e, Element::Table(_)))
        .collect();
    assert!(
        tables.is_empty(),
        "page 2 (prose summary, control) must not classify continuous prose \
         as a Table — found {} table element(s)",
        tables.len()
    );

    let paragraphs = page2
        .iter()
        .filter(|e| matches!(e, Element::Paragraph(_)))
        .count();
    assert!(
        paragraphs >= 1,
        "page 2 (prose) must produce >= 1 Paragraph (structure preserved) — \
         got {} total elements ({:?})",
        page2.len(),
        page2.iter().map(|e| e.type_name()).collect::<Vec<_>>()
    );
}
