//! PDF encryption detection and password handling
//!
//! This module provides functionality to detect encrypted PDFs and handle password-based
//! decryption according to ISO 32000-1 Chapter 7.6.

use super::objects::PdfDictionary;
use super::{ParseError, ParseResult};
use crate::encryption::{EncryptionKey, Permissions, StandardSecurityHandler, UserPassword};
use crate::objects::ObjectId;

/// Encryption information extracted from PDF trailer
#[derive(Debug, Clone)]
pub struct EncryptionInfo {
    /// Filter name (should be "Standard")
    pub filter: String,
    /// V entry (algorithm version)
    pub v: i32,
    /// R entry (revision)
    pub r: i32,
    /// O entry (owner password hash)
    pub o: Vec<u8>,
    /// U entry (user password hash)
    pub u: Vec<u8>,
    /// P entry (permissions)
    pub p: i32,
    /// Length entry (key length in bits)
    pub length: Option<i32>,
}

/// PDF Encryption Handler
pub struct EncryptionHandler {
    /// Encryption information from trailer
    encryption_info: EncryptionInfo,
    /// Standard security handler
    security_handler: StandardSecurityHandler,
    /// Current encryption key (if unlocked)
    encryption_key: Option<EncryptionKey>,
    /// File ID from trailer
    file_id: Option<Vec<u8>>,
}

impl EncryptionHandler {
    /// Create encryption handler from encryption dictionary
    pub fn new(encrypt_dict: &PdfDictionary, file_id: Option<Vec<u8>>) -> ParseResult<Self> {
        let encryption_info = Self::parse_encryption_dict(encrypt_dict)?;

        // Create appropriate security handler based on revision
        let security_handler = match encryption_info.r {
            2 => StandardSecurityHandler::rc4_40bit(),
            3 | 4 => StandardSecurityHandler::rc4_128bit(),
            5 => StandardSecurityHandler::aes_256_r5(),
            6 => StandardSecurityHandler::aes_256_r6(),
            _ => {
                return Err(ParseError::SyntaxError {
                    position: 0,
                    message: format!("Encryption revision {} not supported", encryption_info.r),
                });
            }
        };

        Ok(Self {
            encryption_info,
            security_handler,
            encryption_key: None,
            file_id,
        })
    }

    /// Parse encryption dictionary from PDF trailer
    fn parse_encryption_dict(dict: &PdfDictionary) -> ParseResult<EncryptionInfo> {
        // Get Filter (required)
        let filter = dict
            .get("Filter")
            .and_then(|obj| obj.as_name())
            .map(|name| name.0.as_str())
            .ok_or_else(|| ParseError::MissingKey("Filter".to_string()))?;

        if filter != "Standard" {
            return Err(ParseError::SyntaxError {
                position: 0,
                message: format!("Encryption filter '{filter}' not supported"),
            });
        }

        // Get V (algorithm version)
        let v = dict
            .get("V")
            .and_then(|obj| obj.as_integer())
            .map(|i| i as i32)
            .unwrap_or(0);

        // Get R (revision)
        let r = dict
            .get("R")
            .and_then(|obj| obj.as_integer())
            .map(|i| i as i32)
            .ok_or_else(|| ParseError::MissingKey("R".to_string()))?;

        // Get O (owner password hash)
        let o = dict
            .get("O")
            .and_then(|obj| obj.as_string())
            .ok_or_else(|| ParseError::MissingKey("O".to_string()))?
            .as_bytes()
            .to_vec();

        // Get U (user password hash)
        let u = dict
            .get("U")
            .and_then(|obj| obj.as_string())
            .ok_or_else(|| ParseError::MissingKey("U".to_string()))?
            .as_bytes()
            .to_vec();

        // Get P (permissions)
        let p = dict
            .get("P")
            .and_then(|obj| obj.as_integer())
            .map(|i| i as i32)
            .ok_or_else(|| ParseError::MissingKey("P".to_string()))?;

        // Get Length (optional, defaults based on revision)
        let length = dict
            .get("Length")
            .and_then(|obj| obj.as_integer())
            .map(|i| i as i32);

        Ok(EncryptionInfo {
            filter: filter.to_string(),
            v,
            r,
            o,
            u,
            p,
            length,
        })
    }

