//! Complete Invoice Generation Example
//!
//! This example demonstrates a production-ready invoice generation system using oxidize-pdf.
//! It includes:
//! - Company branding
//! - Professional table layouts
//! - Tax calculations
//! - Multiple currencies
//! - Customer information
//! - Line items with quantities and prices
//! - Professional formatting and styling
//!
//! Run with: `cargo run --example invoice_complete`

use oxidize_pdf::error::Result;
use oxidize_pdf::{Color, Document, Font, Page};
use std::fs;

/// Invoice data structure representing a complete invoice
#[derive(Debug, Clone)]
pub struct Invoice {
    pub invoice_number: String,
    pub date: String,
    pub due_date: String,
    pub company: CompanyInfo,
    pub customer: CustomerInfo,
    pub items: Vec<InvoiceItem>,
    pub currency: String,
    pub tax_rate: f64,
    pub notes: Option<String>,
}

#[derive(Debug, Clone)]
pub struct CompanyInfo {
    pub name: String,
    pub address: String,
    pub city: String,
    pub postal_code: String,
    pub country: String,
    pub email: String,
    pub phone: String,
    pub tax_id: String,
}

#[derive(Debug, Clone)]
pub struct CustomerInfo {
    pub name: String,
    pub address: String,
    pub city: String,
    pub postal_code: String,
    pub country: String,
    pub email: Option<String>,
    pub customer_id: Option<String>,
}

#[derive(Debug, Clone)]
pub struct InvoiceItem {
    pub description: String,
    pub quantity: f64,
    pub unit_price: f64,
    pub unit: String,
}

impl InvoiceItem {
    pub fn total(&self) -> f64 {
        self.quantity * self.unit_price
    }
}

impl Invoice {
    pub fn subtotal(&self) -> f64 {
        self.items.iter().map(|item| item.total()).sum()
    }

    pub fn tax_amount(&self) -> f64 {
        self.subtotal() * self.tax_rate
    }

    pub fn total(&self) -> f64 {
        self.subtotal() + self.tax_amount()
    }

    pub fn format_amount(&self, amount: f64) -> String {
        match self.currency.as_str() {
            "EUR" => format!("{:.2}", amount),
            "USD" => format!("${:.2}", amount),
            "GBP" => format!("{:.2}", amount),
            _ => format!("{} {:.2}", self.currency, amount),
        }
    }
}

/// Invoice styling configuration
pub struct InvoiceStyle {
    pub primary_color: Color,
    pub secondary_color: Color,
    pub text_color: Color,
    pub title_font_size: f64,
    pub header_font_size: f64,
    pub body_font_size: f64,
    pub small_font_size: f64,
    pub margin: f64,
}

impl Default for InvoiceStyle {
    fn default() -> Self {
        InvoiceStyle {
            primary_color: Color::rgb(0.0, 0.35, 0.71),
            secondary_color: Color::rgb(0.9, 0.9, 0.9),
            text_color: Color::rgb(0.2, 0.2, 0.2),
            title_font_size: 24.0,
            header_font_size: 12.0,
            body_font_size: 10.0,
            small_font_size: 8.0,
            margin: 50.0,
        }
    }
}

/// Generate a complete PDF invoice
pub fn generate_invoice(invoice: &Invoice, style: &InvoiceStyle) -> Result<Document> {
    let mut doc = Document::new();
    doc.set_title(&format!("Invoice {}", invoice.invoice_number));

    let mut page = Page::a4();
    let mut y = 780.0;

    // Header - Company info
    y = draw_header(&mut page, &invoice.company, style, y)?;

    // Invoice title and number
    y = draw_invoice_title(&mut page, invoice, style, y)?;

    // Customer info
    y = draw_customer_info(&mut page, &invoice.customer, style, y)?;

    // Invoice details
    y = draw_invoice_details(&mut page, invoice, style, y)?;

    // Line items table
    y = draw_items_table(&mut page, &invoice.items, invoice, style, y)?;

    // Totals
    y = draw_totals(&mut page, invoice, style, y)?;

    // Footer
    draw_footer(&mut page, invoice, style, y)?;

    doc.add_page(page);
    Ok(doc)
}

