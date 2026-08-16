//! Tests for text sanitization in text extraction
//!
//! Issue #116: Extracted text contains NUL bytes (`\0`) and ETX (`\u{3}`)
//! where spaces should appear.
//!
//! These tests follow TDD methodology - written BEFORE implementation.

use oxidize_pdf::text::extraction::{
    sanitize_extracted_text, sanitize_extracted_text_with_policy, CarriageReturnHandling,
};

#[test]
fn test_sanitize_nul_etx_sequence() {
    // Issue #116: The pattern `\0\u{3}` appears between words
    let input = "a\0\u{3}sergeant\0\u{3}and\0\u{3}two";
    let expected = "a sergeant and two";
    assert_eq!(sanitize_extracted_text(input), expected);
}

#[test]
fn test_sanitize_nul_etx_sequence_complex() {
    // Full example from issue #116
    let input = "a\0\u{3}sergeant\0\u{3}and\0\u{3}tw o\0\u{3}constables\0\u{3}ignored";
    let expected = "a sergeant and tw o constables ignored";
    assert_eq!(sanitize_extracted_text(input), expected);
}

#[test]
fn test_sanitize_single_nul() {
    // NUL without ETX should also become space
    let input = "word\0another";
    let expected = "word another";
    assert_eq!(sanitize_extracted_text(input), expected);
}

#[test]
fn test_sanitize_multiple_nul() {
    // Multiple consecutive NULs should collapse to single space
    let input = "word\0\0\0another";
    let expected = "word another";
    assert_eq!(sanitize_extracted_text(input), expected);
}

#[test]
fn test_collapse_multiple_spaces() {
    // Multiple spaces should collapse to single space
    let input = "a    b";
    let expected = "a b";
    assert_eq!(sanitize_extracted_text(input), expected);
}

#[test]
fn test_default_removes_standalone_carriage_returns() {
    let input = "unix\nwindows\r\nclassic-mac\rend";
    let expected = "unix\nwindows\nclassic-macend";
    assert_eq!(sanitize_extracted_text(input), expected);
}

#[test]
fn test_normalize_line_ending_preserves_standalone_carriage_returns() {
    let input = "unix\nwindows\r\nclassic-mac\rend";
    let expected = "unix\nwindows\nclassic-mac\rend";
    assert_eq!(
        sanitize_extracted_text_with_policy(input, CarriageReturnHandling::NormalizeLineEnding),
        expected
    );
}

#[test]
fn test_remove_carriage_returns() {
    let input = "rating-aa\ra-exp\r\nnext";
    let expected = "rating-aaa-exp\nnext";
    assert_eq!(
        sanitize_extracted_text_with_policy(input, CarriageReturnHandling::Remove),
        expected
    );
}

#[test]
fn test_replace_carriage_returns_with_space() {
    let input = "rating-aa\ra-exp\r\nnext";
    let expected = "rating-aa a-exp\nnext";
    assert_eq!(
        sanitize_extracted_text_with_policy(input, CarriageReturnHandling::ReplaceWithSpace),
        expected
    );
}

#[test]
fn test_carriage_return_space_replacement_collapses_adjacent_spaces() {
    let input = "before \r  after";
    assert_eq!(
        sanitize_extracted_text_with_policy(input, CarriageReturnHandling::ReplaceWithSpace),
        "before after"
    );
}

#[test]
fn test_remove_control_characters() {
    // Control chars other than NUL (which becomes space) should be removed
    // SOH (0x01), STX (0x02), EOT (0x04), etc.
    let input = "text\u{1}\u{2}\u{4}more";
    let expected = "textmore";
    assert_eq!(sanitize_extracted_text(input), expected);
}

#[test]
fn test_etx_alone_removed() {
    // ETX (0x03) alone (not preceded by NUL) should be removed
    let input = "text\u{3}more";
    let expected = "textmore";
    assert_eq!(sanitize_extracted_text(input), expected);
}

#[test]
fn test_empty_string() {
    let input = "";
    let expected = "";
    assert_eq!(sanitize_extracted_text(input), expected);
}

#[test]
fn test_only_control_chars() {
    // String with only control chars should become empty or single space
    let input = "\0\u{1}\u{2}\u{3}\u{4}";
    let result = sanitize_extracted_text(input);
    // After sanitization: NUL->space, others removed, collapse spaces
    assert!(result.trim().is_empty() || result == " ");
}

#[test]
fn test_unicode_preservation() {
    // Unicode characters should be preserved
    let input = "中国人\0\u{3}test\0\u{3}日本語";
    let expected = "中国人 test 日本語";
    assert_eq!(sanitize_extracted_text(input), expected);
}

#[test]
fn test_no_change_clean_text() {
    // Clean text without control characters should pass through unchanged
    let input = "This is normal text with spaces.";
    let expected = "This is normal text with spaces.";
    assert_eq!(sanitize_extracted_text(input), expected);
}

#[test]
fn test_mixed_whitespace_and_control() {
    // Mix of normal spaces, NUL sequences, and control chars
    let input = "word1 word2\0\u{3}word3\u{1}word4";
    let expected = "word1 word2 word3word4";
    assert_eq!(sanitize_extracted_text(input), expected);
}

#[test]
fn test_leading_trailing_control_chars() {
    // Control chars at start/end
    let input = "\0\u{3}text\0\u{3}";
    let result = sanitize_extracted_text(input);
    // Should have leading/trailing space or be trimmed - implementation decides
    assert!(result.contains("text"));
    assert!(!result.contains('\0'));
    assert!(!result.contains('\u{3}'));
}
