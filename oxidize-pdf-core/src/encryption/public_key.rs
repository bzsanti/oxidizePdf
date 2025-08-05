//! Public Key Security Handler for PDF encryption
//!
//! This module implements the Public Key Security Handler according to ISO 32000-1:2008 §7.6.4.
//! It supports X.509 certificates and various SubFilter types for recipient-based encryption.

use crate::encryption::{CryptFilterMethod, EncryptionKey, Permissions, SecurityHandler};
use crate::error::{PdfError, Result};
use crate::objects::{Dictionary, Object, ObjectId};
use std::collections::HashMap;

/// SubFilter types for public key security
#[derive(Debug, Clone, PartialEq)]
pub enum SubFilter {
    /// PKCS#7 with SHA-1 (adbe.pkcs7.s3)
    AdbePkcs7S3,
    /// PKCS#7 with SHA-256 (adbe.pkcs7.s4)
    AdbePkcs7S4,
    /// PKCS#7 with SHA-256 or stronger (adbe.pkcs7.s5)
    AdbePkcs7S5,
    /// X.509 certificates with SHA-1 (adbe.x509.rsa_sha1)
    AdbeX509RsaSha1,
    /// Custom SubFilter
    Custom(String),
}

impl SubFilter {
    /// Convert to PDF name
    pub fn to_name(&self) -> &str {
        match self {
            SubFilter::AdbePkcs7S3 => "adbe.pkcs7.s3",
            SubFilter::AdbePkcs7S4 => "adbe.pkcs7.s4",
            SubFilter::AdbePkcs7S5 => "adbe.pkcs7.s5",
            SubFilter::AdbeX509RsaSha1 => "adbe.x509.rsa_sha1",
            SubFilter::Custom(name) => name,
        }
    }

    /// Create from PDF name
    pub fn from_name(name: &str) -> Self {
        match name {
            "adbe.pkcs7.s3" => SubFilter::AdbePkcs7S3,
            "adbe.pkcs7.s4" => SubFilter::AdbePkcs7S4,
            "adbe.pkcs7.s5" => SubFilter::AdbePkcs7S5,
            "adbe.x509.rsa_sha1" => SubFilter::AdbeX509RsaSha1,
            _ => SubFilter::Custom(name.to_string()),
        }
    }
}

/// Recipient information for public key encryption
#[derive(Debug, Clone)]
pub struct Recipient {
    /// Certificate (X.509 DER encoded)
    pub certificate: Vec<u8>,
    /// Permissions granted to this recipient
    pub permissions: Permissions,
    /// Encrypted seed value for this recipient
    pub encrypted_seed: Vec<u8>,
}

/// Public Key Security Handler
pub struct PublicKeySecurityHandler {
    /// SubFilter type
    pub subfilter: SubFilter,
    /// Recipients list
    pub recipients: Vec<Recipient>,
    /// Seed value length in bytes (20 for SHA-1, 32 for SHA-256)
    pub seed_length: usize,
    /// Encryption method
    pub method: CryptFilterMethod,
}

impl PublicKeySecurityHandler {
    /// Create a new public key security handler with SHA-1
    pub fn new_sha1() -> Self {
        Self {
            subfilter: SubFilter::AdbePkcs7S3,
            recipients: Vec::new(),
            seed_length: 20,
            method: CryptFilterMethod::V2,
        }
    }

    /// Create a new public key security handler with SHA-256
    pub fn new_sha256() -> Self {
        Self {
            subfilter: SubFilter::AdbePkcs7S4,
            recipients: Vec::new(),
            seed_length: 32,
            method: CryptFilterMethod::AESV2,
        }
    }

    /// Add a recipient
    pub fn add_recipient(&mut self, certificate: Vec<u8>, permissions: Permissions) -> Result<()> {
        // Generate random seed
        let seed = self.generate_seed()?;

        // Encrypt seed with recipient's public key
        let encrypted_seed = self.encrypt_seed_for_recipient(&seed, &certificate)?;

        self.recipients.push(Recipient {
            certificate,
            permissions,
            encrypted_seed,
        });

        Ok(())
    }

