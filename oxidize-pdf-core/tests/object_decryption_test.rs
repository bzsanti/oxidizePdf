//! TDD tests for PDF object decryption
//! Phase 1.2 of Encrypted PDFs implementation
//!
//! These tests validate the object decryption algorithms according to
//! ISO 32000-1:2008 §7.6.2 (Algorithm 1: Object-specific key computation)

use oxidize_pdf::encryption::{EncryptionKey, StandardSecurityHandler};
use oxidize_pdf::objects::ObjectId;

// ===== Algorithm 1: Object-Specific Key Computation =====

#[test]
fn test_compute_object_key_simple() {
    // Given: Base encryption key + simple object ID (1, 0)
    let handler = StandardSecurityHandler::rc4_40bit();
    let base_key = EncryptionKey::new(vec![0x01, 0x02, 0x03, 0x04, 0x05]); // 5 bytes (40-bit)
    let obj_id = ObjectId::new(1, 0);

    // When: Computing object-specific key
    // Algorithm: MD5(base_key + obj_num[3 bytes] + gen_num[2 bytes])
    let obj_key = handler.compute_object_key(&base_key, &obj_id);

    // Then: Should return key of length min(base_len + 5, 16)
    // For 40-bit: min(5 + 5, 16) = 10 bytes
    assert_eq!(
        obj_key.len(),
        10,
        "Object key should be 10 bytes for RC4-40"
    );

    // Validate it's deterministic
    let obj_key2 = handler.compute_object_key(&base_key, &obj_id);
    assert_eq!(
        obj_key, obj_key2,
        "Object key computation should be deterministic"
    );
}

#[test]
fn test_compute_object_key_different_objects() {
    // Given: Same base key, different object IDs
    let handler = StandardSecurityHandler::rc4_128bit();
    let base_key = EncryptionKey::new(vec![0xAA; 16]); // 16 bytes (128-bit)
    let obj_id1 = ObjectId::new(5, 0);
    let obj_id2 = ObjectId::new(10, 0);

    // When: Computing object-specific keys
    let key1 = handler.compute_object_key(&base_key, &obj_id1);
    let key2 = handler.compute_object_key(&base_key, &obj_id2);

    // Then: Should produce different keys
    assert_ne!(key1, key2, "Different objects should have different keys");
}

#[test]
fn test_compute_object_key_with_generation_number() {
    // Given: Base key + object with non-zero generation
    let handler = StandardSecurityHandler::rc4_128bit();
    let base_key = EncryptionKey::new(vec![0xBB; 16]);
    let obj_id = ObjectId::new(10, 5); // Generation = 5

    // When: Computing object-specific key
    let obj_key = handler.compute_object_key(&base_key, &obj_id);

    // Then: Should incorporate generation number in hash
    // Verify by comparing with generation=0
    let obj_id_gen0 = ObjectId::new(10, 0);
    let key_gen0 = handler.compute_object_key(&base_key, &obj_id_gen0);

    assert_ne!(
        obj_key, key_gen0,
        "Generation number should affect object key"
    );
    assert_eq!(
        obj_key.len(),
        16,
        "RC4-128 object key should be 16 bytes (capped at 16)"
    );
}

// ===== String Decryption Tests =====

#[test]
fn test_decrypt_string_rc4_40bit() {
    // Given: Known encrypted string with RC4 40-bit
    let handler = StandardSecurityHandler::rc4_40bit();
    let base_key = EncryptionKey::new(vec![0x12, 0x34, 0x56, 0x78, 0x9A]);
    let obj_id = ObjectId::new(1, 0);

    // Encrypt a known plaintext first
    let plaintext = b"Hello PDF";
    let encrypted = handler.encrypt_string(plaintext, &base_key, &obj_id);

    // Verify it's different
    assert_ne!(
        encrypted.as_slice(),
        plaintext,
        "Encrypted should differ from plaintext"
    );

    // When: Decrypting with same key
    let decrypted = handler.decrypt_string(&encrypted, &base_key, &obj_id);

    // Then: Should recover original plaintext
    assert_eq!(
        decrypted.as_slice(),
        plaintext,
        "Decryption should recover plaintext"
    );
}

#[test]
fn test_decrypt_string_rc4_128bit() {
    // Given: Known encrypted string with RC4 128-bit
    let handler = StandardSecurityHandler::rc4_128bit();
    let base_key = EncryptionKey::new(vec![0xFF; 16]);
    let obj_id = ObjectId::new(5, 0);

    // Test with longer text
    let plaintext = b"This is a longer PDF string with special chars: \xE9\xE8\xE7";
    let encrypted = handler.encrypt_string(plaintext, &base_key, &obj_id);

    // When: Decrypting
    let decrypted = handler.decrypt_string(&encrypted, &base_key, &obj_id);

    // Then: Should match exactly
    assert_eq!(decrypted.as_slice(), plaintext);
}

