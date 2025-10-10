//! Tests for OcrPostProcessor and related OCR correction functionality

use super::*;
use std::collections::HashSet;

#[test]
fn test_ocr_region_contains_point() {
    let region = OcrRegion::new(100, 200, 300, 400);

    // Inside region
    assert!(region.contains_point(150, 250));
    assert!(region.contains_point(100, 200)); // Top-left corner
    assert!(region.contains_point(399, 599)); // Bottom-right corner (exclusive)

    // Outside region
    assert!(!region.contains_point(50, 150)); // Before region
    assert!(!region.contains_point(400, 600)); // After region
    assert!(!region.contains_point(150, 100)); // Above region
    assert!(!region.contains_point(500, 250)); // Right of region
}

#[test]
fn test_ocr_region_overlaps_with() {
    let region1 = OcrRegion::new(100, 100, 200, 200); // (100,100) to (300,300)

    // Fully overlapping
    let region2 = OcrRegion::new(150, 150, 100, 100); // Inside region1
    assert!(region1.overlaps_with(&region2));
    assert!(region2.overlaps_with(&region1));

    // Partially overlapping
    let region3 = OcrRegion::new(250, 250, 100, 100); // (250,250) to (350,350)
    assert!(region1.overlaps_with(&region3));

    // Edge touching (not overlapping in this implementation)
    let region4 = OcrRegion::new(300, 300, 100, 100); // Starts where region1 ends
    assert!(!region1.overlaps_with(&region4));

    // Completely separate
    let region5 = OcrRegion::new(500, 500, 100, 100);
    assert!(!region1.overlaps_with(&region5));
}

#[test]
fn test_word_confidence_average_character_confidence() {
    // Without character confidences
    let word1 = WordConfidence::new("hello".to_string(), 0.9, 0.0, 50.0);
    assert_eq!(word1.average_character_confidence(), None);

    // With character confidences
    let char_confs = vec![
        CharacterConfidence::new('h', 0.9, 0.0, 10.0),
        CharacterConfidence::new('e', 0.8, 10.0, 10.0),
        CharacterConfidence::new('l', 0.7, 20.0, 10.0),
        CharacterConfidence::new('l', 0.6, 30.0, 10.0),
        CharacterConfidence::new('o', 0.5, 40.0, 10.0),
    ];
    let word2 = WordConfidence::with_characters("hello".to_string(), 0.7, 0.0, 50.0, char_confs);

    let avg = word2.average_character_confidence().unwrap();
    assert!((avg - 0.7).abs() < 0.01); // Average of 0.9, 0.8, 0.7, 0.6, 0.5
}

#[test]
fn test_word_confidence_is_low_confidence() {
    let word = WordConfidence::new("test".to_string(), 0.65, 0.0, 40.0);

    assert!(word.is_low_confidence(0.7)); // 0.65 < 0.7
    assert!(!word.is_low_confidence(0.6)); // 0.65 >= 0.6
    assert!(!word.is_low_confidence(0.65)); // Equal boundary
}

#[test]
fn test_post_processor_character_substitution() {
    let processor = OcrPostProcessor::new();

    // Test character substitution - should generate suggestions
    let suggestions = processor.generate_suggestions("H3ll0");
    assert!(
        !suggestions.is_empty(),
        "Should generate character substitution suggestions"
    );

    // Verify all suggestions are valid character substitutions
    for suggestion in &suggestions {
        assert_eq!(
            suggestion.correction_type,
            CorrectionType::CharacterSubstitution
        );
        // Should have changed at least one character
        assert_ne!(suggestion.corrected_word, "H3ll0");
    }

    // Test "1" -> "l" substitution with simpler word
    let suggestions2 = processor.generate_suggestions("he1lo");
    let has_l_substitution = suggestions2.iter().any(|s| s.corrected_word == "hello");
    assert!(
        has_l_substitution,
        "Should suggest 1 -> l substitution for 'he1lo'"
    );
}

