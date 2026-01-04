//! AES encryption implementation for PDF
//!
//! This module provides AES-128 and AES-256 encryption support according to
//! ISO 32000-1 Section 7.6 (PDF 1.6+ and PDF 2.0).
//!
//! Implementation uses production-grade RustCrypto crates (aes, cbc).

use aes::{Aes128, Aes256};
use cbc::{Decryptor, Encryptor};
use cipher::block_padding::Pkcs7;
use cipher::{BlockDecryptMut, BlockEncryptMut, KeyIvInit};

/// AES key sizes supported by PDF
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AesKeySize {
    /// AES-128 (16 bytes)
    Aes128,
    /// AES-256 (32 bytes)
    Aes256,
}

impl AesKeySize {
    /// Get key size in bytes
    pub fn key_length(&self) -> usize {
        match self {
            AesKeySize::Aes128 => 16,
            AesKeySize::Aes256 => 32,
        }
    }

    /// Get block size (always 16 bytes for AES)
    pub fn block_size(&self) -> usize {
        16
    }
}

/// AES encryption key
#[derive(Debug, Clone)]
pub struct AesKey {
    /// Key bytes
    key: Vec<u8>,
    /// Key size
    size: AesKeySize,
}

impl AesKey {
    /// Create new AES-128 key
    pub fn new_128(key: Vec<u8>) -> Result<Self, AesError> {
        if key.len() != 16 {
            return Err(AesError::InvalidKeyLength {
                expected: 16,
                actual: key.len(),
            });
        }

        Ok(Self {
            key,
            size: AesKeySize::Aes128,
        })
    }

    /// Create new AES-256 key
    pub fn new_256(key: Vec<u8>) -> Result<Self, AesError> {
        if key.len() != 32 {
            return Err(AesError::InvalidKeyLength {
                expected: 32,
                actual: key.len(),
            });
        }

        Ok(Self {
            key,
            size: AesKeySize::Aes256,
        })
    }

    /// Get key bytes
    pub fn key(&self) -> &[u8] {
        &self.key
    }

    /// Get key size
    pub fn size(&self) -> AesKeySize {
        self.size
    }

    /// Get key length in bytes
    pub fn len(&self) -> usize {
        self.key.len()
    }

    /// Check if key is empty (should never happen)
    pub fn is_empty(&self) -> bool {
        self.key.is_empty()
    }
}

/// AES-related errors
#[derive(Debug, Clone, PartialEq)]
pub enum AesError {
    /// Invalid key length
    InvalidKeyLength { expected: usize, actual: usize },
    /// Invalid IV length (must be 16 bytes)
    InvalidIvLength { expected: usize, actual: usize },
    /// Encryption failed
    EncryptionFailed(String),
    /// Decryption failed
    DecryptionFailed(String),
    /// PKCS#7 padding error
    PaddingError(String),
}

impl std::fmt::Display for AesError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AesError::InvalidKeyLength { expected, actual } => {
                write!(f, "Invalid key length: expected {expected}, got {actual}")
            }
            AesError::InvalidIvLength { expected, actual } => {
                write!(f, "Invalid IV length: expected {expected}, got {actual}")
            }
            AesError::EncryptionFailed(msg) => write!(f, "Encryption failed: {msg}"),
            AesError::DecryptionFailed(msg) => write!(f, "Decryption failed: {msg}"),
            AesError::PaddingError(msg) => write!(f, "Padding error: {msg}"),
        }
    }
}

impl std::error::Error for AesError {}

/// AES cipher implementation using RustCrypto (production-grade)
///
/// This implementation uses the audited `aes` and `cbc` crates from the
/// RustCrypto project for secure AES-128 and AES-256 encryption.
pub struct Aes {
    key: AesKey,
}

impl Aes {
    /// Create new AES cipher
    pub fn new(key: AesKey) -> Self {
        Self { key }
    }

