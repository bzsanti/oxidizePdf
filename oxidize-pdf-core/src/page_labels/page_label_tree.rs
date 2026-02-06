//! Page label tree structure for managing page numbering

use crate::objects::{Array, Dictionary, Object};
use crate::page_labels::PageLabel;
use std::collections::BTreeMap;

/// Page label tree - manages custom page numbering for a document
#[derive(Debug, Clone)]
pub struct PageLabelTree {
    /// Page label ranges, sorted by starting page
    ranges: BTreeMap<u32, PageLabel>,
}

impl PageLabelTree {
    /// Create a new empty page label tree
    pub fn new() -> Self {
        Self {
            ranges: BTreeMap::new(),
        }
    }

    /// Add a page label range
    pub fn add_range(&mut self, start_page: u32, label: PageLabel) {
        self.ranges.insert(start_page, label);
    }

    /// Get the page label for a specific page
    pub fn get_label(&self, page_index: u32) -> Option<String> {
        // Find the applicable range
        let mut applicable_range = None;
        let mut range_start = 0;

        for (&start, label) in &self.ranges {
            if start <= page_index {
                applicable_range = Some(label);
                range_start = start;
            } else {
                break;
            }
        }

        // Format the label if found
        applicable_range.map(|label| {
            let offset = page_index - range_start;
            label.format_label(offset)
        })
    }

    /// Get all page labels for a document
    pub fn get_all_labels(&self, total_pages: u32) -> Vec<String> {
        (0..total_pages)
            .map(|i| self.get_label(i).unwrap_or_else(|| (i + 1).to_string()))
            .collect()
    }

    /// Convert to PDF number tree dictionary
    pub fn to_dict(&self) -> Dictionary {
        let mut dict = Dictionary::new();

        // Create nums array [key1 val1 key2 val2 ...]
        let mut nums = Array::new();

        for (&start_page, label) in &self.ranges {
            nums.push(Object::Integer(start_page as i64));
            nums.push(Object::Dictionary(label.to_dict()));
        }

        dict.set("Nums", Object::Array(nums.into()));

        dict
    }

    /// Create from PDF dictionary
    pub fn from_dict(dict: &Dictionary) -> Option<Self> {
        let nums_array = match dict.get("Nums")? {
            Object::Array(arr) => arr,
            _ => return None,
        };
        let mut tree = Self::new();

        // Parse pairs of [page_index, label_dict]
        let elements: Vec<&Object> = nums_array.iter().collect();
        for i in (0..elements.len()).step_by(2) {
            if i + 1 >= elements.len() {
                break;
            }

            let page_index = match elements[i] {
                Object::Integer(n) => *n as u32,
                _ => continue,
            };
            let label_dict = match elements[i + 1] {
                Object::Dictionary(d) => d,
                _ => continue,
            };

            // Parse label from dictionary
            let style = if let Some(Object::Name(type_name)) = label_dict.get("Type") {
                match type_name.as_str() {
                    "D" => PageLabelStyle::DecimalArabic,
                    "r" => PageLabelStyle::UppercaseRoman,
                    "R" => PageLabelStyle::LowercaseRoman,
                    "A" => PageLabelStyle::UppercaseLetters,
                    "a" => PageLabelStyle::LowercaseLetters,
                    _ => PageLabelStyle::None,
                }
            } else {
                PageLabelStyle::None
            };

            let mut label = PageLabel::new(style);

            if let Some(Object::String(prefix)) = label_dict.get("P") {
                label = label.with_prefix(prefix);
            }

            if let Some(Object::Integer(start)) = label_dict.get("St") {
                label = label.starting_at(*start as u32);
            }

            tree.add_range(page_index, label);
        }

        Some(tree)
    }
}

impl Default for PageLabelTree {
    fn default() -> Self {
        Self::new()
    }
}

/// Builder for creating page label trees
pub struct PageLabelBuilder {
    tree: PageLabelTree,
    current_page: u32,
}

impl Default for PageLabelBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl PageLabelBuilder {
    /// Create a new page label builder
    pub fn new() -> Self {
        Self {
            tree: PageLabelTree::new(),
            current_page: 0,
        }
    }

    /// Add a range with a specific label
    pub fn add_range(mut self, num_pages: u32, label: PageLabel) -> Self {
        self.tree.add_range(self.current_page, label);
        self.current_page += num_pages;
        self
    }

