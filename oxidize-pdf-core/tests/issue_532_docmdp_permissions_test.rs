//! Regression coverage for issue #532: incremental structural edits must obey
//! certification policies while ordinary approval signatures remain usable.

mod common;

use common::pdf_assembler::assemble_pdf;
use oxidize_pdf::operations::reorder_pdf_pages_lossless;
#[cfg(feature = "signatures")]
use oxidize_pdf::parser::PdfReader;
#[cfg(feature = "signatures")]
use oxidize_pdf::signatures::{parse_pkcs7_signature_detailed, verify_signature_detailed};
use std::fs;
#[cfg(feature = "signatures")]
use std::io::Cursor;
use std::process::Command;
use tempfile::TempDir;

const CERTIFIED_P2: &[u8] = include_bytes!("fixtures/signatures/docmdp_p2_rsa.pdf");

fn signature(reference: &str) -> Vec<u8> {
    format!(
        "<< /Type /Sig /Filter /Adobe.PPKLite /SubFilter /adbe.pkcs7.detached \
         /ByteRange [0 10000 10001 0] /Contents <00> {reference} >>"
    )
    .into_bytes()
}

fn docmdp_reference(permission: i64) -> String {
    format!(
        "/Reference [<< /Type /SigRef /TransformMethod /DocMDP \
         /TransformParams << /Type /TransformParams /V /1.2 /P {permission} >> >>]"
    )
}

fn pdf_with_policy(primary: Vec<u8>, catalog_perms: Option<&str>, extras: Vec<Vec<u8>>) -> Vec<u8> {
    let perms_entry = catalog_perms
        .map(|value| format!(" /Perms {value}"))
        .unwrap_or_default();
    let mut objects = vec![
        format!("<< /Type /Catalog /Pages 2 0 R{perms_entry} >>").into_bytes(),
        b"<< /Type /Pages /Kids [3 0 R 4 0 R] /Count 2 >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 20 20] >>".to_vec(),
        primary,
        b"<< /DocMDP 5 0 R >>".to_vec(),
    ];
    objects.extend(extras);
    assemble_pdf(&objects)
}

fn reorder_error(source: &[u8]) -> String {
    let directory = TempDir::new().unwrap();
    let input = directory.path().join("input.pdf");
    let output = directory.path().join("output.pdf");
    fs::write(&input, source).unwrap();
    reorder_pdf_pages_lossless(&input, &output, &[1, 0])
        .expect_err("policy must reject page reordering")
        .to_string()
}

fn append_signature_redefinition(base: &[u8], body: &[u8]) -> Vec<u8> {
    let marker = b"startxref\n";
    let marker_at = base
        .windows(marker.len())
        .rposition(|window| window == marker)
        .unwrap();
    let value_start = marker_at + marker.len();
    let value_end = base[value_start..]
        .iter()
        .position(|byte| *byte == b'\n')
        .map(|offset| value_start + offset)
        .unwrap();
    let previous_xref: u64 = std::str::from_utf8(&base[value_start..value_end])
        .unwrap()
        .parse()
        .unwrap();

    let mut output = base.to_vec();
    if !output.ends_with(b"\n") {
        output.push(b'\n');
    }
    let object_offset = output.len();
    output.extend_from_slice(b"5 0 obj\n");
    output.extend_from_slice(body);
    output.extend_from_slice(b"\nendobj\n");
    let xref_offset = output.len();
    output.extend_from_slice(
        format!(
            "xref\n5 1\n{object_offset:010} 00000 n \n\
             trailer\n<< /Size 7 /Root 1 0 R /Prev {previous_xref} >>\n\
             startxref\n{xref_offset}\n%%EOF\n"
        )
        .as_bytes(),
    );
    output
}

#[test]
fn rejects_page_reordering_for_every_docmdp_permission_level() {
    for permission in 1..=3 {
        let source = pdf_with_policy(
            signature(&docmdp_reference(permission)),
            Some("6 0 R"),
            vec![],
        );
        let error = reorder_error(&source);
        assert!(error.contains(&format!("P={permission}")), "{error}");
        assert!(error.contains("page-tree reordering"), "{error}");
    }
}

#[test]
fn permits_multiple_approval_signatures_and_later_incremental_revisions() {
    let directory = TempDir::new().unwrap();
    let input = directory.path().join("input.pdf");
    let first = directory.path().join("first.pdf");
    let second = directory.path().join("second.pdf");
    let source = pdf_with_policy(
        signature(""),
        Some("8 0 R"),
        vec![signature(""), b"<< /UR3 5 0 R >>".to_vec()],
    );
    fs::write(&input, &source).unwrap();

    reorder_pdf_pages_lossless(&input, &first, &[1, 0]).expect("approval signatures are allowed");
    let first_bytes = fs::read(&first).unwrap();
    reorder_pdf_pages_lossless(&first, &second, &[1, 0])
        .expect("a later incremental revision remains allowed");
    assert!(fs::read(second).unwrap().starts_with(&first_bytes));
}

