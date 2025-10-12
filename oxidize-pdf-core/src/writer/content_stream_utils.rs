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
}
