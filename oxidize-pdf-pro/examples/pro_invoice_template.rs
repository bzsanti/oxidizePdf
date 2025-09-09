use oxidize_pdf::Document;
use oxidize_pdf_pro::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Pro Invoice Template Example");

    // Initialize Pro features
    let license_result = initialize(Some("OXIDIZE_PRO_DEV"));
    match license_result {
        Ok(_) => println!("✓ Pro license validated successfully"),
        Err(e) => println!(
            "⚠ License validation failed: {}, continuing with limited features",
            e
        ),
    }

    // Check template features
    println!("\n1. Testing Pro template features...");
    match FeatureGate::check_template_features() {
        Ok(_) => {
            println!("✓ Template features are available");

            // Create professional invoice template
            println!("✓ Creating professional invoice template...");
            let invoice = ProInvoiceTemplate::new()
                .customer("ACME Corporation")
                .invoice_number("INV-2024-001")
                .add_line_item("Professional Services", 2500.00)
                .add_line_item("Consulting Hours", 1200.00)
                .with_tax_rate(0.10)
                .with_schema_org_markup();

            match invoice.build() {
                Ok(document) => {
                    println!("✓ Professional invoice template created successfully");

                    // Save the generated document
                    std::fs::create_dir_all("examples/results")?;
                    document.save("examples/results/pro_invoice.pdf")?;
                    println!("✓ Invoice saved to examples/results/pro_invoice.pdf");

                    // Show some details
                    println!("✓ Invoice details:");
                    println!("  - Customer: ACME Corporation");
                    println!("  - Invoice Number: INV-2024-001");
                    println!("  - Line Items: 2");
                    println!("  - Subtotal: $3,700.00");
                    println!("  - Tax (10%): $370.00");
                    println!("  - Total: $4,070.00");
                    println!("  - Schema.org markup: Enabled");
                }
                Err(e) => println!("⚠ Template creation failed: {}", e),
            }
        }
        Err(e) => println!("⚠ Template features not available: {}", e),
    }

    // Try creating a contract template as well
    println!("\n2. Testing contract template...");
    match FeatureGate::check_template_features() {
        Ok(_) => {
            let contract = ProContractTemplate::new()
                .title("Service Agreement")
                .party("BelowZero Corp", "Client")
                .party("ACME Corporation", "Provider")
                .term("Service delivery within 30 days")
                .term("Payment due within 15 days of completion")
                .with_legal_formatting();

            match contract.build() {
                Ok(document) => {
                    println!("✓ Contract template created successfully");
                    document.save("examples/results/pro_contract.pdf")?;
                    println!("✓ Contract saved to examples/results/pro_contract.pdf");
                }
                Err(e) => println!("⚠ Contract creation failed: {}", e),
            }
        }
        Err(e) => println!("⚠ Template features not available: {}", e),
    }

    println!("\n🎉 Pro template demo completed!");
    Ok(())
}
