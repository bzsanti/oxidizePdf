//! Simple template engine for PDF document generation
//!
//! This module provides basic templating functionality with variable substitution
//! and simple conditionals for dynamic PDF content generation.

use crate::error::PdfError;
use std::collections::HashMap;
use std::fmt;

/// Template value types that can be used in templates
#[derive(Debug, Clone, PartialEq)]
pub enum TemplateValue {
    /// String value
    String(String),
    /// Integer value
    Integer(i64),
    /// Floating point value
    Float(f64),
    /// Boolean value
    Boolean(bool),
    /// Array of values
    Array(Vec<TemplateValue>),
    /// Object with key-value pairs
    Object(HashMap<String, TemplateValue>),
    /// Null value
    Null,
}

impl fmt::Display for TemplateValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TemplateValue::String(s) => write!(f, "{s}"),
            TemplateValue::Integer(i) => write!(f, "{i}"),
            TemplateValue::Float(fl) => write!(f, "{fl:.2}"),
            TemplateValue::Boolean(b) => write!(f, "{b}"),
            TemplateValue::Array(arr) => {
                let items: Vec<String> = arr.iter().map(|v| v.to_string()).collect();
                write!(f, "[{}]", items.join(", "))
            }
            TemplateValue::Object(_) => write!(f, "[Object]"),
            TemplateValue::Null => write!(f, ""),
        }
    }
}

impl From<String> for TemplateValue {
    fn from(s: String) -> Self {
        TemplateValue::String(s)
    }
}

impl From<&str> for TemplateValue {
    fn from(s: &str) -> Self {
        TemplateValue::String(s.to_string())
    }
}

impl From<i64> for TemplateValue {
    fn from(i: i64) -> Self {
        TemplateValue::Integer(i)
    }
}

impl From<i32> for TemplateValue {
    fn from(i: i32) -> Self {
        TemplateValue::Integer(i as i64)
    }
}

impl From<f64> for TemplateValue {
    fn from(f: f64) -> Self {
        TemplateValue::Float(f)
    }
}

impl From<bool> for TemplateValue {
    fn from(b: bool) -> Self {
        TemplateValue::Boolean(b)
    }
}

impl From<Vec<TemplateValue>> for TemplateValue {
    fn from(arr: Vec<TemplateValue>) -> Self {
        TemplateValue::Array(arr)
    }
}

impl From<HashMap<String, TemplateValue>> for TemplateValue {
    fn from(obj: HashMap<String, TemplateValue>) -> Self {
        TemplateValue::Object(obj)
    }
}

/// Template context containing variables for substitution
#[derive(Debug, Clone)]
pub struct TemplateContext {
    variables: HashMap<String, TemplateValue>,
}

impl Default for TemplateContext {
    fn default() -> Self {
        Self::new()
    }
}

impl TemplateContext {
    /// Create a new empty template context
    pub fn new() -> Self {
        Self {
            variables: HashMap::new(),
        }
    }

    /// Set a variable in the context
    pub fn set<T: Into<TemplateValue>>(&mut self, key: impl Into<String>, value: T) -> &mut Self {
        self.variables.insert(key.into(), value.into());
        self
    }

    /// Get a variable from the context
    pub fn get(&self, key: &str) -> Option<&TemplateValue> {
        self.variables.get(key)
    }

    /// Check if a variable exists and is truthy
    pub fn is_truthy(&self, key: &str) -> bool {
        match self.get(key) {
            Some(TemplateValue::Boolean(b)) => *b,
            Some(TemplateValue::String(s)) => !s.is_empty(),
            Some(TemplateValue::Integer(i)) => *i != 0,
            Some(TemplateValue::Float(f)) => *f != 0.0,
            Some(TemplateValue::Array(arr)) => !arr.is_empty(),
            Some(TemplateValue::Object(obj)) => !obj.is_empty(),
            Some(TemplateValue::Null) => false,
            None => false,
        }
    }

    /// Get array values for iteration
    pub fn get_array(&self, key: &str) -> Option<&Vec<TemplateValue>> {
        match self.get(key) {
            Some(TemplateValue::Array(arr)) => Some(arr),
            _ => None,
        }
    }
}

/// Simple template engine for PDF documents
#[derive(Debug, Clone)]
pub struct TemplateEngine {
    /// Template source content
    content: String,
}

