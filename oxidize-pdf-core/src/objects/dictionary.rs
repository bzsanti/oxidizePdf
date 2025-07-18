use crate::objects::Object;
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub struct Dictionary {
    entries: HashMap<String, Object>,
}

impl Dictionary {
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            entries: HashMap::with_capacity(capacity),
        }
    }

    pub fn set(&mut self, key: impl Into<String>, value: impl Into<Object>) {
        self.entries.insert(key.into(), value.into());
    }

    pub fn get(&self, key: &str) -> Option<&Object> {
        self.entries.get(key)
    }

    pub fn get_mut(&mut self, key: &str) -> Option<&mut Object> {
        self.entries.get_mut(key)
    }

    pub fn remove(&mut self, key: &str) -> Option<Object> {
        self.entries.remove(key)
    }

    pub fn contains_key(&self, key: &str) -> bool {
        self.entries.contains_key(key)
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn clear(&mut self) {
        self.entries.clear();
    }

    pub fn keys(&self) -> impl Iterator<Item = &String> {
        self.entries.keys()
    }

    pub fn values(&self) -> impl Iterator<Item = &Object> {
        self.entries.values()
    }

    pub fn entries(&self) -> impl Iterator<Item = (&String, &Object)> {
        self.entries.iter()
    }

    pub fn entries_mut(&mut self) -> impl Iterator<Item = (&String, &mut Object)> {
        self.entries.iter_mut()
    }

    pub fn iter(&self) -> impl Iterator<Item = (&String, &Object)> {
        self.entries.iter()
    }

    pub fn get_dict(&self, key: &str) -> Option<&Dictionary> {
        self.get(key).and_then(|obj| {
            if let Object::Dictionary(dict) = obj {
                Some(dict)
            } else {
                None
            }
        })
    }
}

impl Default for Dictionary {
    fn default() -> Self {
        Self::new()
    }
}

