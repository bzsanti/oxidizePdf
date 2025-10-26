use oxidize_pdf::{measure_text, split_into_words, Document, Font, Page, TextAlign};
use std::fs;
use std::path::Path;

#[test]
fn test_text_measurement() {
    // Test measuring text with different fonts
    let text = "Hello, World!";

    let helvetica_width = measure_text(text, Font::Helvetica, 12.0);
    let times_width = measure_text(text, Font::TimesRoman, 12.0);
    let courier_width = measure_text(text, Font::Courier, 12.0);

    // Helvetica and Times should have different widths
    assert_ne!(helvetica_width, times_width);

    // Courier is monospace, so should be different from proportional fonts
    assert_ne!(courier_width, helvetica_width);

    // Test that font size affects width
    let width_12pt = measure_text(text, Font::Helvetica, 12.0);
    let width_24pt = measure_text(text, Font::Helvetica, 24.0);
    assert_eq!(width_24pt, width_12pt * 2.0);
}

#[test]
fn test_word_splitting() {
    // Test basic word splitting
    let text = "Hello world test";
    let words = split_into_words(text);
    assert_eq!(words.len(), 5); // "Hello", " ", "world", " ", "test"
    assert_eq!(words[0], "Hello");
    assert_eq!(words[1], " ");
    assert_eq!(words[2], "world");
    assert_eq!(words[3], " ");
    assert_eq!(words[4], "test");

    // Test with multiple spaces
    let text = "Hello   world";
    let words = split_into_words(text);
    assert_eq!(words.len(), 3);
    assert_eq!(words[0], "Hello");
    assert_eq!(words[1], "   ");
    assert_eq!(words[2], "world");

    // Test with tabs and newlines
    let text = "Hello\tworld\ntest";
    let words = split_into_words(text);
    assert_eq!(words.len(), 5);
}

#[test]
fn test_margins() {
    let mut page = Page::a4();

    // Test default margins
    let default_margins = page.margins();
    assert_eq!(default_margins.left, 72.0);
    assert_eq!(default_margins.right, 72.0);
    assert_eq!(default_margins.top, 72.0);
    assert_eq!(default_margins.bottom, 72.0);

    // Test setting custom margins
    page.set_margins(50.0, 60.0, 70.0, 80.0);
    let margins = page.margins();
    assert_eq!(margins.left, 50.0);
    assert_eq!(margins.right, 60.0);
    assert_eq!(margins.top, 70.0);
    assert_eq!(margins.bottom, 80.0);

    // Test content area calculations
    assert_eq!(page.content_width(), 595.0 - 50.0 - 60.0);
    assert_eq!(page.content_height(), 842.0 - 70.0 - 80.0);
}

#[test]
fn test_text_wrapping_pdf() {
    let mut doc = Document::new();
    let mut page = Page::a4();
    page.set_margins(50.0, 50.0, 50.0, 50.0);

    let mut text_flow = page.text_flow();

    // Test left alignment
    text_flow
        .set_font(Font::Helvetica, 12.0)
        .set_alignment(TextAlign::Left)
        .at(0.0, 750.0)
        .write_wrapped("This is a test of left-aligned text wrapping.")
        .expect("Failed to write left-aligned text");

    // Test right alignment
    text_flow
        .set_alignment(TextAlign::Right)
        .newline()
        .write_wrapped("This is a test of right-aligned text wrapping.")
        .expect("Failed to write right-aligned text");

    // Test center alignment
    text_flow
        .set_alignment(TextAlign::Center)
        .newline()
        .write_wrapped("This is a test of center-aligned text wrapping.")
        .expect("Failed to write center-aligned text");

    // Test justified alignment
    text_flow
        .set_alignment(TextAlign::Justified)
        .newline()
        .write_wrapped("This is a test of justified text wrapping with multiple words to demonstrate the justification algorithm.")
        .expect("Failed to write justified text");

    page.add_text_flow(&text_flow);
    doc.add_page(page);

    let output_path = "test_text_wrapping.pdf";
    doc.save(output_path).expect("Failed to save PDF");

    // Verify file exists
    assert!(Path::new(output_path).exists());

    // Cleanup
    fs::remove_file(output_path).ok();
}

#[test]
fn test_long_text_wrapping() {
    let mut doc = Document::new();
    let mut page = Page::a4();
    page.set_margins(40.0, 40.0, 40.0, 40.0);

    let mut text_flow = page.text_flow();

    let long_text = "Lorem ipsum dolor sit amet, consectetur adipiscing elit. Sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. Ut enim ad minim veniam, quis nostrud exercitation ullamco laboris nisi ut aliquip ex ea commodo consequat. Duis aute irure dolor in reprehenderit in voluptate velit esse cillum dolore eu fugiat nulla pariatur. Excepteur sint occaecat cupidatat non proident, sunt in culpa qui officia deserunt mollit anim id est laborum.";

    text_flow
        .set_font(Font::Helvetica, 11.0)
        .set_alignment(TextAlign::Justified)
        .at(0.0, 750.0)
        .write_paragraph(long_text)
        .expect("Failed to write long paragraph");

    page.add_text_flow(&text_flow);
    doc.add_page(page);

    let output_path = "test_long_text.pdf";
    doc.save(output_path).expect("Failed to save PDF");

    // Verify file exists
    assert!(Path::new(output_path).exists());

    // Cleanup
    fs::remove_file(output_path).ok();
}

#[test]
fn test_different_fonts_wrapping() {
    let mut doc = Document::new();
    let mut page = Page::a4();
    page.set_margins(50.0, 50.0, 50.0, 50.0);

    let mut text_flow = page.text_flow();

    let test_text = "The quick brown fox jumps over the lazy dog. This pangram contains all letters of the alphabet.";

    // Test with different fonts
    text_flow
        .at(0.0, 750.0)
        .set_font(Font::Helvetica, 12.0)
        .write_wrapped(test_text)
        .expect("Failed with Helvetica");

    text_flow
        .newline()
        .newline()
        .set_font(Font::TimesRoman, 12.0)
        .write_wrapped(test_text)
        .expect("Failed with Times");

    text_flow
        .newline()
        .newline()
        .set_font(Font::Courier, 12.0)
        .write_wrapped(test_text)
        .expect("Failed with Courier");

    page.add_text_flow(&text_flow);
    doc.add_page(page);

    let output_path = "test_fonts_wrapping.pdf";
    doc.save(output_path).expect("Failed to save PDF");

    // Verify file exists
    assert!(Path::new(output_path).exists());

    // Cleanup
    fs::remove_file(output_path).ok();
}
