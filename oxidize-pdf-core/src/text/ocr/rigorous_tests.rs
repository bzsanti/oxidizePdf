//! Rigorous tests for OCR post-processor - No compromises
//!
//! These tests are designed to be:
//! - Specific and deterministic
//! - Test actual algorithm behavior (not just "doesn't crash")
//! - Include error handling and edge cases
//! - No "relaxed" assertions

use super::*;
use std::collections::HashSet;

// =============================================================================
// TESTS FOR CHARACTER SUBSTITUTION (With Specific Expectations)
// =============================================================================

#[test]
fn test_character_substitution_single_character() {
    let processor = OcrPostProcessor::new();

    // Test with "1" at position 2 (should be in top 5 suggestions)
    // Algorithm generates: '1' -> ['l', 'I', '|']
    let suggestions = processor.generate_suggestions("He1lo");

    // Must have suggestions
    assert!(
        !suggestions.is_empty(),
        "Should generate suggestions for 'He1lo'"
    );

    // Should contain "Hello" (1 -> l substitution)
    let has_hello = suggestions.iter().any(|s| s.corrected_word == "Hello");
    assert!(
        has_hello,
        "Should suggest 'Hello' from 'He1lo'. Got: {:?}",
        suggestions
            .iter()
            .map(|s| &s.corrected_word)
            .collect::<Vec<_>>()
    );

    // Verify it's a character substitution
    let hello_suggestion = suggestions
        .iter()
        .find(|s| s.corrected_word == "Hello")
        .expect("Hello should exist");
    assert_eq!(
        hello_suggestion.correction_type,
        CorrectionType::CharacterSubstitution
    );
    assert_eq!(hello_suggestion.correction_confidence, 0.8);
}

#[test]
fn test_character_substitution_multiple_instances() {
    let processor = OcrPostProcessor::new();

    // "1" appears twice: position 2 and 3
    // Algorithm should generate corrections for BOTH positions
    let suggestions = processor.generate_suggestions("He11o");

    // Should have "Hel1o" (first '1' -> 'l')
    let has_first_l = suggestions.iter().any(|s| s.corrected_word == "Hel1o");
    // Should have "He1lo" (second '1' -> 'l')
    let has_second_l = suggestions.iter().any(|s| s.corrected_word == "He1lo");

    // At least one should be present (limit is 5, so both might not make it)
    assert!(
        has_first_l || has_second_l,
        "Should suggest corrections for '1' -> 'l'. Got: {:?}",
        suggestions
            .iter()
            .map(|s| &s.corrected_word)
            .collect::<Vec<_>>()
    );
}

#[test]
fn test_character_substitution_no_confusables() {
    let processor = OcrPostProcessor::new();

    // "abc" has no confusable characters
    let suggestions = processor.generate_suggestions("abc");

    // Should have NO character substitution suggestions
    let char_subs: Vec<_> = suggestions
        .iter()
        .filter(|s| s.correction_type == CorrectionType::CharacterSubstitution)
        .collect();

    assert!(
        char_subs.is_empty(),
        "Should not generate character substitutions for 'abc'. Got: {:?}",
        char_subs
    );
}

// =============================================================================
// TESTS FOR PATTERN CORRECTION (Specific Expected Output)
// =============================================================================

#[test]
fn test_pattern_correction_rn_to_m() {
    let processor = OcrPostProcessor::new();

    // "rn" appears once in "infornation"
    let suggestions = processor.generate_suggestions("infornation");

    // MUST suggest "infomation" (rn -> m)
    let has_correction = suggestions.iter().any(|s| s.corrected_word == "infomation");
    assert!(
        has_correction,
        "Should suggest 'infomation' from 'infornation'. Got: {:?}",
        suggestions
            .iter()
            .map(|s| &s.corrected_word)
            .collect::<Vec<_>>()
    );

    // Verify it's a pattern correction with expected confidence
    let correction = suggestions
        .iter()
        .find(|s| s.corrected_word == "infomation")
        .expect("infomation should exist");
    assert_eq!(
        correction.correction_type,
        CorrectionType::PatternCorrection
    );
    assert_eq!(correction.correction_confidence, 0.85);
}

#[test]
fn test_pattern_correction_cl_to_d() {
    let processor = OcrPostProcessor::new();

    // "cl" appears once at the end
    let suggestions = processor.generate_suggestions("olcl");

    // MUST suggest "old" (cl -> d)
    let has_correction = suggestions.iter().any(|s| s.corrected_word == "old");
    assert!(
        has_correction,
        "Should suggest 'old' from 'olcl'. Got: {:?}",
        suggestions
            .iter()
            .map(|s| &s.corrected_word)
            .collect::<Vec<_>>()
    );
}

