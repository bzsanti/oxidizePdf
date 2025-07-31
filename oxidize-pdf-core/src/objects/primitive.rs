use crate::objects::Dictionary;
use std::fmt;

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

impl fmt::Display for ObjectId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {} R", self.number, self.generation)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Object {
    Null,
    Boolean(bool),
    Integer(i64),
    Real(f64),
    String(String),
    Name(String),
    Array(Vec<Object>),
    Dictionary(Dictionary),
    Stream(Dictionary, Vec<u8>),
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

    pub fn as_string(&self) -> Option<&str> {
        match self {
            Object::String(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_name(&self) -> Option<&str> {
        match self {
            Object::Name(n) => Some(n),
            _ => None,
        }
    }

    pub fn as_array(&self) -> Option<&Vec<Object>> {
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
}

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
        Object::String(s)
    }
}

impl From<&str> for Object {
    fn from(s: &str) -> Self {
        Object::String(s.to_string())
    }
}

impl From<Vec<Object>> for Object {
    fn from(v: Vec<Object>) -> Self {
        Object::Array(v)
    }
}

impl From<Dictionary> for Object {
    fn from(d: Dictionary) -> Self {
        Object::Dictionary(d)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_object_id_new() {
        let id = ObjectId::new(42, 5);
        assert_eq!(id.number(), 42);
        assert_eq!(id.generation(), 5);
    }

    #[test]
    fn test_object_id_display() {
        let id = ObjectId::new(10, 0);
        assert_eq!(format!("{}", id), "10 0 R");

        let id2 = ObjectId::new(999, 65535);
        assert_eq!(format!("{}", id2), "999 65535 R");
    }

    #[test]
    fn test_object_id_equality() {
        let id1 = ObjectId::new(1, 0);
        let id2 = ObjectId::new(1, 0);
        let id3 = ObjectId::new(2, 0);
        let id4 = ObjectId::new(1, 1);

        assert_eq!(id1, id2);
        assert_ne!(id1, id3);
        assert_ne!(id1, id4);
    }

    #[test]
    fn test_object_null() {
        let obj = Object::Null;
        assert!(obj.is_null());
        assert!(obj.as_bool().is_none());
        assert!(obj.as_integer().is_none());
        assert!(obj.as_real().is_none());
        assert!(obj.as_string().is_none());
        assert!(obj.as_name().is_none());
        assert!(obj.as_array().is_none());
        assert!(obj.as_dict().is_none());
    }

    #[test]
    fn test_object_boolean() {
        let obj_true = Object::Boolean(true);
        let obj_false = Object::Boolean(false);

        assert!(!obj_true.is_null());
        assert_eq!(obj_true.as_bool(), Some(true));
        assert_eq!(obj_false.as_bool(), Some(false));
        assert!(obj_true.as_integer().is_none());
    }

    #[test]
    fn test_object_integer() {
        let obj = Object::Integer(42);

        assert_eq!(obj.as_integer(), Some(42));
        assert_eq!(obj.as_real(), Some(42.0));
        assert!(obj.as_bool().is_none());
        assert!(obj.as_string().is_none());
    }

    #[test]
    fn test_object_real() {
        let obj = Object::Real(3.14159);

        assert_eq!(obj.as_real(), Some(3.14159));
        assert!(obj.as_integer().is_none());
        assert!(obj.as_bool().is_none());
    }

    #[test]
    fn test_object_string() {
        let obj = Object::String("Hello PDF".to_string());

        assert_eq!(obj.as_string(), Some("Hello PDF"));
        assert!(obj.as_name().is_none());
        assert!(obj.as_integer().is_none());
    }

    #[test]
    fn test_object_name() {
        let obj = Object::Name("Type".to_string());

        assert_eq!(obj.as_name(), Some("Type"));
        assert!(obj.as_string().is_none());
        assert!(obj.as_integer().is_none());
    }

    #[test]
    fn test_object_array() {
        let arr = vec![Object::Integer(1), Object::Integer(2), Object::Integer(3)];
        let obj = Object::Array(arr.clone());

        assert_eq!(obj.as_array(), Some(&arr));
        assert!(obj.as_dict().is_none());
    }

    #[test]
    fn test_object_dictionary() {
        let mut dict = Dictionary::new();
        dict.set("Key", "Value");
        let obj = Object::Dictionary(dict.clone());

        assert_eq!(obj.as_dict(), Some(&dict));
        assert!(obj.as_array().is_none());
    }

    #[test]
    fn test_object_stream() {
        let mut dict = Dictionary::new();
        dict.set("Length", 5);
        let data = vec![1, 2, 3, 4, 5];
        let obj = Object::Stream(dict, data);

        // Stream doesn't have as_stream method, but we can pattern match
        if let Object::Stream(d, data) = obj {
            assert_eq!(d.get("Length"), Some(&Object::Integer(5)));
            assert_eq!(data.len(), 5);
        } else {
            panic!("Expected Stream object");
        }
    }

    #[test]
    fn test_object_reference() {
        let id = ObjectId::new(10, 0);
        let obj = Object::Reference(id);

        // Reference doesn't have as_reference method, but we can pattern match
        if let Object::Reference(ref_id) = obj {
            assert_eq!(ref_id, id);
        } else {
            panic!("Expected Reference object");
        }
    }

    #[test]
    fn test_from_bool() {
        let obj: Object = true.into();
        assert_eq!(obj, Object::Boolean(true));

        let obj2: Object = false.into();
        assert_eq!(obj2, Object::Boolean(false));
    }

    #[test]
    fn test_from_integers() {
        let obj: Object = 42i32.into();
        assert_eq!(obj, Object::Integer(42));

        let obj2: Object = 9999i64.into();
        assert_eq!(obj2, Object::Integer(9999));

        let obj3: Object = (-100i32).into();
        assert_eq!(obj3, Object::Integer(-100));
    }

    #[test]
    fn test_from_floats() {
        let obj: Object = std::f32::consts::PI.into();
        if let Object::Real(val) = obj {
            assert!((val - std::f64::consts::PI).abs() < 0.001);
        } else {
            panic!("Expected Real object");
        }

        let obj2: Object = 2.71828f64.into();
        assert_eq!(obj2, Object::Real(2.71828));
    }

    #[test]
    fn test_from_strings() {
        let obj: Object = "Hello".into();
        assert_eq!(obj, Object::String("Hello".to_string()));

        let obj2: Object = String::from("World").into();
        assert_eq!(obj2, Object::String("World".to_string()));
    }

    #[test]
    fn test_from_vec() {
        let vec = vec![Object::Integer(1), Object::Integer(2)];
        let obj: Object = vec.clone().into();
        assert_eq!(obj, Object::Array(vec));
    }

    #[test]
    fn test_from_dictionary() {
        let mut dict = Dictionary::new();
        dict.set("Test", 123);
        let obj: Object = dict.clone().into();

        if let Object::Dictionary(d) = obj {
            assert_eq!(d.get("Test"), Some(&Object::Integer(123)));
        } else {
            panic!("Expected Dictionary object");
        }
    }

    #[test]
    fn test_object_equality() {
        assert_eq!(Object::Null, Object::Null);
        assert_eq!(Object::Boolean(true), Object::Boolean(true));
        assert_ne!(Object::Boolean(true), Object::Boolean(false));
        assert_eq!(Object::Integer(42), Object::Integer(42));
        assert_ne!(Object::Integer(42), Object::Integer(43));
        assert_eq!(
            Object::String("A".to_string()),
            Object::String("A".to_string())
        );
        assert_ne!(
            Object::String("A".to_string()),
            Object::String("B".to_string())
        );
    }
}