    /// Encrypt data using AES-CBC mode with PKCS#7 padding
    ///
    /// Uses RustCrypto's `cbc` crate for production-grade encryption.
    pub fn encrypt_cbc(&self, data: &[u8], iv: &[u8]) -> Result<Vec<u8>, AesError> {
        if iv.len() != 16 {
            return Err(AesError::InvalidIvLength {
                expected: 16,
                actual: iv.len(),
            });
        }

        let iv_array: [u8; 16] = iv.try_into().map_err(|_| AesError::InvalidIvLength {
            expected: 16,
            actual: iv.len(),
        })?;

        match self.key.size() {
            AesKeySize::Aes128 => {
                let key_array: [u8; 16] =
                    self.key
                        .key()
                        .try_into()
                        .map_err(|_| AesError::InvalidKeyLength {
                            expected: 16,
                            actual: self.key.len(),
                        })?;
                let encryptor = Encryptor::<Aes128>::new(&key_array.into(), &iv_array.into());
                // Buffer size: plaintext + padding (up to 16 bytes)
                let mut buffer = vec![0u8; data.len() + 16];
                buffer[..data.len()].copy_from_slice(data);
                let ciphertext = encryptor
                    .encrypt_padded_mut::<Pkcs7>(&mut buffer, data.len())
                    .map_err(|e| AesError::EncryptionFailed(format!("Padding error: {e}")))?;
                Ok(ciphertext.to_vec())
            }
            AesKeySize::Aes256 => {
                let key_array: [u8; 32] =
                    self.key
                        .key()
                        .try_into()
                        .map_err(|_| AesError::InvalidKeyLength {
                            expected: 32,
                            actual: self.key.len(),
                        })?;
                let encryptor = Encryptor::<Aes256>::new(&key_array.into(), &iv_array.into());
                let mut buffer = vec![0u8; data.len() + 16];
                buffer[..data.len()].copy_from_slice(data);
                let ciphertext = encryptor
                    .encrypt_padded_mut::<Pkcs7>(&mut buffer, data.len())
                    .map_err(|e| AesError::EncryptionFailed(format!("Padding error: {e}")))?;
                Ok(ciphertext.to_vec())
            }
        }
    }

    /// Decrypt data using AES-CBC mode with PKCS#7 padding removal
    ///
    /// Uses RustCrypto's `cbc` crate for production-grade decryption.
    pub fn decrypt_cbc(&self, data: &[u8], iv: &[u8]) -> Result<Vec<u8>, AesError> {
        if iv.len() != 16 {
            return Err(AesError::InvalidIvLength {
                expected: 16,
                actual: iv.len(),
            });
        }

        if data.len() % 16 != 0 {
            return Err(AesError::DecryptionFailed(
                "Data length must be multiple of 16 bytes".to_string(),
            ));
        }

        let iv_array: [u8; 16] = iv.try_into().map_err(|_| AesError::InvalidIvLength {
            expected: 16,
            actual: iv.len(),
        })?;

        match self.key.size() {
            AesKeySize::Aes128 => {
                let key_array: [u8; 16] =
                    self.key
                        .key()
                        .try_into()
                        .map_err(|_| AesError::InvalidKeyLength {
                            expected: 16,
                            actual: self.key.len(),
                        })?;
                let decryptor = Decryptor::<Aes128>::new(&key_array.into(), &iv_array.into());
                let mut buffer = data.to_vec();
                let plaintext = decryptor
                    .decrypt_padded_mut::<Pkcs7>(&mut buffer)
                    .map_err(|e| AesError::PaddingError(format!("Unpadding error: {e}")))?;
                Ok(plaintext.to_vec())
            }
            AesKeySize::Aes256 => {
                let key_array: [u8; 32] =
                    self.key
                        .key()
                        .try_into()
                        .map_err(|_| AesError::InvalidKeyLength {
                            expected: 32,
                            actual: self.key.len(),
                        })?;
                let decryptor = Decryptor::<Aes256>::new(&key_array.into(), &iv_array.into());
                let mut buffer = data.to_vec();
                let plaintext = decryptor
                    .decrypt_padded_mut::<Pkcs7>(&mut buffer)
                    .map_err(|e| AesError::PaddingError(format!("Unpadding error: {e}")))?;
                Ok(plaintext.to_vec())
            }
        }
    }

    /// Encrypt data using AES-ECB mode (for Perms entry in R6)
    ///
    /// Note: ECB mode is generally insecure but required by PDF spec for Perms entry.
    pub fn encrypt_ecb(&self, data: &[u8]) -> Result<Vec<u8>, AesError> {
        use aes::cipher::{BlockEncrypt, KeyInit};

        if data.len() % 16 != 0 {
            return Err(AesError::EncryptionFailed(
                "Data length must be multiple of 16 bytes for ECB mode".to_string(),
            ));
        }

        let mut encrypted = Vec::with_capacity(data.len());

        match self.key.size() {
            AesKeySize::Aes128 => {
                let key_array: [u8; 16] =
                    self.key
                        .key()
                        .try_into()
                        .map_err(|_| AesError::InvalidKeyLength {
                            expected: 16,
                            actual: self.key.len(),
                        })?;
                let cipher = Aes128::new(&key_array.into());

                for chunk in data.chunks(16) {
                    let mut block =
                        aes::cipher::generic_array::GenericArray::clone_from_slice(chunk);
                    cipher.encrypt_block(&mut block);
                    encrypted.extend_from_slice(&block);
                }
            }
            AesKeySize::Aes256 => {
                let key_array: [u8; 32] =
                    self.key
                        .key()
                        .try_into()
                        .map_err(|_| AesError::InvalidKeyLength {
                            expected: 32,
                            actual: self.key.len(),
                        })?;
                let cipher = Aes256::new(&key_array.into());

                for chunk in data.chunks(16) {
                    let mut block =
                        aes::cipher::generic_array::GenericArray::clone_from_slice(chunk);
                    cipher.encrypt_block(&mut block);
                    encrypted.extend_from_slice(&block);
                }
            }
        }

        Ok(encrypted)
    }

