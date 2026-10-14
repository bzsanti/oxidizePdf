//! Legacy form state is not cryptographic verification (#723).
use oxidize_pdf::{
    error::PdfError,
    forms::{
        signature_field::{
            Certificate, SignatureAlgorithm, SignatureField, SignatureValue, SignerInfo,
        },
        signature_handler::{SignatureHandler, SignatureSettings},
    },
};
fn supplied_field(name: &str) -> SignatureField {
    let mut field = SignatureField::new(name).with_signer(SignerInfo::new("Untrusted"));
    field.signature_value = Some(SignatureValue {
        timestamp: chrono::DateTime::from_timestamp(0, 0).unwrap(),
        document_hash: vec![9],
        algorithm: SignatureAlgorithm::RsaSha256,
        certificates: vec![],
        signature_bytes: vec![1, 2, 3],
    });
    field
}
fn unsupported<T: std::fmt::Debug>(result: Result<T, PdfError>) {
    assert!(
        matches!(result,Err(PdfError::InvalidOperation(ref s)) if s.contains("not supported")),
        "expected explicit unsupported error: {result:?}"
    );
}
#[test]
fn unsigned_field_is_not_valid() {
    let field = SignatureField::new("empty");
    assert!(!field.is_signed());
    assert!(!field.verify().unwrap());
}
#[test]
fn fabricated_signature_is_unverified() {
    let field = supplied_field("fake");
    assert!(field.is_signed());
    unsupported(field.verify());
}
#[test]
fn plausible_metadata_and_algorithms_cannot_authorize_verification() {
    for algorithm in [
        SignatureAlgorithm::RsaSha256,
        SignatureAlgorithm::RsaSha384,
        SignatureAlgorithm::RsaSha512,
        SignatureAlgorithm::EcdsaSha256,
        SignatureAlgorithm::DsaSha256,
    ] {
        let mut field = supplied_field("plausible");
        let sig = field.signature_value.as_mut().unwrap();
        sig.algorithm = algorithm;
        sig.document_hash = vec![0; 32];
        sig.signature_bytes = vec![0; 256];
        sig.certificates.push(Certificate {
            subject: "CN=Untrusted".into(),
            issuer: "CN=Untrusted".into(),
            serial_number: "1".into(),
            not_before: chrono::DateTime::from_timestamp(0, 0).unwrap(),
            not_after: chrono::DateTime::from_timestamp(4102444800, 0).unwrap(),
            public_key_info: "not a cryptographic certificate".into(),
        });
        unsupported(field.verify());
    }
}
#[test]
fn aggregate_validation_does_not_upgrade_presence_to_validity() {
    let mut handler = SignatureHandler::new();
    handler
        .add_signature_field(supplied_field("fake").required())
        .unwrap();
    handler
        .add_signature_field(SignatureField::new("required").required())
        .unwrap();
    handler
        .add_signature_field(SignatureField::new("optional"))
        .unwrap();
    let results = handler.validate_all();
    assert_eq!(results.len(), 3);
    for r in &results {
        assert!(!r.is_valid, "{} was reported valid", r.field_name);
    }
    let fake = results.iter().find(|r| r.field_name == "fake").unwrap();
    assert!(
        fake.errors.iter().any(|e| e.contains("not supported")),
        "{fake:?}"
    );
    let required = results.iter().find(|r| r.field_name == "required").unwrap();
    assert!(!required.errors.is_empty());
    let optional = results.iter().find(|r| r.field_name == "optional").unwrap();
    assert!(optional.errors.is_empty());
    assert!(!optional.warnings.is_empty());
}
#[test]
fn signing_refuses_without_mutating_existing_metadata() {
    let mut field = SignatureField::new("empty")
        .with_signer(SignerInfo::new("Original"))
        .with_reason("Original reason");
    let before = format!("{field:?}");
    unsupported(field.sign(SignerInfo::new("New"), Some("Replacement".into())));
    assert_eq!(format!("{field:?}"), before);
    assert!(!field.is_signed());
}
#[test]
fn already_present_value_is_not_overwritten() {
    let mut field = supplied_field("existing");
    let before = format!("{field:?}");
    assert!(field.sign(SignerInfo::new("New"), None).is_err());
    assert_eq!(format!("{field:?}"), before);
}
#[test]
fn rejected_handler_signing_preserves_locks_and_completion() {
    let settings = SignatureSettings {
        lock_all_after_first_signature: true,
        ..Default::default()
    };
    let mut handler = SignatureHandler::with_settings(settings);
    handler
        .add_signature_field(
            SignatureField::new("first")
                .required()
                .lock_fields_after_signing(vec!["second".into()]),
        )
        .unwrap();
    handler
        .add_signature_field(SignatureField::new("second"))
        .unwrap();
    unsupported(handler.sign_field("first", SignerInfo::new("New"), Some("Approval".into())));
    assert!(!handler.is_field_locked("first"));
    assert!(!handler.is_field_locked("second"));
    assert!(!handler.get_field("first").unwrap().is_signed());
    assert!(!handler.all_required_signed());
    let summary = handler.generate_summary();
    assert_eq!(summary.signed_fields, 0);
    assert_eq!(summary.locked_fields, 0);
    assert!(!summary.is_ready());
}
#[test]
fn appearances_and_presence_remain_available_without_verification() {
    let mut handler = SignatureHandler::new();
    let field = supplied_field("present").required();
    let appearance = field.generate_appearance(200., 50.).unwrap();
    assert!(String::from_utf8(appearance).unwrap().contains("Untrusted"));
    assert!(field.to_dict().get("V").is_some());
    handler.add_signature_field(field).unwrap();
    assert!(handler.all_required_signed());
    assert_eq!(handler.generate_summary().signed_fields, 1);
    assert!(handler
        .generate_summary()
        .to_report()
        .contains("not cryptographically verified"));
    assert!(!handler.validate_all()[0].is_valid);
}
#[test]
fn missing_field_still_reports_not_found() {
    assert!(
        matches!(SignatureHandler::new().sign_field("missing",SignerInfo::new("New"),None),Err(PdfError::FieldNotFound(s)) if s=="missing")
    );
}
