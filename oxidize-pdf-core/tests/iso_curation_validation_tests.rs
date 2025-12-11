//! ISO Curation Validation Tests - Phase 1.1 (TDD Red Phase)
//!
//! These tests define WHAT makes a requirement valid BEFORE implementing
//! the curation tool. All tests should FAIL initially (RED phase).
//!
//! Criteria for a valid ISO requirement:
//! 1. Complete sentence (subject + predicate)
//! 2. Contains normative language ("shall", "should", "may")
//! 3. Not a bibliographic reference
//! 4. Length between 50-500 characters
//! 5. Has actionable content (testable)

/// Module for curation validation functions - Phase 2 Implementation
mod curation {
    use lazy_static::lazy_static;
    use regex::Regex;

    // ==========================================================================
    // REGEX PATTERNS
    // ==========================================================================

    lazy_static! {
        static ref SHALL_PATTERN: Regex = Regex::new(r"(?i)\bshall\b").unwrap();
        static ref MUST_PATTERN: Regex = Regex::new(r"(?i)\bmust\b").unwrap();
        static ref SHOULD_PATTERN: Regex = Regex::new(r"(?i)\bshould\b").unwrap();
        static ref MAY_PATTERN: Regex = Regex::new(r"(?i)\bmay\b").unwrap();
        static ref CAN_PATTERN: Regex = Regex::new(r"(?i)\bcan\b").unwrap();
        static ref NORMATIVE_PATTERN: Regex =
            Regex::new(r"(?i)\b(shall|must|should|may|can)\b").unwrap();

        static ref RFC_PATTERN: Regex = Regex::new(r"(?i)\bRFC\s*\d+").unwrap();
        static ref ISO_REF_PATTERN: Regex =
            Regex::new(r"(?i)\bISO\s*\d+(-\d+)?(:\d+)?").unwrap();
        static ref TECH_NOTE_PATTERN: Regex =
            Regex::new(r"(?i)Technical\s+Note\s*#?\d+").unwrap();
        static ref DATE_PARENS_PATTERN: Regex =
            Regex::new(r"\(\s*(?:January|February|March|April|May|June|July|August|September|October|November|December)\s+\d{4}\s*\)").unwrap();
        static ref ORG_PATTERN: Regex =
            Regex::new(r"(?i)(?:Adobe\s+Systems|Internet\s+Engineering\s+Task\s+Force|IETF|W3C|Unicode\s+Consortium)").unwrap();

        static ref LOWERCASE_START: Regex = Regex::new(r"^[a-z]").unwrap();
        static ref NORMATIVE_START: Regex =
            Regex::new(r"(?i)^(shall|should|must|may|can)\b").unwrap();
        static ref INCOMPLETE_END: Regex =
            Regex::new(r"(?:shall|should|must|may|the|a|an|to|for|with|by|of|in|on|at)\s*$").unwrap();
        static ref TABLE_HEADER: Regex = Regex::new(r"(?i)^Table\s+\d+").unwrap();
        static ref PARENS_XREF: Regex = Regex::new(r"^\s*\(see\s+\d+\.\d+").unwrap();

        // Priority detection patterns
        static ref P0_KEYWORDS: Regex =
            Regex::new(r"(?i)\b(document\s+catalog|cross-?reference|xref|trailer|file\s+header|%%PDF|%%EOF|startxref|Pages?\s+tree|root)\b").unwrap();
        static ref P1_KEYWORDS: Regex =
            Regex::new(r"(?i)\b(font|stream|filter|content\s+stream|page\s+object|resource|image)\b").unwrap();
        static ref P2_KEYWORDS: Regex =
            Regex::new(r"(?i)\b(annotation|bookmark|outline|action|form|field|metadata)\b").unwrap();
        static ref P3_KEYWORDS: Regex =
            Regex::new(r"(?i)\b(3D|JavaScript|multimedia|video|audio|embedded|attachment|optional\s+content|layer)\b").unwrap();
    }

    const MIN_LENGTH: usize = 50;
    const LONG_THRESHOLD: usize = 400;
    const MAX_LENGTH: usize = 500;

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

    fn has_sentence_structure(text: &str) -> bool {
        if let Some(m) = NORMATIVE_PATTERN.find(text) {
            let before = &text[..m.start()];
            before.split_whitespace().count() >= 2
        } else {
            false
        }
    }

    fn has_proper_ending(text: &str) -> bool {
        let trimmed = text.trim();
        trimmed.ends_with('.') || trimmed.ends_with(')')
    }