    /// Decrypt data using AES-ECB mode (for Perms entry verification in R6)
    #[allow(dead_code)] // Will be used in R6 implementation
    pub fn decrypt_ecb(&self, data: &[u8]) -> Result<Vec<u8>, AesError> {
        use aes::cipher::{BlockDecrypt, KeyInit};

        if data.len() % 16 != 0 {
            return Err(AesError::DecryptionFailed(
                "Data length must be multiple of 16 bytes for ECB mode".to_string(),
            ));
        }

        let mut decrypted = Vec::with_capacity(data.len());

        match self.key.size() {
            AesKeySize::Aes128 => {
                let key_array: [u8; 16] =
                    self.key
                        .key()
                        .try_into()
                        .map_err(|_| AesError::InvalidKeyLength {
                            expected: 16,
                            actual: self.key.len(),
                        })?;
                let cipher = Aes128::new(&key_array.into());

                for chunk in data.chunks(16) {
                    let mut block =
                        aes::cipher::generic_array::GenericArray::clone_from_slice(chunk);
                    cipher.decrypt_block(&mut block);
                    decrypted.extend_from_slice(&block);
                }
            }
            AesKeySize::Aes256 => {
                let key_array: [u8; 32] =
                    self.key
                        .key()
                        .try_into()
                        .map_err(|_| AesError::InvalidKeyLength {
                            expected: 32,
                            actual: self.key.len(),
                        })?;
                let cipher = Aes256::new(&key_array.into());

                for chunk in data.chunks(16) {
                    let mut block =
                        aes::cipher::generic_array::GenericArray::clone_from_slice(chunk);
                    cipher.decrypt_block(&mut block);
                    decrypted.extend_from_slice(&block);
                }
            }
        }

        Ok(decrypted)
    }

    /// Encrypt data using AES-CBC mode WITHOUT padding
    ///
    /// Used for R5/R6 UE entry encryption where data is already block-aligned (32 bytes).
    /// Unlike encrypt_cbc, this does not add PKCS#7 padding.
    ///
    /// # Requirements
    /// - Data length must be a multiple of 16 bytes
    /// - IV must be exactly 16 bytes
    pub fn encrypt_cbc_raw(&self, data: &[u8], iv: &[u8]) -> Result<Vec<u8>, AesError> {
        use aes::cipher::{BlockEncrypt, KeyInit};

        if iv.len() != 16 {
            return Err(AesError::InvalidIvLength {
                expected: 16,
                actual: iv.len(),
            });
        }

        if data.len() % 16 != 0 {
            return Err(AesError::EncryptionFailed(
                "Data length must be multiple of 16 bytes for raw CBC mode".to_string(),
            ));
        }

        let mut encrypted = Vec::with_capacity(data.len());
        let mut prev_block: [u8; 16] = iv.try_into().map_err(|_| AesError::InvalidIvLength {
            expected: 16,
            actual: iv.len(),
        })?;

        match self.key.size() {
            AesKeySize::Aes128 => {
                let key_array: [u8; 16] =
                    self.key
                        .key()
                        .try_into()
                        .map_err(|_| AesError::InvalidKeyLength {
                            expected: 16,
                            actual: self.key.len(),
                        })?;
                let cipher = Aes128::new(&key_array.into());

                for chunk in data.chunks(16) {
                    // XOR with previous ciphertext (or IV for first block)
                    let mut block: [u8; 16] = [0u8; 16];
                    for i in 0..16 {
                        block[i] = chunk[i] ^ prev_block[i];
                    }
                    let mut block_ga =
                        aes::cipher::generic_array::GenericArray::clone_from_slice(&block);
                    cipher.encrypt_block(&mut block_ga);
                    prev_block.copy_from_slice(&block_ga);
                    encrypted.extend_from_slice(&block_ga);
                }
            }
            AesKeySize::Aes256 => {
                let key_array: [u8; 32] =
                    self.key
                        .key()
                        .try_into()
                        .map_err(|_| AesError::InvalidKeyLength {
                            expected: 32,
                            actual: self.key.len(),
                        })?;
                let cipher = Aes256::new(&key_array.into());

                for chunk in data.chunks(16) {
                    // XOR with previous ciphertext (or IV for first block)
                    let mut block: [u8; 16] = [0u8; 16];
                    for i in 0..16 {
                        block[i] = chunk[i] ^ prev_block[i];
                    }
                    let mut block_ga =
                        aes::cipher::generic_array::GenericArray::clone_from_slice(&block);
                    cipher.encrypt_block(&mut block_ga);
                    prev_block.copy_from_slice(&block_ga);
                    encrypted.extend_from_slice(&block_ga);
                }
            }
        }

        Ok(encrypted)
    }