#[test]
fn test_post_processor_pattern_correction() {
    let processor = OcrPostProcessor::new();

    // Test "rn" -> "m" pattern
    let suggestions = processor.generate_suggestions("infornation");

    // Should have some suggestions
    assert!(!suggestions.is_empty(), "Should generate suggestions");

    // Check if pattern correction exists and has high confidence
    let pattern_corrections: Vec<_> = suggestions
        .iter()
        .filter(|s| s.correction_type == CorrectionType::PatternCorrection)
        .collect();

    assert!(
        !pattern_corrections.is_empty(),
        "Should have pattern corrections"
    );

    // Verify that "infomation" is suggested (rn -> m replaces "rn" in "infornation")
    let has_m_correction = suggestions.iter().any(|s| s.corrected_word == "infomation");
    assert!(
        has_m_correction,
        "Should suggest rn -> m pattern correction: 'infornation' -> 'infomation'"
    );

    // Test "cl" -> "d" pattern
    let suggestions2 = processor.generate_suggestions("olcl");
    let has_d_correction = suggestions2.iter().any(|s| s.corrected_word == "old");
    assert!(
        has_d_correction,
        "Should suggest cl -> d pattern correction: 'olcl' -> 'old'"
    );
}

#[test]
fn test_post_processor_with_dictionary() {
    let mut dictionary = HashSet::new();
    dictionary.insert("hello".to_string());
    dictionary.insert("world".to_string());
    dictionary.insert("test".to_string());

    let processor = OcrPostProcessor::new().with_dictionary(dictionary);

    // Word already in dictionary - should return no dictionary suggestions
    let suggestions = processor.generate_suggestions("hello");
    let dict_suggestions: Vec<_> = suggestions
        .iter()
        .filter(|s| matches!(s.correction_type, CorrectionType::DictionaryCorrection))
        .collect();
    assert_eq!(
        dict_suggestions.len(),
        0,
        "Valid word should have no dictionary corrections"
    );

    // Word close to dictionary entry (edit distance 1)
    let suggestions2 = processor.generate_suggestions("helo"); // Missing 'l'
    let has_hello = suggestions2.iter().any(|s| s.corrected_word == "hello");
    assert!(
        has_hello,
        "Should suggest 'hello' for 'helo' (edit distance 1)"
    );
}

#[test]
fn test_post_processor_edit_distance() {
    let processor = OcrPostProcessor::new();

    // Identical strings
    assert_eq!(processor.edit_distance("hello", "hello"), 0);

    // Single character difference
    assert_eq!(processor.edit_distance("hello", "hallo"), 1); // Substitution
    assert_eq!(processor.edit_distance("hello", "helo"), 1); // Deletion
    assert_eq!(processor.edit_distance("hello", "helloo"), 1); // Insertion

    // Multiple differences
    assert_eq!(processor.edit_distance("kitten", "sitting"), 3);

    // Empty strings
    assert_eq!(processor.edit_distance("", ""), 0);
    assert_eq!(processor.edit_distance("hello", ""), 5);
    assert_eq!(processor.edit_distance("", "world"), 5);
}

#[test]
fn test_post_processor_process_fragment() {
    let processor = OcrPostProcessor::new();

    // Create fragment with low-confidence words
    let word_confs = vec![
        WordConfidence::new("He11o".to_string(), 0.5, 0.0, 50.0), // Low confidence, has "1" -> "l"
        WordConfidence::new("world".to_string(), 0.9, 55.0, 50.0), // High confidence
        WordConfidence::new("test".to_string(), 0.6, 110.0, 40.0), // Low confidence
    ];

    let fragment = OcrTextFragment::with_word_confidences(
        "He11o world test".to_string(),
        0.7,
        0.0,
        0.0,
        150.0,
        20.0,
        12.0,
        FragmentType::Line,
        word_confs,
    );

    let candidates = processor.process_fragment(&fragment);

    // Should have 2 correction candidates (threshold 0.7)
    assert_eq!(candidates.len(), 2, "Should have 2 low-confidence words");

    // First candidate should be "He11o"
    assert_eq!(candidates[0].word, "He11o");
    assert!(!candidates[0].suggested_corrections.is_empty());

    // Second candidate should be "test"
    assert_eq!(candidates[1].word, "test");
}

