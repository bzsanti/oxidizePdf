//! Key-value pair detection using pattern matching.

use super::types::{KeyValuePair, KeyValuePattern, StructuredDataConfig};
use crate::text::extraction::TextFragment;
use regex::Regex;

/// Detects key-value pairs in text fragments.
///
/// Uses three detection strategies:
/// 1. **Colon-separated**: "Label: Value" patterns
/// 2. **Spatial alignment**: "Label      Value" with significant horizontal gap
/// 3. **Tabular**: "Label\tValue" tab-separated patterns
pub fn detect_key_value_pairs(
    fragments: &[TextFragment],
    _config: &StructuredDataConfig,
) -> Vec<KeyValuePair> {
    let mut pairs = Vec::new();

    // Pattern 1: Colon-separated
    pairs.extend(detect_colon_pattern(fragments));

    // Pattern 2: Spatial alignment
    pairs.extend(detect_spatial_alignment(fragments));

    // Pattern 3: Tabular (tab-separated)
    pairs.extend(detect_tabular_pattern(fragments));

    pairs
}

/// Detects "Label: Value" patterns within single fragments.
fn detect_colon_pattern(fragments: &[TextFragment]) -> Vec<KeyValuePair> {
    let mut pairs = Vec::new();

    // Regex to match "Key: Value" pattern
    // Captures key before colon and value after colon
    let pattern = Regex::new(r"^([^:]+):\s*(.+)$").ok();

    if let Some(re) = pattern {
        for fragment in fragments {
            if let Some(captures) = re.captures(&fragment.text) {
                if captures.len() >= 3 {
                    let key = captures.get(1).map(|m| m.as_str().trim().to_string());
                    let value = captures.get(2).map(|m| m.as_str().trim().to_string());

                    if let (Some(k), Some(v)) = (key, value) {
                        if !k.is_empty() && !v.is_empty() {
                            pairs.push(KeyValuePair::new(
                                k,
                                v,
                                0.95, // High confidence for explicit colon pattern
                                KeyValuePattern::ColonSeparated,
                            ));
                        }
                    }
                }
            }
        }
    }

    pairs
}

/// Detects spatially aligned key-value pairs.
///
/// Algorithm:
/// 1. Group fragments by Y position (same line)
/// 2. For each line with 2 fragments, check if there's a significant horizontal gap
/// 3. If gap > threshold, treat as key-value pair
fn detect_spatial_alignment(fragments: &[TextFragment]) -> Vec<KeyValuePair> {
    let mut pairs = Vec::new();

    // Group fragments by Y position (tolerance of 3.0 units)
    let lines = group_by_y_position(fragments, 3.0);

    for line in lines {
        // Only consider lines with exactly 2 fragments for simple key-value
        if line.len() == 2 {
            let gap = line[1].x - (line[0].x + line[0].width);

            // Significant gap indicates separate columns
            if gap > 20.0 {
                pairs.push(KeyValuePair::new(
                    line[0].text.trim().to_string(),
                    line[1].text.trim().to_string(),
                    0.70, // Medium confidence for spatial pattern
                    KeyValuePattern::SpatialAlignment,
                ));
            }
        }
    }

    pairs
}

/// Detects tab-separated key-value pairs within fragments.
fn detect_tabular_pattern(fragments: &[TextFragment]) -> Vec<KeyValuePair> {
    let mut pairs = Vec::new();

    for fragment in fragments {
        // Check if text contains tab character
        if fragment.text.contains('\t') {
            let parts: Vec<&str> = fragment.text.split('\t').collect();

            // Simple key-value: exactly 2 parts
            if parts.len() == 2 {
                let key = parts[0].trim();
                let value = parts[1].trim();

                if !key.is_empty() && !value.is_empty() {
                    pairs.push(KeyValuePair::new(
                        key.to_string(),
                        value.to_string(),
                        0.85, // High confidence for explicit tab separation
                        KeyValuePattern::Tabular,
                    ));
                }
            }
        }
    }

    pairs
}

