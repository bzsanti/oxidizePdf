//! Owner password tests for R5/R6 encryption
//!
//! These tests verify owner password functionality for AES-256 encryption:
//! - Algorithm 9: Computing O entry (owner hash)
//! - Algorithm 10: Computing OE entry (encrypted encryption key)
//! - Algorithm 12: Validating owner password
//!
//! Reference: ISO 32000-1:2008 §7.6.4.3.3 (R5) and ISO 32000-2:2020 (R6)

use oxidize_pdf::encryption::{OwnerPassword, StandardSecurityHandler, UserPassword};

// ============================================================================
// R5 Owner Password Tests
// ============================================================================

#[test]
fn test_r5_owner_hash_computation() {
    let handler = StandardSecurityHandler::aes_256_r5();
    let owner_pwd = OwnerPassword("owner_r5".to_string());
    let user_pwd = UserPassword("user_r5".to_string());

    let o_entry = handler
        .compute_r5_owner_hash(&owner_pwd, &user_pwd)
        .expect("R5 owner hash computation should succeed");

    // O entry for R5/R6 is 48 bytes: hash(32) + validation_salt(8) + key_salt(8)
    assert_eq!(o_entry.len(), 48, "R5 O entry should be 48 bytes");
}

#[test]
fn test_r5_owner_password_validation_correct() {
    let handler = StandardSecurityHandler::aes_256_r5();
    let owner_pwd = OwnerPassword("correct_owner".to_string());
    let user_pwd = UserPassword("user".to_string());

    // First compute the O entry
    let o_entry = handler
        .compute_r5_owner_hash(&owner_pwd, &user_pwd)
        .expect("Owner hash computation should succeed");

    // Validate with correct owner password
    let is_valid = handler
        .validate_r5_owner_password(&owner_pwd, &o_entry)
        .expect("Validation should not error");

    assert!(is_valid, "R5: correct owner password should validate");
}

#[test]
fn test_r5_owner_password_validation_incorrect() {
    let handler = StandardSecurityHandler::aes_256_r5();
    let correct_owner = OwnerPassword("correct".to_string());
    let wrong_owner = OwnerPassword("wrong".to_string());
    let user_pwd = UserPassword("user".to_string());

    let o_entry = handler
        .compute_r5_owner_hash(&correct_owner, &user_pwd)
        .expect("Owner hash computation should succeed");

    // Validate with wrong owner password
    let is_invalid = handler
        .validate_r5_owner_password(&wrong_owner, &o_entry)
        .expect("Validation should not error");

    assert!(!is_invalid, "R5: wrong owner password should not validate");
}

#[test]
fn test_r5_oe_entry_computation() {
    let handler = StandardSecurityHandler::aes_256_r5();
    let owner_pwd = OwnerPassword("owner".to_string());
    let user_pwd = UserPassword("user".to_string());

    // Compute O entry
    let o_entry = handler
        .compute_r5_owner_hash(&owner_pwd, &user_pwd)
        .expect("Owner hash computation should succeed");

    // Generate encryption key
    let encryption_key = vec![0x42u8; 32];

    // Compute OE entry
    let oe_entry = handler
        .compute_r5_oe_entry(&owner_pwd, &o_entry, &encryption_key)
        .expect("OE entry computation should succeed");

    // OE is 32 bytes (encrypted encryption key)
    assert_eq!(oe_entry.len(), 32, "R5 OE entry should be 32 bytes");
}

#[test]
fn test_r5_owner_encryption_key_recovery() {
    let handler = StandardSecurityHandler::aes_256_r5();
    let owner_pwd = OwnerPassword("recover_owner".to_string());
    let user_pwd = UserPassword("recover_user".to_string());

    // Compute O entry
    let o_entry = handler
        .compute_r5_owner_hash(&owner_pwd, &user_pwd)
        .expect("Owner hash computation should succeed");

    // Original encryption key
    let original_key = vec![0x55u8; 32];

    // Compute OE entry
    let oe_entry = handler
        .compute_r5_oe_entry(&owner_pwd, &o_entry, &original_key)
        .expect("OE entry computation should succeed");

    // Recover key from OE
    let recovered_key = handler
        .recover_r5_owner_encryption_key(&owner_pwd, &o_entry, &oe_entry)
        .expect("Key recovery should succeed");

    assert_eq!(
        recovered_key, original_key,
        "Recovered key should match original"
    );
}