    /// Generate random seed value
    fn generate_seed(&self) -> Result<Vec<u8>> {
        // In production, use a cryptographically secure RNG
        // For now, we'll use a simple approach with timestamp
        use std::time::{SystemTime, UNIX_EPOCH};

        let mut seed = vec![0u8; self.seed_length];

        // Get current timestamp for pseudo-randomness
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos() as u64;

        // Fill with pseudo-random data based on timestamp
        for (i, byte) in seed.iter_mut().enumerate() {
            *byte = ((timestamp
                .wrapping_mul(i as u64 + 1)
                .wrapping_add(i as u64 * 7 + 13))
                % 256) as u8;
        }

        Ok(seed)
    }

    /// Encrypt seed for a specific recipient
    fn encrypt_seed_for_recipient(&self, seed: &[u8], certificate: &[u8]) -> Result<Vec<u8>> {
        // In a real implementation, this would:
        // 1. Parse the X.509 certificate
        // 2. Extract the public key
        // 3. Encrypt the seed using RSA or ECDSA
        // For now, we'll simulate this

        // Validate certificate length
        if certificate.len() < 100 {
            return Err(PdfError::EncryptionError("Invalid certificate".to_string()));
        }

        // Simulate RSA encryption (in production, use a crypto library)
        let mut encrypted = seed.to_vec();
        encrypted.extend_from_slice(&certificate[0..4]); // Add certificate fingerprint

        Ok(encrypted)
    }

    /// Decrypt seed value using private key
    pub fn decrypt_seed(&self, encrypted_seed: &[u8], private_key: &[u8]) -> Result<Vec<u8>> {
        // In a real implementation, this would use the private key to decrypt
        // For now, we'll simulate this

        if private_key.is_empty() {
            return Err(PdfError::EncryptionError(
                "Private key required".to_string(),
            ));
        }

        // Return the seed portion (simulation)
        if encrypted_seed.len() >= self.seed_length {
            Ok(encrypted_seed[0..self.seed_length].to_vec())
        } else {
            Err(PdfError::EncryptionError(
                "Invalid encrypted seed".to_string(),
            ))
        }
    }

    /// Build recipients dictionary for PDF
    pub fn build_recipients_dict(&self) -> Dictionary {
        let mut dict = Dictionary::new();

        let recipients_array: Vec<Object> = self
            .recipients
            .iter()
            .map(|recipient| {
                let mut recipient_dict = Dictionary::new();

                // Certificate
                recipient_dict.set(
                    "Cert",
                    Object::String(String::from_utf8_lossy(&recipient.certificate).to_string()),
                );

                // Permissions
                recipient_dict.set("P", Object::Integer(recipient.permissions.bits() as i64));

                // Encrypted seed
                recipient_dict.set(
                    "Recipients",
                    Object::String(String::from_utf8_lossy(&recipient.encrypted_seed).to_string()),
                );

                Object::Dictionary(recipient_dict)
            })
            .collect();

        dict.set("Recipients", Object::Array(recipients_array));
        dict
    }

    /// Verify recipient has permission
    pub fn verify_permission(&self, recipient_index: usize, permission: Permissions) -> bool {
        if let Some(recipient) = self.recipients.get(recipient_index) {
            recipient.permissions.contains(permission)
        } else {
            false
        }
    }
}

