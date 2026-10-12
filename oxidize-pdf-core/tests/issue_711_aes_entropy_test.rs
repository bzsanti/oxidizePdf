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