#[test]
fn test_r5_owner_full_workflow() {
    // Integration test: O + OE + validation + recovery
    let handler = StandardSecurityHandler::aes_256_r5();
    let owner_pwd = OwnerPassword("full_workflow_owner".to_string());
    let user_pwd = UserPassword("full_workflow_user".to_string());

    // 1. Compute O entry
    let o = handler
        .compute_r5_owner_hash(&owner_pwd, &user_pwd)
        .expect("Owner hash should succeed");

    // 2. Generate encryption key
    let key: Vec<u8> = (0..32).collect();

    // 3. Compute OE entry
    let oe = handler
        .compute_r5_oe_entry(&owner_pwd, &o, &key)
        .expect("OE computation should succeed");

    // 4. Validate owner password
    assert!(
        handler.validate_r5_owner_password(&owner_pwd, &o).unwrap(),
        "Owner password should validate"
    );

    // 5. Recover key
    let recovered = handler
        .recover_r5_owner_encryption_key(&owner_pwd, &o, &oe)
        .expect("Key recovery should succeed");
    assert_eq!(recovered, key);
}

// ============================================================================
// R6 Owner Password Tests
// ============================================================================

#[test]
fn test_r6_owner_hash_computation() {
    let handler = StandardSecurityHandler::aes_256_r6();
    let owner_pwd = OwnerPassword("owner_r6".to_string());
    let user_pwd = UserPassword("user_r6".to_string());

    // For R6, we also need the U entry for Algorithm 2.B
    let u_entry = handler
        .compute_r6_user_hash(&user_pwd)
        .expect("User hash computation should succeed");

    let o_entry = handler
        .compute_r6_owner_hash(&owner_pwd, &u_entry)
        .expect("R6 owner hash computation should succeed");

    // O entry for R6 is 48 bytes
    assert_eq!(o_entry.len(), 48, "R6 O entry should be 48 bytes");
}

#[test]
fn test_r6_owner_password_validation_correct() {
    let handler = StandardSecurityHandler::aes_256_r6();
    let owner_pwd = OwnerPassword("correct_owner_r6".to_string());
    let user_pwd = UserPassword("user_r6".to_string());

    let u_entry = handler
        .compute_r6_user_hash(&user_pwd)
        .expect("User hash should succeed");

    let o_entry = handler
        .compute_r6_owner_hash(&owner_pwd, &u_entry)
        .expect("Owner hash should succeed");

    let is_valid = handler
        .validate_r6_owner_password(&owner_pwd, &o_entry, &u_entry)
        .expect("Validation should not error");

    assert!(is_valid, "R6: correct owner password should validate");
}

#[test]
fn test_r6_owner_password_validation_incorrect() {
    let handler = StandardSecurityHandler::aes_256_r6();
    let correct_owner = OwnerPassword("correct".to_string());
    let wrong_owner = OwnerPassword("wrong".to_string());
    let user_pwd = UserPassword("user".to_string());

    let u_entry = handler
        .compute_r6_user_hash(&user_pwd)
        .expect("User hash should succeed");

    let o_entry = handler
        .compute_r6_owner_hash(&correct_owner, &u_entry)
        .expect("Owner hash should succeed");

    let is_invalid = handler
        .validate_r6_owner_password(&wrong_owner, &o_entry, &u_entry)
        .expect("Validation should not error");

    assert!(!is_invalid, "R6: wrong owner password should not validate");
}

