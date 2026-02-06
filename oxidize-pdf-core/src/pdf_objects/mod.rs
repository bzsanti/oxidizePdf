//! Unified PDF Object Types
//!
//! This module provides the canonical PDF object types used throughout oxidize-pdf.
//! These types unify the previously separate parser and writer type systems.
//!
//! # Migration from v1.5.0
//!
//! - `parser::objects::PdfObject` → `pdf_objects::Object`
//! - `parser::objects::PdfDictionary` → `pdf_objects::Dictionary`
//! - `parser::objects::PdfName` → `pdf_objects::Name`
//! - `parser::objects::PdfArray` → `pdf_objects::Array`
//! - `parser::objects::PdfString` → `pdf_objects::BinaryString`
//! - `objects::primitive::Object` → `pdf_objects::Object`
//! - `objects::primitive::ObjectId` → `pdf_objects::ObjectId`
//! - `objects::dictionary::Dictionary` → `pdf_objects::Dictionary`
//!
//! Type aliases with deprecation warnings are provided for backward compatibility.

use std::collections::HashMap;
use std::fmt;

/// PDF Name - Unique atomic symbol in PDF
///
/// Names are used as keys in dictionaries and to identify PDF constructs.
/// Written with leading slash (/) in PDF syntax but stored without it.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Name(String);

impl Name {
    pub fn new(name: impl Into<String>) -> Self {
        Self(name.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn into_string(self) -> String {
        self.0
    }
}

impl From<&str> for Name {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

impl From<String> for Name {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl AsRef<str> for Name {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Name {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "/{}", self.0)
    }
}

/// Binary String - Arbitrary binary data in PDF
///
/// PDF strings can contain binary data or text in various encodings.
/// This type supports both, unlike Rust's UTF-8 String.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct BinaryString(Vec<u8>);

impl BinaryString {
    pub fn new(data: Vec<u8>) -> Self {
        Self(data)
    }

    pub fn from_str(s: &str) -> Self {
        Self(s.as_bytes().to_vec())
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }

    pub fn into_bytes(self) -> Vec<u8> {
        self.0
    }

    pub fn to_string_lossy(&self) -> String {
        String::from_utf8_lossy(&self.0).to_string()
    }

    pub fn try_to_string(&self) -> Option<String> {
        String::from_utf8(self.0.clone()).ok()
    }
}

impl From<Vec<u8>> for BinaryString {
    fn from(data: Vec<u8>) -> Self {
        Self(data)
    }
}

impl From<&[u8]> for BinaryString {
    fn from(data: &[u8]) -> Self {
        Self(data.to_vec())
    }
}

impl From<String> for BinaryString {
    fn from(s: String) -> Self {
        Self(s.into_bytes())
    }
}

impl From<&str> for BinaryString {
    fn from(s: &str) -> Self {
        Self(s.as_bytes().to_vec())
    }
}

/// Object ID - Reference to indirect PDF object
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ObjectId {
    number: u32,
    generation: u16,
}

impl ObjectId {
    pub fn new(number: u32, generation: u16) -> Self {
        Self { number, generation }
    }

    pub fn number(&self) -> u32 {
        self.number
    }

    pub fn generation(&self) -> u16 {
        self.generation
    }
}

impl From<(u32, u16)> for ObjectId {
    fn from((number, generation): (u32, u16)) -> Self {
        Self::new(number, generation)
    }
}

impl fmt::Display for ObjectId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {} R", self.number, self.generation)
    }
}

/// PDF Array - Ordered collection of objects
#[derive(Debug, Clone, PartialEq)]
pub struct Array(Vec<Object>);

impl Array {
    pub fn new() -> Self {
        Self(Vec::new())
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self(Vec::with_capacity(capacity))
    }

    pub fn push(&mut self, obj: impl Into<Object>) {
        self.0.push(obj.into());
    }

