//! Integration tests for page labels module
//! Tests real-world page labeling scenarios

use oxidize_pdf::page_labels::{PageLabel, PageLabelBuilder, PageLabelStyle, PageLabelTree};
use oxidize_pdf::writer::PdfWriter;
use oxidize_pdf::{Document, Page};
use std::io::Cursor;

#[test]
fn test_book_with_front_matter() {
    // Simulate a book with:
    // - Cover page (no number)
    // - Roman numerals for preface (i-iv)
    // - Decimal for main content (1-100)
    // - Letters for appendices (A-C)

    let tree = PageLabelBuilder::new()
        .prefix_pages(1, "Cover")
        .roman_pages(4, false) // i, ii, iii, iv
        .decimal_pages(100) // 1-100
        .letter_pages(3, true) // A, B, C
        .build();

    // Verify specific pages
    assert_eq!(tree.get_label(0), Some("Cover".to_string()));
    assert_eq!(tree.get_label(1), Some("i".to_string()));
    assert_eq!(tree.get_label(4), Some("iv".to_string()));
    assert_eq!(tree.get_label(5), Some("1".to_string()));
    assert_eq!(tree.get_label(104), Some("100".to_string()));
    assert_eq!(tree.get_label(105), Some("A".to_string()));
    assert_eq!(tree.get_label(107), Some("C".to_string()));
}

#[test]
fn test_academic_paper_numbering() {
    // Academic paper with:
    // - Abstract (no page number)
    // - Table of contents (roman i-ii)
    // - Main text (arabic 1-50)
    // - References (continuing arabic 51-55)
    // - Appendices with prefix (Appendix A, B, C)

    let mut tree = PageLabelTree::new();
    tree.add_range(0, PageLabel::prefix_only("Abstract"));
    tree.add_range(1, PageLabel::roman_lowercase().with_prefix(""));
    tree.add_range(3, PageLabel::decimal());
    tree.add_range(58, PageLabel::letters_uppercase().with_prefix("Appendix "));

    // Test the numbering
    assert_eq!(tree.get_label(0), Some("Abstract".to_string()));
    assert_eq!(tree.get_label(1), Some("i".to_string()));
    assert_eq!(tree.get_label(2), Some("ii".to_string()));
    assert_eq!(tree.get_label(3), Some("1".to_string()));
    assert_eq!(tree.get_label(52), Some("50".to_string()));
    assert_eq!(tree.get_label(57), Some("55".to_string()));
    assert_eq!(tree.get_label(58), Some("Appendix A".to_string()));
    assert_eq!(tree.get_label(59), Some("Appendix B".to_string()));
}

#[test]
fn test_manual_style_numbering() {
    // Technical manual with chapter-based numbering
    let mut tree = PageLabelTree::new();

    // Chapter 1 (pages 1-1 to 1-20)
    tree.add_range(0, PageLabel::decimal().with_prefix("1-"));

    // Chapter 2 (pages 2-1 to 2-15)
    tree.add_range(20, PageLabel::decimal().with_prefix("2-"));

    // Chapter 3 (pages 3-1 to 3-25)
    tree.add_range(35, PageLabel::decimal().with_prefix("3-"));

    // Index (pages Index-1 to Index-5)
    tree.add_range(60, PageLabel::decimal().with_prefix("Index-"));

    assert_eq!(tree.get_label(0), Some("1-1".to_string()));
    assert_eq!(tree.get_label(19), Some("1-20".to_string()));
    assert_eq!(tree.get_label(20), Some("2-1".to_string()));
    assert_eq!(tree.get_label(34), Some("2-15".to_string()));
    assert_eq!(tree.get_label(35), Some("3-1".to_string()));
    assert_eq!(tree.get_label(59), Some("3-25".to_string()));
    assert_eq!(tree.get_label(60), Some("Index-1".to_string()));
    assert_eq!(tree.get_label(64), Some("Index-5".to_string()));
}

#[test]
fn test_mixed_numbering_styles() {
    // Complex document with various numbering styles
    let tree = PageLabelBuilder::new()
        .add_range(2, PageLabel::prefix_only("Cover ")) // Cover pages
        .add_range(3, PageLabel::roman_uppercase()) // I, II, III
        .add_range(5, PageLabel::decimal().starting_at(10)) // 10, 11, 12, 13, 14
        .add_range(4, PageLabel::letters_lowercase()) // a, b, c, d
        .build();

    let all_labels = tree.get_all_labels(14);

    assert_eq!(all_labels[0], "Cover ");
    assert_eq!(all_labels[1], "Cover ");
    assert_eq!(all_labels[2], "I");
    assert_eq!(all_labels[3], "II");
    assert_eq!(all_labels[4], "III");
    assert_eq!(all_labels[5], "10");
    assert_eq!(all_labels[9], "14");
    assert_eq!(all_labels[10], "a");
    assert_eq!(all_labels[13], "d");
}