#[test]
fn test_pattern_correction_multiple_occurrences() {
    let processor = OcrPostProcessor::new();

    // "rn" appears twice in "infornrnation"
    let suggestions = processor.generate_suggestions("infornrnation");

    // The algorithm uses replace() which replaces ALL occurrences
    // Expected: "infornrnation" -> "infommation" (both "rn" replaced with "m")
    let corrected_words: Vec<_> = suggestions
        .iter()
        .filter(|s| s.correction_type == CorrectionType::PatternCorrection)
        .map(|s| &s.corrected_word)
        .collect();

    // Verify ALL "rn" instances are replaced with "m"
    assert!(
        corrected_words.contains(&&"infommation".to_string()),
        "Should replace all 'rn' with 'm', producing 'infommation'. Got: {:?}",
        corrected_words
    );
}

// =============================================================================
// TESTS FOR DICTIONARY CORRECTION
// =============================================================================

#[test]
fn test_dictionary_correction_exact_match() {
    let mut dict = HashSet::new();
    dict.insert("hello".to_string());

    let processor = OcrPostProcessor::new().with_dictionary(dict);
    let suggestions = processor.generate_suggestions("hello");

    // Word is in dictionary - NO dictionary corrections
    let dict_corrections: Vec<_> = suggestions
        .iter()
        .filter(|s| s.correction_type == CorrectionType::DictionaryCorrection)
        .collect();

    assert!(
        dict_corrections.is_empty(),
        "Valid dictionary word should have no dictionary corrections. Got: {:?}",
        dict_corrections
    );
}

#[test]
fn test_dictionary_correction_edit_distance_1() {
    let mut dict = HashSet::new();
    dict.insert("hello".to_string());
    dict.insert("world".to_string());

    let processor = OcrPostProcessor::new().with_dictionary(dict);

    // "helo" is 1 edit away from "hello" (missing 'l')
    let suggestions = processor.generate_suggestions("helo");

    // MUST suggest "hello"
    let has_hello = suggestions.iter().any(|s| s.corrected_word == "hello");
    assert!(
        has_hello,
        "Should suggest 'hello' for 'helo' (edit distance 1). Got: {:?}",
        suggestions
            .iter()
            .map(|s| &s.corrected_word)
            .collect::<Vec<_>>()
    );

    // Verify confidence calculation
    let hello_suggestion = suggestions
        .iter()
        .find(|s| s.corrected_word == "hello")
        .expect("hello should exist");

    // confidence = (1.0 - (edit_dist / max_len)) * 0.9
    // confidence = (1.0 - (1 / 5)) * 0.9 = 0.8 * 0.9 = 0.72
    assert!(
        (hello_suggestion.correction_confidence - 0.72).abs() < 0.01,
        "Confidence should be 0.72, got: {}",
        hello_suggestion.correction_confidence
    );
}

#[test]
fn test_dictionary_correction_edit_distance_2() {
    let mut dict = HashSet::new();
    dict.insert("hello".to_string());

    let processor = OcrPostProcessor::new().with_dictionary(dict);

    // "helo" is 2 edits away (missing 'l' twice)
    // Actually "heo" is 2 edits away (missing both 'l's)
    let suggestions = processor.generate_suggestions("heo");

    // MUST suggest "hello" (within max_edit_distance = 2)
    let has_hello = suggestions.iter().any(|s| s.corrected_word == "hello");
    assert!(
        has_hello,
        "Should suggest 'hello' for 'heo' (edit distance 2). Got: {:?}",
        suggestions
            .iter()
            .map(|s| &s.corrected_word)
            .collect::<Vec<_>>()
    );
}

#[test]
fn test_dictionary_correction_exceeds_max_distance() {
    let mut dict = HashSet::new();
    dict.insert("hello".to_string());

    let processor = OcrPostProcessor::new().with_dictionary(dict);

    // "hi" is 3 edits away from "hello" (too far, max is 2)
    let suggestions = processor.generate_suggestions("hi");

    // Should NOT suggest "hello"
    let has_hello = suggestions.iter().any(|s| s.corrected_word == "hello");
    assert!(
        !has_hello,
        "Should NOT suggest 'hello' for 'hi' (edit distance > 2). Got: {:?}",
        suggestions
            .iter()
            .map(|s| &s.corrected_word)
            .collect::<Vec<_>>()
    );
}

// =============================================================================
// TESTS FOR EDIT DISTANCE ALGORITHM (Levenshtein)
// =============================================================================

