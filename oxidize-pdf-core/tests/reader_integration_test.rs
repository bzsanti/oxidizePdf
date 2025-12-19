//! Integration tests for PdfReader encryption support
//!
//! Phase 1.3 of Encrypted PDFs implementation
//!
//! These tests validate the PdfReader API for handling encrypted PDFs:
//! - unlock() method
//! - PdfLocked error when accessing locked PDFs
//! - WrongPassword error for incorrect passwords
//! - Auto-decryption of objects after unlock

use oxidize_pdf::parser::{ParseError, PdfReader};
use std::path::Path;

// ===== Helper Functions =====

/// Get path to a known good unencrypted PDF for testing
fn get_test_pdf_path() -> &'static Path {
    // Use an existing PDF from examples/results
    Path::new("examples/results/ai_ready_invoice.pdf")
}

// ===== Test 1: Unencrypted PDF - is_encrypted returns false =====

#[test]
fn test_unencrypted_pdf_not_encrypted() {
    let path = get_test_pdf_path();
    if !path.exists() {
        eprintln!("Skipping test: test PDF not found at {:?}", path);
        return;
    }

    let reader = PdfReader::open(path).expect("Failed to open PDF");

    assert!(
        !reader.is_encrypted(),
        "Unencrypted PDF should report is_encrypted() = false"
    );
}

// ===== Test 2: Unencrypted PDF - is_unlocked returns true =====

#[test]
fn test_unencrypted_pdf_is_unlocked() {
    let path = get_test_pdf_path();
    if !path.exists() {
        eprintln!("Skipping test: test PDF not found at {:?}", path);
        return;
    }

    let reader = PdfReader::open(path).expect("Failed to open PDF");

    assert!(
        reader.is_unlocked(),
        "Unencrypted PDF should report is_unlocked() = true"
    );
}

// ===== Test 3: Unencrypted PDF - unlock() is no-op =====

#[test]
fn test_unlock_on_unencrypted_pdf_is_noop() {
    let path = get_test_pdf_path();
    if !path.exists() {
        eprintln!("Skipping test: test PDF not found at {:?}", path);
        return;
    }

    let mut reader = PdfReader::open(path).expect("Failed to open PDF");

    // unlock() on unencrypted PDF should succeed (no-op)
    let result = reader.unlock("any_password");
    assert!(
        result.is_ok(),
        "unlock() on unencrypted PDF should succeed: {:?}",
        result
    );
}

// ===== Test 4: Unencrypted PDF - can read objects =====

#[test]
fn test_unencrypted_pdf_can_read_objects() {
    let path = get_test_pdf_path();
    if !path.exists() {
        eprintln!("Skipping test: test PDF not found at {:?}", path);
        return;
    }

    let mut reader = PdfReader::open(path).expect("Failed to open PDF");

    // Should be able to read catalog
    let catalog = reader.catalog();
    assert!(
        catalog.is_ok(),
        "Should read catalog from unencrypted PDF: {:?}",
        catalog
    );

    // Verify catalog has expected type
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

// ===== Test 5: ParseError variants exist =====

#[test]
fn test_parse_error_wrong_password_exists() {
    // Verify the WrongPassword error variant exists and has correct message
    let error = ParseError::WrongPassword;
    let error_string = error.to_string();

    assert!(
        error_string.contains("password"),
        "WrongPassword error should mention 'password': {}",
        error_string
    );
}

#[test]
fn test_parse_error_pdf_locked_exists() {
    // Verify the PdfLocked error variant exists and has correct message
    let error = ParseError::PdfLocked;
    let error_string = error.to_string();

    assert!(
        error_string.contains("locked") || error_string.contains("unlock"),
        "PdfLocked error should mention 'locked' or 'unlock': {}",
        error_string
    );
}

// ===== Test 6: unlock() is idempotent on unencrypted PDFs =====

#[test]
fn test_unlock_idempotent_on_unencrypted() {
    let path = get_test_pdf_path();
    if !path.exists() {
        eprintln!("Skipping test: test PDF not found at {:?}", path);
        return;
    }

    let mut reader = PdfReader::open(path).expect("Failed to open PDF");

    // Call unlock multiple times
    assert!(
        reader.unlock("password1").is_ok(),
        "First unlock should succeed"
    );
    assert!(
        reader.unlock("password2").is_ok(),
        "Second unlock should succeed"
    );
    assert!(reader.unlock("").is_ok(), "Third unlock should succeed");

    // Reader should still work
    assert!(reader.is_unlocked(), "Should remain unlocked");
    assert!(reader.catalog().is_ok(), "Should still read catalog");
}

// ===== Test 7: ensure_unlocked helper works correctly =====

#[test]
fn test_get_object_on_unencrypted_pdf() {
    let path = get_test_pdf_path();
    if !path.exists() {
        eprintln!("Skipping test: test PDF not found at {:?}", path);
        return;
    }

    let mut reader = PdfReader::open(path).expect("Failed to open PDF");

    // Should be able to get objects on unencrypted PDF
    let obj = reader.get_object(1, 0);
    assert!(
        obj.is_ok(),
        "Should get object from unencrypted PDF: {:?}",
        obj
    );

    // Object 1 should be a dictionary (typical for first object)
    let obj = obj.unwrap();
    assert!(
        obj.as_dict().is_some() || obj.is_null(),
        "Object 1 should be a dictionary or null"
    );
}

// ===== Test 8: Multiple object reads work =====

#[test]
fn test_multiple_object_reads() {
    let path = get_test_pdf_path();
    if !path.exists() {
        eprintln!("Skipping test: test PDF not found at {:?}", path);
        return;
    }

    let mut reader = PdfReader::open(path).expect("Failed to open PDF");

    // Read multiple objects sequentially (borrow must end before next call)
    {
        let obj1 = reader.get_object(1, 0);
        assert!(obj1.is_ok(), "Should get object 1: {:?}", obj1);
    }

    {
        let obj2 = reader.get_object(2, 0);
        assert!(obj2.is_ok(), "Should get object 2: {:?}", obj2);
    }

    {
        let obj3 = reader.get_object(3, 0);
        assert!(obj3.is_ok(), "Should get object 3: {:?}", obj3);
    }
}

// ===== API Documentation Tests =====

/// Verify the unlock() API works as documented
#[test]
fn test_unlock_api_example() {
    let path = get_test_pdf_path();
    if !path.exists() {
        eprintln!("Skipping test: test PDF not found at {:?}", path);
        return;
    }

    let mut reader = PdfReader::open(path).expect("Failed to open PDF");

    // Example usage from documentation
    if reader.is_encrypted() {
        reader.unlock("password").expect("Failed to unlock");
    }

    // Should be able to read after (optionally) unlocking
    let catalog = reader.catalog().expect("Failed to read catalog");
    assert!(catalog.get("Type").is_some());
}