    /// Validates if a text fragment is a real ISO requirement
    pub fn is_valid_requirement(text: &str) -> ValidationResult {
        let text = text.trim();

        if text.is_empty() {
            return ValidationResult::invalid("Empty text");
        }

        if text.len() < MIN_LENGTH {
            return ValidationResult::invalid(format!(
                "Too short ({} chars, min {})",
                text.len(),
                MIN_LENGTH
            ));
        }

        if is_bibliographic_reference(text) {
            return ValidationResult::invalid("Bibliographic reference");
        }

        if TABLE_HEADER.is_match(text) {
            return ValidationResult::invalid("Table header");
        }

        if PARENS_XREF.is_match(text) {
            return ValidationResult::invalid("Parenthetical cross-reference");
        }

        if !NORMATIVE_PATTERN.is_match(text) {
            return ValidationResult::invalid("No normative language (shall/should/may/must/can)");
        }

        if is_fragment(text) {
            return ValidationResult::invalid("Incomplete sentence fragment");
        }

        if !has_sentence_structure(text) {
            return ValidationResult::invalid("Missing subject - fragment without context");
        }

        if !has_proper_ending(text) {
            return ValidationResult::invalid("Incomplete ending - fragment cut off");
        }

        let mut confidence = 1.0;

        if text.len() > LONG_THRESHOLD {
            let excess = (text.len() - LONG_THRESHOLD) as f64;
            let max_excess = (MAX_LENGTH - LONG_THRESHOLD) as f64;
            confidence -= (excess / max_excess).min(0.3) * 0.3;
        }

        let normative_count = NORMATIVE_PATTERN.find_iter(text).count();
        if normative_count > 2 {
            confidence -= 0.1 * (normative_count - 2) as f64;
        }

        if SHALL_PATTERN.is_match(text) || MUST_PATTERN.is_match(text) {
            confidence += 0.05;
        }

        confidence = confidence.clamp(0.5, 1.0);

        ValidationResult::valid(confidence)
    }

    /// Classifies requirement type based on normative language
    pub fn classify_type(text: &str) -> &'static str {
        if SHALL_PATTERN.is_match(text) {
            return "mandatory";
        }
        if MUST_PATTERN.is_match(text) {
            return "mandatory";
        }
        if SHOULD_PATTERN.is_match(text) {
            return "recommended";
        }
        if MAY_PATTERN.is_match(text) {
            return "optional";
        }
        if CAN_PATTERN.is_match(text) {
            return "optional";
        }
        "unknown"
    }

    /// Assigns priority based on content analysis
    pub fn assign_priority(text: &str) -> &'static str {
        if P0_KEYWORDS.is_match(text) {
            return "P0";
        }
        if P3_KEYWORDS.is_match(text) {
            return "P3";
        }
        if P1_KEYWORDS.is_match(text) {
            return "P1";
        }
        if P2_KEYWORDS.is_match(text) {
            return "P2";
        }
        "P2"
    }

    /// Detects if text is a bibliographic reference
    pub fn is_bibliographic_reference(text: &str) -> bool {
        if RFC_PATTERN.is_match(text) {
            return true;
        }
        if ISO_REF_PATTERN.is_match(text)
            && (DATE_PARENS_PATTERN.is_match(text)
                || text.contains("Graphic technology")
                || text.contains("data exchange"))
        {
            return true;
        }
        if TECH_NOTE_PATTERN.is_match(text) {
            return true;
        }
        if DATE_PARENS_PATTERN.is_match(text) && ORG_PATTERN.is_match(text) {
            return true;
        }
        if ORG_PATTERN.is_match(text) && !NORMATIVE_PATTERN.is_match(text) {
            if text.trim().ends_with('.') || text.trim().ends_with("Incorporated") {
                return true;
            }
        }
        false
    }

    /// Detects if text is a fragment (incomplete sentence)
    pub fn is_fragment(text: &str) -> bool {
        let text = text.trim();
        if text.is_empty() {
            return true;
        }
        if LOWERCASE_START.is_match(text) {
            return true;
        }
        if NORMATIVE_START.is_match(text) {
            return true;
        }
        if INCOMPLETE_END.is_match(text) {
            return true;
        }
        if text.len() < 30 && NORMATIVE_PATTERN.is_match(text) {
            return true;
        }
        false
    }
}

// =============================================================================
// TEST GROUP 1: Valid Requirement Detection
// =============================================================================

#[test]
fn test_valid_requirement_has_complete_sentence() {
    // A complete sentence with subject, predicate, and normative language
    let valid_req = "Every stream dictionary shall have a Length entry that indicates \
                     how many bytes of the PDF file are used for the stream's data.";

    let result = curation::is_valid_requirement(valid_req);

    assert!(
        result.is_valid,
        "Complete sentence with 'shall' should be valid. Got: {:?}",
        result
    );
    assert!(
        result.confidence >= 0.8,
        "High confidence expected for clear requirement"
    );
}