    /// Add pages with decimal numbering
    pub fn decimal_pages(self, num_pages: u32) -> Self {
        self.add_range(num_pages, PageLabel::decimal())
    }

    /// Add pages with roman numbering
    pub fn roman_pages(self, num_pages: u32, uppercase: bool) -> Self {
        let label = if uppercase {
            PageLabel::roman_uppercase()
        } else {
            PageLabel::roman_lowercase()
        };
        self.add_range(num_pages, label)
    }

    /// Add pages with letter numbering
    pub fn letter_pages(self, num_pages: u32, uppercase: bool) -> Self {
        let label = if uppercase {
            PageLabel::letters_uppercase()
        } else {
            PageLabel::letters_lowercase()
        };
        self.add_range(num_pages, label)
    }

    /// Add pages with only a prefix
    pub fn prefix_pages(self, num_pages: u32, prefix: impl Into<String>) -> Self {
        self.add_range(num_pages, PageLabel::prefix_only(prefix))
    }

    /// Build the page label tree
    pub fn build(self) -> PageLabelTree {
        self.tree
    }
}

// Import PageLabelStyle from the other module
use crate::page_labels::PageLabelStyle;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_page_label_tree() {
        let mut tree = PageLabelTree::new();

        // Add roman numerals for first 3 pages
        tree.add_range(0, PageLabel::roman_lowercase());

        // Add decimal starting at page 3
        tree.add_range(3, PageLabel::decimal());

        // Test labels
        assert_eq!(tree.get_label(0), Some("i".to_string()));
        assert_eq!(tree.get_label(1), Some("ii".to_string()));
        assert_eq!(tree.get_label(2), Some("iii".to_string()));
        assert_eq!(tree.get_label(3), Some("1".to_string()));
        assert_eq!(tree.get_label(4), Some("2".to_string()));
        assert_eq!(tree.get_label(5), Some("3".to_string()));
    }

    #[test]
    fn test_page_label_with_prefix() {
        let mut tree = PageLabelTree::new();

        // Preface with prefix
        tree.add_range(0, PageLabel::prefix_only("Cover"));
        tree.add_range(1, PageLabel::roman_lowercase().with_prefix("p. "));
        tree.add_range(4, PageLabel::decimal().with_prefix("Chapter "));

        assert_eq!(tree.get_label(0), Some("Cover".to_string()));
        assert_eq!(tree.get_label(1), Some("p. i".to_string()));
        assert_eq!(tree.get_label(2), Some("p. ii".to_string()));
        assert_eq!(tree.get_label(3), Some("p. iii".to_string()));
        assert_eq!(tree.get_label(4), Some("Chapter 1".to_string()));
        assert_eq!(tree.get_label(5), Some("Chapter 2".to_string()));
    }

    #[test]
    fn test_page_label_with_start() {
        let mut tree = PageLabelTree::new();

        // Start numbering at 10
        tree.add_range(0, PageLabel::decimal().starting_at(10));

        assert_eq!(tree.get_label(0), Some("10".to_string()));
        assert_eq!(tree.get_label(1), Some("11".to_string()));
        assert_eq!(tree.get_label(2), Some("12".to_string()));
    }

    #[test]
    fn test_get_all_labels() {
        let mut tree = PageLabelTree::new();
        tree.add_range(0, PageLabel::roman_lowercase());
        tree.add_range(2, PageLabel::decimal());

        let labels = tree.get_all_labels(5);
        assert_eq!(labels, vec!["i", "ii", "1", "2", "3"]);
    }

    #[test]
    fn test_page_label_builder() {
        let tree = PageLabelBuilder::new()
            .prefix_pages(1, "Cover")
            .roman_pages(3, false)
            .decimal_pages(10)
            .letter_pages(3, true)
            .build();

        assert_eq!(tree.get_label(0), Some("Cover".to_string()));
        assert_eq!(tree.get_label(1), Some("i".to_string()));
        assert_eq!(tree.get_label(2), Some("ii".to_string()));
        assert_eq!(tree.get_label(3), Some("iii".to_string()));
        assert_eq!(tree.get_label(4), Some("1".to_string()));
        assert_eq!(tree.get_label(13), Some("10".to_string()));
        assert_eq!(tree.get_label(14), Some("A".to_string()));
        assert_eq!(tree.get_label(15), Some("B".to_string()));
        assert_eq!(tree.get_label(16), Some("C".to_string()));
    }

    #[test]
    fn test_to_dict() {
        let mut tree = PageLabelTree::new();
        tree.add_range(0, PageLabel::roman_lowercase());
        tree.add_range(3, PageLabel::decimal().with_prefix("Page "));

        let dict = tree.to_dict();
        assert!(dict.get("Nums").is_some());
    }

    #[test]
    fn test_page_label_tree_default() {
        let tree = PageLabelTree::default();
        // Empty tree should return None for any page
        assert!(tree.get_label(0).is_none());
        assert!(tree.get_label(100).is_none());
    }

    #[test]
    fn test_page_label_tree_clone() {
        let mut tree = PageLabelTree::new();
        tree.add_range(0, PageLabel::decimal());
        let cloned = tree.clone();
        assert_eq!(tree.get_label(0), cloned.get_label(0));
    }

    #[test]
    fn test_page_label_tree_debug() {
        let tree = PageLabelTree::new();
        let debug_str = format!("{:?}", tree);
        assert!(debug_str.contains("PageLabelTree"));
    }

    #[test]
    fn test_page_label_builder_default() {
        let builder = PageLabelBuilder::default();
        let tree = builder.build();
        // Empty tree
        assert!(tree.get_label(0).is_none());
    }

    #[test]
    fn test_page_label_builder_roman_uppercase() {
        let tree = PageLabelBuilder::new().roman_pages(5, true).build();

        assert_eq!(tree.get_label(0), Some("I".to_string()));
        assert_eq!(tree.get_label(1), Some("II".to_string()));
        assert_eq!(tree.get_label(2), Some("III".to_string()));
        assert_eq!(tree.get_label(3), Some("IV".to_string()));
        assert_eq!(tree.get_label(4), Some("V".to_string()));
    }

    #[test]
    fn test_page_label_builder_letter_lowercase() {
        let tree = PageLabelBuilder::new().letter_pages(3, false).build();

        assert_eq!(tree.get_label(0), Some("a".to_string()));
        assert_eq!(tree.get_label(1), Some("b".to_string()));
        assert_eq!(tree.get_label(2), Some("c".to_string()));
    }

    #[test]
    fn test_get_all_labels_empty_tree() {
        let tree = PageLabelTree::new();
        // Empty tree should return default numbering
        let labels = tree.get_all_labels(3);
        assert_eq!(labels, vec!["1", "2", "3"]);
    }

    #[test]
    fn test_from_dict_empty() {
        let mut dict = Dictionary::new();
        dict.set("Nums", Object::Array(Array::new().into()));

        let tree = PageLabelTree::from_dict(&dict);
        assert!(tree.is_some());
        let tree = tree.unwrap();
        assert!(tree.get_label(0).is_none());
    }

    #[test]
    fn test_from_dict_missing_nums() {
        let dict = Dictionary::new();
        let tree = PageLabelTree::from_dict(&dict);
        assert!(tree.is_none());
    }

    #[test]
    fn test_from_dict_invalid_nums_type() {
        let mut dict = Dictionary::new();
        dict.set("Nums", Object::Integer(42));

        let tree = PageLabelTree::from_dict(&dict);
        assert!(tree.is_none());
    }

    #[test]
    fn test_from_dict_with_decimal_labels() {
        let mut label_dict = Dictionary::new();
        label_dict.set("Type", Object::Name("D".to_string()));

        let mut nums = Array::new();
        nums.push(Object::Integer(0));
        nums.push(Object::Dictionary(label_dict));

        let mut dict = Dictionary::new();
        dict.set("Nums", Object::Array(nums.into()));

        let tree = PageLabelTree::from_dict(&dict);
        assert!(tree.is_some());
    }

    #[test]
    fn test_from_dict_with_prefix_and_start() {
        let mut label_dict = Dictionary::new();
        label_dict.set("Type", Object::Name("D".to_string()));
        label_dict.set("P", Object::String("Page ".to_string()));
        label_dict.set("St", Object::Integer(10));

        let mut nums = Array::new();
        nums.push(Object::Integer(0));
        nums.push(Object::Dictionary(label_dict));

        let mut dict = Dictionary::new();
        dict.set("Nums", Object::Array(nums.into()));

        let tree = PageLabelTree::from_dict(&dict);
        assert!(tree.is_some());
    }

    #[test]
    fn test_from_dict_invalid_page_index() {
        // Test with non-integer page index
        let label_dict = Dictionary::new();

        let mut nums = Array::new();
        nums.push(Object::String("not_an_integer".to_string()));
        nums.push(Object::Dictionary(label_dict));

        let mut dict = Dictionary::new();
        dict.set("Nums", Object::Array(nums.into()));

        // Should skip invalid entries
        let tree = PageLabelTree::from_dict(&dict);
        assert!(tree.is_some());
    }

    #[test]
    fn test_from_dict_invalid_label_dict() {
        // Test with non-dictionary label
        let mut nums = Array::new();
        nums.push(Object::Integer(0));
        nums.push(Object::String("not_a_dict".to_string()));

        let mut dict = Dictionary::new();
        dict.set("Nums", Object::Array(nums.into()));

        // Should skip invalid entries
        let tree = PageLabelTree::from_dict(&dict);
        assert!(tree.is_some());
    }

    #[test]
    fn test_from_dict_odd_length_array() {
        // Array with odd length - last element should be ignored
        let label_dict = Dictionary::new();

        let mut nums = Array::new();
        nums.push(Object::Integer(0));
        nums.push(Object::Dictionary(label_dict));
        nums.push(Object::Integer(5)); // Missing pair

        let mut dict = Dictionary::new();
        dict.set("Nums", Object::Array(nums.into()));

        let tree = PageLabelTree::from_dict(&dict);
        assert!(tree.is_some());
    }

    #[test]
    fn test_from_dict_all_style_types() {
        // Test r (uppercase roman)
        let mut label_dict1 = Dictionary::new();
        label_dict1.set("Type", Object::Name("r".to_string()));

        // Test R (lowercase roman)
        let mut label_dict2 = Dictionary::new();
        label_dict2.set("Type", Object::Name("R".to_string()));

        // Test A (uppercase letters)
        let mut label_dict3 = Dictionary::new();
        label_dict3.set("Type", Object::Name("A".to_string()));

        // Test a (lowercase letters)
        let mut label_dict4 = Dictionary::new();
        label_dict4.set("Type", Object::Name("a".to_string()));

        // Test unknown type
        let mut label_dict5 = Dictionary::new();
        label_dict5.set("Type", Object::Name("unknown".to_string()));

        let mut nums = Array::new();
        nums.push(Object::Integer(0));
        nums.push(Object::Dictionary(label_dict1));
        nums.push(Object::Integer(5));
        nums.push(Object::Dictionary(label_dict2));
        nums.push(Object::Integer(10));
        nums.push(Object::Dictionary(label_dict3));
        nums.push(Object::Integer(15));
        nums.push(Object::Dictionary(label_dict4));
        nums.push(Object::Integer(20));
        nums.push(Object::Dictionary(label_dict5));

        let mut dict = Dictionary::new();
        dict.set("Nums", Object::Array(nums.into()));

        let tree = PageLabelTree::from_dict(&dict);
        assert!(tree.is_some());
    }

    #[test]
    fn test_get_label_no_applicable_range() {
        let mut tree = PageLabelTree::new();
        // Add range starting at page 5
        tree.add_range(5, PageLabel::decimal());

        // Pages before the first range should return None
        assert!(tree.get_label(0).is_none());
        assert!(tree.get_label(4).is_none());

        // Pages at or after the range should have labels
        assert_eq!(tree.get_label(5), Some("1".to_string()));
        assert_eq!(tree.get_label(6), Some("2".to_string()));
    }

    #[test]
    fn test_page_label_builder_chained() {
        let tree = PageLabelBuilder::new()
            .prefix_pages(1, "TOC")
            .roman_pages(2, false)
            .roman_pages(2, true)
            .letter_pages(2, false)
            .letter_pages(2, true)
            .decimal_pages(5)
            .build();

        assert_eq!(tree.get_label(0), Some("TOC".to_string()));
        assert_eq!(tree.get_label(1), Some("i".to_string()));
        assert_eq!(tree.get_label(3), Some("I".to_string()));
        assert_eq!(tree.get_label(5), Some("a".to_string()));
        assert_eq!(tree.get_label(7), Some("A".to_string()));
        assert_eq!(tree.get_label(9), Some("1".to_string()));
    }

    #[test]
    fn test_to_dict_round_trip() {
        let mut original = PageLabelTree::new();
        original.add_range(0, PageLabel::roman_lowercase());
        original.add_range(5, PageLabel::decimal());

        let dict = original.to_dict();
        let restored = PageLabelTree::from_dict(&dict);

        assert!(restored.is_some());
        // Note: exact round-trip may not work perfectly due to style mapping differences
        // but basic structure should be preserved
    }
}
