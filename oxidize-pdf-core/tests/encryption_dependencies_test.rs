/// Tests to verify RustCrypto dependencies are working correctly
///
/// These tests validate that the cryptographic primitives (SHA-256, AES-256-CBC)
/// are properly integrated and produce expected outputs according to NIST standards.
///
/// Phase 1.1 of TDD_PLAN_AES256_ENCRYPTION.md
use aes::Aes256;
use cbc::{Decryptor, Encryptor};
use cipher::block_padding::Pkcs7;
use cipher::{BlockDecryptMut, BlockEncryptMut, KeyIvInit};
use sha2::{Digest, Sha256, Sha512};

/// Helper function to encrypt with AES-256-CBC and PKCS7 padding
fn aes256_cbc_encrypt(key: &[u8; 32], iv: &[u8; 16], plaintext: &[u8]) -> Vec<u8> {
    let encryptor = Encryptor::<Aes256>::new(key.into(), iv.into());
    // Calculate buffer size: plaintext + padding (up to 16 bytes)
    let mut buffer = vec![0u8; plaintext.len() + 16];
    buffer[..plaintext.len()].copy_from_slice(plaintext);
    let ciphertext = encryptor
        .encrypt_padded_mut::<Pkcs7>(&mut buffer, plaintext.len())
        .expect("Encryption should not fail");
    ciphertext.to_vec()
}

/// Helper function to decrypt with AES-256-CBC and PKCS7 padding
fn aes256_cbc_decrypt(key: &[u8; 32], iv: &[u8; 16], ciphertext: &[u8]) -> Vec<u8> {
    let decryptor = Decryptor::<Aes256>::new(key.into(), iv.into());
    let mut buffer = ciphertext.to_vec();
    let plaintext = decryptor
        .decrypt_padded_mut::<Pkcs7>(&mut buffer)
        .expect("Decryption should not fail");
    plaintext.to_vec()
}

/// Test that SHA-256 produces consistent and correct output
#[test]
fn test_sha256_consistency_with_rust_crypto() {
    let data = b"test data for sha256";
    let hash1 = Sha256::digest(data);
    let hash2 = Sha256::digest(data);

    // Consistency: same input produces same output
    assert_eq!(hash1, hash2);

    // Correct length: SHA-256 produces 32 bytes
    assert_eq!(hash1.len(), 32);

    // Known test vector: empty string
    let empty_hash = Sha256::digest(b"");
    let expected_empty = [
        0xe3, 0xb0, 0xc4, 0x42, 0x98, 0xfc, 0x1c, 0x14, 0x9a, 0xfb, 0xf4, 0xc8, 0x99, 0x6f, 0xb9,
        0x24, 0x27, 0xae, 0x41, 0xe4, 0x64, 0x9b, 0x93, 0x4c, 0xa4, 0x95, 0x99, 0x1b, 0x78, 0x52,
        0xb8, 0x55,
    ];
    assert_eq!(empty_hash.as_slice(), expected_empty.as_slice());
}

/// Test that SHA-512 produces correct output (needed for R6 encryption)
#[test]
fn test_sha512_consistency_with_rust_crypto() {
    let data = b"test data for sha512";
    let hash1 = Sha512::digest(data);
    let hash2 = Sha512::digest(data);

    // Consistency
    assert_eq!(hash1, hash2);

    // Correct length: SHA-512 produces 64 bytes
    assert_eq!(hash1.len(), 64);

    // Known test vector: "abc"
    let abc_hash = Sha512::digest(b"abc");
    let expected_abc = [
        0xdd, 0xaf, 0x35, 0xa1, 0x93, 0x61, 0x7a, 0xba, 0xcc, 0x41, 0x73, 0x49, 0xae, 0x20, 0x41,
        0x31, 0x12, 0xe6, 0xfa, 0x4e, 0x89, 0xa9, 0x7e, 0xa2, 0x0a, 0x9e, 0xee, 0xe6, 0x4b, 0x55,
        0xd3, 0x9a, 0x21, 0x92, 0x99, 0x2a, 0x27, 0x4f, 0xc1, 0xa8, 0x36, 0xba, 0x3c, 0x23, 0xa3,
        0xfe, 0xeb, 0xbd, 0x45, 0x4d, 0x44, 0x23, 0x64, 0x3c, 0xe8, 0x0e, 0x2a, 0x9a, 0xc9, 0x4f,
        0xa5, 0x4c, 0xa4, 0x9f,
    ];
    assert_eq!(abc_hash.as_slice(), expected_abc.as_slice());
}

