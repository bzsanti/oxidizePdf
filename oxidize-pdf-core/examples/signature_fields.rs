//! Example demonstrating digital signature fields in PDF forms
//!
//! This example shows how to:
//! - Create signature fields with visual appearance
//! - Sign fields with signer information
//! - Lock fields after signing
//! - Validate signatures
//! - Generate signature reports

use oxidize_pdf::forms::signature_field::{SignatureAppearance, SignatureField, SignerInfo};
use oxidize_pdf::forms::signature_handler::{SignatureHandler, SignatureSettings};
use oxidize_pdf::graphics::Color;
use oxidize_pdf::text::Font;
use oxidize_pdf::{Document, Page};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Digital Signature Fields Examples\n");

    // Example 1: Simple signature field
    create_simple_signature_document()?;

    // Example 2: Multiple signature workflow
    create_approval_workflow_document()?;

    // Example 3: Contract with required signatures
    create_contract_document()?;

    // Example 4: Signature with custom appearance
    create_custom_appearance_document()?;

    println!("\nAll signature examples completed successfully!");
    Ok(())
}

/// Create a simple document with a single signature field
fn create_simple_signature_document() -> Result<(), Box<dyn std::error::Error>> {
    println!("Example 1: Simple Signature Field");
    println!("---------------------------------");

    let mut doc = Document::new();
    let mut page = Page::a4();

    // Add document content
    page.text()
        .set_font(Font::HelveticaBold, 16.0)
        .at(50.0, 750.0)
        .write("Document Requiring Signature")?;

    page.text()
        .set_font(Font::Helvetica, 12.0)
        .at(50.0, 700.0)
        .write("This document requires your signature to proceed.")?;

    page.text()
        .set_font(Font::Helvetica, 11.0)
        .at(50.0, 650.0)
        .write("By signing below, you acknowledge that you have read and understood")?;

    page.text()
        .set_font(Font::Helvetica, 11.0)
        .at(50.0, 630.0)
        .write("the terms and conditions of this agreement.")?;

    // Create signature field
    let sig_field = SignatureField::new("signature1")
        .with_reason("Agreement acceptance")
        .with_location("San Francisco, CA");

    // Create signature handler
    let mut handler = SignatureHandler::new();
    handler.add_signature_field(sig_field)?;

    // Draw signature field placeholder
    page.graphics()
        .save_state()
        .set_stroke_color(Color::black())
        .set_line_width(1.0)
        .rectangle(50.0, 100.0, 200.0, 60.0)
        .stroke()
        .restore_state();

    page.text()
        .set_font(Font::Helvetica, 10.0)
        .at(55.0, 140.0)
        .write("Signature:")?;

    // Sign the field (simulation)
    println!("  Signing document...");
    let signer = SignerInfo::new("John Doe")
        .with_email("john.doe@example.com")
        .with_organization("ACME Corp");

    handler.sign_field("signature1", signer, Some("I agree".to_string()))?;

    // Generate signed appearance
    if let Some(signed_field) = handler.get_field("signature1") {
        let _signed_appearance = signed_field.generate_appearance(200.0, 60.0)?;
        println!(
            "  ✓ Document signed by {}",
            signed_field.signer.as_ref().unwrap().name
        );
    }

    doc.add_page(page);
    doc.save("examples/results/signature_simple.pdf")?;

    println!("  ✓ Created signature_simple.pdf");
    Ok(())
}

