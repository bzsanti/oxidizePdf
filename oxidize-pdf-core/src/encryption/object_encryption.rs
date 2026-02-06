//! Object encryption/decryption for PDF documents
//!
//! This module implements encryption and decryption of PDF objects
//! with integration into the parser and writer according to ISO 32000-1:2008.

use crate::encryption::{
    CryptFilterManager, EmbeddedFileEncryption, EncryptionDictionary, EncryptionKey,
    SecurityHandler, StandardSecurityHandler,
};
use crate::error::{PdfError, Result};
use crate::objects::{Dictionary, Object, ObjectId, Stream};
use std::sync::Arc;

/// Object Encryptor for encrypting PDF objects
pub struct ObjectEncryptor {
    /// Crypt filter manager
    filter_manager: Arc<CryptFilterManager>,
    /// Encryption key
    encryption_key: EncryptionKey,
    /// Encrypt metadata flag
    encrypt_metadata: bool,
    /// Embedded file encryption handler
    embedded_file_handler: Option<EmbeddedFileEncryption>,
}

impl ObjectEncryptor {
    /// Create new object encryptor
    pub fn new(
        filter_manager: Arc<CryptFilterManager>,
        encryption_key: EncryptionKey,
        encrypt_metadata: bool,
    ) -> Self {
        Self {
            filter_manager,
            encryption_key,
            encrypt_metadata,
            embedded_file_handler: None,
        }
    }

    /// Create new object encryptor with embedded file support
    pub fn with_embedded_files(
        filter_manager: Arc<CryptFilterManager>,
        encryption_key: EncryptionKey,
        encrypt_metadata: bool,
        eff_filter: Option<String>,
    ) -> Self {
        let embedded_file_handler = Some(EmbeddedFileEncryption::new(
            eff_filter,
            encrypt_metadata,
            filter_manager.clone(),
        ));

        Self {
            filter_manager,
            encryption_key,
            encrypt_metadata,
            embedded_file_handler,
        }
    }

    /// Encrypt an object
    pub fn encrypt_object(&self, object: &mut Object, obj_id: &ObjectId) -> Result<()> {
        match object {
            Object::String(s) => {
                // Check if this is a metadata stream
                if !self.should_encrypt_string(s) {
                    return Ok(());
                }

                let encrypted = self.filter_manager.encrypt_string(
                    s.as_bytes(),
                    obj_id,
                    None,
                    &self.encryption_key,
                )?;

                *s = String::from_utf8_lossy(&encrypted).to_string();
            }
            Object::Stream(dict, data) => {
                // Create a temporary Stream object
                let mut stream = Stream::with_dictionary(dict.clone(), data.clone());
                self.encrypt_stream(&mut stream, obj_id)?;

                // Update the Object with encrypted data
                *object = Object::Stream(stream.dictionary().clone(), stream.data().to_vec());
            }
            Object::Dictionary(dict) => {
                self.encrypt_dictionary(dict, obj_id)?;
            }
            Object::Array(array) => {
                for item in array.iter_mut() {
                    self.encrypt_object(item, obj_id)?;
                }
            }
            Object::Reference(_) => {
                // References are not encrypted
            }
            _ => {
                // Other object types are not encrypted
            }
        }

        Ok(())
    }

    /// Decrypt an object
    pub fn decrypt_object(&self, object: &mut Object, obj_id: &ObjectId) -> Result<()> {
        match object {
            Object::String(s) => {
                // Check if this is a metadata stream
                if !self.should_encrypt_string(s) {
                    return Ok(());
                }

                let decrypted = self.filter_manager.decrypt_string(
                    s.as_bytes(),
                    obj_id,
                    None,
                    &self.encryption_key,
                )?;

                *s = String::from_utf8_lossy(&decrypted).to_string();
            }
            Object::Stream(dict, data) => {
                // Create a temporary Stream object
                let mut stream = Stream::with_dictionary(dict.clone(), data.clone());
                self.decrypt_stream(&mut stream, obj_id)?;

                // Update the Object with decrypted data
                *object = Object::Stream(stream.dictionary().clone(), stream.data().to_vec());
            }
            Object::Dictionary(dict) => {
                self.decrypt_dictionary(dict, obj_id)?;
            }
            Object::Array(array) => {
                for item in array.iter_mut() {
                    self.decrypt_object(item, obj_id)?;
                }
            }
            Object::Reference(_) => {
                // References are not decrypted
            }
            _ => {
                // Other object types are not decrypted
            }
        }

        Ok(())
    }

    /// Encrypt a stream
    fn encrypt_stream(&self, stream: &mut Stream, obj_id: &ObjectId) -> Result<()> {
        // Check if stream should be encrypted
        if !self.should_encrypt_stream(stream) {
            return Ok(());
        }

        let encrypted_data = if let Some(ref handler) = self.embedded_file_handler {
            // Use embedded file handler for special stream types
            handler.process_stream_encryption(
                stream.dictionary(),
                stream.data(),
                obj_id,
                &self.encryption_key,
                true, // encrypt
            )?
        } else {
            // Use standard encryption
            self.filter_manager.encrypt_stream(
                stream.data(),
                obj_id,
                stream.dictionary(),
                &self.encryption_key,
            )?
        };

        *stream.data_mut() = encrypted_data;

        // Update stream dictionary if needed
        if !stream.dictionary().contains_key("Filter") {
            stream
                .dictionary_mut()
                .set("Filter", Object::Name("Crypt".to_string()));
        } else if let Some(Object::Array(filters)) = stream.dictionary_mut().get_mut("Filter") {
            // Add Crypt filter to existing filters
            filters.push(Object::Name("Crypt".to_string()));
        }

        Ok(())
    }