impl TemplateEngine {
    /// Create a new template from a string
    pub fn new(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
        }
    }

    /// Render the template with the given context
    pub fn render(&self, context: &TemplateContext) -> Result<String, PdfError> {
        let mut result = self.content.clone();
        let mut pos = 0;

        while pos < result.len() {
            if let Some(start) = result[pos..].find("{{") {
                let abs_start = pos + start;
                if let Some(end) = result[abs_start + 2..].find("}}") {
                    let abs_end = abs_start + 2 + end;
                    let expression = &result[abs_start + 2..abs_end];

                    // Process the expression
                    let replacement = self.process_expression(expression.trim(), context)?;
                    result.replace_range(abs_start..abs_end + 2, &replacement);
                    pos = abs_start + replacement.len();
                } else {
                    // No closing braces found, skip this opening
                    pos = abs_start + 2;
                }
            } else {
                // No more template expressions
                break;
            }
        }

        Ok(result)
    }

    /// Process a template expression
    fn process_expression(
        &self,
        expression: &str,
        context: &TemplateContext,
    ) -> Result<String, PdfError> {
        // Handle conditionals
        if expression.starts_with("#if ") {
            return self.process_conditional(expression, context);
        }

        // Handle loops
        if expression.starts_with("#each ") {
            return self.process_loop(expression, context);
        }

        // Handle simple variables
        if let Some(value) = context.get(expression) {
            Ok(value.to_string())
        } else {
            // Variable not found, return empty string or placeholder
            Ok(String::new())
        }
    }

    /// Process conditional expressions ({{#if variable}}...{{/if}})
    fn process_conditional(
        &self,
        _expression: &str,
        _context: &TemplateContext,
    ) -> Result<String, PdfError> {
        // For this simple implementation, we'll handle conditionals in a basic way
        // In a full implementation, this would parse the entire conditional block
        Ok(String::new())
    }

    /// Process loop expressions ({{#each array}}...{{/each}})
    fn process_loop(
        &self,
        _expression: &str,
        _context: &TemplateContext,
    ) -> Result<String, PdfError> {
        // For this simple implementation, we'll handle loops in a basic way
        // In a full implementation, this would parse the entire loop block
        Ok(String::new())
    }

    /// Render template with helper functions for common formatting
    pub fn render_with_helpers(&self, context: &TemplateContext) -> Result<String, PdfError> {
        let mut enhanced_context = context.clone();

        // Add current date helper
        enhanced_context.set(
            "current_date",
            chrono::Local::now().format("%Y-%m-%d").to_string(),
        );

        // Add current time helper
        enhanced_context.set(
            "current_time",
            chrono::Local::now().format("%H:%M:%S").to_string(),
        );

        // Add current datetime helper
        enhanced_context.set(
            "current_datetime",
            chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
        );

        self.render(&enhanced_context)
    }
}

/// Advanced template engine with support for blocks and conditionals
#[derive(Debug, Clone)]
pub struct AdvancedTemplateEngine {
    content: String,
}

