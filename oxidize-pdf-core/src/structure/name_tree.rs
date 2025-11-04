//! Name tree structure according to ISO 32000-1 Section 7.9.6

use crate::error::{PdfError, Result};
use crate::objects::{Array, Dictionary, Object, ObjectId};
use std::collections::BTreeMap;

/// Name tree node
#[derive(Debug, Clone)]
pub struct NameTreeNode {
    /// Names in this node (leaf node)
    names: Option<BTreeMap<String, Object>>,
    /// Child nodes (intermediate node)
    kids: Option<Vec<ObjectId>>,
    /// Limits [min, max] for this node
    limits: Option<(String, String)>,
}

impl NameTreeNode {
    /// Create leaf node
    pub fn leaf(names: BTreeMap<String, Object>) -> Self {
        let limits =
            if let (Some(first), Some(last)) = (names.first_key_value(), names.last_key_value()) {
                Some((first.0.clone(), last.0.clone()))
            } else {
                None
            };

        Self {
            names: Some(names),
            kids: None,
            limits,
        }
    }

    /// Create intermediate node
    pub fn intermediate(kids: Vec<ObjectId>, limits: (String, String)) -> Self {
        Self {
            names: None,
            kids: Some(kids),
            limits: Some(limits),
        }
    }

    /// Convert to dictionary
    pub fn to_dict(&self) -> Dictionary {
        let mut dict = Dictionary::new();

        if let Some(names) = &self.names {
            // Leaf node - create Names array
            let mut names_array = Array::new();
            for (key, value) in names {
                names_array.push(Object::String(key.clone()));
                names_array.push(value.clone());
            }
            dict.set("Names", Object::Array(names_array.into()));
        }

        if let Some(kids) = &self.kids {
            // Intermediate node - create Kids array
            let kids_array: Array = kids.iter().map(|id| Object::Reference(*id)).collect();
            dict.set("Kids", Object::Array(kids_array.into()));
        }

        if let Some((min, max)) = &self.limits {
            let limits_array = Array::from(vec![
                Object::String(min.clone()),
                Object::String(max.clone()),
            ]);
            dict.set("Limits", Object::Array(limits_array.into()));
        }

        dict
    }
}

/// Name tree structure
pub struct NameTree {
    /// Root node
    root: NameTreeNode,
    /// All nodes (for complex trees)
    #[allow(dead_code)]
    nodes: BTreeMap<ObjectId, NameTreeNode>,
}

impl Default for NameTree {
    fn default() -> Self {
        Self::new()
    }
}

impl NameTree {
    /// Create new name tree with single leaf node
    pub fn new() -> Self {
        Self {
            root: NameTreeNode::leaf(BTreeMap::new()),
            nodes: BTreeMap::new(),
        }
    }

    /// Add name-value pair
    pub fn add(&mut self, name: String, value: Object) {
        if let Some(names) = &mut self.root.names {
            names.insert(name.clone(), value);

            // Update limits
            if let Some((min, max)) = &mut self.root.limits {
                if name < *min {
                    *min = name.clone();
                }
                if name > *max {
                    *max = name;
                }
            } else {
                self.root.limits = Some((name.clone(), name));
            }
        }
    }

    /// Get value by name
    pub fn get(&self, name: &str) -> Option<&Object> {
        self.root.names.as_ref()?.get(name)
    }

    /// Convert to dictionary
    pub fn to_dict(&self) -> Dictionary {
        self.root.to_dict()
    }

    /// Create from existing dictionary
    pub fn from_dict(dict: &Dictionary) -> Result<Self> {
        let mut tree = Self::new();

        if let Some(Object::Array(names_array)) = dict.get("Names") {
            let items: Vec<&Object> = names_array.iter().collect();

            if items.len() % 2 != 0 {
                return Err(PdfError::InvalidStructure(
                    "Names array must have even length".to_string(),
                ));
            }

            for i in (0..items.len()).step_by(2) {
                if let (Object::String(key), value) = (items[i], items[i + 1]) {
                    let key = key.clone();
                    tree.add(key, value.clone());
                }
            }
        }

        Ok(tree)
    }
}

