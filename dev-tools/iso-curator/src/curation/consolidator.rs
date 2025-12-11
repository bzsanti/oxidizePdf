//! Fragment consolidation logic
//!
//! Groups related fragments by:
//! 1. Same ISO section (e.g., all 7.3.5.x fragments)
//! 2. Same topic within section
//! 3. Adjacent pages (likely related content)
//!
//! Merges fragments into unified requirements with:
//! - Semantic IDs (e.g., "7.3.5-stream-length")
//! - Combined descriptions
//! - Proper type/priority classification

use super::classifier::{assign_priority, classify_type, detect_feature_area};
use super::validator::is_valid_requirement;
use crate::matrix::FlatRequirement;
use once_cell::sync::Lazy;
use regex::Regex;

/// Pattern to extract section number from ISO section string
static SECTION_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^(\d+(?:\.\d+)*)").unwrap()
});

/// Pattern to extract key topic words
static TOPIC_WORDS: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)\b(stream|dictionary|entry|object|array|name|string|integer|boolean|null|reference|filter|length|type|subtype|catalog|page|font|annotation|action|destination|outline|metadata|xref|trailer|header|content|resource|image|color|path|text|glyph|encoding|encrypt|permission)\b").unwrap()
});

/// A group of related fragments that will be consolidated
#[derive(Debug, Clone)]
pub struct ConsolidationGroup {
    /// Section key (e.g., "7.3.5")
    pub section: String,
    /// Main topic detected
    pub topic: String,
    /// Fragment IDs in this group
    pub fragment_ids: Vec<String>,
    /// Fragment descriptions
    pub descriptions: Vec<String>,
    /// Page range covered
    pub page_range: (u32, u32),
    /// Cohesion score (0.0-1.0)
    pub cohesion_score: f64,
}

/// Result of consolidation - a curated requirement
#[derive(Debug, Clone)]
pub struct CuratedRequirement {
    /// Semantic ID (e.g., "7.3.5-stream-length")
    pub id: String,
    /// Human-readable name
    pub name: String,
    /// Consolidated description
    pub description: String,
    /// ISO section
    pub iso_section: String,
    /// Requirement type (mandatory/recommended/optional)
    pub requirement_type: String,
    /// Priority (P0/P1/P2/P3)
    pub priority: String,
    /// Feature area
    pub feature_area: String,
    /// Original fragment IDs consolidated
    pub consolidates: Vec<String>,
    /// Page range in ISO document
    pub page_range: (u32, u32),
}

/// Groups related fragments into consolidation groups
pub fn consolidate_fragments(fragments: &[FlatRequirement]) -> Vec<ConsolidationGroup> {
    // Group by ISO section first
    let mut section_groups: std::collections::HashMap<String, Vec<&FlatRequirement>> =
        std::collections::HashMap::new();

    for frag in fragments {
        let section = extract_section(&frag.iso_section);
        section_groups.entry(section).or_default().push(frag);
    }

    let mut result = Vec::new();

    // For each section, further group by topic
    for (section, section_frags) in section_groups {
        let topic_groups = group_by_topic(&section_frags);

        for (topic, topic_frags) in topic_groups {
            // Only create group if fragments are valid and cohesive
            let valid_frags: Vec<_> = topic_frags
                .iter()
                .filter(|f| is_valid_requirement(&f.description).is_valid)
                .collect();

            if valid_frags.is_empty() {
                continue;
            }

            let page_range = calculate_page_range(&valid_frags);
            let cohesion = calculate_cohesion(&valid_frags);

            result.push(ConsolidationGroup {
                section: section.clone(),
                topic: topic.clone(),
                fragment_ids: valid_frags.iter().map(|f| f.id.clone()).collect(),
                descriptions: valid_frags.iter().map(|f| f.description.clone()).collect(),
                page_range,
                cohesion_score: cohesion,
            });
        }
    }

    // Sort by section number
    result.sort_by(|a, b| {
        compare_sections(&a.section, &b.section)
    });

    result
}

/// Extract base section number from ISO section string
fn extract_section(iso_section: &str) -> String {
    if let Some(caps) = SECTION_PATTERN.captures(iso_section) {
        // Get first 2-3 levels (e.g., "7.3.5" from "7.3.5.1")
        let full = caps.get(1).map_or("", |m| m.as_str());
        let parts: Vec<_> = full.split('.').collect();
        if parts.len() <= 3 {
            return full.to_string();
        }
        parts[..3].join(".")
    } else {
        "unknown".to_string()
    }
}

/// Group fragments by topic keywords
fn group_by_topic<'a>(
    fragments: &[&'a FlatRequirement],
) -> std::collections::HashMap<String, Vec<&'a FlatRequirement>> {
    let mut groups: std::collections::HashMap<String, Vec<&'a FlatRequirement>> =
        std::collections::HashMap::new();

    for frag in fragments {
        let topic = detect_main_topic(&frag.description);
        groups.entry(topic).or_default().push(*frag);
    }

    groups
}

/// Detect the main topic from a description
fn detect_main_topic(text: &str) -> String {
    let mut topic_counts: std::collections::HashMap<String, usize> =
        std::collections::HashMap::new();

    for cap in TOPIC_WORDS.find_iter(text) {
        let word = cap.as_str().to_lowercase();
        *topic_counts.entry(word).or_insert(0) += 1;
    }

    // Return most frequent topic word, or "general" if none found
    topic_counts
        .into_iter()
        .max_by_key(|(_, count)| *count)
        .map(|(word, _)| word)
        .unwrap_or_else(|| "general".to_string())
}