    /// Decrypt data using AES-CBC mode WITHOUT padding
    ///
    /// Used for R5/R6 UE entry decryption where data is already block-aligned (32 bytes).
    /// Unlike decrypt_cbc, this does not expect or remove PKCS#7 padding.
    ///
    /// # Requirements
    /// - Data length must be a multiple of 16 bytes
    /// - IV must be exactly 16 bytes
    pub fn decrypt_cbc_raw(&self, data: &[u8], iv: &[u8]) -> Result<Vec<u8>, AesError> {
        use aes::cipher::{BlockDecrypt, KeyInit};

        if iv.len() != 16 {
            return Err(AesError::InvalidIvLength {
                expected: 16,
                actual: iv.len(),
            });
        }

        if data.len() % 16 != 0 {
            return Err(AesError::DecryptionFailed(
                "Data length must be multiple of 16 bytes for raw CBC mode".to_string(),
            ));
        }

        let mut decrypted = Vec::with_capacity(data.len());
        let mut prev_block: [u8; 16] = iv.try_into().map_err(|_| AesError::InvalidIvLength {
            expected: 16,
            actual: iv.len(),
        })?;

        match self.key.size() {
            AesKeySize::Aes128 => {
                let key_array: [u8; 16] =
                    self.key
                        .key()
                        .try_into()
                        .map_err(|_| AesError::InvalidKeyLength {
                            expected: 16,
                            actual: self.key.len(),
                        })?;
                let cipher = Aes128::new(&key_array.into());

                for chunk in data.chunks(16) {
                    let mut block =
                        aes::cipher::generic_array::GenericArray::clone_from_slice(chunk);
                    cipher.decrypt_block(&mut block);
                    // XOR with previous ciphertext (or IV for first block)
                    for i in 0..16 {
                        block[i] ^= prev_block[i];
                    }
                    prev_block.copy_from_slice(chunk);
                    decrypted.extend_from_slice(&block);
                }
            }
            AesKeySize::Aes256 => {
                let key_array: [u8; 32] =
                    self.key
                        .key()
                        .try_into()
                        .map_err(|_| AesError::InvalidKeyLength {
                            expected: 32,
                            actual: self.key.len(),
                        })?;
                let cipher = Aes256::new(&key_array.into());

                for chunk in data.chunks(16) {
                    let mut block =
                        aes::cipher::generic_array::GenericArray::clone_from_slice(chunk);
                    cipher.decrypt_block(&mut block);
                    // XOR with previous ciphertext (or IV for first block)
                    for i in 0..16 {
                        block[i] ^= prev_block[i];
                    }
                    prev_block.copy_from_slice(chunk);
                    decrypted.extend_from_slice(&block);
                }
            }
        }

        Ok(decrypted)
    }
}

/// Generate random IV for AES encryption
pub fn generate_iv() -> Vec<u8> {
    // In production, use a cryptographically secure random number generator
    // For now, use a simple approach with multiple entropy sources
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::thread;
    use std::time::SystemTime;

    static COUNTER: AtomicUsize = AtomicUsize::new(0);

    let mut hasher = DefaultHasher::new();

    // Hash multiple entropy sources to ensure uniqueness
    SystemTime::now().hash(&mut hasher);
    thread::current().id().hash(&mut hasher);
    std::process::id().hash(&mut hasher);
    COUNTER.fetch_add(1, Ordering::SeqCst).hash(&mut hasher);

    let seed = hasher.finish();
    let mut iv = Vec::new();

    for i in 0..16 {
        iv.push(((seed >> (i * 4)) as u8) ^ (i as u8));
    }

    iv
}

#[cfg(test)]
mod tests {
    use super::*;

    // ===== AesKey Tests =====