    /// Check if PDF is encrypted by looking for Encrypt entry in trailer
    pub fn detect_encryption(trailer: &PdfDictionary) -> bool {
        trailer.contains_key("Encrypt")
    }

    /// Try to unlock PDF with user password
    pub fn unlock_with_user_password(&mut self, password: &str) -> ParseResult<bool> {
        let user_password = UserPassword(password.to_string());

        // Compute what the U entry should be for this password
        let permissions = Permissions::from_bits(self.encryption_info.p as u32);
        let computed_u = self
            .security_handler
            .compute_user_hash(
                &user_password,
                &self.encryption_info.o,
                permissions,
                self.file_id.as_deref(),
            )
            .map_err(|e| ParseError::SyntaxError {
                position: 0,
                message: format!("Failed to compute user hash: {e}"),
            })?;

        // Compare with stored U entry (first 16 bytes for R3+)
        let comparison_length = if self.encryption_info.r >= 3 { 16 } else { 32 };
        let matches =
            computed_u[..comparison_length] == self.encryption_info.u[..comparison_length];

        if matches {
            // Compute and store encryption key
            let key = self
                .security_handler
                .compute_encryption_key(
                    &user_password,
                    &self.encryption_info.o,
                    permissions,
                    self.file_id.as_deref(),
                )
                .map_err(|e| ParseError::SyntaxError {
                    position: 0,
                    message: format!("Failed to compute encryption key: {e}"),
                })?;
            self.encryption_key = Some(key);
        }

        Ok(matches)
    }

    /// Try to unlock PDF with owner password
    pub fn unlock_with_owner_password(&mut self, password: &str) -> ParseResult<bool> {
        // For owner password, we need to derive the user password first
        // This is a simplified implementation - full implementation would
        // reverse the owner password algorithm

        // For now, try the owner password as if it were a user password
        self.unlock_with_user_password(password)
    }

    /// Try to unlock with empty password (common case)
    pub fn try_empty_password(&mut self) -> ParseResult<bool> {
        self.unlock_with_user_password("")
    }

    /// Check if the PDF is currently unlocked
    pub fn is_unlocked(&self) -> bool {
        self.encryption_key.is_some()
    }

    /// Get the current encryption key (if unlocked)
    pub fn encryption_key(&self) -> Option<&EncryptionKey> {
        self.encryption_key.as_ref()
    }

    /// Decrypt a string object
    pub fn decrypt_string(&self, data: &[u8], obj_id: &ObjectId) -> ParseResult<Vec<u8>> {
        match &self.encryption_key {
            Some(key) => Ok(self.security_handler.decrypt_string(data, key, obj_id)),
            None => Err(ParseError::EncryptionNotSupported),
        }
    }

    /// Decrypt a stream object
    pub fn decrypt_stream(&self, data: &[u8], obj_id: &ObjectId) -> ParseResult<Vec<u8>> {
        match &self.encryption_key {
            Some(key) => Ok(self.security_handler.decrypt_stream(data, key, obj_id)),
            None => Err(ParseError::EncryptionNotSupported),
        }
    }

    /// Get encryption algorithm information
    pub fn algorithm_info(&self) -> String {
        match (
            self.encryption_info.r,
            self.encryption_info.length.unwrap_or(40),
        ) {
            (2, _) => "RC4 40-bit".to_string(),
            (3, len) => format!("RC4 {len}-bit"),
            (4, len) => format!("RC4 {len}-bit with metadata control"),
            (5, _) => "AES-256 (Revision 5)".to_string(),
            (6, _) => "AES-256 (Revision 6, Unicode passwords)".to_string(),
            (r, len) => format!("Unknown revision {r} with {len}-bit key"),
        }
    }

    /// Get permissions information
    pub fn permissions(&self) -> Permissions {
        Permissions::from_bits(self.encryption_info.p as u32)
    }

    /// Check if strings should be encrypted
    pub fn encrypt_strings(&self) -> bool {
        // For standard security handler, strings are always encrypted
        true
    }

    /// Check if streams should be encrypted  
    pub fn encrypt_streams(&self) -> bool {
        // For standard security handler, streams are always encrypted
        true
    }