/// Create a document with multiple signature fields for approval workflow
fn create_approval_workflow_document() -> Result<(), Box<dyn std::error::Error>> {
    println!("\nExample 2: Approval Workflow");
    println!("-----------------------------");

    let mut doc = Document::new();
    let mut page = Page::a4();

    // Document title
    page.text()
        .set_font(Font::HelveticaBold, 18.0)
        .at(50.0, 750.0)
        .write("Purchase Order Approval")?;

    page.text()
        .set_font(Font::Helvetica, 12.0)
        .at(50.0, 700.0)
        .write("PO Number: PO-2024-001")?;

    page.text()
        .set_font(Font::Helvetica, 12.0)
        .at(50.0, 680.0)
        .write("Amount: $45,000.00")?;

    // Create signature handler with settings
    let mut settings = SignatureSettings::default();
    settings.require_all_signatures = true;
    let mut handler = SignatureHandler::with_settings(settings);

    // Create multiple signature fields
    let positions = vec![
        (
            "Manager Approval",
            500.0,
            vec!["reviewer_sig".to_string(), "cfo_sig".to_string()],
        ),
        ("Reviewer Approval", 350.0, vec!["cfo_sig".to_string()]),
        ("CFO Approval", 200.0, vec![]),
    ];

    for (title, y_pos, locks) in positions {
        page.text()
            .set_font(Font::HelveticaBold, 12.0)
            .at(50.0, y_pos + 40.0)
            .write(title)?;

        // Draw signature box
        page.graphics()
            .save_state()
            .set_stroke_color(Color::gray(0.5))
            .set_line_width(1.0)
            .rectangle(50.0, y_pos, 200.0, 50.0)
            .stroke()
            .restore_state();

        // Create signature field
        let field_name = format!("{}_sig", title.to_lowercase().replace(" ", "_"));
        let sig_field = SignatureField::new(&field_name)
            .required()
            .lock_fields_after_signing(locks);

        handler.add_signature_field(sig_field)?;
    }

    // Show signing order
    let order = handler.get_signing_order();
    println!("  Recommended signing order:");
    for (i, field_name) in order.iter().enumerate() {
        println!("    {}. {}", i + 1, field_name);
    }

    // Simulate signing process
    println!("  Simulating approval workflow:");

    // Manager signs first
    let manager = SignerInfo::new("Jane Manager").with_email("jane@company.com");
    handler.sign_field(
        "manager_approval_sig",
        manager,
        Some("Approved".to_string()),
    )?;
    println!("    ✓ Manager approved");

    // Reviewer signs second
    let reviewer = SignerInfo::new("Bob Reviewer").with_email("bob@company.com");
    handler.sign_field(
        "reviewer_approval_sig",
        reviewer,
        Some("Reviewed".to_string()),
    )?;
    println!("    ✓ Reviewer approved");

    // CFO signs last
    let cfo = SignerInfo::new("Alice CFO").with_email("alice@company.com");
    handler.sign_field("cfo_approval_sig", cfo, Some("Final approval".to_string()))?;
    println!("    ✓ CFO approved");

    // Generate summary
    let summary = handler.generate_summary();
    println!("  {}", summary.to_report());

    doc.add_page(page);
    doc.save("examples/results/signature_workflow.pdf")?;

    println!("  ✓ Created signature_workflow.pdf");
    Ok(())
}

/// Create a contract document with required signatures
fn create_contract_document() -> Result<(), Box<dyn std::error::Error>> {
    println!("\nExample 3: Contract with Required Signatures");
    println!("---------------------------------------------");

    let mut doc = Document::new();
    let mut page = Page::a4();

    // Contract header
    page.text()
        .set_font(Font::HelveticaBold, 20.0)
        .at(200.0, 750.0)
        .write("SERVICE AGREEMENT")?;

    page.text()
        .set_font(Font::Helvetica, 11.0)
        .at(50.0, 700.0)
        .write("This Service Agreement is entered into as of the date of final signature")?;

    page.text()
        .set_font(Font::Helvetica, 11.0)
        .at(50.0, 680.0)
        .write("between Company A (Service Provider) and Company B (Client).")?;

    // Contract terms (simplified)
    page.text()
        .set_font(Font::HelveticaBold, 12.0)
        .at(50.0, 630.0)
        .write("Terms and Conditions:")?;

    let terms = vec![
        "1. Service provider agrees to deliver services as specified.",
        "2. Client agrees to pay fees according to the payment schedule.",
        "3. This agreement is valid for 12 months from signature date.",
        "4. Both parties must sign to make this agreement binding.",
    ];

    let mut y = 600.0;
    for term in terms {
        page.text()
            .set_font(Font::Helvetica, 10.0)
            .at(60.0, y)
            .write(term)?;
        y -= 20.0;
    }

    // Create signature handler
    let mut handler = SignatureHandler::new();

    // Service Provider signature
    page.text()
        .set_font(Font::HelveticaBold, 11.0)
        .at(50.0, 250.0)
        .write("Service Provider")?;

    page.graphics()
        .save_state()
        .set_stroke_color(Color::black())
        .set_line_width(1.5)
        .rectangle(50.0, 180.0, 220.0, 60.0)
        .stroke()
        .restore_state();

    let provider_sig = SignatureField::new("provider_signature")
        .required()
        .with_reason("Contract agreement")
        .lock_fields_after_signing(vec!["contract_terms".to_string()]);

    handler.add_signature_field(provider_sig)?;

    // Client signature
    page.text()
        .set_font(Font::HelveticaBold, 11.0)
        .at(320.0, 250.0)
        .write("Client")?;

    page.graphics()
        .save_state()
        .set_stroke_color(Color::black())
        .set_line_width(1.5)
        .rectangle(320.0, 180.0, 220.0, 60.0)
        .stroke()
        .restore_state();

    let client_sig = SignatureField::new("client_signature")
        .required()
        .with_reason("Contract agreement")
        .lock_fields_after_signing(vec!["contract_terms".to_string()]);

    handler.add_signature_field(client_sig)?;

    // Date fields
    page.text()
        .set_font(Font::Helvetica, 10.0)
        .at(50.0, 165.0)
        .write("Date: _________________")?;

    page.text()
        .set_font(Font::Helvetica, 10.0)
        .at(320.0, 165.0)
        .write("Date: _________________")?;

    // Check if contract is ready
    let unsigned = handler.get_unsigned_required_fields();
    if !unsigned.is_empty() {
        println!("  Contract requires signatures from:");
        for field in unsigned {
            println!("    - {}", field);
        }
    }

    // Validate all signatures
    let validation_results = handler.validate_all();
    for result in validation_results {
        if !result.errors.is_empty() {
            println!("  ⚠ {} validation errors:", result.field_name);
            for error in result.errors {
                println!("    - {}", error);
            }
        }
    }

    doc.add_page(page);
    doc.save("examples/results/signature_contract.pdf")?;

    println!("  ✓ Created signature_contract.pdf");
    Ok(())
}

