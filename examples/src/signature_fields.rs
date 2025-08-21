//! Example demonstrating signature fields with widget annotations
//!
//! This example shows how to:
//! - Create signature fields with different visual types
//! - Add widget annotations for signature fields
//! - Generate appearance streams for signatures
//! - Create ink signatures (handwritten)
//! - Mix text and graphics in signatures

use oxidize_pdf::{Document, Page, PdfError};
use oxidize_pdf::forms::{
    signature_field::{SignatureField, SignerInfo, SignatureAppearance},
    signature_widget::{SignatureWidget, SignatureVisualType, InkStroke, 
                       TextPosition, ImageFormat},
    signature_handler::{SignatureHandler, SignatureSettings},
    Widget, WidgetAppearance, BorderStyle,
};
use oxidize_pdf::geometry::Rectangle;
use oxidize_pdf::graphics::Color;
use oxidize_pdf::text::Font;
use std::path::Path;

fn main() -> Result<(), PdfError> {
    println!("📝 Creating PDF with signature fields examples...");

    // Create a new document
    let mut doc = Document::new();

    // Add multiple pages demonstrating different signature types
    create_text_signature_page(&mut doc)?;
    create_ink_signature_page(&mut doc)?;
    create_mixed_signature_page(&mut doc)?;
    create_multiple_signatures_page(&mut doc)?;

    // Save the document
    let output_path = "examples/results/signature_fields_example.pdf";
    doc.save(output_path)?;
    
    println!("✅ PDF with signature fields created successfully!");
    println!("📄 Output: {}", output_path);

    // Demonstrate signing a field
    demonstrate_signing()?;

    Ok(())
}

/// Create a page with text-based signature field
fn create_text_signature_page(doc: &mut Document) -> Result<(), PdfError> {
    let mut page = Page::new(612.0, 792.0); // Letter size
    
    // Add title
    page.add_text("Text-Based Signature Field Example", 50.0, 700.0, Font::HelveticaBold, 16.0)?;
    
    // Add some document content
    page.add_text("This document requires your digital signature for approval.", 
                  50.0, 650.0, Font::Helvetica, 12.0)?;
    page.add_text("By signing this document, you acknowledge that you have read and", 
                  50.0, 630.0, Font::Helvetica, 12.0)?;
    page.add_text("understood all terms and conditions.", 
                  50.0, 610.0, Font::Helvetica, 12.0)?;

    // Create signature field
    let mut sig_field = SignatureField::new("signature1")
        .with_reason("Document Approval")
        .with_location("San Francisco, CA")
        .required();

    // Customize appearance
    let mut appearance = SignatureAppearance::default();
    appearance.show_name = true;
    appearance.show_date = true;
    appearance.show_reason = true;
    appearance.show_location = true;
    appearance.background_color = Some(Color::gray(0.95));
    appearance.border_color = Color::rgb(0.0, 0.0, 0.5);
    appearance.border_width = 2.0;
    appearance.font_size = 10.0;
    
    sig_field = sig_field.with_appearance(appearance);

    // Create widget for the signature field
    let rect = Rectangle::new(100.0, 450.0, 400.0, 550.0);
    let visual = SignatureVisualType::Text {
        show_name: true,
        show_date: true,
        show_reason: true,
        show_location: true,
    };
    
    let mut widget = SignatureWidget::new(rect, visual);
    widget.widget.appearance.border_style = BorderStyle::Solid;
    widget.widget.appearance.border_width = 2.0;
    widget.widget.appearance.border_color = Some(Color::rgb(0.0, 0.0, 0.5));
    widget.widget.appearance.background_color = Some(Color::gray(0.98));

    // Generate appearance stream
    let appearance_stream = widget.generate_appearance_stream(
        false, // unsigned
        None,
        None,
        None,
        None,
    )?;

    // Add signature field label
    page.add_text("Signature:", 50.0, 530.0, Font::Helvetica, 12.0)?;
    
    // Add the widget to the page
    // Note: In a complete implementation, this would be integrated with FormManager
    page.add_content(&appearance_stream)?;

    doc.add_page(page);
    Ok(())
}