impl AdvancedTemplateEngine {
    /// Create a new advanced template
    pub fn new(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
        }
    }

    /// Render template with full support for conditionals and loops
    pub fn render(&self, context: &TemplateContext) -> Result<String, PdfError> {
        let mut result = String::new();
        let tokens = self.tokenize(&self.content)?;
        self.render_tokens(&tokens, context, &mut result)?;
        Ok(result)
    }

    /// Tokenize the template content
    fn tokenize(&self, content: &str) -> Result<Vec<Token>, PdfError> {
        let mut tokens = Vec::new();
        let mut pos = 0;
        let chars: Vec<char> = content.chars().collect();

        while pos < chars.len() {
            if pos + 1 < chars.len() && chars[pos] == '{' && chars[pos + 1] == '{' {
                // Find the end of the expression
                let mut end_pos = pos + 2;
                while end_pos + 1 < chars.len() {
                    if chars[end_pos] == '}' && chars[end_pos + 1] == '}' {
                        break;
                    }
                    end_pos += 1;
                }

                if end_pos + 1 < chars.len() {
                    let expression: String = chars[pos + 2..end_pos].iter().collect();
                    let expr = expression.trim();

                    if let Some(stripped) = expr.strip_prefix("#if ") {
                        let var_name = stripped.trim();
                        tokens.push(Token::IfStart(var_name.to_string()));
                    } else if expr == "/if" {
                        tokens.push(Token::IfEnd);
                    } else if let Some(stripped) = expr.strip_prefix("#each ") {
                        let var_name = stripped.trim();
                        tokens.push(Token::EachStart(var_name.to_string()));
                    } else if expr == "/each" {
                        tokens.push(Token::EachEnd);
                    } else {
                        tokens.push(Token::Variable(expr.to_string()));
                    }

                    pos = end_pos + 2;
                } else {
                    // Malformed expression, treat as text
                    tokens.push(Token::Text(chars[pos].to_string()));
                    pos += 1;
                }
            } else {
                // Collect text until next expression
                let start = pos;
                while pos < chars.len() {
                    if pos + 1 < chars.len() && chars[pos] == '{' && chars[pos + 1] == '{' {
                        break;
                    }
                    pos += 1;
                }

                if pos > start {
                    let text: String = chars[start..pos].iter().collect();
                    tokens.push(Token::Text(text));
                }
            }
        }

        Ok(tokens)
    }

    /// Render a list of tokens
    fn render_tokens(
        &self,
        tokens: &[Token],
        context: &TemplateContext,
        output: &mut String,
    ) -> Result<(), PdfError> {
        let mut i = 0;

        while i < tokens.len() {
            match &tokens[i] {
                Token::Text(text) => {
                    output.push_str(text);
                    i += 1;
                }
                Token::Variable(var_name) => {
                    if let Some(value) = context.get(var_name) {
                        output.push_str(&value.to_string());
                    }
                    i += 1;
                }
                Token::IfStart(var_name) => {
                    let (if_block, next_i) = self.extract_block(tokens, i, "if")?;
                    if context.is_truthy(var_name) {
                        self.render_tokens(&if_block, context, output)?;
                    }
                    i = next_i;
                }
                Token::EachStart(var_name) => {
                    let (loop_block, next_i) = self.extract_block(tokens, i, "each")?;
                    if let Some(array) = context.get_array(var_name) {
                        for item in array {
                            let mut item_context = context.clone();
                            item_context.set("item", item.clone());
                            self.render_tokens(&loop_block, &item_context, output)?;
                        }
                    }
                    i = next_i;
                }
                Token::IfEnd | Token::EachEnd => {
                    // These should be handled by extract_block
                    return Err(PdfError::InvalidStructure(
                        "Unexpected end token".to_string(),
                    ));
                }
            }
        }

        Ok(())
    }

    /// Extract a block of tokens between start and end tokens
    fn extract_block(
        &self,
        tokens: &[Token],
        start: usize,
        block_type: &str,
    ) -> Result<(Vec<Token>, usize), PdfError> {
        let mut block = Vec::new();
        let mut depth = 0;
        let mut i = start + 1; // Skip the start token

        while i < tokens.len() {
            match &tokens[i] {
                Token::IfStart(_) if block_type == "if" => {
                    depth += 1;
                    block.push(tokens[i].clone());
                }
                Token::EachStart(_) if block_type == "each" => {
                    depth += 1;
                    block.push(tokens[i].clone());
                }
                Token::IfEnd if block_type == "if" => {
                    if depth == 0 {
                        return Ok((block, i + 1));
                    } else {
                        depth -= 1;
                        block.push(tokens[i].clone());
                    }
                }
                Token::EachEnd if block_type == "each" => {
                    if depth == 0 {
                        return Ok((block, i + 1));
                    } else {
                        depth -= 1;
                        block.push(tokens[i].clone());
                    }
                }
                _ => {
                    block.push(tokens[i].clone());
                }
            }
            i += 1;
        }

        Err(PdfError::InvalidStructure(format!(
            "Unclosed {block_type} block"
        )))
    }
}

/// Token types for advanced template parsing
#[derive(Debug, Clone)]
enum Token {
    Text(String),
    Variable(String),
    IfStart(String),
    IfEnd,
    EachStart(String),
    EachEnd,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_template_value_display() {
        assert_eq!(
            TemplateValue::String("hello".to_string()).to_string(),
            "hello"
        );
        assert_eq!(TemplateValue::Integer(42).to_string(), "42");
        assert_eq!(TemplateValue::Float(3.14).to_string(), "3.14");
        assert_eq!(TemplateValue::Boolean(true).to_string(), "true");
        assert_eq!(TemplateValue::Null.to_string(), "");
    }

    #[test]
    fn test_template_value_from() {
        assert_eq!(
            TemplateValue::from("test"),
            TemplateValue::String("test".to_string())
        );
        assert_eq!(TemplateValue::from(42i32), TemplateValue::Integer(42));
        assert_eq!(TemplateValue::from(42i64), TemplateValue::Integer(42));
        assert_eq!(TemplateValue::from(3.14), TemplateValue::Float(3.14));
        assert_eq!(TemplateValue::from(true), TemplateValue::Boolean(true));
    }

    #[test]
    fn test_template_context() {
        let mut context = TemplateContext::new();
        context
            .set("name", "Alice")
            .set("age", 30)
            .set("active", true);

        assert_eq!(
            context.get("name"),
            Some(&TemplateValue::String("Alice".to_string()))
        );
        assert_eq!(context.get("age"), Some(&TemplateValue::Integer(30)));
        assert_eq!(context.get("active"), Some(&TemplateValue::Boolean(true)));
        assert_eq!(context.get("missing"), None);
    }

