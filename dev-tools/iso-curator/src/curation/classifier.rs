//! Requirement classification logic
//!
//! Classifies requirements by:
//! 1. Type: mandatory (shall/must), recommended (should), optional (may/can)
//! 2. Priority: P0 (critical), P1 (high), P2 (medium), P3 (low)
//! 3. Feature area: parser, writer, graphics, fonts, text, content, etc.

use super::patterns::*;

/// Classifies requirement type based on normative language
///
/// - "mandatory": Contains "shall" or "must"
/// - "recommended": Contains "should"
/// - "optional": Contains "may" or "can"
/// - "unknown": No normative language found
pub fn classify_type(text: &str) -> &'static str {
    // Priority: shall > must > should > may > can
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
///
/// - "P0": Critical document structure (catalog, xref, trailer, header)
/// - "P1": Core features (fonts, streams, content, pages)
/// - "P2": Common features (annotations, bookmarks, forms)
/// - "P3": Advanced/rare features (3D, JavaScript, multimedia)
pub fn assign_priority(text: &str) -> &'static str {
    // Check for P0 (critical) keywords first
    if P0_KEYWORDS.is_match(text) {
        return "P0";
    }

    // Check for P3 (low) keywords - these override P1/P2
    if P3_KEYWORDS.is_match(text) {
        return "P3";
    }

    // Check for P1 (high) keywords
    if P1_KEYWORDS.is_match(text) {
        return "P1";
    }

    // Check for P2 (medium) keywords
    if P2_KEYWORDS.is_match(text) {
        return "P2";
    }

    // Default to P2 (medium) for requirements without clear priority indicators
    "P2"
}

/// Detects the feature area for a requirement
///
/// Returns one of: parser, writer, graphics, fonts, text, content,
/// encryption, metadata, interactive, advanced
pub fn detect_feature_area(text: &str) -> &'static str {
    // Order matters - more specific patterns first

    // Check for encryption first (security-critical)
    if ENCRYPTION_KEYWORDS.is_match(text) {
        return "encryption";
    }

    // Check for advanced/multimedia features
    if ADVANCED_KEYWORDS.is_match(text) {
        return "advanced";
    }

    // Check for interactive features
    if INTERACTIVE_KEYWORDS.is_match(text) {
        return "interactive";
    }

    // Check for parser/structure (document organization)
    if PARSER_KEYWORDS.is_match(text) {
        return "parser";
    }

    // Check for fonts (text rendering depends on fonts)
    if FONT_KEYWORDS.is_match(text) {
        return "fonts";
    }

    // Check for text operations
    if TEXT_KEYWORDS.is_match(text) {
        return "text";
    }

    // Check for graphics
    if GRAPHICS_KEYWORDS.is_match(text) {
        return "graphics";
    }

    // Check for content streams
    if CONTENT_KEYWORDS.is_match(text) {
        return "content";
    }

    // Check for metadata
    if METADATA_KEYWORDS.is_match(text) {
        return "metadata";
    }

    // Check for writer (often mentioned with conforming writer)
    if WRITER_KEYWORDS.is_match(text) {
        return "writer";
    }

    // Default to parser (most general)
    "parser"
}

/// Returns score indicating how well text matches a feature area (0.0 - 1.0)
pub fn feature_area_score(text: &str, area: &str) -> f64 {
    let pattern = match area {
        "parser" => &PARSER_KEYWORDS,
        "writer" => &WRITER_KEYWORDS,
        "graphics" => &GRAPHICS_KEYWORDS,
        "fonts" => &FONT_KEYWORDS,
        "text" => &TEXT_KEYWORDS,
        "content" => &CONTENT_KEYWORDS,
        "encryption" => &ENCRYPTION_KEYWORDS,
        "metadata" => &METADATA_KEYWORDS,
        "interactive" => &INTERACTIVE_KEYWORDS,
        "advanced" => &ADVANCED_KEYWORDS,
        _ => return 0.0,
    };

    let matches = pattern.find_iter(text).count();
    if matches == 0 {
        return 0.0;
    }

    // Normalize by text length (more matches in shorter text = higher score)
    let word_count = text.split_whitespace().count();
    if word_count == 0 {
        return 0.0;
    }

    let score = matches as f64 / word_count as f64 * 10.0;
    score.min(1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    // ==========================================================================
    // Type Classification Tests
    // ==========================================================================

    #[test]
    fn test_classify_mandatory_shall() {
        assert_eq!(classify_type("Every stream dictionary shall have a Length entry."), "mandatory");
    }

    #[test]
    fn test_classify_mandatory_must() {
        assert_eq!(classify_type("The document catalog must contain a Pages entry."), "mandatory");
    }

    #[test]
    fn test_classify_recommended() {
        assert_eq!(classify_type("A conforming reader should validate the cross-reference table."), "recommended");
    }

    #[test]
    fn test_classify_optional_may() {
        assert_eq!(classify_type("The Version entry may be present in the document catalog."), "optional");
    }

    #[test]
    fn test_classify_optional_can() {
        assert_eq!(classify_type("A conforming writer can include additional metadata entries."), "optional");
    }

    #[test]
    fn test_classify_unknown() {
        assert_eq!(classify_type("PDF supports several annotation types."), "unknown");
    }

    // ==========================================================================
    // Priority Assignment Tests
    // ==========================================================================

    #[test]
    fn test_priority_p0_catalog() {
        let text = "The document catalog shall be the root of the document's object hierarchy.";
        assert_eq!(assign_priority(text), "P0");
    }

    #[test]
    fn test_priority_p0_xref() {
        let text = "Each cross-reference section shall begin with a line containing the keyword xref.";
        assert_eq!(assign_priority(text), "P0");
    }

    #[test]
    fn test_priority_p1_font() {
        let text = "A font dictionary shall specify the font's PostScript name in the BaseFont entry.";
        assert_eq!(assign_priority(text), "P1");
    }

    #[test]
    fn test_priority_p2_annotation() {
        let text = "An annotation dictionary may include an AP entry for appearance streams.";
        assert_eq!(assign_priority(text), "P2");
    }

    #[test]
    fn test_priority_p3_3d() {
        let text = "A 3D annotation may specify a JavaScript action to execute.";
        assert_eq!(assign_priority(text), "P3");
    }

    // ==========================================================================
    // Feature Area Tests
    // ==========================================================================

    #[test]
    fn test_feature_area_parser() {
        let text = "The document catalog shall contain a reference to the page tree.";
        assert_eq!(detect_feature_area(text), "parser");
    }

    #[test]
    fn test_feature_area_fonts() {
        let text = "A font dictionary shall include the BaseFont entry for the font name.";
        assert_eq!(detect_feature_area(text), "fonts");
    }

    #[test]
    fn test_feature_area_graphics() {
        let text = "The current transformation matrix (CTM) shall be applied to coordinates.";
        assert_eq!(detect_feature_area(text), "graphics");
    }

    #[test]
    fn test_feature_area_encryption() {
        let text = "The encrypt dictionary shall specify the security handler.";
        assert_eq!(detect_feature_area(text), "encryption");
    }

    #[test]
    fn test_feature_area_interactive() {
        let text = "A link annotation shall include a destination or action.";
        assert_eq!(detect_feature_area(text), "interactive");
    }

    #[test]
    fn test_feature_area_advanced() {
        let text = "A 3D annotation may include JavaScript for interaction.";
        assert_eq!(detect_feature_area(text), "advanced");
    }
}
