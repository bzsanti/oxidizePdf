use oxidize_pdf::parser::objects::{PdfDictionary, PdfObject, PdfString};
use oxidize_pdf::signatures::{
    prepare_incremental_signature, CertificationPermission, FieldLock, SignaturePreparationOptions,
    SignatureRect, SignatureTarget,
};
use oxidize_pdf::{Document, Page};
use std::process::Command;

fn base_pdf(xref_stream: bool) -> Vec<u8> {
    let mut document = Document::new();
    document.enable_xref_streams(xref_stream);
    document.add_page(Page::a4());
    document.to_bytes().unwrap()
}

fn deterministic_cms() -> &'static [u8] {
    // DER ContentInfo wrapping SignedData with deterministic empty sets. It is
    // structurally valid CMS but intentionally carries no trust assertion.
    b"\x30\x23\x06\x09\x2A\x86\x48\x86\xF7\x0D\x01\x07\x02\xA0\x16\x30\x14\x02\x01\x01\x31\x00\x30\x0B\x06\x09\x2A\x86\x48\x86\xF7\x0D\x01\x07\x01\x31\x00"
}

fn classic_with_fields(fields: &[(u32, u16, &str)]) -> Vec<u8> {
    let references = fields
        .iter()
        .map(|(number, generation, _)| format!("{number} {generation} R"))
        .collect::<Vec<_>>()
        .join(" ");
    let mut objects = vec![
        (
            1,
            0,
            format!("<< /Type /Catalog /Pages 2 0 R /AcroForm << /Fields [{references}] >> >>"),
        ),
        (
            2,
            0,
            "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_string(),
        ),
        (
            3,
            0,
            format!(
                "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 300 300] /Annots [{references}] >>"
            ),
        ),
    ];
    objects.extend(fields.iter().map(|(number, generation, name)| {
        (
            *number,
            *generation,
            format!(
                "<< /Type /Annot /Subtype /Widget /FT /Sig /T ({name}) /Rect [0 0 0 0] /P 3 0 R >>"
            ),
        )
    }));
    build_classic_objects(objects)
}

fn hierarchical_field() -> Vec<u8> {
    build_classic_objects(vec![
        (
            1,
            0,
            "<< /Type /Catalog /Pages 2 0 R /AcroForm << /Fields 4 0 R >> >>".to_string(),
        ),
        (
            2,
            0,
            "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_string(),
        ),
        (
            3,
            0,
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 300 300] /Annots [9 0 R] >>".to_string(),
        ),
        (4, 0, "[5 0 R]".to_string()),
        (5, 0, "<< /FT /Sig /T (Parent) /Kids 6 0 R >>".to_string()),
        (6, 0, "[7 0 R]".to_string()),
        (
            7,
            0,
            "<< /T (Child) /Parent 5 0 R /Kids 8 0 R >>".to_string(),
        ),
        (8, 0, "[9 0 R]".to_string()),
        (
            9,
            0,
            "<< /Type /Annot /Subtype /Widget /Parent 7 0 R /Rect [0 0 0 0] /P 3 0 R >>"
                .to_string(),
        ),
    ])
}

fn build_classic_objects(mut objects: Vec<(u32, u16, String)>) -> Vec<u8> {
    objects.sort_by_key(|(number, _, _)| *number);
    let size = objects.iter().map(|(number, _, _)| *number).max().unwrap() + 1;
    let mut output = b"%PDF-1.7\n".to_vec();
    let mut entries = vec![None; size as usize];
    for (number, generation, body) in objects {
        entries[number as usize] = Some((output.len(), generation));
        output.extend_from_slice(format!("{number} {generation} obj\n{body}\nendobj\n").as_bytes());
    }
    let xref = output.len();
    output.extend_from_slice(format!("xref\n0 {size}\n0000000000 65535 f \n").as_bytes());
    for entry in entries.into_iter().skip(1) {
        match entry {
            Some((offset, generation)) => {
                output.extend_from_slice(format!("{offset:010} {generation:05} n \n").as_bytes())
            }
            None => output.extend_from_slice(b"0000000000 00000 f \n"),
        }
    }
    output.extend_from_slice(
        format!("trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n").as_bytes(),
    );
    output
}

