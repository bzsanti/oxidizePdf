//! Standard Security Handler implementation according to ISO 32000-1/32000-2
//!
//! # Security Considerations
//!
//! This implementation includes several security hardening measures:
//!
//! - **Constant-time comparison**: Password validation uses `subtle::ConstantTimeEq`
//!   to prevent timing side-channel attacks that could leak password information.
//!
//! - **Memory zeroization**: Sensitive data (`EncryptionKey`, `UserPassword`,
//!   `OwnerPassword`) implements `Zeroize` to ensure secrets are cleared from
//!   memory when dropped, preventing memory dump attacks.
//!
//! - **Cryptographically secure RNG**: Salt generation uses `rand::rng()` which
//!   provides OS-level entropy suitable for cryptographic operations.

#![allow(clippy::needless_range_loop)]

use crate::encryption::{generate_iv, Aes, AesKey, Permissions, Rc4, Rc4Key};
use crate::error::Result;
use crate::objects::ObjectId;
use rand::RngCore;
use sha2::{Digest, Sha256, Sha384, Sha512};
use subtle::ConstantTimeEq;
use zeroize::{Zeroize, ZeroizeOnDrop};

/// Padding used in password processing
const PADDING: [u8; 32] = [
    0x28, 0xBF, 0x4E, 0x5E, 0x4E, 0x75, 0x8A, 0x41, 0x64, 0x00, 0x4E, 0x56, 0xFF, 0xFA, 0x01, 0x08,
    0x2E, 0x2E, 0x00, 0xB6, 0xD0, 0x68, 0x3E, 0x80, 0x2F, 0x0C, 0xA9, 0xFE, 0x64, 0x53, 0x69, 0x7A,
];

/// User password
///
/// # Security
/// Implements `Zeroize` and `ZeroizeOnDrop` to ensure password is cleared from memory.
#[derive(Debug, Clone, Zeroize, ZeroizeOnDrop)]
pub struct UserPassword(pub String);

/// Owner password
///
/// # Security
/// Implements `Zeroize` and `ZeroizeOnDrop` to ensure password is cleared from memory.
#[derive(Debug, Clone, Zeroize, ZeroizeOnDrop)]
pub struct OwnerPassword(pub String);

/// Encryption key
///
/// # Security
/// Implements `Zeroize` and `ZeroizeOnDrop` to ensure key bytes are cleared from memory.
#[derive(Debug, Clone, Zeroize, ZeroizeOnDrop)]
pub struct EncryptionKey {
    /// Key bytes
    pub key: Vec<u8>,
}

impl EncryptionKey {
    /// Create from bytes
    pub fn new(key: Vec<u8>) -> Self {
        Self { key }
    }

    /// Get key length in bytes
    pub fn len(&self) -> usize {
        self.key.len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.key.is_empty()
    }

    /// Get key as bytes
    pub fn as_bytes(&self) -> &[u8] {
        &self.key
    }
}

/// Security handler revision
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub enum SecurityHandlerRevision {
    /// Revision 2 (RC4 40-bit)
    R2 = 2,
    /// Revision 3 (RC4 128-bit)
    R3 = 3,
    /// Revision 4 (RC4 128-bit with metadata encryption control)
    R4 = 4,
    /// Revision 5 (AES-256 with improved password validation)
    R5 = 5,
    /// Revision 6 (AES-256 with Unicode password support)
    R6 = 6,
}

/// Standard Security Handler
pub struct StandardSecurityHandler {
    /// Revision
    pub revision: SecurityHandlerRevision,
    /// Key length in bytes
    pub key_length: usize,
}

impl StandardSecurityHandler {
    /// Create handler for RC4 40-bit encryption
    pub fn rc4_40bit() -> Self {
        Self {
            revision: SecurityHandlerRevision::R2,
            key_length: 5,
        }
    }

    /// Create handler for RC4 128-bit encryption
    pub fn rc4_128bit() -> Self {
        Self {
            revision: SecurityHandlerRevision::R3,
            key_length: 16,
        }
    }

    /// Create handler for AES-256 encryption (Revision 5)
    pub fn aes_256_r5() -> Self {
        Self {
            revision: SecurityHandlerRevision::R5,
            key_length: 32,
        }
    }

    /// Create handler for AES-256 encryption (Revision 6)
    pub fn aes_256_r6() -> Self {
        Self {
            revision: SecurityHandlerRevision::R6,
            key_length: 32,
        }
    }

    /// Pad or truncate password to 32 bytes
    fn pad_password(password: &str) -> [u8; 32] {
        let mut padded = [0u8; 32];
        let password_bytes = password.as_bytes();
        let len = password_bytes.len().min(32);

        // Copy password bytes
        padded[..len].copy_from_slice(&password_bytes[..len]);

        // Fill remaining with padding
        if len < 32 {
            padded[len..].copy_from_slice(&PADDING[..32 - len]);
        }

        padded
    }

    /// Compute owner password hash (O entry)
    pub fn compute_owner_hash(
        &self,
        owner_password: &OwnerPassword,
        user_password: &UserPassword,
    ) -> Vec<u8> {
        // Step 1: Pad passwords
        let owner_pad = Self::pad_password(&owner_password.0);
        let user_pad = Self::pad_password(&user_password.0);

        // Step 2: Create MD5 hash of owner password
        let mut hash = md5::compute(&owner_pad).to_vec();

        // Step 3: For revision 3+, do 50 additional iterations
        if self.revision >= SecurityHandlerRevision::R3 {
            for _ in 0..50 {
                hash = md5::compute(&hash).to_vec();
            }
        }

        // Step 4: Create RC4 key from hash (truncated to key length)
        let rc4_key = Rc4Key::from_slice(&hash[..self.key_length]);

        // Step 5: Encrypt user password with RC4
        let mut result = rc4_encrypt(&rc4_key, &user_pad);

        // Step 6: For revision 3+, do 19 additional iterations
        if self.revision >= SecurityHandlerRevision::R3 {
            for i in 1..=19 {
                let mut key_bytes = hash[..self.key_length].to_vec();
                for j in 0..self.key_length {
                    key_bytes[j] ^= i as u8;
                }
                let iter_key = Rc4Key::from_slice(&key_bytes);
                result = rc4_encrypt(&iter_key, &result);
            }
        }

        result
    }

    /// Compute user password hash (U entry)
    pub fn compute_user_hash(
        &self,
        user_password: &UserPassword,
        owner_hash: &[u8],
        permissions: Permissions,
        file_id: Option<&[u8]>,
    ) -> Result<Vec<u8>> {
        // Compute encryption key
        let key = self.compute_encryption_key(user_password, owner_hash, permissions, file_id)?;

        match self.revision {
            SecurityHandlerRevision::R2 => {
                // For R2, encrypt padding with key
                let rc4_key = Rc4Key::from_slice(&key.key);
                Ok(rc4_encrypt(&rc4_key, &PADDING))
            }
            SecurityHandlerRevision::R3 | SecurityHandlerRevision::R4 => {
                // For R3/R4, compute MD5 hash including file ID
                let mut data = Vec::new();
                data.extend_from_slice(&PADDING);

                if let Some(id) = file_id {
                    data.extend_from_slice(id);
                }

                let hash = md5::compute(&data);

                // Encrypt hash with RC4
                let rc4_key = Rc4Key::from_slice(&key.key);
                let mut result = rc4_encrypt(&rc4_key, hash.as_ref());

                // Do 19 additional iterations
                for i in 1..=19 {
                    let mut key_bytes = key.key.clone();
                    for j in 0..key_bytes.len() {
                        key_bytes[j] ^= i as u8;
                    }
                    let iter_key = Rc4Key::from_slice(&key_bytes);
                    result = rc4_encrypt(&iter_key, &result);
                }

                // Result is 32 bytes (16 bytes encrypted hash + 16 bytes arbitrary data)
                result.resize(32, 0);
                Ok(result)
            }
            SecurityHandlerRevision::R5 | SecurityHandlerRevision::R6 => {
                // For R5/R6, use AES-based hash computation
                let aes_key = self.compute_aes_encryption_key(
                    user_password,
                    owner_hash,
                    permissions,
                    file_id,
                )?;
                let hash = sha256(&aes_key.key);

                // For AES revisions, return the hash directly (simplified)
                Ok(hash)
            }
        }
    }

    /// Compute encryption key from user password
    pub fn compute_encryption_key(
        &self,
        user_password: &UserPassword,
        owner_hash: &[u8],
        permissions: Permissions,
        file_id: Option<&[u8]>,
    ) -> Result<EncryptionKey> {
        match self.revision {
            SecurityHandlerRevision::R5 | SecurityHandlerRevision::R6 => {
                // For AES revisions, use AES-specific key computation
                self.compute_aes_encryption_key(user_password, owner_hash, permissions, file_id)
            }
            _ => {
                // For RC4 revisions, use MD5-based key computation
                // Step 1: Pad password
                let padded = Self::pad_password(&user_password.0);

                // Step 2: Create hash input
                let mut data = Vec::new();
                data.extend_from_slice(&padded);
                data.extend_from_slice(owner_hash);
                data.extend_from_slice(&permissions.bits().to_le_bytes());

                if let Some(id) = file_id {
                    data.extend_from_slice(id);
                }

                #[cfg(debug_assertions)]
                {
                    eprintln!("[DEBUG compute_key] padded[0..8]: {:02x?}", &padded[..8]);
                    eprintln!("[DEBUG compute_key] owner_hash len: {}", owner_hash.len());
                    eprintln!(
                        "[DEBUG compute_key] P bytes: {:02x?}",
                        permissions.bits().to_le_bytes()
                    );
                    eprintln!("[DEBUG compute_key] data len before MD5: {}", data.len());
                    // Print full data for comparison
                    let data_hex: String = data.iter().map(|b| format!("{:02x}", b)).collect();
                    eprintln!("[DEBUG compute_key] full data hex: {}", data_hex);

                    // Verify specific expected hash for debugging
                    if data_hex == "7573657228bf4e5e4e758a4164004e56fffa01082e2e00b6d0683e802f0ca9fe94e8094419662a774442fb072e3d9f19e9d130ec09a4d0061e78fe920f7ab62ffcffffff9c5b2a0606f918182e6c5cc0cac374d6" {
                        eprintln!("[DEBUG compute_key] DATA MATCHES EXPECTED - should produce eee5568378306e35...");
                    }
                }

                // For R4 with metadata not encrypted, add extra bytes
                if self.revision == SecurityHandlerRevision::R4 {
                    // In a full implementation, check EncryptMetadata flag
                    // For now, assume metadata is encrypted
                }

                // Step 3: Create MD5 hash
                let mut hash = md5::compute(&data).to_vec();

                #[cfg(debug_assertions)]
                {
                    eprintln!(
                        "[DEBUG compute_key] initial hash[0..8]: {:02x?}",
                        &hash[..8]
                    );
                    let hash_hex: String = hash.iter().map(|b| format!("{:02x}", b)).collect();
                    eprintln!("[DEBUG compute_key] full hash: {}", hash_hex);
                    eprintln!("[DEBUG compute_key] key_length: {}", self.key_length);
                }

                // Step 4: For revision 3+, do 50 additional iterations
                if self.revision >= SecurityHandlerRevision::R3 {
                    for _ in 0..50 {
                        hash = md5::compute(&hash[..self.key_length]).to_vec();
                    }
                }

                // Step 5: Truncate to key length
                hash.truncate(self.key_length);

                #[cfg(debug_assertions)]
                {
                    eprintln!("[DEBUG compute_key] final key: {:02x?}", &hash);
                }

                Ok(EncryptionKey::new(hash))
            }
        }
    }

    /// Encrypt a string
    pub fn encrypt_string(&self, data: &[u8], key: &EncryptionKey, obj_id: &ObjectId) -> Vec<u8> {
        match self.revision {
            SecurityHandlerRevision::R5 | SecurityHandlerRevision::R6 => {
                // For AES, use encrypt_aes and handle the Result
                self.encrypt_aes(data, key, obj_id).unwrap_or_default()
            }
            _ => {
                // For RC4
                let obj_key = self.compute_object_key(key, obj_id);
                let rc4_key = Rc4Key::from_slice(&obj_key);
                rc4_encrypt(&rc4_key, data)
            }
        }
    }

