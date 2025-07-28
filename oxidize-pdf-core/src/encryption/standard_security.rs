//! Standard Security Handler implementation according to ISO 32000-1

#![allow(clippy::needless_range_loop)]

use crate::encryption::{Permissions, Rc4, Rc4Key};
use crate::error::Result;
use crate::objects::ObjectId;

/// Padding used in password processing
const PADDING: [u8; 32] = [
    0x28, 0xBF, 0x4E, 0x5E, 0x4E, 0x75, 0x8A, 0x41, 0x64, 0x00, 0x4E, 0x56, 0xFF, 0xFA, 0x01, 0x08,
    0x2E, 0x2E, 0x00, 0xB6, 0xD0, 0x68, 0x3E, 0x80, 0x2F, 0x0C, 0xA9, 0xFE, 0x64, 0x53, 0x69, 0x7A,
];

/// User password
#[derive(Debug, Clone)]
pub struct UserPassword(pub String);

/// Owner password
#[derive(Debug, Clone)]
pub struct OwnerPassword(pub String);

/// Encryption key
#[derive(Debug, Clone)]
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
}

/// Standard Security Handler
pub struct StandardSecurityHandler {
    /// Revision
    revision: SecurityHandlerRevision,
    /// Key length in bytes
    key_length: usize,
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
                let mut result = rc4_encrypt(&rc4_key, &hash);

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

        // For R4 with metadata not encrypted, add extra bytes
        if self.revision == SecurityHandlerRevision::R4 {
            // In a full implementation, check EncryptMetadata flag
            // For now, assume metadata is encrypted
        }

        // Step 3: Create MD5 hash
        let mut hash = md5::compute(&data).to_vec();

        // Step 4: For revision 3+, do 50 additional iterations
        if self.revision >= SecurityHandlerRevision::R3 {
            for _ in 0..50 {
                hash = md5::compute(&hash[..self.key_length]).to_vec();
            }
        }

        // Step 5: Truncate to key length
        hash.truncate(self.key_length);

        Ok(EncryptionKey::new(hash))
    }

    /// Encrypt a string
    pub fn encrypt_string(&self, data: &[u8], key: &EncryptionKey, obj_id: &ObjectId) -> Vec<u8> {
        let obj_key = self.compute_object_key(key, obj_id);
        let rc4_key = Rc4Key::from_slice(&obj_key);
        rc4_encrypt(&rc4_key, data)
    }

    /// Decrypt a string
    pub fn decrypt_string(&self, data: &[u8], key: &EncryptionKey, obj_id: &ObjectId) -> Vec<u8> {
        // RC4 is symmetric
        self.encrypt_string(data, key, obj_id)
    }

    /// Encrypt a stream
    pub fn encrypt_stream(&self, data: &[u8], key: &EncryptionKey, obj_id: &ObjectId) -> Vec<u8> {
        self.encrypt_string(data, key, obj_id)
    }

    /// Decrypt a stream
    pub fn decrypt_stream(&self, data: &[u8], key: &EncryptionKey, obj_id: &ObjectId) -> Vec<u8> {
        self.decrypt_string(data, key, obj_id)
    }

    /// Compute object-specific encryption key
    fn compute_object_key(&self, key: &EncryptionKey, obj_id: &ObjectId) -> Vec<u8> {
        let mut data = Vec::new();
        data.extend_from_slice(&key.key);
        data.extend_from_slice(&obj_id.number().to_le_bytes()[..3]); // Low 3 bytes
        data.extend_from_slice(&obj_id.generation().to_le_bytes()[..2]); // Low 2 bytes

        let hash = md5::compute(&data);
        let key_len = (key.len() + 5).min(16);
        hash[..key_len].to_vec()
    }
}

/// Helper function for RC4 encryption
fn rc4_encrypt(key: &Rc4Key, data: &[u8]) -> Vec<u8> {
    let mut cipher = Rc4::new(key);
    cipher.process(data)
}

/// MD5 module (simplified for example)
mod md5 {

    pub fn compute(data: &[u8]) -> [u8; 16] {
        // In production, use a proper MD5 implementation
        // This is a placeholder that uses a hash function
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        data.hash(&mut hasher);
        let hash_value = hasher.finish();

        let mut result = [0u8; 16];
        result[..8].copy_from_slice(&hash_value.to_le_bytes());
        result[8..].copy_from_slice(&hash_value.to_be_bytes());
        result
    }
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
}
