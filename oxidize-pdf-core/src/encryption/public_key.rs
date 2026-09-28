//! Compatibility types for unsupported certificate-based recipient encryption.
//!
//! No cryptographic operation in this module is supported. The former simulation
//! has been removed: it did not protect seeds or authenticate private keys.
//! Dictionary helpers only copy caller-supplied metadata; they do not implement
//! ISO 32000 public-key encryption. Signature certificate verification is separate.

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

/// Unvalidated recipient metadata retained for source compatibility.
///
/// Constructing this value does not encrypt a seed or validate a certificate.
#[derive(Debug, Clone)]
pub struct Recipient {
    /// Caller-supplied certificate bytes, not validated as X.509 DER.
    pub certificate: Vec<u8>,
    /// Caller-supplied permission bits; these do not grant access.
    pub permissions: Permissions,
    /// Caller-supplied recipient bytes; no encryption or validation is performed.
    pub encrypted_seed: Vec<u8>,
}

/// Compatibility shell for **unsupported** certificate-based recipient encryption.
///
/// All cryptographic methods return [`PdfError::EncryptionError`] regardless of
/// inputs or public field values. Permission checks always deny access. No seed,
/// file-encryption key or IV is generated. Constructors and dictionary helpers
/// retain their signatures but do not establish cryptographic protection.
///
/// Use the password-based [`crate::encryption::StandardSecurityHandler`] only
/// when password encryption meets the application's requirements. Certificate
/// verification for signatures does not enable recipient encryption.
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
    /// Create unsupported legacy SHA-1 configuration metadata.
    pub fn new_sha1() -> Self {
        Self {
            subfilter: SubFilter::AdbePkcs7S3,
            recipients: Vec::new(),
            seed_length: 20,
            method: CryptFilterMethod::V2,
        }
    }

    /// Create unsupported legacy SHA-256 configuration metadata.
    pub fn new_sha256() -> Self {
        Self {
            subfilter: SubFilter::AdbePkcs7S4,
            recipients: Vec::new(),
            seed_length: 32,
            method: CryptFilterMethod::AESV2,
        }
    }

    /// Reject unsupported recipient encryption without modifying the recipient list.
    ///
    /// # Errors
    /// Always returns [`PdfError::EncryptionError`], including for valid certificates.
    pub fn add_recipient(
        &mut self,
        _certificate: Vec<u8>,
        _permissions: Permissions,
    ) -> Result<()> {
        Err(unsupported_recipient_encryption())
    }

    /// Reject unsupported recipient decryption without releasing seed bytes.
    ///
    /// # Errors
    /// Always returns [`PdfError::EncryptionError`], regardless of the key or data.
    pub fn decrypt_seed(&self, _encrypted_seed: &[u8], _private_key: &[u8]) -> Result<Vec<u8>> {
        Err(unsupported_recipient_encryption())
    }

    /// Copy unvalidated caller-supplied metadata into a dictionary.
    ///
    /// This compatibility helper does not encrypt seeds, validate certificates,
    /// or create a supported PDF encryption dictionary. Its return value must not
    /// be used as evidence that any document or recipient data is protected.
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

    /// Always deny access: recipient metadata cannot authenticate a caller.
    pub fn verify_permission(&self, _recipient_index: usize, _permission: Permissions) -> bool {
        false
    }
}

fn unsupported_recipient_encryption() -> PdfError {
    PdfError::EncryptionError("Certificate-based recipient encryption is not supported".to_string())
}

impl SecurityHandler for PublicKeySecurityHandler {
    fn encrypt_string(
        &self,
        _data: &[u8],
        _encryption_key: &EncryptionKey,
        _obj_id: &ObjectId,
    ) -> Result<Vec<u8>> {
        Err(unsupported_recipient_encryption())
    }

    fn decrypt_string(
        &self,
        _data: &[u8],
        _encryption_key: &EncryptionKey,
        _obj_id: &ObjectId,
    ) -> Result<Vec<u8>> {
        Err(unsupported_recipient_encryption())
    }

    fn encrypt_stream(
        &self,
        _data: &[u8],
        _encryption_key: &EncryptionKey,
        _obj_id: &ObjectId,
    ) -> Result<Vec<u8>> {
        Err(unsupported_recipient_encryption())
    }

    fn decrypt_stream(
        &self,
        _data: &[u8],
        _encryption_key: &EncryptionKey,
        _obj_id: &ObjectId,
    ) -> Result<Vec<u8>> {
        Err(unsupported_recipient_encryption())
    }

    fn encrypt_string_aes(
        &self,
        _data: &[u8],
        _encryption_key: &EncryptionKey,
        _obj_id: &ObjectId,
        _bits: u32,
    ) -> Result<Vec<u8>> {
        Err(unsupported_recipient_encryption())
    }

    fn decrypt_string_aes(
        &self,
        _data: &[u8],
        _encryption_key: &EncryptionKey,
        _obj_id: &ObjectId,
        _bits: u32,
    ) -> Result<Vec<u8>> {
        Err(unsupported_recipient_encryption())
    }

    fn encrypt_stream_aes(
        &self,
        _data: &[u8],
        _encryption_key: &EncryptionKey,
        _obj_id: &ObjectId,
        _bits: u32,
    ) -> Result<Vec<u8>> {
        Err(unsupported_recipient_encryption())
    }

    fn decrypt_stream_aes(
        &self,
        _data: &[u8],
        _encryption_key: &EncryptionKey,
        _obj_id: &ObjectId,
        _bits: u32,
    ) -> Result<Vec<u8>> {
        Err(unsupported_recipient_encryption())
    }
}

/// Raw legacy public-key dictionary metadata, **not** a supported encryption profile.
///
/// These fields and serializers are retained for source compatibility. No
/// certificate validation, seed encryption or file-key derivation is performed.
/// Serializing this value does not establish recipient protection.
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
    /// Copy handler metadata without validating it or performing encryption.
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

    /// Serialize raw metadata; this does not produce a validated encryption dictionary.
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
}
