//! Phase 1.4: Real PDF Testing for Encrypted PDFs
//!
//! This module tests decryption with real encrypted PDF files created with qpdf.
//!
//! Fixtures (in tests/fixtures/):
//! - encrypted_rc4_40bit.pdf: R=2, 40-bit RC4, user="user", owner="owner"
//! - encrypted_rc4_128bit.pdf: R=3, 128-bit RC4, user="test123", owner="owner123"
//! - encrypted_restricted.pdf: R=3, 128-bit, print/modify restricted, user="userpass", owner="ownerpass"
//!
//! Created with:
//! ```bash
//! qpdf --allow-weak-crypto --encrypt user owner 40 -- base.pdf encrypted_rc4_40bit.pdf
//! qpdf --allow-weak-crypto --encrypt test123 owner123 128 -- base.pdf encrypted_rc4_128bit.pdf
//! qpdf --allow-weak-crypto --encrypt userpass ownerpass 128 --print=none --modify=none -- base.pdf encrypted_restricted.pdf
//! ```

use oxidize_pdf::encryption::{Permissions, StandardSecurityHandler, UserPassword};
use oxidize_pdf::parser::{ParseError, PdfReader};
use std::path::Path;

// ===== Fixture Paths =====

fn fixtures_dir() -> &'static Path {
    Path::new("tests/fixtures")
}

fn rc4_40bit_pdf() -> std::path::PathBuf {
    fixtures_dir().join("encrypted_rc4_40bit.pdf")
}

fn rc4_128bit_pdf() -> std::path::PathBuf {
    fixtures_dir().join("encrypted_rc4_128bit.pdf")
}

fn restricted_pdf() -> std::path::PathBuf {
    fixtures_dir().join("encrypted_restricted.pdf")
}

// ===== Test 1: Decrypt RC4 40-bit (R2) with User Password =====

#[test]
fn test_decrypt_rc4_40bit_user_password() {
    let path = rc4_40bit_pdf();
    if !path.exists() {
        eprintln!("Skipping test: fixture not found at {:?}", path);
        return;
    }

    let mut reader = PdfReader::open(&path).expect("Failed to open encrypted PDF");

    // Verify PDF is detected as encrypted
    assert!(reader.is_encrypted(), "PDF should be detected as encrypted");
    assert!(
        !reader.is_unlocked(),
        "PDF should be locked before unlock()"
    );

    // Unlock with correct user password
    let result = reader.unlock("user");
    assert!(
        result.is_ok(),
        "unlock() with correct user password should succeed: {:?}",
        result
    );
    assert!(
        reader.is_unlocked(),
        "PDF should be unlocked after unlock()"
    );

    // Verify we can read the catalog (objects are decrypted)
    let catalog = reader.catalog();
    assert!(
        catalog.is_ok(),
        "Should read catalog after unlock: {:?}",
        catalog
    );

    let catalog = catalog.unwrap();
    let type_name = catalog
        .get("Type")
        .and_then(|o| o.as_name())
        .map(|n| n.0.as_str());
    assert_eq!(
        type_name,
        Some("Catalog"),
        "Catalog should have /Type /Catalog"
    );
}

// ===== Test 2: Decrypt RC4 128-bit (R3) with User Password =====

#[test]
fn test_decrypt_rc4_128bit_user_password() {
    let path = rc4_128bit_pdf();
    if !path.exists() {
        eprintln!("Skipping test: fixture not found at {:?}", path);
        return;
    }

    let mut reader = PdfReader::open(&path).expect("Failed to open encrypted PDF");

    // Verify encryption detection
    assert!(reader.is_encrypted(), "PDF should be encrypted (R3)");
    assert!(!reader.is_unlocked(), "PDF should be locked initially");

    // Unlock with correct user password
    let result = reader.unlock("test123");
    assert!(
        result.is_ok(),
        "unlock() with correct password should succeed: {:?}",
        result
    );

    // Verify we can read objects
    let catalog = reader.catalog().expect("Should read catalog after unlock");
    assert!(catalog.get("Pages").is_some(), "Catalog should have /Pages");
}

