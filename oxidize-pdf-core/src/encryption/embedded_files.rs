//! Embedded files encryption support for PDF
//!
//! This module implements encryption control for embedded files and metadata
//! according to ISO 32000-1:2008 §7.6.5.

use crate::encryption::{CryptFilterManager, EncryptionKey};
use crate::error::Result;
use crate::objects::{Dictionary, Object, ObjectId};
use std::sync::Arc;

/// Embedded file encryption handler
pub struct EmbeddedFileEncryption {
    /// Filter name for embedded files (EFF)
    eff_filter: Option<String>,
    /// Whether to encrypt metadata
    encrypt_metadata: bool,
    /// Crypt filter manager
    filter_manager: Arc<CryptFilterManager>,
}

impl EmbeddedFileEncryption {
    /// Create new embedded file encryption handler
    pub fn new(
        eff_filter: Option<String>,
        encrypt_metadata: bool,
        filter_manager: Arc<CryptFilterManager>,
    ) -> Self {
        Self {
            eff_filter,
            encrypt_metadata,
            filter_manager,
        }
    }

    /// Check if a stream is an embedded file
    pub fn is_embedded_file(stream_dict: &Dictionary) -> bool {
        if let Some(Object::Name(type_name)) = stream_dict.get("Type") {
            type_name == "EmbeddedFile"
        } else {
            false
        }
    }

    /// Check if a stream is metadata
    pub fn is_metadata(stream_dict: &Dictionary) -> bool {
        if let Some(Object::Name(type_name)) = stream_dict.get("Type") {
            type_name == "Metadata"
        } else if let Some(Object::Name(subtype)) = stream_dict.get("Subtype") {
            subtype == "XML" && stream_dict.contains_key("Metadata")
        } else {
            false
        }
    }

    /// Get the appropriate filter for a stream
    pub fn get_stream_filter(&self, stream_dict: &Dictionary) -> Option<String> {
        if Self::is_embedded_file(stream_dict) {
            // Use EFF filter if configured
            self.eff_filter.clone()
        } else if Self::is_metadata(stream_dict) && !self.encrypt_metadata {
            // Don't encrypt metadata if flag is false
            Some("Identity".to_string())
        } else {
            // Use default stream filter
            None
        }
    }

    /// Encrypt embedded file data
    pub fn encrypt_embedded_file(
        &self,
        data: &[u8],
        obj_id: &ObjectId,
        encryption_key: &EncryptionKey,
    ) -> Result<Vec<u8>> {
        if let Some(ref _filter_name) = self.eff_filter {
            self.filter_manager.encrypt_stream(
                data,
                obj_id,
                &Dictionary::new(), // Empty dict for embedded files
                encryption_key,
            )
        } else {
            // No EFF filter configured, return data as-is
            Ok(data.to_vec())
        }
    }

    /// Decrypt embedded file data
    pub fn decrypt_embedded_file(
        &self,
        data: &[u8],
        obj_id: &ObjectId,
        encryption_key: &EncryptionKey,
    ) -> Result<Vec<u8>> {
        if let Some(ref _filter_name) = self.eff_filter {
            self.filter_manager.decrypt_stream(
                data,
                obj_id,
                &Dictionary::new(), // Empty dict for embedded files
                encryption_key,
            )
        } else {
            // No EFF filter configured, return data as-is
            Ok(data.to_vec())
        }
    }

    /// Process a stream for encryption, considering embedded files and metadata
    pub fn process_stream_encryption(
        &self,
        stream_dict: &Dictionary,
        data: &[u8],
        obj_id: &ObjectId,
        encryption_key: &EncryptionKey,
        encrypt: bool,
    ) -> Result<Vec<u8>> {
        // Check if this is an embedded file
        if Self::is_embedded_file(stream_dict) {
            if encrypt {
                self.encrypt_embedded_file(data, obj_id, encryption_key)
            } else {
                self.decrypt_embedded_file(data, obj_id, encryption_key)
            }
        } else if Self::is_metadata(stream_dict) && !self.encrypt_metadata {
            // Don't encrypt/decrypt metadata if flag is false
            Ok(data.to_vec())
        } else {
            // Use standard stream encryption
            if encrypt {
                self.filter_manager
                    .encrypt_stream(data, obj_id, stream_dict, encryption_key)
            } else {
                self.filter_manager
                    .decrypt_stream(data, obj_id, stream_dict, encryption_key)
            }
        }
    }
}

/// Extended encryption dictionary with EFF support
pub struct ExtendedEncryptionDict {
    /// Base encryption dictionary fields
    pub base: crate::encryption::EncryptionDictionary,
    /// Embedded files filter (EFF)
    pub eff: Option<String>,
}

impl ExtendedEncryptionDict {
    /// Create from base dictionary with EFF
    pub fn new(base: crate::encryption::EncryptionDictionary, eff: Option<String>) -> Self {
        Self { base, eff }
    }

