//! Regression tests for issue #492: AES-256 R5 output must include the
//! encrypted `/Perms` entry required by strict PDF consumers such as qpdf.

use oxidize_pdf::document::{DocumentEncryption, EncryptionStrength};
use oxidize_pdf::encryption::{
    EncryptionDictionary, EncryptionKey, Permissions, StandardSecurityHandler,
};
use oxidize_pdf::parser::PdfReader;
use oxidize_pdf::{Document, Page};
use std::io::Cursor;
use std::process::Command;

#[test]
fn aes256_r5_dictionary_contains_binary_perms_entry() {
    let encryption = DocumentEncryption::new(
        "view",
        "modify",
        Permissions::new(),
        EncryptionStrength::Aes256,
    );

    let dictionary = encryption
        .create_encryption_dict(Some(b"issue-492-file-id"))
        .expect("AES-256 dictionary should be created");

    assert_eq!(dictionary.v, 5);
    assert_eq!(dictionary.r, 5);
    assert_eq!(dictionary.perms.as_deref().map(<[u8]>::len), Some(16));

    let serialized = dictionary.to_dict();
    assert!(matches!(
        serialized.get("Perms"),
        Some(oxidize_pdf::objects::Object::ByteString(bytes)) if bytes.len() == 16
    ));
}

#[test]
fn aes256_dictionary_rejects_a_perms_entry_with_the_wrong_size() {
    let result = EncryptionDictionary::aes_256(vec![0; 48], vec![0; 48], Permissions::new(), None)
        .with_perms(vec![0; 15]);

    assert!(result.is_err(), "V=5 /Perms must be exactly 16 bytes");
}

#[test]
fn r5_uses_the_revision_neutral_perms_api() {
    let handler = StandardSecurityHandler::aes_256_r5();
    let key = EncryptionKey::new(vec![0x42; 32]);

    let perms = handler
        .compute_perms_entry(Permissions::new(), &key, true)
        .expect("R5 should support the shared permissions algorithm");

    assert_eq!(perms.len(), 16, "encrypted /Perms length");
}

#[test]
#[ignore = "requires qpdf; exercised by the Ubuntu CI interoperability step"]
fn qpdf_checks_and_linearizes_generated_aes256_r5_pdf() {
    let temp = tempfile::tempdir().expect("temporary directory");
    let input = temp.path().join("aes256.pdf");
    let linearized = temp.path().join("aes256-linearized.pdf");

    let mut expected_permissions = Permissions::new();
    expected_permissions.set_print(true).set_copy(false);

    let mut document = Document::new();
    document.add_page(Page::a4());
    document.set_encryption(DocumentEncryption::new(
        "view",
        "modify",
        expected_permissions,
        EncryptionStrength::Aes256,
    ));
    document.save(&input).expect("write encrypted PDF");

    assert_qpdf_success(&["--password=view", "--check", path(&input)]);
    assert_qpdf_success(&[
        "--password=modify",
        "--linearize",
        path(&input),
        path(&linearized),
    ]);
    assert_qpdf_success(&["--password=view", "--check", path(&linearized)]);
    assert_qpdf_success(&[
        "--password=view",
        "--check-linearization",
        path(&linearized),
    ]);

    let permissions = qpdf(&["--password=view", "--show-encryption", path(&linearized)]);
    assert!(permissions.status.success(), "{}", output(&permissions));
    let stdout = String::from_utf8_lossy(&permissions.stdout);
    assert!(
        stdout.contains(&format!("P = {}", expected_permissions.bits() as i32)),
        "permissions changed:\n{stdout}"
    );

    let bytes = std::fs::read(&linearized).expect("read qpdf output");
    let mut reader = PdfReader::new(Cursor::new(bytes)).expect("parse qpdf output");
    assert!(
        reader
            .unlock_with_password("view")
            .expect("unlock qpdf output"),
        "user password must unlock the qpdf output"
    );
    let actual_permissions = reader
        .encryption_handler()
        .expect("linearized PDF should remain encrypted")
        .permissions();
    assert_eq!(actual_permissions.bits(), expected_permissions.bits());
    assert!(actual_permissions.can_print());
    assert!(!actual_permissions.can_copy());
}

fn path(path: &std::path::Path) -> &str {
    path.to_str().expect("temporary path should be UTF-8")
}

fn qpdf(args: &[&str]) -> std::process::Output {
    Command::new("qpdf").args(args).output().expect("run qpdf")
}

fn assert_qpdf_success(args: &[&str]) {
    let result = qpdf(args);
    assert!(result.status.success(), "{}", output(&result));
}

fn output(result: &std::process::Output) -> String {
    format!(
        "qpdf failed with {}\nstdout:\n{}\nstderr:\n{}",
        result.status,
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    )
}
