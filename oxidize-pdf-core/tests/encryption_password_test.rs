//! TDD tests for PDF encryption password validation
//! Phase 1.1 of Encrypted PDFs implementation
//!
//! These tests validate the password validation algorithms according to
//! ISO 32000-1:2008 §7.6.3 (Algorithms 2, 6, 7)

use oxidize_pdf::encryption::{
    EncryptionKey, OwnerPassword, Permissions, StandardSecurityHandler, UserPassword,
};

// ===== Algorithm 2: Encryption Key Computation Tests =====

#[test]
fn test_compute_encryption_key_rc4_40bit() {
    // Given: Known encryption parameters for RC4 40-bit (R2, V1)
    let handler = StandardSecurityHandler::rc4_40bit();
    let user_pwd = UserPassword("test".to_string());
    let owner_hash = vec![0xAA; 32];
    let permissions = Permissions::new();
    let file_id = b"test_file_id_123";

    // When: Computing encryption key
    let result = handler.compute_encryption_key(&user_pwd, &owner_hash, permissions, Some(file_id));

    // Then: Should return 5-byte key (40 bits)
    assert!(result.is_ok(), "Encryption key computation should succeed");
    let key = result.unwrap();
    assert_eq!(key.len(), 5, "RC4 40-bit key should be 5 bytes");
}

#[test]
fn test_compute_encryption_key_rc4_128bit() {
    // Given: Known encryption parameters for RC4 128-bit (R3, V2)
    let handler = StandardSecurityHandler::rc4_128bit();
    let user_pwd = UserPassword("password123".to_string());
    let owner_hash = vec![0xBB; 32];
    let permissions = Permissions::all();
    let file_id = b"document_id_456";

    // When: Computing encryption key
    let result = handler.compute_encryption_key(&user_pwd, &owner_hash, permissions, Some(file_id));

    // Then: Should return 16-byte key (128 bits)
    assert!(result.is_ok(), "Encryption key computation should succeed");
    let key = result.unwrap();
    assert_eq!(key.len(), 16, "RC4 128-bit key should be 16 bytes");
}

#[test]
fn test_compute_encryption_key_deterministic() {
    // Given: Same parameters
    let handler = StandardSecurityHandler::rc4_128bit();
    let user_pwd = UserPassword("same_password".to_string());
    let owner_hash = vec![0xCC; 32];
    let permissions = Permissions::new();
    let file_id = b"same_file_id";

    // When: Computing key twice
    let key1 = handler
        .compute_encryption_key(&user_pwd, &owner_hash, permissions, Some(file_id))
        .unwrap();
    let key2 = handler
        .compute_encryption_key(&user_pwd, &owner_hash, permissions, Some(file_id))
        .unwrap();

    // Then: Should produce identical keys
    assert_eq!(
        key1.as_bytes(),
        key2.as_bytes(),
        "Same inputs should produce same encryption key"
    );
}

#[test]
fn test_compute_encryption_key_different_passwords() {
    // Given: Different passwords, same other parameters
    let handler = StandardSecurityHandler::rc4_128bit();
    let user_pwd1 = UserPassword("password1".to_string());
    let user_pwd2 = UserPassword("password2".to_string());
    let owner_hash = vec![0xDD; 32];
    let permissions = Permissions::new();
    let file_id = b"file_id";

    // When: Computing keys for different passwords
    let key1 = handler
        .compute_encryption_key(&user_pwd1, &owner_hash, permissions, Some(file_id))
        .unwrap();
    let key2 = handler
        .compute_encryption_key(&user_pwd2, &owner_hash, permissions, Some(file_id))
        .unwrap();

    // Then: Should produce different keys
    assert_ne!(
        key1.as_bytes(),
        key2.as_bytes(),
        "Different passwords should produce different keys"
    );
}

// ===== Algorithm 6: User Password Validation Tests =====

#[test]
fn test_validate_user_password_correct_r2() {
    // Given: Encrypted PDF metadata with known correct password (R2)
    let handler = StandardSecurityHandler::rc4_40bit();
    let user_pwd = UserPassword("correct".to_string());
    let owner_hash = vec![0xEE; 32];
    let permissions = Permissions::new();
    let file_id = b"test_id";

    // Compute the expected user hash
    let user_hash = handler
        .compute_user_hash(&user_pwd, &owner_hash, permissions, Some(file_id))
        .expect("User hash computation should succeed");

    // When: Validating with correct password
    let result = handler.validate_user_password(
        &user_pwd,
        &user_hash,
        &owner_hash,
        permissions,
        Some(file_id),
    );

    // Then: Should return Ok(true)
    assert!(
        result.is_ok(),
        "Password validation should not return error"
    );
    assert!(
        result.unwrap(),
        "Correct password should validate successfully"
    );
}

