//! Invoice Template Demo
//!
//! Demonstrates the template system with variable substitution for generating
//! dynamic PDF invoices from templates.

use oxidize_pdf::{
    templates::{Template, TemplateContext},
    Document, Font, Page, Result,
};
use std::path::Path;

fn main() -> Result<()> {
    println!("=== Template Invoice Demo ===");

    // Create a new document
    let mut doc = Document::new();
    doc.set_title("Invoice Template Demo");

    // Create a template context with invoice data
    let mut context = create_invoice_context();

    // Generate the invoice content using templates
    let invoice_content = generate_invoice_template(&mut context)?;

    // Create the PDF page with the rendered content
    let page = create_invoice_page(&invoice_content)?;
    doc.add_page(page);

    // Save the document
    let output_path = Path::new("examples/results/template_invoice_demo.pdf");
    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    doc.save(output_path)?;

    println!("✅ Invoice PDF created: {}", output_path.display());
    println!("📊 Template features demonstrated:");
    println!("   • Variable substitution with {{variable}}");
    println!("   • Nested object support ({{customer.name}})");
    println!("   • Number and currency formatting");
    println!("   • Multi-line templates");
    println!("   • Date and reference formatting");

    Ok(())
}

/// Create a comprehensive invoice context with sample data
fn create_invoice_context() -> TemplateContext {
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
    customer.insert("address".to_string(), "456 Commerce St".into());
    customer.insert("city".to_string(), "Business City, BC 67890".into());
    customer.insert("email".to_string(), "ap@globalcorp.com".into());

    // Billing information
    let billing = context.create_object("billing");
    billing.insert("subtotal".to_string(), "$2,500.00".into());
    billing.insert("tax_rate".to_string(), "8.5%".into());
    billing.insert("tax_amount".to_string(), "$212.50".into());
    billing.insert("total".to_string(), "$2,712.50".into());

    // Project details
    context.set("project_name", "Website Development");
    context.set("hours", "50");
    context.set("rate", "$50.00/hour");

    // Payment terms
    context.set("payment_terms", "Net 30");
    context.set("late_fee", "1.5% per month");

    context
}

/// Generate invoice content using templates
fn generate_invoice_template(context: &mut TemplateContext) -> Result<InvoiceContent> {
    // Header template
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

    // Customer section template
    let customer_template = r#"
BILL TO:
{{customer.name}}
Attn: {{customer.contact}}
{{customer.address}}
{{customer.city}}
Email: {{customer.email}}
    "#
    .trim();

    // Services template
    let services_template = r#"
SERVICES PROVIDED:
Project: {{project_name}}
Hours: {{hours}} @ {{rate}} = {{billing.subtotal}}
    "#
    .trim();

    // Billing template
    let billing_template = r#"
BILLING SUMMARY:
Subtotal: {{billing.subtotal}}
Tax ({{billing.tax_rate}}): {{billing.tax_amount}}
TOTAL AMOUNT DUE: {{billing.total}}

Payment Terms: {{payment_terms}}
Late Fee: {{late_fee}}
    "#
    .trim();

    // Render all sections
    let header = Template::render(header_template, context)
        .map_err(|e| oxidize_pdf::OxidizePdfError::Custom(format!("Template error: {}", e)))?;

    let customer_section = Template::render(customer_template, context)
        .map_err(|e| oxidize_pdf::OxidizePdfError::Custom(format!("Template error: {}", e)))?;

    let services_section = Template::render(services_template, context)
        .map_err(|e| oxidize_pdf::OxidizePdfError::Custom(format!("Template error: {}", e)))?;

    let billing_section = Template::render(billing_template, context)
        .map_err(|e| oxidize_pdf::OxidizePdfError::Custom(format!("Template error: {}", e)))?;

    Ok(InvoiceContent {
        header,
        customer_section,
        services_section,
        billing_section,
    })
}

/// Create a PDF page with the invoice content
fn create_invoice_page(content: &InvoiceContent) -> Result<Page> {
    let mut page = Page::a4();
    let mut y_position = 750.0;

    // Add header section
    page.text()
        .set_font(Font::HelveticaBold, 16.0)
        .at(50.0, y_position)
        .write(&content.header)?;

    y_position -= content.header.lines().count() as f64 * 18.0 + 30.0;

    // Add customer section
    page.text()
        .set_font(Font::Helvetica, 12.0)
        .at(50.0, y_position)
        .write(&content.customer_section)?;

    y_position -= content.customer_section.lines().count() as f64 * 14.0 + 30.0;

    // Add services section
    page.text()
        .set_font(Font::Helvetica, 12.0)
        .at(50.0, y_position)
        .write(&content.services_section)?;

    y_position -= content.services_section.lines().count() as f64 * 14.0 + 30.0;

    // Add billing section
    page.text()
        .set_font(Font::HelveticaBold, 12.0)
        .at(50.0, y_position)
        .write(&content.billing_section)?;

    // Add a footer with template info
    page.text()
        .set_font(Font::Helvetica, 8.0)
        .at(50.0, 50.0)
        .write("Generated using oxidize-pdf Template System")?;

    Ok(page)
}

/// Structured invoice content
struct InvoiceContent {
    header: String,
    customer_section: String,
    services_section: String,
    billing_section: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use oxidize_pdf::templates::{TemplateError, TemplateResult};

    #[test]
    fn test_template_context_creation() {
        let context = create_invoice_context();

        // Test basic variables
        assert!(context.has("invoice_number"));
        assert!(context.has("date"));

        // Test nested variables
        assert!(context.has("company.name"));
        assert!(context.has("customer.email"));
        assert!(context.has("billing.total"));
    }

    #[test]
    fn test_simple_template_rendering() -> TemplateResult<()> {
        let context = create_invoice_context();

        let template = "Invoice {{invoice_number}} for {{customer.name}}";
        let result = Template::render(template, &context)?;

        assert!(result.contains("INV-2024-001"));
        assert!(result.contains("Global Corp Ltd."));

        Ok(())
    }

    #[test]
    fn test_multi_line_template() -> TemplateResult<()> {
        let context = create_invoice_context();

        let template = r#"
Company: {{company.name}}
Customer: {{customer.name}}
Total: {{billing.total}}
        "#
        .trim();

        let result = Template::render(template, &context)?;
        let lines: Vec<&str> = result.lines().collect();

        assert_eq!(lines.len(), 3);
        assert!(lines[0].contains("Acme Solutions Inc."));
        assert!(lines[1].contains("Global Corp Ltd."));
        assert!(lines[2].contains("$2,712.50"));

        Ok(())
    }

    #[test]
    fn test_missing_variable_error() {
        let context = create_invoice_context();
        let template = "Missing: {{nonexistent_variable}}";
        let result = Template::render(template, &context);

        assert!(matches!(result, Err(TemplateError::VariableNotFound(_))));
    }

    #[test]
    fn test_template_analysis() -> TemplateResult<()> {
        let template = "{{invoice_number}} - {{customer.name}} - {{billing.total}}";
        let analysis = Template::analyze(template)?;

        assert_eq!(analysis.total_placeholders, 3);
        assert_eq!(analysis.unique_variables, 3);
        assert!(analysis
            .variable_names
            .contains(&"invoice_number".to_string()));
        assert!(analysis
            .variable_names
            .contains(&"customer.name".to_string()));
        assert!(analysis
            .variable_names
            .contains(&"billing.total".to_string()));

        Ok(())
    }
}