    #[test]
    fn test_aes_key_creation() {
        // Test AES-128 key
        let key_128 = vec![0u8; 16];
        let aes_key = AesKey::new_128(key_128.clone()).unwrap();
        assert_eq!(aes_key.key(), &key_128);
        assert_eq!(aes_key.size(), AesKeySize::Aes128);
        assert_eq!(aes_key.len(), 16);

        // Test AES-256 key
        let key_256 = vec![1u8; 32];
        let aes_key = AesKey::new_256(key_256.clone()).unwrap();
        assert_eq!(aes_key.key(), &key_256);
        assert_eq!(aes_key.size(), AesKeySize::Aes256);
        assert_eq!(aes_key.len(), 32);
    }

    #[test]
    fn test_aes_key_invalid_length() {
        // Test invalid AES-128 key length
        let key_short = vec![0u8; 15];
        assert!(AesKey::new_128(key_short).is_err());

        let key_long = vec![0u8; 17];
        assert!(AesKey::new_128(key_long).is_err());

        // Test invalid AES-256 key length
        let key_short = vec![0u8; 31];
        assert!(AesKey::new_256(key_short).is_err());

        let key_long = vec![0u8; 33];
        assert!(AesKey::new_256(key_long).is_err());
    }

    #[test]
    fn test_aes_key_size() {
        assert_eq!(AesKeySize::Aes128.key_length(), 16);
        assert_eq!(AesKeySize::Aes256.key_length(), 32);
        assert_eq!(AesKeySize::Aes128.block_size(), 16);
        assert_eq!(AesKeySize::Aes256.block_size(), 16);
    }

    #[test]
    fn test_aes_key_size_equality() {
        assert_eq!(AesKeySize::Aes128, AesKeySize::Aes128);
        assert_eq!(AesKeySize::Aes256, AesKeySize::Aes256);
        assert_ne!(AesKeySize::Aes128, AesKeySize::Aes256);
    }

    #[test]
    fn test_aes_key_size_debug() {
        assert_eq!(format!("{:?}", AesKeySize::Aes128), "Aes128");
        assert_eq!(format!("{:?}", AesKeySize::Aes256), "Aes256");
    }

    #[test]
    fn test_aes_key_is_empty() {
        let key = AesKey::new_128(vec![0u8; 16]).unwrap();
        assert!(!key.is_empty());
    }

    #[test]
    fn test_aes_key_debug() {
        let key = AesKey::new_128(vec![1u8; 16]).unwrap();
        let debug_str = format!("{key:?}");
        assert!(debug_str.contains("AesKey"));
        assert!(debug_str.contains("key:"));
        assert!(debug_str.contains("size:"));
    }

    #[test]
    fn test_aes_key_clone() {
        let key = AesKey::new_128(vec![1u8; 16]).unwrap();
        let cloned = key.clone();
        assert_eq!(key.key(), cloned.key());
        assert_eq!(key.size(), cloned.size());
    }

    #[test]
    fn test_aes_key_various_patterns() {
        let patterns = vec![
            vec![0xFF; 16],                     // All 1s
            vec![0x00; 16],                     // All 0s
            (0..16).map(|i| i as u8).collect(), // Sequential
            vec![0xA5; 16],                     // Alternating bits
        ];

        for pattern in patterns {
            let key = AesKey::new_128(pattern.clone()).unwrap();
            assert_eq!(key.key(), &pattern);
            assert_eq!(key.len(), 16);
        }
    }

    #[test]
    fn test_aes_key_256_various_patterns() {
        let patterns = vec![
            vec![0xFF; 32],
            vec![0x00; 32],
            (0..32).map(|i| i as u8).collect(),
            vec![0x5A; 32],
        ];

        for pattern in patterns {
            let key = AesKey::new_256(pattern.clone()).unwrap();
            assert_eq!(key.key(), &pattern);
            assert_eq!(key.len(), 32);
        }
    }

    // ===== AesError Tests =====

    #[test]
    fn test_aes_error_display() {
        let error1 = AesError::InvalidKeyLength {
            expected: 16,
            actual: 15,
        };
        assert!(error1.to_string().contains("Invalid key length"));

        let error2 = AesError::EncryptionFailed("test".to_string());
        assert!(error2.to_string().contains("Encryption failed"));

        let error3 = AesError::PaddingError("bad padding".to_string());
        assert!(error3.to_string().contains("Padding error"));
    }

    #[test]
    fn test_aes_error_equality() {
        let err1 = AesError::InvalidKeyLength {
            expected: 16,
            actual: 15,
        };
        let err2 = AesError::InvalidKeyLength {
            expected: 16,
            actual: 15,
        };
        let err3 = AesError::InvalidKeyLength {
            expected: 16,
            actual: 17,
        };

        assert_eq!(err1, err2);
        assert_ne!(err1, err3);
    }

