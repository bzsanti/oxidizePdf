#[path = "support/issue_711_contract.rs"]
mod contract;

#[test]
fn direct_iv_and_all_authoring_algorithms_work_natively() {
    contract::direct_iv();
    for algorithm in 0..3 {
        contract::authoring_case(algorithm);
    }
}

#[test]
fn fallible_iv_and_encryption_errors_are_explicit() {
    use oxidize_pdf::encryption::{try_generate_iv, EncryptionKey, StandardSecurityHandler};
    use oxidize_pdf::objects::ObjectId;
    assert_eq!(try_generate_iv().unwrap().len(), 16);
    let handler = StandardSecurityHandler::aes_256_r5();
    let result = handler.try_encrypt_string(
        b"secret",
        &EncryptionKey::new(vec![0; 1]),
        &ObjectId::new(1, 0),
    );
    assert!(matches!(
        result,
        Err(oxidize_pdf::PdfError::EncryptionError(_))
    ));
}

#[test]
fn security_handler_trait_string_propagates_aes_errors() {
    use oxidize_pdf::encryption::{EncryptionKey, SecurityHandler, StandardSecurityHandler};
    use oxidize_pdf::objects::ObjectId;
    let handler = StandardSecurityHandler::aes_256_r5();
    let security: &dyn SecurityHandler = &handler;
    assert!(
        matches!(security.encrypt_string(b"secret", &EncryptionKey::new(vec![0; 1]), &ObjectId::new(1, 0)),
        Err(oxidize_pdf::PdfError::EncryptionError(message)) if message.contains("Invalid key length"))
    );
}

#[test]
fn security_handler_trait_stream_propagates_aes_errors() {
    use oxidize_pdf::encryption::{EncryptionKey, SecurityHandler, StandardSecurityHandler};
    use oxidize_pdf::objects::ObjectId;
    let handler = StandardSecurityHandler::aes_256_r5();
    let security: &dyn SecurityHandler = &handler;
    assert!(
        matches!(security.encrypt_stream(b"secret", &EncryptionKey::new(vec![0; 1]), &ObjectId::new(1, 0)),
        Err(oxidize_pdf::PdfError::EncryptionError(message)) if message.contains("Invalid key length"))
    );
}

#[test]
fn security_handler_trait_encryption_recovers_plaintext_for_rc4_and_aes() {
    use oxidize_pdf::encryption::{EncryptionKey, SecurityHandler, StandardSecurityHandler};
    use oxidize_pdf::objects::ObjectId;
    for (handler, key_len) in [
        (StandardSecurityHandler::rc4_128bit(), 16),
        (StandardSecurityHandler::aes_128_r4(), 16),
        (StandardSecurityHandler::aes_256_r5(), 32),
    ] {
        let key = EncryptionKey::new(vec![0x42; key_len]);
        let security: &dyn SecurityHandler = &handler;
        let id = ObjectId::new(7, 0);
        let plaintext = b"trait encryption preserves this payload";
        let string = security.encrypt_string(plaintext, &key, &id).unwrap();
        let stream = security.encrypt_stream(plaintext, &key, &id).unwrap();
        assert_ne!(string.as_slice(), plaintext);
        assert_ne!(stream.as_slice(), plaintext);
        assert_eq!(
            security.decrypt_string(&string, &key, &id).unwrap(),
            plaintext
        );
        assert_eq!(
            security.decrypt_string(&stream, &key, &id).unwrap(),
            plaintext
        );
    }
}