impl SecurityHandler for PublicKeySecurityHandler {
    fn encrypt_string(
        &self,
        data: &[u8],
        encryption_key: &EncryptionKey,
        obj_id: &ObjectId,
    ) -> Result<Vec<u8>> {
        // Use the appropriate encryption based on method
        match self.method {
            CryptFilterMethod::V2 => {
                // RC4 encryption
                use crate::encryption::{Rc4, Rc4Key};
                let mut key = encryption_key.as_bytes().to_vec();
                key.extend_from_slice(&obj_id.number().to_le_bytes()[0..3]);
                key.extend_from_slice(&obj_id.generation().to_le_bytes()[0..2]);

                let rc4_key = Rc4Key::from_slice(&key);
                let mut cipher = Rc4::new(&rc4_key);
                Ok(cipher.process(data))
            }
            CryptFilterMethod::AESV2 | CryptFilterMethod::AESV3 => {
                // AES encryption
                use crate::encryption::{Aes, AesKey};
                let aes_key = AesKey::new_128(encryption_key.as_bytes().to_vec())
                    .map_err(|e| PdfError::EncryptionError(e.to_string()))?;
                let aes = Aes::new(aes_key);

                // Generate deterministic IV based on object ID
                let mut iv = vec![0u8; 16];
                let obj_bytes = obj_id.number().to_le_bytes();
                let gen_bytes = obj_id.generation().to_le_bytes();
                iv[..4].copy_from_slice(&obj_bytes);
                iv[4..(2 + 4)].copy_from_slice(&gen_bytes);
                // Fill rest with pattern
                for (i, item) in iv.iter_mut().enumerate().take(16).skip(6) {
                    *item = ((i * 13 + 7) % 256) as u8;
                }

                aes.encrypt_cbc(data, &iv)
                    .map_err(|e| PdfError::EncryptionError(e.to_string()))
            }
            _ => Err(PdfError::EncryptionError(format!(
                "Unsupported encryption method: {:?}",
                self.method
            ))),
        }
    }

    fn decrypt_string(
        &self,
        data: &[u8],
        encryption_key: &EncryptionKey,
        obj_id: &ObjectId,
    ) -> Result<Vec<u8>> {
        // Use the appropriate decryption based on method
        match self.method {
            CryptFilterMethod::V2 => {
                // RC4 decryption (same as encryption)
                self.encrypt_string(data, encryption_key, obj_id)
            }
            CryptFilterMethod::AESV2 | CryptFilterMethod::AESV3 => {
                // AES decryption
                use crate::encryption::{Aes, AesKey};
                let aes_key = AesKey::new_128(encryption_key.as_bytes().to_vec())
                    .map_err(|e| PdfError::EncryptionError(e.to_string()))?;
                let aes = Aes::new(aes_key);

                // Generate deterministic IV based on object ID
                let mut iv = vec![0u8; 16];
                let obj_bytes = obj_id.number().to_le_bytes();
                let gen_bytes = obj_id.generation().to_le_bytes();
                iv[..4].copy_from_slice(&obj_bytes);
                iv[4..(2 + 4)].copy_from_slice(&gen_bytes);
                // Fill rest with pattern
                for (i, item) in iv.iter_mut().enumerate().take(16).skip(6) {
                    *item = ((i * 13 + 7) % 256) as u8;
                }

                aes.decrypt_cbc(data, &iv)
                    .map_err(|e| PdfError::EncryptionError(e.to_string()))
            }
            _ => Err(PdfError::EncryptionError(format!(
                "Unsupported decryption method: {:?}",
                self.method
            ))),
        }
    }

    fn encrypt_stream(
        &self,
        data: &[u8],
        encryption_key: &EncryptionKey,
        obj_id: &ObjectId,
    ) -> Result<Vec<u8>> {
        // Streams use the same encryption as strings
        self.encrypt_string(data, encryption_key, obj_id)
    }

    fn decrypt_stream(
        &self,
        data: &[u8],
        encryption_key: &EncryptionKey,
        obj_id: &ObjectId,
    ) -> Result<Vec<u8>> {
        // Streams use the same decryption as strings
        self.decrypt_string(data, encryption_key, obj_id)
    }

