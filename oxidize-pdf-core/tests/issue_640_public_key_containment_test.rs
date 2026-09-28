//! Regression coverage for the unsupported recipient-encryption API (#640).
use oxidize_pdf::encryption::{
    Aes, AesKey, CryptFilterManager, CryptFilterMethod, EncryptionKey, Permissions,
    PublicKeySecurityHandler, Recipient, SecurityHandler, StandardSecurityHandler,
};
use oxidize_pdf::objects::{Dictionary, ObjectId};
use oxidize_pdf::PdfError;

fn assert_unsupported<T: std::fmt::Debug>(result: Result<T, PdfError>) {
    match result {
        Err(PdfError::EncryptionError(message)) => {
            assert!(
                message.contains("recipient encryption is not supported"),
                "{message}"
            );
        }
        other => panic!("expected explicit unsupported recipient-encryption error, got {other:?}"),
    }
}

#[test]
fn adding_invalid_or_real_certificates_never_creates_a_recipient() {
    for mut handler in [
        PublicKeySecurityHandler::new_sha1(),
        PublicKeySecurityHandler::new_sha256(),
    ] {
        for certificate in [
            vec![0; 100],
            vec![],
            vec![0x30; 200],
            include_bytes!("fixtures/signatures/cms_root.der").to_vec(),
        ] {
            assert_unsupported(handler.add_recipient(certificate, Permissions::all()));
            assert!(
                handler.recipients.is_empty(),
                "unsupported encryption must not publish a seed"
            );
        }
    }
}

#[test]
fn failed_add_preserves_preexisting_recipient_data() {
    let mut handler = PublicKeySecurityHandler::new_sha256();
    handler.recipients.push(Recipient {
        certificate: vec![1, 2, 3],
        permissions: Permissions::all(),
        encrypted_seed: vec![4, 5, 6],
    });
    assert_unsupported(handler.add_recipient(vec![0; 100], Permissions::default()));
    assert_eq!(handler.recipients.len(), 1);
    assert_eq!(handler.recipients[0].certificate, [1, 2, 3]);
    assert_eq!(handler.recipients[0].encrypted_seed, [4, 5, 6]);
    assert_eq!(
        handler.recipients[0].permissions.bits(),
        Permissions::all().bits()
    );
}

#[test]
fn malformed_recipients_and_unrelated_keys_never_release_seed_bytes() {
    for handler in [
        PublicKeySecurityHandler::new_sha1(),
        PublicKeySecurityHandler::new_sha256(),
    ] {
        for recipient in [vec![0xAA; 36], vec![], vec![0; 19], vec![0xAA; 24]] {
            for key in [b"a".as_slice(), b"b", b""] {
                assert_unsupported(handler.decrypt_seed(&recipient, key));
            }
        }
    }
}

#[test]
fn injected_recipient_permissions_do_not_authorize_access() {
    let mut handler = PublicKeySecurityHandler::new_sha1();
    handler.recipients.push(Recipient {
        certificate: vec![0; 100],
        permissions: Permissions::all(),
        encrypted_seed: vec![0xAA; 24],
    });
    for index in [0, 1, usize::MAX] {
        assert!(!handler.verify_permission(index, Permissions::all()));
        assert!(!handler.verify_permission(index, Permissions::default()));
    }
}

fn configured_handler(method: CryptFilterMethod) -> PublicKeySecurityHandler {
    let mut handler = PublicKeySecurityHandler::new_sha256();
    // Public fields cannot be used to bypass containment.
    handler.method = method;
    handler.seed_length = usize::MAX;
    handler.recipients.push(Recipient {
        certificate: vec![0; 100],
        permissions: Permissions::all(),
        encrypted_seed: vec![0xAA; 36],
    });
    handler
}

#[test]
fn encrypt_string_rejects_every_method_through_trait_object() {
    for method in [
        CryptFilterMethod::V2,
        CryptFilterMethod::AESV2,
        CryptFilterMethod::AESV3,
        CryptFilterMethod::None,
    ] {
        let handler: Box<dyn SecurityHandler> = Box::new(configured_handler(method));
        for data in [b"".as_slice(), b"private content"] {
            assert_unsupported(handler.encrypt_string(
                data,
                &EncryptionKey::new(vec![1; 16]),
                &ObjectId::new(5, 2),
            ));
        }
    }
}

#[test]
fn decrypt_string_rejects_every_method_through_trait_object() {
    for method in [
        CryptFilterMethod::V2,
        CryptFilterMethod::AESV2,
        CryptFilterMethod::AESV3,
        CryptFilterMethod::None,
    ] {
        let handler: Box<dyn SecurityHandler> = Box::new(configured_handler(method));
        for data in [b"".as_slice(), b"private content"] {
            assert_unsupported(handler.decrypt_string(
                data,
                &EncryptionKey::new(vec![1; 16]),
                &ObjectId::new(5, 2),
            ));
        }
    }
}