#[test]
fn test_fragment_get_low_confidence_words() {
    let word_confs = vec![
        WordConfidence::new("high".to_string(), 0.95, 0.0, 40.0),
        WordConfidence::new("low".to_string(), 0.45, 45.0, 30.0),
        WordConfidence::new("medium".to_string(), 0.75, 80.0, 60.0),
    ];

    let fragment = OcrTextFragment::with_word_confidences(
        "high low medium".to_string(),
        0.7,
        0.0,
        0.0,
        130.0,
        20.0,
        12.0,
        FragmentType::Line,
        word_confs,
    );

    let low_conf = fragment.get_low_confidence_words(0.7);
    assert_eq!(low_conf.len(), 1);
    assert_eq!(low_conf[0].word, "low");
}

#[test]
fn test_fragment_has_low_confidence_words() {
    let word_confs = vec![
        WordConfidence::new("word1".to_string(), 0.9, 0.0, 50.0),
        WordConfidence::new("word2".to_string(), 0.85, 55.0, 50.0),
    ];

    let fragment = OcrTextFragment::with_word_confidences(
        "word1 word2".to_string(),
        0.87,
        0.0,
        0.0,
        100.0,
        20.0,
        12.0,
        FragmentType::Line,
        word_confs,
    );

    assert!(fragment.has_low_confidence_words(0.9)); // Both below 0.9
    assert!(!fragment.has_low_confidence_words(0.8)); // Both above 0.8
}

#[test]
fn test_fragment_confidence_report() {
    let char_confs = vec![
        CharacterConfidence::new('h', 0.9, 0.0, 10.0),
        CharacterConfidence::new('i', 0.8, 10.0, 5.0),
    ];

    let word_confs = vec![
        WordConfidence::with_characters("hi".to_string(), 0.85, 0.0, 15.0, char_confs),
        WordConfidence::new("world".to_string(), 0.9, 20.0, 50.0),
    ];

    let fragment = OcrTextFragment::with_word_confidences(
        "hi world".to_string(),
        0.0,   // x
        0.0,   // y
        70.0,  // width
        20.0,  // height
        0.875, // confidence
        12.0,  // font_size
        FragmentType::Line,
        word_confs,
    );

    let report = fragment.confidence_report();

    println!("Confidence report:\n{}", report);

    // Should contain fragment confidence
    assert!(
        report.contains("87.5%") || report.contains("87."),
        "Should show fragment confidence. Got: {}",
        report
    );

    // Should contain word breakdown
    assert!(report.contains("2 words"), "Should show word count");
    assert!(report.contains("\"hi\""), "Should contain first word");
    assert!(report.contains("\"world\""), "Should contain second word");

    // Should contain character confidences for first word
    assert!(report.contains("'h'"), "Should show character 'h'");
    assert!(report.contains("'i'"), "Should show character 'i'");
}

#[test]
fn test_fragment_get_correction_candidates() {
    let word_confs = vec![
        WordConfidence::new("H3ll0".to_string(), 0.6, 0.0, 50.0),
        WordConfidence::new("w0r1d".to_string(), 0.55, 55.0, 50.0),
        WordConfidence::new("good".to_string(), 0.95, 110.0, 40.0),
    ];

    let fragment = OcrTextFragment::with_word_confidences(
        "H3ll0 w0r1d good".to_string(),
        0.7,
        0.0,
        0.0,
        150.0,
        20.0,
        12.0,
        FragmentType::Line,
        word_confs,
    );

    let candidates = fragment.get_correction_candidates(0.7);

    assert_eq!(
        candidates.len(),
        2,
        "Should have 2 candidates below 0.7 threshold"
    );
    assert_eq!(candidates[0].word, "H3ll0");
    assert_eq!(candidates[0].position_in_fragment, 0);
    assert_eq!(candidates[1].word, "w0r1d");
    assert_eq!(candidates[1].position_in_fragment, 1);
}