fn draw_header(
    page: &mut Page,
    company: &CompanyInfo,
    style: &InvoiceStyle,
    y: f64,
) -> Result<f64> {
    // Company name
    page.text()
        .set_font(Font::HelveticaBold, style.title_font_size)
        .at(style.margin, y)
        .write(&company.name)?;

    let mut current_y = y - 35.0;

    // Company details
    let details = [
        company.address.as_str(),
        &format!("{} {}", company.postal_code, company.city),
        company.country.as_str(),
        &format!("Email: {}", company.email),
        &format!("Phone: {}", company.phone),
        &format!("Tax ID: {}", company.tax_id),
    ];

    for detail in &details {
        page.text()
            .set_font(Font::Helvetica, style.body_font_size)
            .at(style.margin, current_y)
            .write(detail)?;
        current_y -= 12.0;
    }

    // Separator line
    current_y -= 10.0;
    page.graphics()
        .set_stroke_color(style.secondary_color)
        .set_line_width(1.0)
        .move_to(style.margin, current_y)
        .line_to(545.0, current_y)
        .stroke();

    Ok(current_y - 20.0)
}

fn draw_invoice_title(
    page: &mut Page,
    invoice: &Invoice,
    style: &InvoiceStyle,
    y: f64,
) -> Result<f64> {
    // INVOICE title on the right
    page.text()
        .set_font(Font::HelveticaBold, style.title_font_size)
        .at(400.0, y)
        .write("INVOICE")?;

    page.text()
        .set_font(Font::Helvetica, style.header_font_size)
        .at(400.0, y - 25.0)
        .write(&format!("# {}", invoice.invoice_number))?;

    Ok(y - 50.0)
}

fn draw_customer_info(
    page: &mut Page,
    customer: &CustomerInfo,
    style: &InvoiceStyle,
    y: f64,
) -> Result<f64> {
    page.text()
        .set_font(Font::HelveticaBold, style.header_font_size)
        .at(style.margin, y)
        .write("BILL TO:")?;

    let mut current_y = y - 20.0;

    let details = [
        customer.name.as_str(),
        customer.address.as_str(),
        &format!("{} {}", customer.postal_code, customer.city),
        customer.country.as_str(),
    ];

    for detail in &details {
        page.text()
            .set_font(Font::Helvetica, style.body_font_size)
            .at(style.margin, current_y)
            .write(detail)?;
        current_y -= 12.0;
    }

    if let Some(ref email) = customer.email {
        page.text()
            .set_font(Font::Helvetica, style.small_font_size)
            .at(style.margin, current_y)
            .write(&format!("Email: {}", email))?;
        current_y -= 10.0;
    }

    if let Some(ref customer_id) = customer.customer_id {
        page.text()
            .set_font(Font::Helvetica, style.small_font_size)
            .at(style.margin, current_y)
            .write(&format!("Customer ID: {}", customer_id))?;
        current_y -= 10.0;
    }

    Ok(current_y - 20.0)
}

fn draw_invoice_details(
    page: &mut Page,
    invoice: &Invoice,
    style: &InvoiceStyle,
    y: f64,
) -> Result<f64> {
    let details_x = 400.0;
    let mut current_y = y;

    let details = [
        ("Invoice Date:", invoice.date.as_str()),
        ("Due Date:", invoice.due_date.as_str()),
        ("Currency:", invoice.currency.as_str()),
    ];

    for (label, value) in &details {
        page.text()
            .set_font(Font::Helvetica, style.body_font_size)
            .at(details_x, current_y)
            .write(label)?;

        page.text()
            .set_font(Font::Helvetica, style.body_font_size)
            .at(details_x + 80.0, current_y)
            .write(value)?;

        current_y -= 14.0;
    }

    Ok(current_y - 20.0)
}

