//! Crypt Filters implementation for PDF encryption
//!
//! This module implements functional crypt filters according to ISO 32000-1:2008
//! Section 7.6.5, supporting selective encryption of streams and strings.

use crate::encryption::{CryptFilterMethod, EncryptionKey, StandardSecurityHandler};
use crate::error::{PdfError, Result};
use crate::objects::{Dictionary, Object, ObjectId};
use std::collections::HashMap;

/// Functional Crypt Filter implementation
#[derive(Debug, Clone)]
pub struct FunctionalCryptFilter {
    /// Filter name
    pub name: String,
    /// Encryption method
    pub method: CryptFilterMethod,
    /// Length in bytes (for RC4)
    pub length: Option<u32>,
    /// Authentication event (when filter is applied)
    pub auth_event: AuthEvent,
    /// Recipients (for public key)
    pub recipients: Option<Vec<String>>,
}

/// Authentication event for crypt filters
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AuthEvent {
    /// Apply filter when document is opened
    DocOpen,
    /// Apply filter for embedded files only
    EFOpen,
}

impl AuthEvent {
    /// Get PDF name
    pub fn pdf_name(&self) -> &'static str {
        match self {
            AuthEvent::DocOpen => "DocOpen",
            AuthEvent::EFOpen => "EFOpen",
        }
    }
}

/// Crypt Filter Manager
pub struct CryptFilterManager {
    /// Map of filter names to filters
    filters: HashMap<String, FunctionalCryptFilter>,
    /// Default filter for streams
    default_stream_filter: String,
    /// Default filter for strings
    default_string_filter: String,
    /// Embedded files filter
    embedded_files_filter: Option<String>,
    /// Security handler
    security_handler: Box<dyn SecurityHandler>,
}

impl CryptFilterManager {
    /// Create new crypt filter manager
    pub fn new(
        security_handler: Box<dyn SecurityHandler>,
        default_stream_filter: String,
        default_string_filter: String,
    ) -> Self {
        let mut manager = Self {
            filters: HashMap::new(),
            default_stream_filter,
            default_string_filter,
            embedded_files_filter: None,
            security_handler,
        };

        // Add identity filter (no encryption)
        manager.add_filter(FunctionalCryptFilter {
            name: "Identity".to_string(),
            method: CryptFilterMethod::None,
            length: None,
            auth_event: AuthEvent::DocOpen,
            recipients: None,
        });

        manager
    }

    /// Add a crypt filter
    pub fn add_filter(&mut self, filter: FunctionalCryptFilter) {
        self.filters.insert(filter.name.clone(), filter);
    }

    /// Set embedded files filter
    pub fn set_embedded_files_filter(&mut self, filter_name: String) {
        self.embedded_files_filter = Some(filter_name);
    }

    /// Get filter by name
    pub fn get_filter(&self, name: &str) -> Option<&FunctionalCryptFilter> {
        self.filters.get(name)
    }

    /// Encrypt string with appropriate filter
    pub fn encrypt_string(
        &self,
        data: &[u8],
        obj_id: &ObjectId,
        filter_name: Option<&str>,
        encryption_key: &EncryptionKey,
    ) -> Result<Vec<u8>> {
        let filter_name = filter_name.unwrap_or(&self.default_string_filter);
        let filter = self.get_filter(filter_name).ok_or_else(|| {
            PdfError::EncryptionError(format!("Crypt filter '{filter_name}' not found"))
        })?;

        match filter.method {
            CryptFilterMethod::None => Ok(data.to_vec()),
            CryptFilterMethod::V2 => {
                // RC4 encryption
                self.security_handler
                    .encrypt_string(data, encryption_key, obj_id)
            }
            CryptFilterMethod::AESV2 => {
                // AES-128 encryption
                self.security_handler
                    .encrypt_string_aes(data, encryption_key, obj_id, 128)
            }
            CryptFilterMethod::AESV3 => {
                // AES-256 encryption
                self.security_handler
                    .encrypt_string_aes(data, encryption_key, obj_id, 256)
            }
        }
    }