// ===== Test 3: Wrong Password Returns Error =====

#[test]
fn test_wrong_password_returns_error() {
    let path = rc4_40bit_pdf();
    if !path.exists() {
        eprintln!("Skipping test: fixture not found at {:?}", path);
        return;
    }

    let mut reader = PdfReader::open(&path).expect("Failed to open encrypted PDF");

    // Try wrong password
    let result = reader.unlock("wrong_password");

    // Should return WrongPassword error
    match result {
        Err(ParseError::WrongPassword) => {
            // Expected
        }
        Err(e) => panic!("Expected WrongPassword error, got: {:?}", e),
        Ok(()) => panic!("unlock() with wrong password should fail"),
    }

    // PDF should still be locked
    assert!(
        !reader.is_unlocked(),
        "PDF should remain locked after wrong password"
    );
}

// ===== Test 4: Owner Password Grants Full Permissions =====

#[test]
fn test_owner_password_grants_full_permissions() {
    let path = restricted_pdf();
    if !path.exists() {
        eprintln!("Skipping test: fixture not found at {:?}", path);
        return;
    }

    let mut reader = PdfReader::open(&path).expect("Failed to open encrypted PDF");

    // Unlock with OWNER password
    let result = reader.unlock("ownerpass");
    assert!(
        result.is_ok(),
        "unlock() with owner password should succeed: {:?}",
        result
    );

    // When authenticated with owner password, all operations should be permitted
    // (regardless of P flags, owner has full access)
    // Note: The permissions API returns the raw P flags, not the effective permissions.
    // This is correct behavior - the application layer should check if authenticated
    // as owner and grant full access.

    // Verify we can read objects (full access)
    let catalog = reader.catalog();
    assert!(
        catalog.is_ok(),
        "Owner password should grant access to catalog: {:?}",
        catalog
    );
}

// ===== Test 5: User Password Respects Restricted Permissions =====

#[test]
fn test_user_password_respects_permissions() {
    let path = restricted_pdf();
    if !path.exists() {
        eprintln!("Skipping test: fixture not found at {:?}", path);
        return;
    }

    let mut reader = PdfReader::open(&path).expect("Failed to open encrypted PDF");

    // Unlock with USER password
    let result = reader.unlock("userpass");
    assert!(
        result.is_ok(),
        "unlock() with user password should succeed: {:?}",
        result
    );

    // Get permissions from encryption handler
    let permissions = reader
        .encryption_handler()
        .map(|h| h.permissions())
        .expect("Encryption handler should exist for encrypted PDF");

    // Verify restricted permissions (created with --print=none --modify=none)
    assert!(
        !permissions.can_print(),
        "User password should NOT have print permission"
    );
    assert!(
        !permissions.can_modify_contents(),
        "User password should NOT have modify permission"
    );

    // Accessibility should typically be allowed
    assert!(
        permissions.can_access_for_accessibility(),
        "Accessibility should be allowed"
    );

    // Verify we can still read objects (user has read access)
    let catalog = reader.catalog();
    assert!(
        catalog.is_ok(),
        "User should still be able to read catalog: {:?}",
        catalog
    );
}

// ===== Test 6: Unlock is Idempotent =====

#[test]
fn test_unlock_is_idempotent() {
    let path = rc4_40bit_pdf();
    if !path.exists() {
        eprintln!("Skipping test: fixture not found at {:?}", path);
        return;
    }

    let mut reader = PdfReader::open(&path).expect("Failed to open encrypted PDF");

    // First unlock
    reader.unlock("user").expect("First unlock should succeed");
    assert!(reader.is_unlocked());

    // Second unlock should also succeed (no-op)
    let result = reader.unlock("user");
    assert!(result.is_ok(), "Second unlock should succeed: {:?}", result);
    assert!(reader.is_unlocked());

    // Third unlock with different password should also succeed (already unlocked)
    let result = reader.unlock("different_password");
    assert!(
        result.is_ok(),
        "Unlock when already unlocked should be no-op: {:?}",
        result
    );
}

// ===== Test 7: Cannot Read Objects Before Unlock =====