#[test]
fn encrypt_stream_rejects_every_method_through_trait_object() {
    for method in [
        CryptFilterMethod::V2,
        CryptFilterMethod::AESV2,
        CryptFilterMethod::AESV3,
        CryptFilterMethod::None,
    ] {
        let handler: Box<dyn SecurityHandler> = Box::new(configured_handler(method));
        for data in [b"".as_slice(), b"private content"] {
            assert_unsupported(handler.encrypt_stream(
                data,
                &EncryptionKey::new(vec![1; 16]),
                &ObjectId::new(5, 2),
            ));
        }
    }
}

#[test]
fn decrypt_stream_rejects_every_method_through_trait_object() {
    for method in [
        CryptFilterMethod::V2,
        CryptFilterMethod::AESV2,
        CryptFilterMethod::AESV3,
        CryptFilterMethod::None,
    ] {
        let handler: Box<dyn SecurityHandler> = Box::new(configured_handler(method));
        for data in [b"".as_slice(), b"private content"] {
            assert_unsupported(handler.decrypt_stream(
                data,
                &EncryptionKey::new(vec![1; 16]),
                &ObjectId::new(5, 2),
            ));
        }
    }
}

#[test]
fn encrypt_string_aes_rejects_valid_aes_inputs_through_trait_object() {
    let handler: Box<dyn SecurityHandler> = Box::new(configured_handler(CryptFilterMethod::AESV3));
    for bits in [128, 256] {
        let key_bytes = vec![1; bits as usize / 8];
        // Valid CBC input with the old simulation's fixed IV ensures the former
        // decryptor would succeed, rather than merely failing on padding.
        let aes_key = if bits == 128 {
            AesKey::new_128(key_bytes.clone())
        } else {
            AesKey::new_256(key_bytes.clone())
        }
        .unwrap();
        let iv: Vec<u8> = (0..16).map(|i| (i * 13 + 7) as u8).collect();
        let ciphertext = Aes::new(aes_key)
            .encrypt_cbc(b"private content", &iv)
            .unwrap();
        assert_unsupported(handler.encrypt_string_aes(
            &ciphertext,
            &EncryptionKey::new(key_bytes),
            &ObjectId::new(5, 2),
            bits,
        ));
    }
}

#[test]
fn decrypt_string_aes_rejects_valid_aes_inputs_through_trait_object() {
    let handler: Box<dyn SecurityHandler> = Box::new(configured_handler(CryptFilterMethod::AESV3));
    for bits in [128, 256] {
        let key_bytes = vec![1; bits as usize / 8];
        // Valid CBC input with the old simulation's fixed IV ensures the former
        // decryptor would succeed, rather than merely failing on padding.
        let aes_key = if bits == 128 {
            AesKey::new_128(key_bytes.clone())
        } else {
            AesKey::new_256(key_bytes.clone())
        }
        .unwrap();
        let iv: Vec<u8> = (0..16).map(|i| (i * 13 + 7) as u8).collect();
        let ciphertext = Aes::new(aes_key)
            .encrypt_cbc(b"private content", &iv)
            .unwrap();
        assert_unsupported(handler.decrypt_string_aes(
            &ciphertext,
            &EncryptionKey::new(key_bytes),
            &ObjectId::new(5, 2),
            bits,
        ));
    }
}

#[test]
fn encrypt_stream_aes_rejects_valid_aes_inputs_through_trait_object() {
    let handler: Box<dyn SecurityHandler> = Box::new(configured_handler(CryptFilterMethod::AESV3));
    for bits in [128, 256] {
        let key_bytes = vec![1; bits as usize / 8];
        // Valid CBC input with the old simulation's fixed IV ensures the former
        // decryptor would succeed, rather than merely failing on padding.
        let aes_key = if bits == 128 {
            AesKey::new_128(key_bytes.clone())
        } else {
            AesKey::new_256(key_bytes.clone())
        }
        .unwrap();
        let iv: Vec<u8> = (0..16).map(|i| (i * 13 + 7) as u8).collect();
        let ciphertext = Aes::new(aes_key)
            .encrypt_cbc(b"private content", &iv)
            .unwrap();
        assert_unsupported(handler.encrypt_stream_aes(
            &ciphertext,
            &EncryptionKey::new(key_bytes),
            &ObjectId::new(5, 2),
            bits,
        ));
    }
}