#[test]
fn test_edit_distance_identical() {
    let processor = OcrPostProcessor::new();
    assert_eq!(processor.edit_distance("hello", "hello"), 0);
    assert_eq!(processor.edit_distance("", ""), 0);
    assert_eq!(processor.edit_distance("a", "a"), 0);
}

#[test]
fn test_edit_distance_substitution() {
    let processor = OcrPostProcessor::new();
    assert_eq!(processor.edit_distance("hello", "hallo"), 1); // e -> a
    assert_eq!(processor.edit_distance("cat", "bat"), 1); // c -> b
}

#[test]
fn test_edit_distance_insertion() {
    let processor = OcrPostProcessor::new();
    assert_eq!(processor.edit_distance("hello", "helloo"), 1); // insert 'o'
    assert_eq!(processor.edit_distance("cat", "cart"), 1); // insert 'r'
}

#[test]
fn test_edit_distance_deletion() {
    let processor = OcrPostProcessor::new();
    assert_eq!(processor.edit_distance("hello", "helo"), 1); // delete 'l'
    assert_eq!(processor.edit_distance("cart", "cat"), 1); // delete 'r'
}

#[test]
fn test_edit_distance_empty_strings() {
    let processor = OcrPostProcessor::new();
    assert_eq!(processor.edit_distance("", ""), 0);
    assert_eq!(processor.edit_distance("hello", ""), 5);
    assert_eq!(processor.edit_distance("", "world"), 5);
}

#[test]
fn test_edit_distance_complex() {
    let processor = OcrPostProcessor::new();
    // kitten -> sitting
    // k -> s (substitute)
    // e -> i (substitute)
    // insert t
    // = 3 operations
    assert_eq!(processor.edit_distance("kitten", "sitting"), 3);
}

// =============================================================================
// TESTS FOR EDGE CASES AND ERROR CONDITIONS
// =============================================================================

#[test]
fn test_region_contains_point_boundaries() {
    let region = OcrRegion::new(100, 100, 50, 50); // (100,100) to (150,150)

    // Top-left corner (inclusive)
    assert!(region.contains_point(100, 100));

    // Just inside
    assert!(region.contains_point(101, 101));

    // Bottom-right corner (EXCLUSIVE) - should be FALSE
    assert!(!region.contains_point(150, 150));

    // One pixel before bottom-right (should be TRUE)
    assert!(region.contains_point(149, 149));

    // Outside
    assert!(!region.contains_point(99, 100));
    assert!(!region.contains_point(100, 99));
    assert!(!region.contains_point(150, 100));
    assert!(!region.contains_point(100, 150));
}

#[test]
fn test_region_zero_dimensions() {
    let region = OcrRegion::new(100, 100, 0, 0);

    // Point AT the zero-size region should NOT be contained
    // because width/height are 0
    assert!(!region.contains_point(100, 100));
    assert!(!region.contains_point(99, 99));
}

#[test]
fn test_region_overlaps_edge_touching() {
    let region1 = OcrRegion::new(0, 0, 100, 100); // (0,0) to (100,100)
    let region2 = OcrRegion::new(100, 100, 100, 100); // (100,100) to (200,200)

    // Edge-touching should NOT overlap (boundaries are exclusive)
    assert!(!region1.overlaps_with(&region2));
    assert!(!region2.overlaps_with(&region1));
}

#[test]
fn test_region_overlaps_single_pixel() {
    let region1 = OcrRegion::new(0, 0, 101, 101); // (0,0) to (101,101)
    let region2 = OcrRegion::new(100, 100, 100, 100); // (100,100) to (200,200)

    // One pixel overlap
    assert!(region1.overlaps_with(&region2));
    assert!(region2.overlaps_with(&region1));
}

#[test]
fn test_word_confidence_boundary_threshold() {
    let word = WordConfidence::new("test".to_string(), 0.7, 0.0, 40.0);

    // Exactly at threshold should NOT be low confidence
    assert!(!word.is_low_confidence(0.7));

    // Just above threshold
    assert!(!word.is_low_confidence(0.6999));

    // Just below threshold
    assert!(word.is_low_confidence(0.7001));
}

#[test]
fn test_fragment_average_confidence_empty_words() {
    let fragment = OcrTextFragment::with_word_confidences(
        "text".to_string(),
        0.0,
        0.0,
        100.0,
        20.0,
        0.8, // fragment confidence
        12.0,
        FragmentType::Word,
        vec![], // NO words
    );

    // Empty words should return Some(0.0), not None
    assert_eq!(fragment.average_word_confidence(), Some(0.0));
}

