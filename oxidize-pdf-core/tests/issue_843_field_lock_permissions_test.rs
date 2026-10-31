//! Signature-field /Lock /P is a document permission, independent of field names.
use oxidize_pdf::parser::PdfReader;
use oxidize_pdf::signatures::{
    prepare_document_timestamp, prepare_incremental_signature, DocumentTimestampOptions,
    SignaturePreparationOptions,
};
use oxidize_pdf::writer::IncrementalObjectEditor;
use std::io::Cursor;

#[path = "common/pdf_assembler.rs"]
mod pdf_assembler;

// Structural CMS for policy tests; the independent OpenSSL test uses real CMS.
const CMS: &[u8] = b"\x30\x23\x06\x09\x2A\x86\x48\x86\xF7\x0D\x01\x07\x02\xA0\x16\x30\x14\x02\x01\x01\x31\x00\x30\x0B\x06\x09\x2A\x86\x48\x86\xF7\x0D\x01\x07\x01\x31\x00";

fn base(permission: &str) -> Vec<u8> {
    pdf_assembler::assemble_pdf_with_version("2.0", &[
        b"<< /Type /Catalog /Pages 2 0 R /AcroForm << /Fields [4 0 R] >> >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Annots [4 0 R] >>".to_vec(),
        format!("<< /Type /Annot /Subtype /Widget /FT /Sig /T (First) /P 3 0 R /Rect [0 0 0 0] /Lock << /Type /SigFieldLock {permission} /Action /Include /Fields [(Unrelated)] >> >>").into_bytes(),
    ])
}
fn signed(permission: &str) -> Vec<u8> {
    prepare_incremental_signature(
        &base(permission),
        &SignaturePreparationOptions::existing("First"),
    )
    .unwrap()
    .finalize(CMS)
    .unwrap()
}
fn approval(bytes: &[u8]) -> bool {
    prepare_incremental_signature(bytes, &SignaturePreparationOptions::invisible("Second")).is_ok()
}

#[test]
fn signed_p1_denies_approval_without_docmdp() {
    let pdf = signed("/P 1");
    assert!(
        !approval(&pdf),
        "signed field P=1 must prohibit a later approval signature"
    );
}

#[test]
fn p2_p3_and_absent_p_allow_approval() {
    for permission in ["/P 2", "/P 3", ""] {
        assert!(approval(&signed(permission)), "{permission}");
    }
}

#[test]
fn unsigned_field_does_not_activate_document_permissions() {
    assert!(approval(&base("/P 1")));
}

#[test]
fn p1_allows_only_the_invisible_timestamp_path() {
    let pdf = signed("/P 1");
    let timestamp =
        prepare_document_timestamp(&pdf, &DocumentTimestampOptions::invisible("Archive")).unwrap();
    assert!(timestamp.prepared_pdf().starts_with(&pdf));
    assert!(!approval(&pdf));
    assert!(
        IncrementalObjectEditor::new(&pdf).is_err(),
        "generic edits cannot claim the DSS-only exemption"
    );
    let mut spoof = SignaturePreparationOptions::invisible("SpoofedTimestamp");
    spoof.sub_filter = "ETSI.RFC3161".into();
    assert!(prepare_incremental_signature(&pdf, &spoof).is_err());
}

// Independent incremental writer for adversarial input; intentionally bypasses editor policy.
fn append_object(pdf: &[u8], number: u32, body: &str) -> Vec<u8> {
    let reader = PdfReader::new(Cursor::new(pdf)).unwrap();
    let trailer = reader.trailer();
    let root = trailer.root().unwrap();
    let mut result = pdf.to_vec();
    result.push(b'\n');
    let object_offset = result.len();
    result.extend_from_slice(format!("{number} 0 obj\n{body}\nendobj\n").as_bytes());
    let xref = result.len();
    result.extend_from_slice(format!("xref\n{number} 1\n{object_offset:010} 00000 n \ntrailer\n<< /Size {} /Root {} {} R /Prev {} >>\nstartxref\n{xref}\n%%EOF\n", trailer.size().unwrap(), root.0, root.1, trailer.xref_offset).as_bytes());
    result
}

#[test]
fn later_field_redefinition_cannot_relax_signed_permission() {
    let original = signed("/P 1");
    let mut reader = PdfReader::new(Cursor::new(&original)).unwrap();
    let value = reader
        .get_object(4, 0)
        .unwrap()
        .as_dict()
        .unwrap()
        .get("V")
        .unwrap()
        .as_reference()
        .unwrap();
    for permission in ["", "/P 3"] {
        let revised = append_object(&original, 4, &format!("<< /FT /Sig /T (First) /V {} {} R /Lock << /Type /SigFieldLock {permission} /Action /Include /Fields [(Unrelated)] >> >>", value.0, value.1));
        assert!(revised.starts_with(&original));
        assert!(!approval(&revised));
    }
}

#[test]
fn removing_signed_field_from_latest_form_does_not_remove_policy() {
    let original = signed("/P 1");
    let revised = append_object(&original, 1, "<< /Type /Catalog /Pages 2 0 R >>");
    assert!(!approval(&revised));
}

#[test]
fn malformed_signed_permission_is_not_silently_ignored() {
    for permission in ["/P 0", "/P 4", "/P -1", "/P (1)", "/P null", "/P 1.5"] {
        assert!(!approval(&signed(permission)), "{permission}");
    }
}

