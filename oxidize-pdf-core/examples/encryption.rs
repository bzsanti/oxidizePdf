//! Example demonstrating PDF encryption capabilities
//!
//! This example shows the various encryption options available:
//! - RC4 40-bit and 128-bit encryption
//! - AES-128 and AES-256 encryption
//! - User and owner passwords
//! - Permission settings

use oxidize_pdf::document::{DocumentEncryption, EncryptionStrength};
use oxidize_pdf::encryption::{PermissionFlags, Permissions};
use oxidize_pdf::{Document, Page};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("PDF Encryption Examples\n");

    // Example 1: Basic RC4 encryption
    rc4_encryption_example()?;

    // Example 2: AES encryption
    aes_encryption_example()?;

    // Example 3: Permissions and restrictions
    permissions_example()?;

    // Example 4: Owner vs User passwords
    dual_password_example()?;

    println!("\nAll encryption examples completed successfully!");
    println!("Note: Encrypted PDFs are saved in examples/results/");

    Ok(())
}

/// Example 1: RC4 encryption (40-bit and 128-bit)
fn rc4_encryption_example() -> Result<(), Box<dyn std::error::Error>> {
    println!("Example 1: RC4 Encryption");
    println!("-------------------------");

    // Create a sample document
    let mut doc = create_sample_document("RC4 Encrypted Document")?;

    // Apply RC4 128-bit encryption
    doc.encrypt_with_passwords(
        "user123",  // User password
        "owner456", // Owner password
    );

    doc.save("examples/results/encrypted_rc4_128.pdf")?;
    println!("✓ Created RC4 128-bit encrypted PDF");
    println!("  User password: user123");
    println!("  Owner password: owner456");

    // Create another with RC4 40-bit (legacy)
    let mut doc_40 = create_sample_document("RC4 40-bit Document")?;
    doc_40.encrypt_with_passwords("legacy", "owner");

    doc_40.save("examples/results/encrypted_rc4_40.pdf")?;
    println!("✓ Created RC4 40-bit encrypted PDF (legacy)");

    Ok(())
}

/// Example 2: AES encryption (128-bit and 256-bit)
fn aes_encryption_example() -> Result<(), Box<dyn std::error::Error>> {
    println!("\nExample 2: AES Encryption");
    println!("-------------------------");

    // AES-128 encryption
    let mut doc = create_sample_document("AES-128 Encrypted Document")?;
    doc.encrypt_with_passwords("secure", "admin");

    doc.save("examples/results/encrypted_aes_128.pdf")?;
    println!("✓ Created AES 128-bit encrypted PDF");
    println!("  User password: secure");

    // AES-256 encryption (highest security)
    let mut doc_256 = create_sample_document("AES-256 Maximum Security")?;
    doc_256.encrypt_with_passwords("topsecret", "administrator");

    doc_256.save("examples/results/encrypted_aes_256.pdf")?;
    println!("✓ Created AES 256-bit encrypted PDF (maximum security)");
    println!("  User password: topsecret");

    Ok(())
}

/// Example 3: Setting permissions and restrictions
fn permissions_example() -> Result<(), Box<dyn std::error::Error>> {
    println!("\nExample 3: Permissions and Restrictions");
    println!("----------------------------------------");

    let mut doc = create_sample_document("Restricted Document")?;

    // Create custom permissions
    let flags = PermissionFlags {
        print: true,               // Allow printing
        copy: false,               // Deny copying
        modify_contents: false,    // Deny modification
        fill_forms: true,          // Allow form filling
        print_high_quality: false, // Deny high-quality printing
        ..Default::default()
    };
    let permissions = Permissions::from_flags(flags);

    // Create encryption with permissions
    let encryption = DocumentEncryption::new(
        "readonly",  // User password
        "admin2024", // Owner password
        permissions,
        EncryptionStrength::Rc4_128bit,
    );

    doc.set_encryption(encryption);
    doc.save("examples/results/encrypted_restricted.pdf")?;

    println!("✓ Created PDF with restricted permissions:");
    println!("  ✅ Printing allowed (low quality only)");
    println!("  ❌ Content copying denied");
    println!("  ❌ Modification denied");
    println!("  ✅ Form filling allowed");
    println!("  User password: readonly");

    Ok(())
}