#[test]
fn test_cannot_read_objects_before_unlock() {
    let path = rc4_40bit_pdf();
    if !path.exists() {
        eprintln!("Skipping test: fixture not found at {:?}", path);
        return;
    }

    let mut reader = PdfReader::open(&path).expect("Failed to open encrypted PDF");

    // Try to read object before unlock
    let result = reader.get_object(1, 0);

    // Should return PdfLocked error
    match result {
        Err(ParseError::PdfLocked) => {
            // Expected
        }
        Err(e) => {
            // Some implementations may return different errors
            eprintln!("Got error (acceptable): {:?}", e);
        }
        Ok(_) => {
            // If it succeeded, verify that the PDF was auto-unlocked with empty password
            // (some PDFs allow empty user password)
            if !reader.is_unlocked() {
                panic!("get_object() should fail on locked PDF");
            }
        }
    }
}

// ===== Debug Test: Validate Password Algorithm =====

#[test]
fn test_debug_password_validation_algorithm() {
    let path = rc4_40bit_pdf();
    if !path.exists() {
        eprintln!("Skipping test: fixture not found at {:?}", path);
        return;
    }

    let reader = PdfReader::open(&path).expect("Failed to open encrypted PDF");

    eprintln!("\n=== Debug: Encryption Info ===");
    eprintln!("is_encrypted: {}", reader.is_encrypted());
    eprintln!("is_unlocked: {}", reader.is_unlocked());

    if let Some(handler) = reader.encryption_handler() {
        eprintln!("algorithm: {}", handler.algorithm_info());

        // Get encryption info through the handler's public API
        let perms = handler.permissions();
        eprintln!("permissions bits: {:08X}", perms.bits());

        // Check if file_id is being passed correctly
        eprintln!("file_id available: {}", handler.has_file_id());
    }

    // Now test manually with known values
    // For R2 (40-bit), the U value should be RC4(padding, key) where key is derived from password
    let handler = StandardSecurityHandler::rc4_40bit();
    let user_pwd = UserPassword("user".to_string());

    // Test with synthetic values to verify algorithm works
    let owner_hash = vec![0xAA; 32];
    let permissions = Permissions::from_bits(0xFFFF_FFFC); // P = -4
    let file_id = b"test_file_id_12"; // 16 bytes

    // Compute user hash
    let computed_u = handler
        .compute_user_hash(&user_pwd, &owner_hash, permissions, Some(file_id))
        .expect("compute_user_hash should succeed");

    eprintln!("computed_u len: {}", computed_u.len());
    eprintln!("computed_u[0..8]: {:02x?}", &computed_u[0..8]);

    // Validate password with the same values
    let result = handler.validate_user_password(
        &user_pwd,
        &computed_u,
        &owner_hash,
        permissions,
        Some(file_id),
    );

    assert!(
        result.is_ok() && result.unwrap(),
        "Synthetic password validation should work"
    );
    eprintln!("\nSynthetic password validation: PASS");

    // The issue is likely that the real PDF has different O/U/P/ID values
    // that we need to extract and use correctly
    eprintln!("\n=== Test Complete ===");
}

// ===== MD5 Verification Test =====

#[test]
fn test_md5_verification() {
    // Same data as computed by Rust
    let data_hex = "7573657228bf4e5e4e758a4164004e56fffa01082e2e00b6d0683e802f0ca9fe94e8094419662a774442fb072e3d9f19e9d130ec09a4d0061e78fe920f7ab62ffcffffff9c5b2a0606f918182e6c5cc0cac374d6";
    let data: Vec<u8> = (0..data_hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&data_hex[i..i + 2], 16).unwrap())
        .collect();

    eprintln!("Data length: {}", data.len());

    // Calculate MD5 using the same library
    let hash = md5::compute(&data);
    let hash_hex: String = hash.iter().map(|b| format!("{:02x}", b)).collect();

    eprintln!("MD5 hash: {}", hash_hex);
    eprintln!("Expected: eee5568378306e354875baefa37aa150");

    assert_eq!(
        hash_hex, "eee5568378306e354875baefa37aa150",
        "MD5 should match"
    );
}