fn draw_items_table(
    page: &mut Page,
    items: &[InvoiceItem],
    invoice: &Invoice,
    style: &InvoiceStyle,
    y: f64,
) -> Result<f64> {
    let mut current_y = y;
    let col_x = [style.margin, 250.0, 310.0, 380.0, 460.0];

    // Table header background
    page.graphics()
        .set_fill_color(style.secondary_color)
        .rect(style.margin, current_y - 5.0, 495.0, 20.0)
        .fill();

    // Header text
    let headers = ["Description", "Qty", "Unit", "Price", "Total"];
    for (i, header) in headers.iter().enumerate() {
        page.text()
            .set_font(Font::HelveticaBold, style.body_font_size)
            .at(col_x[i], current_y)
            .write(header)?;
    }

    current_y -= 25.0;

    // Table rows
    for (row_idx, item) in items.iter().enumerate() {
        // Alternate row background
        if row_idx % 2 == 0 {
            page.graphics()
                .set_fill_color(Color::rgb(0.98, 0.98, 0.98))
                .rect(style.margin, current_y - 3.0, 495.0, 16.0)
                .fill();
        }

        // Description (truncate if too long)
        let desc = if item.description.len() > 35 {
            format!("{}...", &item.description[..32])
        } else {
            item.description.clone()
        };

        page.text()
            .set_font(Font::Helvetica, style.body_font_size)
            .at(col_x[0], current_y)
            .write(&desc)?;

        page.text()
            .set_font(Font::Helvetica, style.body_font_size)
            .at(col_x[1], current_y)
            .write(&format!("{:.1}", item.quantity))?;

        page.text()
            .set_font(Font::Helvetica, style.body_font_size)
            .at(col_x[2], current_y)
            .write(&item.unit)?;

        page.text()
            .set_font(Font::Helvetica, style.body_font_size)
            .at(col_x[3], current_y)
            .write(&invoice.format_amount(item.unit_price))?;

        page.text()
            .set_font(Font::Helvetica, style.body_font_size)
            .at(col_x[4], current_y)
            .write(&invoice.format_amount(item.total()))?;

        current_y -= 18.0;
    }

    // Table border
    page.graphics()
        .set_stroke_color(style.text_color)
        .set_line_width(0.5)
        .rect(style.margin, current_y, 495.0, y - current_y + 15.0)
        .stroke();

    Ok(current_y - 25.0)
}

fn draw_totals(page: &mut Page, invoice: &Invoice, style: &InvoiceStyle, y: f64) -> Result<f64> {
    let totals_x = 380.0;
    let values_x = 480.0;
    let mut current_y = y;

    // Totals background
    page.graphics()
        .set_fill_color(style.secondary_color)
        .rect(totals_x - 10.0, current_y - 45.0, 175.0, 60.0)
        .fill();

    // Subtotal
    page.text()
        .set_font(Font::Helvetica, style.body_font_size)
        .at(totals_x, current_y)
        .write("Subtotal:")?;
    page.text()
        .set_font(Font::Helvetica, style.body_font_size)
        .at(values_x, current_y)
        .write(&invoice.format_amount(invoice.subtotal()))?;

    current_y -= 15.0;

    // Tax
    page.text()
        .set_font(Font::Helvetica, style.body_font_size)
        .at(totals_x, current_y)
        .write(&format!("Tax ({:.0}%):", invoice.tax_rate * 100.0))?;
    page.text()
        .set_font(Font::Helvetica, style.body_font_size)
        .at(values_x, current_y)
        .write(&invoice.format_amount(invoice.tax_amount()))?;

    current_y -= 15.0;

    // Total
    page.text()
        .set_font(Font::HelveticaBold, style.header_font_size)
        .at(totals_x, current_y)
        .write("TOTAL:")?;
    page.text()
        .set_font(Font::HelveticaBold, style.header_font_size)
        .at(values_x, current_y)
        .write(&invoice.format_amount(invoice.total()))?;

    Ok(current_y - 40.0)
}

fn draw_footer(page: &mut Page, invoice: &Invoice, style: &InvoiceStyle, y: f64) -> Result<f64> {
    let mut current_y = y;

    // Notes
    if let Some(ref notes) = invoice.notes {
        page.text()
            .set_font(Font::HelveticaBold, style.body_font_size)
            .at(style.margin, current_y)
            .write("Notes:")?;

        current_y -= 15.0;

        // Word wrap notes (simple approach)
        let max_chars = 80;
        for chunk in notes.as_bytes().chunks(max_chars) {
            let line = String::from_utf8_lossy(chunk);
            page.text()
                .set_font(Font::Helvetica, style.small_font_size)
                .at(style.margin, current_y)
                .write(&line)?;
            current_y -= 10.0;
        }

        current_y -= 10.0;
    }

    // Payment due reminder
    page.text()
        .set_font(Font::Helvetica, style.small_font_size)
        .at(style.margin, current_y)
        .write(&format!("Payment is due by {}", invoice.due_date))?;

    current_y -= 12.0;

    page.text()
        .set_font(Font::Helvetica, style.small_font_size)
        .at(style.margin, current_y)
        .write("Thank you for your business!")?;

    Ok(current_y)
}