#[test]
fn two_phase_signing_preserves_the_source_and_exact_byte_range() {
    let source = base_pdf(false);
    let prepared = prepare_incremental_signature(
        &source,
        &SignaturePreparationOptions::invisible("Approval.1"),
    )
    .unwrap();
    assert!(prepared.prepared_pdf().starts_with(&source));
    prepared.byte_range().validate().unwrap();

    let expected: Vec<_> = prepared
        .byte_range()
        .ranges()
        .iter()
        .flat_map(|&(offset, length)| {
            prepared.prepared_pdf()[offset as usize..(offset + length) as usize]
                .iter()
                .copied()
        })
        .collect();
    assert_eq!(prepared.bytes_to_digest(), expected);

    let signed = prepared.finalize(deterministic_cms()).unwrap();
    assert!(signed.starts_with(&source));
    assert!(signed.windows(8).any(|bytes| bytes == b"30230609"));
}

#[test]
fn visible_and_successive_signatures_are_incremental_for_both_xref_formats() {
    for xref_stream in [false, true] {
        let source = base_pdf(xref_stream);
        let mut first = SignaturePreparationOptions::invisible("Approval.1");
        first.target = SignatureTarget::New {
            field_name: "Approval.1".to_string(),
            page_index: 0,
            rect: Some(SignatureRect {
                left: 40.0,
                bottom: 40.0,
                right: 180.0,
                top: 90.0,
            }),
        };
        let signed_once = prepare_incremental_signature(&source, &first)
            .unwrap()
            .finalize(deterministic_cms())
            .unwrap();
        assert!(signed_once.windows(3).any(|bytes| bytes == b"/AP"));

        let signed_twice = prepare_incremental_signature(
            &signed_once,
            &SignaturePreparationOptions::invisible("Approval.2"),
        )
        .unwrap()
        .finalize(deterministic_cms())
        .unwrap();
        assert!(signed_twice.starts_with(&signed_once));
        assert_eq!(
            signed_twice
                .windows(b"/ByteRange".len())
                .filter(|value| *value == b"/ByteRange")
                .count(),
            2
        );
    }
}

#[test]
fn profile_extensions_and_mdp_transforms_are_serialized() {
    let source = base_pdf(false);
    let mut options = SignaturePreparationOptions::invisible("Certification");
    options.sub_filter = "ETSI.CAdES.detached".to_string();
    options.certification = Some(CertificationPermission::FormFillAndSign);
    options.field_lock = Some(FieldLock::Include(vec!["Approval.2".to_string()]));
    options.additional_signature_entries.insert(
        "ProfileMarker".to_string(),
        PdfObject::String(PdfString::new(b"private-extension".to_vec())),
    );
    let prepared = prepare_incremental_signature(&source, &options).unwrap();
    let text = String::from_utf8_lossy(prepared.prepared_pdf());
    assert!(text.contains("/SubFilter /ETSI.CAdES.detached"));
    assert!(text.contains("/TransformMethod /DocMDP"));
    assert!(text.contains("/TransformMethod /FieldMDP"));
    assert!(text.contains("/ProfileMarker"));
}

#[test]
fn malformed_inputs_and_unsafe_extension_overrides_fail_closed() {
    let source = base_pdf(false);
    assert!(prepare_incremental_signature(
        b"not a PDF",
        &SignaturePreparationOptions::invisible("Approval")
    )
    .is_err());
    let mut encrypted = Document::new();
    encrypted.add_page(Page::a4());
    encrypted.encrypt_with_passwords("user", "owner");
    let encrypted = encrypted.to_bytes().unwrap();
    assert!(prepare_incremental_signature(
        &encrypted,
        &SignaturePreparationOptions::invisible("Approval")
    )
    .is_err());
    let prepared =
        prepare_incremental_signature(&source, &SignaturePreparationOptions::invisible("Approval"))
            .unwrap();
    assert!(prepared.finalize(b"not DER CMS").is_err());

    let mut options = SignaturePreparationOptions::invisible("Approval");
    options.additional_signature_entries = PdfDictionary::new();
    options.additional_signature_entries.insert(
        "Contents".to_string(),
        PdfObject::String(PdfString::new(vec![1, 2, 3])),
    );
    assert!(prepare_incremental_signature(&source, &options).is_err());

    let mut small = SignaturePreparationOptions::invisible("Approval");
    small.placeholder_bytes = 256;
    let prepared = prepare_incremental_signature(&source, &small).unwrap();
    let mut oversized_cms = vec![0x30, 0x82, 0x01, 0x2c];
    oversized_cms.extend(vec![0x01; 300]);
    assert!(prepared.finalize(&oversized_cms).is_err());
}