    /// Decrypt string with appropriate filter
    pub fn decrypt_string(
        &self,
        data: &[u8],
        obj_id: &ObjectId,
        filter_name: Option<&str>,
        encryption_key: &EncryptionKey,
    ) -> Result<Vec<u8>> {
        let filter_name = filter_name.unwrap_or(&self.default_string_filter);
        let filter = self.get_filter(filter_name).ok_or_else(|| {
            PdfError::EncryptionError(format!("Crypt filter '{filter_name}' not found"))
        })?;

        match filter.method {
            CryptFilterMethod::None => Ok(data.to_vec()),
            CryptFilterMethod::V2 => {
                // RC4 decryption
                self.security_handler
                    .decrypt_string(data, encryption_key, obj_id)
            }
            CryptFilterMethod::AESV2 => {
                // AES-128 decryption
                self.security_handler
                    .decrypt_string_aes(data, encryption_key, obj_id, 128)
            }
            CryptFilterMethod::AESV3 => {
                // AES-256 decryption
                self.security_handler
                    .decrypt_string_aes(data, encryption_key, obj_id, 256)
            }
        }
    }

    /// Encrypt stream with appropriate filter
    pub fn encrypt_stream(
        &self,
        data: &[u8],
        obj_id: &ObjectId,
        stream_dict: &Dictionary,
        encryption_key: &EncryptionKey,
    ) -> Result<Vec<u8>> {
        // Check if stream has a specific filter
        let filter_name = self.get_stream_filter_name(stream_dict);
        let filter = self.get_filter(&filter_name).ok_or_else(|| {
            PdfError::EncryptionError(format!("Crypt filter '{filter_name}' not found"))
        })?;

        match filter.method {
            CryptFilterMethod::None => Ok(data.to_vec()),
            CryptFilterMethod::V2 => {
                // RC4 encryption
                self.security_handler
                    .encrypt_stream(data, encryption_key, obj_id)
            }
            CryptFilterMethod::AESV2 => {
                // AES-128 encryption
                self.security_handler
                    .encrypt_stream_aes(data, encryption_key, obj_id, 128)
            }
            CryptFilterMethod::AESV3 => {
                // AES-256 encryption
                self.security_handler
                    .encrypt_stream_aes(data, encryption_key, obj_id, 256)
            }
        }
    }

    /// Decrypt stream with appropriate filter
    pub fn decrypt_stream(
        &self,
        data: &[u8],
        obj_id: &ObjectId,
        stream_dict: &Dictionary,
        encryption_key: &EncryptionKey,
    ) -> Result<Vec<u8>> {
        // Check if stream has a specific filter
        let filter_name = self.get_stream_filter_name(stream_dict);
        let filter = self.get_filter(&filter_name).ok_or_else(|| {
            PdfError::EncryptionError(format!("Crypt filter '{filter_name}' not found"))
        })?;

        match filter.method {
            CryptFilterMethod::None => Ok(data.to_vec()),
            CryptFilterMethod::V2 => {
                // RC4 decryption
                self.security_handler
                    .decrypt_stream(data, encryption_key, obj_id)
            }
            CryptFilterMethod::AESV2 => {
                // AES-128 decryption
                self.security_handler
                    .decrypt_stream_aes(data, encryption_key, obj_id, 128)
            }
            CryptFilterMethod::AESV3 => {
                // AES-256 decryption
                self.security_handler
                    .decrypt_stream_aes(data, encryption_key, obj_id, 256)
            }
        }
    }

    /// Get stream filter name from dictionary
    fn get_stream_filter_name(&self, stream_dict: &Dictionary) -> String {
        // Check for Filter array
        if let Some(Object::Array(filters)) = stream_dict.get("Filter") {
            // Look for Crypt filter in array
            for filter in filters {
                if let Object::Name(name) = filter {
                    if name == "Crypt" {
                        // Check DecodeParms for crypt filter name
                        if let Some(Object::Dictionary(decode_parms)) =
                            stream_dict.get("DecodeParms")
                        {
                            if let Some(Object::Name(crypt_name)) = decode_parms.get("Name") {
                                return crypt_name.clone();
                            }
                        }
                    }
                }
            }
        }

        // Use default stream filter
        self.default_stream_filter.clone()
    }

    /// Create StdCF filter for standard encryption
    pub fn create_standard_filter(
        method: CryptFilterMethod,
        key_length: Option<u32>,
    ) -> FunctionalCryptFilter {
        FunctionalCryptFilter {
            name: "StdCF".to_string(),
            method,
            length: key_length,
            auth_event: AuthEvent::DocOpen,
            recipients: None,
        }
    }

    /// Convert filters to PDF dictionary
    pub fn to_cf_dict(&self) -> Dictionary {
        let mut cf_dict = Dictionary::new();

        for (name, filter) in &self.filters {
            if name != "Identity" {
                cf_dict.set(name, Object::Dictionary(filter.to_dict()));
            }
        }

        cf_dict
    }
}