impl FromIterator<(String, Object)> for Dictionary {
    fn from_iter<T: IntoIterator<Item = (String, Object)>>(iter: T) -> Self {
        let mut dict = Dictionary::new();
        for (key, value) in iter {
            dict.set(key, value);
        }
        dict
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_dictionary() {
        let dict = Dictionary::new();
        assert!(dict.is_empty());
        assert_eq!(dict.len(), 0);
    }

    #[test]
    fn test_with_capacity() {
        let dict = Dictionary::with_capacity(10);
        assert!(dict.is_empty());
        assert_eq!(dict.len(), 0);
    }

    #[test]
    fn test_set_and_get() {
        let mut dict = Dictionary::new();
        dict.set("Name", "Test");
        dict.set("Age", 42);
        dict.set("Active", true);

        assert_eq!(dict.get("Name"), Some(&Object::String("Test".to_string())));
        assert_eq!(dict.get("Age"), Some(&Object::Integer(42)));
        assert_eq!(dict.get("Active"), Some(&Object::Boolean(true)));
        assert_eq!(dict.get("Missing"), None);
    }

    #[test]
    fn test_get_mut() {
        let mut dict = Dictionary::new();
        dict.set("Counter", 1);

        if let Some(Object::Integer(val)) = dict.get_mut("Counter") {
            *val = 2;
        }

        assert_eq!(dict.get("Counter"), Some(&Object::Integer(2)));
    }

    #[test]
    fn test_remove() {
        let mut dict = Dictionary::new();
        dict.set("Temp", "Value");

        assert!(dict.contains_key("Temp"));
        let removed = dict.remove("Temp");
        assert_eq!(removed, Some(Object::String("Value".to_string())));
        assert!(!dict.contains_key("Temp"));
        assert_eq!(dict.remove("Temp"), None);
    }

    #[test]
    fn test_contains_key() {
        let mut dict = Dictionary::new();
        dict.set("Exists", true);

        assert!(dict.contains_key("Exists"));
        assert!(!dict.contains_key("NotExists"));
    }

    #[test]
    fn test_clear() {
        let mut dict = Dictionary::new();
        dict.set("A", 1);
        dict.set("B", 2);
        dict.set("C", 3);

        assert_eq!(dict.len(), 3);
        dict.clear();
        assert_eq!(dict.len(), 0);
        assert!(dict.is_empty());
    }

    #[test]
    fn test_keys() {
        let mut dict = Dictionary::new();
        dict.set("First", 1);
        dict.set("Second", 2);
        dict.set("Third", 3);

        let mut keys: Vec<_> = dict.keys().collect();
        keys.sort();
        assert_eq!(keys, vec!["First", "Second", "Third"]);
    }

    #[test]
    fn test_values() {
        let mut dict = Dictionary::new();
        dict.set("A", 100);
        dict.set("B", 200);

        let values: Vec<_> = dict.values().collect();
        assert!(values.contains(&&Object::Integer(100)));
        assert!(values.contains(&&Object::Integer(200)));
    }

    #[test]
    fn test_entries_and_iter() {
        let mut dict = Dictionary::new();
        dict.set("Name", "Test");
        dict.set("Value", 42);

        let entries: Vec<_> = dict.entries().collect();
        assert_eq!(entries.len(), 2);

        // Test that iter() works the same way
        let iter_entries: Vec<_> = dict.iter().collect();
        assert_eq!(entries, iter_entries);
    }

    #[test]
    fn test_entries_mut() {
        let mut dict = Dictionary::new();
        dict.set("X", 10);
        dict.set("Y", 20);

        for (_, value) in dict.entries_mut() {
            if let Object::Integer(val) = value {
                *val *= 2;
            }
        }

        assert_eq!(dict.get("X"), Some(&Object::Integer(20)));
        assert_eq!(dict.get("Y"), Some(&Object::Integer(40)));
    }

    #[test]
    fn test_get_dict() {
        let mut parent = Dictionary::new();
        let mut child = Dictionary::new();
        child.set("ChildKey", "ChildValue");

        parent.set("Child", Object::Dictionary(child));
        parent.set("NotDict", "String");

        // Should return Some for dictionary objects
        let child_dict = parent.get_dict("Child");
        assert!(child_dict.is_some());
        assert_eq!(
            child_dict.unwrap().get("ChildKey"),
            Some(&Object::String("ChildValue".to_string()))
        );

        // Should return None for non-dictionary objects
        assert!(parent.get_dict("NotDict").is_none());

        // Should return None for missing keys
        assert!(parent.get_dict("Missing").is_none());
    }

    #[test]
    fn test_from_iterator() {
        let items = vec![
            ("Name".to_string(), Object::String("Test".to_string())),
            ("Count".to_string(), Object::Integer(5)),
            ("Enabled".to_string(), Object::Boolean(true)),
        ];

        let dict: Dictionary = items.into_iter().collect();

        assert_eq!(dict.len(), 3);
        assert_eq!(dict.get("Name"), Some(&Object::String("Test".to_string())));
        assert_eq!(dict.get("Count"), Some(&Object::Integer(5)));
        assert_eq!(dict.get("Enabled"), Some(&Object::Boolean(true)));
    }

    #[test]
    fn test_default() {
        let dict: Dictionary = Default::default();
        assert!(dict.is_empty());
        assert_eq!(dict.len(), 0);
    }

    #[test]
    fn test_nested_dictionaries() {
        let mut root = Dictionary::new();
        let mut level1 = Dictionary::new();
        let mut level2 = Dictionary::new();

        level2.set("DeepValue", "Found");
        level1.set("Level2", Object::Dictionary(level2));
        root.set("Level1", Object::Dictionary(level1));

        // Navigate through nested dictionaries
        let deep_value = root
            .get_dict("Level1")
            .and_then(|l1| l1.get_dict("Level2"))
            .and_then(|l2| l2.get("DeepValue"));

        assert_eq!(deep_value, Some(&Object::String("Found".to_string())));
    }
}
