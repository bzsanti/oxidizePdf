use regex::Regex;
use std::collections::HashSet;

use super::error::{TemplateError, TemplateResult};

/// A placeholder found in template text
#[derive(Debug, Clone, PartialEq)]
pub struct Placeholder {
    /// The full placeholder text including delimiters (e.g., "{{name}}")
    pub full_text: String,
    /// The variable name (e.g., "name")
    pub variable_name: String,
    /// Start position in the original text
    pub start: usize,
    /// End position in the original text
    pub end: usize,
}

impl Placeholder {
    /// Create a new placeholder
    pub fn new(full_text: String, variable_name: String, start: usize, end: usize) -> Self {
        Self {
            full_text,
            variable_name,
            start,
            end,
        }
    }
}

/// Template parser for finding and validating placeholders
pub struct TemplateParser {
    /// Regex for matching placeholders
    placeholder_regex: Regex,
}

impl TemplateParser {
    /// Create a new template parser
    pub fn new() -> TemplateResult<Self> {
        // Regex to match {{variable_name}} patterns
        // Supports alphanumeric, underscore, and dot notation
        let placeholder_regex = Regex::new(r"\{\{\s*([a-zA-Z_][a-zA-Z0-9_.]*)\s*\}\}")?;

        Ok(Self { placeholder_regex })
    }

    /// Parse a template string and extract all placeholders
    pub fn parse(&self, template: &str) -> TemplateResult<Vec<Placeholder>> {
        let mut placeholders = Vec::new();

        for captures in self.placeholder_regex.captures_iter(template) {
            // Group 0 (full match) and group 1 (captured group) are guaranteed by successful regex match
            let full_match = &captures[0];
            let variable_name_str = &captures[1];

            let full_text = full_match.to_string();
            let variable_name = variable_name_str.to_string();
            let start = captures.get(0).map(|m| m.start()).unwrap_or(0);
            let end = captures.get(0).map(|m| m.end()).unwrap_or(0);

            // Validate variable name
            self.validate_variable_name(&variable_name)?;

            placeholders.push(Placeholder::new(full_text, variable_name, start, end));
        }

        // Check for invalid placeholder patterns first (empty, malformed, etc.)
        self.check_for_invalid_patterns(template)?;

        // Then check for invalid variable names in remaining double brace patterns
        self.check_for_invalid_variable_names_in_braces(template)?;

        Ok(placeholders)
    }

    /// Get all unique variable names from a template
    pub fn get_variable_names(&self, template: &str) -> TemplateResult<Vec<String>> {
        let placeholders = self.parse(template)?;
        let mut names: HashSet<String> = HashSet::new();

        for placeholder in placeholders {
            names.insert(placeholder.variable_name);
        }

        let mut result: Vec<String> = names.into_iter().collect();
        result.sort();
        Ok(result)
    }

    /// Validate a variable name
    fn validate_variable_name(&self, name: &str) -> TemplateResult<()> {
        if name.is_empty() {
            return Err(TemplateError::InvalidVariableName(name.to_string()));
        }

        // Check if it starts with a letter or underscore
        if let Some(first_char) = name.chars().next() {
            if !first_char.is_alphabetic() && first_char != '_' {
                return Err(TemplateError::InvalidVariableName(name.to_string()));
            }
        } else {
            // Empty name (should have been caught earlier, but be defensive)
            return Err(TemplateError::InvalidVariableName(name.to_string()));
        }

        // Check if all characters are valid
        for ch in name.chars() {
            if !ch.is_alphanumeric() && ch != '_' && ch != '.' {
                return Err(TemplateError::InvalidVariableName(name.to_string()));
            }
        }

        Ok(())
    }

    /// Check for invalid placeholder patterns that might confuse users
    fn check_for_invalid_patterns(&self, template: &str) -> TemplateResult<()> {
        // Check for empty placeholders FIRST
        let empty_placeholder_regex = Regex::new(r"\{\{\s*\}\}")?;
        if let Some(empty_match) = empty_placeholder_regex.find(template) {
            return Err(TemplateError::InvalidPlaceholder(format!(
                "Empty placeholder found at position {}: '{}'",
                empty_match.start(),
                empty_match.as_str()
            )));
        }

        // Check for malformed placeholders with too many braces
        let malformed_regex = Regex::new(r"\{\{\{+|\}\}\}+")?;
        if let Some(malformed_match) = malformed_regex.find(template) {
            return Err(TemplateError::InvalidPlaceholder(format!(
                "Malformed placeholder at position {}: '{}' - use exactly two braces",
                malformed_match.start(),
                malformed_match.as_str()
            )));
        }

        // Check for single braces that might indicate user error
        // Remove all double brace patterns (valid and invalid), then look for remaining single braces
        let all_double_braces_regex = Regex::new(r"\{\{\s*[^}]*\s*\}\}")?;
        let cleaned = all_double_braces_regex.replace_all(template, "");

        // Now check for any remaining single braces
        let single_brace_regex = Regex::new(r"[{}]")?;
        if let Some(invalid_match) = single_brace_regex.find(&cleaned) {
            // Find the position in the original string
            let position = self.find_original_position(&cleaned, invalid_match.start(), template);
            return Err(TemplateError::InvalidPlaceholder(format!(
                "Found single brace near position {}: '{}' - did you mean to use double braces {{{{}}}}?",
                position,
                invalid_match.as_str()
            )));
        }

        Ok(())
    }