#[test]
#[cfg(feature = "signatures")]
#[ignore = "requires OpenSSL and qpdf; run in interoperability CI"]
fn genuine_signed_lock_denies_second_approval() {
    use std::process::Command;
    let dir = tempfile::tempdir().unwrap();
    let run = |args: &[&str]| {
        let result = Command::new("openssl")
            .args(args)
            .current_dir(dir.path())
            .output()
            .unwrap();
        assert!(
            result.status.success(),
            "openssl {args:?}: {}",
            String::from_utf8_lossy(&result.stderr)
        );
    };
    run(&[
        "req",
        "-x509",
        "-newkey",
        "rsa:2048",
        "-nodes",
        "-days",
        "1",
        "-subj",
        "/CN=issue843 synthetic signer",
        "-keyout",
        "key.pem",
        "-out",
        "cert.pem",
    ]);
    let finalize = |prepared: oxidize_pdf::signatures::PreparedSignature| {
        std::fs::write(dir.path().join("digest.bin"), prepared.bytes_to_digest()).unwrap();
        run(&[
            "cms",
            "-sign",
            "-binary",
            "-md",
            "sha256",
            "-nosmimecap",
            "-in",
            "digest.bin",
            "-signer",
            "cert.pem",
            "-inkey",
            "key.pem",
            "-outform",
            "DER",
            "-out",
            "cms.der",
        ]);
        run(&[
            "cms",
            "-verify",
            "-binary",
            "-inform",
            "DER",
            "-in",
            "cms.der",
            "-content",
            "digest.bin",
            "-noverify",
            "-out",
            "verified.bin",
        ]);
        prepared
            .finalize(&std::fs::read(dir.path().join("cms.der")).unwrap())
            .unwrap()
    };
    let first = finalize(
        prepare_incremental_signature(
            &base("/P 1"),
            &SignaturePreparationOptions::existing("First"),
        )
        .unwrap(),
    );
    std::fs::write(dir.path().join("signed.pdf"), &first).unwrap();
    assert!(Command::new("qpdf")
        .arg("--check")
        .arg(dir.path().join("signed.pdf"))
        .output()
        .unwrap()
        .status
        .success());
    let verified = PdfReader::new(Cursor::new(&first))
        .unwrap()
        .verify_signatures()
        .unwrap();
    assert_eq!(verified.len(), 1);
    assert!(verified[0].hash_valid && verified[0].signature_valid);
    let second =
        prepare_incremental_signature(&first, &SignaturePreparationOptions::invisible("Second"));
    if let Ok(prepared) = second {
        let invalid_update = finalize(prepared);
        assert!(invalid_update.starts_with(&first));
        let verified = PdfReader::new(Cursor::new(&invalid_update))
            .unwrap()
            .verify_signatures()
            .unwrap();
        assert_eq!(verified.len(), 2);
        assert!(verified.iter().all(|v| v.hash_valid && v.signature_valid));
        panic!("both CMS signatures verify, but the second approval violates signed /Lock /P 1");
    }
}

// Independent signed-object graph: the permission check must resolve PDF names
// even when the library's field-writing path would emit a direct /FT.
fn signed_field_graph(field_type: &str, inherited: bool, permission: &str) -> Vec<u8> {
    let field = format!("<< /FT {field_type} /T (First) /V 5 0 R /Lock 7 0 R >>");
    let mut objects = vec![
        b"<< /Type /Catalog /Pages 2 0 R /AcroForm << /Fields [4 0 R] >> >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] >>".to_vec(),
        field.into_bytes(),
        b"<< /Type /Sig /ByteRange [0000000000 0000000000 0000000000 0000000000] /Contents <302306092A864886F70D010702A01630140201013100300B06092A864886F70D0107013100> >>".to_vec(),
        b"/Sig".to_vec(),
        b"<< /Type /SigFieldLock /P 8 0 R /Action /Include /Fields [(Unrelated)] >>".to_vec(),
        permission.as_bytes().to_vec(),
        b"/SigFieldLock".to_vec(),
    ];
    if field_type != "/Sig" {
        objects[6] = b"<< /Type 9 0 R /P 8 0 R /Action /Include /Fields [(Unrelated)] >>".to_vec();
    }
    if inherited {
        objects[3] = format!("<< /FT {field_type} /T (Parent) /Kids [10 0 R] >>").into_bytes();
        objects.push(b"<< /Parent 4 0 R /T (First) /V 5 0 R /Lock 7 0 R >>".to_vec());
    }
    let mut pdf = pdf_assembler::assemble_pdf_with_version("2.0", &objects);
    let find = |needle: &[u8]| pdf.windows(needle.len()).position(|v| v == needle).unwrap();
    let range_start = find(b"/ByteRange [") + b"/ByteRange [".len();
    let contents_start = find(b"/Contents <") + b"/Contents ".len();
    let contents_end = contents_start + 2 + CMS.len() * 2;
    let ranges = format!(
        "{:010} {:010} {:010} {:010}",
        0,
        contents_start,
        contents_end,
        pdf.len() - contents_end
    );
    pdf[range_start..range_start + ranges.len()].copy_from_slice(ranges.as_bytes());
    pdf
}

#[test]
fn indirect_and_inherited_field_types_enforce_indirect_lock_permissions() {
    for kind in ["/Sig", "6 0 R"] {
        for inherited in [false, true] {
            assert!(
                !approval(&signed_field_graph(kind, inherited, "1")),
                "{kind}, inherited={inherited}"
            );
            assert!(
                approval(&signed_field_graph(kind, inherited, "2")),
                "{kind}, inherited={inherited}"
            );
        }
    }
}

#[test]
fn malformed_field_type_cannot_hide_a_signed_lock() {
    assert!(!approval(&signed_field_graph("42", false, "1")));
}