    #[test]
    fn test_aes_error_clone() {
        let errors = vec![
            AesError::InvalidKeyLength {
                expected: 16,
                actual: 15,
            },
            AesError::InvalidIvLength {
                expected: 16,
                actual: 15,
            },
            AesError::EncryptionFailed("test".to_string()),
            AesError::DecryptionFailed("test".to_string()),
            AesError::PaddingError("test".to_string()),
        ];

        for error in errors {
            let cloned = error.clone();
            assert_eq!(error, cloned);
        }
    }

    #[test]
    fn test_aes_error_debug() {
        let error = AesError::InvalidKeyLength {
            expected: 16,
            actual: 15,
        };
        let debug_str = format!("{error:?}");
        assert!(debug_str.contains("InvalidKeyLength"));
        assert!(debug_str.contains("expected: 16"));
        assert!(debug_str.contains("actual: 15"));
    }

    #[test]
    fn test_aes_error_display_all_variants() {
        let errors = vec![
            (
                AesError::InvalidKeyLength {
                    expected: 16,
                    actual: 15,
                },
                "Invalid key length",
            ),
            (
                AesError::InvalidIvLength {
                    expected: 16,
                    actual: 15,
                },
                "Invalid IV length",
            ),
            (
                AesError::EncryptionFailed("custom error".to_string()),
                "Encryption failed: custom error",
            ),
            (
                AesError::DecryptionFailed("custom error".to_string()),
                "Decryption failed: custom error",
            ),
            (
                AesError::PaddingError("custom error".to_string()),
                "Padding error: custom error",
            ),
        ];

        for (error, expected_substring) in errors {
            let display = error.to_string();
            assert!(display.contains(expected_substring));
        }
    }

    #[test]
    fn test_aes_error_is_std_error() {
        let error: Box<dyn std::error::Error> =
            Box::new(AesError::PaddingError("test".to_string()));
        assert_eq!(error.to_string(), "Padding error: test");
    }

    // ===== AES Encryption/Decryption Tests (Using RustCrypto) =====

    #[test]
    fn test_aes_128_encrypt_decrypt_roundtrip() {
        // Now with RustCrypto, roundtrip should work perfectly
        let key = AesKey::new_128(vec![
            0x2b, 0x7e, 0x15, 0x16, 0x28, 0xae, 0xd2, 0xa6, 0xab, 0xf7, 0x15, 0x88, 0x09, 0xcf,
            0x4f, 0x3c,
        ])
        .unwrap();
        let aes = Aes::new(key);

        let data = b"Hello, AES World!";
        let iv = vec![
            0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d,
            0x0e, 0x0f,
        ];

        let encrypted = aes.encrypt_cbc(data, &iv).unwrap();
        assert_ne!(encrypted, data);
        assert!(encrypted.len() >= data.len());
        assert_eq!(encrypted.len() % 16, 0);

        // With RustCrypto, decryption should produce exact original
        let decrypted = aes.decrypt_cbc(&encrypted, &iv).unwrap();
        assert_eq!(decrypted.as_slice(), data.as_slice());
    }

    #[test]
    fn test_aes_256_encrypt_decrypt_roundtrip() {
        let key = AesKey::new_256(vec![0x42; 32]).unwrap();
        let aes = Aes::new(key);

        let data = b"This is a test for AES-256 encryption!";
        let iv = vec![0x33; 16];

        let encrypted = aes.encrypt_cbc(data, &iv).unwrap();
        assert_ne!(encrypted.as_slice(), data.as_slice());

        let decrypted = aes.decrypt_cbc(&encrypted, &iv).unwrap();
        assert_eq!(decrypted.as_slice(), data.as_slice());
    }

    #[test]
    fn test_aes_empty_data() {
        let key = AesKey::new_128(vec![0u8; 16]).unwrap();
        let aes = Aes::new(key);
        let iv = vec![0u8; 16];

        let data = b"";
        let encrypted = aes.encrypt_cbc(data, &iv).unwrap();
        assert_eq!(encrypted.len(), 16); // One block due to PKCS#7 padding

        let decrypted = aes.decrypt_cbc(&encrypted, &iv).unwrap();
        assert_eq!(decrypted.as_slice(), data.as_slice());
    }

    #[test]
    fn test_aes_invalid_iv() {
        let key = AesKey::new_128(vec![0u8; 16]).unwrap();
        let aes = Aes::new(key);

        let data = b"test data";
        let iv_short = vec![0u8; 15];
        let iv_long = vec![0u8; 17];

        assert!(aes.encrypt_cbc(data, &iv_short).is_err());
        assert!(aes.encrypt_cbc(data, &iv_long).is_err());

        let encrypted = aes.encrypt_cbc(data, &[0u8; 16]).unwrap();
        assert!(aes.decrypt_cbc(&encrypted, &iv_short).is_err());
        assert!(aes.decrypt_cbc(&encrypted, &iv_long).is_err());
    }