/// Create a document with custom signature appearance
fn create_custom_appearance_document() -> Result<(), Box<dyn std::error::Error>> {
    println!("\nExample 4: Custom Signature Appearance");
    println!("---------------------------------------");

    let mut doc = Document::new();
    let mut page = Page::a4();

    // Document title
    page.text()
        .set_font(Font::HelveticaBold, 16.0)
        .at(50.0, 750.0)
        .write("Custom Signature Appearances")?;

    // Create different signature appearances
    let appearances = vec![
        ("Minimal", create_minimal_appearance()),
        ("Detailed", create_detailed_appearance()),
        ("Branded", create_branded_appearance()),
    ];

    let mut y_pos = 650.0;
    let mut handler = SignatureHandler::new();

    for (style_name, appearance) in appearances {
        // Label
        page.text()
            .set_font(Font::HelveticaBold, 12.0)
            .at(50.0, y_pos)
            .write(&format!("{} Style:", style_name))?;

        // Create signature field with custom appearance
        let field_name = format!("{}_signature", style_name.to_lowercase());
        let mut sig_field = SignatureField::new(&field_name).with_appearance(appearance);

        // Sign it for demonstration
        let signer = SignerInfo::new(format!("{} Signer", style_name))
            .with_email("signer@example.com")
            .with_organization("Demo Company");

        sig_field.sign(signer, Some("Demonstration".to_string()))?;

        // Draw the signature appearance
        page.graphics()
            .save_state()
            .set_stroke_color(Color::gray(0.7))
            .rectangle(50.0, y_pos - 70.0, 250.0, 60.0)
            .stroke()
            .restore_state();

        // Generate and show appearance
        let appearance_stream = sig_field.generate_appearance(250.0, 60.0)?;
        println!(
            "  {} appearance: {} bytes",
            style_name,
            appearance_stream.len()
        );

        handler.add_signature_field(sig_field)?;

        y_pos -= 120.0;
    }

    doc.add_page(page);
    doc.save("examples/results/signature_custom.pdf")?;

    println!("  ✓ Created signature_custom.pdf");
    Ok(())
}

/// Create a minimal signature appearance
fn create_minimal_appearance() -> SignatureAppearance {
    let mut appearance = SignatureAppearance::default();
    appearance.show_name = true;
    appearance.show_date = false;
    appearance.show_reason = false;
    appearance.show_location = false;
    appearance.show_dn = false;
    appearance.show_labels = false;
    appearance.background_color = None;
    appearance.border_width = 0.5;
    appearance.font_size = 12.0;
    appearance
}

/// Create a detailed signature appearance
fn create_detailed_appearance() -> SignatureAppearance {
    let mut appearance = SignatureAppearance::default();
    appearance.show_name = true;
    appearance.show_date = true;
    appearance.show_reason = true;
    appearance.show_location = true;
    appearance.show_dn = true;
    appearance.show_labels = true;
    appearance.background_color = Some(Color::rgb(0.95, 0.95, 1.0));
    appearance.border_color = Color::rgb(0.0, 0.0, 0.5);
    appearance.border_width = 2.0;
    appearance.text_color = Color::rgb(0.0, 0.0, 0.5);
    appearance.font = Font::TimesRoman;
    appearance.font_size = 9.0;
    appearance
}

/// Create a branded signature appearance
fn create_branded_appearance() -> SignatureAppearance {
    let mut appearance = SignatureAppearance::default();
    appearance.show_name = true;
    appearance.show_date = true;
    appearance.show_reason = false;
    appearance.show_location = false;
    appearance.show_dn = false;
    appearance.show_labels = true;
    appearance.background_color = Some(Color::rgb(0.98, 0.98, 0.98));
    appearance.border_color = Color::rgb(0.2, 0.4, 0.8);
    appearance.border_width = 3.0;
    appearance.text_color = Color::rgb(0.2, 0.4, 0.8);
    appearance.font = Font::HelveticaBold;
    appearance.font_size = 11.0;
    // In a real implementation, would include logo_data
    appearance
}