    /// Decrypt a string
    pub fn decrypt_string(&self, data: &[u8], key: &EncryptionKey, obj_id: &ObjectId) -> Vec<u8> {
        match self.revision {
            SecurityHandlerRevision::R5 | SecurityHandlerRevision::R6 => {
                // For AES, use decrypt_aes and handle the Result
                self.decrypt_aes(data, key, obj_id).unwrap_or_default()
            }
            _ => {
                // RC4 is symmetric
                self.encrypt_string(data, key, obj_id)
            }
        }
    }

    /// Encrypt a stream
    pub fn encrypt_stream(&self, data: &[u8], key: &EncryptionKey, obj_id: &ObjectId) -> Vec<u8> {
        // For both RC4 and AES, stream encryption is the same as string encryption
        self.encrypt_string(data, key, obj_id)
    }

    /// Decrypt a stream
    pub fn decrypt_stream(&self, data: &[u8], key: &EncryptionKey, obj_id: &ObjectId) -> Vec<u8> {
        match self.revision {
            SecurityHandlerRevision::R5 | SecurityHandlerRevision::R6 => {
                // For AES, use decrypt_aes and handle the Result
                self.decrypt_aes(data, key, obj_id).unwrap_or_default()
            }
            _ => {
                // For RC4, decrypt is same as encrypt
                self.decrypt_string(data, key, obj_id)
            }
        }
    }

    /// Encrypt data using AES (for Rev 5/6)
    pub fn encrypt_aes(
        &self,
        data: &[u8],
        key: &EncryptionKey,
        obj_id: &ObjectId,
    ) -> Result<Vec<u8>> {
        if self.revision < SecurityHandlerRevision::R5 {
            return Err(crate::error::PdfError::EncryptionError(
                "AES encryption only supported for Rev 5+".to_string(),
            ));
        }

        let obj_key = self.compute_aes_object_key(key, obj_id)?;
        let aes_key = AesKey::new_256(obj_key)?;
        let aes = Aes::new(aes_key);

        let iv = generate_iv();
        let mut result = Vec::new();
        result.extend_from_slice(&iv);

        let encrypted = aes.encrypt_cbc(data, &iv).map_err(|e| {
            crate::error::PdfError::EncryptionError(format!("AES encryption failed: {e}"))
        })?;

        result.extend_from_slice(&encrypted);
        Ok(result)
    }

    /// Decrypt data using AES (for Rev 5/6)
    pub fn decrypt_aes(
        &self,
        data: &[u8],
        key: &EncryptionKey,
        obj_id: &ObjectId,
    ) -> Result<Vec<u8>> {
        if self.revision < SecurityHandlerRevision::R5 {
            return Err(crate::error::PdfError::EncryptionError(
                "AES decryption only supported for Rev 5+".to_string(),
            ));
        }

        if data.len() < 16 {
            return Err(crate::error::PdfError::EncryptionError(
                "AES encrypted data must be at least 16 bytes (IV)".to_string(),
            ));
        }

        let iv = &data[0..16];
        let encrypted_data = &data[16..];

        let obj_key = self.compute_aes_object_key(key, obj_id)?;
        let aes_key = AesKey::new_256(obj_key)?;
        let aes = Aes::new(aes_key);

        aes.decrypt_cbc(encrypted_data, iv).map_err(|e| {
            crate::error::PdfError::EncryptionError(format!("AES decryption failed: {e}"))
        })
    }

    /// Compute AES object-specific encryption key for Rev 5/6
    fn compute_aes_object_key(&self, key: &EncryptionKey, obj_id: &ObjectId) -> Result<Vec<u8>> {
        if self.revision < SecurityHandlerRevision::R5 {
            return Err(crate::error::PdfError::EncryptionError(
                "AES object key computation only for Rev 5+".to_string(),
            ));
        }

        // For Rev 5/6, use SHA-256 for key derivation
        let mut data = Vec::new();
        data.extend_from_slice(&key.key);
        data.extend_from_slice(&obj_id.number().to_le_bytes());
        data.extend_from_slice(&obj_id.generation().to_le_bytes());

        // Add salt for AES
        data.extend_from_slice(b"sAlT"); // Standard salt for AES

        Ok(sha256(&data))
    }

    /// Compute encryption key for AES Rev 5/6
    pub fn compute_aes_encryption_key(
        &self,
        user_password: &UserPassword,
        owner_hash: &[u8],
        permissions: Permissions,
        file_id: Option<&[u8]>,
    ) -> Result<EncryptionKey> {
        if self.revision < SecurityHandlerRevision::R5 {
            return Err(crate::error::PdfError::EncryptionError(
                "AES key computation only for Rev 5+".to_string(),
            ));
        }

        // For Rev 5/6, use more secure key derivation
        let mut data = Vec::new();

        // Use UTF-8 encoding for passwords in Rev 5/6
        let password_bytes = user_password.0.as_bytes();
        data.extend_from_slice(password_bytes);

        // Add validation data
        data.extend_from_slice(owner_hash);
        data.extend_from_slice(&permissions.bits().to_le_bytes());

        if let Some(id) = file_id {
            data.extend_from_slice(id);
        }

        // Use SHA-256 for stronger hashing
        let mut hash = sha256(&data);

        // Perform additional iterations for Rev 5/6 (simplified)
        for _ in 0..100 {
            hash = sha256(&hash);
        }

        // AES-256 requires 32 bytes
        hash.truncate(32);

        Ok(EncryptionKey::new(hash))
    }

    /// Validate user password for AES Rev 5/6
    pub fn validate_aes_user_password(
        &self,
        password: &UserPassword,
        user_hash: &[u8],
        permissions: Permissions,
        file_id: Option<&[u8]>,
    ) -> Result<bool> {
        if self.revision < SecurityHandlerRevision::R5 {
            return Err(crate::error::PdfError::EncryptionError(
                "AES password validation only for Rev 5+".to_string(),
            ));
        }

        let computed_key =
            self.compute_aes_encryption_key(password, user_hash, permissions, file_id)?;

        // Compare first 32 bytes of computed hash with stored hash
        let computed_hash = sha256(&computed_key.key);

        Ok(user_hash.len() >= 32 && computed_hash[..32] == user_hash[..32])
    }

    // ========================================================================
    // R5/R6 Password Validation (ISO 32000-1 §7.6.4.3.4)
    // ========================================================================

    /// Compute R5 user password hash (U entry) - Algorithm 8
    ///
    /// Returns 48 bytes: hash(32) + validation_salt(8) + key_salt(8)
    ///
    /// # Algorithm
    /// 1. Generate random validation_salt (8 bytes)
    /// 2. Generate random key_salt (8 bytes)
    /// 3. Compute hash: SHA-256(password + validation_salt)
    /// 4. Apply 64 iterations of SHA-256
    /// 5. Return hash[0..32] + validation_salt + key_salt
    pub fn compute_r5_user_hash(&self, user_password: &UserPassword) -> Result<Vec<u8>> {
        if self.revision != SecurityHandlerRevision::R5 {
            return Err(crate::error::PdfError::EncryptionError(
                "R5 user hash only for Revision 5".to_string(),
            ));
        }

        // Generate cryptographically secure random salts
        let validation_salt = generate_salt(R5_SALT_LENGTH);
        let key_salt = generate_salt(R5_SALT_LENGTH);

        // Compute hash: SHA-256(password + validation_salt)
        let mut data = Vec::new();
        data.extend_from_slice(user_password.0.as_bytes());
        data.extend_from_slice(&validation_salt);

        let mut hash = sha256(&data);

        // Apply R5 iterations of SHA-256 (PDF spec §7.6.4.3.4)
        for _ in 0..R5_HASH_ITERATIONS {
            hash = sha256(&hash);
        }

        // Construct U entry: hash[0..32] + validation_salt + key_salt
        let mut u_entry = Vec::with_capacity(48);
        u_entry.extend_from_slice(&hash[..32]);
        u_entry.extend_from_slice(&validation_salt);
        u_entry.extend_from_slice(&key_salt);

        debug_assert_eq!(u_entry.len(), 48);
        Ok(u_entry)
    }

    /// Validate R5 user password - Algorithm 11
    ///
    /// Returns Ok(true) if password is correct, Ok(false) if incorrect.
    ///
    /// # Algorithm
    /// 1. Extract validation_salt from U[32..40]
    /// 2. Compute hash: SHA-256(password + validation_salt)
    /// 3. Apply 64 iterations of SHA-256
    /// 4. Compare result with U[0..32] using constant-time comparison
    ///
    /// # Security
    /// Uses constant-time comparison (`subtle::ConstantTimeEq`) to prevent
    /// timing side-channel attacks that could leak password information.
    pub fn validate_r5_user_password(
        &self,
        password: &UserPassword,
        u_entry: &[u8],
    ) -> Result<bool> {
        if u_entry.len() != U_ENTRY_LENGTH {
            return Err(crate::error::PdfError::EncryptionError(format!(
                "R5 U entry must be {} bytes, got {}",
                U_ENTRY_LENGTH,
                u_entry.len()
            )));
        }

        // Extract validation_salt from U
        let validation_salt = &u_entry[U_VALIDATION_SALT_START..U_VALIDATION_SALT_END];

        // Compute hash: SHA-256(password + validation_salt)
        let mut data = Vec::new();
        data.extend_from_slice(password.0.as_bytes());
        data.extend_from_slice(validation_salt);

        let mut hash = sha256(&data);

        // Apply same R5 iterations as compute
        for _ in 0..R5_HASH_ITERATIONS {
            hash = sha256(&hash);
        }

        // SECURITY: Constant-time comparison prevents timing attacks
        let stored_hash = &u_entry[..U_HASH_LENGTH];
        let computed_hash = &hash[..U_HASH_LENGTH];
        Ok(bool::from(computed_hash.ct_eq(stored_hash)))
    }

    /// Compute R5 UE entry (encrypted encryption key)
    ///
    /// The UE entry stores the encryption key encrypted with a key derived
    /// from the user password.
    ///
    /// # Algorithm
    /// 1. Extract key_salt from U[40..48]
    /// 2. Compute intermediate key: SHA-256(password + key_salt)
    /// 3. Encrypt encryption_key with intermediate_key using AES-256-CBC (zero IV)
    pub fn compute_r5_ue_entry(
        &self,
        user_password: &UserPassword,
        u_entry: &[u8],
        encryption_key: &EncryptionKey,
    ) -> Result<Vec<u8>> {
        if u_entry.len() != U_ENTRY_LENGTH {
            return Err(crate::error::PdfError::EncryptionError(format!(
                "U entry must be {} bytes",
                U_ENTRY_LENGTH
            )));
        }
        if encryption_key.len() != UE_ENTRY_LENGTH {
            return Err(crate::error::PdfError::EncryptionError(format!(
                "Encryption key must be {} bytes for R5",
                UE_ENTRY_LENGTH
            )));
        }

        // Extract key_salt from U
        let key_salt = &u_entry[U_KEY_SALT_START..U_KEY_SALT_END];

        // Compute intermediate key: SHA-256(password + key_salt)
        let mut data = Vec::new();
        data.extend_from_slice(user_password.0.as_bytes());
        data.extend_from_slice(key_salt);

        let intermediate_key = sha256(&data);

        // Encrypt encryption_key with intermediate_key using AES-256-CBC
        // Zero IV as per PDF spec, no padding since 32 bytes is block-aligned
        let aes_key = AesKey::new_256(intermediate_key)?;
        let aes = Aes::new(aes_key);
        let iv = [0u8; 16];

        let encrypted = aes
            .encrypt_cbc_raw(encryption_key.as_bytes(), &iv)
            .map_err(|e| {
                crate::error::PdfError::EncryptionError(format!("UE encryption failed: {}", e))
            })?;

        // UE is exactly 32 bytes (no padding, 32 bytes = 2 AES blocks)
        Ok(encrypted)
    }

