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
    use crate::page_labels::{PageLabel, PageLabelStyle, PageLabelTree};
    use crate::structure::{Destination, NamedDestinations, PageDestination};
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

    #[test]
    fn test_write_catalog_includes_named_destinations() {
        let mut document = Document::new();
        document.add_page(Page::a4());
        let mut dests = NamedDestinations::new();
        dests.add_destination(
            "target".to_string(),
            Destination::fit(PageDestination::PageNumber(0)).to_array(),
        );
        document.set_named_destinations(dests);

        let content = serialize(&mut document);

        // /Names in the catalog is a Name Dictionary (ISO 32000-1 §7.7.4,
        // Table 31). Its /Dests entry is the name tree for named
        // destinations (§12.3.2.3).
        assert!(
            content.contains("/Names"),
            "catalog should emit /Names when named destinations are set"
        );
        assert!(
            content.contains("/Dests"),
            "/Names dictionary must contain /Dests for named destinations"
        );
        // The name tree leaf exposes the entry key as a string literal.
        assert!(
            content.contains("(target)"),
            "named destination key should appear as a string in the name tree \
             (expected (target))"
        );
    }

    #[test]
    fn test_write_catalog_includes_all_four_entries_simultaneously() {
        // Regression: the four catalog entries are emitted as independent
        // `if let` blocks, so a bug in one could suppress another. This
        // test exercises the full cross-product by enabling all four
        // features on the same Document and asserts each entry — and its
        // characteristic payload — survives serialisation.
        let mut document = Document::new();
        document.add_page(Page::a4());

        document.set_open_action(Action::goto(Destination::fit(PageDestination::PageNumber(
            0,
        ))));
        document.set_viewer_preferences(ViewerPreferences::new().hide_toolbar(true));
        let mut dests = NamedDestinations::new();
        dests.add_destination(
            "combined-target".to_string(),
            Destination::fit(PageDestination::PageNumber(0)).to_array(),
        );
        document.set_named_destinations(dests);
        let mut labels = PageLabelTree::new();
        labels.add_range(0, PageLabel::new(PageLabelStyle::DecimalArabic));
        document.set_page_labels(labels);

        let content = serialize(&mut document);

        // OpenAction
        assert!(content.contains("/OpenAction"));
        assert!(content.contains("/S /GoTo"));
        // ViewerPreferences
        assert!(content.contains("/ViewerPreferences"));
        assert!(content.contains("/HideToolbar true"));
        // Names (named destinations)
        assert!(content.contains("/Names"));
        assert!(content.contains("/Dests"));
        assert!(content.contains("(combined-target)"));
        // PageLabels
        assert!(content.contains("/PageLabels"));
        assert!(content.contains("/S /D"));
    }

    #[test]
    fn test_write_catalog_includes_page_labels() {
        let mut document = Document::new();
        document.add_page(Page::a4());
        let mut labels = PageLabelTree::new();
        labels.add_range(0, PageLabel::new(PageLabelStyle::DecimalArabic));
        document.set_page_labels(labels);

        let content = serialize(&mut document);

        // /PageLabels in the catalog is a number tree (ISO 32000-1 §7.7.2
        // Table 28, §12.4.2).
        assert!(
            content.contains("/PageLabels"),
            "catalog should emit /PageLabels when set on Document"
        );
        // Per ISO 32000-1 §12.4.2 Table 159 the numbering style entry is
        // named /S (not /Type).
        assert!(
            content.contains("/S /D"),
            "decimal page label dict should serialise as /S /D per spec"
        );
    }

    #[test]
    fn test_page_labels_uppercase_roman_emits_uppercase_s_name() {
        // Per ISO 32000-1 §12.4.2 Table 159, /R means "uppercase Roman
        // numerals" and /r means "lowercase Roman numerals". A
        // spec-conforming viewer uses the /S name to decide the case of
        // the rendered numeral, so the writer must emit /S /R for
        // PageLabelStyle::UppercaseRoman — anything else makes the
        // viewer render the opposite case of what the Rust API promised.
        let mut document = Document::new();
        document.add_page(Page::a4());
        let mut labels = PageLabelTree::new();
        labels.add_range(0, PageLabel::roman_uppercase());
        document.set_page_labels(labels);

        let content = serialize(&mut document);

        assert!(
            content.contains("/S /R"),
            "UppercaseRoman must serialise as /S /R per ISO 32000-1 §12.4.2 \
             Table 159; PDF never emitted /S /R"
        );
        assert!(
            !content.contains("/S /r"),
            "UppercaseRoman must NOT emit /S /r (that is the lowercase style); \
             /S /r found in output for a label built from roman_uppercase()"
        );
    }

    #[test]
    fn test_page_labels_lowercase_roman_emits_lowercase_s_name() {
        // Mirror of the uppercase test: /S /r is the lowercase form per
        // ISO 32000-1 §12.4.2 Table 159.
        let mut document = Document::new();
        document.add_page(Page::a4());
        let mut labels = PageLabelTree::new();
        labels.add_range(0, PageLabel::roman_lowercase());
        document.set_page_labels(labels);

        let content = serialize(&mut document);

        assert!(
            content.contains("/S /r"),
            "LowercaseRoman must serialise as /S /r per ISO 32000-1 §12.4.2 \
             Table 159; PDF never emitted /S /r"
        );
        assert!(
            !content.contains("/S /R"),
            "LowercaseRoman must NOT emit /S /R (that is the uppercase style); \
             /S /R found in output for a label built from roman_lowercase()"
        );
    }

    #[test]
    fn test_page_labels_mixed_roman_ranges_keep_correct_case() {
        // Regression: a document that uses both roman cases in separate
        // ranges must emit /S /R and /S /r side by side. The v2.5.5 bug
        // made this doubly visible: a user who explicitly wanted front
        // matter in uppercase and appendix in lowercase would get the
        // opposite in a spec-conforming viewer.
        let mut document = Document::new();
        document.add_page(Page::a4());
        document.add_page(Page::a4());
        let mut labels = PageLabelTree::new();
        labels.add_range(0, PageLabel::roman_uppercase());
        labels.add_range(1, PageLabel::roman_lowercase());
        document.set_page_labels(labels);

        let content = serialize(&mut document);

        assert!(
            content.contains("/S /R"),
            "mixed-case roman ranges must emit /S /R for the uppercase range"
        );
        assert!(
            content.contains("/S /r"),
            "mixed-case roman ranges must emit /S /r for the lowercase range"
        );
    }
}