#[test]
fn test_restarting_numbering() {
    // Test restarting numbering at different points
    let mut tree = PageLabelTree::new();

    // First section: 1-5
    tree.add_range(0, PageLabel::decimal());

    // Second section: restart at 1
    tree.add_range(5, PageLabel::decimal().with_prefix("Section 2, Page "));

    // Third section: restart at 100
    tree.add_range(10, PageLabel::decimal().starting_at(100));

    assert_eq!(tree.get_label(0), Some("1".to_string()));
    assert_eq!(tree.get_label(4), Some("5".to_string()));
    assert_eq!(tree.get_label(5), Some("Section 2, Page 1".to_string()));
    assert_eq!(tree.get_label(9), Some("Section 2, Page 5".to_string()));
    assert_eq!(tree.get_label(10), Some("100".to_string()));
    assert_eq!(tree.get_label(14), Some("104".to_string()));
}

#[test]
fn test_dictionary_round_trip() {
    // Create a complex tree
    let mut tree = PageLabelTree::new();
    tree.add_range(0, PageLabel::roman_lowercase());
    tree.add_range(3, PageLabel::decimal().with_prefix("Page ").starting_at(10));
    tree.add_range(8, PageLabel::letters_uppercase().with_prefix("Appendix "));

    // Convert to dictionary
    let dict = tree.to_dict();

    // Should have Nums array
    assert!(dict.get("Nums").is_some());

    // In a real scenario, we'd parse it back
    // For now, just verify structure
    if let Some(oxidize_pdf::objects::Object::Array(nums)) = dict.get("Nums") {
        // Should have 6 elements (3 ranges * 2 elements each)
        assert_eq!(nums.len(), 6);

        // First range should start at 0
        if let oxidize_pdf::objects::Object::Integer(n) = &nums[0] {
            assert_eq!(*n, 0);
        }

        // Second range should start at 3
        if let oxidize_pdf::objects::Object::Integer(n) = &nums[2] {
            assert_eq!(*n, 3);
        }

        // Third range should start at 8
        if let oxidize_pdf::objects::Object::Integer(n) = &nums[4] {
            assert_eq!(*n, 8);
        }
    }
}

#[test]
fn test_empty_tree() {
    let tree = PageLabelTree::new();

    // Empty tree returns None for any page
    assert_eq!(tree.get_label(0), None);
    assert_eq!(tree.get_label(100), None);

    // get_all_labels with empty tree should return default numbering
    let labels = tree.get_all_labels(5);
    assert_eq!(labels, vec!["1", "2", "3", "4", "5"]);
}

#[test]
fn test_single_range_tree() {
    let mut tree = PageLabelTree::new();
    tree.add_range(0, PageLabel::decimal().starting_at(100));

    // All pages use the same range
    assert_eq!(tree.get_label(0), Some("100".to_string()));
    assert_eq!(tree.get_label(50), Some("150".to_string()));
    assert_eq!(tree.get_label(999), Some("1099".to_string()));
}

#[test]
fn test_unicode_prefixes() {
    let mut tree = PageLabelTree::new();

    // Test with various Unicode prefixes
    tree.add_range(0, PageLabel::decimal().with_prefix("第")); // Chinese
    tree.add_range(3, PageLabel::decimal().with_prefix("עמוד ")); // Hebrew
    tree.add_range(6, PageLabel::decimal().with_prefix("Σελίδα ")); // Greek
    tree.add_range(9, PageLabel::decimal().with_prefix("صفحة ")); // Arabic

    assert_eq!(tree.get_label(0), Some("第1".to_string()));
    assert_eq!(tree.get_label(3), Some("עמוד 1".to_string()));
    assert_eq!(tree.get_label(6), Some("Σελίδα 1".to_string()));
    assert_eq!(tree.get_label(9), Some("صفحة 1".to_string()));
}

#[test]
fn test_large_document_performance() {
    // Test with a large number of pages
    let tree = PageLabelBuilder::new()
        .roman_pages(10, false) // i-x
        .decimal_pages(1000) // 1-1000
        .letter_pages(26, true) // A-Z
        .build();

    // Test specific pages
    assert_eq!(tree.get_label(0), Some("i".to_string()));
    assert_eq!(tree.get_label(9), Some("x".to_string()));
    assert_eq!(tree.get_label(10), Some("1".to_string()));
    assert_eq!(tree.get_label(1009), Some("1000".to_string()));
    assert_eq!(tree.get_label(1010), Some("A".to_string()));
    assert_eq!(tree.get_label(1035), Some("Z".to_string()));

    // Get all labels for performance check
    let all_labels = tree.get_all_labels(1036);
    assert_eq!(all_labels.len(), 1036);
    assert_eq!(all_labels[0], "i");
    assert_eq!(all_labels[10], "1");
    assert_eq!(all_labels[1035], "Z");
}