impl FunctionalCryptFilter {
    /// Convert to PDF dictionary
    pub fn to_dict(&self) -> Dictionary {
        let mut dict = Dictionary::new();

        dict.set("CFM", Object::Name(self.method.pdf_name().to_string()));

        if let Some(length) = self.length {
            dict.set("Length", Object::Integer(length as i64));
        }

        dict.set(
            "AuthEvent",
            Object::Name(self.auth_event.pdf_name().to_string()),
        );

        if let Some(ref recipients) = self.recipients {
            let recipient_array: Vec<Object> = recipients
                .iter()
                .map(|r| Object::String(r.clone()))
                .collect();
            dict.set("Recipients", Object::Array(recipient_array));
        }

        dict
    }
}

/// Security Handler trait for encryption/decryption operations
pub trait SecurityHandler: Send + Sync {
    /// Encrypt string
    fn encrypt_string(
        &self,
        data: &[u8],
        key: &EncryptionKey,
        obj_id: &ObjectId,
    ) -> Result<Vec<u8>>;

    /// Decrypt string
    fn decrypt_string(
        &self,
        data: &[u8],
        key: &EncryptionKey,
        obj_id: &ObjectId,
    ) -> Result<Vec<u8>>;

    /// Encrypt stream
    fn encrypt_stream(
        &self,
        data: &[u8],
        key: &EncryptionKey,
        obj_id: &ObjectId,
    ) -> Result<Vec<u8>>;

    /// Decrypt stream
    fn decrypt_stream(
        &self,
        data: &[u8],
        key: &EncryptionKey,
        obj_id: &ObjectId,
    ) -> Result<Vec<u8>>;

    /// Encrypt string with AES
    fn encrypt_string_aes(
        &self,
        data: &[u8],
        key: &EncryptionKey,
        obj_id: &ObjectId,
        bits: u32,
    ) -> Result<Vec<u8>>;

    /// Decrypt string with AES
    fn decrypt_string_aes(
        &self,
        data: &[u8],
        key: &EncryptionKey,
        obj_id: &ObjectId,
        bits: u32,
    ) -> Result<Vec<u8>>;

    /// Encrypt stream with AES
    fn encrypt_stream_aes(
        &self,
        data: &[u8],
        key: &EncryptionKey,
        obj_id: &ObjectId,
        bits: u32,
    ) -> Result<Vec<u8>>;

    /// Decrypt stream with AES
    fn decrypt_stream_aes(
        &self,
        data: &[u8],
        key: &EncryptionKey,
        obj_id: &ObjectId,
        bits: u32,
    ) -> Result<Vec<u8>>;
}

/// Standard Security Handler implementation
impl SecurityHandler for StandardSecurityHandler {
    fn encrypt_string(
        &self,
        data: &[u8],
        key: &EncryptionKey,
        obj_id: &ObjectId,
    ) -> Result<Vec<u8>> {
        Ok(self.encrypt_string(data, key, obj_id))
    }

    fn decrypt_string(
        &self,
        data: &[u8],
        key: &EncryptionKey,
        obj_id: &ObjectId,
    ) -> Result<Vec<u8>> {
        Ok(self.decrypt_string(data, key, obj_id))
    }

    fn encrypt_stream(
        &self,
        data: &[u8],
        key: &EncryptionKey,
        obj_id: &ObjectId,
    ) -> Result<Vec<u8>> {
        Ok(self.encrypt_stream(data, key, obj_id))
    }

    fn decrypt_stream(
        &self,
        data: &[u8],
        key: &EncryptionKey,
        obj_id: &ObjectId,
    ) -> Result<Vec<u8>> {
        Ok(self.decrypt_stream(data, key, obj_id))
    }

    fn encrypt_string_aes(
        &self,
        data: &[u8],
        key: &EncryptionKey,
        obj_id: &ObjectId,
        bits: u32,
    ) -> Result<Vec<u8>> {
        match bits {
            128 | 256 => {
                // For AES, use the same method as streams
                self.encrypt_aes(data, key, obj_id)
            }
            _ => Err(PdfError::EncryptionError(format!(
                "Unsupported AES key size: {bits} bits"
            ))),
        }
    }

