//! Regex patterns for requirement detection and classification

use once_cell::sync::Lazy;
use regex::Regex;

// =============================================================================
// NORMATIVE LANGUAGE PATTERNS
// =============================================================================

/// Matches "shall" (mandatory)
pub static SHALL_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)\bshall\b").unwrap()
});

/// Matches "must" (mandatory)
pub static MUST_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)\bmust\b").unwrap()
});

/// Matches "should" (recommended)
pub static SHOULD_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)\bshould\b").unwrap()
});

/// Matches "may" (optional)
pub static MAY_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)\bmay\b").unwrap()
});

/// Matches "can" (optional)
pub static CAN_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)\bcan\b").unwrap()
});

/// Combined pattern for any normative language
pub static NORMATIVE_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)\b(shall|must|should|may|can)\b").unwrap()
});

// =============================================================================
// BIBLIOGRAPHIC REFERENCE PATTERNS
// =============================================================================

/// Matches RFC references (e.g., "RFC 1950")
pub static RFC_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)\bRFC\s*\d+").unwrap()
});

/// Matches ISO references (e.g., "ISO 15930-4:2003")
pub static ISO_REF_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)\bISO\s*\d+(-\d+)?(:\d+)?").unwrap()
});

/// Matches Technical Note references
pub static TECH_NOTE_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)Technical\s+Note\s*#?\d+").unwrap()
});

/// Matches date patterns typical in references (e.g., "(May 1996)")
pub static DATE_PARENS_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"\(\s*(?:January|February|March|April|May|June|July|August|September|October|November|December)\s+\d{4}\s*\)").unwrap()
});

/// Matches organization names typical in references
pub static ORG_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)(?:Adobe\s+Systems|Internet\s+Engineering\s+Task\s+Force|IETF|W3C|Unicode\s+Consortium)").unwrap()
});

// =============================================================================
// FRAGMENT DETECTION PATTERNS
// =============================================================================

/// Starts with lowercase (likely continuation)
pub static LOWERCASE_START: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^[a-z]").unwrap()
});

/// Starts with "shall", "should", etc. (no subject)
pub static NORMATIVE_START: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)^(shall|should|must|may|can)\b").unwrap()
});

/// Ends with incomplete thought (no period, ends with preposition, etc.)
pub static INCOMPLETE_END: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?:shall|should|must|may|the|a|an|to|for|with|by|of|in|on|at)\s*$").unwrap()
});

/// Table header pattern
pub static TABLE_HEADER: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)^Table\s+\d+").unwrap()
});

/// Section cross-reference in parentheses
pub static PARENS_XREF: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^\s*\(see\s+\d+\.\d+").unwrap()
});

// =============================================================================
// FEATURE AREA DETECTION PATTERNS
// =============================================================================

/// Parser/structure related keywords
pub static PARSER_KEYWORDS: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)\b(parser?|document\s+catalog|cross-?reference|xref|trailer|header|%%EOF|startxref|object\s+stream|indirect\s+object)\b").unwrap()
});

/// Writer related keywords
pub static WRITER_KEYWORDS: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)\b(writer?|conforming\s+writer|PDF\s+file|output|generate)\b").unwrap()
});

/// Graphics related keywords
pub static GRAPHICS_KEYWORDS: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)\b(graphics?|image|color\s+space|rendering|path|stroke|fill|transform|matrix|CTM)\b").unwrap()
});

/// Font related keywords
pub static FONT_KEYWORDS: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)\b(font|glyph|Type\s*[01]|TrueType|CID|encoding|cmap|ToUnicode|character)\b").unwrap()
});

/// Text related keywords
pub static TEXT_KEYWORDS: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)\b(text|string|Tj|TJ|Tc|Tw|Tf|Tm|T\*|BT|ET|text\s+object)\b").unwrap()
});

/// Content stream related keywords
pub static CONTENT_KEYWORDS: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)\b(content\s+stream|operator|operand|resource|XObject|Form)\b").unwrap()
});

/// Encryption related keywords
pub static ENCRYPTION_KEYWORDS: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)\b(encrypt|decrypt|security|password|permission|AES|RC4|handler)\b").unwrap()
});

/// Metadata related keywords
pub static METADATA_KEYWORDS: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)\b(metadata|XMP|Info|dictionary|Producer|Creator|Author|Title|Subject)\b").unwrap()
});

/// Interactive features keywords
pub static INTERACTIVE_KEYWORDS: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)\b(annotation|form|field|widget|action|link|bookmark|outline|dest)\b").unwrap()
});

/// Advanced/multimedia keywords
pub static ADVANCED_KEYWORDS: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)\b(3D|multimedia|video|audio|JavaScript|embedded|attachment|rich\s+media)\b").unwrap()
});

// =============================================================================
// PRIORITY DETECTION PATTERNS
// =============================================================================

/// Critical structure keywords (P0)
pub static P0_KEYWORDS: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)\b(document\s+catalog|cross-?reference|xref|trailer|file\s+header|%%PDF|%%EOF|startxref|Pages?\s+tree|root)\b").unwrap()
});

/// High priority keywords (P1)
pub static P1_KEYWORDS: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)\b(font|stream|filter|content\s+stream|page\s+object|resource|image)\b").unwrap()
});

/// Medium priority keywords (P2)
pub static P2_KEYWORDS: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)\b(annotation|bookmark|outline|action|form|field|metadata)\b").unwrap()
});

/// Low priority keywords (P3)
pub static P3_KEYWORDS: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)\b(3D|JavaScript|multimedia|video|audio|embedded|attachment|optional\s+content|layer)\b").unwrap()
});

// =============================================================================
// VALIDATION HELPERS
// =============================================================================

/// Check if text has a sentence structure (subject before predicate)
pub fn has_sentence_structure(text: &str) -> bool {
    // A proper sentence should have words before the normative language
    if let Some(m) = NORMATIVE_PATTERN.find(text) {
        let before = &text[..m.start()];
        // Should have at least 2 words before the normative keyword
        let word_count = before.split_whitespace().count();
        word_count >= 2
    } else {
        false
    }
}

/// Check if text ends with proper punctuation
pub fn has_proper_ending(text: &str) -> bool {
    let trimmed = text.trim();
    trimmed.ends_with('.') || trimmed.ends_with(')')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shall_pattern() {
        assert!(SHALL_PATTERN.is_match("The document shall contain"));
        assert!(SHALL_PATTERN.is_match("SHALL be"));
        assert!(!SHALL_PATTERN.is_match("marshall"));
    }

    #[test]
    fn test_rfc_pattern() {
        assert!(RFC_PATTERN.is_match("RFC 1950"));
        assert!(RFC_PATTERN.is_match("See RFC1234 for details"));
        assert!(!RFC_PATTERN.is_match("reference"));
    }

    #[test]
    fn test_sentence_structure() {
        assert!(has_sentence_structure("The document shall contain a catalog"));
        assert!(!has_sentence_structure("shall contain a catalog"));
        assert!(!has_sentence_structure("A shall")); // only 1 word before
    }

    #[test]
    fn test_proper_ending() {
        assert!(has_proper_ending("This is a sentence."));
        assert!(has_proper_ending("(see section 7.3)"));
        assert!(!has_proper_ending("shall indicate the"));
    }
}
