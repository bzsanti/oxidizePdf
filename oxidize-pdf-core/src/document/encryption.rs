//! Document encryption support

use crate::encryption::{
    EncryptionDictionary, EncryptionKey, OwnerPassword, Permissions, StandardSecurityHandler,
    UserPassword,
};
use crate::error::Result;
use crate::objects::ObjectId;

/// Encryption settings for a document
#[derive(Debug, Clone)]
pub struct DocumentEncryption {
    /// User password
    pub user_password: UserPassword,
    /// Owner password
    pub owner_password: OwnerPassword,
    /// Permissions
    pub permissions: Permissions,
    /// Encryption strength
    pub strength: EncryptionStrength,
}

/// Encryption strength
#[derive(Debug, Clone, Copy)]
pub enum EncryptionStrength {
    /// RC4 40-bit encryption
    Rc4_40bit,
    /// RC4 128-bit encryption
    Rc4_128bit,
}

impl DocumentEncryption {
    /// Create new encryption settings
    pub fn new(
        user_password: impl Into<String>,
        owner_password: impl Into<String>,
        permissions: Permissions,
        strength: EncryptionStrength,
    ) -> Self {
        Self {
            user_password: UserPassword(user_password.into()),
            owner_password: OwnerPassword(owner_password.into()),
            permissions,
            strength,
        }
    }

    /// Create with default permissions (all allowed)
    pub fn with_passwords(
        user_password: impl Into<String>,
        owner_password: impl Into<String>,
    ) -> Self {
        Self::new(
            user_password,
            owner_password,
            Permissions::all(),
            EncryptionStrength::Rc4_128bit,
        )
    }

    /// Get the security handler
    pub fn handler(&self) -> StandardSecurityHandler {
        match self.strength {
            EncryptionStrength::Rc4_40bit => StandardSecurityHandler::rc4_40bit(),
            EncryptionStrength::Rc4_128bit => StandardSecurityHandler::rc4_128bit(),
        }
    }

    /// Create encryption dictionary
    pub fn create_encryption_dict(&self, file_id: Option<&[u8]>) -> Result<EncryptionDictionary> {
        let handler = self.handler();

        // Compute password hashes
        let owner_hash = handler.compute_owner_hash(&self.owner_password, &self.user_password);
        let user_hash = handler.compute_user_hash(
            &self.user_password,
            &owner_hash,
            self.permissions,
            file_id,
        )?;

        // Create encryption dictionary
        let enc_dict = match self.strength {
            EncryptionStrength::Rc4_40bit => EncryptionDictionary::rc4_40bit(
                owner_hash,
                user_hash,
                self.permissions,
                file_id.map(|id| id.to_vec()),
            ),
            EncryptionStrength::Rc4_128bit => EncryptionDictionary::rc4_128bit(
                owner_hash,
                user_hash,
                self.permissions,
                file_id.map(|id| id.to_vec()),
            ),
        };

        Ok(enc_dict)
    }

    /// Get encryption key
    pub fn get_encryption_key(
        &self,
        enc_dict: &EncryptionDictionary,
        file_id: Option<&[u8]>,
    ) -> Result<EncryptionKey> {
        let handler = self.handler();
        handler.compute_encryption_key(&self.user_password, &enc_dict.o, self.permissions, file_id)
    }
}

/// Encryption context for encrypting objects
#[allow(dead_code)]
pub struct EncryptionContext {
    /// Security handler
    handler: StandardSecurityHandler,
    /// Encryption key
    key: EncryptionKey,
}

#[allow(dead_code)]
impl EncryptionContext {
    /// Create new encryption context
    pub fn new(handler: StandardSecurityHandler, key: EncryptionKey) -> Self {
        Self { handler, key }
    }

    /// Encrypt a string
    pub fn encrypt_string(&self, data: &[u8], obj_id: &ObjectId) -> Vec<u8> {
        self.handler.encrypt_string(data, &self.key, obj_id)
    }

    /// Decrypt a string
    pub fn decrypt_string(&self, data: &[u8], obj_id: &ObjectId) -> Vec<u8> {
        self.handler.decrypt_string(data, &self.key, obj_id)
    }

    /// Encrypt a stream
    pub fn encrypt_stream(&self, data: &[u8], obj_id: &ObjectId) -> Vec<u8> {
        self.handler.encrypt_stream(data, &self.key, obj_id)
    }

    /// Decrypt a stream
    pub fn decrypt_stream(&self, data: &[u8], obj_id: &ObjectId) -> Vec<u8> {
        self.handler.decrypt_stream(data, &self.key, obj_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_document_encryption_new() {
        let enc = DocumentEncryption::new(
            "user123",
            "owner456",
            Permissions::all(),
            EncryptionStrength::Rc4_128bit,
        );

        assert_eq!(enc.user_password.0, "user123");
        assert_eq!(enc.owner_password.0, "owner456");
    }

    #[test]
    fn test_with_passwords() {
        let enc = DocumentEncryption::with_passwords("user", "owner");
        assert_eq!(enc.user_password.0, "user");
        assert_eq!(enc.owner_password.0, "owner");
        assert!(enc.permissions.can_print());
        assert!(enc.permissions.can_modify_contents());
    }

    #[test]
    fn test_encryption_dict_creation() {
        let enc = DocumentEncryption::new(
            "test",
            "owner",
            Permissions::new(),
            EncryptionStrength::Rc4_40bit,
        );

        let enc_dict = enc.create_encryption_dict(None).unwrap();
        assert_eq!(enc_dict.v, 1);
        assert_eq!(enc_dict.r, 2);
        assert_eq!(enc_dict.length, Some(5));
    }

    #[test]
    fn test_encryption_context() {
        let handler = StandardSecurityHandler::rc4_40bit();
        let key = EncryptionKey::new(vec![1, 2, 3, 4, 5]);
        let ctx = EncryptionContext::new(handler, key);

        let obj_id = ObjectId::new(1, 0);
        let plaintext = b"Hello, World!";

        let encrypted = ctx.encrypt_string(plaintext, &obj_id);
        assert_ne!(encrypted, plaintext);

        let decrypted = ctx.decrypt_string(&encrypted, &obj_id);
        assert_eq!(decrypted, plaintext);
    }
}