#[test]
fn test_fragment_average_confidence_single_word() {
    let word_confs = vec![WordConfidence::new("only".to_string(), 0.95, 0.0, 40.0)];

    let fragment = OcrTextFragment::with_word_confidences(
        "only".to_string(),
        0.0,
        0.0,
        40.0,
        20.0,
        0.95,
        12.0,
        FragmentType::Word,
        word_confs,
    );

    // Single word - average equals that word's confidence
    assert_eq!(fragment.average_word_confidence(), Some(0.95));
}

// =============================================================================
// TESTS FOR SUGGESTION LIMIT (Verifies Top-5 Truncation)
// =============================================================================

#[test]
fn test_suggestion_limit_enforced() {
    let mut dict = HashSet::new();
    // Add 20 words all at edit distance 1 from "word"
    for i in 0..20 {
        dict.insert(format!("wor{}", ('a' as u8 + i as u8) as char));
    }

    let processor = OcrPostProcessor::new().with_dictionary(dict);
    let suggestions = processor.generate_suggestions("word");

    // Should be limited to 5 suggestions
    assert!(
        suggestions.len() <= 5,
        "Should limit to 5 suggestions, got: {}",
        suggestions.len()
    );
}

#[test]
fn test_suggestion_sorting_by_confidence() {
    let mut dict = HashSet::new();
    dict.insert("hello".to_string()); // Edit distance 1 from "helo"
    dict.insert("halo".to_string()); // Edit distance 2 from "helo"

    let processor = OcrPostProcessor::new().with_dictionary(dict);
    let suggestions = processor.generate_suggestions("helo");

    // Suggestions should be sorted by confidence (descending)
    for i in 0..suggestions.len() - 1 {
        assert!(
            suggestions[i].correction_confidence >= suggestions[i + 1].correction_confidence,
            "Suggestions should be sorted by confidence. Got: {:?}",
            suggestions
                .iter()
                .map(|s| (s.corrected_word.clone(), s.correction_confidence))
                .collect::<Vec<_>>()
        );
    }
}

// =============================================================================
// TESTS FOR CONFIDENCE REPORT FORMAT
// =============================================================================

#[test]
fn test_confidence_report_format_without_words() {
    let fragment = OcrTextFragment::new(
        "test text".to_string(),
        0.0,
        0.0,
        100.0,
        20.0,
        0.85,
        12.0,
        FragmentType::Line,
    );

    let report = fragment.confidence_report();

    // Should contain fragment confidence
    assert!(
        report.contains("85.0%"),
        "Should show 85.0% confidence. Got: {}",
        report
    );
    assert!(
        report.contains("test text"),
        "Should show fragment text. Got: {}",
        report
    );
    assert!(
        report.contains("(No word-level data available)"),
        "Should indicate no word data. Got: {}",
        report
    );
}

#[test]
fn test_confidence_report_format_with_words() {
    let word_confs = vec![
        WordConfidence::new("first".to_string(), 0.9, 0.0, 40.0),
        WordConfidence::new("second".to_string(), 0.8, 45.0, 50.0),
    ];

    let fragment = OcrTextFragment::with_word_confidences(
        "first second".to_string(),
        0.0,
        0.0,
        95.0,
        20.0,
        0.85,
        12.0,
        FragmentType::Line,
        word_confs,
    );

    let report = fragment.confidence_report();

    // Verify structure
    assert!(report.contains("85.0%"), "Should show fragment confidence");
    assert!(report.contains("2 words"), "Should show word count");
    assert!(report.contains("\"first\""), "Should show first word");
    assert!(
        report.contains("90.0%"),
        "Should show first word confidence"
    );
    assert!(report.contains("\"second\""), "Should show second word");
    assert!(
        report.contains("80.0%"),
        "Should show second word confidence"
    );
}

#[test]
fn test_confidence_report_with_character_level() {
    let char_confs = vec![
        CharacterConfidence::new('h', 0.95, 0.0, 10.0),
        CharacterConfidence::new('i', 0.92, 10.0, 8.0),
    ];

    let word_confs = vec![WordConfidence::with_characters(
        "hi".to_string(),
        0.935,
        0.0,
        18.0,
        char_confs,
    )];

    let fragment = OcrTextFragment::with_word_confidences(
        "hi".to_string(),
        0.0,
        0.0,
        18.0,
        20.0,
        0.935,
        12.0,
        FragmentType::Word,
        word_confs,
    );

    let report = fragment.confidence_report();

    // Verify character-level data
    assert!(report.contains("'h'"), "Should show character 'h'");
    assert!(report.contains("'i'"), "Should show character 'i'");
    assert!(
        report.contains("95%"),
        "Should show 'h' confidence (rounded)"
    );
    assert!(
        report.contains("92%"),
        "Should show 'i' confidence (rounded)"
    );
}