/// Named destinations
pub struct NamedDestinations {
    /// Name tree for destinations
    tree: NameTree,
}

impl Default for NamedDestinations {
    fn default() -> Self {
        Self::new()
    }
}

impl NamedDestinations {
    /// Create new named destinations
    pub fn new() -> Self {
        Self {
            tree: NameTree::new(),
        }
    }

    /// Add named destination
    pub fn add_destination(&mut self, name: String, destination: Array) {
        self.tree.add(name, Object::Array(destination.into()));
    }

    /// Get destination by name
    pub fn get_destination(&self, name: &str) -> Option<Array> {
        match self.tree.get(name)? {
            Object::Array(arr) => Some(Array::from(arr.clone())),
            _ => None,
        }
    }

    /// Convert to dictionary
    pub fn to_dict(&self) -> Dictionary {
        self.tree.to_dict()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_name_tree_node_leaf() {
        let mut names = BTreeMap::new();
        names.insert("First".to_string(), Object::Integer(1));
        names.insert("Second".to_string(), Object::Integer(2));

        let node = NameTreeNode::leaf(names);
        assert!(node.names.is_some());
        assert!(node.kids.is_none());
        assert_eq!(
            node.limits,
            Some(("First".to_string(), "Second".to_string()))
        );
    }

    #[test]
    fn test_name_tree_add() {
        let mut tree = NameTree::new();
        tree.add("Apple".to_string(), Object::Integer(1));
        tree.add("Banana".to_string(), Object::Integer(2));
        tree.add("Cherry".to_string(), Object::Integer(3));

        assert_eq!(tree.get("Banana"), Some(&Object::Integer(2)));
        assert_eq!(
            tree.root.limits,
            Some(("Apple".to_string(), "Cherry".to_string()))
        );
    }

    #[test]
    fn test_name_tree_to_dict() {
        let mut tree = NameTree::new();
        tree.add("Test".to_string(), Object::Boolean(true));

        let dict = tree.to_dict();
        assert!(dict.get("Names").is_some());
    }

    #[test]
    fn test_named_destinations() {
        use crate::structure::destination::{Destination, PageDestination};

        let mut dests = NamedDestinations::new();
        let dest = Destination::fit(PageDestination::PageNumber(0));
        dests.add_destination("Home".to_string(), dest.to_array());

        let retrieved = dests.get_destination("Home");
        assert!(retrieved.is_some());
    }

    #[test]
    fn test_name_tree_from_dict() {
        let mut dict = Dictionary::new();
        let names_array = Array::from(vec![
            Object::String("First".to_string()),
            Object::Integer(1),
            Object::String("Second".to_string()),
            Object::Integer(2),
        ]);
        dict.set("Names", Object::Array(names_array.into()));

        let tree = NameTree::from_dict(&dict).unwrap();
        assert_eq!(tree.get("First"), Some(&Object::Integer(1)));
        assert_eq!(tree.get("Second"), Some(&Object::Integer(2)));
    }

    #[test]
    fn test_name_tree_node_debug_clone() {
        let mut names = BTreeMap::new();
        names.insert("Test".to_string(), Object::Boolean(true));
        let node = NameTreeNode::leaf(names);

        let debug_str = format!("{node:?}");
        assert!(debug_str.contains("NameTreeNode"));

        let cloned = node.clone();
        assert_eq!(cloned.limits, node.limits);
    }

    #[test]
    fn test_name_tree_node_intermediate() {
        let kids = vec![ObjectId::new(10, 0), ObjectId::new(20, 0)];
        let limits = ("Alpha".to_string(), "Zeta".to_string());

        let node = NameTreeNode::intermediate(kids.clone(), limits.clone());
        assert!(node.names.is_none());
        assert_eq!(node.kids, Some(kids));
        assert_eq!(node.limits, Some(limits));
    }

    #[test]
    fn test_name_tree_node_empty_leaf() {
        let node = NameTreeNode::leaf(BTreeMap::new());
        assert!(node.names.is_some());
        assert!(node.limits.is_none());

        let dict = node.to_dict();
        assert!(dict.get("Names").is_some());
        assert!(dict.get("Limits").is_none());
    }

    #[test]
    fn test_name_tree_node_to_dict_intermediate() {
        let kids = vec![ObjectId::new(5, 0), ObjectId::new(6, 0)];
        let limits = ("A".to_string(), "M".to_string());
        let node = NameTreeNode::intermediate(kids, limits);

        let dict = node.to_dict();
        assert!(dict.get("Kids").is_some());
        assert!(dict.get("Limits").is_some());
        assert!(dict.get("Names").is_none());

        // Verify Kids array
        match dict.get("Kids") {
            Some(Object::Array(arr)) => {
                assert_eq!(arr.len(), 2);
                assert!(matches!(arr.first(), Some(Object::Reference(_))));
            }
            _ => panic!("Kids should be an array"),
        }

        // Verify Limits array
        match dict.get("Limits") {
            Some(Object::Array(arr)) => {
                assert_eq!(arr.len(), 2);
                assert_eq!(arr.first(), Some(&Object::String("A".to_string())));
                assert_eq!(arr.get(1), Some(&Object::String("M".to_string())));
            }
            _ => panic!("Limits should be an array"),
        }
    }

    #[test]
    fn test_name_tree_default() {
        let tree = NameTree::default();
        assert!(tree.root.names.is_some());
        assert_eq!(tree.get("anything"), None);
    }

    #[test]
    fn test_name_tree_add_updates_limits() {
        let mut tree = NameTree::new();

        // Add first item
        tree.add("Middle".to_string(), Object::Integer(1));
        assert_eq!(
            tree.root.limits,
            Some(("Middle".to_string(), "Middle".to_string()))
        );

        // Add item before
        tree.add("Beginning".to_string(), Object::Integer(2));
        assert_eq!(
            tree.root.limits,
            Some(("Beginning".to_string(), "Middle".to_string()))
        );

        // Add item after
        tree.add("Zulu".to_string(), Object::Integer(3));
        assert_eq!(
            tree.root.limits,
            Some(("Beginning".to_string(), "Zulu".to_string()))
        );
    }

    #[test]
    fn test_name_tree_add_multiple() {
        let mut tree = NameTree::new();

        // Add items in non-alphabetical order
        let items = vec![
            ("Dog", Object::Integer(1)),
            ("Apple", Object::Integer(2)),
            ("Cat", Object::Integer(3)),
            ("Banana", Object::Integer(4)),
        ];

        for (name, value) in items {
            tree.add(name.to_string(), value);
        }

        // Verify all items are retrievable
        assert_eq!(tree.get("Dog"), Some(&Object::Integer(1)));
        assert_eq!(tree.get("Apple"), Some(&Object::Integer(2)));
        assert_eq!(tree.get("Cat"), Some(&Object::Integer(3)));
        assert_eq!(tree.get("Banana"), Some(&Object::Integer(4)));

        // Verify limits
        assert_eq!(
            tree.root.limits,
            Some(("Apple".to_string(), "Dog".to_string()))
        );
    }

    #[test]
    fn test_name_tree_get_nonexistent() {
        let mut tree = NameTree::new();
        tree.add("Exists".to_string(), Object::Boolean(true));

        assert!(tree.get("Exists").is_some());
        assert!(tree.get("DoesNotExist").is_none());
    }

    #[test]
    fn test_name_tree_to_dict_multiple_entries() {
        let mut tree = NameTree::new();
        tree.add("First".to_string(), Object::Integer(1));
        tree.add("Second".to_string(), Object::Integer(2));
        tree.add("Third".to_string(), Object::Integer(3));

        let dict = tree.to_dict();

        match dict.get("Names") {
            Some(Object::Array(arr)) => {
                assert_eq!(arr.len(), 6); // 3 key-value pairs = 6 elements
                                          // Names array should be: ["First", 1, "Second", 2, "Third", 3]
                assert_eq!(arr.first(), Some(&Object::String("First".to_string())));
                assert_eq!(arr.get(1), Some(&Object::Integer(1)));
                assert_eq!(arr.get(2), Some(&Object::String("Second".to_string())));
                assert_eq!(arr.get(3), Some(&Object::Integer(2)));
            }
            _ => panic!("Names should be an array"),
        }
    }

    #[test]
    fn test_name_tree_from_dict_empty() {
        let dict = Dictionary::new();
        let tree = NameTree::from_dict(&dict).unwrap();
        assert!(tree.root.names.is_some());
        assert_eq!(tree.get("anything"), None);
    }

    #[test]
    fn test_name_tree_from_dict_odd_length_array() {
        let mut dict = Dictionary::new();
        let names_array = Array::from(vec![
            Object::String("First".to_string()),
            Object::Integer(1),
            Object::String("Second".to_string()),
            // Missing value for "Second"
        ]);
        dict.set("Names", Object::Array(names_array.into()));

        let result = NameTree::from_dict(&dict);
        assert!(result.is_err());
    }

    #[test]
    fn test_name_tree_from_dict_non_string_keys() {
        let mut dict = Dictionary::new();
        let names_array = Array::from(vec![
            Object::Integer(123), // Not a string
            Object::Integer(1),
            Object::String("Valid".to_string()),
            Object::Integer(2),
        ]);
        dict.set("Names", Object::Array(names_array.into()));

        let tree = NameTree::from_dict(&dict).unwrap();
        // Should only have the valid entry
        assert_eq!(tree.get("Valid"), Some(&Object::Integer(2)));
        assert!(tree.get("123").is_none());
    }

    #[test]
    fn test_name_tree_from_dict_various_value_types() {
        let mut dict = Dictionary::new();
        let names_array = Array::from(vec![
            Object::String("Bool".to_string()),
            Object::Boolean(true),
            Object::String("Real".to_string()),
            Object::Real(std::f64::consts::PI),
            Object::String("Ref".to_string()),
            Object::Reference(ObjectId::new(5, 0)),
        ]);
        dict.set("Names", Object::Array(names_array.into()));

        let tree = NameTree::from_dict(&dict).unwrap();
        assert_eq!(tree.get("Bool"), Some(&Object::Boolean(true)));
        assert_eq!(tree.get("Real"), Some(&Object::Real(std::f64::consts::PI)));
        assert_eq!(
            tree.get("Ref"),
            Some(&Object::Reference(ObjectId::new(5, 0)))
        );
    }

    #[test]
    fn test_named_destinations_default() {
        let dests = NamedDestinations::default();
        assert!(dests.get_destination("anything").is_none());
    }

    #[test]
    fn test_named_destinations_add_and_get() {
        use crate::structure::destination::{Destination, PageDestination};

        let mut dests = NamedDestinations::new();

        // Add various types of destinations
        let dest1 = Destination::fit(PageDestination::PageNumber(0));
        let dest2 = Destination::xyz(
            PageDestination::PageNumber(5),
            Some(100.0),
            Some(200.0),
            None,
        );

        dests.add_destination("TOC".to_string(), dest1.to_array());
        dests.add_destination("Chapter1".to_string(), dest2.to_array());

        // Retrieve and verify
        let toc = dests.get_destination("TOC");
        assert!(toc.is_some());
        assert!(toc.unwrap().len() >= 2);

        let ch1 = dests.get_destination("Chapter1");
        assert!(ch1.is_some());
        assert!(ch1.unwrap().len() >= 5); // XYZ has 5 elements

        assert!(dests.get_destination("NotFound").is_none());
    }

    #[test]
    fn test_named_destinations_to_dict() {
        use crate::structure::destination::{Destination, PageDestination};

        let mut dests = NamedDestinations::new();
        let dest = Destination::fit(PageDestination::PageNumber(10));
        dests.add_destination("Appendix".to_string(), dest.to_array());

        let dict = dests.to_dict();
        assert!(dict.get("Names").is_some());
    }

    #[test]
    fn test_named_destinations_get_non_array_value() {
        let mut dests = NamedDestinations::new();
        // Manually add a non-array value to the tree
        dests.tree.add("Invalid".to_string(), Object::Integer(123));

        // Should return None for non-array values
        assert!(dests.get_destination("Invalid").is_none());
    }

    #[test]
    fn test_name_tree_case_sensitive() {
        let mut tree = NameTree::new();
        tree.add("Test".to_string(), Object::Integer(1));
        tree.add("test".to_string(), Object::Integer(2));
        tree.add("TEST".to_string(), Object::Integer(3));

        assert_eq!(tree.get("Test"), Some(&Object::Integer(1)));
        assert_eq!(tree.get("test"), Some(&Object::Integer(2)));
        assert_eq!(tree.get("TEST"), Some(&Object::Integer(3)));
    }

    #[test]
    fn test_name_tree_unicode_names() {
        let mut tree = NameTree::new();
        tree.add("café".to_string(), Object::Integer(1));
        tree.add("naïve".to_string(), Object::Integer(2));
        tree.add("日本語".to_string(), Object::Integer(3));
        tree.add("🎉".to_string(), Object::Integer(4));

        assert_eq!(tree.get("café"), Some(&Object::Integer(1)));
        assert_eq!(tree.get("naïve"), Some(&Object::Integer(2)));
        assert_eq!(tree.get("日本語"), Some(&Object::Integer(3)));
        assert_eq!(tree.get("🎉"), Some(&Object::Integer(4)));
    }

    #[test]
    fn test_name_tree_empty_string_key() {
        let mut tree = NameTree::new();
        tree.add("".to_string(), Object::Boolean(true));
        tree.add("Normal".to_string(), Object::Boolean(false));

        assert_eq!(tree.get(""), Some(&Object::Boolean(true)));
        assert_eq!(tree.get("Normal"), Some(&Object::Boolean(false)));
    }

    #[test]
    fn test_name_tree_overwrite_value() {
        let mut tree = NameTree::new();
        tree.add("Key".to_string(), Object::Integer(1));
        tree.add("Key".to_string(), Object::Integer(2)); // Overwrite

        assert_eq!(tree.get("Key"), Some(&Object::Integer(2)));
    }

    #[test]
    fn test_name_tree_dictionary_values() {
        let mut tree = NameTree::new();

        let mut dict_value = Dictionary::new();
        dict_value.set("Type", Object::Name("Test".to_string()));
        dict_value.set("Count", Object::Integer(42));

        tree.add(
            "DictEntry".to_string(),
            Object::Dictionary(dict_value.clone()),
        );

        match tree.get("DictEntry") {
            Some(Object::Dictionary(d)) => {
                assert_eq!(d.get("Type"), Some(&Object::Name("Test".to_string())));
                assert_eq!(d.get("Count"), Some(&Object::Integer(42)));
            }
            _ => panic!("Should get dictionary value"),
        }
    }

    #[test]
    fn test_name_tree_array_values() {
        let mut tree = NameTree::new();

        let array_value = Array::from(vec![
            Object::Integer(1),
            Object::Integer(2),
            Object::Integer(3),
        ]);

        tree.add("ArrayEntry".to_string(), Object::Array(array_value.into()));

        match tree.get("ArrayEntry") {
            Some(Object::Array(arr)) => {
                assert_eq!(arr.len(), 3);
                assert_eq!(arr.first(), Some(&Object::Integer(1)));
            }
            _ => panic!("Should get array value"),
        }
    }

    #[test]
    fn test_name_tree_btree_ordering() {
        let mut tree = NameTree::new();

        // Add items in random order
        let items = ["Zebra", "Alpha", "Mike", "Charlie", "Bravo"];
        for (i, name) in items.iter().enumerate() {
            tree.add((*name).to_string(), Object::Integer(i as i64));
        }

        // Convert to dict and check that names are in sorted order
        let dict = tree.to_dict();
        match dict.get("Names") {
            Some(Object::Array(arr)) => {
                // BTreeMap should maintain sorted order
                assert_eq!(arr.first(), Some(&Object::String("Alpha".to_string())));
                assert_eq!(arr.get(2), Some(&Object::String("Bravo".to_string())));
                assert_eq!(arr.get(4), Some(&Object::String("Charlie".to_string())));
                assert_eq!(arr.get(6), Some(&Object::String("Mike".to_string())));
                assert_eq!(arr.get(8), Some(&Object::String("Zebra".to_string())));
            }
            _ => panic!("Names should be an array"),
        }
    }
}
