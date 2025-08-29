use std::collections::HashSet;

use super::context::TemplateContext;
use super::error::{TemplateError, TemplateResult};
use super::parser::{Placeholder, TemplateParser};

/// Template renderer that performs variable substitution
pub struct TemplateRenderer {
    parser: TemplateParser,
}

impl TemplateRenderer {
    /// Create a new template renderer
    pub fn new() -> TemplateResult<Self> {
        Ok(Self {
            parser: TemplateParser::new()?,
        })
    }

    /// Render a template with the given context
    pub fn render(&self, template: &str, context: &TemplateContext) -> TemplateResult<String> {
        let placeholders = self.parser.parse(template)?;

        if placeholders.is_empty() {
            return Ok(template.to_string());
        }

        // Validate that all variables exist in context
        self.validate_variables(&placeholders, context)?;

        // Replace placeholders with values
        self.substitute_variables(template, &placeholders, context)
    }

    /// Validate that all required variables exist in the context
    fn validate_variables(
        &self,
        placeholders: &[Placeholder],
        context: &TemplateContext,
    ) -> TemplateResult<()> {
        let mut missing_variables = Vec::new();

        for placeholder in placeholders {
            if !context.has(&placeholder.variable_name) {
                missing_variables.push(placeholder.variable_name.clone());
            }
        }

        if !missing_variables.is_empty() {
            // Remove duplicates and sort for consistent error messages
            let mut unique_missing: Vec<String> = missing_variables
                .into_iter()
                .collect::<HashSet<_>>()
                .into_iter()
                .collect();
            unique_missing.sort();

            return Err(TemplateError::VariableNotFound(format!(
                "Variables not found: [{}]",
                unique_missing.join(", ")
            )));
        }

        Ok(())
    }

    /// Substitute variables with their values
    fn substitute_variables(
        &self,
        template: &str,
        placeholders: &[Placeholder],
        context: &TemplateContext,
    ) -> TemplateResult<String> {
        let mut result = template.to_string();

        // Sort placeholders by start position in reverse order
        // This ensures that we replace from right to left, preventing position shifts
        let mut sorted_placeholders = placeholders.to_vec();
        sorted_placeholders.sort_by(|a, b| b.start.cmp(&a.start));

        for placeholder in sorted_placeholders {
            let value = context.get_string(&placeholder.variable_name)?;

            // Replace the placeholder with the value
            result.replace_range(placeholder.start..placeholder.end, &value);
        }

        Ok(result)
    }

    /// Get all variable names required by a template
    pub fn get_required_variables(&self, template: &str) -> TemplateResult<Vec<String>> {
        self.parser.get_variable_names(template)
    }

    /// Check if a template is valid (all placeholders have correct syntax)
    pub fn validate_template(&self, template: &str) -> TemplateResult<()> {
        self.parser.parse(template)?;
        Ok(())
    }

    /// Check if template has any placeholders
    pub fn has_placeholders(&self, template: &str) -> bool {
        self.parser.has_placeholders(template)
    }

    /// Get detailed information about placeholders in a template
    pub fn analyze_template(&self, template: &str) -> TemplateResult<TemplateAnalysis> {
        let placeholders = self.parser.parse(template)?;
        let variable_names = self.parser.get_variable_names(template)?;

        Ok(TemplateAnalysis {
            total_placeholders: placeholders.len(),
            unique_variables: variable_names.len(),
            variable_names,
            placeholders,
        })
    }
}

/// Information about a template's structure
#[derive(Debug)]
pub struct TemplateAnalysis {
    /// Total number of placeholders (including duplicates)
    pub total_placeholders: usize,
    /// Number of unique variables
    pub unique_variables: usize,
    /// List of all unique variable names
    pub variable_names: Vec<String>,
    /// All placeholders found in the template
    pub placeholders: Vec<Placeholder>,
}

impl Default for TemplateRenderer {
    fn default() -> Self {
        Self::new().expect("Failed to create default TemplateRenderer")
    }
}

/// High-level template API
pub struct Template;

impl Template {
    /// Render a template string with the given context
    pub fn render(template: &str, context: &TemplateContext) -> TemplateResult<String> {
        let renderer = TemplateRenderer::new()?;
        renderer.render(template, context)
    }

    /// Render a template with a quick context from key-value pairs
    pub fn render_simple<S: AsRef<str>>(template: &str, vars: &[(S, S)]) -> TemplateResult<String> {
        let mut context = TemplateContext::new();
        for (key, value) in vars {
            context.set(key.as_ref(), value.as_ref());
        }
        Self::render(template, &context)
    }

    /// Validate that a template has correct syntax
    pub fn validate(template: &str) -> TemplateResult<()> {
        let renderer = TemplateRenderer::new()?;
        renderer.validate_template(template)
    }