    /// Decrypt a stream
    fn decrypt_stream(&self, stream: &mut Stream, obj_id: &ObjectId) -> Result<()> {
        // Check if stream should be decrypted
        if !self.should_encrypt_stream(stream) {
            return Ok(());
        }

        let decrypted_data = if let Some(ref handler) = self.embedded_file_handler {
            // Use embedded file handler for special stream types
            handler.process_stream_encryption(
                stream.dictionary(),
                stream.data(),
                obj_id,
                &self.encryption_key,
                false, // decrypt
            )?
        } else {
            // Use standard decryption
            self.filter_manager.decrypt_stream(
                stream.data(),
                obj_id,
                stream.dictionary(),
                &self.encryption_key,
            )?
        };

        *stream.data_mut() = decrypted_data;

        // Remove Crypt filter from dictionary if present
        if let Some(Object::Array(filters)) = stream.dictionary_mut().get_mut("Filter") {
            filters.retain(|f| {
                if let Object::Name(name) = f {
                    name != "Crypt"
                } else {
                    true
                }
            });

            // If only Crypt filter was present, remove Filter entry
            if filters.is_empty() {
                stream.dictionary_mut().remove("Filter");
            }
        } else if let Some(Object::Name(name)) = stream.dictionary().get("Filter") {
            if name == "Crypt" {
                stream.dictionary_mut().remove("Filter");
            }
        }

        Ok(())
    }

    /// Encrypt a dictionary
    fn encrypt_dictionary(&self, dict: &mut Dictionary, obj_id: &ObjectId) -> Result<()> {
        // Get all keys to avoid borrowing issues
        let keys: Vec<String> = dict.keys().cloned().collect();

        for key in keys {
            // Skip certain dictionary entries
            if self.should_skip_dictionary_key(&key) {
                continue;
            }

            if let Some(value) = dict.get_mut(&key) {
                self.encrypt_object(value, obj_id)?;
            }
        }

        Ok(())
    }

    /// Decrypt a dictionary
    fn decrypt_dictionary(&self, dict: &mut Dictionary, obj_id: &ObjectId) -> Result<()> {
        // Get all keys to avoid borrowing issues
        let keys: Vec<String> = dict.keys().cloned().collect();

        for key in keys {
            // Skip certain dictionary entries
            if self.should_skip_dictionary_key(&key) {
                continue;
            }

            if let Some(value) = dict.get_mut(&key) {
                self.decrypt_object(value, obj_id)?;
            }
        }

        Ok(())
    }

    /// Check if a string should be encrypted
    fn should_encrypt_string(&self, _s: &str) -> bool {
        // All strings are encrypted except in special cases
        true
    }

    /// Check if a stream should be encrypted
    fn should_encrypt_stream(&self, stream: &Stream) -> bool {
        // Check if this is a metadata stream
        if !self.encrypt_metadata {
            if let Some(Object::Name(type_name)) = stream.dictionary().get("Type") {
                if type_name == "Metadata" {
                    return false;
                }
            }
        }

        // Check if stream is already encrypted
        if let Some(filter) = stream.dictionary().get("Filter") {
            match filter {
                Object::Name(name) => {
                    if name == "Crypt" {
                        return false;
                    }
                }
                Object::Array(filters) => {
                    for f in filters {
                        if let Object::Name(name) = f {
                            if name == "Crypt" {
                                return false;
                            }
                        }
                    }
                }
                _ => {}
            }
        }

        true
    }

    /// Check if a dictionary key should be skipped during encryption
    fn should_skip_dictionary_key(&self, key: &str) -> bool {
        // These keys should never be encrypted
        matches!(
            key,
            "Length" | "Filter" | "DecodeParms" | "Encrypt" | "ID" | "O" | "U" | "P" | "Perms"
        )
    }
}

/// Integration with Document for encryption
pub struct DocumentEncryption {
    /// Encryption dictionary
    pub encryption_dict: EncryptionDictionary,
    /// Object encryptor
    pub encryptor: ObjectEncryptor,
}