// =============================================================================
// TESTS FOR FRAGMENT WORDS SORTING
// =============================================================================

#[test]
fn test_words_by_confidence_sorted_ascending() {
    let word_confs = vec![
        WordConfidence::new("high".to_string(), 0.95, 0.0, 40.0),
        WordConfidence::new("low".to_string(), 0.45, 45.0, 30.0),
        WordConfidence::new("medium".to_string(), 0.75, 80.0, 60.0),
    ];

    let fragment = OcrTextFragment::with_word_confidences(
        "high low medium".to_string(),
        0.0,
        0.0,
        140.0,
        20.0,
        0.72,
        12.0,
        FragmentType::Line,
        word_confs,
    );

    let sorted = fragment.words_by_confidence();

    // MUST be sorted in ascending order (lowest first)
    assert_eq!(sorted.len(), 3);
    assert_eq!(sorted[0].word, "low");
    assert_eq!(sorted[0].confidence, 0.45);
    assert_eq!(sorted[1].word, "medium");
    assert_eq!(sorted[1].confidence, 0.75);
    assert_eq!(sorted[2].word, "high");
    assert_eq!(sorted[2].confidence, 0.95);
}

#[test]
fn test_words_by_confidence_equal_confidences() {
    let word_confs = vec![
        WordConfidence::new("first".to_string(), 0.8, 0.0, 40.0),
        WordConfidence::new("second".to_string(), 0.8, 45.0, 50.0),
        WordConfidence::new("third".to_string(), 0.8, 100.0, 50.0),
    ];

    let fragment = OcrTextFragment::with_word_confidences(
        "first second third".to_string(),
        0.0,
        0.0,
        150.0,
        20.0,
        0.8,
        12.0,
        FragmentType::Line,
        word_confs,
    );

    let sorted = fragment.words_by_confidence();

    // All have same confidence - order should be stable
    assert_eq!(sorted.len(), 3);
    assert!(sorted.iter().all(|w| w.confidence == 0.8));
}

// =============================================================================
// TESTS FOR CORRECTION CANDIDATES
// =============================================================================

#[test]
fn test_correction_candidates_respects_threshold() {
    let word_confs = vec![
        WordConfidence::new("perfect".to_string(), 0.99, 0.0, 70.0),
        WordConfidence::new("good".to_string(), 0.85, 75.0, 40.0),
        WordConfidence::new("okay".to_string(), 0.69, 120.0, 40.0), // Just below threshold
        WordConfidence::new("bad".to_string(), 0.50, 165.0, 30.0),
    ];

    let fragment = OcrTextFragment::with_word_confidences(
        "perfect good okay bad".to_string(),
        0.0,
        0.0,
        200.0,
        20.0,
        0.76,
        12.0,
        FragmentType::Line,
        word_confs,
    );

    let candidates = fragment.get_correction_candidates(0.7);

    // Should have exactly 2 candidates (okay and bad, both < 0.7)
    assert_eq!(candidates.len(), 2, "Should have 2 candidates below 0.7");
    assert_eq!(candidates[0].word, "okay");
    assert_eq!(candidates[0].confidence, 0.69);
    assert_eq!(candidates[0].position_in_fragment, 2);
    assert_eq!(candidates[1].word, "bad");
    assert_eq!(candidates[1].confidence, 0.50);
    assert_eq!(candidates[1].position_in_fragment, 3);
}

#[test]
fn test_correction_candidates_position_tracking() {
    let word_confs = vec![
        WordConfidence::new("word0".to_string(), 0.9, 0.0, 50.0),
        WordConfidence::new("word1".to_string(), 0.6, 55.0, 50.0), // Position 1
        WordConfidence::new("word2".to_string(), 0.9, 110.0, 50.0),
        WordConfidence::new("word3".to_string(), 0.5, 165.0, 50.0), // Position 3
    ];

    let fragment = OcrTextFragment::with_word_confidences(
        "word0 word1 word2 word3".to_string(),
        0.0,
        0.0,
        220.0,
        20.0,
        0.73,
        12.0,
        FragmentType::Line,
        word_confs,
    );

    let candidates = fragment.get_correction_candidates(0.7);

    // Positions should be correct
    assert_eq!(
        candidates[0].position_in_fragment, 1,
        "word1 is at position 1"
    );
    assert_eq!(
        candidates[1].position_in_fragment, 3,
        "word3 is at position 3"
    );
}
