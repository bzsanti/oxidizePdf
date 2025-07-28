//! Example demonstrating template rendering in PDF documents

use oxidize_pdf::{
    text::{AdvancedTemplateEngine, TemplateContext, TemplateEngine, TemplateValue},
    Document, Font, Page, Result,
};

fn main() -> Result<()> {
    // Create a new document
    let mut doc = Document::new();

    // Create a new page
    let mut page = Page::a4();

    // Example 1: Simple variable substitution
    {
        let template = TemplateEngine::new("Hello {{name}}, welcome to {{company}}!");
        let mut context = TemplateContext::new();
        context
            .set("name", "Alice Johnson")
            .set("company", "Rust Solutions Inc.");

        let rendered = template.render(&context)?;

        page.text()
            .set_font(Font::Helvetica, 14.0)
            .at(50.0, 750.0)
            .write(&rendered)?;
    }

    // Example 2: Template with helpers (dates and times)
    {
        let template =
            TemplateEngine::new("Report generated on {{current_date}} at {{current_time}}");
        let context = TemplateContext::new();

        let rendered = template.render_with_helpers(&context)?;

        page.text()
            .set_font(Font::Helvetica, 10.0)
            .at(50.0, 720.0)
            .write(&rendered)?;
    }

    // Example 3: Advanced template with conditionals
    {
        let template = AdvancedTemplateEngine::new(
            "{{#if premium}}Premium User: {{name}} ({{level}}){{/if}}{{#if standard}}Standard User: {{name}}{{/if}}"
        );

        // Premium user
        let mut context = TemplateContext::new();
        context
            .set("premium", true)
            .set("standard", false)
            .set("name", "Bob Smith")
            .set("level", "Gold");

        let rendered = template.render(&context)?;

        page.text()
            .set_font(Font::HelveticaBold, 12.0)
            .at(50.0, 680.0)
            .write(&rendered)?;
    }

    // Example 4: Template with loops
    {
        let template =
            AdvancedTemplateEngine::new("Shopping List:\n{{#each items}}- {{item}}\n{{/each}}");

        let mut context = TemplateContext::new();
        let items = vec![
            TemplateValue::from("Apples"),
            TemplateValue::from("Bananas"),
            TemplateValue::from("Carrots"),
            TemplateValue::from("Dates"),
        ];
        context.set("items", items);

        let rendered = template.render(&context)?;

        page.text()
            .set_font(Font::Courier, 11.0)
            .at(50.0, 620.0)
            .write(&rendered)?;
    }

    // Example 5: Invoice template with mixed data types
    {
        let template = AdvancedTemplateEngine::new(
            "INVOICE #{{invoice_number}}\n\nCustomer: {{customer_name}}\nTotal Amount: ${{total}}\n{{#if paid}}Status: PAID{{/if}}{{#if unpaid}}Status: UNPAID - Due: {{due_date}}{{/if}}\n\nItems:\n{{#each line_items}}- {{item}} x{{quantity}} @ ${{price}} = ${{subtotal}}\n{{/each}}"
        );

        let mut context = TemplateContext::new();
        context
            .set("invoice_number", "INV-2024-001")
            .set("customer_name", "Tech Corp Ltd.")
            .set("total", 1299.97)
            .set("paid", false)
            .set("unpaid", true)
            .set("due_date", "2024-02-15");

        // Create line items array
        let mut item1 = std::collections::HashMap::new();
        item1.insert("item".to_string(), TemplateValue::from("Laptop"));
        item1.insert("quantity".to_string(), TemplateValue::from(1));
        item1.insert("price".to_string(), TemplateValue::from(999.99));
        item1.insert("subtotal".to_string(), TemplateValue::from(999.99));

        let mut item2 = std::collections::HashMap::new();
        item2.insert("item".to_string(), TemplateValue::from("Mouse"));
        item2.insert("quantity".to_string(), TemplateValue::from(2));
        item2.insert("price".to_string(), TemplateValue::from(29.99));
        item2.insert("subtotal".to_string(), TemplateValue::from(59.98));

        let mut item3 = std::collections::HashMap::new();
        item3.insert("item".to_string(), TemplateValue::from("Keyboard"));
        item3.insert("quantity".to_string(), TemplateValue::from(1));
        item3.insert("price".to_string(), TemplateValue::from(240.00));
        item3.insert("subtotal".to_string(), TemplateValue::from(240.00));

        let line_items = vec![
            TemplateValue::Object(item1),
            TemplateValue::Object(item2),
            TemplateValue::Object(item3),
        ];
        context.set("line_items", line_items);

        let rendered = template.render(&context)?;

        page.text()
            .set_font(Font::Courier, 9.0)
            .at(50.0, 500.0)
            .write(&rendered)?;
    }

    // Example 6: User profile template
    {
        let template = AdvancedTemplateEngine::new(
            "USER PROFILE\n\nName: {{name}}\nEmail: {{email}}\n{{#if admin}}Role: Administrator{{/if}}{{#if user}}Role: Standard User{{/if}}\n\n{{#if skills}}Skills:\n{{#each skills}}- {{item}}\n{{/each}}{{/if}}\n{{#if bio}}Bio: {{bio}}{{/if}}"
        );

        let mut context = TemplateContext::new();
        context
            .set("name", "Charlie Brown")
            .set("email", "charlie.brown@example.com")
            .set("admin", false)
            .set("user", true)
            .set(
                "bio",
                "Software developer with 5 years of experience in Rust and PDF processing.",
            );

        let skills = vec![
            TemplateValue::from("Rust Programming"),
            TemplateValue::from("PDF Processing"),
            TemplateValue::from("System Design"),
            TemplateValue::from("API Development"),
        ];
        context.set("skills", skills);

        let rendered = template.render(&context)?;

        page.text()
            .set_font(Font::TimesRoman, 10.0)
            .at(50.0, 250.0)
            .write(&rendered)?;
    }

    // Add page to document
    doc.add_page(page);

    // Save the document
    doc.save("template_example.pdf")?;
    println!("Created template_example.pdf with various template examples");

    Ok(())
}