/// Groups text fragments by Y position.
///
/// Fragments with Y positions within `tolerance` are grouped together.
fn group_by_y_position(fragments: &[TextFragment], tolerance: f64) -> Vec<Vec<TextFragment>> {
    if fragments.is_empty() {
        return vec![];
    }

    let mut sorted = fragments.to_vec();
    sorted.sort_by(|a, b| {
        b.y.partial_cmp(&a.y)
            .unwrap()
            .then_with(|| a.x.partial_cmp(&b.x).unwrap())
    });

    let mut lines: Vec<Vec<TextFragment>> = vec![vec![sorted[0].clone()]];

    for fragment in &sorted[1..] {
        let last_line = lines.last_mut().unwrap();
        let last_y = last_line[0].y;

        if (fragment.y - last_y).abs() <= tolerance {
            last_line.push(fragment.clone());
        } else {
            lines.push(vec![fragment.clone()]);
        }
    }

    lines
}

/// Calculates confidence score for a key-value pair.
///
/// Based on pattern type and additional heuristics.
#[allow(dead_code)]
fn calculate_kv_confidence(pattern: KeyValuePattern, key: &str, value: &str) -> f64 {
    let base_confidence = match pattern {
        KeyValuePattern::ColonSeparated => 0.95,
        KeyValuePattern::SpatialAlignment => 0.70,
        KeyValuePattern::Tabular => 0.85,
    };

    // Reduce confidence if key or value is very short
    let length_penalty: f64 = if key.len() < 2 || value.len() < 2 {
        0.1
    } else {
        0.0
    };

    f64::max(base_confidence - length_penalty, 0.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_fragment(text: &str, x: f64, y: f64, width: f64) -> TextFragment {
        TextFragment {
            text: text.to_string(),
            x,
            y,
            width,
            height: 12.0,
            font_size: 12.0,
        }
    }

    #[test]
    fn test_detect_colon_simple() {
        let fragments = vec![create_fragment("Name: John Doe", 100.0, 700.0, 80.0)];

        let pairs = detect_colon_pattern(&fragments);

        assert_eq!(pairs.len(), 1);
        assert_eq!(pairs[0].key, "Name");
        assert_eq!(pairs[0].value, "John Doe");
        assert_eq!(pairs[0].pattern, KeyValuePattern::ColonSeparated);
        assert_eq!(pairs[0].confidence, 0.95);
    }

    #[test]
    fn test_detect_colon_multiple() {
        let fragments = vec![
            create_fragment("Name: John", 100.0, 700.0, 60.0),
            create_fragment("Age: 30", 100.0, 680.0, 50.0),
            create_fragment("City: NYC", 100.0, 660.0, 55.0),
        ];

        let pairs = detect_colon_pattern(&fragments);

        assert_eq!(pairs.len(), 3);
        assert_eq!(pairs[0].key, "Name");
        assert_eq!(pairs[1].key, "Age");
        assert_eq!(pairs[2].key, "City");
    }

    #[test]
    fn test_detect_colon_no_match() {
        let fragments = vec![
            create_fragment("Just text", 100.0, 700.0, 50.0),
            create_fragment("No colon here", 100.0, 680.0, 70.0),
        ];

        let pairs = detect_colon_pattern(&fragments);

        assert_eq!(pairs.len(), 0);
    }

    #[test]
    fn test_detect_spatial_alignment() {
        let fragments = vec![
            create_fragment("Name", 100.0, 700.0, 40.0),
            create_fragment("John Doe", 200.0, 700.0, 60.0), // Gap of 60.0
        ];

        let pairs = detect_spatial_alignment(&fragments);

        assert_eq!(pairs.len(), 1);
        assert_eq!(pairs[0].key, "Name");
        assert_eq!(pairs[0].value, "John Doe");
        assert_eq!(pairs[0].pattern, KeyValuePattern::SpatialAlignment);
        assert_eq!(pairs[0].confidence, 0.70);
    }

    #[test]
    fn test_detect_spatial_no_gap() {
        let fragments = vec![
            create_fragment("Name", 100.0, 700.0, 40.0),
            create_fragment("John", 145.0, 700.0, 30.0), // Gap of only 5.0
        ];

        let pairs = detect_spatial_alignment(&fragments);

        assert_eq!(pairs.len(), 0); // Gap too small
    }

    #[test]
    fn test_detect_tabular() {
        let fragments = vec![create_fragment("Name\tJohn Doe", 100.0, 700.0, 80.0)];

        let pairs = detect_tabular_pattern(&fragments);

        assert_eq!(pairs.len(), 1);
        assert_eq!(pairs[0].key, "Name");
        assert_eq!(pairs[0].value, "John Doe");
        assert_eq!(pairs[0].pattern, KeyValuePattern::Tabular);
        assert_eq!(pairs[0].confidence, 0.85);
    }

    #[test]
    fn test_detect_tabular_multiple_tabs() {
        let fragments = vec![create_fragment("A\tB\tC", 100.0, 700.0, 60.0)];

        let pairs = detect_tabular_pattern(&fragments);

        // Only detects simple 2-part pairs
        assert_eq!(pairs.len(), 0);
    }

    #[test]
    fn test_group_by_y_position() {
        let fragments = vec![
            create_fragment("A", 100.0, 700.0, 20.0),
            create_fragment("B", 150.0, 701.0, 20.0), // Same line (within tolerance)
            create_fragment("C", 100.0, 680.0, 20.0), // Different line
        ];

        let lines = group_by_y_position(&fragments, 3.0);

        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0].len(), 2); // A and B on same line
        assert_eq!(lines[1].len(), 1); // C on different line
    }

    #[test]
    fn test_calculate_kv_confidence() {
        assert_eq!(
            calculate_kv_confidence(KeyValuePattern::ColonSeparated, "Name", "John"),
            0.95
        );
        assert_eq!(
            calculate_kv_confidence(KeyValuePattern::SpatialAlignment, "Name", "John"),
            0.70
        );
        assert_eq!(
            calculate_kv_confidence(KeyValuePattern::Tabular, "Name", "John"),
            0.85
        );

        // Short key/value penalty
        assert_eq!(
            calculate_kv_confidence(KeyValuePattern::ColonSeparated, "N", "J"),
            0.85
        );
    }

    #[test]
    fn test_detect_key_value_pairs_integrated() {
        let config = StructuredDataConfig::default();
        let fragments = vec![
            create_fragment("Name: Alice", 100.0, 700.0, 70.0),
            create_fragment("Age", 100.0, 680.0, 30.0),
            create_fragment("25", 180.0, 680.0, 20.0),
            create_fragment("City\tBoston", 100.0, 660.0, 80.0),
        ];

        let pairs = detect_key_value_pairs(&fragments, &config);

        // Should detect:
        // 1. "Name: Alice" (colon)
        // 2. "Age" + "25" (spatial)
        // 3. "City\tBoston" (tabular)
        assert_eq!(pairs.len(), 3);

        let colon_pair = pairs
            .iter()
            .find(|p| p.pattern == KeyValuePattern::ColonSeparated);
        assert!(colon_pair.is_some());
        assert_eq!(colon_pair.unwrap().key, "Name");

        let spatial_pair = pairs
            .iter()
            .find(|p| p.pattern == KeyValuePattern::SpatialAlignment);
        assert!(spatial_pair.is_some());
        assert_eq!(spatial_pair.unwrap().key, "Age");

        let tabular_pair = pairs.iter().find(|p| p.pattern == KeyValuePattern::Tabular);
        assert!(tabular_pair.is_some());
        assert_eq!(tabular_pair.unwrap().key, "City");
    }
}