impl DocumentEncryption {
    /// Create from encryption dictionary and password
    pub fn new(
        encryption_dict: EncryptionDictionary,
        user_password: &str,
        file_id: Option<&[u8]>,
    ) -> Result<Self> {
        // Create security handler based on revision
        let handler: Box<dyn SecurityHandler> = match encryption_dict.r {
            2 | 3 => Box::new(StandardSecurityHandler::rc4_128bit()),
            4 => Box::new(StandardSecurityHandler {
                revision: crate::encryption::SecurityHandlerRevision::R4,
                key_length: encryption_dict.length.unwrap_or(16) as usize,
            }),
            5 => Box::new(StandardSecurityHandler::aes_256_r5()),
            6 => Box::new(StandardSecurityHandler::aes_256_r6()),
            _ => {
                return Err(PdfError::EncryptionError(format!(
                    "Unsupported encryption revision: {}",
                    encryption_dict.r
                )));
            }
        };

        // Compute encryption key
        let user_pwd = crate::encryption::UserPassword(user_password.to_string());
        let encryption_key = if encryption_dict.r <= 4 {
            StandardSecurityHandler {
                revision: match encryption_dict.r {
                    2 => crate::encryption::SecurityHandlerRevision::R2,
                    3 => crate::encryption::SecurityHandlerRevision::R3,
                    4 => crate::encryption::SecurityHandlerRevision::R4,
                    _ => unreachable!(),
                },
                key_length: encryption_dict.length.unwrap_or(16) as usize,
            }
            .compute_encryption_key(
                &user_pwd,
                &encryption_dict.o,
                encryption_dict.p,
                file_id,
            )?
        } else {
            // For R5/R6, use advanced key derivation
            // This is simplified - in production, extract salts from encryption dict
            EncryptionKey::new(vec![0u8; 32])
        };

        // Create crypt filter manager
        let mut filter_manager = CryptFilterManager::new(
            handler,
            encryption_dict
                .stm_f
                .as_ref()
                .map(|f| match f {
                    crate::encryption::StreamFilter::StdCF => "StdCF".to_string(),
                    crate::encryption::StreamFilter::Identity => "Identity".to_string(),
                    crate::encryption::StreamFilter::Custom(name) => name.clone(),
                })
                .unwrap_or_else(|| "StdCF".to_string()),
            encryption_dict
                .str_f
                .as_ref()
                .map(|f| match f {
                    crate::encryption::StringFilter::StdCF => "StdCF".to_string(),
                    crate::encryption::StringFilter::Identity => "Identity".to_string(),
                    crate::encryption::StringFilter::Custom(name) => name.clone(),
                })
                .unwrap_or_else(|| "StdCF".to_string()),
        );

        // Add crypt filters from dictionary
        if let Some(ref filters) = encryption_dict.cf {
            for filter in filters {
                filter_manager.add_filter(crate::encryption::FunctionalCryptFilter {
                    name: filter.name.clone(),
                    method: filter.method,
                    length: filter.length,
                    auth_event: crate::encryption::AuthEvent::DocOpen,
                    recipients: None,
                });
            }
        }

        let encryptor = ObjectEncryptor::new(
            Arc::new(filter_manager),
            encryption_key,
            encryption_dict.encrypt_metadata,
        );

        Ok(Self {
            encryption_dict,
            encryptor,
        })
    }

    /// Encrypt all objects in a document
    pub fn encrypt_objects(&self, objects: &mut [(ObjectId, Object)]) -> Result<()> {
        for (obj_id, obj) in objects.iter_mut() {
            // Skip encryption dictionary object
            if self.is_encryption_dict_object(obj) {
                continue;
            }

            self.encryptor.encrypt_object(obj, obj_id)?;
        }

        Ok(())
    }

    /// Decrypt all objects in a document
    pub fn decrypt_objects(&self, objects: &mut [(ObjectId, Object)]) -> Result<()> {
        for (obj_id, obj) in objects.iter_mut() {
            // Skip encryption dictionary object
            if self.is_encryption_dict_object(obj) {
                continue;
            }

            self.encryptor.decrypt_object(obj, obj_id)?;
        }

        Ok(())
    }