#[test]
fn docmdp_and_fieldmdp_are_enforced_on_later_signatures() {
    let source = base_pdf(false);
    let mut no_changes = SignaturePreparationOptions::invisible("Certification");
    no_changes.certification = Some(CertificationPermission::NoChanges);
    let certified = prepare_incremental_signature(&source, &no_changes)
        .unwrap()
        .finalize(deterministic_cms())
        .unwrap();
    assert!(prepare_incremental_signature(
        &certified,
        &SignaturePreparationOptions::invisible("Approval")
    )
    .is_err());

    let mut locked = SignaturePreparationOptions::invisible("Certification");
    locked.certification = Some(CertificationPermission::FormFillAndSign);
    locked.field_lock = Some(FieldLock::Include(vec!["Approval".to_string()]));
    let certified = prepare_incremental_signature(&source, &locked)
        .unwrap()
        .finalize(deterministic_cms())
        .unwrap();
    assert!(prepare_incremental_signature(
        &certified,
        &SignaturePreparationOptions::invisible("Approval")
    )
    .is_err());
    assert!(prepare_incremental_signature(
        &certified,
        &SignaturePreparationOptions::invisible("Unrestricted")
    )
    .is_ok());
}

#[test]
fn selects_indirect_existing_fields_with_nonzero_generations_and_rejects_ambiguity() {
    let source = classic_with_fields(&[(4, 7, "Existing")]);
    let mut options = SignaturePreparationOptions::invisible("unused");
    options.target = SignatureTarget::Existing {
        field_name: "Existing".to_string(),
        widget_index: None,
        rect: None,
    };
    let signed = prepare_incremental_signature(&source, &options)
        .unwrap()
        .finalize(deterministic_cms())
        .unwrap();
    assert!(signed.starts_with(&source));
    assert!(
        signed
            .windows(b"4 7 obj".len())
            .filter(|window| *window == b"4 7 obj")
            .count()
            >= 2
    );

    let ambiguous = classic_with_fields(&[(4, 0, "Duplicate"), (5, 0, "Duplicate")]);
    options.target = SignatureTarget::Existing {
        field_name: "Duplicate".to_string(),
        widget_index: None,
        rect: None,
    };
    assert!(prepare_incremental_signature(&ambiguous, &options).is_err());

    let hierarchical = hierarchical_field();
    options.target = SignatureTarget::Existing {
        field_name: "Parent.Child".to_string(),
        widget_index: Some(0),
        rect: Some(SignatureRect {
            left: 10.0,
            bottom: 10.0,
            right: 100.0,
            top: 40.0,
        }),
    };
    let prepared = prepare_incremental_signature(&hierarchical, &options).unwrap();
    let text = String::from_utf8_lossy(prepared.prepared_pdf());
    assert!(text.contains("/AP"));
    assert!(text.contains("/Rect [10 10 100 40]"));
}

#[test]
#[ignore = "requires qpdf; exercised by the Ubuntu CI interoperability step"]
fn qpdf_accepts_finalized_signatures_for_both_xref_formats() {
    for (name, xref_stream) in [("classic", false), ("xref-stream", true)] {
        let signed = prepare_incremental_signature(
            &base_pdf(xref_stream),
            &SignaturePreparationOptions::invisible("Approval"),
        )
        .unwrap()
        .finalize(deterministic_cms())
        .unwrap();
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join(format!("{name}.pdf"));
        std::fs::write(&path, signed).unwrap();
        let cms_path = directory.path().join("fixture.cms");
        std::fs::write(&cms_path, deterministic_cms()).unwrap();
        let cms_output = Command::new("openssl")
            .args(["cms", "-cmsout", "-inform", "DER", "-in"])
            .arg(&cms_path)
            .arg("-noout")
            .output()
            .unwrap();
        assert!(
            cms_output.status.success(),
            "deterministic CMS fixture: {}",
            String::from_utf8_lossy(&cms_output.stderr)
        );
        let output = Command::new("qpdf")
            .arg("--check")
            .arg(path)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{name}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}