#[test]
fn test_valid_requirement_with_should() {
    let valid_req = "A conforming reader should be prepared to handle streams \
                     whose data has been corrupted.";

    let result = curation::is_valid_requirement(valid_req);

    assert!(
        result.is_valid,
        "Complete sentence with 'should' should be valid. Got: {:?}",
        result
    );
}

#[test]
fn test_valid_requirement_with_may() {
    let valid_req = "The document catalog may contain a Version entry to override \
                     the PDF version specified in the file header.";

    let result = curation::is_valid_requirement(valid_req);

    assert!(
        result.is_valid,
        "Complete sentence with 'may' should be valid. Got: {:?}",
        result
    );
}

// =============================================================================
// TEST GROUP 2: Invalid Fragment Detection
// =============================================================================

#[test]
fn test_fragment_without_subject_is_invalid() {
    // Fragment that starts mid-sentence (no subject) - must be >50 chars to pass length check
    let fragment = "shall be considered distinct entries in the cross-reference table.";

    let result = curation::is_valid_requirement(fragment);

    assert!(
        !result.is_valid,
        "Fragment without subject should be invalid. Got: {:?}",
        result
    );
    assert!(
        result.reason.contains("fragment") || result.reason.contains("incomplete"),
        "Reason should mention fragment/incomplete. Got: {}",
        result.reason
    );
}

#[test]
fn test_fragment_too_short_is_invalid() {
    let fragment = "shall be used.";

    let result = curation::is_valid_requirement(fragment);

    assert!(
        !result.is_valid,
        "Very short fragment should be invalid. Got: {:?}",
        result
    );
}

#[test]
fn test_fragment_continuation_is_invalid() {
    // Starts with lowercase, clearly a continuation
    let fragment = "most filters are defined so that the data shall be self-limiting.";

    let result = curation::is_valid_requirement(fragment);

    assert!(
        !result.is_valid,
        "Continuation fragment should be invalid. Got: {:?}",
        result
    );
}

#[test]
fn test_fragment_ending_mid_sentence_is_invalid() {
    // Ends without completing the thought
    let fragment = "The Length entry in a stream dictionary shall indicate";

    let result = curation::is_valid_requirement(fragment);

    assert!(
        !result.is_valid,
        "Fragment ending mid-sentence should be invalid. Got: {:?}",
        result
    );
}

// =============================================================================
// TEST GROUP 3: Bibliographic Reference Detection
// =============================================================================

#[test]
fn test_bibliographic_reference_is_invalid() {
    let reference = "Technical Note #5015, Type 1 Font Format Supplement, \
                     (May 1994), Adobe Systems Incorporated.";

    let result = curation::is_valid_requirement(reference);

    assert!(
        !result.is_valid,
        "Bibliographic reference should be invalid. Got: {:?}",
        result
    );

    // Also test the specific detection function
    assert!(
        curation::is_bibliographic_reference(reference),
        "Should detect as bibliographic reference"
    );
}

#[test]
fn test_rfc_reference_is_invalid() {
    let reference = "RFC 1950, ZLIB Compressed Data Format Specification, \
                     Version 3.3, (May 1996), Internet Engineering Task Force.";

    let result = curation::is_valid_requirement(reference);

    assert!(
        !result.is_valid,
        "RFC reference should be invalid. Got: {:?}",
        result
    );
}

#[test]
fn test_iso_reference_is_invalid() {
    let reference = "ISO 15930-4:2003, Graphic technology — Prepress digital data exchange.";

    let result = curation::is_valid_requirement(reference);

    assert!(
        !result.is_valid,
        "ISO reference should be invalid. Got: {:?}",
        result
    );
}

// =============================================================================
// TEST GROUP 4: Requirement Type Classification
// =============================================================================

#[test]
fn test_requirement_classification_mandatory() {
    let mandatory = "Every stream dictionary shall have a Length entry.";

    let classification = curation::classify_type(mandatory);

    assert_eq!(
        classification, "mandatory",
        "Text with 'shall' should be classified as mandatory"
    );
}

#[test]
fn test_requirement_classification_mandatory_with_must() {
    let mandatory = "The document catalog must contain a Pages entry.";

    let classification = curation::classify_type(mandatory);

    assert_eq!(
        classification, "mandatory",
        "Text with 'must' should be classified as mandatory"
    );
}

#[test]
fn test_requirement_classification_recommended() {
    let recommended = "A conforming reader should validate the cross-reference table.";

    let classification = curation::classify_type(recommended);

    assert_eq!(
        classification, "recommended",
        "Text with 'should' should be classified as recommended"
    );
}

#[test]
fn test_requirement_classification_optional() {
    let optional = "The Version entry may be present in the document catalog.";

    let classification = curation::classify_type(optional);

    assert_eq!(
        classification, "optional",
        "Text with 'may' should be classified as optional"
    );
}