    #[test]
    fn test_template_context_is_truthy() {
        let mut context = TemplateContext::new();
        context
            .set("true_bool", true)
            .set("false_bool", false)
            .set("non_empty_string", "hello")
            .set("empty_string", "")
            .set("positive_int", 1)
            .set("zero_int", 0)
            .set("non_empty_array", vec![TemplateValue::from("item")])
            .set("empty_array", Vec::<TemplateValue>::new());

        assert!(context.is_truthy("true_bool"));
        assert!(!context.is_truthy("false_bool"));
        assert!(context.is_truthy("non_empty_string"));
        assert!(!context.is_truthy("empty_string"));
        assert!(context.is_truthy("positive_int"));
        assert!(!context.is_truthy("zero_int"));
        assert!(context.is_truthy("non_empty_array"));
        assert!(!context.is_truthy("empty_array"));
        assert!(!context.is_truthy("missing"));
    }

    #[test]
    fn test_simple_template_engine() {
        let template = TemplateEngine::new("Hello {{name}}, you are {{age}} years old!");
        let mut context = TemplateContext::new();
        context.set("name", "Bob").set("age", 25);

        let result = template.render(&context).unwrap();
        assert_eq!(result, "Hello Bob, you are 25 years old!");
    }

    #[test]
    fn test_template_with_missing_variable() {
        let template = TemplateEngine::new("Hello {{name}}, {{missing}} variable!");
        let mut context = TemplateContext::new();
        context.set("name", "Charlie");

        let result = template.render(&context).unwrap();
        assert_eq!(result, "Hello Charlie,  variable!");
    }

    #[test]
    fn test_template_with_helpers() {
        let template = TemplateEngine::new("Generated on {{current_date}} at {{current_time}}");
        let context = TemplateContext::new();

        let result = template.render_with_helpers(&context).unwrap();
        assert!(result.contains("Generated on"));
        assert!(result.contains("at"));
    }

    #[test]
    fn test_advanced_template_tokenizer() {
        let template = AdvancedTemplateEngine::new("Hello {{name}}!");
        let tokens = template.tokenize("Hello {{name}}!").unwrap();

        assert_eq!(tokens.len(), 3);
        match &tokens[0] {
            Token::Text(text) => assert_eq!(text, "Hello "),
            _ => panic!("Expected text token"),
        }
        match &tokens[1] {
            Token::Variable(var) => assert_eq!(var, "name"),
            _ => panic!("Expected variable token"),
        }
        match &tokens[2] {
            Token::Text(text) => assert_eq!(text, "!"),
            _ => panic!("Expected text token"),
        }
    }

    #[test]
    fn test_advanced_template_conditional() {
        let template = AdvancedTemplateEngine::new("{{#if show}}Hello {{name}}!{{/if}}");
        let mut context = TemplateContext::new();
        context.set("show", true).set("name", "Dave");

        let result = template.render(&context).unwrap();
        assert_eq!(result, "Hello Dave!");

        context.set("show", false);
        let result2 = template.render(&context).unwrap();
        assert_eq!(result2, "");
    }

    #[test]
    fn test_advanced_template_loop() {
        let template = AdvancedTemplateEngine::new("{{#each items}}Item: {{item}}\n{{/each}}");
        let mut context = TemplateContext::new();
        context.set(
            "items",
            vec![
                TemplateValue::from("First"),
                TemplateValue::from("Second"),
                TemplateValue::from("Third"),
            ],
        );

        let result = template.render(&context).unwrap();
        assert_eq!(result, "Item: First\nItem: Second\nItem: Third\n");
    }

    #[test]
    fn test_template_array_access() {
        let mut context = TemplateContext::new();
        let items = vec![
            TemplateValue::from("apple"),
            TemplateValue::from("banana"),
            TemplateValue::from("cherry"),
        ];
        context.set("fruits", items);

        let array = context.get_array("fruits");
        assert!(array.is_some());
        assert_eq!(array.unwrap().len(), 3);

        let non_array = context.get_array("missing");
        assert!(non_array.is_none());
    }

    #[test]
    fn test_nested_templates() {
        let template = AdvancedTemplateEngine::new(
            "{{#if user}}Hello {{name}}! {{#if admin}}You are an admin.{{/if}}{{/if}}",
        );
        let mut context = TemplateContext::new();
        context
            .set("user", true)
            .set("name", "Eve")
            .set("admin", true);

        let result = template.render(&context).unwrap();
        assert_eq!(result, "Hello Eve! You are an admin.");
    }

    #[test]
    fn test_template_value_array_display() {
        let array = vec![
            TemplateValue::from("a"),
            TemplateValue::from("b"),
            TemplateValue::from("c"),
        ];
        let value = TemplateValue::Array(array);
        assert_eq!(value.to_string(), "[a, b, c]");
    }

    #[test]
    fn test_template_context_default() {
        let context = TemplateContext::default();
        assert!(context.variables.is_empty());
    }

    #[test]
    fn test_malformed_template() {
        let template = TemplateEngine::new("Hello {{name without closing");
        let context = TemplateContext::new();

        // Should handle malformed templates gracefully
        let result = template.render(&context).unwrap();
        assert_eq!(result, "Hello {{name without closing");
    }
}