#[test]
fn test_special_characters_in_prefix() {
    let mut tree = PageLabelTree::new();

    // Test with special characters
    tree.add_range(0, PageLabel::decimal().with_prefix("§"));
    tree.add_range(3, PageLabel::decimal().with_prefix("№"));
    tree.add_range(6, PageLabel::decimal().with_prefix("©"));
    tree.add_range(9, PageLabel::decimal().with_prefix("™ Page "));

    assert_eq!(tree.get_label(0), Some("§1".to_string()));
    assert_eq!(tree.get_label(3), Some("№1".to_string()));
    assert_eq!(tree.get_label(6), Some("©1".to_string()));
    assert_eq!(tree.get_label(9), Some("™ Page 1".to_string()));
}

#[test]
fn test_builder_chaining() {
    // Test all builder methods
    let tree = PageLabelBuilder::new()
        .prefix_pages(1, "Title")
        .roman_pages(2, true) // Uppercase
        .roman_pages(2, false) // Lowercase
        .decimal_pages(5)
        .letter_pages(2, true) // Uppercase
        .letter_pages(2, false) // Lowercase
        .add_range(
            3,
            PageLabel::decimal().with_prefix("Extra ").starting_at(100),
        )
        .build();

    let labels = tree.get_all_labels(17);

    assert_eq!(labels[0], "Title");
    assert_eq!(labels[1], "I");
    assert_eq!(labels[2], "II");
    assert_eq!(labels[3], "i");
    assert_eq!(labels[4], "ii");
    assert_eq!(labels[5], "1");
    assert_eq!(labels[9], "5");
    assert_eq!(labels[10], "A");
    assert_eq!(labels[11], "B");
    assert_eq!(labels[12], "a");
    assert_eq!(labels[13], "b");
    assert_eq!(labels[14], "Extra 100");
    assert_eq!(labels[15], "Extra 101");
    assert_eq!(labels[16], "Extra 102");
}

#[test]
fn test_overlapping_ranges() {
    // Test that later ranges override earlier ones
    let mut tree = PageLabelTree::new();
    tree.add_range(0, PageLabel::decimal());
    tree.add_range(5, PageLabel::roman_uppercase());
    tree.add_range(5, PageLabel::letters_lowercase()); // Override the roman

    assert_eq!(tree.get_label(4), Some("5".to_string()));
    assert_eq!(tree.get_label(5), Some("a".to_string())); // Should be letter, not roman
    assert_eq!(tree.get_label(6), Some("b".to_string()));
}

#[test]
fn test_gaps_in_ranges() {
    // Test behavior with gaps (pages not explicitly covered)
    let mut tree = PageLabelTree::new();
    tree.add_range(0, PageLabel::decimal());
    tree.add_range(10, PageLabel::roman_lowercase());

    // Pages 0-9 use decimal
    assert_eq!(tree.get_label(0), Some("1".to_string()));
    assert_eq!(tree.get_label(9), Some("10".to_string()));

    // Pages 10+ use roman
    assert_eq!(tree.get_label(10), Some("i".to_string()));
    assert_eq!(tree.get_label(15), Some("vi".to_string()));
}

#[test]
fn test_pdf_generation_with_labels() {
    // Create a document with page labels
    let mut document = Document::new();
    document.set_title("Page Labels Test");

    // Add some pages
    for i in 0..5 {
        let mut page = Page::a4();
        page.text()
            .set_font(oxidize_pdf::graphics::Font::Helvetica, 12.0)
            .at(100.0, 700.0)
            .write(&format!("Physical Page {}", i + 1))
            .unwrap();
        document.add_page(page);
    }

    // Create page labels
    let tree = PageLabelBuilder::new()
        .roman_pages(2, false) // i, ii
        .decimal_pages(3) // 1, 2, 3
        .build();

    // In a real implementation, we'd set the labels on the document
    // For now, just verify we can create the dictionary
    let labels_dict = tree.to_dict();
    assert!(labels_dict.get("Nums").is_some());

    // Write to buffer to ensure document is valid
    let mut buffer = Vec::new();
    let mut writer = PdfWriter::new(&mut buffer);
    writer.write_document(&mut document).unwrap();

    // Verify PDF structure
    let pdf_content = String::from_utf8_lossy(&buffer);
    assert!(pdf_content.starts_with("%PDF-"));
    assert!(pdf_content.contains("%%EOF"));
}