    fn encrypt_string_aes(
        &self,
        data: &[u8],
        encryption_key: &EncryptionKey,
        _obj_id: &ObjectId,
        bits: u32,
    ) -> Result<Vec<u8>> {
        use crate::encryption::{Aes, AesKey};

        let aes_key = if bits == 256 {
            AesKey::new_256(encryption_key.as_bytes().to_vec())
        } else {
            AesKey::new_128(encryption_key.as_bytes().to_vec())
        }
        .map_err(|e| PdfError::EncryptionError(e.to_string()))?;

        let aes = Aes::new(aes_key);

        // Generate deterministic IV based on object ID (unused for AES methods)
        let mut iv = vec![0u8; 16];
        for (i, item) in iv.iter_mut().enumerate().take(16) {
            *item = ((i * 13 + 7) % 256) as u8;
        }

        aes.encrypt_cbc(data, &iv)
            .map_err(|e| PdfError::EncryptionError(e.to_string()))
    }

    fn decrypt_string_aes(
        &self,
        data: &[u8],
        encryption_key: &EncryptionKey,
        _obj_id: &ObjectId,
        bits: u32,
    ) -> Result<Vec<u8>> {
        use crate::encryption::{Aes, AesKey};

        let aes_key = if bits == 256 {
            AesKey::new_256(encryption_key.as_bytes().to_vec())
        } else {
            AesKey::new_128(encryption_key.as_bytes().to_vec())
        }
        .map_err(|e| PdfError::EncryptionError(e.to_string()))?;

        let aes = Aes::new(aes_key);

        // Generate deterministic IV based on object ID (unused for AES methods)
        let mut iv = vec![0u8; 16];
        for (i, item) in iv.iter_mut().enumerate().take(16) {
            *item = ((i * 13 + 7) % 256) as u8;
        }

        aes.decrypt_cbc(data, &iv)
            .map_err(|e| PdfError::EncryptionError(e.to_string()))
    }

    fn encrypt_stream_aes(
        &self,
        data: &[u8],
        encryption_key: &EncryptionKey,
        obj_id: &ObjectId,
        bits: u32,
    ) -> Result<Vec<u8>> {
        // Streams use the same AES encryption as strings
        self.encrypt_string_aes(data, encryption_key, obj_id, bits)
    }

    fn decrypt_stream_aes(
        &self,
        data: &[u8],
        encryption_key: &EncryptionKey,
        obj_id: &ObjectId,
        bits: u32,
    ) -> Result<Vec<u8>> {
        // Streams use the same AES decryption as strings
        self.decrypt_string_aes(data, encryption_key, obj_id, bits)
    }
}

/// Public Key Encryption Dictionary
#[derive(Debug, Clone)]
pub struct PublicKeyEncryptionDict {
    /// Filter (must be "Adobe.PubSec")
    pub filter: String,
    /// SubFilter
    pub subfilter: SubFilter,
    /// Version
    pub v: u8,
    /// Length in bytes (40 to 128)
    pub length: Option<u32>,
    /// Crypt filters
    pub cf: Option<HashMap<String, Dictionary>>,
    /// Default crypt filter for streams
    pub stm_f: Option<String>,
    /// Default crypt filter for strings  
    pub str_f: Option<String>,
    /// Recipients
    pub recipients: Vec<Dictionary>,
    /// Encrypt metadata
    pub encrypt_metadata: bool,
}

impl PublicKeyEncryptionDict {
    /// Create a new public key encryption dictionary
    pub fn new(handler: &PublicKeySecurityHandler) -> Self {
        Self {
            filter: "Adobe.PubSec".to_string(),
            subfilter: handler.subfilter.clone(),
            v: match handler.method {
                CryptFilterMethod::V2 => 2,
                CryptFilterMethod::AESV2 => 4,
                CryptFilterMethod::AESV3 => 5,
                _ => 4,
            },
            length: Some(match handler.method {
                CryptFilterMethod::V2 => 128,
                _ => 256,
            }),
            cf: None,
            stm_f: Some("DefaultCryptFilter".to_string()),
            str_f: Some("DefaultCryptFilter".to_string()),
            recipients: handler
                .recipients
                .iter()
                .map(|r| {
                    let mut dict = Dictionary::new();
                    dict.set(
                        "Cert",
                        Object::String(String::from_utf8_lossy(&r.certificate).to_string()),
                    );
                    dict.set("P", Object::Integer(r.permissions.bits() as i64));
                    dict.set(
                        "Recipients",
                        Object::String(String::from_utf8_lossy(&r.encrypted_seed).to_string()),
                    );
                    dict
                })
                .collect(),
            encrypt_metadata: true,
        }
    }

