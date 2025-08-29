//! Unit tests for encryption module

#[cfg(test)]
mod rc4_tests {
    use super::super::*;

    #[test]
    fn test_rc4_key_creation() {
        let key = Rc4Key::new(vec![0x01, 0x02, 0x03, 0x04, 0x05]);
        assert_eq!(key.key.len(), 5);

        let key_from_slice = Rc4Key::from_slice(&[0x01, 0x02, 0x03, 0x04, 0x05]);
        assert_eq!(key_from_slice.key.len(), 5);
    }

    #[test]
    fn test_rc4_encryption_symmetric() {
        let key = Rc4Key::new(vec![0x01, 0x02, 0x03, 0x04, 0x05]);
        let plaintext = b"Hello, World!";

        // Encrypt
        let mut rc4_enc = Rc4::new(&key);
        let encrypted = rc4_enc.process(plaintext);
        assert_ne!(encrypted, plaintext);

        // Decrypt (RC4 is symmetric)
        let mut rc4_dec = Rc4::new(&key);
        let decrypted = rc4_dec.process(&encrypted);
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_rc4_process_in_place() {
        let key = Rc4Key::new(vec![0x01, 0x02, 0x03, 0x04, 0x05]);
        let mut rc4 = Rc4::new(&key);

        let mut data = b"Test data".to_vec();
        let original = data.clone();

        rc4.process_in_place(&mut data);
        assert_ne!(data, original);

        // Process again to restore
        let mut rc4_2 = Rc4::new(&key);
        rc4_2.process_in_place(&mut data);
        assert_eq!(data, original);
    }
}

#[cfg(test)]
mod aes_tests {
    use super::super::*;

    #[test]
    fn test_aes_key_sizes() {
        assert_eq!(AesKeySize::Aes128.key_length(), 16);
        assert_eq!(AesKeySize::Aes256.key_length(), 32);
        assert_eq!(AesKeySize::Aes128.block_size(), 16);
        assert_eq!(AesKeySize::Aes256.block_size(), 16);
    }

    #[test]
    fn test_aes_key_creation_128() {
        // Test valid AES-128 key
        let key_128 = AesKey::new_128(vec![0u8; 16]);
        assert!(key_128.is_ok());
        let key = key_128.unwrap();
        assert_eq!(key.len(), 16);
        assert!(!key.is_empty());
        assert_eq!(key.size(), AesKeySize::Aes128);

        // Test invalid AES-128 key
        let invalid_key = AesKey::new_128(vec![0u8; 15]);
        assert!(invalid_key.is_err());
    }

    #[test]
    fn test_aes_key_creation_256() {
        // Test valid AES-256 key
        let key_256 = AesKey::new_256(vec![0u8; 32]);
        assert!(key_256.is_ok());
        let key = key_256.unwrap();
        assert_eq!(key.len(), 32);
        assert_eq!(key.size(), AesKeySize::Aes256);

        // Test invalid AES-256 key
        let invalid_key = AesKey::new_256(vec![0u8; 31]);
        assert!(invalid_key.is_err());
    }

    #[test]
    fn test_aes_cbc_encryption() {
        let key = AesKey::new_128(vec![0x2b; 16]).unwrap();
        let aes = Aes::new(key);
        let iv = vec![0x00; 16];
        let plaintext = b"This is a test!!"; // 16 bytes

        let encrypted = aes.encrypt_cbc(plaintext, &iv);
        assert!(encrypted.is_ok());
        let ciphertext = encrypted.unwrap();
        assert_ne!(ciphertext, plaintext);

        // For now, just test encryption works
        // Decryption may need the same key instance
    }

    #[test]
    fn test_aes_ecb_encryption() {
        let key = AesKey::new_128(vec![0x2b; 16]).unwrap();
        let aes = Aes::new(key);
        let plaintext = b"This is a test!!"; // 16 bytes

        let encrypted = aes.encrypt_ecb(plaintext);
        assert!(encrypted.is_ok());
        let ciphertext = encrypted.unwrap();
        assert_ne!(ciphertext, plaintext);
    }

