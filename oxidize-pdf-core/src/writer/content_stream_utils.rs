/// Utilities for analyzing and modifying PDF content streams
///
/// This module provides functions to extract font references, remap names,
/// and perform other content stream transformations needed for overlay operations.
use std::collections::{HashMap, HashSet};

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
/// ```ignore
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

/// Rename fonts in a dictionary by adding a prefix
///
/// Takes a font dictionary and renames all font keys by adding "Orig" prefix.
/// This prevents naming conflicts between preserved fonts and overlay fonts.
///
/// # Arguments
/// * `fonts` - Font dictionary from preserved resources
///
/// # Returns
/// New dictionary with renamed fonts (/F1 → /OrigF1, /Arial → /OrigArial)
///
/// # Limitations
/// - Does not detect naming collisions if /OrigF1 already exists (rare but possible)
/// - Clones entire font dictionaries (acceptable for typical PDFs with <50 fonts)
/// - No validation that font names are valid PDF names
///
/// # Example
/// ```ignore
/// use std::collections::HashMap;
/// let mut fonts = HashMap::new();
/// fonts.insert("F1".to_string(), "font_dict_1");
/// fonts.insert("F2".to_string(), "font_dict_2");
///
/// let renamed = rename_preserved_fonts(&fonts);
/// assert!(renamed.contains_key("OrigF1"));
/// assert!(renamed.contains_key("OrigF2"));
/// ```
#[allow(dead_code)] // Will be used in Phase 2.3
pub fn rename_preserved_fonts(fonts: &crate::objects::Dictionary) -> crate::objects::Dictionary {
    let mut renamed = crate::objects::Dictionary::new();

    for (key, value) in fonts.iter() {
        // Add "Orig" prefix to font name
        let new_name = format!("Orig{}", key);
        renamed.set(new_name, value.clone());
    }

    renamed
}