    pub fn get(&self, index: usize) -> Option<&Object> {
        self.0.get(index)
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = &Object> {
        self.0.iter()
    }

    pub fn as_slice(&self) -> &[Object] {
        &self.0
    }

    pub fn into_vec(self) -> Vec<Object> {
        self.0
    }
}

impl Default for Array {
    fn default() -> Self {
        Self::new()
    }
}

impl From<Vec<Object>> for Array {
    fn from(vec: Vec<Object>) -> Self {
        Self(vec)
    }
}

impl FromIterator<Object> for Array {
    fn from_iter<T: IntoIterator<Item = Object>>(iter: T) -> Self {
        Self(iter.into_iter().collect())
    }
}

/// PDF Dictionary - Key-value mapping with Name keys
#[derive(Debug, Clone, PartialEq)]
pub struct Dictionary {
    entries: HashMap<Name, Object>,
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

    pub fn set(&mut self, key: impl Into<Name>, value: impl Into<Object>) {
        self.entries.insert(key.into(), value.into());
    }

    pub fn get(&self, key: impl AsRef<str>) -> Option<&Object> {
        self.entries.get(&Name::new(key.as_ref()))
    }

    pub fn get_mut(&mut self, key: impl AsRef<str>) -> Option<&mut Object> {
        self.entries.get_mut(&Name::new(key.as_ref()))
    }

    pub fn remove(&mut self, key: impl AsRef<str>) -> Option<Object> {
        self.entries.remove(&Name::new(key.as_ref()))
    }