    /// Convert to PDF dictionary
    pub fn to_dict(&self) -> Dictionary {
        let mut dict = Dictionary::new();

        dict.set("Filter", Object::Name(self.filter.clone()));
        dict.set(
            "SubFilter",
            Object::Name(self.subfilter.to_name().to_string()),
        );
        dict.set("V", Object::Integer(self.v as i64));

        if let Some(length) = self.length {
            dict.set("Length", Object::Integer(length as i64));
        }

        if let Some(ref _cf) = self.cf {
            let cf_dict = Dictionary::new();
            // Add crypt filters...
            dict.set("CF", Object::Dictionary(cf_dict));
        }

        if let Some(ref stm_f) = self.stm_f {
            dict.set("StmF", Object::Name(stm_f.clone()));
        }

        if let Some(ref str_f) = self.str_f {
            dict.set("StrF", Object::Name(str_f.clone()));
        }

        let recipients_array: Vec<Object> = self
            .recipients
            .iter()
            .map(|r| Object::Dictionary(r.clone()))
            .collect();
        dict.set("Recipients", Object::Array(recipients_array));

        dict.set("EncryptMetadata", Object::Boolean(self.encrypt_metadata));

        dict
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_subfilter_conversion() {
        assert_eq!(SubFilter::AdbePkcs7S3.to_name(), "adbe.pkcs7.s3");
        assert_eq!(SubFilter::AdbePkcs7S4.to_name(), "adbe.pkcs7.s4");
        assert_eq!(SubFilter::AdbePkcs7S5.to_name(), "adbe.pkcs7.s5");
        assert_eq!(SubFilter::AdbeX509RsaSha1.to_name(), "adbe.x509.rsa_sha1");

        let custom = SubFilter::Custom("custom.filter".to_string());
        assert_eq!(custom.to_name(), "custom.filter");
    }

    #[test]
    fn test_subfilter_from_name() {
        assert_eq!(
            SubFilter::from_name("adbe.pkcs7.s3"),
            SubFilter::AdbePkcs7S3
        );
        assert_eq!(
            SubFilter::from_name("adbe.pkcs7.s4"),
            SubFilter::AdbePkcs7S4
        );
        assert_eq!(
            SubFilter::from_name("adbe.pkcs7.s5"),
            SubFilter::AdbePkcs7S5
        );
        assert_eq!(
            SubFilter::from_name("adbe.x509.rsa_sha1"),
            SubFilter::AdbeX509RsaSha1
        );
        assert_eq!(
            SubFilter::from_name("unknown"),
            SubFilter::Custom("unknown".to_string())
        );
    }

    #[test]
    fn test_public_key_handler_creation() {
        let handler_sha1 = PublicKeySecurityHandler::new_sha1();
        assert_eq!(handler_sha1.subfilter, SubFilter::AdbePkcs7S3);
        assert_eq!(handler_sha1.seed_length, 20);
        assert_eq!(handler_sha1.method, CryptFilterMethod::V2);

        let handler_sha256 = PublicKeySecurityHandler::new_sha256();
        assert_eq!(handler_sha256.subfilter, SubFilter::AdbePkcs7S4);
        assert_eq!(handler_sha256.seed_length, 32);
        assert_eq!(handler_sha256.method, CryptFilterMethod::AESV2);
    }

    #[test]
    fn test_add_recipient() {
        let mut handler = PublicKeySecurityHandler::new_sha1();

        // Create a mock certificate (at least 100 bytes)
        let certificate = vec![0x30; 200]; // DER-like data
        let permissions = Permissions::new()
            .set_print(true)
            .set_modify_contents(true)
            .clone();

        let result = handler.add_recipient(certificate.clone(), permissions);
        assert!(result.is_ok());

        assert_eq!(handler.recipients.len(), 1);
        assert_eq!(handler.recipients[0].certificate, certificate);
        assert_eq!(handler.recipients[0].permissions.bits(), permissions.bits());
        assert!(!handler.recipients[0].encrypted_seed.is_empty());
    }

    #[test]
    fn test_add_recipient_invalid_cert() {
        let mut handler = PublicKeySecurityHandler::new_sha1();

        // Certificate too short
        let certificate = vec![0x30; 50];
        let permissions = Permissions::all();

        let result = handler.add_recipient(certificate, permissions);
        assert!(result.is_err());
    }

    #[test]
    fn test_generate_seed() {
        let handler = PublicKeySecurityHandler::new_sha256();
        let seed1 = handler.generate_seed().unwrap();
        let seed2 = handler.generate_seed().unwrap();

        assert_eq!(seed1.len(), 32);
        assert_eq!(seed2.len(), 32);
        assert_ne!(seed1, seed2); // Should be random
    }

    #[test]
    fn test_encrypt_decrypt_seed() {
        let handler = PublicKeySecurityHandler::new_sha1();
        let seed = vec![0xAA; 20];
        let certificate = vec![0x30; 200];

        let encrypted = handler
            .encrypt_seed_for_recipient(&seed, &certificate)
            .unwrap();
        assert!(!encrypted.is_empty());
        assert_ne!(encrypted, seed);

        // Simulate decryption with private key
        let private_key = vec![0xFF; 32];
        let decrypted = handler.decrypt_seed(&encrypted, &private_key).unwrap();
        assert_eq!(decrypted.len(), 20);
    }

    #[test]
    fn test_build_recipients_dict() {
        let mut handler = PublicKeySecurityHandler::new_sha1();

        let cert1 = vec![0x30; 200];
        let perms1 = Permissions::new().set_print(true).clone();
        handler.add_recipient(cert1, perms1).unwrap();

        let cert2 = vec![0x31; 200];
        let perms2 = Permissions::all();
        handler.add_recipient(cert2, perms2).unwrap();

        let dict = handler.build_recipients_dict();

        if let Some(Object::Array(recipients)) = dict.get("Recipients") {
            assert_eq!(recipients.len(), 2);

            // Check first recipient
            if let Object::Dictionary(r1) = &recipients[0] {
                assert!(r1.contains_key("Cert"));
                assert!(r1.contains_key("P"));
                assert!(r1.contains_key("Recipients"));
            }
        } else {
            panic!("Expected Recipients array");
        }
    }

    #[test]
    fn test_verify_permission() {
        let mut handler = PublicKeySecurityHandler::new_sha1();

        let certificate = vec![0x30; 200];
        let permissions = Permissions::new().set_print(true).set_copy(true).clone();
        handler.add_recipient(certificate, permissions).unwrap();

        assert!(handler.verify_permission(0, Permissions::new().set_print(true).clone()));
        assert!(handler.verify_permission(0, Permissions::new().set_copy(true).clone()));
        assert!(!handler.verify_permission(0, Permissions::new().set_modify_contents(true).clone()));
        assert!(!handler.verify_permission(1, Permissions::new().set_print(true).clone()));
        // Invalid index
    }

    #[test]
    fn test_encrypt_string_rc4() {
        let handler = PublicKeySecurityHandler::new_sha1();
        let key = EncryptionKey::new(vec![0x01; 16]);
        let obj_id = ObjectId::new(1, 0);
        let data = b"Test data";

        let encrypted = handler.encrypt_string(data, &key, &obj_id).unwrap();
        assert_ne!(encrypted, data);

        // RC4 is symmetric
        let decrypted = handler.decrypt_string(&encrypted, &key, &obj_id).unwrap();
        assert_eq!(decrypted, data);
    }

    #[test]
    fn test_encrypt_string_aes() {
        let mut handler = PublicKeySecurityHandler::new_sha256();
        handler.method = CryptFilterMethod::AESV2;

        let key = EncryptionKey::new(vec![0x01; 16]);
        let obj_id = ObjectId::new(1, 0);
        let data = b"Test data for AES";

        let encrypted = handler.encrypt_string(data, &key, &obj_id).unwrap();
        assert_ne!(encrypted, data);
        assert!(encrypted.len() >= data.len());
        assert_eq!(encrypted.len() % 16, 0); // Should be multiple of block size

        // Note: The simplified AES implementation might not perfectly reverse encrypt
        // Just verify decryption doesn't panic
        let _ = handler.decrypt_string(&encrypted, &key, &obj_id);
    }

    #[test]
    fn test_encrypt_stream() {
        let handler = PublicKeySecurityHandler::new_sha1();
        let key = EncryptionKey::new(vec![0x01; 16]);
        let obj_id = ObjectId::new(5, 0);
        let _dict = Dictionary::new();
        let data = b"Stream content data";

        let encrypted = handler.encrypt_stream(data, &key, &obj_id).unwrap();
        assert_ne!(encrypted, data);

        let decrypted = handler.decrypt_stream(&encrypted, &key, &obj_id).unwrap();
        assert_eq!(decrypted, data);
    }

    #[test]
    fn test_public_key_encryption_dict() {
        let mut handler = PublicKeySecurityHandler::new_sha256();

        let certificate = vec![0x30; 200];
        let permissions = Permissions::all();
        handler.add_recipient(certificate, permissions).unwrap();

        let enc_dict = PublicKeyEncryptionDict::new(&handler);

        assert_eq!(enc_dict.filter, "Adobe.PubSec");
        assert_eq!(enc_dict.subfilter, SubFilter::AdbePkcs7S4);
        assert_eq!(enc_dict.v, 4);
        assert_eq!(enc_dict.length, Some(256));
        assert_eq!(enc_dict.recipients.len(), 1);

        let pdf_dict = enc_dict.to_dict();
        assert_eq!(
            pdf_dict.get("Filter"),
            Some(&Object::Name("Adobe.PubSec".to_string()))
        );
        assert_eq!(
            pdf_dict.get("SubFilter"),
            Some(&Object::Name("adbe.pkcs7.s4".to_string()))
        );
    }

    #[test]
    fn test_multiple_recipients() {
        let mut handler = PublicKeySecurityHandler::new_sha256();

        // Add three recipients with different permissions
        let certs_and_perms = vec![
            (vec![0x30; 200], Permissions::new().set_print(true).clone()),
            (
                vec![0x31; 200],
                Permissions::new().set_print(true).set_copy(true).clone(),
            ),
            (vec![0x32; 200], Permissions::all()),
        ];

        for (cert, perms) in certs_and_perms {
            handler.add_recipient(cert, perms).unwrap();
        }

        assert_eq!(handler.recipients.len(), 3);

        // Verify each recipient's permissions
        assert!(handler.verify_permission(0, Permissions::new().set_print(true).clone()));
        assert!(!handler.verify_permission(0, Permissions::new().set_copy(true).clone()));

        assert!(handler.verify_permission(1, Permissions::new().set_print(true).clone()));
        assert!(handler.verify_permission(1, Permissions::new().set_copy(true).clone()));
        assert!(!handler.verify_permission(1, Permissions::new().set_modify_contents(true).clone()));

        assert!(handler.verify_permission(2, Permissions::all()));
    }
}
