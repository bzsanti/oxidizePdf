//! ISO 32000-1 12.4.2: labels use repeated letters, not spreadsheet columns.
use oxidize_pdf::page_labels::{PageLabel, PageLabelStyle, PageLabelTree};

#[test]
fn repeated_letter_boundaries_and_prefix_offsets() {
    for (number, upper, lower) in [
        (1, "A", "a"),
        (26, "Z", "z"),
        (27, "AA", "aa"),
        (28, "BB", "bb"),
        (52, "ZZ", "zz"),
        (53, "AAA", "aaa"),
        (78, "ZZZ", "zzz"),
    ] {
        assert_eq!(PageLabelStyle::UppercaseLetters.format(number), upper);
        assert_eq!(PageLabelStyle::LowercaseLetters.format(number), lower);
    }
    let mut tree = PageLabelTree::new();
    tree.add_range(
        0,
        PageLabel::letters_uppercase()
            .with_prefix("App-")
            .starting_at(26),
    );
    tree.add_range(
        3,
        PageLabel::letters_lowercase()
            .with_prefix("ann-")
            .starting_at(52),
    );
    assert_eq!(
        tree.get_all_labels(5),
        ["App-Z", "App-AA", "App-BB", "ann-zz", "ann-aaa"]
    );
    assert_eq!(
        PageLabelTree::from_dict(&tree.to_dict())
            .unwrap()
            .get_all_labels(5),
        tree.get_all_labels(5)
    );
}

// The emitted artifact can also be checked with scripts/check_issue_743_labels.py.
#[test]
fn emitted_pdf_contains_the_same_label_ranges() {
    use oxidize_pdf::{parser::PdfReader, writer::PdfWriter, Document, Page};
    let mut tree = PageLabelTree::new();
    tree.add_range(
        0,
        PageLabel::letters_uppercase()
            .with_prefix("App-")
            .starting_at(26),
    );
    tree.add_range(
        3,
        PageLabel::letters_lowercase()
            .with_prefix("ann-")
            .starting_at(52),
    );
    let mut document = Document::new();
    for _ in 0..5 {
        document.add_page(Page::a4());
    }
    document.set_page_labels(tree);
    let mut buffer = Vec::new();
    PdfWriter::new_with_writer(&mut buffer)
        .write_document(&mut document)
        .unwrap();
    let mut reader = PdfReader::new(std::io::Cursor::new(buffer.clone())).unwrap();
    let catalog = reader.catalog().unwrap().clone();
    let labels = catalog.get("PageLabels").unwrap();
    let labels = reader.resolve(labels).unwrap();
    let nums = labels
        .as_dict()
        .unwrap()
        .get("Nums")
        .unwrap()
        .as_array()
        .unwrap();
    assert_eq!(nums.len(), 4);
    assert_eq!(nums.get(0).unwrap().as_integer(), Some(0));
    assert_eq!(nums.get(2).unwrap().as_integer(), Some(3));
    for (index, style, prefix, start) in [
        (1, "A", b"App-".as_slice(), 26),
        (3, "a", b"ann-".as_slice(), 52),
    ] {
        let dict = nums.get(index).unwrap().as_dict().unwrap();
        assert_eq!(
            dict.get("S").unwrap().as_name().map(|n| n.as_str()),
            Some(style)
        );
        assert_eq!(dict.get("St").unwrap().as_integer(), Some(start));
        assert_eq!(
            dict.get("P").unwrap().as_string().map(|s| s.as_bytes()),
            Some(prefix)
        );
    }
    if let Ok(path) = std::env::var("OXIDIZE_ISSUE743_PDF") {
        std::fs::write(path, buffer).unwrap();
    }
}