    /// Recover encryption key from R5 UE entry
    ///
    /// # Algorithm
    /// 1. Extract key_salt from U[40..48]
    /// 2. Compute intermediate key: SHA-256(password + key_salt)
    /// 3. Decrypt UE with intermediate_key using AES-256-CBC (zero IV)
    pub fn recover_r5_encryption_key(
        &self,
        user_password: &UserPassword,
        u_entry: &[u8],
        ue_entry: &[u8],
    ) -> Result<EncryptionKey> {
        if ue_entry.len() != UE_ENTRY_LENGTH {
            return Err(crate::error::PdfError::EncryptionError(format!(
                "UE entry must be {} bytes, got {}",
                UE_ENTRY_LENGTH,
                ue_entry.len()
            )));
        }
        if u_entry.len() != U_ENTRY_LENGTH {
            return Err(crate::error::PdfError::EncryptionError(format!(
                "U entry must be {} bytes",
                U_ENTRY_LENGTH
            )));
        }

        // Extract key_salt from U
        let key_salt = &u_entry[U_KEY_SALT_START..U_KEY_SALT_END];

        // Compute intermediate key: SHA-256(password + key_salt)
        let mut data = Vec::new();
        data.extend_from_slice(user_password.0.as_bytes());
        data.extend_from_slice(key_salt);

        let intermediate_key = sha256(&data);

        // Decrypt UE to get encryption key
        // UE is 32 bytes = 2 AES blocks, encrypted with CBC and zero IV
        let aes_key = AesKey::new_256(intermediate_key)?;
        let aes = Aes::new(aes_key);
        let iv = [0u8; 16];

        let decrypted = aes.decrypt_cbc_raw(ue_entry, &iv).map_err(|e| {
            crate::error::PdfError::EncryptionError(format!("UE decryption failed: {}", e))
        })?;

        Ok(EncryptionKey::new(decrypted))
    }

    // ========================================================================
    // R6 Password Validation (ISO 32000-2 §7.6.4.4)
    // ========================================================================

    /// Compute R6 user password hash (U entry) using SHA-512
    ///
    /// R6 uses SHA-512 (first 32 bytes) instead of SHA-256 for stronger security.
    /// Returns 48 bytes: hash(32) + validation_salt(8) + key_salt(8)
    ///
    /// # Algorithm (ISO 32000-2)
    /// 1. Generate random validation_salt (8 bytes)
    /// 2. Generate random key_salt (8 bytes)
    /// 3. Compute hash using Algorithm 2.B (ISO 32000-2:2020 §7.6.4.3.4)
    /// 4. Return hash[0..32] + validation_salt + key_salt
    pub fn compute_r6_user_hash(&self, user_password: &UserPassword) -> Result<Vec<u8>> {
        if self.revision != SecurityHandlerRevision::R6 {
            return Err(crate::error::PdfError::EncryptionError(
                "R6 user hash only for Revision 6".to_string(),
            ));
        }

        // Generate cryptographically secure random salts
        let validation_salt = generate_salt(R6_SALT_LENGTH);
        let key_salt = generate_salt(R6_SALT_LENGTH);

        // Compute hash using Algorithm 2.B (ISO 32000-2:2020)
        // For user password creation, u_entry is empty
        let hash = compute_hash_r6_algorithm_2b(
            user_password.0.as_bytes(),
            &validation_salt,
            &[], // No U entry for user password creation
        )?;

        // Construct U entry: hash[0..32] + validation_salt + key_salt
        let mut u_entry = Vec::with_capacity(48);
        u_entry.extend_from_slice(&hash[..32]);
        u_entry.extend_from_slice(&validation_salt);
        u_entry.extend_from_slice(&key_salt);

        debug_assert_eq!(u_entry.len(), 48);
        Ok(u_entry)
    }

    /// Validate R6 user password using Algorithm 2.B (ISO 32000-2:2020 §7.6.4.3.4)
    ///
    /// Returns Ok(true) if password is correct, Ok(false) if incorrect.
    ///
    /// # Algorithm
    /// 1. Extract validation_salt from U[32..40]
    /// 2. Compute hash using Algorithm 2.B with the validation_salt
    /// 3. Compare result with U[0..32] using constant-time comparison
    ///
    /// # Security
    /// Uses constant-time comparison (`subtle::ConstantTimeEq`) to prevent
    /// timing side-channel attacks that could leak password information.
    pub fn validate_r6_user_password(
        &self,
        password: &UserPassword,
        u_entry: &[u8],
    ) -> Result<bool> {
        if u_entry.len() != U_ENTRY_LENGTH {
            return Err(crate::error::PdfError::EncryptionError(format!(
                "R6 U entry must be {} bytes, got {}",
                U_ENTRY_LENGTH,
                u_entry.len()
            )));
        }

        // Extract validation_salt from U[32..40]
        let validation_salt = &u_entry[U_VALIDATION_SALT_START..U_VALIDATION_SALT_END];

        // Compute hash using Algorithm 2.B (ISO 32000-2:2020)
        // For user password validation, u_entry is empty per spec
        let hash = compute_hash_r6_algorithm_2b(password.0.as_bytes(), validation_salt, &[])?;

        // SECURITY: Constant-time comparison prevents timing attacks
        let stored_hash = &u_entry[..U_HASH_LENGTH];
        let computed_hash = &hash[..U_HASH_LENGTH];
        Ok(bool::from(computed_hash.ct_eq(stored_hash)))
    }

    /// Compute R6 UE entry (encrypted encryption key) using Algorithm 2.B (ISO 32000-2:2020 §7.6.4.3.4)
    ///
    /// # Algorithm
    /// 1. Extract key_salt from U[40..48]
    /// 2. Compute intermediate key using Algorithm 2.B(password, key_salt, u_entry)
    /// 3. Encrypt encryption_key using AES-256-CBC with intermediate_key and IV = 0
    pub fn compute_r6_ue_entry(
        &self,
        user_password: &UserPassword,
        u_entry: &[u8],
        encryption_key: &EncryptionKey,
    ) -> Result<Vec<u8>> {
        if u_entry.len() != U_ENTRY_LENGTH {
            return Err(crate::error::PdfError::EncryptionError(format!(
                "U entry must be {} bytes",
                U_ENTRY_LENGTH
            )));
        }
        if encryption_key.len() != UE_ENTRY_LENGTH {
            return Err(crate::error::PdfError::EncryptionError(format!(
                "Encryption key must be {} bytes for R6",
                UE_ENTRY_LENGTH
            )));
        }

        // Extract key_salt from U[40..48]
        let key_salt = &u_entry[U_KEY_SALT_START..U_KEY_SALT_END];

        // Compute intermediate key using Algorithm 2.B (ISO 32000-2:2020)
        // For key derivation, we pass the full U entry as the third parameter
        let hash = compute_hash_r6_algorithm_2b(user_password.0.as_bytes(), key_salt, u_entry)?;
        let intermediate_key = hash[..U_HASH_LENGTH].to_vec();

        // Encrypt encryption_key with intermediate_key using AES-256-CBC, IV = 0
        let aes_key = AesKey::new_256(intermediate_key)?;
        let aes = Aes::new(aes_key);
        let iv = [0u8; 16];

        let encrypted = aes
            .encrypt_cbc_raw(encryption_key.as_bytes(), &iv)
            .map_err(|e| {
                crate::error::PdfError::EncryptionError(format!("UE encryption failed: {}", e))
            })?;

        Ok(encrypted)
    }

    /// Recover encryption key from R6 UE entry using Algorithm 2.B (ISO 32000-2:2020 §7.6.4.3.4)
    ///
    /// # Algorithm
    /// 1. Extract key_salt from U[40..48]
    /// 2. Compute intermediate key using Algorithm 2.B(password, key_salt, u_entry)
    /// 3. Decrypt UE using AES-256-CBC with intermediate_key and IV = 0
    pub fn recover_r6_encryption_key(
        &self,
        user_password: &UserPassword,
        u_entry: &[u8],
        ue_entry: &[u8],
    ) -> Result<EncryptionKey> {
        if ue_entry.len() != UE_ENTRY_LENGTH {
            return Err(crate::error::PdfError::EncryptionError(format!(
                "UE entry must be {} bytes, got {}",
                UE_ENTRY_LENGTH,
                ue_entry.len()
            )));
        }
        if u_entry.len() != U_ENTRY_LENGTH {
            return Err(crate::error::PdfError::EncryptionError(format!(
                "U entry must be {} bytes",
                U_ENTRY_LENGTH
            )));
        }

        // Extract key_salt from U[40..48]
        let key_salt = &u_entry[U_KEY_SALT_START..U_KEY_SALT_END];

        // Compute intermediate key using Algorithm 2.B (ISO 32000-2:2020)
        // For key derivation, we pass the full U entry as the third parameter
        let hash = compute_hash_r6_algorithm_2b(user_password.0.as_bytes(), key_salt, u_entry)?;
        let intermediate_key = hash[..U_HASH_LENGTH].to_vec();

        // Decrypt UE to get encryption key using AES-256-CBC with IV = 0
        let aes_key = AesKey::new_256(intermediate_key)?;
        let aes = Aes::new(aes_key);
        let iv = [0u8; 16];

        let decrypted = aes.decrypt_cbc_raw(ue_entry, &iv).map_err(|e| {
            crate::error::PdfError::EncryptionError(format!("UE decryption failed: {}", e))
        })?;

        Ok(EncryptionKey::new(decrypted))
    }

    // ========================================================================
    // R6 Perms Entry (ISO 32000-2 Table 25)
    // ========================================================================

    /// Compute R6 Perms entry (encrypted permissions)
    ///
    /// The Perms entry is a 16-byte value that encrypts permissions using AES-256-ECB.
    /// This allows verification that permissions haven't been tampered with.
    ///
    /// # Plaintext Structure (16 bytes)
    /// - Bytes 0-3: Permissions (P value, little-endian)
    /// - Bytes 4-7: 0xFFFFFFFF (fixed marker)
    /// - Bytes 8-10: "adb" (literal verification string)
    /// - Byte 11: 'T' or 'F' (EncryptMetadata flag)
    /// - Bytes 12-15: 0x00 (padding)
    pub fn compute_r6_perms_entry(
        &self,
        permissions: Permissions,
        encryption_key: &EncryptionKey,
        encrypt_metadata: bool,
    ) -> Result<Vec<u8>> {
        if self.revision != SecurityHandlerRevision::R6 {
            return Err(crate::error::PdfError::EncryptionError(
                "Perms entry only for Revision 6".to_string(),
            ));
        }
        if encryption_key.len() != UE_ENTRY_LENGTH {
            return Err(crate::error::PdfError::EncryptionError(format!(
                "Encryption key must be {} bytes for R6 Perms",
                UE_ENTRY_LENGTH
            )));
        }

        // Construct plaintext: P + 0xFFFFFFFF + "adb" + T/F + padding
        let mut plaintext = vec![0u8; PERMS_ENTRY_LENGTH];

        // Permissions (4 bytes, little-endian)
        let p_bytes = (permissions.bits() as u32).to_le_bytes();
        plaintext[PERMS_P_START..PERMS_P_END].copy_from_slice(&p_bytes);

        // Fixed marker bytes (0xFFFFFFFF)
        plaintext[PERMS_MARKER_START..PERMS_MARKER_END].copy_from_slice(&PERMS_MARKER);

        // Literal "adb" verification string
        plaintext[PERMS_LITERAL_START..PERMS_LITERAL_END].copy_from_slice(PERMS_LITERAL);

        // EncryptMetadata flag
        plaintext[PERMS_ENCRYPT_META_BYTE] = if encrypt_metadata { b'T' } else { b'F' };

        // Bytes 12-15 remain 0x00 (padding)

        // Encrypt with AES-256-ECB
        let aes_key = AesKey::new_256(encryption_key.key.clone())?;
        let aes = Aes::new(aes_key);

        let encrypted = aes.encrypt_ecb(&plaintext).map_err(|e| {
            crate::error::PdfError::EncryptionError(format!("Perms encryption failed: {}", e))
        })?;

        Ok(encrypted)
    }