    #[test]
    fn test_aes_multiple_blocks() {
        let key = AesKey::new_128(vec![0x42; 16]).unwrap();
        let aes = Aes::new(key);
        let iv = vec![0x37; 16];

        // Test data that spans multiple blocks
        let data = vec![0x55; 48]; // 3 blocks exactly
        let encrypted = aes.encrypt_cbc(&data, &iv).unwrap();
        assert_eq!(encrypted.len(), 64); // PKCS#7 adds padding even for exact blocks

        let decrypted = aes.decrypt_cbc(&encrypted, &iv).unwrap();
        assert_eq!(decrypted, data);
    }

    #[test]
    fn test_aes_large_data() {
        let key = AesKey::new_128(vec![0x11; 16]).unwrap();
        let aes = Aes::new(key);
        let iv = vec![0x22; 16];

        // Test with larger data (1KB)
        let data = vec![0x33; 1024];
        let encrypted = aes.encrypt_cbc(&data, &iv).unwrap();
        assert!(encrypted.len() >= 1024);
        assert_eq!(encrypted.len() % 16, 0);

        let decrypted = aes.decrypt_cbc(&encrypted, &iv).unwrap();
        assert_eq!(decrypted, data);
    }

    #[test]
    fn test_aes_various_data_sizes() {
        let key = AesKey::new_128(vec![0xAA; 16]).unwrap();
        let aes = Aes::new(key);
        let iv = vec![0xBB; 16];

        for size in [1, 15, 16, 17, 31, 32, 33, 63, 64, 65, 127, 128, 129] {
            let data = vec![0xCC; size];
            let encrypted = aes.encrypt_cbc(&data, &iv).unwrap();

            // PKCS#7 always adds padding
            let expected_size = if size % 16 == 0 {
                size + 16
            } else {
                (size / 16 + 1) * 16
            };
            assert_eq!(encrypted.len(), expected_size, "size={size}");

            let decrypted = aes.decrypt_cbc(&encrypted, &iv).unwrap();
            assert_eq!(decrypted, data, "size={size}");
        }
    }

    #[test]
    fn test_decrypt_invalid_data_length() {
        let key = AesKey::new_128(vec![0u8; 16]).unwrap();
        let aes = Aes::new(key);
        let iv = vec![0u8; 16];

        // Data not multiple of block size
        let invalid_data = vec![0u8; 17];
        let result = aes.decrypt_cbc(&invalid_data, &iv);
        assert!(result.is_err());
        match result.unwrap_err() {
            AesError::DecryptionFailed(msg) => {
                assert!(msg.contains("multiple of 16"));
            }
            _ => panic!("Expected DecryptionFailed error"),
        }
    }

    #[test]
    fn test_encrypt_with_different_ivs() {
        let key = AesKey::new_128(vec![0x42; 16]).unwrap();
        let aes = Aes::new(key);

        let data = b"Same data encrypted with different IVs";
        let iv1 = vec![0x00; 16];
        let iv2 = vec![0xFF; 16];

        let encrypted1 = aes.encrypt_cbc(data, &iv1).unwrap();
        let encrypted2 = aes.encrypt_cbc(data, &iv2).unwrap();

        // Same data with different IVs should produce different ciphertexts
        assert_ne!(encrypted1, encrypted2);
        assert_eq!(encrypted1.len(), encrypted2.len());

        // Both should decrypt correctly with their respective IVs
        let decrypted1 = aes.decrypt_cbc(&encrypted1, &iv1).unwrap();
        let decrypted2 = aes.decrypt_cbc(&encrypted2, &iv2).unwrap();
        assert_eq!(decrypted1.as_slice(), data.as_slice());
        assert_eq!(decrypted2.as_slice(), data.as_slice());
    }

    #[test]
    fn test_cbc_mode_no_patterns() {
        let key = AesKey::new_128(vec![0x11; 16]).unwrap();
        let aes = Aes::new(key);

        // Two identical plaintext blocks
        let data = vec![0x44; 32];
        let iv = vec![0x55; 16];

        let encrypted = aes.encrypt_cbc(&data, &iv).unwrap();

        // In CBC mode, the two encrypted blocks should be different
        // even though plaintext blocks are identical
        let block1 = &encrypted[0..16];
        let block2 = &encrypted[16..32];
        assert_ne!(block1, block2);
    }