    /// Convert to PDF dictionary with EFF field
    pub fn to_dict(&self) -> Dictionary {
        let mut dict = self.base.to_dict();

        // Add EFF field if present
        if let Some(ref eff) = self.eff {
            dict.set("EFF", Object::Name(eff.clone()));
        }

        dict
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::encryption::{
        AuthEvent, CryptFilterMethod, FunctionalCryptFilter, StandardSecurityHandler,
    };

    fn create_test_filter_manager() -> Arc<CryptFilterManager> {
        let handler = Box::new(StandardSecurityHandler::rc4_128bit());
        let mut manager =
            CryptFilterManager::new(handler, "StdCF".to_string(), "StdCF".to_string());

        // Add standard filter
        manager.add_filter(FunctionalCryptFilter {
            name: "StdCF".to_string(),
            method: CryptFilterMethod::V2,
            length: Some(16),
            auth_event: AuthEvent::DocOpen,
            recipients: None,
        });

        // Add EFF filter
        manager.add_filter(FunctionalCryptFilter {
            name: "EmbeddedFileFilter".to_string(),
            method: CryptFilterMethod::AESV2,
            length: None,
            auth_event: AuthEvent::EFOpen,
            recipients: None,
        });

        Arc::new(manager)
    }

    #[test]
    fn test_is_embedded_file() {
        let mut dict = Dictionary::new();
        assert!(!EmbeddedFileEncryption::is_embedded_file(&dict));

        dict.set("Type", Object::Name("EmbeddedFile".to_string()));
        assert!(EmbeddedFileEncryption::is_embedded_file(&dict));

        dict.set("Type", Object::Name("Stream".to_string()));
        assert!(!EmbeddedFileEncryption::is_embedded_file(&dict));
    }

    #[test]
    fn test_is_metadata() {
        let mut dict = Dictionary::new();
        assert!(!EmbeddedFileEncryption::is_metadata(&dict));

        // Type-based metadata
        dict.set("Type", Object::Name("Metadata".to_string()));
        assert!(EmbeddedFileEncryption::is_metadata(&dict));

        // Subtype-based metadata
        let mut dict2 = Dictionary::new();
        dict2.set("Subtype", Object::Name("XML".to_string()));
        dict2.set("Metadata", Object::Null);
        assert!(EmbeddedFileEncryption::is_metadata(&dict2));
    }

    #[test]
    fn test_get_stream_filter() {
        let filter_manager = create_test_filter_manager();
        let handler = EmbeddedFileEncryption::new(
            Some("EmbeddedFileFilter".to_string()),
            false,
            filter_manager,
        );

        // Test embedded file
        let mut ef_dict = Dictionary::new();
        ef_dict.set("Type", Object::Name("EmbeddedFile".to_string()));
        assert_eq!(
            handler.get_stream_filter(&ef_dict),
            Some("EmbeddedFileFilter".to_string())
        );

        // Test metadata (encrypt_metadata = false)
        let mut meta_dict = Dictionary::new();
        meta_dict.set("Type", Object::Name("Metadata".to_string()));
        assert_eq!(
            handler.get_stream_filter(&meta_dict),
            Some("Identity".to_string())
        );

        // Test regular stream
        let regular_dict = Dictionary::new();
        assert_eq!(handler.get_stream_filter(&regular_dict), None);
    }

    #[test]
    fn test_get_stream_filter_with_metadata_encryption() {
        let filter_manager = create_test_filter_manager();
        let handler = EmbeddedFileEncryption::new(
            Some("EmbeddedFileFilter".to_string()),
            true, // encrypt_metadata = true
            filter_manager,
        );

        // Test metadata with encryption enabled
        let mut meta_dict = Dictionary::new();
        meta_dict.set("Type", Object::Name("Metadata".to_string()));
        assert_eq!(handler.get_stream_filter(&meta_dict), None); // Use default
    }

    #[test]
    fn test_encrypt_embedded_file() {
        let filter_manager = create_test_filter_manager();
        let handler = EmbeddedFileEncryption::new(
            Some("EmbeddedFileFilter".to_string()),
            false,
            filter_manager,
        );

        let data = b"Embedded file content";
        let obj_id = ObjectId::new(1, 0);
        let key = EncryptionKey::new(vec![0x01; 16]);

        let encrypted = handler.encrypt_embedded_file(data, &obj_id, &key).unwrap();
        assert_ne!(encrypted, data);
    }

    #[test]
    fn test_encrypt_embedded_file_no_eff() {
        let filter_manager = create_test_filter_manager();
        let handler = EmbeddedFileEncryption::new(
            None, // No EFF filter
            false,
            filter_manager,
        );

        let data = b"Embedded file content";
        let obj_id = ObjectId::new(1, 0);
        let key = EncryptionKey::new(vec![0x01; 16]);

        let result = handler.encrypt_embedded_file(data, &obj_id, &key).unwrap();
        assert_eq!(result, data); // Should return unchanged
    }

    #[test]
    fn test_process_stream_encryption_embedded_file() {
        let filter_manager = create_test_filter_manager();
        let handler = EmbeddedFileEncryption::new(
            Some("EmbeddedFileFilter".to_string()),
            false,
            filter_manager,
        );

        let mut dict = Dictionary::new();
        dict.set("Type", Object::Name("EmbeddedFile".to_string()));

        let data = b"Embedded file data";
        let obj_id = ObjectId::new(1, 0);
        let key = EncryptionKey::new(vec![0x01; 16]);

        let encrypted = handler
            .process_stream_encryption(
                &dict, data, &obj_id, &key, true, // encrypt
            )
            .unwrap();

        assert_ne!(encrypted, data);
    }

    #[test]
    fn test_process_stream_encryption_metadata_no_encrypt() {
        let filter_manager = create_test_filter_manager();
        let handler = EmbeddedFileEncryption::new(
            Some("EmbeddedFileFilter".to_string()),
            false, // Don't encrypt metadata
            filter_manager,
        );

        let mut dict = Dictionary::new();
        dict.set("Type", Object::Name("Metadata".to_string()));

        let data = b"Metadata content";
        let obj_id = ObjectId::new(1, 0);
        let key = EncryptionKey::new(vec![0x01; 16]);

        let result = handler
            .process_stream_encryption(
                &dict, data, &obj_id, &key, true, // encrypt
            )
            .unwrap();

        assert_eq!(result, data); // Should not be encrypted
    }

    #[test]
    fn test_process_stream_encryption_regular_stream() {
        let filter_manager = create_test_filter_manager();
        let handler = EmbeddedFileEncryption::new(
            Some("EmbeddedFileFilter".to_string()),
            true,
            filter_manager,
        );

        let dict = Dictionary::new(); // Regular stream
        let data = b"Regular stream data";
        let obj_id = ObjectId::new(1, 0);
        let key = EncryptionKey::new(vec![0x01; 16]);

        let encrypted = handler
            .process_stream_encryption(
                &dict, data, &obj_id, &key, true, // encrypt
            )
            .unwrap();

        assert_ne!(encrypted, data); // Should be encrypted normally
    }

    #[test]
    fn test_extended_encryption_dict() {
        let base = crate::encryption::EncryptionDictionary::rc4_128bit(
            vec![0u8; 32],
            vec![1u8; 32],
            crate::encryption::Permissions::all(),
            None,
        );

        let extended = ExtendedEncryptionDict::new(base, Some("EmbeddedFileFilter".to_string()));

        let dict = extended.to_dict();
        assert_eq!(
            dict.get("EFF"),
            Some(&Object::Name("EmbeddedFileFilter".to_string()))
        );
    }

    #[test]
    fn test_extended_encryption_dict_no_eff() {
        let base = crate::encryption::EncryptionDictionary::rc4_128bit(
            vec![0u8; 32],
            vec![1u8; 32],
            crate::encryption::Permissions::all(),
            None,
        );

        let extended = ExtendedEncryptionDict::new(base, None);
        let dict = extended.to_dict();

        assert!(!dict.contains_key("EFF"));
    }

    #[test]
    fn test_decrypt_embedded_file() {
        let filter_manager = create_test_filter_manager();
        let handler = EmbeddedFileEncryption::new(
            Some("EmbeddedFileFilter".to_string()),
            false,
            filter_manager,
        );

        let data = b"Encrypted embedded file";
        let obj_id = ObjectId::new(1, 0);
        let key = EncryptionKey::new(vec![0x01; 16]);

        // First encrypt
        let encrypted = handler.encrypt_embedded_file(data, &obj_id, &key).unwrap();

        // Then decrypt
        let decrypted = handler
            .decrypt_embedded_file(&encrypted, &obj_id, &key)
            .unwrap();

        // RC4 should be reversible
        assert_eq!(decrypted, data);
    }

    #[test]
    fn test_process_stream_decryption() {
        let filter_manager = create_test_filter_manager();
        let handler = EmbeddedFileEncryption::new(
            Some("EmbeddedFileFilter".to_string()),
            false,
            filter_manager,
        );

        let mut dict = Dictionary::new();
        dict.set("Type", Object::Name("EmbeddedFile".to_string()));

        let original_data = b"Original embedded file";
        let obj_id = ObjectId::new(1, 0);
        let key = EncryptionKey::new(vec![0x01; 16]);

        // Encrypt
        let encrypted = handler
            .process_stream_encryption(&dict, original_data, &obj_id, &key, true)
            .unwrap();

        // Decrypt
        let decrypted = handler
            .process_stream_encryption(&dict, &encrypted, &obj_id, &key, false)
            .unwrap();

        assert_eq!(decrypted, original_data);
    }
}
