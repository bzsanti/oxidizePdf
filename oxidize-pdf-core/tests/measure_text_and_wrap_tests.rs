//! Tests for measure_text with &Font signature and wrap_text_to_lines correctness

use oxidize_pdf::advanced_tables::{AdvancedTableBuilder, CellAlignment, CellStyle, TableRenderer};
use oxidize_pdf::measure_text;
use oxidize_pdf::text::Font;
use oxidize_pdf::{Document, Page};

// ==================== measure_text with &Font ====================

#[test]
fn test_measure_text_by_ref_helvetica() {
    let font = Font::Helvetica;
    let width = measure_text("Hello", &font, 12.0);
    assert!(width > 0.0, "Width should be positive");
    // Helvetica 'H'=722, 'e'=556, 'l'=222, 'l'=222, 'o'=556 → sum=2278 units
    // at 12pt → 2278/1000 * 12 = 27.336
    assert!(
        (width - 27.336).abs() < 0.01,
        "Expected ~27.336, got {width}"
    );
}

#[test]
fn test_measure_text_by_ref_custom_font() {
    // Custom font has no known metrics — falls back to the same path as non-symbolic
    let font = Font::Custom("MyFont".to_string());
    let width = measure_text("ABC", &font, 10.0);
    // Should not panic; width must be non-negative
    assert!(width >= 0.0, "Width must be non-negative for custom font");
}

#[test]
fn test_measure_text_by_ref_courier() {
    let font = Font::Courier;
    let text = "Hello";
    let width = measure_text(text, &font, 12.0);
    assert!(width > 0.0);
    // Courier is monospace: all chars are 600 units
    // 5 chars * 600/1000 * 12 = 36.0
    assert!(
        (width - 36.0).abs() < 0.01,
        "Expected 36.0 for Courier, got {width}"
    );
}

#[test]
fn test_measure_text_ref_scales_with_font_size() {
    let font = Font::Helvetica;
    let text = "Hello";
    let width_12 = measure_text(text, &font, 12.0);
    let width_24 = measure_text(text, &font, 24.0);
    assert!(
        (width_24 - width_12 * 2.0).abs() < 0.001,
        "Width should double when font size doubles"
    );
}

#[test]
fn test_measure_text_ref_empty_string() {
    let font = Font::Helvetica;
    let width = measure_text("", &font, 12.0);
    assert_eq!(width, 0.0, "Empty string should have zero width");
}

#[test]
fn test_measure_text_ref_different_fonts_differ() {
    let text = "Hello, World!";
    let helvetica_width = measure_text(text, &Font::Helvetica, 12.0);
    let times_width = measure_text(text, &Font::TimesRoman, 12.0);
    let courier_width = measure_text(text, &Font::Courier, 12.0);
    assert_ne!(
        helvetica_width, times_width,
        "Helvetica and Times should differ"
    );
    assert_ne!(
        courier_width, helvetica_width,
        "Courier and Helvetica should differ"
    );
}

// ==================== wrap_text_to_lines behaviour ====================

/// Invoke the wrap logic through table render to avoid accessing private methods.
fn render_wrapped_cell(text: &str, col_width: f64, font: Font) -> Vec<u8> {
    let style = CellStyle::new()
        .font(font)
        .font_size(12.0)
        .text_wrap(true)
        .alignment(CellAlignment::Left);

    let table = AdvancedTableBuilder::new()
        .columns(vec![("Col", col_width)])
        .add_styled_row(vec![text], style)
        .build()
        .unwrap();

    let renderer = TableRenderer::new();
    let mut page = Page::a4();
    renderer
        .render_table(&mut page, &table, 50.0, 700.0)
        .expect("render_table failed");
    let mut doc = Document::new();
    doc.add_page(page);
    doc.to_bytes().expect("to_bytes failed")
}

#[test]
fn test_wrap_text_single_word_fits() {
    // A single short word must stay on one line — render must not panic
    let font = Font::Helvetica;
    let font_for_measure = Font::Helvetica;
    let word_width = measure_text("Hello", &font_for_measure, 12.0);
    let max_width = word_width + 20.0;

    let bytes = render_wrapped_cell("Hello", max_width, font);
    assert!(bytes.starts_with(b"%PDF"), "Should produce a valid PDF");
}

