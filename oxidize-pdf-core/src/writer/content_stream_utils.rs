/// Utilities for analyzing and modifying PDF content streams
///
/// This module provides functions to extract font references, remap names,
/// and perform other content stream transformations needed for overlay operations.
use std::collections::HashSet;

/// Extract all font references from a content stream
///
/// Searches for "/Fx" patterns where x is a number or name, typically appearing
/// in font selection operators like "BT /F1 12 Tf (Hello) Tj ET"
///
/// # Arguments
/// * `content` - Raw content stream bytes
///
/// # Returns
/// Set of font names referenced in the stream (e.g., ["F1", "F2", "Arial"])
///
/// # Example
/// ```
/// let content = b"BT /F1 12 Tf (Hello) Tj /F2 10 Tf (World) Tj ET";
/// let fonts = extract_font_references(content);
/// assert!(fonts.contains("F1"));
/// assert!(fonts.contains("F2"));
/// ```
#[allow(dead_code)] // Will be used in Phase 2
pub fn extract_font_references(content: &[u8]) -> HashSet<String> {
    let mut font_names = HashSet::new();

    // Convert to string for easier parsing
    let content_str = String::from_utf8_lossy(content);

    // Look for "/FontName" patterns followed by Tf operator
    // Pattern: /FontName <number> Tf
    for line in content_str.lines() {
        let tokens: Vec<&str> = line.split_whitespace().collect();

        for (i, token) in tokens.iter().enumerate() {
            // Check if this is a font name (starts with /)
            if token.starts_with('/') {
                // Check if followed by number and Tf (font selection operator)
                if i + 2 < tokens.len() {
                    // tokens[i+1] should be number (size)
                    // tokens[i+2] should be "Tf"
                    if tokens[i + 2] == "Tf" {
                        // Extract font name (remove leading /)
                        let font_name = token[1..].to_string();
                        font_names.insert(font_name);
                    }
                }
            }
        }
    }

    font_names
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_font_references_simple() {
        let content = b"BT /F1 12 Tf (Hello) Tj ET";
        let fonts = extract_font_references(content);

        assert_eq!(fonts.len(), 1);
        assert!(fonts.contains("F1"));
    }

    #[test]
    fn test_extract_font_references_multiple() {
        let content = b"BT /F1 12 Tf (Hello) Tj ET BT /F2 10 Tf (World) Tj ET";
        let fonts = extract_font_references(content);

        assert_eq!(fonts.len(), 2);
        assert!(fonts.contains("F1"));
        assert!(fonts.contains("F2"));
    }

    #[test]
    fn test_extract_font_references_with_named_fonts() {
        let content = b"BT /ArialBold 14 Tf (Test) Tj /Helvetica 10 Tf (More) Tj ET";
        let fonts = extract_font_references(content);

        assert_eq!(fonts.len(), 2);
        assert!(fonts.contains("ArialBold"));
        assert!(fonts.contains("Helvetica"));
    }

    #[test]
    fn test_extract_font_references_multiline() {
        let content = b"BT\n/F1 12 Tf\n(Line 1) Tj\nET\nBT\n/F2 10 Tf\n(Line 2) Tj\nET";
        let fonts = extract_font_references(content);

        assert_eq!(fonts.len(), 2);
        assert!(fonts.contains("F1"));
        assert!(fonts.contains("F2"));
    }

    #[test]
    fn test_extract_font_references_no_fonts() {
        let content = b"100 200 m 300 400 l S";
        let fonts = extract_font_references(content);

        assert_eq!(fonts.len(), 0);
    }

    #[test]
    fn test_extract_font_references_ignore_false_positives() {
        // /Pattern shouldn't be detected as a font (not followed by Tf)
        let content = b"/Pattern cs /P1 scn 100 100 m 200 200 l S";
        let fonts = extract_font_references(content);

        assert_eq!(fonts.len(), 0);
    }
}