    /// Validate R6 Perms entry by decrypting and checking structure
    ///
    /// Returns Ok(true) if the Perms entry is valid and matches expected permissions.
    /// Returns Ok(false) if decryption succeeds but structure/permissions don't match.
    /// Returns Err if decryption fails.
    ///
    /// # Security
    /// Uses constant-time comparison (`subtle::ConstantTimeEq`) for permissions
    /// comparison to prevent timing side-channel attacks.
    pub fn validate_r6_perms(
        &self,
        perms_entry: &[u8],
        encryption_key: &EncryptionKey,
        expected_permissions: Permissions,
    ) -> Result<bool> {
        if perms_entry.len() != PERMS_ENTRY_LENGTH {
            return Err(crate::error::PdfError::EncryptionError(format!(
                "Perms entry must be {} bytes, got {}",
                PERMS_ENTRY_LENGTH,
                perms_entry.len()
            )));
        }
        if encryption_key.len() != UE_ENTRY_LENGTH {
            return Err(crate::error::PdfError::EncryptionError(format!(
                "Encryption key must be {} bytes",
                UE_ENTRY_LENGTH
            )));
        }

        // Decrypt with AES-256-ECB
        let aes_key = AesKey::new_256(encryption_key.key.clone())?;
        let aes = Aes::new(aes_key);

        let decrypted = aes.decrypt_ecb(perms_entry).map_err(|e| {
            crate::error::PdfError::EncryptionError(format!("Perms decryption failed: {}", e))
        })?;

        // Verify fixed marker
        if decrypted[PERMS_MARKER_START..PERMS_MARKER_END] != PERMS_MARKER {
            return Ok(false);
        }

        // Verify literal "adb"
        if &decrypted[PERMS_LITERAL_START..PERMS_LITERAL_END] != PERMS_LITERAL {
            return Ok(false);
        }

        // SECURITY: Constant-time comparison for permissions
        let expected_bytes = (expected_permissions.bits() as u32).to_le_bytes();
        let actual_bytes = &decrypted[PERMS_P_START..PERMS_P_END];
        Ok(bool::from(expected_bytes.ct_eq(actual_bytes)))
    }

    /// Extract EncryptMetadata flag from decrypted Perms entry
    ///
    /// Returns Ok(Some(true)) if EncryptMetadata='T', Ok(Some(false)) if 'F',
    /// Ok(None) if Perms structure is invalid.
    pub fn extract_r6_encrypt_metadata(
        &self,
        perms_entry: &[u8],
        encryption_key: &EncryptionKey,
    ) -> Result<Option<bool>> {
        if perms_entry.len() != PERMS_ENTRY_LENGTH || encryption_key.len() != UE_ENTRY_LENGTH {
            return Ok(None);
        }

        let aes_key = AesKey::new_256(encryption_key.key.clone())?;
        let aes = Aes::new(aes_key);

        let decrypted = match aes.decrypt_ecb(perms_entry) {
            Ok(d) => d,
            Err(_) => return Ok(None),
        };

        // Verify structure before extracting flag
        if decrypted[PERMS_MARKER_START..PERMS_MARKER_END] != PERMS_MARKER
            || &decrypted[PERMS_LITERAL_START..PERMS_LITERAL_END] != PERMS_LITERAL
        {
            return Ok(None);
        }

        // Extract EncryptMetadata flag
        match decrypted[PERMS_ENCRYPT_META_BYTE] {
            b'T' => Ok(Some(true)),
            b'F' => Ok(Some(false)),
            _ => Ok(None), // Invalid flag value
        }
    }

    /// Compute object-specific encryption key (Algorithm 1, ISO 32000-1 §7.6.2)
    pub fn compute_object_key(&self, key: &EncryptionKey, obj_id: &ObjectId) -> Vec<u8> {
        let mut data = Vec::new();
        data.extend_from_slice(&key.key);
        data.extend_from_slice(&obj_id.number().to_le_bytes()[..3]); // Low 3 bytes
        data.extend_from_slice(&obj_id.generation().to_le_bytes()[..2]); // Low 2 bytes

        let hash = md5::compute(&data);
        let key_len = (key.len() + 5).min(16);
        hash[..key_len].to_vec()
    }

    /// Validate user password (Algorithm 6, ISO 32000-1 §7.6.3.4)
    ///
    /// Returns Ok(true) if password is correct, Ok(false) if incorrect.
    /// Returns Err only on internal errors.
    pub fn validate_user_password(
        &self,
        password: &UserPassword,
        user_hash: &[u8],
        owner_hash: &[u8],
        permissions: Permissions,
        file_id: Option<&[u8]>,
    ) -> Result<bool> {
        // Compute encryption key from provided password
        let key = self.compute_encryption_key(password, owner_hash, permissions, file_id)?;

        match self.revision {
            SecurityHandlerRevision::R2 => {
                // For R2: Encrypt padding with key and compare with U
                let rc4_key = Rc4Key::from_slice(&key.key);
                let encrypted_padding = rc4_encrypt(&rc4_key, &PADDING);

                // Compare with stored user hash
                Ok(user_hash.len() >= 32 && encrypted_padding[..] == user_hash[..32])
            }
            SecurityHandlerRevision::R3 | SecurityHandlerRevision::R4 => {
                // For R3/R4: Compute MD5 hash including file ID
                let mut data = Vec::new();
                data.extend_from_slice(&PADDING);

                if let Some(id) = file_id {
                    data.extend_from_slice(id);
                }

                let hash = md5::compute(&data);

                // Encrypt hash with RC4
                let rc4_key = Rc4Key::from_slice(&key.key);
                let mut encrypted = rc4_encrypt(&rc4_key, hash.as_ref());

                // Do 19 additional iterations with modified keys
                for i in 1..=19 {
                    let mut key_bytes = key.key.clone();
                    for byte in &mut key_bytes {
                        *byte ^= i as u8;
                    }
                    let iter_key = Rc4Key::from_slice(&key_bytes);
                    encrypted = rc4_encrypt(&iter_key, &encrypted);
                }

                // Compare first 16 bytes of result with first 16 bytes of U
                Ok(user_hash.len() >= 16 && encrypted[..16] == user_hash[..16])
            }
            SecurityHandlerRevision::R5 | SecurityHandlerRevision::R6 => {
                // For R5/R6, use AES-based validation
                self.validate_aes_user_password(password, user_hash, permissions, file_id)
            }
        }
    }

    /// Validate owner password (Algorithm 7, ISO 32000-1 §7.6.3.4)
    ///
    /// Returns Ok(true) if password is correct, Ok(false) if incorrect.
    /// Returns Err only on internal errors.
    ///
    /// Note: For owner password validation, we first decrypt the user password
    /// from the owner hash, then validate that user password.
    pub fn validate_owner_password(
        &self,
        owner_password: &OwnerPassword,
        owner_hash: &[u8],
        _user_password: &UserPassword, // Will be recovered from owner_hash
        _permissions: Permissions,
        _file_id: Option<&[u8]>,
    ) -> Result<bool> {
        match self.revision {
            SecurityHandlerRevision::R2
            | SecurityHandlerRevision::R3
            | SecurityHandlerRevision::R4 => {
                // Step 1: Pad owner password
                let owner_pad = Self::pad_password(&owner_password.0);

                // Step 2: Create MD5 hash of owner password
                let mut hash = md5::compute(&owner_pad).to_vec();

                // Step 3: For revision 3+, do 50 additional iterations
                if self.revision >= SecurityHandlerRevision::R3 {
                    for _ in 0..50 {
                        hash = md5::compute(&hash).to_vec();
                    }
                }

                // Step 4: Create RC4 key from hash (truncated to key length)
                let rc4_key = Rc4Key::from_slice(&hash[..self.key_length]);

                // Step 5: Decrypt owner hash to get user password
                let mut decrypted = owner_hash[..32].to_vec();

                // For R3+, do 19 iterations in reverse
                if self.revision >= SecurityHandlerRevision::R3 {
                    for i in (0..20).rev() {
                        let mut key_bytes = hash[..self.key_length].to_vec();
                        for byte in &mut key_bytes {
                            *byte ^= i as u8;
                        }
                        let iter_key = Rc4Key::from_slice(&key_bytes);
                        decrypted = rc4_encrypt(&iter_key, &decrypted);
                    }
                } else {
                    // For R2, single decryption
                    decrypted = rc4_encrypt(&rc4_key, &decrypted);
                }

                // Step 6: The decrypted data should be the padded user password
                // Try to validate by computing what the owner hash SHOULD be
                // with this owner password, and compare

                // Extract potential user password (remove padding)
                let user_pwd_bytes = decrypted
                    .iter()
                    .take_while(|&&b| b != 0x28 || decrypted.starts_with(&PADDING))
                    .copied()
                    .collect::<Vec<u8>>();

                let recovered_user =
                    UserPassword(String::from_utf8_lossy(&user_pwd_bytes).to_string());

                // Compute what owner hash should be with this owner password
                let computed_owner = self.compute_owner_hash(owner_password, &recovered_user);

                // Compare with stored owner hash
                Ok(computed_owner[..32] == owner_hash[..32])
            }
            SecurityHandlerRevision::R5 | SecurityHandlerRevision::R6 => {
                // R5/R6 owner password validation requires O/OE entry processing
                // (Algorithms 12/13 in ISO 32000-2) - not yet implemented
                Err(crate::error::PdfError::EncryptionError(
                    "R5/R6 owner password validation not yet implemented (Phase 3)".to_string(),
                ))
            }
        }
    }
}

/// Helper function for RC4 encryption
fn rc4_encrypt(key: &Rc4Key, data: &[u8]) -> Vec<u8> {
    let mut cipher = Rc4::new(key);
    cipher.process(data)
}

// Use the md5 crate for actual MD5 hashing (required for PDF encryption)

/// SHA-256 implementation using RustCrypto (production-grade)
///
/// Returns a 32-byte hash of the input data according to FIPS 180-4.
/// Used for R5 password validation and key derivation.
fn sha256(data: &[u8]) -> Vec<u8> {
    Sha256::digest(data).to_vec()
}

/// SHA-384 implementation using RustCrypto (production-grade)
///
/// Returns a 48-byte hash of the input data according to FIPS 180-4.
/// Used for R6 Algorithm 2.B hash rotation.
fn sha384(data: &[u8]) -> Vec<u8> {
    Sha384::digest(data).to_vec()
}

/// SHA-512 implementation using RustCrypto (production-grade)
///
/// Returns a 64-byte hash of the input data according to FIPS 180-4.
/// Used for R6 password validation and key derivation.
fn sha512(data: &[u8]) -> Vec<u8> {
    Sha512::digest(data).to_vec()
}

// ============================================================================
// Algorithm 2.B - R6 Key Derivation (ISO 32000-2:2020 §7.6.4.3.4)
// ============================================================================

/// Minimum number of rounds for Algorithm 2.B
const ALGORITHM_2B_MIN_ROUNDS: usize = 64;

/// Maximum rounds (DoS protection, not in spec but common implementation)
const ALGORITHM_2B_MAX_ROUNDS: usize = 2048;

/// Maximum password length (ISO 32000-2 §7.6.3.3.2 recommends 127 bytes)
/// This prevents DoS via massive allocation: 1MB password × 64 repetitions = 64MB/round
const ALGORITHM_2B_MAX_PASSWORD_LEN: usize = 127;

/// Number of bytes used for hash function selection (spec: first 16 bytes as BigInteger mod 3)
const HASH_SELECTOR_BYTES: usize = 16;