    /// Get all variable names required by a template
    pub fn get_variables(template: &str) -> TemplateResult<Vec<String>> {
        let renderer = TemplateRenderer::new()?;
        renderer.get_required_variables(template)
    }

    /// Get detailed analysis of a template
    pub fn analyze(template: &str) -> TemplateResult<TemplateAnalysis> {
        let renderer = TemplateRenderer::new()?;
        renderer.analyze_template(template)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_rendering() {
        let mut context = TemplateContext::new();
        context.set("name", "Alice");
        context.set("age", "30");

        let template = "Hello {{name}}, you are {{age}} years old.";
        let result = Template::render(template, &context).unwrap();

        assert_eq!(result, "Hello Alice, you are 30 years old.");
    }

    #[test]
    fn test_template_without_placeholders() {
        let context = TemplateContext::new();
        let template = "This is just plain text.";
        let result = Template::render(template, &context).unwrap();

        assert_eq!(result, "This is just plain text.");
    }

    #[test]
    fn test_missing_variable_error() {
        let context = TemplateContext::new();
        let template = "Hello {{name}}!";
        let result = Template::render(template, &context);

        assert!(matches!(result, Err(TemplateError::VariableNotFound(_))));
    }

    #[test]
    fn test_nested_variables() {
        let mut context = TemplateContext::new();
        let user_obj = context.create_object("user");
        user_obj.insert("name".to_string(), "Bob".into());
        user_obj.insert("email".to_string(), "bob@example.com".into());

        let template = "User: {{user.name}} ({{user.email}})";
        let result = Template::render(template, &context).unwrap();

        assert_eq!(result, "User: Bob (bob@example.com)");
    }

    #[test]
    fn test_duplicate_placeholders() {
        let mut context = TemplateContext::new();
        context.set("word", "hello");

        let template = "{{word}} {{word}} {{word}}!";
        let result = Template::render(template, &context).unwrap();

        assert_eq!(result, "hello hello hello!");
    }

    #[test]
    fn test_render_simple() {
        let vars = vec![("name", "Charlie"), ("city", "New York")];

        let template = "{{name}} lives in {{city}}.";
        let result = Template::render_simple(template, &vars).unwrap();

        assert_eq!(result, "Charlie lives in New York.");
    }

    #[test]
    fn test_template_analysis() {
        let template = "{{name}} is {{age}} years old. {{name}} works in {{city}}.";
        let analysis = Template::analyze(template).unwrap();

        assert_eq!(analysis.total_placeholders, 4);
        assert_eq!(analysis.unique_variables, 3);
        assert_eq!(analysis.variable_names, vec!["age", "city", "name"]);
    }

    #[test]
    fn test_get_variables() {
        let template = "Invoice #{{number}} for {{customer}} - Total: {{amount}}";
        let variables = Template::get_variables(template).unwrap();

        assert_eq!(variables, vec!["amount", "customer", "number"]);
    }

    #[test]
    fn test_template_validation() {
        // Valid template
        assert!(Template::validate("Hello {{name}}!").is_ok());

        // Invalid template (single braces)
        assert!(Template::validate("Hello {name}!").is_err());

        // Invalid template (empty placeholder)
        assert!(Template::validate("Hello {{}}!").is_err());
    }

    #[test]
    fn test_number_and_boolean_values() {
        let mut context = TemplateContext::new();
        context.set_number("price", 19.99);
        context.set_integer("count", 5);
        context.set_boolean("available", true);

        let template = "Price: ${{price}}, Count: {{count}}, Available: {{available}}";
        let result = Template::render(template, &context).unwrap();

        assert_eq!(result, "Price: $19.99, Count: 5, Available: true");
    }

    #[test]
    fn test_complex_template() {
        let mut context = TemplateContext::new();
        context.set("invoice_number", "INV-001");
        context.set("date", "2024-01-15");

        let customer = context.create_object("customer");
        customer.insert("name".to_string(), "Acme Corp".into());
        customer.insert("email".to_string(), "billing@acme.com".into());

        let billing = context.create_object("billing");
        billing.insert("subtotal".to_string(), "$1,200.00".into());
        billing.insert("tax".to_string(), "$120.00".into());
        billing.insert("total".to_string(), "$1,320.00".into());

        let template = r#"
INVOICE {{invoice_number}}
Date: {{date}}

Bill To: {{customer.name}}
Email: {{customer.email}}

Subtotal: {{billing.subtotal}}
Tax: {{billing.tax}}
Total: {{billing.total}}
        "#
        .trim();

        let result = Template::render(template, &context).unwrap();
        let expected = r#"
INVOICE INV-001
Date: 2024-01-15

Bill To: Acme Corp
Email: billing@acme.com

Subtotal: $1,200.00
Tax: $120.00
Total: $1,320.00
        "#
        .trim();

        assert_eq!(result, expected);
    }
}