#[test]
fn test_r6_oe_entry_computation() {
    let handler = StandardSecurityHandler::aes_256_r6();
    let owner_pwd = OwnerPassword("owner".to_string());
    let user_pwd = UserPassword("user".to_string());

    let u_entry = handler
        .compute_r6_user_hash(&user_pwd)
        .expect("User hash should succeed");

    let o_entry = handler
        .compute_r6_owner_hash(&owner_pwd, &u_entry)
        .expect("Owner hash should succeed");

    let encryption_key = vec![0x42u8; 32];

    let oe_entry = handler
        .compute_r6_oe_entry(&owner_pwd, &o_entry, &u_entry, &encryption_key)
        .expect("OE entry computation should succeed");

    assert_eq!(oe_entry.len(), 32, "R6 OE entry should be 32 bytes");
}

#[test]
fn test_r6_owner_encryption_key_recovery() {
    let handler = StandardSecurityHandler::aes_256_r6();
    let owner_pwd = OwnerPassword("recover_owner".to_string());
    let user_pwd = UserPassword("recover_user".to_string());

    let u_entry = handler
        .compute_r6_user_hash(&user_pwd)
        .expect("User hash should succeed");

    let o_entry = handler
        .compute_r6_owner_hash(&owner_pwd, &u_entry)
        .expect("Owner hash should succeed");

    let original_key = vec![0x55u8; 32];

    let oe_entry = handler
        .compute_r6_oe_entry(&owner_pwd, &o_entry, &u_entry, &original_key)
        .expect("OE computation should succeed");

    let recovered_key = handler
        .recover_r6_owner_encryption_key(&owner_pwd, &o_entry, &u_entry, &oe_entry)
        .expect("Key recovery should succeed");

    assert_eq!(
        recovered_key, original_key,
        "Recovered key should match original"
    );
}

#[test]
fn test_r6_owner_full_workflow() {
    let handler = StandardSecurityHandler::aes_256_r6();
    let owner_pwd = OwnerPassword("full_workflow".to_string());
    let user_pwd = UserPassword("full_workflow".to_string());

    // 1. Compute U entry
    let u = handler
        .compute_r6_user_hash(&user_pwd)
        .expect("User hash should succeed");

    // 2. Compute O entry
    let o = handler
        .compute_r6_owner_hash(&owner_pwd, &u)
        .expect("Owner hash should succeed");

    // 3. Generate encryption key
    let key: Vec<u8> = (0..32).collect();

    // 4. Compute OE entry
    let oe = handler
        .compute_r6_oe_entry(&owner_pwd, &o, &u, &key)
        .expect("OE computation should succeed");

    // 5. Validate owner password
    assert!(
        handler
            .validate_r6_owner_password(&owner_pwd, &o, &u)
            .unwrap(),
        "Owner password should validate"
    );

    // 6. Recover key
    let recovered = handler
        .recover_r6_owner_encryption_key(&owner_pwd, &o, &u, &oe)
        .expect("Key recovery should succeed");
    assert_eq!(recovered, key);
}

// ============================================================================
// Edge Cases
// ============================================================================

#[test]
fn test_r5_owner_empty_password() {
    let handler = StandardSecurityHandler::aes_256_r5();
    let empty_owner = OwnerPassword(String::new());
    let user_pwd = UserPassword("user".to_string());

    let o = handler
        .compute_r5_owner_hash(&empty_owner, &user_pwd)
        .expect("Empty owner password should work");

    assert_eq!(o.len(), 48);

    assert!(handler
        .validate_r5_owner_password(&empty_owner, &o)
        .unwrap());
}

#[test]
fn test_r5_owner_o_entry_invalid_length() {
    let handler = StandardSecurityHandler::aes_256_r5();
    let owner_pwd = OwnerPassword("owner".to_string());

    let short_o = vec![0u8; 32]; // Should be 48

    let result = handler.validate_r5_owner_password(&owner_pwd, &short_o);
    assert!(result.is_err(), "Should error with invalid O entry length");
}

#[test]
fn test_r5_owner_oe_entry_invalid_length() {
    let handler = StandardSecurityHandler::aes_256_r5();
    let owner_pwd = OwnerPassword("owner".to_string());
    let o_entry = vec![0u8; 48];

    let short_oe = vec![0u8; 16]; // Should be 32

    let result = handler.recover_r5_owner_encryption_key(&owner_pwd, &o_entry, &short_oe);
    assert!(result.is_err(), "Should error with invalid OE entry length");
}