    fn decrypt_string_aes(
        &self,
        data: &[u8],
        key: &EncryptionKey,
        obj_id: &ObjectId,
        bits: u32,
    ) -> Result<Vec<u8>> {
        match bits {
            128 | 256 => {
                // For AES, use the same method as streams
                self.decrypt_aes(data, key, obj_id)
            }
            _ => Err(PdfError::EncryptionError(format!(
                "Unsupported AES key size: {bits} bits"
            ))),
        }
    }

    fn encrypt_stream_aes(
        &self,
        data: &[u8],
        key: &EncryptionKey,
        obj_id: &ObjectId,
        bits: u32,
    ) -> Result<Vec<u8>> {
        self.encrypt_string_aes(data, key, obj_id, bits)
    }

    fn decrypt_stream_aes(
        &self,
        data: &[u8],
        key: &EncryptionKey,
        obj_id: &ObjectId,
        bits: u32,
    ) -> Result<Vec<u8>> {
        self.decrypt_string_aes(data, key, obj_id, bits)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auth_event_pdf_names() {
        assert_eq!(AuthEvent::DocOpen.pdf_name(), "DocOpen");
        assert_eq!(AuthEvent::EFOpen.pdf_name(), "EFOpen");
    }

    #[test]
    fn test_functional_crypt_filter_creation() {
        let filter = FunctionalCryptFilter {
            name: "TestFilter".to_string(),
            method: CryptFilterMethod::AESV2,
            length: Some(16),
            auth_event: AuthEvent::DocOpen,
            recipients: None,
        };

        assert_eq!(filter.name, "TestFilter");
        assert_eq!(filter.method, CryptFilterMethod::AESV2);
        assert_eq!(filter.length, Some(16));
        assert_eq!(filter.auth_event, AuthEvent::DocOpen);
    }

    #[test]
    fn test_crypt_filter_to_dict() {
        let filter = FunctionalCryptFilter {
            name: "MyFilter".to_string(),
            method: CryptFilterMethod::V2,
            length: Some(16),
            auth_event: AuthEvent::EFOpen,
            recipients: Some(vec!["recipient1".to_string(), "recipient2".to_string()]),
        };

        let dict = filter.to_dict();
        assert_eq!(dict.get("CFM"), Some(&Object::Name("V2".to_string())));
        assert_eq!(dict.get("Length"), Some(&Object::Integer(16)));
        assert_eq!(
            dict.get("AuthEvent"),
            Some(&Object::Name("EFOpen".to_string()))
        );
        assert!(dict.get("Recipients").is_some());
    }

    #[test]
    fn test_crypt_filter_manager_creation() {
        let handler = StandardSecurityHandler::rc4_128bit();
        let manager =
            CryptFilterManager::new(Box::new(handler), "StdCF".to_string(), "StdCF".to_string());

        // Should have Identity filter by default
        assert!(manager.get_filter("Identity").is_some());
    }

    #[test]
    fn test_add_and_get_filter() {
        let handler = StandardSecurityHandler::rc4_128bit();
        let mut manager =
            CryptFilterManager::new(Box::new(handler), "StdCF".to_string(), "StdCF".to_string());

        let filter = FunctionalCryptFilter {
            name: "CustomFilter".to_string(),
            method: CryptFilterMethod::AESV3,
            length: None,
            auth_event: AuthEvent::DocOpen,
            recipients: None,
        };

        manager.add_filter(filter);

        let retrieved = manager.get_filter("CustomFilter");
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().method, CryptFilterMethod::AESV3);
    }

    #[test]
    fn test_standard_filter_creation() {
        let filter = CryptFilterManager::create_standard_filter(CryptFilterMethod::AESV2, Some(16));

        assert_eq!(filter.name, "StdCF");
        assert_eq!(filter.method, CryptFilterMethod::AESV2);
        assert_eq!(filter.length, Some(16));
        assert_eq!(filter.auth_event, AuthEvent::DocOpen);
    }

    #[test]
    fn test_encrypt_decrypt_string_identity() {
        let handler = StandardSecurityHandler::rc4_128bit();
        let manager = CryptFilterManager::new(
            Box::new(handler),
            "Identity".to_string(),
            "Identity".to_string(),
        );

        let data = b"Test data";
        let obj_id = ObjectId::new(1, 0);
        let key = EncryptionKey::new(vec![0u8; 16]);

        let encrypted = manager.encrypt_string(data, &obj_id, None, &key).unwrap();
        assert_eq!(encrypted, data);

        let decrypted = manager
            .decrypt_string(&encrypted, &obj_id, None, &key)
            .unwrap();
        assert_eq!(decrypted, data);
    }