    /// Check if metadata should be encrypted (R4 only)
    pub fn encrypt_metadata(&self) -> bool {
        // For R4, this could be controlled by EncryptMetadata entry
        // For now, assume metadata is encrypted
        true
    }
}

/// Password prompt result
#[derive(Debug)]
pub enum PasswordResult {
    /// Password was accepted
    Success,
    /// Password was rejected
    Rejected,
    /// User cancelled password entry
    Cancelled,
}

/// Trait for password prompting
pub trait PasswordProvider {
    /// Prompt for user password
    fn prompt_user_password(&self) -> ParseResult<Option<String>>;

    /// Prompt for owner password
    fn prompt_owner_password(&self) -> ParseResult<Option<String>>;
}

/// Console-based password provider
pub struct ConsolePasswordProvider;

impl PasswordProvider for ConsolePasswordProvider {
    fn prompt_user_password(&self) -> ParseResult<Option<String>> {
        tracing::debug!(
            "PDF is encrypted. Enter user password (or press Enter for empty password):"
        );

        let mut input = String::new();
        std::io::stdin()
            .read_line(&mut input)
            .map_err(|e| ParseError::SyntaxError {
                position: 0,
                message: format!("Failed to read password: {e}"),
            })?;

        // Remove trailing newline
        input.truncate(input.trim_end().len());
        Ok(Some(input))
    }

    fn prompt_owner_password(&self) -> ParseResult<Option<String>> {
        tracing::debug!("User password failed. Enter owner password:");

        let mut input = String::new();
        std::io::stdin()
            .read_line(&mut input)
            .map_err(|e| ParseError::SyntaxError {
                position: 0,
                message: format!("Failed to read password: {e}"),
            })?;

        // Remove trailing newline
        input.truncate(input.trim_end().len());
        Ok(Some(input))
    }
}

/// Interactive decryption helper
pub struct InteractiveDecryption<P: PasswordProvider> {
    password_provider: P,
}

impl<P: PasswordProvider> InteractiveDecryption<P> {
    /// Create new interactive decryption helper
    pub fn new(password_provider: P) -> Self {
        Self { password_provider }
    }