    #[test]
    fn test_error_propagation() {
        let key = AesKey::new_128(vec![0u8; 16]).unwrap();
        let aes = Aes::new(key);

        // Test encryption with invalid IV
        let result = aes.encrypt_cbc(b"test", &[0u8; 15]);
        assert!(matches!(result, Err(AesError::InvalidIvLength { .. })));

        // Test decryption with invalid IV
        let valid_encrypted = vec![0u8; 16];
        let result = aes.decrypt_cbc(&valid_encrypted, &[0u8; 17]);
        assert!(matches!(result, Err(AesError::InvalidIvLength { .. })));
    }

    // ===== ECB Mode Tests =====

    #[test]
    fn test_ecb_encrypt_decrypt_roundtrip() {
        let key = AesKey::new_256(vec![0x55; 32]).unwrap();
        let aes = Aes::new(key);

        // ECB requires data multiple of 16
        let data = vec![0xAB; 32]; // Two blocks

        let encrypted = aes.encrypt_ecb(&data).unwrap();
        assert_eq!(encrypted.len(), 32);
        assert_ne!(encrypted, data);

        let decrypted = aes.decrypt_ecb(&encrypted).unwrap();
        assert_eq!(decrypted, data);
    }

    #[test]
    fn test_ecb_invalid_data_length() {
        let key = AesKey::new_128(vec![0u8; 16]).unwrap();
        let aes = Aes::new(key);

        // ECB requires data multiple of 16
        let invalid_data = vec![0u8; 17];
        assert!(aes.encrypt_ecb(&invalid_data).is_err());
        assert!(aes.decrypt_ecb(&invalid_data).is_err());
    }

    #[test]
    fn test_ecb_shows_patterns() {
        // ECB mode produces same ciphertext for same plaintext blocks
        let key = AesKey::new_128(vec![0x11; 16]).unwrap();
        let aes = Aes::new(key);

        let data = vec![0x44; 32]; // Two identical blocks

        let encrypted = aes.encrypt_ecb(&data).unwrap();

        // In ECB mode, both blocks should be identical (unlike CBC)
        let block1 = &encrypted[0..16];
        let block2 = &encrypted[16..32];
        assert_eq!(
            block1, block2,
            "ECB should produce same ciphertext for identical blocks"
        );
    }

    // ===== generate_iv Tests =====

    #[test]
    fn test_generate_iv() {
        let iv1 = generate_iv();
        let iv2 = generate_iv();

        assert_eq!(iv1.len(), 16);
        assert_eq!(iv2.len(), 16);
    }

    #[test]
    fn test_generate_iv_properties() {
        let ivs: Vec<Vec<u8>> = (0..10).map(|_| generate_iv()).collect();

        // All should be 16 bytes
        for iv in &ivs {
            assert_eq!(iv.len(), 16);
        }

        // Check that not all IVs are identical (with counter, they should differ)
        let first = &ivs[0];
        let all_same = ivs.iter().all(|iv| iv == first);
        assert!(
            !all_same || ivs.len() == 1,
            "IVs should not all be identical"
        );
    }

    // ===== NIST Test Vector =====

    #[test]
    fn test_aes_256_cbc_nist_vector() {
        // NIST test vector for AES-256-CBC from SP 800-38A
        let key = AesKey::new_256(vec![
            0x60, 0x3d, 0xeb, 0x10, 0x15, 0xca, 0x71, 0xbe, 0x2b, 0x73, 0xae, 0xf0, 0x85, 0x7d,
            0x77, 0x81, 0x1f, 0x35, 0x2c, 0x07, 0x3b, 0x61, 0x08, 0xd7, 0x2d, 0x98, 0x10, 0xa3,
            0x09, 0x14, 0xdf, 0xf4,
        ])
        .unwrap();
        let aes = Aes::new(key);

        let iv = vec![
            0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d,
            0x0e, 0x0f,
        ];

        let plaintext = vec![
            0x6b, 0xc1, 0xbe, 0xe2, 0x2e, 0x40, 0x9f, 0x96, 0xe9, 0x3d, 0x7e, 0x11, 0x73, 0x93,
            0x17, 0x2a,
        ];

        let expected_ciphertext = vec![
            0xf5, 0x8c, 0x4c, 0x04, 0xd6, 0xe5, 0xf1, 0xba, 0x77, 0x9e, 0xab, 0xfb, 0x5f, 0x7b,
            0xfb, 0xd6,
        ];

        let encrypted = aes.encrypt_cbc(&plaintext, &iv).unwrap();

        // First 16 bytes should match NIST expected ciphertext
        assert_eq!(
            &encrypted[..16],
            expected_ciphertext.as_slice(),
            "AES-256-CBC encryption should match NIST test vector"
        );
    }
}