/// Create a sample invoice for demonstration
fn create_sample_invoice() -> Invoice {
    Invoice {
        invoice_number: "INV-2025-001".to_string(),
        date: "2025-08-27".to_string(),
        due_date: "2025-09-27".to_string(),
        currency: "EUR".to_string(),
        tax_rate: 0.21,

        company: CompanyInfo {
            name: "Rust PDF Solutions Ltd.".to_string(),
            address: "123 Innovation Street".to_string(),
            city: "Amsterdam".to_string(),
            postal_code: "1012 AB".to_string(),
            country: "Netherlands".to_string(),
            email: "hello@rustpdf.com".to_string(),
            phone: "+31 20 123 4567".to_string(),
            tax_id: "NL123456789B01".to_string(),
        },

        customer: CustomerInfo {
            name: "Tech Startup BV".to_string(),
            address: "456 Business Park".to_string(),
            city: "Rotterdam".to_string(),
            postal_code: "3000 CD".to_string(),
            country: "Netherlands".to_string(),
            email: Some("accounting@techstartup.nl".to_string()),
            customer_id: Some("CUST-2025-042".to_string()),
        },

        items: vec![
            InvoiceItem {
                description: "PDF Processing Library Development".to_string(),
                quantity: 40.0,
                unit_price: 85.00,
                unit: "hours".to_string(),
            },
            InvoiceItem {
                description: "API Documentation and Examples".to_string(),
                quantity: 12.0,
                unit_price: 75.00,
                unit: "hours".to_string(),
            },
            InvoiceItem {
                description: "Performance Optimization".to_string(),
                quantity: 8.0,
                unit_price: 95.00,
                unit: "hours".to_string(),
            },
            InvoiceItem {
                description: "Support and Maintenance (Monthly)".to_string(),
                quantity: 1.0,
                unit_price: 250.00,
                unit: "month".to_string(),
            },
        ],

        notes: Some(
            "Payment terms: Net 30 days. Late payments subject to 1.5% monthly interest."
                .to_string(),
        ),
    }
}

fn main() -> Result<()> {
    println!("Generating professional invoice PDF...");

    // Create sample invoice
    let invoice = create_sample_invoice();

    // Use default styling
    let style = InvoiceStyle::default();

    // Generate the PDF
    let mut document = generate_invoice(&invoice, &style)?;

    // Save to file
    let output_path = "examples/results/invoice_complete.pdf";
    fs::create_dir_all("examples/results")?;
    document.save(output_path)?;

    // Print summary
    println!("Invoice generated successfully!");
    println!("File: {}", output_path);
    println!("Invoice Summary:");
    println!("   Invoice #: {}", invoice.invoice_number);
    println!("   Customer: {}", invoice.customer.name);
    println!("   Items: {} line items", invoice.items.len());
    println!("   Subtotal: {}", invoice.format_amount(invoice.subtotal()));
    println!(
        "   Tax ({:.0}%): {}",
        invoice.tax_rate * 100.0,
        invoice.format_amount(invoice.tax_amount())
    );
    println!("   Total: {}", invoice.format_amount(invoice.total()));

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_invoice_calculations() {
        let invoice = create_sample_invoice();

        assert_eq!(invoice.items[0].total(), 3400.00);
        assert_eq!(invoice.items[1].total(), 900.00);
        assert_eq!(invoice.items[2].total(), 760.00);
        assert_eq!(invoice.items[3].total(), 250.00);

        assert_eq!(invoice.subtotal(), 5310.00);
        assert_eq!(invoice.tax_amount(), 1115.10);
        assert_eq!(invoice.total(), 6425.10);
    }

    #[test]
    fn test_currency_formatting() {
        let mut invoice = create_sample_invoice();

        invoice.currency = "USD".to_string();
        assert_eq!(invoice.format_amount(1234.56), "$1234.56");

        invoice.currency = "JPY".to_string();
        assert_eq!(invoice.format_amount(1234.56), "JPY 1234.56");
    }
}