#[test]
fn test_fragment_words_by_confidence() {
    let word_confs = vec![
        WordConfidence::new("high".to_string(), 0.95, 0.0, 40.0),
        WordConfidence::new("low".to_string(), 0.45, 45.0, 30.0),
        WordConfidence::new("medium".to_string(), 0.75, 80.0, 60.0),
    ];

    let fragment = OcrTextFragment::with_word_confidences(
        "high low medium".to_string(),
        0.7,
        0.0,
        0.0,
        130.0,
        20.0,
        12.0,
        FragmentType::Line,
        word_confs,
    );

    let sorted = fragment.words_by_confidence();

    // Should be sorted lowest to highest
    assert_eq!(sorted.len(), 3);
    assert_eq!(sorted[0].word, "low"); // 0.45
    assert_eq!(sorted[1].word, "medium"); // 0.75
    assert_eq!(sorted[2].word, "high"); // 0.95
}

#[test]
fn test_fragment_average_word_confidence() {
    let word_confs = vec![
        WordConfidence::new("word1".to_string(), 0.8, 0.0, 50.0),
        WordConfidence::new("word2".to_string(), 0.6, 55.0, 50.0),
        WordConfidence::new("word3".to_string(), 0.7, 110.0, 50.0),
    ];

    let fragment = OcrTextFragment::with_word_confidences(
        "word1 word2 word3".to_string(),
        0.7,
        0.0,
        0.0,
        150.0,
        20.0,
        12.0,
        FragmentType::Line,
        word_confs,
    );

    let avg = fragment.average_word_confidence().unwrap();
    assert!((avg - 0.7).abs() < 0.01); // Average of 0.8, 0.6, 0.7

    // Test with empty word confidences
    let fragment2 = OcrTextFragment::with_word_confidences(
        "text".to_string(),
        0.8,
        0.0,
        0.0,
        50.0,
        20.0,
        12.0,
        FragmentType::Word,
        vec![],
    );
    assert_eq!(fragment2.average_word_confidence(), Some(0.0));
}

#[test]
fn test_ocr_region_with_label() {
    let region = OcrRegion::with_label(50, 100, 200, 150, "header");

    assert_eq!(region.x, 50);
    assert_eq!(region.y, 100);
    assert_eq!(region.width, 200);
    assert_eq!(region.height, 150);
    assert_eq!(region.label, Some("header".to_string()));

    // Test with String
    let region2 = OcrRegion::with_label(0, 0, 100, 100, "table".to_string());
    assert_eq!(region2.label, Some("table".to_string()));
}

#[test]
fn test_post_processor_suggestion_limit() {
    let mut dictionary = HashSet::new();
    // Add many similar words to test truncation
    for i in 0..20 {
        dictionary.insert(format!("word{i}"));
    }

    let processor = OcrPostProcessor::new().with_dictionary(dictionary);

    // This should generate many suggestions but be limited to 5
    let suggestions = processor.generate_suggestions("word");
    assert!(suggestions.len() <= 5, "Should limit suggestions to 5");
}

#[test]
fn test_correction_types() {
    use std::collections::HashMap;

    // Test that CorrectionType implements Hash
    let mut types_map: HashMap<CorrectionType, u32> = HashMap::new();
    types_map.insert(CorrectionType::CharacterSubstitution, 1);
    types_map.insert(CorrectionType::DictionaryCorrection, 2);
    types_map.insert(CorrectionType::PatternCorrection, 3);

    assert_eq!(types_map.len(), 3);
    assert_eq!(
        *types_map
            .get(&CorrectionType::CharacterSubstitution)
            .unwrap(),
        1
    );
}