    /// Check if a template has any placeholders
    pub fn has_placeholders(&self, template: &str) -> bool {
        self.placeholder_regex.is_match(template)
    }

    /// Count the number of placeholders in a template
    pub fn count_placeholders(&self, template: &str) -> usize {
        self.placeholder_regex.find_iter(template).count()
    }

    /// Helper method to find position in original string after replacements
    fn find_original_position(&self, _cleaned: &str, cleaned_pos: usize, original: &str) -> usize {
        // This is a simplified approximation - for better accuracy we'd need more complex tracking
        // But for error reporting purposes, this should be sufficient
        cleaned_pos.min(original.len().saturating_sub(1))
    }

    /// Check for double brace patterns with invalid variable names
    fn check_for_invalid_variable_names_in_braces(&self, template: &str) -> TemplateResult<()> {
        // Find all potential double brace patterns (regardless of variable name validity)
        let all_double_braces_regex = Regex::new(r"\{\{\s*([^}]*)\s*\}\}")?;

        for captures in all_double_braces_regex.captures_iter(template) {
            // Group 1 is guaranteed by successful regex match (pattern has one capture group)
            let variable_name = captures[1].trim();

            // If this variable name is invalid, report it
            if self.validate_variable_name(variable_name).is_err() {
                return Err(TemplateError::InvalidVariableName(
                    variable_name.to_string(),
                ));
            }
        }

        Ok(())
    }
}

impl Default for TemplateParser {
    fn default() -> Self {
        Self::new().expect("Failed to create default TemplateParser")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_placeholder_parsing() {
        let parser = TemplateParser::new().unwrap();        let template = "Hello {{name}}, your total is {{total}}.";

        let placeholders = parser.parse(template).unwrap();        assert_eq!(placeholders.len(), 2);

        assert_eq!(placeholders[0].variable_name, "name");
        assert_eq!(placeholders[0].full_text, "{{name}}");
        assert_eq!(placeholders[0].start, 6);

        assert_eq!(placeholders[1].variable_name, "total");
        assert_eq!(placeholders[1].full_text, "{{total}}");
    }

    #[test]
    fn test_dot_notation_variables() {
        let parser = TemplateParser::new().unwrap();        let template = "User: {{user.name}} ({{user.age}} years old)";

        let placeholders = parser.parse(template).unwrap();        assert_eq!(placeholders.len(), 2);

        assert_eq!(placeholders[0].variable_name, "user.name");
        assert_eq!(placeholders[1].variable_name, "user.age");
    }

    #[test]
    fn test_whitespace_handling() {
        let parser = TemplateParser::new().unwrap();        let template = "{{ name }} and {{  total  }}";

        let placeholders = parser.parse(template).unwrap();        assert_eq!(placeholders.len(), 2);

        assert_eq!(placeholders[0].variable_name, "name");
        assert_eq!(placeholders[1].variable_name, "total");
    }

    #[test]
    fn test_get_variable_names() {
        let parser = TemplateParser::new().unwrap();        let template = "{{name}} {{total}} {{name}} {{user.age}}";

        let names = parser.get_variable_names(template).unwrap();        assert_eq!(names, vec!["name", "total", "user.age"]);
    }

    #[test]
    fn test_invalid_variable_names() {
        let parser = TemplateParser::new().unwrap();
        // Test invalid starting character
        let template = "{{123invalid}}";
        let result = parser.parse(template);
        assert!(matches!(result, Err(TemplateError::InvalidVariableName(_))));
    }

    #[test]
    fn test_invalid_placeholder_patterns() {
        let parser = TemplateParser::new().unwrap();
        // Test single braces
        let template = "Hello {name}";
        let result = parser.parse(template);
        assert!(matches!(result, Err(TemplateError::InvalidPlaceholder(_))));

        // Test empty placeholder
        let template = "Hello {{}}";
        let result = parser.parse(template);
        assert!(matches!(result, Err(TemplateError::InvalidPlaceholder(_))));

        // Test too many braces
        let template = "Hello {{{name}}}";
        let result = parser.parse(template);
        assert!(matches!(result, Err(TemplateError::InvalidPlaceholder(_))));
    }

    #[test]
    fn test_has_placeholders() {
        let parser = TemplateParser::new().unwrap();
        assert!(parser.has_placeholders("Hello {{name}}"));
        assert!(!parser.has_placeholders("Hello world"));
    }

    #[test]
    fn test_count_placeholders() {
        let parser = TemplateParser::new().unwrap();
        assert_eq!(parser.count_placeholders("{{a}} {{b}} {{c}}"), 3);
        assert_eq!(parser.count_placeholders("No placeholders here"), 0);
        assert_eq!(parser.count_placeholders("{{duplicate}} {{duplicate}}"), 2);
    }

    #[test]
    fn test_placeholder_positions() {
        let parser = TemplateParser::new().unwrap();        let template = "Start {{var1}} middle {{var2}} end";

        let placeholders = parser.parse(template).unwrap();        assert_eq!(placeholders[0].start, 6);
        assert_eq!(placeholders[0].end, 14); // 6 + len("{{var1}}") = 6 + 8 = 14
        assert_eq!(placeholders[1].start, 22);
        assert_eq!(placeholders[1].end, 30); // 22 + len("{{var2}}") = 22 + 8 = 30

        // Verify the extracted text matches
        assert_eq!(
            &template[placeholders[0].start..placeholders[0].end],
            "{{var1}}"
        );
        assert_eq!(
            &template[placeholders[1].start..placeholders[1].end],
            "{{var2}}"
        );
    }
}