#[test]
fn test_validate_user_password_incorrect_r2() {
    // Given: Encrypted PDF metadata with wrong password (R2)
    let handler = StandardSecurityHandler::rc4_40bit();
    let correct_pwd = UserPassword("correct".to_string());
    let wrong_pwd = UserPassword("wrong".to_string());
    let owner_hash = vec![0xFF; 32];
    let permissions = Permissions::new();
    let file_id = b"test_id";

    // Compute user hash with correct password
    let user_hash = handler
        .compute_user_hash(&correct_pwd, &owner_hash, permissions, Some(file_id))
        .expect("User hash computation should succeed");

    // When: Validating with wrong password
    let result = handler.validate_user_password(
        &wrong_pwd,
        &user_hash,
        &owner_hash,
        permissions,
        Some(file_id),
    );

    // Then: Should return Ok(false) - NOT an error
    assert!(
        result.is_ok(),
        "Password validation should not return error for wrong password"
    );
    assert!(
        !result.unwrap(),
        "Wrong password should not validate successfully"
    );
}

#[test]
fn test_validate_user_password_correct_r3() {
    // Given: Encrypted PDF metadata with known correct password (R3)
    let handler = StandardSecurityHandler::rc4_128bit();
    let user_pwd = UserPassword("mypassword".to_string());
    let owner_hash = vec![0x11; 32];
    let permissions = Permissions::all();
    let file_id = b"doc_id_r3";

    // Compute the expected user hash
    let user_hash = handler
        .compute_user_hash(&user_pwd, &owner_hash, permissions, Some(file_id))
        .expect("User hash computation should succeed");

    // When: Validating with correct password
    let result = handler.validate_user_password(
        &user_pwd,
        &user_hash,
        &owner_hash,
        permissions,
        Some(file_id),
    );

    // Then: Should return Ok(true)
    assert!(
        result.is_ok(),
        "Password validation should not return error"
    );
    assert!(
        result.unwrap(),
        "Correct password should validate successfully"
    );
}

#[test]
fn test_validate_user_password_incorrect_r3() {
    // Given: Encrypted PDF metadata with wrong password (R3)
    let handler = StandardSecurityHandler::rc4_128bit();
    let correct_pwd = UserPassword("correct_r3".to_string());
    let wrong_pwd = UserPassword("wrong_r3".to_string());
    let owner_hash = vec![0x22; 32];
    let permissions = Permissions::new();
    let file_id = b"doc_id_r3";

    // Compute user hash with correct password
    let user_hash = handler
        .compute_user_hash(&correct_pwd, &owner_hash, permissions, Some(file_id))
        .expect("User hash computation should succeed");

    // When: Validating with wrong password
    let result = handler.validate_user_password(
        &wrong_pwd,
        &user_hash,
        &owner_hash,
        permissions,
        Some(file_id),
    );

    // Then: Should return Ok(false) - NOT an error
    assert!(
        result.is_ok(),
        "Password validation should not return error for wrong password"
    );
    assert!(
        !result.unwrap(),
        "Wrong password should not validate successfully"
    );
}

#[test]
fn test_validate_empty_user_password() {
    // Given: PDF encrypted with empty user password (common case ~40%)
    let handler = StandardSecurityHandler::rc4_128bit();
    let empty_pwd = UserPassword("".to_string());
    let owner_hash = vec![0x33; 32];
    let permissions = Permissions::new();
    let file_id = b"empty_pwd_doc";

    // Compute user hash with empty password
    let user_hash = handler
        .compute_user_hash(&empty_pwd, &owner_hash, permissions, Some(file_id))
        .expect("User hash computation should succeed");

    // When: Validating with empty password
    let result = handler.validate_user_password(
        &empty_pwd,
        &user_hash,
        &owner_hash,
        permissions,
        Some(file_id),
    );

    // Then: Should return Ok(true)
    assert!(
        result.is_ok(),
        "Empty password validation should not return error"
    );
    assert!(
        result.unwrap(),
        "Empty password should validate successfully when document uses empty password"
    );
}

// ===== Algorithm 7: Owner Password Validation Tests =====

#[test]
fn test_validate_owner_password_correct() {
    // Given: Encrypted PDF metadata with known correct owner password
    let handler = StandardSecurityHandler::rc4_128bit();
    let owner_pwd = OwnerPassword("owner_secret".to_string());
    let user_pwd = UserPassword("user123".to_string());
    let permissions = Permissions::new();
    let file_id = b"owner_test_id";

    // Compute owner hash
    let owner_hash = handler.compute_owner_hash(&owner_pwd, &user_pwd);

    // When: Validating with correct owner password
    let result = handler.validate_owner_password(
        &owner_pwd,
        &owner_hash,
        &user_pwd,
        permissions,
        Some(file_id),
    );

    // Then: Should return Ok(true)
    assert!(
        result.is_ok(),
        "Owner password validation should not return error"
    );
    assert!(
        result.unwrap(),
        "Correct owner password should validate successfully"
    );
}

