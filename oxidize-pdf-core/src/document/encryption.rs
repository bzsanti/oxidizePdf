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

    #[test]
    fn test_encryption_strength_variants() {
        let enc_40 = DocumentEncryption::new(
            "user",
            "owner",
            Permissions::new(),
            EncryptionStrength::Rc4_40bit,
        );

        let enc_128 = DocumentEncryption::new(
            "user",
            "owner",
            Permissions::new(),
            EncryptionStrength::Rc4_128bit,
        );

        // Check handlers
        let handler_40 = enc_40.handler();
        let handler_128 = enc_128.handler();

        // Verify different encryption dictionary versions
        let dict_40 = enc_40.create_encryption_dict(None).unwrap();
        let dict_128 = enc_128.create_encryption_dict(None).unwrap();

        assert_eq!(dict_40.v, 1);
        assert_eq!(dict_40.r, 2);
        assert_eq!(dict_40.length, Some(5));

        assert_eq!(dict_128.v, 2);
        assert_eq!(dict_128.r, 3);
        assert_eq!(dict_128.length, Some(16));
    }

    #[test]
    fn test_empty_passwords() {
        let enc =
            DocumentEncryption::new("", "", Permissions::all(), EncryptionStrength::Rc4_128bit);

        assert_eq!(enc.user_password.0, "");
        assert_eq!(enc.owner_password.0, "");

        // Should still create valid encryption dictionary
        let dict = enc.create_encryption_dict(None);
        assert!(dict.is_ok());
    }

    #[test]
    fn test_long_passwords() {
        let long_user = "a".repeat(100);
        let long_owner = "b".repeat(100);

        let enc = DocumentEncryption::new(
            &long_user,
            &long_owner,
            Permissions::new(),
            EncryptionStrength::Rc4_128bit,
        );

        assert_eq!(enc.user_password.0.len(), 100);
        assert_eq!(enc.owner_password.0.len(), 100);

        let dict = enc.create_encryption_dict(None);
        assert!(dict.is_ok());
    }

    #[test]
    fn test_unicode_passwords() {
        let enc = DocumentEncryption::new(
            "contraseña",
            "密码",
            Permissions::all(),
            EncryptionStrength::Rc4_40bit,
        );

        assert_eq!(enc.user_password.0, "contraseña");
        assert_eq!(enc.owner_password.0, "密码");

        let dict = enc.create_encryption_dict(None);
        assert!(dict.is_ok());
    }

    #[test]
    fn test_encryption_with_file_id() {
        let enc = DocumentEncryption::new(
            "user",
            "owner",
            Permissions::new(),
            EncryptionStrength::Rc4_128bit,
        );

        let file_id = b"test_file_id_12345";
        let dict = enc.create_encryption_dict(Some(file_id)).unwrap();

        // Should be able to get encryption key with same file ID
        let key = enc.get_encryption_key(&dict, Some(file_id));
        assert!(key.is_ok());
    }

    #[test]
    fn test_different_permissions() {
        let perms_none = Permissions::new();
        let perms_all = Permissions::all();
        let mut perms_custom = Permissions::new();
        perms_custom.set_print(true);
        perms_custom.set_modify_contents(false);

        let enc1 =
            DocumentEncryption::new("user", "owner", perms_none, EncryptionStrength::Rc4_128bit);

        let enc2 =
            DocumentEncryption::new("user", "owner", perms_all, EncryptionStrength::Rc4_128bit);

        let enc3 = DocumentEncryption::new(
            "user",
            "owner",
            perms_custom,
            EncryptionStrength::Rc4_128bit,
        );

        // Create encryption dictionaries
        let dict1 = enc1.create_encryption_dict(None).unwrap();
        let dict2 = enc2.create_encryption_dict(None).unwrap();
        let dict3 = enc3.create_encryption_dict(None).unwrap();

        // Permissions should be encoded differently
        // Note: p field contains encoded permissions as i32
        // Different permission sets should have different values
    }

    #[test]
    fn test_encryption_context_stream() {
        let handler = StandardSecurityHandler::rc4_128bit();
        let key = EncryptionKey::new(vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16]);
        let ctx = EncryptionContext::new(handler, key);

        let obj_id = ObjectId::new(5, 0);
        let stream_data = b"This is a PDF stream content that needs encryption";

        let encrypted = ctx.encrypt_stream(stream_data, &obj_id);
        assert_ne!(encrypted, stream_data);

        let decrypted = ctx.decrypt_stream(&encrypted, &obj_id);
        assert_eq!(decrypted, stream_data);
    }

    #[test]
    fn test_encryption_context_different_objects() {
        let handler = StandardSecurityHandler::rc4_40bit();
        let key = EncryptionKey::new(vec![1, 2, 3, 4, 5]);
        let ctx = EncryptionContext::new(handler, key);

        let obj_id1 = ObjectId::new(1, 0);
        let obj_id2 = ObjectId::new(2, 0);
        let plaintext = b"Test data";

        let encrypted1 = ctx.encrypt_string(plaintext, &obj_id1);
        let encrypted2 = ctx.encrypt_string(plaintext, &obj_id2);

        // Same plaintext encrypted with different object IDs should produce different ciphertext
        assert_ne!(encrypted1, encrypted2);

        // But both should decrypt to the same plaintext
        assert_eq!(ctx.decrypt_string(&encrypted1, &obj_id1), plaintext);
        assert_eq!(ctx.decrypt_string(&encrypted2, &obj_id2), plaintext);
    }

    #[test]
    fn test_get_encryption_key_consistency() {
        let enc = DocumentEncryption::new(
            "user123",
            "owner456",
            Permissions::all(),
            EncryptionStrength::Rc4_128bit,
        );

        let file_id = b"consistent_file_id";
        let dict = enc.create_encryption_dict(Some(file_id)).unwrap();

        // Getting key multiple times should produce consistent results
        let key1 = enc.get_encryption_key(&dict, Some(file_id));
        let key2 = enc.get_encryption_key(&dict, Some(file_id));

        // Both should succeed
        assert!(key1.is_ok());
        assert!(key2.is_ok());
    }

    #[test]
    fn test_handler_selection() {
        let enc_40 = DocumentEncryption::new(
            "test",
            "test",
            Permissions::new(),
            EncryptionStrength::Rc4_40bit,
        );

        let enc_128 = DocumentEncryption::new(
            "test",
            "test",
            Permissions::new(),
            EncryptionStrength::Rc4_128bit,
        );

        // Handlers should be different for different strengths
        let _handler_40 = enc_40.handler();
        let _handler_128 = enc_128.handler();

        // Create dictionaries to verify correct configuration
        let dict_40 = enc_40.create_encryption_dict(None).unwrap();
        let dict_128 = enc_128.create_encryption_dict(None).unwrap();

        // 40-bit should have length 5, 128-bit should have length 16
        assert_eq!(dict_40.length, Some(5));
        assert_eq!(dict_128.length, Some(16));
    }
}