/// Rewrite font references in a content stream using a name mapping
///
/// Searches for font selection operators ("/FontName size Tf") and replaces
/// the font names according to the provided mapping. This is used to update
/// content streams when fonts have been renamed to avoid conflicts.
///
/// # Arguments
/// * `content` - Original content stream bytes
/// * `mappings` - Map from old font names to new font names (e.g., "F1" → "OrigF1")
///
/// # Returns
/// New content stream with updated font references
///
/// # Limitations
/// - **Whitespace normalization**: Original whitespace (multiple spaces, tabs) is
///   normalized to single spaces. PDF remains valid but loses formatting fidelity.
/// - **Binary data risk**: Uses lossy UTF-8 conversion. Safe for text-only content streams,
///   but may corrupt streams with inline images or binary data (rare in practice).
/// - **Performance**: Creates complete copy of content stream. For very large streams
///   (>5MB), consider streaming approach.
/// - **No validation**: Does not verify that resulting PDF operators are valid.
///
/// # Example
/// ```ignore
/// use std::collections::HashMap;
/// let content = b"BT /F1 12 Tf (Hello) Tj ET";
/// let mut mappings = HashMap::new();
/// mappings.insert("F1".to_string(), "OrigF1".to_string());
///
/// let rewritten = rewrite_font_references(content, &mappings);
/// // Result: b"BT /OrigF1 12 Tf (Hello) Tj ET"
/// ```
#[allow(dead_code)] // Will be used in Phase 2.3
pub fn rewrite_font_references(content: &[u8], mappings: &HashMap<String, String>) -> Vec<u8> {
    let content_str = String::from_utf8_lossy(content);
    let mut result = String::new();

    for line in content_str.lines() {
        let tokens: Vec<&str> = line.split_whitespace().collect();
        let mut rewritten_line = String::new();

        let mut i = 0;
        while i < tokens.len() {
            let token = tokens[i];

            // Check if this is a font name (starts with /) followed by size and Tf
            if token.starts_with('/') && i + 2 < tokens.len() && tokens[i + 2] == "Tf" {
                // Extract font name (without leading /)
                let font_name = &token[1..];

                // Check if we have a mapping for this font
                if let Some(new_name) = mappings.get(font_name) {
                    // Write renamed font
                    rewritten_line.push('/');
                    rewritten_line.push_str(new_name);
                } else {
                    // Keep original font name
                    rewritten_line.push_str(token);
                }
            } else {
                // Not a font reference, keep as-is
                rewritten_line.push_str(token);
            }

            // Add space after token (except for last token)
            if i < tokens.len() - 1 {
                rewritten_line.push(' ');
            }

            i += 1;
        }

        result.push_str(&rewritten_line);
        result.push('\n');
    }

    // Remove trailing newline if original didn't have it
    if !content.ends_with(b"\n") && result.ends_with('\n') {
        result.pop();
    }

    result.into_bytes()
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

    // Tests for rename_preserved_fonts

    #[test]
    fn test_rename_preserved_fonts_simple() {
        use crate::objects::{Dictionary, Object};

        let mut fonts = Dictionary::new();
        fonts.set("F1", Object::Integer(1));
        fonts.set("F2", Object::Integer(2));

        let renamed = rename_preserved_fonts(&fonts);

        assert_eq!(renamed.len(), 2);
        assert!(renamed.contains_key("OrigF1"));
        assert!(renamed.contains_key("OrigF2"));
        assert!(!renamed.contains_key("F1")); // Original keys should not exist
        assert!(!renamed.contains_key("F2"));
    }

    #[test]
    fn test_rename_preserved_fonts_named_fonts() {
        use crate::objects::{Dictionary, Object};

        let mut fonts = Dictionary::new();
        fonts.set("Arial", Object::Integer(10));
        fonts.set("Helvetica", Object::Integer(20));
        fonts.set("TimesNewRoman", Object::Integer(30));

        let renamed = rename_preserved_fonts(&fonts);

        assert_eq!(renamed.len(), 3);
        assert!(renamed.contains_key("OrigArial"));
        assert!(renamed.contains_key("OrigHelvetica"));
        assert!(renamed.contains_key("OrigTimesNewRoman"));
    }

    #[test]
    fn test_rename_preserved_fonts_preserves_values() {
        use crate::objects::{Dictionary, Object};

        let mut fonts = Dictionary::new();
        fonts.set("F1", Object::Integer(42));
        fonts.set("Arial", Object::String("test".to_string()));

        let renamed = rename_preserved_fonts(&fonts);

        // Values should be preserved
        assert_eq!(renamed.get("OrigF1"), Some(&Object::Integer(42)));
        assert_eq!(
            renamed.get("OrigArial"),
            Some(&Object::String("test".to_string()))
        );
    }

    #[test]
    fn test_rename_preserved_fonts_empty_dictionary() {
        use crate::objects::Dictionary;

        let fonts = Dictionary::new();
        let renamed = rename_preserved_fonts(&fonts);

        assert_eq!(renamed.len(), 0);
    }

    #[test]
    fn test_rename_preserved_fonts_complex_objects() {
        use crate::objects::{Dictionary, Object};

        let mut fonts = Dictionary::new();

        // Create a complex font dictionary
        let mut font_dict = Dictionary::new();
        font_dict.set("Type", Object::Name("Font".to_string()));
        font_dict.set("Subtype", Object::Name("Type1".to_string()));
        font_dict.set("BaseFont", Object::Name("Helvetica".to_string()));

        fonts.set("F1", Object::Dictionary(font_dict.clone()));

        let renamed = rename_preserved_fonts(&fonts);

        assert_eq!(renamed.len(), 1);
        assert!(renamed.contains_key("OrigF1"));

        // Verify the complex object is preserved
        if let Some(Object::Dictionary(dict)) = renamed.get("OrigF1") {
            assert_eq!(dict.get("Type"), Some(&Object::Name("Font".to_string())));
            assert_eq!(
                dict.get("Subtype"),
                Some(&Object::Name("Type1".to_string()))
            );
            assert_eq!(
                dict.get("BaseFont"),
                Some(&Object::Name("Helvetica".to_string()))
            );
        } else {
            panic!("Expected dictionary object");
        }
    }

    #[test]
    fn test_rename_preserved_fonts_all_keys_prefixed() {
        use crate::objects::{Dictionary, Object};

        let mut fonts = Dictionary::new();
        fonts.set("F1", Object::Integer(1));
        fonts.set("F2", Object::Integer(2));
        fonts.set("Arial", Object::Integer(3));
        fonts.set("Helvetica", Object::Integer(4));

        let renamed = rename_preserved_fonts(&fonts);

        // Verify ALL keys have "Orig" prefix
        for key in renamed.keys() {
            assert!(
                key.starts_with("Orig"),
                "Key '{}' should start with 'Orig'",
                key
            );
        }
    }

    // Tests for rewrite_font_references

    #[test]
    fn test_rewrite_font_references_simple() {
        let content = b"BT /F1 12 Tf (Hello) Tj ET";
        let mut mappings = HashMap::new();
        mappings.insert("F1".to_string(), "OrigF1".to_string());

        let rewritten = rewrite_font_references(content, &mappings);
        let result = String::from_utf8(rewritten).unwrap();

        assert_eq!(result, "BT /OrigF1 12 Tf (Hello) Tj ET");
    }

    #[test]
    fn test_rewrite_font_references_multiple() {
        let content = b"BT /F1 12 Tf (Hello) Tj ET BT /F2 10 Tf (World) Tj ET";
        let mut mappings = HashMap::new();
        mappings.insert("F1".to_string(), "OrigF1".to_string());
        mappings.insert("F2".to_string(), "OrigF2".to_string());

        let rewritten = rewrite_font_references(content, &mappings);
        let result = String::from_utf8(rewritten).unwrap();

        assert_eq!(
            result,
            "BT /OrigF1 12 Tf (Hello) Tj ET BT /OrigF2 10 Tf (World) Tj ET"
        );
    }

    #[test]
    fn test_rewrite_font_references_named_fonts() {
        let content = b"BT /Arial 14 Tf (Test) Tj /Helvetica 10 Tf (More) Tj ET";
        let mut mappings = HashMap::new();
        mappings.insert("Arial".to_string(), "OrigArial".to_string());
        mappings.insert("Helvetica".to_string(), "OrigHelvetica".to_string());

        let rewritten = rewrite_font_references(content, &mappings);
        let result = String::from_utf8(rewritten).unwrap();

        assert_eq!(
            result,
            "BT /OrigArial 14 Tf (Test) Tj /OrigHelvetica 10 Tf (More) Tj ET"
        );
    }

    #[test]
    fn test_rewrite_font_references_multiline() {
        let content = b"BT\n/F1 12 Tf\n(Line 1) Tj\nET\nBT\n/F2 10 Tf\n(Line 2) Tj\nET";
        let mut mappings = HashMap::new();
        mappings.insert("F1".to_string(), "OrigF1".to_string());
        mappings.insert("F2".to_string(), "OrigF2".to_string());

        let rewritten = rewrite_font_references(content, &mappings);
        let result = String::from_utf8(rewritten).unwrap();

        assert!(result.contains("/OrigF1 12 Tf"));
        assert!(result.contains("/OrigF2 10 Tf"));
        assert!(!result.contains("/F1 12 Tf"));
        assert!(!result.contains("/F2 10 Tf"));
    }

    #[test]
    fn test_rewrite_font_references_partial_mapping() {
        // Only map F1, leave F2 unchanged
        let content = b"BT /F1 12 Tf (Hello) Tj /F2 10 Tf (World) Tj ET";
        let mut mappings = HashMap::new();
        mappings.insert("F1".to_string(), "OrigF1".to_string());

        let rewritten = rewrite_font_references(content, &mappings);
        let result = String::from_utf8(rewritten).unwrap();

        assert!(result.contains("/OrigF1 12 Tf"));
        assert!(result.contains("/F2 10 Tf")); // F2 unchanged
        assert!(!result.contains("/F1 12 Tf"));
    }

    #[test]
    fn test_rewrite_font_references_no_mappings() {
        let content = b"BT /F1 12 Tf (Hello) Tj ET";
        let mappings = HashMap::new();

        let rewritten = rewrite_font_references(content, &mappings);
        let result = String::from_utf8(rewritten).unwrap();

        // Should remain unchanged
        assert_eq!(result, "BT /F1 12 Tf (Hello) Tj ET");
    }

    #[test]
    fn test_rewrite_font_references_non_font_operators() {
        // Content with /Pattern (not a font)
        let content = b"/Pattern cs /P1 scn 100 100 m 200 200 l S";
        let mut mappings = HashMap::new();
        mappings.insert("Pattern".to_string(), "OrigPattern".to_string());

        let rewritten = rewrite_font_references(content, &mappings);
        let result = String::from_utf8(rewritten).unwrap();

        // /Pattern should NOT be rewritten (not followed by Tf)
        assert!(result.contains("/Pattern cs"));
        assert!(!result.contains("/OrigPattern"));
    }

    #[test]
    fn test_rewrite_font_references_preserves_other_content() {
        let content = b"100 200 m 300 400 l S BT /F1 12 Tf (Text) Tj ET q Q";
        let mut mappings = HashMap::new();
        mappings.insert("F1".to_string(), "OrigF1".to_string());

        let rewritten = rewrite_font_references(content, &mappings);
        let result = String::from_utf8(rewritten).unwrap();

        // Font should be rewritten
        assert!(result.contains("/OrigF1 12 Tf"));
        // Other operators preserved
        assert!(result.contains("100 200 m"));
        assert!(result.contains("300 400 l"));
        assert!(result.contains("(Text) Tj"));
    }

    // Edge case tests - documenting known limitations

    #[test]
    fn test_rewrite_font_references_normalizes_whitespace() {
        // DOCUMENTED LIMITATION: Whitespace is normalized
        let content = b"BT  /F1   12  Tf  (Text)  Tj  ET"; // Multiple spaces
        let mut mappings = HashMap::new();
        mappings.insert("F1".to_string(), "OrigF1".to_string());

        let rewritten = rewrite_font_references(content, &mappings);
        let result = String::from_utf8(rewritten).unwrap();

        // Font renamed correctly
        assert!(result.contains("/OrigF1 12 Tf"));
        // Whitespace normalized (not "  /OrigF1   12")
        assert!(!result.contains("  /OrigF1"));
        // PDF is still valid (single spaces are sufficient)
    }

    #[test]
    fn test_rewrite_font_references_with_indentation() {
        // DOCUMENTED LIMITATION: Indentation is lost
        let content = b"BT\n  /F1 12 Tf\n  100 700 Td\n  (Text) Tj\nET";
        let mut mappings = HashMap::new();
        mappings.insert("F1".to_string(), "OrigF1".to_string());

        let rewritten = rewrite_font_references(content, &mappings);
        let result = String::from_utf8(rewritten).unwrap();

        // Font renamed
        assert!(result.contains("/OrigF1 12 Tf"));
        // Original indentation lost (becomes single line tokens)
        // This is acceptable - PDF readers don't care about formatting
    }

    #[test]
    fn test_rename_preserved_fonts_no_collision_detection() {
        // DOCUMENTED LIMITATION: No collision detection
        use crate::objects::{Dictionary, Object};

        let mut fonts = Dictionary::new();
        fonts.set("F1", Object::Integer(1));
        fonts.set("OrigF1", Object::Integer(2)); // Already has "Orig" prefix!

        let renamed = rename_preserved_fonts(&fonts);

        // Both get renamed (collision not detected)
        assert!(renamed.contains_key("OrigF1")); // From original "OrigF1"
        assert!(renamed.contains_key("OrigOrigF1")); // From "F1"

        // This is acceptable - naming collisions are extremely rare in real PDFs
        // If needed, integration code can detect and handle this
    }

    #[test]
    fn test_rewrite_font_references_with_tabs() {
        // Tabs are normalized to spaces
        let content = b"BT\t/F1\t12\tTf\t(Text)\tTj\tET";
        let mut mappings = HashMap::new();
        mappings.insert("F1".to_string(), "OrigF1".to_string());

        let rewritten = rewrite_font_references(content, &mappings);
        let result = String::from_utf8(rewritten).unwrap();

        // Font renamed correctly despite tabs
        assert!(result.contains("/OrigF1 12 Tf"));
    }

    #[test]
    fn test_rewrite_font_references_hyphenated_font_names() {
        // Font names with hyphens (common in real PDFs: Arial-Bold, etc.)
        let content = b"BT /Arial-Bold 14 Tf (Text) Tj /Times-Italic 12 Tf (More) Tj ET";
        let mut mappings = HashMap::new();
        mappings.insert("Arial-Bold".to_string(), "OrigArial-Bold".to_string());
        mappings.insert("Times-Italic".to_string(), "OrigTimes-Italic".to_string());

        let rewritten = rewrite_font_references(content, &mappings);
        let result = String::from_utf8(rewritten).unwrap();

        assert!(result.contains("/OrigArial-Bold 14 Tf"));
        assert!(result.contains("/OrigTimes-Italic 12 Tf"));
    }
}