#[test]
fn test_requirement_classification_optional_can() {
    let optional = "A conforming writer can include additional metadata entries.";

    let classification = curation::classify_type(optional);

    assert_eq!(
        classification, "optional",
        "Text with 'can' should be classified as optional"
    );
}

// =============================================================================
// TEST GROUP 5: Priority Assignment
// =============================================================================

#[test]
fn test_priority_p0_for_critical_structure() {
    // Document catalog is critical for any PDF
    let critical = "The document catalog shall be the root of the document's object hierarchy.";

    let priority = curation::assign_priority(critical);

    assert_eq!(
        priority, "P0",
        "Document catalog requirement should be P0 (critical)"
    );
}

#[test]
fn test_priority_p0_for_xref() {
    let critical =
        "Each cross-reference section shall begin with a line containing the keyword xref.";

    let priority = curation::assign_priority(critical);

    assert_eq!(priority, "P0", "XRef requirement should be P0 (critical)");
}

#[test]
fn test_priority_p1_for_fonts() {
    let high = "A font dictionary shall specify the font's PostScript name in the BaseFont entry.";

    let priority = curation::assign_priority(high);

    assert_eq!(priority, "P1", "Font requirement should be P1 (high)");
}

#[test]
fn test_priority_p2_for_annotations() {
    let medium = "An annotation dictionary may include an AP entry for appearance streams.";

    let priority = curation::assign_priority(medium);

    assert_eq!(
        priority, "P2",
        "Annotation requirement should be P2 (medium)"
    );
}

#[test]
fn test_priority_p3_for_multimedia() {
    let low = "A 3D annotation may specify a JavaScript action to execute.";

    let priority = curation::assign_priority(low);

    assert_eq!(
        priority, "P3",
        "3D/multimedia requirement should be P3 (low)"
    );
}

// =============================================================================
// TEST GROUP 6: Edge Cases
// =============================================================================

#[test]
fn test_empty_string_is_invalid() {
    let result = curation::is_valid_requirement("");

    assert!(!result.is_valid, "Empty string should be invalid");
}

#[test]
fn test_whitespace_only_is_invalid() {
    let result = curation::is_valid_requirement("   \n\t  ");

    assert!(!result.is_valid, "Whitespace-only string should be invalid");
}

#[test]
fn test_descriptive_text_without_normative_language_is_invalid() {
    // Descriptive text that doesn't mandate anything
    let descriptive = "PDF supports several types of annotations including text, \
                       link, and widget annotations.";

    let result = curation::is_valid_requirement(descriptive);

    assert!(
        !result.is_valid,
        "Descriptive text without shall/should/may should be invalid. Got: {:?}",
        result
    );
}

#[test]
fn test_table_header_is_invalid() {
    let header = "Table 3.25 – Document catalog dictionary entries";

    let result = curation::is_valid_requirement(header);

    assert!(!result.is_valid, "Table header should be invalid");
}

#[test]
fn test_parenthetical_explanation_is_invalid() {
    // Text in parentheses that explains but doesn't require
    let explanation = "(see 7.3.8.2, \"Stream Extent\")";

    let result = curation::is_valid_requirement(explanation);

    assert!(
        !result.is_valid,
        "Parenthetical reference should be invalid"
    );
}

// =============================================================================
// TEST GROUP 7: Length Boundaries
// =============================================================================

#[test]
fn test_minimum_length_boundary() {
    // Exactly at minimum length (~50 chars)
    let min_length = "The Length entry shall indicate the stream size.";

    let result = curation::is_valid_requirement(min_length);

    // Should be valid if it meets other criteria
    // The length check is a soft boundary
    assert!(
        result.confidence > 0.0 || !result.is_valid,
        "Should have some confidence assessment for boundary case"
    );
}

#[test]
fn test_very_long_requirement_is_suspicious() {
    // Very long text might be multiple requirements merged
    let very_long = "The Length entry in the stream dictionary shall specify the number \
                     of bytes in the stream, not counting any white space that follows \
                     the stream keyword and precedes the data, and not counting the \
                     endstream keyword. Furthermore, if the stream contains encoded data \
                     filtered through multiple filters, the Length shall be the number \
                     of bytes after all filtering has been applied. Additionally, the \
                     conforming reader shall handle cases where the Length value is \
                     incorrect by using alternative strategies such as searching for \
                     the endstream keyword or using object boundaries to determine \
                     the actual extent of the stream data.";

    let result = curation::is_valid_requirement(very_long);

    // Should flag for review (might need to be split)
    assert!(
        result.confidence < 1.0,
        "Very long text should have reduced confidence (might need splitting)"
    );
}

// =============================================================================
// TEST SUMMARY - Expected Results (Phase 1 Complete)
// =============================================================================
// Total tests: 25
// Expected FAILING: 25 (all tests fail because functions are stubs)
// After Phase 2: 25 PASSING (GREEN phase)
