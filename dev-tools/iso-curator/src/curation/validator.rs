//! Requirement validation logic
//!
//! Validates if a text fragment is a real ISO requirement based on:
//! 1. Complete sentence (subject + predicate)
//! 2. Contains normative language ("shall", "should", "may")
//! 3. Not a bibliographic reference
//! 4. Length between 50-500 characters
//! 5. Has actionable content (testable)

use super::patterns::*;

/// Minimum length for a valid requirement
const MIN_LENGTH: usize = 50;

/// Maximum length before flagging as potentially needing split
const MAX_LENGTH: usize = 500;

/// Length above which confidence is reduced
const LONG_THRESHOLD: usize = 400;

/// Result of validating a requirement
#[derive(Debug, Clone, PartialEq)]
pub struct ValidationResult {
    pub is_valid: bool,
    pub reason: String,
    pub confidence: f64,
}

impl ValidationResult {
    fn valid(confidence: f64) -> Self {
        Self {
            is_valid: true,
            reason: "Valid requirement".to_string(),
            confidence,
        }
    }

    fn invalid(reason: impl Into<String>) -> Self {
        Self {
            is_valid: false,
            reason: reason.into(),
            confidence: 0.0,
        }
    }
}

/// Validates if a text fragment is a real ISO requirement
pub fn is_valid_requirement(text: &str) -> ValidationResult {
    let text = text.trim();

    // Edge case: empty or whitespace only
    if text.is_empty() {
        return ValidationResult::invalid("Empty text");
    }

    // Length check - too short
    if text.len() < MIN_LENGTH {
        return ValidationResult::invalid(format!(
            "Too short ({} chars, min {})",
            text.len(),
            MIN_LENGTH
        ));
    }

    // Check for bibliographic reference
    if is_bibliographic_reference(text) {
        return ValidationResult::invalid("Bibliographic reference");
    }

    // Check for table header
    if TABLE_HEADER.is_match(text) {
        return ValidationResult::invalid("Table header");
    }

    // Check for parenthetical cross-reference
    if PARENS_XREF.is_match(text) {
        return ValidationResult::invalid("Parenthetical cross-reference");
    }

    // Must contain normative language
    if !NORMATIVE_PATTERN.is_match(text) {
        return ValidationResult::invalid("No normative language (shall/should/may/must/can)");
    }

    // Check for incomplete sentence (fragment)
    if is_fragment(text) {
        return ValidationResult::invalid("Incomplete sentence fragment");
    }

    // Must have proper sentence structure
    if !has_sentence_structure(text) {
        return ValidationResult::invalid("Missing subject - fragment without context");
    }

    // Must end properly
    if !has_proper_ending(text) {
        return ValidationResult::invalid("Incomplete ending - fragment cut off");
    }

    // Calculate confidence based on quality indicators
    let mut confidence = 1.0;

    // Reduce confidence for very long text (might need splitting)
    if text.len() > LONG_THRESHOLD {
        let excess = (text.len() - LONG_THRESHOLD) as f64;
        let max_excess = (MAX_LENGTH - LONG_THRESHOLD) as f64;
        confidence -= (excess / max_excess).min(0.3) * 0.3;
    }

    // Reduce confidence if multiple normative keywords (might be compound)
    let normative_count = NORMATIVE_PATTERN.find_iter(text).count();
    if normative_count > 2 {
        confidence -= 0.1 * (normative_count - 2) as f64;
    }

    // Boost confidence for clear mandatory requirements
    if SHALL_PATTERN.is_match(text) || MUST_PATTERN.is_match(text) {
        confidence += 0.05;
    }

    // Clamp confidence
    confidence = confidence.clamp(0.5, 1.0);

    ValidationResult::valid(confidence)
}

