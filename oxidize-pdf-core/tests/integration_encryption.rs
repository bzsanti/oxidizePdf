//! Integration tests for PDF encryption functionality

use oxidize_pdf::document::DocumentEncryption;
use oxidize_pdf::encryption::{
    EncryptionDictionary, EncryptionKey, Permissions, StandardSecurityHandler,
};
use oxidize_pdf::objects::ObjectId;
use oxidize_pdf::{Document, Page};

#[test]
#[ignore = "Encryption feature under development"]
fn test_create_encrypted_pdf() {
    // Create a new PDF document
    let mut doc = Document::new();

    // Add a page with some content
    let mut page = Page::new(595.0, 842.0);
    page.text()
        .set_font(oxidize_pdf::text::Font::Helvetica, 12.0)
        .at(100.0, 700.0)
        .write("This is encrypted content")
        .ok();
    doc.add_page(page);

    // Set up encryption using DocumentEncryption
    doc.encrypt_with_passwords("userpass", "ownerpass");

    // Save to bytes
    let pdf_bytes = doc.to_bytes().unwrap();

    // Verify the PDF contains encryption markers
    let pdf_str = String::from_utf8_lossy(&pdf_bytes);
    assert!(pdf_str.contains("/Encrypt"));
}

#[test]
fn test_encrypt_decrypt_stream() {
    let handler = StandardSecurityHandler::rc4_128bit();
    let key = EncryptionKey::new(vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16]);
    let obj_id = ObjectId::new(10, 0);

    let plaintext = b"This is some test data that should be encrypted and then decrypted";

    // Encrypt
    let encrypted = handler.encrypt_stream(plaintext, &key, &obj_id);
    assert_ne!(encrypted, plaintext);

    // Decrypt
    let decrypted = handler.decrypt_stream(&encrypted, &key, &obj_id);
    assert_eq!(decrypted, plaintext);
}

#[test]
fn test_permissions_enforcement() {
    let mut perms = Permissions::new();

    // Test permission setting
    perms.set_print(true);
    perms.set_copy(false);
    perms.set_modify_contents(false);
    perms.set_fill_forms(true);

    assert!(perms.can_print());
    assert!(!perms.can_copy());
    assert!(!perms.can_modify_contents());
    assert!(perms.can_fill_forms());

    // Test permission bits
    let bits = perms.bits();
    let restored = Permissions::from_bits(bits);

    assert_eq!(restored.can_print(), perms.can_print());
    assert_eq!(restored.can_copy(), perms.can_copy());
    assert_eq!(restored.can_modify_contents(), perms.can_modify_contents());
    assert_eq!(restored.can_fill_forms(), perms.can_fill_forms());
}

#[test]
fn test_40bit_encryption() {
    let handler = StandardSecurityHandler::rc4_40bit();
    let key = EncryptionKey::new(vec![1, 2, 3, 4, 5]); // 40-bit key
    let obj_id = ObjectId::new(1, 0);

    let plaintext = b"Short message";

    let encrypted = handler.encrypt_string(plaintext, &key, &obj_id);
    assert_ne!(encrypted, plaintext);

    let decrypted = handler.decrypt_string(&encrypted, &key, &obj_id);
    assert_eq!(decrypted, plaintext);
}

#[test]
fn test_encryption_dictionary_serialization() {
    let perms = Permissions::all();
    let user_pass = vec![0u8; 32];
    let owner_pass = vec![0xFF; 32];
    let file_id = vec![0xAA; 16];

    let enc_dict = EncryptionDictionary::rc4_128bit(user_pass, owner_pass, perms, Some(file_id));

    // Convert to PDF dictionary
    let dict = enc_dict.to_dict();

    // Verify required fields
    assert!(dict.get("Filter").is_some());
    assert!(dict.get("V").is_some());
    assert!(dict.get("R").is_some());
    assert!(dict.get("O").is_some());
    assert!(dict.get("U").is_some());
    assert!(dict.get("P").is_some());
}