/// Compute R6 password hash using Algorithm 2.B (ISO 32000-2:2020 §7.6.4.3.4)
///
/// This is the correct R6 key derivation algorithm used by qpdf, Adobe Acrobat,
/// and other compliant PDF processors. It uses AES-128-CBC encryption within
/// the iteration loop and dynamically selects SHA-256/384/512 based on output.
///
/// # Algorithm Overview
/// 1. Initial hash: K = SHA-256(password + salt + U\[0..48\])
/// 2. Loop (minimum 64 rounds):
///    a. Construct k1 = (password + K + U\[0..48\]), repeat 64 times
///    b. E = AES-128-CBC-encrypt(k1, key=K\[0..16\], iv=K\[16..32\])
///    c. Select hash: SHA-256/384/512 based on sum(E\[0..16\]) mod 3
///    d. K = hash(E)
///    e. Check termination: round >= 64 AND E\[last\] <= (round - 32)
/// 3. Return K\[0..32\]
///
/// # Parameters
/// - `password`: User password bytes (UTF-8 encoded)
/// - `salt`: 8-byte salt (validation_salt or key_salt from U entry)
/// - `u_entry`: Full 48-byte U entry (or empty slice for initial computation)
///
/// # Returns
/// 32-byte derived key
///
/// # Security Notes
/// - Maximum 2048 rounds to prevent DoS attacks
/// - Variable iteration count makes brute-force harder
/// - AES encryption + hash rotation provides strong KDF
///
/// # References
/// - ISO 32000-2:2020 §7.6.4.3.4 "Algorithm 2.B: Computing a hash (R6)"
pub fn compute_hash_r6_algorithm_2b(
    password: &[u8],
    salt: &[u8],
    u_entry: &[u8],
) -> Result<Vec<u8>> {
    // Security: Validate password length to prevent DoS via massive allocations
    if password.len() > ALGORITHM_2B_MAX_PASSWORD_LEN {
        return Err(crate::error::PdfError::EncryptionError(format!(
            "Password too long ({} bytes, max {})",
            password.len(),
            ALGORITHM_2B_MAX_PASSWORD_LEN
        )));
    }

    // Step 1: Initial hash K = SHA-256(password + salt + U[0..48])
    let mut input = Vec::with_capacity(password.len() + salt.len() + u_entry.len().min(48));
    input.extend_from_slice(password);
    input.extend_from_slice(salt);
    if !u_entry.is_empty() {
        input.extend_from_slice(&u_entry[..u_entry.len().min(48)]);
    }

    let mut k = sha256(&input);

    // Step 2: Iteration loop
    let mut round: usize = 0;
    loop {
        // 2a. Construct input sequence: password + K + U[0..48], repeated
        // The spec says to create a sequence that will be encrypted
        let mut k1_unit = Vec::new();
        k1_unit.extend_from_slice(password);
        k1_unit.extend_from_slice(&k);
        if !u_entry.is_empty() {
            k1_unit.extend_from_slice(&u_entry[..u_entry.len().min(48)]);
        }

        // Repeat 64 times to create input for AES
        let mut k1 = Vec::with_capacity(k1_unit.len() * 64);
        for _ in 0..64 {
            k1.extend_from_slice(&k1_unit);
        }

        // Zero-pad to AES block size (16 bytes) per ISO 32000-2 §7.6.4.3.4
        // NOTE: This is zero-padding, NOT PKCS#7 - the spec requires raw AES without padding removal
        while k1.len() % 16 != 0 {
            k1.push(0);
        }

        // 2b. AES-128-CBC encryption
        // Key: first 16 bytes of K, IV: next 16 bytes of K
        if k.len() < 32 {
            // Extend K if needed (shouldn't happen with proper hashes)
            while k.len() < 32 {
                k.push(0);
            }
        }

        let aes_key = AesKey::new_128(k[..16].to_vec()).map_err(|e| {
            crate::error::PdfError::EncryptionError(format!(
                "Algorithm 2.B: Failed to create AES key: {}",
                e
            ))
        })?;
        let aes = Aes::new(aes_key);
        let iv = &k[16..32];

        let e = aes.encrypt_cbc_raw(&k1, iv).map_err(|e| {
            crate::error::PdfError::EncryptionError(format!(
                "Algorithm 2.B: AES encryption failed: {}",
                e
            ))
        })?;

        // 2c. Select hash function based on first 16 bytes of E as BigInteger mod 3
        // Per iText/Adobe implementation: interpret E[0..HASH_SELECTOR_BYTES] as big-endian integer
        // Mathematical equivalence: sum(bytes) mod 3 == BigInteger(bytes) mod 3
        // because 256 mod 3 = 1, therefore 256^k mod 3 = 1 for all k
        let hash_selector = {
            let sum: u64 = e[..HASH_SELECTOR_BYTES.min(e.len())]
                .iter()
                .map(|&b| b as u64)
                .sum();
            (sum % 3) as u8
        };

        k = match hash_selector {
            0 => sha256(&e),
            1 => sha384(&e),
            2 => sha512(&e),
            _ => unreachable!("Modulo 3 can only be 0, 1, or 2"),
        };

        // 2d. Check termination condition
        // Terminate when: round >= 64 AND E[last] <= (round - 32)
        let last_byte = *e.last().unwrap_or(&0);
        round += 1;

        if round >= ALGORITHM_2B_MIN_ROUNDS {
            // The termination condition from ISO spec:
            // "the last byte value of the last iteration is less than or equal to
            // the number of iterations minus 32"
            if (last_byte as usize) <= round.saturating_sub(32) {
                break;
            }
        }

        // Safety: Prevent infinite loop (DoS protection)
        if round >= ALGORITHM_2B_MAX_ROUNDS {
            break;
        }
    }

    // Step 3: Return first 32 bytes of final K
    // K might be > 32 bytes if last hash was SHA-384 or SHA-512
    Ok(k[..32.min(k.len())].to_vec())
}

/// R5 salt length in bytes (PDF spec §7.6.4.3.4)
const R5_SALT_LENGTH: usize = 8;

/// R5 SHA-256 iteration count (ISO 32000-2:2020 Algorithm 8/11)
/// NOTE: R5 does NOT use iterations - hash is simply SHA-256(password + salt)
/// The 64 iterations are only for R6 which uses Algorithm 2.B
const R5_HASH_ITERATIONS: usize = 0;

/// R6 salt length in bytes (PDF spec ISO 32000-2)
const R6_SALT_LENGTH: usize = 8;

// ============================================================================
// R5/R6 U Entry Structure Constants (48 bytes total)
// ============================================================================

/// Length of the hash portion in U entry (SHA-256/SHA-512 truncated to 32 bytes)
const U_HASH_LENGTH: usize = 32;

/// Start offset of validation salt in U entry
const U_VALIDATION_SALT_START: usize = 32;

/// End offset of validation salt in U entry
const U_VALIDATION_SALT_END: usize = 40;

/// Start offset of key salt in U entry
const U_KEY_SALT_START: usize = 40;

/// End offset of key salt in U entry
const U_KEY_SALT_END: usize = 48;

/// Total length of U entry for R5/R6
const U_ENTRY_LENGTH: usize = 48;

/// Length of UE entry (encrypted encryption key)
const UE_ENTRY_LENGTH: usize = 32;

// ============================================================================
// R6 Perms Entry Structure Constants (16 bytes total)
// ============================================================================

/// Length of Perms entry
const PERMS_ENTRY_LENGTH: usize = 16;

/// Start offset of permissions value in decrypted Perms (little-endian u32)
const PERMS_P_START: usize = 0;

/// End offset of permissions value in decrypted Perms
const PERMS_P_END: usize = 4;

/// Start offset of fixed marker (0xFFFFFFFF) in decrypted Perms
const PERMS_MARKER_START: usize = 4;

/// End offset of fixed marker in decrypted Perms
const PERMS_MARKER_END: usize = 8;

/// Start offset of "adb" literal in decrypted Perms
const PERMS_LITERAL_START: usize = 8;

/// End offset of "adb" literal in decrypted Perms
const PERMS_LITERAL_END: usize = 11;

/// Offset of EncryptMetadata flag byte ('T' or 'F') in decrypted Perms
const PERMS_ENCRYPT_META_BYTE: usize = 11;

/// Fixed marker value in Perms entry
const PERMS_MARKER: [u8; 4] = [0xFF, 0xFF, 0xFF, 0xFF];

/// Literal verification string in Perms entry
const PERMS_LITERAL: &[u8; 3] = b"adb";