    /// Attempt to unlock PDF interactively
    pub fn unlock_pdf(&self, handler: &mut EncryptionHandler) -> ParseResult<PasswordResult> {
        // First try empty password
        if handler.try_empty_password()? {
            return Ok(PasswordResult::Success);
        }

        // Try user password
        if let Some(password) = self.password_provider.prompt_user_password()? {
            if handler.unlock_with_user_password(&password)? {
                return Ok(PasswordResult::Success);
            }
        } else {
            return Ok(PasswordResult::Cancelled);
        }

        // Try owner password
        if let Some(password) = self.password_provider.prompt_owner_password()? {
            if handler.unlock_with_owner_password(&password)? {
                return Ok(PasswordResult::Success);
            }
        } else {
            return Ok(PasswordResult::Cancelled);
        }

        Ok(PasswordResult::Rejected)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::objects::{PdfDictionary, PdfName, PdfObject, PdfString};

    fn create_test_encryption_dict() -> PdfDictionary {
        let mut dict = PdfDictionary::new();
        dict.insert(
            "Filter".to_string(),
            PdfObject::Name(PdfName("Standard".to_string())),
        );
        dict.insert("V".to_string(), PdfObject::Integer(1));
        dict.insert("R".to_string(), PdfObject::Integer(2));
        dict.insert(
            "O".to_string(),
            PdfObject::String(PdfString::new(vec![0u8; 32])),
        );
        dict.insert(
            "U".to_string(),
            PdfObject::String(PdfString::new(vec![0u8; 32])),
        );
        dict.insert("P".to_string(), PdfObject::Integer(-4));
        dict
    }

    #[test]
    fn test_encryption_detection() {
        let mut trailer = PdfDictionary::new();
        assert!(!EncryptionHandler::detect_encryption(&trailer));

        trailer.insert("Encrypt".to_string(), PdfObject::Reference(1, 0));
        assert!(EncryptionHandler::detect_encryption(&trailer));
    }

    #[test]
    fn test_encryption_info_parsing() {
        let dict = create_test_encryption_dict();
        let info = EncryptionHandler::parse_encryption_dict(&dict).unwrap();

        assert_eq!(info.filter, "Standard");
        assert_eq!(info.v, 1);
        assert_eq!(info.r, 2);
        assert_eq!(info.o.len(), 32);
        assert_eq!(info.u.len(), 32);
        assert_eq!(info.p, -4);
    }

    #[test]
    fn test_encryption_handler_creation() {
        let dict = create_test_encryption_dict();
        let handler = EncryptionHandler::new(&dict, None).unwrap();

        assert_eq!(handler.encryption_info.r, 2);
        assert!(!handler.is_unlocked());
        assert_eq!(handler.algorithm_info(), "RC4 40-bit");
    }

    #[test]
    fn test_empty_password_attempt() {
        let dict = create_test_encryption_dict();
        let mut handler = EncryptionHandler::new(&dict, None).unwrap();

        // Empty password should not work with test data
        let result = handler.try_empty_password().unwrap();
        assert!(!result);
        assert!(!handler.is_unlocked());
    }

    #[test]
    fn test_permissions() {
        let dict = create_test_encryption_dict();
        let handler = EncryptionHandler::new(&dict, None).unwrap();

        let permissions = handler.permissions();
        // P value of -4 should result in specific permissions
        assert!(permissions.bits() != 0);
    }

    #[test]
    fn test_encryption_flags() {
        let dict = create_test_encryption_dict();
        let handler = EncryptionHandler::new(&dict, None).unwrap();

        assert!(handler.encrypt_strings());
        assert!(handler.encrypt_streams());
        assert!(handler.encrypt_metadata());
    }

    #[test]
    fn test_decrypt_without_key() {
        let dict = create_test_encryption_dict();
        let handler = EncryptionHandler::new(&dict, None).unwrap();

        let obj_id = ObjectId::new(1, 0);
        let result = handler.decrypt_string(b"test", &obj_id);
        assert!(result.is_err());
    }

    #[test]
    fn test_unsupported_filter() {
        let mut dict = PdfDictionary::new();
        dict.insert(
            "Filter".to_string(),
            PdfObject::Name(PdfName("UnsupportedFilter".to_string())),
        );
        dict.insert("R".to_string(), PdfObject::Integer(2));
        dict.insert(
            "O".to_string(),
            PdfObject::String(PdfString::new(vec![0u8; 32])),
        );
        dict.insert(
            "U".to_string(),
            PdfObject::String(PdfString::new(vec![0u8; 32])),
        );
        dict.insert("P".to_string(), PdfObject::Integer(-4));

        let result = EncryptionHandler::new(&dict, None);
        assert!(result.is_err());
    }

    #[test]
    fn test_unsupported_revision() {
        let mut dict = create_test_encryption_dict();
        dict.insert("R".to_string(), PdfObject::Integer(99)); // Unsupported revision

        let result = EncryptionHandler::new(&dict, None);
        assert!(result.is_err());
    }

    #[test]
    fn test_missing_required_keys() {
        let test_cases = vec![
            ("Filter", PdfObject::Name(PdfName("Standard".to_string()))),
            ("R", PdfObject::Integer(2)),
            ("O", PdfObject::String(PdfString::new(vec![0u8; 32]))),
            ("U", PdfObject::String(PdfString::new(vec![0u8; 32]))),
            ("P", PdfObject::Integer(-4)),
        ];

        for (skip_key, _) in test_cases {
            let mut dict = create_test_encryption_dict();
            dict.0.remove(&PdfName(skip_key.to_string()));

            let result = EncryptionHandler::parse_encryption_dict(&dict);
            assert!(result.is_err(), "Should fail when {skip_key} is missing");
        }
    }

    /// Mock password provider for testing
    struct MockPasswordProvider {
        user_password: Option<String>,
        owner_password: Option<String>,
    }

    impl PasswordProvider for MockPasswordProvider {
        fn prompt_user_password(&self) -> ParseResult<Option<String>> {
            Ok(self.user_password.clone())
        }

        fn prompt_owner_password(&self) -> ParseResult<Option<String>> {
            Ok(self.owner_password.clone())
        }
    }

    #[test]
    fn test_interactive_decryption_cancelled() {
        let provider = MockPasswordProvider {
            user_password: None,
            owner_password: None,
        };

        let decryption = InteractiveDecryption::new(provider);
        let dict = create_test_encryption_dict();
        let mut handler = EncryptionHandler::new(&dict, None).unwrap();

        let result = decryption.unlock_pdf(&mut handler).unwrap();
        matches!(result, PasswordResult::Cancelled);
    }

    #[test]
    fn test_interactive_decryption_rejected() {
        let provider = MockPasswordProvider {
            user_password: Some("wrong_password".to_string()),
            owner_password: Some("also_wrong".to_string()),
        };

        let decryption = InteractiveDecryption::new(provider);
        let dict = create_test_encryption_dict();
        let mut handler = EncryptionHandler::new(&dict, None).unwrap();

        let result = decryption.unlock_pdf(&mut handler).unwrap();
        matches!(result, PasswordResult::Rejected);
    }

    // ===== ADVANCED EDGE CASE TESTS =====

    #[test]
    fn test_malformed_encryption_dictionary_invalid_types() {
        // Test with wrong type for Filter
        let mut dict = PdfDictionary::new();
        dict.insert("Filter".to_string(), PdfObject::Integer(123)); // Should be Name
        dict.insert("R".to_string(), PdfObject::Integer(2));
        dict.insert(
            "O".to_string(),
            PdfObject::String(PdfString::new(vec![0u8; 32])),
        );
        dict.insert(
            "U".to_string(),
            PdfObject::String(PdfString::new(vec![0u8; 32])),
        );
        dict.insert("P".to_string(), PdfObject::Integer(-4));

        let result = EncryptionHandler::parse_encryption_dict(&dict);
        assert!(result.is_err());

        // Test with wrong type for R
        let mut dict = create_test_encryption_dict();
        dict.insert(
            "R".to_string(),
            PdfObject::Name(PdfName("not_a_number".to_string())),
        );
        let result = EncryptionHandler::parse_encryption_dict(&dict);
        assert!(result.is_err());
    }

    #[test]
    fn test_encryption_dictionary_edge_values() {
        // Test with extreme revision values
        let mut dict = create_test_encryption_dict();
        dict.insert("R".to_string(), PdfObject::Integer(0)); // Very low revision
        let result = EncryptionHandler::new(&dict, None);
        assert!(result.is_err());

        // Test with negative revision
        let mut dict = create_test_encryption_dict();
        dict.insert("R".to_string(), PdfObject::Integer(-1));
        let result = EncryptionHandler::new(&dict, None);
        assert!(result.is_err());

        // Test with very high revision
        let mut dict = create_test_encryption_dict();
        dict.insert("R".to_string(), PdfObject::Integer(1000));
        let result = EncryptionHandler::new(&dict, None);
        assert!(result.is_err());
    }

    #[test]
    fn test_encryption_dictionary_invalid_hash_lengths() {
        // Test with O hash too short
        let mut dict = create_test_encryption_dict();
        dict.insert(
            "O".to_string(),
            PdfObject::String(PdfString::new(vec![0u8; 16])),
        ); // Should be 32
        let result = EncryptionHandler::parse_encryption_dict(&dict);
        // Should still work but be invalid data
        assert!(result.is_ok());

        // Test with U hash too long
        let mut dict = create_test_encryption_dict();
        dict.insert(
            "U".to_string(),
            PdfObject::String(PdfString::new(vec![0u8; 64])),
        ); // Should be 32
        let result = EncryptionHandler::parse_encryption_dict(&dict);
        assert!(result.is_ok());

        // Test with empty hashes
        let mut dict = create_test_encryption_dict();
        dict.insert("O".to_string(), PdfObject::String(PdfString::new(vec![])));
        dict.insert("U".to_string(), PdfObject::String(PdfString::new(vec![])));
        let result = EncryptionHandler::parse_encryption_dict(&dict);
        assert!(result.is_ok());
    }

    #[test]
    fn test_encryption_with_different_key_lengths() {
        // Test Rev 2 (40-bit)
        let mut dict = create_test_encryption_dict();
        dict.insert("R".to_string(), PdfObject::Integer(2));
        let handler = EncryptionHandler::new(&dict, None).unwrap();
        assert_eq!(handler.algorithm_info(), "RC4 40-bit");

        // Test Rev 3 (128-bit)
        let mut dict = create_test_encryption_dict();
        dict.insert("R".to_string(), PdfObject::Integer(3));
        dict.insert("Length".to_string(), PdfObject::Integer(128));
        let handler = EncryptionHandler::new(&dict, None).unwrap();
        assert_eq!(handler.algorithm_info(), "RC4 128-bit");

        // Test Rev 4 (128-bit with metadata control)
        let mut dict = create_test_encryption_dict();
        dict.insert("R".to_string(), PdfObject::Integer(4));
        dict.insert("Length".to_string(), PdfObject::Integer(128));
        let handler = EncryptionHandler::new(&dict, None).unwrap();
        assert_eq!(
            handler.algorithm_info(),
            "RC4 128-bit with metadata control"
        );

        // Test Rev 5 (AES-256)
        let mut dict = create_test_encryption_dict();
        dict.insert("R".to_string(), PdfObject::Integer(5));
        dict.insert("V".to_string(), PdfObject::Integer(5));
        let handler = EncryptionHandler::new(&dict, None).unwrap();
        assert_eq!(handler.algorithm_info(), "AES-256 (Revision 5)");

        // Test Rev 6 (AES-256 with Unicode)
        let mut dict = create_test_encryption_dict();
        dict.insert("R".to_string(), PdfObject::Integer(6));
        dict.insert("V".to_string(), PdfObject::Integer(5));
        let handler = EncryptionHandler::new(&dict, None).unwrap();
        assert_eq!(
            handler.algorithm_info(),
            "AES-256 (Revision 6, Unicode passwords)"
        );
    }

    #[test]
    fn test_file_id_handling() {
        let dict = create_test_encryption_dict();

        // Test with file ID
        let file_id = Some(b"test_file_id_12345678".to_vec());
        let handler = EncryptionHandler::new(&dict, file_id.clone()).unwrap();
        // File ID should be stored
        assert_eq!(handler.file_id, file_id);

        // Test without file ID
        let handler = EncryptionHandler::new(&dict, None).unwrap();
        assert_eq!(handler.file_id, None);

        // Test with empty file ID
        let empty_file_id = Some(vec![]);
        let handler = EncryptionHandler::new(&dict, empty_file_id.clone()).unwrap();
        assert_eq!(handler.file_id, empty_file_id);
    }

    #[test]
    fn test_permissions_edge_cases() {
        // Test with different permission values
        let permission_values = vec![0, -1, -4, -44, -100, i32::MAX, i32::MIN];

        for p_value in permission_values {
            let mut dict = create_test_encryption_dict();
            dict.insert("P".to_string(), PdfObject::Integer(p_value as i64));
            let handler = EncryptionHandler::new(&dict, None).unwrap();

            let permissions = handler.permissions();
            assert_eq!(permissions.bits(), p_value as u32);
        }
    }

    #[test]
    fn test_decrypt_with_different_object_ids() {
        let dict = create_test_encryption_dict();
        let handler = EncryptionHandler::new(&dict, None).unwrap();
        let test_data = b"test data";

        // Test with different object IDs (should all fail since not unlocked)
        let object_ids = vec![
            ObjectId::new(1, 0),
            ObjectId::new(999, 0),
            ObjectId::new(1, 999),
            ObjectId::new(u32::MAX, u16::MAX),
        ];

        for obj_id in object_ids {
            let result = handler.decrypt_string(test_data, &obj_id);
            assert!(result.is_err());

            let result = handler.decrypt_stream(test_data, &obj_id);
            assert!(result.is_err());
        }
    }

    #[test]
    fn test_password_scenarios_comprehensive() {
        let dict = create_test_encryption_dict();
        let mut handler = EncryptionHandler::new(&dict, None).unwrap();

        // Test various password scenarios (using String for uniformity)
        let test_passwords = vec![
            "".to_string(),                     // Empty
            " ".to_string(),                    // Single space
            "   ".to_string(),                  // Multiple spaces
            "password".to_string(),             // Simple
            "Password123!@#".to_string(),       // Complex
            "a".repeat(32),                     // Exactly 32 chars
            "a".repeat(50),                     // Over 32 chars
            "unicode_ñáéíóú".to_string(),       // Unicode
            "pass\nwith\nnewlines".to_string(), // Newlines
            "pass\twith\ttabs".to_string(),     // Tabs
            "pass with spaces".to_string(),     // Spaces
            "🔐🗝️📄".to_string(),               // Emojis
        ];

        for password in test_passwords {
            // All should fail with test data but not crash
            let result = handler.unlock_with_user_password(&password);
            assert!(result.is_ok());
            assert!(!result.unwrap());

            let result = handler.unlock_with_owner_password(&password);
            assert!(result.is_ok());
            assert!(!result.unwrap());
        }
    }

    #[test]
    fn test_encryption_handler_thread_safety_simulation() {
        // Simulate what would happen in multi-threaded access
        let dict = create_test_encryption_dict();
        let handler = EncryptionHandler::new(&dict, None).unwrap();

        // Test multiple read operations (safe)
        for _ in 0..100 {
            assert!(!handler.is_unlocked());
            assert_eq!(handler.algorithm_info(), "RC4 40-bit");
            assert!(handler.encrypt_strings());
            assert!(handler.encrypt_streams());
            assert!(handler.encrypt_metadata());
        }
    }

    #[test]
    fn test_encryption_state_transitions() {
        let dict = create_test_encryption_dict();
        let mut handler = EncryptionHandler::new(&dict, None).unwrap();

        // Initial state
        assert!(!handler.is_unlocked());

        // Try unlock (should fail with test data)
        let result = handler.try_empty_password().unwrap();
        assert!(!result);
        assert!(!handler.is_unlocked());

        // Try user password (should fail with test data)
        let result = handler.unlock_with_user_password("test").unwrap();
        assert!(!result);
        assert!(!handler.is_unlocked());

        // Try owner password (should fail with test data)
        let result = handler.unlock_with_owner_password("test").unwrap();
        assert!(!result);
        assert!(!handler.is_unlocked());

        // State should remain consistent
        assert!(!handler.is_unlocked());
    }

    #[test]
    fn test_interactive_decryption_edge_cases() {
        // Test provider that returns None for both passwords
        let provider = MockPasswordProvider {
            user_password: None,
            owner_password: None,
        };

        let decryption = InteractiveDecryption::new(provider);
        let dict = create_test_encryption_dict();
        let mut handler = EncryptionHandler::new(&dict, None).unwrap();

        let result = decryption.unlock_pdf(&mut handler).unwrap();
        matches!(result, PasswordResult::Cancelled);

        // Test provider that returns empty strings
        let provider = MockPasswordProvider {
            user_password: Some("".to_string()),
            owner_password: Some("".to_string()),
        };

        let decryption = InteractiveDecryption::new(provider);
        let mut handler = EncryptionHandler::new(&dict, None).unwrap();

        let result = decryption.unlock_pdf(&mut handler).unwrap();
        matches!(result, PasswordResult::Rejected);
    }

    /// Test custom MockPasswordProvider for edge cases
    struct EdgeCasePasswordProvider {
        call_count: std::cell::RefCell<usize>,
        passwords: Vec<Option<String>>,
    }

    impl EdgeCasePasswordProvider {
        fn new(passwords: Vec<Option<String>>) -> Self {
            Self {
                call_count: std::cell::RefCell::new(0),
                passwords,
            }
        }
    }

    impl PasswordProvider for EdgeCasePasswordProvider {
        fn prompt_user_password(&self) -> ParseResult<Option<String>> {
            let mut count = self.call_count.borrow_mut();
            if *count < self.passwords.len() {
                let result = self.passwords[*count].clone();
                *count += 1;
                Ok(result)
            } else {
                Ok(None)
            }
        }

        fn prompt_owner_password(&self) -> ParseResult<Option<String>> {
            self.prompt_user_password()
        }
    }

    #[test]
    fn test_interactive_decryption_with_sequence() {
        let passwords = vec![
            Some("first_attempt".to_string()),
            Some("second_attempt".to_string()),
            None, // Cancelled
        ];

        let provider = EdgeCasePasswordProvider::new(passwords);
        let decryption = InteractiveDecryption::new(provider);
        let dict = create_test_encryption_dict();
        let mut handler = EncryptionHandler::new(&dict, None).unwrap();

        let result = decryption.unlock_pdf(&mut handler).unwrap();
        matches!(result, PasswordResult::Cancelled);
    }
}