/// Create a page with ink (handwritten) signature field
fn create_ink_signature_page(doc: &mut Document) -> Result<(), PdfError> {
    let mut page = Page::new(612.0, 792.0);
    
    // Add title
    page.add_text("Ink Signature Field Example", 50.0, 700.0, Font::HelveticaBold, 16.0)?;
    
    // Add instructions
    page.add_text("Please sign with your finger or stylus in the box below:", 
                  50.0, 650.0, Font::Helvetica, 12.0)?;

    // Create sample ink strokes (simulating a signature)
    let stroke1 = InkStroke {
        points: vec![
            (110.0, 500.0),
            (130.0, 480.0),
            (150.0, 490.0),
            (170.0, 480.0),
            (190.0, 500.0),
        ],
        pressures: None,
    };
    
    let stroke2 = InkStroke {
        points: vec![
            (200.0, 490.0),
            (220.0, 470.0),
            (240.0, 480.0),
            (260.0, 470.0),
            (280.0, 490.0),
        ],
        pressures: None,
    };
    
    let stroke3 = InkStroke {
        points: vec![
            (120.0, 460.0),
            (280.0, 460.0),
        ],
        pressures: None,
    };

    // Create ink signature widget
    let rect = Rectangle::new(100.0, 430.0, 400.0, 530.0);
    let visual = SignatureVisualType::InkSignature {
        strokes: vec![stroke1, stroke2, stroke3],
        color: Color::rgb(0.0, 0.0, 0.5),
        width: 2.0,
    };
    
    let mut widget = SignatureWidget::new(rect, visual);
    widget.widget.appearance.border_style = BorderStyle::Dashed;
    widget.widget.appearance.border_width = 1.0;
    widget.widget.appearance.border_color = Some(Color::gray(0.5));

    // Generate appearance stream
    let appearance_stream = widget.generate_appearance_stream(
        true, // signed
        Some("John Doe"),
        Some("Agreement"),
        Some("New York, NY"),
        Some("2025-08-13 10:30:00"),
    )?;

    // Add the appearance to the page
    page.add_content(&appearance_stream)?;

    doc.add_page(page);
    Ok(())
}

/// Create a page with mixed text and graphic signature
fn create_mixed_signature_page(doc: &mut Document) -> Result<(), PdfError> {
    let mut page = Page::new(612.0, 792.0);
    
    // Add title
    page.add_text("Mixed Signature Field Example", 50.0, 700.0, Font::HelveticaBold, 16.0)?;
    
    // Add description
    page.add_text("This signature combines an image with text information:", 
                  50.0, 650.0, Font::Helvetica, 12.0)?;

    // Create mixed signature widget
    let rect = Rectangle::new(100.0, 450.0, 450.0, 550.0);
    
    // Create placeholder image data (in real use, this would be actual image data)
    let image_data = vec![0u8; 100]; // Placeholder
    
    let visual = SignatureVisualType::Mixed {
        image_data,
        format: ImageFormat::PNG,
        text_position: TextPosition::Below,
        show_details: true,
    };
    
    let mut widget = SignatureWidget::new(rect, visual);
    widget.widget.appearance.border_style = BorderStyle::Beveled;
    widget.widget.appearance.border_width = 2.0;
    widget.widget.appearance.border_color = Some(Color::rgb(0.2, 0.2, 0.6));
    widget.widget.appearance.background_color = Some(Color::gray(0.95));

    // Generate appearance stream
    let appearance_stream = widget.generate_appearance_stream(
        true,
        Some("Jane Smith"),
        Some("Review and Approval"),
        Some("Los Angeles, CA"),
        Some("2025-08-13 14:45:00"),
    )?;

    page.add_content(&appearance_stream)?;

    doc.add_page(page);
    Ok(())
}

/// Create a page with multiple signature fields
fn create_multiple_signatures_page(doc: &mut Document) -> Result<(), PdfError> {
    let mut page = Page::new(612.0, 792.0);
    
    // Add title
    page.add_text("Multiple Signatures Example", 50.0, 700.0, Font::HelveticaBold, 16.0)?;
    
    // Add description
    page.add_text("This document requires multiple signatures:", 
                  50.0, 650.0, Font::Helvetica, 12.0)?;

    // First signature - Author
    page.add_text("Author Signature:", 50.0, 600.0, Font::HelveticaBold, 12.0)?;
    let rect1 = Rectangle::new(50.0, 520.0, 250.0, 580.0);
    let visual1 = SignatureVisualType::Text {
        show_name: true,
        show_date: true,
        show_reason: false,
        show_location: false,
    };
    
    let widget1 = SignatureWidget::new(rect1, visual1);
    let appearance1 = widget1.generate_appearance_stream(
        false, None, None, None, None
    )?;
    page.add_content(&appearance1)?;

    // Second signature - Reviewer
    page.add_text("Reviewer Signature:", 320.0, 600.0, Font::HelveticaBold, 12.0)?;
    let rect2 = Rectangle::new(320.0, 520.0, 520.0, 580.0);
    let visual2 = SignatureVisualType::Text {
        show_name: true,
        show_date: true,
        show_reason: false,
        show_location: false,
    };
    
    let widget2 = SignatureWidget::new(rect2, visual2);
    let appearance2 = widget2.generate_appearance_stream(
        false, None, None, None, None
    )?;
    page.add_content(&appearance2)?;

    // Third signature - Approver
    page.add_text("Approver Signature:", 50.0, 450.0, Font::HelveticaBold, 12.0)?;
    let rect3 = Rectangle::new(50.0, 370.0, 250.0, 430.0);
    let visual3 = SignatureVisualType::Text {
        show_name: true,
        show_date: true,
        show_reason: true,
        show_location: false,
    };
    
    let widget3 = SignatureWidget::new(rect3, visual3);
    let appearance3 = widget3.generate_appearance_stream(
        true,
        Some("Michael Johnson"),
        Some("Final Approval"),
        None,
        Some("2025-08-13 16:00:00"),
    )?;
    page.add_content(&appearance3)?;

    // Fourth signature - Witness
    page.add_text("Witness Signature:", 320.0, 450.0, Font::HelveticaBold, 12.0)?;
    let rect4 = Rectangle::new(320.0, 370.0, 520.0, 430.0);
    
    // Create an ink signature for witness
    let witness_stroke1 = InkStroke {
        points: vec![
            (330.0, 400.0),
            (350.0, 380.0),
            (370.0, 400.0),
            (390.0, 380.0),
            (410.0, 400.0),
        ],
        pressures: None,
    };
    
    let visual4 = SignatureVisualType::InkSignature {
        strokes: vec![witness_stroke1],
        color: Color::black(),
        width: 1.5,
    };
    
    let widget4 = SignatureWidget::new(rect4, visual4);
    let appearance4 = widget4.generate_appearance_stream(
        true,
        Some("Sarah Wilson"),
        None,
        None,
        None,
    )?;
    page.add_content(&appearance4)?;

    doc.add_page(page);
    Ok(())
}