/// Calculate page range for a set of fragments
fn calculate_page_range(fragments: &[&&FlatRequirement]) -> (u32, u32) {
    let pages: Vec<u32> = fragments.iter().map(|f| f.page).filter(|&p| p > 0).collect();

    if pages.is_empty() {
        return (0, 0);
    }

    let min = *pages.iter().min().unwrap_or(&0);
    let max = *pages.iter().max().unwrap_or(&0);
    (min, max)
}

/// Calculate cohesion score for a group of fragments (0.0 - 1.0)
fn calculate_cohesion(fragments: &[&&FlatRequirement]) -> f64 {
    if fragments.len() <= 1 {
        return 1.0;
    }

    // Factors affecting cohesion:
    // 1. Page proximity (closer pages = higher cohesion)
    // 2. Topic overlap (shared keywords = higher cohesion)
    // 3. Same requirement type (shall/should/may)

    let page_range = calculate_page_range(fragments);
    let page_spread = (page_range.1 - page_range.0) as f64;
    let page_score = 1.0 / (1.0 + page_spread / 5.0); // 5 pages spread = 0.5 score

    // Check if all have same normative language
    let types: std::collections::HashSet<_> = fragments
        .iter()
        .map(|f| classify_type(&f.description))
        .collect();
    let type_score = if types.len() == 1 { 1.0 } else { 0.7 };

    // Average scores
    (page_score + type_score) / 2.0
}

/// Compare section strings for sorting (e.g., "7.3.5" < "7.3.10" < "7.4")
fn compare_sections(a: &str, b: &str) -> std::cmp::Ordering {
    let a_parts: Vec<u32> = a.split('.').filter_map(|s| s.parse().ok()).collect();
    let b_parts: Vec<u32> = b.split('.').filter_map(|s| s.parse().ok()).collect();

    for (a_part, b_part) in a_parts.iter().zip(b_parts.iter()) {
        match a_part.cmp(b_part) {
            std::cmp::Ordering::Equal => continue,
            other => return other,
        }
    }
    a_parts.len().cmp(&b_parts.len())
}

/// Generate a semantic ID for a consolidated requirement
pub fn generate_semantic_id(section: &str, topic: &str, index: Option<u32>) -> String {
    let clean_topic = topic
        .to_lowercase()
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == ' ')
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join("-");

    let base = format!("{}-{}", section, clean_topic);

    if let Some(idx) = index {
        format!("{}-{}", base, idx)
    } else {
        base
    }
}

/// Merge a consolidation group into a curated requirement
pub fn merge_group(group: &ConsolidationGroup) -> CuratedRequirement {
    // Combine descriptions (deduplicate if similar)
    let merged_description = merge_descriptions(&group.descriptions);

    // Classify the merged text
    let req_type = classify_type(&merged_description);
    let priority = assign_priority(&merged_description);
    let feature_area = detect_feature_area(&merged_description);

    // Generate semantic ID
    let id = generate_semantic_id(&group.section, &group.topic, None);

    // Generate human-readable name
    let name = generate_name(&group.topic, &group.section);

    CuratedRequirement {
        id,
        name,
        description: merged_description,
        iso_section: group.section.clone(),
        requirement_type: req_type.to_string(),
        priority: priority.to_string(),
        feature_area: feature_area.to_string(),
        consolidates: group.fragment_ids.clone(),
        page_range: group.page_range,
    }
}

/// Merge multiple descriptions into one coherent text
fn merge_descriptions(descriptions: &[String]) -> String {
    if descriptions.len() == 1 {
        return descriptions[0].clone();
    }

    // For now, join with space (could be smarter about deduplication)
    descriptions.join(" ")
}

/// Generate a human-readable name from topic and section
fn generate_name(topic: &str, section: &str) -> String {
    // Capitalize topic
    let capitalized = topic
        .split_whitespace()
        .map(|w| {
            let mut chars = w.chars();
            match chars.next() {
                None => String::new(),
                Some(c) => c.to_uppercase().chain(chars).collect(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ");

    format!("{} ({})", capitalized, section)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_section() {
        assert_eq!(extract_section("7.3.5"), "7.3.5");
        assert_eq!(extract_section("7.3.5.1"), "7.3.5");
        assert_eq!(extract_section("7.3.5.1.2"), "7.3.5");
        assert_eq!(extract_section("7"), "7");
    }

    #[test]
    fn test_detect_main_topic() {
        assert_eq!(
            detect_main_topic("The stream dictionary shall have a Length entry."),
            "stream"
        );
        assert_eq!(
            detect_main_topic("The font dictionary shall include BaseFont."),
            "font"
        );
    }

    #[test]
    fn test_generate_semantic_id() {
        assert_eq!(generate_semantic_id("7.3.5", "stream", None), "7.3.5-stream");
        assert_eq!(
            generate_semantic_id("7.3.5", "stream length", None),
            "7.3.5-stream-length"
        );
        assert_eq!(
            generate_semantic_id("7.3.5", "stream", Some(1)),
            "7.3.5-stream-1"
        );
    }

    #[test]
    fn test_compare_sections() {
        assert_eq!(compare_sections("7.3.5", "7.3.10"), std::cmp::Ordering::Less);
        assert_eq!(compare_sections("7.3.5", "7.3.5"), std::cmp::Ordering::Equal);
        assert_eq!(compare_sections("7.4", "7.3.5"), std::cmp::Ordering::Greater);
    }

    #[test]
    fn test_generate_name() {
        assert_eq!(generate_name("stream", "7.3.5"), "Stream (7.3.5)");
        assert_eq!(
            generate_name("font dictionary", "9.1"),
            "Font Dictionary (9.1)"
        );
    }
}