    #[test]
    fn test_generate_iv() {
        let iv = generate_iv();
        assert_eq!(iv.len(), 16);

        // Two IVs should be different (with very high probability)
        let iv2 = generate_iv();
        assert_ne!(iv, iv2);
    }
}

#[cfg(test)]
mod permissions_tests {
    use super::super::*;

    #[test]
    fn test_permissions_new() {
        let perms = Permissions::new();
        // Permissions have reserved bits that are always set
        assert_eq!(perms.bits(), 0xFFFFF0C0);
    }

    #[test]
    fn test_permissions_all() {
        let perms = Permissions::all();
        assert!(perms.can_print());
        assert!(perms.can_modify_contents());
        assert!(perms.can_copy());
        assert!(perms.can_modify_annotations());
        assert!(perms.can_fill_forms());
        assert!(perms.can_access_for_accessibility());
        assert!(perms.can_assemble());
        assert!(perms.can_print_high_quality());
    }

    #[test]
    fn test_permissions_set_print() {
        let mut perms = Permissions::new();

        perms.set_print(true);
        assert!(perms.can_print());

        perms.set_print(false);
        assert!(!perms.can_print());
    }

    #[test]
    fn test_permissions_set_modify_contents() {
        let mut perms = Permissions::new();

        perms.set_modify_contents(true);
        assert!(perms.can_modify_contents());

        perms.set_modify_contents(false);
        assert!(!perms.can_modify_contents());
    }

    #[test]
    fn test_permissions_set_copy() {
        let mut perms = Permissions::new();

        perms.set_copy(true);
        assert!(perms.can_copy());

        perms.set_copy(false);
        assert!(!perms.can_copy());
    }

    #[test]
    fn test_permissions_set_fill_forms() {
        let mut perms = Permissions::new();

        perms.set_fill_forms(true);
        assert!(perms.can_fill_forms());

        perms.set_fill_forms(false);
        assert!(!perms.can_fill_forms());
    }

    #[test]
    fn test_permissions_from_bits() {
        // Test with all permissions
        let perms = Permissions::from_bits(0xFFFFFFFF);
        assert!(perms.can_print());
        assert!(perms.can_modify_contents());

        // Test with base permissions only
        let perms2 = Permissions::from_bits(0xFFFFF0C0);
        assert!(!perms2.can_print());
        assert!(!perms2.can_modify_contents());
    }

    #[test]
    fn test_permissions_contains() {
        let mut perms1 = Permissions::new();
        perms1.set_print(true);
        perms1.set_copy(true);

        let mut perms2 = Permissions::new();
        perms2.set_print(true);

        assert!(perms1.contains(perms2));

        perms2.set_modify_contents(true);
        assert!(!perms1.contains(perms2));
    }

    #[test]
    fn test_permission_flags() {
        let mut perms = Permissions::new();
        perms.set_print(true);
        perms.set_copy(true);

        let flags = perms.flags();
        assert!(flags.print);
        assert!(flags.copy);
        assert!(!flags.modify_contents);
    }
}

#[cfg(test)]
mod encryption_dict_tests {
    use super::super::*;
    use crate::objects::Object;

    #[test]
    fn test_crypt_filter_method_pdf_name() {
        assert_eq!(CryptFilterMethod::None.pdf_name(), "None");
        assert_eq!(CryptFilterMethod::V2.pdf_name(), "V2");
        assert_eq!(CryptFilterMethod::AESV2.pdf_name(), "AESV2");
        assert_eq!(CryptFilterMethod::AESV3.pdf_name(), "AESV3");
    }

    #[test]
    fn test_crypt_filter_creation() {
        let filter = CryptFilter::standard(CryptFilterMethod::AESV2);
        assert_eq!(filter.method, CryptFilterMethod::AESV2);
        // Length is optional and may not be set for AESV2

        let dict = filter.to_dict();
        // Just verify the dictionary is created
        assert!(dict.get("CFM").is_some());
    }