/// Test AES-256-CBC encryption/decryption roundtrip
#[test]
fn test_aes_256_cbc_roundtrip() {
    // Test key (32 bytes for AES-256)
    let key = [0x42u8; 32];
    // Test IV (16 bytes for CBC)
    let iv = [0x00u8; 16];
    let plaintext = b"This is AES-256 test data with padding!";

    // Encrypt
    let ciphertext = aes256_cbc_encrypt(&key, &iv, plaintext);

    // Ciphertext should be different from plaintext
    assert_ne!(ciphertext.as_slice(), plaintext.as_slice());

    // Ciphertext should be padded to block size (16 bytes)
    assert_eq!(ciphertext.len() % 16, 0);

    // Decrypt
    let decrypted = aes256_cbc_decrypt(&key, &iv, &ciphertext);

    // Roundtrip: decrypted should match original plaintext
    assert_eq!(decrypted.as_slice(), plaintext.as_slice());
}

/// Test AES-256-CBC with NIST test vector
#[test]
fn test_aes_256_cbc_nist_vector() {
    // NIST test vector for AES-256-CBC
    // From NIST Special Publication 800-38A
    let key: [u8; 32] = [
        0x60, 0x3d, 0xeb, 0x10, 0x15, 0xca, 0x71, 0xbe, 0x2b, 0x73, 0xae, 0xf0, 0x85, 0x7d, 0x77,
        0x81, 0x1f, 0x35, 0x2c, 0x07, 0x3b, 0x61, 0x08, 0xd7, 0x2d, 0x98, 0x10, 0xa3, 0x09, 0x14,
        0xdf, 0xf4,
    ];
    let iv: [u8; 16] = [
        0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e,
        0x0f,
    ];
    let plaintext: [u8; 16] = [
        0x6b, 0xc1, 0xbe, 0xe2, 0x2e, 0x40, 0x9f, 0x96, 0xe9, 0x3d, 0x7e, 0x11, 0x73, 0x93, 0x17,
        0x2a,
    ];
    let expected_ciphertext: [u8; 16] = [
        0xf5, 0x8c, 0x4c, 0x04, 0xd6, 0xe5, 0xf1, 0xba, 0x77, 0x9e, 0xab, 0xfb, 0x5f, 0x7b, 0xfb,
        0xd6,
    ];

    // For this test, we encrypt and check first block
    let ciphertext = aes256_cbc_encrypt(&key, &iv, &plaintext);

    // First 16 bytes should match NIST expected ciphertext
    assert_eq!(
        &ciphertext[..16],
        expected_ciphertext.as_slice(),
        "AES-256-CBC encryption should match NIST test vector"
    );
}

/// Test AES-256-CBC with empty IV (used in PDF encryption for UE entry)
#[test]
fn test_aes_256_cbc_zero_iv() {
    let key = [0xAA; 32];
    let iv = [0x00; 16]; // Zero IV as used in PDF R5/R6 encryption
    let plaintext = b"encryption key placeholder data!"; // 32 bytes

    // Encrypt
    let ciphertext = aes256_cbc_encrypt(&key, &iv, plaintext);

    // Decrypt
    let decrypted = aes256_cbc_decrypt(&key, &iv, &ciphertext);

    assert_eq!(decrypted.as_slice(), plaintext.as_slice());
}

/// Test that different keys produce different ciphertext
#[test]
fn test_aes_256_different_keys() {
    let key1 = [0x11; 32];
    let key2 = [0x22; 32];
    let iv = [0x00; 16];
    let plaintext = b"same plaintext for both";

    let ciphertext1 = aes256_cbc_encrypt(&key1, &iv, plaintext);
    let ciphertext2 = aes256_cbc_encrypt(&key2, &iv, plaintext);

    // Different keys should produce different ciphertext
    assert_ne!(
        ciphertext1, ciphertext2,
        "Different keys must produce different ciphertext"
    );
}

/// Test that different IVs produce different ciphertext
#[test]
fn test_aes_256_different_ivs() {
    let key = [0x33; 32];
    let iv1 = [0x00; 16];
    let iv2 = [0xFF; 16];
    let plaintext = b"same plaintext for both";

    let ciphertext1 = aes256_cbc_encrypt(&key, &iv1, plaintext);
    let ciphertext2 = aes256_cbc_encrypt(&key, &iv2, plaintext);

    // Different IVs should produce different ciphertext
    assert_ne!(
        ciphertext1, ciphertext2,
        "Different IVs must produce different ciphertext"
    );
}

/// Test SHA-256 known vector for "abc" (NIST FIPS 180-4)
#[test]
fn test_sha256_nist_abc_vector() {
    let hash = Sha256::digest(b"abc");
    let expected = [
        0xba, 0x78, 0x16, 0xbf, 0x8f, 0x01, 0xcf, 0xea, 0x41, 0x41, 0x40, 0xde, 0x5d, 0xae, 0x22,
        0x23, 0xb0, 0x03, 0x61, 0xa3, 0x96, 0x17, 0x7a, 0x9c, 0xb4, 0x10, 0xff, 0x61, 0xf2, 0x00,
        0x15, 0xad,
    ];
    assert_eq!(
        hash.as_slice(),
        expected.as_slice(),
        "SHA-256('abc') should match NIST test vector"
    );
}