    #[test]
    fn test_set_embedded_files_filter() {
        let handler = StandardSecurityHandler::rc4_128bit();
        let mut manager =
            CryptFilterManager::new(Box::new(handler), "StdCF".to_string(), "StdCF".to_string());

        manager.set_embedded_files_filter("EFFFilter".to_string());
        assert_eq!(manager.embedded_files_filter, Some("EFFFilter".to_string()));
    }

    #[test]
    fn test_to_cf_dict() {
        let handler = StandardSecurityHandler::rc4_128bit();
        let mut manager =
            CryptFilterManager::new(Box::new(handler), "StdCF".to_string(), "StdCF".to_string());

        let filter1 = FunctionalCryptFilter {
            name: "Filter1".to_string(),
            method: CryptFilterMethod::V2,
            length: Some(16),
            auth_event: AuthEvent::DocOpen,
            recipients: None,
        };

        let filter2 = FunctionalCryptFilter {
            name: "Filter2".to_string(),
            method: CryptFilterMethod::AESV2,
            length: None,
            auth_event: AuthEvent::EFOpen,
            recipients: None,
        };

        manager.add_filter(filter1);
        manager.add_filter(filter2);

        let cf_dict = manager.to_cf_dict();

        // Should not include Identity filter
        assert!(cf_dict.get("Identity").is_none());

        // Should include other filters
        assert!(cf_dict.get("Filter1").is_some());
        assert!(cf_dict.get("Filter2").is_some());
    }

    #[test]
    fn test_get_stream_filter_name_default() {
        let handler = StandardSecurityHandler::rc4_128bit();
        let manager = CryptFilterManager::new(
            Box::new(handler),
            "DefaultStreamFilter".to_string(),
            "StdCF".to_string(),
        );

        let stream_dict = Dictionary::new();
        let filter_name = manager.get_stream_filter_name(&stream_dict);
        assert_eq!(filter_name, "DefaultStreamFilter");
    }

    #[test]
    fn test_get_stream_filter_name_with_crypt() {
        let handler = StandardSecurityHandler::rc4_128bit();
        let manager = CryptFilterManager::new(
            Box::new(handler),
            "DefaultStreamFilter".to_string(),
            "StdCF".to_string(),
        );

        let mut stream_dict = Dictionary::new();
        let filters = vec![
            Object::Name("FlateDecode".to_string()),
            Object::Name("Crypt".to_string()),
        ];
        stream_dict.set("Filter", Object::Array(filters));

        let mut decode_parms = Dictionary::new();
        decode_parms.set("Name", Object::Name("SpecialFilter".to_string()));
        stream_dict.set("DecodeParms", Object::Dictionary(decode_parms));

        let filter_name = manager.get_stream_filter_name(&stream_dict);
        assert_eq!(filter_name, "SpecialFilter");
    }

    #[test]
    fn test_auth_event_equality() {
        assert_eq!(AuthEvent::DocOpen, AuthEvent::DocOpen);
        assert_eq!(AuthEvent::EFOpen, AuthEvent::EFOpen);
        assert_ne!(AuthEvent::DocOpen, AuthEvent::EFOpen);
    }

    #[test]
    fn test_crypt_filter_with_recipients() {
        let recipients = vec![
            "user1@example.com".to_string(),
            "user2@example.com".to_string(),
        ];

        let filter = FunctionalCryptFilter {
            name: "PublicKeyFilter".to_string(),
            method: CryptFilterMethod::AESV3,
            length: None,
            auth_event: AuthEvent::DocOpen,
            recipients: Some(recipients),
        };

        let dict = filter.to_dict();

        if let Some(Object::Array(recipient_array)) = dict.get("Recipients") {
            assert_eq!(recipient_array.len(), 2);
        } else {
            panic!("Recipients not found in dictionary");
        }
    }

    #[test]
    fn test_filter_not_found_error() {
        let handler = StandardSecurityHandler::rc4_128bit();
        let manager =
            CryptFilterManager::new(Box::new(handler), "StdCF".to_string(), "StdCF".to_string());

        let obj_id = ObjectId::new(1, 0);
        let key = EncryptionKey::new(vec![0u8; 16]);

        // Try to use non-existent filter
        let result = manager.encrypt_string(b"test", &obj_id, Some("NonExistentFilter"), &key);

        assert!(result.is_err());
        if let Err(PdfError::EncryptionError(msg)) = result {
            assert!(msg.contains("not found"));
        }
    }
}