#[test]
fn rejects_docmdp_without_catalog_certification_entry() {
    let source = pdf_with_policy(signature(&docmdp_reference(2)), None, vec![]);
    assert!(reorder_error(&source).contains("without catalog /Perms /DocMDP"));
}

#[test]
fn rejects_conflicting_certification_signatures() {
    let source = pdf_with_policy(
        signature(&docmdp_reference(2)),
        Some("6 0 R"),
        vec![signature(&docmdp_reference(3))],
    );
    assert!(reorder_error(&source).contains("inconsistent"));
}

#[test]
fn rejects_a_certification_dictionary_redefined_after_signing() {
    let source = pdf_with_policy(signature(&docmdp_reference(1)), Some("6 0 R"), vec![]);
    let replacement = signature(&format!(
        "/Reference [<< /TransformMethod /DocMDP /TransformParams \
         << /P 3 >> >>] /Reason (forged) /SignedLength {}",
        source.len()
    ));
    let replacement = String::from_utf8(replacement)
        .unwrap()
        .replace(
            "/ByteRange [0 10000 10001 0]",
            &format!("/ByteRange [0 {} {} 0]", source.len(), source.len() + 1),
        )
        .into_bytes();
    let incrementally_redefined = append_signature_redefinition(&source, &replacement);

    assert!(reorder_error(&incrementally_redefined).contains("outside its signed byte ranges"));
}

#[test]
fn rejects_malformed_or_unsupported_transform_parameters() {
    for reference in [
        "/Reference [<< /TransformMethod /DocMDP >>]".to_string(),
        docmdp_reference(0),
        "/Reference [<< /TransformMethod /DocMDP /TransformParams \
         << /P 2 /V /2.0 >> >>]"
            .to_string(),
        "/Reference [
          << /TransformMethod /DocMDP /TransformParams << /P 2 >> >>
          << /TransformMethod /DocMDP /TransformParams << /P 3 >> >>
        ]"
        .to_string(),
    ] {
        let source = pdf_with_policy(signature(&reference), Some("6 0 R"), vec![]);
        assert!(reorder_error(&source).contains("DocMDP"));
    }

    let missing_signature_data = format!("<< /Type /Sig {} >>", docmdp_reference(2)).into_bytes();
    let source = pdf_with_policy(missing_signature_data, Some("6 0 R"), vec![]);
    assert!(reorder_error(&source).contains("ByteRange"));

    let source = pdf_with_policy(
        signature(&docmdp_reference(2)),
        Some("<< /DocMDP 5 >>"),
        vec![],
    );
    assert!(reorder_error(&source).contains("indirect reference"));
}

#[test]
fn permits_a_real_cryptographically_intact_approval_signature() {
    let directory = TempDir::new().unwrap();
    let input = directory.path().join("signed.pdf");
    let output = directory.path().join("output.pdf");
    let source = include_bytes!("fixtures/signatures/signed_rsa.pdf");
    fs::write(&input, source).unwrap();

    reorder_pdf_pages_lossless(&input, &output, &[0])
        .expect("an approval signature without DocMDP must be permitted");
    assert!(fs::read(output).unwrap().starts_with(source));
}

#[test]
fn rejects_a_cryptographically_intact_certification_signature() {
    let error = reorder_error(CERTIFIED_P2);
    assert!(error.contains("P=2"), "{error}");
    assert!(error.contains("page-tree reordering"), "{error}");
}

#[cfg(feature = "signatures")]
#[test]
fn certification_fixture_hash_and_rsa_signature_are_intact() {
    let mut reader = PdfReader::new(Cursor::new(CERTIFIED_P2)).unwrap();
    let signatures = reader.signatures().unwrap();
    assert_eq!(signatures.len(), 1);
    let parsed = parse_pkcs7_signature_detailed(&signatures[0].contents).unwrap();
    let verification =
        verify_signature_detailed(CERTIFIED_P2, &parsed, &signatures[0].byte_range).unwrap();
    assert!(verification.hash_valid);
    assert!(verification.signature_valid);
}

#[test]
fn qpdf_accepts_the_certification_fixture_when_available() {
    let directory = TempDir::new().unwrap();
    let input = directory.path().join("certified.pdf");
    fs::write(&input, CERTIFIED_P2).unwrap();

    let result = match Command::new("qpdf").arg("--check").arg(&input).output() {
        Ok(result) => result,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return,
        Err(error) => panic!("failed to execute qpdf: {error}"),
    };
    assert!(
        result.status.success(),
        "qpdf rejected fixture: {}",
        String::from_utf8_lossy(&result.stderr)
    );
}