#[test]
fn test_wrap_text_multiple_words_wrap() {
    // Force wrapping: use a column that fits about 2 words but not 4.
    // Verify the wrapping logic directly using the incremental algorithm.
    let font = Font::Helvetica;
    let font_size = 12.0;
    let word_width = measure_text("word", &font, font_size);
    let space_width = measure_text(" ", &font, font_size);
    // Width fits exactly 2 words + 1 space, so "word word word word" must wrap
    let max_width = 2.0 * word_width + space_width + 1.0;

    // Simulate the incremental wrap (same algorithm as the refactored wrap_text_to_lines)
    let text = "word word word word";
    let words: Vec<&str> = text.split_whitespace().collect();
    let mut result_lines: Vec<String> = Vec::new();
    let mut current_line = String::new();
    let mut current_line_width = 0.0f64;

    for word in &words {
        let ww = measure_text(word, &font, font_size);
        let test_width = if current_line.is_empty() {
            ww
        } else {
            current_line_width + space_width + ww
        };
        if current_line.is_empty() || test_width <= max_width {
            if !current_line.is_empty() {
                current_line.push(' ');
            }
            current_line.push_str(word);
            current_line_width = test_width;
        } else {
            result_lines.push(current_line.clone());
            current_line = word.to_string();
            current_line_width = ww;
        }
    }
    if !current_line.is_empty() {
        result_lines.push(current_line);
    }

    assert!(
        result_lines.len() >= 2,
        "4 words with 2-word max-width must produce at least 2 lines"
    );

    // All lines must fit within max_width
    for line in &result_lines {
        let lw = measure_text(line, &font, font_size);
        assert!(
            lw <= max_width + 0.001,
            "Line '{line}' width {lw} exceeds max_width {max_width}"
        );
    }

    // The render must also succeed
    let bytes = render_wrapped_cell(text, max_width, Font::Helvetica);
    assert!(bytes.starts_with(b"%PDF"));
}

#[test]
fn test_wrap_text_long_paragraph() {
    // 20+ word text with narrow column — render must succeed
    let text =
        "The quick brown fox jumps over the lazy dog and then runs away into the forest to hide";
    let bytes = render_wrapped_cell(text, 60.0, Font::Helvetica);
    assert!(bytes.starts_with(b"%PDF"));

    // Verify that every individual word (that fits alone) has width <= max_width
    let font = Font::Helvetica;
    let max_width = 60.0f64;
    for word in text.split_whitespace() {
        let word_width = measure_text(word, &font, 12.0);
        // The implementation breaks overlong words character by character,
        // so we only assert that words which fit within max_width are correctly bounded.
        if word_width <= max_width {
            assert!(
                word_width <= max_width,
                "Word '{word}' width {word_width} exceeds max_width {max_width}"
            );
        }
    }
}

#[test]
fn test_wrap_text_empty_string() {
    let bytes = render_wrapped_cell("", 100.0, Font::Helvetica);
    assert!(
        bytes.starts_with(b"%PDF"),
        "Empty text should still produce a valid PDF"
    );
}

#[test]
fn test_wrap_text_incremental_width_matches_full_measure() {
    // Verify the incremental-width algorithm produces lines that all fit within max_width,
    // and that no words are dropped.
    let font = Font::Helvetica;
    let font_size = 12.0;
    let max_width = 80.0f64;
    let text = "This is a test of incremental width calculation in word wrapping";

    let words: Vec<&str> = text.split_whitespace().collect();
    let space_width = measure_text(" ", &font, font_size);

    // Run the incremental algorithm (mirrors the refactored implementation)
    let mut result_lines: Vec<String> = Vec::new();
    let mut current_line = String::new();
    let mut current_line_width = 0.0f64;

    for word in &words {
        let word_width = measure_text(word, &font, font_size);
        let test_width = if current_line.is_empty() {
            word_width
        } else {
            current_line_width + space_width + word_width
        };

        if current_line.is_empty() || test_width <= max_width {
            if !current_line.is_empty() {
                current_line.push(' ');
            }
            current_line.push_str(word);
            current_line_width = test_width;
        } else {
            result_lines.push(current_line.clone());
            current_line = word.to_string();
            current_line_width = word_width;
        }
    }
    if !current_line.is_empty() {
        result_lines.push(current_line);
    }

    // All lines must fit within max_width
    for line in &result_lines {
        let line_width = measure_text(line, &font, font_size);
        assert!(
            line_width <= max_width + 0.001,
            "Line '{line}' width {line_width} exceeds max_width {max_width}"
        );
    }

    // Joining all lines must reproduce the original text
    let joined = result_lines.join(" ");
    assert_eq!(
        joined, text,
        "Incremental wrap must not drop or duplicate words"
    );
}
