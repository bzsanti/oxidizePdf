use std::collections::HashMap;
use std::fmt;

use super::error::{TemplateError, TemplateResult};

/// A value that can be used in template substitution
#[derive(Debug, Clone)]
pub enum TemplateValue {
    String(String),
    Number(f64),
    Integer(i64),
    Boolean(bool),
    /// Nested context for dot notation support (e.g., {{user.name}})
    Object(HashMap<String, TemplateValue>),
}

impl TemplateValue {
    /// Convert the value to a string for template rendering
    pub fn as_string(&self) -> String {
        match self {
            Self::String(s) => s.clone(),
            Self::Number(n) => {
                if n.fract() == 0.0 {
                    format!("{:.0}", n)
                } else {
                    format!("{}", n)
                }
            }
            Self::Integer(i) => format!("{}", i),
            Self::Boolean(b) => format!("{}", b),
            Self::Object(_) => "[Object]".to_string(),
        }
    }

    /// Get a nested value using dot notation
    pub fn get_nested(&self, path: &str) -> Option<&TemplateValue> {
        let parts: Vec<&str> = path.split('.').collect();
        let mut current = self;

        for part in parts {
            match current {
                Self::Object(map) => {
                    current = map.get(part)?;
                }
                _ => return None,
            }
        }

        Some(current)
    }
}

impl fmt::Display for TemplateValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_string())
    }
}

/// Template context that holds variables for substitution
#[derive(Debug, Clone)]
pub struct TemplateContext {
    variables: HashMap<String, TemplateValue>,
}

impl TemplateContext {
    /// Create a new empty template context
    pub fn new() -> Self {
        Self {
            variables: HashMap::new(),
        }
    }

    /// Set a string variable
    pub fn set<K: Into<String>, V: Into<String>>(&mut self, key: K, value: V) -> &mut Self {
        self.variables
            .insert(key.into(), TemplateValue::String(value.into()));
        self
    }

    /// Set a number variable
    pub fn set_number<K: Into<String>>(&mut self, key: K, value: f64) -> &mut Self {
        self.variables
            .insert(key.into(), TemplateValue::Number(value));
        self
    }

    /// Set an integer variable
    pub fn set_integer<K: Into<String>>(&mut self, key: K, value: i64) -> &mut Self {
        self.variables
            .insert(key.into(), TemplateValue::Integer(value));
        self
    }

    /// Set a boolean variable
    pub fn set_boolean<K: Into<String>>(&mut self, key: K, value: bool) -> &mut Self {
        self.variables
            .insert(key.into(), TemplateValue::Boolean(value));
        self
    }

    /// Set any template value
    pub fn set_value<K: Into<String>>(&mut self, key: K, value: TemplateValue) -> &mut Self {
        self.variables.insert(key.into(), value);
        self
    }

    /// Get a variable by name, supporting dot notation for nested objects
    pub fn get(&self, name: &str) -> TemplateResult<&TemplateValue> {
        if name.contains('.') {
            // Handle dot notation
            let parts: Vec<&str> = name.splitn(2, '.').collect();
            let root_key = parts[0];
            let nested_path = parts[1];

            let root_value = self
                .variables
                .get(root_key)
                .ok_or_else(|| TemplateError::VariableNotFound(root_key.to_string()))?;

            root_value
                .get_nested(nested_path)
                .ok_or_else(|| TemplateError::VariableNotFound(name.to_string()))
        } else {
            self.variables
                .get(name)
                .ok_or_else(|| TemplateError::VariableNotFound(name.to_string()))
        }
    }

    /// Get a variable as a string
    pub fn get_string(&self, name: &str) -> TemplateResult<String> {
        Ok(self.get(name)?.as_string())
    }

    /// Check if a variable exists
    pub fn has(&self, name: &str) -> bool {
        self.get(name).is_ok()
    }

    /// Remove a variable
    pub fn remove(&mut self, name: &str) -> Option<TemplateValue> {
        self.variables.remove(name)
    }

    /// Get all variable names
    pub fn keys(&self) -> Vec<String> {
        self.variables.keys().cloned().collect()
    }

    /// Clear all variables
    pub fn clear(&mut self) {
        self.variables.clear();
    }

    /// Merge another context into this one
    pub fn merge(&mut self, other: &TemplateContext) {
        for (key, value) in &other.variables {
            self.variables.insert(key.clone(), value.clone());
        }
    }

    /// Create a nested object for dot notation support
    pub fn create_object<K: Into<String>>(
        &mut self,
        key: K,
    ) -> &mut HashMap<String, TemplateValue> {
        let key = key.into();
        self.variables
            .insert(key.clone(), TemplateValue::Object(HashMap::new()));

        match self.variables.get_mut(&key).unwrap() {
            TemplateValue::Object(map) => map,
            _ => unreachable!(),
        }
    }
}

impl Default for TemplateContext {
    fn default() -> Self {
        Self::new()
    }
}

// Convenient conversions
impl From<&str> for TemplateValue {
    fn from(s: &str) -> Self {
        TemplateValue::String(s.to_string())
    }
}

impl From<String> for TemplateValue {
    fn from(s: String) -> Self {
        TemplateValue::String(s)
    }
}

impl From<f64> for TemplateValue {
    fn from(n: f64) -> Self {
        TemplateValue::Number(n)
    }
}

impl From<i64> for TemplateValue {
    fn from(i: i64) -> Self {
        TemplateValue::Integer(i)
    }
}

impl From<bool> for TemplateValue {
    fn from(b: bool) -> Self {
        TemplateValue::Boolean(b)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_template_context_basic() {
        let mut ctx = TemplateContext::new();
        ctx.set("name", "John Doe");
        ctx.set_number("age", 30.5);
        ctx.set_integer("count", 42);
        ctx.set_boolean("active", true);

        assert_eq!(ctx.get_string("name").unwrap(), "John Doe");
        assert_eq!(ctx.get_string("age").unwrap(), "30.5");
        assert_eq!(ctx.get_string("count").unwrap(), "42");
        assert_eq!(ctx.get_string("active").unwrap(), "true");
    }

    #[test]
    fn test_nested_objects() {
        let mut ctx = TemplateContext::new();
        let user_obj = ctx.create_object("user");
        user_obj.insert("name".to_string(), "Alice".into());
        user_obj.insert("age".to_string(), TemplateValue::Integer(25));

        assert_eq!(ctx.get_string("user.name").unwrap(), "Alice");
        assert_eq!(ctx.get_string("user.age").unwrap(), "25");
    }

    #[test]
    fn test_variable_not_found() {
        let ctx = TemplateContext::new();
        let result = ctx.get("nonexistent");
        assert!(matches!(result, Err(TemplateError::VariableNotFound(_))));
    }

    #[test]
    fn test_context_merge() {
        let mut ctx1 = TemplateContext::new();
        ctx1.set("name", "Alice");

        let mut ctx2 = TemplateContext::new();
        ctx2.set("age", "30");

        ctx1.merge(&ctx2);

        assert_eq!(ctx1.get_string("name").unwrap(), "Alice");
        assert_eq!(ctx1.get_string("age").unwrap(), "30");
    }

    #[test]
    fn test_template_value_conversions() {
        let val: TemplateValue = "hello".into();
        assert_eq!(val.to_string(), "hello");

        let val: TemplateValue = 42.0.into();
        assert_eq!(val.to_string(), "42");

        let val: TemplateValue = 42.5.into();
        assert_eq!(val.to_string(), "42.5");

        let val: TemplateValue = true.into();
        assert_eq!(val.to_string(), "true");
    }
}