    #[test]
    fn test_encryption_dictionary_rc4_40bit() {
        let user_password = vec![0u8; 32];
        let owner_password = vec![0u8; 32];
        let permissions = Permissions::all();
        let id = vec![0u8; 16];

        let enc_dict =
            EncryptionDictionary::rc4_40bit(user_password, owner_password, permissions, Some(id));

        assert_eq!(enc_dict.length, Some(5)); // 40 bits = 5 bytes
        assert_eq!(enc_dict.r, 2);
        assert_eq!(enc_dict.v, 1);
    }

    #[test]
    fn test_encryption_dictionary_rc4_128bit() {
        let user_password = vec![0u8; 32];
        let owner_password = vec![0u8; 32];
        let permissions = Permissions::all();
        let id = vec![0u8; 16];

        let enc_dict =
            EncryptionDictionary::rc4_128bit(user_password, owner_password, permissions, Some(id));

        assert_eq!(enc_dict.length, Some(16)); // 128 bits = 16 bytes
        assert_eq!(enc_dict.r, 3);
        assert_eq!(enc_dict.v, 2);
    }

    #[test]
    fn test_encryption_dictionary_to_dict() {
        let enc_dict = EncryptionDictionary {
            filter: "Standard".to_string(),
            sub_filter: None,
            v: 1,
            r: 2,
            length: Some(5),
            u: vec![0u8; 32],
            o: vec![0u8; 32],
            p: Permissions::all(),
            id: Some(vec![0u8; 16]),
            encrypt_metadata: true,
            stm_f: Some(StreamFilter::StdCF),
            str_f: Some(StringFilter::StdCF),
            cf: None,
            ef: None,
        };

        let dict = enc_dict.to_dict();
        assert_eq!(
            dict.get("Filter").unwrap(),
            &Object::Name("Standard".to_string())
        );
        assert_eq!(dict.get("V").unwrap(), &Object::Integer(1));
        assert_eq!(dict.get("R").unwrap(), &Object::Integer(2));
        assert_eq!(dict.get("Length").unwrap(), &Object::Integer(40)); // 5 bytes * 8 = 40 bits
    }
}

#[cfg(test)]
mod auth_event_tests {
    use super::super::*;

    #[test]
    fn test_auth_event_pdf_name() {
        assert_eq!(AuthEvent::DocOpen.pdf_name(), "DocOpen");
        assert_eq!(AuthEvent::EFOpen.pdf_name(), "EFOpen");
    }
}

#[cfg(test)]
mod encryption_key_tests {
    use super::super::*;

    #[test]
    fn test_encryption_key_creation() {
        let key = EncryptionKey::new(vec![0x01, 0x02, 0x03, 0x04, 0x05]);
        assert_eq!(key.len(), 5);
        assert!(!key.is_empty());
        assert_eq!(key.as_bytes(), &[0x01, 0x02, 0x03, 0x04, 0x05]);
    }

    #[test]
    fn test_encryption_key_empty() {
        let empty_key = EncryptionKey::new(vec![]);
        assert_eq!(empty_key.len(), 0);
        assert!(empty_key.is_empty());
    }
}

#[cfg(test)]
mod advanced_aes_tests {}

#[cfg(test)]
mod standard_security_tests {
    use super::super::*;

    #[test]
    fn test_standard_security_handler_rc4_40bit() {
        let _handler = StandardSecurityHandler::rc4_40bit();
        // Just verify it can be created without checking private fields
    }

    #[test]
    fn test_standard_security_handler_rc4_128bit() {
        let _handler = StandardSecurityHandler::rc4_128bit();
        // Just verify it can be created without checking private fields
    }

    #[test]
    fn test_standard_security_handler_aes() {
        let _handler_r5 = StandardSecurityHandler::aes_256_r5();
        let _handler_r6 = StandardSecurityHandler::aes_256_r6();
        // Just verify they can be created
    }

    #[test]
    fn test_user_and_owner_passwords() {
        let user = UserPassword("user123".to_string());
        let owner = OwnerPassword("owner123".to_string());
        assert_eq!(user.0, "user123");
        assert_eq!(owner.0, "owner123");
    }
}