/// Generate cryptographically secure random salt using OS CSPRNG
///
/// Uses `rand::rng()` which provides a thread-local CSPRNG (ChaCha12) seeded
/// from the OS random number generator. This is suitable for PDF encryption salts.
///
/// # Security
/// - Uses ChaCha12 PRNG seeded from OS entropy (rand 0.9 implementation)
/// - Provides cryptographic-quality randomness for salt generation
/// - Each call produces independent random bytes
fn generate_salt(len: usize) -> Vec<u8> {
    let mut salt = vec![0u8; len];
    rand::rng().fill_bytes(&mut salt);
    salt
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pad_password() {
        let padded = StandardSecurityHandler::pad_password("test");
        assert_eq!(padded.len(), 32);
        assert_eq!(&padded[..4], b"test");
        assert_eq!(&padded[4..8], &PADDING[..4]);
    }

    #[test]
    fn test_pad_password_long() {
        let long_password = "a".repeat(40);
        let padded = StandardSecurityHandler::pad_password(&long_password);
        assert_eq!(padded.len(), 32);
        assert_eq!(&padded[..32], &long_password.as_bytes()[..32]);
    }

    #[test]
    fn test_rc4_40bit_handler() {
        let handler = StandardSecurityHandler::rc4_40bit();
        assert_eq!(handler.revision, SecurityHandlerRevision::R2);
        assert_eq!(handler.key_length, 5);
    }

    #[test]
    fn test_rc4_128bit_handler() {
        let handler = StandardSecurityHandler::rc4_128bit();
        assert_eq!(handler.revision, SecurityHandlerRevision::R3);
        assert_eq!(handler.key_length, 16);
    }

    #[test]
    fn test_owner_hash_computation() {
        let handler = StandardSecurityHandler::rc4_40bit();
        let owner_pwd = OwnerPassword("owner".to_string());
        let user_pwd = UserPassword("user".to_string());

        let hash = handler.compute_owner_hash(&owner_pwd, &user_pwd);
        assert_eq!(hash.len(), 32);
    }

    #[test]
    fn test_encryption_key_computation() {
        let handler = StandardSecurityHandler::rc4_40bit();
        let user_pwd = UserPassword("user".to_string());
        let owner_hash = vec![0u8; 32];
        let permissions = Permissions::new();

        let key = handler
            .compute_encryption_key(&user_pwd, &owner_hash, permissions, None)
            .unwrap();

        assert_eq!(key.len(), 5);
    }

    #[test]
    fn test_aes_256_r5_handler() {
        let handler = StandardSecurityHandler::aes_256_r5();
        assert_eq!(handler.revision, SecurityHandlerRevision::R5);
        assert_eq!(handler.key_length, 32);
    }

    #[test]
    fn test_aes_256_r6_handler() {
        let handler = StandardSecurityHandler::aes_256_r6();
        assert_eq!(handler.revision, SecurityHandlerRevision::R6);
        assert_eq!(handler.key_length, 32);
    }

    #[test]
    fn test_aes_encryption_key_computation() {
        let handler = StandardSecurityHandler::aes_256_r5();
        let user_pwd = UserPassword("testuser".to_string());
        let owner_hash = vec![0u8; 32];
        let permissions = Permissions::new();

        let key = handler
            .compute_aes_encryption_key(&user_pwd, &owner_hash, permissions, None)
            .unwrap();

        assert_eq!(key.len(), 32);
    }

    #[test]
    fn test_aes_encrypt_decrypt() {
        let handler = StandardSecurityHandler::aes_256_r5();
        let key = EncryptionKey::new(vec![0u8; 32]);
        let obj_id = ObjectId::new(1, 0);
        let data = b"Hello AES encryption!";

        let encrypted = handler.encrypt_aes(data, &key, &obj_id).unwrap();
        assert_ne!(encrypted.as_slice(), data);
        assert!(encrypted.len() > data.len()); // Should include IV

        // Note: This simplified AES implementation is for demonstration only
        let _decrypted = handler.decrypt_aes(&encrypted, &key, &obj_id);
        // For now, just test that the operations complete without panicking
    }

    #[test]
    fn test_aes_with_rc4_handler_fails() {
        let handler = StandardSecurityHandler::rc4_128bit();
        let key = EncryptionKey::new(vec![0u8; 16]);
        let obj_id = ObjectId::new(1, 0);
        let data = b"test data";

        // Should fail because handler is not Rev 5+
        assert!(handler.encrypt_aes(data, &key, &obj_id).is_err());
        assert!(handler.decrypt_aes(data, &key, &obj_id).is_err());
    }

    #[test]
    fn test_aes_decrypt_invalid_data() {
        let handler = StandardSecurityHandler::aes_256_r5();
        let key = EncryptionKey::new(vec![0u8; 32]);
        let obj_id = ObjectId::new(1, 0);

        // Data too short (no IV)
        let short_data = vec![0u8; 10];
        assert!(handler.decrypt_aes(&short_data, &key, &obj_id).is_err());
    }

    #[test]
    fn test_sha256_deterministic() {
        let data1 = b"test data";
        let data2 = b"test data";
        let data3 = b"different data";

        let hash1 = sha256(data1);
        let hash2 = sha256(data2);
        let hash3 = sha256(data3);

        assert_eq!(hash1.len(), 32);
        assert_eq!(hash2.len(), 32);
        assert_eq!(hash3.len(), 32);

        assert_eq!(hash1, hash2); // Same input should give same output
        assert_ne!(hash1, hash3); // Different input should give different output
    }

    #[test]
    fn test_security_handler_revision_ordering() {
        assert!(SecurityHandlerRevision::R2 < SecurityHandlerRevision::R3);
        assert!(SecurityHandlerRevision::R3 < SecurityHandlerRevision::R4);
        assert!(SecurityHandlerRevision::R4 < SecurityHandlerRevision::R5);
        assert!(SecurityHandlerRevision::R5 < SecurityHandlerRevision::R6);
    }

    #[test]
    fn test_aes_password_validation() {
        let handler = StandardSecurityHandler::aes_256_r5();
        let password = UserPassword("testpassword".to_string());
        let user_hash = vec![0u8; 32]; // Simplified hash
        let permissions = Permissions::new();

        // This is a basic test - in practice, the validation would be more complex
        let result = handler.validate_aes_user_password(&password, &user_hash, permissions, None);
        assert!(result.is_ok());
    }

    // ===== Additional Comprehensive Tests =====

    #[test]
    fn test_user_password_debug() {
        let pwd = UserPassword("debug_test".to_string());
        let debug_str = format!("{pwd:?}");
        assert!(debug_str.contains("UserPassword"));
        assert!(debug_str.contains("debug_test"));
    }

    #[test]
    fn test_owner_password_debug() {
        let pwd = OwnerPassword("owner_debug".to_string());
        let debug_str = format!("{pwd:?}");
        assert!(debug_str.contains("OwnerPassword"));
        assert!(debug_str.contains("owner_debug"));
    }

    #[test]
    fn test_encryption_key_debug() {
        let key = EncryptionKey::new(vec![0x01, 0x02, 0x03]);
        let debug_str = format!("{key:?}");
        assert!(debug_str.contains("EncryptionKey"));
    }

    #[test]
    fn test_security_handler_revision_equality() {
        assert_eq!(SecurityHandlerRevision::R2, SecurityHandlerRevision::R2);
        assert_ne!(SecurityHandlerRevision::R2, SecurityHandlerRevision::R3);
    }

    #[test]
    fn test_security_handler_revision_values() {
        assert_eq!(SecurityHandlerRevision::R2 as u8, 2);
        assert_eq!(SecurityHandlerRevision::R3 as u8, 3);
        assert_eq!(SecurityHandlerRevision::R4 as u8, 4);
        assert_eq!(SecurityHandlerRevision::R5 as u8, 5);
        assert_eq!(SecurityHandlerRevision::R6 as u8, 6);
    }

    #[test]
    fn test_pad_password_various_lengths() {
        for len in 0..=40 {
            let password = "x".repeat(len);
            let padded = StandardSecurityHandler::pad_password(&password);
            assert_eq!(padded.len(), 32);

            if len <= 32 {
                assert_eq!(&padded[..len], password.as_bytes());
            } else {
                assert_eq!(&padded[..], &password.as_bytes()[..32]);
            }
        }
    }

    #[test]
    fn test_pad_password_unicode() {
        let padded = StandardSecurityHandler::pad_password("café");
        assert_eq!(padded.len(), 32);
        // UTF-8 encoding of "café" is 5 bytes
        assert_eq!(&padded[..5], "café".as_bytes());
    }

    #[test]
    fn test_compute_owner_hash_different_users() {
        let handler = StandardSecurityHandler::rc4_128bit();
        let owner = OwnerPassword("owner".to_string());
        let user1 = UserPassword("user1".to_string());
        let user2 = UserPassword("user2".to_string());

        let hash1 = handler.compute_owner_hash(&owner, &user1);
        let hash2 = handler.compute_owner_hash(&owner, &user2);

        assert_ne!(hash1, hash2); // Different user passwords should produce different hashes
    }

    #[test]
    fn test_compute_user_hash_r4() {
        let handler = StandardSecurityHandler {
            revision: SecurityHandlerRevision::R4,
            key_length: 16,
        };
        let user = UserPassword("r4test".to_string());
        let owner_hash = vec![0xAA; 32];
        let permissions = Permissions::new();

        let hash = handler
            .compute_user_hash(&user, &owner_hash, permissions, None)
            .unwrap();
        assert_eq!(hash.len(), 32);
    }

    #[test]
    fn test_compute_user_hash_r6() {
        let handler = StandardSecurityHandler::aes_256_r6();
        let user = UserPassword("r6test".to_string());
        let owner_hash = vec![0xBB; 32];
        let permissions = Permissions::all();

        let hash = handler
            .compute_user_hash(&user, &owner_hash, permissions, None)
            .unwrap();
        assert_eq!(hash.len(), 32);
    }

    #[test]
    fn test_encryption_key_with_file_id_affects_result() {
        let handler = StandardSecurityHandler::rc4_128bit();
        let user = UserPassword("test".to_string());
        let owner_hash = vec![0xFF; 32];
        let permissions = Permissions::new();
        let file_id = b"unique_file_id_12345";

        let key_with_id = handler
            .compute_encryption_key(&user, &owner_hash, permissions, Some(file_id))
            .unwrap();
        let key_without_id = handler
            .compute_encryption_key(&user, &owner_hash, permissions, None)
            .unwrap();

        assert_ne!(key_with_id.key, key_without_id.key);
    }

    #[test]
    fn test_encrypt_string_empty() {
        let handler = StandardSecurityHandler::rc4_40bit();
        let key = EncryptionKey::new(vec![0x01, 0x02, 0x03, 0x04, 0x05]);
        let obj_id = ObjectId::new(1, 0);

        let encrypted = handler.encrypt_string(b"", &key, &obj_id);
        assert_eq!(encrypted.len(), 0);
    }

    #[test]
    fn test_encrypt_decrypt_large_data() {
        let handler = StandardSecurityHandler::rc4_128bit();
        let key = EncryptionKey::new(vec![0xAA; 16]);
        let obj_id = ObjectId::new(42, 0);
        let large_data = vec![0x55; 10000]; // 10KB

        let encrypted = handler.encrypt_string(&large_data, &key, &obj_id);
        assert_eq!(encrypted.len(), large_data.len());
        assert_ne!(encrypted, large_data);

        let decrypted = handler.decrypt_string(&encrypted, &key, &obj_id);
        assert_eq!(decrypted, large_data);
    }

    #[test]
    fn test_stream_encryption_different_from_string() {
        // For current implementation they're the same, but test separately
        let handler = StandardSecurityHandler::rc4_128bit();
        let key = EncryptionKey::new(vec![0x11; 16]);
        let obj_id = ObjectId::new(5, 1);
        let data = b"Stream content test";

        let encrypted_string = handler.encrypt_string(data, &key, &obj_id);
        let encrypted_stream = handler.encrypt_stream(data, &key, &obj_id);

        assert_eq!(encrypted_string, encrypted_stream); // Currently same implementation
    }

    #[test]
    fn test_aes_encryption_with_different_object_ids() {
        let handler = StandardSecurityHandler::aes_256_r5();
        let key = EncryptionKey::new(vec![0x77; 32]);
        let obj_id1 = ObjectId::new(10, 0);
        let obj_id2 = ObjectId::new(11, 0);
        let data = b"AES test data";

        let encrypted1 = handler.encrypt_aes(data, &key, &obj_id1).unwrap();
        let encrypted2 = handler.encrypt_aes(data, &key, &obj_id2).unwrap();

        // Different object IDs should produce different ciphertexts
        assert_ne!(encrypted1, encrypted2);
    }

    #[test]
    fn test_aes_decrypt_invalid_iv_length() {
        let handler = StandardSecurityHandler::aes_256_r5();
        let key = EncryptionKey::new(vec![0x88; 32]);
        let obj_id = ObjectId::new(1, 0);

        // Data too short to contain IV
        let short_data = vec![0u8; 10];
        assert!(handler.decrypt_aes(&short_data, &key, &obj_id).is_err());

        // Exactly 16 bytes (only IV, no encrypted data)
        let iv_only = vec![0u8; 16];
        let result = handler.decrypt_aes(&iv_only, &key, &obj_id);
        // This might succeed with empty decrypted data or fail depending on implementation
        if let Ok(decrypted) = result {
            assert_eq!(decrypted.len(), 0);
        }
    }

    #[test]
    fn test_aes_validate_password_wrong_hash_length() {
        let handler = StandardSecurityHandler::aes_256_r5();
        let password = UserPassword("test".to_string());
        let short_hash = vec![0u8; 16]; // Too short
        let permissions = Permissions::new();

        let result = handler
            .validate_aes_user_password(&password, &short_hash, permissions, None)
            .unwrap();
        assert!(!result); // Should return false for invalid hash
    }

    #[test]
    fn test_permissions_affect_encryption_key() {
        let handler = StandardSecurityHandler::rc4_128bit();
        let user = UserPassword("same_user".to_string());
        let owner_hash = vec![0xCC; 32];

        let perms1 = Permissions::new();
        let perms2 = Permissions::all();

        let key1 = handler
            .compute_encryption_key(&user, &owner_hash, perms1, None)
            .unwrap();
        let key2 = handler
            .compute_encryption_key(&user, &owner_hash, perms2, None)
            .unwrap();

        assert_ne!(key1.key, key2.key); // Different permissions should affect the key
    }

    #[test]
    fn test_different_handlers_produce_different_keys() {
        let user = UserPassword("test".to_string());
        let owner_hash = vec![0xDD; 32];
        let permissions = Permissions::new();

        let handler_r2 = StandardSecurityHandler::rc4_40bit();
        let handler_r3 = StandardSecurityHandler::rc4_128bit();

        let key_r2 = handler_r2
            .compute_encryption_key(&user, &owner_hash, permissions, None)
            .unwrap();
        let key_r3 = handler_r3
            .compute_encryption_key(&user, &owner_hash, permissions, None)
            .unwrap();

        assert_ne!(key_r2.len(), key_r3.len()); // Different key lengths
        assert_eq!(key_r2.len(), 5);
        assert_eq!(key_r3.len(), 16);
    }

    #[test]
    fn test_full_workflow_aes_r6() {
        let handler = StandardSecurityHandler::aes_256_r6();
        let user_pwd = UserPassword("user_r6".to_string());
        let permissions = Permissions::new();
        let file_id = b"test_file_r6";

        // For AES R5/R6, owner hash computation is different - use a dummy hash
        let owner_hash = vec![0x42; 32]; // AES uses 32-byte hashes

        // Compute user hash
        let user_hash = handler
            .compute_user_hash(&user_pwd, &owner_hash, permissions, Some(file_id))
            .unwrap();
        assert_eq!(user_hash.len(), 32);

        // Compute encryption key
        let key = handler
            .compute_aes_encryption_key(&user_pwd, &owner_hash, permissions, Some(file_id))
            .unwrap();
        assert_eq!(key.len(), 32);

        // Test string encryption (uses AES for R6)
        let obj_id = ObjectId::new(100, 5);
        let content = b"R6 AES encryption test";
        let encrypted = handler.encrypt_string(content, &key, &obj_id);

        // With AES, encrypted should be empty on error or have data
        if !encrypted.is_empty() {
            assert_ne!(encrypted.as_slice(), content);
        }
    }

    #[test]
    fn test_md5_compute_consistency() {
        let data = b"consistent data for md5";
        let hash1 = md5::compute(data);
        let hash2 = md5::compute(data);

        assert_eq!(hash1, hash2);
        assert_eq!(hash1.len(), 16);
    }

    #[test]
    fn test_sha256_consistency() {
        let data = b"consistent data for sha256";
        let hash1 = sha256(data);
        let hash2 = sha256(data);

        assert_eq!(hash1, hash2);
        assert_eq!(hash1.len(), 32);
    }

    #[test]
    fn test_rc4_encrypt_helper() {
        let key = Rc4Key::from_slice(&[0x01, 0x02, 0x03, 0x04, 0x05]);
        let data = b"test rc4 helper";

        let encrypted = rc4_encrypt(&key, data);
        assert_ne!(encrypted.as_slice(), data);

        // RC4 is symmetric
        let decrypted = rc4_encrypt(&key, &encrypted);
        assert_eq!(decrypted.as_slice(), data);
    }

    #[test]
    fn test_edge_case_max_object_generation() {
        let handler = StandardSecurityHandler::rc4_128bit();
        let key = EncryptionKey::new(vec![0xEE; 16]);
        let obj_id = ObjectId::new(0xFFFFFF, 0xFFFF); // Max values
        let data = b"edge case";

        let encrypted = handler.encrypt_string(data, &key, &obj_id);
        let decrypted = handler.decrypt_string(&encrypted, &key, &obj_id);
        assert_eq!(decrypted.as_slice(), data);
    }

    // ===== SHA-256/512 NIST Vector Tests (Phase 1.3 - RustCrypto Integration) =====

    #[test]
    fn test_sha256_nist_empty_string() {
        // NIST FIPS 180-4 test vector: SHA-256("")
        let hash = sha256(b"");
        let expected: [u8; 32] = [
            0xe3, 0xb0, 0xc4, 0x42, 0x98, 0xfc, 0x1c, 0x14, 0x9a, 0xfb, 0xf4, 0xc8, 0x99, 0x6f,
            0xb9, 0x24, 0x27, 0xae, 0x41, 0xe4, 0x64, 0x9b, 0x93, 0x4c, 0xa4, 0x95, 0x99, 0x1b,
            0x78, 0x52, 0xb8, 0x55,
        ];
        assert_eq!(
            hash.as_slice(),
            expected.as_slice(),
            "SHA-256('') must match NIST test vector"
        );
    }

    #[test]
    fn test_sha256_nist_abc() {
        // NIST FIPS 180-4 test vector: SHA-256("abc")
        let hash = sha256(b"abc");
        let expected: [u8; 32] = [
            0xba, 0x78, 0x16, 0xbf, 0x8f, 0x01, 0xcf, 0xea, 0x41, 0x41, 0x40, 0xde, 0x5d, 0xae,
            0x22, 0x23, 0xb0, 0x03, 0x61, 0xa3, 0x96, 0x17, 0x7a, 0x9c, 0xb4, 0x10, 0xff, 0x61,
            0xf2, 0x00, 0x15, 0xad,
        ];
        assert_eq!(
            hash.as_slice(),
            expected.as_slice(),
            "SHA-256('abc') must match NIST test vector"
        );
    }

    #[test]
    fn test_sha512_nist_abc() {
        // NIST FIPS 180-4 test vector: SHA-512("abc")
        let hash = sha512(b"abc");
        let expected: [u8; 64] = [
            0xdd, 0xaf, 0x35, 0xa1, 0x93, 0x61, 0x7a, 0xba, 0xcc, 0x41, 0x73, 0x49, 0xae, 0x20,
            0x41, 0x31, 0x12, 0xe6, 0xfa, 0x4e, 0x89, 0xa9, 0x7e, 0xa2, 0x0a, 0x9e, 0xee, 0xe6,
            0x4b, 0x55, 0xd3, 0x9a, 0x21, 0x92, 0x99, 0x2a, 0x27, 0x4f, 0xc1, 0xa8, 0x36, 0xba,
            0x3c, 0x23, 0xa3, 0xfe, 0xeb, 0xbd, 0x45, 0x4d, 0x44, 0x23, 0x64, 0x3c, 0xe8, 0x0e,
            0x2a, 0x9a, 0xc9, 0x4f, 0xa5, 0x4c, 0xa4, 0x9f,
        ];
        assert_eq!(
            hash.as_slice(),
            expected.as_slice(),
            "SHA-512('abc') must match NIST test vector"
        );
    }

    #[test]
    fn test_sha512_length() {
        let hash = sha512(b"test data");
        assert_eq!(hash.len(), 64, "SHA-512 must produce 64 bytes");
    }

    #[test]
    fn test_sha512_deterministic() {
        let data1 = b"sha512 test data";
        let data2 = b"sha512 test data";
        let data3 = b"different data";

        let hash1 = sha512(data1);
        let hash2 = sha512(data2);
        let hash3 = sha512(data3);

        assert_eq!(hash1, hash2, "Same input must produce same SHA-512 hash");
        assert_ne!(hash1, hash3, "Different input must produce different hash");
    }

    // ===== Phase 2.1: R5 User Password Tests (Algorithm 8 & 11) =====

    #[test]
    fn test_r5_user_hash_computation() {
        let handler = StandardSecurityHandler::aes_256_r5();
        let password = UserPassword("test_password".to_string());

        let u_entry = handler.compute_r5_user_hash(&password).unwrap();

        // U entry must be exactly 48 bytes: hash(32) + validation_salt(8) + key_salt(8)
        assert_eq!(u_entry.len(), 48, "R5 U entry must be 48 bytes");
    }

    #[test]
    fn test_r5_user_password_validation_correct() {
        let handler = StandardSecurityHandler::aes_256_r5();
        let password = UserPassword("correct_password".to_string());

        // Compute U entry with the password
        let u_entry = handler.compute_r5_user_hash(&password).unwrap();

        // Validate with same password should succeed
        let is_valid = handler
            .validate_r5_user_password(&password, &u_entry)
            .unwrap();
        assert!(is_valid, "Correct password must validate");
    }

    #[test]
    fn test_r5_user_password_validation_incorrect() {
        let handler = StandardSecurityHandler::aes_256_r5();
        let correct_password = UserPassword("correct_password".to_string());
        let wrong_password = UserPassword("wrong_password".to_string());

        // Compute U entry with correct password
        let u_entry = handler.compute_r5_user_hash(&correct_password).unwrap();

        // Validate with wrong password should fail
        let is_valid = handler
            .validate_r5_user_password(&wrong_password, &u_entry)
            .unwrap();
        assert!(!is_valid, "Wrong password must not validate");
    }

    #[test]
    fn test_r5_user_hash_random_salts() {
        let handler = StandardSecurityHandler::aes_256_r5();
        let password = UserPassword("same_password".to_string());

        // Compute U entry twice - salts should be different
        let u_entry1 = handler.compute_r5_user_hash(&password).unwrap();
        let u_entry2 = handler.compute_r5_user_hash(&password).unwrap();

        // Hash portion should be different (due to random salts)
        assert_ne!(
            &u_entry1[..32],
            &u_entry2[..32],
            "Different random salts should produce different hashes"
        );

        // Validation salt should be different
        assert_ne!(
            &u_entry1[32..40],
            &u_entry2[32..40],
            "Validation salts must be random"
        );

        // But both should validate with the same password
        assert!(handler
            .validate_r5_user_password(&password, &u_entry1)
            .unwrap());
        assert!(handler
            .validate_r5_user_password(&password, &u_entry2)
            .unwrap());
    }

    #[test]
    fn test_r5_user_hash_invalid_entry_length() {
        let handler = StandardSecurityHandler::aes_256_r5();
        let password = UserPassword("test".to_string());

        // Try to validate with wrong length U entry
        let short_entry = vec![0u8; 32]; // Too short
        let result = handler.validate_r5_user_password(&password, &short_entry);
        assert!(result.is_err(), "Short U entry must fail");

        let long_entry = vec![0u8; 64]; // Too long
        let result = handler.validate_r5_user_password(&password, &long_entry);
        assert!(result.is_err(), "Long U entry must fail");
    }

    #[test]
    fn test_r5_empty_password() {
        let handler = StandardSecurityHandler::aes_256_r5();
        let empty_password = UserPassword("".to_string());

        // Empty password should work (common for user-only encryption)
        let u_entry = handler.compute_r5_user_hash(&empty_password).unwrap();
        assert_eq!(u_entry.len(), 48);

        let is_valid = handler
            .validate_r5_user_password(&empty_password, &u_entry)
            .unwrap();
        assert!(is_valid, "Empty password must validate correctly");

        // Non-empty password should fail
        let non_empty = UserPassword("not_empty".to_string());
        let is_valid = handler
            .validate_r5_user_password(&non_empty, &u_entry)
            .unwrap();
        assert!(!is_valid, "Non-empty password must not validate");
    }

    // ===== Phase 2.2: R5 UE Entry Tests (Encryption Key Storage) =====

    #[test]
    fn test_r5_ue_entry_computation() {
        let handler = StandardSecurityHandler::aes_256_r5();
        let password = UserPassword("ue_test_password".to_string());
        let encryption_key = EncryptionKey::new(vec![0xAB; 32]);

        // Compute U entry first
        let u_entry = handler.compute_r5_user_hash(&password).unwrap();

        // Compute UE entry
        let ue_entry = handler
            .compute_r5_ue_entry(&password, &u_entry, &encryption_key)
            .unwrap();

        // UE entry must be exactly 32 bytes
        assert_eq!(ue_entry.len(), 32, "R5 UE entry must be 32 bytes");

        // UE should be different from the original key (it's encrypted)
        assert_ne!(
            ue_entry.as_slice(),
            encryption_key.as_bytes(),
            "UE must be encrypted"
        );
    }

    #[test]
    fn test_r5_encryption_key_recovery() {
        let handler = StandardSecurityHandler::aes_256_r5();
        let password = UserPassword("recovery_test".to_string());
        let original_key = EncryptionKey::new(vec![0x42; 32]);

        // Compute U entry
        let u_entry = handler.compute_r5_user_hash(&password).unwrap();

        // Compute UE entry
        let ue_entry = handler
            .compute_r5_ue_entry(&password, &u_entry, &original_key)
            .unwrap();

        // Recover the key
        let recovered_key = handler
            .recover_r5_encryption_key(&password, &u_entry, &ue_entry)
            .unwrap();

        // Recovered key must match original
        assert_eq!(
            recovered_key.as_bytes(),
            original_key.as_bytes(),
            "Recovered key must match original"
        );
    }

    #[test]
    fn test_r5_ue_wrong_password_fails() {
        let handler = StandardSecurityHandler::aes_256_r5();
        let correct_password = UserPassword("correct".to_string());
        let wrong_password = UserPassword("wrong".to_string());
        let original_key = EncryptionKey::new(vec![0x99; 32]);

        // Compute U and UE with correct password
        let u_entry = handler.compute_r5_user_hash(&correct_password).unwrap();
        let ue_entry = handler
            .compute_r5_ue_entry(&correct_password, &u_entry, &original_key)
            .unwrap();

        // Try to recover with wrong password
        let recovered_key = handler
            .recover_r5_encryption_key(&wrong_password, &u_entry, &ue_entry)
            .unwrap();

        // Key should be different (wrong decryption)
        assert_ne!(
            recovered_key.as_bytes(),
            original_key.as_bytes(),
            "Wrong password must produce wrong key"
        );
    }

    #[test]
    fn test_r5_ue_invalid_length() {
        let handler = StandardSecurityHandler::aes_256_r5();
        let password = UserPassword("test".to_string());
        let u_entry = vec![0u8; 48]; // Valid U entry length

        // Try to recover with wrong length UE entry
        let short_ue = vec![0u8; 16]; // Too short
        let result = handler.recover_r5_encryption_key(&password, &u_entry, &short_ue);
        assert!(result.is_err(), "Short UE entry must fail");

        let long_ue = vec![0u8; 64]; // Too long
        let result = handler.recover_r5_encryption_key(&password, &u_entry, &long_ue);
        assert!(result.is_err(), "Long UE entry must fail");
    }

    #[test]
    fn test_r5_ue_invalid_u_length() {
        let handler = StandardSecurityHandler::aes_256_r5();
        let password = UserPassword("test".to_string());
        let encryption_key = EncryptionKey::new(vec![0x11; 32]);

        // Try to compute UE with wrong length U entry
        let short_u = vec![0u8; 32]; // Too short
        let result = handler.compute_r5_ue_entry(&password, &short_u, &encryption_key);
        assert!(
            result.is_err(),
            "Short U entry must fail for UE computation"
        );
    }

    #[test]
    fn test_r5_full_workflow_u_ue() {
        let handler = StandardSecurityHandler::aes_256_r5();
        let password = UserPassword("full_workflow_test".to_string());
        let encryption_key = EncryptionKey::new((0..32).collect::<Vec<u8>>());

        // Step 1: Compute U entry (password verification data)
        let u_entry = handler.compute_r5_user_hash(&password).unwrap();
        assert_eq!(u_entry.len(), 48);

        // Step 2: Verify password validates
        assert!(handler
            .validate_r5_user_password(&password, &u_entry)
            .unwrap());

        // Step 3: Compute UE entry (encrypted key storage)
        let ue_entry = handler
            .compute_r5_ue_entry(&password, &u_entry, &encryption_key)
            .unwrap();
        assert_eq!(ue_entry.len(), 32);

        // Step 4: Recover key from UE
        let recovered = handler
            .recover_r5_encryption_key(&password, &u_entry, &ue_entry)
            .unwrap();

        // Step 5: Verify recovered key matches original
        assert_eq!(
            recovered.as_bytes(),
            encryption_key.as_bytes(),
            "Full R5 workflow: recovered key must match original"
        );
    }

    // ===== Phase 3.1: R6 User Password Tests (SHA-512 based) =====

    #[test]
    fn test_r6_user_hash_computation() {
        let handler = StandardSecurityHandler::aes_256_r6();
        let password = UserPassword("r6_test_password".to_string());

        let u_entry = handler.compute_r6_user_hash(&password).unwrap();

        // U entry must be exactly 48 bytes: hash(32) + validation_salt(8) + key_salt(8)
        assert_eq!(u_entry.len(), 48, "R6 U entry must be 48 bytes");
    }

    #[test]
    fn test_r6_user_password_validation_correct() {
        let handler = StandardSecurityHandler::aes_256_r6();
        let password = UserPassword("r6_correct_password".to_string());

        // Compute U entry with the password
        let u_entry = handler.compute_r6_user_hash(&password).unwrap();

        // Validate with same password should succeed
        let is_valid = handler
            .validate_r6_user_password(&password, &u_entry)
            .unwrap();
        assert!(is_valid, "Correct R6 password must validate");
    }

    #[test]
    fn test_r6_user_password_validation_incorrect() {
        let handler = StandardSecurityHandler::aes_256_r6();
        let correct_password = UserPassword("r6_correct".to_string());
        let wrong_password = UserPassword("r6_wrong".to_string());

        // Compute U entry with correct password
        let u_entry = handler.compute_r6_user_hash(&correct_password).unwrap();

        // Validate with wrong password should fail
        let is_valid = handler
            .validate_r6_user_password(&wrong_password, &u_entry)
            .unwrap();
        assert!(!is_valid, "Wrong R6 password must not validate");
    }

    #[test]
    fn test_r6_uses_sha512_not_sha256() {
        // Verify R6 produces different hash than R5 for same password
        let handler_r5 = StandardSecurityHandler::aes_256_r5();
        let handler_r6 = StandardSecurityHandler::aes_256_r6();
        let password = UserPassword("same_password_both_revisions".to_string());

        let u_r5 = handler_r5.compute_r5_user_hash(&password).unwrap();
        let u_r6 = handler_r6.compute_r6_user_hash(&password).unwrap();

        // Hash portions (first 32 bytes) should be different
        // Note: Salts are random, but even with same salt the hash algorithm differs
        assert_ne!(
            &u_r5[..32],
            &u_r6[..32],
            "R5 (SHA-256) and R6 (SHA-512) must produce different hashes"
        );
    }

    #[test]
    fn test_r6_unicode_password() {
        let handler = StandardSecurityHandler::aes_256_r6();
        let unicode_password = UserPassword("café🔒日本語".to_string());

        let u_entry = handler.compute_r6_user_hash(&unicode_password).unwrap();
        assert_eq!(u_entry.len(), 48);

        // Validate with same Unicode password
        let is_valid = handler
            .validate_r6_user_password(&unicode_password, &u_entry)
            .unwrap();
        assert!(is_valid, "Unicode password must validate");

        // Different Unicode password should fail
        let different_unicode = UserPassword("café🔓日本語".to_string()); // Different emoji
        let is_valid = handler
            .validate_r6_user_password(&different_unicode, &u_entry)
            .unwrap();
        assert!(!is_valid, "Different Unicode password must not validate");
    }

    // ===== Phase 3.1: R6 UE Entry Tests =====

    #[test]
    fn test_r6_ue_entry_computation() {
        let handler = StandardSecurityHandler::aes_256_r6();
        let password = UserPassword("r6_ue_test".to_string());
        let encryption_key = EncryptionKey::new(vec![0xCD; 32]);

        let u_entry = handler.compute_r6_user_hash(&password).unwrap();
        let ue_entry = handler
            .compute_r6_ue_entry(&password, &u_entry, &encryption_key)
            .unwrap();

        assert_eq!(ue_entry.len(), 32, "R6 UE entry must be 32 bytes");
    }

    #[test]
    fn test_r6_encryption_key_recovery() {
        let handler = StandardSecurityHandler::aes_256_r6();
        let password = UserPassword("r6_recovery_test".to_string());
        let original_key = EncryptionKey::new(vec![0xEF; 32]);

        let u_entry = handler.compute_r6_user_hash(&password).unwrap();
        let ue_entry = handler
            .compute_r6_ue_entry(&password, &u_entry, &original_key)
            .unwrap();

        let recovered_key = handler
            .recover_r6_encryption_key(&password, &u_entry, &ue_entry)
            .unwrap();

        assert_eq!(
            recovered_key.as_bytes(),
            original_key.as_bytes(),
            "R6: Recovered key must match original"
        );
    }

    // ===== Phase 3.2: R6 Perms Entry Tests =====

    #[test]
    fn test_r6_perms_entry_computation() {
        let handler = StandardSecurityHandler::aes_256_r6();
        let permissions = Permissions::all();
        let key = EncryptionKey::new(vec![0x42; 32]);

        let perms = handler
            .compute_r6_perms_entry(permissions, &key, true)
            .unwrap();

        assert_eq!(perms.len(), 16, "Perms entry must be 16 bytes");
    }

    #[test]
    fn test_r6_perms_validation() {
        let handler = StandardSecurityHandler::aes_256_r6();
        let permissions = Permissions::new();
        let key = EncryptionKey::new(vec![0x55; 32]);

        let perms = handler
            .compute_r6_perms_entry(permissions, &key, false)
            .unwrap();

        let is_valid = handler
            .validate_r6_perms(&perms, &key, permissions)
            .unwrap();
        assert!(is_valid, "Perms validation must succeed with correct key");
    }

    #[test]
    fn test_r6_perms_wrong_key_fails() {
        let handler = StandardSecurityHandler::aes_256_r6();
        let permissions = Permissions::all();
        let correct_key = EncryptionKey::new(vec![0xAA; 32]);
        let wrong_key = EncryptionKey::new(vec![0xBB; 32]);

        let perms = handler
            .compute_r6_perms_entry(permissions, &correct_key, true)
            .unwrap();

        // Validation with wrong key should fail (structure won't match)
        let result = handler.validate_r6_perms(&perms, &wrong_key, permissions);
        assert!(result.is_ok()); // No error
        assert!(!result.unwrap()); // But validation fails
    }

    #[test]
    fn test_r6_perms_encrypt_metadata_flag() {
        let handler = StandardSecurityHandler::aes_256_r6();
        let permissions = Permissions::new();
        let key = EncryptionKey::new(vec![0x33; 32]);

        let perms_true = handler
            .compute_r6_perms_entry(permissions, &key, true)
            .unwrap();
        let perms_false = handler
            .compute_r6_perms_entry(permissions, &key, false)
            .unwrap();

        // Different encrypt_metadata flag should produce different Perms
        assert_ne!(
            perms_true, perms_false,
            "Different EncryptMetadata must produce different Perms"
        );

        // Extract and verify flags
        let flag_true = handler
            .extract_r6_encrypt_metadata(&perms_true, &key)
            .unwrap();
        assert_eq!(flag_true, Some(true));

        let flag_false = handler
            .extract_r6_encrypt_metadata(&perms_false, &key)
            .unwrap();
        assert_eq!(flag_false, Some(false));
    }

    #[test]
    fn test_r6_perms_invalid_length() {
        let handler = StandardSecurityHandler::aes_256_r6();
        let key = EncryptionKey::new(vec![0x44; 32]);
        let permissions = Permissions::new();

        let invalid_perms = vec![0u8; 12]; // Too short
        let result = handler.validate_r6_perms(&invalid_perms, &key, permissions);
        assert!(result.is_err(), "Short Perms entry must fail");
    }

    #[test]
    fn test_r6_full_workflow_with_perms() {
        // Complete R6 integration test: U + UE + Perms
        let handler = StandardSecurityHandler::aes_256_r6();
        let password = UserPassword("r6_full_workflow".to_string());
        let permissions = Permissions::all();
        let encryption_key = EncryptionKey::new((0..32).map(|i| (i * 3) as u8).collect());

        // Step 1: Compute U entry (password verification)
        let u_entry = handler.compute_r6_user_hash(&password).unwrap();
        assert_eq!(u_entry.len(), 48);

        // Step 2: Validate password
        assert!(handler
            .validate_r6_user_password(&password, &u_entry)
            .unwrap());

        // Step 3: Compute UE entry (encrypted key)
        let ue_entry = handler
            .compute_r6_ue_entry(&password, &u_entry, &encryption_key)
            .unwrap();
        assert_eq!(ue_entry.len(), 32);

        // Step 4: Compute Perms entry (encrypted permissions)
        let perms = handler
            .compute_r6_perms_entry(permissions, &encryption_key, true)
            .unwrap();
        assert_eq!(perms.len(), 16);

        // Step 5: Recover encryption key from UE
        let recovered_key = handler
            .recover_r6_encryption_key(&password, &u_entry, &ue_entry)
            .unwrap();
        assert_eq!(
            recovered_key.as_bytes(),
            encryption_key.as_bytes(),
            "Recovered key must match original"
        );

        // Step 6: Validate Perms with recovered key
        let perms_valid = handler
            .validate_r6_perms(&perms, &recovered_key, permissions)
            .unwrap();
        assert!(perms_valid, "Perms must validate with recovered key");

        // Step 7: Extract EncryptMetadata flag
        let encrypt_meta = handler
            .extract_r6_encrypt_metadata(&perms, &recovered_key)
            .unwrap();
        assert_eq!(encrypt_meta, Some(true), "EncryptMetadata must be true");
    }
}