    pub fn contains_key(&self, key: impl AsRef<str>) -> bool {
        self.entries.contains_key(&Name::new(key.as_ref()))
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

    pub fn keys(&self) -> impl Iterator<Item = &Name> {
        self.entries.keys()
    }

    pub fn values(&self) -> impl Iterator<Item = &Object> {
        self.entries.values()
    }

    pub fn iter(&self) -> impl Iterator<Item = (&Name, &Object)> {
        self.entries.iter()
    }

    pub fn get_dict(&self, key: impl AsRef<str>) -> Option<&Dictionary> {
        self.get(key).and_then(|obj| {
            if let Object::Dictionary(dict) = obj {
                Some(dict)
            } else {
                None
            }
        })
    }

    pub fn get_type(&self) -> Option<&str> {
        self.get("Type").and_then(|obj| {
            if let Object::Name(name) = obj {
                Some(name.as_str())
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

/// PDF Stream - Dictionary with binary data
#[derive(Debug, Clone, PartialEq)]
pub struct Stream {
    pub dict: Dictionary,
    pub data: Vec<u8>,
}

impl Stream {
    pub fn new(dict: Dictionary, data: Vec<u8>) -> Self {
        Self { dict, data }
    }
}

/// PDF Object - The fundamental PDF data type
#[derive(Debug, Clone, PartialEq)]
pub enum Object {
    /// Null object - represents undefined/absent values
    Null,
    /// Boolean value
    Boolean(bool),
    /// Integer number
    Integer(i64),
    /// Real (floating-point) number
    Real(f64),
    /// String data (can be binary or text)
    String(BinaryString),
    /// Name object - unique identifier
    Name(Name),
    /// Array - ordered collection
    Array(Array),
    /// Dictionary - key-value pairs
    Dictionary(Dictionary),
    /// Stream - dictionary with binary data
    Stream(Stream),
    /// Indirect object reference
    Reference(ObjectId),
}

impl Object {
    pub fn is_null(&self) -> bool {
        matches!(self, Object::Null)
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Object::Boolean(b) => Some(*b),
            _ => None,
        }
    }

    pub fn as_integer(&self) -> Option<i64> {
        match self {
            Object::Integer(i) => Some(*i),
            _ => None,
        }
    }

    pub fn as_real(&self) -> Option<f64> {
        match self {
            Object::Real(f) => Some(*f),
            Object::Integer(i) => Some(*i as f64),
            _ => None,
        }
    }

    pub fn as_string(&self) -> Option<&BinaryString> {
        match self {
            Object::String(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_name(&self) -> Option<&Name> {
        match self {
            Object::Name(n) => Some(n),
            _ => None,
        }
    }

    pub fn as_array(&self) -> Option<&Array> {
        match self {
            Object::Array(arr) => Some(arr),
            _ => None,
        }
    }

    pub fn as_dict(&self) -> Option<&Dictionary> {
        match self {
            Object::Dictionary(dict) => Some(dict),
            _ => None,
        }
    }

    pub fn as_stream(&self) -> Option<&Stream> {
        match self {
            Object::Stream(stream) => Some(stream),
            _ => None,
        }
    }

    pub fn as_reference(&self) -> Option<ObjectId> {
        match self {
            Object::Reference(id) => Some(*id),
            _ => None,
        }
    }
}

// Conversions from primitive types
impl From<bool> for Object {
    fn from(b: bool) -> Self {
        Object::Boolean(b)
    }
}

impl From<i32> for Object {
    fn from(i: i32) -> Self {
        Object::Integer(i as i64)
    }
}

impl From<i64> for Object {
    fn from(i: i64) -> Self {
        Object::Integer(i)
    }
}

impl From<f32> for Object {
    fn from(f: f32) -> Self {
        Object::Real(f as f64)
    }
}

impl From<f64> for Object {
    fn from(f: f64) -> Self {
        Object::Real(f)
    }
}

impl From<String> for Object {
    fn from(s: String) -> Self {
        Object::String(BinaryString::from(s))
    }
}

impl From<&str> for Object {
    fn from(s: &str) -> Self {
        Object::String(BinaryString::from(s))
    }
}

impl From<BinaryString> for Object {
    fn from(s: BinaryString) -> Self {
        Object::String(s)
    }
}

impl From<Name> for Object {
    fn from(n: Name) -> Self {
        Object::Name(n)
    }
}

impl From<Array> for Object {
    fn from(a: Array) -> Self {
        Object::Array(a)
    }
}

impl From<Vec<Object>> for Object {
    fn from(v: Vec<Object>) -> Self {
        Object::Array(Array::from(v))
    }
}

impl From<Dictionary> for Object {
    fn from(d: Dictionary) -> Self {
        Object::Dictionary(d)
    }
}

impl From<Stream> for Object {
    fn from(s: Stream) -> Self {
        Object::Stream(s)
    }
}

impl From<ObjectId> for Object {
    fn from(id: ObjectId) -> Self {
        Object::Reference(id)
    }
}

impl From<(u32, u16)> for Object {
    fn from((number, generation): (u32, u16)) -> Self {
        Object::Reference(ObjectId::new(number, generation))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // =============================================================================
    // Name tests
    // =============================================================================

    #[test]
    fn test_name() {
        let name = Name::new("Type");
        assert_eq!(name.as_str(), "Type");
        assert_eq!(format!("{}", name), "/Type");
    }

    #[test]
    fn test_name_into_string() {
        let name = Name::new("Type");
        let s = name.into_string();
        assert_eq!(s, "Type");
    }

    #[test]
    fn test_name_from_string() {
        let name: Name = String::from("Test").into();
        assert_eq!(name.as_str(), "Test");
    }

    #[test]
    fn test_name_as_ref() {
        let name = Name::new("Type");
        let s: &str = name.as_ref();
        assert_eq!(s, "Type");
    }

    #[test]
    fn test_name_from_str() {
        let name: Name = "Type".into();
        assert_eq!(name.as_str(), "Type");
    }

    // =============================================================================
    // BinaryString tests
    // =============================================================================

    #[test]
    fn test_binary_string() {
        let s1 = BinaryString::from("Hello");
        assert_eq!(s1.to_string_lossy(), "Hello");

        let s2 = BinaryString::new(vec![0xFF, 0xFE, 0x48, 0x69]);
        assert!(s2.try_to_string().is_none());
        assert!(!s2.to_string_lossy().is_empty());
    }

    #[test]
    fn test_binary_string_from_str_method() {
        let s = BinaryString::from_str("Hello");
        assert_eq!(s.as_bytes(), b"Hello");
    }

    #[test]
    fn test_binary_string_into_bytes() {
        let s = BinaryString::new(vec![1, 2, 3]);
        let bytes = s.into_bytes();
        assert_eq!(bytes, vec![1, 2, 3]);
    }

    #[test]
    fn test_binary_string_from_vec() {
        let s: BinaryString = vec![65, 66, 67].into();
        assert_eq!(s.as_bytes(), &[65, 66, 67]);
    }

    #[test]
    fn test_binary_string_from_slice() {
        let data: &[u8] = &[1, 2, 3];
        let s: BinaryString = data.into();
        assert_eq!(s.as_bytes(), &[1, 2, 3]);
    }

    #[test]
    fn test_binary_string_from_string() {
        let s: BinaryString = String::from("Test").into();
        assert_eq!(s.as_bytes(), b"Test");
    }

    #[test]
    fn test_binary_string_try_to_string_valid_utf8() {
        let s = BinaryString::from_str("Valid UTF-8");
        assert_eq!(s.try_to_string(), Some("Valid UTF-8".to_string()));
    }

    // =============================================================================
    // ObjectId tests
    // =============================================================================

    #[test]
    fn test_object_id() {
        let id = ObjectId::new(10, 0);
        assert_eq!(id.number(), 10);
        assert_eq!(id.generation(), 0);
        assert_eq!(format!("{}", id), "10 0 R");
    }

    #[test]
    fn test_object_id_from_tuple() {
        let id: ObjectId = (42u32, 1u16).into();
        assert_eq!(id.number(), 42);
        assert_eq!(id.generation(), 1);
    }

    // =============================================================================
    // Array tests
    // =============================================================================

    #[test]
    fn test_array() {
        let mut arr = Array::new();
        arr.push(Object::Integer(1));
        arr.push(Object::Integer(2));
        arr.push(Object::Integer(3));

        assert_eq!(arr.len(), 3);
        assert_eq!(arr.get(1), Some(&Object::Integer(2)));
    }

    #[test]
    fn test_array_with_capacity() {
        let arr = Array::with_capacity(100);
        assert!(arr.is_empty());
        assert_eq!(arr.len(), 0);
    }

    #[test]
    fn test_array_is_empty() {
        let arr = Array::new();
        assert!(arr.is_empty());
    }

    #[test]
    fn test_array_iter() {
        let mut arr = Array::new();
        arr.push(Object::Integer(1));
        arr.push(Object::Integer(2));

        let sum: i64 = arr.iter().filter_map(|o| o.as_integer()).sum();
        assert_eq!(sum, 3);
    }

    #[test]
    fn test_array_as_slice() {
        let mut arr = Array::new();
        arr.push(Object::Integer(1));

        let slice = arr.as_slice();
        assert_eq!(slice.len(), 1);
    }

    #[test]
    fn test_array_into_vec() {
        let mut arr = Array::new();
        arr.push(Object::Integer(42));

        let vec = arr.into_vec();
        assert_eq!(vec.len(), 1);
    }

    #[test]
    fn test_array_default() {
        let arr: Array = Default::default();
        assert!(arr.is_empty());
    }

    #[test]
    fn test_array_from_vec() {
        let vec = vec![Object::Integer(1), Object::Integer(2)];
        let arr = Array::from(vec);
        assert_eq!(arr.len(), 2);
    }

    #[test]
    fn test_array_from_iterator() {
        let arr: Array = vec![Object::Integer(1), Object::Integer(2)]
            .into_iter()
            .collect();
        assert_eq!(arr.len(), 2);
    }

    #[test]
    fn test_array_get_out_of_bounds() {
        let arr = Array::new();
        assert!(arr.get(0).is_none());
    }

    // =============================================================================
    // Dictionary tests
    // =============================================================================

    #[test]
    fn test_dictionary() {
        let mut dict = Dictionary::new();
        dict.set("Type", Name::new("Page"));
        dict.set("Count", 5);

        assert_eq!(dict.get_type(), Some("Page"));
        assert_eq!(dict.get("Count"), Some(&Object::Integer(5)));
    }

    #[test]
    fn test_dictionary_with_capacity() {
        let dict = Dictionary::with_capacity(100);
        assert!(dict.is_empty());
    }

    #[test]
    fn test_dictionary_get_mut() {
        let mut dict = Dictionary::new();
        dict.set("Count", 5);

        if let Some(obj) = dict.get_mut("Count") {
            *obj = Object::Integer(10);
        }
        assert_eq!(dict.get("Count"), Some(&Object::Integer(10)));
    }

    #[test]
    fn test_dictionary_remove() {
        let mut dict = Dictionary::new();
        dict.set("Key", 1);

        let removed = dict.remove("Key");
        assert_eq!(removed, Some(Object::Integer(1)));
        assert!(dict.is_empty());
    }

    #[test]
    fn test_dictionary_remove_nonexistent() {
        let mut dict = Dictionary::new();
        assert!(dict.remove("NonExistent").is_none());
    }

    #[test]
    fn test_dictionary_contains_key() {
        let mut dict = Dictionary::new();
        dict.set("Key", 1);

        assert!(dict.contains_key("Key"));
        assert!(!dict.contains_key("Other"));
    }

    #[test]
    fn test_dictionary_clear() {
        let mut dict = Dictionary::new();
        dict.set("A", 1);
        dict.set("B", 2);

        dict.clear();
        assert!(dict.is_empty());
    }

    #[test]
    fn test_dictionary_keys() {
        let mut dict = Dictionary::new();
        dict.set("A", 1);
        dict.set("B", 2);

        let keys: Vec<&str> = dict.keys().map(|n| n.as_str()).collect();
        assert!(keys.contains(&"A"));
        assert!(keys.contains(&"B"));
    }

    #[test]
    fn test_dictionary_values() {
        let mut dict = Dictionary::new();
        dict.set("A", 1);
        dict.set("B", 2);

        let count = dict.values().count();
        assert_eq!(count, 2);
    }

    #[test]
    fn test_dictionary_iter() {
        let mut dict = Dictionary::new();
        dict.set("A", 1);

        for (name, _obj) in dict.iter() {
            assert_eq!(name.as_str(), "A");
        }
    }

    #[test]
    fn test_dictionary_get_dict() {
        let mut inner = Dictionary::new();
        inner.set("Inner", 1);

        let mut outer = Dictionary::new();
        outer.set("Nested", Object::Dictionary(inner));

        let nested = outer.get_dict("Nested");
        assert!(nested.is_some());
        assert_eq!(nested.unwrap().len(), 1);
    }

    #[test]
    fn test_dictionary_get_dict_not_dict() {
        let mut dict = Dictionary::new();
        dict.set("Key", 42);

        assert!(dict.get_dict("Key").is_none());
    }

    #[test]
    fn test_dictionary_get_type_not_name() {
        let mut dict = Dictionary::new();
        dict.set("Type", 42);

        assert!(dict.get_type().is_none());
    }

    #[test]
    fn test_dictionary_default() {
        let dict: Dictionary = Default::default();
        assert!(dict.is_empty());
    }

    // =============================================================================
    // Stream tests
    // =============================================================================

    #[test]
    fn test_stream_new() {
        let dict = Dictionary::new();
        let data = vec![1, 2, 3, 4, 5];
        let stream = Stream::new(dict, data.clone());

        assert!(stream.dict.is_empty());
        assert_eq!(stream.data, data);
    }

    // =============================================================================
    // Object tests
    // =============================================================================

    #[test]
    fn test_object_conversions() {
        let obj1: Object = true.into();
        assert_eq!(obj1, Object::Boolean(true));

        let obj2: Object = 42.into();
        assert_eq!(obj2, Object::Integer(42));

        let obj3: Object = "test".into();
        if let Object::String(s) = obj3 {
            assert_eq!(s.to_string_lossy(), "test");
        } else {
            panic!("Expected String");
        }
    }

    #[test]
    fn test_object_is_null() {
        assert!(Object::Null.is_null());
        assert!(!Object::Integer(0).is_null());
    }

    #[test]
    fn test_object_as_bool() {
        assert_eq!(Object::Boolean(true).as_bool(), Some(true));
        assert_eq!(Object::Boolean(false).as_bool(), Some(false));
        assert_eq!(Object::Integer(1).as_bool(), None);
    }

    #[test]
    fn test_object_as_integer() {
        assert_eq!(Object::Integer(42).as_integer(), Some(42));
        assert_eq!(Object::Real(3.14).as_integer(), None);
    }

    #[test]
    fn test_object_as_real() {
        assert_eq!(Object::Real(3.14).as_real(), Some(3.14));
        // Integer can be converted to real
        assert_eq!(Object::Integer(42).as_real(), Some(42.0));
        assert_eq!(Object::Boolean(true).as_real(), None);
    }

    #[test]
    fn test_object_as_string() {
        let s = BinaryString::from("test");
        let obj = Object::String(s.clone());
        assert!(obj.as_string().is_some());
        assert_eq!(Object::Integer(1).as_string(), None);
    }

    #[test]
    fn test_object_as_name() {
        let name = Name::new("Type");
        let obj = Object::Name(name);
        assert!(obj.as_name().is_some());
        assert_eq!(Object::Integer(1).as_name(), None);
    }

    #[test]
    fn test_object_as_array() {
        let arr = Array::new();
        let obj = Object::Array(arr);
        assert!(obj.as_array().is_some());
        assert_eq!(Object::Integer(1).as_array(), None);
    }

    #[test]
    fn test_object_as_dict() {
        let dict = Dictionary::new();
        let obj = Object::Dictionary(dict);
        assert!(obj.as_dict().is_some());
        assert_eq!(Object::Integer(1).as_dict(), None);
    }

    #[test]
    fn test_object_as_stream() {
        let stream = Stream::new(Dictionary::new(), vec![]);
        let obj = Object::Stream(stream);
        assert!(obj.as_stream().is_some());
        assert_eq!(Object::Integer(1).as_stream(), None);
    }

    #[test]
    fn test_object_as_reference() {
        let id = ObjectId::new(1, 0);
        let obj = Object::Reference(id);
        assert_eq!(obj.as_reference(), Some(id));
        assert_eq!(Object::Integer(1).as_reference(), None);
    }

    #[test]
    fn test_object_from_i64() {
        let obj: Object = 42i64.into();
        assert_eq!(obj.as_integer(), Some(42));
    }

    #[test]
    fn test_object_from_f32() {
        let obj: Object = 3.14f32.into();
        assert!(obj.as_real().is_some());
    }

    #[test]
    fn test_object_from_f64() {
        let obj: Object = 3.14f64.into();
        assert_eq!(obj.as_real(), Some(3.14));
    }

    #[test]
    fn test_object_from_string() {
        let obj: Object = String::from("test").into();
        assert!(obj.as_string().is_some());
    }

    #[test]
    fn test_object_from_binary_string() {
        let bs = BinaryString::from("test");
        let obj: Object = bs.into();
        assert!(obj.as_string().is_some());
    }

    #[test]
    fn test_object_from_name() {
        let name = Name::new("Type");
        let obj: Object = name.into();
        assert!(obj.as_name().is_some());
    }

    #[test]
    fn test_object_from_array() {
        let arr = Array::new();
        let obj: Object = arr.into();
        assert!(obj.as_array().is_some());
    }

    #[test]
    fn test_object_from_vec() {
        let vec = vec![Object::Integer(1)];
        let obj: Object = vec.into();
        assert!(obj.as_array().is_some());
    }

    #[test]
    fn test_object_from_dictionary() {
        let dict = Dictionary::new();
        let obj: Object = dict.into();
        assert!(obj.as_dict().is_some());
    }

    #[test]
    fn test_object_from_stream() {
        let stream = Stream::new(Dictionary::new(), vec![]);
        let obj: Object = stream.into();
        assert!(obj.as_stream().is_some());
    }

    #[test]
    fn test_object_from_object_id() {
        let id = ObjectId::new(1, 0);
        let obj: Object = id.into();
        assert!(obj.as_reference().is_some());
    }

    #[test]
    fn test_object_from_tuple() {
        let obj: Object = (1u32, 0u16).into();
        assert!(obj.as_reference().is_some());
    }
}