/// Detects if text is a bibliographic reference
pub fn is_bibliographic_reference(text: &str) -> bool {
    // Check for RFC reference
    if RFC_PATTERN.is_match(text) {
        return true;
    }

    // Check for ISO document reference (not our ISO 32000)
    // These are references TO other standards, typically formatted differently
    if ISO_REF_PATTERN.is_match(text) {
        // Check if it looks like a full reference (with title or date)
        if DATE_PARENS_PATTERN.is_match(text) || text.contains("Graphic technology") || text.contains("data exchange") {
            return true;
        }
    }

    // Check for Technical Note reference
    if TECH_NOTE_PATTERN.is_match(text) {
        return true;
    }

    // Check for date in parentheses (common in references)
    if DATE_PARENS_PATTERN.is_match(text) && ORG_PATTERN.is_match(text) {
        return true;
    }

    // Check for organization name without normative language (pure reference)
    if ORG_PATTERN.is_match(text) && !NORMATIVE_PATTERN.is_match(text) {
        // If it has organization name and ends with period but no normative language
        // it's likely a reference
        if text.trim().ends_with('.') || text.trim().ends_with("Incorporated") {
            return true;
        }
    }

    false
}

/// Detects if text is a fragment (incomplete sentence)
pub fn is_fragment(text: &str) -> bool {
    let text = text.trim();

    // Empty check
    if text.is_empty() {
        return true;
    }

    // Starts with lowercase (continuation)
    if LOWERCASE_START.is_match(text) {
        return true;
    }

    // Starts directly with normative keyword (no subject)
    if NORMATIVE_START.is_match(text) {
        return true;
    }

    // Ends without completing thought
    if INCOMPLETE_END.is_match(text) {
        return true;
    }

    // Very short but has normative language (likely fragment)
    if text.len() < 30 && NORMATIVE_PATTERN.is_match(text) {
        return true;
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_requirement_with_shall() {
        let text = "Every stream dictionary shall have a Length entry that indicates \
                    how many bytes of the PDF file are used for the stream's data.";
        let result = is_valid_requirement(text);
        assert!(result.is_valid, "Expected valid, got: {:?}", result);
        assert!(result.confidence >= 0.8);
    }

    #[test]
    fn test_valid_requirement_with_should() {
        let text = "A conforming reader should be prepared to handle streams \
                    whose data has been corrupted and recover gracefully from such errors.";
        let result = is_valid_requirement(text);
        assert!(result.is_valid, "Expected valid, got: {:?}", result);
    }

    #[test]
    fn test_valid_requirement_with_may() {
        let text = "The document catalog may contain a Version entry to override \
                    the PDF version specified in the file header.";
        let result = is_valid_requirement(text);
        assert!(result.is_valid, "Expected valid, got: {:?}", result);
    }

    #[test]
    fn test_fragment_without_subject() {
        let text = "shall be considered distinct.";
        let result = is_valid_requirement(text);
        assert!(!result.is_valid);
        assert!(result.reason.contains("fragment") || result.reason.contains("short"));
    }

    #[test]
    fn test_fragment_too_short() {
        let text = "shall be used.";
        let result = is_valid_requirement(text);
        assert!(!result.is_valid);
    }

    #[test]
    fn test_bibliographic_reference() {
        let text = "Technical Note #5015, Type 1 Font Format Supplement, \
                    (May 1994), Adobe Systems Incorporated.";
        let result = is_valid_requirement(text);
        assert!(!result.is_valid);
        assert!(is_bibliographic_reference(text));
    }

    #[test]
    fn test_rfc_reference() {
        let text = "RFC 1950, ZLIB Compressed Data Format Specification, \
                    Version 3.3, (May 1996), Internet Engineering Task Force.";
        let result = is_valid_requirement(text);
        assert!(!result.is_valid);
    }

    #[test]
    fn test_empty_string() {
        let result = is_valid_requirement("");
        assert!(!result.is_valid);
    }

    #[test]
    fn test_whitespace_only() {
        let result = is_valid_requirement("   \n\t  ");
        assert!(!result.is_valid);
    }

    #[test]
    fn test_descriptive_without_normative() {
        let text = "PDF supports several types of annotations including text, \
                    link, and widget annotations for various purposes.";
        let result = is_valid_requirement(text);
        assert!(!result.is_valid);
        assert!(result.reason.contains("normative"));
    }

    #[test]
    fn test_table_header() {
        let text = "Table 3.25 - Document catalog dictionary entries";
        let result = is_valid_requirement(text);
        assert!(!result.is_valid);
    }
}
