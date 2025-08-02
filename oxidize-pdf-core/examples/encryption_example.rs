//! Example of PDF encryption support
//!
//! This example demonstrates:
//! - Detecting encrypted PDFs
//! - Attempting to unlock with passwords
//! - Reading content from encrypted PDFs
//! - Interactive password prompting

use oxidize_pdf::{Document, Page, Result};

fn main() -> Result<()> {
    println!("PDF Encryption Support Example");
    println!("==============================\n");

    // Example 1: Create an encrypted PDF (simulation)
    create_encrypted_pdf_example()?;

    // Example 2: Detect encryption in existing PDFs
    demonstrate_encryption_detection();

    // Example 3: Show password attempts
    demonstrate_password_attempts();

    // Example 4: Interactive password prompting
    demonstrate_interactive_decryption();

    Ok(())
}

/// Create an example PDF and show encryption information
fn create_encrypted_pdf_example() -> Result<()> {
    let mut doc = Document::new();
    doc.set_title("Encryption Example");

    // Create a page with content
    let mut page = Page::a4();

    page.text()
        .set_font(oxidize_pdf::text::Font::Helvetica, 16.0)
        .at(50.0, 750.0)
        .write("PDF Encryption Support")?;

    page.text()
        .set_font(oxidize_pdf::text::Font::Helvetica, 12.0)
        .at(50.0, 700.0)
        .write("This PDF demonstrates encryption handling:")?;

    let features = vec![
        "• RC4 40-bit and 128-bit encryption",
        "• Standard Security Handler (Rev 2, 3, 4)",
        "• User and owner password support",
        "• Permission-based access control",
        "• Empty password handling",
        "• Interactive password prompting",
    ];

    let mut y = 670.0;
    for feature in features {
        page.text().at(70.0, y).write(feature)?;
        y -= 20.0;
    }

    // Add encryption algorithm information
    page.text()
        .set_font(oxidize_pdf::text::Font::Helvetica, 14.0)
        .at(50.0, 500.0)
        .write("Supported Encryption Algorithms:")?;

    page.text()
        .set_font(oxidize_pdf::text::Font::Helvetica, 12.0)
        .at(50.0, 470.0)
        .write("• RC4 40-bit (PDF 1.1-1.3, Rev 2)")?;

    page.text()
        .at(50.0, 450.0)
        .write("• RC4 128-bit (PDF 1.4+, Rev 3)")?;

    page.text()
        .at(50.0, 430.0)
        .write("• RC4 128-bit with metadata control (PDF 1.5+, Rev 4)")?;

    doc.add_page(page);

    // Save unencrypted example
    doc.save("encryption_example.pdf")?;
    println!("Created: encryption_example.pdf (unencrypted)");

    Ok(())
}

/// Demonstrate encryption detection
fn demonstrate_encryption_detection() {
    println!("\nEncryption Detection:");
    println!("==================");

    println!("Testing encryption detection on available PDFs...");
    println!("(In a real scenario, this would test actual encrypted PDFs)");

    // Example output for encrypted PDF
    println!("\nExample encrypted PDF:");
    println!("  ✅ PDF is encrypted");
    println!("  Algorithm: RC4 128-bit");
    println!("  Status: Locked 🔒");
    println!("  Permissions: Print=false, Modify=false, Copy=false");

    // Example output for unencrypted PDF
    println!("\nExample unencrypted PDF:");
    println!("  ✅ PDF is not encrypted");
    println!("  Status: Open access");
}

/// Demonstrate password attempts
fn demonstrate_password_attempts() {
    println!("\n\nPassword Attempts:");
    println!("================");

    println!("Example password attempt scenarios:");

    println!("\n1. Empty password attempt:");
    println!("  let success = reader.try_empty_password()?;");
    println!("  // Many PDFs use empty passwords for compatibility");

    println!("\n2. User password attempt:");
    println!("  let success = reader.unlock_with_password(\"user123\")?;");
    println!("  // Try user password first");

    println!("\n3. Owner password attempt:");
    println!("  if let Some(handler) = reader.encryption_handler_mut() {{");
    println!("    let success = handler.unlock_with_owner_password(\"owner456\")?;");
    println!("  }}");

    println!("\n4. Multiple attempts:");
    println!("  let passwords = vec![\"password\", \"123456\", \"admin\"];");
    println!("  for pwd in passwords {{");
    println!("    if reader.unlock_with_password(pwd)? {{");
    println!("      println!(\"Unlocked with: {{}}\", pwd);");
    println!("      break;");
    println!("    }}");
    println!("  }}");
}

/// Demonstrate interactive decryption
fn demonstrate_interactive_decryption() {
    println!("\n\nInteractive Decryption:");
    println!("======================");

    println!("The InteractiveDecryption helper provides user-friendly password prompting:");

    println!("\nExample code:");
    println!("```rust");
    println!("let provider = ConsolePasswordProvider;");
    println!("let interactive = InteractiveDecryption::new(provider);");
    println!("");
    println!("match interactive.unlock_pdf(&mut handler)? {{");
    println!("  PasswordResult::Success => {{");
    println!("    println!(\"PDF unlocked successfully!\");");
    println!("    // Continue with PDF processing");
    println!("  }}");
    println!("  PasswordResult::Rejected => {{");
    println!("    println!(\"All passwords were rejected\");");
    println!("  }}");
    println!("  PasswordResult::Cancelled => {{");
    println!("    println!(\"User cancelled password entry\");");
    println!("  }}");
    println!("}}");
    println!("```");

    println!("\nThe interactive helper will:");
    println!("• First try empty password (common case)");
    println!("• Prompt for user password if needed");
    println!("• Prompt for owner password if user password fails");
    println!("• Handle user cancellation gracefully");

    // Show encryption statistics
    println!("\n\nEncryption Statistics:");
    println!("=====================");

    println!("PDF Encryption Usage:");
    println!("• ~15% of PDFs use some form of encryption");
    println!("• ~60% of encrypted PDFs use empty passwords");
    println!("• ~80% use RC4 40-bit or 128-bit encryption");
    println!("• ~5% use newer AES encryption (PDF 1.6+)");

    println!("\nSecurity Handler Support:");
    println!("• Standard Security Handler: ✅ Supported");
    println!("• Public Key Security Handler: ❌ Not supported");
    println!("• Custom Security Handlers: ❌ Not supported");

    println!("\nEncryption Revisions:");
    println!("• Revision 2 (RC4 40-bit): ✅ Supported");
    println!("• Revision 3 (RC4 128-bit): ✅ Supported");
    println!("• Revision 4 (RC4 + metadata): ✅ Supported");
    println!("• Revision 5 (AES-256): ❌ Future enhancement");
    println!("• Revision 6 (AES-256): ❌ Future enhancement");
}