#[test]
fn decrypt_stream_aes_rejects_valid_aes_inputs_through_trait_object() {
    let handler: Box<dyn SecurityHandler> = Box::new(configured_handler(CryptFilterMethod::AESV3));
    for bits in [128, 256] {
        let key_bytes = vec![1; bits as usize / 8];
        // Valid CBC input with the old simulation's fixed IV ensures the former
        // decryptor would succeed, rather than merely failing on padding.
        let aes_key = if bits == 128 {
            AesKey::new_128(key_bytes.clone())
        } else {
            AesKey::new_256(key_bytes.clone())
        }
        .unwrap();
        let iv: Vec<u8> = (0..16).map(|i| (i * 13 + 7) as u8).collect();
        let ciphertext = Aes::new(aes_key)
            .encrypt_cbc(b"private content", &iv)
            .unwrap();
        assert_unsupported(handler.decrypt_stream_aes(
            &ciphertext,
            &EncryptionKey::new(key_bytes),
            &ObjectId::new(5, 2),
            bits,
        ));
    }
}

#[test]
fn crypt_filter_dispatch_cannot_bypass_containment() {
    for method in [
        CryptFilterMethod::V2,
        CryptFilterMethod::AESV2,
        CryptFilterMethod::AESV3,
    ] {
        let mut manager = CryptFilterManager::new(
            Box::new(configured_handler(method)),
            "StdCF".into(),
            "StdCF".into(),
        );
        manager.add_filter(CryptFilterManager::create_standard_filter(method, None));
        let key = EncryptionKey::new(vec![
            1;
            if method == CryptFilterMethod::AESV3 {
                32
            } else {
                16
            }
        ]);
        let id = ObjectId::new(1, 0);
        assert_unsupported(manager.encrypt_string(b"secret", &id, None, &key));
        assert_unsupported(manager.decrypt_string(b"secret", &id, None, &key));
        assert_unsupported(manager.encrypt_stream(b"secret", &id, &Dictionary::new(), &key));
        assert_unsupported(manager.decrypt_stream(b"secret", &id, &Dictionary::new(), &key));
        // Explicit Identity is a documented no-encryption filter, independent of handlers.
        assert_eq!(
            manager
                .encrypt_string(b"plain", &id, Some("Identity"), &key)
                .unwrap(),
            b"plain"
        );
    }
}

#[test]
fn standard_password_handler_still_encrypts_and_decrypts() {
    let handler: Box<dyn SecurityHandler> = Box::new(StandardSecurityHandler::rc4_128bit());
    let key = EncryptionKey::new(vec![1; 16]);
    let id = ObjectId::new(1, 0);
    let encrypted = handler.encrypt_string(b"password path", &key, &id).unwrap();
    assert_ne!(encrypted, b"password path");
    assert_eq!(
        handler.decrypt_string(&encrypted, &key, &id).unwrap(),
        b"password path"
    );
}

#[test]
fn failed_recipient_creation_leaves_dictionary_helpers_empty() {
    use oxidize_pdf::encryption::PublicKeyEncryptionDict;
    use oxidize_pdf::objects::Object;

    let mut handler = PublicKeySecurityHandler::new_sha256();
    handler.seed_length = usize::MAX;
    assert_unsupported(handler.add_recipient(vec![0; 100], Permissions::all()));
    assert_unsupported(handler.decrypt_seed(b"data", b"key"));
    assert_eq!(
        handler.build_recipients_dict().get("Recipients"),
        Some(&Object::Array(vec![]))
    );
    let metadata = PublicKeyEncryptionDict::new(&handler);
    assert!(metadata.recipients.is_empty());
    assert_eq!(
        metadata.to_dict().get("Recipients"),
        Some(&Object::Array(vec![]))
    );
}

#[test]
fn dictionary_helpers_only_copy_explicit_caller_metadata() {
    use oxidize_pdf::encryption::PublicKeyEncryptionDict;
    use oxidize_pdf::objects::Object;

    let mut handler = PublicKeySecurityHandler::new_sha256();
    handler.recipients.push(Recipient {
        certificate: b"unvalidated certificate".to_vec(),
        permissions: Permissions::all(),
        encrypted_seed: b"unvalidated recipient bytes".to_vec(),
    });
    let mut expected = Dictionary::new();
    expected.set("Cert", Object::String("unvalidated certificate".into()));
    expected.set("P", Object::Integer(Permissions::all().bits() as i64));
    expected.set(
        "Recipients",
        Object::String("unvalidated recipient bytes".into()),
    );
    let expected_array = Object::Array(vec![Object::Dictionary(expected.clone())]);
    assert_eq!(
        handler.build_recipients_dict().get("Recipients"),
        Some(&expected_array)
    );
    let metadata = PublicKeyEncryptionDict::new(&handler);
    assert_eq!(metadata.recipients, vec![expected]);
    let serialized = metadata.to_dict();
    assert_eq!(serialized.get("Recipients"), Some(&expected_array));
    assert_eq!(
        serialized.get("Filter"),
        Some(&Object::Name("Adobe.PubSec".into()))
    );
    assert!(!handler.verify_permission(0, Permissions::all()));
    assert_unsupported(handler.decrypt_seed(b"unvalidated recipient bytes", b"key"));
}
