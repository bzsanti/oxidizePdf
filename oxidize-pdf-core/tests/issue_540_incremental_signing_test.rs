use oxidize_pdf::parser::objects::{PdfDictionary, PdfObject, PdfString};
use oxidize_pdf::signatures::{
    prepare_incremental_signature, prepare_incremental_signature_with_appearance,
    CertificationPermission, FieldLock, SignatureAppearance, SignaturePreparationOptions,
    SignatureRect, SignatureTarget, SignatureWatermark,
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

fn custom_appearance() -> SignatureAppearance {
    SignatureAppearance {
        signer_name: Some("Santiago Fernández".to_string()),
        signing_date: Some("2026-09-16".to_string()),
        text: vec!["Approved by Studio".to_string()],
        watermark: Some(SignatureWatermark {
            width: 2,
            height: 1,
            rgb: vec![255, 0, 0, 0, 128, 255],
            opacity: 1.0,
        }),
    }
}

fn openssl_cms_for_digest(directory: &tempfile::TempDir, digest: &[u8]) -> Vec<u8> {
    let key = directory.path().join("signer-key.pem");
    let certificate = directory.path().join("signer-cert.pem");
    let digest_path = directory.path().join("digest.bin");
    let cms = directory.path().join("signature.der");
    std::fs::write(&digest_path, digest).unwrap();
    let output = Command::new("openssl")
        .args([
            "req",
            "-x509",
            "-newkey",
            "rsa:2048",
            "-keyout",
            key.to_str().unwrap(),
            "-out",
            certificate.to_str().unwrap(),
            "-nodes",
            "-subj",
            "/CN=oxidize-pdf test signer",
            "-days",
            "1",
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "generate test certificate: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let output = Command::new("openssl")
        .args([
            "cms",
            "-sign",
            "-binary",
            "-in",
            digest_path.to_str().unwrap(),
            "-signer",
            certificate.to_str().unwrap(),
            "-inkey",
            key.to_str().unwrap(),
            "-outform",
            "DER",
            "-out",
            cms.to_str().unwrap(),
            "-nosmimecap",
            "-md",
            "sha256",
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "sign prepared byte range: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let verified = directory.path().join("verified-digest.bin");
    let output = Command::new("openssl")
        .args([
            "cms",
            "-verify",
            "-binary",
            "-inform",
            "DER",
            "-in",
            cms.to_str().unwrap(),
            "-content",
            digest_path.to_str().unwrap(),
            "-noverify",
            "-out",
            verified.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "verify prepared byte range CMS: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(std::fs::read(verified).unwrap(), digest);
    std::fs::read(cms).unwrap()
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

fn rotated_cropped_pdf() -> Vec<u8> {
    build_classic_objects(vec![
        (
            1,
            0,
            "<< /Type /Catalog /Pages 2 0 R >>".to_string(),
        ),
        (
            2,
            0,
            "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_string(),
        ),
        (
            3,
            0,
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 300 200] /CropBox [20 10 280 180] /Rotate 90 >>".to_string(),
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
    let appearance = SignatureAppearance {
        text: vec!["Child widget".to_string()],
        ..SignatureAppearance::default()
    };
    let prepared =
        prepare_incremental_signature_with_appearance(&hierarchical, &options, &appearance)
            .unwrap();
    let text = String::from_utf8_lossy(prepared.prepared_pdf());
    assert!(text.contains("/AP"));
    assert!(text.contains("/Rect [10 10 100 40]"));
    assert!(text.contains("(Child widget) Tj"));
}

#[test]
fn visible_combined_field_is_replaced_once_with_value_and_appearance() {
    let source = classic_with_fields(&[(4, 7, "Existing")]);
    let mut options = SignaturePreparationOptions::existing("Existing");
    options.target = SignatureTarget::Existing {
        field_name: "Existing".to_string(),
        widget_index: None,
        rect: Some(SignatureRect {
            left: 10.0,
            bottom: 20.0,
            right: 110.0,
            top: 70.0,
        }),
    };

    let prepared = prepare_incremental_signature(&source, &options).unwrap();
    let text = String::from_utf8_lossy(prepared.prepared_pdf());
    // One occurrence belongs to the source PDF and one to its incremental
    // revision. Emitting `/V` and `/AP` separately corrupts this update.
    assert_eq!(
        prepared
            .prepared_pdf()
            .windows(b"4 7 obj".len())
            .filter(|window| *window == b"4 7 obj")
            .count(),
        2
    );
    assert!(text.contains("/V "));
    assert!(text.contains("/AP << /N "));
    assert!(text.contains("/Rect [10 20 110 70]"));
}

#[test]
fn custom_visible_appearance_embeds_text_and_watermark_before_signing() {
    let source = classic_with_fields(&[(4, 7, "Existing")]);
    let mut options = SignaturePreparationOptions::existing("Existing");
    options.target = SignatureTarget::Existing {
        field_name: "Existing".to_string(),
        widget_index: None,
        rect: Some(SignatureRect {
            left: 10.0,
            bottom: 20.0,
            right: 210.0,
            top: 90.0,
        }),
    };
    let appearance = custom_appearance();
    let prepared =
        prepare_incremental_signature_with_appearance(&source, &options, &appearance).unwrap();
    let rendered = String::from_utf8_lossy(prepared.prepared_pdf());
    assert!(rendered.contains("/BaseFont /Helvetica"));
    assert!(rendered.contains("/Subtype /Image"));
    assert!(rendered.contains("/Im0 Do"));
    assert!(rendered.contains("(Santiago Fern\\341ndez) Tj"));
    assert!(rendered.contains("(2026-09-16) Tj"));
    assert!(rendered.contains("(Approved by Studio) Tj"));
    assert!(prepared
        .bytes_to_digest()
        .windows(b"Santiago Fern\\341ndez".len())
        .any(|window| window == b"Santiago Fern\\341ndez"));
    assert!(prepared.finalize(deterministic_cms()).is_ok());
}

#[test]
fn custom_appearance_rejects_missing_geometry_and_malformed_watermarks() {
    let source = base_pdf(false);
    let mut options = SignaturePreparationOptions::invisible("Approval");
    let appearance = SignatureAppearance::default();
    assert!(prepare_incremental_signature_with_appearance(&source, &options, &appearance).is_err());

    options.target = SignatureTarget::New {
        field_name: "Approval".to_string(),
        page_index: 0,
        rect: Some(SignatureRect {
            left: 10.0,
            bottom: 10.0,
            right: 100.0,
            top: 40.0,
        }),
    };
    let malformed_watermark = SignatureAppearance {
        watermark: Some(SignatureWatermark {
            width: 2,
            height: 2,
            rgb: vec![0; 3],
            opacity: 1.0,
        }),
        ..SignatureAppearance::default()
    };
    assert!(
        prepare_incremental_signature_with_appearance(&source, &options, &malformed_watermark)
            .is_err()
    );

    let invalid_text = SignatureAppearance {
        signer_name: Some("東".to_string()),
        ..SignatureAppearance::default()
    };
    assert!(
        prepare_incremental_signature_with_appearance(&source, &options, &invalid_text).is_err()
    );
}

#[test]
fn custom_appearance_preserves_widget_local_coordinates_on_rotated_cropped_pages() {
    let source = rotated_cropped_pdf();
    let mut options = SignaturePreparationOptions::invisible("Rotated");
    options.target = SignatureTarget::New {
        field_name: "Rotated".to_string(),
        page_index: 0,
        rect: Some(SignatureRect {
            left: 30.0,
            bottom: 20.0,
            right: 150.0,
            top: 70.0,
        }),
    };
    let appearance = custom_appearance();
    let prepared =
        prepare_incremental_signature_with_appearance(&source, &options, &appearance).unwrap();
    let rendered = String::from_utf8_lossy(prepared.prepared_pdf());
    assert!(prepared.prepared_pdf().starts_with(&source));
    assert!(rendered.contains("/Rect [30 20 150 70]"));
    assert!(rendered.contains("(Santiago Fern\\341ndez) Tj"));
}

#[test]
#[ignore = "requires qpdf; exercised by the Ubuntu CI interoperability step"]
fn qpdf_accepts_finalized_visible_combined_field() {
    let source = classic_with_fields(&[(4, 7, "Existing")]);
    let mut options = SignaturePreparationOptions::existing("Existing");
    options.target = SignatureTarget::Existing {
        field_name: "Existing".to_string(),
        widget_index: None,
        rect: Some(SignatureRect {
            left: 10.0,
            bottom: 20.0,
            right: 110.0,
            top: 70.0,
        }),
    };
    let signed = prepare_incremental_signature(&source, &options)
        .unwrap()
        .finalize(deterministic_cms())
        .unwrap();

    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("combined-visible-signature.pdf");
    std::fs::write(&path, signed).unwrap();
    let output = Command::new("qpdf")
        .arg("--check")
        .arg(&path)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "combined visible signature: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
#[ignore = "requires qpdf, openssl, and pdftoppm; exercised by the Ubuntu CI interoperability step"]
fn external_tools_render_and_verify_custom_visible_signature_appearance() {
    let source = rotated_cropped_pdf();
    let mut options = SignaturePreparationOptions::invisible("Rotated");
    options.target = SignatureTarget::New {
        field_name: "Rotated".to_string(),
        page_index: 0,
        rect: Some(SignatureRect {
            left: 30.0,
            bottom: 20.0,
            right: 150.0,
            top: 70.0,
        }),
    };
    let appearance = custom_appearance();
    let prepared =
        prepare_incremental_signature_with_appearance(&source, &options, &appearance).unwrap();
    let directory = tempfile::tempdir().unwrap();
    let cms = openssl_cms_for_digest(&directory, &prepared.bytes_to_digest());
    let digest = prepared.bytes_to_digest();
    let ranges = prepared.byte_range().ranges().to_vec();
    let signed = prepared.finalize(&cms).unwrap();
    let finalized_digest: Vec<u8> = ranges
        .iter()
        .flat_map(|&(offset, length)| {
            signed[offset as usize..(offset + length) as usize]
                .iter()
                .copied()
        })
        .collect();
    assert_eq!(
        finalized_digest, digest,
        "finalization must preserve every byte verified by OpenSSL, including the appearance"
    );
    let path = directory.path().join("custom-visible-signature.pdf");
    std::fs::write(&path, signed).unwrap();
    let output = Command::new("qpdf")
        .arg("--check")
        .arg(&path)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "custom visible signature: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let prefix = directory.path().join("rendered");
    let output = Command::new("pdftoppm")
        .args(["-f", "1", "-l", "1", "-r", "72", "-cropbox", "-singlefile"])
        .arg(&path)
        .arg(&prefix)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "render custom visible signature: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let ppm = std::fs::read(prefix.with_extension("ppm")).unwrap();
    let pixels = ppm
        .windows(b"\n255\n".len())
        .position(|window| window == b"\n255\n")
        .map(|offset| &ppm[offset + b"\n255\n".len()..])
        .expect("pdftoppm must emit a PPM header with max value 255");
    assert!(
        pixels
            .chunks_exact(3)
            .any(|pixel| pixel[0] > 240 && pixel[1] < 15 && pixel[2] < 15),
        "the rendered rotated/cropped page must contain the opaque red watermark"
    );
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