/// Demonstrate signing a signature field
fn demonstrate_signing() -> Result<(), PdfError> {
    println!("\n📝 Demonstrating signature field signing...");

    // Create a signature handler
    let mut handler = SignatureHandler::new();
    
    // Configure settings
    let settings = SignatureSettings {
        incremental_save: true,
        require_all_signatures: false,
        lock_all_after_first_signature: false,
        default_algorithm: oxidize_pdf::forms::signature_field::SignatureAlgorithm::RsaSha256,
        enable_timestamps: true,
    };
    
    let mut handler = SignatureHandler::with_settings(settings);

    // Create a signature field
    let mut sig_field = SignatureField::new("contract_signature")
        .with_reason("Contract Agreement")
        .with_location("San Francisco, CA")
        .required();

    // Create signer info
    let signer = SignerInfo::new("Alice Johnson")
        .with_email("alice@example.com")
        .with_organization("ACME Corporation");

    // Sign the field
    sig_field.sign(signer.clone(), Some("I agree to the terms".to_string()))?;
    
    println!("✅ Field signed successfully!");
    println!("   Signer: {}", signer.name);
    println!("   Signed: {}", sig_field.is_signed());
    
    // Verify the signature
    let is_valid = sig_field.verify()?;
    println!("   Valid: {}", is_valid);

    // Add the field to the handler
    handler.add_signature_field(sig_field)?;
    
    println!("✅ Signature field added to handler");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_signature_field_creation() {
        let field = SignatureField::new("test_sig")
            .with_reason("Testing")
            .required();
        
        assert_eq!(field.name, "test_sig");
        assert_eq!(field.reason, Some("Testing".to_string()));
        assert!(field.required);
        assert!(!field.is_signed());
    }

    #[test]
    fn test_signature_widget_types() {
        let rect = Rectangle::new(0.0, 0.0, 200.0, 100.0);
        
        // Test text signature
        let text_visual = SignatureVisualType::Text {
            show_name: true,
            show_date: true,
            show_reason: false,
            show_location: false,
        };
        let text_widget = SignatureWidget::new(rect.clone(), text_visual);
        assert!(text_widget.field_ref.is_none());
        
        // Test ink signature
        let ink_visual = SignatureVisualType::InkSignature {
            strokes: vec![],
            color: Color::black(),
            width: 2.0,
        };
        let ink_widget = SignatureWidget::new(rect.clone(), ink_visual);
        assert!(ink_widget.handler_ref.is_none());
    }

    #[test]
    fn test_signing_process() {
        let mut field = SignatureField::new("test");
        let signer = SignerInfo::new("Test User");
        
        // Sign the field
        assert!(field.sign(signer.clone(), None).is_ok());
        assert!(field.is_signed());
        
        // Cannot sign twice
        assert!(field.sign(signer, None).is_err());
        
        // Verify signature
        assert!(field.verify().unwrap());
    }

    #[test]
    fn test_appearance_generation() {
        let rect = Rectangle::new(0.0, 0.0, 300.0, 100.0);
        let visual = SignatureVisualType::Text {
            show_name: true,
            show_date: false,
            show_reason: false,
            show_location: false,
        };
        
        let widget = SignatureWidget::new(rect, visual);
        let appearance = widget.generate_appearance_stream(
            true,
            Some("John Doe"),
            None,
            None,
            None,
        );
        
        assert!(appearance.is_ok());
        let stream = appearance.unwrap();
        assert!(!stream.is_empty());
        
        // Verify content
        let content = String::from_utf8_lossy(&stream);
        assert!(content.contains("John Doe"));
    }
}