/// Example 4: Owner vs User passwords demonstration
fn dual_password_example() -> Result<(), Box<dyn std::error::Error>> {
    println!("\nExample 4: Owner vs User Passwords");
    println!("-----------------------------------");

    let mut doc = create_sample_document("Dual Password Document")?;

    // Set restrictive permissions
    let flags = PermissionFlags {
        print: true,
        copy: false,
        modify_contents: false,
        ..Default::default()
    };
    let permissions = Permissions::from_flags(flags);

    // User password: Opens with restrictions
    // Owner password: Opens with full access
    let encryption = DocumentEncryption::new(
        "viewer",        // User password (restricted access)
        "administrator", // Owner password (full access)
        permissions,
        EncryptionStrength::Rc4_128bit,
    );

    doc.set_encryption(encryption);
    doc.save("examples/results/encrypted_dual_password.pdf")?;

    println!("✓ Created PDF with dual password protection:");
    println!("  User password 'viewer': Opens with restrictions");
    println!("    - Can view and print");
    println!("    - Cannot copy or modify");
    println!("  Owner password 'administrator': Full access");
    println!("    - All operations allowed");

    Ok(())
}

/// Helper function to create a sample document
fn create_sample_document(title: &str) -> Result<Document, Box<dyn std::error::Error>> {
    let mut doc = Document::new();

    // Set metadata
    doc.set_title(title);
    doc.set_author("Encryption Example");
    doc.set_subject("Demonstrating PDF encryption");

    // Add a page with content
    let mut page = Page::a4();

    // Title
    page.text()
        .set_font(oxidize_pdf::text::Font::Helvetica, 24.0)
        .at(50.0, 750.0)
        .write(title)?;

    // Content
    page.text()
        .set_font(oxidize_pdf::text::Font::Helvetica, 14.0)
        .at(50.0, 680.0)
        .write("This is a confidential document.")?;

    page.text()
        .set_font(oxidize_pdf::text::Font::Helvetica, 12.0)
        .at(50.0, 650.0)
        .write("It has been encrypted to protect its contents.")?;

    page.text()
        .set_font(oxidize_pdf::text::Font::Helvetica, 12.0)
        .at(50.0, 620.0)
        .write("Only authorized users with the correct password can access it.")?;

    // Add some sensitive data
    page.text()
        .set_font(oxidize_pdf::text::Font::Helvetica, 14.0)
        .at(50.0, 550.0)
        .write("Sensitive Information:")?;

    page.text()
        .set_font(oxidize_pdf::text::Font::Helvetica, 11.0)
        .at(70.0, 520.0)
        .write("• Account Number: XXXX-XXXX-XXXX-1234")?;

    page.text()
        .set_font(oxidize_pdf::text::Font::Helvetica, 11.0)
        .at(70.0, 500.0)
        .write("• Security Code: ***-***-***")?;

    page.text()
        .set_font(oxidize_pdf::text::Font::Helvetica, 11.0)
        .at(70.0, 480.0)
        .write("• Expiration: XX/XX")?;

    doc.add_page(page);
    Ok(doc)
}

/// Bonus: Demonstrate encryption strength comparison
#[allow(dead_code)]
fn encryption_strength_comparison() -> Result<(), Box<dyn std::error::Error>> {
    println!("\nEncryption Strength Comparison");
    println!("-------------------------------");

    let strengths = vec![
        (EncryptionStrength::Rc4_40bit, "RC4 40-bit", "legacy"),
        (EncryptionStrength::Rc4_128bit, "RC4 128-bit", "standard"),
        (EncryptionStrength::Rc4_128bit, "AES 128-bit", "strong"),
        (EncryptionStrength::Rc4_128bit, "AES 256-bit", "maximum"),
    ];

    for (strength, name, level) in strengths {
        println!("  {} - Security level: {}", name, level);
    }

    println!("\nRecommendations:");
    println!("  • Use AES-256 for maximum security");
    println!("  • Use AES-128 for good security with better performance");
    println!("  • Avoid RC4 for new documents (legacy only)");

    Ok(())
}