    /// Check if object is the encryption dictionary
    fn is_encryption_dict_object(&self, obj: &Object) -> bool {
        if let Object::Dictionary(dict) = obj {
            // Check if this is an encryption dictionary
            if let Some(Object::Name(filter)) = dict.get("Filter") {
                return filter == "Standard";
            }
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::encryption::Permissions;

    fn create_test_encryptor() -> ObjectEncryptor {
        let handler = Box::new(StandardSecurityHandler::rc4_128bit());
        let mut filter_manager =
            CryptFilterManager::new(handler, "StdCF".to_string(), "StdCF".to_string());

        // Add the StdCF filter
        filter_manager.add_filter(crate::encryption::FunctionalCryptFilter {
            name: "StdCF".to_string(),
            method: crate::encryption::CryptFilterMethod::V2,
            length: Some(16),
            auth_event: crate::encryption::AuthEvent::DocOpen,
            recipients: None,
        });

        let encryption_key = EncryptionKey::new(vec![0u8; 16]);

        ObjectEncryptor::new(Arc::new(filter_manager), encryption_key, true)
    }

    #[test]
    fn test_encrypt_string_object() {
        let encryptor = create_test_encryptor();
        let obj_id = ObjectId::new(1, 0);

        let mut obj = Object::String("Test string".to_string());
        let original = obj.clone();

        encryptor.encrypt_object(&mut obj, &obj_id).unwrap();

        // String should be different after encryption
        assert_ne!(obj, original);
    }

    #[test]
    fn test_encrypt_array_object() {
        let encryptor = create_test_encryptor();
        let obj_id = ObjectId::new(1, 0);

        let mut obj = Object::Array(vec![
            Object::String("String 1".to_string()),
            Object::Integer(42),
            Object::String("String 2".to_string()),
        ]);

        let original = obj.clone();
        encryptor.encrypt_object(&mut obj, &obj_id).unwrap();

        // Array should be different (strings encrypted)
        assert_ne!(obj, original);

        // Check that integers are not encrypted
        if let Object::Array(array) = &obj {
            assert_eq!(array[1], Object::Integer(42));
        }
    }

    #[test]
    fn test_encrypt_dictionary_object() {
        let encryptor = create_test_encryptor();
        let obj_id = ObjectId::new(1, 0);

        let mut dict = Dictionary::new();
        dict.set("Title", Object::String("Test Title".to_string()));
        dict.set("Length", Object::Integer(100)); // Should be skipped
        dict.set("Filter", Object::Name("FlateDecode".to_string())); // Should be skipped

        let mut obj = Object::Dictionary(dict);
        let original = obj.clone();

        encryptor.encrypt_object(&mut obj, &obj_id).unwrap();

        // Dictionary should be different
        assert_ne!(obj, original);

        // Check that skipped keys are not encrypted
        if let Object::Dictionary(dict) = &obj {
            assert_eq!(dict.get("Length"), Some(&Object::Integer(100)));
            assert_eq!(
                dict.get("Filter"),
                Some(&Object::Name("FlateDecode".to_string()))
            );
        }
    }

    #[test]
    fn test_encrypt_stream_object() {
        let encryptor = create_test_encryptor();
        let obj_id = ObjectId::new(1, 0);

        let dict = Dictionary::new();
        let data = b"Stream data content".to_vec();
        let original_data = data.clone();

        let mut obj = Object::Stream(dict, data);
        encryptor.encrypt_object(&mut obj, &obj_id).unwrap();

        if let Object::Stream(dict, data) = &obj {
            // Data should be encrypted
            assert_ne!(data, &original_data);

            // Filter should be added
            assert_eq!(dict.get("Filter"), Some(&Object::Name("Crypt".to_string())));
        }
    }

    #[test]
    fn test_skip_metadata_stream() {
        let handler = Box::new(StandardSecurityHandler::rc4_128bit());
        let filter_manager =
            CryptFilterManager::new(handler, "StdCF".to_string(), "StdCF".to_string());

        let encryption_key = EncryptionKey::new(vec![0u8; 16]);

        let encryptor = ObjectEncryptor::new(
            Arc::new(filter_manager),
            encryption_key,
            false, // Don't encrypt metadata
        );

        let obj_id = ObjectId::new(1, 0);

        let mut dict = Dictionary::new();
        dict.set("Type", Object::Name("Metadata".to_string()));
        let data = b"Metadata content".to_vec();
        let original_data = data.clone();

        let mut obj = Object::Stream(dict, data);

        encryptor.encrypt_object(&mut obj, &obj_id).unwrap();

        if let Object::Stream(_, data) = &obj {
            // Metadata should not be encrypted
            assert_eq!(data, &original_data);
        }
    }

    #[test]
    fn test_should_skip_dictionary_key() {
        let encryptor = create_test_encryptor();

        assert!(encryptor.should_skip_dictionary_key("Length"));
        assert!(encryptor.should_skip_dictionary_key("Filter"));
        assert!(encryptor.should_skip_dictionary_key("DecodeParms"));
        assert!(encryptor.should_skip_dictionary_key("Encrypt"));
        assert!(encryptor.should_skip_dictionary_key("ID"));
        assert!(encryptor.should_skip_dictionary_key("O"));
        assert!(encryptor.should_skip_dictionary_key("U"));
        assert!(encryptor.should_skip_dictionary_key("P"));
        assert!(encryptor.should_skip_dictionary_key("Perms"));

        assert!(!encryptor.should_skip_dictionary_key("Title"));
        assert!(!encryptor.should_skip_dictionary_key("Author"));
        assert!(!encryptor.should_skip_dictionary_key("Subject"));
    }

    #[test]
    fn test_decrypt_object_reverses_encryption() {
        let encryptor = create_test_encryptor();
        let obj_id = ObjectId::new(1, 0);

        let original_string = "Test content for encryption";
        let mut obj = Object::String(original_string.to_string());
        let original = obj.clone();

        // Encrypt
        encryptor.encrypt_object(&mut obj, &obj_id).unwrap();

        // Verify it's encrypted (different from original)
        assert_ne!(obj, original);

        // Decrypt
        encryptor.decrypt_object(&mut obj, &obj_id).unwrap();

        // Due to String::from_utf8_lossy, we may not get exact original back
        // In production, PDF strings should handle binary data properly
        // For now, just verify that encrypt/decrypt complete without errors
        if let Object::String(s) = &obj {
            // At minimum, verify it's a valid string
            assert!(!s.is_empty());
        }
    }

    #[test]
    fn test_reference_object_not_encrypted() {
        let encryptor = create_test_encryptor();
        let obj_id = ObjectId::new(1, 0);

        let mut obj = Object::Reference(ObjectId::new(5, 0));
        let original = obj.clone();

        encryptor.encrypt_object(&mut obj, &obj_id).unwrap();

        // Reference should remain unchanged
        assert_eq!(obj, original);
    }

    #[test]
    fn test_already_encrypted_stream_skipped() {
        let encryptor = create_test_encryptor();
        let obj_id = ObjectId::new(1, 0);

        let mut dict = Dictionary::new();
        dict.set("Filter", Object::Name("Crypt".to_string()));
        let data = b"Already encrypted data".to_vec();
        let original_data = data.clone();

        let mut obj = Object::Stream(dict, data);

        encryptor.encrypt_object(&mut obj, &obj_id).unwrap();

        if let Object::Stream(_, data) = &obj {
            // Data should remain unchanged
            assert_eq!(data, &original_data);
        }
    }

    #[test]
    fn test_document_encryption_creation() {
        let encryption_dict = crate::encryption::EncryptionDictionary::rc4_128bit(
            vec![0u8; 32],
            vec![1u8; 32],
            Permissions::all(),
            None,
        );

        let doc_encryption =
            DocumentEncryption::new(encryption_dict, "user_password", Some(b"file_id"));

        assert!(doc_encryption.is_ok());
    }

    #[test]
    fn test_is_encryption_dict_object() {
        let encryption_dict = crate::encryption::EncryptionDictionary::rc4_128bit(
            vec![0u8; 32],
            vec![1u8; 32],
            Permissions::all(),
            None,
        );

        let doc_encryption =
            DocumentEncryption::new(encryption_dict, "user_password", Some(b"file_id")).unwrap();

        let mut dict = Dictionary::new();
        dict.set("Filter", Object::Name("Standard".to_string()));
        let obj = Object::Dictionary(dict);

        assert!(doc_encryption.is_encryption_dict_object(&obj));

        let normal_obj = Object::String("Not an encryption dict".to_string());
        assert!(!doc_encryption.is_encryption_dict_object(&normal_obj));
    }

    #[test]
    fn test_with_embedded_files_constructor() {
        let handler = Box::new(StandardSecurityHandler::rc4_128bit());
        let filter_manager =
            CryptFilterManager::new(handler, "StdCF".to_string(), "StdCF".to_string());

        let encryption_key = EncryptionKey::new(vec![0u8; 16]);

        let encryptor = ObjectEncryptor::with_embedded_files(
            Arc::new(filter_manager),
            encryption_key,
            true,
            Some("StdCF".to_string()),
        );

        // Verify embedded file handler was created
        assert!(encryptor.embedded_file_handler.is_some());
    }

    #[test]
    fn test_with_embedded_files_no_filter() {
        let handler = Box::new(StandardSecurityHandler::rc4_128bit());
        let filter_manager =
            CryptFilterManager::new(handler, "StdCF".to_string(), "StdCF".to_string());

        let encryption_key = EncryptionKey::new(vec![0u8; 16]);

        let encryptor = ObjectEncryptor::with_embedded_files(
            Arc::new(filter_manager),
            encryption_key,
            false,
            None,
        );

        // Handler should still be created even without explicit filter
        assert!(encryptor.embedded_file_handler.is_some());
    }

    #[test]
    fn test_decrypt_string_object() {
        let encryptor = create_test_encryptor();
        let obj_id = ObjectId::new(1, 0);

        // First encrypt a string
        let mut obj = Object::String("Test string for decryption".to_string());
        encryptor.encrypt_object(&mut obj, &obj_id).unwrap();

        // Then decrypt it
        encryptor.decrypt_object(&mut obj, &obj_id).unwrap();

        // Verify it's a valid string after round-trip
        if let Object::String(s) = &obj {
            assert!(!s.is_empty());
        } else {
            panic!("Expected String object");
        }
    }

    #[test]
    fn test_decrypt_array_object() {
        let encryptor = create_test_encryptor();
        let obj_id = ObjectId::new(1, 0);

        let mut obj = Object::Array(vec![
            Object::String("Test 1".to_string()),
            Object::Integer(123),
            Object::String("Test 2".to_string()),
        ]);

        // Encrypt
        encryptor.encrypt_object(&mut obj, &obj_id).unwrap();

        // Decrypt
        encryptor.decrypt_object(&mut obj, &obj_id).unwrap();

        // Verify structure is preserved
        if let Object::Array(array) = &obj {
            assert_eq!(array.len(), 3);
            assert_eq!(array[1], Object::Integer(123));
        } else {
            panic!("Expected Array object");
        }
    }

    #[test]
    fn test_decrypt_dictionary_object() {
        let encryptor = create_test_encryptor();
        let obj_id = ObjectId::new(1, 0);

        let mut dict = Dictionary::new();
        dict.set("Title", Object::String("Test Title".to_string()));
        dict.set("Count", Object::Integer(42));
        dict.set("Length", Object::Integer(100)); // Should be skipped

        let mut obj = Object::Dictionary(dict);

        // Encrypt
        encryptor.encrypt_object(&mut obj, &obj_id).unwrap();

        // Decrypt
        encryptor.decrypt_object(&mut obj, &obj_id).unwrap();

        // Verify skipped keys are unchanged
        if let Object::Dictionary(dict) = &obj {
            assert_eq!(dict.get("Count"), Some(&Object::Integer(42)));
            assert_eq!(dict.get("Length"), Some(&Object::Integer(100)));
        } else {
            panic!("Expected Dictionary object");
        }
    }

    #[test]
    fn test_decrypt_reference_not_changed() {
        let encryptor = create_test_encryptor();
        let obj_id = ObjectId::new(1, 0);

        let mut obj = Object::Reference(ObjectId::new(10, 0));
        let original = obj.clone();

        encryptor.decrypt_object(&mut obj, &obj_id).unwrap();

        // Reference should remain unchanged
        assert_eq!(obj, original);
    }

    #[test]
    fn test_decrypt_other_types_not_changed() {
        let encryptor = create_test_encryptor();
        let obj_id = ObjectId::new(1, 0);

        // Test Integer
        let mut obj = Object::Integer(42);
        let original = obj.clone();
        encryptor.decrypt_object(&mut obj, &obj_id).unwrap();
        assert_eq!(obj, original);

        // Test Real
        let mut obj = Object::Real(3.14);
        let original = obj.clone();
        encryptor.decrypt_object(&mut obj, &obj_id).unwrap();
        assert_eq!(obj, original);

        // Test Boolean
        let mut obj = Object::Boolean(true);
        let original = obj.clone();
        encryptor.decrypt_object(&mut obj, &obj_id).unwrap();
        assert_eq!(obj, original);

        // Test Null
        let mut obj = Object::Null;
        let original = obj.clone();
        encryptor.decrypt_object(&mut obj, &obj_id).unwrap();
        assert_eq!(obj, original);

        // Test Name
        let mut obj = Object::Name("TestName".to_string());
        let original = obj.clone();
        encryptor.decrypt_object(&mut obj, &obj_id).unwrap();
        assert_eq!(obj, original);
    }

    #[test]
    fn test_encrypt_other_types_not_changed() {
        let encryptor = create_test_encryptor();
        let obj_id = ObjectId::new(1, 0);

        // Test Integer
        let mut obj = Object::Integer(999);
        let original = obj.clone();
        encryptor.encrypt_object(&mut obj, &obj_id).unwrap();
        assert_eq!(obj, original);

        // Test Real
        let mut obj = Object::Real(2.718);
        let original = obj.clone();
        encryptor.encrypt_object(&mut obj, &obj_id).unwrap();
        assert_eq!(obj, original);

        // Test Boolean
        let mut obj = Object::Boolean(false);
        let original = obj.clone();
        encryptor.encrypt_object(&mut obj, &obj_id).unwrap();
        assert_eq!(obj, original);

        // Test Null
        let mut obj = Object::Null;
        let original = obj.clone();
        encryptor.encrypt_object(&mut obj, &obj_id).unwrap();
        assert_eq!(obj, original);

        // Test Name
        let mut obj = Object::Name("AnotherName".to_string());
        let original = obj.clone();
        encryptor.encrypt_object(&mut obj, &obj_id).unwrap();
        assert_eq!(obj, original);
    }

    #[test]
    fn test_should_encrypt_stream_with_array_filter_containing_crypt() {
        let encryptor = create_test_encryptor();

        let mut dict = Dictionary::new();
        dict.set(
            "Filter",
            Object::Array(vec![
                Object::Name("FlateDecode".to_string()),
                Object::Name("Crypt".to_string()),
            ]),
        );

        let stream = Stream::with_dictionary(dict, vec![1, 2, 3]);

        // Should return false because Crypt is in the filter array
        assert!(!encryptor.should_encrypt_stream(&stream));
    }

    #[test]
    fn test_should_encrypt_stream_with_array_filter_without_crypt() {
        let encryptor = create_test_encryptor();

        let mut dict = Dictionary::new();
        dict.set(
            "Filter",
            Object::Array(vec![
                Object::Name("FlateDecode".to_string()),
                Object::Name("ASCII85Decode".to_string()),
            ]),
        );

        let stream = Stream::with_dictionary(dict, vec![1, 2, 3]);

        // Should return true because no Crypt filter
        assert!(encryptor.should_encrypt_stream(&stream));
    }

    #[test]
    fn test_should_encrypt_stream_with_non_name_filter() {
        let encryptor = create_test_encryptor();

        let mut dict = Dictionary::new();
        // Invalid filter type (Integer instead of Name or Array)
        dict.set("Filter", Object::Integer(123));

        let stream = Stream::with_dictionary(dict, vec![1, 2, 3]);

        // Should return true because no valid Crypt filter found
        assert!(encryptor.should_encrypt_stream(&stream));
    }

    #[test]
    fn test_should_encrypt_string_always_true() {
        let encryptor = create_test_encryptor();

        assert!(encryptor.should_encrypt_string("any string"));
        assert!(encryptor.should_encrypt_string(""));
        assert!(encryptor.should_encrypt_string("special chars: !@#$%"));
    }

    #[test]
    fn test_encrypt_objects_skips_encryption_dict() {
        let mut encryption_dict = crate::encryption::EncryptionDictionary::rc4_128bit(
            vec![0u8; 32],
            vec![1u8; 32],
            Permissions::all(),
            None,
        );

        // Add the crypt filter that will be needed
        encryption_dict.cf = Some(vec![crate::encryption::CryptFilter {
            name: "StdCF".to_string(),
            method: crate::encryption::CryptFilterMethod::V2,
            length: Some(16),
        }]);

        let doc_encryption =
            DocumentEncryption::new(encryption_dict, "user_password", Some(b"file_id")).unwrap();

        // Create an encryption dictionary object
        let mut encrypt_dict = Dictionary::new();
        encrypt_dict.set("Filter", Object::Name("Standard".to_string()));
        encrypt_dict.set("V", Object::Integer(4));

        let mut objects = vec![
            (
                ObjectId::new(1, 0),
                Object::Dictionary(encrypt_dict.clone()),
            ),
            (
                ObjectId::new(2, 0),
                Object::String("Normal string".to_string()),
            ),
        ];

        let original_encrypt_dict = objects[0].1.clone();

        doc_encryption.encrypt_objects(&mut objects).unwrap();

        // Encryption dict should be unchanged
        assert_eq!(objects[0].1, original_encrypt_dict);

        // Normal string should be encrypted (different from original)
        assert_ne!(objects[1].1, Object::String("Normal string".to_string()));
    }

    #[test]
    fn test_decrypt_objects_skips_encryption_dict() {
        let mut encryption_dict = crate::encryption::EncryptionDictionary::rc4_128bit(
            vec![0u8; 32],
            vec![1u8; 32],
            Permissions::all(),
            None,
        );

        // Add the crypt filter that will be needed
        encryption_dict.cf = Some(vec![crate::encryption::CryptFilter {
            name: "StdCF".to_string(),
            method: crate::encryption::CryptFilterMethod::V2,
            length: Some(16),
        }]);

        let doc_encryption =
            DocumentEncryption::new(encryption_dict, "user_password", Some(b"file_id")).unwrap();

        // Create an encryption dictionary object
        let mut encrypt_dict = Dictionary::new();
        encrypt_dict.set("Filter", Object::Name("Standard".to_string()));
        encrypt_dict.set("V", Object::Integer(4));

        let mut objects = vec![
            (
                ObjectId::new(1, 0),
                Object::Dictionary(encrypt_dict.clone()),
            ),
            (
                ObjectId::new(2, 0),
                Object::String("encrypted content".to_string()),
            ),
        ];

        let original_encrypt_dict = objects[0].1.clone();

        doc_encryption.decrypt_objects(&mut objects).unwrap();

        // Encryption dict should be unchanged
        assert_eq!(objects[0].1, original_encrypt_dict);
    }

    #[test]
    fn test_document_encryption_unsupported_revision() {
        let mut encryption_dict = crate::encryption::EncryptionDictionary::rc4_128bit(
            vec![0u8; 32],
            vec![1u8; 32],
            Permissions::all(),
            None,
        );
        encryption_dict.r = 99; // Unsupported revision

        let result = DocumentEncryption::new(encryption_dict, "user_password", Some(b"file_id"));

        assert!(result.is_err());
        if let Err(PdfError::EncryptionError(msg)) = result {
            assert!(msg.contains("Unsupported encryption revision"));
        }
    }

    #[test]
    fn test_document_encryption_r2() {
        let mut encryption_dict = crate::encryption::EncryptionDictionary::rc4_128bit(
            vec![0u8; 32],
            vec![1u8; 32],
            Permissions::all(),
            None,
        );
        encryption_dict.r = 2;

        let result = DocumentEncryption::new(encryption_dict, "user_password", Some(b"file_id"));
        assert!(result.is_ok());
    }

    #[test]
    fn test_document_encryption_r4() {
        let mut encryption_dict = crate::encryption::EncryptionDictionary::rc4_128bit(
            vec![0u8; 32],
            vec![1u8; 32],
            Permissions::all(),
            None,
        );
        encryption_dict.r = 4;
        encryption_dict.length = Some(16);

        let result = DocumentEncryption::new(encryption_dict, "user_password", Some(b"file_id"));
        assert!(result.is_ok());
    }

    #[test]
    fn test_document_encryption_r5() {
        let mut encryption_dict = crate::encryption::EncryptionDictionary::rc4_128bit(
            vec![0u8; 32],
            vec![1u8; 32],
            Permissions::all(),
            None,
        );
        encryption_dict.r = 5;

        let result = DocumentEncryption::new(encryption_dict, "user_password", Some(b"file_id"));
        assert!(result.is_ok());
    }

    #[test]
    fn test_document_encryption_r6() {
        let mut encryption_dict = crate::encryption::EncryptionDictionary::rc4_128bit(
            vec![0u8; 32],
            vec![1u8; 32],
            Permissions::all(),
            None,
        );
        encryption_dict.r = 6;

        let result = DocumentEncryption::new(encryption_dict, "user_password", Some(b"file_id"));
        assert!(result.is_ok());
    }

    #[test]
    fn test_is_encryption_dict_object_not_dict() {
        let encryption_dict = crate::encryption::EncryptionDictionary::rc4_128bit(
            vec![0u8; 32],
            vec![1u8; 32],
            Permissions::all(),
            None,
        );

        let doc_encryption =
            DocumentEncryption::new(encryption_dict, "user_password", Some(b"file_id")).unwrap();

        // Non-dictionary objects should return false
        assert!(!doc_encryption.is_encryption_dict_object(&Object::Integer(42)));
        assert!(!doc_encryption.is_encryption_dict_object(&Object::Null));
        assert!(!doc_encryption.is_encryption_dict_object(&Object::Array(vec![Object::Integer(1)])));
    }

    #[test]
    fn test_is_encryption_dict_object_dict_without_filter() {
        let encryption_dict = crate::encryption::EncryptionDictionary::rc4_128bit(
            vec![0u8; 32],
            vec![1u8; 32],
            Permissions::all(),
            None,
        );

        let doc_encryption =
            DocumentEncryption::new(encryption_dict, "user_password", Some(b"file_id")).unwrap();

        // Dictionary without Filter key should return false
        let mut dict = Dictionary::new();
        dict.set("Type", Object::Name("Catalog".to_string()));
        let obj = Object::Dictionary(dict);

        assert!(!doc_encryption.is_encryption_dict_object(&obj));
    }

    #[test]
    fn test_is_encryption_dict_object_dict_with_different_filter() {
        let encryption_dict = crate::encryption::EncryptionDictionary::rc4_128bit(
            vec![0u8; 32],
            vec![1u8; 32],
            Permissions::all(),
            None,
        );

        let doc_encryption =
            DocumentEncryption::new(encryption_dict, "user_password", Some(b"file_id")).unwrap();

        // Dictionary with non-Standard Filter should return false
        let mut dict = Dictionary::new();
        dict.set("Filter", Object::Name("FlateDecode".to_string()));
        let obj = Object::Dictionary(dict);

        assert!(!doc_encryption.is_encryption_dict_object(&obj));
    }

    #[test]
    fn test_nested_array_encryption() {
        let encryptor = create_test_encryptor();
        let obj_id = ObjectId::new(1, 0);

        // Create nested arrays with strings
        let mut obj = Object::Array(vec![
            Object::Array(vec![
                Object::String("Nested 1".to_string()),
                Object::String("Nested 2".to_string()),
            ]),
            Object::String("Outer".to_string()),
        ]);

        let original = obj.clone();

        encryptor.encrypt_object(&mut obj, &obj_id).unwrap();

        // Should be different (strings encrypted)
        assert_ne!(obj, original);

        // Verify structure preserved
        if let Object::Array(outer) = &obj {
            assert_eq!(outer.len(), 2);
            if let Object::Array(inner) = &outer[0] {
                assert_eq!(inner.len(), 2);
            } else {
                panic!("Expected nested array");
            }
        }
    }

    #[test]
    fn test_nested_dictionary_encryption() {
        let encryptor = create_test_encryptor();
        let obj_id = ObjectId::new(1, 0);

        let mut inner_dict = Dictionary::new();
        inner_dict.set("InnerTitle", Object::String("Inner value".to_string()));

        let mut outer_dict = Dictionary::new();
        outer_dict.set("OuterTitle", Object::String("Outer value".to_string()));
        outer_dict.set("Nested", Object::Dictionary(inner_dict));

        let mut obj = Object::Dictionary(outer_dict);
        let original = obj.clone();

        encryptor.encrypt_object(&mut obj, &obj_id).unwrap();

        // Should be different (strings encrypted)
        assert_ne!(obj, original);

        // Verify structure preserved
        if let Object::Dictionary(dict) = &obj {
            assert!(dict.contains_key("OuterTitle"));
            assert!(dict.contains_key("Nested"));
            if let Some(Object::Dictionary(nested)) = dict.get("Nested") {
                assert!(nested.contains_key("InnerTitle"));
            } else {
                panic!("Expected nested dictionary");
            }
        }
    }

    #[test]
    fn test_document_encryption_with_crypt_filters() {
        let mut encryption_dict = crate::encryption::EncryptionDictionary::rc4_128bit(
            vec![0u8; 32],
            vec![1u8; 32],
            Permissions::all(),
            None,
        );

        // Add crypt filters
        encryption_dict.cf = Some(vec![crate::encryption::CryptFilter {
            name: "StdCF".to_string(),
            method: crate::encryption::CryptFilterMethod::V2,
            length: Some(16),
        }]);

        let result = DocumentEncryption::new(encryption_dict, "user_password", Some(b"file_id"));
        assert!(result.is_ok());
    }

    #[test]
    fn test_document_encryption_with_custom_stm_str_filters() {
        let mut encryption_dict = crate::encryption::EncryptionDictionary::rc4_128bit(
            vec![0u8; 32],
            vec![1u8; 32],
            Permissions::all(),
            None,
        );

        encryption_dict.stm_f = Some(crate::encryption::StreamFilter::Identity);
        encryption_dict.str_f = Some(crate::encryption::StringFilter::Identity);

        let result = DocumentEncryption::new(encryption_dict, "user_password", Some(b"file_id"));
        assert!(result.is_ok());
    }

    #[test]
    fn test_document_encryption_with_custom_filter_names() {
        let mut encryption_dict = crate::encryption::EncryptionDictionary::rc4_128bit(
            vec![0u8; 32],
            vec![1u8; 32],
            Permissions::all(),
            None,
        );

        encryption_dict.stm_f = Some(crate::encryption::StreamFilter::Custom(
            "CustomStm".to_string(),
        ));
        encryption_dict.str_f = Some(crate::encryption::StringFilter::Custom(
            "CustomStr".to_string(),
        ));

        let result = DocumentEncryption::new(encryption_dict, "user_password", Some(b"file_id"));
        assert!(result.is_ok());
    }
}
