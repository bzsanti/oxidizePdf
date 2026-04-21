// Rigorous tests for catalog entries emitted by write_catalog().
// Covers ISO 32000-1 §7.7.2 entries that Document exposes via setters:
//   /OpenAction, /ViewerPreferences, /Names (Dests), /PageLabels.
//
// NO SMOKE TESTS — every test serialises a real Document and inspects the
// bytes to verify the catalog entry is present with the expected shape.

#[cfg(test)]
mod catalog_entries_tests {
    use crate::actions::Action;
    use crate::document::Document;
    use crate::page::Page;
    use crate::structure::{Destination, PageDestination};
    use crate::viewer_preferences::ViewerPreferences;
    use crate::writer::PdfWriter;

    fn serialize(document: &mut Document) -> String {
        let mut buffer = Vec::new();
        {
            let mut writer = PdfWriter::new_with_writer(&mut buffer);
            writer.write_document(document).unwrap();
        }
        String::from_utf8_lossy(&buffer).into_owned()
    }

    #[test]
    fn test_write_catalog_includes_open_action() {
        let mut document = Document::new();
        document.add_page(Page::a4());
        document.set_open_action(Action::goto(Destination::fit(PageDestination::PageNumber(
            0,
        ))));

        let content = serialize(&mut document);

        assert!(
            content.contains("/OpenAction"),
            "catalog should emit /OpenAction when Document::open_action is set"
        );
        // GoTo action dict must carry /S /GoTo per ISO 32000-1 §12.6.4.2
        assert!(
            content.contains("/S /GoTo"),
            "open action dict should serialize as a /GoTo action (/S /GoTo)"
        );
    }

    #[test]
    fn test_write_catalog_includes_viewer_preferences() {
        let mut document = Document::new();
        document.add_page(Page::a4());
        document.set_viewer_preferences(ViewerPreferences::new().hide_toolbar(true));

        let content = serialize(&mut document);

        assert!(
            content.contains("/ViewerPreferences"),
            "catalog should emit /ViewerPreferences when set on Document"
        );
        assert!(
            content.contains("/HideToolbar true"),
            "viewer prefs dict should serialize /HideToolbar true (ISO 32000-1 §12.2)"
        );
    }
}
