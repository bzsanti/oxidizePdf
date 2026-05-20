//! Tests for paragraph reconstruction in the text extraction pipeline.
//!
//! Reproduces issue #261: prior to the fix, fragments arriving from PDF Tj/TJ
//! operators are passed through the partitioner one-per-fragment, producing
//! per-word "chunks" that are unusable for RAG ingestion.

use oxidize_pdf::text::{ExtractionOptions, TextExtractor, TextFragment};

fn frag(text: &str, x: f64, y: f64, width: f64, font_size: f64) -> TextFragment {
    TextFragment {
        text: text.to_string(),
        x,
        y,
        width,
        height: font_size,
        font_size,
        font_name: Some("Helvetica".to_string()),
        is_bold: false,
        is_italic: false,
        color: None,
        space_decisions: Vec::new(),
    }
}

#[test]
fn five_fragments_on_one_line_collapse_to_one_paragraph() {
    let opts = ExtractionOptions {
        reconstruct_paragraphs: true,
        ..ExtractionOptions::default()
    };
    let extractor = TextExtractor::with_options(opts);
    let input = vec![
        frag("V", 100.0, 50.0, 5.0, 8.0),
        frag("erificable en https://www", 105.0, 50.0, 140.0, 8.0),
        frag(".boe.es", 245.0, 50.0, 35.0, 8.0),
        frag("cve: BOE-A-2022-7191", 290.0, 50.0, 110.0, 8.0),
    ];

    let merged = extractor.merge_fragments_for_partition(&input);

    assert_eq!(
        merged.len(),
        1,
        "all four fragments on one line must collapse to one"
    );
    let f = &merged[0];
    assert!(
        f.text.contains("Verificable en https://www.boe.es"),
        "merged text must contain the joined URL, got {:?}",
        f.text
    );
    assert!(
        f.text.contains("cve: BOE-A-2022-7191"),
        "merged text must contain both spans, got {:?}",
        f.text
    );
    assert!(
        f.width >= 295.0,
        "merged width must span the entire line, got {}",
        f.width
    );
}

#[test]
fn fragments_on_three_consecutive_lines_collapse_to_one_paragraph() {
    let opts = ExtractionOptions {
        reconstruct_paragraphs: true,
        ..ExtractionOptions::default()
    };
    let extractor = TextExtractor::with_options(opts);
    let input = vec![
        // Line 1 at y=400
        frag("The", 50.0, 400.0, 20.0, 12.0),
        frag("first", 75.0, 400.0, 30.0, 12.0),
        frag("line.", 110.0, 400.0, 30.0, 12.0),
        // Line 2 at y=386
        frag("Second", 50.0, 386.0, 38.0, 12.0),
        frag("line", 93.0, 386.0, 25.0, 12.0),
        frag("here.", 123.0, 386.0, 30.0, 12.0),
        // Line 3 at y=372
        frag("Third.", 50.0, 372.0, 35.0, 12.0),
    ];

    let merged = extractor.merge_fragments_for_partition(&input);

    assert_eq!(
        merged.len(),
        1,
        "three lines with normal leading must form one paragraph"
    );
    let f = &merged[0];
    assert!(
        f.text.starts_with("The first line."),
        "first line preserved: {:?}",
        f.text
    );
    assert!(
        f.text.contains("Second line here."),
        "second line preserved: {:?}",
        f.text
    );
    assert!(
        f.text.ends_with("Third."),
        "third line at end: {:?}",
        f.text
    );
    assert_eq!(
        f.text.matches('\n').count(),
        2,
        "two newlines between three lines"
    );
}

#[test]
fn two_paragraphs_separated_by_large_gap_stay_separate() {
    let opts = ExtractionOptions {
        reconstruct_paragraphs: true,
        ..ExtractionOptions::default()
    };
    let extractor = TextExtractor::with_options(opts);
    let input = vec![
        frag("Paragraph", 50.0, 400.0, 60.0, 12.0),
        frag("one.", 115.0, 400.0, 25.0, 12.0),
        // 42pt below — three blank lines
        frag("Paragraph", 50.0, 358.0, 60.0, 12.0),
        frag("two.", 115.0, 358.0, 25.0, 12.0),
    ];

    let merged = extractor.merge_fragments_for_partition(&input);

    assert_eq!(merged.len(), 2, "gap > 1.5× leading must split paragraphs");
    assert!(
        merged[0].text.contains("Paragraph one."),
        "first paragraph: {:?}",
        merged[0].text
    );
    assert!(
        merged[1].text.contains("Paragraph two."),
        "second paragraph: {:?}",
        merged[1].text
    );
}

#[test]
fn hyphenated_line_break_joins_without_space() {
    let opts = ExtractionOptions {
        reconstruct_paragraphs: true,
        merge_hyphenated: true,
        ..ExtractionOptions::default()
    };
    let extractor = TextExtractor::with_options(opts);
    let input = vec![
        // Line 1 ending with hyphen
        frag("crypto-", 50.0, 400.0, 50.0, 12.0),
        // Line 2 starting with "graphy"
        frag("graphy", 50.0, 386.0, 40.0, 12.0),
    ];

    let merged = extractor.merge_fragments_for_partition(&input);

    assert_eq!(merged.len(), 1, "hyphenated word must form one paragraph");
    assert!(
        merged[0].text.contains("cryptography") && !merged[0].text.contains("crypto-"),
        "hyphen must be elided and word joined: {:?}",
        merged[0].text
    );
}

#[test]
fn empty_input_returns_empty() {
    let opts = ExtractionOptions {
        reconstruct_paragraphs: true,
        ..ExtractionOptions::default()
    };
    let extractor = TextExtractor::with_options(opts);
    let merged = extractor.merge_fragments_for_partition(&[]);
    assert!(merged.is_empty());
}

#[test]
fn reconstruct_disabled_returns_input_unchanged_modulo_kerning_fix() {
    let opts = ExtractionOptions {
        reconstruct_paragraphs: false,
        preserve_layout: true,
        ..ExtractionOptions::default()
    };
    let extractor = TextExtractor::with_options(opts);
    let input = vec![
        frag("V", 100.0, 50.0, 5.0, 8.0),
        frag("erificable en https://www", 105.0, 50.0, 140.0, 8.0),
        frag(".boe.es", 245.0, 50.0, 35.0, 8.0),
        frag("cve: BOE-A-2022-7191", 290.0, 50.0, 110.0, 8.0),
    ];
    let merged = extractor.merge_fragments_for_partition(&input);
    assert!(
        merged.len() > 1,
        "with reconstruct_paragraphs=false, fragments must NOT be collapsed to paragraphs"
    );
}