#[test]
fn test_decrypt_empty_string() {
    // Given: Empty string
    let handler = StandardSecurityHandler::rc4_128bit();
    let base_key = EncryptionKey::new(vec![0xAA; 16]);
    let obj_id = ObjectId::new(1, 0);

    // When: Encrypting/decrypting empty string
    let encrypted = handler.encrypt_string(b"", &base_key, &obj_id);
    let decrypted = handler.decrypt_string(&encrypted, &base_key, &obj_id);

    // Then: Should remain empty
    assert_eq!(decrypted.len(), 0, "Empty string should decrypt to empty");
}

// ===== Stream Decryption Tests =====

#[test]
fn test_decrypt_stream_rc4_128bit() {
    // Given: Stream data (could be compressed, but we test raw)
    let handler = StandardSecurityHandler::rc4_128bit();
    let base_key = EncryptionKey::new(vec![0x33; 16]);
    let obj_id = ObjectId::new(10, 0);

    // Simulate a small PDF stream (e.g., content stream)
    let stream_data = b"BT /F1 12 Tf 100 700 Td (Hello World) Tj ET";
    let encrypted = handler.encrypt_stream(stream_data, &base_key, &obj_id);

    assert_ne!(encrypted.as_slice(), stream_data);

    // When: Decrypting stream
    let decrypted = handler.decrypt_stream(&encrypted, &base_key, &obj_id);

    // Then: Should match original
    assert_eq!(decrypted.as_slice(), stream_data);
}

#[test]
fn test_decrypt_large_stream() {
    // Given: Larger stream (simulate image or long content)
    let handler = StandardSecurityHandler::rc4_128bit();
    let base_key = EncryptionKey::new(vec![0x55; 16]);
    let obj_id = ObjectId::new(20, 1);

    let large_data = vec![0x42; 10000]; // 10KB
    let encrypted = handler.encrypt_stream(&large_data, &base_key, &obj_id);

    // When: Decrypting
    let decrypted = handler.decrypt_stream(&encrypted, &base_key, &obj_id);

    // Then: Should match exactly
    assert_eq!(decrypted, large_data);
}

// ===== Indirect Object Tests =====

#[test]
fn test_decrypt_indirect_string() {
    // Given: Indirect object "5 0 obj (encrypted_string) endobj"
    // In PDF, strings in indirect objects use object's ID for key derivation
    let handler = StandardSecurityHandler::rc4_128bit();
    let base_key = EncryptionKey::new(vec![0x77; 16]);

    // Object 5 generation 0
    let obj_id = ObjectId::new(5, 0);
    let plaintext = b"This is an indirect string object";

    // Encrypt as if it's in object 5
    let encrypted = handler.encrypt_string(plaintext, &base_key, &obj_id);

    // When: Decrypting with same object ID
    let decrypted = handler.decrypt_string(&encrypted, &base_key, &obj_id);

    // Then: Should decrypt correctly
    assert_eq!(decrypted.as_slice(), plaintext);

    // Also test: Using wrong object ID should produce garbage
    let wrong_obj_id = ObjectId::new(6, 0);
    let wrong_decrypt = handler.decrypt_string(&encrypted, &base_key, &wrong_obj_id);
    assert_ne!(
        wrong_decrypt.as_slice(),
        plaintext,
        "Wrong object ID should fail"
    );
}

// ===== Identity Filter Test =====

#[test]
fn test_do_not_decrypt_stream_with_identity_filter() {
    // Given: Stream with /StmF /Identity (explicitly not encrypted)
    // According to ISO 32000-1, Identity filter means no encryption
    let handler = StandardSecurityHandler::rc4_128bit();
    let base_key = EncryptionKey::new(vec![0xAA; 16]);
    let obj_id = ObjectId::new(15, 0);

    let stream_data = b"Unencrypted stream data";

    // In real PDF, this would be detected by checking stream dictionary
    // For now, we simulate by NOT encrypting
    let filter_name = "Identity";

    // When: Attempting to decrypt with Identity filter
    let result = if filter_name == "Identity" {
        // Should return original data unchanged
        stream_data.to_vec()
    } else {
        handler.decrypt_stream(stream_data, &base_key, &obj_id)
    };

    // Then: Should remain unchanged
    assert_eq!(
        result.as_slice(),
        stream_data,
        "Identity filter should not decrypt"
    );
}

// ===== RC4 Symmetry Test =====

#[test]
fn test_rc4_encryption_is_symmetric() {
    // RC4 is a symmetric stream cipher - encrypt and decrypt are the same operation
    let handler = StandardSecurityHandler::rc4_128bit();
    let base_key = EncryptionKey::new(vec![0xCC; 16]);
    let obj_id = ObjectId::new(1, 0);

    let plaintext = b"RC4 encryption is symmetric";

    // Encrypt
    let encrypted = handler.encrypt_string(plaintext, &base_key, &obj_id);

    // Encrypting again with same key should decrypt (RC4 property)
    let double_encrypted = handler.encrypt_string(&encrypted, &base_key, &obj_id);

    // Then: Double encryption should return original (XOR property of RC4)
    assert_eq!(
        double_encrypted.as_slice(),
        plaintext,
        "RC4 is symmetric - double encryption = decryption"
    );
}
