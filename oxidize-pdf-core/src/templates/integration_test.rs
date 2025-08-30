//! Integration tests for the template system

use super::*;
use std::collections::HashMap;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_invoice_template_integration() {
        // Create a template context similar to the invoice demo
        let mut context = TemplateContext::new();

        // Invoice header information
        context.set("invoice_number", "INV-2024-001");
        context.set("date", "2024-01-15");
        context.set("due_date", "2024-02-14");

        // Company information
        let company = context.create_object("company");
        company.insert("name".to_string(), "Acme Solutions Inc.".into());
        company.insert("address".to_string(), "123 Business Ave".into());
        company.insert("city".to_string(), "Tech City, TC 12345".into());
        company.insert("phone".to_string(), "(555) 123-4567".into());
        company.insert("email".to_string(), "billing@acmesolutions.com".into());

        // Customer information
        let customer = context.create_object("customer");
        customer.insert("name".to_string(), "Global Corp Ltd.".into());
        customer.insert("contact".to_string(), "Jane Smith".into());

        // Billing information
        let billing = context.create_object("billing");
        billing.insert("subtotal".to_string(), "$2,500.00".into());
        billing.insert("tax_rate".to_string(), "8.5%".into());
        billing.insert("tax_amount".to_string(), "$212.50".into());
        billing.insert("total".to_string(), "$2,712.50".into());

        // Test header template
        let header_template = r#"
INVOICE {{invoice_number}}
Date: {{date}}
Due Date: {{due_date}}

{{company.name}}
{{company.address}}
{{company.city}}
Phone: {{company.phone}}
Email: {{company.email}}
        "#
        .trim();

        let header_result = Template::render(header_template, &context).unwrap();
        assert!(header_result.contains("INV-2024-001"));
        assert!(header_result.contains("Acme Solutions Inc."));
        assert!(header_result.contains("(555) 123-4567"));

        // Test customer template
        let customer_template = r#"
BILL TO:
{{customer.name}}
Attn: {{customer.contact}}
        "#
        .trim();

        let customer_result = Template::render(customer_template, &context).unwrap();
        assert!(customer_result.contains("Global Corp Ltd."));
        assert!(customer_result.contains("Jane Smith"));

        // Test billing template
        let billing_template = r#"
BILLING SUMMARY:
Subtotal: {{billing.subtotal}}
Tax ({{billing.tax_rate}}): {{billing.tax_amount}}
TOTAL AMOUNT DUE: {{billing.total}}
        "#
        .trim();

        let billing_result = Template::render(billing_template, &context).unwrap();
        assert!(billing_result.contains("$2,500.00"));
        assert!(billing_result.contains("8.5%"));
        assert!(billing_result.contains("$212.50"));
        assert!(billing_result.contains("$2,712.50"));

        // Test that all required variables are present
        let all_variables = Template::get_variables(billing_template).unwrap();
        for var in &all_variables {
            assert!(context.has(var), "Variable '{}' not found in context", var);
        }
    }

    #[test]
    fn test_template_error_handling() {
        let context = TemplateContext::new();

        // Test with missing variable
        let template = "Hello {{missing_var}}!";
        let result = Template::render(template, &context);
        assert!(matches!(result, Err(TemplateError::VariableNotFound(_))));

        // Test with invalid syntax
        let invalid_template = "Hello {single_brace}!";
        let result = Template::validate(invalid_template);
        assert!(matches!(result, Err(TemplateError::InvalidPlaceholder(_))));

        // Test with empty placeholder
        let empty_template = "Hello {{}}!";
        let result = Template::validate(empty_template);
        assert!(matches!(result, Err(TemplateError::InvalidPlaceholder(_))));
    }

    #[test]
    fn test_template_render_simple() {
        let vars = vec![
            ("company", "Acme Corp"),
            ("amount", "$1,000.00"),
            ("date", "2024-01-15"),
        ];

        let template = "Invoice from {{company}} for {{amount}} on {{date}}";
        let result = Template::render_simple(template, &vars).unwrap();

        assert_eq!(result, "Invoice from Acme Corp for $1,000.00 on 2024-01-15");
    }

    #[test]
    fn test_template_analysis() {
        let template = "{{name}} owes {{amount}} due on {{date}}. Contact {{name}} at {{email}}.";
        let analysis = Template::analyze(template).unwrap();

        assert_eq!(analysis.total_placeholders, 5); // name appears twice
        assert_eq!(analysis.unique_variables, 4); // name, amount, date, email
        assert!(analysis.variable_names.contains(&"name".to_string()));
        assert!(analysis.variable_names.contains(&"amount".to_string()));
        assert!(analysis.variable_names.contains(&"date".to_string()));
        assert!(analysis.variable_names.contains(&"email".to_string()));
    }

    #[test]
    fn test_nested_object_deep() {
        let mut context = TemplateContext::new();
        let user = context.create_object("user");
        user.insert("name".to_string(), "John Doe".into());

        let profile = HashMap::new();
        user.insert("profile".to_string(), TemplateValue::Object(profile));

        // Test that we can access nested properties
        assert!(context.has("user.name"));
        assert_eq!(context.get_string("user.name").unwrap(), "John Doe");
    }
}
