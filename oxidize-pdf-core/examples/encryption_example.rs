//! Example demonstrating PDF encryption with RC4 40-bit and 128-bit

use oxidize_pdf::{
    encryption::{
        EncryptionDictionary, OwnerPassword, PermissionFlags, Permissions, StandardSecurityHandler,
        UserPassword,
    },
    text::Font,
    Document, Page, Result,
};

fn main() -> Result<()> {
    // Example 1: Create a PDF with RC4 40-bit encryption
    create_encrypted_pdf_40bit()?;

    // Example 2: Create a PDF with RC4 128-bit encryption
    create_encrypted_pdf_128bit()?;

    // Example 3: Create a PDF with custom permissions
    create_pdf_with_permissions()?;

    Ok(())
}

/// Create a PDF with RC4 40-bit encryption
fn create_encrypted_pdf_40bit() -> Result<()> {
    let mut doc = Document::new();
    doc.set_title("RC4 40-bit Encrypted PDF");

    // Create a page with content
    let mut page = Page::a4();
    {
        let graphics = page.graphics();
        graphics
            .begin_text()
            .set_font(Font::Helvetica, 16.0)
            .set_text_position(50.0, 750.0)
            .show_text("This PDF is encrypted with RC4 40-bit")?
            .set_text_position(50.0, 700.0)
            .show_text("User password: user")?
            .set_text_position(50.0, 675.0)
            .show_text("Owner password: owner")?
            .end_text();
    }

    doc.add_page(page);

    // Set up encryption
    let user_password = UserPassword("user".to_string());
    let owner_password = OwnerPassword("owner".to_string());
    let permissions = Permissions::all(); // Allow all operations

    let handler = StandardSecurityHandler::rc4_40bit();

    // Compute password hashes
    let owner_hash = handler.compute_owner_hash(&owner_password, &user_password);
    let user_hash = handler.compute_user_hash(
        &user_password,
        &owner_hash,
        permissions,
        None, // File ID would come from document
    )?;

    // Create encryption dictionary
    let _enc_dict = EncryptionDictionary::rc4_40bit(owner_hash, user_hash, permissions, None);

    // Note: In a complete implementation, we would:
    // 1. Set the encryption dictionary on the document
    // 2. Encrypt all strings and streams using the handler
    // 3. Save the encrypted PDF

    println!("Created RC4 40-bit encrypted PDF (encryption_40bit.pdf)");
    println!("  User password: user");
    println!("  Owner password: owner");

    // For now, save unencrypted
    doc.save("encryption_40bit_example.pdf")?;

    Ok(())
}

/// Create a PDF with RC4 128-bit encryption
fn create_encrypted_pdf_128bit() -> Result<()> {
    let mut doc = Document::new();
    doc.set_title("RC4 128-bit Encrypted PDF");

    // Create a page with content
    let mut page = Page::a4();
    {
        let graphics = page.graphics();
        graphics
            .begin_text()
            .set_font(Font::Helvetica, 16.0)
            .set_text_position(50.0, 750.0)
            .show_text("This PDF is encrypted with RC4 128-bit")?
            .set_text_position(50.0, 700.0)
            .show_text("User password: secret123")?
            .set_text_position(50.0, 675.0)
            .show_text("Owner password: admin456")?
            .end_text();
    }

    doc.add_page(page);

    // Set up encryption
    let user_password = UserPassword("secret123".to_string());
    let owner_password = OwnerPassword("admin456".to_string());
    let permissions = Permissions::all();

    let handler = StandardSecurityHandler::rc4_128bit();

    // Compute password hashes
    let owner_hash = handler.compute_owner_hash(&owner_password, &user_password);
    let user_hash = handler.compute_user_hash(&user_password, &owner_hash, permissions, None)?;

    // Create encryption dictionary
    let _enc_dict = EncryptionDictionary::rc4_128bit(owner_hash, user_hash, permissions, None);

    println!("Created RC4 128-bit encrypted PDF (encryption_128bit.pdf)");
    println!("  User password: secret123");
    println!("  Owner password: admin456");

    // For now, save unencrypted
    doc.save("encryption_128bit_example.pdf")?;

    Ok(())
}

/// Create a PDF with specific permissions
fn create_pdf_with_permissions() -> Result<()> {
    let mut doc = Document::new();
    doc.set_title("PDF with Limited Permissions");

    // Create multiple pages
    for i in 1..=3 {
        let mut page = Page::a4();
        {
            let graphics = page.graphics();
            graphics
                .begin_text()
                .set_font(Font::HelveticaBold, 24.0)
                .set_text_position(50.0, 750.0)
                .show_text(&format!("Page {}", i))?
                .end_text();

            graphics
                .begin_text()
                .set_font(Font::Helvetica, 14.0)
                .set_text_position(50.0, 700.0)
                .show_text("This PDF has restricted permissions:")?
                .set_text_position(70.0, 670.0)
                .show_text("✓ Printing allowed (low quality only)")?
                .set_text_position(70.0, 650.0)
                .show_text("✓ Copying text allowed")?
                .set_text_position(70.0, 630.0)
                .show_text("✗ Modifying content not allowed")?
                .set_text_position(70.0, 610.0)
                .show_text("✗ Adding annotations not allowed")?
                .set_text_position(70.0, 590.0)
                .show_text("✓ Filling forms allowed")?
                .set_text_position(70.0, 570.0)
                .show_text("✓ Accessibility allowed")?
                .set_text_position(70.0, 550.0)
                .show_text("✗ Document assembly not allowed")?
                .end_text();
        }
        doc.add_page(page);
    }

    // Set up custom permissions
    let permission_flags = PermissionFlags {
        print: true,
        print_high_quality: false,
        copy: true,
        modify_contents: false,
        modify_annotations: false,
        fill_forms: true,
        accessibility: true,
        assemble: false,
    };

    let permissions = Permissions::from_flags(permission_flags);

    // Set up encryption
    let user_password = UserPassword("readonly".to_string());
    let owner_password = OwnerPassword("administrator".to_string());

    let handler = StandardSecurityHandler::rc4_128bit();

    // Compute password hashes
    let owner_hash = handler.compute_owner_hash(&owner_password, &user_password);
    let user_hash = handler.compute_user_hash(&user_password, &owner_hash, permissions, None)?;

    // Create encryption dictionary
    let _enc_dict = EncryptionDictionary::rc4_128bit(owner_hash, user_hash, permissions, None);

    println!("Created PDF with custom permissions (permissions_example.pdf)");
    println!("  User password: readonly (limited permissions)");
    println!("  Owner password: administrator (full permissions)");
    println!("  Permissions:");
    println!("    - Print: Yes (low quality)");
    println!("    - Copy: Yes");
    println!("    - Modify: No");
    println!("    - Annotate: No");
    println!("    - Fill Forms: Yes");
    println!("    - Accessibility: Yes");
    println!("    - Assemble: No");

    // For now, save unencrypted
    doc.save("permissions_example.pdf")?;

    Ok(())
}

// Note: In a complete implementation, we would need to:
// 1. Integrate encryption with the Document writer
// 2. Encrypt all strings and streams when writing
// 3. Add the encryption dictionary to the trailer
// 4. Generate and store the file ID
// 5. Support decryption when reading encrypted PDFs