#[test]
fn test_validate_owner_password_incorrect() {
    // Given: Encrypted PDF metadata with wrong owner password
    let handler = StandardSecurityHandler::rc4_128bit();
    let correct_owner = OwnerPassword("correct_owner".to_string());
    let wrong_owner = OwnerPassword("wrong_owner".to_string());
    let user_pwd = UserPassword("user456".to_string());
    let permissions = Permissions::new();
    let file_id = b"owner_test_id2";

    // Compute owner hash with correct password
    let owner_hash = handler.compute_owner_hash(&correct_owner, &user_pwd);

    // When: Validating with wrong owner password
    let result = handler.validate_owner_password(
        &wrong_owner,
        &owner_hash,
        &user_pwd,
        permissions,
        Some(file_id),
    );

    // Then: Should return Ok(false) - NOT an error
    assert!(
        result.is_ok(),
        "Owner password validation should not return error for wrong password"
    );
    assert!(
        !result.unwrap(),
        "Wrong owner password should not validate successfully"
    );
}

// ===== Edge Cases and Integration Tests =====

#[test]
fn test_validate_password_no_file_id() {
    // Given: PDF without file ID (optional in some cases)
    let handler = StandardSecurityHandler::rc4_128bit();
    let user_pwd = UserPassword("test".to_string());
    let owner_hash = vec![0x44; 32];
    let permissions = Permissions::new();

    // Compute user hash without file ID
    let user_hash = handler
        .compute_user_hash(&user_pwd, &owner_hash, permissions, None)
        .expect("User hash computation without file_id should succeed");

    // When: Validating without file ID
    let result =
        handler.validate_user_password(&user_pwd, &user_hash, &owner_hash, permissions, None);

    // Then: Should still work
    assert!(result.is_ok(), "Validation without file_id should work");
    assert!(
        result.unwrap(),
        "Password should validate even without file_id"
    );
}

#[test]
fn test_validate_password_with_all_permissions() {
    // Given: PDF with all permissions enabled
    let handler = StandardSecurityHandler::rc4_128bit();
    let user_pwd = UserPassword("all_perms".to_string());
    let owner_hash = vec![0x55; 32];
    let permissions = Permissions::all();
    let file_id = b"all_perms_id";

    // Compute user hash
    let user_hash = handler
        .compute_user_hash(&user_pwd, &owner_hash, permissions, Some(file_id))
        .expect("User hash computation should succeed");

    // When: Validating password
    let result = handler.validate_user_password(
        &user_pwd,
        &user_hash,
        &owner_hash,
        permissions,
        Some(file_id),
    );

    // Then: Should validate successfully
    assert!(result.is_ok());
    assert!(result.unwrap(), "Should validate with all permissions");
}

#[test]
fn test_validate_password_different_permissions() {
    // Given: Two PDFs with different permissions but same password
    let handler = StandardSecurityHandler::rc4_128bit();
    let user_pwd = UserPassword("same_pwd".to_string());
    let owner_hash = vec![0x66; 32];
    let file_id = b"perms_test";

    let perms1 = Permissions::new();
    let perms2 = Permissions::all();

    // Compute user hashes with different permissions
    let user_hash1 = handler
        .compute_user_hash(&user_pwd, &owner_hash, perms1, Some(file_id))
        .unwrap();
    let user_hash2 = handler
        .compute_user_hash(&user_pwd, &owner_hash, perms2, Some(file_id))
        .unwrap();

    // Then: Should produce different hashes (permissions affect the key)
    assert_ne!(
        user_hash1, user_hash2,
        "Different permissions should produce different user hashes"
    );
}

#[test]
fn test_encryption_key_is_cloneable() {
    // Test that EncryptionKey implements Clone (required for interior mutability)
    let key = EncryptionKey::new(vec![1, 2, 3, 4, 5]);
    let cloned = key.clone();

    assert_eq!(key.as_bytes(), cloned.as_bytes());
    assert_eq!(key.len(), cloned.len());
}

#[test]
fn test_password_structs_are_cloneable() {
    // Test that password structs implement Clone
    let user = UserPassword("user".to_string());
    let owner = OwnerPassword("owner".to_string());

    let user_clone = user.clone();
    let owner_clone = owner.clone();

    assert_eq!(user.0, user_clone.0);
    assert_eq!(owner.0, owner_clone.0);
}
